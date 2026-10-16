//! The parts of the reference's theme classes that are not colour arithmetic: the properties of a
//! theme instance, the JavaScript operators its code uses on them (`||`, `??`, truthiness and the
//! conversion to a number), `mkBorder` and `applyOverride` of `theme-helpers.js`, and the
//! `calculate` that runs the constructor, the overrides and `updateColors` in the reference's order.
//!
//! A property of the reference's `this` is a [`Js`] value: `None` is `undefined`, which is what a
//! property that was never set is, and `Some(Value::Null)` is `null`. Only the `undefined` ones are
//! left out of the object that the theme returns, as `JSON.stringify` leaves them out.

use std::collections::BTreeMap;

use crate::value::Value;

use super::Res;
use super::ThemeError;
use super::khroma::{self, Channel, js_number};

/// A JavaScript value of a theme property. `None` is `undefined`.
pub(crate) type Js = Option<Value>;

/// The properties of a theme instance, the reference's `this`.
#[derive(Debug, Default)]
pub(crate) struct Vars {
    properties: BTreeMap<String, Value>,
}

impl Vars {
    /// `this[key]`: a copy of the property, `undefined` when it is not set.
    pub(crate) fn get(&self, key: &str) -> Js {
        self.properties.get(key).cloned()
    }

    /// `this[key] = value`. Setting `undefined` removes the property.
    pub(crate) fn set(&mut self, key: &str, value: Js) {
        match value {
            Some(value) => {
                self.properties.insert(key.to_owned(), value);
            }
            None => {
                self.properties.remove(key);
            }
        }
    }

    /// `this[key] = 'text'`.
    pub(crate) fn text(&mut self, key: &str, value: &str) {
        self.set(key, text(value));
    }

    /// `this[key] = 600`, a number.
    pub(crate) fn number(&mut self, key: &str, value: f64) {
        self.set(key, Some(Value::Number(value)));
    }

    /// `this[key] = true`, a boolean.
    pub(crate) fn flag(&mut self, key: &str, value: bool) {
        self.set(key, Some(Value::Bool(value)));
    }

    /// Whether `this[key]` is truthy.
    pub(crate) fn truthy(&self, key: &str) -> bool {
        truthy(&self.get(key))
    }

    /// `this[key] = this[key] || fallback()`. The fallback is computed only when the property is
    /// not truthy, as the `||` of JavaScript computes it.
    pub(crate) fn or(&mut self, key: &str, fallback: impl FnOnce(&Vars) -> Res<Js>) -> Res<()> {
        if !self.truthy(key) {
            let value = fallback(self)?;
            self.set(key, value);
        }
        Ok(())
    }

    /// `this[key] = this[key] || 'text'`.
    pub(crate) fn or_text(&mut self, key: &str, value: &str) {
        if !self.truthy(key) {
            self.text(key, value);
        }
    }

    /// `this[key] = this[key] || this[other]`.
    pub(crate) fn or_key(&mut self, key: &str, other: &str) {
        if !self.truthy(key) {
            let value = self.get(other);
            self.set(key, value);
        }
    }

    /// `this[key] = this[key] ?? 'text'`.
    pub(crate) fn nullish_text(&mut self, key: &str, value: &str) {
        if nullish(&self.get(key)) {
            self.text(key, value);
        }
    }

    /// `this[key] = this[key] ?? this[other]`.
    pub(crate) fn nullish_key(&mut self, key: &str, other: &str) {
        if nullish(&self.get(key)) {
            let value = self.get(other);
            self.set(key, value);
        }
    }

    /// `this[key] = this[key] ?? fallback()`.
    pub(crate) fn nullish(
        &mut self,
        key: &str,
        fallback: impl FnOnce(&Vars) -> Res<Js>,
    ) -> Res<()> {
        if nullish(&self.get(key)) {
            let value = fallback(self)?;
            self.set(key, value);
        }
        Ok(())
    }

    /// The properties, for the object that the theme returns.
    pub(crate) fn into_properties(self) -> BTreeMap<String, Value> {
        self.properties
    }
}

/// A JavaScript string.
pub(crate) fn text(value: &str) -> Js {
    Some(Value::String(value.to_owned()))
}

/// A JavaScript string from an owned string, as the colour functions return them.
pub(crate) fn owned(value: String) -> Js {
    Some(Value::String(value))
}

/// An array of JavaScript strings, such as `borderColorArray`.
pub(crate) fn text_array(items: &[&str]) -> Js {
    Some(Value::Array(
        items
            .iter()
            .map(|item| Value::String((*item).to_owned()))
            .collect(),
    ))
}

