//! The pie diagram: a port of `packages/mermaid/src/diagrams/pie/pieRenderer.ts` and
//! `pieStyles.ts`. The layout follows the reference step by step, so that the geometry can be
//! compared with it (`tools/port/`); the text widths come from `ferrigura-text`, where the reference
//! measures the DOM.

use std::f64::consts::PI;

use ferrigura_model::config::PieDiagramConfig;
use ferrigura_model::pie::{PieDiagram, PieSection};
use ferrigura_model::value::Value;
use ferrigura_text::{FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure};

use crate::RenderError;
use crate::svg::{arc_path, escape, font_faces, number};

const MARGIN: f64 = 40.0;
const LEGEND_RECT_SIZE: f64 = 18.0;
const LEGEND_SPACING: f64 = 4.0;
const HEIGHT: f64 = 450.0;
const PIE_WIDTH: f64 = HEIGHT;

/// What the pie renderer draws: the model, the pie configuration and the theme's variables.
pub struct PieInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a PieDiagram,
    /// The pie section of the effective configuration.
    pub config: &'a PieDiagramConfig,
    /// The `themeVariables` object of the theme.
    pub theme: &'a Value,
    /// The id of the SVG element.
    pub id: &'a str,
}

/// One slice of the pie: its section, its start and end angles, and whether it is drawn.
struct Slice<'a> {
    section: &'a PieSection,
    start: f64,
    end: f64,
}

/// Renders a pie diagram as an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset.
pub fn render(input: &PieInput<'_>) -> Result<String, RenderError> {
    let theme = Theme {
        variables: input.theme,
    };
    let config = input.config;
    let sections = &input.diagram.sections;
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);

    let outer_stroke_width = theme.px("pieOuterStrokeWidth").unwrap_or(2.0);
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
    let colors: Vec<Option<String>> = (0..12)
        .map(|index| theme.text(&format!("pie{}", index + 1)))
        .collect();
    let color_of = |label: &str| -> Option<String> {
        let index = sections.iter().position(|section| section.label == label)?;
        colors[index % colors.len()].clone()
    };
    let visible: Vec<&Slice<'_>> = arcs
        .iter()
        .filter(|arc| to_fixed0(arc.section.value / total * 100.0) != "0")
        .collect();

    let section_size = theme.px("pieSectionTextSize").unwrap_or(16.0);
    let legend_size = theme.px("pieLegendTextSize").unwrap_or(16.0);
    let title_size = theme.px("pieTitleTextSize").unwrap_or(25.0);

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
    let mut longest_text_width: f64 = 0.0;
    for label in &legend_labels {
        longest_text_width = longest_text_width.max(measure(sans, label, legend_size)?);
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
    let title_width = if title.is_empty() {
        0.0
    } else {
        measure(sans, title, title_size)?
    };
    let title_left = PIE_WIDTH / 2.0 - title_width / 2.0;
    let title_right = PIE_WIDTH / 2.0 + title_width / 2.0;
    let view_box_x = 0.0_f64.min(title_left);
    let view_box_right = chart_width.max(title_right);
    let total_width = view_box_right - view_box_x;

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
    for label in &legend_labels {
        runs.push(Run {
            text: label,
            face: sans,
        });
    }
    for label in &slice_labels {
        runs.push(Run {
            text: label,
            face: sans,
        });
    }
    let families = embed_runs(&runs)?;
    let family_name = families
        .first()
        .map_or_else(|| "sans-serif".to_owned(), |family| family.name.clone());

    let mut body = String::new();
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
        "<circle class=\"pieOuterCircle\" cx=\"0\" cy=\"0\" r=\"{}\"/>",
        number(radius + outer_stroke_width / 2.0)
    ));
    for arc in &visible {
        let class = slice_class(input.config, arc.section);
        let fill = color_of(&arc.section.label)
            .map(|color| format!(" fill=\"{}\"", escape(&color)))
            .unwrap_or_default();
        body.push_str(&format!(
            "<path class=\"{class}\" d=\"{}\"{fill}/>",
            arc_path(inner_radius, radius, arc.start, arc.end)
        ));
    }
    for (arc, text) in visible.iter().zip(&slice_labels) {
        let middle = (arc.start + arc.end) / 2.0 - PI / 2.0;
        let (cx, cy) = (label_radius * middle.cos(), label_radius * middle.sin());
        let width = measure(sans, text, section_size)?;
        body.push_str(&format!(
            "<text class=\"slice\" x=\"{}\" y=\"{}\">{}</text>",
            number(cx - width / 2.0),
            number(cy),
            escape(text)
        ));
    }
    body.push_str("</g>");

    let title_x = -title_width / 2.0;
    body.push_str(&format!(
        "<text class=\"pieTitleText\" x=\"{}\" y=\"{}\">{}</text>",
        number(title_x),
        number(-(HEIGHT - 50.0) / 2.0),
        escape(title)
    ));

    for (index, section) in sections.iter().enumerate() {
        let color = color_of(&section.label)
            .map(|color| escape(&color))
            .unwrap_or_default();
        body.push_str(&format!(
            "<g class=\"legend\" transform=\"{}\"><rect width=\"{}\" height=\"{}\" style=\"fill: {color}; stroke: {color};\"/><text x=\"{}\" y=\"{}\">{}</text></g>",
            legend_transform(index),
            number(LEGEND_RECT_SIZE),
            number(LEGEND_RECT_SIZE),
            number(LEGEND_RECT_SIZE + LEGEND_SPACING),
            number(LEGEND_RECT_SIZE - LEGEND_SPACING),
            escape(&legend_labels[index]),
        ));
    }
    body.push_str("</g>");

    let style = styles(&theme, &family_name, &families);
    let size_attributes = if input.config.use_max_width {
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
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"{id}\" {size_attributes} viewBox=\"{x} 0 {w} {h}\" role=\"graphics-document document\" aria-roledescription=\"pie\"><style>{style}</style>{body}</svg>",
        id = escape(input.id),
        x = number(view_box_x),
        w = number(total_width),
        h = number(chart_height),
    ))
}

