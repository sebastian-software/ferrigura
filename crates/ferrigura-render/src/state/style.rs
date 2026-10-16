//! The state diagram's style sheet: the global rules of `packages/mermaid/src/styles.ts` that a state
//! diagram gets, then the state rules of `packages/mermaid/src/diagrams/state/styles.js` (the palette
//! rules of its `genColor` first, as the reference's `getStyles` writes them), then the `classDef`
//! rules of the diagram.
//!
//! Colours follow `docs-port/decisions/0007`: every colour is a custom property declared on the
//! diagram's id, once for each set of variables, and the rules refer to it with `var(…)`. Where
//! styles.js writes `a || b`, the rule names the first of the two that the theme gives, which is what
//! the reference's `undefined` fallback does. The `//` comments and the nested `text` rule of
//! `.state-note` are not CSS, and are not written (`docs-port/state/NOTES.md` 5.3).

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::state::ClassDef;
use ferrigura_model::value::Value;
use ferrigura_text::EmbeddedFamily;

use crate::pie::Themes;
use crate::svg::font_faces;
use crate::unified::palette::{Palette, palette};
use crate::variables::{css_text, text_of, truthy};

/// The colour tokens that the rules name, each declared as a custom property on the diagram's id.
const COLOUR_TOKENS: [&str; 24] = [
    "mainBkg",
    "stateBkg",
    "nodeBorder",
    "stateBorder",
    "lineColor",
    "transitionColor",
    "textColor",
    "stateLabelColor",
    "background",
    "noteBorderColor",
    "noteBkgColor",
    "noteTextColor",
    "labelBackgroundColor",
    "edgeLabelBackground",
    "transitionLabelColor",
    "tertiaryTextColor",
    "specialStateColor",
    "innerEndBackground",
    "compositeBackground",
    "altBackground",
    "compositeTitleBackground",
    "errorBkgColor",
    "errorTextColor",
    "clusterBkg",
];

/// The flood of the drop shadows in a light scheme, and in a dark one.
const LIGHT_FLOOD: &str = "#000000";
const DARK_FLOOD: &str = "#FFFFFF";

/// The fill of an alternate composite and of a region when the theme gives no `altBackground`.
const ALT_BACKGROUND: &str = "#efefef";

