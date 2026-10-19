//! The class diagram's style sheet: `packages/mermaid/src/diagrams/class/styles.js`, with the common
//! rules of `packages/mermaid/src/styles.ts` that every diagram gets. Each selector starts with the
//! diagram's id, as the reference's stylis scoping writes it.
//!
//! Colours follow `docs-port/decisions/0007`: every colour is a custom property declared on the
//! diagram's id (once, or once per scheme under `prefers-color-scheme`), and the rules refer to it
//! with `var(…)`. The palette of the colour themes gives each class box its border and fill by its
//! colour slot (`genColor`, `styles.js` 14-38). The elements carry no colour attribute of their own,
//! as the flowchart's do not (`flowchart/style.rs`).

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::theme::ThemeTokens;
use ferrigura_model::value::Value;
use ferrigura_text::EmbeddedFamily;

use crate::context::Globals;
use crate::flowchart::style::look_defs;
use crate::svg::font_faces;
use crate::theme::Themes;
use crate::unified::Bridge;
use crate::unified::palette::{Palette, palette};
use crate::variables::{css_text, text_of, texts_of, truthy};

/// The colour tokens of the class rules: the theme variables that `styles.js` reads.
const COLOUR_TOKENS: [&str; 10] = [
    "nodeBorder",
    "mainBkg",
    "classText",
    "titleColor",
    "clusterBkg",
    "clusterBorder",
    "noteTextColor",
    "lineColor",
    "textColor",
    "edgeLabelBackground",
];

/// The flood of the drop shadows in a light scheme (`insertLookDefs.ts:30`).
const LIGHT_FLOOD: &str = "#000000";

/// The flood of the drop shadows in a dark scheme (`insertLookDefs.ts:30`).
const DARK_FLOOD: &str = "#FFFFFF";

/// The `<style>` element's text of a class diagram with the id `svg_id`. The font of the rules is
/// the embedded family of the diagram when it has one (`families`), else the theme's `fontFamily`.
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
    let shadow = match text_of(variables, "dropShadow") {
        Some(shadow) if !shadow.is_empty() => shadow.replacen(
            "url(#drop-shadow)",
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        ),
        _ => "none".to_owned(),
    };
    let neo_stroke = if truthy(variables, "useGradient") {
        format!("url(#{svg_id}-gradient)")
    } else {
        "var(--nodeBorder)".to_owned()
    };
    let look = safe_look(&config.look);

    let faces = if families.is_empty() {
        String::new()
    } else {
        font_faces(families)
    };
    let mut css = custom_properties(svg_id, themes, config);
    css.push_str(&format!(
        "{id}{{font-family:{font};font-size:{font_size};fill:var(--textColor);}}\
         @keyframes dash{{to{{stroke-dashoffset:0;}}}}\
         {id} .edge-animation-slow{{stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 50s linear infinite;stroke-linecap:round;}}\
         {id} .edge-animation-fast{{stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 20s linear infinite;stroke-linecap:round;}}\
         {id} .edge-thickness-normal{{stroke-width:{stroke_width}px;}}\
         {id} .edge-thickness-thick{{stroke-width:3.5px;}}\
         {id} .edge-pattern-solid{{stroke-dasharray:0;}}\
         {id} .edge-thickness-invisible{{stroke-width:0;fill:none;}}\
         {id} .edge-pattern-dashed{{stroke-dasharray:3;}}\
         {id} .edge-pattern-dotted{{stroke-dasharray:2;}}\
         {id} .marker{{fill:var(--lineColor);stroke:var(--lineColor);}}\
         {id} .marker.cross{{stroke:var(--lineColor);}}\
         {id} svg{{font-family:{font};font-size:{font_size};}}\
         {id} p{{margin:0;}}\
         {id} .node .neo-node{{stroke:var(--nodeBorder);}}\
         {id} [data-look=\"neo\"].node rect,{id} [data-look=\"neo\"].cluster rect,{id} [data-look=\"neo\"].node polygon{{stroke:{neo_stroke};filter:{shadow};}}\
         {id} [data-look=\"neo\"].swimlane.cluster rect{{filter:none;}}\
         {id} [data-look=\"neo\"].node path{{stroke:{neo_stroke};stroke-width:{stroke_width}px;}}\
         {id} [data-look=\"neo\"].node .outer-path{{filter:{shadow};}}\
         {id} [data-look=\"neo\"].node .neo-line path{{stroke:var(--nodeBorder);filter:none;}}\
         {id} [data-look=\"neo\"].node circle{{stroke:{neo_stroke};filter:{shadow};}}\
         {id} [data-look=\"neo\"].node circle .state-start{{fill:#000000;}}\
         {id} [data-look=\"neo\"].icon-shape .icon{{fill:{neo_stroke};filter:{shadow};}}\
         {id} [data-look=\"neo\"].icon-shape .icon-neo path{{stroke:{neo_stroke};filter:{shadow};}}"
    ));
    css.push_str(&class_rules(svg_id, look, &font, variables, config));
    css_text(&format!("{faces}{css}"))
}

