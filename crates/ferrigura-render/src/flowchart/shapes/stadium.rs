//! The stadium: a rectangle with semicircular ends. A port of `shapes/stadium.ts`. Its outline is
//! sampled into 50 points per end and drawn by rough.js at roughness 0, so the outline is the
//! polygon of those points (`docs-port/flowchart/render-notes/shapes-core.md` §2.1 and §2.3).

use std::f64::consts::PI;

use super::rough;
use super::util::{self, Part, label_helper, styles2string};
use super::{Intersect, ShapeInput};
use crate::flowchart::geom::Point;

/// The points that sample each semicircular end.
const ARC_POINTS: usize = 50;

/// The size of the stadium around a label: its height is the label's plus the vertical padding,
/// and its width is the widest of three bounds (`stadiumDimensions`).
fn dimensions(label_width: f64, label_height: f64, pad_x: f64, pad_y: f64) -> (f64, f64) {
    let h = label_height + pad_y;
    // The sampled arcs sit just inside the circles, so the inscribed diameter is the one that
    // clears the label.
    let inscribed = h * (PI / (2.0 * (ARC_POINTS as f64 - 1.0))).cos();
    let cap = (inscribed * inscribed - label_height * label_height)
        .max(0.0)
        .sqrt();
    let w = (label_width + h / 4.0 + pad_x)
        .max(1.5 * h + (h - inscribed))
        .max(label_width + pad_x + h - cap);
    (w, h)
}

/// `stadium.ts`: the stadium around the label. The neo look pads the label by 40 and 24, the
/// classic look by the padding.
pub(super) fn stadium(input: &ShapeInput<'_>) -> Part {
    let node = input.node;
    let styles = styles2string(node);
    let (pad_x, pad_y) = if node.is_neo() {
        (40.0, 24.0)
    } else {
        (node.padding, node.padding)
    };
    let label = label_helper(input, false);
    let (w, h) = dimensions(label.bbox.width, label.bbox.height, pad_x, pad_y);
    let radius = h / 2.0;

    let mut points = vec![
        Point::new(-w / 2.0 + radius, -h / 2.0),
        Point::new(w / 2.0 - radius, -h / 2.0),
    ];
    points.extend(util::generate_circle_points(
        -w / 2.0 + radius,
        0.0,
        radius,
        ARC_POINTS,
        90.0,
        270.0,
    ));
    points.push(Point::new(w / 2.0 - radius, h / 2.0));
    points.extend(util::generate_circle_points(
        w / 2.0 - radius,
        0.0,
        radius,
        ARC_POINTS,
        270.0,
        450.0,
    ));

    let outline = rough::polygon(&points);
    let svg = format!(
        "<g class=\"basic label-container outer-path\">{}</g>",
        rough::paths(
            &outline,
            Some(util::path_style(node, &styles).as_str()),
            &util::stroke_width(&styles),
            true,
        ),
    );
    Part::with_centred_label(svg, outline.bbox, Intersect::Polygon { points }, label)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{draw_box, node};
    use super::*;
    use crate::flowchart::geom::BBox;
    use crate::flowchart::label::LabelLayout;
    use crate::flowchart::shapes::ShapeOutput;

    /// Whether a point is inside a convex polygon: on one side of every edge.
    fn contains(points: &[Point], point: Point) -> bool {
        let crosses: Vec<f64> = points
            .iter()
            .enumerate()
            .map(|(index, a)| {
                let b = points[(index + 1) % points.len()];
                (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x)
            })
            .collect();
        crosses.iter().all(|cross| *cross >= -1e-7) || crosses.iter().all(|cross| *cross <= 1e-7)
    }

    fn outline(shape: &ShapeOutput) -> Vec<Point> {
        match &shape.intersect {
            Intersect::Polygon { points } => points.clone(),
            other => panic!("a stadium intersects a polygon, not {other:?}"),
        }
    }

    /// The stadium of a label of the given box, in a look, with the given padding.
    fn render(width: f64, height: f64, look: &str, padding: f64) -> ShapeOutput {
        let mut node = node("stadium", look);
        node.padding = padding;
        node.label = "a label".to_owned();
        draw_box(
            &node,
            &LabelLayout::of_box(BBox::new(-width / 2.0, 0.0, width, height)),
        )
    }

    // stadium.spec.ts, "stadium geometry".

    #[test]
    fn keeps_tall_neo_and_classic_nodes_elongated() {
        for look in ["neo", "classic"] {
            for height in [170.0, 231.0, 651.0] {
                let shape = render(120.0, height, look, 15.0);
                assert!(
                    shape.bbox.width / shape.bbox.height >= 1.5 - 1e-7,
                    "{look} {height}"
                );
                let points = outline(&shape);
                let half_padding = if look == "neo" { 20.0 } else { 7.5 };
                for x in [-60.0 - half_padding, 60.0 + half_padding] {
                    for y in [-height / 2.0, height / 2.0] {
                        assert!(
                            contains(&points, Point::new(x, y)),
                            "{look} {height} ({x}, {y})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn preserves_the_existing_short_neo_stadium_dimensions() {
        let shape = render(120.0, 21.0, "neo", 15.0);
        // The width is measured from the path's float32 coordinates, so it is exact to that
        // precision (about 3e-6 from the double value here).
        assert!(
            (shape.bbox.width - 171.22687973).abs() < 1e-5,
            "{:?}",
            shape.bbox
        );
        assert!((shape.bbox.height - 45.0).abs() < 1e-9, "{:?}", shape.bbox);
    }

    #[test]
    #[ignore = "port: the reference counts its label measurements through a DOM mock; a Rust box is measured once by the caller"]
    fn measures_its_label_once_at_the_configured_wrapping_width() {}

    #[test]
    #[ignore = "port: explicit node widths are not read by the stadium (it never reflows), and no fixture sets one; the reference counts label measurements through a DOM mock"]
    fn does_not_reflow_an_explicitly_sized_node() {}

    #[test]
    fn contains_wide_label_corners_with_padding() {
        for padding in [0.0, 0.01, 15.0] {
            let shape = render(400.0, 200.0, "classic", padding);
            let points = outline(&shape);
            for x in [-200.0 - padding / 2.0, 200.0 + padding / 2.0] {
                for y in [-100.0, 100.0] {
                    assert!(
                        contains(&points, Point::new(x, y)),
                        "padding {padding} ({x}, {y})"
                    );
                }
            }
        }
    }

    #[test]
    fn keeps_edge_intersections_on_the_painted_outline_after_translating_the_node() {
        use super::super::intersect::polygon as polygon_intersect;
        let shape = render(120.0, 651.0, "neo", 15.0);
        let points = outline(&shape);
        // The node moved to (300, 500): the intersection is computed in its own box.
        let node_box = crate::flowchart::geom::CenterBox::new(
            300.0,
            500.0,
            shape.bbox.width,
            shape.bbox.height,
        );
        for (x, y) in [
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (1.0, 1.0),
            (-1.0, 2.0),
        ] {
            let crossing = polygon_intersect(
                node_box,
                &points,
                Point::new(300.0 + x * 2000.0, 500.0 + y * 2000.0),
            );
            let local = Point::new(crossing.x - 300.0, crossing.y - 500.0);
            assert!(contains(
                &points,
                Point::new(local.x * 0.999, local.y * 0.999)
            ));
            assert!(!contains(
                &points,
                Point::new(local.x * 1.001, local.y * 1.001)
            ));
        }
    }
}
