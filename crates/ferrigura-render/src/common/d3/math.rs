//! JavaScript's `Math.max` and `Math.min` with two arguments, as the reference's arithmetic calls
//! them. Unlike Rust's `f64::max` and `f64::min`, they give NaN when an argument is NaN, and they
//! order the zeros: `+0` is above `-0`.

/// `Math.max(a, b)`.
#[must_use]
pub fn math_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == b {
        // Equal values, or the two zeros: `+0` is the larger one.
        return if a.is_sign_negative() { b } else { a };
    }
    if a > b { a } else { b }
}

/// `Math.min(a, b)`.
#[must_use]
pub fn math_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == b {
        // Equal values, or the two zeros: `-0` is the smaller one.
        return if a.is_sign_negative() { a } else { b };
    }
    if a < b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nan_argument_gives_nan() {
        assert!(math_max(1.0, f64::NAN).is_nan());
        assert!(math_min(f64::NAN, 1.0).is_nan());
    }

    #[test]
    fn the_zeros_are_ordered() {
        assert!(math_max(-0.0, 0.0).is_sign_positive());
        assert!(math_min(0.0, -0.0).is_sign_negative());
    }

    #[test]
    fn the_larger_or_smaller_argument_is_taken() {
        assert_eq!(math_max(1.0, -f64::INFINITY), 1.0);
        assert_eq!(math_min(f64::INFINITY, 3.0), 3.0);
    }
}
