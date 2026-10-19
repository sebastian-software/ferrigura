//! The curly braces: `curlyBraceLeft`, `curlyBraceRight` and `curlyBraces`. Ports of
//! `shapes/curlyBraceLeft.ts`, `curlyBraceRight.ts` and `curlyBraces.ts`. Each brace is a rough.js
//! path at roughness 0 with `fill: none`, so only its stroke is drawn, through sampled points; a
//! second, invisible outline with the same geometry (`stroke-opacity` 0) is drawn with it, and the
//! intersect follows that second outline. See `docs-port/flowchart/render-notes/shapes-core.md`
//! §2.3.

use std::f64::consts::PI;

use ferrigura_jsmath::{cos, sin};

use super::rough::{self, Outline};
use super::util::{
    self, Part, generate_circle_points, label_helper, path_style, stroke_width, styles2string,
};
use super::{Intersect, ShapeInput};
use crate::svg::number;
use crate::unified::geom::Point;
use crate::unified::label::LabelLayout;

/// The points of an arc of a circle, from `start` to `end` degrees, as the brace files compute
/// them: the arc is not negated, as `curlyBraceRight.ts` writes it.
fn arc_points(
    centre_x: f64,
    centre_y: f64,
    radius: f64,
    count: usize,
    start: f64,
    end: f64,
) -> Vec<Point> {
    let start = start * PI / 180.0;
    let end = end * PI / 180.0;
    let step = (end - start) / (count - 1) as f64;
    (0..count)
        .map(|index| {
            let angle = start + index as f64 * step;
            Point::new(
                centre_x + radius * cos(angle),
                centre_y + radius * sin(angle),
            )
        })
        .collect()
}

/// The outline of an open polyline, as the brace's `rc.path`: the points as rough.js draws them at
/// roughness 0 with no fill, and no closing side (the reference removes the `Z` of the path).
fn open_outline(points: &[Point]) -> Outline {
    let mut segments = Vec::with_capacity(points.len());
    for (index, point) in points.iter().enumerate() {
        segments.push(if index == 0 {
            rough::Segment::Move(*point)
        } else {
            rough::Segment::Line(*point)
        });
    }
    rough::path(&segments, rough::Paint::STROKE)
}

/// The outline of a closed polyline, as the brace's rectangle: `createPathFromPoints`, with its `Z`.
fn closed_outline(points: &[Point]) -> Outline {
    rough::path(&rough::points_path(points), rough::Paint::STROKE)
}

/// The SVG of a brace's strokes, in the order the reference inserts them: the visible stroke, then
/// the invisible rectangle with the same geometry, which the reference hides with `stroke-opacity`.
fn strokes(curve: &Outline, rect: &Outline, style: Option<&str>, width: &str) -> String {
    format!(
        "<g>{}</g><g stroke-opacity=\"0\">{}</g>",
        rough::paths(curve, style, width),
        rough::paths(rect, style, width),
    )
}

