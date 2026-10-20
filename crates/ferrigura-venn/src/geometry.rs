//! `circleintersection.js` as the bundle has it (build lines 1-266): the area of circles, the
//! overlap of two circles, their intersection points, and the area of an intersection with its arcs.

use std::f64::consts::PI;

use ferrigura_jsmath::{acos, atan2, cos, sin};

use crate::Circle;
use crate::js::{js_min, js_sort};

/// The tolerance of the containment and the arc tests (`SMALL` in the source).
pub(crate) const SMALL: f64 = 1e-10;

/// A point of the plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub(crate) fn centre(circle: &Circle) -> Point {
    Point {
        x: circle.x,
        y: circle.y,
    }
}

/// `distance(p1, p2)`: the euclidean distance.
pub(crate) fn distance(p1: Point, p2: Point) -> f64 {
    ((p1.x - p2.x) * (p1.x - p2.x) + (p1.y - p2.y) * (p1.y - p2.y)).sqrt()
}

/// `circleArea(r, width)`: the area of a circular segment of width `width`.
pub(crate) fn circle_area(r: f64, width: f64) -> f64 {
    r * r * acos(1.0 - width / r) - (r - width) * (width * (2.0 * r - width)).sqrt()
}

/// `circleOverlap(r1, r2, d)`: the overlap of two circles whose centres are `d` apart.
pub(crate) fn circle_overlap(r1: f64, r2: f64, d: f64) -> f64 {
    if d >= r1 + r2 {
        return 0.0;
    }
    if d <= (r1 - r2).abs() {
        return PI * js_min(r1, r2) * js_min(r1, r2);
    }
    let w1 = r1 - (d * d - r2 * r2 + r1 * r1) / (2.0 * d);
    let w2 = r2 - (d * d - r1 * r1 + r2 * r2) / (2.0 * d);
    circle_area(r1, w1) + circle_area(r2, w2)
}

/// `circleCircleIntersection(p1, p2)`: the points where two circles meet, or none.
pub(crate) fn circle_circle_intersection(p1: Circle, p2: Circle) -> Vec<Point> {
    let d = distance(centre(&p1), centre(&p2));
    let r1 = p1.radius;
    let r2 = p2.radius;
    if d >= r1 + r2 || d <= (r1 - r2).abs() {
        return Vec::new();
    }
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let h = (r1 * r1 - a * a).sqrt();
    let x0 = p1.x + (a * (p2.x - p1.x)) / d;
    let y0 = p1.y + (a * (p2.y - p1.y)) / d;
    let rx = -(p2.y - p1.y) * (h / d);
    let ry = -(p2.x - p1.x) * (h / d);
    vec![
        Point {
            x: x0 + rx,
            y: y0 - ry,
        },
        Point {
            x: x0 - rx,
            y: y0 + ry,
        },
    ]
}

/// The centre of a bunch of points: the mean, summed in order.
pub(crate) fn get_center(points: &[Point]) -> Point {
    let mut center = Point { x: 0.0, y: 0.0 };
    for point in points {
        center.x += point.x;
        center.y += point.y;
    }
    center.x /= points.len() as f64;
    center.y /= points.len() as f64;
    center
}

/// An intersection point, with the two circles it lies on (their indices) and its angle about the
/// centre of the inner points.
#[derive(Clone, Copy, Debug)]
struct InnerPoint {
    point: Point,
    parent: [usize; 2],
    angle: f64,
}

/// An arc of the boundary of an intersection: the circle it lies on, where it starts (`p1`), and the
/// point the previous arc ended at (`p2`, the arc's other end, as the library's `arc` object has it).
/// `large` is the flag of the arc's `A` command: the arc is wider than a half circle, by the
/// library's test on the clamped width. The sweep flag is always set (`sweep: true`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Arc {
    pub circle: Circle,
    pub p1: Point,
    pub p2: Point,
    pub large: bool,
}

/// The area of an intersection, with its arcs in order (`intersectionArea`'s `stats`).
#[derive(Clone, Debug)]
pub(crate) struct IntersectionArea {
    pub area: f64,
    pub arcs: Vec<Arc>,
}

