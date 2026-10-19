//! The pie diagram: a port of `packages/mermaid/src/diagrams/pie/pieRenderer.ts` and
//! `pieStyles.ts`. The layout follows the reference step by step, so that the geometry can be
//! compared with it (`tools/port/`); the text widths come from `ferrigura-text`, where the reference
//! measures the DOM.
//!
//! The colours follow `docs-port/decisions/0007`: every colour is a CSS custom property, declared on
//! the diagram's own id, and every element that has a colour also carries its concrete value as a
//! presentation attribute, so a viewer without custom properties still draws the colours. No colour
//! is set inline. Every selector starts with the diagram's id, so the diagram does not reach other
//! diagrams or the page around it.

use std::f64::consts::PI;

use ferrigura_model::config::PieDiagramConfig;
use ferrigura_model::pie::{PieDiagram, PieSection};
use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::{
    EmbeddedFamily, FontFamily, FontStyle, FontWeight, Run, embed_runs, face, layout_width,
    measure, measure_box,
};

use crate::RenderError;
use crate::a11y;
use crate::context::{Context, ThemeView};
use crate::svg::{arc_path, css_text, escape, font_faces, number};

const MARGIN: f64 = 40.0;
const LEGEND_RECT_SIZE: f64 = 18.0;
const LEGEND_SPACING: f64 = 4.0;
const HEIGHT: f64 = 450.0;
const PIE_WIDTH: f64 = HEIGHT;

/// The number of slice colours: the reference's `myGeneratedColors` has twelve, and a slice takes
/// the colour of its position among all the sections, wrapping around.
const SLICE_COLOURS: usize = 12;

/// One slice of the pie: its section and its position among the sections, and its start and end angles.
struct Slice<'a> {
    section: &'a PieSection,
    index: usize,
    start: f64,
    end: f64,
}

