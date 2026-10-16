//! The flowchart as the layout and the renderer read it: a port of `getData` in
//! `packages/mermaid/src/diagrams/flowchart/flowDb.ts`, which fills the layout data of
//! [`crate::unified::data`]. See `docs-port/flowchart/render-notes/getdata.md`. The differential run
//! (`pnpm port:diff flowchart-data`) compares it with the reference's `getData()`.

use std::collections::{HashMap, HashSet};

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::{FlowClass, FlowSubgraph, FlowVertex, FlowchartDiagram};
use ferrigura_model::value::Value;

pub use crate::unified::data::{Edge, LayoutData, Node};

/// The padding of a subgraph, which the reference hard-codes (`flowDb.ts` 1245 and 1263).
const SUBGRAPH_PADDING: f64 = 8.0;

/// The padding of a vertex when the configuration gives none: the reference's `|| 8` (`flowDb.ts`
/// 1071), which also replaces a configured `0`.
const DEFAULT_PADDING: f64 = 8.0;

/// The class attribute of every edge path that is not invisible (`flowDb.ts` 1322-1325).
const EDGE_CLASSES: &str = "edge-thickness-normal edge-pattern-solid flowchart-link";

/// Maps the parsed flowchart to the layout data, as `getData` does (`flowDb.ts`, `getData`, lines
/// 1119-1347): the nodes of the subgraphs and the vertices, the edges, and the collapsed subgraphs
/// with the edges they redirect.
///
/// The configuration is read as the reference reads it at this point: `look` from the top level,
/// `flowchart.padding`, `flowchart.minNodeWidth`, `flowchart.curve` and `flowchart.inheritDir`.
#[must_use]
pub fn get_data(diagram: &FlowchartDiagram, config: &MermaidConfig) -> LayoutData {
    let classes: HashMap<&str, &FlowClass> = diagram
        .classes
        .iter()
        .map(|class| (class.id.as_str(), class))
        .collect();
    let tooltips: HashMap<&str, &str> = diagram
        .tooltips
        .iter()
        .map(|tooltip| (tooltip.id.as_str(), tooltip.tooltip.as_str()))
        .collect();
    let structure = Structure::new(diagram);

    let mut nodes = subgraph_nodes(diagram, config, &structure, &classes);
    vertex_nodes(diagram, config, &structure, &classes, &tooltips, &mut nodes);
    let edges = edges(diagram, config, &structure, &classes);

    LayoutData {
        nodes,
        edges,
        direction: diagram.direction.clone(),
        default_style: diagram.default_style.clone(),
        default_interpolate: diagram.default_interpolate.clone(),
        centre_node_labels: false,
        diagram_type: String::new(),
    }
}

/// The subgraph structure that `getData` derives from the containment of the subgraphs: the
/// parents, the collapsed subgraphs and the ids that they hide, and the colour slots.
struct Structure<'a> {
    /// The subgraph that directly contains each member, the innermost one (`parentDB`, built from
    /// the visible subgraphs only).
    member_parent: HashMap<&'a str, &'a str>,
    /// The visible subgraphs that have members (`subGraphDB`). A vertex with one of these ids is
    /// a group.
    has_members: HashSet<&'a str>,
    /// The subgraphs and members that are not drawn, inside a collapsed ancestor (`hiddenIds`).
    hidden: HashSet<&'a str>,
    /// Each hidden id to the visible collapsed node that replaces it, and the edges are redirected
    /// to (`collapsedAncestorMap`).
    ancestor_of: HashMap<&'a str, &'a str>,
    /// The colour slot of every subgraph, in source order (`declarationIndex`).
    declaration_index: HashMap<&'a str, u32>,
}

