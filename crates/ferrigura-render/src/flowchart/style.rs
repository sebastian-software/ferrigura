//! The flowchart's style sheet and look definitions: a port of
//! `packages/mermaid/src/diagrams/flowchart/styles.ts`, the rules of `packages/mermaid/src/styles.ts`
//! that a flowchart gets, the `classDef` rules of `mermaidAPI.ts` and `insertLookDefs.ts`.
//!
//! Colors follow `docs/decisions/0011-themes-as-css-variables.md`: every color is a custom property
//! declared on the diagram's id, once for each set of variables, and the rules refer to it with
//! `var(…)`. The elements carry the same colors as presentation attributes, where they have one
//! (see `clusters.rs`). The rules that the reference writes but no flowchart element can match are
//! left out, each for a stated reason;
//! `docs/diagrams/flowchart.md#style-sheet-looks-and-the-svg-root` says why, and
//! `tests/flowchart_styles.rs` checks every other rule in `compat` against the reference's compiled
//! CSS.

use std::borrow::Cow;
use std::fmt::{self, Write as _};

use ferrigura_model::flowchart::FlowClass;
use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::EmbeddedFamily;

use crate::context::{Globals, ThemeView};
use crate::design::{self, Tone};
use crate::svg::{assemble_stylesheet, escape};
use crate::unified::palette::{Palette, palette};

/// The color tokens of the rules, each declared as a custom property on the diagram's id, in the order
/// of the custom properties.
fn color_tokens(tokens: &ThemeTokens) -> [(&'static str, &Token); 9] {
    [
        ("nodeBorder", &tokens.node_border),
        ("mainBkg", &tokens.main_bkg),
        ("clusterBkg", &tokens.cluster_bkg),
        ("clusterBorder", &tokens.cluster_border),
        ("lineColor", &tokens.line_color),
        ("textColor", &tokens.text_color),
        ("nodeTextColor", &tokens.node_text_color),
        ("titleColor", &tokens.title_color),
        ("edgeLabelBackground", &tokens.edge_label_background),
    ]
}

/// The flood of the drop shadows in a light scheme (`insertLookDefs.ts:30`).
const LIGHT_FLOOD: &str = "#000000";

/// The flood of the drop shadows in a dark scheme (`insertLookDefs.ts:30`).
const DARK_FLOOD: &str = "#FFFFFF";

/// Room for the rules of a style sheet besides its font faces: those of a palette theme under a
/// generated id take about 11 KiB.
const RULES_CAPACITY: usize = 12 * 1024;

/// The `<style>` element's content for a flowchart with the id `svg_id`, escaped for XML so that it can
/// be written as the element's text. The look is read from the globals, the palette and the shadow from
/// the theme, and the `classDef` rules come from `classes`.
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    classes: &[FlowClass],
) -> String {
    style_sheet_with_fonts(svg_id, theme, globals, classes, &[], None)
}

