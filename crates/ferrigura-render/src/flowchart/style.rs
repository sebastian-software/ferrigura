//! The flowchart's style sheet and look definitions: a port of
//! `packages/mermaid/src/diagrams/flowchart/styles.ts`, the rules of `packages/mermaid/src/styles.ts`
//! that a flowchart gets, the `classDef` rules of `mermaidAPI.ts` and `insertLookDefs.ts`.
//!
//! Colours follow `docs-port/decisions/0007`: every colour is a custom property declared on the
//! diagram's id, once for each set of variables, and the rules refer to it with `var(…)`. The
//! elements carry the same colours as presentation attributes, where they have one (see
//! `clusters.rs`). The rules that the reference writes but no flowchart element can match are left
//! out, each for a stated reason; `docs-port/flowchart/render-notes/styles.md` lists them, and
//! `tests/flowchart_styles.rs` checks every other rule against the reference's compiled CSS.

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::FlowClass;
use ferrigura_model::value::Value;
use ferrigura_text::EmbeddedFamily;

use crate::pie::Themes;
use crate::svg::{escape, font_faces};
use crate::unified::palette::{Palette, palette};
use crate::variables::{css_text, text_of, texts_of, truthy};

/// The colour tokens of the rules, each declared as a custom property on the diagram's id.
const COLOUR_TOKENS: [&str; 9] = [
    "nodeBorder",
    "mainBkg",
    "clusterBkg",
    "clusterBorder",
    "lineColor",
    "textColor",
    "nodeTextColor",
    "titleColor",
    "edgeLabelBackground",
];

/// The flood of the drop shadows in a light scheme (`insertLookDefs.ts:30`).
const LIGHT_FLOOD: &str = "#000000";

/// The flood of the drop shadows in a dark scheme (`insertLookDefs.ts:30`).
const DARK_FLOOD: &str = "#FFFFFF";

/// The `<style>` element's content for a flowchart with the id `svg_id`, escaped for XML so that it can
/// be written as the element's text. The look is read from `config.look`, the palette and the
/// shadow from the theme, and the `classDef` rules come from `classes`.
#[must_use]
pub fn style_sheet(
    svg_id: &str,
    themes: Themes<'_>,
    config: &MermaidConfig,
    classes: &[FlowClass],
) -> String {
    style_sheet_with_fonts(svg_id, themes, config, classes, &[])
}

