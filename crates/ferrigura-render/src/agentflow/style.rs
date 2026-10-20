//! The agentflow style sheet: the rules of `packages/mermaid/src/diagrams/agentflow/styles.ts` (the palette
//! rules of `genColor` first, then the static rules), the `classDef` rules of `createCssStyles`, and the
//! global rules that every diagram takes (`common::style::base_style_sheet`). The colours are custom
//! properties declared on the diagram's id (`docs-port/decisions/0007`), and the rules refer to them with
//! `var(…)`; the palette's slots are `--borderColor<n>` and `--bkgColor<n>`.
//!
//! The reference compiles the rules with the diagram's id as the scope, so every selector here starts
//! with `#id`, and the nested rules (`.edgeLabel p`) are written flat, in the order stylis writes them.

use ferrigura_model::agentflow::ClassDef;
use ferrigura_model::theme::khroma;
use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::EmbeddedFamily;

use super::colours::{KINDS, container_slot, container_slot_count, kind_class, kind_slot};
use crate::common::style::base_style_sheet;
use crate::context::{Globals, ThemeView};
use crate::unified::palette::{Palette, palette};

/// The style sheet's content for an agentflow diagram with the id `svg_id`, escaped for XML. The global
/// rules come first, then the diagram's: the palette rules, the static rules and the `classDef` rules.
/// `families` are the embedded fonts of the labels, whose `@font-face` rules come first.
#[must_use]
pub fn style_sheet_with_fonts(
    svg_id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    classes: &[ClassDef],
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
    let palette = palette(globals, tokens);
    let own = own_rules(&id, theme, globals, &font, palette.as_ref(), classes);
    base_style_sheet(svg_id, theme, &font, &font_size, families, &own)
}

/// The custom properties of the agentflow colours that the base sheet does not declare: the label and
/// title colours, the node and cluster fills, the edge label background, the arrowhead and the palette
/// slots. One block per scheme, as the base sheet writes its own (decision 0007).
fn custom_properties(id: &str, theme: ThemeView<'_>, palette: Option<&Palette>) -> String {
    match theme.dark {
        None => format!("{id}{{{}}}", declarations(theme.light, palette)),
        Some(dark) => format!(
            "{id}{{{}}}@media (prefers-color-scheme: dark){{{id}{{{}}}}}",
            declarations(theme.light, palette),
            declarations(dark, palette)
        ),
    }
}

/// The declarations of one scheme's colours. A colour the theme does not give is not declared, so a
/// rule that names it is invalid, as the reference's `undefined` makes it.
fn declarations(tokens: &ThemeTokens, palette: Option<&Palette>) -> String {
    let border2 = tokens.border.get(2).text();
    let colours: [(&str, Option<String>); 9] = [
        ("nodeTextColor", tokens.node_text_color.text()),
        ("titleColor", tokens.title_color.text()),
        ("mainBkg", tokens.main_bkg.text()),
        ("clusterBkg", tokens.cluster_bkg.text()),
        ("clusterBorder", tokens.cluster_border.text()),
        ("edgeLabelBackground", tokens.edge_label_background.text()),
        ("arrowheadColor", tokens.arrowhead_color.text()),
        ("tertiaryColor", tokens.tertiary_color.text()),
        ("border2", border2),
    ];
    let mut out = String::new();
    for (name, colour) in colours {
        if let Some(colour) = colour {
            out.push_str(&format!("--{name}:{colour};"));
        }
    }
    if let Some(palette) = palette {
        for (index, colour) in palette.border.iter().enumerate() {
            out.push_str(&format!("--borderColor{index}:{colour};"));
        }
        for (index, colour) in palette.background.iter().enumerate() {
            out.push_str(&format!("--bkgColor{index}:{colour};"));
        }
    }
    out
}

