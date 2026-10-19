//! The nodes (`g.wardley-nodes`): one group per node, with its sourcing overlay, its circle or square,
//! its market symbol, its inertia mark and its label (`wardleyRenderer.ts` 584-823).
//!
//! The reference draws each element in a pass over the nodes, so that the elements of a pass come
//! before the next pass's. Each group receives its elements from its own node only, so the
//! children of a group are the same when the passes are run one node at a time, in the passes'
//! order. This module does that: the order of the children is the order of the reference's passes.

use std::f64::consts::PI;

use ferrigura_jsmath::{cos, sin};
use ferrigura_model::wardley::{Node, NodeClass, Strategy};

use super::Canvas;
use super::plot::{Placement, Placements};
use super::text::{Drawn, label};
use crate::common::Element;
use crate::common::d3::js_number;

/// The radius of the small circles of a market symbol, as a share of the node radius
/// (`wardleyRenderer.ts` 661).
const MARKET_DOT_SHARE: f64 = 0.7;
/// The distance of a market symbol's corners from its centre, as a share of the node radius
/// (`wardleyRenderer.ts` 662).
const MARKET_TRIANGLE_SHARE: f64 = 1.2;
/// The distance of an inertia mark from the node's centre, past the node's edge (`:750`).
const INERTIA_GAP: f64 = 15.0;
/// The extra distance an inertia mark takes when the node has a sourcing overlay (`:753`).
const INERTIA_OVERLAY_GAP: f64 = 10.0;

/// The class of a node's group: `wardley-node`, and the node's class when it has one.
fn group_class(node: &Node) -> String {
    match node.class_name {
        Some(class) => format!("wardley-node wardley-node--{}", class_text(class)),
        None => "wardley-node".to_owned(),
    }
}

/// The name of a node's class, as the DB gives it.
fn class_text(class: NodeClass) -> &'static str {
    match class {
        NodeClass::Anchor => "anchor",
        NodeClass::Component => "component",
        NodeClass::PipelineComponent => "pipeline-component",
    }
}

/// The circle of a node: centred on it, with the given radius, fill and stroke.
fn circle(
    class: Option<&str>,
    at: Placement,
    radius: f64,
    fill: &str,
    stroke: &str,
    width: &str,
) -> Element {
    let mut element = Element::new("circle");
    if let Some(class) = class {
        element.set("class", class);
    }
    element
        .with("cx", js_number(at.x))
        .with("cy", js_number(at.y))
        .with("r", js_number(radius))
        .with("fill", fill)
        .with("stroke", stroke)
        .with("stroke-width", width)
}

