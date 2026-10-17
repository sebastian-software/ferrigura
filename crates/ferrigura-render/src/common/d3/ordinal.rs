//! `scaleOrdinal` of `d3-scale` (`ordinal.js` 1-14) with a range and no domain: the domain grows as
//! the scale is called, each new key taking the next index, and the value is `range[index % length]`.
//! The reference calls its three colour scales in a fixed order, so the first key a scale sees
//! decides its colour, and the order of the calls is the order of the renderer's code.

use std::collections::HashMap;

/// An ordinal scale over a range, with an implicit domain.
#[derive(Debug, Clone)]
pub struct Ordinal<'r> {
    range: &'r [String],
    index: HashMap<String, usize>,
}

impl<'r> Ordinal<'r> {
    /// A scale with the given range and an empty domain.
    #[must_use]
    pub fn new(range: &'r [String]) -> Self {
        Self {
            range,
            index: HashMap::new(),
        }
    }

    /// `scale(key)`: the range value of the key. A key the scale has not seen joins its domain.
    ///
    /// # Panics
    ///
    /// Never for a non-empty range; an empty range has no value to return, and the reference's
    /// scales always have one.
    pub fn get(&mut self, key: &str) -> &'r str {
        let next = self.index.len();
        let position = *self.index.entry(key.to_owned()).or_insert(next);
        &self.range[position % self.range.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_key_takes_the_first_value_and_the_range_repeats() {
        let range: Vec<String> = ["a", "b", "c"].iter().map(|s| (*s).to_owned()).collect();
        let mut scale = Ordinal::new(&range);
        assert_eq!(scale.get("x"), "a");
        assert_eq!(scale.get("y"), "b");
        assert_eq!(scale.get("x"), "a");
        assert_eq!(scale.get("z"), "c");
        assert_eq!(scale.get("w"), "a");
    }
}