/// The diagram's own rules: the custom properties of its colours, then the palette rules (`genColor`), the
/// static rules of `getStyles`, the icon rules of `getIconStyles`, and the `classDef` rules of the classes.
/// The raw CSS text: the caller escapes the whole sheet.
fn own_rules(
    id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    font: &str,
    palette: Option<&Palette>,
    classes: &[ClassDef],
) -> String {
    let tokens = theme.light;
    let look = safe_look(&globals.look);
    let text_colour = if tokens.node_text_color.is_absent() {
        "textColor"
    } else {
        "nodeTextColor"
    };
    let title_colour = "titleColor";
    let stroke_width = px(&tokens.stroke_width, "1");
    let edge_stroke_width = px(&tokens.stroke_width, "2");
    let edge_label = "edgeLabelBackground";

    let mut css = custom_properties(id, theme, palette);
    if let Some(palette) = palette {
        css.push_str(&palette_rules(id, look, palette));
    }
    let static_rules = [
        rule(
            &format!("{id} .label"),
            &format!("font-family:{font};color:var(--{text_colour});"),
        ),
        rule(
            &format!("{id} .cluster-label text"),
            &format!("fill:var(--{title_colour});"),
        ),
        rule(
            &format!("{id} .cluster-label span"),
            &format!("color:var(--{title_colour});"),
        ),
        rule(
            &format!("{id} .cluster-label span p"),
            "background-color:transparent;",
        ),
        rule(
            &format!("{id} .label text,{id} span"),
            &format!("fill:var(--{text_colour});color:var(--{text_colour});"),
        ),
        rule(
            &format!(
                "{id} .node rect,{id} .node circle,{id} .node ellipse,{id} .node polygon,{id} .node path"
            ),
            &format!("fill:var(--mainBkg);stroke:var(--nodeBorder);stroke-width:{stroke_width};"),
        ),
        rule(
            &format!(
                "{id} .rough-node .label text,{id} .node .label text,{id} .image-shape .label,{id} .icon-shape .label"
            ),
            "text-anchor:middle;",
        ),
        rule(
            &format!("{id} .node .katex path"),
            &format!("fill:var(--{text_colour});stroke:var(--{text_colour});stroke-width:1px;"),
        ),
        rule(
            &format!(
                "{id} .rough-node .label,{id} .node .label,{id} .image-shape .label,{id} .icon-shape .label"
            ),
            "text-align:center;",
        ),
        rule(&format!("{id} .node.clickable"), "cursor:pointer;"),
        rule(
            &format!("{id} .root .anchor path"),
            "fill:var(--lineColor) !important;stroke-width:0;stroke:var(--lineColor);",
        ),
        rule(
            &format!("{id} .arrowheadPath"),
            "fill:var(--arrowheadColor);",
        ),
        rule(
            &format!("{id} .edgePaths .path"),
            &format!("stroke:var(--lineColor);stroke-width:{edge_stroke_width};"),
        ),
        rule(
            &format!("{id} .flowchart-link"),
            "stroke:var(--lineColor);fill:none;",
        ),
        rule(
            &format!("{id} .edgeLabel"),
            &format!("background-color:var(--{edge_label});text-align:center;"),
        ),
        rule(
            &format!("{id} .edgeLabel p"),
            &format!("background-color:var(--{edge_label});"),
        ),
        rule(
            &format!("{id} .edgeLabel rect"),
            &format!("opacity:0.5;background-color:var(--{edge_label});fill:var(--{edge_label});"),
        ),
        rule(
            &format!("{id} .labelBkg"),
            &format!("background-color:{};", faded(tokens, 0.5)),
        ),
        rule(
            &format!("{id} .cluster rect"),
            "fill:var(--clusterBkg);stroke:var(--clusterBorder);stroke-width:1px;",
        ),
        rule(
            &format!("{id} .flow-cluster rect"),
            "fill:none;stroke:var(--clusterBorder);stroke-width:0.75px;",
        ),
        rule(
            &format!("{id} .node .collapsed-indicator"),
            "fill:var(--nodeBorder);stroke:none;stroke-width:0;opacity:0.5;",
        ),
        rule(
            &format!("{id} .node .collapsed-separator"),
            "stroke-width:0.75px;",
        ),
        rule(
            &format!("{id} .cluster text"),
            &format!("fill:var(--{title_colour});"),
        ),
        rule(
            &format!("{id} .cluster span"),
            &format!("color:var(--{title_colour});"),
        ),
        rule(
            &format!("{id} div.mermaidTooltip"),
            &format!(
                "position:absolute;text-align:center;max-width:200px;padding:2px;font-family:{font};font-size:12px;background:var(--tertiaryColor);border:1px solid var(--border2);border-radius:2px;pointer-events:none;z-index:100;"
            ),
        ),
        rule(
            &format!("{id} .agentflowTitleText"),
            "text-anchor:middle;font-size:18px;fill:var(--textColor);",
        ),
        rule(&format!("{id} rect.text"), "fill:none;stroke-width:0;"),
        rule(
            &format!("{id} .icon-shape,{id} .image-shape"),
            &format!("background-color:var(--{edge_label});text-align:center;"),
        ),
        rule(
            &format!("{id} .icon-shape p,{id} .image-shape p"),
            &format!("background-color:var(--{edge_label});padding:2px;"),
        ),
        rule(
            &format!("{id} .icon-shape .label rect,{id} .image-shape .label rect"),
            &format!("opacity:0.5;background-color:var(--{edge_label});fill:var(--{edge_label});"),
        ),
    ];
    css.push_str(&static_rules.concat());
    css.push_str(&format!(
        "{}{}",
        rule(
            &format!("{id} .label-icon"),
            "display:inline-block;height:1em;overflow:visible;vertical-align:-0.125em;"
        ),
        rule(
            &format!("{id} .node .label-icon path"),
            "fill:currentColor;stroke:revert;stroke-width:revert;"
        ),
    ));
    css.push_str(&class_rules(id, classes));
    css
}

