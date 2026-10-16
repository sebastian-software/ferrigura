//! The symbols: `choice` (a small diamond with no label), `hourglass` (two triangles meeting at a
//! point), `flippedTriangle` (a triangle pointing down, for a manual file) and `crossedCircle` (a
//! circle with a cross). Ports of `shapes/choice.ts`, `hourglass.ts`, `flippedTriangle.ts` and
//! `crossedCircle.ts`, for the neo and classic looks. The outlines are rough.js paths at roughness
//! 0 (`rough`). See `docs-port/flowchart/render-notes/shapes-core.md` §2.3 and §2.4.

use std::f64::consts::FRAC_PI_4;

use super::rough;
use super::util::{self, Part, label_helper, path_style, stroke_width, styles2string};
use super::{Intersect, ShapeInput};
use crate::svg::number;
use crate::unified::geom::{BBox, Point};
use crate::unified::label::LabelLayout;

/// `choice.ts`: a diamond of 28 across, with no label.
pub(super) fn choice(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let size = 28.0_f64.max(node.width.unwrap_or(0.0));
    let points = [
        Point::new(0.0, size / 2.0),
        Point::new(size / 2.0, 0.0),
        Point::new(0.0, -size / 2.0),
        Point::new(-size / 2.0, 0.0),
    ];
    // `rc.path` of the closed path of the diamond: the fill takes its closing side too.
    let outline = rough::closed_polygon(&points, rough::Paint::BOTH);
    let style = path_style(node, &styles);
    let svg = format!(
        "<g>{}</g>",
        rough::paths(&outline, Some(style.as_str()), &stroke_width(&styles)),
    );
    Part::without_label(
        svg,
        outline.bbox,
        Intersect::Polygon {
            points: points.to_vec(),
        },
    )
}

/// `hourglass.ts`: two triangles meeting at a point, 30 by 30 at least. The reference empties the
/// node's label before it lays one out, so the label group is empty; the label is centred on the
/// node's centre, as the default label offset is.
pub(super) fn hourglass(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    // The label is empty, so it has no minimum width: the layout the caller gave is the label.
    let label = input
        .label
        .cloned()
        .unwrap_or_else(|| LabelLayout::of_box(BBox::default()));
    let width = 30.0_f64.max(node.width.unwrap_or(0.0));
    let height = 30.0_f64.max(node.height.unwrap_or(0.0));
    let points = [
        Point::new(0.0, 0.0),
        Point::new(width, 0.0),
        Point::new(0.0, height),
        Point::new(width, height),
    ];
    let outline = rough::closed_polygon(&points, rough::Paint::BOTH);
    let style = path_style(node, &styles);
    let (translate_x, translate_y) = (-width / 2.0, -height / 2.0);
    let svg = format!(
        "<g class=\"basic label-container outer-path\" transform=\"translate({}, {})\">{}</g>",
        number(translate_x),
        number(translate_y),
        rough::paths(&outline, Some(style.as_str()), &stroke_width(&styles)),
    );
    Part {
        svg,
        after_label: String::new(),
        bbox: outline.bbox.translate(translate_x, translate_y),
        intersect: Intersect::Polygon {
            points: points.to_vec(),
        },
        label_offset: Some(util::centred_label_offset(&label)),
        label: Some(label),
    }
}

