//! The style sheet of the sequence diagram: the rules of `packages/mermaid/src/diagrams/sequence/styles.js`
//! and the neo rules of the look, each scoped to the diagram's id. The colors are the custom
//! properties declared on the id (decision 0011), and the rules refer to them with
//! `var(…)`; the elements carry the same colors as presentation attributes where they have them.

use std::borrow::Cow;
use std::fmt::Write as _;

use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::EmbeddedFamily;

use crate::common::style::base_style_sheet_with_capacity;
use crate::context::ThemeView;
use crate::design::Tone;
use crate::svg::escape_css_tail_in_place;

/// Room for the sequence diagram's own rules appended after the base style sheet: about 3 KiB.
const SEQUENCE_RULES_CAPACITY: usize = 4 * 1024;

/// The color tokens of the sequence rules, each declared on the diagram's id, per scheme, in the
/// order the sheet declares them.
fn color_tokens(tokens: &ThemeTokens) -> [(&'static str, &Token); 16] {
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

/// Appends the declarations of the tokens of one set of variables, as custom properties. In the
/// default rendering its own properties (the role colors and the flood of the shadow) follow.
fn write_declarations(css: &mut String, tokens: &ThemeTokens, design: Option<Tone>) {
    for (name, token) in color_tokens(tokens) {
        let value = token.text().unwrap_or_else(|| "none".to_owned());
        let _ = write!(css, "--{name}:{value};");
    }
    if let Some(tone) = design {
        css.push_str(crate::design::declarations(tone));
    }
}

/// The `<style>` text of a sequence diagram with the id `svg_id`, escaped for XML. The base rules
/// come first, from [`crate::common::style::base_style_sheet`], then the diagram's own.
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    theme: ThemeView<'_>,
    font_family: &str,
    font_size: &str,
    families: &[EmbeddedFamily],
    look_neo: bool,
    design: Option<Tone>,
) -> String {
    let mut css = base_style_sheet_with_capacity(
        svg_id,
        theme,
        font_family,
        font_size,
        families,
        "",
        SEQUENCE_RULES_CAPACITY,
    );
    let seq_start = css.len();
    let id = format!("#{svg_id}");
    let shadow: Cow<'_, str> = if look_neo {
        Cow::Owned(format!("url(#{svg_id}-drop-shadow)"))
    } else {
        Cow::Borrowed("none")
    };
    match theme.dark {
        None => {
            css.push_str(&id);
            css.push('{');
            write_declarations(&mut css, theme.light, design);
            css.push('}');
        }
        Some(dark) => {
            let mut light_block = String::new();
            light_block.push_str(&id);
            light_block.push('{');
            write_declarations(&mut light_block, theme.light, design.map(|_| Tone::Light));
            light_block.push('}');
            let mut dark_block = String::new();
            dark_block.push_str(&id);
            dark_block.push('{');
            write_declarations(&mut dark_block, dark, design.map(|_| Tone::Dark));
            dark_block.push('}');
            css.push_str(&crate::scheme::both(&light_block, &dark_block));
        }
    }
    let _ = write!(
        css,
        "{id} .actor{{stroke:var(--actorBorder);fill:var(--actorBkg);stroke-width:1;}}\
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
    if design.is_some() {
        write_design_rules(&mut css, &id);
    }
    escape_css_tail_in_place(&mut css, seq_start);
    css
}

/// The rules of the default rendering (`docs/diagrams/sequence.md#default-look`), after the
/// reference's, which they override: hairline outlines, square flat notes, quiet frames, and the
/// colors of the roles.
fn write_design_rules(css: &mut String, id: &str) {
    let _ = write!(
        css,
        "{id} .actor{{stroke-width:{node};}}\
{id} .actor-line{{stroke-width:1px;}}\
{id} .messageLine0,{id} .messageLine1{{stroke-width:{edge};}}\
{id} .loopLine{{stroke-width:{node};stroke-dasharray:none;stroke:var(--labelBoxBorderColor);fill:none;}}\
{id} .labelBox{{stroke:none;fill:var(--labelBoxBkgColor);filter:none;}}\
{id} .note{{stroke-width:1px;filter:none;}}\
{id} .activation0,{id} .activation1,{id} .activation2{{stroke-width:{node};}}\
{id} g rect.rect{{filter:none;stroke:var(--labelBoxBorderColor);stroke-width:1px;}}\
{id} [id$=\"-crosshead\"] path{{stroke-width:{edge};stroke-linecap:round;}}\
{id} [id$=\"-sequencenumber\"]{{fill:var(--signalColor);}}\
{id} .centralConnection circle{{fill:var(--signalColor);stroke:none;}}\
{id} .actor-man line,{id} .actor-man circle,{id} .actor-man path{{stroke:var(--actorBorder);stroke-width:{node};stroke-linecap:round;}}\
{id} .actor-man circle{{fill:var(--actorBkg);}}\
{id} [data-role=\"actor\"] line,{id} [data-role=\"actor\"] circle{{stroke:var(--terminalBorder);stroke-width:{edge};}}\
{id} g>rect.rect{{fill-opacity:0.16;}}\
{id} [data-role=\"data\"] path{{fill:var(--dataBkg);stroke:var(--dataBorder);stroke-width:{node};}}\
{id} [data-role=\"decision\"] .loopLine{{stroke:var(--decisionBorder);}}\
{id} [data-role=\"decision\"] .labelBox{{fill:var(--decisionBkg);}}\
{id} [data-role=\"decision\"] .labelText,{id} [data-role=\"decision\"] .labelText>tspan{{fill:var(--decisionText);}}\
{id}-drop-shadow feDropShadow{{flood-color:var(--floodColor);flood-opacity:1;}}",
        node = crate::design::NODE_STROKE_WIDTH,
        edge = crate::design::EDGE_STROKE_WIDTH,
    );
}
