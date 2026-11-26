//! The state diagram's style sheet: the global rules of `packages/mermaid/src/styles.ts` that a state
//! diagram gets, then the state rules of `packages/mermaid/src/diagrams/state/styles.js` (the palette
//! rules of its `genColor` first, as the reference's `getStyles` writes them), then the `classDef`
//! rules of the diagram.
//!
//! Colors follow `docs/decisions/0011-themes-as-css-variables.md`: every color is a custom property declared on the
//! diagram's id, once for each set of variables, and the rules refer to it with `var(…)`. Where
//! styles.js writes `a || b`, the rule names the first of the two that the theme gives, which is what
//! the reference's `undefined` fallback does. The `//` comments are not CSS, and are not written; the
//! nested `text` rule of `.state-note` is written flat, as stylis writes it.

use std::fmt::Write as _;

use ferrigura_model::state::ClassDef;
use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::EmbeddedFamily;

use crate::context::{Globals, ThemeView};
use crate::design::{self, Tone};
use crate::svg::{css_text, font_faces};
use crate::unified::palette::{Palette, palette};

/// The color tokens that the rules name, each declared as a custom property on the diagram's id, in the
/// order of the custom properties.
fn color_tokens(tokens: &ThemeTokens) -> [(&'static str, &Token); 24] {
    [
        ("mainBkg", &tokens.main_bkg),
        ("stateBkg", &tokens.state_bkg),
        ("nodeBorder", &tokens.node_border),
        ("stateBorder", &tokens.state_border),
        ("lineColor", &tokens.line_color),
        ("transitionColor", &tokens.transition_color),
        ("textColor", &tokens.text_color),
        ("stateLabelColor", &tokens.state_label_color),
        ("background", &tokens.background),
        ("noteBorderColor", &tokens.note_border_color),
        ("noteBkgColor", &tokens.note_bkg_color),
        ("noteTextColor", &tokens.note_text_color),
        ("labelBackgroundColor", &tokens.label_background_color),
        ("edgeLabelBackground", &tokens.edge_label_background),
        ("transitionLabelColor", &tokens.transition_label_color),
        ("tertiaryTextColor", &tokens.tertiary_text_color),
        ("specialStateColor", &tokens.special_state_color),
        ("innerEndBackground", &tokens.inner_end_background),
        ("compositeBackground", &tokens.composite_background),
        ("altBackground", &tokens.alt_background),
        (
            "compositeTitleBackground",
            &tokens.composite_title_background,
        ),
        ("errorBkgColor", &tokens.error_bkg_color),
        ("errorTextColor", &tokens.error_text_color),
        ("clusterBkg", &tokens.cluster_bkg),
    ]
}

/// The flood of the drop shadows in a light scheme, and in a dark one.
const LIGHT_FLOOD: &str = "#000000";
const DARK_FLOOD: &str = "#FFFFFF";

/// The fill of an alternate composite and of a region when the theme gives no `altBackground`.
const ALT_BACKGROUND: &str = "#efefef";

/// The `<style>` element's content for a state diagram with the id `svg_id`, escaped for XML so that
/// it can be written as the element's text. `families` are the embedded font subsets of the text,
/// whose `@font-face` rules come first, as for the flowchart. In the design mode (`design`), the
/// mode's rules follow the state rules, and the mode's own properties take the place of the
/// palette and the look's flood (`docs/decisions/0023-design-mode.md`).
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    classes: &[ClassDef],
    families: &[EmbeddedFamily],
    design: Option<Tone>,
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
    let stroke_width = tokens.stroke_width.text().unwrap_or_else(|| "1".to_owned());
    let shadow = match tokens.drop_shadow.text() {
        Some(shadow) if !shadow.is_empty() => shadow.replacen(
            "url(#drop-shadow)",
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        ),
        _ => "none".to_owned(),
    };
    let stroke_neo = if tokens.use_gradient.truthy() {
        format!("url(#{svg_id}-gradient)")
    } else {
        colour(&[("nodeBorder", &tokens.node_border)])
    };
    let radius = tokens.radius.text().unwrap_or_else(|| "5".to_owned());

    let mut rules = String::new();
    // styles.ts: the global rules every diagram gets.
    rules.push_str(&rule(
        &id,
        &format!(
            "font-family:{font};font-size:{font_size};fill:{};",
            colour(&[("textColor", &tokens.text_color)])
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
    if tokens.error_bkg_color.text().is_some() {
        rules.push_str(&scoped(
            &id,
            ".error-icon",
            &format!(
                "fill:{};",
                colour(&[("errorBkgColor", &tokens.error_bkg_color)])
            ),
        ));
    }
    if tokens.error_text_color.text().is_some() {
        let text = colour(&[("errorTextColor", &tokens.error_text_color)]);
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
    let line = colour(&[("lineColor", &tokens.line_color)]);
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
        &format!("stroke:{};", colour(&[("nodeBorder", &tokens.node_border)])),
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
        &format!(
            "stroke:{};filter:none;",
            colour(&[("nodeBorder", &tokens.node_border)])
        ),
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

    // styles.js: the palette rules of each container slot, then the state rules. The design mode
    // uses no palette: its groups are neutral.
    if design.is_none() {
        rules.push_str(&palette_rules(&id, globals, tokens));
    }
    rules.push_str(&state_rules(&id, tokens, &stroke_width, &radius, &shadow));
    if design.is_some() {
        rules.push_str(&design_rules(&id));
    }

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
        font_faces(svg_id, families)
    };
    css_text(&format!(
        "{faces}{}{rules}",
        custom_properties(svg_id, theme, globals, design)
    ))
}

/// The rules of the design mode, after the state rules and before the `classDef` rules, which keep
/// the last word (`docs/decisions/0023-design-mode.md`): hairlines, rounder states, an edge label
/// without a background, a neutral composite state with its title in a row of its own, a
/// concurrency region as a dashed frame without a fill, a flat note, and the colors of the roles.
fn design_rules(id: &str) -> String {
    let node_width = design::NODE_STROKE_WIDTH;
    let edge_width = design::EDGE_STROKE_WIDTH;
    let thick_width = design::THICK_EDGE_STROKE_WIDTH;
    let radius = design::NODE_RADIUS;
    let group_radius = design::GROUP_RADIUS;
    let region_radius = design::REGION_RADIUS;
    let mut out = format!(
        "{id} .edge-thickness-normal,{id} .transition{{stroke-width:{edge_width};}}\
{id} .edge-thickness-thick{{stroke-width:{thick_width};}}\
{id} .node rect,{id} .node polygon,{id} [data-look=\"neo\"].node path{{stroke-width:{node_width};}}\
{id} .statediagram-state rect.basic,{id} .statediagram-state rect.title-state{{rx:{radius}px;ry:{radius}px;}}\
{id} .edgeLabel rect,{id} .edgeLabel .label rect{{fill:none;stroke:none;}}\
{id} defs [id$=\"-barbEnd\"],{id} defs [id$=\"-barbEnd-margin\"]{{fill:var(--transitionColor);stroke:none;}}\
{id} [data-look].statediagram-cluster rect.outer{{fill:var(--compositeBackground);stroke:var(--stateBorder);stroke-width:1px;rx:{group_radius}px;ry:{group_radius}px;filter:none;}}\
{id} [data-look].statediagram-cluster rect.inner{{fill:none;stroke:none;}}\
{id} .statediagram-cluster .cluster-label text{{fill:var(--titleColor);}}\
{id} [data-look].statediagram-cluster rect.divider{{fill:none;stroke:var(--stateBorder);stroke-width:1px;stroke-dasharray:4 4;rx:{region_radius}px;ry:{region_radius}px;}}\
{id} .statediagram-note .outer-path,{id} .statediagram-note path{{filter:none;}}\
{id}-drop-shadow feDropShadow,{id}-drop-shadow-small feDropShadow{{flood-color:var(--floodColor);flood-opacity:1;}}"
    );
    for role in design::STATE_ROLES {
        let (fill, stroke, text) = role.properties();
        let node = format!("{id} [data-role=\"{}\"].node", role.name());
        let _ = write!(
            out,
            "{node} rect,{node} circle,{node} polygon,{node} path{{fill:var(--{fill});stroke:var(--{stroke});}}\
{node} .label text{{fill:var(--{text});}}"
        );
    }
    // The end state is a ring around a dot: the ring takes the role's outline around the ground of
    // a state, the dot the role's fill.
    let (fill, stroke, _) = design::Role::Terminal.properties();
    let end = format!("{id} [data-role=\"terminal\"].node .outer-path");
    let _ = write!(
        out,
        "{end} > path[stroke=\"none\"]{{fill:var(--mainBkg);}}\
{end} > path[fill=\"none\"]{{fill:none;stroke:var(--{stroke});}}\
{end} g path{{fill:var(--{fill});stroke:var(--{fill});}}"
    );
    out
}

/// The state rules of `styles.js` `getStyles`, in the reference's order, after the palette rules.
fn state_rules(
    id: &str,
    tokens: &ThemeTokens,
    stroke_width: &str,
    radius: &str,
    shadow: &str,
) -> String {
    let transition = colour(&[("transitionColor", &tokens.transition_color)]);
    let main_bkg = colour(&[("mainBkg", &tokens.main_bkg)]);
    let state_border = colour(&[
        ("stateBorder", &tokens.state_border),
        ("nodeBorder", &tokens.node_border),
    ]);
    let text = colour(&[("textColor", &tokens.text_color)]);
    let state_label = colour(&[("stateLabelColor", &tokens.state_label_color)]);
    let line = colour(&[("lineColor", &tokens.line_color)]);
    let background = colour(&[("background", &tokens.background)]);
    let composite_background = colour(&[
        ("compositeBackground", &tokens.composite_background),
        ("background", &tokens.background),
    ]);
    let alt_background = if tokens.alt_background.text().is_some() {
        colour(&[("altBackground", &tokens.alt_background)])
    } else {
        ALT_BACKGROUND.to_owned()
    };
    let special = colour(&[("specialStateColor", &tokens.special_state_color)]);
    let inner_end = colour(&[("innerEndBackground", &tokens.inner_end_background)]);
    let edge_label = colour(&[("edgeLabelBackground", &tokens.edge_label_background)]);
    let label_background = colour(&[("labelBackgroundColor", &tokens.label_background_color)]);
    let transition_label = colour(&[
        ("transitionLabelColor", &tokens.transition_label_color),
        ("tertiaryTextColor", &tokens.tertiary_text_color),
    ]);
    let note_border = colour(&[("noteBorderColor", &tokens.note_border_color)]);
    let note_bkg = colour(&[("noteBkgColor", &tokens.note_bkg_color)]);
    let note_text = colour(&[("noteTextColor", &tokens.note_text_color)]);
    let composite_title = colour(&[(
        "compositeTitleBackground",
        &tokens.composite_title_background,
    )]);
    let node_border = colour(&[("nodeBorder", &tokens.node_border)]);
    let state_bkg = colour(&[
        ("stateBkg", &tokens.state_bkg),
        ("mainBkg", &tokens.main_bkg),
    ]);
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
    // The nested `text` rule of `.state-note`, which stylis flattens.
    add(
        ".state-note text",
        &format!("fill:{note_text};stroke:none;font-size:10px;"),
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
            if tokens.use_gradient.truthy() {
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
/// outline and the tint of its containers. A theme without a palette, or one that is not a color
/// theme, has none.
fn palette_rules(id: &str, globals: &Globals, tokens: &ThemeTokens) -> String {
    let Some(palette) = palette(globals, tokens) else {
        return String::new();
    };
    let look = safe_look(&globals.look);
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

/// A color of the rules: `var(--name)` for the first of the names the theme gives, else for the
/// first name (the reference's `a || b`).
fn colour(names: &[(&str, &Token)]) -> String {
    let name = names
        .iter()
        .find(|(_, token)| token.text().is_some())
        .map_or(names[0].0, |(name, _)| name);
    format!("var(--{name})")
}

/// One rule under the diagram's id: the selectors of `selectors`, each scoped to `id`.
fn scoped(id: &str, selectors: &str, declarations: &str) -> String {
    let mut scoped = String::new();
    for (index, selector) in selectors.split(',').enumerate() {
        if index > 0 {
            scoped.push(',');
        }
        let _ = write!(scoped, "{id} {}", selector.trim());
    }
    rule(&scoped, declarations)
}

/// One rule: the selector and its declarations, in braces.
fn rule(selector: &str, declarations: &str) -> String {
    format!("{selector}{{{declarations}}}")
}

/// The custom properties of the colors, declared on the diagram's id. With one set of variables they
/// are written as they are; with a light and a dark set, the values that differ are
/// `light-dark()` pairs, as the flowchart writes them. In the design mode, the mode's own
/// properties for the tone of each set take the place of the look's flood.
fn custom_properties(
    svg_id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    design: Option<Tone>,
) -> String {
    let id = format!("#{svg_id}");
    let block = |tokens: &ThemeTokens, flood: &str, tone: Tone| match design {
        Some(_) => {
            let mut declarations = colour_declarations(tokens);
            // The title of a composite state reads the title color, which the state rules do not.
            if let Some(colour) = tokens.title_color.text() {
                let _ = write!(declarations, "--titleColor:{};", css_text(&colour));
            }
            declarations.push_str(design::declarations(tone));
            rule(&id, &declarations)
        }
        None => rule(&id, &declarations(tokens, flood)),
    };
    match theme.dark {
        None => block(theme.light, flood(globals), design.unwrap_or(Tone::Light)),
        Some(dark) => crate::scheme::both(
            &block(theme.light, LIGHT_FLOOD, Tone::Light),
            &block(dark, DARK_FLOOD, Tone::Dark),
        ),
    }
}

/// The flood of the drop shadows for a configuration: white for a theme whose name says dark.
fn flood(globals: &Globals) -> &'static str {
    if globals.theme.contains("dark") {
        DARK_FLOOD
    } else {
        LIGHT_FLOOD
    }
}

/// The declarations of the custom properties of one set of variables.
fn declarations(tokens: &ThemeTokens, flood: &str) -> String {
    let mut out = colour_declarations(tokens);
    out.push_str(&format!("--floodColor:{flood};"));
    out
}

/// The declarations of the color tokens of one set of variables.
fn colour_declarations(tokens: &ThemeTokens) -> String {
    let mut out = String::new();
    for (name, token) in color_tokens(tokens) {
        if let Some(colour) = token.text() {
            out.push_str(&format!("--{name}:{};", css_text(&colour)));
        }
    }
    out
}
