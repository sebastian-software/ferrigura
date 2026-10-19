//! The base style sheet of every diagram: the rules of `packages/mermaid/src/styles.ts` that do not
//! depend on the diagram type (the font and text colour, the edge animation and thickness, the
//! markers, the `neo` look of the nodes, and `:root`'s font variable). The reference compiles them
//! with the diagram's id as the scope; here every colour is a custom property declared on the id
//! (`docs-port/decisions/0007`), and the rules refer to it with `var(…)`.
//!
//! A diagram's own rules (`styles.ts` of the type, such as `flowchart/style.rs`) come between the
//! `p` rule and the `neo` rules, as the reference's `getStyles` places them. The info diagram has no
//! rules of its own, so this sheet is its whole style.

use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::EmbeddedFamily;

use crate::context::ThemeView;
use crate::svg::font_faces;
use crate::theme::Themes;
use crate::variables::{css_text, text_of};

/// The colour tokens of the base rules, each declared on the diagram's id.
const TOKENS: [(&str, &str); 5] = [
    ("textColor", "#333333"),
    ("lineColor", "#333333"),
    ("nodeBorder", "#9370DB"),
    ("errorBkgColor", "#552222"),
    ("errorTextColor", "#552222"),
];

/// The values of the base rules that a theme gives, and the colours of the rules: what [`sheet`] writes.
struct Parts {
    /// `strokeWidth`, as the edges and the neo nodes use it; the sheet writes `1` when it is absent.
    stroke_width: Option<String>,
    /// `dropShadow`; the sheet writes `none` when it is absent or empty.
    drop_shadow: Option<String>,
    /// `fontFamily` of the theme, which `:root` names; the sheet writes the configured family when it is absent.
    root_font: Option<String>,
    /// The custom properties of the colours, each scheme in its own block.
    colours: String,
}

/// The `<style>` text of the base rules for the diagram with the id `svg_id`, with the embedded
/// `@font-face` rules first. The font family is `font_family` as CSS reads it (quoted when it is a
/// name), and `font_size` is the CSS size. The result is escaped for an XML `<style>` element.
#[must_use]
pub fn base_style_sheet(
    svg_id: &str,
    themes: Themes<'_>,
    font_family: &str,
    font_size: &str,
    families: &[EmbeddedFamily],
) -> String {
    base_style_sheet_with(svg_id, themes, font_family, font_size, families, "")
}

/// [`base_style_sheet`] with the diagram's own raw CSS, `own_rules`, placed after the base rules of
/// `p` and before the `neo` look rules, as the reference's `getStyles` places a diagram's `styles.js`
/// (`styles.ts` 40-45 and the rule order of `mermaidAPI`). The own rules are not escaped here: the
/// caller writes them as CSS text, and the whole sheet is escaped once.
#[must_use]
pub fn base_style_sheet_with(
    svg_id: &str,
    themes: Themes<'_>,
    font_family: &str,
    font_size: &str,
    families: &[EmbeddedFamily],
    own_rules: &str,
) -> String {
    let variables = themes.primary();
    let id = format!("#{svg_id}");
    let tokens = declarations(variables);
    let colours = match themes {
        Themes::Single(_) => format!("{id}{{{tokens}}}"),
        Themes::Schemes { light, dark } => format!(
            "{id}{{{}}}@media (prefers-color-scheme: dark){{{id}{{{}}}}}",
            declarations(light),
            declarations(dark)
        ),
    };
    let parts = Parts {
        stroke_width: text_of(variables, "strokeWidth"),
        drop_shadow: text_of(variables, "dropShadow"),
        root_font: text_of(variables, "fontFamily"),
        colours,
    };
    sheet(svg_id, parts, font_family, font_size, families, own_rules)
}