/// [`style_sheet`] for a diagram whose text is set in embedded font subsets. The rules name the first
/// family, and its `@font-face` rules come first. With no families, the rules use the theme's
/// `fontFamily`, as the reference does.
#[must_use]
pub fn style_sheet_with_fonts(
    svg_id: &str,
    themes: Themes<'_>,
    config: &MermaidConfig,
    classes: &[FlowClass],
    families: &[EmbeddedFamily],
) -> String {
    let variables = themes.primary();
    let id = format!("#{svg_id}");
    let font = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| format!("\"{}\"", family.name),
    );
    let font_size = text_of(variables, "fontSize").unwrap_or_else(|| "16px".to_owned());
    let stroke_width = format!(
        "{}px",
        text_of(variables, "strokeWidth").unwrap_or_else(|| "1".to_owned())
    );
    let text_colour = if variables.get("nodeTextColor").is_some() {
        "nodeTextColor"
    } else {
        "textColor"
    };
    let palette = palette(config, variables);
    let look = safe_look(&config.look);
    let neo_stroke = if truthy(variables, "useGradient") {
        format!("url(#{svg_id}-gradient)")
    } else {
        "var(--nodeBorder)".to_owned()
    };
    let shadow = match text_of(variables, "dropShadow") {
        Some(shadow) if !shadow.is_empty() => shadow.replacen(
            "url(#drop-shadow)",
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        ),
        _ => "none".to_owned(),
    };

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
            "fill:var(--lineColor);stroke:var(--lineColor);",
        ),
        rule(&format!("{id} .marker.cross"), "stroke:var(--lineColor);"),
    ];
    if let Some(palette) = &palette {
        rules.push(palette_rules(&id, look, palette));
    }
    rules.extend([
        rule(
            &format!("{id} .label"),
            &format!("font-family:{font};color:var(--{text_colour});"),
        ),
        rule(&format!("{id} .cluster-label text"), "fill:var(--titleColor);"),
        rule(
            &format!("{id} .label text"),
            &format!("fill:var(--{text_colour});color:var(--{text_colour});"),
        ),
        rule(
            &format!("{id} .node rect,{id} .node circle,{id} .node ellipse,{id} .node polygon,{id} .node path"),
            &format!("fill:var(--mainBkg);stroke:var(--nodeBorder);stroke-width:{stroke_width};"),
        ),
        rule(
            &format!("{id} .node .label text,{id} .image-shape .label,{id} .icon-shape .label"),
            "text-anchor:middle;",
        ),
        rule(
            &format!("{id} .node .label,{id} .image-shape .label,{id} .icon-shape .label"),
            "text-align:center;",
        ),
        rule(&format!("{id} .node.clickable"), "cursor:pointer;"),
        rule(
            &format!("{id} .flowchart-link"),
            "stroke:var(--lineColor);fill:none;",
        ),
        rule(
            &format!("{id} .edgeLabel"),
            "background-color:var(--edgeLabelBackground);text-align:center;",
        ),
        rule(
            &format!("{id} .edgeLabel rect"),
            "opacity:0.5;background-color:var(--edgeLabelBackground);fill:var(--edgeLabelBackground);",
        ),
        rule(
            &format!("{id} .cluster rect"),
            "fill:var(--clusterBkg);stroke:var(--clusterBorder);stroke-width:1px;",
        ),
        rule(&format!("{id} .cluster text"), "fill:var(--titleColor);"),
        rule(
            &format!("{id} .node .collapsed-indicator"),
            "fill:var(--clusterBorder);stroke:none;opacity:0.6;",
        ),
        rule(
            &format!("{id} .node .collapsed-separator"),
            "stroke:var(--clusterBorder);stroke-width:0.75px;",
        ),
        rule(
            &format!("{id} .flowchartTitleText"),
            "text-anchor:middle;font-size:18px;fill:var(--textColor);",
        ),
        rule(&format!("{id} rect.text"), "fill:none;stroke-width:0;"),
        rule(
            &format!("{id} .icon-shape,{id} .image-shape"),
            "background-color:var(--edgeLabelBackground);text-align:center;",
        ),
        rule(
            &format!("{id} .icon-shape .label rect,{id} .image-shape .label rect"),
            "opacity:0.5;background-color:var(--edgeLabelBackground);fill:var(--edgeLabelBackground);",
        ),
    ]);

    // The look's rules. They match only the elements of their look, so both looks carry them.
    let filter = format!("filter:{shadow};");
    rules.extend([
        rule(
            &format!(
                "{id} [data-look=\"neo\"].node rect,{id} [data-look=\"neo\"].cluster rect,{id} [data-look=\"neo\"].node polygon"
            ),
            &format!("stroke:{neo_stroke};{filter}"),
        ),
        rule(
            &format!("{id} [data-look=\"neo\"].node path"),
            &format!("stroke:{neo_stroke};stroke-width:{stroke_width};"),
        ),
        rule(
            &format!("{id} [data-look=\"neo\"].node .outer-path"),
            &filter,
        ),
        rule(
            &format!("{id} [data-look=\"neo\"].node circle"),
            &format!("stroke:{neo_stroke};{filter}"),
        ),
        rule(
            &format!("{id}-drop-shadow feDropShadow,{id}-drop-shadow-small feDropShadow"),
            "flood-color:var(--floodColor);",
        ),
    ]);
    rules.push(class_rules(&id, classes));
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

