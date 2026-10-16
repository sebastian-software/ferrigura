//! The intersect functions of `rendering-elements/intersect/`: where the line from a node's centre
//! towards a point meets the node's outline. A port of `intersect-rect.js`, `intersect-ellipse.js`,
//! `intersect-circle.js`, `intersect-polygon.js`, `intersect-line.js` and the cylinder's intersect
//! in `shapes/cylinder.ts`.

use crate::unified::geom::{CenterBox, Point};

/// The tolerance of the segment test in `intersectLine`.
const EPSILON: f64 = 1e-6;

/// `intersect-rect.js`: the point where the line from the centre meets the box's edge.
#[must_use]
pub(super) fn rect(node: CenterBox, point: Point) -> Point {
    let (x, y) = (node.x, node.y);
    let dx = point.x - x;
    let dy = point.y - y;
    let mut w = node.width / 2.0;
    let mut h = node.height / 2.0;
    let (sx, sy);
    if dy.abs() * w > dx.abs() * h {
        // The intersection is at the top or the bottom.
        if dy < 0.0 {
            h = -h;
        }
        sx = if dy == 0.0 { 0.0 } else { h * dx / dy };
        sy = h;
    } else {
        // The intersection is at a side.
        if dx < 0.0 {
            w = -w;
        }
        sx = w;
        sy = if dx == 0.0 { 0.0 } else { w * dy / dx };
    }
    Point::new(x + sx, y + sy)
}

/// `intersect-ellipse.js`: the point on the ellipse with radii `rx` and `ry` in the direction of
/// `point`. At the centre the reference gives NaN; so does this.
#[must_use]
pub(super) fn ellipse(node: CenterBox, rx: f64, ry: f64, point: Point) -> Point {
    let (cx, cy) = (node.x, node.y);
    let px = cx - point.x;
    let py = cy - point.y;
    let det = (rx * rx * py * py + ry * ry * px * px).sqrt();
    let mut dx = (rx * ry * px / det).abs();
    if point.x < cx {
        dx = -dx;
    }
    let mut dy = (rx * ry * py / det).abs();
    if point.y < cy {
        dy = -dy;
    }
    Point::new(cx + dx, cy + dy)
}

/// `intersect-circle.js`: the ellipse with equal radii.
#[must_use]
pub(super) fn circle(node: CenterBox, radius: f64, point: Point) -> Point {
    ellipse(node, radius, radius, point)
}

/// `intersect-polygon.js`: the point where the line from the centre meets the polygon's edges,
/// nearest to `point`. The points are in the shape's own coordinates; they are moved to the
/// node's box, whose top-left corner is found from the node's size and the points' least x and
/// y. With no crossing the reference returns the node, so this returns the centre.
#[must_use]
pub(super) fn polygon(node: CenterBox, points: &[Point], point: Point) -> Point {
    let min_x = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let min_y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let left = node.x - node.width / 2.0 - min_x;
    let top = node.y - node.height / 2.0 - min_y;

    let mut nearest: Option<(Point, f64)> = None;
    for (index, &p1) in points.iter().enumerate() {
        let p2 = points[if index + 1 < points.len() {
            index + 1
        } else {
            0
        }];
        let crossing = intersect_line(
            node.center(),
            point,
            Point::new(left + p1.x, top + p1.y),
            Point::new(left + p2.x, top + p2.y),
        );
        if let Some(crossing) = crossing {
            // The reference sorts by distance and takes the first; a tie keeps the earlier one.
            let distance = crossing.distance(point);
            if nearest.is_none_or(|(_, best)| distance < best) {
                nearest = Some((crossing, distance));
            }
        }
    }
    nearest.map_or_else(|| node.center(), |(crossing, _)| crossing)
}

/// The cylinder's intersect (`shapes/cylinder.ts`): the box's edge, then, where the edge is on the
/// sides or the cap, the ellipse of the cap moved down or up to the outline.
#[must_use]
pub(super) fn cylinder(node: CenterBox, rx: f64, ry: f64, point: Point) -> Point {
    let mut position = rect(node, point);
    let x = position.x - node.x;
    if rx != 0.0
        && (x.abs() < node.width / 2.0
            || (x.abs() == node.width / 2.0
                && (position.y - node.y).abs() > node.height / 2.0 - ry))
    {
        let mut y = ry * ry * (1.0 - (x * x) / (rx * rx));
        if y > 0.0 {
            y = y.sqrt();
        }
        y = ry - y;
        if point.y - node.y > 0.0 {
            y = -y;
        }
        position.y += y;
    }
    position
}

/// `tiltedCylinder.ts`'s intersect: the box, then the cap on its left or right side, which bulges
/// by `rx` as the cylinder's ellipse does.
#[must_use]
pub(super) fn tilted_cylinder(node: CenterBox, rx: f64, ry: f64, point: Point) -> Point {
    let mut position = rect(node, point);
    let y = position.y - node.y;
    if ry != 0.0
        && (y.abs() < node.height / 2.0
            || (y.abs() == node.height / 2.0
                && (position.x - node.x).abs() > node.width / 2.0 - rx))
    {
        let mut x = rx * rx * (1.0 - (y * y) / (ry * ry));
        if x != 0.0 {
            x = x.abs().sqrt();
        }
        x = rx - x;
        if point.x - node.x > 0.0 {
            x = -x;
        }
        position.x += x;
    }
    position
}

