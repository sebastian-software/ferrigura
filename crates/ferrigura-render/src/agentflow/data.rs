//! The agentflow diagram as the layout and the renderer read it: a port of `getData` in
//! `packages/mermaid/src/diagrams/agentflow/agentflowDb.ts` (1696-1972), which fills the layout data of
//! [`crate::unified::data`]. The collapsed containers and the edges they redirect, the parents, the
//! nodes and the edges, the shape normalisation, and the colour slots of `colorSlots.ts` are read here,
//! from the model the parse leaves (`docs-port/agentflow/NOTES.md`, section 3.6).

use std::collections::{HashMap, HashSet};

use ferrigura_model::agentflow::{AgentflowDiagram, ClassDef, Edge as ModelEdge, SubGraph, Vertex};
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::value::Value;
use ferrigura_syntax::agentflow::semantic::{render_shape, vertex_kind};

use super::AgentflowSection;
use super::colours;
use crate::common::js_trim;
use crate::context::Globals;
use crate::unified::data::{Edge, LayoutData, Node};

/// The diagram type the renderer sets on the layout data: the markers and the stylesheet's ids use it
/// (`diag.type`, `renderer.ts` 38).
pub const DIAGRAM_TYPE: &str = "agentflow";

/// The padding of a container, which `getData` writes as `padding: 8` (`agentflowDb.ts` 1798, 1818).
const SUBGRAPH_PADDING: f64 = 8.0;

/// The padding of a vertex when `flowchart.padding` is not set: the `|| 8` of `agentflowDb.ts` 1636, which
/// also replaces a configured `0`.
const DEFAULT_PADDING: f64 = 8.0;

/// The class attribute of every edge path that is not invisible (`agentflowDb.ts` 1877).
const EDGE_CLASSES: &str = "edge-thickness-normal edge-pattern-solid flowchart-link";

/// The shape of an expanded container (`SUBGRAPH_TYPE_TO_SHAPE`, `agentflowDb.ts` 110-112).
const FLOW_GROUP: &str = "flowGroup";

/// The shape of a collapsed container (`shapes.ts`, `collapsedGroup`).
const COLLAPSED_GROUP: &str = "collapsedGroup";

/// The `fill` of an edge's arrow, which `Edge.arrowheadStyle` carries and nothing reads
/// (`agentflowDb.ts` 1880-1882).
const ARROWHEAD_STYLE: &str = "fill: #333";

/// The `getData` structure of the containers and the nodes: the collapsed containers and what they hide,
/// the parent of each node, and the containers that have members (`agentflowDb.ts` 1706-1785).
struct Structure<'a> {
    /// Hidden ids: the members of a collapsed container, and their members (`hiddenIds`).
    hidden: HashSet<&'a str>,
    /// Each hidden id to the outermost collapsed container that hides it (`collapsedAncestorMap`).
    ancestor_of: HashMap<&'a str, &'a str>,
    /// The innermost container of each node (`parentDB`), the containers that are not hidden.
    parent: HashMap<&'a str, &'a str>,
}

impl<'a> Structure<'a> {
    fn new(diagram: &'a AgentflowDiagram) -> Self {
        let by_id: HashMap<&str, &SubGraph> = diagram
            .sub_graphs
            .iter()
            .map(|sub_graph| (sub_graph.id.as_str(), sub_graph))
            .collect();

        // `collectDescendants` from each collapsed container that is not hidden already. The ancestor
        // is the container the walk starts from, and the `seen` set stops a containment cycle
        // (`agentflowDb.ts` 1714-1740).
        let mut hidden = HashSet::new();
        let mut ancestor_of = HashMap::new();
        for sub_graph in &diagram.sub_graphs {
            if !is_collapsed(sub_graph) || hidden.contains(sub_graph.id.as_str()) {
                continue;
            }
            let mut seen = HashSet::new();
            collect_descendants(
                &by_id,
                sub_graph.id.as_str(),
                sub_graph.id.as_str(),
                &mut hidden,
                &mut ancestor_of,
                &mut seen,
            );
        }

        // The parent assignment, containers last to first. A parent that would close a containment
        // cycle is refused (the warning is the semantic reading's, not the layout's).
        let mut parent: HashMap<&str, &str> = HashMap::new();
        for sub_graph in diagram.sub_graphs.iter().rev() {
            if hidden.contains(sub_graph.id.as_str()) {
                continue;
            }
            for member in &sub_graph.nodes {
                if closes_cycle(&parent, member, sub_graph.id.as_str()) {
                    continue;
                }
                parent.insert(member.as_str(), sub_graph.id.as_str());
            }
        }

        Self {
            hidden,
            ancestor_of,
            parent,
        }
    }
}