/// `flippedTriangle.ts`: an isosceles triangle pointing down, its base as wide as it is high. Its
/// width is the label's plus the label padding (twice the padding in the neo look), and its height
/// is that width plus the label's height. The label sits in its upper part.
pub(super) fn flipped_triangle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let label = label_helper(input, false);
    let label_padding_x = if node.is_neo() {
        node.padding * 2.0
    } else {
        node.padding
    };
    let w = node.width.unwrap_or(label.bbox.width) + label_padding_x;
    let h = node.height.unwrap_or(w + label.bbox.height);
    let triangle_width = h;
    let points = [
        Point::new(0.0, -h),
        Point::new(triangle_width, -h),
        Point::new(triangle_width / 2.0, 0.0),
    ];
    let outline = rough::closed_polygon(&points, rough::Paint::BOTH);
    let style = path_style(node, &styles);
    let (translate_x, translate_y) = (-h / 2.0, h / 2.0);
    let svg = format!(
        "<g class=\"outer-path\" transform=\"translate({}, {})\">{}</g>",
        number(translate_x),
        number(translate_y),
        rough::paths(&outline, Some(style.as_str()), &stroke_width(&styles)),
    );
    let label_offset = Point::new(
        -label.bbox.width / 2.0 - label.bbox.x,
        -h / 2.0 + node.padding / 2.0 + label.bbox.y,
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

/// `crossedCircle.ts`: a circle of at least 30 radius with a cross of two diagonals, the summary
/// symbol. It has no label. The cross is a second rough.js path inside the circle's group.
pub(super) fn crossed_circle(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let radius = 30.0_f64.max(node.width.unwrap_or(0.0));
    let circle = rough::ellipse(0.0, 0.0, radius * 2.0, radius * 2.0, rough::Paint::BOTH);

    // The diagonals of the circle, from quadrant II to IV and from I to III (`createLine`).
    let line_length = radius * 2.0;
    let diagonal = line_length / 2.0 * FRAC_PI_4.cos();
    let diagonal_y = line_length / 2.0 * FRAC_PI_4.sin();
    let q1 = Point::new(diagonal, diagonal_y);
    let q2 = Point::new(-diagonal, diagonal_y);
    let q3 = Point::new(-diagonal, -diagonal_y);
    let q4 = Point::new(diagonal, -diagonal_y);
    // `createLine`: two sub-paths, `M q2 L q4` and `M q1 L q3`, drawn as one rc.path. Each has two
    // points, so its fill has no polygon to draw.
    let segments = [
        rough::Segment::Move(q2),
        rough::Segment::Line(q4),
        rough::Segment::Move(q1),
        rough::Segment::Line(q3),
    ];
    let line = rough::path(&segments, rough::Paint::BOTH);

    let style = path_style(node, &styles);
    let width = stroke_width(&styles);
    let svg = format!(
        "<g class=\"outer-path\">{}<g>{}</g></g>",
        rough::paths(&circle, Some(style.as_str()), &width),
        rough::paths(&line, Some(style.as_str()), &width),
    );
    Part::without_label(
        svg,
        circle.bbox.union(line.bbox),
        Intersect::Circle { radius },
    )
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{draw_box, node};
    use super::*;
    use crate::unified::label::LabelLayout;

    #[test]
    fn a_choice_is_a_diamond_of_twenty_eight_without_a_label() {
        let shape = draw_box(
            &node("choice", "neo"),
            &LabelLayout::of_box(BBox::new(0.0, 0.0, 0.0, 0.0)),
        );
        assert_eq!(shape.label, None);
        assert!((shape.bbox.width - 28.0).abs() < 1e-9, "{:?}", shape.bbox);
        assert!((shape.bbox.height - 28.0).abs() < 1e-9, "{:?}", shape.bbox);
    }

    #[test]
    fn an_hourglass_is_at_least_thirty_square() {
        let shape = draw_box(
            &node("hourglass", "neo"),
            &LabelLayout::of_box(BBox::new(0.0, 0.0, 0.0, 0.0)),
        );
        assert!((shape.bbox.width - 30.0).abs() < 1e-9, "{:?}", shape.bbox);
        assert!((shape.bbox.height - 30.0).abs() < 1e-9, "{:?}", shape.bbox);
    }

    #[test]
    fn a_crossed_circle_is_a_circle_of_thirty_radius_with_a_cross() {
        let shape = draw_box(
            &node("cross-circ", "neo"),
            &LabelLayout::of_box(BBox::new(0.0, 0.0, 0.0, 0.0)),
        );
        assert_eq!(shape.intersect, Intersect::Circle { radius: 30.0 });
        assert!(
            shape.svg.starts_with("<g class=\"outer-path\">"),
            "{}",
            shape.svg
        );
        // The two diagonals have two sub-paths of two points each: their fill is empty, and their
        // stroke is two double lines (`path` in `generator.js`).
        assert!(shape.svg.contains("<g><path d=\"\""), "{}", shape.svg);
        assert!(shape.svg.contains("<path d=\"M-21.2"), "{}", shape.svg);
    }
}
