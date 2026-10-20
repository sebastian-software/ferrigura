//! The swimlane layout step of the unified renderer: the preparation of the layout data, the swimlane
//! engine's layout, and the clipping of its edges. It stands beside the ELK step of [`super::pipeline`].
//!
//! The reference runs these in `layout-algorithms/swimlanes/` (`index.ts`, `helpers.ts`,
//! `edgeLabelNodes.ts`, `layoutCore.ts`) and in `insertEdge`'s swimlane branch (`edges.js` 654-700).
//! The engine itself is `ferrigura-swimlane`, which takes the layout data as the reference's seam does
//! (`runSwimlaneLayoutCore`), and this module converts to and from its model.
//!
//! The order is the reference's: `prepare` (`prepareLayoutForSwimlanes`, then `createEdgeLabelNodes`)
//! before the measure, so that the label nodes are measured like the other nodes; then the engine after
//! the measure; then the paint, which clips the edges.

use std::collections::{HashMap, HashSet};

use ferrigura_model::config::SwimlaneDiagramConfig;
use ferrigura_swimlane::{
    Config as EngineConfig, Edge as EngineEdge, FlowchartConfig, LayoutData as EngineLayout,
    Node as EngineNode, Point as EnginePoint, SwimlaneConfig,
};

use super::Config;
use super::data::{Edge, LayoutData, Node, SwimlaneFields};
use super::edges::{self, swimlane_points};
use super::geom::{CenterBox, Point};
use super::line_jumps::JumpStyle;
use super::pipeline::Measured;
use super::shapes::{Intersect, ShapeOutput};
use crate::RenderError;

/// The id of the lane that holds the nodes outside every subgraph, `DEFAULT_SWIMLANE_ID` in
/// `helpers.ts`.
pub const DEFAULT_LANE_ID: &str = "__swimlane_default__";

/// The padding of the synthesised lane (`helpers.ts` 129).
const DEFAULT_LANE_PADDING: f64 = 20.0;

/// What the preparation did to the diagram, which the engine's layout and the paint need.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Prepared {
    /// The label node of each edge that had a label, in the order of the edges (`createEdgeLabelNodes`).
    pub labels: Vec<EdgeLabel>,
    /// The curve of each edge as `getData` gave it. The measure clears it (`edges::prepare`), and the
    /// engine reads it: `basis` and no curve become `rounded`.
    pub curves: Vec<Option<String>>,
}

/// The label node of a labelled edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeLabel {
    /// The index of the edge in the diagram's edges.
    pub edge: usize,
    /// The id of the label node, `edge-label-<start>-<end>-<edge id>`.
    pub node_id: String,
}

/// Prepares the layout data for the swimlane engine: the lanes, the default lane for the loose nodes, and a
/// label node for every labelled edge (`prepareSwimlaneLayout` in `index.ts`). The layout data changes in
/// place: the edges lose their labels, which the label nodes now carry.
pub fn prepare(data: &mut LayoutData, config: &Config<'_>) -> Prepared {
    let curves = data.edges.iter().map(|edge| edge.curve.clone()).collect();
    prepare_lanes(data, config);
    let labels = create_edge_label_nodes(data);
    Prepared { labels, curves }
}

/// `prepareLayoutForSwimlanes` (`helpers.ts` 116-146): every top-level subgraph is a lane, which takes the
/// diagram's direction, and the loose nodes go into the default lane, which is made when there is none.
fn prepare_lanes(data: &mut LayoutData, config: &Config<'_>) {
    let direction = data.direction.clone();
    for node in data.nodes.iter_mut() {
        if node.is_group && is_top_level(node) {
            make_lane(node, &direction);
        }
    }

    let loose: Vec<usize> = data
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| !node.is_group && is_top_level(node))
        .map(|(index, _)| index)
        .collect();
    if loose.is_empty() {
        return;
    }

    match data
        .nodes
        .iter()
        .position(|node| node.id == DEFAULT_LANE_ID)
    {
        None => {
            // Synthesised, so nothing upstream gave it a look or a palette slot: the next slot after the
            // largest one that any node has (`helpers.ts` 124-129).
            let colour_index = data
                .nodes
                .iter()
                .filter_map(|node| node.color_index)
                .max()
                .map_or(0, |largest| largest + 1);
            let mut lane = Node {
                id: DEFAULT_LANE_ID.to_owned(),
                dom_id: DEFAULT_LANE_ID.to_owned(),
                padding: DEFAULT_LANE_PADDING,
                look: config.globals.look.clone(),
                is_group: true,
                color_index: Some(colour_index),
                ..Node::default()
            };
            make_lane(&mut lane, &direction);
            data.nodes.push(lane);
        }
        Some(index) => {
            if data.nodes[index].is_group {
                make_lane(&mut data.nodes[index], &direction);
            }
        }
    }
    for index in loose {
        data.nodes[index].parent_id = Some(DEFAULT_LANE_ID.to_owned());
    }
}

/// A top-level group, or a node with no parent: the reference's `!parentId`.
fn is_top_level(node: &Node) -> bool {
    node.parent_id.as_deref().unwrap_or_default().is_empty()
}

