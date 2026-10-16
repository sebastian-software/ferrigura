//! Numeric conversions as GWT compiles them in elkjs 0.9.3.

/// GWT's `round_int`: clamp to the `int` range, then truncate toward zero.
///
/// This is not `Math.round`. For `x >= 0` it equals `floor(x)`, which is what
/// `java.util.Random` relies on in `nextInt(bound)` for powers of two.
pub fn round_int(x: f64) -> i32 {
    // `as` truncates toward zero and saturates, which is the clamp GWT applies.
    x as i32
}

/// `Math.round(double)` as elkjs runs it: GWT compiles Java's call to JavaScript's
/// `Math.round`, which rounds ties toward +infinity and gives exact results near
/// the edges of the double range.
///
/// `floor(x + 0.5)` is wrong at `0.49999999999999994` (the addition rounds up to
/// 1.0) and above 2^52 (the addition rounds to an even integer), so the value is
/// taken from the floor and the fractional part instead. Non-finite input follows
/// Java's `long` conversion, which saturates; NaN gives zero.
pub fn java_round(x: f64) -> i64 {
    let floor = x.floor();
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    rounded as i64
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
    fn java_round_rounds_ties_toward_positive_infinity() {
        assert_eq!(java_round(2.5), 3);
        assert_eq!(java_round(-2.5), -2);
        assert_eq!(java_round(-2.6), -3);
        assert_eq!(java_round(0.5), 1);
        assert_eq!(java_round(-0.5), 0);
    }

    #[test]
    fn java_round_is_exact_just_below_one_half() {
        // `x + 0.5` would round up to 1.0 here, but Math.round gives 0.
        assert_eq!(java_round(0.49999999999999994), 0);
    }

    #[test]
    fn java_round_is_exact_above_two_to_the_52() {
        // `x + 0.5` would round to an even integer here; the value is already integral.
        let x = 4_503_599_627_370_497.0_f64; // 2^52 + 1
        assert_eq!(java_round(x), 4_503_599_627_370_497);
    }

    #[test]
    fn java_round_saturates_and_maps_nan_to_zero() {
        assert_eq!(java_round(f64::NAN), 0);
        assert_eq!(java_round(f64::INFINITY), i64::MAX);
        assert_eq!(java_round(f64::NEG_INFINITY), i64::MIN);
    }
}
