//! The value of a configuration, as the reference's JavaScript objects hold it: `null`, booleans,
//! numbers, strings, arrays and objects.
//!
//! It is JSON with one difference: a number may be `NaN` or infinite, because YAML reads `.nan`,
//! `.inf` and `-.inf` as numbers and a configuration keeps them as numbers. A string in their place
//! would let a numeric option turn into text without anybody noticing. Only the JSON that the
//! differential run writes spells them out, as the strings `"NaN"`, `"Infinity"` and `"-Infinity"`.
//!
//! The value is also a serde deserializer, so the typed view of the configuration reads it
//! directly: a number that is whole becomes an integer, every other number an `f64`.

use std::collections::{BTreeMap, btree_map};
use std::vec;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A configuration value. Objects are sorted by key, as JSON written by `serde_json` is without
/// its `preserve_order` feature.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    /// `null`.
    #[default]
    Null,
    /// A boolean.
    Bool(bool),
    /// A number, which may be `NaN` or infinite.
    Number(f64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object.
    Object(BTreeMap<String, Value>),
}

impl Value {
    /// An empty object.
    #[must_use]
    pub fn empty_object() -> Self {
        Self::Object(BTreeMap::new())
    }

    /// The value of a key, if this is an object that has the key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Self::Object(map) => map.get(key),
            _ => None,
        }
    }

    /// The value of a key, mutably, if this is an object that has the key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Self::Object(map) => map.get_mut(key),
            _ => None,
        }
    }

    /// The text, if this is a string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    /// The number, if this is a number.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// Whether this is `null`.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Whether this is an object.
    #[must_use]
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    /// Whether this is an array.
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    /// The items, if this is an array.
    #[must_use]
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The entries, if this is an object.
    #[must_use]
    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    /// The entries, mutably, if this is an object.
    pub fn as_object_mut(&mut self) -> Option<&mut BTreeMap<String, Value>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }
}

/// The value of a key of an object, or `null` when it has none, as `serde_json` indexes them.
static NULL: Value = Value::Null;

impl std::ops::Index<&str> for Value {
    type Output = Value;

    fn index(&self, key: &str) -> &Value {
        self.get(key).unwrap_or(&NULL)
    }
}

impl From<&str> for Value {
    fn from(text: &str) -> Self {
        Self::String(text.to_owned())
    }
}

impl From<String> for Value {
    fn from(text: String) -> Self {
        Self::String(text)
    }
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(flag) => Self::Bool(flag),
            serde_json::Value::Number(number) => Self::Number(number.as_f64().unwrap_or(f64::NAN)),
            serde_json::Value::String(text) => Self::String(text),
            serde_json::Value::Array(items) => {
                Self::Array(items.into_iter().map(Self::from).collect())
            }
            serde_json::Value::Object(map) => Self::Object(
                map.into_iter()
                    .map(|(key, value)| (key, Self::from(value)))
                    .collect(),
            ),
        }
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(flag) => serializer.serialize_bool(*flag),
            Self::Number(number) => serializer.serialize_f64(*number),
            Self::String(text) => serializer.serialize_str(text),
            Self::Array(items) => items.serialize(serializer),
            Self::Object(map) => map.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a configuration value")
    }

    fn visit_bool<E>(self, flag: bool) -> Result<Value, E> {
        Ok(Value::Bool(flag))
    }

    fn visit_i64<E>(self, number: i64) -> Result<Value, E> {
        Ok(Value::Number(number as f64))
    }

    fn visit_u64<E>(self, number: u64) -> Result<Value, E> {
        Ok(Value::Number(number as f64))
    }

    fn visit_f64<E>(self, number: f64) -> Result<Value, E> {
        Ok(Value::Number(number))
    }

    fn visit_str<E>(self, text: &str) -> Result<Value, E> {
        Ok(Value::String(text.to_owned()))
    }

    fn visit_string<E>(self, text: String) -> Result<Value, E> {
        Ok(Value::String(text))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        Value::deserialize(deserializer)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element::<Value>()? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut entries = BTreeMap::new();
        while let Some((key, value)) = map.next_entry::<String, Value>()? {
            entries.insert(key, value);
        }
        Ok(Value::Object(entries))
    }
}

/// Reads a value as a deserializer. Whole numbers are offered as integers, so an `i64` field
/// accepts `500`; other numbers are offered as `f64`, `NaN` and infinity included.
impl<'de> Deserializer<'de> for Value {
    type Error = serde_json::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Null => visitor.visit_unit(),
            Self::Bool(flag) => visitor.visit_bool(flag),
            Self::Number(number) => {
                if number.is_finite() && number.fract() == 0.0 && number.abs() < 9.0e15 {
                    visitor.visit_i64(number as i64)
                } else {
                    visitor.visit_f64(number)
                }
            }
            Self::String(text) => visitor.visit_string(text),
            Self::Array(items) => visitor.visit_seq(Items {
                items: items.into_iter(),
            }),
            Self::Object(map) => visitor.visit_map(Entries {
                entries: map.into_iter(),
                value: None,
            }),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Null => visitor.visit_none(),
            other => visitor.visit_some(other),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
    }
}

struct Items {
    items: vec::IntoIter<Value>,
}

impl<'de> SeqAccess<'de> for Items {
    type Error = serde_json::Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        match self.items.next() {
            Some(item) => seed.deserialize(item).map(Some),
            None => Ok(None),
        }
    }
}

struct Entries {
    entries: btree_map::IntoIter<String, Value>,
    value: Option<Value>,
}

impl<'de> MapAccess<'de> for Entries {
    type Error = serde_json::Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.entries.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(de::value::StringDeserializer::<serde_json::Error>::new(key))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(de::Error::custom("a map value is missing its key")),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_non_finite_number_stays_a_number() {
        let value = Value::Object(BTreeMap::from([(
            "limit".to_owned(),
            Value::Number(f64::INFINITY),
        )]));
        let limit: f64 =
            serde::Deserialize::deserialize(value.get("limit").cloned().unwrap_or_default())
                .unwrap_or(0.0);
        assert!(limit.is_infinite());
    }

    #[test]
    fn whole_numbers_read_into_integers_and_others_into_floats() {
        let whole: i64 = Deserialize::deserialize(Value::Number(500.0)).expect("an integer");
        assert_eq!(whole, 500);
        assert!(i64::deserialize(Value::Number(0.5)).is_err());
        let float: f64 = Deserialize::deserialize(Value::Number(0.5)).expect("a float");
        assert_eq!(float, 0.5);
    }

    #[test]
    fn a_json_value_converts() {
        let value = Value::from(json!({ "a": [1, "x", null], "b": true }));
        assert_eq!(value.get("b"), Some(&Value::Bool(true)));
        assert_eq!(
            value.get("a"),
            Some(&Value::Array(vec![
                Value::Number(1.0),
                Value::String("x".to_owned()),
                Value::Null,
            ]))
        );
    }

    #[test]
    fn a_string_does_not_read_as_a_number() {
        assert!(f64::deserialize(Value::String("Infinity".to_owned())).is_err());
    }
}
