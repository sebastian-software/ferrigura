//! The ER diagram's style sheet: `diagrams/er/styles.ts` (`getStyles` and `genColor`), between the
//! shared rules of `packages/mermaid/src/styles.ts` that an ER diagram gets, in the order the
//! reference compiles them: the root font and the edge classes, the markers, the palette rules, the
//! entity rules, the `neo` look rules, and the root's font variable (`createCssStyles`, which gives
//! an ER diagram no class rules, as its renderer has no `getClasses`).
//!
//! Colors follow `docs/decisions/0011-themes-as-css-variables.md`: every color is a custom property declared on the
//! diagram's id, once for each set of variables, and the rules refer to it with `var(…)`. The
//! palette's slots are `borderColor<i>` and `bkgColor<i>`, as the flowchart's are.

use std::borrow::Cow;
use std::fmt::Write as _;

use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::EmbeddedFamily;

use crate::context::{Globals, ThemeView};
use crate::design::{self, Tone};
use crate::svg::assemble_stylesheet;
use crate::unified::palette::{Palette, palette};

/// The color tokens of the entity, edge and cluster rules, in the order of the custom properties. A
/// token the theme lacks is not declared (`genColor`, `getStyles` of `styles.ts`).
fn color_tokens(tokens: &ThemeTokens) -> [(&'static str, &Token); 13] {
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

/// Room for the rules of an ER style sheet besides its font faces: about 6 KiB.
const RULES_CAPACITY: usize = 8 * 1024;

/// The `<style>` element's content for an ER diagram with the id `svg_id`. The look is read from
/// `config.look`, the palette from the theme, and the families of the embedded text give the
/// `@font-face` rules that come first. In the design mode (`design`), the mode's rules follow the
/// entity rules, and the mode's own properties take the place of the palette and the look's flood
/// (`docs/decisions/0023-design-mode.md`).
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    families: &[EmbeddedFamily],
    design: Option<Tone>,
) -> String {
    let tokens = theme.light;
    let id = format!("#{svg_id}");
    let font: Cow<'_, str> = match families.first() {
        Some(family) => Cow::Borrowed(&family.css_family),
        None => Cow::Owned(
            tokens
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned()),
        ),
    };
    let font_size = tokens.font_size.text().unwrap_or_else(|| "16px".to_owned());
    let stroke_width = format!(
        "{}px",
        tokens.stroke_width.text().unwrap_or_else(|| "1".to_owned())
    );
    // `look === 'neo' ? strokeWidth : '1px'` (`styles.ts` 117, 122).
    let neo = globals.look == "neo";
    let entity_stroke = if neo { stroke_width.as_str() } else { "1px" };
    // The design mode uses no palette: every entity is neutral.
    let palette = palette(globals, tokens).filter(|_| design.is_none());
    let shadow: Cow<'_, str> = match tokens.drop_shadow.text() {
        Some(shadow) if !shadow.is_empty() => Cow::Owned(shadow.replacen(
            "url(#drop-shadow)",
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        )),
        _ => Cow::Borrowed("none"),
    };
    let tertiary_fade = fade_channels(&tokens.tertiary_color, 0.5);
    // `COLOR_THEMES.has(theme) && erEdgeLabelBackground ? erEdgeLabelBackground : …` (`styles.ts`
    // 96-108): the color theme's ER edge label color, else the tertiary color faded for `.labelBkg`
    // and the edge label background for `.edgeLabel`.
    let er_edge_label = tokens.er_edge_label_background.text().is_some();
    let label_background: Cow<'_, str> = if er_edge_label {
        Cow::Borrowed("var(--erEdgeLabelBackground)")
    } else {
        Cow::Owned(format!("rgba({tertiary_fade})"))
    };
    let edge_label_background = if er_edge_label {
        "var(--erEdgeLabelBackground)"
    } else {
        "var(--edgeLabelBackground)"
    };
    let label_text_color = if !tokens.node_text_color.is_absent() {
        "var(--nodeTextColor)"
    } else {
        "var(--textColor)"
    };

    let mut css = String::with_capacity(RULES_CAPACITY);
    match design {
        Some(tone) => write_design_properties(&mut css, &id, theme, tone),
        None => write_custom_properties(&mut css, &id, theme, globals, palette.as_ref()),
    }
    let _ = write!(
        css,
        "{id}{{font-family:{font};font-size:{font_size};fill:var(--textColor);}}\
@keyframes edge-animation-frame{{from{{stroke-dashoffset:0;}}}}\
@keyframes dash{{to{{stroke-dashoffset:0;}}}}\
{id} .edge-animation-slow{{stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 50s linear infinite;stroke-linecap:round;}}\
{id} .edge-animation-fast{{stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 20s linear infinite;stroke-linecap:round;}}\
{id} .error-icon{{fill:var(--errorBkgColor);}}\
{id} .error-text{{fill:var(--errorTextColor);stroke:var(--errorTextColor);}}\
{id} .edge-thickness-normal{{stroke-width:{stroke_width};}}\
{id} .edge-thickness-thick{{stroke-width:3.5px;}}\
{id} .edge-pattern-solid{{stroke-dasharray:0;}}\
{id} .edge-thickness-invisible{{stroke-width:0;fill:none;}}\
{id} .edge-pattern-dashed{{stroke-dasharray:3;}}\
{id} .edge-pattern-dotted{{stroke-dasharray:2;}}\
{id} .marker{{fill:var(--lineColor);stroke:var(--lineColor);}}\
{id} .marker.cross{{stroke:var(--lineColor);}}\
{id} svg{{font-family:{font};font-size:{font_size};}}\
{id} p{{margin:0;}}"
    );
    if let Some(palette) = &palette {
        write_palette_rules(&mut css, &id, &globals.look, palette);
    }
    let _ = write!(
        css,
        "{id} .entityBox{{fill:var(--mainBkg);stroke:var(--nodeBorder);}}\
{id} .relationshipLabelBox{{fill:var(--tertiaryColor);opacity:0.7;background-color:var(--tertiaryColor);}}\
{id} .relationshipLabelBox rect{{opacity:0.5;}}\
{id} .labelBkg{{background-color:{label_background};}}\
{id} .edgeLabel{{background-color:{edge_label_background};}}\
{id} .edgeLabel .label rect{{fill:{edge_label_background};}}\
{id} .edgeLabel .label text{{fill:var(--textColor);}}\
{id} .edgeLabel .label{{fill:var(--nodeBorder);font-size:14px;}}\
{id} .label{{font-family:{font};color:{label_text_color};}}\
{id} .edge-pattern-dashed{{stroke-dasharray:8,8;}}\
{id} .node rect,{id} .node circle,{id} .node ellipse,{id} .node polygon{{fill:var(--mainBkg);stroke:var(--nodeBorder);stroke-width:{entity_stroke};}}\
{id} .relationshipLine{{stroke:var(--lineColor);stroke-width:{entity_stroke};fill:none;}}\
{id} .marker{{fill:none !important;stroke:var(--lineColor) !important;stroke-width:1;}}\
{id} [data-look=neo].labelBkg{{background-color:rgba({tertiary_fade});}}\
{id} .cluster rect{{fill:var(--clusterBkg);stroke:var(--clusterBorder);stroke-width:1px;}}\
{id} .cluster text{{fill:var(--titleColor);}}\
{id} .cluster-label text{{fill:var(--titleColor);}}\
{id} .node .neo-node{{stroke:var(--nodeBorder);}}\
{id} [data-look=\"neo\"].node rect,{id} [data-look=\"neo\"].cluster rect,{id} [data-look=\"neo\"].node polygon{{stroke:var(--nodeBorder);filter:{shadow};}}\
{id} [data-look=\"neo\"].swimlane.cluster rect{{filter:none;}}\
{id} [data-look=\"neo\"].node path{{stroke:var(--nodeBorder);stroke-width:{stroke_width};}}\
{id} [data-look=\"neo\"].node .outer-path{{filter:{shadow};}}\
{id} [data-look=\"neo\"].node .neo-line path{{stroke:var(--nodeBorder);filter:none;}}\
{id} [data-look=\"neo\"].node circle{{stroke:var(--nodeBorder);filter:{shadow};}}\
{id} [data-look=\"neo\"].node circle .state-start{{fill:#000000;}}\
{id} [data-look=\"neo\"].icon-shape .icon{{fill:var(--nodeBorder);filter:{shadow};}}\
{id} [data-look=\"neo\"].icon-shape .icon-neo path{{stroke:var(--nodeBorder);filter:{shadow};}}\
:root{{--mermaid-font-family: {font};}}"
    );
    if design.is_some() {
        write_design_rules(&mut css, &id);
    }
    assemble_stylesheet(svg_id, families, css, false)
}

