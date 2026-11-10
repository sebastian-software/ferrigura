//! Helpers shared by the integration tests: the Chromium reference and the style names it uses.

#![allow(dead_code)]

use ferrigura_text::{Face, FontRegistry, FontStyle, FontWeight};
use serde_json::Value;
use std::sync::{Arc, OnceLock};

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
/// descent are read from `getBBox` of a `<text>` at a known baseline, and the canvas font bounding
/// box gives the same whole pixels. Wrapped lines are Mermaid's `splitLineToFitWidth`
/// (`rendering-util/splitText.ts`), ported line for line and run in the browser. The fixture is
/// Chromium only: WebKit reports unkerned widths through `getComputedTextLength` and `getBBox`, and
/// its box heights differ by up to 0.38 px. The extents agree with the spike's
/// `getComputedTextLength` to 0.0 px on the same labels, and the box heights with its `getBBox`
/// exactly.
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

/// The explicitly registered fixture face for a style name, such as `sans-bolditalic`.
pub fn face_for(style: &str) -> &'static Face {
    static FACES: OnceLock<Vec<(String, Arc<Face>)>> = OnceLock::new();
    let faces = FACES.get_or_init(|| {
        let mut registry = FontRegistry::new();
        let mut choices = Vec::new();
        for (family, prefix) in [
            ("sans", "LexicaUltralegible"),
            ("mono", "AtkinsonHyperlegibleMono"),
        ] {
            for (suffix, file_style, weight, style) in [
                ("regular", "Regular", FontWeight::Normal, FontStyle::Normal),
                ("italic", "Italic", FontWeight::Normal, FontStyle::Italic),
                ("bold", "Bold", FontWeight::Bold, FontStyle::Normal),
                (
                    "bolditalic",
                    "BoldItalic",
                    FontWeight::Bold,
                    FontStyle::Italic,
                ),
            ] {
                registry
                    .register_file(
                        family,
                        format!(
                            "{}/fonts/{prefix}-{file_style}.ttf",
                            env!("CARGO_MANIFEST_DIR")
                        ),
                        weight,
                        style,
                    )
                    .expect("fixture registers");
                choices.push((format!("{family}-{suffix}"), family, weight, style));
            }
        }
        for (family, file) in [
            ("arrows", "NotoSansMath-Regular-Arrows.ttf"),
            ("symbols", "NotoSansSymbols2-Regular-TextSymbols.ttf"),
            ("emoji", "TwemojiMozilla-COLRv0-Emoji.ttf"),
        ] {
            registry
                .register_file(
                    family,
                    format!("{}/fonts/{file}", env!("CARGO_MANIFEST_DIR")),
                    FontWeight::Normal,
                    FontStyle::Normal,
                )
                .expect("fallback fixture registers");
        }
        registry.set_fallbacks(["arrows", "symbols", "emoji"]);
        choices
            .into_iter()
            .map(|(name, family, weight, style)| {
                (
                    name,
                    registry
                        .resolve(family, weight, style)
                        .expect("fixture resolves"),
                )
            })
            .collect()
    });
    &faces
        .iter()
        .find(|(name, _)| name == style)
        .unwrap_or_else(|| panic!("unknown fixture style {style}"))
        .1
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
