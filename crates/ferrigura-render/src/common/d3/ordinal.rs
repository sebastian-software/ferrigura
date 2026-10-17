//! `scaleOrdinal(range)` of `d3-scale` 4.0.2 (`ordinal.js`) and the `schemeTableau10` of
//! `d3-scale-chromatic` 3.1.0 (`Tableau10.js`).
//!
//! A value takes the next slot of the range the first time it is seen, and the slot repeats every
//! `range.length` values, so the colour of a value depends on the order in which the values were
//! first asked for.

use std::collections::HashMap;

/// `schemeTableau10`: the ten colours, in order.
pub const SCHEME_TABLEAU10: [&str; 10] = [
    "#4e79a7", "#f28e2c", "#e15759", "#76b7b2", "#59a14f", "#edc949", "#af7aa1", "#ff9da7",
    "#9c755f", "#bab0ab",
];

/// An ordinal scale over string values with a range of colours.
#[derive(Debug, Clone)]
pub struct OrdinalScale {
    index: HashMap<String, usize>,
    range: Vec<&'static str>,
}

impl OrdinalScale {
    /// A scale with the given range and an empty domain.
    #[must_use]
    pub fn new(range: &[&'static str]) -> Self {
        Self {
            index: HashMap::new(),
            range: range.to_vec(),
        }
    }

    /// `scale(value)`: the range value of `value`, which joins the domain when it is new.
    ///
    /// # Panics
    ///
    /// Panics if the range is empty, which the reference's scales never are.
    pub fn apply(&mut self, value: &str) -> &'static str {
        let next = self.index.len();
        let slot = *self.index.entry(value.to_owned()).or_insert(next);
        self.range[slot % self.range.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_takes_the_next_slot_when_it_is_first_seen() {
        let mut scale = OrdinalScale::new(&SCHEME_TABLEAU10);
        assert_eq!(scale.apply("b"), "#4e79a7");
        assert_eq!(scale.apply("a"), "#f28e2c");
        assert_eq!(scale.apply("b"), "#4e79a7");
    }

    #[test]
    fn the_slots_repeat_with_the_range() {
        let mut scale = OrdinalScale::new(&SCHEME_TABLEAU10);
        for index in 0..10 {
            scale.apply(&format!("n{index}"));
        }
        assert_eq!(scale.apply("eleventh"), "#4e79a7");
    }
}
