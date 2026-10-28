//! The pipelines: the box around the components of each pipeline, the dotted links between its
//! components in the order of their evolution, and the move of the parent to the top of the box
//! (`wardleyRenderer.ts` 306-380). The move changes the placements that the links, the trends and
//! the nodes read after it, so the steps run in the reference's order.

use super::Canvas;
use super::plot::{Placement, Placements};
use crate::common::Element;
use crate::common::d3::{js_number, math_max, math_min, sort_js};

/// The padding of a pipeline's box around its components, on each side (`wardleyRenderer.ts` 354).
const BOX_PADDING: f64 = 15.0;

/// `g.wardley-pipelines` and `g.wardley-pipeline-links`, when the map has any pipeline. Each is
/// created even when no pipeline has components to draw in it.
#[must_use]
pub fn draw(canvas: &Canvas<'_>, placements: &mut Placements<'_>) -> Option<(Element, Element)> {
    let diagram = canvas.diagram;
    if diagram.pipelines.is_empty() {
        return None;
    }
    let node_radius = canvas.node_radius();
    let square_size = canvas.square_size();
    let mut boxes = Element::new("g").with("class", "wardley-pipelines");
    let mut links = Element::new("g").with("class", "wardley-pipeline-links");
    for pipeline in &diagram.pipelines {
        if pipeline.component_ids.is_empty() {
            continue;
        }

        // The components that have a node, sorted by their evolution (`sortedComponents`). The sort
        // is V8's, so that two components with the same evolution keep the order of the text.
        let mut sorted: Vec<usize> = pipeline
            .component_ids
            .iter()
            .filter_map(|id| placements.index_of(id))
            .collect();
        sort_js(&mut sorted, |a, b| diagram.nodes[a].x - diagram.nodes[b].x);

        // A dotted link between consecutive components, from their positions before the move below.
        for pair in sorted.windows(2) {
            let current = placements.at(pair[0]);
            let next = placements.at(pair[1]);
            links.push_child(
                Element::new("line")
                    .with("class", "wardley-pipeline-evolution-link")
                    .with("x1", js_number(current.x))
                    .with("y1", js_number(current.y))
                    .with("x2", js_number(next.x))
                    .with("y2", js_number(next.y))
                    .with("stroke", canvas.colours.link_stroke.as_str())
                    .with("stroke-width", "1")
                    .with("stroke-dasharray", "4 4"),
            );
        }

        // The extent of the components, and the height of the last component's position.
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut y = 0.0;
        for id in &pipeline.component_ids {
            if let Some(index) = placements.index_of(id) {
                let position = placements.at(index);
                min_x = math_min(min_x, position.x);
                max_x = math_max(max_x, position.x);
                y = position.y;
            }
        }

        if min_x != f64::INFINITY && max_x != f64::NEG_INFINITY {
            let height = node_radius * 4.0;
            let box_top = y - height / 2.0;
            // The parent sits at the top of the box, two thirds outside it and one third inside.
            if let Some(parent) = placements.index_of(&pipeline.node_id) {
                placements.set(
                    parent,
                    Placement {
                        x: (min_x + max_x) / 2.0,
                        y: box_top - square_size / 6.0,
                    },
                );
            }
            boxes.push_child(
                Element::new("rect")
                    .with("class", "wardley-pipeline-box")
                    .with("x", js_number(min_x - BOX_PADDING))
                    .with("y", js_number(box_top))
                    .with("width", js_number(max_x - min_x + BOX_PADDING * 2.0))
                    .with("height", js_number(height))
                    .with("fill", "none")
                    .with("stroke", canvas.colours.axis.as_str())
                    .with("stroke-width", "1.5")
                    .with("rx", "4")
                    .with("ry", "4"),
            );
        }
    }
    Some((boxes, links))
}
