//! What the custom renderers of the reference share: the text measurement of `utils.ts`
//! ([`text`]), the drawing helpers of `diagrams/common/svgDrawCommon.ts` ([`draw`], whose URL
//! sanitiser is [`ferrigura_syntax::url::sanitize_url`]), the sizing of `setupGraphViewbox.js`
//! ([`viewbox`]), the title of `insertTitle` ([`title`]), and `decodeEntities`
//! ([`decode_entities`]), which the unified renderer's labels use as well. The element they write
//! is [`element::Element`].

pub mod d3;
pub mod draw;
pub mod element;
pub mod style;
pub mod text;
pub mod title;
pub mod viewbox;

pub use element::Element;
pub use ferrigura_syntax::decode_entities;
pub use text::{BreakString, MeasureError, TextDimensions, TextMemo, TextStyle, WrapStyle};

/// JavaScript's `\s`: white space and the line terminators, as `String.prototype.trim` removes them.
#[must_use]
pub fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `String.prototype.trim`.
#[must_use]
pub fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_whitespace)
}

/// The text an SVG text node shows for a string of the diagram. The reference's text is the string
/// itself; its markup escapes it, and `decodeEntities` (`utils.ts` 904-906) turns the placeholders of
/// the entities the user typed into the references they stand for. The browser reads those
/// references, so the shown text is the string with its placeholders read and its references decoded
/// (`decode_references`), and the element that holds it escapes it again on output. A title that
/// holds `&amp;` after sanitising therefore shows `&amp;`, as the reference's does.
#[must_use]
pub fn shown_text(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    ferrigura_syntax::sanitize::decode_references(&decode_entities(&escaped))
}

/// `${value}`: the text that a template literal of the reference writes for a theme value. A number
/// is written as JavaScript writes it, a list as its items joined with commas (`null` items empty), an
/// object as `[object Object]`, and `null` as `null`. A theme value that is text is written as it is.
#[must_use]
pub fn template_text(value: &ferrigura_model::value::Value) -> String {
    use ferrigura_model::value::Value;
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => crate::svg::number(*number),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => "null".to_owned(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                other => template_text(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}
