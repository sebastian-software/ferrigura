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

use std::borrow::Cow;
use std::fmt::{self, Write as _};

use ferrigura_model::state::ClassDef;
use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::EmbeddedFamily;

use crate::context::{Globals, ThemeView};
use crate::design::{self, Tone};
use crate::svg::assemble_stylesheet;
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

/// Room for the rules of a state style sheet besides its font faces: about 14 KiB with a palette.
const RULES_CAPACITY: usize = 16 * 1024;

/// The `<style>` element's content for a state diagram with the id `svg_id`, escaped for XML so
/// that it can be written as the element's text. `families` are the embedded font subsets of the
/// text, whose `@font-face` rules come first, as for the flowchart. In the default rendering
/// (`design`), the mode's rules follow the state rules, and the default's own properties take the
/// place of the palette and the look's flood (`docs/decisions/0023-the-default-rendering.md`).
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
    let stroke_width = tokens.stroke_width.text().unwrap_or_else(|| "1".to_owned());
    let shadow: Cow<'_, str> = match tokens.drop_shadow.text() {
        Some(shadow) if !shadow.is_empty() => Cow::Owned(shadow.replacen(
            "url(#drop-shadow)",
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        )),
        _ => Cow::Borrowed("none"),
    };
    let stroke_neo: Cow<'_, str> = if tokens.use_gradient.truthy() {
        Cow::Owned(format!("url(#{svg_id}-gradient)"))
    } else {
        Cow::Owned(colour(&[("nodeBorder", &tokens.node_border)]))
    };
    let radius = tokens.radius.text().unwrap_or_else(|| "5".to_owned());

    let mut css = String::with_capacity(RULES_CAPACITY);
    write_custom_properties(&mut css, &id, theme, globals, design);

    // styles.ts: the global rules every diagram gets.
    let _ = write!(
        css,
        "{id}{{font-family:{font};font-size:{font_size};fill:{};}}\
@keyframes edge-animation-frame{{from{{stroke-dashoffset:0;}}}}\
@keyframes dash{{to{{stroke-dashoffset:0;}}}}\
{id} .edge-animation-slow{{stroke-dasharray:9,5 !important;stroke-dashoffset:900;animation:dash 50s linear infinite;stroke-linecap:round;}}\
{id} .edge-animation-fast{{stroke-dasharray:9,5 !important;stroke-dashoffset:900;animation:dash 20s linear infinite;stroke-linecap:round;}}",
        colour(&[("textColor", &tokens.text_color)]),
    );
    if tokens.error_bkg_color.text().is_some() {
        let _ = write!(
            css,
            "{id} .error-icon{{fill:{};}}",
            colour(&[("errorBkgColor", &tokens.error_bkg_color)]),
        );
    }
    if tokens.error_text_color.text().is_some() {
        let text = colour(&[("errorTextColor", &tokens.error_text_color)]);
        let _ = write!(css, "{id} .error-text{{fill:{text};stroke:{text};}}");
    }
    let line = colour(&[("lineColor", &tokens.line_color)]);
    let node_border = colour(&[("nodeBorder", &tokens.node_border)]);
    let _ = write!(
        css,
        "{id} .edge-thickness-normal{{stroke-width:{stroke_width}px;}}\
{id} .edge-thickness-thick{{stroke-width:3.5px;}}\
{id} .edge-pattern-solid{{stroke-dasharray:0;}}\
{id} .edge-thickness-invisible{{stroke-width:0;fill:none;}}\
{id} .edge-pattern-dashed{{stroke-dasharray:3;}}\
{id} .edge-pattern-dotted{{stroke-dasharray:2;}}\
{id} .marker{{fill:{line};stroke:{line};}}\
{id} .marker.cross{{stroke:{line};}}\
{id} svg{{font-family:{font};font-size:{font_size};}}\
{id} p{{margin:0;}}\
{id} .node .neo-node{{stroke:{node_border};}}\
{id} [data-look=\"neo\"].node rect,{id} [data-look=\"neo\"].cluster rect,{id} [data-look=\"neo\"].node polygon{{stroke:{stroke_neo};filter:{shadow};}}\
{id} [data-look=\"neo\"].swimlane.cluster rect{{filter:none;}}\
{id} [data-look=\"neo\"].node path{{stroke:{stroke_neo};stroke-width:{stroke_width}px;}}\
{id} [data-look=\"neo\"].node .outer-path{{filter:{shadow};}}\
{id} [data-look=\"neo\"].node .neo-line path{{stroke:{node_border};filter:none;}}\
{id} [data-look=\"neo\"].node circle{{stroke:{stroke_neo};filter:{shadow};}}\
{id} [data-look=\"neo\"].node circle .state-start{{fill:#000000;}}\
{id} [data-look=\"neo\"].icon-shape .icon{{fill:{stroke_neo};filter:{shadow};}}\
{id} [data-look=\"neo\"].icon-shape .icon-neo path{{stroke:{stroke_neo};filter:{shadow};}}"
    );

    // styles.js: the palette rules of each container slot, then the state rules. The default
    // rendering uses no palette: its groups are neutral.
    if design.is_none() {
        write_palette_rules(&mut css, &id, globals, tokens);
    }
    write_state_rules(&mut css, &id, tokens, &stroke_width, &radius, &shadow);
    if design.is_some() {
        write_design_rules(&mut css, &id);
    }

    // The diagram's `classDef` rules: the classes' declarations, on every state of the class. A
    // class whose id holds braces or a comma, or whose styles hold braces, is left out so it
    // cannot break out of the diagram's id scope.
    for class in classes {
        if class.id.contains(['{', '}', ','])
            || class.styles.is_empty()
            || class.styles.iter().any(|style| style.contains(['{', '}']))
        {
            continue;
        }
        let _ = write!(css, "{id} .{} > *{{", class.id);
        for style in &class.styles {
            let _ = write!(css, "{style} !important;");
        }
        css.push('}');
    }

    assemble_stylesheet(svg_id, families, css, false)
}

