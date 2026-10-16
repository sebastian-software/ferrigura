//! Helpers shared by the integration tests: the Chromium reference and the style names it uses.

#![allow(dead_code)]

use ferrigura_text::{Face, FontFamily, FontStyle, FontWeight, face};
use serde_json::Value;

/// The largest difference allowed between a width or box measured here and in Chromium.
///
/// Chromium lays text out in units of 1/64 px, so a width can differ from the exact sum of
/// advances by up to 0.0156 px.
pub const CHROMIUM_TOLERANCE: f64 = 0.02;

/// The Chromium reference in `fixtures/chromium.json`.
///
/// Playwright's Chromium drew every request with the spike's font files embedded as data URLs.
/// Widths are the drawn extent of a text element, from the start of its first character to the end
/// of its last one, read with `getStartPositionOfChar` and `getEndPositionOfChar`. Box ascent and
/// descent are the canvas font bounding box, which Chromium rounds to whole pixels. Wrapped lines
/// are Mermaid's `splitLineToFitWidth` (`rendering-util/splitText.ts`), ported line for line and run
/// in the browser. `getComputedTextLength` and `getBBox` are not used: WebKit reports unkerned
/// widths through them. The extents agree with the spike's `getComputedTextLength` to 0.0 px on
/// the same labels, and the box heights with its `getBBox` exactly.
pub fn chromium() -> Value {
    serde_json::from_str(include_str!("../fixtures/chromium.json"))
        .expect("chromium.json is valid JSON")
}

/// The entries of one section of the Chromium reference.
pub fn entries(reference: &Value, section: &str) -> Vec<Value> {
    reference[section]
        .as_array()
        .unwrap_or_else(|| panic!("chromium.json has no `{section}` array"))
        .clone()
}

/// The bundled face for a style name of the fixture, such as `sans-bolditalic`.
pub fn face_for(style: &str) -> &'static Face {
    let (family, rest) = style
        .split_once('-')
        .unwrap_or_else(|| panic!("unknown style {style}"));
    let family = match family {
        "sans" => FontFamily::Sans,
        "mono" => FontFamily::Mono,
        _ => panic!("unknown family in {style}"),
    };
    let (weight, slant) = match rest {
        "regular" => (FontWeight::Normal, FontStyle::Normal),
        "italic" => (FontWeight::Normal, FontStyle::Italic),
        "bold" => (FontWeight::Bold, FontStyle::Normal),
        "bolditalic" => (FontWeight::Bold, FontStyle::Italic),
        _ => panic!("unknown style {style}"),
    };
    face(family, weight, slant)
}

/// The text of a fixture entry.
pub fn text_of(entry: &Value) -> &str {
    entry["text"].as_str().expect("entry has text")
}

/// The style name of a fixture entry.
pub fn style_of(entry: &Value) -> &str {
    entry["style"].as_str().expect("entry has style")
}

/// The font size of a fixture entry.
pub fn size_of(entry: &Value) -> f64 {
    entry["size"].as_f64().expect("entry has size")
}

/// A number stored in a fixture entry under `key`.
pub fn number(entry: &Value, key: &str) -> f64 {
    entry[key]
        .as_f64()
        .unwrap_or_else(|| panic!("entry has no number `{key}`"))
}