/// `curlyBraceLeft.ts`: a brace opening to the right, around the label. The neo look pads the label
/// by 18 across and 12 high; the classic look by the padding. The brace is drawn `radius` to the
/// right of the node's centre.
pub(super) fn curly_brace_left(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let (padding_x, padding_y) = if node.is_neo() {
        (18.0, 12.0)
    } else {
        (node.padding, node.padding)
    };
    let w = label.bbox.width + padding_x;
    let h = label.bbox.height + padding_y;
    let radius = 5.0_f64.max(h * 0.1);

    let mut curve = generate_circle_points(w / 2.0, -h / 2.0, radius, 30, -90.0, 0.0);
    curve.push(Point::new(-w / 2.0 - radius, radius));
    curve.extend(generate_circle_points(
        w / 2.0 + radius * 2.0,
        -radius,
        radius,
        20,
        -180.0,
        -270.0,
    ));
    curve.extend(generate_circle_points(
        w / 2.0 + radius * 2.0,
        radius,
        radius,
        20,
        -90.0,
        -180.0,
    ));
    curve.push(Point::new(-w / 2.0 - radius, -h / 2.0));
    curve.extend(generate_circle_points(
        w / 2.0,
        h / 2.0,
        radius,
        20,
        0.0,
        90.0,
    ));

    let mut rect = vec![
        Point::new(w / 2.0, -h / 2.0 - radius),
        Point::new(-w / 2.0, -h / 2.0 - radius),
    ];
    rect.extend(generate_circle_points(
        w / 2.0,
        -h / 2.0,
        radius,
        20,
        -90.0,
        0.0,
    ));
    rect.push(Point::new(-w / 2.0 - radius, -radius));
    rect.extend(generate_circle_points(
        w / 2.0 + w * 0.1,
        -radius,
        radius,
        20,
        -180.0,
        -270.0,
    ));
    rect.extend(generate_circle_points(
        w / 2.0 + w * 0.1,
        radius,
        radius,
        20,
        -90.0,
        -180.0,
    ));
    rect.push(Point::new(-w / 2.0 - radius, h / 2.0));
    rect.extend(generate_circle_points(
        w / 2.0,
        h / 2.0,
        radius,
        20,
        0.0,
        90.0,
    ));
    rect.push(Point::new(-w / 2.0, h / 2.0 + radius));
    rect.push(Point::new(w / 2.0, h / 2.0 + radius));

    finish(
        input,
        label,
        &styles,
        &curve,
        &rect,
        radius,
        Point::new(-w / 2.0 + radius, -h / 2.0 + node.padding / 2.0),
    )
}

/// `curlyBraceRight.ts`: a brace opening to the left, around the label. The neo look pads the label
/// by 36 across and 24 high; the classic look by the padding. The brace is drawn `radius` to the
/// left of the node's centre.
pub(super) fn curly_brace_right(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let (padding_x, padding_y) = if node.is_neo() {
        (18.0 * 2.0, 12.0 * 2.0)
    } else {
        (node.padding, node.padding)
    };
    let w = label.bbox.width + padding_x;
    let h = label.bbox.height + padding_y;
    let radius = 5.0_f64.max(h * 0.1);

    let mut curve = arc_points(w / 2.0, -h / 2.0, radius, 20, -90.0, 0.0);
    curve.push(Point::new(w / 2.0 + radius, -radius));
    curve.extend(arc_points(
        w / 2.0 + radius * 2.0,
        -radius,
        radius,
        20,
        -180.0,
        -270.0,
    ));
    curve.extend(arc_points(
        w / 2.0 + radius * 2.0,
        radius,
        radius,
        20,
        -90.0,
        -180.0,
    ));
    curve.push(Point::new(w / 2.0 + radius, h / 2.0));
    curve.extend(arc_points(w / 2.0, h / 2.0, radius, 20, 0.0, 90.0));

    let mut rect = vec![
        Point::new(-w / 2.0, -h / 2.0 - radius),
        Point::new(w / 2.0, -h / 2.0 - radius),
    ];
    rect.extend(arc_points(w / 2.0, -h / 2.0, radius, 20, -90.0, 0.0));
    rect.push(Point::new(w / 2.0 + radius, -radius));
    rect.extend(arc_points(
        w / 2.0 + radius * 2.0,
        -radius,
        radius,
        20,
        -180.0,
        -270.0,
    ));
    rect.extend(arc_points(
        w / 2.0 + radius * 2.0,
        radius,
        radius,
        20,
        -90.0,
        -180.0,
    ));
    rect.push(Point::new(w / 2.0 + radius, h / 2.0));
    rect.extend(arc_points(w / 2.0, h / 2.0, radius, 20, 0.0, 90.0));
    rect.push(Point::new(w / 2.0, h / 2.0 + radius));
    rect.push(Point::new(-w / 2.0, h / 2.0 + radius));

    finish(
        input,
        label,
        &styles,
        &curve,
        &rect,
        -radius,
        Point::new(-w / 2.0 + node.padding / 2.0, -h / 2.0 + node.padding / 2.0),
    )
}