/// The rules of the default rendering, after the state rules and before the `classDef` rules, which
/// keep the last word (`docs/decisions/0023-the-default-rendering.md`): hairlines, rounder states,
/// an edge label without a background, a neutral composite state with its title in a row of its
/// own, a concurrency region as a dashed frame without a fill, a flat note, and the colors of the
/// roles.
fn write_design_rules(out: &mut String, id: &str) {
    let node_width = design::NODE_STROKE_WIDTH;
    let edge_width = design::EDGE_STROKE_WIDTH;
    let thick_width = design::THICK_EDGE_STROKE_WIDTH;
    let radius = design::NODE_RADIUS;
    let group_radius = design::GROUP_RADIUS;
    let region_radius = design::REGION_RADIUS;
    let _ = write!(
        out,
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
}

/// The state rules of `styles.js` `getStyles`, in the reference's order, after the palette rules.
fn write_state_rules(
    out: &mut String,
    id: &str,
    tokens: &ThemeTokens,
    stroke_width: &str,
    radius: &str,
    shadow: &str,
) {
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
    let neo_cluster_stroke: Cow<'_, str> = if tokens.use_gradient.truthy() {
        Cow::Owned(format!("url(#{}-gradient)", id.trim_start_matches('#')))
    } else {
        Cow::Borrowed(state_border.as_str())
    };

    write_scoped(
        out,
        id,
        "defs [id$=\"-barbEnd\"]",
        format_args!("fill:{transition};stroke:{transition};"),
    );
    write_scoped(
        out,
        id,
        "g.stateGroup text",
        format_args!("fill:{node_border};stroke:none;font-size:10px;"),
    );
    write_scoped(
        out,
        id,
        "g.stateGroup text",
        format_args!("fill:{text};stroke:none;font-size:10px;"),
    );
    write_scoped(
        out,
        id,
        "g.stateGroup .state-title",
        format_args!("font-weight:bolder;fill:{state_label};"),
    );
    write_scoped(
        out,
        id,
        "g.stateGroup rect",
        format_args!("fill:{main_bkg};stroke:{node_border};"),
    );
    write_scoped(
        out,
        id,
        "g.stateGroup line",
        format_args!("stroke:{line};stroke-width:{stroke_width};"),
    );
    write_scoped(
        out,
        id,
        ".transition",
        format_args!("stroke:{transition};stroke-width:{stroke_width};fill:none;"),
    );
    write_scoped(
        out,
        id,
        ".stateGroup .composit",
        format_args!("fill:{background};border-bottom:1px;"),
    );
    write_scoped(
        out,
        id,
        ".stateGroup .alt-composit",
        format_args!("fill:#e0e0e0;border-bottom:1px;"),
    );
    write_scoped(
        out,
        id,
        ".state-note",
        format_args!("stroke:{note_border};fill:{note_bkg};"),
    );
    // The nested `text` rule of `.state-note`, which stylis flattens.
    write_scoped(
        out,
        id,
        ".state-note text",
        format_args!("fill:{note_text};stroke:none;font-size:10px;"),
    );
    write_scoped(
        out,
        id,
        ".stateLabel .box",
        format_args!("stroke:none;stroke-width:0;fill:{main_bkg};opacity:0.5;"),
    );
    write_scoped(
        out,
        id,
        ".edgeLabel .label rect",
        format_args!("fill:{label_background};opacity:0.5;"),
    );
    write_scoped(
        out,
        id,
        ".edgeLabel",
        format_args!("background-color:{edge_label};text-align:center;"),
    );
    write_scoped(
        out,
        id,
        ".edgeLabel p",
        format_args!("background-color:{edge_label};"),
    );
    write_scoped(
        out,
        id,
        ".edgeLabel rect",
        format_args!("opacity:0.5;background-color:{edge_label};fill:{edge_label};"),
    );
    write_scoped(
        out,
        id,
        ".edgeLabel .label text",
        format_args!("fill:{transition_label};"),
    );
    write_scoped(
        out,
        id,
        ".label div .edgeLabel",
        format_args!("color:{transition_label};"),
    );
    write_scoped(
        out,
        id,
        ".stateLabel text",
        format_args!("fill:{state_label};font-size:10px;font-weight:bold;"),
    );
    write_scoped(
        out,
        id,
        ".node circle.state-start",
        format_args!("fill:{special};stroke:{special};"),
    );
    write_scoped(
        out,
        id,
        ".node .fork-join",
        format_args!("fill:{special};stroke:{special};"),
    );
    write_scoped(
        out,
        id,
        ".node circle.state-end",
        format_args!("fill:{inner_end};stroke:{background};stroke-width:1.5;"),
    );
    write_scoped(
        out,
        id,
        ".end-state-inner",
        format_args!("fill:{composite_background};stroke-width:1.5;"),
    );
    write_scoped(
        out,
        id,
        ".node rect",
        format_args!("fill:{state_bkg};stroke:{state_border};stroke-width:{stroke_width}px;"),
    );
    write_scoped(
        out,
        id,
        ".node polygon",
        format_args!("fill:{main_bkg};stroke:{state_border};stroke-width:{stroke_width}px;"),
    );
    write_scoped(out, id, "[id$=\"-barbEnd\"]", format_args!("fill:{line};"));
    write_scoped(
        out,
        id,
        ".statediagram-cluster rect",
        format_args!("fill:{composite_title};stroke:{state_border};stroke-width:{stroke_width}px;"),
    );
    write_scoped(
        out,
        id,
        ".cluster-label, .nodeLabel",
        format_args!("color:{state_label};"),
    );
    write_scoped(
        out,
        id,
        ".statediagram-cluster rect.outer",
        format_args!("rx:5px;ry:5px;"),
    );
    write_scoped(
        out,
        id,
        ".statediagram-state .divider",
        format_args!("stroke:{state_border};"),
    );
    write_scoped(
        out,
        id,
        ".statediagram-state .title-state",
        format_args!("rx:5px;ry:5px;"),
    );
    write_scoped(
        out,
        id,
        ".statediagram-cluster.statediagram-cluster .inner",
        format_args!("fill:{composite_background};"),
    );
    write_scoped(
        out,
        id,
        ".statediagram-cluster.statediagram-cluster-alt .inner",
        format_args!("fill:{alt_background};"),
    );
    write_scoped(
        out,
        id,
        ".statediagram-cluster .inner",
        format_args!("rx:0;ry:0;"),
    );
    write_scoped(
        out,
        id,
        ".statediagram-state rect.basic",
        format_args!("rx:5px;ry:5px;"),
    );
    write_scoped(
        out,
        id,
        ".statediagram-state rect.divider",
        format_args!("stroke-dasharray:10,10;fill:{alt_background};"),
    );
    write_scoped(out, id, ".note-edge", format_args!("stroke-dasharray:5;"));
    write_scoped(
        out,
        id,
        ".statediagram-note rect",
        format_args!("fill:{note_bkg};stroke:{note_border};stroke-width:1px;rx:0;ry:0;"),
    );
    write_scoped(
        out,
        id,
        ".statediagram-note text",
        format_args!("fill:{note_text};"),
    );
    write_scoped(
        out,
        id,
        ".statediagram-note .nodeLabel",
        format_args!("color:{note_text};"),
    );
    write_scoped(
        out,
        id,
        ".statediagram .edgeLabel",
        format_args!("color:red;"),
    );
    write_scoped(
        out,
        id,
        "[id$=\"-dependencyStart\"], [id$=\"-dependencyEnd\"]",
        format_args!("fill:{line};stroke:{line};stroke-width:{stroke_width};"),
    );
    write_scoped(
        out,
        id,
        ".statediagramTitleText",
        format_args!("text-anchor:middle;font-size:18px;fill:{text};"),
    );
    write_scoped(
        out,
        id,
        "[data-look=\"neo\"].statediagram-cluster rect",
        format_args!("fill:{main_bkg};stroke:{neo_cluster_stroke};stroke-width:{stroke_width};"),
    );
    write_scoped(
        out,
        id,
        "[data-look=\"neo\"].statediagram-cluster rect.outer",
        format_args!("rx:{radius}px;ry:{radius}px;filter:{shadow};"),
    );
}