/// [`style_sheet`] for a diagram whose text is set in embedded font subsets. The rules name the first
/// family, and its `@font-face` rules come first. With no families, the rules use the theme's
/// `fontFamily`, as the reference does.
#[must_use]
pub fn style_sheet_with_fonts(
    svg_id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    classes: &[FlowClass],
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
    let theme_width = format!(
        "{}px",
        tokens.stroke_width.text().unwrap_or_else(|| "1".to_owned())
    );
    // The default rendering draws hairlines: an outline lighter than an edge, both lighter than the
    // theme's.
    let (stroke_width, edge_width, thick_width) = match design {
        Some(_) => (
            design::NODE_STROKE_WIDTH,
            design::EDGE_STROKE_WIDTH,
            design::THICK_EDGE_STROKE_WIDTH,
        ),
        None => (theme_width.as_str(), theme_width.as_str(), "3.5px"),
    };
    let text_colour = if tokens.node_text_color.is_absent() {
        "textColor"
    } else {
        "nodeTextColor"
    };
    let palette = design.map_or_else(|| palette(globals, tokens), |_| None);
    let look = safe_look(&globals.look);
    let neo_stroke: Cow<'_, str> = if tokens.use_gradient.truthy() {
        Cow::Owned(format!("url(#{svg_id}-gradient)"))
    } else {
        Cow::Borrowed("var(--nodeBorder)")
    };
    let shadow: Cow<'_, str> = match tokens.drop_shadow.text() {
        Some(shadow) if !shadow.is_empty() => Cow::Owned(shadow.replacen(
            "url(#drop-shadow)",
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        )),
        _ => Cow::Borrowed("none"),
    };
    // Match Mermaid's normally opaque HTML paragraph backgrounds in the SVG-only default.
    // SVG labels mask the line in both renderings.
    // Keep explicitly authored theme colors, including their alpha channel.
    let label_opacity = "1";

    let mut css = String::with_capacity(RULES_CAPACITY);
    write_custom_properties(&mut css, &id, theme, globals, design, palette.as_ref());
    let _ = write!(
        css,
        "{id}{{font-family:{font};font-size:{font_size};fill:var(--textColor);}}\
@keyframes dash{{to{{stroke-dashoffset:0;}}}}\
{id} .edge-animation-slow{{stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 50s linear infinite;stroke-linecap:round;}}\
{id} .edge-animation-fast{{stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 20s linear infinite;stroke-linecap:round;}}\
{id} .edge-thickness-normal{{stroke-width:{edge_width};}}\
{id} .edge-thickness-thick{{stroke-width:{thick_width};}}\
{id} .edge-pattern-solid{{stroke-dasharray:0;}}\
{id} .edge-thickness-invisible{{stroke-width:0;fill:none;}}\
{id} .edge-pattern-dashed{{stroke-dasharray:3;}}\
{id} .edge-pattern-dotted{{stroke-dasharray:2;}}\
{id} .marker{{fill:var(--lineColor);stroke:var(--lineColor);}}\
{id} .marker.cross{{stroke:var(--lineColor);}}"
    );
    if let Some(palette) = &palette {
        write_palette_rules(&mut css, &id, look, palette);
    }
    let _ = write!(
        css,
        "{id} .label{{font-family:{font};color:var(--{text_colour});}}\
{id} .cluster-label text{{fill:var(--titleColor);}}\
{id} .label text{{fill:var(--{text_colour});color:var(--{text_colour});}}\
{id} .node rect,{id} .node circle,{id} .node ellipse,{id} .node polygon,{id} .node path{{fill:var(--mainBkg);stroke:var(--nodeBorder);stroke-width:{stroke_width};}}\
{id} .node .label text,{id} .image-shape .label,{id} .icon-shape .label{{text-anchor:middle;}}\
{id} .node .label,{id} .image-shape .label,{id} .icon-shape .label{{text-align:center;}}\
{id} .node.clickable{{cursor:pointer;}}\
{id} .flowchart-link{{stroke:var(--lineColor);fill:none;}}\
{id} .edgeLabel{{background-color:var(--edgeLabelBackground);text-align:center;}}\
{id} .edgeLabel rect{{opacity:{label_opacity};background-color:var(--edgeLabelBackground);fill:var(--edgeLabelBackground);}}\
{id} .cluster rect{{fill:var(--clusterBkg);stroke:var(--clusterBorder);stroke-width:1px;}}\
{id} .cluster text{{fill:var(--titleColor);}}\
{id} .node .collapsed-indicator{{fill:var(--clusterBorder);stroke:none;opacity:0.6;}}\
{id} .node .collapsed-separator{{stroke:var(--clusterBorder);stroke-width:0.75px;}}\
{id} .flowchartTitleText{{text-anchor:middle;font-size:18px;fill:var(--textColor);}}\
{id} rect.text{{fill:none;stroke-width:0;}}\
{id} .icon-shape,{id} .image-shape{{background-color:var(--edgeLabelBackground);text-align:center;}}\
{id} .icon-shape .label rect,{id} .image-shape .label rect{{opacity:0.5;background-color:var(--edgeLabelBackground);fill:var(--edgeLabelBackground);}}"
    );
    // The look's rules. They match only the elements of their look, so both looks carry them.
    let _ = write!(
        css,
        "{id} [data-look=\"neo\"].node rect,{id} [data-look=\"neo\"].cluster rect,{id} [data-look=\"neo\"].node polygon{{stroke:{neo_stroke};filter:{shadow};}}\
{id} [data-look=\"neo\"].node path{{stroke:{neo_stroke};stroke-width:{stroke_width};}}\
{id} [data-look=\"neo\"].node .outer-path{{filter:{shadow};}}\
{id} [data-look=\"neo\"].node circle{{stroke:{neo_stroke};filter:{shadow};}}\
{id}-drop-shadow feDropShadow,{id}-drop-shadow-small feDropShadow{{flood-color:var(--floodColor);}}"
    );
    if design.is_some() {
        write_design_rules(&mut css, &id);
    }
    write_class_rules(&mut css, &id, classes);
    let _ = write!(css, "{id} text{{white-space:pre;}}");
    assemble_stylesheet(svg_id, families, css, false)
}

