//! The time scale of d3-scale 4.0.2 (`scaleTime().domain(...).rangeRound(...)`), in UTC.
//!
//! The reference maps a time to `Math.round(a * (1 - t) + b * t)` with `t` the position of the time
//! in the domain (`d3-interpolate/src/round.js`, `d3-scale/src/continuous.js`). Every output is
//! therefore an integer, and the text of the bars is placed at half-integers, as the reference
//! writes them.

use super::time::{TickCount, ticks};
use crate::common::d3::{js_round, normalize};

/// A time scale with a domain of two times and a range from 0 to `range_max`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TimeScale {
    domain: [f64; 2],
    range_max: f64,
}

impl TimeScale {
    /// `scaleTime().domain(domain).rangeRound([0, range_max])`. A missing time is NaN.
    pub(crate) fn new(domain: [f64; 2], range_max: f64) -> Self {
        Self { domain, range_max }
    }

    /// The domain, as given.
    pub(crate) fn domain(&self) -> [f64; 2] {
        self.domain
    }

    /// The end of the range, `scale.range()[1]`; the range starts at 0.
    pub(crate) fn range_max(&self) -> f64 {
        self.range_max
    }

    /// `scale(time)`: the position of the time in the range, rounded to a whole number.
    pub(crate) fn apply(&self, time: f64) -> f64 {
        let [d0, d1] = self.domain;
        if d1 < d0 {
            // bimap with a reversed domain: the range is interpolated from its end.
            round_interpolate(self.range_max, 0.0, normalize(d1, d0, time))
        } else {
            round_interpolate(0.0, self.range_max, normalize(d0, d1, time))
        }
    }

    /// `scale.ticks(count)`: the ticks of the domain, as `d3-scale/src/time.js` takes them.
    pub(crate) fn ticks(&self, count: TickCount) -> Vec<f64> {
        let [start, stop] = self.domain;
        ticks(start, stop, count)
    }
}

/// `interpolateRound(a, b)(t)`: `Math.round(a * (1 - t) + b * t)`, with JavaScript's rounding (a tie
/// goes towards plus infinity).
fn round_interpolate(a: f64, b: f64, t: f64) -> f64 {
    js_round(a * (1.0 - t) + b * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_maps_to_its_rounded_position() {
        // A domain of 0 to 10 over a range of 1050: 2.5 maps to 262.5, which rounds up.
        let scale = TimeScale::new([0.0, 10.0], 1050.0);
        assert_eq!(scale.apply(0.0), 0.0);
        assert_eq!(scale.apply(10.0), 1050.0);
        assert_eq!(scale.apply(2.5), 263.0);
        assert_eq!(scale.apply(2.0), 210.0);
    }

    #[test]
    fn a_tie_rounds_towards_plus_infinity() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(-2.6), -3.0);
    }

    #[test]
    fn a_reversed_domain_maps_the_range_backwards() {
        let scale = TimeScale::new([10.0, 0.0], 100.0);
        assert_eq!(scale.apply(10.0), 0.0);
        assert_eq!(scale.apply(0.0), 100.0);
    }

    #[test]
    fn a_single_instant_maps_to_the_middle() {
        let scale = TimeScale::new([5.0, 5.0], 100.0);
        assert_eq!(scale.apply(5.0), 50.0);
        assert!(TimeScale::new([f64::NAN, 5.0], 100.0).apply(5.0).is_nan());
    }
}
