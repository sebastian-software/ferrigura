//! The scales of `d3-scale` 4.0.2 that the custom renderers use: `scaleLinear` over two values
//! (`continuous.js`, `bimap`) and `scaleBand` over an ordinal domain (`band.js`, `ordinal.js`).
//!
//! The operations keep the reference's order, because the last digit of a position is part of the
//! geometry the checks compare.

use std::collections::HashMap;

use super::number::interpolate_number;
use super::ticks::ticks;

/// `normalize(a, b)(x)` of `continuous.js`: the position of `x` between `a` and `b`, as the
/// parameter `t` of the interpolation. A zero-length domain maps every value to the middle (0.5),
/// and a NaN domain gives NaN.
#[must_use]
pub fn normalize(a: f64, b: f64, x: f64) -> f64 {
    let span = b - a;
    if span == 0.0 {
        0.5
    } else if span.is_nan() {
        f64::NAN
    } else {
        (x - a) / span
    }
}

/// `scaleLinear().domain(domain).range(range)` with two values on each side, as `bimap` maps them:
/// a descending domain interpolates the range from its end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearScale {
    domain: [f64; 2],
    range: [f64; 2],
}

impl LinearScale {
    /// A scale from `domain` to `range`.
    #[must_use]
    pub fn new(domain: [f64; 2], range: [f64; 2]) -> Self {
        Self { domain, range }
    }

    /// The domain, as given.
    #[must_use]
    pub fn domain(&self) -> [f64; 2] {
        self.domain
    }

    /// `scale(x)`: the position of `x` in the range. A value that is not a number (`null`,
    /// `undefined`, NaN) gives `None`, which d3 returns as `undefined`.
    #[must_use]
    pub fn apply(&self, x: f64) -> Option<f64> {
        if x.is_nan() {
            return None;
        }
        let [d0, d1] = self.domain;
        let [r0, r1] = self.range;
        Some(if d1 < d0 {
            interpolate_number(r1, r0, normalize(d1, d0, x))
        } else {
            interpolate_number(r0, r1, normalize(d0, d1, x))
        })
    }

    /// `scale.ticks(count)` of `linearish`: the ticks of the domain, from its first value to its
    /// last. A reversed domain gives the ticks in descending order.
    #[must_use]
    pub fn ticks(&self, count: f64) -> Vec<f64> {
        ticks(self.domain[0], self.domain[1], count)
    }
}

/// `scaleBand().domain(domain).range(range)` with the padding and the alignment that the reference
/// sets. A value that repeats takes the first place it has (`ordinal.js`), and `n` counts the
/// distinct values, so a repeated value leaves its band empty.
#[derive(Debug, Clone, PartialEq)]
pub struct BandScale {
    domain: Vec<String>,
    index: HashMap<String, usize>,
    range: [f64; 2],
    padding_inner: f64,
    padding_outer: f64,
    align: f64,
    values: Vec<f64>,
}

impl BandScale {
    /// A band scale over `domain` in `range`, with d3's defaults: no padding and the bands aligned
    /// at the middle. Set the padding with [`BandScale::padding_inner`] and
    /// [`BandScale::padding_outer`].
    #[must_use]
    pub fn new<S: AsRef<str>>(domain: &[S], range: [f64; 2]) -> Self {
        let mut unique = Vec::new();
        let mut index = HashMap::new();
        for value in domain {
            let value = value.as_ref();
            if !index.contains_key(value) {
                index.insert(value.to_owned(), unique.len());
                unique.push(value.to_owned());
            }
        }
        let mut scale = Self {
            domain: unique,
            index,
            range,
            padding_inner: 0.0,
            padding_outer: 0.0,
            align: 0.5,
            values: Vec::new(),
        };
        scale.rescale();
        scale
    }

    /// `paddingInner(p)`: the share of each step left empty between two bands, at most 1.
    #[must_use]
    pub fn padding_inner(mut self, padding: f64) -> Self {
        self.padding_inner = padding.min(1.0);
        self.rescale();
        self
    }

    /// `paddingOuter(p)`: the empty space at each end, in steps.
    #[must_use]
    pub fn padding_outer(mut self, padding: f64) -> Self {
        self.padding_outer = padding;
        self.rescale();
        self
    }

    /// `align(a)`: where the bands sit in the space that the padding leaves, between 0 and 1.
    #[must_use]
    pub fn align(mut self, align: f64) -> Self {
        self.align = align.clamp(0.0, 1.0);
        self.rescale();
        self
    }

    /// `scale(value)`: the start of the band of `value`, or `None` for a value outside the domain
    /// (d3 returns `undefined`).
    #[must_use]
    pub fn apply(&self, value: &str) -> Option<f64> {
        self.index.get(value).map(|&i| self.values[i])
    }

    /// The distinct values of the domain, in order.
    #[must_use]
    pub fn domain(&self) -> &[String] {
        &self.domain
    }