/// [`base_style_sheet_with`] for a theme given as typed tokens (`docs-port/typed-views.md`): the same
/// sheet, byte for byte, with the colours and the values read from the tokens.
#[must_use]
pub fn base_style_sheet_tokens(
    svg_id: &str,
    theme: ThemeView<'_>,
    font_family: &str,
    font_size: &str,
    families: &[EmbeddedFamily],
    own_rules: &str,
) -> String {
    let id = format!("#{svg_id}");
    let colours = match theme.dark {
        None => format!("{id}{{{}}}", token_declarations(theme.light)),
        Some(dark) => format!(
            "{id}{{{}}}@media (prefers-color-scheme: dark){{{id}{{{}}}}}",
            token_declarations(theme.light),
            token_declarations(dark)
        ),
    };
    let parts = Parts {
        stroke_width: theme.light.stroke_width.text(),
        drop_shadow: theme.light.drop_shadow.text(),
        root_font: theme.light.font_family.text(),
        colours,
    };
    sheet(svg_id, parts, font_family, font_size, families, own_rules)
}

/// The sheet of the base rules: the faces, the colours, then the rules, escaped once.
fn sheet(
    svg_id: &str,
    parts: Parts,
    font_family: &str,
    font_size: &str,
    families: &[EmbeddedFamily],
    own_rules: &str,
) -> String {
    let id = format!("#{svg_id}");
    let stroke_width = parts.stroke_width.unwrap_or_else(|| "1".to_owned());
    let shadow = match parts.drop_shadow {
        Some(value) if !value.is_empty() => value.replacen(
            "url(#drop-shadow)",
            // `svgId` is the selector `#id` in styles.ts, so the reference names `url(#id-drop-shadow)`.
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        ),
        _ => "none".to_owned(),
    };
    // `:root` names the configured family (mermaidAPI.ts), not the embedded subset of this diagram.
    let root_font = parts.root_font.unwrap_or_else(|| font_family.to_owned());
    let colours = parts.colours;
    let faces = if families.is_empty() {
        String::new()
    } else {
        font_faces(families)
    };
    let rules = format!(
        "{id}{{font-family:{font_family};font-size:{font_size};fill:var(--textColor);}}\
@keyframes edge-animation-frame{{from{{stroke-dashoffset:0;}}}}\
@keyframes dash{{to{{stroke-dashoffset:0;}}}}\
{id} .edge-animation-slow{{stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 50s linear infinite;stroke-linecap:round;}}\
{id} .edge-animation-fast{{stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 20s linear infinite;stroke-linecap:round;}}\
{id} .error-icon{{fill:var(--errorBkgColor);}}\
{id} .error-text{{fill:var(--errorTextColor);stroke:var(--errorTextColor);}}\
{id} .edge-thickness-normal{{stroke-width:{stroke_width}px;}}\
{id} .edge-thickness-thick{{stroke-width:3.5px;}}\
{id} .edge-pattern-solid{{stroke-dasharray:0;}}\
{id} .edge-thickness-invisible{{stroke-width:0;fill:none;}}\
{id} .edge-pattern-dashed{{stroke-dasharray:3;}}\
{id} .edge-pattern-dotted{{stroke-dasharray:2;}}\
{id} .marker{{fill:var(--lineColor);stroke:var(--lineColor);}}\
{id} .marker.cross{{stroke:var(--lineColor);}}\
{id} svg{{font-family:{font_family};font-size:{font_size};}}\
{id} p{{margin:0;}}\
{own_rules}\
{id} .node .neo-node{{stroke:var(--nodeBorder);}}\
{id} [data-look=\"neo\"].node rect,{id} [data-look=\"neo\"].cluster rect,{id} [data-look=\"neo\"].node polygon{{stroke:var(--nodeBorder);filter:{shadow};}}\
{id} [data-look=\"neo\"].swimlane.cluster rect{{filter:none;}}\
{id} [data-look=\"neo\"].node path{{stroke:var(--nodeBorder);stroke-width:{stroke_width}px;}}\
{id} [data-look=\"neo\"].node .outer-path{{filter:{shadow};}}\
{id} [data-look=\"neo\"].node .neo-line path{{stroke:var(--nodeBorder);filter:none;}}\
{id} [data-look=\"neo\"].node circle{{stroke:var(--nodeBorder);filter:{shadow};}}\
{id} [data-look=\"neo\"].node circle .state-start{{fill:#000000;}}\
{id} [data-look=\"neo\"].icon-shape .icon{{fill:var(--nodeBorder);filter:{shadow};}}\
{id} [data-look=\"neo\"].icon-shape .icon-neo path{{stroke:var(--nodeBorder);filter:{shadow};}}\
:root{{--mermaid-font-family:{root_font};}}"
    );
    css_text(&format!("{faces}{colours}{rules}"))
}