/// The `<defs>` that the look's rules refer to, as `insertLookDefs.ts` appends them: the two drop
/// shadows, always, and the gradient only when the theme uses one. The ids are scoped to `svg_id`.
/// The shadows' flood is black, or white for a dark theme; the style sheet sets it per scheme.
#[must_use]
pub fn look_defs(svg_id: &str, themes: Themes<'_>, config: &MermaidConfig) -> String {
    let variables = themes.primary();
    let flood = match themes {
        Themes::Single(_) => flood_colour(config),
        Themes::Schemes { .. } => LIGHT_FLOOD,
    };
    let shadow = |id: &str, size: &str, offset: &str| {
        format!(
            "<defs><filter id=\"{}\" height=\"{size}\" width=\"{size}\"><feDropShadow dx=\"{offset}\" dy=\"{offset}\" stdDeviation=\"0\" flood-opacity=\"0.06\" flood-color=\"{flood}\"/></filter></defs>",
            escape(id)
        )
    };
    let mut defs = shadow(&format!("{svg_id}-drop-shadow"), "130%", "4");
    defs.push_str(&shadow(&format!("{svg_id}-drop-shadow-small"), "150%", "2"));
    if truthy(variables, "useGradient") {
        let start = text_of(variables, "gradientStart").unwrap_or_default();
        let stop = text_of(variables, "gradientStop").unwrap_or_default();
        defs.push_str(&format!(
            "<linearGradient id=\"{}-gradient\" gradientUnits=\"objectBoundingBox\" x1=\"0%\" y1=\"0%\" x2=\"100%\" y2=\"0%\"><stop offset=\"0%\" stop-color=\"{}\" stop-opacity=\"1\"/><stop offset=\"100%\" stop-color=\"{}\" stop-opacity=\"1\"/></linearGradient>",
            escape(svg_id),
            escape(&start),
            escape(&stop),
        ));
    }
    defs
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
fn flood_colour(config: &MermaidConfig) -> &'static str {
    if config.theme.contains("dark") {
        DARK_FLOOD
    } else {
        LIGHT_FLOOD
    }
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
            let slots = palette(config, variables).map(|palette| slot_counts(&palette));
            rule(&id, &declarations(variables, flood_colour(config), slots))
        }
        Themes::Schemes { light, dark } => {
            let slots = palette(config, light).map(|palette| slot_counts(&palette));
            format!(
                "{}@media (prefers-color-scheme: dark){{{}}}",
                rule(&id, &declarations(light, LIGHT_FLOOD, slots)),
                rule(&id, &declarations(dark, DARK_FLOOD, slots)),
            )
        }
    }
}

/// The number of border slots and of fill slots of a palette.
fn slot_counts(palette: &Palette) -> (usize, usize) {
    (palette.border.len(), palette.background.len())
}

/// The declarations of the custom properties of one set. A set without fills gives every fill slot its
/// cluster background, so that a palette of the other scheme fills a cluster as this scheme's own rules
/// would.
fn declarations(variables: &Value, flood: &str, slots: Option<(usize, usize)>) -> String {
    let mut out = String::new();
    for token in COLOUR_TOKENS {
        if let Some(colour) = text_of(variables, token) {
            out.push_str(&format!("--{token}:{};", css_text(&colour)));
        }
    }
    if let Some((borders, fills)) = slots {
        let border_list = texts_of(variables, "borderColorArray");
        let fill_list = texts_of(variables, "bkgColorArray");
        for index in 0..borders {
            if let Some(colour) = cyclic(&border_list, index) {
                out.push_str(&format!("--borderColor{index}:{};", css_text(colour)));
            }
        }
        for index in 0..fills {
            let colour = cyclic(&fill_list, index)
                .map(str::to_owned)
                .or_else(|| text_of(variables, "clusterBkg"));
            if let Some(colour) = colour {
                out.push_str(&format!("--bkgColor{index}:{};", css_text(&colour)));
            }
        }
    }
    out.push_str(&format!("--floodColor:{flood};"));
    out
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
fn palette_rules(id: &str, look: &str, palette: &Palette) -> String {
    let fills = palette.background.len();
    let mut css = String::new();
    for index in 0..palette.border.len() {
        let slot = format!("{id} [data-look=\"{look}\"][data-color-id=\"color-{index}\"]");
        let stroke = format!("stroke:var(--borderColor{index});");
        let fill = if fills == 0 {
            String::new()
        } else {
            format!("fill:var(--bkgColor{});", index % fills)
        };
        css.push_str(&rule(
            &format!("{slot}.cluster:not(.swimlane) rect"),
            &format!("{stroke}{fill}"),
        ));
        css.push_str(&rule(
            &format!("{slot}.node .collapsed-group"),
            &format!("{stroke}{fill}"),
        ));
        css.push_str(&rule(
            &format!("{slot}.node .collapsed-indicator"),
            &format!("fill:var(--borderColor{index});"),
        ));
        css.push_str(&rule(
            &format!("{slot}.node .collapsed-separator"),
            &format!("stroke:var(--borderColor{index});"),
        ));
    }
    css
}

/// The `classDef` rules of the classes (`createCssStyles` in `mermaidAPI.ts`): each class styles the
/// shapes of its nodes with `!important`, and the text of its labels when it has a colour. A class
/// whose styles hold braces is left out: the reference would scope such text under the id, which is
/// not worth reproducing for a style that could only be an attack.
fn class_rules(id: &str, classes: &[FlowClass]) -> String {
    let mut css = String::new();
    for class in classes {
        if let Some(declarations) = important(&class.styles) {
            for element in ["rect", "polygon", "ellipse", "circle", "path"] {
                css.push_str(&rule(
                    &format!("{id} .{} {element}", class.id),
                    &declarations,
                ));
            }
        }
        // The DB keeps the entries with a colour as text styles, with `fill` renamed `bgFill`; the
        // rule then names the colour `fill`, as `mermaidAPI.ts` does.
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
            css.push_str(&rule(&format!("{id} .{} tspan", class.id), &declarations));
        }
    }
    css
}

