//! Skia's tight bounds of a path (`SkPathPriv::ComputeTightBounds`, `core/SkPathPriv.cpp` 1443-1520): the
//! extremes of each curve are found in `float`, and the box is the float minimum and maximum of those and
//! the end points. The roots (`SkFindUnitQuadRoots`, `SkFindCubicExtrema`, `SkFindQuadExtrema`,
//! `conic_find_extrema`) and the evaluations (`SkCubicCoeff`, `SkQuadCoeff`, `SkConicCoeff`) are the
//! ones of `core/SkGeometry.cpp` and `core/SkGeometry.h`.

use crate::builder::Verb;
use crate::segments::Point;

/// `valid_unit_divide` (`SkGeometry.cpp` 54-75): `numer / denom` when it is a root in (0, 1).
fn valid_unit_divide(numer: f32, denom: f32) -> Option<f32> {
    let (mut numer, mut denom) = (numer, denom);
    if numer < 0.0 {
        numer = -numer;
        denom = -denom;
    }
    if denom == 0.0 || numer == 0.0 || numer >= denom {
        return None;
    }
    let ratio = numer / denom;
    if ratio.is_nan() || ratio == 0.0 {
        return None;
    }
    Some(ratio)
}

/// `SkFindUnitQuadRoots` (`SkGeometry.cpp` 95-125): the roots in (0, 1) of `a t² + b t + c`, with the
/// discriminant in double and the roots in float.
fn unit_quad_roots(a: f32, b: f32, c: f32) -> Vec<f32> {
    if a == 0.0 {
        return valid_unit_divide(-c, b).into_iter().collect();
    }
    let discriminant = f64::from(b) * f64::from(b) - 4.0 * f64::from(a) * f64::from(c);
    if discriminant < 0.0 {
        return Vec::new();
    }
    let root = discriminant.sqrt() as f32;
    if !root.is_finite() {
        return Vec::new();
    }
    let q = if b < 0.0 {
        -(b - root) / 2.0
    } else {
        -(b + root) / 2.0
    };
    let mut roots = Vec::with_capacity(2);
    roots.extend(valid_unit_divide(q, a));
    roots.extend(valid_unit_divide(c, q));
    if roots.len() == 2 {
        if roots[0] > roots[1] {
            roots.swap(0, 1);
        } else if roots[0] == roots[1] {
            // A double root is one root.
            roots.pop();
        }
    }
    roots
}

/// `SkFindCubicExtrema` (`SkGeometry.cpp` 454-461): the extremes of one axis of a cubic, with the
/// coefficients divided by three.
fn cubic_extrema(a: f32, b: f32, c: f32, d: f32) -> Vec<f32> {
    let big_a = d - a + 3.0 * (b - c);
    let big_b = 2.0 * (a - b - b + c);
    let big_c = b - a;
    unit_quad_roots(big_a, big_b, big_c)
}

/// `SkCubicCoeff::eval` for one axis (`SkGeometry.h` 475-495): the coefficients in float, and the
/// polynomial by Horner's rule.
fn eval_cubic_axis(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let a = p3 + 3.0 * (p1 - p2) - p0;
    let b = 3.0 * (p2 - (p1 + p1) + p0);
    let c = 3.0 * (p1 - p0);
    ((a * t + b) * t + c) * t + p0
}

/// `SkEvalCubicAt` (`SkGeometry.cpp` 418-424), for the location.
fn eval_cubic(src: &[Point; 4], t: f32) -> Point {
    Point {
        x: eval_cubic_axis(src[0].x, src[1].x, src[2].x, src[3].x, t),
        y: eval_cubic_axis(src[0].y, src[1].y, src[2].y, src[3].y, t),
    }
}

/// `SkFindQuadExtrema` (`SkGeometry.cpp` 265-270): the extreme of one axis of a quadratic.
fn quad_extrema(a: f32, b: f32, c: f32) -> Option<f32> {
    valid_unit_divide(a - b, a - b - b + c)
}

/// `SkQuadCoeff::eval` (`SkGeometry.h` 420-440), for one axis: `fA = p2 - 2 p1 + p0`, `fB = 2 (p1 - p0)`,
/// `fC = p0`.
fn eval_quad_axis(p0: f32, p1: f32, p2: f32, t: f32) -> f32 {
    let a = (p2 - (p1 + p1)) + p0;
    let b = (p1 - p0) + (p1 - p0);
    (a * t + b) * t + p0
}

/// `compute_quad_extremas` (`SkPathPriv.cpp` 1443-1453).
fn quad_extremas(src: &[Point; 3], out: &mut Vec<Point>) {
    let mut ts = Vec::with_capacity(2);
    ts.extend(quad_extrema(src[0].x, src[1].x, src[2].x));
    ts.extend(quad_extrema(src[0].y, src[1].y, src[2].y));
    for t in ts {
        out.push(Point {
            x: eval_quad_axis(src[0].x, src[1].x, src[2].x, t),
            y: eval_quad_axis(src[0].y, src[1].y, src[2].y, t),
        });
    }
    out.push(src[2]);
}