/// Whether a container is collapsed: `metadata.view === 'collapsed'`.
fn is_collapsed(sub_graph: &SubGraph) -> bool {
    matches!(
        sub_graph
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("view")),
        Some(Value::String(view)) if view == "collapsed"
    )
}

/// `collectDescendants(sgId, ancestor)` (`agentflowDb.ts` 1714-1740): every member of a container, and
/// their members, is hidden by `ancestor`, except the ancestor itself and an id already seen.
fn collect_descendants<'a>(
    by_id: &HashMap<&'a str, &'a SubGraph>,
    sub_graph_id: &'a str,
    ancestor: &'a str,
    hidden: &mut HashSet<&'a str>,
    ancestor_of: &mut HashMap<&'a str, &'a str>,
    seen: &mut HashSet<&'a str>,
) {
    if !seen.insert(sub_graph_id) {
        return;
    }
    let Some(sub_graph) = by_id.get(sub_graph_id) else {
        return;
    };
    for child in &sub_graph.nodes {
        let child = child.as_str();
        if child == ancestor || seen.contains(child) {
            continue;
        }
        hidden.insert(child);
        ancestor_of.insert(child, ancestor);
        collect_descendants(by_id, child, ancestor, hidden, ancestor_of, seen);
    }
}

/// `wouldCloseContainmentCycle(child, parent)` (`agentflowDb.ts` 1754-1772): whether the chain of parents
/// from `parent` reaches `child`. The walk stops at a repeated id.
fn closes_cycle(parent: &HashMap<&str, &str>, child: &str, container: &str) -> bool {
    let mut seen = HashSet::new();
    let mut current = Some(container);
    while let Some(id) = current {
        if !seen.insert(id) {
            break;
        }
        if id == child {
            return true;
        }
        current = parent.get(id).copied();
    }
    false
}

/// Maps the parsed diagram to the layout data (`getData`), with the configuration of the section and the
/// globals. `palette_len` is the number of colours of the theme's palette (`borderColorArray`, 0 when the
/// theme has none): the colour slots of the containers cycle through it (`colorSlots.ts`).
#[must_use]
pub fn get_data(
    diagram: &AgentflowDiagram,
    section: &AgentflowSection<'_>,
    globals: &Globals,
    palette_len: usize,
) -> LayoutData {
    let structure = Structure::new(diagram);
    let classes: HashMap<&str, &ClassDef> = diagram
        .classes
        .iter()
        .map(|class| (class.id.as_str(), class))
        .collect();
    let vertices: HashMap<&str, &Vertex> = diagram
        .vertices
        .iter()
        .map(|vertex| (vertex.id.as_str(), vertex))
        .collect();

    let mut nodes = subgraph_nodes(diagram, section, globals, &structure, &classes);
    // `normaliseNodeShapes` (`agentflowDb.ts` 1908) runs over the vertices in `vertex_nodes`, through
    // `render_shape`; the containers keep their shape.
    vertex_nodes(diagram, section, globals, &structure, &classes, &mut nodes);
    let edges = edges(diagram, section, globals, &structure, &classes);

    // `assignColorSlots` (`agentflowDb.ts` 1950-1963): a container takes its slot from the declaration
    // order, a vertex that is a kind takes the kind's class.
    let container_order = declaration_order(diagram, &structure.parent);
    let connectors: HashSet<&str> = diagram
        .connectors
        .iter()
        .map(|connector| connector.id.as_str())
        .collect();
    colours::assign(&mut nodes, palette_len, &container_order, |id| {
        if connectors.contains(id) {
            Some("connector")
        } else {
            vertices.get(id).map(|vertex| vertex_kind(vertex))
        }
    });

    LayoutData {
        nodes,
        edges,
        direction: diagram.direction.clone(),
        default_style: diagram.default_style.clone(),
        default_interpolate: diagram.default_interpolate.clone(),
        diagram_type: DIAGRAM_TYPE.to_owned(),
    }
}

