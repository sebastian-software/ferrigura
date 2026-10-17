//! An object of the reference's JavaScript, with its keys in the order JavaScript gives them.
//!
//! `for…in`, `Object.keys` and `JSON.stringify` visit the keys of an ordinary object in one order
//! (`OrdinaryOwnPropertyKeys`, ECMA-262 10.1.11.1): first the array indices, the canonical decimal
//! integers from 0 to 2^32 − 2, in ascending order, then the other keys in the order they were
//! added. Assigning to a key that is there already replaces its value and keeps its place.
//! [`JsObject`] keeps its entries in that order, so where the reference draws or lists the keys of
//! an object, the port does so in the order of the entries.
//!
//! It is written as a JSON object with the keys in that order, and read from one in the order of
//! the text, as `JSON.parse` adds them.

use std::fmt;
use std::slice;
use std::vec;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::value::Value;

/// A JavaScript object of string keys, with its entries in JavaScript's key order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JsObject {
    /// The entries: the array indices first, ascending, then the other keys in insertion order.
    entries: Vec<(String, Value)>,
}

impl JsObject {
    /// An empty object, `{}`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `object[key] = value`: a key that is there keeps its place and takes the new value; a new
    /// array index goes among the indices in ascending order, any other new key goes last.
    pub fn insert(&mut self, key: String, value: Value) {
        if let Some(entry) = self.entries.iter_mut().find(|(name, _)| *name == key) {
            entry.1 = value;
            return;
        }
        let place = match array_index(&key) {
            Some(index) => self
                .entries
                .iter()
                .position(|(name, _)| array_index(name).is_none_or(|other| other > index))
                .unwrap_or(self.entries.len()),
            None => self.entries.len(),
        };
        self.entries.insert(place, (key, value));
    }

    /// The value of a key, if the object has it.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// The number of keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the object has no keys.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries in key order.
    #[must_use]
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.entries.iter())
    }

    /// The keys in key order, as `Object.keys` gives them.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.iter().map(|(key, _)| key)
    }
}

/// The array index a key stands for: a decimal integer from 0 to 2^32 − 2, written without a sign
/// or a leading zero (ECMA-262 6.1.7). `"01"`, `"-1"`, `"1.5"` and `"4294967295"` are not indices.
fn array_index(key: &str) -> Option<u32> {
    let canonical = !key.is_empty()
        && key.bytes().all(|byte| byte.is_ascii_digit())
        && (key == "0" || !key.starts_with('0'));
    if !canonical {
        return None;
    }
    key.parse::<u32>().ok().filter(|&index| index != u32::MAX)
}

/// The entries of a [`JsObject`] in key order.
#[derive(Debug, Clone)]
pub struct Iter<'a>(slice::Iter<'a, (String, Value)>);

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a String, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, value)| (key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> IntoIterator for &'a JsObject {
    type Item = (&'a String, &'a Value);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

impl IntoIterator for JsObject {
    type Item = (String, Value);
    type IntoIter = vec::IntoIter<(String, Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

/// Assigns the entries one after the other, as [`JsObject::insert`] does.
impl FromIterator<(String, Value)> for JsObject {
    fn from_iter<I: IntoIterator<Item = (String, Value)>>(entries: I) -> Self {
        let mut object = Self::new();
        for (key, value) in entries {
            object.insert(key, value);
        }
        object
    }
}

impl Serialize for JsObject {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for (key, value) in &self.entries {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for JsObject {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(JsObjectVisitor)
    }
}

struct JsObjectVisitor;

impl<'de> Visitor<'de> for JsObjectVisitor {
    type Value = JsObject;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object")
    }

    /// The members in the order of the text, each assigned as `JSON.parse` does, so a repeated key
    /// keeps its first place and takes its last value.
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<JsObject, A::Error> {
        let mut object = JsObject::new();
        while let Some((key, value)) = map.next_entry::<String, Value>()? {
            object.insert(key, value);
        }
        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(object: &JsObject) -> Vec<&str> {
        object.keys().map(String::as_str).collect()
    }

    fn object(keys: &[&str]) -> JsObject {
        keys.iter()
            .map(|key| ((*key).to_owned(), Value::from(*key)))
            .collect()
    }

    #[test]
    fn other_keys_keep_their_insertion_order() {
        assert_eq!(keys(&object(&["b", "a", "c"])), ["b", "a", "c"]);
    }

    #[test]
    fn array_indices_come_first_in_ascending_order() {
        let object = object(&["b", "10", "a", "2", "0"]);
        assert_eq!(keys(&object), ["0", "2", "10", "b", "a"]);
    }

    #[test]
    fn a_repeated_key_keeps_its_place_and_takes_the_new_value() {
        let mut object = object(&["b", "a"]);
        object.insert("b".to_owned(), Value::from("again"));
        assert_eq!(keys(&object), ["b", "a"]);
        assert_eq!(object.get("b"), Some(&Value::from("again")));
    }

    #[test]
    fn only_canonical_integers_below_the_array_length_limit_are_indices() {
        // Checked in Node 24: `Object.keys` of the same object gives this order.
        let object = object(&[
            "b",
            "4294967294",
            "4294967295",
            "01",
            "1",
            "-1",
            "1.5",
            "+1",
            " 1",
        ]);
        assert_eq!(
            keys(&object),
            [
                "1",
                "4294967294",
                "b",
                "4294967295",
                "01",
                "-1",
                "1.5",
                "+1",
                " 1"
            ]
        );
    }

    #[test]
    fn json_keeps_the_key_order_both_ways() {
        let text = r#"{"b":"1","10":"2","a":"3","2":"4","b":"5"}"#;
        let object: JsObject = serde_json::from_str(text).expect("an object");
        assert_eq!(keys(&object), ["2", "10", "b", "a"]);
        assert_eq!(object.get("b"), Some(&Value::from("5")));
        assert_eq!(
            serde_json::to_string(&object).expect("written"),
            r#"{"2":"4","10":"2","b":"5","a":"3"}"#
        );
    }

    #[test]
    fn json_that_is_not_an_object_is_rejected() {
        assert!(serde_json::from_str::<JsObject>("[1, 2]").is_err());
        assert!(serde_json::from_str::<JsObject>(r#""ab""#).is_err());
    }
}
