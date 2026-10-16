//! Reading theme variables for the SVG writers: colour tokens as text, sizes in pixels, lists of
//! colours, and the truthiness the reference's JavaScript gives a variable. Shared by the pie and the
//! flowchart.

use ferrigura_model::value::Value;

use crate::svg::number;

/// A variable as text: a string as it is, a number as JavaScript prints it.
pub(crate) fn text_of(variables: &Value, name: &str) -> Option<String> {
    match variables.get(name)? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number_value) => Some(number(*number_value)),
        _ => None,
    }
}

/// The items of a list variable as text, in order; items that are not text or numbers are skipped.
pub(crate) fn texts_of(variables: &Value, name: &str) -> Vec<String> {
    match variables.get(name) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(text) => Some(text.clone()),
                Value::Number(number_value) => Some(number(*number_value)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether a variable is truthy as JavaScript reads it: `true`, a non-empty string, a non-zero
/// number. A missing variable is not truthy.
pub(crate) fn truthy(variables: &Value, name: &str) -> bool {
    match variables.get(name) {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Number(number_value)) => *number_value != 0.0 && !number_value.is_nan(),
        Some(Value::Array(_) | Value::Object(_)) => true,
        Some(Value::Null) | None => false,
    }
}

/// The leading pixel count of a font size or stroke width, as `parseFontSize` reads it.
pub(crate) fn px(variables: &Value, name: &str) -> Option<f64> {
    leading_integer(&text_of(variables, name)?)
}

/// `parseInt(text, 10)` for the leading digits of a size such as `25px`.
pub(crate) fn leading_integer(text: &str) -> Option<f64> {
    let trimmed = text.trim_start();
    let digits: String = trimmed
        .char_indices()
        .take_while(|(index, c)| c.is_ascii_digit() || (*index == 0 && (*c == '-' || *c == '+')))
        .map(|(_, c)| c)
        .collect();
    digits.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// Text for a `<style>` element: the characters that would end the element or start a reference
/// are escaped, and the quotes of CSS are kept as they are.
pub(crate) fn css_text(css: &str) -> String {
    css.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
