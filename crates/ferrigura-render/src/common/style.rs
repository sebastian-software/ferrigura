//! The base style sheet of every diagram: the rules of `packages/mermaid/src/styles.ts` that do not
//! depend on the diagram type (the font and text colour, the edge animation and thickness, the
//! markers, the `neo` look of the nodes, and `:root`'s font variable). The reference compiles them
//! with the diagram's id as the scope; here every colour is a custom property declared on the id
//! (`docs-port/decisions/0007`), and the rules refer to it with `var(…)`.
//!
//! A diagram's own rules (`styles.ts` of the type, such as `flowchart/style.rs`) come between the
//! `p` rule and the `neo` rules, as the reference's `getStyles` places them. The info diagram has no
//! rules of its own, so this sheet is its whole style.

use ferrigura_text::EmbeddedFamily;

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
    let variables = themes.primary();
    let id = format!("#{svg_id}");
    let stroke_width = text_of(variables, "strokeWidth").unwrap_or_else(|| "1".to_owned());
    let shadow = match text_of(variables, "dropShadow") {
        Some(value) if !value.is_empty() => value.replacen(
            "url(#drop-shadow)",
            // `svgId` is the selector `#id` in styles.ts, so the reference names `url(#id-drop-shadow)`.
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        ),
        _ => "none".to_owned(),
    };
    let tokens = declarations(variables);
    // `:root` names the configured family (mermaidAPI.ts), not the embedded subset of this diagram.
    let root_font = text_of(variables, "fontFamily").unwrap_or_else(|| font_family.to_owned());
    let colours = match themes {
        Themes::Single(_) => format!("{id}{{{tokens}}}"),
        Themes::Schemes { light, dark } => format!(
            "{id}{{{}}}@media (prefers-color-scheme: dark){{{id}{{{}}}}}",
            declarations(light),
            declarations(dark)
        ),
    };
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
