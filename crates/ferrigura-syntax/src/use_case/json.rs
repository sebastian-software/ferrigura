//! Port of `packages/mermaid/src/diagrams/usecase/parser/usecaseJson.ts`: the object text of a use
//! case `json` statement, read as `JSON.parse` reads it, together with the order of its keys.
//!
//! The reference runs `JSON.parse` for validity and value, then walks the same text again for the
//! order of the keys, because a JavaScript object does not keep the text order of its keys (the
//! integer-like keys come first). This port reads the text once and does both at the same time: the
//! value is built as the parser reads it, and the keys of each object are recorded in text order as
//! they are read. The result is the reference's: its walk only runs over text that `JSON.parse` has
//! accepted, and it records the same entries that this single pass records.
//!
//! Where the port differs from the JavaScript values:
//!
//! - A lone surrogate, written as a `\u` escape, becomes U+FFFD. JavaScript keeps it as a code unit,
//!   but a Rust `String` holds only scalar values. A high surrogate escape that is followed by a low
//!   surrogate escape is one code point, as in JavaScript.
//! - The keys of `Value::Object` are sorted, as a `BTreeMap` sorts them, so the order of
//!   `Object.keys` is not reproduced. The text order is in `property_order`, which is what the
//!   renderer reads.
//! - Arrays and objects nested deeper than `MAX_DEPTH` are an error. The reference's walk is
//!   recursive too, and it fails on very deep text, but at a depth that the reference does not state.
//!
//! The offset of an error is the byte offset of the first character that makes the text invalid, or
//! the length of the text when it ends too early. This is the position that V8 reports for
//! `JSON.parse`, and the one the reference's walk falls back to when the message has no position.
//!
//! The rows of a JSON table, which the reference builds with `flattenJsonRows` (`usecaseDb.ts`
//! 149-211), are `json_rows`. It reads the value and the property order that this module returns.

use std::collections::BTreeMap;
use std::fmt;

use ferrigura_model::use_case::JsonRow;
use ferrigura_model::value::Value;

use crate::sanitize::sanitize_text;

/// The deepest nesting of arrays and objects that is accepted, the root object included.
const MAX_DEPTH: usize = 512;

/// `OrderedJson` of the reference: the object and the order of its keys.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderedJsonObject {
    /// The object, as `JSON.parse` gives it. A number beyond the range of a double is infinite,
    /// as in JavaScript (`1e400` is infinity and `-1e400` is negative infinity), and a repeated key
    /// keeps its last value. Its keys are in a `BTreeMap`, so their text order is in
    /// `property_order` instead.
    pub value: Value,
    /// The keys of every object reached, keyed by its JSON Pointer (RFC 6901), the way the reference's
    /// `flattenJsonRows` looks them up. The root is the empty string. A property of an object is the
    /// pointer of the object, `/` and the key, with `~` written as `~0` and `/` as `~1`. An element of
    /// an array is the pointer of the array, `/` and its index. Every object has an entry, its keys
    /// in text order. A repeated key is listed once, at its first position, and its value is the last.
    pub property_order: BTreeMap<String, Vec<String>>,
}

/// A failure, with the byte offset of the offending character in the text given to the parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    /// The byte offset of the first character that makes the text invalid. It is the length of the
    /// text when the text ends too early, and 0 when the root is not an object.
    pub offset: usize,
    /// What is wrong, in English.
    pub message: String,
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Invalid JSON at byte {}: {}",
            self.offset, self.message
        )
    }
}

impl std::error::Error for JsonError {}

/// `parseOrderedJsonObject(text)`: `JSON.parse` for validity and value, then the property order.
///
/// The text is JSON as RFC 8259 defines it: the whitespace is space, tab, LF and CR only, and no
/// content may follow the value. The root must be an object.
///
/// # Errors
///
/// Fails when the text is not JSON, when its root is not an object (`null`, an array, a number, a
/// string or a boolean), and when it nests deeper than the parser allows. A root that is not an
/// object fails at offset 0, the start of the text, as the reference reports it.
pub fn parse_ordered_json_object(text: &str) -> Result<OrderedJsonObject, JsonError> {
    let mut parser = Parser::new(text);
    let root = parser.parse_value("")?;
    parser.finish()?;
    if !root.is_object() {
        return Err(error(0, "JSON value must have an object root"));
    }
    Ok(OrderedJsonObject {
        value: root,
        property_order: parser.property_order,
    })
}