/// The palette rules of `genColor` (`styles.js` 21-99): for each slot of the border palette, the
/// outline and the tint of its containers. A theme without a palette, or one that is not a color
/// theme, has none.
fn write_palette_rules(out: &mut String, id: &str, globals: &Globals, tokens: &ThemeTokens) {
    let Some(palette) = palette(globals, tokens) else {
        return;
    };
    let look = safe_look(&globals.look);
    let mut slot = String::new();
    for index in 0..palette.border.len() {
        slot.clear();
        let _ = write!(
            slot,
            "{id} [data-look=\"{look}\"][data-color-id=\"color-{index}\"]"
        );
        let border = &palette.border[index];
        let tint = tint(&palette, index);
        let _ = write!(
            out,
            "{slot}.statediagram-cluster rect.outer{{stroke:{border};{tint}}}\
{slot}.statediagram-cluster rect.inner{{stroke:{border};}}\
{slot}.statediagram-cluster rect.divider{{stroke:{border};{tint}}}\
{slot}.statediagram-cluster .outer path[stroke='none']{{{tint}}}\
{slot}.statediagram-cluster .outer path[fill='none']{{stroke:{border};}}\
{slot}.statediagram-cluster .divider path[stroke='none']{{{tint}}}\
{slot}.statediagram-cluster .divider path[fill='none']{{stroke:{border};}}"
        );
    }
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

/// Appends one rule under the diagram's id: the selectors of `selectors`, each scoped to `id`.
fn write_scoped(out: &mut String, id: &str, selectors: &str, declarations: fmt::Arguments<'_>) {
    for (index, selector) in selectors.split(',').enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "{id} {}", selector.trim());
    }
    let _ = write!(out, "{{{declarations}}}");
}