/// The `<style>` element's content for a state diagram with the id `svg_id`, escaped for XML so that
/// it can be written as the element's text. `families` are the embedded font subsets of the text,
/// whose `@font-face` rules come first, as for the flowchart.
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    themes: Themes<'_>,
    config: &MermaidConfig,
    classes: &[ClassDef],
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
    let shadow = match text_of(variables, "dropShadow") {
        Some(shadow) if !shadow.is_empty() => shadow.replacen(
            "url(#drop-shadow)",
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        ),
        _ => "none".to_owned(),
    };
    let stroke_neo = if truthy(variables, "useGradient") {
        format!("url(#{svg_id}-gradient)")
    } else {
        colour(variables, &["nodeBorder"])
    };
    let radius = text_of(variables, "radius").unwrap_or_else(|| "5".to_owned());

    let mut rules = String::new();
    // styles.ts: the global rules every diagram gets.
    rules.push_str(&rule(
        &id,
        &format!(
            "font-family:{font};font-size:{font_size};fill:{};",
            colour(variables, &["textColor"])
        ),
    ));
    rules.push_str("@keyframes edge-animation-frame{from{stroke-dashoffset:0;}}");
    rules.push_str("@keyframes dash{to{stroke-dashoffset:0;}}");
    rules.push_str(&scoped(
        &id,
        ".edge-animation-slow",
        "stroke-dasharray:9,5 !important;stroke-dashoffset:900;animation:dash 50s linear infinite;stroke-linecap:round;",
    ));
    rules.push_str(&scoped(
        &id,
        ".edge-animation-fast",
        "stroke-dasharray:9,5 !important;stroke-dashoffset:900;animation:dash 20s linear infinite;stroke-linecap:round;",
    ));
    if text_of(variables, "errorBkgColor").is_some() {
        rules.push_str(&scoped(
            &id,
            ".error-icon",
            &format!("fill:{};", colour(variables, &["errorBkgColor"])),
        ));
    }
    if text_of(variables, "errorTextColor").is_some() {
        let text = colour(variables, &["errorTextColor"]);
        rules.push_str(&scoped(
            &id,
            ".error-text",
            &format!("fill:{text};stroke:{text};"),
        ));
    }
    rules.push_str(&scoped(
        &id,
        ".edge-thickness-normal",
        &format!("stroke-width:{stroke_width}px;"),
    ));
    rules.push_str(&scoped(&id, ".edge-thickness-thick", "stroke-width:3.5px;"));
    rules.push_str(&scoped(&id, ".edge-pattern-solid", "stroke-dasharray:0;"));
    rules.push_str(&scoped(
        &id,
        ".edge-thickness-invisible",
        "stroke-width:0;fill:none;",
    ));
    rules.push_str(&scoped(&id, ".edge-pattern-dashed", "stroke-dasharray:3;"));
    rules.push_str(&scoped(&id, ".edge-pattern-dotted", "stroke-dasharray:2;"));
    let line = colour(variables, &["lineColor"]);
    rules.push_str(&scoped(
        &id,
        ".marker",
        &format!("fill:{line};stroke:{line};"),
    ));
    rules.push_str(&scoped(&id, ".marker.cross", &format!("stroke:{line};")));
    rules.push_str(&scoped(
        &id,
        "svg",
        &format!("font-family:{font};font-size:{font_size};"),
    ));
    rules.push_str(&scoped(&id, "p", "margin:0;"));
    rules.push_str(&scoped(
        &id,
        ".node .neo-node",
        &format!("stroke:{};", colour(variables, &["nodeBorder"])),
    ));
    rules.push_str(&scoped(
        &id,
        "[data-look=\"neo\"].node rect,[data-look=\"neo\"].cluster rect,[data-look=\"neo\"].node polygon",
        &format!("stroke:{stroke_neo};filter:{shadow};"),
    ));
    rules.push_str(&scoped(
        &id,
        "[data-look=\"neo\"].swimlane.cluster rect",
        "filter:none;",
    ));
    rules.push_str(&scoped(
        &id,
        "[data-look=\"neo\"].node path",
        &format!("stroke:{stroke_neo};stroke-width:{stroke_width}px;"),
    ));
    rules.push_str(&scoped(
        &id,
        "[data-look=\"neo\"].node .outer-path",
        &format!("filter:{shadow};"),
    ));
    rules.push_str(&scoped(
        &id,
        "[data-look=\"neo\"].node .neo-line path",
        &format!("stroke:{};filter:none;", colour(variables, &["nodeBorder"])),
    ));
    rules.push_str(&scoped(
        &id,
        "[data-look=\"neo\"].node circle",
        &format!("stroke:{stroke_neo};filter:{shadow};"),
    ));
    rules.push_str(&scoped(
        &id,
        "[data-look=\"neo\"].node circle .state-start",
        "fill:#000000;",
    ));
    rules.push_str(&scoped(
        &id,
        "[data-look=\"neo\"].icon-shape .icon",
        &format!("fill:{stroke_neo};filter:{shadow};"),
    ));
    rules.push_str(&scoped(
        &id,
        "[data-look=\"neo\"].icon-shape .icon-neo path",
        &format!("stroke:{stroke_neo};filter:{shadow};"),
    ));

    // styles.js: the palette rules of each container slot, then the state rules.
    rules.push_str(&palette_rules(&id, config, variables));
    rules.push_str(&state_rules(
        &id,
        variables,
        &stroke_width,
        &radius,
        &shadow,
    ));

    // The diagram's `classDef` rules: the classes' declarations, on every state of the class.
    for class in classes {
        if class.styles.is_empty() {
            continue;
        }
        let declarations: Vec<String> = class
            .styles
            .iter()
            .map(|style| format!("{style} !important"))
            .collect();
        rules.push_str(&scoped(
            &id,
            &format!(".{} > *", class.id),
            &format!("{};", declarations.join(";")),
        ));
    }

    let faces = if families.is_empty() {
        String::new()
    } else {
        font_faces(families)
    };
    css_text(&format!(
        "{faces}{}{rules}",
        custom_properties(svg_id, themes, config)
    ))
}

