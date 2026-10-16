//! The polygon shapes: `question`, `hexagon`, `lean_right`, `lean_left`, `trapezoid`,
//! `inv_trapezoid`, `subroutine` and `rect_left_inv_arrow`. Ports of the handlers in `shapes/`
//! with their neo and classic padding. See `docs-port/flowchart/render-notes/shapes-core.md` §2.1
//! and §2.4.

use super::rough;
use super::util::{self, Part, attribute, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::flowchart::geom::{BBox, Point};
use crate::flowchart::label::LabelLayout;
use crate::svg::number;

/// `question.ts`: a square turned on its corner, with the label in it. The outline is drawn half
/// a unit to the right of the box it is measured with (`question.ts:55-56`).
pub(super) fn question(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let label = label_helper(input, false);
    let width = label.bbox.width + node.padding;
    let height = label.bbox.height + node.padding;
    let size = width + height;
    let points = [
        Point::new(size / 2.0, 0.0),
        Point::new(size, -size / 2.0),
        Point::new(size / 2.0, -size),
        Point::new(0.0, -size / 2.0),
    ];
    let translate_x = -size / 2.0 + 0.5;
    let translate_y = size / 2.0;
    let styles = styles2string(node);
    let style = attribute_if(&styles.node);
    let svg = format!(
        "<polygon points=\"{}\" class=\"label-container\" transform=\"translate({}, {})\"{style}/>",
        util::points_attribute(&points),
        number(translate_x),
        number(translate_y),
    );
    let bbox = BBox::from_points(points)
        .unwrap_or_default()
        .translate(translate_x, translate_y);
    Part::with_centred_label(
        svg,
        bbox,
        Intersect::Polygon {
            points: points.to_vec(),
        },
        label,
    )
}

/// `hexagon.ts`: a hexagon whose sides are `m` long, with the label in it. The neo look pads the
/// height by 70 and the width by 32; the classic look pads both by the padding.
pub(super) fn hexagon(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let (factor, pad_height, pad_width) = if node.is_neo() {
        (3.5, 70.0, 32.0)
    } else {
        (4.0, node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let height = label.bbox.height + pad_height;
    let m = height / factor;
    let width = label.bbox.width + 2.0 * m + pad_width;
    let points = [
        Point::new(m, 0.0),
        Point::new(width - m, 0.0),
        Point::new(width, -height / 2.0),
        Point::new(width - m, -height),
        Point::new(m, -height),
        Point::new(0.0, -height / 2.0),
    ];
    polygon_part(input, label, &points, width, height)
}

/// `leanRight.ts`: a parallelogram leaning right, with the label in it.
pub(super) fn lean_right(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let pad_x = if node.is_neo() {
        2.0 * node.padding
    } else {
        node.padding
    };
    let label = label_helper(input, false);
    let height = label.bbox.height + node.padding;
    let width = label.bbox.width + pad_x;
    let points = [
        Point::new(-3.0 * height / 6.0, 0.0),
        Point::new(width, 0.0),
        Point::new(width + 3.0 * height / 6.0, -height),
        Point::new(0.0, -height),
    ];
    polygon_part(input, label, &points, width, height)
}

/// `leanLeft.ts`: a parallelogram leaning left, with the label in it.
pub(super) fn lean_left(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let pad_x = if node.is_neo() {
        2.0 * node.padding
    } else {
        node.padding
    };
    let label = label_helper(input, false);
    let height = label.bbox.height + node.padding;
    let width = label.bbox.width + pad_x;
    let points = [
        Point::new(0.0, 0.0),
        Point::new(width + 3.0 * height / 6.0, 0.0),
        Point::new(width, -height),
        Point::new(-3.0 * height / 6.0, -height),
    ];
    polygon_part(input, label, &points, width, height)
}

/// `trapezoid.ts`: a trapezoid wider at the bottom, with the label in it.
pub(super) fn trapezoid(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let pad_x = if node.is_neo() {
        2.0 * node.padding
    } else {
        node.padding
    };
    let label = label_helper(input, false);
    let height = label.bbox.height + node.padding;
    let width = label.bbox.width + pad_x;
    let points = [
        Point::new(-3.0 * height / 6.0, 0.0),
        Point::new(width + 3.0 * height / 6.0, 0.0),
        Point::new(width, -height),
        Point::new(0.0, -height),
    ];
    polygon_part(input, label, &points, width, height)
}

/// `invertedTrapezoid.ts`: a trapezoid wider at the top, with the label in it.
pub(super) fn inv_trapezoid(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let pad_x = if node.is_neo() {
        2.0 * node.padding
    } else {
        node.padding
    };
    let label = label_helper(input, false);
    let height = label.bbox.height + 2.0 * node.padding;
    let width = label.bbox.width + 2.0 * pad_x;
    let points = [
        Point::new(0.0, 0.0),
        Point::new(width, 0.0),
        Point::new(width + 3.0 * height / 6.0, -height),
        Point::new(-3.0 * height / 6.0, -height),
    ];
    polygon_part(input, label, &points, width, height)
}

/// `subroutine.ts`: a rectangle with a frame of 8 on each side, drawn as one polygon of ten
/// points, with the label in it.
pub(super) fn subroutine(input: &ShapeInput<'_>) -> Part {
    const FRAME_WIDTH: f64 = 8.0;
    let node = input.node;
    let (pad_x, pad_y) = if node.is_neo() {
        (28.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let total_width = (label.bbox.width + 2.0 * FRAME_WIDTH + pad_x).max(node.width.unwrap_or(0.0));
    let total_height = (label.bbox.height + pad_y).max(node.height.unwrap_or(0.0));
    let width = total_width - 2.0 * FRAME_WIDTH;
    let height = total_height;
    let points = [
        Point::new(0.0, 0.0),
        Point::new(width, 0.0),
        Point::new(width, -height),
        Point::new(0.0, -height),
        Point::new(0.0, 0.0),
        Point::new(-FRAME_WIDTH, 0.0),
        Point::new(width + FRAME_WIDTH, 0.0),
        Point::new(width + FRAME_WIDTH, -height),
        Point::new(-FRAME_WIDTH, -height),
        Point::new(-FRAME_WIDTH, 0.0),
    ];
    polygon_part(input, label, &points, width, height)
}

/// `rectLeftInvArrow.ts`: a rectangle with a notch on the left, drawn as a path that rough.js
/// outlines at roughness 0. The notch is a quarter of the height deep, and the outline is moved
/// half of it to the right.
pub(super) fn rect_left_inv_arrow(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (21.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let label_width = label.bbox.width + if node.is_neo() { 2.0 * pad_x } else { pad_x };
    let height = (label.bbox.height + if node.is_neo() { 2.0 * pad_y } else { pad_y })
        .max(node.height.unwrap_or(0.0));
    let notch_width = height / 4.0;
    let width = label_width.max(node.width.unwrap_or(0.0) - notch_width);

    let x = -width / 2.0;
    let y = -height / 2.0;
    let notch = y / 2.0;
    let points = [
        Point::new(x + notch, y),
        Point::new(x, 0.0),
        Point::new(x + notch, -y),
        Point::new(-x, -y),
        Point::new(-x, y),
    ];
    let outline = rough::polygon(&points);
    let path_style = util::path_style(node, &styles);
    let shift = -notch / 2.0;
    let svg = format!(
        "<g class=\"basic label-container outer-path\" transform=\"translate({},0)\">{}</g>",
        number(shift),
        rough::paths(
            &outline,
            Some(path_style.as_str()),
            &util::stroke_width(&styles),
            true
        ),
    );
    let label_offset = Point::new(
        -notch / 2.0 - label.bbox.width / 2.0 - label.bbox.x,
        -(label.bbox.height / 2.0) - label.bbox.y,
    );
    let bbox = outline.bbox.translate(shift, 0.0);
    Part {
        svg,
        after_label: String::new(),
        bbox,
        intersect: Intersect::Polygon {
            points: points.to_vec(),
        },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// `lightningBolt.ts` (`bolt`): a zigzag bolt of 35 by 35 with a gap of 7 between its strokes. It
/// has no label. Its outline is moved up and to the left of its centre.
pub(super) fn lightning_bolt(input: &ShapeInput<'_>) -> Part {
    const GAP: f64 = 7.0;
    let node = input.node;
    let styles = styles2string(node);
    let width = 35.0_f64.max(node.width.unwrap_or(0.0));
    let height = 35.0_f64.max(node.height.unwrap_or(0.0));
    let points = [
        Point::new(width, 0.0),
        Point::new(0.0, height + GAP / 2.0),
        Point::new(width - 2.0 * GAP, height + GAP / 2.0),
        Point::new(0.0, 2.0 * height),
        Point::new(width, height - GAP / 2.0),
        Point::new(2.0 * GAP, height - GAP / 2.0),
    ];
    let outline = rough::polygon(&points);
    let style = util::path_style(node, &styles);
    let translate_x = -width / 2.0;
    let translate_y = -height;
    let svg = format!(
        "<g class=\"outer-path\" transform=\"translate({}, {})\">{}</g>",
        number(translate_x),
        number(translate_y),
        rough::paths(
            &outline,
            Some(style.as_str()),
            &util::stroke_width(&styles),
            true
        ),
    );
    Part::without_label(
        svg,
        outline.bbox.translate(translate_x, translate_y),
        Intersect::Polygon {
            points: points.to_vec(),
        },
    )
}

/// `trapezoidalPentagon.ts` (`notch-pent`): a rectangle whose top corners are cut in. The neo look
/// pads the label by 16 and 12; the classic look by the padding. The label sits at the default
/// centred offset.
pub(super) fn trapezoidal_pentagon(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (16.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let w = label.bbox.width + pad_x * 2.0;
    let h = label.bbox.height + pad_y * 2.0;
    let points = vec![
        Point::new((-w / 2.0) * 0.8, -h / 2.0),
        Point::new((w / 2.0) * 0.8, -h / 2.0),
        Point::new(w / 2.0, (-h / 2.0) * 0.6),
        Point::new(w / 2.0, h / 2.0),
        Point::new(-w / 2.0, h / 2.0),
        Point::new(-w / 2.0, (-h / 2.0) * 0.6),
    ];
    let outline = rough::polygon(&points);
    let style = util::path_style(node, &styles);
    let svg = format!(
        "<g class=\"basic label-container outer-path\">{}</g>",
        rough::paths(
            &outline,
            Some(style.as_str()),
            &util::stroke_width(&styles),
            true
        ),
    );
    Part::with_centred_label(svg, outline.bbox, Intersect::Polygon { points }, label)
}

/// `triangle.ts` (`tri`): an isosceles triangle on a base as wide as its height, with the label
/// in the lower part. The neo look pads the width by twice the padding, the classic look by
/// the padding. Its outline is moved to sit centred on the node.
pub(super) fn triangle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let padding_x = if node.is_neo() {
        2.0 * node.padding
    } else {
        node.padding
    };
    let label = label_helper(input, false);
    let width = label.bbox.width + padding_x;
    let height = width + label.bbox.height;
    let points = [
        Point::new(0.0, 0.0),
        Point::new(height, 0.0),
        Point::new(height / 2.0, -height),
    ];
    let outline = rough::polygon(&points);
    let style = util::path_style(node, &styles);
    let translate_x = -height / 2.0;
    let translate_y = height / 2.0;
    let svg = format!(
        "<g class=\"outer-path\" transform=\"translate({}, {})\">{}</g>",
        number(translate_x),
        number(translate_y),
        rough::paths(
            &outline,
            Some(style.as_str()),
            &util::stroke_width(&styles),
            true
        ),
    );
    // The label sits under the apex: its bottom is `padding` above the base's line, measured in
    // the SVG text box (`htmlLabels` off: the padding is not halved).
    let label_offset = Point::new(
        -(label.bbox.width / 2.0) - label.bbox.x,
        height / 2.0 - (label.bbox.height + node.padding - label.bbox.y),
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline.bbox.translate(translate_x, translate_y),
        intersect: Intersect::Polygon {
            points: points.to_vec(),
        },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// The part of a polygon shape with its label at the centred offset: the polygon as
/// `insertPolygonShape` writes it, with the node's style when there is one.
fn polygon_part(
    input: &ShapeInput<'_>,
    label: LabelLayout,
    points: &[Point],
    width: f64,
    height: f64,
) -> Part {
    let styles = styles2string(input.node);
    let style = (!styles.node.is_empty()).then_some(styles.node.as_str());
    let (svg, bbox) = util::polygon_shape(points, width, height, style);
    Part::with_centred_label(
        svg,
        bbox,
        Intersect::Polygon {
            points: points.to_vec(),
        },
        label,
    )
}

/// The ` style="…"` of a declaration list, when there is one.
fn attribute_if(style: &str) -> String {
    if style.is_empty() {
        String::new()
    } else {
        attribute("style", style)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{draw_box, node};
    use super::*;
    use crate::flowchart::label::LabelLayout;

    // The bounding boxes of the reference's polygon shapes for a 120 by 17 label, as their
    // handlers compute them (shapes-core.md §2.1).

    #[test]
    fn a_neo_hexagon_is_the_label_plus_the_neo_padding_on_each_side() {
        let label = LabelLayout::of_box(BBox::new(-60.0, 0.0, 120.0, 17.0));
        let shape = draw_box(&node("hexagon", "neo"), &label);
        // height 17 + 70 = 87; m = 87 / 3.5; width 120 + 2m + 32. The box is measured from the
        // path's coordinates as the browser reads them, so it is exact to float32 precision.
        let m = 87.0 / 3.5;
        let width = 120.0 + 2.0 * m + 32.0;
        assert!((shape.bbox.width - width).abs() < 1e-4, "{:?}", shape.bbox);
        assert!((shape.bbox.height - 87.0).abs() < 1e-4, "{:?}", shape.bbox);
    }

    #[test]
    fn a_question_is_a_square_of_the_label_plus_the_padding() {
        let label = LabelLayout::of_box(BBox::new(-60.0, 0.0, 120.0, 17.0));
        let shape = draw_box(&node("question", "classic"), &label);
        // size = (120 + 15) + (17 + 15) = 167; the outline is moved by -size/2 + 0.5.
        assert!((shape.bbox.width - 167.0).abs() < 1e-9, "{:?}", shape.bbox);
        assert!(
            (shape.bbox.x - (-167.0 / 2.0 + 0.5)).abs() < 1e-9,
            "{:?}",
            shape.bbox
        );
    }

    #[test]
    fn a_subroutine_frame_is_eight_wide_on_each_side() {
        let label = LabelLayout::of_box(BBox::new(-60.0, 0.0, 120.0, 17.0));
        let shape = draw_box(&node("subroutine", "neo"), &label);
        // width 120 + 16 + 28 = 164; height 17 + 12 = 29.
        assert!((shape.bbox.width - 164.0).abs() < 1e-9, "{:?}", shape.bbox);
        assert!((shape.bbox.height - 29.0).abs() < 1e-9, "{:?}", shape.bbox);
    }
}
