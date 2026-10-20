//! The collapsed subgraph (`flowchart` `@{ view: collapsed }`): a box with its title, a dashed
//! separator and three dots. A port of `shapes/collapsedGroup.ts`, for the default container type
//! (the `flow` type of agentflow is the same geometry with other colours and a stroke width).
//! The colours of the box are the style task's; the geometry and the structure are here.

use super::util::{Part, attribute, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::svg::number;
use crate::unified::geom::{BBox, Point};
use crate::unified::palette::{palette, palette_slot};

/// The space under the title, where the separator is drawn.
const SEPARATOR_GAP: f64 = 8.0;
/// The height of the row of dots under the separator.
const INDICATOR_ROW_HEIGHT: f64 = 20.0;
/// The narrowest a collapsed group is.
const MIN_WIDTH: f64 = 80.0;
/// The corner radius of the default container, the same as an expanded subgraph's.
const RADIUS: f64 = 8.0;
/// The radius of a dot.
const DOT_RADIUS: f64 = 2.5;
/// The distance between the centres of two dots.
const DOT_SPACING: f64 = 10.0;

/// The colour attribute of a collapsed group: the `data-color-id` of its palette slot, which
/// `stampColorSlot` writes on the node's group before the look (`collapsedGroup.ts` 73-77), for
/// every container type. Empty when the theme has no palette or the container has no slot.
pub(super) fn colour_attribute(input: &ShapeInput<'_>) -> String {
    let Some(palette) = palette(input.config.globals, input.config.theme) else {
        return String::new();
    };
    palette_slot(input.node.color_index, &palette).map_or_else(String::new, |slot| {
        format!(" data-color-id=\"color-{slot}\"")
    })
}

/// `collapsedGroup.ts`: the box around the title, the dashed separator under it, and three dots.
/// The title is moved up by half the separator gap and the indicator row, so the title sits in
/// the top part of the box.
pub(super) fn collapsed_group(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let is_flow = node
        .metadata
        .get("containerType")
        .and_then(|value| value.as_str())
        == Some("flow");
    let radius = if is_flow { 10.0 } else { RADIUS };
    let label = label_helper(input, false);

    let title_height = label.bbox.height;
    let padding = node.padding;
    let total_width = (label.bbox.width + padding * 2.0)
        .max(MIN_WIDTH)
        .max(node.width.unwrap_or(0.0));
    let total_height = (title_height + SEPARATOR_GAP + INDICATOR_ROW_HEIGHT + padding * 2.0)
        .max(node.height.unwrap_or(0.0));
    let x = -total_width / 2.0;
    let y = -total_height / 2.0;

    // The label moves up into the title area, to leave room for the separator and the dots.
    let label_shift_y = -(SEPARATOR_GAP + INDICATOR_ROW_HEIGHT) / 2.0;
    let label_offset = Point::new(0.0, -title_height / 2.0 + label_shift_y);

    let mut rect = format!(
        "<rect class=\"basic label-container {}\"{} rx=\"{}\" ry=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
        if is_flow {
            "flow-collapsed"
        } else {
            "collapsed-group"
        },
        attribute("style", &styles.node),
        number(radius),
        number(radius),
        number(x),
        number(y),
        number(total_width),
        number(total_height),
    );
    if is_flow {
        // The fill of the flow container is `none`; its colours are the style task's.
        rect.push_str(" fill=\"none\" stroke-width=\"0.75px\"");
    }
    rect.push_str("/>");

    // The separator and the dots are drawn after the label, as the reference appends them.
    let separator_y = y + padding + title_height + SEPARATOR_GAP;
    let mut after_label = format!(
        "<line class=\"collapsed-separator\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke-dasharray=\"3, 3\"/>",
        number(x + 8.0),
        number(separator_y),
        number(x + total_width - 8.0),
        number(separator_y),
    );
    let dot_y = separator_y + INDICATOR_ROW_HEIGHT / 2.0;
    for index in -1..=1 {
        after_label.push_str(&format!(
            "<circle class=\"collapsed-indicator\" cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
            number(f64::from(index) * DOT_SPACING),
            number(dot_y),
            number(DOT_RADIUS),
        ));
    }

    // The box of the group: the rectangle, the separator (zero high) and the dots.
    let mut bbox = BBox::new(x, y, total_width, total_height);
    bbox = bbox.union(BBox::new(x + 8.0, separator_y, total_width - 16.0, 0.0));
    for index in -1..=1 {
        let centre_x = f64::from(index) * DOT_SPACING;
        bbox = bbox.union(BBox::new(
            centre_x - DOT_RADIUS,
            dot_y - DOT_RADIUS,
            2.0 * DOT_RADIUS,
            2.0 * DOT_RADIUS,
        ));
    }

    Part {
        svg: rect,
        after_label,
        bbox,
        intersect: Intersect::Rect,
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{draw_box, node};
    use super::*;
    use crate::unified::label::LabelLayout;
    use ferrigura_model::value::Value;

    #[test]
    fn a_collapsed_group_is_at_least_eighty_wide_and_sits_its_title_high() {
        let mut node = node("collapsedGroup", "neo");
        node.padding = 8.0;
        let label = LabelLayout::of_box(BBox::new(-30.0, 0.0, 60.0, 17.0));
        let shape = draw_box(&node, &label);
        // The title is 60 wide, so the box is 80 (the minimum) wide; the height is the title's
        // 17 plus 8 + 20 + 2 * 8.
        assert!((shape.bbox.width - 80.0).abs() < 1e-9, "{:?}", shape.bbox);
        assert!(
            (shape.bbox.height - (17.0 + 8.0 + 20.0 + 16.0)).abs() < 1e-9,
            "{:?}",
            shape.bbox
        );
        assert_eq!(shape.label_offset, Some(Point::new(0.0, -8.5 - 14.0)));
        assert!(
            shape.after_label.contains("collapsed-separator"),
            "{}",
            shape.after_label
        );
    }

    #[test]
    fn the_flow_container_type_has_its_own_corner_radius() {
        let mut node = node("collapsedGroup", "neo");
        node.padding = 8.0;
        node.metadata
            .insert("containerType".to_owned(), Value::String("flow".to_owned()));
        let shape = draw_box(&node, &LabelLayout::of_box(BBox::new(0.0, 0.0, 0.0, 0.0)));
        assert!(shape.svg.contains("rx=\"10\""), "{}", shape.svg);
        assert!(shape.svg.contains("flow-collapsed"), "{}", shape.svg);
    }
}