/// `String(number)` in JavaScript: the Number-to-String conversion of ECMA-262 (6.1.6.1.20).
///
/// `NaN`, `Infinity` and `-Infinity` are spelled out, and both zeros give `"0"`. Otherwise the digits
/// are the shortest that read back as the same double, and the number is written in plain notation
/// when its decimal exponent `n` (the position of the point) is from -5 to 21, and in exponent
/// notation (`1e-7`, `1.5e+21`) otherwise.
#[must_use]
pub fn js_number_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let (digits, point) = shortest_digits(value.abs());
    format!("{sign}{}", number_body(&digits, point))
}

/// The shortest digits that read back as `magnitude`, and the position `n` of the decimal point
/// relative to them: `magnitude` is `0.digits` times ten to the power `n`. `magnitude` is finite,
/// positive and not zero.
///
/// ECMA-262 takes the candidate closest to `magnitude` among the shortest ones that read back, and
/// the even one when two are equally close. `{:e}` rounds such an exact tie up, so the digits are
/// corrected by `even_tie_digits`.
fn shortest_digits(magnitude: f64) -> (String, i64) {
    // `{:e}` without a precision writes the shortest digits that round-trip, as in `1.2345e3`.
    let scientific = format!("{magnitude:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .unwrap_or((scientific.as_str(), "0"));
    let collected: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = collected.trim_end_matches('0');
    let point = exponent.parse::<i64>().unwrap_or(0) + 1;
    match even_tie_digits(magnitude, digits, point) {
        Some(corrected) => corrected,
        None => (digits.to_owned(), point),
    }
}

/// The digits that ECMA-262 picks when `magnitude` lies exactly halfway between the shortest digits
/// and a neighbour, and the even one of the two reads back. `digits` are the shortest digits `D` of
/// `magnitude`, so that `magnitude` is `D` times ten to the power `scale`. The neighbours are `D - 1`
/// and `D + 1`, and the midpoint of `D` and a neighbour is `(2D ± 1) * 10^scale / 2`.
fn even_tie_digits(magnitude: f64, digits: &str, point: i64) -> Option<(String, i64)> {
    let shortest: u64 = digits.parse().ok()?;
    if shortest == 0 {
        return None;
    }
    let scale = point.checked_sub(i64::try_from(digits.len()).ok()?)?;
    let (mantissa, exponent) = odd_part(magnitude);
    let twice = u128::from(shortest) * 2;
    let neighbours = [(shortest - 1, twice - 1), (shortest + 1, twice + 1)];
    for (neighbour, twice_midpoint) in neighbours {
        if is_midpoint(twice_midpoint, scale, mantissa, exponent) {
            let chosen = if shortest.is_multiple_of(2) {
                shortest
            } else {
                neighbour
            };
            return if chosen == shortest {
                None
            } else {
                reads_back(chosen, scale, magnitude)
            };
        }
    }
    None
}

/// The digits `chosen`, times ten to the power `scale`, with their point, when they read back as
/// `magnitude`.
fn reads_back(chosen: u64, scale: i64, magnitude: f64) -> Option<(String, i64)> {
    let text = chosen.to_string();
    let read: f64 = format!("{text}e{scale}").parse().ok()?;
    if read != magnitude {
        return None;
    }
    let point = scale.checked_add(i64::try_from(text.len()).ok()?)?;
    Some((text.trim_end_matches('0').to_owned(), point))
}

/// Whether `twice_midpoint * 10^scale / 2` is exactly `mantissa * 2^exponent`, where `twice_midpoint`
/// and `mantissa` are odd. The midpoint is `twice_midpoint * 5^scale * 2^(scale - 1)`, so the two sides
/// agree when the powers of two agree and the odd parts agree. The odd part of the midpoint is an
/// integer only when its power of five divides `twice_midpoint`.
fn is_midpoint(twice_midpoint: u128, scale: i64, mantissa: u64, exponent: i64) -> bool {
    if exponent != scale - 1 {
        return false;
    }
    let mantissa = u128::from(mantissa);
    if scale >= 0 {
        // Above 22 the power of five exceeds the mantissa, which is below 2^53.
        let Ok(power) = u32::try_from(scale) else {
            return false;
        };
        if power > 22 {
            return false;
        }
        twice_midpoint * 5u128.pow(power) == mantissa
    } else {
        // Above 40 the power of five exceeds the midpoint, which is below 2^59.
        let Ok(power) = u32::try_from(scale.unsigned_abs()) else {
            return false;
        };
        if power > 40 {
            return false;
        }
        let five = 5u128.pow(power);
        twice_midpoint.is_multiple_of(five) && twice_midpoint / five == mantissa
    }
}

/// The odd part of `magnitude`, a positive finite number: `magnitude` is `mantissa` times `2^exponent`,
/// with `mantissa` odd.
fn odd_part(magnitude: f64) -> (u64, i64) {
    let bits = magnitude.to_bits();
    let biased = i64::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let fraction = bits & ((1 << 52) - 1);
    let (mantissa, exponent) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), biased - 1075)
    };
    let shift = mantissa.trailing_zeros();
    (mantissa >> shift, exponent + i64::from(shift))
}

