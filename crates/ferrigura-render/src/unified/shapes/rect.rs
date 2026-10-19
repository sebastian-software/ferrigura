//! The rectangular shapes: `squareRect` and `roundedRect` (both through `drawRect`), `text` and
//! `labelRect`. Ports of `shapes/squareRect.ts`, `roundedRect.ts`, `drawRect.ts`, `text.ts` and
//! `labelRect.ts`. See `docs-port/flowchart/render-notes/shapes-core.md` §2.1 and §2.4.

use super::util::{self, Part, attribute, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::svg::number;
use crate::unified::geom::{BBox, Point};

/// `squareRect.ts`: a rectangle around the label. The neo look pads the label by 16 and 12, the
/// classic look by twice the padding and the padding.
pub(super) fn square_rect(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (2.0 * node.padding, node.padding)
    };
    let label = label_helper(input, false);
    draw_rect(input, label, pad_x, pad_y, None)
}

/// `roundedRect.ts`: a rectangle with corners of the theme's radius (5 when the theme has none).
pub(super) fn rounded_rect(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let radius = util::theme_number(&input.config.theme.radius).unwrap_or(5.0);
    let label = label_helper(input, false);
    draw_rect(input, label, node.padding, node.padding, Some(radius))
}

/// `drawRect.ts`: the `rect.basic.label-container` around the label, padded on each side, at
/// least as wide and tall as the node's own size. The rectangle takes the node's declarations as
/// its style, and its radius when it has one.
fn draw_rect(
    input: &ShapeInput<'_>,
    label: crate::unified::label::LabelLayout,
    pad_x: f64,
    pad_y: f64,
    radius: Option<f64>,
) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let total_width = (label.bbox.width + 2.0 * pad_x).max(node.width.unwrap_or(0.0));
    let total_height = (label.bbox.height + 2.0 * pad_y).max(node.height.unwrap_or(0.0));
    let x = -total_width / 2.0;
    let y = -total_height / 2.0;
    let radius = radius.map_or_else(String::new, |radius| {
        format!(" rx=\"{0}\" ry=\"{0}\"", number(radius))
    });
    let svg = format!(
        "<rect class=\"basic label-container\"{}{radius} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
        attribute("style", &styles.node),
        number(x),
        number(y),
        number(total_width),
        number(total_height),
    );
    let bbox = BBox::new(x, y, total_width, total_height);
    Part::with_centred_label(svg, bbox, Intersect::Rect, label)
}

/// `text.ts`: a box without an outline (its style hides it), sized from the label and the node's
/// padding.
pub(super) fn text(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let total_width = (label.bbox.width + node.padding).max(node.width.unwrap_or(0.0));
    let total_height = (label.bbox.height + node.padding).max(node.height.unwrap_or(0.0));
    let x = -total_width / 2.0;
    let y = -total_height / 2.0;
    let svg = format!(
        "<rect class=\"text\"{} rx=\"0\" ry=\"0\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
        attribute("style", &styles.node),
        number(x),
        number(y),
        number(total_width),
        number(total_height),
    );
    let bbox = BBox::new(x, y, total_width, total_height);
    Part::with_centred_label(svg, bbox, Intersect::Rect, label)
}

/// `labelRect.ts`: a 0.1 by 0.1 box at the origin, with the label moved so that its centre is at
/// the origin. It only carries the label of a swimlane edge (`layout-algorithms/swimlanes`).
pub(super) fn label_rect(input: &ShapeInput<'_>) -> Part {
    let label = label_helper(input, false);
    let offset = Point::new(
        -(label.bbox.width / 2.0) - label.bbox.x,
        -(label.bbox.height / 2.0) - label.bbox.y,
    );
    let svg = "<rect width=\"0.1\" height=\"0.1\"/>".to_owned();
    Part {
        svg,
        after_label: String::new(),
        bbox: BBox::new(0.0, 0.0, 0.1, 0.1),
        intersect: Intersect::Rect,
        label_offset: Some(offset),
        label: Some(label),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{draw_box, node};
    use super::*;
    use crate::unified::label::LabelLayout;

    // squareRect.spec.ts: the colour slot tests belong to the style task, see the report.
    #[test]
    #[ignore = "port: colour slots (data-color-id) are written by the style task; no flowchart vertex has a colour index"]
    fn leaves_a_node_with_no_assigned_slot_unstamped_on_a_palette_theme() {}

    #[test]
    #[ignore = "port: colour slots (data-color-id) are written by the style task; no flowchart vertex has a colour index"]
    fn stamps_the_assigned_slot_on_a_palette_theme() {}

    #[test]
    #[ignore = "port: colour slots (data-color-id) are written by the style task; no flowchart vertex has a colour index"]
    fn stays_unstamped_on_a_theme_without_a_palette() {}

    #[test]
    fn a_square_rect_is_the_label_plus_the_neo_padding() {
        let label = LabelLayout::of_box(BBox::new(-60.0, 0.0, 120.0, 17.0));
        let shape = draw_box(&node("squareRect", "neo"), &label);
        // 120 + 2 * 16 by 17 + 2 * 12: the box of the reference's neo `squareRect`.
        assert_eq!(shape.bbox, BBox::new(-76.0, -20.5, 152.0, 41.0));
        assert_eq!(shape.label_offset, Some(Point::new(0.0, -8.5)));
    }

    #[test]
    fn a_square_rect_in_classic_is_the_label_plus_twice_the_padding() {
        let label = LabelLayout::of_box(BBox::new(-60.0, 0.0, 120.0, 17.0));
        let shape = draw_box(&node("squareRect", "classic"), &label);
        assert_eq!(shape.bbox, BBox::new(-90.0, -23.5, 180.0, 47.0));
    }

    #[test]
    fn a_rounded_rect_takes_the_theme_radius() {
        let label = LabelLayout::of_box(BBox::new(-60.0, 0.0, 120.0, 17.0));
        let shape = draw_box(&node("roundedRect", "classic"), &label);
        assert!(shape.svg.contains("rx=\"12\" ry=\"12\""), "{}", shape.svg);
        // roundedRect pads by the padding on each side: 120 + 2 * 15 by 17 + 2 * 15.
        assert_eq!(shape.bbox, BBox::new(-75.0, -23.5, 150.0, 47.0));
    }

    #[test]
    fn an_empty_label_is_not_widened_to_the_minimum_width() {
        let label = LabelLayout::of_box(BBox::new(0.0, 0.0, 0.0, 0.0));
        let shape = draw_box(&node("squareRect", "neo"), &label);
        assert_eq!(shape.bbox.width, 32.0);
    }
}