/// The custom properties of the base colours of a set of variables, as `--name:value;` pairs. A
/// colour the variables do not give falls back to the default of [`TOKENS`].
fn declarations(variables: &ferrigura_model::value::Value) -> String {
    TOKENS
        .iter()
        .map(|(name, fallback)| {
            let value = text_of(variables, name).unwrap_or_else(|| (*fallback).to_owned());
            format!("--{name}:{value};")
        })
        .collect()
}

/// The custom properties of the base colours of a theme's tokens: [`declarations`] for the tokens,
/// with the same names, order and defaults.
fn token_declarations(tokens: &ThemeTokens) -> String {
    let colour =
        |token: &Token, fallback: &str| token.text().unwrap_or_else(|| fallback.to_owned());
    format!(
        "--textColor:{};--lineColor:{};--nodeBorder:{};--errorBkgColor:{};--errorTextColor:{};",
        colour(&tokens.text_color, "#333333"),
        colour(&tokens.line_color, "#333333"),
        colour(&tokens.node_border, "#9370DB"),
        colour(&tokens.error_bkg_color, "#552222"),
        colour(&tokens.error_text_color, "#552222"),
    )
}

#[cfg(test)]
mod tests {
    use ferrigura_model::theme::{Theme, ThemeTokens};
    use ferrigura_model::value::Value;
    use ferrigura_text::{FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

    use super::*;

    /// The sheet from the tokens is the sheet from the map, byte for byte, with one scheme and with
    /// both, for the shipped themes and for overrides that change the values the sheet reads.
    #[test]
    fn the_sheet_of_the_tokens_is_the_sheet_of_the_map() {
        let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
        let families = embed_runs(&[Run {
            text: "v12.2.0",
            face: sans,
        }])
        .expect("the bundled font");
        let overrides = Value::from(serde_json::json!({
            "dropShadow": "url(#drop-shadow) drop-shadow(1px 1px #000)",
            "strokeWidth": 3,
            "nodeBorder": 5,
            "errorTextColor": "#123",
            "fontFamily": "Georgia, serif",
        }));
        let light = Theme::ReduxColor.variables(Some(&overrides));
        let dark = Theme::ReduxDarkColor.variables(None);
        let light_tokens = ThemeTokens::from_value(&light);
        let dark_tokens = ThemeTokens::from_value(&dark);
        for families in [&families[..], &[]] {
            let single = base_style_sheet_with(
                "ferrigura-test",
                Themes::Single(&light),
                "\"Recursive\", sans",
                "16px",
                families,
                "own{x:1}",
            );
            let single_tokens = base_style_sheet_tokens(
                "ferrigura-test",
                ThemeView {
                    light: &light_tokens,
                    dark: None,
                },
                "\"Recursive\", sans",
                "16px",
                families,
                "own{x:1}",
            );
            assert_eq!(single, single_tokens);

            let both = base_style_sheet_with(
                "ferrigura-test",
                Themes::Schemes {
                    light: &light,
                    dark: &dark,
                },
                "\"Recursive\", sans",
                "16px",
                families,
                "",
            );
            let both_tokens = base_style_sheet_tokens(
                "ferrigura-test",
                ThemeView {
                    light: &light_tokens,
                    dark: Some(&dark_tokens),
                },
                "\"Recursive\", sans",
                "16px",
                families,
                "",
            );
            assert_eq!(both, both_tokens);
        }
    }
}
