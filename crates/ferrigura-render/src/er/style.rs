//! The ER diagram's style sheet: `diagrams/er/styles.ts` (`getStyles` and `genColor`), between the
//! shared rules of `packages/mermaid/src/styles.ts` that an ER diagram gets, in the order the
//! reference compiles them: the root font and the edge classes, the markers, the palette rules, the
//! entity rules, the `neo` look rules, and the root's font variable (`createCssStyles`, which gives
//! an ER diagram no class rules, as its renderer has no `getClasses`). The compiled sheet of
//! the reference is in `docs-port/er/NOTES.md`, section 5.7.
//!
//! Colours follow `docs-port/decisions/0007`: every colour is a custom property declared on the
//! diagram's id, once for each set of variables, and the rules refer to it with `var(…)`. The
//! palette's slots are `borderColor<i>` and `bkgColor<i>`, as the flowchart's are.

use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::EmbeddedFamily;

use crate::context::{Globals, ThemeView};
use crate::svg::{css_text, escape, font_faces};
use crate::unified::palette::{Palette, palette};

/// The colour tokens of the entity, edge and cluster rules, in the order of the custom properties. A
/// token the theme lacks is not declared (`genColor`, `getStyles` of `styles.ts`).
fn colour_tokens(tokens: &ThemeTokens) -> [(&'static str, &Token); 13] {
    [
        ("nodeBorder", &tokens.node_border),
        ("mainBkg", &tokens.main_bkg),
        ("lineColor", &tokens.line_color),
        ("textColor", &tokens.text_color),
        ("nodeTextColor", &tokens.node_text_color),
        ("tertiaryColor", &tokens.tertiary_color),
        ("edgeLabelBackground", &tokens.edge_label_background),
        ("erEdgeLabelBackground", &tokens.er_edge_label_background),
        ("clusterBkg", &tokens.cluster_bkg),
        ("clusterBorder", &tokens.cluster_border),
        ("titleColor", &tokens.title_color),
        ("errorBkgColor", &tokens.error_bkg_color),
        ("errorTextColor", &tokens.error_text_color),
    ]
}

/// The flood of the drop shadows in a light scheme (`insertLookDefs.ts:30`).
const LIGHT_FLOOD: &str = "#000000";

/// The flood of the drop shadows in a dark scheme (`insertLookDefs.ts:30`).
const DARK_FLOOD: &str = "#FFFFFF";

/// The `<style>` element's content for an ER diagram with the id `svg_id`. The look is read from
/// `config.look`, the palette from the theme, and the families of the embedded text give the
/// `@font-face` rules that come first.
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    families: &[EmbeddedFamily],
) -> String {
    let tokens = theme.light;
    let id = format!("#{svg_id}");
    let font = families.first().map_or_else(
        || {
            tokens
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned())
        },
        |family| family.css_family.clone(),
    );
    let font_size = tokens.font_size.text().unwrap_or_else(|| "16px".to_owned());
    let stroke_width = format!(
        "{}px",
        tokens.stroke_width.text().unwrap_or_else(|| "1".to_owned())
    );
    // `look === 'neo' ? strokeWidth : '1px'` (`styles.ts` 117, 122).
    let neo = globals.look == "neo";
    let entity_stroke = if neo { stroke_width.as_str() } else { "1px" };
    let palette = palette(globals, tokens);
    let shadow = match tokens.drop_shadow.text() {
        Some(shadow) if !shadow.is_empty() => shadow.replacen(
            "url(#drop-shadow)",
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        ),
        _ => "none".to_owned(),
    };
    let filter = format!("filter:{shadow};");
    let colour = |name: &str| format!("var(--{name})");
    // `COLOR_THEMES.has(theme) && erEdgeLabelBackground ? erEdgeLabelBackground : …` (`styles.ts`
    // 96-108): the colour theme's ER edge label colour, else the tertiary colour faded for `.labelBkg`
    // and the edge label background for `.edgeLabel`.
    let er_edge_label = tokens.er_edge_label_background.text().is_some();
    let label_background = if er_edge_label {
        colour("erEdgeLabelBackground")
    } else {
        format!("rgba({})", fade_channels(&tokens.tertiary_color, 0.5))
    };
    let edge_label_background = if er_edge_label {
        colour("erEdgeLabelBackground")
    } else {
        colour("edgeLabelBackground")
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
            "fill:var(--errorTextColor);stroke:var(--errorTextColor);",
        ),
        rule(
            &format!("{id} .edge-thickness-normal"),
            &format!("stroke-width:{stroke_width};"),
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
    if let Some(palette) = &palette {
        rules.push(palette_rules(&id, &globals.look, palette));
    }
    rules.extend([
        // `.entityBox` (`styles.ts` 81-84).
        rule(
            &format!("{id} .entityBox"),
            &format!(
                "fill:{};stroke:{};",
                colour("mainBkg"),
                colour("nodeBorder")
            ),
        ),
        // `.relationshipLabelBox` (`styles.ts` 86-94).
        rule(
            &format!("{id} .relationshipLabelBox"),
            &format!(
                "fill:{};opacity:0.7;background-color:{};",
                colour("tertiaryColor"),
                colour("tertiaryColor")
            ),
        ),
        rule(&format!("{id} .relationshipLabelBox rect"), "opacity:0.5;"),
        // `.labelBkg` and `.edgeLabel` take the ER edge label colour of a colour theme (`styles.ts`
        // 96-108).
        rule(
            &format!("{id} .labelBkg"),
            &format!("background-color:{label_background};"),
        ),
        rule(
            &format!("{id} .edgeLabel"),
            &format!("background-color:{edge_label_background};"),
        ),
        rule(
            &format!("{id} .edgeLabel .label rect"),
            &format!("fill:{edge_label_background};"),
        ),
        rule(
            &format!("{id} .edgeLabel .label text"),
            &format!("fill:{};", colour("textColor")),
        ),
        rule(
            &format!("{id} .edgeLabel .label"),
            &format!("fill:{};font-size:14px;", colour("nodeBorder")),
        ),
        rule(
            &format!("{id} .label"),
            &format!(
                "font-family:{font};color:{};",
                if !tokens.node_text_color.is_absent() {
                    colour("nodeTextColor")
                } else {
                    colour("textColor")
                }
            ),
        ),
        rule(
            &format!("{id} .edge-pattern-dashed"),
            "stroke-dasharray:8,8;",
        ),
        rule(
            &format!("{id} .node rect,{id} .node circle,{id} .node ellipse,{id} .node polygon"),
            &format!(
                "fill:{};stroke:{};stroke-width:{entity_stroke};",
                colour("mainBkg"),
                colour("nodeBorder")
            ),
        ),
        rule(
            &format!("{id} .relationshipLine"),
            &format!(
                "stroke:{};stroke-width:{entity_stroke};fill:none;",
                colour("lineColor")
            ),
        ),
        rule(
            &format!("{id} .marker"),
            &format!(
                "fill:none !important;stroke:{} !important;stroke-width:1;",
                colour("lineColor")
            ),
        ),
        rule(
            &format!("{id} [data-look=neo].labelBkg"),
            &format!(
                "background-color:rgba({});",
                fade_channels(&tokens.tertiary_color, 0.5)
            ),
        ),
        rule(
            &format!("{id} .cluster rect"),
            "fill:var(--clusterBkg);stroke:var(--clusterBorder);stroke-width:1px;",
        ),
        rule(&format!("{id} .cluster text"), "fill:var(--titleColor);"),
        rule(
            &format!("{id} .cluster-label text"),
            "fill:var(--titleColor);",
        ),
        rule(
            &format!("{id} .node .neo-node"),
            "stroke:var(--nodeBorder);",
        ),
    ]);

    // The look's rules of `styles.ts` (120-160). They match only the elements of their look.
    rules.extend([
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
            &format!("stroke:{};stroke-width:{stroke_width};", colour("nodeBorder")),
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
    // `userStyles`: the root's font variable, which `createCssStyles` writes for the font family.
    rules.push(format!(":root{{--mermaid-font-family: {font};}}"));

    let faces = if families.is_empty() {
        String::new()
    } else {
        font_faces(families)
    };
    css_text(&format!(
        "{faces}{}{}",
        custom_properties(svg_id, theme, globals),
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
fn custom_properties(svg_id: &str, theme: ThemeView<'_>, globals: &Globals) -> String {
    let id = format!("#{svg_id}");
    match theme.dark {
        None => {
            let palette = palette(globals, theme.light);
            rule(
                &id,
                &declarations(theme.light, flood_colour(globals), palette.as_ref()),
            )
        }
        Some(dark) => {
            let palette = palette(globals, theme.light);
            format!(
                "{}@media (prefers-color-scheme: dark){{{}}}",
                rule(
                    &id,
                    &declarations(theme.light, LIGHT_FLOOD, palette.as_ref())
                ),
                rule(&id, &declarations(dark, DARK_FLOOD, palette.as_ref())),
            )
        }
    }
}

/// The flood of the drop shadows for a configuration: white for a theme whose name says dark.
fn flood_colour(globals: &Globals) -> &'static str {
    if globals.theme.contains("dark") {
        DARK_FLOOD
    } else {
        LIGHT_FLOOD
    }
}

/// The declarations of the custom properties of one set: the colour tokens the set has, the slots of
/// the palette (`borderColor<i>` and `bkgColor<i>`, the fills wrapped around the palette's length),
/// and the shadow's flood.
fn declarations(tokens: &ThemeTokens, flood: &str, palette: Option<&Palette>) -> String {
    let mut out = String::new();
    for (name, token) in colour_tokens(tokens) {
        if let Some(colour) = token.text() {
            out.push_str(&format!("--{name}:{};", css_text(&colour)));
        }
    }
    if let Some(palette) = palette {
        for (index, colour) in palette.border.iter().enumerate() {
            out.push_str(&format!("--borderColor{index}:{};", css_text(colour)));
        }
        let fills = palette.background.len();
        for index in 0..palette.border.len() {
            let colour = if fills == 0 {
                tokens.cluster_bkg.text()
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

/// The palette rules of `genColor` (`styles.ts` 22-62): for each slot, the entity's path and rect take
/// the slot's border, and its fill when the palette has fills. The slots are the border palette's
/// length, so each entity's `data-color-id` has a rule.
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

/// The colour of a token as `khroma.rgba(channel…, alpha)` writes it: the red, green and blue
/// channels of a hex colour, comma-separated, with the alpha left to the caller. A colour that is not
/// a hex colour gives the channels of white (the reference's `fade` on a named colour is not reached by
/// the shipped themes, whose tertiary colour is a hex colour).
fn fade_channels(token: &Token, alpha: f64) -> String {
    let colour = token.text().unwrap_or_default();
    let (red, green, blue) = hex_channels(&colour).unwrap_or((255, 255, 255));
    format!("{red}, {green}, {blue}, {}", crate::svg::number(alpha))
}

/// The channels of a `#rgb` or `#rrggbb` colour.
fn hex_channels(colour: &str) -> Option<(u8, u8, u8)> {
    let digits = colour.strip_prefix('#')?;
    let expanded: String = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 => digits.to_owned(),
        _ => return None,
    };
    let channel = |range: std::ops::Range<usize>| u8::from_str_radix(&expanded[range], 16).ok();
    Some((channel(0..2)?, channel(2..4)?, channel(4..6)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hex_colour_gives_its_channels() {
        assert_eq!(hex_channels("#ffffff"), Some((255, 255, 255)));
        assert_eq!(hex_channels("#28253D"), Some((0x28, 0x25, 0x3d)));
        assert_eq!(hex_channels("#fff"), Some((255, 255, 255)));
        assert_eq!(hex_channels("red"), None);
    }
}