impl<'a> Structure<'a> {
    /// Derives the structure of `diagram`, as the start of `getData` does (`flowDb.ts`
    /// 1124-1215).
    fn new(diagram: &'a FlowchartDiagram) -> Self {
        let subgraphs = &diagram.subgraphs;
        let by_id: HashMap<&str, &FlowSubgraph> = subgraphs
            .iter()
            .map(|subgraph| (subgraph.id.as_str(), subgraph))
            .collect();

        // `subGraphParent`: a member that is a subgraph id belongs to the subgraph that lists it,
        // and a later listing in closing order overwrites an earlier one.
        let mut parent: HashMap<&'a str, &'a str> = HashMap::new();
        for subgraph in subgraphs {
            for child in &subgraph.nodes {
                if by_id.contains_key(child.as_str()) {
                    parent.insert(child.as_str(), subgraph.id.as_str());
                }
            }
        }

        let declaration_index = declaration_index(subgraphs, &parent);

        // `outermostCollapsed`, for every subgraph: the hidden subgraphs and members, each with
        // the collapsed ancestor that replaces it.
        let mut hidden = HashSet::new();
        let mut ancestor_of: HashMap<&'a str, &'a str> = HashMap::new();
        for subgraph in subgraphs {
            let Some(ancestor) = outermost_collapsed(subgraph.id.as_str(), &parent, &by_id) else {
                continue;
            };
            if subgraph.id.as_str() != ancestor {
                hidden.insert(subgraph.id.as_str());
                ancestor_of.insert(subgraph.id.as_str(), ancestor);
            }
            for child in &subgraph.nodes {
                if child.as_str() == ancestor {
                    continue;
                }
                hidden.insert(child.as_str());
                ancestor_of.insert(child.as_str(), ancestor);
            }
        }

        // The members of the visible subgraphs. The walk runs backwards and each write overwrites, so
        // a member gets the subgraph with the lowest closing index: the innermost one, since nested
        // subgraphs close first (`flowDb.ts` 1218-1229).
        let mut member_parent = HashMap::new();
        let mut has_members = HashSet::new();
        for subgraph in subgraphs.iter().rev() {
            if hidden.contains(subgraph.id.as_str()) {
                continue;
            }
            if !subgraph.nodes.is_empty() {
                has_members.insert(subgraph.id.as_str());
            }
            for member in &subgraph.nodes {
                member_parent.insert(member.as_str(), subgraph.id.as_str());
            }
        }

        Self {
            member_parent,
            has_members,
            hidden,
            ancestor_of,
            declaration_index,
        }
    }
}

/// `declarationIndex` (`flowDb.ts` 1156-1175): the colour slot of each subgraph, from a pre-order
/// walk of the containment forest, roots and children in source order. Hidden and collapsed
/// subgraphs keep their slot.
fn declaration_index<'a>(
    subgraphs: &'a [FlowSubgraph],
    parent: &HashMap<&'a str, &'a str>,
) -> HashMap<&'a str, u32> {
    let mut children: HashMap<&'a str, Vec<&'a str>> = HashMap::new();
    for subgraph in subgraphs {
        if let Some(&owner) = parent.get(subgraph.id.as_str()) {
            children
                .entry(owner)
                .or_default()
                .push(subgraph.id.as_str());
        }
    }

    fn walk<'a>(
        id: &'a str,
        children: &HashMap<&'a str, Vec<&'a str>>,
        next: &mut u32,
        index: &mut HashMap<&'a str, u32>,
    ) {
        index.insert(id, *next);
        *next += 1;
        if let Some(list) = children.get(id) {
            for child in list {
                walk(child, children, next, index);
            }
        }
    }

    let mut index = HashMap::new();
    let mut next = 0;
    for subgraph in subgraphs {
        if !parent.contains_key(subgraph.id.as_str()) {
            walk(subgraph.id.as_str(), &children, &mut next, &mut index);
        }
    }
    index
}

/// `outermostCollapsed` (`flowDb.ts` 1179-1191): the last collapsed subgraph on the way up from
/// `start`, which is the outermost. A seen set guards the walk.
fn outermost_collapsed<'a>(
    start: &'a str,
    parent: &HashMap<&'a str, &'a str>,
    by_id: &HashMap<&str, &FlowSubgraph>,
) -> Option<&'a str> {
    let mut result = None;
    let mut seen = HashSet::new();
    let mut current = Some(start);
    while let Some(id) = current {
        if !seen.insert(id) {
            break;
        }
        if by_id.get(id).is_some_and(|subgraph| is_collapsed(subgraph)) {
            result = Some(id);
        }
        current = parent.get(id).copied();
    }
    result
}

/// Whether a subgraph is collapsed: its `view` metadata is `collapsed` (`flowDb.ts` 1177-1178).
fn is_collapsed(subgraph: &FlowSubgraph) -> bool {
    subgraph.metadata.get("view").and_then(Value::as_str) == Some("collapsed")
}

/// The direction of a subgraph: its own `direction` statement, else the diagram's direction when
/// `flowchart.inheritDir` is set. The reference applies the inheritance when the subgraph is
/// added (`flowDb.ts` 721-726); the model keeps only the statement.
fn subgraph_dir(
    subgraph: &FlowSubgraph,
    diagram: &FlowchartDiagram,
    config: &MermaidConfig,
) -> Option<String> {
    subgraph.dir.clone().or_else(|| {
        config
            .flowchart
            .inherit_dir
            .then(|| diagram.direction.clone())
    })
}