/// The digits of a finite, non-zero, positive number, placed as ECMA-262 places them.
fn number_body(digits: &str, point: i64) -> String {
    let count = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    if count <= point && point <= 21 {
        let zeros = usize::try_from(point - count).unwrap_or(0);
        format!("{digits}{}", "0".repeat(zeros))
    } else if 0 < point && point <= 21 {
        let split = usize::try_from(point).unwrap_or(0);
        let (whole, fraction) = (digits.get(..split).unwrap_or(digits), digits.get(split..));
        format!("{whole}.{}", fraction.unwrap_or(""))
    } else if -6 < point && point <= 0 {
        let zeros = usize::try_from(-point).unwrap_or(0);
        format!("0.{}{digits}", "0".repeat(zeros))
    } else {
        let exponent = point - 1;
        let first = digits.get(..1).unwrap_or(digits);
        let rest = digits.get(1..).unwrap_or("");
        let dot = if rest.is_empty() { "" } else { "." };
        let sign = if exponent < 0 { '-' } else { '+' };
        format!("{first}{dot}{rest}e{sign}{}", exponent.abs())
    }
}

/// The rows of a JSON table: `flattenJsonRows(value, propertyOrder, sanitize)` of the reference
/// (`usecaseDb.ts` 149-211), with `sanitize_text` as the sanitiser.
///
/// The rows are depth first, in the text order of the keys. An object lists its keys by its
/// `property_order` entry, or, without one, in the `BTreeMap` order of its keys. An empty array and an
/// empty object are one row each, `[]` and `{}`. An array of scalars is one row per item: the first row
/// has the path as its key, and the rows after it an empty key, so the values read as a column. Any
/// other array is listed item by item, with `[index]` added to the path. The key, the accessible key
/// and the value of every row go through `sanitize_text`.
#[must_use]
pub fn json_rows(value: &Value, property_order: &BTreeMap<String, Vec<String>>) -> Vec<JsonRow> {
    let mut rows = Vec::new();
    flatten_into(value, "", "", property_order, &mut rows);
    rows
}