/// The `<defs>` that the look's rules refer to, as `insertLookDefs.ts` appends them: the two drop
/// shadows, always, and the gradient only when the theme uses one. The ids are scoped to `svg_id`.
/// The shadows' flood is black, or white for a dark theme; the style sheet sets it per scheme.
#[must_use]
pub fn look_defs(svg_id: &str, theme: ThemeView<'_>, globals: &Globals) -> String {
    look_defs_for(svg_id, theme, globals, None)
}

/// [`look_defs`] for a flowchart, which draws the default rendering's shadows in that mode.
#[must_use]
pub fn look_defs_for(
    svg_id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    design: Option<Tone>,
) -> String {
    let tokens = theme.light;
    if let Some(tone) = design {
        return design_defs(svg_id, tone);
    }
    let flood = match theme.dark {
        None => flood_color(globals),
        Some(_) => LIGHT_FLOOD,
    };
    let shadow = |id: &str, size: &str, offset: &str| {
        format!(
            "<defs><filter id=\"{}\" height=\"{size}\" width=\"{size}\"><feDropShadow dx=\"{offset}\" dy=\"{offset}\" stdDeviation=\"0\" flood-opacity=\"0.06\" flood-color=\"{flood}\"/></filter></defs>",
            escape(id)
        )
    };
    let mut defs = shadow(&format!("{svg_id}-drop-shadow"), "130%", "4");
    defs.push_str(&shadow(&format!("{svg_id}-drop-shadow-small"), "150%", "2"));
    if tokens.use_gradient.truthy() {
        let start = tokens.gradient_start.text().unwrap_or_default();
        let stop = tokens.gradient_stop.text().unwrap_or_default();
        defs.push_str(&format!(
            "<linearGradient id=\"{}-gradient\" gradientUnits=\"objectBoundingBox\" x1=\"0%\" y1=\"0%\" x2=\"100%\" y2=\"0%\"><stop offset=\"0%\" stop-color=\"{}\" stop-opacity=\"1\"/><stop offset=\"100%\" stop-color=\"{}\" stop-opacity=\"1\"/></linearGradient>",
            escape(svg_id),
            escape(&start),
            escape(&stop),
        ));
    }
    defs
}

/// The shadows of the default rendering: short and soft, under the same ids as the look's, so that
/// the look's rules refer to them. The style sheet sets the flood per scheme.
fn design_defs(svg_id: &str, tone: Tone) -> String {
    let (flood, opacity) = design::flood_attributes(tone);
    let shadow = |id: &str, size: &str, blur: &str| {
        format!(
            "<defs><filter id=\"{}\" height=\"{size}\" width=\"{size}\"><feDropShadow dx=\"0\" dy=\"1\" stdDeviation=\"{blur}\" flood-opacity=\"{opacity}\" flood-color=\"{flood}\"/></filter></defs>",
            escape(id)
        )
    };
    let mut defs = shadow(&format!("{svg_id}-drop-shadow"), "130%", "1.5");
    defs.push_str(&shadow(&format!("{svg_id}-drop-shadow-small"), "150%", "1"));
    defs
}