/// The containers in the subgraph loop's order: the reverse of the model's completion order, the
/// containers that are not hidden or collapsed. The nodes of the containers come first, in that order
/// (`agentflowDb.ts` 1788-1834).
fn subgraph_nodes(
    diagram: &AgentflowDiagram,
    section: &AgentflowSection<'_>,
    globals: &Globals,
    structure: &Structure<'_>,
    classes: &HashMap<&str, &ClassDef>,
) -> Vec<Node> {
    let mut nodes = Vec::new();
    for sub_graph in diagram.sub_graphs.iter().rev() {
        let id = sub_graph.id.as_str();
        if structure.hidden.contains(id) {
            continue;
        }
        // The container's direction: its own, else `flowchart.inheritDir`, which gives the diagram's
        // direction (`addSubGraph`, `agentflowDb.ts` 1213-1218; the model keeps the statement only).
        let dir = sub_graph.dir.clone().or_else(|| {
            section
                .unified
                .flowchart
                .inherit_dir
                .then(|| diagram.direction.clone())
        });
        let base = Node {
            id: sub_graph.id.clone(),
            dom_id: sub_graph.id.clone(),
            label: sub_graph.title.clone(),
            label_type: label_type(&sub_graph.label_type),
            label_style: String::new(),
            parent_id: structure.parent.get(id).map(|&parent| parent.to_owned()),
            padding: SUBGRAPH_PADDING,
            css_compiled_styles: compiled_styles(
                classes,
                sub_graph.classes.iter().map(String::as_str),
            ),
            css_classes: sub_graph.classes.join(" "),
            dir,
            look: globals.look.clone(),
            ..Node::default()
        };
        if is_collapsed(sub_graph) {
            // `collapsedGroup` with the container's type in its metadata (`agentflowDb.ts` 1788-1816).
            let mut metadata = sub_graph.metadata.clone().unwrap_or_default();
            metadata.insert(
                "containerType".to_owned(),
                Value::String(sub_graph.kind.clone()),
            );
            nodes.push(Node {
                shape: COLLAPSED_GROUP.to_owned(),
                is_group: false,
                metadata,
                ..base
            });
        } else {
            nodes.push(Node {
                shape: FLOW_GROUP.to_owned(),
                is_group: true,
                metadata: sub_graph.metadata.clone().unwrap_or_default(),
                ..base
            });
        }
    }
    nodes
}

