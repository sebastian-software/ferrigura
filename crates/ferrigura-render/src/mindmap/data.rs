//! The layout data of a mindmap: a port of `getData` (`mindmapDb.ts` 357-420) with the flatten and
//! edge steps it calls (`flattenNodes` 227-306, `generateEdges` 313-350), and the shape overrides of
//! `mindmapRenderer.ts` 58-74, which the renderer applies to these records before it draws them.
//!
//! The nodes are in pre-order, the edges in the order `generateEdges` pushes them (each child's edge,
//! then the edges below that child, before the next child). The records are the unified renderer's
//! [`LayoutData`], so the shared measure step, the shapes and the tidy tree take them as they are.
//! The reference's dead fields (`shapes`, `nodeSpacing`, `rankSpacing`, `diagramId`, `type`) are not
//! kept. The layout name is not read here: the caller resolves it (`layout.rs`).

use std::collections::BTreeMap;

use ferrigura_model::config::MindmapDiagramConfig;
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::mindmap::{MindmapDiagram, MindmapNode};

use crate::context::Globals;
use crate::unified::data::{Edge, LayoutData, Node};

/// The name of the diagram type, which the markers and the root's ids are built from.
pub const DIAGRAM_TYPE: &str = "mindmap";

/// `MAX_SECTIONS` (`mindmapDb.ts` 12) minus one: a section is `index % 11` (`mindmapDb.ts` 216).
const SECTIONS: u32 = 11;

/// The node types of the DB (`mindmapDb.ts` 28-37).
const DEFAULT_TYPE: u8 = 0;
const ROUNDED_RECT_TYPE: u8 = 1;
const RECT_TYPE: u8 = 2;
const CIRCLE_TYPE: u8 = 3;
const CLOUD_TYPE: u8 = 4;
const BANG_TYPE: u8 = 5;
const HEXAGON_TYPE: u8 = 6;

/// The layout data of a mindmap tree, as `getData` returns it, without the layout choice (which is
/// the caller's, see `layout.rs`).
#[must_use]
pub fn layout_data(
    diagram: &MindmapDiagram,
    globals: &Globals,
    mindmap: &MindmapDiagramConfig,
) -> LayoutData {
    let look = if globals.look.is_empty() {
        "classic".to_owned()
    } else {
        globals.look.clone()
    };
    let mut data = LayoutData {
        direction: "TB".to_owned(),
        diagram_type: DIAGRAM_TYPE.to_owned(),
        ..LayoutData::default()
    };
    let Some(root) = diagram.root.as_ref() else {
        // `getData` returns no nodes and no edges for a tree without a root (`mindmapDb.ts` 369-375).
        return data;
    };
    let by_depth = mindmap.color_by == "depth";

    // `assignSections` (`mindmapDb.ts` 199-220) and `flattenNodes` (227-306), in pre-order.
    let mut placed = Vec::new();
    place(root, None, 0, by_depth, &mut placed);
    let sections: BTreeMap<u32, Option<u32>> = placed
        .iter()
        .map(|(node, section)| (node.id, *section))
        .collect();
    for (node, section) in &placed {
        data.nodes.push(flatten(node, *section, &look));
    }

    // `generateEdges` (313-350): each child's edge, then the edges below it, before the next child.
    generate_edges(root, &sections, &look, &mut data.edges);
    data
}

/// `assignSections`: the section of a node, from its parent's and its depth. The root has none; the
/// root's children take `index % 11`, and their descendants inherit it; with `colorBy: depth` every
/// node takes `(depth - 1) % 11`.
fn place<'a>(
    node: &'a MindmapNode,
    section_param: Option<u32>,
    depth: u32,
    by_depth: bool,
    out: &mut Vec<(&'a MindmapNode, Option<u32>)>,
) {
    let section = if node.level == 0 {
        None
    } else if by_depth {
        Some((depth - 1) % SECTIONS)
    } else {
        section_param
    };
    out.push((node, section));
    for (index, child) in node.children.iter().enumerate() {
        // The children of the root are numbered; the others inherit the parameter of their parent
        // (`mindmapDb.ts` 216), not the section they were given.
        let child_section = if node.level == 0 {
            Some((index as u32) % SECTIONS)
        } else {
            section_param
        };
        place(child, child_section, depth + 1, by_depth, out);
    }
}

