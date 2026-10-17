//! The reductions of `d3-array` 2.12.1 that the layouts use, with their accessor form: `min` and
//! `max` skip the values that are not numbers (NaN) and give `undefined` (here `None`) when no
//! value is left, and `sum` adds the values that are truthy, so zero and NaN do not count
//! (`min.js`, `max.js`, `sum.js`).

/// `min(values, valueof)`: the smallest value that is not NaN, or `None`.
#[must_use]
pub fn min_by<T>(items: &[T], value: impl Fn(&T) -> f64) -> Option<f64> {
    let mut min: Option<f64> = None;
    for item in items {
        let v = value(item);
        // `min > value`, or while nothing is taken yet, `value >= value` (which is false for NaN).
        let take = match min {
            Some(current) => current > v,
            None => !v.is_nan(),
        };
        if take {
            min = Some(v);
        }
    }
    min
}

/// `max(values, valueof)`: the largest value that is not NaN, or `None`.
#[must_use]
pub fn max_by<T>(items: &[T], value: impl Fn(&T) -> f64) -> Option<f64> {
    let mut max: Option<f64> = None;
    for item in items {
        let v = value(item);
        let take = match max {
            Some(current) => current < v,
            None => !v.is_nan(),
        };
        if take {
            max = Some(v);
        }
    }
    max
}

/// `sum(values, valueof)`: the sum of the values that are neither zero nor NaN, from `0`.
/// Negative values are added; a sum of no values is `0`.
#[must_use]
pub fn sum_by<T>(items: &[T], value: impl Fn(&T) -> f64) -> f64 {
    let mut sum = 0.0;
    for item in items {
        let v = value(item);
        if v != 0.0 && !v.is_nan() {
            sum += v;
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_and_max_skip_nan_and_are_none_without_values() {
        let values = [f64::NAN, 3.0, f64::NAN, -1.0];
        assert_eq!(min_by(&values, |v| *v), Some(-1.0));
        assert_eq!(max_by(&values, |v| *v), Some(3.0));
        let nothing = [f64::NAN];
        assert_eq!(min_by(&nothing, |v| *v), None);
        assert_eq!(max_by(&nothing, |v| *v), None);
    }

    #[test]
    fn min_and_max_keep_infinities() {
        let values = [f64::INFINITY, 2.0];
        assert_eq!(min_by(&values, |v| *v), Some(2.0));
        assert_eq!(max_by(&values, |v| *v), Some(f64::INFINITY));
    }

    #[test]
    fn sum_skips_zero_and_nan_but_keeps_negatives() {
        let values = [0.0, f64::NAN, -2.0, 5.0];
        assert_eq!(sum_by(&values, |v| *v), 3.0);
        assert_eq!(sum_by::<f64>(&[], |v| *v), 0.0);
    }
}
