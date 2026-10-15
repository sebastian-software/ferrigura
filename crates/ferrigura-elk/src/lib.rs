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
    for (json, node) in output.children.iter_mut().zip(elk.children.iter_mut()) {
        json.x = Some(node.shape.x);
        json.y = Some(node.shape.y);
        json.width = Some(node.shape.width);
        json.height = Some(node.shape.height);
    }
    // Each edge's route: one section, `<id>_s<index>`, with its shapes as the endpoint ids.
    for (json, edge) in output.edges.iter_mut().zip(elk.edges.iter()) {
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
    Ok(Layout {
        graph: output,
        trace,
    })
}