/// The subgraph nodes, in reverse closing order (`flowDb.ts` 1232-1279): a collapsed subgraph as a
/// `collapsedGroup`, any other as an expanded group. Hidden subgraphs are skipped.
fn subgraph_nodes(
    diagram: &FlowchartDiagram,
    config: &MermaidConfig,
    structure: &Structure<'_>,
    classes: &HashMap<&str, &FlowClass>,
) -> Vec<Node> {
    let mut nodes = Vec::new();
    for subgraph in diagram.subgraphs.iter().rev() {
        let id = subgraph.id.as_str();
        if structure.hidden.contains(id) {
            continue;
        }
        let base = Node {
            id: subgraph.id.clone(),
            dom_id: subgraph.id.clone(),
            label: subgraph.title.clone(),
            label_type: subgraph.label_type,
            label_style: String::new(),
            parent_id: structure
                .member_parent
                .get(id)
                .map(|&parent| parent.to_owned()),
            padding: SUBGRAPH_PADDING,
            css_compiled_styles: compiled_styles(
                classes,
                subgraph.classes.iter().map(String::as_str),
            ),
            css_classes: subgraph.classes.join(" "),
            dir: subgraph_dir(subgraph, diagram, config),
            look: config.look.clone(),
            color_index: structure.declaration_index.get(id).copied(),
            ..Node::default()
        };
        nodes.push(if is_collapsed(subgraph) {
            Node {
                shape: String::from("collapsedGroup"),
                is_group: false,
                ..base
            }
        } else {
            Node {
                shape: String::from("rect"),
                is_group: true,
                metadata: subgraph.metadata.clone(),
                ..base
            }
        });
    }
    nodes
}

/// The vertex nodes, in creation order (`flowDb.ts` 1281-1288 and `addNodeFromVertex`,
/// 1048-1103). A vertex with the id of a subgraph node already in `nodes` changes only that node's
/// styles and classes, as the reference does; a hidden vertex is skipped.
fn vertex_nodes(
    diagram: &FlowchartDiagram,
    config: &MermaidConfig,
    structure: &Structure<'_>,
    classes: &HashMap<&str, &FlowClass>,
    tooltips: &HashMap<&str, &str>,
    nodes: &mut Vec<Node>,
) {
    let padding = if config.flowchart.padding == 0.0 || config.flowchart.padding.is_nan() {
        DEFAULT_PADDING
    } else {
        config.flowchart.padding
    };
    // `config.look || 'classic'`, passed to every vertex node (`flowDb.ts` 1287).
    let look = if config.look.is_empty() {
        String::from("classic")
    } else {
        config.look.clone()
    };
    let mut positions: HashMap<String, usize> = nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.id.clone(), position))
        .collect();

    for vertex in &diagram.vertices {
        let id = vertex.id.as_str();
        if structure.hidden.contains(id) {
            continue;
        }
        if let Some(&position) = positions.get(id) {
            let node = &mut nodes[position];
            node.css_styles = vertex.styles.clone();
            node.css_compiled_styles =
                compiled_styles(classes, vertex.classes.iter().map(String::as_str));
            node.css_classes = vertex.classes.join(" ");
            continue;
        }

        let is_group = structure.has_members.contains(id);
        let base = Node {
            id: vertex.id.clone(),
            dom_id: format!("flowchart-{}-{}", vertex.id, vertex.dom_counter),
            label: vertex.text.clone(),
            label_type: vertex.label_type,
            label_style: String::new(),
            parent_id: structure
                .member_parent
                .get(id)
                .map(|&parent| parent.to_owned()),
            padding,
            css_styles: vertex.styles.clone(),
            css_compiled_styles: compiled_styles(
                classes,
                ["default", "node"]
                    .into_iter()
                    .chain(vertex.classes.iter().map(String::as_str)),
            ),
            css_classes: format!("default {}", vertex.classes.join(" ")),
            dir: None,
            look: look.clone(),
            link: vertex.link.clone(),
            link_target: vertex.link_target.clone(),
            tooltip: tooltips.get(id).map(|&tooltip| tooltip.to_owned()),
            icon: vertex.icon.clone(),
            pos: vertex.pos.clone(),
            img: vertex.img.clone(),
            asset_width: vertex.asset_width,
            asset_height: vertex.asset_height,
            constraint: vertex.constraint.clone(),
            ..Node::default()
        };
        positions.insert(vertex.id.clone(), nodes.len());
        nodes.push(if is_group {
            Node {
                is_group: true,
                shape: String::from("rect"),
                ..base
            }
        } else {
            Node {
                is_group: false,
                shape: vertex_shape(vertex),
                min_width: Some(config.flowchart.min_node_width),
                ..base
            }
        });
    }
}

