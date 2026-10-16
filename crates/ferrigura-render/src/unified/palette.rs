//! Colour slots and node styles that the shapes and clusters of the unified renderer share: the
//! palette of a colour theme (`diagrams/common/colorThemeGate.ts`) and the style declarations of a
//! node as `styles2String` writes them (`shapes/handDrawnShapeStyles.ts`).

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;

use super::data::Node;
use crate::variables::texts_of;

/// The colour themes that carry a palette (`COLOR_THEMES` in `diagrams/common/colorThemeGate.ts`).
pub(crate) const COLOUR_THEMES: [&str; 2] = ["redux-color", "redux-dark-color"];

/// The palette of a theme that has one: the border colours, one per slot, and the fills.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Palette {
    /// `borderColorArray`. The number of slots is its length.
    pub border: Vec<String>,
    /// `bkgColorArray`: the fill of a slot is the entry at the slot's index modulo the length. Empty
    /// when the theme has no fills, as the dark theme has none.
    pub background: Vec<String>,
}

/// The palette of a configuration: only the colour themes have one, and only with border colours
/// (`isColorTheme` in `colorThemeGate.ts`).
pub(crate) fn palette(config: &MermaidConfig, variables: &Value) -> Option<Palette> {
    if !COLOUR_THEMES.contains(&config.theme.as_str()) {
        return None;
    }
    let border = texts_of(variables, "borderColorArray");
    if border.is_empty() {
        return None;
    }
    Some(Palette {
        border,
        background: texts_of(variables, "bkgColorArray"),
    })
}

/// The palette slot of an element with a colour index: the index wrapped to the palette, as
/// `stampColorSlot` writes it.
pub(crate) fn palette_slot(index: Option<u32>, palette: &Palette) -> Option<usize> {
    index.map(|index| index as usize % palette.border.len())
}

/// The declarations of a node's style as `styles2String` writes them for its rect: the compiled styles
/// of its classes, then its own `style` statements, then its label style. A key keeps the position it
/// first had, and the last value wins. Label keys are left out, since they style the label.
#[must_use]
pub(crate) fn node_style(node: &Node) -> String {
    let label = style_list(&node.label_style);
    let mut map: Vec<(String, String)> = Vec::new();
    for style in node
        .css_compiled_styles
        .iter()
        .chain(&node.css_styles)
        .chain(&label)
    {
        let mut parts = style.split(':');
        let key = parts.next().unwrap_or_default().trim().to_owned();
        let value = parts
            .next()
            .map(|value| value.trim().to_owned())
            .unwrap_or_default();
        match map.iter_mut().find(|(existing, _)| *existing == key) {
            Some(entry) => entry.1 = value,
            None => map.push((key, value)),
        }
    }
    map.iter()
        .filter(|(key, _)| !is_label_style(key))
        .map(|(key, value)| format!("{key}:{value} !important"))
        .collect::<Vec<_>>()
        .join(";")
}

/// The entries of a style string: `;`-separated, trimmed, without the empty ones (`normalizeStyleList`).
#[must_use]
pub(crate) fn style_list(text: &str) -> Vec<String> {
    text.split(';')
        .map(str::trim)
        .filter(|style| !style.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The keys that style the label rather than the shape (`isLabelStyle` in `handDrawnShapeStyles.ts`).
fn is_label_style(key: &str) -> bool {
    matches!(
        key,
        "color"
            | "font-size"
            | "font-family"
            | "font-weight"
            | "font-style"
            | "text-decoration"
            | "text-align"
            | "text-transform"
            | "line-height"
            | "letter-spacing"
            | "word-spacing"
            | "text-shadow"
            | "text-overflow"
            | "white-space"
            | "word-wrap"
            | "word-break"
            | "overflow-wrap"
            | "hyphens"
    )
}
