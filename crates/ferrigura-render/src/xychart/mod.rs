//! The xychart diagram: `xychartRenderer.ts` of the reference, over the chart builder in
//! [`builder`]. The research is in `docs-port/xychart/NOTES.md`, section 6, and the quirks it lists
//! (R1 to R22) are reproduced where the reference writes them, except R20: the typed configuration
//! reads a numeric string such as a `titlePadding` of `"5"` as a number, which the reference joins.

mod builder;
mod config;

use ferrigura_model::config::XYChartConfig;
use ferrigura_model::xychart::XyChartDiagram;
use ferrigura_text::{EmbeddedFamily, FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

use crate::RenderError;
use crate::a11y;
use crate::common::d3::js_number;
use crate::common::style::base_style_sheet_tokens;
use crate::common::viewbox::configure_svg_size;
use crate::common::{Element, decode_entities};
use crate::context::Context;
use crate::svg::{escape, font_faces};
use builder::interfaces::{Drawable, HorizontalPos, PathElem, RectElem, TextElem, VerticalPos};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
/// The width of a character as a share of the font size, which the reference estimates the label's
/// width with (`charWidthFactor`).
const CHAR_WIDTH_FACTOR: f64 = 0.7;
/// The space between a horizontal bar's end and its label (`rightMargin`).
const RIGHT_MARGIN: f64 = 10.0;
/// The space between a vertical bar's top and its label (`yOffset`).
const Y_OFFSET: f64 = 10.0;

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if the chart has no plot, if a bar's label has no value to read (R1), or if the
/// bundled font cannot be read.
pub fn render(input: &Context<'_, XyChartDiagram, XYChartConfig>) -> Result<String, RenderError> {
    let diagram = input.diagram;
    let tokens = input.theme.light;
    let config = config::chart_config(input.section, diagram.orientation == "horizontal");
    let theme = config::theme_config(tokens.xy_chart.as_ref());
    let data = config::chart_data(diagram);

    // `labelData` is read before the chart is built (`xychartRenderer.ts` 17). With no plot, the
    // reference fails there.
    let first = data.plots.first().ok_or_else(|| {
        RenderError::Layout("Cannot read properties of undefined (reading 'data')".to_owned())
    })?;
    let label_data: Vec<Option<f64>> = first.data().iter().map(|point| point.y).collect();

    let shapes = builder::build(&config, &data, &theme)?;

    let mut main = vec![Node::Element(
        Element::new("rect")
            .with("width", js_number(config.width))
            .with("height", js_number(config.height))
            .with("class", "background")
            .with("fill", theme.background_color.clone()),
    )];
    let mut texts: Vec<String> = Vec::new();
    for shape in &shapes {
        if shape.is_empty() {
            continue;
        }
        let parent = group_children(&mut main, shape.groups());
        match shape {
            Drawable::Rect { data: bars, .. } => {
                for rect in bars {
                    parent.push(Node::Element(rect_element(rect)));
                }
                if config.show_data_label {
                    for label in data_labels(bars, &label_data, &config, &theme)? {
                        texts.push(label.text.clone());
                        parent.push(Node::Element(label.element()));
                    }
                }
            }
            Drawable::Text { data: items, .. } => {
                for text in items {
                    texts.push(decode_entities(&text.text));
                    parent.push(Node::Element(text_element(text)));
                }
            }
            Drawable::Path { data: paths, .. } => {
                for path in paths {
                    parent.push(Node::Element(path_element(path)));
                }
            }
        }
    }

    // The fonts: every character the chart draws, in the sans face (regular).
    let drawn: String = texts.concat();
    let runs = [Run {
        text: &drawn,
        face: face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
    }];
    let families: Vec<EmbeddedFamily> = embed_runs(&runs)?;
    let font_family = families.first().map_or_else(
        || {
            tokens
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned())
        },
        |family| family.css_family.clone(),
    );
    let font_size = tokens.font_size.text().unwrap_or_else(|| "16px".to_owned());
    let style = base_style_sheet_tokens(
        input.id,
        input.theme,
        &font_family,
        &font_size,
        &families,
        "",
    );
    let faces = if families.is_empty() {
        String::new()
    } else {
        font_faces(&families)
    };

    let title = decode_entities(&diagram.acc_title);
    let description = decode_entities(&diagram.acc_descr);
    let mut root = Element::new("svg")
        .with("id", input.id)
        .with("xmlns", SVG_NAMESPACE);
    // `configureSvgSize(svg, height, width, true)`: the width is a percentage, the size is the style.
    configure_svg_size(&mut root, config.height, config.width, true);
    root.set(
        "viewBox",
        format!(
            "0 0 {} {}",
            js_number(config.width),
            js_number(config.height)
        ),
    );
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "xychart");
    if !description.is_empty() {
        root.set("aria-describedby", a11y::description_id(input.id));
    }
    if !title.is_empty() {
        root.set("aria-labelledby", a11y::title_id(input.id));
        root.push_child(
            Element::new("title")
                .with("id", a11y::title_id(input.id))
                .raw_text(markup(&title)),
        );
    }
    if !description.is_empty() {
        root.push_child(
            Element::new("desc")
                .with("id", a11y::description_id(input.id))
                .raw_text(markup(&description)),
        );
    }
    root.push_child(Element::new("style").raw_text(format!("{style}{faces}")));
    // The empty group `mermaidAPI` appends before the renderer draws (`mermaidAPI.ts:419`).
    root.push_child(Element::new("g"));
    let mut main_element = Element::new("g").with("class", "main");
    for node in main {
        main_element.push_child(node.into_element());
    }
    root.push_child(main_element);
    // The group the measurement appends to, which the reference leaves empty (R18).
    root.push_child(Element::new("g").with("class", "mermaid-tmp-group"));
    Ok(root.render())
}