/// Makes a node a lane: its shape is `swimlane`, and it takes the diagram's direction.
fn make_lane(node: &mut Node, direction: &str) {
    node.shape = "swimlane".to_owned();
    node.swimlane = Some(SwimlaneFields {
        direction: direction.to_owned(),
        content_top: None,
    });
}

/// `createEdgeLabelNodes` (`edgeLabelNodes.ts`): each labelled edge gets a label node, which the layout
/// places between its ends, in the lane of its source, or of its target when the edge crosses lanes. The
/// edge loses its label, and it is stamped with the label node. An edge whose end is not a node keeps its
/// label, as the reference does (it logs a warning and skips the edge).
fn create_edge_label_nodes(data: &mut LayoutData) -> Vec<EdgeLabel> {
    let index_of: HashMap<&str, usize> = data
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();

    let mut created: Vec<(usize, Node, String)> = Vec::new();
    for (index, edge) in data.edges.iter().enumerate() {
        if edge.label.is_empty() {
            continue;
        }
        let (Some(&source), Some(&target)) = (
            index_of.get(edge.start.as_str()),
            index_of.get(edge.end.as_str()),
        ) else {
            continue;
        };
        let (source_node, target_node) = (&data.nodes[source], &data.nodes[target]);
        let node_id = format!("edge-label-{}-{}-{}", edge.start, edge.end, edge.id);
        // A cross-lane edge's label goes to the target's lane, which keeps it near where the edge heads.
        let lane = if source_node.parent_id != target_node.parent_id {
            target_node.parent_id.clone()
        } else {
            source_node.parent_id.clone()
        };
        let label = Node {
            id: node_id.clone(),
            dom_id: node_id.clone(),
            label: edge.label.clone(),
            // `Array.isArray(edge.labelStyle) ? edge.labelStyle[0] : ...`: the first declaration.
            label_style: edge.label_style.first().cloned().unwrap_or_default(),
            parent_id: lane,
            shape: "labelRect".to_owned(),
            dir: source_node.dir.clone(),
            ..Node::default()
        };
        created.push((index, label, node_id));
    }

    let mut labels = Vec::with_capacity(created.len());
    for (index, label, node_id) in created {
        data.edges[index].label = String::new();
        data.nodes.push(label);
        labels.push(EdgeLabel {
            edge: index,
            node_id,
        });
    }
    labels
}

/// Lays the measured diagram out with the swimlane engine, and writes its result back: the positions and
/// sizes of the nodes and lanes, and the points and curves of the edges (`runSwimlaneLayoutCore`, with the
/// layout data that the reference's `runLayoutCore` gets).
///
/// The engine's virtual edges, which carry each label node between its edge's ends, are laid out and never
/// drawn; they are added here and dropped after the layout.
///
/// # Errors
///
/// Returns an error if the engine does not lay the diagram out: a non-default option of the swimlane
/// section, or an id or a direction the engine does not take.
pub fn lay_out(
    measured: &mut Measured,
    config: &Config<'_>,
    prepared: &Prepared,
    swimlane: &SwimlaneDiagramConfig,
) -> Result<(), RenderError> {
    let mut layout = to_engine(&measured.data, config, prepared, swimlane);
    ferrigura_swimlane::run_swimlane_layout_core(&mut layout)
        .map_err(|error| RenderError::Layout(error.to_string()))?;
    write_back(&mut measured.data, &layout);
    Ok(())
}