/// The `<defs>` of the look (the drop shadows and the gradient), which the class diagram writes after
/// its title as the flowchart does.
#[must_use]
pub fn defs(svg_id: &str, themes: Themes<'_>, config: &MermaidConfig) -> String {
    let bridge = Bridge::new(config, themes);
    look_defs(svg_id, bridge.view(), &bridge.globals)
}

/// The class rules of `styles.js` (lines 42-241) after the common ones, with the palette rules first
/// (`genColor`).
fn class_rules(
    svg_id: &str,
    look: &str,
    font: &str,
    variables: &Value,
    config: &MermaidConfig,
) -> String {
    let id = format!("#{svg_id}");
    let mut css = palette_rules(
        &id,
        look,
        palette(&Globals::of(config), &ThemeTokens::from_value(variables)).as_ref(),
    );
    css.push_str(&format!(
        "{id} g.classGroup text{{fill:var(--nodeBorder);stroke:none;font-family:{font};font-size:10px;}}\
         {id} g.classGroup text .title{{font-weight:bolder;}}\
         {id} .cluster-label text{{fill:var(--titleColor);}}\
         {id} .cluster-label span{{color:var(--titleColor);}}\
         {id} .cluster-label span p{{background-color:transparent;}}\
         {id} .cluster rect{{fill:var(--clusterBkg);stroke:var(--clusterBorder);stroke-width:1px;}}\
         {id} .cluster text{{fill:var(--titleColor);}}\
         {id} .cluster span{{color:var(--titleColor);}}\
         {id} .nodeLabel,{id} .edgeLabel{{color:var(--classText);}}\
         {id} .noteLabel .nodeLabel,{id} .noteLabel .edgeLabel{{color:var(--noteTextColor);}}\
         {id} .edgeLabel .label rect{{fill:var(--mainBkg);}}\
         {id} .label text{{fill:var(--classText);}}\
         {id} .labelBkg{{background:var(--mainBkg);}}\
         {id} .edgeLabel .label span{{background:var(--mainBkg);}}\
         {id} .classTitle{{font-weight:bolder;}}\
         {id} .node rect,{id} .node circle,{id} .node ellipse,{id} .node polygon,{id} .node path{{fill:var(--mainBkg);stroke:var(--nodeBorder);stroke-width:var(--strokeWidth);}}\
         {id} .divider{{stroke:var(--nodeBorder);stroke-width:1;}}\
         {id} g.clickable{{cursor:pointer;}}\
         {id} g.classGroup rect{{fill:var(--mainBkg);stroke:var(--nodeBorder);}}\
         {id} g.classGroup line{{stroke:var(--nodeBorder);stroke-width:1;}}\
         {id} .classLabel .box{{stroke:none;stroke-width:0;fill:var(--mainBkg);opacity:0.5;}}\
         {id} .classLabel .label{{fill:var(--nodeBorder);font-size:10px;}}\
         {id} .relation{{stroke:var(--lineColor);stroke-width:var(--strokeWidth);fill:none;}}\
         {id} .dashed-line{{stroke-dasharray:3;}}\
         {id} .dotted-line{{stroke-dasharray:1 2;}}\
         {id} [id$=\"-compositionStart\"],{id} .composition{{fill:var(--lineColor)!important;stroke:var(--lineColor)!important;stroke-width:1;}}\
         {id} [id$=\"-compositionEnd\"],{id} .composition{{fill:var(--lineColor)!important;stroke:var(--lineColor)!important;stroke-width:1;}}\
         {id} [id$=\"-dependencyStart\"],{id} .dependency{{fill:var(--lineColor)!important;stroke:var(--lineColor)!important;stroke-width:1;}}\
         {id} [id$=\"-dependencyEnd\"],{id} .dependency{{fill:var(--lineColor)!important;stroke:var(--lineColor)!important;stroke-width:1;}}\
         {id} [id$=\"-extensionStart\"],{id} .extension{{fill:transparent!important;stroke:var(--lineColor)!important;stroke-width:1;}}\
         {id} [id$=\"-extensionEnd\"],{id} .extension{{fill:transparent!important;stroke:var(--lineColor)!important;stroke-width:1;}}\
         {id} [id$=\"-aggregationStart\"],{id} .aggregation{{fill:transparent!important;stroke:var(--lineColor)!important;stroke-width:1;}}\
         {id} [id$=\"-aggregationEnd\"],{id} .aggregation{{fill:transparent!important;stroke:var(--lineColor)!important;stroke-width:1;}}\
         {id} [id$=\"-lollipopStart\"],{id} .lollipop{{fill:var(--mainBkg)!important;stroke:var(--lineColor)!important;stroke-width:1;}}\
         {id} [id$=\"-lollipopEnd\"],{id} .lollipop{{fill:var(--mainBkg)!important;stroke:var(--lineColor)!important;stroke-width:1;}}\
         {id} .edgeTerminals{{font-size:11px;line-height:initial;}}\
         {id} .classTitleText{{text-anchor:middle;font-size:18px;fill:var(--textColor);}}\
         {id} .edgeLabel[data-look=\"neo\"]{{background-color:var(--edgeLabelBackground);}}\
         {id} .edgeLabel[data-look=\"neo\"] p{{background-color:var(--edgeLabelBackground);}}\
         {id} .edgeLabel[data-look=\"neo\"] rect{{opacity:0.5;background-color:var(--edgeLabelBackground);fill:var(--edgeLabelBackground);}}\
         {id} .edgeLabel[data-look=\"neo\"]{{text-align:center;}}"
    ));
    css
}