/// The declarations of a `classDef` with `!important`, or `None` when there are none or they hold braces.
fn important(styles: &[String]) -> Option<String> {
    let declarations: String = styles
        .iter()
        .map(|style| style.trim())
        .filter(|style| !style.is_empty())
        .map(|style| format!("{style} !important;"))
        .collect();
    if declarations.is_empty() || declarations.chars().any(|c| c == '{' || c == '}') {
        None
    } else {
        Some(declarations)
    }
}

#[cfg(test)]
mod tests {
    use ferrigura_model::config::{defaults, typed};
    use ferrigura_model::theme::Theme;

    use super::*;
    use crate::unified::data::Node;
    use crate::unified::palette::node_style;

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
    fn the_colours_are_custom_properties_on_the_diagram_id() {
        let sheet = style_sheet(
            "flowchart",
            Themes::Single(&light()),
            &config("redux-color", "neo"),
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
    fn a_dark_scheme_follows_prefers_color_scheme() {
        let light = light();
        let dark = dark();
        let sheet = style_sheet(
            "flowchart",
            Themes::Schemes {
                light: &light,
                dark: &dark,
            },
            &config("redux-color", "neo"),
            &[],
        );
        assert!(
            sheet.contains("@media (prefers-color-scheme: dark){#flowchart{--nodeBorder:#FFFFFF;"),
            "{sheet}"
        );
        assert!(sheet.contains("--floodColor:#FFFFFF;}}"), "{sheet}");
        // The dark theme has no fills, so the fill slots take its cluster background.
        assert!(sheet.contains("--bkgColor0:#1E1A2E;"), "{sheet}");
    }

    #[test]
    fn the_dark_theme_has_no_palette_fill() {
        let sheet = style_sheet(
            "flowchart",
            Themes::Single(&dark()),
            &config("redux-dark-color", "classic"),
            &[],
        );
        assert!(!sheet.contains("bkgColor0:"), "{sheet}");
        assert!(sheet.contains("[data-look=\"classic\"][data-color-id=\"color-0\"].cluster:not(.swimlane) rect{stroke:var(--borderColor0);}"), "{sheet}");
    }

    #[test]
    fn a_theme_without_palette_has_no_palette_rules() {
        let sheet = style_sheet(
            "flowchart",
            Themes::Single(&light()),
            &config("default", "neo"),
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
            Themes::Single(&light()),
            &config("redux-color", "neo"),
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
        let classes = [FlowClass {
            id: "bad".to_owned(),
            styles: vec!["fill:red}body{color:blue".to_owned()],
        }];
        let sheet = style_sheet(
            "flowchart",
            Themes::Single(&light()),
            &config("redux-color", "neo"),
            &classes,
        );
        assert!(!sheet.contains(".bad"), "{sheet}");
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
            Themes::Single(&light()),
            &config("redux-color", "neo"),
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
            Themes::Single(&gradient),
            &config("redux-color", "neo"),
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
            family: ferrigura_text::FontFamily::Sans,
            name: "fg-0123456789abcdef-sans".to_owned(),
            css: "@font-face{font-family:\"fg-0123456789abcdef-sans\";src:url(data:font/ttf;base64,AA==)}".to_owned(),
        };
        let sheet = style_sheet_with_fonts(
            "flowchart",
            Themes::Single(&light()),
            &config("redux-color", "neo"),
            &[],
            std::slice::from_ref(&family),
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
            Themes::Single(&dark()),
            &config("redux-dark-color", "neo"),
        );
        assert!(defs.contains("flood-color=\"#FFFFFF\""), "{defs}");
    }
}