/// The rules of the design mode, after the entity rules (`docs/decisions/0023-design-mode.md`):
/// hairline outlines and dividers, a relationship in the edge's weight, dashed when it is not
/// identifying, the markers' paint, the muted columns of an attribute row, the relationship labels
/// in the gap of their line, and a group frame without the node's shadow. An entity's own styles
/// are written on its elements and keep the last word.
fn write_design_rules(out: &mut String, id: &str) {
    let node_width = design::NODE_STROKE_WIDTH;
    let edge_width = design::EDGE_STROKE_WIDTH;
    let thick_width = design::THICK_EDGE_STROKE_WIDTH;
    let _ = write!(
        out,
        "{id} .edge-thickness-normal,{id} .relationshipLine{{stroke-width:{edge_width};}}\
{id} .edge-thickness-thick{{stroke-width:{thick_width};}}\
{id} .edge-pattern-dashed{{stroke-dasharray:5 4 !important;}}\
{id} .node rect,{id} .node polygon,{id} .node path,{id} [data-look=\"neo\"].node path{{stroke-width:{node_width};}}\
{id} .node line.divider{{stroke:var(--nodeBorder);stroke-width:1px;}}\
{id} .node line.row-divider{{stroke:var(--nodeBorder);stroke-width:1px;stroke-opacity:0.3;}}\
{id} .attribute-type text,{id} .attribute-keys text,{id} .attribute-comment text{{fill:var(--titleColor);}}\
{id} .marker.er .design-line{{fill:none;stroke:var(--lineColor);stroke-width:{edge_width};stroke-linecap:butt;stroke-linejoin:round;stroke-dasharray:none;}}\
{id} .marker.er .design-ring{{fill:var(--background);stroke:var(--lineColor);stroke-width:{edge_width};stroke-dasharray:none;}}\
{id} .edgeLabel .label text{{fill:var(--titleColor);}}\
{id} .edgeLabel rect.background,{id} .edgeLabel[data-look] rect.background{{fill:none;stroke:none;opacity:1;}}\
{id} [data-look=\"neo\"].cluster rect{{stroke:var(--clusterBorder);filter:none;}}\
{id}-drop-shadow feDropShadow,{id}-drop-shadow-small feDropShadow{{flood-color:var(--floodColor);flood-opacity:1;}}"
    );
}

