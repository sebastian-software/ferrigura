//! The `@{ … }` metadata of agentflow: js-yaml with `JSON_SCHEMA` on the block, a retry without the
//! line-trailing commas of a multi-line block, the removal of the prototype keys, and the
//! JavaScript view of the value the DB spreads into its records (`agentflowDb.ts:368-400`,
//! `:124-139`, `:383-391`).

use std::collections::BTreeMap;

use ferrigura_model::value::Value;

use crate::text::is_js_space;
use crate::yaml::{Yaml, load};

/// The keys that `stripPrototypeKeys` removes at every depth.
const PROTOTYPE_KEYS: [&str; 3] = ["__proto__", "constructor", "prototype"];

/// `addVertex`'s reading of a metadata block: the value js-yaml gives the block, with the braces
/// around a single-line block and without them around a multi-line one. A block that does not
/// read is read again without its line-trailing commas, when it has any (`agentflowDb.ts:368-391`).
/// A block that is blank reads as `{}`, as `?? {}` gives it.
pub(crate) fn parse_metadata(block: &str) -> Result<Value, String> {
    let multi_line = block.contains('\n');
    let wrapped = if multi_line {
        format!("{block}\n")
    } else {
        format!("{{\n{block}\n}}")
    };
    match load(&wrapped) {
        Ok(doc) => Ok(strip_prototype_keys(document(doc))),
        Err(error) => {
            if multi_line {
                let stripped = strip_line_trailing_commas(block);
                if stripped != block {
                    return match load(&format!("{stripped}\n")) {
                        Ok(doc) => Ok(strip_prototype_keys(document(doc))),
                        Err(_) => Err(error.message),
                    };
                }
            }
            Err(error.message)
        }
    }
}

/// The value of a document, with an empty one read as `{}`.
fn document(doc: Option<Yaml>) -> Value {
    match doc {
        None | Some(Yaml::Null) => Value::Object(BTreeMap::new()),
        Some(value) => Value::from(&value),
    }
}

/// `stripPrototypeKeys`: removes the keys of [`PROTOTYPE_KEYS`] from every object, recursively.
pub(crate) fn strip_prototype_keys(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(key, _)| !PROTOTYPE_KEYS.contains(&key.as_str()))
                .map(|(key, value)| (key, strip_prototype_keys(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(strip_prototype_keys).collect()),
        other => other,
    }
}

/// JavaScript truthiness of a value: blank strings, zero, `NaN`, `false` and `null` are false.
pub(crate) fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => *number != 0.0 && !number.is_nan(),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// The own entries that `{ …value }` spreads: the keys of an object, the indices of an array, and
/// the UTF-16 units of a string. Other values spread nothing.
pub(crate) fn spread(value: &Value) -> BTreeMap<String, Value> {
    match value {
        Value::Object(map) => map.clone(),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), item.clone()))
            .collect(),
        Value::String(text) => text
            .encode_utf16()
            .enumerate()
            .map(|(index, unit)| {
                let unit = char::from_u32(u32::from(unit)).unwrap_or('\u{fffd}');
                (index.to_string(), Value::String(unit.to_string()))
            })
            .collect(),
        _ => BTreeMap::new(),
    }
}

/// `{ ...existing, ...doc }`: the entries of `doc` replace those of `existing`.
pub(crate) fn merge_metadata(
    existing: Option<BTreeMap<String, Value>>,
    doc: Option<&Value>,
) -> BTreeMap<String, Value> {
    let mut merged = existing.unwrap_or_default();
    if let Some(doc) = doc {
        merged.extend(spread(doc));
    }
    merged
}