/// `curlyBraces.ts`: a brace on each side of the label, the two braces of a comment. The neo look
/// pads the label by 36 across and 24 high; the classic look by the padding. The outline is drawn
/// `radius - radius / 4` to the right of the node's centre.
pub(super) fn curly_braces(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let (padding_x, padding_y) = if node.is_neo() {
        (18.0 * 2.0, 12.0 * 2.0)
    } else {
        (node.padding, node.padding)
    };
    let w = label.bbox.width + padding_x;
    let h = label.bbox.height + padding_y;
    let radius = 5.0_f64.max(h * 0.1);

    let mut left = generate_circle_points(w / 2.0, -h / 2.0, radius, 30, -90.0, 0.0);
    left.push(Point::new(-w / 2.0 - radius, radius));
    left.extend(generate_circle_points(
        w / 2.0 + radius * 2.0,
        -radius,
        radius,
        20,
        -180.0,
        -270.0,
    ));
    left.extend(generate_circle_points(
        w / 2.0 + radius * 2.0,
        radius,
        radius,
        20,
        -90.0,
        -180.0,
    ));
    left.push(Point::new(-w / 2.0 - radius, -h / 2.0));
    left.extend(generate_circle_points(
        w / 2.0,
        h / 2.0,
        radius,
        20,
        0.0,
        90.0,
    ));

    let mut right = generate_circle_points(
        -w / 2.0 + radius + radius / 2.0,
        -h / 2.0,
        radius,
        20,
        -90.0,
        -180.0,
    );
    right.push(Point::new(w / 2.0 - radius / 2.0, radius));
    right.extend(generate_circle_points(
        -w / 2.0 - radius / 2.0,
        -radius,
        radius,
        20,
        0.0,
        90.0,
    ));
    right.extend(generate_circle_points(
        -w / 2.0 - radius / 2.0,
        radius,
        radius,
        20,
        -90.0,
        0.0,
    ));
    right.push(Point::new(w / 2.0 - radius / 2.0, -radius));
    right.extend(generate_circle_points(
        -w / 2.0 + radius + radius / 2.0,
        h / 2.0,
        radius,
        30,
        -180.0,
        -270.0,
    ));

    let mut rect = vec![
        Point::new(w / 2.0, -h / 2.0 - radius),
        Point::new(-w / 2.0, -h / 2.0 - radius),
    ];
    rect.extend(generate_circle_points(
        w / 2.0,
        -h / 2.0,
        radius,
        20,
        -90.0,
        0.0,
    ));
    rect.push(Point::new(-w / 2.0 - radius, -radius));
    rect.extend(generate_circle_points(
        w / 2.0 + radius * 2.0,
        -radius,
        radius,
        20,
        -180.0,
        -270.0,
    ));
    rect.extend(generate_circle_points(
        w / 2.0 + radius * 2.0,
        radius,
        radius,
        20,
        -90.0,
        -180.0,
    ));
    rect.push(Point::new(-w / 2.0 - radius, h / 2.0));
    rect.extend(generate_circle_points(
        w / 2.0,
        h / 2.0,
        radius,
        20,
        0.0,
        90.0,
    ));
    rect.push(Point::new(-w / 2.0, h / 2.0 + radius));
    rect.push(Point::new(
        w / 2.0 - radius - radius / 2.0,
        h / 2.0 + radius,
    ));
    rect.extend(generate_circle_points(
        -w / 2.0 + radius + radius / 2.0,
        -h / 2.0,
        radius,
        20,
        -90.0,
        -180.0,
    ));
    rect.push(Point::new(w / 2.0 - radius / 2.0, radius));
    rect.extend(generate_circle_points(
        -w / 2.0 - radius / 2.0,
        -radius,
        radius,
        20,
        0.0,
        90.0,
    ));
    rect.extend(generate_circle_points(
        -w / 2.0 - radius / 2.0,
        radius,
        radius,
        20,
        -90.0,
        0.0,
    ));
    rect.push(Point::new(w / 2.0 - radius / 2.0, -radius));
    rect.extend(generate_circle_points(
        -w / 2.0 + radius + radius / 2.0,
        h / 2.0,
        radius,
        30,
        -180.0,
        -270.0,
    ));

    let style = path_style(node, &styles);
    let style = Some(style.as_str());
    let left_outline = open_outline(&left);
    let right_outline = open_outline(&right);
    let rect_outline = closed_outline(&rect);
    let width = stroke_width(&styles);
    let svg = format!(
        "<g class=\"text\" transform=\"translate({}, 0)\"><g>{}</g><g>{}</g><g stroke-opacity=\"0\">{}</g></g>",
        number(radius - radius / 4.0),
        rough::paths(&right_outline, style, &width),
        rough::paths(&left_outline, style, &width),
        rough::paths(&rect_outline, style, &width),
    );
    let shift = radius - radius / 4.0;
    let bbox = right_outline
        .bbox
        .union(left_outline.bbox)
        .union(rect_outline.bbox)
        .translate(shift, 0.0);
    let label_offset = Point::new(
        -w / 2.0 + node.padding / 2.0 - label.bbox.x,
        -h / 2.0 + node.padding / 2.0 - label.bbox.y,
    );
    Part {
        svg,
        after_label: String::new(),
        bbox,
        intersect: Intersect::Polygon { points: rect },
        label_offset: Some(label_offset),
        label: Some(label),
    }
}

