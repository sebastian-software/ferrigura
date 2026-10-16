//! The style sheet of the sequence diagram: the rules of `packages/mermaid/src/diagrams/sequence/styles.js`
//! and the neo rules of the look, each scoped to the diagram's id. The colours are the custom
//! properties declared on the id (`docs-port/decisions/0007`), and the rules refer to them with
//! `var(…)`; the elements carry the same colours as presentation attributes where they have them.

use ferrigura_model::value::Value;
use ferrigura_text::EmbeddedFamily;

use crate::common::style::base_style_sheet;
use crate::theme::Themes;
use crate::variables::{css_text, text_of};

/// The colour tokens of the sequence rules, each declared on the diagram's id, per scheme.
const TOKENS: [&str; 16] = [
    "actorBorder",
    "actorBkg",
    "actorTextColor",
    "actorLineColor",
    "signalColor",
    "signalTextColor",
    "labelBoxBorderColor",
    "labelBoxBkgColor",
    "labelTextColor",
    "loopTextColor",
    "noteBorderColor",
    "noteBkgColor",
    "noteTextColor",
    "activationBorderColor",
    "activationBkgColor",
    "sequenceNumberColor",
];

/// The declarations of the tokens of one set of variables, as custom properties.
fn declarations(variables: &Value) -> String {
    TOKENS
        .iter()
        .map(|name| {
            let value = text_of(variables, name).unwrap_or_else(|| "none".to_owned());
            format!("--{name}:{value};")
        })
        .collect()
}

/// The `<style>` text of a sequence diagram with the id `svg_id`, escaped for XML. The base rules
/// come first, from [`base_style_sheet`], then the diagram's own.
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    themes: Themes<'_>,
    font_family: &str,
    font_size: &str,
    families: &[EmbeddedFamily],
    look_neo: bool,
) -> String {
    let base = base_style_sheet(svg_id, themes, font_family, font_size, families);
    let id = format!("#{svg_id}");
    let variables = themes.primary();
    let shadow = if look_neo {
        format!("url(#{svg_id}-drop-shadow)")
    } else {
        "none".to_owned()
    };
    let scoped = match themes {
        Themes::Single(_) => format!("{id}{{{}}}", declarations(variables)),
        Themes::Schemes { light, dark } => format!(
            "{id}{{{}}}@media (prefers-color-scheme: dark){{{id}{{{}}}}}",
            declarations(light),
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
