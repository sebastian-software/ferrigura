//! The requirement diagram's style sheet: `diagrams/requirement/styles.js` (`getStyles` and
//! `genColor`) between the shared rules of `packages/mermaid/src/styles.ts` that the diagram gets, in
//! the order the reference compiles them (`docs-port/requirement/NOTES.md`, section 6.7). The compiled
//! sheet of the reference is what the rules below reproduce, rule by rule, apart from the colours.
//!
//! Colours follow `docs-port/decisions/0007`: every colour is a custom property declared on the
//! diagram's id, once for each set of variables, and the rules refer to it with `var(…)`. The
//! palette's slots are `borderColor<i>` and `bkgColor<i>`, as the flowchart's are. The reference's
//! unused rules (`.reqBox`, `.reqTitle`, `.reqLabel`, `.reqLabelBox`, `.req-title-line`,
//! `.relationshipLabel`) are written too: the sheet of the reference has them, and they match nothing.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;
use ferrigura_text::EmbeddedFamily;

use crate::svg::{escape, font_faces};
use crate::theme::Themes;
use crate::unified::palette::{Palette, palette};
use crate::variables::{css_text, text_of};

/// The colour tokens of the requirement rules and the shared rules, each declared as a custom
/// property on the diagram's id. A token the theme lacks is not declared.
const COLOUR_TOKENS: [&str; 15] = [
    "nodeBorder",
    "mainBkg",
    "lineColor",
    "textColor",
    "nodeTextColor",
    "relationColor",
    "requirementBackground",
    "requirementBorderColor",
    "requirementTextColor",
    "relationLabelBackground",
    "relationLabelColor",
    "edgeLabelBackground",
    "requirementEdgeLabelBackground",
    "errorBkgColor",
    "errorTextColor",
];

/// The flood of the drop shadows in a light scheme (`insertLookDefs.ts:30`).
const LIGHT_FLOOD: &str = "#000000";

/// The flood of the drop shadows in a dark scheme (`insertLookDefs.ts:30`).
const DARK_FLOOD: &str = "#FFFFFF";

