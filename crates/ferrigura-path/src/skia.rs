//! Skia's `SkPathBuilder::arcTo` (`core/SkPathBuilder.cpp` 563-675) and the `SkMatrix` operations it uses
//! (`core/SkMatrix.cpp`): an SVG arc becomes conics, each at most a third of a turn, computed in `float`
//! with the scalar helpers of `include/core/SkScalar.h` and `include/private/SkFloatingPoint.h`.

use crate::builder::Verb;
use crate::segments::Point;

/// `SK_ScalarPI`: `(float)M_PI`.
const PI: f32 = std::f32::consts::PI;
/// `SK_ScalarNearlyZero`, the tolerance of `SkScalarNearlyZero`.
const NEARLY_ZERO: f32 = 1.0 / 4096.0;
/// `SK_ScalarSinCosNearlyZero`: the sine and cosine below it snap to zero.
const SIN_COS_NEARLY_ZERO: f32 = 1.0 / 65536.0;

/// The sine and the cosine of one angle, each the float nearest to the exact value. `SkMatrix::setRotate`
/// and the arc loop take `SkScalarSinSnapToZero` and `SkScalarCosSnapToZero` of the same radians, which
/// Chromium on macOS computes with one `__sincosf_stret` call. That call is not correctly rounded: on 5.9
/// million floats in [-8, 8] it is off the nearest float in 0.03 percent of them (measured), and its
/// algorithm is not published, so the port takes the correctly rounded values, the same on every host
/// (`ferrigura_jsmath`'s correctly rounded `sin` and `cos` in double, rounded to float; no input of that
/// measurement lies near a midpoint, so the double rounding does not change a result). Two boxes of the
/// corpus differ for it (`tests/corpus/known-differences.json`). The separate `sinf` and `cosf` of the
/// macOS libm differ from that call more often: with them, 85 boxes of the corpus differed.
fn sin_cos(radians: f32) -> (f32, f32) {
    let x = f64::from(radians);
    (
        ferrigura_jsmath::sin(x) as f32,
        ferrigura_jsmath::cos(x) as f32,
    )
}

/// `SkScalarSinSnapToZero` and `SkScalarCosSnapToZero` (`SkScalar.h` 114-122): a sine or cosine of
/// magnitude at most `SK_ScalarSinCosNearlyZero` is zero.
fn sin_cos_snap(radians: f32) -> (f32, f32) {
    let (sin, cos) = sin_cos(radians);
    (snap(sin), snap(cos))
}

fn snap(v: f32) -> f32 {
    if v.abs() <= SIN_COS_NEARLY_ZERO {
        0.0
    } else {
        v
    }
}

/// `SkScalarRoundToScalar` (`sk_float_round`, `SkFloatingPoint.h` 38 and 126): the rounding is in double,
/// `(float)std::floor((double)x + 0.5)`.
fn round_scalar(x: f32) -> f32 {
    (f64::from(x) + 0.5).floor() as f32
}

/// `muladdmul` (`SkMatrix.cpp` 603-605): `a * b + c * d` in double, rounded to float once.
fn muladdmul(a: f32, b: f32, c: f32, d: f32) -> f32 {
    (f64::from(a) * f64::from(b) + f64::from(c) * f64::from(d)) as f32
}

/// A 2x3 matrix in `SkMatrix`'s storage: `[scaleX, skewX, transX, skewY, scaleY, transY]`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Matrix {
    m: [f32; 6],
}

impl Matrix {
    fn identity() -> Self {
        Self {
            m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        }
    }

    fn is_identity(&self) -> bool {
        self.m == Self::identity().m
    }

    /// `setScale(sx, sy)`.
    fn set_scale(&mut self, sx: f32, sy: f32) {
        self.m = [sx, 0.0, 0.0, 0.0, sy, 0.0];
    }

    /// `setSinCos(sin, cos)` (`SkMatrix.cpp` 440-452): the rotation about the origin.
    fn set_sin_cos(&mut self, sin: f32, cos: f32) {
        self.m = [cos, -sin, 0.0, sin, cos, 0.0];
    }

    /// `setRotate(degrees)` (`SkMatrix.cpp` 461-464): the radians are `degrees * (π / 180)`.
    fn set_rotate(&mut self, degrees: f32) {
        let radians = degrees * (PI / 180.0);
        let (sin, cos) = sin_cos_snap(radians);
        self.set_sin_cos(sin, cos);
    }

