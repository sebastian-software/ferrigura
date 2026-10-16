//! The geometry of the ELK post-processing: a port of `packages/mermaid/src/rendering-util/layout-algorithms/elk/geometry.ts`
//! and of `find-common-ancestor.ts` in the same folder.
//!
//! Boxes are the reference's centre-and-size boxes ([`CenterBox`]). A node's outline is a
//! [`NodeOutline`]; a node with none (a group during layout) is passed as `None`, which is what the
//! reference's `node.intersect` being undefined means. See `docs-port/flowchart/render-notes/elk-apply.md`
//! sections 3.6 and 3.7.

use std::collections::HashMap;

use super::geom::{CenterBox, Point};

/// The shared tolerance of "is this point on a border" and of the other near-equal tests in
/// `geometry.ts` (`EPS`).
pub const EPS: f64 = 1.0;

/// How far a point is pushed radially outward when it sits too close to a border (`PUSH_OUT`).
pub const PUSH_OUT: f64 = 10.0;

/// Bisection steps used to walk a ray onto the node outline (`OUTLINE_RAY_STEPS`). Twenty steps
/// take a 200px bracket to about 2e-4px.
const OUTLINE_RAY_STEPS: usize = 20;

/// How far off an axis a departure may sit and still count as axis-aligned (`DEPARTURE_AXIS_EPS`).
const DEPARTURE_AXIS_EPS: f64 = 1e-6;

/// The outline of a node: the reference's `node.intersect`. Returns the point where the ray from
/// the node's centre through `probe` leaves the outline, or `None` when the outline gives no answer.
pub trait NodeOutline {
    /// The crossing of the ray from the centre through `probe`.
    fn crossing(&self, probe: Point) -> Option<Point>;
}

impl<F> NodeOutline for F
where
    F: Fn(Point) -> Option<Point>,
{
    fn crossing(&self, probe: Point) -> Option<Point> {
        self(probe)
    }
}

/// Which end of a polyline is meant: `'start'` or `'end'` in the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The first point.
    Start,
    /// The last point.
    End,
}

/// `Math.sign`: `1`, `-1`, `0` for zero (keeping its sign), and `NaN` for `NaN`. Unlike
/// `f64::signum`, zero stays zero.
#[must_use]
pub fn js_sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