/// `flattenNodes` for one node (`mindmapDb.ts` 227-306), with the shape overrides of
/// `mindmapRenderer.ts` 58-74 already applied, since the renderer applies them before the render.
fn flatten(node: &MindmapNode, section: Option<u32>, look: &str) -> Node {
    let mut classes = vec!["mindmap-node".to_owned()];
    if node.level == 0 {
        classes.push("section-root".to_owned());
        classes.push("section--1".to_owned());
    } else if let Some(section) = section {
        classes.push(format!("section-{section}"));
    }
    if !node.class.is_empty() {
        classes.push(node.class.clone());
    }

    let mut width = node.width.as_f64();
    let mut padding = node.padding.as_f64().unwrap_or(0.0);
    let mut height = Some(0.0);
    let shape = shape_of(node.node_type);
    match shape {
        // `rounded`: width 0 and padding 15 (`mindmapRenderer.ts` 59-64).
        "rounded" => {
            width = Some(0.0);
            padding = 15.0;
        }
        // `rect`: width 0 and padding 10 (`mindmapRenderer.ts` 67-69).
        "rect" => {
            width = Some(0.0);
            padding = 10.0;
        }
        // `hexagon`: width and height 0 (`mindmapRenderer.ts` 70-73).
        "hexagon" => {
            width = Some(0.0);
            height = Some(0.0);
        }
        _ => {}
    }

    Node {
        id: node.id.to_string(),
        dom_id: format!("node_{}", node.id),
        label: node.descr.clone(),
        label_type: LabelType::Markdown,
        padding,
        css_classes: classes.join(" "),
        look: look.to_owned(),
        // An icon is stored and never drawn (`mindmap/NOTES.md` 8, question 3), but the label
        // takes a background when the node has one (`labelHelper`, `util.ts` 88).
        icon: (!node.icon.is_empty()).then(|| node.icon.clone()),
        shape: shape.to_owned(),
        width,
        height,
        center_label: Some(true),
        metadata: BTreeMap::new(),
        ..Node::default()
    }
}

/// `getShapeFromType` (`mindmapDb.ts` 248-270). Only the redux themes ship, so the default node is
/// the rounded rectangle; the classic themes' `defaultMindmapNode` is out of scope.
#[must_use]
pub fn shape_of(node_type: u8) -> &'static str {
    match node_type {
        CIRCLE_TYPE => "mindmapCircle",
        RECT_TYPE => "rect",
        ROUNDED_RECT_TYPE => "rounded",
        CLOUD_TYPE => "cloud",
        BANG_TYPE => "bang",
        HEXAGON_TYPE => "hexagon",
        DEFAULT_TYPE => "rounded",
        _ => "rect",
    }
}

/// `generateEdges` (`mindmapDb.ts` 313-350): the edge into each child, then the edges below the child.
/// The child's section is the one [`place`] gave it.
fn generate_edges(
    node: &MindmapNode,
    sections: &BTreeMap<u32, Option<u32>>,
    look: &str,
    edges: &mut Vec<Edge>,
) {
    for child in &node.children {
        let child_section = sections.get(&child.id).copied().flatten();
        let mut classes = String::from("edge");
        if let Some(section) = child_section {
            classes.push_str(&format!(" section-edge-{section}"));
        }
        classes.push_str(&format!(" edge-depth-{}", node.level + 1));
        edges.push(Edge {
            id: format!("edge_{}_{}", node.id, child.id),
            start: node.id.to_string(),
            end: child.id.to_string(),
            edge_type: "normal".to_owned(),
            curve: Some("basis".to_owned()),
            thickness: "normal".to_owned(),
            look: look.to_owned(),
            classes,
            ..Edge::default()
        });
        generate_edges(child, sections, look, edges);
    }
}