/// A node of the drawing: a group that holds more nodes, or an element. The groups are the classes
/// of `getGroup`: a group is created with its first element and holds the elements that follow.
enum Node {
    Group { class: String, children: Vec<Node> },
    Element(Element),
}

impl Node {
    fn into_element(self) -> Element {
        match self {
            Self::Group { class, children } => {
                let mut group = Element::new("g").with("class", class);
                for child in children {
                    group.push_child(child.into_element());
                }
                group
            }
            Self::Element(element) => element,
        }
    }
}

/// `getGroup(groupTexts)`: the group at the end of the classes, created where it is missing. The
/// children of a group are the nodes that the shapes append to it, in order.
fn group_children<'a>(children: &'a mut Vec<Node>, classes: &[String]) -> &'a mut Vec<Node> {
    let mut current = children;
    for class in classes {
        let index = match current
            .iter()
            .position(|node| matches!(node, Node::Group { class: name, .. } if name == class))
        {
            Some(index) => index,
            None => {
                current.push(Node::Group {
                    class: class.clone(),
                    children: Vec::new(),
                });
                current.len() - 1
            }
        };
        current = match &mut current[index] {
            Node::Group { children, .. } => children,
            Node::Element(_) => unreachable!("a group is never an element"),
        };
    }
    current
}

/// A rectangle as the reference writes it: `x`, `y` (when the value is there), `width`, `height`,
/// `fill`, `stroke` and `stroke-width`.
fn rect_element(rect: &RectElem) -> Element {
    let mut element = Element::new("rect").with("x", js_number(rect.x));
    if let Some(y) = rect.y {
        element.set("y", js_number(y));
    }
    element
        .with("width", js_number(rect.width))
        .with("height", js_number(rect.height))
        .with("fill", rect.fill.clone())
        .with("stroke", rect.stroke_fill.clone())
        .with("stroke-width", js_number(rect.stroke_width))
}

/// A path: its `d`, its fill (`none` when it has none), its stroke and its width.
fn path_element(path: &PathElem) -> Element {
    Element::new("path")
        .with("d", path.path.clone())
        .with(
            "fill",
            path.fill.clone().unwrap_or_else(|| "none".to_owned()),
        )
        .with("stroke", path.stroke_fill.clone())
        .with("stroke-width", js_number(path.stroke_width))
}

/// The markup of a text: the text escaped as the DOM serialises it, then the placeholders of
/// `encodeEntities` decoded on the markup, as the reference decodes its final SVG (`utils.ts`
/// 904-906). A placeholder of `#name;` therefore reads as an entity in the browser.
fn markup(text: &str) -> String {
    decode_entities(&escape(text))
}

/// A text, with its position in the transform (`translate(x, y) rotate(r)`), as the reference
/// writes it.
fn text_element(text: &TextElem) -> Element {
    let rotation = if text.rotation.is_nan() || text.rotation == 0.0 {
        0.0
    } else {
        text.rotation
    };
    let baseline = match text.vertical_pos {
        VerticalPos::Top => "text-before-edge",
        VerticalPos::Middle => "middle",
    };
    let anchor = match text.horizontal_pos {
        HorizontalPos::Left => "start",
        HorizontalPos::Right => "end",
        HorizontalPos::Center => "middle",
    };
    Element::new("text")
        .with("x", "0")
        .with("y", "0")
        .with("fill", text.fill.clone())
        .with("font-size", js_number(text.font_size))
        .with("dominant-baseline", baseline)
        .with("text-anchor", anchor)
        .with(
            "transform",
            format!(
                "translate({}, {}) rotate({})",
                js_number(text.x),
                js_number(text.y),
                js_number(rotation)
            ),
        )
        .raw_text(markup(&text.text))
}

/// A data label of a bar: its text, its anchor, its baseline and its font size, all as the
/// reference writes them on the `text` element.
struct DataLabel {
    text: String,
    x: f64,
    y: f64,
    anchor: &'static str,
    baseline: &'static str,
    fill: String,
    font_size: f64,
}

impl DataLabel {
    fn element(&self) -> Element {
        Element::new("text")
            .with("x", js_number(self.x))
            .with("y", js_number(self.y))
            .with("text-anchor", self.anchor)
            .with("dominant-baseline", self.baseline)
            .with("fill", self.fill.clone())
            .with("font-size", format!("{}px", js_number(self.font_size)))
            .raw_text(markup(&self.text))
    }
}