/// Appends the custom properties of the design mode: the color tokens of each set and the page's
/// ground, which fills a marker's ring, with the mode's own properties for its tone. With a light
/// and a dark set, the values that differ are `light-dark()` pairs.
fn write_design_properties(css: &mut String, id: &str, theme: ThemeView<'_>, tone: Tone) {
    let write_block = |out: &mut String, tokens: &ThemeTokens, tone: Tone| {
        out.push_str(id);
        out.push('{');
        for (name, token) in color_tokens(tokens)
            .into_iter()
            .chain([("background", &tokens.background)])
        {
            if let Some(color) = token.text() {
                let _ = write!(out, "--{name}:{color};");
            }
        }
        out.push_str(design::declarations(tone));
        out.push('}');
    };
    match theme.dark {
        None => write_block(css, theme.light, tone),
        Some(dark) => {
            let mut light_block = String::new();
            write_block(&mut light_block, theme.light, Tone::Light);
            let mut dark_block = String::new();
            write_block(&mut dark_block, dark, Tone::Dark);
            css.push_str(&crate::scheme::both(&light_block, &dark_block));
        }
    }
}

/// Appends the custom properties of the colors, declared on the diagram's id. With one set of
/// variables they are written as they are; with a light and a dark set, the values that differ are
/// `light-dark()` pairs, so one SVG follows the page's or the viewer's scheme.
fn write_custom_properties(
    css: &mut String,
    id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    palette: Option<&Palette>,
) {
    match theme.dark {
        None => {
            css.push_str(id);
            css.push('{');
            write_declarations(css, theme.light, flood_color(globals), palette);
            css.push('}');
        }
        Some(dark) => {
            let mut light_block = format!("{id}{{");
            write_declarations(&mut light_block, theme.light, LIGHT_FLOOD, palette);
            light_block.push('}');
            let mut dark_block = format!("{id}{{");
            write_declarations(&mut dark_block, dark, DARK_FLOOD, palette);
            dark_block.push('}');
            css.push_str(&crate::scheme::both(&light_block, &dark_block));
        }
    }
}

