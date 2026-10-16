//! ELK Layered, ported from the Eclipse Layout Kernel Java sources.
//!
//! The port follows the elkjs 0.9.3 build, which GWT compiled from ELK commit
//! `9bc93474e` (see `docs-port/elk/SOURCES.md` in the repository). Java
//! semantics that GWT emulates differently from the JVM are reproduced where
//! they affect results: `java.util.Random`, the merge sort used by
//! `Collections.sort`, and 32-bit integer arithmetic.
//!
//! Modules are named after the Java packages they port. Files keep the Eclipse
//! Public License 2.0 notice of the original sources; see `LICENSE` and `NOTICE.md`.

use std::collections::HashMap;

pub mod common;
pub mod core;
pub mod error;
pub mod graph;
pub mod java;
pub mod json;
pub mod layered;

pub use error::LayoutError;

/// The output of a layout run: the input graph with coordinates, and the processor trace.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The graph as elkjs returns it: the input with `x`, `y`, `width` and `height` set.
    pub graph: json::JsonNode,
    /// The logging tree, present when `logging` was requested.
    pub trace: core::util::progress::TraceNode,
}

/// Lays out an elkjs JSON graph the way `elk.layout(graph, { logging })` does.
pub fn layout(graph: &json::JsonNode, logging: bool) -> Result<Layout, LayoutError> {
    let mut elk = graph::json_import::import_graph(graph)?;
    let (mut progress, root) = core::util::progress::ProgressMonitor::new(logging);
    core::recursive::layout(&mut elk, &mut progress, root)?;
    let trace = progress.trace(root);
    let mut output = graph.clone();
    output.width = Some(elk.shape.width);
    output.height = Some(elk.shape.height);
    write_level(&mut output, &elk);
    let mut edges = HashMap::new();
    collect_edges(&elk, &mut edges);
    write_edges(&mut output, &edges);
    Ok(Layout {
        graph: output,
        trace,
    })
}

/// Writes the layout of the children of one level into their JSON: each child with its position,
/// size and labels, then the child's own level.
fn write_level(output: &mut json::JsonNode, elk: &graph::ElkNode) {
    for (json, node) in output.children.iter_mut().zip(elk.children.iter()) {
        json.x = Some(node.shape.x);
        json.y = Some(node.shape.y);
        json.width = Some(node.shape.width);
        json.height = Some(node.shape.height);
        // The labels with text are laid out; the others keep what the input gave them.
        for (json_label, label) in json.labels.iter_mut().zip(node.labels.iter()) {
            if label.text.as_deref().is_some_and(|text| !text.is_empty()) {
                json_label.x = Some(label.shape.x);
                json_label.y = Some(label.shape.y);
                json_label.width = Some(label.shape.width);
                json_label.height = Some(label.shape.height);
            }
        }
        write_level(json, node);
    }
}

/// Every edge of the graph, by id, from the level it was moved to.
fn collect_edges<'a>(node: &'a graph::ElkNode, edges: &mut HashMap<&'a str, &'a graph::ElkEdge>) {
    for edge in &node.edges {
        edges.insert(edge.id.as_str(), edge);
    }
    for child in &node.children {
        collect_edges(child, edges);
    }
}

/// Writes the edges' routes into the JSON of the level where the input has each edge, found by id.
fn write_edges(output: &mut json::JsonNode, edges: &HashMap<&str, &graph::ElkEdge>) {
    for json in output.edges.iter_mut() {
        let Some(edge) = edges.get(json.id.as_str()) else {
            continue;
        };
        // Each edge's route: one section, `<id>_s<index>`, with its shapes as the endpoint ids.
        json.sections = edge
            .sections
            .iter()
            .enumerate()
            .map(|(index, section)| json::JsonEdgeSection {
                id: format!("{}_s{}", edge.id, index),
                start_point: Some(json::JsonPoint {
                    x: section.start.x,
                    y: section.start.y,
                }),
                end_point: Some(json::JsonPoint {
                    x: section.end.x,
                    y: section.end.y,
                }),
                bend_points: section
                    .bend_points
                    .iter()
                    .map(|point| json::JsonPoint {
                        x: point.x,
                        y: point.y,
                    })
                    .collect(),
                incoming_shape: edge.sources.first().cloned(),
                outgoing_shape: edge.targets.first().cloned(),
                ..json::JsonEdgeSection::default()
            })
            .collect();
        // The labels with text are laid out; the others keep what the input gave them.
        for (json_label, label) in json.labels.iter_mut().zip(edge.labels.iter()) {
            if label.text.as_deref().is_some_and(|text| !text.is_empty()) {
                json_label.x = Some(label.shape.x);
                json_label.y = Some(label.shape.y);
                json_label.width = Some(label.shape.width);
                json_label.height = Some(label.shape.height);
            }
        }
    }
    for child in output.children.iter_mut() {
        write_edges(child, edges);
    }
}