/// The `<style>` element's content for a requirement diagram with the id `svg_id`. The look is read
/// from `config.look`, the palette from the theme, and the families of the embedded text give the
/// `@font-face` rules that come first.
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    themes: Themes<'_>,
    config: &MermaidConfig,
    families: &[EmbeddedFamily],
) -> String {
    let variables = themes.primary();
    let id = format!("#{svg_id}");
    let font = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| family.css_family.clone(),
    );
    let font_size = text_of(variables, "fontSize").unwrap_or_else(|| "16px".to_owned());
    let stroke_width = text_of(variables, "strokeWidth").unwrap_or_else(|| "1".to_owned());
    // `styles.js` 91: the relation line takes the theme's stroke width on the neo look, else `1px`.
    let neo = config.look == "neo";
    let relation_stroke = if neo {
        stroke_width.clone()
    } else {
        "1px".to_owned()
    };
    // `styles.js` 70-88: the box and title rules take the requirement's own border size.
    let border_size = text_of(variables, "requirementBorderSize").unwrap_or_else(|| "1".to_owned());
    let palette = palette(config, variables);
    let shadow = match text_of(variables, "dropShadow") {
        Some(shadow) if !shadow.is_empty() => shadow.replacen(
            "url(#drop-shadow)",
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        ),
        _ => "none".to_owned(),
    };
    let filter = format!("filter:{shadow};");
    let colour = |name: &str| format!("var(--{name})");
    // `getStyles` of `styles.js` 117-118: the label background of a requirement is its own colour
    // when the theme has one, else the edge label background.
    let label_background = if text_of(variables, "requirementEdgeLabelBackground").is_some() {
        colour("requirementEdgeLabelBackground")
    } else {
        colour("edgeLabelBackground")
    };
    // `styles.js` 109-116: the label text takes the node's text colour, else the text colour.
    let label_colour = if variables.get("nodeTextColor").is_some() {
        colour("nodeTextColor")
    } else {
        colour("textColor")
    };

    let mut rules = vec![
        rule(
            &id,
            &format!("font-family:{font};font-size:{font_size};fill:var(--textColor);"),
        ),
        "@keyframes edge-animation-frame{from{stroke-dashoffset:0;}}".to_owned(),
        "@keyframes dash{to{stroke-dashoffset:0;}}".to_owned(),
        rule(
            &format!("{id} .edge-animation-slow"),
            "stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 50s linear infinite;stroke-linecap:round;",
        ),
        rule(
            &format!("{id} .edge-animation-fast"),
            "stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 20s linear infinite;stroke-linecap:round;",
        ),
        rule(
            &format!("{id} .error-icon"),
            &format!("fill:{};", colour("errorBkgColor")),
        ),
        rule(
            &format!("{id} .error-text"),
            &format!(
                "fill:{};stroke:{};",
                colour("errorTextColor"),
                colour("errorTextColor")
            ),
        ),
        rule(
            &format!("{id} .edge-thickness-normal"),
            &format!("stroke-width:{stroke_width}px;"),
        ),
        rule(
            &format!("{id} .edge-thickness-thick"),
            "stroke-width:3.5px;",
        ),
        rule(&format!("{id} .edge-pattern-solid"), "stroke-dasharray:0;"),
        rule(
            &format!("{id} .edge-thickness-invisible"),
            "stroke-width:0;fill:none;",
        ),
        rule(&format!("{id} .edge-pattern-dashed"), "stroke-dasharray:3;"),
        rule(&format!("{id} .edge-pattern-dotted"), "stroke-dasharray:2;"),
        rule(
            &format!("{id} .marker"),
            &format!(
                "fill:{};stroke:{};",
                colour("lineColor"),
                colour("lineColor")
            ),
        ),
        rule(
            &format!("{id} .marker.cross"),
            &format!("stroke:{};", colour("lineColor")),
        ),
        rule(
            &format!("{id} svg"),
            &format!("font-family:{font};font-size:{font_size};"),
        ),
        rule(&format!("{id} p"), "margin:0;"),
    ];

    // `genColor` (`styles.js` 4-48): one rule per slot of the border palette, for the path and the
    // rect of a node of that slot.
    if let Some(palette) = &palette {
        rules.push(palette_rules(&id, &config.look, palette));
    }

    // `getStyles` (`styles.js` 54-120), in its order.
    rules.extend([
        rule(
            &format!("{id} marker"),
            &format!(
                "fill:{};stroke:{};",
                colour("relationColor"),
                colour("relationColor")
            ),
        ),
        rule(
            &format!("{id} marker.cross"),
            &format!("stroke:{};", colour("lineColor")),
        ),
        rule(
            &format!("{id} svg"),
            &format!("font-family:{font};font-size:{font_size};"),
        ),
        rule(
            &format!("{id} .reqBox"),
            &format!(
                "fill:{};fill-opacity:1.0;stroke:{};stroke-width:{border_size};",
                colour("requirementBackground"),
                colour("requirementBorderColor"),
            ),
        ),
        rule(
            &format!("{id} .reqTitle,{id} .reqLabel"),
            &format!("fill:{};", colour("requirementTextColor")),
        ),
        rule(
            &format!("{id} .reqLabelBox"),
            &format!(
                "fill:{};fill-opacity:1.0;",
                colour("relationLabelBackground")
            ),
        ),
        rule(
            &format!("{id} .req-title-line"),
            &format!(
                "stroke:{};stroke-width:{border_size};",
                colour("requirementBorderColor")
            ),
        ),
        rule(
            &format!("{id} .relationshipLine"),
            &format!(
                "stroke:{};stroke-width:{relation_stroke};",
                colour("relationColor")
            ),
        ),
        rule(
            &format!("{id} .relationshipLabel"),
            &format!("fill:{};", colour("relationLabelColor")),
        ),
        rule(
            &format!("{id} .edgeLabel"),
            &format!("background-color:{};", colour("edgeLabelBackground")),
        ),
        rule(
            &format!("{id} .edgeLabel .label rect"),
            &format!("fill:{};", colour("edgeLabelBackground")),
        ),
        rule(
            &format!("{id} .edgeLabel .label text"),
            &format!("fill:{};", colour("relationLabelColor")),
        ),
        rule(
            &format!("{id} .divider"),
            &format!("stroke:{};stroke-width:1;", colour("nodeBorder")),
        ),
        rule(
            &format!("{id} .label"),
            &format!("font-family:{font};color:{label_colour};"),
        ),
        rule(
            &format!("{id} .label text,{id} span"),
            &format!("fill:{label_colour};color:{label_colour};"),
        ),
        rule(
            &format!("{id} .labelBkg"),
            &format!("background-color:{label_background};"),
        ),
    ]);

    // The shared rules after the diagram's (`styles.ts` 124-161): the node's neo border and the
    // look's shadows, then the root's font variable (`userStyles`).
    rules.extend([
        rule(&format!("{id} .node .neo-node"), &format!("stroke:{};", colour("nodeBorder"))),
        rule(
            &format!(
                "{id} [data-look=\"neo\"].node rect,{id} [data-look=\"neo\"].cluster rect,{id} [data-look=\"neo\"].node polygon"
            ),
            &format!("stroke:{};{filter}", colour("nodeBorder")),
        ),
        rule(
            &format!("{id} [data-look=\"neo\"].swimlane.cluster rect"),
            "filter:none;",
        ),
        rule(
            &format!("{id} [data-look=\"neo\"].node path"),
            &format!("stroke:{};stroke-width:{stroke_width}px;", colour("nodeBorder")),
        ),
        rule(&format!("{id} [data-look=\"neo\"].node .outer-path"), &filter),
        rule(
            &format!("{id} [data-look=\"neo\"].node .neo-line path"),
            &format!("stroke:{};filter:none;", colour("nodeBorder")),
        ),
        rule(
            &format!("{id} [data-look=\"neo\"].node circle"),
            &format!("stroke:{};{filter}", colour("nodeBorder")),
        ),
        rule(
            &format!("{id} [data-look=\"neo\"].node circle .state-start"),
            "fill:#000000;",
        ),
        rule(
            &format!("{id} [data-look=\"neo\"].icon-shape .icon"),
            &format!("fill:{};{filter}", colour("nodeBorder")),
        ),
        rule(
            &format!("{id} [data-look=\"neo\"].icon-shape .icon-neo path"),
            &format!("stroke:{};{filter}", colour("nodeBorder")),
        ),
    ]);
    rules.push(rule(
        &format!("{id} :root"),
        &format!("--mermaid-font-family: {font};"),
    ));

    let faces = if families.is_empty() {
        String::new()
    } else {
        font_faces(families)
    };
    css_text(&format!(
        "{faces}{}{}",
        custom_properties(svg_id, themes, config),
        rules.concat()
    ))
}