/// The data labels of a bar shape (`xychartRenderer.ts` 102-227): each bar that has a width and a
/// height gets its label, at one font size for all of them. A rectangle of a legend marker takes the
/// same labels, as the reference's `rect` case does.
fn data_labels(
    bars: &[RectElem],
    label_data: &[Option<f64>],
    config: &builder::interfaces::ChartConfig,
    theme: &builder::interfaces::ThemeConfig,
) -> Result<Vec<DataLabel>, RenderError> {
    // `labelData[i].toString()` for every bar, before the filter.
    let mut items: Vec<(&RectElem, String)> = Vec::with_capacity(bars.len());
    for (index, rect) in bars.iter().enumerate() {
        let value = label_data.get(index).copied().flatten().ok_or_else(|| {
            RenderError::Layout(
                "Cannot read properties of undefined (reading 'toString')".to_owned(),
            )
        })?;
        items.push((rect, js_number(value)));
    }
    let valid: Vec<(&RectElem, String)> = items
        .into_iter()
        .filter(|(rect, _)| rect.width > 0.0 && rect.height > 0.0)
        .collect();
    if valid.is_empty() {
        return Ok(Vec::new());
    }

    let outside = config.show_data_label_outside_bar;
    let fill = theme.data_label_color.clone();
    if config.horizontal {
        let candidates: Vec<f64> = valid
            .iter()
            .map(|(rect, label)| {
                let mut font = rect.height * 0.7;
                while !fits_horizontally(rect, label, font) && font > 0.0 {
                    font -= 1.0;
                }
                font
            })
            .collect();
        let uniform = js_floor(candidates.iter().copied().fold(f64::INFINITY, f64::min));
        Ok(valid
            .iter()
            .map(|(rect, label)| DataLabel {
                text: label.clone(),
                x: if outside {
                    rect.x + rect.width + RIGHT_MARGIN
                } else {
                    rect.x + rect.width - RIGHT_MARGIN
                },
                y: rect.y.unwrap_or(f64::NAN) + rect.height / 2.0,
                anchor: if outside { "start" } else { "end" },
                baseline: "middle",
                fill: fill.clone(),
                font_size: uniform,
            })
            .collect())
    } else {
        let candidates: Vec<f64> = valid
            .iter()
            .map(|(rect, label)| {
                let mut font = rect.width / (utf16_len(label) * CHAR_WIDTH_FACTOR);
                while !fits_in_bar(rect, label, font) && font > 0.0 {
                    font -= 1.0;
                }
                font
            })
            .collect();
        let uniform = js_floor(candidates.iter().copied().fold(f64::INFINITY, f64::min));
        Ok(valid
            .iter()
            .map(|(rect, label)| DataLabel {
                text: label.clone(),
                x: rect.x + rect.width / 2.0,
                y: rect.y.unwrap_or(f64::NAN) + if outside { -Y_OFFSET } else { Y_OFFSET },
                anchor: "middle",
                baseline: if outside { "auto" } else { "hanging" },
                fill: fill.clone(),
                font_size: uniform,
            })
            .collect())
    }
}

/// `fitsHorizontally`: the label fits in the bar, with a margin on its right.
fn fits_horizontally(rect: &RectElem, label: &str, font: f64) -> bool {
    let text_width = font * utf16_len(label) * CHAR_WIDTH_FACTOR;
    text_width <= rect.width - RIGHT_MARGIN
}

/// `fitsInBar`: the label fits in the bar, across and down.
fn fits_in_bar(rect: &RectElem, label: &str, font: f64) -> bool {
    let text_width = font * utf16_len(label) * CHAR_WIDTH_FACTOR;
    let centre_x = rect.x + rect.width / 2.0;
    let left_edge = centre_x - text_width / 2.0;
    let right_edge = centre_x + text_width / 2.0;
    let horizontal_fits = left_edge >= rect.x && right_edge <= rect.x + rect.width;
    let top = rect.y.unwrap_or(f64::NAN);
    let vertical_fits = top + Y_OFFSET + font <= top + rect.height;
    horizontal_fits && vertical_fits
}

/// `label.length`: the length in UTF-16 code units, as JavaScript counts a string.
fn utf16_len(text: &str) -> f64 {
    text.encode_utf16().count() as f64
}

/// `Math.floor`.
fn js_floor(value: f64) -> f64 {
    value.floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_is_placed_with_a_translate_and_a_rotation() {
        let element = text_element(&TextElem {
            text: "Sales".to_owned(),
            x: 10.5,
            y: 20.0,
            fill: "#000".to_owned(),
            vertical_pos: VerticalPos::Top,
            horizontal_pos: HorizontalPos::Center,
            font_size: 14.0,
            rotation: 270.0,
        });
        let markup = element.render();
        assert!(markup.contains("transform=\"translate(10.5, 20) rotate(270)\""));
        assert!(markup.contains("dominant-baseline=\"text-before-edge\""));
    }
}