/// Appends the rows of `current`, which has the display path `path` and the JSON pointer `pointer`.
fn flatten_into(
    current: &Value,
    path: &str,
    pointer: &str,
    property_order: &BTreeMap<String, Vec<String>>,
    rows: &mut Vec<JsonRow>,
) {
    match current {
        Value::Array(items) if items.is_empty() => push_row(rows, path, path, "[]"),
        Value::Array(items) if items.iter().all(is_scalar) => {
            for (index, item) in items.iter().enumerate() {
                let key = if index == 0 { path } else { "" };
                push_row(rows, key, path, &scalar_text(item));
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let element_path = format!("{path}[{index}]");
                let element_pointer = format!("{pointer}/{index}");
                flatten_into(item, &element_path, &element_pointer, property_order, rows);
            }
        }
        Value::Object(entries) => {
            let keys: Vec<&str> = match property_order.get(pointer) {
                Some(keys) => keys.iter().map(String::as_str).collect(),
                None => entries.keys().map(String::as_str).collect(),
            };
            if keys.is_empty() {
                push_row(rows, path, path, "{}");
                return;
            }
            for key in keys {
                let property_path = if path.is_empty() {
                    key.to_owned()
                } else {
                    format!("{path}.{key}")
                };
                let property_pointer = child_pointer(pointer, key);
                match entries.get(key) {
                    Some(child) => {
                        flatten_into(
                            child,
                            &property_path,
                            &property_pointer,
                            property_order,
                            rows,
                        );
                    }
                    // The reference reads an absent property as `undefined`, which `String` writes so.
                    None => push_row(rows, &property_path, &property_path, "undefined"),
                }
            }
        }
        _ => push_row(rows, path, path, &scalar_text(current)),
    }
}

/// Appends one row, with each of its three cells sanitised.
fn push_row(rows: &mut Vec<JsonRow>, key: &str, accessible_key: &str, value: &str) {
    rows.push(JsonRow {
        key: sanitize_text(key),
        accessible_key: sanitize_text(accessible_key),
        value: sanitize_text(value),
    });
}

/// Whether `value` is a scalar, which an array of scalars lists as one column.
fn is_scalar(value: &Value) -> bool {
    !matches!(value, Value::Array(_) | Value::Object(_))
}

/// The text of a scalar as `displayJsonScalar` writes it: a string as it is, and the other scalars as
/// `String()` writes them. Arrays and objects never reach this function, so they give an empty text.
fn scalar_text(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => js_number_to_string(*number),
        Value::String(text) => text.clone(),
        Value::Array(_) | Value::Object(_) => String::new(),
    }
}

/// A failure at `offset`, with `message`.
fn error(offset: usize, message: impl Into<String>) -> JsonError {
    JsonError {
        offset,
        message: message.into(),
    }
}

/// The pointer of a property: the pointer of its object, `/` and the key, escaped as RFC 6901 says.
fn child_pointer(pointer: &str, key: &str) -> String {
    format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"))
}