/// `getTypeFromVertex` (`flowDb.ts` 995-1023): the shape of a vertex. An image wins over an icon,
/// and an icon over the bracket shape.
fn vertex_shape(vertex: &FlowVertex) -> String {
    if truthy(&vertex.img) {
        return String::from("imageSquare");
    }
    if truthy(&vertex.icon) {
        let shape = match vertex.form.as_deref() {
            Some("circle") => "iconCircle",
            Some("square") => "iconSquare",
            Some("rounded") => "iconRounded",
            _ => "icon",
        };
        return shape.to_owned();
    }
    match vertex.shape.as_deref() {
        None | Some("square") => String::from("squareRect"),
        Some("round") => String::from("roundedRect"),
        Some(other) => other.to_owned(),
    }
}

/// A JavaScript truthy string: present and not empty.
fn truthy(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|text| !text.is_empty())
}

/// The edges in creation order (`flowDb.ts` 1290-1344), redirected to the collapsed subgraphs,
/// and without the edges that a collapsed subgraph hides inside itself.
fn edges(
    diagram: &FlowchartDiagram,
    config: &MermaidConfig,
    structure: &Structure<'_>,
    classes: &HashMap<&str, &FlowClass>,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    for raw in &diagram.edges {
        let (arrow_type_start, arrow_type_end) = destruct_edge_type(&raw.edge_type);

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
        // An edge that becomes a self-loop only because a collapsed subgraph holds both ends is
        // dropped; a self-loop on a node that was never collapsed stays.
        if start == end
            && (structure.ancestor_of.contains_key(raw.start.as_str())
                || structure.ancestor_of.contains_key(raw.end.as_str()))
        {
            continue;
        }

        let mut styles = diagram.default_style.clone().unwrap_or_default();
        if let Some(style) = &raw.style {
            styles.extend(style.iter().cloned());
        }
        let invisible = raw.stroke == "invisible";
        let hidden_arrows = invisible || raw.edge_type == "arrow_open";
        // `rawEdge.interpolate || defaultInterpolate || config.flowchart?.curve`, where the empty
        // string is false in JavaScript.
        let curve = [
            raw.interpolate.as_deref(),
            diagram.default_interpolate.as_deref(),
            Some(config.flowchart.curve.as_str()),
        ]
        .into_iter()
        .flatten()
        .find(|value| !value.is_empty())
        .map(str::to_owned);

        edges.push(Edge {
            id: raw.id.clone(),
            is_user_defined_id: raw.is_user_defined_id,
            start: start.to_owned(),
            end: end.to_owned(),
            edge_type: raw.edge_type.clone(),
            label: raw.text.clone(),
            label_type: raw.label_type,
            labelpos: String::from("c"),
            thickness: raw.stroke.clone(),
            pattern: raw.stroke.clone(),
            minlen: raw.length,
            classes: if invisible {
                String::new()
            } else {
                String::from(EDGE_CLASSES)
            },
            arrow_type_start: if hidden_arrows {
                String::from("none")
            } else {
                arrow_type_start
            },
            arrow_type_end: if hidden_arrows {
                String::from("none")
            } else {
                arrow_type_end
            },
            arrowhead_style: String::from("fill: #333"),
            css_compiled_styles: compiled_styles(classes, raw.classes.iter().map(String::as_str)),
            label_style: styles.clone(),
            style: styles,
            look: config.look.clone(),
            animate: raw.animate,
            animation: raw.animation.clone(),
            curve,
            ..Edge::default()
        });
    }
    edges
}

/// `destructEdgeType` (`flowDb.ts` 1028-1046): the marker at the start and at the end of an arrow
/// type. The defaults are no marker at the start and a point at the end; a double arrow has the
/// same marker at both ends.
fn destruct_edge_type(edge_type: &str) -> (String, String) {
    match edge_type {
        "arrow_point" | "arrow_circle" | "arrow_cross" => {
            (String::from("none"), edge_type.to_owned())
        }
        "double_arrow_point" | "double_arrow_circle" | "double_arrow_cross" => {
            let arrow = edge_type.replacen("double_", "", 1);
            (arrow.clone(), arrow)
        }
        _ => (String::from("none"), String::from("arrow_point")),
    }
}

/// `getCompiledStyles` (`flowDb.ts` 1105-1117): for each class name that is defined, its styles and
/// then its text styles, each trimmed. The text styles are the styles that mention `color`, with the
/// first `fill` renamed `bgFill` (`flowDb.ts` 431-434), which the reference derives while adding the
/// class.
fn compiled_styles<'n>(
    classes: &HashMap<&str, &FlowClass>,
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
                .styles
                .iter()
                .filter(|style| style.contains("color"))
                .map(|style| js_trim(&style.replacen("fill", "bgFill", 1)).to_owned()),
        );
    }
    compiled
}

/// `String.prototype.trim`: Rust's whitespace less U+0085, and with the byte order mark, which
/// JavaScript trims and Rust does not.
fn js_trim(text: &str) -> &str {
    text.trim_matches(|c: char| (c.is_whitespace() && c != '\u{85}') || c == '\u{FEFF}')
}