/// Renders a pie diagram as an SVG document. The context carries the pie section of the configuration
/// as its section, and the theme's tokens.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset.
pub fn render(input: &Context<'_, PieDiagram, PieDiagramConfig>) -> Result<String, RenderError> {
    let tokens = input.theme.light;
    let config = input.section;
    let sections = &input.diagram.sections;
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);

    let outer_stroke_width = tokens.pie_outer_stroke_width.px().unwrap_or(2.0);
    let inner_hole = if config.donut_hole > 0.0 && config.donut_hole <= 0.9 {
        config.donut_hole
    } else {
        0.0
    };
    let radius = PIE_WIDTH.min(HEIGHT) / 2.0 - MARGIN;
    let label_radius = radius * config.text_position;
    let inner_radius = inner_hole * radius;

    let total: f64 = sections.iter().map(|section| section.value).sum();
    let arcs = pie_arcs(sections, total);
    let visible: Vec<&Slice<'_>> = arcs
        .iter()
        .filter(|arc| to_fixed0(arc.section.value / total * 100.0) != "0")
        .collect();

    let section_size = tokens.pie_section_text_size.px().unwrap_or(16.0);
    let legend_size = tokens.pie_legend_text_size.px().unwrap_or(16.0);
    let title_size = tokens.pie_title_text_size.px().unwrap_or(25.0);

    let legend_labels: Vec<String> = sections
        .iter()
        .map(|section| {
            if input.diagram.show_data {
                format!("{} [{}]", section.label, number(section.value))
            } else {
                section.label.clone()
            }
        })
        .collect();
    // The legend's widths are the boxes its `getBoundingClientRect` gives (`pieRenderer.ts` 183-188).
    let mut longest_text_width: f64 = 0.0;
    for label in &legend_labels {
        longest_text_width = longest_text_width.max(measure_box(sans, label, legend_size)?.width());
    }

    let legend_height = LEGEND_RECT_SIZE + LEGEND_SPACING;
    let total_legend_height = sections.len() as f64 * legend_height;
    let mut chart_height = HEIGHT;
    let mut chart_width = PIE_WIDTH + MARGIN;
    let legend_offset = legend_height * sections.len() as f64 / 2.0;
    let horizontal_text = -longest_text_width / 2.0 - (LEGEND_RECT_SIZE + LEGEND_SPACING);
    let mut pie_transform: Option<String> = None;
    let legend_transform = |index: usize| -> String {
        let (x, y) = match config.legend_position.as_str() {
            "center" => (
                horizontal_text,
                index as f64 * legend_height - legend_offset,
            ),
            "top" => (horizontal_text, index as f64 * legend_height - radius),
            "bottom" => (
                horizontal_text,
                index as f64 * legend_height - (-radius - legend_height),
            ),
            "left" => (
                -radius - (LEGEND_RECT_SIZE + LEGEND_SPACING),
                index as f64 * legend_height - legend_offset,
            ),
            _ => (
                12.0 * LEGEND_RECT_SIZE,
                index as f64 * legend_height - legend_offset,
            ),
        };
        format!("translate({}, {})", number(x), number(y))
    };
    match config.legend_position.as_str() {
        "center" => {}
        "top" => {
            chart_height += total_legend_height;
            pie_transform = Some(format!(
                "translate(0, {})",
                number(total_legend_height + legend_height)
            ));
        }
        "bottom" => chart_height += total_legend_height,
        "left" => {
            chart_width += LEGEND_RECT_SIZE + LEGEND_SPACING + longest_text_width;
            pie_transform = Some(format!(
                "translate({}, 0)",
                number(longest_text_width + LEGEND_RECT_SIZE + LEGEND_SPACING)
            ));
        }
        _ => chart_width += LEGEND_RECT_SIZE + LEGEND_SPACING + longest_text_width,
    }

    let title = input.diagram.title.as_str();
    // The title is centred by `text-anchor: middle` on its advance, and the view box takes its box.
    let title_advance = if title.is_empty() {
        0.0
    } else {
        layout_width(measure(sans, title, title_size)?)
    };
    let title_width = if title.is_empty() {
        0.0
    } else {
        measure_box(sans, title, title_size)?.width()
    };
    let title_left = PIE_WIDTH / 2.0 - title_width / 2.0;
    let title_right = PIE_WIDTH / 2.0 + title_width / 2.0;
    let view_box_x = 0.0_f64.min(title_left);
    let view_box_right = chart_width.max(title_right);
    let total_width = layout_unit(view_box_right - view_box_x);

    let slice_labels: Vec<String> = visible
        .iter()
        .map(|arc| slice_text(arc.section, total))
        .collect();

    // Every text the diagram draws, to subset the font once for the whole diagram.
    let mut runs: Vec<Run<'_>> = Vec::new();
    runs.push(Run {
        text: title,
        face: sans,
    });
    for label in legend_labels.iter().chain(&slice_labels) {
        runs.push(Run {
            text: label,
            face: sans,
        });
    }
    let families = embed_runs(&runs)?;
    let family_name = families.first().map_or_else(
        || "\"sans-serif\"".to_owned(),
        |family| family.css_family.clone(),
    );

    let mut body = String::new();
    // `appendDivSvgG` leaves an empty group before the diagram's own elements (`mermaidAPI.ts:418`).
    body.push_str("<g></g>");
    body.push_str(&format!(
        "<g transform=\"translate({},{})\">",
        number(PIE_WIDTH / 2.0),
        number(HEIGHT / 2.0)
    ));
    let pie_open = pie_transform.as_ref().map_or_else(
        || "<g>".to_owned(),
        |transform| format!("<g transform=\"{transform}\">"),
    );
    body.push_str(&pie_open);
    body.push_str(&format!(
        "<circle class=\"pieOuterCircle\" cx=\"0\" cy=\"0\" r=\"{}\"{}/>",
        number(radius + outer_stroke_width / 2.0),
        attribute("stroke", &tokens.pie_outer_stroke_color),
    ));
    for arc in &visible {
        let mut class = format!("pieCircle {}", slot_class(arc.index));
        if config.highlight_slice == "hover" {
            class.push_str(" highlightedOnHover");
        } else if config.highlight_slice == arc.section.label {
            class.push_str(" highlighted");
        }
        body.push_str(&format!(
            "<path class=\"{class}\" d=\"{}\"{}{}/>",
            arc_path(inner_radius, radius, arc.start, arc.end),
            attribute("fill", slot_colour(tokens, arc.index)),
            attribute("stroke", &tokens.pie_stroke_color),
        ));
    }
    for (arc, text) in visible.iter().zip(&slice_labels) {
        let middle = (arc.start + arc.end) / 2.0 - PI / 2.0;
        let (cx, cy) = (label_radius * middle.cos(), label_radius * middle.sin());
        let width = layout_width(measure(sans, text, section_size)?);
        body.push_str(&format!(
            "<text class=\"slice\"{} x=\"{}\" y=\"{}\">{}</text>",
            attribute("fill", &tokens.pie_section_text_color),
            number(cx - width / 2.0),
            number(cy),
            escape(text)
        ));
    }
    body.push_str("</g>");

    let title_x = -title_advance / 2.0;
    body.push_str(&format!(
        "<text class=\"pieTitleText\"{} x=\"{}\" y=\"{}\">{}</text>",
        attribute("fill", &tokens.pie_title_text_color),
        number(title_x),
        number(-(HEIGHT - 50.0) / 2.0),
        escape(title)
    ));

    for (index, label) in legend_labels.iter().enumerate() {
        body.push_str(&format!(
            "<g class=\"legend\" transform=\"{}\"><rect class=\"{}\" width=\"{}\" height=\"{}\"{}{}/><text{} x=\"{}\" y=\"{}\">{}</text></g>",
            legend_transform(index),
            slot_class(index),
            number(LEGEND_RECT_SIZE),
            number(LEGEND_RECT_SIZE),
            attribute("fill", slot_colour(tokens, index)),
            attribute("stroke", slot_colour(tokens, index)),
            attribute("fill", &tokens.pie_legend_text_color),
            number(LEGEND_RECT_SIZE + LEGEND_SPACING),
            number(LEGEND_RECT_SIZE - LEGEND_SPACING),
            escape(label),
        ));
    }
    body.push_str("</g>");

    let size_attributes = if input.section.use_max_width {
        format!(
            "width=\"100%\" style=\"max-width: {}px;\"",
            number(total_width)
        )
    } else {
        format!(
            "height=\"{}\" width=\"{}\"",
            number(chart_height),
            number(total_width)
        )
    };
    let style = style_element(input.id, input.theme, &family_name, &families);
    let accessibility =
        a11y::children(input.id, &input.diagram.acc_title, &input.diagram.acc_descr);
    let accessible_attributes = a11y::root_attributes(
        input.id,
        "pie",
        &input.diagram.acc_title,
        &input.diagram.acc_descr,
    );
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"{id}\" {size_attributes} viewBox=\"{x} 0 {w} {h}\"{accessible_attributes}>{accessibility}{style}{body}</svg>",
        id = escape(input.id),
        x = number(view_box_x),
        w = number(total_width),
        h = number(chart_height),
    ))
}