/// The state rules of `styles.js` `getStyles`, in the reference's order, after the palette rules.
fn state_rules(
    id: &str,
    variables: &Value,
    stroke_width: &str,
    radius: &str,
    shadow: &str,
) -> String {
    let transition = colour(variables, &["transitionColor"]);
    let main_bkg = colour(variables, &["mainBkg"]);
    let state_border = colour(variables, &["stateBorder", "nodeBorder"]);
    let text = colour(variables, &["textColor"]);
    let state_label = colour(variables, &["stateLabelColor"]);
    let line = colour(variables, &["lineColor"]);
    let background = colour(variables, &["background"]);
    let composite_background = colour(variables, &["compositeBackground", "background"]);
    let alt_background = if text_of(variables, "altBackground").is_some() {
        colour(variables, &["altBackground"])
    } else {
        ALT_BACKGROUND.to_owned()
    };
    let special = colour(variables, &["specialStateColor"]);
    let inner_end = colour(variables, &["innerEndBackground"]);
    let edge_label = colour(variables, &["edgeLabelBackground"]);
    let label_background = colour(variables, &["labelBackgroundColor"]);
    let transition_label = colour(variables, &["transitionLabelColor", "tertiaryTextColor"]);
    let note_border = colour(variables, &["noteBorderColor"]);
    let note_bkg = colour(variables, &["noteBkgColor"]);
    let note_text = colour(variables, &["noteTextColor"]);
    let composite_title = colour(variables, &["compositeTitleBackground"]);
    let node_border = colour(variables, &["nodeBorder"]);
    let state_bkg = colour(variables, &["stateBkg", "mainBkg"]);
    let stroke_px = format!("{stroke_width}px");
    let rounded = format!("{radius}px");

    let mut out = String::new();
    let mut add = |selector: &str, declarations: &str| {
        out.push_str(&scoped(id, selector, declarations));
    };
    add(
        "defs [id$=\"-barbEnd\"]",
        &format!("fill:{transition};stroke:{transition};"),
    );
    add(
        "g.stateGroup text",
        &format!("fill:{node_border};stroke:none;font-size:10px;"),
    );
    add(
        "g.stateGroup text",
        &format!("fill:{text};stroke:none;font-size:10px;"),
    );
    add(
        "g.stateGroup .state-title",
        &format!("font-weight:bolder;fill:{state_label};"),
    );
    add(
        "g.stateGroup rect",
        &format!("fill:{main_bkg};stroke:{node_border};"),
    );
    add(
        "g.stateGroup line",
        &format!("stroke:{line};stroke-width:{stroke_width};"),
    );
    add(
        ".transition",
        &format!("stroke:{transition};stroke-width:{stroke_width};fill:none;"),
    );
    add(
        ".stateGroup .composit",
        &format!("fill:{background};border-bottom:1px;"),
    );
    add(
        ".stateGroup .alt-composit",
        "fill:#e0e0e0;border-bottom:1px;",
    );
    add(
        ".state-note",
        &format!("stroke:{note_border};fill:{note_bkg};"),
    );
    add(
        ".stateLabel .box",
        &format!("stroke:none;stroke-width:0;fill:{main_bkg};opacity:0.5;"),
    );
    add(
        ".edgeLabel .label rect",
        &format!("fill:{label_background};opacity:0.5;"),
    );
    add(
        ".edgeLabel",
        &format!("background-color:{edge_label};text-align:center;"),
    );
    add(".edgeLabel p", &format!("background-color:{edge_label};"));
    add(
        ".edgeLabel rect",
        &format!("opacity:0.5;background-color:{edge_label};fill:{edge_label};"),
    );
    add(
        ".edgeLabel .label text",
        &format!("fill:{transition_label};"),
    );
    add(
        ".label div .edgeLabel",
        &format!("color:{transition_label};"),
    );
    add(
        ".stateLabel text",
        &format!("fill:{state_label};font-size:10px;font-weight:bold;"),
    );
    add(
        ".node circle.state-start",
        &format!("fill:{special};stroke:{special};"),
    );
    add(
        ".node .fork-join",
        &format!("fill:{special};stroke:{special};"),
    );
    add(
        ".node circle.state-end",
        &format!("fill:{inner_end};stroke:{background};stroke-width:1.5;"),
    );
    add(
        ".end-state-inner",
        &format!("fill:{composite_background};stroke-width:1.5;"),
    );
    add(
        ".node rect",
        &format!("fill:{state_bkg};stroke:{state_border};stroke-width:{stroke_px};"),
    );
    add(
        ".node polygon",
        &format!("fill:{main_bkg};stroke:{state_border};stroke-width:{stroke_px};"),
    );
    add("[id$=\"-barbEnd\"]", &format!("fill:{line};"));
    add(
        ".statediagram-cluster rect",
        &format!("fill:{composite_title};stroke:{state_border};stroke-width:{stroke_px};"),
    );
    add(
        ".cluster-label, .nodeLabel",
        &format!("color:{state_label};"),
    );
    add(".statediagram-cluster rect.outer", "rx:5px;ry:5px;");
    add(
        ".statediagram-state .divider",
        &format!("stroke:{state_border};"),
    );
    add(".statediagram-state .title-state", "rx:5px;ry:5px;");
    add(
        ".statediagram-cluster.statediagram-cluster .inner",
        &format!("fill:{composite_background};"),
    );
    add(
        ".statediagram-cluster.statediagram-cluster-alt .inner",
        &format!("fill:{alt_background};"),
    );
    add(".statediagram-cluster .inner", "rx:0;ry:0;");
    add(".statediagram-state rect.basic", "rx:5px;ry:5px;");
    add(
        ".statediagram-state rect.divider",
        &format!("stroke-dasharray:10,10;fill:{alt_background};"),
    );
    add(".note-edge", "stroke-dasharray:5;");
    add(
        ".statediagram-note rect",
        &format!("fill:{note_bkg};stroke:{note_border};stroke-width:1px;rx:0;ry:0;"),
    );
    add(".statediagram-note text", &format!("fill:{note_text};"));
    add(
        ".statediagram-note .nodeLabel",
        &format!("color:{note_text};"),
    );
    add(".statediagram .edgeLabel", "color:red;");
    add(
        "[id$=\"-dependencyStart\"], [id$=\"-dependencyEnd\"]",
        &format!("fill:{line};stroke:{line};stroke-width:{stroke_width};"),
    );
    add(
        ".statediagramTitleText",
        &format!("text-anchor:middle;font-size:18px;fill:{text};"),
    );
    add(
        "[data-look=\"neo\"].statediagram-cluster rect",
        &format!(
            "fill:{main_bkg};stroke:{};stroke-width:{stroke_width};",
            if truthy(variables, "useGradient") {
                format!("url(#{}-gradient)", id.trim_start_matches('#'))
            } else {
                state_border.clone()
            }
        ),
    );
    add(
        "[data-look=\"neo\"].statediagram-cluster rect.outer",
        &format!("rx:{rounded};ry:{rounded};filter:{shadow};"),
    );
    out
}

