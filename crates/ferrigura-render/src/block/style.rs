//! The block diagram's style sheet: the rules of `diagrams/block/styles.ts`, the global rules of the
//! unified stylesheet that its elements use (`styles.ts` at the top level, `mermaidAPI.ts`), the
//! `classDef` rules of `createCssStyles` and the fonts. The palette rules of a colour theme are
//! `styles.ts`'s `genColor`, one per slot, on the composites only.
//!
//! Colours follow decision 0007: each colour is a custom property declared on the diagram's id, and the
//! rules refer to it with `var(…)`. The one exception is the composite's fill and stroke (`.node .cluster`),
//! which the reference makes with `khroma`'s `fade`: an `rgba` with the alpha written in, since a custom
//! property cannot carry one.

use ferrigura_model::block::ClassDef;
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::value::Value;
use ferrigura_text::EmbeddedFamily;

use crate::flowchart::style::{custom_properties, rule};
use crate::svg::font_faces;
use crate::theme::Themes;
use crate::unified::palette::palette;
use crate::variables::{css_text, text_of};

/// The style sheet's content for a block diagram `svg_id`, escaped for XML: the custom properties, then
/// the rules. `families` are the embedded fonts of the labels, whose `@font-face` rules come first.
#[must_use]
pub fn style_sheet_with_fonts(
    svg_id: &str,
    themes: Themes<'_>,
    config: &MermaidConfig,
    class_defs: &[ClassDef],
    families: &[EmbeddedFamily],
) -> String {
    let variables = themes.primary();
    let id = format!("#{svg_id}");
    let font = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| format!("\"{}\"", family.name),
    );
    let font_size = text_of(variables, "fontSize").unwrap_or_else(|| "16px".to_owned());
    let stroke_width = text_of(variables, "strokeWidth").unwrap_or_else(|| "1".to_owned());
    let text_colour = if variables.get("nodeTextColor").is_some() {
        "nodeTextColor"
    } else {
        "textColor"
    };
    let look = safe_look(&config.look);

    let mut rules = vec![
        rule(
            &id,
            &format!("font-family:{font};font-size:{font_size};fill:var(--textColor);"),
        ),
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
            "fill:var(--lineColor);stroke:var(--lineColor);",
        ),
        rule(&format!("{id} .marker.cross"), "stroke:var(--lineColor);"),
        // `styles.ts`: the label and the text of the nodes and the clusters.
        rule(
            &format!("{id} .label"),
            &format!("font-family:{font};color:var(--{text_colour});"),
        ),
        rule(
            &format!("{id} .cluster-label text"),
            "fill:var(--titleColor);",
        ),
        rule(
            &format!("{id} .cluster-label span"),
            "color:var(--titleColor);",
        ),
        rule(
            &format!("{id} .label text,{id} span"),
            &format!("fill:var(--{text_colour});color:var(--{text_colour});"),
        ),
        rule(
            &format!(
                "{id} .node rect,{id} .node circle,{id} .node ellipse,{id} .node polygon,{id} .node path"
            ),
            &format!("fill:var(--mainBkg);stroke:var(--nodeBorder);stroke-width:{stroke_width}px;"),
        ),
        rule(
            &format!("{id} .flowchart-label text"),
            "text-anchor:middle;",
        ),
        rule(&format!("{id} .node .label"), "text-align:center;"),
        rule(&format!("{id} .node.clickable"), "cursor:pointer;"),
        rule(
            &format!("{id} .arrowheadPath"),
            "fill:var(--arrowheadColor);",
        ),
        rule(
            &format!("{id} .edgePaths .path"),
            "stroke:var(--lineColor);stroke-width:2.0px;",
        ),
        rule(
            &format!("{id} .flowchart-link"),
            "stroke:var(--lineColor);fill:none;",
        ),
        rule(
            &format!("{id} .edgeLabel"),
            "background-color:var(--edgeLabelBackground);text-align:center;",
        ),
        rule(
            &format!("{id} .edgeLabel p"),
            "margin:0;padding:0;display:inline;",
        ),
        rule(
            &format!("{id} .edgeLabel rect"),
            "opacity:0.5;background-color:var(--edgeLabelBackground);fill:var(--edgeLabelBackground);",
        ),
        rule(
            &format!("{id} .labelBkg"),
            "background-color:var(--edgeLabelBackground);",
        ),
        rule(&format!("{id} .cluster text"), "fill:var(--titleColor);"),
        rule(&format!("{id} .cluster span"), "color:var(--titleColor);"),
        rule(
            &format!("{id} .flowchartTitleText"),
            "text-anchor:middle;font-size:18px;fill:var(--textColor);",
        ),
        composite_cluster_colours(&id, themes, &stroke_width),
    ];
    if let Some(palette) = palette(config, variables) {
        rules.push(palette_rules(&id, look, palette.border.len()));
    }
    rules.push(class_rules(&id, class_defs));
    rules.push(rule(&format!("{id} text"), "white-space:pre;"));

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

