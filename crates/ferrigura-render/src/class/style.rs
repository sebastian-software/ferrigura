//! The class diagram's style sheet: `packages/mermaid/src/diagrams/class/styles.js`, with the common
//! rules of `packages/mermaid/src/styles.ts` that every diagram gets. Each selector starts with the
//! diagram's id, as the reference's stylis scoping writes it.
//!
//! Colors follow `docs/decisions/0011-themes-as-css-variables.md`: every color is a custom property declared on the
//! diagram's id (once, with `light-dark()` values when both schemes are written), and the rules refer to it
//! with `var(…)`. The palette of the color themes gives each class box its border and fill by its
//! color slot (`genColor`, `styles.js` 14-38). The elements carry no color attribute of their own,
//! as the flowchart's do not (`flowchart/style.rs`).

use std::borrow::Cow;
use std::fmt::Write as _;

use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::EmbeddedFamily;

use crate::context::{Globals, ThemeView};
use crate::design::{self, Tone};
use crate::flowchart::style::look_defs_for;
use crate::svg::{escape_css_tail_in_place, font_faces_capacity, font_faces_css_into};
use crate::unified::palette::{Palette, palette};

/// The color tokens of the class rules, the theme variables that `styles.js` reads, in the order of the
/// custom properties.
fn color_tokens(tokens: &ThemeTokens) -> [(&'static str, &Token); 10] {
    [
        ("nodeBorder", &tokens.node_border),
        ("mainBkg", &tokens.main_bkg),
        ("classText", &tokens.class_text),
        ("titleColor", &tokens.title_color),
        ("clusterBkg", &tokens.cluster_bkg),
        ("clusterBorder", &tokens.cluster_border),
        ("noteTextColor", &tokens.note_text_color),
        ("lineColor", &tokens.line_color),
        ("textColor", &tokens.text_color),
        ("edgeLabelBackground", &tokens.edge_label_background),
    ]
}

/// The flood of the drop shadows in a light scheme (`insertLookDefs.ts:30`).
const LIGHT_FLOOD: &str = "#000000";

/// The flood of the drop shadows in a dark scheme (`insertLookDefs.ts:30`).
const DARK_FLOOD: &str = "#FFFFFF";

/// Room for the rules of a class style sheet besides its font faces: about 8 KiB.
const RULES_CAPACITY: usize = 10 * 1024;

/// The `<style>` element's text of a class diagram with the id `svg_id`. The font of the rules is
/// the embedded family of the diagram when it has one (`families`), else the theme's `fontFamily`.
/// In the design mode (`design`), the mode's rules follow the class rules, and the mode's own
/// properties take the place of the palette and the look's flood
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
    let stroke_width = tokens.stroke_width.text().unwrap_or_else(|| "1".to_owned());
    let shadow: Cow<'_, str> = match tokens.drop_shadow.text() {
        Some(shadow) if !shadow.is_empty() => Cow::Owned(shadow.replacen(
            "url(#drop-shadow)",
            &format!("url(#{svg_id}-drop-shadow)"),
            1,
        )),
        _ => Cow::Borrowed("none"),
    };
    let neo_stroke: Cow<'_, str> = if tokens.use_gradient.truthy() {
        Cow::Owned(format!("url(#{svg_id}-gradient)"))
    } else {
        Cow::Borrowed("var(--nodeBorder)")
    };
    let look = safe_look(&globals.look);

    let mut css = String::with_capacity(font_faces_capacity(svg_id, families) + RULES_CAPACITY);
    font_faces_css_into(&mut css, svg_id, families);
    let rules_start = css.len();
    write_custom_properties(&mut css, &id, theme, globals, design);
    let _ = write!(
        css,
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
    );
    write_class_rules(
        &mut css,
        &id,
        look,
        &font,
        &stroke_width,
        tokens,
        globals,
        design,
    );
    if design.is_some() {
        write_design_rules(&mut css, &id);
    }
    escape_css_tail_in_place(&mut css, rules_start);
    css
}