/// The palette rules of `genColor` (`styles.js` 21-99): for each slot of the border palette, the
/// outline and the tint of its containers. A theme without a palette, or one that is not a colour
/// theme, has none.
fn palette_rules(id: &str, config: &MermaidConfig, variables: &Value) -> String {
    let Some(palette) = palette(config, variables) else {
        return String::new();
    };
    let look = safe_look(&config.look);
    let mut out = String::new();
    for index in 0..palette.border.len() {
        let slot = format!("[data-look=\"{look}\"][data-color-id=\"color-{index}\"]");
        let border = &palette.border[index];
        let tint = tint(&palette, index);
        out.push_str(&scoped(
            id,
            &format!("{slot}.statediagram-cluster rect.outer"),
            &format!("stroke:{border};{tint}"),
        ));
        out.push_str(&scoped(
            id,
            &format!("{slot}.statediagram-cluster rect.inner"),
            &format!("stroke:{border};"),
        ));
        out.push_str(&scoped(
            id,
            &format!("{slot}.statediagram-cluster rect.divider"),
            &format!("stroke:{border};{tint}"),
        ));
        out.push_str(&scoped(
            id,
            &format!("{slot}.statediagram-cluster .outer path[stroke='none']"),
            &tint,
        ));
        out.push_str(&scoped(
            id,
            &format!("{slot}.statediagram-cluster .outer path[fill='none']"),
            &format!("stroke:{border};"),
        ));
        out.push_str(&scoped(
            id,
            &format!("{slot}.statediagram-cluster .divider path[stroke='none']"),
            &tint,
        ));
        out.push_str(&scoped(
            id,
            &format!("{slot}.statediagram-cluster .divider path[fill='none']"),
            &format!("stroke:{border};"),
        ));
    }
    out
}

