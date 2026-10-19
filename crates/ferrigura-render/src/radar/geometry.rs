//! The polar arithmetic of the radar diagram: the angles of the axes and of the curve points, the
//! scale of a value, the closed curve through the points, and the number of ticks, as
//! `packages/mermaid/src/diagrams/radar/renderer.ts` computes them. The reference does its arithmetic
//! in the order written, so each function here keeps that order: `(2 * i * PI) / n - PI / 2` is
//! `((2 * i) * PI) / n - PI / 2` in both.
//!
//! The sines and cosines are `ferrigura_jsmath`'s, which match V8's values
//! (`docs-port/P2-RENDERERS.md`, the math of the reference).

use std::f64::consts::PI;

use crate::svg::number;

/// A point of a curve, in the diagram's own coordinates (`renderer.ts` 154-157).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// The horizontal coordinate.
    pub x: f64,
    /// The vertical coordinate.
    pub y: f64,
}

/// The angle of the axis `index` of `count` axes, measured from twelve o'clock: the first axis points
/// up (`renderer.ts` 105). The graticule polygon's corners take the same angles (`renderer.ts` 84).
#[must_use]
pub fn axis_angle(index: usize, count: usize) -> f64 {
    (2.0 * index as f64 * PI) / count as f64 - PI / 2.0
}

/// The angle of the point `index` of a curve with `count` entries (`renderer.ts` 154). The reference
/// writes the factors in another order than the axes', and the two can differ in the last bit, so both
/// are kept as written.
#[must_use]
pub fn point_angle(index: usize, count: usize) -> f64 {
    (2.0 * PI * index as f64) / count as f64 - PI / 2.0
}

/// The number of ticks that are drawn: the reference's `for (let i = 0; i < ticks; i++)` runs for each
/// whole `i` below `ticks`, so a fraction draws one more circle (`renderer.ts` 73 and 80; 2.5 gives three), and
/// a zero or a negative count draws none.
#[must_use]
pub fn tick_count(ticks: f64) -> usize {
    if ticks.is_nan() || ticks <= 0.0 {
        0
    } else {
        ticks.ceil() as usize
    }
}

/// `relativeRadius` (`renderer.ts` 175-182): the value clipped to the scale, as a distance from the
/// centre. A maximum equal to the minimum divides by zero, and the result is `NaN`, as the reference's
/// is (`docs-port/radar/NOTES.md`, section 6.11, item 1).
#[must_use]
pub fn relative_radius(value: f64, min: f64, max: f64, radius: f64) -> f64 {
    let clipped = js_min(js_max(value, min), max);
    (radius * (clipped - min)) / (max - min)
}

/// `closedRoundCurve` (`renderer.ts` 185-207): a closed Catmull-Rom curve through the points, as cubic
/// Bézier segments. Each segment from `p1` to `p2` has its control points `p1 + (p2 - p0) * tension`
/// and `p2 - (p3 - p1) * tension`, with `p0` and `p3` the neighbours, cyclically. A tension of 0 gives
/// straight segments. The points are the curve's, one for each axis; an empty list gives an empty path.
#[must_use]
pub fn closed_round_curve(points: &[Point], tension: f64) -> String {
    let Some(first) = points.first() else {
        return String::new();
    };
    let count = points.len();
    let mut path = format!("M{},{}", number(first.x), number(first.y));
    for index in 0..count {
        let p0 = points[(index + count - 1) % count];
        let p1 = points[index];
        let p2 = points[(index + 1) % count];
        let p3 = points[(index + 2) % count];
        let cp1x = p1.x + (p2.x - p0.x) * tension;
        let cp1y = p1.y + (p2.y - p0.y) * tension;
        let cp2x = p2.x - (p3.x - p1.x) * tension;
        let cp2y = p2.y - (p3.y - p1.y) * tension;
        path.push_str(&format!(
            " C{},{} {},{} {},{}",
            number(cp1x),
            number(cp1y),
            number(cp2x),
            number(cp2y),
            number(p2.x),
            number(p2.y)
        ));
    }
    path.push_str(" Z");
    path
}

/// `Math.max` of two numbers: `NaN` when either is `NaN`, where Rust's `f64::max` ignores a `NaN`.
#[must_use]
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// `Math.min` of two numbers: `NaN` when either is `NaN`, where Rust's `f64::min` ignores a `NaN`.
#[must_use]
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}