/// The slices with their angles, as `d3-shape`'s pie makes them: the slices that are at least one
/// per cent of the total, in order, sharing the full circle between them.
fn pie_arcs(sections: &[PieSection], total: f64) -> Vec<Slice<'_>> {
    let included: Vec<(usize, &PieSection)> = sections
        .iter()
        .enumerate()
        .filter(|(_, section)| (section.value / total) * 100.0 >= 1.0)
        .collect();
    let sum: f64 = included.iter().map(|(_, section)| section.value).sum();
    let k = if sum != 0.0 && !sum.is_nan() {
        2.0 * PI / sum
    } else {
        0.0
    };
    let mut angle = 0.0;
    included
        .into_iter()
        .map(|(index, section)| {
            let start = angle;
            angle += if section.value > 0.0 {
                section.value * k
            } else {
                0.0
            };
            Slice {
                section,
                index,
                start,
                end: angle,
            }
        })
        .collect()
}

/// The class of a slice's colour slot: the slot is the position among the sections, wrapping around
/// after twelve colours.
fn slot_class(index: usize) -> String {
    format!("pie-slot-{}", index % SLICE_COLOURS)
}

/// The colour of the slot of a slice: `pie1` to `pie12`, wrapping around after twelve colours.
fn slot_colour(tokens: &ThemeTokens, index: usize) -> &Token {
    tokens.pie.get(index % SLICE_COLOURS + 1)
}

/// The presentation attribute of a colour token, with a leading space, or nothing when the theme
/// does not set the token.
fn attribute(name: &str, token: &Token) -> String {
    token
        .text()
        .map(|colour| format!(" {name}=\"{}\"", escape(&colour)))
        .unwrap_or_default()
}

/// The CSS custom property declarations of the colour tokens of a set of tokens.
fn declarations(tokens: &ThemeTokens) -> String {
    colour_tokens(tokens)
        .iter()
        .filter_map(|(name, token)| {
            token
                .text()
                .map(|colour| format!("--{name}: {};", css_text(&colour)))
        })
        .collect()
}