/// The `<defs>` of the look (the drop shadows and the gradient), which the class diagram writes after
/// its title as the flowchart does. The design mode draws its own shadows under the same ids.
#[must_use]
pub fn defs(svg_id: &str, theme: ThemeView<'_>, globals: &Globals, design: Option<Tone>) -> String {
    look_defs_for(svg_id, theme, globals, design)
}

/// The rules of the design mode, after the class rules (`docs/decisions/0023-design-mode.md`):
/// hairlines, a relation in the edge's weight, the markers' paint, muted annotations, visibility
/// marks, edge labels and cardinalities, a group frame without the node's shadow, a flat note, and
/// the colors of the roles. A class's own styles are written on its elements and keep the last
/// word.
fn write_design_rules(out: &mut String, id: &str) {
    let node_width = design::NODE_STROKE_WIDTH;
    let edge_width = design::EDGE_STROKE_WIDTH;
    let thick_width = design::THICK_EDGE_STROKE_WIDTH;
    let _ = write!(
        out,
        "{id} .edge-thickness-normal,{id} .relation{{stroke-width:{edge_width};}}\
{id} .edge-thickness-thick{{stroke-width:{thick_width};}}\
{id} .edge-pattern-dashed,{id} .dashed-line{{stroke-dasharray:5 4;}}\
{id} .node rect,{id} .node polygon,{id} .node path,{id} [data-look=\"neo\"].node path{{stroke-width:{node_width};}}\
{id} .node line.divider{{stroke:var(--nodeBorder);stroke-width:1px;}}\
{id} .annotation-group text,{id} .visibility text{{fill:var(--titleColor);}}\
{id} .marker.classDiagram .design-hollow{{fill:var(--mainBkg);stroke:var(--lineColor);stroke-width:{node_width};stroke-linejoin:round;stroke-dasharray:none;}}\
{id} .marker.classDiagram .design-filled{{fill:var(--lineColor);stroke:var(--lineColor);stroke-width:{node_width};stroke-linejoin:round;stroke-dasharray:none;}}\
{id} .marker.classDiagram .design-open{{fill:none;stroke:var(--lineColor);stroke-width:{edge_width};stroke-linejoin:round;stroke-linecap:round;stroke-dasharray:none;}}\
{id} .edgeLabel .label text{{fill:var(--titleColor);}}\
{id} .edgeLabel rect.background,{id} .edgeLabel[data-look] rect.background{{fill:none;stroke:none;opacity:1;}}\
{id} .edgeTerminals text,{id} .edgeTerminals .label text{{fill:var(--titleColor);}}\
{id} [data-look=\"neo\"].cluster rect{{stroke:var(--clusterBorder);filter:none;}}\
{id}-drop-shadow feDropShadow,{id}-drop-shadow-small feDropShadow{{flood-color:var(--floodColor);flood-opacity:1;}}"
    );
    for role in design::CLASS_ROLES {
        let (fill, stroke, text) = role.properties();
        let node = format!("{id} [data-role=\"{}\"].node", role.name());
        let _ = write!(
            out,
            "{node} rect.outline{{fill:var(--{fill});stroke:var(--{stroke});}}\
{node} line.divider{{stroke:var(--{stroke});}}\
{node} .label-group text,{node} .members-group .label text,{node} .methods-group .label text{{fill:var(--{text});}}"
        );
    }
}

/// The class rules of `styles.js` (lines 42-241) after the common ones, with the palette rules first
/// (`genColor`).
#[allow(clippy::too_many_arguments)]
fn write_class_rules(
    css: &mut String,
    id: &str,
    look: &str,
    font: &str,
    stroke_width: &str,
    tokens: &ThemeTokens,
    globals: &Globals,
    design: Option<Tone>,
) {
    // The design mode uses no palette: a class is neutral unless its role says otherwise.
    if design.is_none()
        && let Some(palette) = palette(globals, tokens)
    {
        write_palette_rules(css, id, look, &palette);
    }
    let _ = write!(
        css,
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
         {id} .node rect,{id} .node circle,{id} .node ellipse,{id} .node polygon,{id} .node path{{fill:var(--mainBkg);stroke:var(--nodeBorder);stroke-width:{stroke_width};}}\
         {id} .divider{{stroke:var(--nodeBorder);stroke-width:1;}}\
         {id} g.clickable{{cursor:pointer;}}\
         {id} g.classGroup rect{{fill:var(--mainBkg);stroke:var(--nodeBorder);}}\
         {id} g.classGroup line{{stroke:var(--nodeBorder);stroke-width:1;}}\
         {id} .classLabel .box{{stroke:none;stroke-width:0;fill:var(--mainBkg);opacity:0.5;}}\
         {id} .classLabel .label{{fill:var(--nodeBorder);font-size:10px;}}\
         {id} .relation{{stroke:var(--lineColor);stroke-width:{stroke_width};fill:none;}}\
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
    );
}