/// The part of a brace with one curve and the rectangle that its intersect follows. `shift` moves
/// the drawn brace along the x axis. The label is placed at `label_base` less the label's own box
/// origin, as the reference's `label.attr('transform', ...)` writes it.
fn finish(
    input: &ShapeInput<'_>,
    label: LabelLayout,
    styles: &util::Styles,
    curve: &[Point],
    rect: &[Point],
    shift: f64,
    label_base: Point,
) -> Part {
    let node = input.node;
    let style = path_style(node, styles);
    let width = stroke_width(styles);
    let curve_outline = open_outline(curve);
    let rect_outline = closed_outline(rect);
    let svg = format!(
        "<g class=\"text\" transform=\"translate({}, 0)\">{}</g>",
        number(shift),
        strokes(&curve_outline, &rect_outline, Some(style.as_str()), &width),
    );
    let bbox = curve_outline
        .bbox
        .union(rect_outline.bbox)
        .translate(shift, 0.0);
    Part {
        svg,
        after_label: String::new(),
        bbox,
        intersect: Intersect::Polygon {
            points: rect.to_vec(),
        },
        label_offset: Some(Point::new(
            label_base.x - label.bbox.x,
            label_base.y - label.bbox.y,
        )),
        label: Some(label),
    }
}

#[cfg(test)]
mod tests {
    use ferrigura_jsmath::hypot;

    use super::*;

    #[test]
    fn the_brace_points_are_on_their_circles() {
        for point in arc_points(0.0, 0.0, 5.0, 20, -90.0, 0.0) {
            assert!((hypot(point.x, point.y) - 5.0).abs() < 1e-9, "{point:?}");
        }
    }

    #[test]
    fn a_brace_outline_is_the_stroke_of_an_open_path() {
        // One segment, drawn twice, and no fill: the path has no closing side.
        let outline = open_outline(&[
            Point::new(0.0, 0.0),
            Point::new(10.0, 5.0),
            Point::new(10.0, 0.0),
        ]);
        assert!(outline.fill.is_none());
        let stroke = outline.stroke.expect("a stroke");
        assert_eq!(
            stroke
                .iter()
                .filter(|op| matches!(op, rough::Op::Move(_)))
                .count(),
            4
        );
    }
}