/// Every colour token with its name: the five text and stroke colours, and the twelve slice colours.
fn colour_tokens(tokens: &ThemeTokens) -> Vec<(String, &Token)> {
    [
        ("pieStrokeColor", &tokens.pie_stroke_color),
        ("pieOuterStrokeColor", &tokens.pie_outer_stroke_color),
        ("pieTitleTextColor", &tokens.pie_title_text_color),
        ("pieSectionTextColor", &tokens.pie_section_text_color),
        ("pieLegendTextColor", &tokens.pie_legend_text_color),
    ]
    .into_iter()
    .map(|(name, token)| (name.to_owned(), token))
    .chain((1..=SLICE_COLOURS).map(|index| (tokens.pie.name(index), tokens.pie.get(index))))
    .collect()
}

/// The `<style>` element: the colour tokens of the themes, the diagram's rules as `pieStyles.ts`
/// writes them with the colours as `var(…)` references, and the embedded font faces. Every rule is
/// scoped to the diagram's id.
fn style_element(
    id: &str,
    theme: ThemeView<'_>,
    family: &str,
    families: &[EmbeddedFamily],
) -> String {
    let tokens = theme.light;
    let declared = match theme.dark {
        None => format!("#{id}{{{}}}", declarations(tokens)),
        Some(dark) => format!(
            "#{id}{{{}}}@media (prefers-color-scheme: dark){{#{id}{{{}}}}}",
            declarations(tokens),
            declarations(dark)
        ),
    };
    let stroke_width = tokens.pie_stroke_width.text().unwrap_or_default();
    let opacity = tokens.pie_opacity.text().unwrap_or_default();
    let outer_stroke_width = tokens.pie_outer_stroke_width.text().unwrap_or_default();
    let title_size = tokens.pie_title_text_size.text().unwrap_or_default();
    let section_size = tokens.pie_section_text_size.text().unwrap_or_default();
    let legend_size = tokens.pie_legend_text_size.text().unwrap_or_default();
    let slots: String = (0..SLICE_COLOURS)
        .map(|slot| {
            format!(
                "#{id} .pieCircle.pie-slot-{slot}{{fill: var(--pie{});}}#{id} .legend rect.pie-slot-{slot}{{fill: var(--pie{}); stroke: var(--pie{});}}",
                slot + 1,
                slot + 1,
                slot + 1
            )
        })
        .collect();
    let faces = if families.is_empty() {
        String::new()
    } else {
        font_faces(families)
    };
    let rules = format!(
        "#{id} .pieCircle{{stroke: var(--pieStrokeColor);stroke-width: {stroke_width};opacity: {opacity};}}\
#{id} .pieCircle.highlighted{{scale: 1.05;opacity: 1;}}\
#{id} .pieCircle.highlightedOnHover:hover{{transition-duration: 250ms;scale: 1.05;opacity: 1;}}\
#{id} .pieOuterCircle{{stroke: var(--pieOuterStrokeColor);stroke-width: {outer_stroke_width};fill: none;}}\
#{id} .pieTitleText{{text-anchor: start;font-size: {title_size};fill: var(--pieTitleTextColor);font-family: {family};}}\
#{id} .slice{{font-family: {family};fill: var(--pieSectionTextColor);font-size: {section_size};}}\
#{id} .legend text{{fill: var(--pieLegendTextColor);font-family: {family};font-size: {legend_size};}}\
#{id} text{{white-space: pre;}}"
    );
    format!(
        "<style>{}{declared}{rules}{slots}</style>",
        css_text(&faces)
    )
}

/// A length rounded to the browser's layout unit of 1/64 px. A width that is not on that grid makes
/// the browser scale the drawing by a fraction, and it then lays the text out at the scaled size,
/// which moves the text's width by a few hundredths of a pixel. The reference's widths are on the
/// grid, because the browser measured them.
fn layout_unit(length: f64) -> f64 {
    (length * 64.0).round() / 64.0
}

/// The percentage of a slice, as the reference's `toFixed(0)` writes it.
fn to_fixed0(percent: f64) -> String {
    if percent.is_nan() {
        return "NaN".to_owned();
    }
    number((percent + 0.5).floor())
}

/// The text of a slice's label: its share of the total in whole per cent.
fn slice_text(section: &PieSection, total: f64) -> String {
    format!("{}%", to_fixed0(section.value / total * 100.0))
}