/// The palette rules of `genColor` (`styles.ts` 42-107): one rule per kind on its node's shapes, and one
/// per container slot on the expanded and the collapsed frames. A fill is written only when the theme has
/// fills (`hasBkgColors`), so a dark palette keeps the node's own background.
fn palette_rules(id: &str, look: &str, palette: &Palette) -> String {
    let border_count = palette.border.len();
    let fills = palette.background.len();
    let fill = |slot: usize| {
        if fills == 0 {
            String::new()
        } else {
            format!("fill:var(--bkgColor{});", slot % fills)
        }
    };
    let mut css = String::new();
    for kind in KINDS {
        let Some(slot) = kind_slot(kind) else {
            continue;
        };
        let slot = slot % border_count;
        // The `.node` is named on purpose (`styles.ts` 64-68): the neo stylesheet has rules of the same
        // specificity without it, and the palette has to win by weight.
        let selector = format!("{id} [data-look=\"{look}\"].node.{}", kind_class(kind));
        css.push_str(&rule(
            &format!("{selector} rect,{selector} path,{selector} polygon"),
            &format!("stroke:var(--borderColor{slot});{}", fill(slot)),
        ));
    }
    for index in 0..container_slot_count(border_count) {
        let slot = container_slot(index, border_count) % border_count;
        let expanded =
            format!("{id} [data-look=\"{look}\"][data-color-id=\"color-{slot}\"].cluster");
        let collapsed = format!("{id} [data-look=\"{look}\"][data-color-id=\"color-{slot}\"].node");
        let declarations = format!("stroke:var(--borderColor{slot});{}", fill(slot));
        let list = ["rect", "path"]
            .iter()
            .map(|suffix| format!("{expanded} {suffix},{collapsed} {suffix}"))
            .collect::<Vec<_>>()
            .join(",");
        css.push_str(&rule(&list, &declarations));
    }
    css
}

/// The `classDef` rules (`createCssStyles`, `mermaidAPI.ts`): each class styles the shapes of its nodes
/// with `!important`, and the text of its labels when it has a colour. A class whose declarations hold
/// braces is left out.
fn class_rules(id: &str, classes: &[ClassDef]) -> String {
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
    if declarations.is_empty() || declarations.contains(['{', '}']) {
        None
    } else {
        Some(declarations)
    }
}

/// One rule: the selector and its declarations, in braces.
fn rule(selector: &str, declarations: &str) -> String {
    format!("{selector}{{{declarations}}}")
}

/// A stroke width as the reference writes it in a `px` declaration: the theme's value, else `fallback`.
fn px(token: &Token, fallback: &str) -> String {
    format!("{}px", token.text().unwrap_or_else(|| fallback.to_owned()))
}

/// `fade(edgeLabelBackground, 0.5)` (`styles.ts`): khroma's `fade` writes the colour's red, green and blue
/// channels with the given alpha, as `rgba(r, g, b, a)`. The channels come from `mix` with black at full
/// weight, which keeps the first colour; an opaque colour comes back as `#rrggbb`. A colour that does not
/// come back so is kept as it is.
fn faded(tokens: &ThemeTokens, opacity: f64) -> String {
    let colour = tokens.edge_label_background.text().unwrap_or_default();
    let Ok(mixed) = khroma::mix(&colour, "#000000", 100.0) else {
        return colour;
    };
    let channel = |range: std::ops::Range<usize>| {
        mixed
            .strip_prefix('#')
            .filter(|hex| hex.len() == 6)
            .and_then(|hex| u8::from_str_radix(&hex[range], 16).ok())
    };
    match (channel(0..2), channel(2..4), channel(4..6)) {
        (Some(red), Some(green), Some(blue)) => {
            khroma::rgba(f64::from(red), f64::from(green), f64::from(blue), opacity)
        }
        _ => colour,
    }
}

/// The look a stylesheet is written for: a bare word, else `classic` (`safeLook`, `colorThemeGate.ts`).
fn safe_look(look: &str) -> &str {
    let bare = !look.is_empty()
        && look
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare { look } else { "classic" }
}

#[cfg(test)]
mod tests {
    use ferrigura_model::theme::{Theme, ThemeTokens};

    use super::*;

    /// The fade of a colour is the rgba of its channels, with the alpha written in (khroma's `fade`).
    #[test]
    fn the_label_background_is_faded_to_its_rgba() {
        let tokens = ThemeTokens::from_value(&Theme::ReduxColor.variables(None));
        assert_eq!(faded(&tokens, 0.5), "rgba(204, 204, 204, 0.5)");
    }
}