/// `g.wardley-nodes`: one group per node, in the order of the nodes (`wardleyRenderer.ts` 584).
#[must_use]
pub fn draw(canvas: &Canvas<'_>, placements: &Placements<'_>, drawn: &mut Drawn) -> Element {
    let node_radius = canvas.node_radius();
    let square_size = canvas.square_size();
    let colours = &canvas.colours;
    let (cos30, sin30) = (cos(PI / 6.0), sin(PI / 6.0));
    let triangle = node_radius * MARKET_TRIANGLE_SHARE;
    let dot_radius = node_radius * MARKET_DOT_SHARE;

    let mut nodes = Element::new("g").with("class", "wardley-nodes");
    for (index, node) in canvas.diagram.nodes.iter().enumerate() {
        let at = placements.at(index);
        let mut group = Element::new("g").with("class", group_class(node));
        let parent = node.is_pipeline_parent == Some(true);
        let market = node.source_strategy == Some(Strategy::Market);

        // The sourcing overlay, behind the circle: the outsourced, bought and built components
        // have a larger circle, and a market component a larger white one.
        match node.source_strategy {
            Some(Strategy::Outsource) => group.push_child(circle(
                Some("wardley-outsource-overlay"),
                at,
                node_radius * 2.0,
                "#666",
                &colours.component_stroke,
                "1",
            )),
            Some(Strategy::Buy) => group.push_child(circle(
                Some("wardley-buy-overlay"),
                at,
                node_radius * 2.0,
                "#ccc",
                &colours.component_stroke,
                "1",
            )),
            Some(Strategy::Build) => group.push_child(circle(
                Some("wardley-build-overlay"),
                at,
                node_radius * 2.0,
                "#eee",
                "#000",
                "1",
            )),
            Some(Strategy::Market) => group.push_child(circle(
                Some("wardley-market-overlay"),
                at,
                node_radius * 2.0,
                "white",
                &colours.component_stroke,
                "1",
            )),
            None => {}
        }

        // The circle of a component, but not of an anchor, a market component or a pipeline parent.
        if !parent && !market && node.class_name != Some(NodeClass::Anchor) {
            group.push_child(circle(
                None,
                at,
                node_radius,
                &colours.component_fill,
                &colours.component_stroke,
                "1",
            ));
        }

        // The market symbol: three lines joining three small circles at the corners of a triangle.
        if market {
            let top = Placement {
                x: at.x,
                y: at.y - triangle,
            };
            let bottom_left = Placement {
                x: at.x - triangle * cos30,
                y: at.y + triangle * sin30,
            };
            let bottom_right = Placement {
                x: at.x + triangle * cos30,
                y: at.y + triangle * sin30,
            };
            for (from, to) in [
                (top, bottom_left),
                (bottom_left, bottom_right),
                (bottom_right, top),
            ] {
                group.push_child(
                    Element::new("line")
                        .with("class", "wardley-market-line")
                        .with("x1", js_number(from.x))
                        .with("y1", js_number(from.y))
                        .with("x2", js_number(to.x))
                        .with("y2", js_number(to.y))
                        .with("stroke", colours.component_stroke.as_str())
                        .with("stroke-width", "1"),
                );
            }
            for corner in [top, bottom_left, bottom_right] {
                group.push_child(
                    Element::new("circle")
                        .with("class", "wardley-market-dot")
                        .with("cx", js_number(corner.x))
                        .with("cy", js_number(corner.y))
                        .with("r", js_number(dot_radius))
                        .with("fill", "white")
                        .with("stroke", colours.component_stroke.as_str())
                        .with("stroke-width", "2"),
                );
            }
        }

        // The square of a pipeline parent.
        if parent {
            group.push_child(
                Element::new("rect")
                    .with("x", js_number(at.x - square_size / 2.0))
                    .with("y", js_number(at.y - square_size / 2.0))
                    .with("width", js_number(square_size))
                    .with("height", js_number(square_size))
                    .with("fill", colours.component_fill.as_str())
                    .with("stroke", colours.component_stroke.as_str())
                    .with("stroke-width", "1"),
            );
        }

        // The inertia mark, to the right of the node: further out when the node has an overlay.
        if node.inertia == Some(true) {
            let mut offset = if parent {
                square_size / 2.0 + INERTIA_GAP
            } else {
                node_radius + INERTIA_GAP
            };
            if node.source_strategy.is_some() {
                offset += node_radius + INERTIA_OVERLAY_GAP;
            }
            let line_height = if parent {
                square_size
            } else {
                node_radius * 2.0
            };
            group.push_child(
                Element::new("line")
                    .with("class", "wardley-inertia")
                    .with("x1", js_number(at.x + offset))
                    .with("y1", js_number(at.y - line_height / 2.0))
                    .with("x2", js_number(at.x + offset))
                    .with("y2", js_number(at.y + line_height / 2.0))
                    .with("stroke", colours.component_stroke.as_str())
                    .with("stroke-width", "6"),
            );
        }

        // The label, last in the group. An anchor's label is bold.
        let (x, y) = label_position(node, at, canvas);
        let (fill, weight, anchor, baseline) = if node.class_name == Some(NodeClass::Anchor) {
            drawn.bold(&node.label);
            ("#000", "bold", "middle", "middle")
        } else {
            drawn.regular(&node.label);
            (colours.component_label.as_str(), "normal", "start", "auto")
        };
        group.push_child(label(
            &node.label,
            vec![
                ("x", js_number(x)),
                ("y", js_number(y)),
                ("class", "wardley-node-label".to_owned()),
                ("fill", fill.to_owned()),
                ("font-size", js_number(canvas.config.label_font_size)),
                ("font-weight", weight.to_owned()),
                ("text-anchor", anchor.to_owned()),
                ("dominant-baseline", baseline.to_owned()),
            ],
        ));
        nodes.push_child(group);
    }
    nodes
}

/// Where a node's label starts. An anchor's label is centred on the anchor, with no offset unless
/// the statement gives one. A component's label is offset to the right and up, by the configured
/// offset, and by 10 more when it has a sourcing strategy and the statement gives no offset
/// (`wardleyRenderer.ts` 784-808).
fn label_position(node: &Node, at: Placement, canvas: &Canvas<'_>) -> (f64, f64) {
    if node.class_name == Some(NodeClass::Anchor) {
        let x = node.label_offset_x.map_or(at.x, |offset| at.x + offset.0);
        let y = node
            .label_offset_y
            .map_or(at.y - 3.0, |offset| at.y + offset.0);
        return (x, y);
    }
    let mut default_x = canvas.config.node_label_offset;
    if node.source_strategy.is_some() && node.label_offset_x.is_none() {
        default_x += 10.0;
    }
    let mut default_y = -canvas.config.node_label_offset;
    if node.source_strategy.is_some() && node.label_offset_y.is_none() {
        default_y -= 10.0;
    }
    let x = at.x + node.label_offset_x.map_or(default_x, |offset| offset.0);
    let y = at.y + node.label_offset_y.map_or(default_y, |offset| offset.0);
    (x, y)
}
