//! The outlines that rough.js draws at roughness 0, as deterministic paths with the same geometry.
//!
//! At roughness 0 rough.js draws every line through its control points on the segment, so a
//! polygon is its own outline and its box is the box of its points. A circle is not: rough.js
//! samples the ellipse at a quarter of its step and joins the samples with cubic curves whose
//! control points lie outside the ellipse, so the curve overshoots the diameter (14 becomes
//! 14.0133 wide). The box is what the layout uses, so the curve is reproduced here
//! (`renderer.js` `_computeEllipsePoints` with `coreOnly`, `_curve`, and `generateEllipseParams`).
//! The `handDrawnSeed` and the random offsets play no part at roughness 0. See
//! `docs-port/flowchart/render-notes/shapes-core.md` §2.3.

use std::f64::consts::PI;

use super::util::attribute;
use crate::flowchart::geom::{BBox, Point};
use crate::svg::number;

/// rough.js's default `curveStepCount`.
const CURVE_STEP_COUNT: f64 = 9.0;

/// An outline: its path data and its box, in the coordinates of the points.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Outline {
    /// The path data.
    pub d: String,
    /// The box of the path's geometry.
    pub bbox: BBox,
}

/// The outline of a polygon: rough.js draws each side as a line, so the path is the points in
/// order, closed. The box is the box of the points.
#[must_use]
pub(super) fn polygon(points: &[Point]) -> Outline {
    Outline {
        d: super::util::create_path_from_points(points),
        bbox: BBox::from_points(points.iter().copied()).unwrap_or_default(),
    }
}

/// The outline of a rectangle, as `rc.rectangle` draws it at roughness 0: its four corners.
#[must_use]
pub(super) fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Outline {
    polygon(&[
        Point::new(x, y),
        Point::new(x + width, y),
        Point::new(x + width, y + height),
        Point::new(x, y + height),
    ])
}

/// The outline of a rectangle with rounded corners, from `createRoundedRectPathD`
/// (`shapes/roundedRectPath.ts`). Its arcs are quarter circles that end on the sides of the
/// rectangle, so its box is the rectangle's.
#[must_use]
pub(super) fn rounded_rectangle(x: f64, y: f64, width: f64, height: f64, radius: f64) -> Outline {
    let r = number(radius);
    let d = format!(
        "M{},{} H{} A{r},{r} 0 0 1 {},{} V{} A{r},{r} 0 0 1 {},{} H{} A{r},{r} 0 0 1 {},{} V{} A{r},{r} 0 0 1 {},{} Z",
        number(x + radius),
        number(y),
        number(x + width - radius),
        number(x + width),
        number(y + radius),
        number(y + height - radius),
        number(x + width - radius),
        number(y + height),
        number(x + radius),
        number(x),
        number(y + height - radius),
        number(y + radius),
        number(x + radius),
        number(y),
    );
    Outline {
        d,
        bbox: BBox::new(x, y, width, height),
    }
}

/// The outline of an ellipse with the given diameters, centred on `(cx, cy)`, as
/// `rc.ellipse(cx, cy, width, height)` draws it at roughness 0.
#[must_use]
pub(super) fn ellipse(cx: f64, cy: f64, width: f64, height: f64) -> Outline {
    let rx = (width / 2.0).abs();
    let ry = (height / 2.0).abs();
    // `generateEllipseParams`: the number of steps grows with the size.
    let psq = (2.0 * PI * (((width / 2.0).powi(2) + (height / 2.0).powi(2)) / 2.0).sqrt()).sqrt();
    let steps = CURVE_STEP_COUNT
        .max(CURVE_STEP_COUNT / 200.0_f64.sqrt() * psq)
        .ceil();
    let increment = 2.0 * PI / steps;
    // `_computeEllipsePoints` at roughness 0 samples a quarter of the step, from one step before
    // the start to one step past the end.
    let step = increment / 4.0;
    let sample = |angle: f64| Point::new(cx + rx * angle.cos(), cy + ry * angle.sin());
    let mut points = vec![sample(-step)];
    let mut angle = 0.0;
    while angle <= 2.0 * PI {
        points.push(sample(angle));
        angle += step;
    }
    points.push(sample(0.0));
    points.push(sample(step));

    // `_curve` with tightness 0: a Catmull-Rom spline through the samples, from the second sample
    // to the one before the last.
    let mut d = format!("M{},{}", number(points[1].x), number(points[1].y));
    let mut bounds = vec![points[1]];
    for i in 1..points.len().saturating_sub(2) {
        let (before, here, next, after) = (points[i - 1], points[i], points[i + 1], points[i + 2]);
        let control_1 = Point::new(
            here.x + (next.x - before.x) / 6.0,
            here.y + (next.y - before.y) / 6.0,
        );
        let control_2 = Point::new(
            next.x + (here.x - after.x) / 6.0,
            next.y + (here.y - after.y) / 6.0,
        );
        d.push_str(&format!(
            " C{},{} {},{} {},{}",
            number(control_1.x),
            number(control_1.y),
            number(control_2.x),
            number(control_2.y),
            number(next.x),
            number(next.y),
        ));
        bounds.extend(cubic_extremes(here, control_1, control_2, next));
    }
    Outline {
        d,
        bbox: BBox::from_points(bounds).unwrap_or_default(),
    }
}

