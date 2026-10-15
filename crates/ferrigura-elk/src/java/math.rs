//! Numeric conversions as GWT compiles them in elkjs 0.9.3.

/// GWT's `round_int`: clamp to the `int` range, then truncate toward zero.
///
/// This is not `Math.round`. For `x >= 0` it equals `floor(x)`, which is what
/// `java.util.Random` relies on in `nextInt(bound)` for powers of two.
pub fn round_int(x: f64) -> i32 {
    // `as` truncates toward zero and saturates, which is the clamp GWT applies.
    x as i32
}

/// `java.lang.Math.round(double)`: `floor(x + 0.5)`, widened to `i64`.
///
/// Rust's `f64::round` rounds half away from zero, which differs for
/// negative halves (`-2.5` rounds to `-3` in Rust and to `-2` in Java).
pub fn java_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_int_truncates_toward_zero() {
        assert_eq!(round_int(2.9), 2);
        assert_eq!(round_int(-2.9), -2);
        assert_eq!(round_int(1e12), i32::MAX);
        assert_eq!(round_int(-1e12), i32::MIN);
    }

    #[test]
    fn java_round_is_floor_of_x_plus_half() {
        assert_eq!(java_round(2.5), 3);
        assert_eq!(java_round(-2.5), -2);
        assert_eq!(java_round(-2.6), -3);
    }
}
