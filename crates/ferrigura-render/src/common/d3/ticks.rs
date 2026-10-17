//! `ticks(start, stop, count)` of `d3-array` 3.2.4 (`ticks.js`): the round values of a linear
//! domain, about `count` of them.

use crate::common::d3::js_round;
use ferrigura_jsmath::pow;

/// `tickSpec(start, stop, count)`: the first and last tick index and the increment, from the step
/// of the range rounded to 1, 2 or 5 times a power of ten.
fn tick_spec(start: f64, stop: f64, count: f64) -> (f64, f64, f64) {
    // `Math.sqrt(50)`, `Math.sqrt(10)` and `Math.sqrt(2)`: the thresholds of the factors.
    let (e10, e5, e2) = (50.0_f64.sqrt(), 10.0_f64.sqrt(), 2.0_f64.sqrt());
    let step = (stop - start) / count.max(0.0);
    // `Math.floor(Math.log10(step))`: the power only matters where a step is a power of ten, and
    // there both libraries give the exact integer.
    let power = step.log10().floor();
    let error = step / pow(10.0, power);
    let factor = if error >= e10 {
        10.0
    } else if error >= e5 {
        5.0
    } else if error >= e2 {
        2.0
    } else {
        1.0
    };
    let (mut i1, mut i2, mut inc);
    if power < 0.0 {
        inc = pow(10.0, -power) / factor;
        i1 = js_round(start * inc);
        i2 = js_round(stop * inc);
        if i1 / inc < start {
            i1 += 1.0;
        }
        if i2 / inc > stop {
            i2 -= 1.0;
        }
        inc = -inc;
    } else {
        inc = pow(10.0, power) * factor;
        i1 = js_round(start / inc);
        i2 = js_round(stop / inc);
        if i1 * inc < start {
            i1 += 1.0;
        }
        if i2 * inc > stop {
            i2 -= 1.0;
        }
    }
    if i2 < i1 && (0.5..2.0).contains(&count) {
        return tick_spec(start, stop, count * 2.0);
    }
    (i1, i2, inc)
}

/// `ticks(start, stop, count)`: the tick values of the domain from `start` to `stop`. A reversed
/// domain gives the same values in descending order.
#[must_use]
pub fn ticks(start: f64, stop: f64, count: f64) -> Vec<f64> {
    // `!(count > 0)`: a NaN count gives no ticks too.
    if count.is_nan() || count <= 0.0 {
        return Vec::new();
    }
    if start == stop {
        return vec![start];
    }
    let reverse = stop < start;
    let (i1, i2, inc) = if reverse {
        tick_spec(stop, start, count)
    } else {
        tick_spec(start, stop, count)
    };
    // `!(i2 >= i1)`: NaN bounds give no ticks.
    if i1.is_nan() || i2.is_nan() || i2 < i1 {
        return Vec::new();
    }
    let n = (i2 - i1 + 1.0) as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let index = i as f64;
        let value = if reverse {
            if inc < 0.0 {
                (i2 - index) / -inc
            } else {
                (i2 - index) * inc
            }
        } else if inc < 0.0 {
            (i1 + index) / -inc
        } else {
            (i1 + index) * inc
        };
        out.push(value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_of_zero_to_ten_has_eleven_ticks() {
        assert_eq!(
            ticks(0.0, 10.0, 10.0),
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
        );
    }

    #[test]
    fn a_reversed_domain_gives_the_ticks_in_descending_order() {
        assert_eq!(ticks(10.0, 0.0, 10.0).first(), Some(&10.0));
        assert_eq!(ticks(10.0, 0.0, 10.0).last(), Some(&0.0));
    }

    #[test]
    fn the_ticks_of_a_domain_match_d3() {
        // The probe of d3-array 3.2.4 with ten as the count.
        assert_eq!(
            ticks(-2.4, 3.5, 10.0),
            vec![
                -2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5
            ]
        );
        assert_eq!(ticks(0.0, 0.3, 10.0).len(), 16);
        assert_eq!(ticks(1e-7, 3e-7, 10.0).len(), 11);
        assert_eq!(ticks(0.5, 0.75, 10.0).len(), 13);
        assert_eq!(ticks(1e-7, 3e-7, 10.0)[1], 1.2e-7);
    }

    #[test]
    fn a_single_value_domain_gives_that_value() {
        assert_eq!(ticks(5.0, 5.0, 10.0), vec![5.0]);
    }

    #[test]
    fn a_count_that_is_not_positive_gives_no_ticks() {
        assert!(ticks(0.0, 10.0, 0.0).is_empty());
    }
}