/// `conic_deriv_coeff` and `conic_find_extrema` (`SkGeometry.cpp` 1247-1275): the extreme of one axis of a
/// conic. Only a single root counts: two roots give no extreme.
fn conic_extremum(p0: f32, p1: f32, p2: f32, w: f32) -> Option<f32> {
    let p20 = p2 - p0;
    let p10 = p1 - p0;
    let wp10 = w * p10;
    let coeff = [w * p20 - p20, p20 - 2.0 * wp10, wp10];
    let roots = unit_quad_roots(coeff[0], coeff[1], coeff[2]);
    if roots.len() == 1 {
        Some(roots[0])
    } else {
        None
    }
}

/// `SkConicCoeff::eval` at `t` (`SkGeometry.h` 447-475): the numerator and the denominator are quadratics,
/// and the point is their quotient.
fn eval_conic(src: &[Point; 3], w: f32, t: f32) -> Point {
    let axis = |p0: f32, p1: f32, p2: f32| -> f32 {
        let p1w = p1 * w;
        // numer: fA = p2 - 2 (p1 w) + p0, fB = 2 (p1 w - p0), fC = p0.
        let n_a = (p2 - (p1w + p1w)) + p0;
        let n_b = (p1w - p0) + (p1w - p0);
        let numer = (n_a * t + n_b) * t + p0;
        // denom: fC = 1, fB = 2 (w - 1), fA = -fB.
        let d_b = (w - 1.0) + (w - 1.0);
        let d_a = 0.0 - d_b;
        let denom = (d_a * t + d_b) * t + 1.0;
        numer / denom
    };
    Point {
        x: axis(src[0].x, src[1].x, src[2].x),
        y: axis(src[0].y, src[1].y, src[2].y),
    }
}

/// `compute_conic_extremas` (`SkPathPriv.cpp` 1455-1466).
fn conic_extremas(src: &[Point; 3], w: f32, out: &mut Vec<Point>) {
    let mut ts = Vec::with_capacity(2);
    ts.extend(conic_extremum(src[0].x, src[1].x, src[2].x, w));
    ts.extend(conic_extremum(src[0].y, src[1].y, src[2].y, w));
    for t in ts {
        out.push(eval_conic(src, w, t));
    }
    out.push(src[2]);
}

/// `compute_cubic_extremas` (`SkPathPriv.cpp` 1468-1480): the extremes of the cubic, then its end.
fn cubic_extremas(src: &[Point; 4], out: &mut Vec<Point>) {
    let mut ts = cubic_extrema(src[0].x, src[1].x, src[2].x, src[3].x);
    ts.extend(cubic_extrema(src[0].y, src[1].y, src[2].y, src[3].y));
    for t in ts {
        out.push(eval_cubic(src, t));
    }
    out.push(src[3]);
}

/// The tight box of the verbs as `[left, top, width, height]` (`ComputeTightBounds` and
/// `gfx::SkRectToRectF`), or `None` for a path with no verb.
pub(crate) fn tight_bounds(verbs: &[Verb]) -> Option<[f32; 4]> {
    let first = match verbs.first()? {
        Verb::Move(point) => *point,
        // A path starts with a move.
        _ => return None,
    };
    let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);

    // The last point of the `SkPath` (the start of the curve), and the start of the sub-path, to which a
    // close returns the current point.
    let mut current = first;
    let mut sub_path_start = first;
    let mut extremas: Vec<Point> = Vec::with_capacity(5);
    for verb in verbs {
        extremas.clear();
        match *verb {
            Verb::Move(point) => {
                sub_path_start = point;
                current = point;
                extremas.push(point);
            }
            Verb::Line(point) => {
                current = point;
                extremas.push(point);
            }
            Verb::Quad(control, end) => {
                quad_extremas(&[current, control, end], &mut extremas);
                current = end;
            }
            Verb::Cubic(c1, c2, end) => {
                cubic_extremas(&[current, c1, c2, end], &mut extremas);
                current = end;
            }
            Verb::Conic(control, end, weight) => {
                conic_extremas(&[current, control, end], weight, &mut extremas);
                current = end;
            }
            Verb::Close => current = sub_path_start,
        }
        for point in &extremas {
            left = left.min(point.x);
            top = top.min(point.y);
            right = right.max(point.x);
            bottom = bottom.max(point.y);
        }
    }
    Some(rect_f(left, top, right, bottom))
}

/// `SkRectToRectF` (`ui/gfx/geometry/skia_conversions.cc` 68-72): the `BoundingRect` of the two corners.
/// Its width is `right - left` in float, and when `left + width` does not give `right` back, the width
/// grows to the next float (`ui/gfx/geometry/rect_f.cc` 312-336). That rounding is in the browser's boxes.
fn rect_f(left: f32, top: f32, right: f32, bottom: f32) -> [f32; 4] {
    let mut width = right - left;
    if left + width != right {
        width = next_up(width);
    }
    let mut height = bottom - top;
    if top + height != bottom {
        height = next_up(height);
    }
    [left, top, width, height]
}

/// `std::nextafter(x, +infinity)` for a float, saturating at the largest float as `BoundingRect` does.
fn next_up(x: f32) -> f32 {
    if x.is_infinite() {
        return if x > 0.0 { f32::MAX } else { -f32::MAX };
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    if x > 0.0 {
        f32::from_bits(x.to_bits() + 1)
    } else {
        f32::from_bits(x.to_bits() - 1)
    }
}