/// `genColor` (`styles.js` 14-38): for each colour slot of the palette, the outline of a class box
/// takes the slot's border and fill, and its divider the border. The rules name the look and the
/// slot in their selectors, and the custom properties of the slot.
fn palette_rules(id: &str, look: &str, palette: Option<&Palette>) -> String {
    let Some(palette) = palette else {
        return String::new();
    };
    let fills = palette.background.len();
    let mut css = String::new();
    for index in 0..palette.border.len() {
        let slot = format!("{id} [data-look=\"{look}\"][data-color-id=\"color-{index}\"].node");
        let fill = if fills == 0 {
            String::new()
        } else {
            format!("fill:var(--bkgColor{});", index % fills)
        };
        css.push_str(&format!(
            "{slot} .outer-path path{{stroke:var(--borderColor{index});{fill}}}\
             {slot} .divider path{{stroke:var(--borderColor{index});}}"
        ));
    }
    css
}

/// The custom properties of the colours, declared on the diagram's id. With one set they are
/// written as they are; with a light and a dark set, the dark one applies under
/// `prefers-color-scheme: dark` (decision 0007).
fn custom_properties(svg_id: &str, themes: Themes<'_>, config: &MermaidConfig) -> String {
    let id = format!("#{svg_id}");
    match themes {
        Themes::Single(variables) => {
            let flood = if config.theme.contains("dark") {
                DARK_FLOOD
            } else {
                LIGHT_FLOOD
            };
            format!(
                "{id}{{{}}}",
                declarations(
                    variables,
                    palette(&Globals::of(config), &ThemeTokens::from_value(variables)),
                    flood
                )
            )
        }
        Themes::Schemes { light, dark } => format!(
            "{id}{{{}}}@media (prefers-color-scheme: dark){{{id}{{{}}}}}",
            declarations(
                light,
                palette(&Globals::of(config), &ThemeTokens::from_value(light)),
                LIGHT_FLOOD
            ),
            declarations(
                dark,
                palette(&Globals::of(config), &ThemeTokens::from_value(dark)),
                DARK_FLOOD
            ),
        ),
    }
}

/// The declarations of the custom properties of one set of variables.
fn declarations(variables: &Value, palette: Option<Palette>, flood: &str) -> String {
    let mut out = String::new();
    for token in COLOUR_TOKENS {
        if let Some(colour) = text_of(variables, token) {
            out.push_str(&format!("--{token}:{};", css_text(&colour)));
        }
    }
    if let Some(palette) = palette {
        for (index, colour) in palette.border.iter().enumerate() {
            out.push_str(&format!("--borderColor{index}:{};", css_text(colour)));
        }
        let fills = texts_of(variables, "bkgColorArray");
        for index in 0..palette.background.len() {
            let colour = if fills.is_empty() {
                text_of(variables, "clusterBkg").unwrap_or_default()
            } else {
                fills[index % fills.len()].clone()
            };
            out.push_str(&format!("--bkgColor{index}:{};", css_text(&colour)));
        }
    }
    out.push_str(&format!("--floodColor:{flood};"));
    out
}

/// The look a stylesheet is written for: a bare word, else `classic` (`safeLook`).
fn safe_look(look: &str) -> &str {
    let bare = !look.is_empty()
        && look
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare { look } else { "classic" }
}