/// The flood of the drop shadows for a configuration: white for a theme whose name says dark.
fn flood_color(globals: &Globals) -> &'static str {
    if globals.theme.contains("dark") {
        DARK_FLOOD
    } else {
        LIGHT_FLOOD
    }
}

/// Appends the declarations of the custom properties of one set: the color tokens the set has, the
/// slots of the palette (`borderColor<i>` and `bkgColor<i>`, the fills wrapped around the
/// palette's length), and the shadow's flood.
fn write_declarations(
    out: &mut String,
    tokens: &ThemeTokens,
    flood: &str,
    palette: Option<&Palette>,
) {
    for (name, token) in color_tokens(tokens) {
        if let Some(color) = token.text() {
            let _ = write!(out, "--{name}:{color};");
        }
    }
    if let Some(palette) = palette {
        for (index, color) in palette.border.iter().enumerate() {
            let _ = write!(out, "--borderColor{index}:{color};");
        }
        let fills = palette.background.len();
        for index in 0..palette.border.len() {
            let color: Option<Cow<'_, str>> = if fills == 0 {
                tokens.cluster_bkg.text().map(Cow::Owned)
            } else {
                palette
                    .background
                    .get(index % fills)
                    .map(|value| Cow::Borrowed(value.as_str()))
            };
            if let Some(color) = color {
                let _ = write!(out, "--bkgColor{index}:{color};");
            }
        }
    }
    let _ = write!(out, "--floodColor:{flood};");
}

/// The palette rules of `genColor` (`styles.ts` 22-62): for each slot, the entity's path and rect take
/// the slot's border, and its fill when the palette has fills. The slots are the border palette's
/// length, so each entity's `data-color-id` has a rule.
fn write_palette_rules(css: &mut String, id: &str, look: &str, palette: &Palette) {
    let look = safe_look(look);
    let fills = palette.background.len();
    let mut slot = String::new();
    for index in 0..palette.border.len() {
        slot.clear();
        let _ = write!(
            slot,
            "{id} [data-look=\"{look}\"][data-color-id=\"color-{index}\"]"
        );
        for element in [".node path", ".node rect"] {
            let _ = write!(css, "{slot}{element}{{stroke:var(--borderColor{index});");
            if fills > 0 {
                let _ = write!(css, "fill:var(--bkgColor{});", index % fills);
            }
            css.push('}');
        }
    }
}

/// The look a stylesheet is written for: a bare word, else `classic` (`safeLook` in `colorThemeGate.ts`).
fn safe_look(look: &str) -> &str {
    let bare = !look.is_empty()
        && look
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare { look } else { "classic" }
}

/// The color of a token as `khroma.rgba(channel…, alpha)` writes it: the red, green and blue
/// channels of a hex color, comma-separated, with the alpha left to the caller. A color that is not
/// a hex color gives the channels of white (the reference's `fade` on a named color is not reached by
/// the shipped themes, whose tertiary color is a hex color).
fn fade_channels(token: &Token, alpha: f64) -> String {
    let colour = token.text().unwrap_or_default();
    let (red, green, blue) = hex_channels(&colour).unwrap_or((255, 255, 255));
    format!("{red}, {green}, {blue}, {}", crate::svg::number(alpha))
}

/// The channels of a `#rgb` or `#rrggbb` color.
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
    fn a_hex_color_gives_its_channels() {
        assert_eq!(hex_channels("#ffffff"), Some((255, 255, 255)));
        assert_eq!(hex_channels("#28253D"), Some((0x28, 0x25, 0x3d)));
        assert_eq!(hex_channels("#fff"), Some((255, 255, 255)));
        assert_eq!(hex_channels("red"), None);
    }
}