/// The slices with their angles, as `d3-shape`'s pie makes them: the slices that are at least one
/// per cent of the total, in order, sharing the full circle between them.
fn pie_arcs(sections: &[PieSection], total: f64) -> Vec<Slice<'_>> {
    let included: Vec<&PieSection> = sections
        .iter()
        .filter(|section| (section.value / total) * 100.0 >= 1.0)
        .collect();
    let sum: f64 = included.iter().map(|section| section.value).sum();
    let k = if sum != 0.0 && !sum.is_nan() {
        2.0 * PI / sum
    } else {
        0.0
    };
    let mut angle = 0.0;
    included
        .into_iter()
        .map(|section| {
            let start = angle;
            angle += if section.value > 0.0 {
                section.value * k
            } else {
                0.0
            };
            Slice {
                section,
                start,
                end: angle,
            }
        })
        .collect()
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

fn slice_class(config: &PieDiagramConfig, section: &PieSection) -> String {
    let mut class = "pieCircle".to_owned();
    if config.highlight_slice == "hover" {
        class.push_str(" highlightedOnHover");
    } else if config.highlight_slice == section.label {
        class.push_str(" highlighted");
    }
    class
}

/// The CSS of the pie, as `pieStyles.ts` writes it, with the embedded font family.
fn styles(theme: &Theme<'_>, family: &str, families: &[ferrigura_text::EmbeddedFamily]) -> String {
    let stroke = theme.text("pieStrokeColor").unwrap_or_default();
    let stroke_width = theme.text("pieStrokeWidth").unwrap_or_default();
    let opacity = theme.text("pieOpacity").unwrap_or_default();
    let outer_stroke = theme.text("pieOuterStrokeColor").unwrap_or_default();
    let outer_stroke_width = theme.text("pieOuterStrokeWidth").unwrap_or_default();
    let title_size = theme.text("pieTitleTextSize").unwrap_or_default();
    let title_color = theme.text("pieTitleTextColor").unwrap_or_default();
    let section_color = theme.text("pieSectionTextColor").unwrap_or_default();
    let section_size = theme.text("pieSectionTextSize").unwrap_or_default();
    let legend_color = theme.text("pieLegendTextColor").unwrap_or_default();
    let legend_size = theme.text("pieLegendTextSize").unwrap_or_default();
    format!(
        "{faces}\
.pieCircle{{stroke: {stroke};stroke-width: {stroke_width};opacity: {opacity};}}\
.pieCircle.highlighted{{scale: 1.05;opacity: 1;}}\
.pieCircle.highlightedOnHover:hover{{transition-duration: 250ms;scale: 1.05;opacity: 1;}}\
.pieOuterCircle{{stroke: {outer_stroke};stroke-width: {outer_stroke_width};fill: none;}}\
.pieTitleText{{text-anchor: start;font-size: {title_size};fill: {title_color};font-family: \"{family}\";}}\
.slice{{font-family: \"{family}\";fill: {section_color};font-size: {section_size};}}\
.legend text{{fill: {legend_color};font-family: \"{family}\";font-size: {legend_size};}}\
text{{white-space: pre;}}",
        faces = if families.is_empty() {
            String::new()
        } else {
            font_faces(families)
        },
    )
}

/// The `themeVariables` of the diagram, read as the renderer reads them.
struct Theme<'a> {
    variables: &'a Value,
}

impl Theme<'_> {
    /// A variable as text: a string as it is, a number as JavaScript prints it.
    fn text(&self, name: &str) -> Option<String> {
        match self.variables.get(name)? {
            Value::String(text) => Some(text.clone()),
            Value::Number(number_value) => Some(number(*number_value)),
            _ => None,
        }
    }

    /// The leading pixel count of a font size or stroke width, as `parseFontSize` reads it.
    fn px(&self, name: &str) -> Option<f64> {
        leading_integer(&self.text(name)?)
    }
}

/// `parseInt(text, 10)` for the leading digits of a size such as `25px`.
fn leading_integer(text: &str) -> Option<f64> {
    let trimmed = text.trim_start();
    let digits: String = trimmed
        .char_indices()
        .take_while(|(index, c)| c.is_ascii_digit() || (*index == 0 && (*c == '-' || *c == '+')))
        .map(|(_, c)| c)
        .collect();
    digits.parse::<f64>().ok().filter(|value| value.is_finite())
}