/// The recursive-descent reader. It decides validity the way `JSON.parse` does, builds the value,
/// and records the keys of every object in text order, so the reference's second walk is not needed.
struct Parser<'a> {
    text: &'a str,
    /// The byte offset of the next character to read.
    offset: usize,
    /// How many arrays and objects enclose the next character.
    depth: usize,
    /// The keys of every object read so far, by pointer.
    property_order: BTreeMap<String, Vec<String>>,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            offset: 0,
            depth: 0,
            property_order: BTreeMap::new(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.peek_at(self.offset)
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.text.as_bytes().get(offset).copied()
    }

    fn char_at(&self, offset: usize) -> Option<char> {
        self.text.get(offset..).and_then(|rest| rest.chars().next())
    }

    /// What is at `offset`, for a message.
    fn describe(&self, offset: usize) -> String {
        match self.char_at(offset) {
            Some(character) => format!("character {character:?}"),
            None => "end of the text".to_owned(),
        }
    }

    /// A failure at `offset`, because something else was expected there.
    fn unexpected(&self, offset: usize, expected: &str) -> JsonError {
        let message = format!("unexpected {}, expected {expected}", self.describe(offset));
        error(offset, message)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.offset += 1;
        }
    }

    fn skip_digits(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.offset += 1;
        }
    }

    /// Enters an array or an object, which must stay within `MAX_DEPTH`.
    fn enter(&mut self) -> Result<(), JsonError> {
        if self.depth >= MAX_DEPTH {
            return Err(error(
                self.offset,
                format!("the nesting is deeper than the {MAX_DEPTH} levels the parser allows"),
            ));
        }
        self.depth += 1;
        Ok(())
    }

    /// Reads one value, with the pointer it has in the document.
    fn parse_value(&mut self, pointer: &str) -> Result<Value, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(pointer),
            Some(b'[') => self.parse_array(pointer),
            Some(b'"') => self.parse_string().map(Value::String),
            Some(b't') => self.parse_literal("true", Value::Bool(true)),
            Some(b'f') => self.parse_literal("false", Value::Bool(false)),
            Some(b'n') => self.parse_literal("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            _ => Err(self.unexpected(self.offset, "a value")),
        }
    }

    fn parse_object(&mut self, pointer: &str) -> Result<Value, JsonError> {
        self.enter()?;
        self.offset += 1;
        let mut entries = BTreeMap::new();
        let mut keys = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.offset += 1;
        } else {
            loop {
                self.skip_whitespace();
                if self.peek() != Some(b'"') {
                    return Err(self.unexpected(self.offset, "a property name"));
                }
                let key = self.parse_string()?;
                let property = child_pointer(pointer, &key);
                if entries.contains_key(&key) {
                    // A repeated key keeps its first position and its last value. The subtree of
                    // the earlier value is replaced by the subtree of the later one.
                    self.remove_subtree(&property);
                } else {
                    keys.push(key.clone());
                }
                self.skip_whitespace();
                if self.peek() != Some(b':') {
                    return Err(self.unexpected(self.offset, "':' after the property name"));
                }
                self.offset += 1;
                let value = self.parse_value(&property)?;
                entries.insert(key, value);
                self.skip_whitespace();
                match self.peek() {
                    Some(b',') => self.offset += 1,
                    Some(b'}') => {
                        self.offset += 1;
                        break;
                    }
                    _ => return Err(self.unexpected(self.offset, "',' or '}'")),
                }
            }
        }
        self.depth -= 1;
        self.property_order.insert(pointer.to_owned(), keys);
        Ok(Value::Object(entries))
    }

    fn parse_array(&mut self, pointer: &str) -> Result<Value, JsonError> {
        self.enter()?;
        self.offset += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.offset += 1;
        } else {
            loop {
                let element = format!("{pointer}/{}", items.len());
                items.push(self.parse_value(&element)?);
                self.skip_whitespace();
                match self.peek() {
                    Some(b',') => self.offset += 1,
                    Some(b']') => {
                        self.offset += 1;
                        break;
                    }
                    _ => return Err(self.unexpected(self.offset, "',' or ']'")),
                }
            }
        }
        self.depth -= 1;
        Ok(Value::Array(items))
    }

    /// Drops the keys recorded for `pointer` and for everything below it.
    fn remove_subtree(&mut self, pointer: &str) {
        self.property_order.remove(pointer);
        let prefix = format!("{pointer}/");
        let descendants: Vec<String> = self
            .property_order
            .range(prefix.clone()..)
            .take_while(|(key, _)| key.starts_with(prefix.as_str()))
            .map(|(key, _)| key.clone())
            .collect();
        for key in descendants {
            self.property_order.remove(&key);
        }
    }

    /// Reads a string that starts at the current quotation mark, and returns its decoded text.
    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.offset += 1;
        let mut decoded = String::new();
        loop {
            let Some(byte) = self.peek() else {
                return Err(error(self.offset, "the string is not terminated"));
            };
            match byte {
                b'"' => {
                    self.offset += 1;
                    return Ok(decoded);
                }
                b'\\' => self.parse_escape(&mut decoded)?,
                0x00..=0x1F => {
                    let message =
                        format!("the control character U+{byte:04X} is not allowed in a string");
                    return Err(error(self.offset, message));
                }
                _ => {
                    let Some(character) = self.char_at(self.offset) else {
                        return Err(self.unexpected(self.offset, "a character"));
                    };
                    decoded.push(character);
                    self.offset += character.len_utf8();
                }
            }
        }
    }

    /// Reads the escape sequence that starts at the current backslash.
    fn parse_escape(&mut self, decoded: &mut String) -> Result<(), JsonError> {
        self.offset += 1;
        let escape_offset = self.offset;
        let Some(byte) = self.peek() else {
            return Err(error(self.offset, "the string is not terminated"));
        };
        let character = match byte {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
                self.offset += 1;
                let unit = self.parse_hex_unit()?;
                let character = self.decode_unit(unit);
                decoded.push(character);
                return Ok(());
            }
            _ => return Err(self.unexpected(escape_offset, "an escape character")),
        };
        decoded.push(character);
        self.offset += 1;
        Ok(())
    }

    /// The value of one hex digit, if the byte at `offset` is one.
    fn hex_digit_at(&self, offset: usize) -> Option<u32> {
        self.peek_at(offset)
            .and_then(|byte| char::from(byte).to_digit(16))
    }

    /// Reads the four hex digits of a `\u` escape, which starts at the current offset.
    fn parse_hex_unit(&mut self) -> Result<u32, JsonError> {
        let mut unit = 0;
        for index in 0..4_usize {
            let position = self.offset + index;
            let Some(digit) = self.hex_digit_at(position) else {
                let message = format!(
                    "expected a hex digit in a \\u escape, found {}",
                    self.describe(position)
                );
                return Err(error(position, message));
            };
            unit = unit * 16 + digit;
        }
        self.offset += 4;
        Ok(unit)
    }

    /// The character of a `\u` escape with code unit `unit`. A high surrogate reads the low
    /// surrogate escape that follows it, if there is one, and the two make one code point.
    fn decode_unit(&mut self, unit: u32) -> char {
        if (0xDC00..=0xDFFF).contains(&unit) {
            return char::REPLACEMENT_CHARACTER;
        }
        if !(0xD800..=0xDBFF).contains(&unit) {
            return char::from_u32(unit).unwrap_or(char::REPLACEMENT_CHARACTER);
        }
        match self.low_surrogate_escape_at(self.offset) {
            Some(low) => {
                self.offset += 6;
                let code = 0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00);
                char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
            }
            None => char::REPLACEMENT_CHARACTER,
        }
    }

    /// The code unit of a low surrogate `\u` escape that starts at `offset`, if there is one.
    fn low_surrogate_escape_at(&self, offset: usize) -> Option<u32> {
        if self.peek_at(offset) != Some(b'\\') || self.peek_at(offset + 1) != Some(b'u') {
            return None;
        }
        (0..4_usize)
            .try_fold(0, |unit, index| {
                self.hex_digit_at(offset + 2 + index)
                    .map(|digit| unit * 16 + digit)
            })
            .filter(|unit| (0xDC00..=0xDFFF).contains(unit))
    }

    /// Reads one of the literals `true`, `false` and `null`.
    fn parse_literal(&mut self, literal: &str, value: Value) -> Result<Value, JsonError> {
        for (index, expected) in literal.bytes().enumerate() {
            let position = self.offset + index;
            if self.peek_at(position) != Some(expected) {
                let expected = format!("'{literal}'");
                return Err(self.unexpected(position, &expected));
            }
        }
        self.offset += literal.len();
        Ok(value)
    }

    /// Reads a number of the JSON grammar and converts it as `JSON.parse` does, to the nearest double.
    fn parse_number(&mut self) -> Result<Value, JsonError> {
        let start = self.offset;
        if self.peek() == Some(b'-') {
            self.offset += 1;
        }
        match self.peek() {
            Some(b'0') => self.offset += 1,
            Some(b'1'..=b'9') => self.skip_digits(),
            _ => return Err(self.unexpected(self.offset, "a digit")),
        }
        if self.peek() == Some(b'.') {
            self.offset += 1;
            if !self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                return Err(self.unexpected(self.offset, "a digit after the decimal point"));
            }
            self.skip_digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.offset += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            if !self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                return Err(self.unexpected(self.offset, "a digit in the exponent"));
            }
            self.skip_digits();
        }
        self.text
            .get(start..self.offset)
            .and_then(|number| number.parse::<f64>().ok())
            .map(Value::Number)
            .ok_or_else(|| error(start, "the number is not valid"))
    }

    /// Reads the whole text: one value, then only whitespace.
    fn finish(&mut self) -> Result<(), JsonError> {
        self.skip_whitespace();
        if self.offset != self.text.len() {
            let message = format!(
                "unexpected {}, expected the end of the text",
                self.describe(self.offset)
            );
            return Err(error(self.offset, message));
        }
        Ok(())
    }
}
