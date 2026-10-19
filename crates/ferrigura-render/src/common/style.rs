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
use crate::svg::{css_text, font_faces};

/// The `<style>` text of the base rules for the diagram with the id `svg_id`, with the embedded
/// `@font-face` rules first. The font family is `font_family` as CSS reads it (quoted when it is a
/// name), and `font_size` is the CSS size. The colours are custom properties on the id, one block
/// per scheme of `theme`; the stroke width, the drop shadow and the font that `:root` names come from
/// the light (or only) scheme. The result is escaped for an XML `<style>` element.
///
/// `own_rules` is the diagram's own raw CSS (`""` for none), placed after the base rules of `p` and
/// before the `neo` look rules, as the reference's `getStyles` places a diagram's `styles.js`
/// (`styles.ts` 40-45 and the rule order of `mermaidAPI`). The own rules are not escaped here: the
/// caller writes them as CSS text, and the whole sheet is escaped once.
#[must_use]
pub fn base_style_sheet(
    svg_id: &str,
    theme: ThemeView<'_>,
    font_family: &str,
    font_size: &str,
    families: &[EmbeddedFamily],
    own_rules: &str,
) -> String {
    let id = format!("#{svg_id}");
    let colours = match theme.dark {
        None => format!("{id}{{{}}}", declarations(theme.light)),
        Some(dark) => format!(
            "{id}{{{}}}@media (prefers-color-scheme: dark){{{id}{{{}}}}}",
            declarations(theme.light),
            declarations(dark)
        ),
    };
    let tokens = theme.light;
    let stroke_width = tokens.stroke_width.text().unwrap_or_else(|| "1".to_owned());
    let shadow = match tokens.drop_shadow.text() {
        Some(value) if !value.is_empty() => value.replacen(
            "url(#drop-shadow)",
            // `svgId` is the selector `#id` in styles.ts, so the reference names `url(#id-drop-shadow)`.
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        ),
        _ => "none".to_owned(),
    };
    // `:root` names the configured family (mermaidAPI.ts), not the embedded subset of this diagram.
    let root_font = tokens
        .font_family
        .text()
        .unwrap_or_else(|| font_family.to_owned());
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

/// The custom properties of the base colours of a theme's tokens, as `--name:value;` pairs. A colour
/// the theme does not give falls back to the default of `redux-color`.
fn declarations(tokens: &ThemeTokens) -> String {
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

    use super::*;

    fn sheet(theme: ThemeView<'_>) -> String {
        base_style_sheet(
            "ferrigura-test",
            theme,
            "\"Recursive\", sans",
            "16px",
            &[],
            "own{x:1}",
        )
    }

    /// One scheme gives one block of colours; both give the dark one under the media query.
    #[test]
    fn the_colours_are_declared_per_scheme() {
        let light = ThemeTokens::from_value(&Theme::ReduxColor.variables(None));
        let dark = ThemeTokens::from_value(&Theme::ReduxDarkColor.variables(None));
        let single = sheet(ThemeView {
            light: &light,
            dark: None,
        });
        assert!(single.starts_with("#ferrigura-test{--textColor:"));
        assert!(!single.contains("@media"));
        let both = sheet(ThemeView {
            light: &light,
            dark: Some(&dark),
        });
        assert!(both.contains("@media (prefers-color-scheme: dark){#ferrigura-test{--textColor:"));
        assert!(both.contains("own{x:1}#ferrigura-test .node .neo-node"));
    }

    /// The values the theme gives are written as they are; the shadow names the diagram's filter.
    #[test]
    fn the_sheet_takes_the_values_of_the_theme() {
        let tokens = ThemeTokens::from_value(&Value::from(serde_json::json!({
            "dropShadow": "url(#drop-shadow) drop-shadow(1px 1px #000)",
            "strokeWidth": 3,
            "nodeBorder": 5,
            "fontFamily": "Georgia, serif",
        })));
        let css = sheet(ThemeView {
            light: &tokens,
            dark: None,
        });
        assert!(css.contains("--nodeBorder:5;"));
        assert!(css.contains(".edge-thickness-normal{stroke-width:3px;}"));
        assert!(css.contains("filter:url(#ferrigura-test-drop-shadow) drop-shadow(1px 1px #000);"));
        assert!(css.contains(":root{--mermaid-font-family:Georgia, serif;}"));
    }

    /// A theme without the values gives the defaults, and `:root` names the configured family.
    #[test]
    fn the_sheet_falls_back_without_the_values() {
        let tokens = ThemeTokens::from_value(&Value::empty_object());
        let css = sheet(ThemeView {
            light: &tokens,
            dark: None,
        });
        assert!(css.starts_with(
            "#ferrigura-test{--textColor:#333333;--lineColor:#333333;--nodeBorder:#9370DB;"
        ));
        assert!(css.contains(".edge-thickness-normal{stroke-width:1px;}"));
        assert!(css.contains("filter:none;"));
        assert!(css.contains(":root{--mermaid-font-family:\"Recursive\", sans;}"));
    }
}
