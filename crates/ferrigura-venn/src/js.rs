//! The JavaScript behaviours the library's numbers depend on.
//!
//! The library is JavaScript, and four of its behaviours decide which numbers come out, so they are
//! reproduced here: the order of `Object.keys` ([`JsObject`], [`js_key_order`]), the order of
//! `Array.prototype.sort` ([`js_sort`]), the results of `Math.min` and `Math.max` on NaN and signed
//! zero ([`js_min`], [`js_max`]), and the code-unit order of strings ([`js_str_cmp`]). Membership
//! with `in` on a plain object is [`js_has`].

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// The names a plain object inherits from `Object.prototype`. The library tests membership with
/// `in` on plain objects (`greedyLayout`'s `positioned`, `computeTextCentres`' `areaids` and
/// `exclude`), which is true for these names whether or not they were set, so [`js_has`] reports
/// them. `__proto__` is not here: assigning to it changes the prototype instead of adding a key, so
/// the crate refuses that set name (`Error::ReservedName`).
const INHERITED_NAMES: [&str; 11] = [
    "constructor",
    "__defineGetter__",
    "__defineSetter__",
    "hasOwnProperty",
    "__lookupGetter__",
    "__lookupSetter__",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toString",
    "valueOf",
    "toLocaleString",
];

/// A JavaScript object keyed by set ids. Its entries are kept in insertion order, and
/// [`JsObject::iter`] gives them in `Object.keys` order. Assigning an existing key keeps its place.
#[derive(Clone, Debug)]
pub(crate) struct JsObject<V> {
    entries: Vec<(String, V)>,
    index: HashMap<String, usize>,
}

impl<V> JsObject<V> {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, key: &str, value: V) {
        if let Some(&position) = self.index.get(key) {
            self.entries[position].1 = value;
        } else {
            self.index.insert(key.to_owned(), self.entries.len());
            self.entries.push((key.to_owned(), value));
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<&V> {
        self.index
            .get(key)
            .map(|&position| &self.entries[position].1)
    }

    pub(crate) fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let position = *self.index.get(key)?;
        Some(&mut self.entries[position].1)
    }

    /// The position of `key` in insertion order, which indexes [`JsObject::values`].
    pub(crate) fn position(&self, key: &str) -> Option<usize> {
        self.index.get(key).copied()
    }

    /// The values in insertion order.
    pub(crate) fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.entries.iter().map(|(_, value)| value)
    }

    /// The entries in the order `Object.keys` gives them.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &V)> + '_ {
        js_key_order(self.entries.iter().map(|(key, _)| key.as_str()))
            .into_iter()
            .map(move |position| {
                let (key, value) = &self.entries[position];
                (key.as_str(), value)
            })
    }
}

/// The positions of `keys` in the order `Object.keys` gives them: the array indices first, in
/// ascending numeric order, then the other keys in insertion order.
pub(crate) fn js_key_order<'a>(keys: impl Iterator<Item = &'a str>) -> Vec<usize> {
    let mut indices: Vec<(u32, usize)> = Vec::new();
    let mut named: Vec<usize> = Vec::new();
    for (position, key) in keys.enumerate() {
        match array_index(key) {
            Some(index) => indices.push((index, position)),
            None => named.push(position),
        }
    }
    indices.sort_unstable();
    indices
        .into_iter()
        .map(|(_, position)| position)
        .chain(named)
        .collect()
}

/// The value of a key that is an array index: a canonical decimal number up to 2^32 - 2.
fn array_index(key: &str) -> Option<u32> {
    let bytes = key.as_bytes();
    if bytes.is_empty() || bytes.len() > 10 || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    if bytes.len() > 1 && bytes[0] == b'0' {
        return None;
    }
    let value: u64 = key.parse().ok()?;
    (value <= 4_294_967_294).then_some(value as u32)
}

/// Whether `key in object` holds for a plain object whose own keys are `own`.
pub(crate) fn js_has(own: &HashSet<String>, key: &str) -> bool {
    own.contains(key) || INHERITED_NAMES.contains(&key)
}

/// `Array.prototype.sort` with a comparator, as V8 runs it. For arrays of fewer than 64 elements
/// V8 finds one run and sorts the rest by binary insertion, which is what this does: the comparator
/// result is tested with `< 0` and `>= 0` as V8 tests it, so a NaN counts as "not less". With a
/// consistent comparator every stable sort gives this order; the difference is only for NaN results
/// in arrays of 64 or more, where V8 merges runs.
pub(crate) fn js_sort<T>(items: &mut [T], compare: impl Fn(&T, &T) -> f64) {
    let length = items.len();
    if length < 2 {
        return;
    }
    let descending = compare(&items[1], &items[0]) < 0.0;
    let mut run = 2;
    while run < length {
        let order = compare(&items[run], &items[run - 1]);
        if (descending && order >= 0.0) || (!descending && order < 0.0) {
            break;
        }
        run += 1;
    }
    if descending {
        items[..run].reverse();
    }
    for start in run..length {
        let mut left = 0;
        let mut right = start;
        while left < right {
            let middle = left + ((right - left) >> 1);
            if compare(&items[start], &items[middle]) < 0.0 {
                right = middle;
            } else {
                left = middle + 1;
            }
        }
        items[left..=start].rotate_right(1);
    }
}

/// `Math.min(a, b)`: NaN if either is NaN, and -0 before +0.
pub(crate) fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { a } else { b }
    } else if a < b {
        a
    } else {
        b
    }
}

/// `Math.max(a, b)`: NaN if either is NaN, and +0 before -0.
pub(crate) fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_positive() { a } else { b }
    } else if a > b {
        a
    } else {
        b
    }
}

/// The order of JavaScript's `<` on strings: UTF-16 code units.
pub(crate) fn js_str_cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_keys_come_first_in_numeric_order() {
        let keys = ["b", "10", "a", "2", "1"];
        let order = js_key_order(keys.iter().copied());
        let ordered: Vec<&str> = order.iter().map(|&i| keys[i]).collect();
        assert_eq!(ordered, ["1", "2", "10", "b", "a"]);
    }

    #[test]
    fn leading_zero_and_large_integers_are_named_keys() {
        assert_eq!(array_index("01"), None);
        assert_eq!(array_index("4294967294"), Some(4_294_967_294));
        assert_eq!(array_index("4294967295"), None);
        assert_eq!(array_index("-1"), None);
    }

    #[test]
    fn sort_is_stable_and_treats_nan_as_not_less() {
        let mut values = vec![(2, 'a'), (1, 'b'), (2, 'c'), (1, 'd')];
        js_sort(&mut values, |a, b| f64::from(a.0 - b.0));
        assert_eq!(values, [(1, 'b'), (1, 'd'), (2, 'a'), (2, 'c')]);
        let mut with_nan = vec![3.0, f64::NAN, 1.0];
        js_sort(&mut with_nan, |_, _| f64::NAN);
        assert_eq!(with_nan[0], 3.0);
    }

    #[test]
    fn min_and_max_follow_javascript() {
        assert!(js_min(f64::NAN, 1.0).is_nan());
        assert!(js_max(0.0, f64::NAN).is_nan());
        assert!(js_min(0.0, -0.0).is_sign_negative());
        assert!(js_max(-0.0, 0.0).is_sign_positive());
    }

    #[test]
    fn inherited_names_are_members() {
        let own = HashSet::from(["A".to_owned()]);
        assert!(js_has(&own, "A"));
        assert!(js_has(&own, "constructor"));
        assert!(!js_has(&own, "B"));
    }
}
