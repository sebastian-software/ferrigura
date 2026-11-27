//! What the custom renderers of the reference share: the text measurement of `utils.ts`
//! ([`text`]), the drawing helpers of `diagrams/common/svgDrawCommon.ts` ([`draw`], whose URL
//! sanitizer is [`ferrigura_syntax::url::sanitize_url`]), the sizing of `setupGraphViewbox.js`
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

use std::borrow::Cow;

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
/// holds `&amp;` after sanitizing therefore shows `&amp;`, as the reference's does.
#[must_use]
pub fn shown_text(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    ferrigura_syntax::sanitize::decode_references(&decode_entities(&escaped))
}

/// Resolves Mermaid's internal entity placeholders (`ﬂ°°<digits>¶ß` and `ﬂ°<name>¶ß`) into their
/// characters using HTML attribute-value reference rules while leaving literal `&`, `<`, and `>`
/// outside placeholders unchanged. Returns a borrowed slice without allocating when no placeholder
/// is present.
#[must_use]
pub(crate) fn resolve_entity_placeholders(text: &str) -> Cow<'_, str> {
    if !text.contains("ﬂ°") {
        return Cow::Borrowed(text);
    }
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    Cow::Owned(ferrigura_syntax::sanitize::decode_attribute_references(
        &decode_entities(&escaped),
    ))
}

/// Resolves Mermaid's entity placeholders inside a URL at an SVG sink, handling both literal
/// placeholders (`ﬂ°°<digits>¶ß` / `ﬂ°<name>¶ß`) and placeholders percent-encoded by WHATWG URL
/// normalization (`%EF%AC%82%C2%B0(%C2%B0)?<name>%C2%B6%C3%9F`) without decoding unrelated `%XX`
/// URL escapes.
#[must_use]
pub(crate) fn resolve_url_entity_placeholders(url: &str) -> Cow<'_, str> {
    const PCT_OPEN: &str = "%EF%AC%82%C2%B0";
    const PCT_NUM: &str = "%C2%B0";
    const PCT_CLOSE: &str = "%C2%B6%C3%9F";
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    if !url.contains("ﬂ°") && !url.contains(PCT_OPEN) {
        return Cow::Borrowed(url);
    }

    let mut working = Cow::Borrowed(url);
    if url.contains(PCT_OPEN) {
        let mut out = String::with_capacity(url.len());
        let mut cursor = 0;
        while let Some(rel) = url[cursor..].find(PCT_OPEN) {
            let start = cursor + rel;
            out.push_str(&url[cursor..start]);
            let mut pos = start + PCT_OPEN.len();
            let numeric = url[pos..].starts_with(PCT_NUM);
            if numeric {
                pos += PCT_NUM.len();
            }
            let body_start = pos;
            while pos < url.len()
                && (url.as_bytes()[pos].is_ascii_alphanumeric() || url.as_bytes()[pos] == b'_')
            {
                pos += 1;
            }
            if pos > body_start && url[pos..].starts_with(PCT_CLOSE) {
                let body = &url[body_start..pos];
                let reference = if numeric {
                    format!("&#{body};")
                } else {
                    format!("&{body};")
                };
                let decoded = ferrigura_syntax::sanitize::decode_attribute_references(&reference);
                for byte in decoded.bytes() {
                    if byte <= 0x20 || matches!(byte, b'"' | b'<' | b'>' | b'`') || byte >= 0x7F {
                        out.push('%');
                        out.push(HEX[(byte >> 4) as usize] as char);
                        out.push(HEX[(byte & 0x0F) as usize] as char);
                    } else {
                        out.push(byte as char);
                    }
                }
                cursor = pos + PCT_CLOSE.len();
            } else {
                out.push_str(PCT_OPEN);
                cursor = start + PCT_OPEN.len();
            }
        }
        out.push_str(&url[cursor..]);
        working = Cow::Owned(out);
    }

    if working.contains("ﬂ°") {
        Cow::Owned(resolve_entity_placeholders(working.as_ref()).into_owned())
    } else {
        working
    }
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