/// The fill of a slot: the palette's background at the slot's index, wrapped to its length, or
/// nothing when the palette has no fills (`genColor`'s `tint`).
fn tint(palette: &Palette, index: usize) -> String {
    if palette.background.is_empty() {
        String::new()
    } else {
        format!(
            "fill:{};",
            palette.background[index % palette.background.len()]
        )
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

/// A colour of the rules: `var(--name)` for the first of the names the theme gives, else for the
/// first name (the reference's `a || b`).
fn colour(variables: &Value, names: &[&str]) -> String {
    let name = names
        .iter()
        .find(|name| text_of(variables, name).is_some())
        .unwrap_or(&names[0]);
    format!("var(--{name})")
}

/// One rule under the diagram's id: the selectors of `selectors`, each scoped to `id`.
fn scoped(id: &str, selectors: &str, declarations: &str) -> String {
    let scoped: Vec<String> = selectors
        .split(',')
        .map(|selector| format!("{id} {}", selector.trim()))
        .collect();
    rule(&scoped.join(","), declarations)
}

/// One rule: the selector and its declarations, in braces.
fn rule(selector: &str, declarations: &str) -> String {
    format!("{selector}{{{declarations}}}")
}

/// The custom properties of the colours, declared on the diagram's id. With one set of variables they
/// are written as they are; with a light and a dark set, the dark set applies under
/// `prefers-color-scheme: dark`, as the flowchart does.
fn custom_properties(svg_id: &str, themes: Themes<'_>, config: &MermaidConfig) -> String {
    let id = format!("#{svg_id}");
    match themes {
        Themes::Single(variables) => rule(&id, &declarations(variables, flood(config))),
        Themes::Schemes { light, dark } => format!(
            "{}@media (prefers-color-scheme: dark){{{}}}",
            rule(&id, &declarations(light, LIGHT_FLOOD)),
            rule(&id, &declarations(dark, DARK_FLOOD)),
        ),
    }
}

/// The flood of the drop shadows for a configuration: white for a theme whose name says dark.
fn flood(config: &MermaidConfig) -> &'static str {
    if config.theme.contains("dark") {
        DARK_FLOOD
    } else {
        LIGHT_FLOOD
    }
}

/// The declarations of the custom properties of one set of variables.
fn declarations(variables: &Value, flood: &str) -> String {
    let mut out = String::new();
    for token in COLOUR_TOKENS {
        if let Some(colour) = text_of(variables, token) {
            out.push_str(&format!("--{token}:{};", css_text(&colour)));
        }
    }
    out.push_str(&format!("--floodColor:{flood};"));
    out
}