/// `intersectionArea(circles)`. `circles` must not be empty.
pub(crate) fn intersection_area(circles: &[Circle]) -> IntersectionArea {
    let mut intersection_points: Vec<InnerPoint> = Vec::new();
    for i in 0..circles.len() {
        for j in (i + 1)..circles.len() {
            for point in circle_circle_intersection(circles[i], circles[j]) {
                intersection_points.push(InnerPoint {
                    point,
                    parent: [i, j],
                    angle: 0.0,
                });
            }
        }
    }
    let mut inner: Vec<InnerPoint> = intersection_points
        .into_iter()
        .filter(|candidate| contained_in_circles(candidate.point, circles))
        .collect();

    let mut arc_area = 0.0;
    let mut polygon_area = 0.0;
    let mut arcs: Vec<Arc> = Vec::new();

    if inner.len() > 1 {
        let points: Vec<Point> = inner.iter().map(|candidate| candidate.point).collect();
        let center = get_center(&points);
        for candidate in inner.iter_mut() {
            candidate.angle = atan2(candidate.point.x - center.x, candidate.point.y - center.y);
        }
        js_sort(&mut inner, |a, b| b.angle - a.angle);

        let mut p2 = inner[inner.len() - 1];
        for &p1 in &inner {
            polygon_area += (p2.point.x + p1.point.x) * (p1.point.y - p2.point.y);

            let mid_point = Point {
                x: (p1.point.x + p2.point.x) / 2.0,
                y: (p1.point.y + p2.point.y) / 2.0,
            };
            // The circle whose arc is narrowest: the first one wins a tie, as `arc.width > width`
            // is strict.
            let mut arc: Option<(Circle, f64)> = None;
            for j in 0..2 {
                if p2.parent.contains(&p1.parent[j]) {
                    let circle = circles[p1.parent[j]];
                    let a1 = atan2(p1.point.x - circle.x, p1.point.y - circle.y);
                    let a2 = atan2(p2.point.x - circle.x, p2.point.y - circle.y);

                    let mut angle_diff = a2 - a1;
                    if angle_diff < 0.0 {
                        angle_diff += 2.0 * PI;
                    }

                    let a = a2 - angle_diff / 2.0;
                    let mut width = distance(
                        mid_point,
                        Point {
                            x: circle.x + circle.radius * sin(a),
                            y: circle.y + circle.radius * cos(a),
                        },
                    );
                    if width > circle.radius * 2.0 {
                        width = circle.radius * 2.0;
                    }

                    let replaces = match arc {
                        None => true,
                        Some((_, current)) => current > width,
                    };
                    if replaces {
                        arc = Some((circle, width));
                    }
                }
            }

            if let Some((circle, width)) = arc {
                arcs.push(Arc {
                    circle,
                    p1: p1.point,
                    p2: p2.point,
                    large: width > circle.radius,
                });
                arc_area += circle_area(circle.radius, width);
                p2 = p1;
            }
        }
    } else {
        let mut smallest = circles[0];
        for circle in &circles[1..] {
            if circle.radius < smallest.radius {
                smallest = *circle;
            }
        }

        let mut disjoint = false;
        for circle in circles {
            if distance(centre(circle), centre(&smallest)) > (smallest.radius - circle.radius).abs()
            {
                disjoint = true;
                break;
            }
        }

        if disjoint {
            arc_area = 0.0;
            polygon_area = 0.0;
        } else {
            arc_area = smallest.radius * smallest.radius * PI;
            // The library's arc has no `p2` here. It is never read: one arc is drawn as its circle.
            let p1 = Point {
                x: smallest.x,
                y: smallest.y + smallest.radius,
            };
            arcs.push(Arc {
                circle: smallest,
                p1,
                p2: p1,
                large: false,
            });
        }
    }

    polygon_area /= 2.0;
    IntersectionArea {
        area: arc_area + polygon_area,
        arcs,
    }
}

/// Whether a point lies inside every circle, within `SMALL`.
fn contained_in_circles(point: Point, circles: &[Circle]) -> bool {
    circles
        .iter()
        .all(|circle| distance(point, centre(circle)) < circle.radius + SMALL)
}