/// `intersectLine` (`intersect-line.js`): where the segment `q1`-`q2` crosses the line `p1`-`p2`,
/// by the Graphics Gems formula. Returns `None` when the segments do not meet or are collinear.
///
/// The reference's half-unit nudge is not here: it was an integer-division correction that
/// JavaScript does not need (`intersect-line.js` 49-76).
#[must_use]
pub(super) fn intersect_line(p1: Point, p2: Point, q1: Point, q2: Point) -> Option<Point> {
    // The line through p1 and p2 is a1 x + b1 y + c1 = 0.
    let a1 = p2.y - p1.y;
    let b1 = p1.x - p2.x;
    let c1 = p2.x * p1.y - p1.x * p2.y;
    let r3 = a1 * q1.x + b1 * q1.y + c1;
    let r4 = a1 * q2.x + b1 * q2.y + c1;
    if r3 != 0.0 && r4 != 0.0 && same_sign(r3, r4) {
        return None;
    }

    // The line through q1 and q2 is a2 x + b2 y + c2 = 0.
    let a2 = q2.y - q1.y;
    let b2 = q1.x - q2.x;
    let c2 = q2.x * q1.y - q1.x * q2.y;
    let r1 = a2 * p1.x + b2 * p1.y + c2;
    let r2 = a2 * p2.x + b2 * p2.y + c2;
    // The reference's test is gated by both values being within EPSILON, so it never rejects a
    // crossing that the first test accepts (intersect-line.js 39).
    if r1.abs() < EPSILON && r2.abs() < EPSILON && same_sign(r1, r2) {
        return None;
    }

    let denominator = a1 * b2 - a2 * b1;
    if denominator == 0.0 {
        return None;
    }
    let x = (b1 * c2 - b2 * c1) / denominator;
    let y = (a2 * c1 - a1 * c2) / denominator;
    Some(Point::new(x, y))
}

/// Whether two numbers have the same sign, as the reference's `sameSign`.
fn same_sign(a: f64, b: f64) -> bool {
    a * b > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    // intersect-line.spec.ts, "intersectLine".

    #[test]
    fn intersects_a_vertical_line_with_a_horizontal_segment_exactly_at_the_crossing() {
        // Vertical line x = 10, horizontal segment y = 20.
        let result = intersect_line(p(10.0, 0.0), p(10.0, 100.0), p(0.0, 20.0), p(50.0, 20.0));
        assert_eq!(result, Some(p(10.0, 20.0)));
    }

    #[test]
    fn intersects_a_horizontal_line_with_a_vertical_segment_exactly_at_the_crossing() {
        let result = intersect_line(p(0.0, 7.0), p(100.0, 7.0), p(33.0, 0.0), p(33.0, 50.0));
        assert_eq!(result, Some(p(33.0, 7.0)));
    }

    #[test]
    fn is_unbiased_whichever_way_the_numerator_and_denominator_sign_out() {
        // The same crossing, (33, 57), with the segments walked in each direction.
        let expected = p(33.0, 57.0);
        let combos = [
            [p(33.0, 0.0), p(33.0, 100.0), p(0.0, 57.0), p(100.0, 57.0)],
            [p(33.0, 100.0), p(33.0, 0.0), p(0.0, 57.0), p(100.0, 57.0)],
            [p(33.0, 0.0), p(33.0, 100.0), p(100.0, 57.0), p(0.0, 57.0)],
            [p(33.0, 100.0), p(33.0, 0.0), p(100.0, 57.0), p(0.0, 57.0)],
        ];
        for [p1, p2, q1, q2] in combos {
            let result = intersect_line(p1, p2, q1, q2).expect("the segments cross");
            assert!((result.x - expected.x).abs() < 1e-9, "{result:?}");
            assert!((result.y - expected.y).abs() < 1e-9, "{result:?}");
        }
    }

    #[test]
    fn keeps_the_crossing_on_the_query_line_for_non_integer_coordinates() {
        // Node centres are fractional in real layouts; the attachment stays on the ray.
        let x = 285.01588439941406;
        let result = intersect_line(p(x, 34.5), p(x, 67.0), p(0.0, 57.0), p(400.0, 57.0))
            .expect("the segments cross");
        assert!((result.x - x).abs() < 1e-9);
        assert!((result.y - 57.0).abs() < 1e-9);
    }

    // intersect-line.spec.ts, "intersectPolygon".

    #[test]
    fn attaches_on_the_outline_on_the_axis_the_query_ray_travelled() {
        // A diamond, 100 by 100, centred at (200, 200), with its vertices at the midpoints of the
        // bounding box's sides. A ray due east from the centre attaches at the east vertex.
        let node = CenterBox::new(200.0, 200.0, 100.0, 100.0);
        let points = [p(0.0, -50.0), p(50.0, 0.0), p(0.0, 50.0), p(-50.0, 0.0)];
        let result = polygon(node, &points, p(400.0, 200.0));
        assert!((result.x - 250.0).abs() < 1e-9);
        assert!((result.y - 200.0).abs() < 1e-9);
    }
}
