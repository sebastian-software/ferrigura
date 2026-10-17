//! The quadrant chart diagram: `quadrantRenderer.ts` of the reference (`draw`), over the chart
//! builder in [`builder`]. The research is in `docs-port/quadrant/NOTES.md`, section 6; the parser
//! and the model are in `docs-port/quadrant/PLAN.md`.
//!
//! The reference's `draw` writes the root `svg`, then five groups in `g.main`, in this order:
//! `quadrants`, `border`, `data-points`, `labels` and `title`, each created even when empty. The
//! attributes of each element are the ones the reference's d3 calls set, and the values are
//! written as JavaScript writes them (`common::d3::js_number`), so a coordinate has the same last
//! digit.
//!
//! The reference's quirks are reproduced where the reference writes them: a coordinate the scale
//! cannot read drops the `cx` or `cy` attribute and writes `undefined` or `NaN` in a transform
//! (NOTES 2.3). A point's default fill is the theme's `quadrantPointFill`, which the redux themes
//! give as `hsl(240, 90%, NaN%)` (NOTES 6.6); that value is copied, not corrected.

pub mod builder;

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::quadrant_chart::QuadrantChartDiagram;
use ferrigura_text::{EmbeddedFamily, FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

use crate::RenderError;
use crate::a11y;
use crate::common::d3::js_number;
use crate::common::style::base_style_sheet;
use crate::common::viewbox::calculate_svg_size_attrs;
use crate::common::{Element, decode_entities};
use crate::svg::{escape, font_faces};
use crate::theme::Themes;
use crate::variables::text_of;
use builder::{Chart, HorizontalPos, Line, Point, Quadrant, Text, Theme, VerticalPos};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// What the quadrant chart renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct QuadrantChartInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a QuadrantChartDiagram,
    /// The effective configuration, in the quadrant chart diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read or subset.
pub fn render(input: &QuadrantChartInput<'_>) -> Result<String, RenderError> {
    let config = &input.config.quadrant_chart;
    let variables = input.themes.primary();
    let theme = Theme::new(variables);
    let chart = builder::build(config, input.diagram, &theme);

    // The groups of `g.main` (`quadrantRenderer.ts` 64-68), filled in the reference's order.
    let quadrants = quadrants_group(&chart.quadrants);
    let border = border_group(&chart.borders);
    let data_points = data_points_group(&chart.points);
    let labels = labels_group(&chart.axis_labels);
    let title = title_group(chart.title.as_ref());

    // The fonts: every character the chart shows, in the sans face (regular).
    let drawn = drawn_text(&chart);
    let runs = [Run {
        text: &drawn,
        face: face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
    }];
    let families: Vec<EmbeddedFamily> = embed_runs(&runs)?;
    let font_family = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| format!("\"{}\"", family.name),
    );
    let font_size = text_of(variables, "fontSize").unwrap_or_else(|| "16px".to_owned());
    // The diagram's own rules are none (`styles.ts` of the type is `() => ''`), so the sheet is
    // the base one.
    let style = base_style_sheet(input.id, input.themes, &font_family, &font_size, &families);
    let faces = if families.is_empty() {
        String::new()
    } else {
        font_faces(&families)
    };

    // The accessible texts keep the placeholders: the whole-SVG pass decodes them in `markup`.
    let title_text = &input.diagram.acc_title;
    let description = &input.diagram.acc_descr;
    let mut root = Element::new("svg").with("id", input.id);
    // `configureSvgSize(svg, height, width, useMaxWidth)` (`quadrantRenderer.ts` 49-52). The
    // reference's root takes the width first, then the namespace, then the other size attribute.
    let sizes = calculate_svg_size_attrs(
        config.chart_height,
        config.chart_width,
        config.use_max_width,
    );
    for (name, value) in sizes.iter().filter(|(name, _)| *name == "width") {
        root.set(name, value.clone());
    }
    root.set("xmlns", SVG_NAMESPACE);
    for (name, value) in sizes.into_iter().filter(|(name, _)| *name != "width") {
        root.set(name, value);
    }
    root.set(
        "viewBox",
        format!(
            "0 0 {} {}",
            js_number(config.chart_width),
            js_number(config.chart_height)
        ),
    );
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "quadrantChart");
    if !description.is_empty() {
        root.set("aria-describedby", a11y::description_id(input.id));
    }
    if !title_text.is_empty() {
        root.set("aria-labelledby", a11y::title_id(input.id));
        root.push_child(
            Element::new("title")
                .with("id", a11y::title_id(input.id))
                .raw_text(markup(title_text)),
        );
    }
    if !description.is_empty() {
        root.push_child(
            Element::new("desc")
                .with("id", a11y::description_id(input.id))
                .raw_text(markup(description)),
        );
    }
    root.push_child(Element::new("style").raw_text(format!("{style}{faces}")));
    // The empty group `mermaidAPI` appends before the renderer draws (`mermaidAPI.ts:419`).
    root.push_child(Element::new("g"));
    root.push_child(
        Element::new("g")
            .with("class", "main")
            .child(quadrants)
            .child(border)
            .child(data_points)
            .child(labels)
            .child(title),
    );
    Ok(root.render())
}