/// The layout data of the engine, as the reference's `runSwimlaneLayoutCore` reads it.
fn to_engine(
    data: &LayoutData,
    config: &Config<'_>,
    prepared: &Prepared,
    swimlane: &SwimlaneDiagramConfig,
) -> EngineLayout {
    let label_nodes: HashSet<&str> = prepared
        .labels
        .iter()
        .map(|label| label.node_id.as_str())
        .collect();
    let nodes = data
        .nodes
        .iter()
        .map(|node| {
            let is_label = label_nodes.contains(node.id.as_str());
            EngineNode {
                id: node.id.clone(),
                parent_id: node.parent_id.clone(),
                is_group: Some(node.is_group),
                shape: Some(node.shape.clone()),
                is_dummy: is_label.then_some(true),
                is_edge_label: is_label.then_some(true),
                width: node.width,
                height: node.height,
                // A label node has no padding in the reference (`createEdgeLabelNodes`).
                padding: (!is_label).then_some(node.padding),
                look: (!node.look.is_empty()).then(|| node.look.clone()),
                color_index: node.color_index.map(f64::from),
                dir: node.dir.clone(),
                ..EngineNode::default()
            }
        })
        .collect();

    let label_of_edge: HashMap<usize, &str> = prepared
        .labels
        .iter()
        .map(|label| (label.edge, label.node_id.as_str()))
        .collect();
    let mut edges: Vec<EngineEdge> = data
        .edges
        .iter()
        .enumerate()
        .map(|(index, edge)| EngineEdge {
            id: edge.id.clone(),
            start: Some(edge.start.clone()),
            end: Some(edge.end.clone()),
            label: (!edge.label.is_empty()).then(|| edge.label.clone()),
            label_node_id: label_of_edge.get(&index).map(|id| (*id).to_owned()),
            curve: prepared.curves.get(index).cloned().flatten(),
            arrow_type_start: Some(edge.arrow_type_start.clone()),
            arrow_type_end: Some(edge.arrow_type_end.clone()),
            ..EngineEdge::default()
        })
        .collect();
    // The two virtual edges of each label, `<id>-to-label` and `<id>-from-label` (`edgeLabelNodes.ts`).
    for label in &prepared.labels {
        let edge = &data.edges[label.edge];
        edges.push(EngineEdge {
            id: format!("{}-to-label", edge.id),
            start: Some(edge.start.clone()),
            end: Some(label.node_id.clone()),
            is_layout_only: Some(true),
            ..EngineEdge::default()
        });
        edges.push(EngineEdge {
            id: format!("{}-from-label", edge.id),
            start: Some(label.node_id.clone()),
            end: Some(edge.end.clone()),
            is_layout_only: Some(true),
            ..EngineEdge::default()
        });
    }

    let section = config.section.flowchart;
    EngineLayout {
        nodes,
        edges,
        direction: (!data.direction.is_empty()).then(|| data.direction.clone()),
        config: EngineConfig {
            flowchart: Some(FlowchartConfig {
                node_spacing: Some(section.node_spacing as f64),
                rank_spacing: Some(section.rank_spacing as f64),
                ..FlowchartConfig::default()
            }),
            swimlane: Some(SwimlaneConfig {
                ignore_cross_lane_edges: Some(swimlane.ignore_cross_lane_edges),
                optimize_ranks_by_crossings: Some(swimlane.optimize_ranks_by_crossings),
                automatic_lane_ordering: Some(swimlane.automatic_lane_ordering),
                ..SwimlaneConfig::default()
            }),
            ..EngineConfig::default()
        },
        ..EngineLayout::default()
    }
}

/// Writes the engine's result back into the layout data (`writeBackToLayoutData` and the edges' points).
fn write_back(data: &mut LayoutData, layout: &EngineLayout) {
    for (node, result) in data.nodes.iter_mut().zip(&layout.nodes) {
        node.x = result.x;
        node.y = result.y;
        node.width = result.width;
        node.height = result.height;
        if let Some(lane) = node.swimlane.as_mut() {
            lane.content_top = result.swimlane_content_top;
        }
    }
    // The engine's virtual edges come after the diagram's own, so the zip stops at the diagram's edges.
    for (edge, result) in data.edges.iter_mut().zip(&layout.edges) {
        edge.points = result
            .points
            .as_ref()
            .map_or_else(Vec::new, |points| points.iter().map(engine_point).collect());
        edge.curve = result.curve.clone();
    }
}

fn engine_point(point: &EnginePoint) -> Point {
    Point::new(point.x, point.y)
}

/// The edges of the diagram, clipped the way the swimlane branch of `insertEdge` clips them: each end of an
/// edge with at least two points is clipped to the outline of its node, or of its lane. The lanes are given
/// with their final sizes (`lanes`, by id), since `swimlane.js` sets them while it draws them.
#[must_use]
pub fn clip_edges(
    edges: &[Edge],
    nodes: &[Node],
    shapes: &HashMap<String, ShapeOutput>,
    lanes: &HashMap<String, (f64, f64)>,
) -> Vec<Edge> {
    let by_id: HashMap<&str, &Node> = nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let outline = |id: &str| -> Option<(Intersect, CenterBox)> {
        let node = by_id.get(id)?;
        let (x, y) = (node.x.unwrap_or(0.0), node.y.unwrap_or(0.0));
        if node.is_group {
            // A lane is a rectangle (`intersectRect`), drawn with its final size.
            let &(width, height) = lanes.get(id)?;
            Some((Intersect::Rect, CenterBox::new(x, y, width, height)))
        } else {
            let shape = shapes.get(id)?;
            let (width, height) = (node.width.unwrap_or(0.0), node.height.unwrap_or(0.0));
            Some((shape.intersect.clone(), CenterBox::new(x, y, width, height)))
        }
    };

    edges
        .iter()
        .map(|edge| {
            let mut clipped = edge.clone();
            // `insertEdge` clips an edge with two points or more, whose two ends both have an outline.
            if let (true, Some((tail, tail_box)), Some((head, head_box))) = (
                edge.points.len() >= 2,
                outline(&edge.start),
                outline(&edge.end),
            ) {
                clipped.points = swimlane_points(
                    &edge.points,
                    |point| tail.intersect(tail_box, point),
                    |point| head.intersect(head_box, point),
                );
            }
            clipped
        })
        .collect()
}

/// The line hops of the swimlane section, `swimlane.lineHops`: none when it is `false`.
#[must_use]
pub fn line_hops(swimlane: &SwimlaneDiagramConfig) -> Option<JumpStyle> {
    edges::jump_style_of(swimlane.line_hops.as_ref())
}
