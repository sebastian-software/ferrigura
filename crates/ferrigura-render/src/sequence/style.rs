//! The style sheet of the sequence diagram: the rules of `packages/mermaid/src/diagrams/sequence/styles.js`
//! and the neo rules of the look, each scoped to the diagram's id. The colours are the custom
//! properties declared on the id (`docs-port/decisions/0007`), and the rules refer to them with
//! `var(…)`; the elements carry the same colours as presentation attributes where they have them.

use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::EmbeddedFamily;

use crate::common::style::base_style_sheet_tokens;
use crate::context::ThemeView;
use crate::svg::css_text;

/// The colour tokens of the sequence rules, each declared on the diagram's id, per scheme, in the
/// order the sheet declares them.
fn colour_tokens(tokens: &ThemeTokens) -> [(&'static str, &Token); 16] {
    [
        ("actorBorder", &tokens.actor_border),
        ("actorBkg", &tokens.actor_bkg),
        ("actorTextColor", &tokens.actor_text_color),
        ("actorLineColor", &tokens.actor_line_color),
        ("signalColor", &tokens.signal_color),
        ("signalTextColor", &tokens.signal_text_color),
        ("labelBoxBorderColor", &tokens.label_box_border_color),
        ("labelBoxBkgColor", &tokens.label_box_bkg_color),
        ("labelTextColor", &tokens.label_text_color),
        ("loopTextColor", &tokens.loop_text_color),
        ("noteBorderColor", &tokens.note_border_color),
        ("noteBkgColor", &tokens.note_bkg_color),
        ("noteTextColor", &tokens.note_text_color),
        ("activationBorderColor", &tokens.activation_border_color),
        ("activationBkgColor", &tokens.activation_bkg_color),
        ("sequenceNumberColor", &tokens.sequence_number_color),
    ]
}

/// The declarations of the tokens of one set of variables, as custom properties.
fn declarations(tokens: &ThemeTokens) -> String {
    colour_tokens(tokens)
        .iter()
        .map(|(name, token)| {
            let value = token.text().unwrap_or_else(|| "none".to_owned());
            format!("--{name}:{value};")
        })
        .collect()
}

/// The `<style>` text of a sequence diagram with the id `svg_id`, escaped for XML. The base rules
/// come first, from [`base_style_sheet_tokens`], then the diagram's own.
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    theme: ThemeView<'_>,
    font_family: &str,
    font_size: &str,
    families: &[EmbeddedFamily],
    look_neo: bool,
) -> String {
    let base = base_style_sheet_tokens(svg_id, theme, font_family, font_size, families, "");
    let id = format!("#{svg_id}");
    let shadow = if look_neo {
        format!("url(#{svg_id}-drop-shadow)")
    } else {
        "none".to_owned()
    };
    let scoped = match theme.dark {
        None => format!("{id}{{{}}}", declarations(theme.light)),
        Some(dark) => format!(
            "{id}{{{}}}@media (prefers-color-scheme: dark){{{id}{{{}}}}}",
            declarations(theme.light),
            declarations(dark)
        ),
    };
    let rules = format!(
        "{scoped}\
{id} .actor{{stroke:var(--actorBorder);fill:var(--actorBkg);stroke-width:1;}}\
{id} rect.actor.outer-path[data-look=\"neo\"],{id} rect.note[data-look=\"neo\"]{{filter:{shadow};}}\
{id} text.actor>tspan{{fill:var(--actorTextColor);stroke:none;}}\
{id} .actor-line{{stroke:var(--actorLineColor);}}\
{id} .innerArc{{stroke-width:1.5;stroke-dasharray:none;}}\
{id} .messageLine0{{stroke-width:1.5;stroke-dasharray:none;stroke:var(--signalColor);}}\
{id} .messageLine1{{stroke-width:1.5;stroke-dasharray:2, 2;stroke:var(--signalColor);}}\
{id} [id$=\"-arrowhead\"] path{{fill:var(--signalColor);stroke:var(--signalColor);}}\
{id} .sequenceNumber{{fill:var(--sequenceNumberColor);}}\
{id} [id$=\"-sequencenumber\"]{{fill:var(--signalColor);}}\
{id} [id$=\"-crosshead\"] path{{fill:var(--signalColor);stroke:var(--signalColor);}}\
{id} .messageText{{fill:var(--signalTextColor);stroke:none;}}\
{id} .labelBox{{stroke:var(--labelBoxBorderColor);fill:var(--labelBoxBkgColor);filter:{shadow};}}\
{id} .labelText,{id} .labelText>tspan{{fill:var(--labelTextColor);stroke:none;}}\
{id} .loopText,{id} .loopText>tspan{{fill:var(--loopTextColor);stroke:none;}}\
{id} .sectionTitle,{id} .sectionTitle>tspan{{fill:var(--loopTextColor);stroke:none;}}\
{id} .loopLine{{stroke-width:2px;stroke-dasharray:2, 2;stroke:var(--labelBoxBorderColor);fill:var(--labelBoxBorderColor);}}\
{id} .note{{stroke:var(--noteBorderColor);fill:var(--noteBkgColor);}}\
{id} .noteText,{id} .noteText>tspan{{fill:var(--noteTextColor);stroke:none;}}\
{id} .activation0,{id} .activation1,{id} .activation2{{fill:var(--activationBkgColor);stroke:var(--activationBorderColor);}}\
{id} .actorPopupMenu{{position:absolute;}}\
{id} .actorPopupMenuPanel{{position:absolute;fill:var(--actorBkg);box-shadow:0px 8px 16px 0px rgba(0,0,0,0.2);filter:drop-shadow(3px 5px 2px rgb(0 0 0 / 0.4));}}\
{id} .actor-man circle,{id} line{{fill:var(--actorBkg);stroke-width:2px;}}\
{id} g rect.rect{{filter:{shadow};stroke:var(--nodeBorder);}}"
    );
    format!("{base}{}", css_text(&rules))
}
