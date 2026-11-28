//! Adapts Ferrigura's measured flowchart data to the experimental TALA engine.

use ferrigura_model::flowchart::FlowchartDiagram;

use crate::RenderError;
use crate::animation::Animation;
use crate::context::Context;
use crate::unified::{Config, Section};

/// Draws a flowchart with the experimental TALA engine and the normal painter.
///
/// # Errors
/// Returns a layout error for unsupported or invalid geometry and the usual render errors.
pub fn render_with_tala(
    input: &Context<'_, FlowchartDiagram, Section<'_>>,
    animation: Option<Animation>,
) -> Result<String, RenderError> {
    let config = Config::of(input);
    let mut measured = super::measure(input)?;
    let graph = ferrigura_tala::Graph {
        direction: Some(measured.data.direction.clone()),
        nodes: measured
            .data
            .nodes
            .iter()
            .map(|node| {
                let (label_width, label_height) = node.label_bbox.unwrap_or_default();
                ferrigura_tala::Node {
                    id: node.id.clone(),
                    parent_id: node.parent_id.clone(),
                    direction: node.dir.clone(),
                    is_group: node.is_group,
                    shape: node.shape.clone(),
                    width: node.width.unwrap_or_default(),
                    height: node.height.unwrap_or_default(),
                    label: node.label.clone(),
                    label_width,
                    label_height,
                }
            })
            .collect(),
        edges: measured
            .data
            .edges
            .iter()
            .map(|edge| ferrigura_tala::Edge {
                id: edge.id.clone(),
                source: edge.start.clone(),
                target: edge.end.clone(),
                source_arrow: !matches!(edge.arrow_type_start.as_str(), "" | "none"),
                target_arrow: !matches!(edge.arrow_type_end.as_str(), "" | "none"),
                label: edge.label.clone(),
                label_width: edge.width.unwrap_or_default(),
                label_height: edge.height.unwrap_or_default(),
            })
            .collect(),
    };
    let layout =
        ferrigura_tala::layout(&graph).map_err(|error| RenderError::Layout(error.to_string()))?;
    super::pilot::apply_native(&mut measured, layout, &config)?;
    super::paint_measured(input, &measured, &config, animation)
}
