//! The stylesheet of a journey: the base rules of the common style, then the rules of
//! `styles.js` that the journey's markup uses (the line and legend colours, the face, the mouth,
//! the label colour, and the colour types of the sections and tasks). The reference's rules for the
//! flowchart and the other types (`.node`, `.edgeLabel`, `.cluster`, the tooltip) match nothing in a
//! journey, and are not written.
//!
//! Every colour is a custom property declared on the diagram's id, for the light scheme and, when
//! both schemes are written, for the dark one under `prefers-color-scheme: dark` (decision 0007).

use ferrigura_model::theme::ThemeTokens;
use ferrigura_text::EmbeddedFamily;

use super::tasks::SECTION_TYPES;
use crate::common::style::base_style_sheet_tokens;
use crate::context::ThemeView;

/// The face colour: `styles.js` 29 uses `faceColor` when a theme sets it, and no shipped theme does.
const FACE_COLOUR: &str = "#FFF8DC";

/// The stroke of a face (`styles.js` 30).
const FACE_STROKE: &str = "#999999";

/// The colour of the mouth (`styles.js` 9).
const MOUTH_COLOUR: &str = "#666666";

/// The text colour when a theme gives none (the colour of the base rules).
const DEFAULT_TEXT_COLOUR: &str = "#333333";

/// The fill of a colour type when a theme gives none: the reference's initial task fill.
const DEFAULT_FILL: &str = "#CCCCCC";

/// The text colour of the theme: the colour the reference's `.label` takes (`styles.js` 4-7).
#[must_use]
pub fn text_colour(tokens: &ThemeTokens) -> String {
    tokens
        .text_color
        .text()
        .unwrap_or_else(|| DEFAULT_TEXT_COLOUR.to_owned())
}

/// The fill of each colour type, `fillType0` to `fillType{SECTION_TYPES - 1}` of the theme.
#[must_use]
pub fn fills(tokens: &ThemeTokens) -> Vec<String> {
    (0..SECTION_TYPES)
        .map(|number_type| {
            tokens
                .fill_type
                .get(number_type)
                .text()
                .unwrap_or_else(|| DEFAULT_FILL.to_owned())
        })
        .collect()
}

/// The `<style>` text of the journey for the diagram with the id `svg_id`: the base sheet, its
/// embedded faces first, then the journey's colour properties and rules. The result is escaped
/// for an XML `<style>` element, as the base sheet is; the journey's own text has no character
/// that needs escaping.
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    theme: ThemeView<'_>,
    font_family: &str,
    font_size: &str,
    families: &[EmbeddedFamily],
) -> String {
    let base = base_style_sheet_tokens(svg_id, theme, font_family, font_size, families, "");
    format!("{base}{}{}", tokens(svg_id, theme), rules(svg_id))
}

/// The colour properties of the journey on the id, for each scheme.
fn tokens(svg_id: &str, theme: ThemeView<'_>) -> String {
    match theme.dark {
        None => format!("#{svg_id}{{{}}}", declarations(theme.light)),
        Some(dark) => format!(
            "#{svg_id}{{{}}}@media (prefers-color-scheme: dark){{#{svg_id}{{{}}}}}",
            declarations(theme.light),
            declarations(dark)
        ),
    }
}

/// The properties of one scheme: the text colour, the face colour and the fill of each type.
fn declarations(tokens: &ThemeTokens) -> String {
    let mut out = format!(
        "--textColor:{};--faceColor:{FACE_COLOUR};",
        text_colour(tokens)
    );
    for (number_type, fill) in fills(tokens).iter().enumerate() {
        out.push_str(&format!("--fillType{number_type}:{fill};"));
    }
    out
}

/// The rules of `styles.js` the journey uses, on the id.
fn rules(svg_id: &str) -> String {
    let mut css = format!(
        "#{svg_id} .mouth{{stroke:{MOUTH_COLOUR};}}\
#{svg_id} line{{stroke:var(--textColor);}}\
#{svg_id} .legend{{fill:var(--textColor);}}\
#{svg_id} .label{{color:var(--textColor);}}\
#{svg_id} .face{{fill:var(--faceColor);stroke:{FACE_STROKE};}}"
    );
    for number_type in 0..SECTION_TYPES {
        css.push_str(&format!(
            "#{svg_id} .task-type-{number_type},#{svg_id} .section-type-{number_type}{{fill:var(--fillType{number_type});}}"
        ));
    }
    css
}

#[cfg(test)]
mod tests {
    use ferrigura_model::value::Value;

    use super::*;

    #[test]
    fn the_fills_are_the_theme_fill_types_in_order() {
        let variables = Value::Object(
            [
                ("fillType0".to_owned(), Value::String("#111111".to_owned())),
                ("fillType1".to_owned(), Value::String("#222222".to_owned())),
            ]
            .into_iter()
            .collect(),
        );
        let fills = fills(&ThemeTokens::from_value(&variables));
        assert_eq!(fills.len(), SECTION_TYPES);
        assert_eq!(fills[0], "#111111");
        assert_eq!(fills[1], "#222222");
        assert_eq!(fills[2], DEFAULT_FILL);
    }

    #[test]
    fn the_rules_name_the_id_and_the_colour_properties() {
        let css = rules("chart");
        assert!(css.contains("#chart .face{fill:var(--faceColor);stroke:#999999;}"));
        assert!(css.contains("#chart .section-type-6{fill:var(--fillType6);}"));
        assert!(!css.contains("type-7"));
    }
}