/// `String(value)` for the values a label can be: a string as it is, a number, a boolean, an
/// array joined with commas, and any object as `[object Object]`.
pub(crate) fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => js_number_text(*number),
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                other => js_string(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// `JSON.stringify(value)`, for the message of an unknown shape.
pub(crate) fn json_text(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// The text of a number as JavaScript writes it, for the common cases: the sentinels, the whole
/// numbers, and the shortest decimal otherwise.
fn js_number_text(number: f64) -> String {
    if number.is_nan() {
        return "NaN".to_owned();
    }
    if number.is_infinite() {
        return if number > 0.0 {
            "Infinity"
        } else {
            "-Infinity"
        }
        .to_owned();
    }
    if number.fract() == 0.0 && number.abs() < 1e21 {
        return format!("{number:.0}");
    }
    format!("{number}")
}

/// `stripLineTrailingCommas(body)`: removes the commas that end a line of a block, except inside
/// quotes, flow collections and block scalars (`agentflowDb.ts:153-210`). The state of the quotes
/// and of the flow depth carries from one line to the next.
pub(crate) fn strip_line_trailing_commas(body: &str) -> String {
    let mut in_single = false;
    let mut in_double = false;
    let mut flow_depth: usize = 0;
    let mut block_scalar_indent: Option<usize> = None;
    let mut out: Vec<String> = Vec::new();
    for line in body.split('\n') {
        let indent = leading_blanks(line);
        if let Some(scalar_indent) = block_scalar_indent {
            if line.chars().all(is_js_space) || indent > scalar_indent {
                // Block scalar content is verbatim text: its commas are content.
                out.push(line.to_owned());
                continue;
            }
            block_scalar_indent = None;
        }
        let chars: Vec<char> = line.chars().collect();
        let mut comment_start: Option<usize> = None;
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            if in_double {
                if ch == '\\' {
                    i += 1;
                } else if ch == '"' {
                    in_double = false;
                }
            } else if in_single {
                if ch == '\'' {
                    in_single = false;
                }
            } else if ch == '"' {
                in_double = true;
            } else if ch == '\'' {
                in_single = true;
            } else if ch == '#' && (i == 0 || chars[i - 1] == ' ' || chars[i - 1] == '\t') {
                comment_start = Some(i);
                break;
            } else if ch == '[' || ch == '{' {
                flow_depth += 1;
            } else if ch == ']' || ch == '}' {
                flow_depth = flow_depth.saturating_sub(1);
            }
            i += 1;
        }
        if in_single || in_double || flow_depth > 0 {
            // The line ends inside a quoted scalar or a flow collection: its comma is not a stray.
            out.push(line.to_owned());
            continue;
        }
        let split = comment_start.unwrap_or(chars.len());
        let code: String = chars[..split].iter().collect();
        let comment: String = chars[split..].iter().collect();
        if opens_block_scalar(&code) {
            block_scalar_indent = Some(indent);
            out.push(line.to_owned());
            continue;
        }
        out.push(strip_trailing_comma(&code) + &comment);
    }
    out.join("\n")
}

/// The number of leading blanks of a line, in characters.
fn leading_blanks(line: &str) -> usize {
    line.chars().take_while(|&c| is_js_space(c)).count()
}

/// `/:\s*[>|][\d+-]*\s*$/`: a key whose value opens a block scalar.
fn opens_block_scalar(code: &str) -> bool {
    let Some(colon) = code.rfind(':') else {
        return false;
    };
    let after = code[colon + 1..].trim_start_matches(is_js_space);
    let Some(rest) = after.strip_prefix(['>', '|']) else {
        return false;
    };
    rest.trim_end_matches(is_js_space)
        .chars()
        .all(|c| c.is_ascii_digit() || c == '+' || c == '-')
}

/// `code.replace(/,([\t ]*)$/, '$1')`: the comma that ends the code, if any, goes.
fn strip_trailing_comma(code: &str) -> String {
    let trimmed = code.trim_end_matches([' ', '\t']);
    match trimmed.strip_suffix(',') {
        Some(rest) => format!("{rest}{}", &code[trimmed.len()..]),
        None => code.to_owned(),
    }
}