    /// `preScale(sx, sy)` (`SkMatrix.cpp` 329-345): the columns are scaled, the translation is not.
    fn pre_scale(&mut self, sx: f32, sy: f32) {
        if sx == 1.0 && sy == 1.0 {
            return;
        }
        self.m[0] *= sx;
        self.m[3] *= sx;
        self.m[1] *= sy;
        self.m[4] *= sy;
    }

    /// `preRotate(degrees)` (`SkMatrix.cpp` 472-478): the matrix times the rotation.
    fn pre_rotate(&mut self, degrees: f32) {
        let mut rotation = Matrix::identity();
        rotation.set_rotate(degrees);
        if !rotation.is_identity() {
            *self = concat(*self, rotation);
        }
    }

    /// `mapPoint`: `Affine_vpts` (`SkMatrix.cpp` 988-1012), each coordinate as `x * scale + y * skew +
    /// translation`, in float.
    fn map(&self, p: Point) -> Point {
        let m = self.m;
        Point {
            x: (p.x * m[0] + p.y * m[1]) + m[2],
            y: (p.y * m[4] + p.x * m[3]) + m[5],
        }
    }
}

/// `setConcat(a, b)` (`SkMatrix.cpp` 615-680): the product `a * b`. A matrix without a skew stays
/// scale and translation; otherwise the product is taken in double (`muladdmul`).
fn concat(a: Matrix, b: Matrix) -> Matrix {
    if a.is_identity() {
        return b;
    }
    if b.is_identity() {
        return a;
    }
    let (am, bm) = (a.m, b.m);
    if am[1] == 0.0 && am[3] == 0.0 && bm[1] == 0.0 && bm[3] == 0.0 {
        return Matrix {
            m: [
                am[0] * bm[0],
                0.0,
                am[0] * bm[2] + am[2],
                0.0,
                am[4] * bm[4],
                am[4] * bm[5] + am[5],
            ],
        };
    }
    Matrix {
        m: [
            muladdmul(am[0], bm[0], am[1], bm[3]),
            muladdmul(am[0], bm[1], am[1], bm[4]),
            muladdmul(am[0], bm[2], am[1], bm[5]) + am[2],
            muladdmul(am[3], bm[0], am[4], bm[3]),
            muladdmul(am[3], bm[1], am[4], bm[4]),
            muladdmul(am[3], bm[2], am[4], bm[5]) + am[5],
        ],
    }
}

/// `SkScalarIsInt`-like: whether the float is a whole number (`scalar == SkScalarFloorToScalar(scalar)`).
fn is_integer(x: f32) -> bool {
    x == x.floor()
}

/// The conic of `SkPathBuilder::conicTo` (`SkPathBuilder.cpp` 202-215): a conic with a weight of one is a
/// quadratic, and a weight that is not positive is a line.
fn conic_verb(control: Point, end: Point, weight: f32) -> Verb {
    if weight <= 0.0 {
        Verb::Line(end)
    } else if weight == 1.0 {
        Verb::Quad(control, end)
    } else {
        Verb::Conic(control, end, weight)
    }
}

