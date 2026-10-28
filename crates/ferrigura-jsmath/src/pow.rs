//! `Math.pow(x, y)` as V8 computes it.
//!
//! V8 has two implementations. `v8::internal::math::pow` (`src/numbers/ieee754.cc`) calls
//! `std::pow` by default (the flag `--use-std-math-pow` is on), so V8's result is the platform's
//! `pow`. Only with the flag off does V8 use its own fdlibm-derived `legacy::pow`, which is not
//! ported here. This function therefore applies V8's special cases and calls the platform's
//! `pow`, so it agrees with V8 on the same platform, and not necessarily on another one.

/// `Math.pow(x, y)`: V8's special cases, and otherwise the platform's `pow`.
#[must_use]
pub fn pow(x: f64, y: f64) -> f64 {
    if y.is_nan() {
        // ECMAScript: an exponent of NaN gives NaN, even for a base of 1.
        return f64::NAN;
    }
    if y.is_infinite() && (x == 1.0 || x == -1.0) {
        // ECMAScript: |base| = 1 with an infinite exponent gives NaN, where C gives 1.
        return f64::NAN;
    }
    if y == 2.0 {
        // V8 computes x ** 2 as x * x, as optimizing compilers do.
        return x * x;
    }
    if y == 0.5 {
        // V8 computes x ** 0.5 as sqrt(x), except that -Infinity gives +Infinity. The `+ 0.0`
        // turns -0 into +0, so that -0 ** 0.5 is +0.
        if x.is_infinite() {
            return f64::INFINITY;
        }
        return (x + 0.0).sqrt();
    }
    x.powf(y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_values() {
        assert_eq!(pow(2.0, 10.0), 1024.0);
        assert!(pow(1.0, f64::NAN).is_nan());
        assert!(pow(1.0, f64::INFINITY).is_nan());
        assert!(pow(-1.0, f64::NEG_INFINITY).is_nan());
        assert_eq!(pow(f64::NAN, 0.0), 1.0);
        assert_eq!(pow(-0.0, 0.5).to_bits(), 0.0f64.to_bits());
        assert_eq!(pow(f64::NEG_INFINITY, 0.5), f64::INFINITY);
        assert_eq!(pow(3.0, 2.0), 9.0);
    }
}