/// The vertices, in creation order (`agentflowDb.ts` 1836-1843 and `addNodeFromVertex`, 1613-1652). A
/// vertex whose id is a container node already in `nodes` changes only that node's styles and classes,
/// as the reference does; a hidden vertex is skipped.
fn vertex_nodes(
    diagram: &AgentflowDiagram,
    section: &AgentflowSection<'_>,
    globals: &Globals,
    structure: &Structure<'_>,
    classes: &HashMap<&str, &ClassDef>,
    nodes: &mut Vec<Node>,
) {
    // `config.flowchart?.padding || 8`: zero and NaN give the default.
    let padding = match section.unified.flowchart.padding {
        padding if padding != 0.0 && !padding.is_nan() => padding,
        _ => DEFAULT_PADDING,
    };
    for vertex in &diagram.vertices {
        let id = vertex.id.as_str();
        if structure.hidden.contains(id) {
            continue;
        }
        if let Some(node) = nodes.iter_mut().find(|node| node.id == vertex.id) {
            node.css_styles = vertex.styles.clone();
            node.css_compiled_styles =
                compiled_styles(classes, vertex.classes.iter().map(String::as_str));
            node.css_classes = vertex.classes.join(" ");
            continue;
        }
        let mut css_classes = String::from("default ");
        css_classes.push_str(&vertex.classes.join(" "));
        let mut compiled_names = vec!["default", "node"];
        compiled_names.extend(vertex.classes.iter().map(String::as_str));
        nodes.push(Node {
            id: vertex.id.clone(),
            dom_id: vertex.dom_id.clone(),
            label: vertex.text.clone(),
            label_type: label_type(&vertex.label_type),
            label_style: String::new(),
            parent_id: structure.parent.get(id).map(|&parent| parent.to_owned()),
            padding,
            css_styles: vertex.styles.clone(),
            css_compiled_styles: compiled_styles(classes, compiled_names),
            css_classes,
            dir: None,
            look: globals.look.clone(),
            link: vertex.link.clone(),
            link_target: vertex.link_target.clone(),
            is_group: false,
            shape: render_shape(vertex),
            min_width: Some(section.agentflow.min_node_width),
            metadata: vertex.metadata.clone().unwrap_or_default(),
            wrapping_width: Some(section.agentflow.wrapping_width),
            ..Node::default()
        });
    }
}

/// The edges in creation order, redirected to the outermost collapsed container that hides an end, and
/// the edges whose ends collapse into one container dropped (`agentflowDb.ts` 1845-1901).
fn edges(
    diagram: &AgentflowDiagram,
    section: &AgentflowSection<'_>,
    globals: &Globals,
    structure: &Structure<'_>,
    classes: &HashMap<&str, &ClassDef>,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    for raw in &diagram.edges {
        let start = structure
            .ancestor_of
            .get(raw.start.as_str())
            .copied()
            .unwrap_or(raw.start.as_str());
        let end = structure
            .ancestor_of
            .get(raw.end.as_str())
            .copied()
            .unwrap_or(raw.end.as_str());
        if start == end && raw.start != raw.end {
            continue;
        }
        let (arrow_start, arrow_end) = arrow_types(&raw.arrow);
        let invisible = raw.stroke == "invisible";
        let hidden_arrows = invisible || raw.arrow == "arrow_open";
        let mut styles: Vec<String> = diagram.default_style.clone().unwrap_or_default();
        if let Some(style) = &raw.style {
            styles.extend(style.iter().cloned());
        }
        edges.push(Edge {
            id: raw.id.clone(),
            is_user_defined_id: raw.is_user_defined_id,
            start: start.to_owned(),
            end: end.to_owned(),
            edge_type: raw.arrow.clone(),
            label: raw.text.clone(),
            label_type: label_type(&raw.label_type),
            labelpos: "c".to_owned(),
            thickness: raw.stroke.clone(),
            pattern: raw.stroke.clone(),
            minlen: Some(raw.length),
            classes: if invisible {
                String::new()
            } else {
                EDGE_CLASSES.to_owned()
            },
            arrow_type_start: if hidden_arrows {
                "none".to_owned()
            } else {
                arrow_start.to_owned()
            },
            arrow_type_end: if hidden_arrows {
                "none".to_owned()
            } else {
                arrow_end.to_owned()
            },
            arrowhead_style: ARROWHEAD_STYLE.to_owned(),
            css_compiled_styles: compiled_styles(classes, raw.classes.iter().map(String::as_str)),
            label_style: styles.clone(),
            style: styles,
            look: globals.look.clone(),
            animate: raw.animate.as_ref().and_then(as_bool),
            animation: raw.animation.as_ref().and_then(as_text),
            curve: curve(raw, diagram, &section.unified.flowchart.curve),
            ..Edge::default()
        });
    }
    edges
}