/// `genColor` (`styles.js` 14-38): for each color slot of the palette, the outline of a class box
/// takes the slot's border and fill, and its divider the border. The rules name the look and the
/// slot in their selectors, and the custom properties of the slot.
fn write_palette_rules(css: &mut String, id: &str, look: &str, palette: &Palette) {
    let fills = palette.background.len();
    let mut slot = String::new();
    for index in 0..palette.border.len() {
        slot.clear();
        let _ = write!(
            slot,
            "{id} [data-look=\"{look}\"][data-color-id=\"color-{index}\"].node"
        );
        let _ = write!(
            css,
            "{slot} .outer-path path{{stroke:var(--borderColor{index});"
        );
        if fills > 0 {
            let _ = write!(css, "fill:var(--bkgColor{});", index % fills);
        }
        let _ = write!(
            css,
            "}}{slot} .divider path{{stroke:var(--borderColor{index});}}"
        );
    }
}

/// Appends the custom properties of the colors, declared on the diagram's id. With one set they are
/// written as they are; with a light and a dark set, the values that differ
/// are `light-dark()` pairs (decision 0011).
fn write_custom_properties(
    css: &mut String,
    id: &str,
    theme: ThemeView<'_>,
    globals: &Globals,
    design: Option<Tone>,
) {
    if let Some(tone) = design {
        let write_block = |out: &mut String, tokens: &ThemeTokens, tone: Tone| {
            out.push_str(id);
            out.push('{');
            for (name, token) in color_tokens(tokens) {
                if let Some(colour) = token.text() {
                    let _ = write!(out, "--{name}:{colour};");
                }
            }
            out.push_str(design::declarations(tone));
            out.push_str(design::class_declarations(tone));
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
        return;
    }
    match theme.dark {
        None => {
            let flood = if globals.theme.contains("dark") {
                DARK_FLOOD
            } else {
                LIGHT_FLOOD
            };
            css.push_str(id);
            css.push('{');
            write_declarations(
                css,
                theme.light,
                palette(globals, theme.light).as_ref(),
                flood,
            );
            css.push('}');
        }
        Some(dark) => {
            let mut light_block = format!("{id}{{");
            write_declarations(
                &mut light_block,
                theme.light,
                palette(globals, theme.light).as_ref(),
                LIGHT_FLOOD,
            );
            light_block.push('}');
            let mut dark_block = format!("{id}{{");
            write_declarations(
                &mut dark_block,
                dark,
                palette(globals, dark).as_ref(),
                DARK_FLOOD,
            );
            dark_block.push('}');
            css.push_str(&crate::scheme::both(&light_block, &dark_block));
        }
    }
}

/// Appends the declarations of the custom properties of one set of variables.
fn write_declarations(
    out: &mut String,
    tokens: &ThemeTokens,
    palette: Option<&Palette>,
    flood: &str,
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
        let fills = tokens.bkg_color_array.texts();
        for index in 0..palette.background.len() {
            let color: Cow<'_, str> = if fills.is_empty() {
                Cow::Owned(tokens.cluster_bkg.text().unwrap_or_default())
            } else {
                Cow::Borrowed(&fills[index % fills.len()])
            };
            let _ = write!(out, "--bkgColor{index}:{color};");
        }
    }
    let _ = write!(out, "--floodColor:{flood};");
}

/// The look a stylesheet is written for: a bare word, else `classic` (`safeLook`).
fn safe_look(look: &str) -> &str {
    let bare = !look.is_empty()
        && look
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare { look } else { "classic" }
}
