//! The links between nodes (`g.wardley-links`: the lines, then their labels), and the trends
//! (`g.wardley-trends`) (`wardleyRenderer.ts` 382-582). A line stops at the edge of each end's
//! circle, or of its square for a pipeline parent. A link with a coincident end writes `NaN`, as the
//! reference's division does.

use std::collections::{HashMap, HashSet};

use ferrigura_jsmath::atan2;
use ferrigura_model::wardley::{Flow, Link, Node, WardleyDiagram};

use super::Canvas;
use super::plot::Placements;
use super::text::{Drawn, label};
use crate::common::Element;
use crate::common::d3::js_number;

/// The distance of a link's label from the middle of its line (`wardleyRenderer.ts` 488).
const LABEL_OFFSET: f64 = 8.0;
/// The gap a trend's arrow keeps from the node's circle (`wardleyRenderer.ts` 555).
const TREND_GAP: f64 = 2.0;

/// The radius a link stops short of a node: the circle's, or half the diagonal of a pipeline
/// parent's square (`squareSize / Math.sqrt(2)`).
fn radius_of(node: &Node, canvas: &Canvas<'_>) -> f64 {
    if node.is_pipeline_parent == Some(true) {
        canvas.square_size() / 2f64.sqrt()
    } else {
        canvas.node_radius()
    }
}

/// The links that draw: those whose two ends are nodes, without the dotted link from a pipeline's
/// component to its parent (`wardleyRenderer.ts` 390-403). Each is returned with the indices of its
/// ends.
fn valid_links<'a>(
    diagram: &'a WardleyDiagram,
    placements: &Placements<'_>,
) -> Vec<(usize, usize, &'a Link)> {
    let members: HashMap<&str, HashSet<&str>> = diagram
        .pipelines
        .iter()
        .map(|pipeline| {
            (
                pipeline.node_id.as_str(),
                pipeline.component_ids.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    diagram
        .links
        .iter()
        .filter_map(|link| {
            let source = placements.index_of(&link.source)?;
            let target = placements.index_of(&link.target)?;
            let from_component = members
                .get(link.target.as_str())
                .is_some_and(|components| components.contains(link.source.as_str()));
            (!from_component).then_some((source, target, link))
        })
        .collect()
}

/// `g.wardley-links`: one line per valid link, then one label per valid link that has a label
/// (`wardleyRenderer.ts` 382-530).
#[must_use]
pub fn links(canvas: &Canvas<'_>, placements: &Placements<'_>, drawn: &mut Drawn) -> Element {
    let valid = valid_links(canvas.diagram, placements);
    let mut group = Element::new("g").with("class", "wardley-links");
    for (source, target, link) in &valid {
        let from = placements.at(*source);
        let to = placements.at(*target);
        let source_radius = radius_of(&canvas.diagram.nodes[*source], canvas);
        let target_radius = radius_of(&canvas.diagram.nodes[*target], canvas);
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let distance = (dx * dx + dy * dy).sqrt();
        let (back_dx, back_dy) = (from.x - to.x, from.y - to.y);
        let back_distance = (back_dx * back_dx + back_dy * back_dy).sqrt();

        let mut line = Element::new("line")
            .with(
                "class",
                if link.dashed {
                    "wardley-link wardley-link--dashed"
                } else {
                    "wardley-link"
                },
            )
            .with("x1", js_number(from.x + (dx / distance) * source_radius))
            .with("y1", js_number(from.y + (dy / distance) * source_radius))
            .with(
                "x2",
                js_number(to.x + (back_dx / back_distance) * target_radius),
            )
            .with(
                "y2",
                js_number(to.y + (back_dy / back_distance) * target_radius),
            )
            .with("stroke", canvas.colours.link_stroke.as_str())
            .with("stroke-width", "1");
        if link.dashed {
            line.set("stroke-dasharray", "6 6");
        }
        if matches!(link.flow, Some(Flow::Forward | Flow::Bidirectional)) {
            line.set("marker-end", format!("url(#link-arrow-end-{})", canvas.id));
        }
        if matches!(link.flow, Some(Flow::Backward | Flow::Bidirectional)) {
            line.set(
                "marker-start",
                format!("url(#link-arrow-start-{})", canvas.id),
            );
        }
        group.push_child(line);
    }

    for (source, target, link) in &valid {
        let Some(text) = link.label.as_deref().filter(|text| !text.is_empty()) else {
            continue;
        };
        let from = placements.at(*source);
        let to = placements.at(*target);
        let mid_x = (from.x + to.x) / 2.0;
        let mid_y = (from.y + to.y) / 2.0;
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let distance = (dx * dx + dy * dy).sqrt();
        // The normal of the line, pointing up from it.
        let perp_x = dy / distance;
        let perp_y = -dx / distance;
        let label_x = mid_x + perp_x * LABEL_OFFSET;
        let label_y = mid_y + perp_y * LABEL_OFFSET;
        let mut angle = (atan2(dy, dx) * 180.0) / std::f64::consts::PI;
        // A label that would read upside down is turned over.
        if angle > 90.0 || angle < -90.0 {
            angle += 180.0;
        }
        drawn.regular(text);
        group.push_child(label(
            text,
            vec![
                ("class", "wardley-link-label".to_owned()),
                ("x", js_number(label_x)),
                ("y", js_number(label_y)),
                ("fill", canvas.colours.axis_text.clone()),
                ("font-size", js_number(canvas.config.label_font_size)),
                ("text-anchor", "middle".to_owned()),
                ("dominant-baseline", "middle".to_owned()),
                (
                    "transform",
                    format!(
                        "rotate({} {} {})",
                        js_number(angle),
                        js_number(label_x),
                        js_number(label_y)
                    ),
                ),
            ],
        ));
    }
    group
}

/// `g.wardley-trends`: one dashed arrow per `evolve` statement whose node is drawn, from the node
/// towards its target, stopping `nodeRadius + 2` short of the target (`wardleyRenderer.ts`
/// 532-582). The arrow starts at the node's placement after the pipeline move, and it ends at the
/// target of the statement, which is not moved with the parent.
#[must_use]
pub fn trends(canvas: &Canvas<'_>, placements: &Placements<'_>) -> Element {
    let plot = &canvas.plot;
    let mut group = Element::new("g").with("class", "wardley-trends");
    for trend in &canvas.diagram.trends {
        let Some(index) = placements.index_of(&trend.node_id) else {
            continue;
        };
        let origin = placements.at(index);
        let target_x = plot.x(trend.target_x);
        let target_y = plot.y(trend.target_y);
        let (dx, dy) = (target_x - origin.x, target_y - origin.y);
        let distance = (dx * dx + dy * dy).sqrt();
        let shorten_by = canvas.node_radius() + TREND_GAP;
        let (x2, y2) = if distance > shorten_by {
            (
                target_x - (dx / distance) * shorten_by,
                target_y - (dy / distance) * shorten_by,
            )
        } else {
            (target_x, target_y)
        };
        group.push_child(
            Element::new("line")
                .with("class", "wardley-trend")
                .with("x1", js_number(origin.x))
                .with("y1", js_number(origin.y))
                .with("x2", js_number(x2))
                .with("y2", js_number(y2))
                .with("stroke", canvas.colours.evolution_stroke.as_str())
                .with("stroke-width", "1")
                .with("stroke-dasharray", "4 4")
                .with("marker-end", format!("url(#arrow-{})", canvas.id)),
        );
    }
    group
}