/// `destructEdgeType` (`agentflowDb.ts` 1583-1611): the arrow at the start and at the end of a link. The
/// model's arrows are `arrow_point`, `arrow_cross` and `arrow_open`; the last has no arrow (the caller
/// sets both ends to `none`).
fn arrow_types(arrow: &str) -> (&'static str, &'static str) {
    match arrow {
        "arrow_point" => ("none", "arrow_point"),
        "arrow_cross" => ("none", "arrow_cross"),
        _ => ("none", "arrow_point"),
    }
}

/// `curve`: the edge's own interpolation, else the default one from `linkStyle default interpolate`,
/// else `flowchart.curve` (`agentflowDb.ts` 1894). A value that is not text is not used.
fn curve(raw: &ModelEdge, diagram: &AgentflowDiagram, flowchart_curve: &str) -> Option<String> {
    raw.interpolate
        .as_ref()
        .and_then(as_text)
        .or_else(|| {
            diagram
                .default_interpolate
                .clone()
                .filter(|text| !text.is_empty())
        })
        .or_else(|| Some(flowchart_curve.to_owned()))
}

/// The declaration order of the containers (`containerOrder`, `agentflowDb.ts` 1926-1948): a pre-order
/// walk of the containment forest, roots in the order of the subgraphs, each one before its children. A
/// container refused by the parent assignment has no parent and is a root.
fn declaration_order<'a>(
    diagram: &'a AgentflowDiagram,
    parent: &HashMap<&'a str, &'a str>,
) -> HashMap<&'a str, usize> {
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for sub_graph in &diagram.sub_graphs {
        if let Some(&container) = parent.get(sub_graph.id.as_str()) {
            children
                .entry(container)
                .or_default()
                .push(sub_graph.id.as_str());
        }
    }
    let mut order = HashMap::new();
    for sub_graph in &diagram.sub_graphs {
        if !parent.contains_key(sub_graph.id.as_str()) {
            walk(&children, sub_graph.id.as_str(), &mut order);
        }
    }
    order
}

/// One step of [`declaration_order`]: the id gets the next number, then its children do.
fn walk<'a>(
    children: &HashMap<&str, Vec<&'a str>>,
    id: &'a str,
    order: &mut HashMap<&'a str, usize>,
) {
    if order.contains_key(id) {
        return;
    }
    let next = order.len();
    order.insert(id, next);
    for &child in children.get(id).map_or(&[][..], Vec::as_slice) {
        walk(children, child, order);
    }
}

/// The label type of a model's record: `string`, `markdown`, else `text`.
fn label_type(text: &str) -> LabelType {
    match text {
        "string" => LabelType::String,
        "markdown" => LabelType::Markdown,
        _ => LabelType::Text,
    }
}

/// The declarations of a node's classes, as `getCompiledStyles` gives them: for each class that exists,
/// its styles, then its text styles, each trimmed (`agentflowDb.ts` 1613-1625).
fn compiled_styles<'n>(
    classes: &HashMap<&str, &ClassDef>,
    names: impl IntoIterator<Item = &'n str>,
) -> Vec<String> {
    let mut compiled = Vec::new();
    for name in names {
        let Some(class) = classes.get(name) else {
            continue;
        };
        compiled.extend(class.styles.iter().map(|style| js_trim(style).to_owned()));
        compiled.extend(
            class
                .text_styles
                .iter()
                .map(|style| js_trim(style).to_owned()),
        );
    }
    compiled
}

/// A JavaScript truthy text: a non-empty string.
fn as_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        _ => None,
    }
}

/// A boolean value, when the metadata's `animate` is one.
fn as_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(flag) => Some(*flag),
        _ => None,
    }
}