    /// `rescale()` of `band.js`: the start of each band, from the first value of the range. A range
    /// that runs backwards gives the starts in reverse.
    fn rescale(&mut self) {
        let n = self.domain.len() as f64;
        let reverse = self.range[1] < self.range[0];
        let (mut start, stop) = if reverse {
            (self.range[1], self.range[0])
        } else {
            (self.range[0], self.range[1])
        };
        let step = (stop - start) / 1.0_f64.max(n - self.padding_inner + self.padding_outer * 2.0);
        start += (stop - start - step * (n - self.padding_inner)) * self.align;
        let mut values: Vec<f64> = (0..self.domain.len())
            .map(|i| start + step * i as f64)
            .collect();
        if reverse {
            values.reverse();
        }
        self.values = values;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_domain_of_one_value_sits_in_the_middle_of_the_band() {
        // `scaleBand().domain(['a']).range([10, 110])`: the one band starts at 60, the middle.
        let scale = BandScale::new(&["a"], [10.0, 110.0])
            .padding_inner(1.0)
            .padding_outer(0.0)
            .align(0.5);
        assert_eq!(scale.apply("a"), Some(60.0));
    }

    #[test]
    fn the_bands_of_three_values_span_the_range() {
        let scale = BandScale::new(&["a", "b", "c"], [0.0, 200.0])
            .padding_inner(1.0)
            .padding_outer(0.0)
            .align(0.5);
        assert_eq!(scale.apply("a"), Some(0.0));
        assert_eq!(scale.apply("b"), Some(100.0));
        assert_eq!(scale.apply("c"), Some(200.0));
        assert_eq!(scale.apply("d"), None);
    }

    #[test]
    fn a_repeated_value_takes_its_first_place() {
        // The reference's `[a, a, b]`: `a` at 0 and `b` at 100, the second `a` is not a band.
        let scale = BandScale::new(&["a", "a", "b"], [0.0, 200.0])
            .padding_inner(1.0)
            .padding_outer(0.0)
            .align(0.5);
        assert_eq!(scale.apply("a"), Some(0.0));
        assert_eq!(scale.apply("b"), Some(200.0));
        assert_eq!(scale.domain().len(), 2);
    }

    #[test]
    fn a_linear_scale_maps_the_ends_of_its_domain() {
        let scale = LinearScale::new([0.0, 10.0], [100.0, 300.0]);
        assert_eq!(scale.apply(0.0), Some(100.0));
        assert_eq!(scale.apply(10.0), Some(300.0));
        assert_eq!(scale.apply(5.0), Some(200.0));
        assert_eq!(scale.apply(f64::NAN), None);
    }

    #[test]
    fn a_descending_domain_interpolates_the_range_backwards() {
        // The left axis of the reference reverses its domain: the largest value is at the top.
        let scale = LinearScale::new([10.0, 0.0], [300.0, 100.0]);
        assert_eq!(scale.apply(10.0), Some(300.0));
        assert_eq!(scale.apply(0.0), Some(100.0));
    }

    #[test]
    fn the_bands_of_seven_values_match_d3() {
        // `scaleBand` of d3-scale 4.0.2 over [39.97, 690.41], the values the probe gave.
        let scale = BandScale::new(&["a", "b", "c", "d", "e", "f", "g"], [39.97, 690.41])
            .padding_inner(1.0)
            .padding_outer(0.0)
            .align(0.5);
        let expected = [
            39.97,
            148.37666666666667,
            256.7833333333333,
            365.18999999999994,
            473.5966666666666,
            582.0033333333333,
            690.41,
        ];
        for (value, expected) in ["a", "b", "c", "d", "e", "f", "g"].iter().zip(expected) {
            assert_eq!(scale.apply(value), Some(expected), "band {value}");
        }
    }

    #[test]
    fn a_descending_band_range_gives_the_starts_in_reverse() {
        let scale = BandScale::new(&["a", "b", "c"], [200.0, 0.0])
            .padding_inner(1.0)
            .padding_outer(0.0)
            .align(0.5);
        assert_eq!(scale.apply("a"), Some(200.0));
        assert_eq!(scale.apply("c"), Some(0.0));
    }

    #[test]
    fn the_linear_scale_matches_d3_on_the_probed_values() {
        // The probe of d3-scale 4.0.2: [0, 10] over [100, 300] at 7.3, and a descending domain.
        assert_eq!(
            LinearScale::new([0.0, 10.0], [100.0, 300.0]).apply(7.3),
            Some(246.0)
        );
        assert_eq!(
            LinearScale::new([-2.4, 3.5], [467.0, 0.0]).apply(0.6),
            Some(229.54237288135593)
        );
        assert_eq!(
            LinearScale::new([10.0, 0.0], [300.0, 100.0]).apply(3.3),
            Some(166.0)
        );
    }

    #[test]
    fn a_zero_width_domain_maps_every_value_to_the_middle() {
        let scale = LinearScale::new([5.0, 5.0], [0.0, 100.0]);
        assert_eq!(scale.apply(5.0), Some(50.0));
        assert_eq!(scale.apply(7.0), Some(50.0));
    }
}