/// Appends the custom properties of the colors, declared on the diagram's id. With one set of
/// variables they are written as they are; with a light and a dark set, the values that differ are
/// `light-dark()` pairs, as the flowchart writes them. In the default rendering, the default's own
/// properties for the tone of each set take the place of the look's flood.
fn write_custom_properties(
    css: &mut String,
    id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    design: Option<Tone>,
) {
    let write_block = |out: &mut String, tokens: &ThemeTokens, flood: &str, tone: Tone| {
        out.push_str(id);
        out.push('{');
        write_color_declarations(out, tokens);
        match design {
            Some(_) => {
                // The title of a composite state reads the title color, which the state rules do not.
                if let Some(color) = tokens.title_color.text() {
                    let _ = write!(out, "--titleColor:{color};");
                }
                out.push_str(design::declarations(tone));
            }
            None => {
                let _ = write!(out, "--floodColor:{flood};");
            }
        }
        out.push('}');
    };
    match theme.dark {
        None => write_block(
            css,
            theme.light,
            flood(globals),
            design.unwrap_or(Tone::Light),
        ),
        Some(dark) => {
            let mut light_block = String::new();
            write_block(&mut light_block, theme.light, LIGHT_FLOOD, Tone::Light);
            let mut dark_block = String::new();
            write_block(&mut dark_block, dark, DARK_FLOOD, Tone::Dark);
            css.push_str(&crate::scheme::both(&light_block, &dark_block));
        }
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

/// Appends the declarations of the color tokens of one set of variables.
fn write_color_declarations(out: &mut String, tokens: &ThemeTokens) {
    for (name, token) in color_tokens(tokens) {
        if let Some(color) = token.text() {
            let _ = write!(out, "--{name}:{color};");
        }
    }
}

#[cfg(test)]
mod tests {
    use ferrigura_model::config::MermaidConfig;
    use ferrigura_model::theme::Theme;
    use ferrigura_model::value::Value;

    use super::*;

    #[test]
    fn class_def_with_braces_or_comma_is_left_out_and_ampersand_is_escaped_once() {
        let mut vars = Theme::ReduxColor.variables(None);
        if let Some(obj) = vars.as_object_mut() {
            obj.insert("stateBkg".to_owned(), Value::from("a&b"));
        }
        let tokens = ThemeTokens::from_value(&vars);
        let classes = [
            ClassDef {
                id: "evilStyle".to_owned(),
                styles: vec!["fill:red} body{display:none} .x{color:red".to_owned()],
                text_styles: Vec::new(),
            },
            ClassDef {
                id: "evil,body".to_owned(),
                styles: vec!["fill:red".to_owned()],
                text_styles: Vec::new(),
            },
            ClassDef {
                id: "evil{x}".to_owned(),
                styles: vec!["fill:red".to_owned()],
                text_styles: Vec::new(),
            },
            ClassDef {
                id: "good".to_owned(),
                styles: vec!["fill:green".to_owned()],
                text_styles: Vec::new(),
            },
        ];
        let sheet = style_sheet(
            "state-test",
            ThemeView {
                light: &tokens,
                dark: None,
            },
            &Globals::of(&MermaidConfig::default()),
            &classes,
            &[],
            None,
        );
        assert!(sheet.contains("--stateBkg:a&amp;b;"), "{sheet}");
        assert!(!sheet.contains("&amp;amp;"), "{sheet}");
        assert!(!sheet.contains("body"), "{sheet}");
        assert!(!sheet.contains("evil"), "{sheet}");
        assert!(
            sheet.contains("#state-test .good &gt; *{fill:green !important;}"),
            "{sheet}"
        );
    }
}