/// The composite's cluster colours for the scheme or schemes of the theme. With a light and a dark set, the
/// dark rule applies under `prefers-color-scheme: dark`, as the custom properties do, since the colours are
/// written with their alpha and cannot follow a custom property.
fn composite_cluster_colours(id: &str, themes: Themes<'_>, stroke_width: &str) -> String {
    match themes {
        Themes::Single(variables) => composite_cluster_rule(id, variables, stroke_width),
        Themes::Schemes { light, dark } => format!(
            "{}@media (prefers-color-scheme: dark){{{}}}",
            composite_cluster_rule(id, light, stroke_width),
            composite_cluster_rule(id, dark, stroke_width),
        ),
    }
}

/// `styles.ts` `.node .cluster`: a composite's fill and stroke are the cluster colours at half and a
/// fifth of their opacity, with the shadow of a cluster.
fn composite_cluster_rule(id: &str, variables: &Value, stroke_width: &str) -> String {
    let fill =
        text_of(variables, "clusterBkg").map_or_else(String::new, |colour| fade(&colour, 0.5));
    let stroke =
        text_of(variables, "clusterBorder").map_or_else(String::new, |colour| fade(&colour, 0.2));
    rule(
        &format!("{id} .node .cluster"),
        &format!(
            "fill:{fill};stroke:{stroke};box-shadow:rgba(50, 50, 93, 0.25) 0px 13px 27px -5px, rgba(0, 0, 0, 0.3) 0px 8px 16px -8px;stroke-width:{stroke_width}px;"
        ),
    )
}

/// `khroma`'s `fade` (`styles.ts`): the colour with the given opacity, written as `rgba(r, g, b, a)`. A
/// colour that is not a six- or three-digit hex colour is kept as it is.
fn fade(colour: &str, opacity: f64) -> String {
    let Some(hex) = colour.trim().strip_prefix('#') else {
        return colour.to_owned();
    };
    let channels: Option<Vec<u8>> = match hex.len() {
        3 => hex
            .chars()
            .map(|c| c.to_digit(16).map(|value| (value * 17) as u8))
            .collect(),
        6 => (0..3)
            .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok())
            .collect(),
        _ => None,
    };
    match channels.as_deref() {
        Some([r, g, b]) => format!("rgba({r}, {g}, {b}, {opacity})"),
        _ => colour.to_owned(),
    }
}

/// The palette rules (`genColor`): for each slot, the composites of that slot take its border and its
/// fill. They are not `!important`, so a block's own `style` wins over the palette.
fn palette_rules(id: &str, look: &str, slots: usize) -> String {
    let mut css = String::new();
    for index in 0..slots {
        css.push_str(&rule(
            &format!(
                "{id} [data-look=\"{look}\"][data-color-id=\"color-{index}\"].node rect.composite"
            ),
            &format!("stroke:var(--borderColor{index});fill:var(--bkgColor{index});"),
        ));
    }
    css
}

/// The `classDef` rules (`createCssStyles`, `mermaidAPI.ts` 161-196): each class styles the shapes of its
/// nodes with `!important`, and the text of its labels when it names a colour. A class whose styles hold
/// braces is left out.
fn class_rules(id: &str, class_defs: &[ClassDef]) -> String {
    let mut css = String::new();
    for class in class_defs {
        if let Some(declarations) = important(&class.styles) {
            for element in ["rect", "polygon", "ellipse", "circle", "path"] {
                css.push_str(&rule(
                    &format!("{id} .{} {element}", class.id),
                    &declarations,
                ));
            }
        }
        if let Some(declarations) = important(&class.text_styles) {
            css.push_str(&rule(&format!("{id} .{} tspan", class.id), &declarations));
        }
    }
    css
}

/// The declarations of a list with `!important`, or `None` when there are none or they hold braces.
fn important(styles: &[String]) -> Option<String> {
    let declarations: String = styles
        .iter()
        .map(|style| style.trim())
        .filter(|style| !style.is_empty())
        .map(|style| format!("{style} !important;"))
        .collect();
    if declarations.is_empty() || declarations.contains(['{', '}']) {
        None
    } else {
        Some(declarations)
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