/// `SkPathBuilder::arcTo(rad, angle, arcLarge, arcSweep, endPt)` (`SkPathBuilder.cpp` 563-675): the arc
/// from `from` (the last point of the path) to `end`, as the verbs it adds. `sweep` is the SVG flag, and
/// `sweep == true` is clockwise (`kCW`), as `PathBuilder::ArcTo` passes it.
pub(crate) fn arc_to(
    from: Point,
    end: Point,
    rx0: f32,
    ry0: f32,
    angle: f32,
    large: bool,
    sweep: bool,
) -> Vec<Verb> {
    // A radius of zero, or a zero-length arc, is a line (`lineTo(endPt)`).
    if rx0 == 0.0 || ry0 == 0.0 || from == end {
        return vec![Verb::Line(end)];
    }
    let mut rx = rx0.abs();
    let mut ry = ry0.abs();

    // The midpoint, rotated back by the angle and scaled by the radii.
    let mid = Point {
        x: (from.x - end.x) * 0.5,
        y: (from.y - end.y) * 0.5,
    };
    let mut transform = Matrix::identity();
    transform.set_rotate(-angle);
    let transformed = transform.map(mid);
    let square_rx = rx * rx;
    let square_ry = ry * ry;
    let square_x = transformed.x * transformed.x;
    let square_y = transformed.y * transformed.y;

    // The radii are scaled up when they are too small for the chord.
    let mut radii_scale = square_x / square_rx + square_y / square_ry;
    if radii_scale > 1.0 {
        radii_scale = radii_scale.sqrt();
        rx *= radii_scale;
        ry *= radii_scale;
    }

    // The unit circle: scale by 1/r, then rotate by -angle.
    transform.set_scale(1.0 / rx, 1.0 / ry);
    transform.pre_rotate(-angle);
    let mut unit = [transform.map(from), transform.map(end)];
    let delta = Point {
        x: unit[1].x - unit[0].x,
        y: unit[1].y - unit[0].y,
    };

    let d = delta.x * delta.x + delta.y * delta.y;
    let scale_factor_squared = {
        let v = 1.0 / d - 0.25;
        if v < 0.0 { 0.0 } else { v }
    };
    let mut scale_factor = scale_factor_squared.sqrt();
    // `(arcSweep == kCCW) != arcLarge`: the flipped test of the original implementation.
    if (!sweep) != large {
        scale_factor = -scale_factor;
    }
    let delta = Point {
        x: delta.x * scale_factor,
        y: delta.y * scale_factor,
    };
    let mut centre = Point {
        x: (unit[0].x + unit[1].x) * 0.5,
        y: (unit[0].y + unit[1].y) * 0.5,
    };
    centre.x += -delta.y;
    centre.y += delta.x;
    for point in &mut unit {
        point.x -= centre.x;
        point.y -= centre.y;
    }

    let theta1 = unit[0].y.atan2(unit[0].x);
    let theta2 = unit[1].y.atan2(unit[1].x);
    let mut theta_arc = theta2 - theta1;
    if theta_arc < 0.0 && sweep {
        theta_arc += PI * 2.0;
    } else if theta_arc > 0.0 && !sweep {
        theta_arc -= PI * 2.0;
    }
    // A tiny sweep is a line: the arithmetic below would go wrong (skbug.com/40040578).
    if theta_arc.abs() < PI / (1000.0 * 1000.0) {
        return vec![Verb::Line(end)];
    }

    transform.set_rotate(angle);
    transform.pre_scale(rx, ry);

    // At most a third of a turn per conic, so a quarter circle is at most two.
    let segments = (theta_arc / (2.0 * PI / 3.0)).abs().ceil() as i32;
    let theta_width = theta_arc / segments as f32;
    let t = (0.5 * theta_width).tan();
    if !t.is_finite() {
        return Vec::new();
    }
    let mut start_theta = theta1;
    let w = (0.5 + theta_width.cos() * 0.5).sqrt();
    // The conics of an arc on whole numbers are placed on whole numbers too (the rounding of the
    // arc's own arithmetic would move them off the marks).
    let expect_integers = (PI / 2.0 - theta_width.abs()).abs() <= NEARLY_ZERO
        && is_integer(rx)
        && is_integer(ry)
        && is_integer(end.x)
        && is_integer(end.y);

    let mut verbs = Vec::with_capacity(segments as usize);
    for _ in 0..segments {
        let end_theta = start_theta + theta_width;
        let (sin_end, cos_end) = sin_cos_snap(end_theta);
        let mut on_circle = Point {
            x: cos_end,
            y: sin_end,
        };
        on_circle.x += centre.x;
        on_circle.y += centre.y;
        let control = Point {
            x: on_circle.x + t * sin_end,
            y: on_circle.y + -t * cos_end,
        };
        let mut mapped = [transform.map(control), transform.map(on_circle)];
        if expect_integers {
            for point in &mut mapped {
                point.x = round_scalar(point.x);
                point.y = round_scalar(point.y);
            }
        }
        verbs.push(conic_verb(mapped[0], mapped[1], w));
        start_theta = end_theta;
    }

    // The last point of the arc is the end point itself, whatever the rounding of the conics.
    if let Some(Verb::Conic(control, _, weight)) = verbs.last().copied() {
        let last = verbs.len() - 1;
        verbs[last] = Verb::Conic(control, end, weight);
    } else if let Some(Verb::Quad(control, _)) = verbs.last().copied() {
        let last = verbs.len() - 1;
        verbs[last] = Verb::Quad(control, end);
    }
    verbs
}