/// JavaScript truthiness: `undefined`, `null`, `false`, zero, NaN and the empty string are falsy.
pub(crate) fn truthy(value: &Js) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => *number != 0.0 && !number.is_nan(),
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Array(_) | Value::Object(_)) => true,
    }
}

/// Whether a value is `undefined` or `null`, which is what `??` looks at.
pub(crate) fn nullish(value: &Js) -> bool {
    matches!(value, None | Some(Value::Null))
}

/// `a || b || c`: the first truthy value of the keys, or the last key's value when none is truthy.
pub(crate) fn first_truthy(vars: &Vars, keys: &[&str]) -> Js {
    let mut value = None;
    for key in keys {
        value = vars.get(key);
        if truthy(&value) {
            return value;
        }
    }
    value
}

/// `value[key]` for a number key, as JavaScript reads it. An array or a string gives the element or
/// the character at an integer index, an object gives the property of the key's text, and any other
/// value gives `undefined`. Reading a property of `null` or `undefined` throws.
///
/// A string is indexed by UTF-16 code unit, as in JavaScript. A unit that is half of a surrogate
/// pair cannot be a Rust string, so it is written as the replacement character.
pub(crate) fn element(value: &Js, key: f64) -> Res<Js> {
    let index = integer_index(key);
    match value {
        None | Some(Value::Null) => Err(ThemeError::new("Cannot read properties of null")),
        Some(Value::Array(items)) => Ok(index.and_then(|index| items.get(index).cloned())),
        Some(Value::String(text)) => Ok(index.and_then(|index| {
            text.encode_utf16()
                .nth(index)
                .map(|unit| Value::String(String::from_utf16_lossy(&[unit])))
        })),
        Some(Value::Object(map)) => Ok(map.get(&khroma::js_number(key)).cloned()),
        Some(_) => Ok(None),
    }
}

/// The index of an array or string for a number key: a whole, non-negative number.
fn integer_index(key: f64) -> Option<usize> {
    if key >= 0.0 && key.fract() == 0.0 && key < 9_007_199_254_740_992.0 {
        Some(key as usize)
    } else {
        None
    }
}

/// `value.length` as JavaScript reads it, as a number: the element count of an array, the UTF-16
/// length of a string, or the `length` property of an object converted to a number. Any other value
/// has no length, which is NaN here, as `undefined` is in arithmetic. Reading it from `null` or
/// `undefined` throws.
pub(crate) fn length_of(value: &Js) -> Res<f64> {
    match value {
        None | Some(Value::Null) => Err(ThemeError::new("Cannot read properties of null")),
        Some(Value::Array(items)) => Ok(items.len() as f64),
        Some(Value::String(text)) => Ok(text.encode_utf16().count() as f64),
        Some(Value::Object(map)) => Ok(map.get("length").map_or(f64::NAN, number_of)),
        Some(_) => Ok(f64::NAN),
    }
}

/// `arr[i % arr.length]`, as the reference's `venn` colours read a palette.
pub(crate) fn cycle(value: &Js, index: usize) -> Res<Js> {
    let length = length_of(value)?;
    element(value, index as f64 % length)
}

/// The text of a colour. A value that is not a string is not a colour that khroma reads.
pub(crate) fn colour(value: &Js) -> Res<&str> {
    match value {
        Some(Value::String(text)) => Ok(text),
        _ => Err(ThemeError::new("a colour must be a string")),
    }
}

/// `adjust(color, channels)` of khroma, on a theme property.
pub(crate) fn adjust(value: &Js, channels: &[(Channel, f64)]) -> Res<Js> {
    Ok(owned(khroma::adjust(colour(value)?, channels)?))
}

/// `darken(color, amount)` of khroma, on a theme property.
pub(crate) fn darken(value: &Js, amount: f64) -> Res<Js> {
    Ok(owned(khroma::darken(colour(value)?, amount)?))
}

/// `lighten(color, amount)` of khroma, on a theme property.
pub(crate) fn lighten(value: &Js, amount: f64) -> Res<Js> {
    Ok(owned(khroma::lighten(colour(value)?, amount)?))
}

/// `invert(color)` of khroma, with its default weight of 100, on a theme property.
pub(crate) fn invert(value: &Js) -> Res<Js> {
    Ok(owned(khroma::invert(colour(value)?, 100.0)?))
}

/// `isDark(color)` of khroma, on a theme property.
pub(crate) fn is_dark(value: &Js) -> Res<bool> {
    khroma::is_dark(colour(value)?)
}

/// `mkBorder(col, darkMode)` of `theme-helpers.js`: a border shade of a colour, darker on a light
/// theme and lighter on a dark one.
pub(crate) fn mk_border(value: &Js, dark: bool) -> Res<Js> {
    if dark {
        adjust(value, &[(Channel::S, -40.0), (Channel::L, 10.0)])
    } else {
        adjust(value, &[(Channel::S, -40.0), (Channel::L, -10.0)])
    }
}