/// The rules of the default rendering, after the look's and before the `classDef` rules, which keep
/// the last word: a group frame without the node's outline and shadow, an edge label in the title's
/// muted color, and the fill, outline and text of each role.
fn write_design_rules(css: &mut String, id: &str) {
    let _ = write!(
        css,
        "{id} [data-look=\"neo\"].cluster rect{{stroke:var(--clusterBorder);filter:none;}}\
{id} .edgeLabel .label text{{fill:var(--titleColor);}}"
    );
    // The edge is masked under its label, so the label has no background of its own.
    let _ = write!(
        css,
        "{id} .edgeLabel rect.background{{fill:none;stroke:none;}}\
{id}-drop-shadow feDropShadow,{id}-drop-shadow-small feDropShadow{{flood-opacity:1;}}"
    );
    for role in design::ROLES {
        let (fill, stroke, text) = role.properties();
        let node = format!("{id} [data-role=\"{}\"].node", role.name());
        let _ = write!(
            css,
            "{node} rect,{node} circle,{node} ellipse,{node} polygon,{node} path{{fill:var(--{fill});stroke:var(--{stroke});}}\
{node} .label text{{fill:var(--{text});color:var(--{text});}}"
        );
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

/// The flood of the drop shadows for a configuration: white for a theme whose name says dark.
fn flood_color(globals: &Globals) -> &'static str {
    if globals.theme.contains("dark") {
        DARK_FLOOD
    } else {
        LIGHT_FLOOD
    }
}

/// Appends the custom properties of the colors to `css`, declared on `id` (`#` and the SVG's id),
/// with the palette of the light (or only) scheme. With one set of variables they are written as
/// they are; with a light and a dark set, the values that differ are `light-dark()` pairs, so one
/// SVG follows the page's or the viewer's scheme. In the default rendering, the default's own
/// properties take the place of the palette and the look's flood.
fn write_custom_properties(
    css: &mut String,
    id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    design: Option<Tone>,
    palette: Option<&Palette>,
) {
    let light = theme.light;
    // With both schemes, the merge reads the two blocks whole, so each is written on its own first.
    if let Some(tone) = design {
        match theme.dark {
            None => write_design_block(css, id, light, tone),
            Some(dark) => {
                let mut light_block = String::new();
                write_design_block(&mut light_block, id, light, Tone::Light);
                let mut dark_block = String::new();
                write_design_block(&mut dark_block, id, dark, Tone::Dark);
                css.push_str(&crate::scheme::both(&light_block, &dark_block));
            }
        }
        return;
    }
    let slots = palette.map(slot_counts);
    match theme.dark {
        None => write_declaration_block(css, id, light, flood_color(globals), slots),
        Some(dark) => {
            let mut light_block = String::new();
            write_declaration_block(&mut light_block, id, light, LIGHT_FLOOD, slots);
            let mut dark_block = String::new();
            write_declaration_block(&mut dark_block, id, dark, DARK_FLOOD, slots);
            css.push_str(&crate::scheme::both(&light_block, &dark_block));
        }
    }
}

/// The number of border slots and of fill slots of a palette.
fn slot_counts(palette: &Palette) -> (usize, usize) {
    (palette.border.len(), palette.background.len())
}

/// The custom properties of one set, as one rule on `id`. A set without fills gives every fill slot
/// its cluster background, so that a palette of the other scheme fills a cluster as this scheme's own
/// rules would.
fn write_declaration_block(
    css: &mut String,
    id: &str,
    tokens: &ThemeTokens,
    flood: &str,
    slots: Option<(usize, usize)>,
) {
    css.push_str(id);
    css.push('{');
    write_color_declarations(css, tokens);
    if let Some((borders, fills)) = slots {
        let border_list = tokens.border_color_array.texts();
        let fill_list = tokens.bkg_color_array.texts();
        for index in 0..borders {
            if let Some(colour) = cyclic(&border_list, index) {
                write_property(css, format_args!("--borderColor{index}"), colour);
            }
        }
        for index in 0..fills {
            let colour = cyclic(&fill_list, index)
                .map(str::to_owned)
                .or_else(|| tokens.cluster_bkg.text());
            if let Some(colour) = colour {
                write_property(css, format_args!("--bkgColor{index}"), &colour);
            }
        }
    }
    let _ = write!(css, "--floodColor:{flood};}}");
}

/// The custom properties of one set in the default rendering, as one rule on `id`: the theme's
/// colors and the default's own properties for `tone`.
fn write_design_block(css: &mut String, id: &str, tokens: &ThemeTokens, tone: Tone) {
    css.push_str(id);
    css.push('{');
    write_color_declarations(css, tokens);
    css.push_str(design::declarations(tone));
    css.push('}');
}

/// The declarations of the color tokens of one set.
fn write_color_declarations(css: &mut String, tokens: &ThemeTokens) {
    for (name, token) in color_tokens(tokens) {
        if let Some(colour) = token.text() {
            write_property(css, format_args!("--{name}"), &colour);
        }
    }
}

/// One custom property whose value is a theme's text. The whole sheet is escaped for XML once when
/// it is complete.
fn write_property(css: &mut String, name: fmt::Arguments<'_>, value: &str) {
    let _ = write!(css, "{name}:{value};");
}

/// The entry of a list at an index, wrapped around; `None` for an empty list.
fn cyclic(list: &[String], index: usize) -> Option<&str> {
    if list.is_empty() {
        None
    } else {
        list.get(index % list.len()).map(String::as_str)
    }
}

/// The palette rules of a look (`genColor` in `flowchart/styles.ts`): for each slot, the cluster frame
/// takes the slot's border and fill, and so do the collapsed group and its indicator and separator.
/// Lanes (`:not(.swimlane)`) and the `path` forms of clusters are left out, since a flowchart
/// cluster is a `rect`.
fn write_palette_rules(css: &mut String, id: &str, look: &str, palette: &Palette) {
    let fills = palette.background.len();
    let mut slot = String::new();
    for index in 0..palette.border.len() {
        slot.clear();
        let _ = write!(
            slot,
            "{id} [data-look=\"{look}\"][data-color-id=\"color-{index}\"]"
        );
        for frame in [".cluster:not(.swimlane) rect", ".node .collapsed-group"] {
            let _ = write!(css, "{slot}{frame}{{stroke:var(--borderColor{index});");
            if fills > 0 {
                let _ = write!(css, "fill:var(--bkgColor{});", index % fills);
            }
            css.push('}');
        }
        let _ = write!(
            css,
            "{slot}.node .collapsed-indicator{{fill:var(--borderColor{index});}}\
{slot}.node .collapsed-separator{{stroke:var(--borderColor{index});}}"
        );
    }
}

/// The `classDef` rules of the classes (`createCssStyles` in `mermaidAPI.ts`): each class styles the
/// shapes of its nodes with `!important`, and the text of its labels when it has a color. A class
/// whose id holds braces or a comma, or whose styles hold braces, is left out: the reference would
/// scope such text under the id, which is not worth reproducing for a style that could only be an
/// attack.
fn write_class_rules(css: &mut String, id: &str, classes: &[FlowClass]) {
    for class in classes {
        let class_id = &class.id;
        if class_id.contains(['{', '}', ',']) {
            continue;
        }
        if let Some(declarations) = important(&class.styles) {
            for element in ["rect", "polygon", "ellipse", "circle", "path"] {
                let _ = write!(css, "{id} .{class_id} {element}{{{declarations}}}");
            }
        }
        // The DB keeps the entries with a color as text styles, with `fill` renamed `bgFill`; the
        // rule then names the color `fill`, as `mermaidAPI.ts` does.
        let text: Vec<String> = class
            .styles
            .iter()
            .filter(|style| style.contains("color"))
            .map(|style| {
                style
                    .replacen("fill", "bgFill", 1)
                    .replacen("color", "fill", 1)
            })
            .collect();
        if let Some(declarations) = important(&text) {
            let _ = write!(css, "{id} .{class_id} tspan{{{declarations}}}");
        }
    }
}

/// The declarations of a `classDef` with `!important`, or `None` when there are none or they hold braces.
fn important(styles: &[String]) -> Option<String> {
    let mut declarations = String::new();
    for style in styles.iter().map(|style| style.trim()) {
        if !style.is_empty() {
            declarations.push_str(style);
            declarations.push_str(" !important;");
        }
    }
    if declarations.is_empty() || declarations.chars().any(|c| c == '{' || c == '}') {
        None
    } else {
        Some(declarations)
    }
}

#[cfg(test)]
mod tests {
    use ferrigura_model::config::{MermaidConfig, defaults, typed};
    use ferrigura_model::theme::Theme;
    use ferrigura_model::value::Value;

    use super::*;
    use crate::unified::data::Node;
    use crate::unified::palette::node_style;

    /// The globals of a configuration with the given theme and look.
    fn globals(theme: &str, look: &str) -> Globals {
        Globals::of(&config(theme, look))
    }

    /// A configuration with the given theme and look, from the defaults.
    fn config(theme: &str, look: &str) -> MermaidConfig {
        let mut value = defaults();
        if let Some(object) = value.as_object_mut() {
            object.insert("theme".to_owned(), Value::from(theme));
            object.insert("look".to_owned(), Value::from(look));
        }
        typed(&value).expect("the defaults read back into the configuration types")
    }

    fn light() -> Value {
        Theme::ReduxColor.variables(None)
    }

    fn dark() -> Value {
        Theme::ReduxDarkColor.variables(None)
    }

    #[test]
    fn the_colors_are_custom_properties_on_the_diagram_id() {
        let sheet = style_sheet(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&light()),
                dark: None,
            },
            &globals("redux-color", "neo"),
            &[],
        );
        assert!(
            sheet.starts_with("#flowchart{--nodeBorder:#28253D;"),
            "{sheet}"
        );
        assert!(sheet.contains("--clusterBorder:#BDBCCC;"), "{sheet}");
        assert!(sheet.contains("--borderColor0:#E879F9;"), "{sheet}");
        assert!(sheet.contains("--bkgColor0:#FDF4FF;"), "{sheet}");
        assert!(sheet.contains("--floodColor:#000000;"), "{sheet}");
        assert!(
            sheet.contains(
                "#flowchart .cluster rect{fill:var(--clusterBkg);stroke:var(--clusterBorder);"
            ),
            "{sheet}"
        );
    }

    #[test]
    fn a_dark_scheme_writes_light_dark_values() {
        let light = light();
        let dark = dark();
        let sheet = style_sheet(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&light),
                dark: Some(&ThemeTokens::from_value(&dark)),
            },
            &globals("redux-color", "neo"),
            &[],
        );
        assert!(
            sheet.contains("#flowchart{--nodeBorder:light-dark(#28253D,#FFFFFF);"),
            "{sheet}"
        );
        assert!(
            sheet.contains("--floodColor:light-dark(#000000,#FFFFFF);"),
            "{sheet}"
        );
        // The dark theme has no fills, so the fill slots take its cluster background.
        assert!(sheet.contains(",#1E1A2E);"), "{sheet}");
        assert!(!sheet.contains("@media"), "{sheet}");
    }

    #[test]
    fn the_dark_theme_has_no_palette_fill() {
        let sheet = style_sheet(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&dark()),
                dark: None,
            },
            &globals("redux-dark-color", "classic"),
            &[],
        );
        assert!(!sheet.contains("bkgColor0:"), "{sheet}");
        assert!(sheet.contains("[data-look=\"classic\"][data-color-id=\"color-0\"].cluster:not(.swimlane) rect{stroke:var(--borderColor0);}"), "{sheet}");
    }

    #[test]
    fn a_theme_without_palette_has_no_palette_rules() {
        let sheet = style_sheet(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&light()),
                dark: None,
            },
            &globals("default", "neo"),
            &[],
        );
        assert!(!sheet.contains("color-0"), "{sheet}");
    }

    #[test]
    fn the_look_of_the_palette_rules_is_a_bare_word() {
        assert_eq!(safe_look("neo"), "neo");
        assert_eq!(safe_look("classic"), "classic");
        assert_eq!(safe_look("a]{b"), "classic");
        assert_eq!(safe_look(""), "classic");
    }

    #[test]
    fn class_definitions_name_the_shapes_and_the_text() {
        let classes = [FlowClass {
            id: "banner".to_owned(),
            styles: vec!["color:#fff".to_owned(), " background-color:#0f0".to_owned()],
        }];
        let sheet = style_sheet(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&light()),
                dark: None,
            },
            &globals("redux-color", "neo"),
            &classes,
        );
        assert!(
            sheet.contains(
                "#flowchart .banner rect{color:#fff !important;background-color:#0f0 !important;}"
            ),
            "{sheet}"
        );
        assert!(
            sheet.contains(
                "#flowchart .banner tspan{fill:#fff !important;background-fill:#0f0 !important;}"
            ),
            "{sheet}"
        );
    }

    #[test]
    fn a_class_with_braces_is_left_out() {
        let classes = [
            FlowClass {
                id: "bad".to_owned(),
                styles: vec!["fill:red}body{color:blue".to_owned()],
            },
            FlowClass {
                id: "bad,body".to_owned(),
                styles: vec!["fill:red".to_owned()],
            },
            FlowClass {
                id: "bad{x}".to_owned(),
                styles: vec!["fill:red".to_owned()],
            },
        ];
        let sheet = style_sheet(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&light()),
                dark: None,
            },
            &globals("redux-color", "neo"),
            &classes,
        );
        assert!(!sheet.contains(".bad"), "{sheet}");
        assert!(!sheet.contains("body"), "{sheet}");
    }

    #[test]
    fn ampersand_in_theme_variables_is_escaped_once() {
        let mut vars = light();
        if let Some(obj) = vars.as_object_mut() {
            obj.insert("nodeBorder".to_owned(), Value::from("a&b"));
        }
        let sheet = style_sheet(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&vars),
                dark: None,
            },
            &globals("redux-color", "neo"),
            &[],
        );
        assert!(sheet.contains("--nodeBorder:a&amp;b;"), "{sheet}");
        assert!(!sheet.contains("&amp;amp;"), "{sheet}");
    }

    #[test]
    fn the_label_style_of_a_node_is_left_out_of_its_rect() {
        let node = Node {
            css_styles: vec!["fill:#f9f".to_owned(), "color:#fff".to_owned()],
            label_style: "font-size:20px".to_owned(),
            ..Node::default()
        };
        assert_eq!(node_style(&node), "fill:#f9f !important");
    }

    #[test]
    fn a_later_value_replaces_the_earlier_one_in_place() {
        let node = Node {
            css_compiled_styles: vec!["fill:red".to_owned(), "stroke:#000".to_owned()],
            css_styles: vec!["fill:blue".to_owned()],
            ..Node::default()
        };
        assert_eq!(
            node_style(&node),
            "fill:blue !important;stroke:#000 !important"
        );
    }

    #[test]
    fn the_defs_are_the_two_shadows_and_a_gradient_only_with_use_gradient() {
        let defs = look_defs(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&light()),
                dark: None,
            },
            &globals("redux-color", "neo"),
        );
        assert_eq!(defs.matches("<filter ").count(), 2, "{defs}");
        assert!(
            defs.contains("id=\"flowchart-drop-shadow\" height=\"130%\" width=\"130%\""),
            "{defs}"
        );
        assert!(
            defs.contains("id=\"flowchart-drop-shadow-small\""),
            "{defs}"
        );
        assert!(defs.contains("flood-color=\"#000000\""), "{defs}");
        assert!(!defs.contains("linearGradient"), "{defs}");

        let mut overrides = Value::empty_object();
        if let Some(object) = overrides.as_object_mut() {
            object.insert("useGradient".to_owned(), Value::Bool(true));
        }
        let gradient = Theme::ReduxColor.variables(Some(&overrides));
        let defs = look_defs(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&gradient),
                dark: None,
            },
            &globals("redux-color", "neo"),
        );
        assert!(
            defs.contains("<linearGradient id=\"flowchart-gradient\""),
            "{defs}"
        );
        assert!(defs.contains("stop-color=\"#0042eb\""), "{defs}");
        assert!(defs.contains("stop-color=\"#eb0042\""), "{defs}");
    }

    #[test]
    fn embedded_fonts_are_named_by_the_rules_and_declared_first() {
        let family = ferrigura_text::EmbeddedFamily {
            variables: Vec::new(),
            weight: None,
            data_fingerprint: None,
            style: None,
            source_name: "Lexica Ultralegible".to_owned(),
            family: ferrigura_text::FontFamily::Sans,
            name: "fg-0123456789abcdef-sans".to_owned(),
            css_family: "\"fg-0123456789abcdef-sans\"".to_owned(),
            css: "@font-face{font-family:\"fg-0123456789abcdef-sans\";src:url(data:font/ttf;base64,AA==)}".to_owned(),
        };
        let sheet = style_sheet_with_fonts(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&light()),
                dark: None,
            },
            &globals("redux-color", "neo"),
            &[],
            std::slice::from_ref(&family),
            None,
        );
        assert!(
            sheet.starts_with("@font-face{font-family:\"fg-0123456789abcdef-sans\";"),
            "{sheet}"
        );
        assert!(
            sheet.contains("#flowchart{font-family:\"fg-0123456789abcdef-sans\";font-size:14px;"),
            "{sheet}"
        );
        assert!(
            sheet.contains("#flowchart .label{font-family:\"fg-0123456789abcdef-sans\";"),
            "{sheet}"
        );
    }

    #[test]
    fn the_flood_of_a_dark_theme_is_white() {
        let defs = look_defs(
            "flowchart",
            ThemeView {
                light: &ThemeTokens::from_value(&dark()),
                dark: None,
            },
            &globals("redux-dark-color", "neo"),
        );
        assert!(defs.contains("flood-color=\"#FFFFFF\""), "{defs}");
    }
}
