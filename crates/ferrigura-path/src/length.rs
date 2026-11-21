//! Bézier length with the renderer's composite Simpson rule.

use ferrigura_jsmath::hypot;
use wide::f64x4;

const STEPS: usize = 2000;

/// Integrates the speed of a quadratic Bézier with endpoint derivatives `(ax, ay)` and `(bx, by)`.
///
/// Samples are accumulated in scalar order to preserve the renderer's floating-point results.
#[must_use]
pub fn quadratic_length(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let speed = |t: f64| hypot((1.0 - t) * ax + t * bx, (1.0 - t) * ay + t * by);
    // This bound ensures the interpolated components and their magnitudes remain finite.
    // Nonfinite and extreme inputs retain V8 hypot's original special-value behavior.
    if ![ax, ay, bx, by].iter().all(|v| v.abs() <= f64::MAX / 4.0) {
        return scalar_length(speed);
    }
    batched_length(speed, |t| {
        let u = f64x4::ONE - t;
        (u * ax + t * bx, u * ay + t * by)
    })
}

/// Integrates a cubic Bézier using its three successive control-point differences.
///
/// Samples are accumulated in scalar order to preserve the renderer's floating-point results.
#[must_use]
pub fn cubic_length(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> f64 {
    let speed = |t: f64| {
        let u = 1.0 - t;
        hypot(
            3.0 * (u * u * ax + 2.0 * u * t * bx + t * t * cx),
            3.0 * (u * u * ay + 2.0 * u * t * by + t * t * cy),
        )
    };
    // Keep all intermediate components finite; exceptional inputs use V8 hypot directly.
    if ![ax, ay, bx, by, cx, cy]
        .iter()
        .all(|v| v.abs() <= f64::MAX / 32.0)
    {
        return scalar_length(speed);
    }
    batched_length(speed, |t| {
        let u = f64x4::ONE - t;
        (
            3.0 * (u * u * ax + 2.0 * u * t * bx + t * t * cx),
            3.0 * (u * u * ay + 2.0 * u * t * by + t * t * cy),
        )
    })
}

fn batched_length(speed: impl Fn(f64) -> f64, components: impl Fn(f64x4) -> (f64x4, f64x4)) -> f64 {
    let h = 1.0 / STEPS as f64;
    let mut sum = speed(0.0) + speed(1.0);
    for i in (1..STEPS - 3).step_by(4) {
        let t = f64x4::from([i as f64, (i + 1) as f64, (i + 2) as f64, (i + 3) as f64]) * h;
        let (x, y) = components(t);
        let x = x.abs();
        let y = y.abs();
        let max = x.fast_max(y);
        // For a nonzero maximum, its normalized component is exactly one. A zero
        // maximum produces NaN here, which must become the positive zero of V8 hypot.
        let ratio = x.fast_min(y) / max;
        let values = (ratio * ratio + f64x4::ONE).sqrt() * max;
        let values = values.is_nan().select(f64x4::ZERO, values).to_array();
        sum += 4.0 * values[0];
        sum += 2.0 * values[1];
        sum += 4.0 * values[2];
        sum += 2.0 * values[3];
    }
    for i in STEPS - 3..STEPS {
        sum += (if i % 2 == 1 { 4.0 } else { 2.0 }) * speed(i as f64 * h);
    }
    sum * h / 3.0
}

fn scalar_length(speed: impl Fn(f64) -> f64) -> f64 {
    let h = 1.0 / STEPS as f64;
    let mut sum = speed(0.0) + speed(1.0);
    for i in 1..STEPS {
        let weight = if i % 2 == 1 { 4.0 } else { 2.0 };
        sum += weight * speed(i as f64 * h);
    }
    sum * h / 3.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_scalar_for_special_and_finite_derivatives() {
        let values = [
            0.0,
            -0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            0.1,
            -1.0,
            42.0,
            f64::MAX / 4.0,
            f64::MAX,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        for &ax in &values {
            for &ay in &values {
                for &(bx, by) in &[(0.0, 0.0), (-ax, -ay), (ay, ax), (1.0, -1.0)] {
                    let expected =
                        scalar_length(|t| hypot((1.0 - t) * ax + t * bx, (1.0 - t) * ay + t * by));
                    let actual = quadratic_length(ax, ay, bx, by);
                    assert!(
                        actual.to_bits() == expected.to_bits()
                            || (actual.is_nan() && expected.is_nan()),
                        "{ax:?}, {ay:?}, {bx:?}, {by:?}: {actual:?} != {expected:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn cubic_matches_scalar_for_special_and_finite_derivatives() {
        let values = [
            0.0,
            -0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            0.1,
            -1.0,
            42.0,
            f64::MAX / 32.0,
            f64::MAX,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        for &ax in &values {
            for &ay in &values {
                for &(bx, by, cx, cy) in &[
                    (0.0, 0.0, 0.0, 0.0),
                    (-ax, -ay, ax, ay),
                    (ay, ax, -ay, -ax),
                    (1.0, -1.0, 0.1, -0.1),
                ] {
                    let expected = scalar_length(|t| {
                        let u = 1.0 - t;
                        hypot(
                            3.0 * (u * u * ax + 2.0 * u * t * bx + t * t * cx),
                            3.0 * (u * u * ay + 2.0 * u * t * by + t * t * cy),
                        )
                    });
                    let actual = cubic_length(ax, ay, bx, by, cx, cy);
                    assert!(
                        actual.to_bits() == expected.to_bits()
                            || (actual.is_nan() && expected.is_nan()),
                        "{ax:?}, {ay:?}, {bx:?}, {by:?}, {cx:?}, {cy:?}: {actual:?} != {expected:?}"
                    );
                }
            }
        }
    }
}