/// `Math.hypot(x, y)` as V8 computes it: the larger magnitude factored out, the squares summed with
/// Kahan compensation, then the square root scaled back. Infinity wins over `NaN`, as in V8.
#[must_use]
pub fn js_hypot(x: f64, y: f64) -> f64 {
    let (a, b) = (x.abs(), y.abs());
    if a == f64::INFINITY || b == f64::INFINITY {
        return f64::INFINITY;
    }
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    let max = if a > b { a } else { b };
    if max == 0.0 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for value in [a, b] {
        let n = value / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    f64::sqrt(sum) * max
}

/// Whether `p` lies on one of the four sides of `bounds`, within `tol` (`onBorder`). The sides
/// extend `tol` past the corners.
#[must_use]
pub fn on_border(bounds: CenterBox, p: Point, tol: f64) -> bool {
    let half_w = bounds.width / 2.0;
    let half_h = bounds.height / 2.0;
    let left = bounds.x - half_w;
    let right = bounds.x + half_w;
    let top = bounds.y - half_h;
    let bottom = bounds.y + half_h;

    let on_left = (p.x - left).abs() <= tol && p.y >= top - tol && p.y <= bottom + tol;
    let on_right = (p.x - right).abs() <= tol && p.y >= top - tol && p.y <= bottom + tol;
    let on_top = (p.y - top).abs() <= tol && p.x >= left - tol && p.x <= right + tol;
    let on_bottom = (p.y - bottom).abs() <= tol && p.x >= left - tol && p.x <= right + tol;
    on_left || on_right || on_top || on_bottom
}

/// The point where the segment from `inside` to `outside` leaves the box `node` (`intersection`).
///
/// This version does not snap to `outside` when a term is zero: it only enforces the axis-constant
/// case for purely vertical or horizontal approaches.
#[must_use]
pub fn intersection(node: CenterBox, outside: Point, inside: Point) -> Point {
    let x = node.x;
    let y = node.y;

    let w = node.width / 2.0;
    let h = node.height / 2.0;

    let q_len = (outside.y - inside.y).abs();
    let r_len = (outside.x - inside.x).abs();

    if (y - outside.y).abs() * w > (x - outside.x).abs() * h {
        // The intersection is on the top or bottom of the box.
        let q = if inside.y < outside.y {
            outside.y - h - y
        } else {
            y - h - outside.y
        };
        let r = (r_len * q) / q_len;
        let mut res = Point::new(
            if inside.x < outside.x {
                inside.x + r
            } else {
                inside.x - r_len + r
            },
            if inside.y < outside.y {
                inside.y + q_len - q
            } else {
                inside.y - q_len + q
            },
        );
        if r_len == 0.0 {
            res.x = outside.x;
        }
        if q_len == 0.0 {
            res.y = outside.y;
        }
        res
    } else {
        // The intersection is on a side of the box.
        let r = if inside.x < outside.x {
            outside.x - w - x
        } else {
            x - w - outside.x
        };
        let q = (q_len * r) / r_len;
        let mut res_x = if inside.x < outside.x {
            inside.x + r_len - r
        } else {
            inside.x - r_len + r
        };
        let mut res_y = if inside.y < outside.y {
            inside.y + q
        } else {
            inside.y - q
        };
        if r_len == 0.0 {
            res_x = outside.x;
        }
        if q_len == 0.0 {
            res_y = outside.y;
        }
        Point::new(res_x, res_y)
    }
}

/// Whether `p` is outside the box, a point on the border counting as outside (`outsideNode`).
#[must_use]
pub fn outside_node(node: CenterBox, p: Point) -> bool {
    let dx = (p.x - node.x).abs();
    let dy = (p.y - node.y).abs();
    dx >= node.width / 2.0 || dy >= node.height / 2.0
}

/// Pushes `p` radially outward by `push` when it is within `EPS` of a side of `bounds`
/// (`ensureTrulyOutside`). Points at the centre are returned unchanged.
#[must_use]
pub fn ensure_truly_outside(bounds: CenterBox, p: Point, push: f64) -> Point {
    let dx = (p.x - bounds.x).abs();
    let dy = (p.y - bounds.y).abs();
    let w = bounds.width / 2.0;
    let h = bounds.height / 2.0;
    if (dx - w).abs() < EPS || (dy - h).abs() < EPS {
        let dir_x = p.x - bounds.x;
        let dir_y = p.y - bounds.y;
        let len = (dir_x * dir_x + dir_y * dir_y).sqrt();
        if len > 0.0 {
            return Point::new(
                bounds.x + (dir_x / len) * (len + push),
                bounds.y + (dir_y / len) * (len + push),
            );
        }
    }
    p
}

/// A point inside the box that is on the same side as `outside`, for the fallback intersection
/// (`makeInsidePoint`).
#[must_use]
pub fn make_inside_point(bounds: CenterBox, outside: Point, center: Point) -> Point {
    let is_vertical = (outside.x - bounds.x).abs() < EPS;
    let is_horizontal = (outside.y - bounds.y).abs() < EPS;
    Point::new(
        if is_vertical {
            outside.x
        } else if outside.x < bounds.x {
            bounds.x - bounds.width / 4.0
        } else {
            bounds.x + bounds.width / 4.0
        },
        if is_horizontal { outside.y } else { center.y },
    )
}

/// The node's own crossing, if it gives one that is on the right side and beyond `EPS`
/// (`tryNodeIntersect`). `None` for a node without an outline.
#[must_use]
pub fn try_node_intersect(
    node: Option<&dyn NodeOutline>,
    bounds: CenterBox,
    outside: Point,
) -> Option<Point> {
    let res = node?.crossing(outside)?;
    let wrong_side =
        (outside.x < bounds.x && res.x > bounds.x) || (outside.x > bounds.x && res.x < bounds.x);
    if wrong_side {
        return None;
    }
    let dist = js_hypot(outside.x - res.x, outside.y - res.y);
    if dist <= EPS {
        return None;
    }
    Some(res)
}

/// The rectangle formula, used when the node gives no crossing (`fallbackIntersection`).
#[must_use]
pub fn fallback_intersection(bounds: CenterBox, outside: Point, center: Point) -> Point {
    let inside = make_inside_point(bounds, outside, center);
    intersection(bounds, outside, inside)
}

/// Whether `probe` is inside the outline: no further from the centre than the outline's crossing
/// along the same ray (`insideOutline`).
fn inside_outline(node: &dyn NodeOutline, centre: Point, probe: Point) -> bool {
    let Some(crossing) = node.crossing(probe) else {
        return false;
    };
    let probe_dist = js_hypot(probe.x - centre.x, probe.y - centre.y);
    let outline_dist = js_hypot(crossing.x - centre.x, crossing.y - centre.y);
    probe_dist <= outline_dist + 1e-9
}

/// Where the outline meets the line that enters the node at `port` against the direction the edge
/// departs in (`outlineAttachPoint`).
///
/// For a rectangle the port is already on the outline, so it is returned. For any other shape the
/// attachment slides along the departure axis, so the edge stays straight out of the port. Returns
/// `None` when there is no outline, the departure is diagonal, or the ray cannot be resolved.
#[must_use]
pub fn outline_attach_point(
    node: Option<&dyn NodeOutline>,
    bounds: CenterBox,
    port: Point,
    next: Point,
) -> Option<Point> {
    let node = node?;
    let dx = next.x - port.x;
    let dy = next.y - port.y;
    if dx == 0.0 && dy == 0.0 {
        return None;
    }
    // A diagonal departure has no single axis to keep, so it is declined.
    if dx.abs() > DEPARTURE_AXIS_EPS && dy.abs() > DEPARTURE_AXIS_EPS {
        return None;
    }

    let centre = Point::new(bounds.x, bounds.y);
    let horizontal = dx.abs() > dy.abs();
    let along = |t: f64| {
        if horizontal {
            Point::new(t, port.y)
        } else {
            Point::new(port.x, t)
        }
    };

    // From the centre-line towards the port: inside at one end, on or outside at the other.
    let mut inner = if horizontal { centre.x } else { centre.y };
    let mut outer = if horizontal { port.x } else { port.y };
    if !inside_outline(node, centre, along(inner)) {
        return None;
    }
    if inside_outline(node, centre, along(outer)) {
        // The port is on or inside the outline, so it is the attachment.
        return Some(port);
    }

    for _ in 0..OUTLINE_RAY_STEPS {
        let mid = (inner + outer) / 2.0;
        if inside_outline(node, centre, along(mid)) {
            inner = mid;
        } else {
            outer = mid;
        }
    }
    Some(along(inner))
}

/// The attachment point of an edge that leaves the node towards `outside`, from the node's centre
/// `center` (`computeNodeIntersection`).
#[must_use]
pub fn compute_node_intersection(
    node: Option<&dyn NodeOutline>,
    bounds: CenterBox,
    outside: Point,
    center: Point,
) -> Point {
    let outside = ensure_truly_outside(bounds, outside, PUSH_OUT);
    try_node_intersect(node, bounds, outside)
        .unwrap_or_else(|| fallback_intersection(bounds, outside, center))
}

/// Replaces the first or last point with `value` (`replaceEndpoint`). When `value` is within
/// `tol` of the existing endpoint, that endpoint is removed instead and `value` is discarded.
pub fn replace_endpoint(points: &mut Vec<Point>, which: Side, value: Point, tol: f64) {
    if points.is_empty() {
        return;
    }
    match which {
        Side::Start => {
            let first = points[0];
            if (first.x - value.x).abs() < tol && (first.y - value.y).abs() < tol {
                points.remove(0);
            } else {
                points[0] = value;
            }
        }
        Side::End => {
            let last = points.len() - 1;
            let end = points[last];
            if (end.x - value.x).abs() < tol && (end.y - value.y).abs() < tol {
                points.pop();
            } else {
                points[last] = value;
            }
        }
    }
}

/// The nearest common ancestor of two nodes in the subgraph tree, or `None` for the root
/// (`findCommonAncestor`, see [`super::elk_tree::find_common_ancestor`]). A node is its own
/// ancestor when it is the other one's parent, so an edge from a subgraph to its own member gets
/// the subgraph.
#[must_use]
pub fn find_common_ancestor(
    id1: &str,
    id2: &str,
    parent_by_id: &HashMap<String, String>,
) -> Option<String> {
    let ancestor = super::elk_tree::find_common_ancestor(id1, id2, parent_by_id);
    // `calcOffset` treats the root by name.
    (ancestor != "root").then_some(ancestor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    /// A diamond whose outline runs through the midpoints of its box sides, as in the reference's
    /// `diamond-intersections` fixture. Written out, since the polygon intersect is ported with the
    /// shapes.
    fn diamond_crossing(bounds: CenterBox, half: f64, probe: Point) -> Point {
        let dx = probe.x - bounds.x;
        let dy = probe.y - bounds.y;
        let t = half / (dx.abs() + dy.abs());
        Point::new(bounds.x + dx * t, bounds.y + dy * t)
    }

    /// The rectangle's own crossing: the ray from the centre leaves through the nearer side.
    fn rect_crossing(bounds: CenterBox, probe: Point) -> Point {
        let dx = probe.x - bounds.x;
        let dy = probe.y - bounds.y;
        let t = f64::min(
            (bounds.width / 2.0) / dx.abs().max(1e-9),
            (bounds.height / 2.0) / dy.abs().max(1e-9),
        );
        Point::new(bounds.x + dx * t, bounds.y + dy * t)
    }

    // geometry.spec.ts: "geometry helpers"

    #[test]
    fn intersection_vertical_approach_hits_bottom_border() {
        // geometry.spec.ts: "intersection: vertical approach hits bottom border"
        let rect = CenterBox::new(0.0, 0.0, 100.0, 50.0);
        let res = intersection(rect, Point::new(0.0, 100.0), Point::new(0.0, 0.0));
        assert!(approx(res.x, 0.0, 1e-6));
        assert!(approx(res.y, 25.0, 1e-6));
    }

    #[test]
    fn ensure_truly_outside_nudges_near_boundary_point_outward() {
        // geometry.spec.ts: "ensureTrulyOutside nudges near-boundary point outward"
        let rect = CenterBox::new(0.0, 0.0, 100.0, 50.0);
        let out = ensure_truly_outside(rect, Point::new(0.0, 24.8), 10.0);
        assert!(out.y > 25.0);
    }

    #[test]
    fn make_inside_point_keeps_x_for_vertical_and_y_from_center() {
        // geometry.spec.ts: "makeInsidePoint keeps x for vertical and y from center"
        let rect = CenterBox::new(10.0, 5.0, 100.0, 50.0);
        let outside = Point::new(10.0, 40.0);
        let center = Point::new(99.0, -123.0);
        let inside = make_inside_point(rect, outside, center);
        assert_eq!(inside.x, outside.x);
        assert_eq!(inside.y, center.y);
    }

    #[test]
    fn try_node_intersect_returns_none_for_wrong_side_intersections() {
        // geometry.spec.ts: "tryNodeIntersect returns null for wrong-side intersections"
        let rect = CenterBox::new(0.0, 0.0, 100.0, 50.0);
        let node = |_: Point| Some(Point::new(10.0, 0.0));
        let res = try_node_intersect(Some(&node), rect, Point::new(-50.0, 0.0));
        assert_eq!(res, None);
    }

    #[test]
    fn replace_endpoint_dedup_removes_end_and_start_appropriately() {
        // geometry.spec.ts: "replaceEndpoint dedup removes end/start appropriately"
        let mut pts = vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
        replace_endpoint(&mut pts, Side::End, Point::new(1.0, 1.0), 0.1);
        assert_eq!(pts.len(), 1);

        let mut pts2 = vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
        replace_endpoint(&mut pts2, Side::Start, Point::new(0.0, 0.0), 0.1);
        assert_eq!(pts2.len(), 1);
    }

    // geometry.spec.ts: "geometry helpers" > "outlineAttachPoint"

    const HALF: f64 = 52.74;

    fn diamond_bounds() -> CenterBox {
        CenterBox::new(76.74, 428.5, HALF * 2.0, HALF * 2.0)
    }

    #[test]
    fn outline_attach_point_attaches_at_the_ports_own_offset_along_the_side() {
        // geometry.spec.ts: "attaches on the outline at the port's own offset along the side"
        let bounds = diamond_bounds();
        let diamond = |p: Point| Some(diamond_crossing(bounds, HALF, p));
        let port = Point::new(129.48, 454.87);
        let next = Point::new(144.48, 454.87);

        let attach = outline_attach_point(Some(&diamond), bounds, port, next).unwrap();

        assert_eq!(attach.y, port.y);
        assert!(approx(attach.x, 103.11, 0.01));
        assert!(approx(
            (attach.x - bounds.x).abs() + (attach.y - bounds.y).abs(),
            HALF,
            0.01
        ));
    }

    #[test]
    fn outline_attach_point_keeps_a_vertical_departure_vertical() {
        // geometry.spec.ts: "keeps a vertical departure vertical"
        let bounds = diamond_bounds();
        let diamond = |p: Point| Some(diamond_crossing(bounds, HALF, p));
        let port = Point::new(60.0, 481.24);
        let next = Point::new(60.0, 520.0);

        let attach = outline_attach_point(Some(&diamond), bounds, port, next).unwrap();

        assert_eq!(attach.x, port.x);
        assert!(approx(
            (attach.x - bounds.x).abs() + (attach.y - bounds.y).abs(),
            HALF,
            0.01
        ));
    }

    #[test]
    fn outline_attach_point_returns_the_port_unchanged_for_a_shape_whose_outline_is_its_box() {
        // geometry.spec.ts: "returns the port unchanged for a shape whose outline is its box"
        let rect = CenterBox::new(100.0, 100.0, 80.0, 40.0);
        let rect_node = |p: Point| Some(rect_crossing(rect, p));
        let port = Point::new(140.0, 112.0);

        let attach =
            outline_attach_point(Some(&rect_node), rect, port, Point::new(180.0, 112.0)).unwrap();

        assert!(approx(attach.x, port.x, 0.01));
        assert!(approx(attach.y, port.y, 0.01));
    }

    #[test]
    fn outline_attach_point_declines_a_shapeless_node_rather_than_guessing() {
        // geometry.spec.ts: "declines a shapeless node rather than guessing"
        assert_eq!(
            outline_attach_point(
                None,
                diamond_bounds(),
                Point::new(129.48, 454.87),
                Point::new(144.48, 454.87)
            ),
            None
        );
    }

    #[test]
    fn outline_attach_point_declines_a_diagonal_departure_rather_than_forcing_an_axis() {
        // geometry.spec.ts: "declines a diagonal departure rather than forcing it onto an axis"
        let bounds = diamond_bounds();
        let diamond = |p: Point| Some(diamond_crossing(bounds, HALF, p));
        let port = Point::new(129.48, 454.87);

        assert_eq!(
            outline_attach_point(Some(&diamond), bounds, port, Point::new(144.48, 469.87)),
            None
        );
        assert_eq!(
            outline_attach_point(Some(&diamond), bounds, port, Point::new(174.48, 459.87)),
            None
        );
        assert_eq!(
            outline_attach_point(Some(&diamond), bounds, port, Point::new(134.48, 494.87)),
            None
        );
    }

    #[test]
    fn outline_attach_point_still_accepts_a_departure_carrying_floating_point_dust() {
        // geometry.spec.ts: "still accepts a departure carrying floating-point dust"
        let bounds = diamond_bounds();
        let diamond = |p: Point| Some(diamond_crossing(bounds, HALF, p));
        let port = Point::new(129.48, 454.87);

        assert!(
            outline_attach_point(
                Some(&diamond),
                bounds,
                port,
                Point::new(144.48, 454.87 + 1e-9)
            )
            .is_some()
        );
    }

    // Helpers that the reference's render.ts uses but no spec pins down.

    #[test]
    fn js_sign_keeps_zero_and_takes_the_sign_of_the_rest() {
        assert_eq!(js_sign(3.5), 1.0);
        assert_eq!(js_sign(-0.25), -1.0);
        assert_eq!(js_sign(0.0), 0.0);
        assert!(js_sign(f64::NAN).is_nan());
    }

    #[test]
    fn js_hypot_matches_the_plain_formula_on_ordinary_inputs() {
        // V8 scales by the larger magnitude, so the result can differ from `hypot` in the last bit,
        // never by more than rounding.
        assert!(approx(js_hypot(3.0, 4.0), 5.0, 1e-15));
        assert_eq!(js_hypot(0.0, 0.0), 0.0);
        assert_eq!(js_hypot(f64::INFINITY, f64::NAN), f64::INFINITY);
        assert!(js_hypot(f64::NAN, 1.0).is_nan());
    }

    #[test]
    fn find_common_ancestor_of_a_node_with_itself_is_its_parent() {
        // find-common-ancestor.ts:10-13: a self edge uses the parent, or the root.
        let parents = HashMap::from([("a".to_string(), "g".to_string())]);
        assert_eq!(
            find_common_ancestor("a", "a", &parents),
            Some("g".to_string())
        );
        assert_eq!(find_common_ancestor("g", "g", &parents), None);
    }

    #[test]
    fn find_common_ancestor_returns_an_ancestor_that_contains_the_other_node() {
        // find-common-ancestor.ts:15-23: when one endpoint is an ancestor of the other, it is returned.
        let parents = HashMap::from([
            ("a".to_string(), "g".to_string()),
            ("g".to_string(), "h".to_string()),
        ]);
        assert_eq!(
            find_common_ancestor("a", "g", &parents),
            Some("g".to_string())
        );
        assert_eq!(
            find_common_ancestor("g", "a", &parents),
            Some("g".to_string())
        );
        assert_eq!(
            find_common_ancestor("a", "h", &parents),
            Some("h".to_string())
        );
        assert_eq!(find_common_ancestor("a", "b", &parents), None);
    }
}