/// `applyOverride(generated, override)` of `theme-helpers.js`. An object keeps every key the theme
/// generated and takes the override's keys on top; any other override is the value.
pub(crate) fn apply_override(generated: Js, value: &Value) -> Value {
    match (generated, value) {
        (Some(Value::Object(mut merged)), Value::Object(overrides)) => {
            merged.extend(
                overrides
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
            Value::Object(merged)
        }
        _ => value.clone(),
    }
}

/// `xyChart` of the themes: the object with each of its colours taken from the user's object when
/// that is truthy, or else from the theme's own colour.
pub(crate) fn set_xy_chart(vars: &mut Vars) {
    let current = vars.get("xyChart");
    let field = |key: &str| -> Js {
        match &current {
            Some(Value::Object(map)) => map.get(key).cloned(),
            _ => None,
        }
    };
    let background = vars.get("background");
    let primary_text = vars.get("primaryTextColor");
    let fallbacks: [(&str, Js); 11] = [
        ("backgroundColor", background),
        ("titleColor", primary_text.clone()),
        ("legendTextColor", primary_text.clone()),
        ("xAxisTitleColor", primary_text.clone()),
        ("xAxisLabelColor", primary_text.clone()),
        ("xAxisTickColor", primary_text.clone()),
        ("xAxisLineColor", primary_text.clone()),
        ("yAxisTitleColor", primary_text.clone()),
        ("yAxisLabelColor", primary_text.clone()),
        ("yAxisTickColor", primary_text.clone()),
        ("yAxisLineColor", primary_text),
    ];
    let mut map = BTreeMap::new();
    for (key, fallback) in fallbacks {
        let value = or_else_value(field(key), fallback);
        if let Some(value) = value {
            map.insert(key.to_owned(), value);
        }
    }
    let palette = or_else_value(
        field("plotColorPalette"),
        text("#FFF4DD,#FFD8B1,#FFA07A,#ECEFF1,#D6DBDF,#C3E0A8,#FFB6A4,#FFD74D,#738FA7,#FFFFF0"),
    );
    if let Some(palette) = palette {
        map.insert("plotColorPalette".to_owned(), palette);
    }
    vars.set("xyChart", Some(Value::Object(map)));
}

/// The indexes of `for (let i = 0; i < limit; i++)`: from zero while the index is below the limit,
/// which may be any number, as the reference compares it.
pub(crate) fn scale_indices(limit: f64) -> impl Iterator<Item = usize> {
    (0_usize..).take_while(move |index| (*index as f64) < limit)
}

/// `a || b` on values that are already computed.
fn or_else_value(value: Js, fallback: Js) -> Js {
    if truthy(&value) { value } else { fallback }
}

/// `calculate(overrides)` of the theme classes: the user's overrides are copied in, the colours are
/// derived with `update_colors`, and then the overrides are copied in again, so that an override
/// wins over the value its derivation would have given. Only an object (or an array, whose keys are
/// its indexes, as `Object.keys` gives them) is taken as overrides; anything else, `null` included,
/// is no overrides.
pub(crate) fn calculate(
    vars: &mut Vars,
    overrides: Option<&Value>,
    update_colors: fn(&mut Vars) -> Res<()>,
) -> Res<()> {
    let entries: Vec<(String, Value)> = match overrides {
        Some(Value::Object(map)) => map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        Some(Value::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        _ => return update_colors(vars),
    };

    for (key, value) in &entries {
        vars.set(key, Some(value.clone()));
    }
    update_colors(vars)?;
    for (key, value) in &entries {
        let generated = vars.get(key);
        vars.set(key, Some(apply_override(generated, value)));
    }
    Ok(())
}

/// `Number(value)` of JavaScript, as the reference's numeric comparisons see a value such as
/// `THEME_COLOR_LIMIT`. `undefined` is NaN.
pub(crate) fn to_number(value: &Js) -> f64 {
    match value {
        None => f64::NAN,
        Some(value) => number_of(value),
    }
}

/// `ToNumber` of a configuration value.
fn number_of(value: &Value) -> f64 {
    match value {
        Value::Null => 0.0,
        Value::Bool(flag) => {
            if *flag {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(number) => *number,
        Value::String(text) => string_number(text),
        Value::Array(_) | Value::Object(_) => string_number(&string_of(value)),
    }
}

/// `ToString` of a configuration value, as the string an array or an object turns into.
fn string_of(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => js_number(*number),
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                if matches!(item, Value::Null) {
                    String::new()
                } else {
                    string_of(item)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// Whether a character is white space or a line terminator for JavaScript.
fn is_js_space(character: char) -> bool {
    matches!(
        character,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// `StringToNumber` of JavaScript: white space around a decimal, hexadecimal, octal or binary
/// literal, `Infinity`, or the empty string (zero). Anything else is NaN.
fn string_number(text: &str) -> f64 {
    let trimmed = text.trim_matches(is_js_space);
    if trimmed.is_empty() {
        return 0.0;
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(digits) = trimmed.strip_prefix(prefix) {
            return radix_number(digits, radix);
        }
    }
    let (sign, unsigned) = if let Some(rest) = trimmed.strip_prefix('-') {
        (-1.0, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (1.0, rest)
    } else {
        (1.0, trimmed)
    };
    if unsigned == "Infinity" {
        return sign * f64::INFINITY;
    }
    if is_decimal_literal(unsigned) {
        trimmed.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// The value of digits in a radix, or NaN when there are none or one is not a digit of the radix.
fn radix_number(digits: &str, radix: u32) -> f64 {
    if digits.is_empty() {
        return f64::NAN;
    }
    let mut value = 0.0;
    for character in digits.chars() {
        match character.to_digit(radix) {
            Some(digit) => value = value * f64::from(radix) + f64::from(digit),
            None => return f64::NAN,
        }
    }
    value
}

/// Whether the text is a decimal literal without a sign: digits with an optional point and an
/// optional exponent, with at least one digit before or after the point.
fn is_decimal_literal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let length = bytes.len();
    let mut index = 0;
    let mut digits = 0;
    while index < length && bytes[index].is_ascii_digit() {
        index += 1;
        digits += 1;
    }
    if index < length && bytes[index] == b'.' {
        index += 1;
        while index < length && bytes[index].is_ascii_digit() {
            index += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return false;
    }
    if index < length && matches!(bytes[index], b'e' | b'E') {
        index += 1;
        if index < length && matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        let exponent_start = index;
        while index < length && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == exponent_start {
            return false;
        }
    }
    index == length
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthiness_follows_javascript() {
        assert!(!truthy(&None));
        assert!(!truthy(&Some(Value::Null)));
        assert!(!truthy(&Some(Value::Number(0.0))));
        assert!(!truthy(&Some(Value::Number(f64::NAN))));
        assert!(!truthy(&text("")));
        assert!(!truthy(&Some(Value::Bool(false))));
        assert!(truthy(&Some(Value::Number(-1.0))));
        assert!(truthy(&Some(Value::Array(Vec::new()))));
        assert!(truthy(&text("0")));
    }

    #[test]
    fn numbers_convert_as_javascript_converts_them() {
        assert_eq!(to_number(&Some(Value::String("8".to_owned()))), 8.0);
        assert_eq!(to_number(&Some(Value::String(" 0x10 ".to_owned()))), 16.0);
        assert_eq!(to_number(&Some(Value::String(String::new()))), 0.0);
        assert!(to_number(&Some(Value::String("8px".to_owned()))).is_nan());
        assert_eq!(to_number(&Some(Value::Bool(true))), 1.0);
        assert_eq!(to_number(&Some(Value::Null)), 0.0);
        assert_eq!(
            to_number(&Some(Value::Array(vec![Value::Number(5.0)]))),
            5.0
        );
        assert!(to_number(&Some(Value::Object(BTreeMap::new()))).is_nan());
        assert!(to_number(&None).is_nan());
        assert_eq!(to_number(&text("5.")), 5.0);
        assert_eq!(to_number(&text(".5e1")), 5.0);
    }

    #[test]
    fn an_object_override_keeps_the_generated_keys() {
        let generated = Some(Value::Object(BTreeMap::from([
            ("a".to_owned(), Value::Number(1.0)),
            ("b".to_owned(), Value::Number(2.0)),
        ])));
        let over = Value::Object(BTreeMap::from([("b".to_owned(), Value::Number(3.0))]));
        let merged = apply_override(generated, &over);
        assert_eq!(
            merged,
            Value::Object(BTreeMap::from([
                ("a".to_owned(), Value::Number(1.0)),
                ("b".to_owned(), Value::Number(3.0)),
            ]))
        );
        assert_eq!(
            apply_override(text("x"), &Value::Number(4.0)),
            Value::Number(4.0)
        );
    }

    #[test]
    fn mk_border_darkens_on_light_and_lightens_on_dark() {
        assert_eq!(
            mk_border(&text("#28253D"), false).ok().flatten(),
            Some(Value::String("hsl(247.5, 0%, 9.2156862745%)".to_owned()))
        );
        assert_eq!(
            mk_border(&text("#28253D"), true).ok().flatten(),
            Some(Value::String("hsl(247.5, 0%, 29.2156862745%)".to_owned()))
        );
    }
}