/// One rule: the selector and its declarations, in braces.
fn rule(selector: &str, declarations: &str) -> String {
    format!("{selector}{{{declarations}}}")
}

/// The custom properties of the colours, declared on the diagram's id. With one set of variables they
/// are written as they are; with a light and a dark set, the dark set applies under
/// `prefers-color-scheme: dark`, so one SVG follows the viewer's scheme.
fn custom_properties(svg_id: &str, themes: Themes<'_>, config: &MermaidConfig) -> String {
    let id = format!("#{svg_id}");
    match themes {
        Themes::Single(variables) => {
            let palette = palette(config, variables);
            rule(
                &id,
                &declarations(variables, flood_colour(config), palette.as_ref()),
            )
        }
        Themes::Schemes { light, dark } => {
            let palette = palette(config, light);
            format!(
                "{}@media (prefers-color-scheme: dark){{{}}}",
                rule(&id, &declarations(light, LIGHT_FLOOD, palette.as_ref())),
                rule(&id, &declarations(dark, DARK_FLOOD, palette.as_ref())),
            )
        }
    }
}

/// The flood of the drop shadows for a configuration: white for a theme whose name says dark.
fn flood_colour(config: &MermaidConfig) -> &'static str {
    if config.theme.contains("dark") {
        DARK_FLOOD
    } else {
        LIGHT_FLOOD
    }
}

/// The declarations of the custom properties of one set: the colour tokens the set has, the slots of
/// the palette (`borderColor<i>` and `bkgColor<i>`, the fills wrapped around the palette's length),
/// and the shadow's flood.
fn declarations(variables: &Value, flood: &str, palette: Option<&Palette>) -> String {
    let mut out = String::new();
    for token in COLOUR_TOKENS {
        if let Some(colour) = text_of(variables, token) {
            out.push_str(&format!("--{token}:{};", css_text(&colour)));
        }
    }
    if let Some(palette) = palette {
        for (index, colour) in palette.border.iter().enumerate() {
            // The theme's entries may carry a trailing space (`'#A3E635 '`), which CSS trims.
            out.push_str(&format!(
                "--borderColor{index}:{};",
                css_text(colour.trim())
            ));
        }
        let fills = palette.background.len();
        for index in 0..palette.border.len() {
            // A palette without fills declares no background slot, and its rules write no fill.
            let colour = if fills == 0 {
                None
            } else {
                palette.background.get(index % fills).cloned()
            };
            if let Some(colour) = colour {
                out.push_str(&format!("--bkgColor{index}:{};", css_text(&colour)));
            }
        }
    }
    out.push_str(&format!("--floodColor:{flood};"));
    out
}

/// The palette rules of `genColor` (`styles.js` 4-48): for each slot, the node's path and rect take
/// the slot's border, and its fill when the palette has fills. The slots are the border palette's
/// length, so each node's `data-color-id` has a rule.
fn palette_rules(id: &str, look: &str, palette: &Palette) -> String {
    let fills = palette.background.len();
    let mut css = String::new();
    for index in 0..palette.border.len() {
        let slot = format!(
            "{id} [data-look=\"{}\"][data-color-id=\"color-{index}\"]",
            escape(look)
        );
        let stroke = format!("stroke:var(--borderColor{index});");
        let fill = if fills == 0 {
            String::new()
        } else {
            format!("fill:var(--bkgColor{});", index % fills)
        };
        css.push_str(&rule(
            &format!("{slot}.node path"),
            &format!("{stroke}{fill}"),
        ));
        css.push_str(&rule(
            &format!("{slot}.node rect"),
            &format!("{stroke}{fill}"),
        ));
    }
    css
}