/// The points where a cubic curve is furthest in x or y, with its end points: the points that
/// give its tight box, as `getBBox` computes it.
fn cubic_extremes(p0: Point, c1: Point, c2: Point, p3: Point) -> Vec<Point> {
    let mut points = vec![p0, p3];
    for axis in [Axis::X, Axis::Y] {
        // The derivative of the curve is a quadratic in t: a t^2 + b t + c.
        let (a0, a1, a2, a3) = axis.pick(p0, c1, c2, p3);
        let d0 = a1 - a0;
        let d1 = a2 - a1;
        let d2 = a3 - a2;
        let a = d0 - 2.0 * d1 + d2;
        let b = 2.0 * (d1 - d0);
        let c = d0;
        for t in quadratic_roots(a, b, c) {
            if t > 0.0 && t < 1.0 {
                points.push(evaluate(p0, c1, c2, p3, t));
            }
        }
    }
    points
}

/// The real roots of `a t^2 + b t + c`, a linear equation when `a` is zero.
fn quadratic_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    if a.abs() < 1e-12 {
        return if b.abs() < 1e-12 {
            Vec::new()
        } else {
            vec![-c / b]
        };
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return Vec::new();
    }
    let root = discriminant.sqrt();
    vec![(-b + root) / (2.0 * a), (-b - root) / (2.0 * a)]
}

/// The point of a cubic curve at `t`.
fn evaluate(p0: Point, c1: Point, c2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let weight = |a: f64, b: f64, c: f64, d: f64| {
        u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
    };
    Point::new(
        weight(p0.x, c1.x, c2.x, p3.x),
        weight(p0.y, c1.y, c2.y, p3.y),
    )
}

#[derive(Debug, Clone, Copy)]
enum Axis {
    X,
    Y,
}

impl Axis {
    fn pick(self, p0: Point, c1: Point, c2: Point, p3: Point) -> (f64, f64, f64, f64) {
        match self {
            Self::X => (p0.x, c1.x, c2.x, p3.x),
            Self::Y => (p0.y, c1.y, c2.y, p3.y),
        }
    }
}

/// The two paths that rough.js writes for one outline: the fill, then the stroke. The fill path
/// has no stroke; the stroke path has no fill. The colours come from the style sheet
/// (`docs-port/decisions/0007`), so neither path carries one. `style` is written on both paths,
/// as the reference does for the node's declarations. `stroke_visible` is false for an outline
/// with `stroke: none`.
#[must_use]
pub(super) fn paths(
    outline: &Outline,
    style: Option<&str>,
    stroke_width: &str,
    stroke_visible: bool,
) -> String {
    let style = style.map_or_else(String::new, |style| attribute("style", style));
    let stroke = if stroke_visible {
        ""
    } else {
        " stroke=\"none\""
    };
    format!(
        "<path d=\"{d}\" stroke=\"none\" stroke-width=\"0\"{style}/><path d=\"{d}\" fill=\"none\"{stroke} stroke-width=\"{stroke_width}\" stroke-dasharray=\"0 0\"{style}/>",
        d = outline.d,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_circle_of_diameter_fourteen_overshoots_its_diameter_by_the_reference_amount() {
        // The rough circle of `filledCircle` is 14.0133 wide and exactly 14 high (shapes-core.md
        // R5, computed with the same cubic bounds).
        let outline = ellipse(0.0, 0.0, 14.0, 14.0);
        assert!(
            (outline.bbox.width - 14.0133).abs() < 5e-5,
            "{:?}",
            outline.bbox
        );
        assert!(
            (outline.bbox.height - 14.0).abs() < 1e-9,
            "{:?}",
            outline.bbox
        );
    }

    #[test]
    fn a_circle_of_diameter_five_overshoots_by_the_reference_amount() {
        let outline = ellipse(0.0, 0.0, 5.0, 5.0);
        assert!(
            (outline.bbox.width - 5.0047).abs() < 5e-5,
            "{:?}",
            outline.bbox
        );
    }

    #[test]
    fn a_polygon_outline_has_the_box_of_its_points() {
        let outline = polygon(&[
            Point::new(0.0, 0.0),
            Point::new(70.0, 0.0),
            Point::new(70.0, -10.0),
        ]);
        assert_eq!(outline.bbox, BBox::new(0.0, -10.0, 70.0, 10.0));
    }
}
