//! `Math.hypot(x, y)` as V8 computes it.
//!
//! V8 implements `Math.hypot` in Torque (`src/builtins/math.tq`, `FastMathHypot`), not with
//! fdlibm. With two arguments the fast path scales both by the larger magnitude and takes the root
//! of the plain sum of squares, with no compensation. The compensated sum that V8 uses for more
//! than three arguments (its slow path) is a different formula and is not this function.

/// `Math.hypot(x, y)`, as V8's two-argument fast path computes it.
///
/// An infinite argument gives `Infinity` even when the other is `NaN`, as in V8.
#[must_use]
pub fn hypot(x: f64, y: f64) -> f64 {
    let a = x.abs();
    let b = y.abs();
    if a == f64::INFINITY || b == f64::INFINITY {
        return f64::INFINITY;
    }
    // V8 takes the maximum with Float64Max, which is NaN if either argument is.
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    let max = if a > b { a } else { b };
    if max == 0.0 {
        return 0.0;
    }
    ((a / max) * (a / max) + (b / max) * (b / max)).sqrt() * max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_values() {
        assert_eq!(hypot(3.0, 4.0), 5.0);
        assert_eq!(hypot(0.0, -0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(hypot(f64::NAN, f64::INFINITY), f64::INFINITY);
        assert!(hypot(f64::NAN, 1.0).is_nan());
        assert_eq!(hypot(-5.0, 0.0), 5.0);
    }
}