/// The characters every element of the chart shows, for the font subset. The subset takes the
/// characters the reader sees (`'` and `"` among them), not the markup they are written as.
fn drawn_text(chart: &Chart) -> String {
    let mut texts: Vec<&str> = Vec::new();
    for quadrant in &chart.quadrants {
        texts.push(&quadrant.text.text);
    }
    for label in &chart.axis_labels {
        texts.push(&label.text);
    }
    for point in &chart.points {
        texts.push(&point.text.text);
    }
    if let Some(title) = &chart.title {
        texts.push(&title.text);
    }
    texts.iter().map(|text| shown(text)).collect()
}

/// The characters a text shows: the placeholders of `encodeEntities` decoded to the characters
/// they stand for (`#35;` shows `#`), and the other characters as they are.
fn shown(text: &str) -> String {
    let decoded = decode_entities(text);
    let mut out = String::with_capacity(decoded.len());
    let mut rest = decoded.as_str();
    while let Some(start) = rest.find("&#") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find(';') {
            Some(end) if end > 0 && after[..end].bytes().all(|byte| byte.is_ascii_digit()) => {
                out.extend(after[..end].parse::<u32>().ok().and_then(char::from_u32));
                rest = &after[end + 1..];
            }
            _ => {
                out.push_str("&#");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The markup of a text, as the reference's whole-SVG pass leaves it: the characters escaped as the
/// DOM serialises them, then the placeholders of `encodeEntities` decoded (`mermaidAPI.ts` 359).
fn markup(text: &str) -> String {
    decode_entities(&escape(text))
}

/// `g.quadrants`: one `g.quadrant` per quadrant, with its rectangle and its label.
fn quadrants_group(quadrants: &[Quadrant]) -> Element {
    let mut group = Element::new("g").with("class", "quadrants");
    for quadrant in quadrants {
        group.push_child(
            Element::new("g")
                .with("class", "quadrant")
                .child(
                    Element::new("rect")
                        .with("x", js_number(quadrant.x))
                        .with("y", js_number(quadrant.y))
                        .with("width", js_number(quadrant.width))
                        .with("height", js_number(quadrant.height))
                        .with_optional("fill", quadrant.fill.as_deref()),
                )
                .child(text_element(&quadrant.text)),
        );
    }
    group
}

/// `g.border`: one `line` per border, with its stroke as a style.
fn border_group(borders: &[Line]) -> Element {
    let mut group = Element::new("g").with("class", "border");
    for line in borders {
        group.push_child(line_element(line));
    }
    group
}

/// A border line: its end points as attributes, and the stroke and its width as a `style`, which
/// the browser serialises (`quadrantRenderer.ts` 83-95; `d3` `style` sets a CSS declaration).
fn line_element(line: &Line) -> Element {
    let mut declarations = Vec::new();
    if let Some(stroke) = line.stroke.as_deref().and_then(css_colour) {
        declarations.push(format!("stroke: {stroke};"));
    }
    declarations.push(format!("stroke-width: {};", js_number(line.stroke_width)));
    Element::new("line")
        .with("x1", js_number(line.x1))
        .with("y1", js_number(line.y1))
        .with("x2", js_number(line.x2))
        .with("y2", js_number(line.y2))
        .with("style", declarations.join(" "))
}

/// The colour of a `style` declaration as the browser serialises it, or `None` when the browser
/// drops the declaration (an invalid colour, such as an `hsl()` with a `NaN` component). A hex
/// colour (`#rgb` or `#rrggbb`) becomes `rgb(r, g, b)`, and an `hsl()` colour is converted the way
/// Blink does it, in single precision: the lightness of the redux themes' border,
/// `9.2156862745%`, is 23.5 in single precision and so reads as 24 (`rgb(24, 24, 24)`), where
/// double precision reads 23. Any other value is written as it is (NOTES 8.3, question 5).
fn css_colour(value: &str) -> Option<String> {
    if let Some(digits) = value.strip_prefix('#') {
        if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(value.to_owned());
        }
        let nibbles: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(16)).collect();
        let rgb: Vec<u32> = match nibbles.len() {
            3 => nibbles.iter().map(|nibble| nibble * 17).collect(),
            6 => nibbles
                .chunks(2)
                .map(|pair| pair[0] * 16 + pair[1])
                .collect(),
            _ => return Some(value.to_owned()),
        };
        return Some(format!("rgb({}, {}, {})", rgb[0], rgb[1], rgb[2]));
    }
    if let Some(arguments) = value
        .strip_prefix("hsl(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let parts: Vec<&str> = arguments.split(',').map(str::trim).collect();
        let numbers: Option<Vec<f64>> = parts
            .iter()
            .map(|part| {
                let number = part.strip_suffix('%').unwrap_or(part).parse::<f64>().ok()?;
                number.is_finite().then_some(number)
            })
            .collect();
        return match numbers.as_deref() {
            Some([hue, saturation, lightness]) => {
                // The fractions are computed in double precision and then rounded to single.
                let [r, g, b] = hsl_to_rgb(
                    (hue / 360.0) as f32,
                    (saturation / 100.0) as f32,
                    (lightness / 100.0) as f32,
                );
                Some(format!(
                    "rgb({}, {}, {})",
                    channel(r),
                    channel(g),
                    channel(b)
                ))
            }
            _ => None,
        };
    }
    Some(value.to_owned())
}

/// `HSLToRGB` of Blink, in single precision: the hue, the saturation and the lightness as fractions
/// of a turn and of one.
fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> [f32; 3] {
    if saturation == 0.0 {
        return [lightness, lightness, lightness];
    }
    let temp2 = if lightness < 0.5 {
        lightness * (1.0 + saturation)
    } else {
        lightness + saturation - lightness * saturation
    };
    let temp1 = 2.0 * lightness - temp2;
    let channel_of = |hue: f32| {
        let mut hue = hue;
        if hue < 0.0 {
            hue += 1.0;
        }
        if hue > 1.0 {
            hue -= 1.0;
        }
        if 6.0 * hue < 1.0 {
            temp1 + (temp2 - temp1) * 6.0 * hue
        } else if 2.0 * hue < 1.0 {
            temp2
        } else if 3.0 * hue < 2.0 {
            temp1 + (temp2 - temp1) * (2.0 / 3.0 - hue) * 6.0
        } else {
            temp1
        }
    };
    [
        channel_of(hue + 1.0 / 3.0),
        channel_of(hue),
        channel_of(hue - 1.0 / 3.0),
    ]
}

/// A channel of 0 to 1 as an 8-bit value, rounded half away from zero and clamped.
fn channel(value: f32) -> u32 {
    (value * 255.0).round().clamp(0.0, 255.0) as u32
}

/// `g.data-points`: one `g.data-point` per point, with its circle and its label.
fn data_points_group(points: &[Point]) -> Element {
    let mut group = Element::new("g").with("class", "data-points");
    for point in points {
        let mut circle = Element::new("circle");
        // A coordinate the scale cannot read is `undefined`, and d3 removes the attribute.
        if let Some(x) = point.x {
            circle.set("cx", js_number(x));
        }
        if let Some(y) = point.y {
            circle.set("cy", js_number(y));
        }
        circle.set("r", js_number(point.radius));
        circle.set_optional("fill", point.fill.as_deref());
        circle.set_optional("stroke", point.stroke_color.as_deref());
        circle.set("stroke-width", point.stroke_width.clone());
        group.push_child(
            Element::new("g")
                .with("class", "data-point")
                .child(circle)
                .child(text_element(&point.text)),
        );
    }
    group
}

/// `g.labels`: one `g.label` per axis text.
fn labels_group(labels: &[Text]) -> Element {
    let mut group = Element::new("g").with("class", "labels");
    for label in labels {
        group.push_child(
            Element::new("g")
                .with("class", "label")
                .child(text_element(label)),
        );
    }
    group
}

/// `g.title`: the title's text, when the chart has a title.
fn title_group(title: Option<&Text>) -> Element {
    let mut group = Element::new("g").with("class", "title");
    if let Some(title) = title {
        group.push_child(text_element(title));
    }
    group
}

/// A text as `quadrantRenderer.ts` writes it: `x` and `y` at 0, then the fill, the font size, the
/// baseline, the anchor, and the transform, which places it (`getTransformation`).
fn text_element(text: &Text) -> Element {
    let anchor = match text.vertical_pos {
        VerticalPos::Left => "start",
        VerticalPos::Center => "middle",
    };
    let baseline = match text.horizontal_pos {
        HorizontalPos::Top => "hanging",
        HorizontalPos::Middle => "middle",
    };
    // `translate(${x}, ${y}) rotate(${rotation || 0})`: an `undefined` coordinate is the word.
    let transform = format!(
        "translate({}, {}) rotate({})",
        optional_number(text.x),
        optional_number(text.y),
        js_number(text.rotation)
    );
    Element::new("text")
        .with("x", "0")
        .with("y", "0")
        .with_optional("fill", text.fill.as_deref())
        .with("font-size", js_number(text.font_size))
        .with("dominant-baseline", baseline)
        .with("text-anchor", anchor)
        .with("transform", transform)
        .raw_text(markup(&text.text))
}

/// An attribute that is set only when its value is there: d3 removes an attribute given
/// `undefined`, and a theme that gives no colour gives none.
trait OptionalAttribute {
    fn with_optional(self, name: &str, value: Option<&str>) -> Self;
    fn set_optional(&mut self, name: &str, value: Option<&str>);
}

impl OptionalAttribute for Element {
    fn with_optional(mut self, name: &str, value: Option<&str>) -> Self {
        self.set_optional(name, value);
        self
    }

    fn set_optional(&mut self, name: &str, value: Option<&str>) {
        if let Some(value) = value {
            self.set(name, value);
        }
    }
}

/// A coordinate as a JavaScript template string writes it: the number, or `undefined`.
fn optional_number(value: Option<f64>) -> String {
    value.map_or_else(|| "undefined".to_owned(), js_number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hex_colour_is_written_as_the_browser_writes_it_in_a_style() {
        assert_eq!(css_colour("#181818").as_deref(), Some("rgb(24, 24, 24)"));
        assert_eq!(css_colour("#abc").as_deref(), Some("rgb(170, 187, 204)"));
        assert_eq!(css_colour("#FF0000").as_deref(), Some("rgb(255, 0, 0)"));
    }

    #[test]
    fn an_hsl_colour_is_converted_in_single_precision() {
        // The redux border `hsl(247.5, 0%, 9.2156862745%)` reads as 24 in Chromium's style.
        assert_eq!(
            css_colour("hsl(247.5, 0%, 9.2156862745%)").as_deref(),
            Some("rgb(24, 24, 24)")
        );
        assert_eq!(
            css_colour("hsl(0, 100%, 50%)").as_deref(),
            Some("rgb(255, 0, 0)")
        );
    }

    #[test]
    fn a_colour_the_browser_drops_is_not_written() {
        // The point fill of the redux themes is `hsl(240, 90%, NaN%)`: an invalid style value.
        assert_eq!(css_colour("hsl(240, 90%, NaN%)"), None);
    }

    #[test]
    fn a_colour_that_is_not_hex_or_hsl_is_written_as_it_is() {
        assert_eq!(css_colour("red").as_deref(), Some("red"));
        assert_eq!(css_colour("#12").as_deref(), Some("#12"));
    }

    #[test]
    fn an_undefined_coordinate_is_written_as_the_word() {
        assert_eq!(optional_number(None), "undefined");
        assert_eq!(optional_number(Some(f64::NAN)), "NaN");
        assert_eq!(optional_number(Some(0.5)), "0.5");
    }
}
