//! The text of the sequence diagram: the measurement the layout reads, and the two ways the
//! reference places a label: `drawText` of `svgDraw.js` (messages, notes and blocks) and the
//! `byTspan` placement of the actors' and the boxes' labels (`_drawTextCandidateFunc`).
//!
//! The layout reads text sizes through the shared [`TextMemo`] (`calculateTextDimensions`), and
//! the heights of single lines as `getBBox` gives them: the box of the font's rounded ascent and
//! descent (`ferrigura_text::text_box`). Chromium measures the drawn text itself, so the markup
//! only has to carry the attributes the reference sets.

use ferrigura_text::{Face, FontFamily, FontStyle, FontWeight, face, text_box};

use super::conf::Font;
use crate::RenderError;
use crate::common::text::{MeasureError, TextDimensions, split_line_breaks};
use crate::common::{Element, TextMemo, WrapStyle};
use crate::svg::number;

/// `ZERO_WIDTH_SPACE` (`utils.ts`): what an empty line is drawn with.
pub const ZERO_WIDTH_SPACE: &str = "\u{200b}";

/// The line break the reference joins wrapped text with (`wrapLabel`'s `joinWith`).
pub const JOIN_WITH: &str = "<br/>";

/// Turns a measurement error into the renderer's error.
pub fn measure_error(error: MeasureError) -> RenderError {
    match error {
        MeasureError::NotInRenderTree => {
            RenderError::Layout("svg element not in render tree".to_owned())
        }
        MeasureError::Text(error) => RenderError::Text(error),
    }
}

/// The face a font of the diagram is set in: the bundled sans, bold from weight 500 up.
#[must_use]
pub fn face_of(font: &Font) -> &'static Face {
    let weight = if font.weight > 500.0 {
        FontWeight::Bold
    } else {
        FontWeight::Normal
    };
    face(FontFamily::Sans, weight, FontStyle::Normal)
}

/// The height of one line of a font: the `getBBox().height` of a `<text>` with that line.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn line_height(font: &Font) -> Result<f64, RenderError> {
    Ok(text_box(face_of(font), font.size)?.height)
}

/// `calculateTextDimensions(text, font)`: the memoised measurement of a text.
///
/// # Errors
///
/// See [`measure_error`].
pub fn dimensions(
    memo: &mut TextMemo,
    text: &str,
    font: &Font,
) -> Result<TextDimensions, RenderError> {
    memo.dimensions(text, &font.text_style()).map_err(measure_error)
}

/// `wrapLabel(label, maxWidth, font)`: the label broken into lines, joined with `<br/>`.
///
/// # Errors
///
/// See [`measure_error`].
pub fn wrap(
    memo: &mut TextMemo,
    label: &str,
    max_width: f64,
    font: &Font,
) -> Result<String, RenderError> {
    let style = WrapStyle {
        text: font.text_style(),
        join_with: JOIN_WITH.to_owned(),
    };
    memo.wrap_label(label, max_width, &style).map_err(measure_error)
}

/// `common.splitBreaks(text).length`: the number of lines of a text.
#[must_use]
pub fn line_count(text: &str) -> usize {
    split_line_breaks(text).len()
}

/// The style of a text element: the attributes `drawText` gives it, in the order it sets them.
/// Font properties go into one `style` attribute, as d3's `style` calls do.
fn font_style(font: &Font) -> String {
    format!(
        "font-family: {}; font-size: {}; font-weight: {};",
        font.family,
        font.size_css(),
        number(font.weight)
    )
}

/// The options of `drawText` (`svgDraw.js`, the `textData` of the reference): what the reference
/// reads from the text object it is given.
#[derive(Debug, Clone)]
pub struct TextData<'a> {
    /// The text. Its `<br>` tags are lines.
    pub text: &'a str,
    /// The x of the text; moved by the anchor when `width` is set.
    pub x: f64,
    /// The y of the first line; moved by the vertical alignment.
    pub y: f64,
    /// The width the anchor is measured against, when the text has one.
    pub width: Option<f64>,
    /// The anchor: `start`/`left`, `middle`/`center`, `end`/`right`.
    pub anchor: Option<&'a str>,
    /// The vertical alignment: `top`/`start`, `middle`/`center`, `bottom`/`end`.
    pub valign: Option<&'a str>,
    /// The margin: the anchor's inset and the lines' spacing.
    pub text_margin: Option<f64>,
    /// The `dy` of each line (`1em`), when set.
    pub dy: Option<&'a str>,
    /// The class of the text.
    pub class: &'a str,
    /// Whether each line is written in a `tspan` at the text's x.
    pub tspan: bool,
    /// The font.
    pub font: &'a Font,
}

/// `drawText(elem, textData)`: one `<text>` per line of the text, placed as the reference places
/// them. Returns the elements, in order, with the heights each line added to the text's height.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn draw_text(data: &TextData<'_>) -> Result<Vec<Element>, RenderError> {
    let lines = split_line_breaks(data.text);
    let line_size = data.font.size;
    let mut x = data.x;
    let mut anchor = data.anchor.map(str::to_owned);
    let mut dominant = None;
    if let (Some(name), Some(margin), Some(width)) = (data.anchor, data.text_margin, data.width) {
        let shifted = match name {
            "left" | "start" => Some((js_round(x + margin), "start")),
            "middle" | "center" => Some((js_round(x + width / 2.0), "middle")),
            "right" | "end" => Some((js_round(x + width - margin), "end")),
            _ => None,
        };
        if let Some((new_x, new_anchor)) = shifted {
            x = new_x;
            anchor = Some(new_anchor.to_owned());
            dominant = Some("middle");
        }
    }
    let margin = data.text_margin.filter(|margin| *margin > 0.0);
    // `valign` only moves the lines when the text has a margin above zero.
    let aligned = data.valign.is_some() && margin.is_some();
    let mut text_height = 0.0_f64;
    let mut elements = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        let dy = match data.text_margin {
            Some(margin) if margin == 0.0 => index as f64 * line_size,
            _ => 0.0,
        };
        let y = match (data.valign, margin) {
            (Some("top" | "start"), Some(margin)) => js_round(data.y + margin),
            (Some("middle" | "center"), Some(margin)) => {
                js_round(data.y + (text_height + text_height + margin) / 2.0)
            }
            (Some("bottom" | "end"), Some(margin)) => {
                js_round(data.y + (text_height + text_height + 2.0 * margin) - margin)
            }
            _ => data.y,
        };
        let mut element = Element::new("text").with("x", number(x)).with("y", number(y));
        if let Some(anchor) = &anchor {
            element.set("text-anchor", anchor.as_str());
            // The baseline is set only when the anchor was mapped (`dominantBaseline` in the
            // reference), so the attribute is absent for any other anchor.
            if let Some(baseline) = dominant {
                element.set("dominant-baseline", baseline);
                element.set("alignment-baseline", baseline);
            }
        }
        element.set("style", font_style(data.font));
        element.set("class", data.class);
        if let Some(dy) = data.dy {
            element.set("dy", dy);
        } else if dy != 0.0 {
            element.set("dy", number(dy));
        }
        let text = if line.is_empty() {
            ZERO_WIDTH_SPACE
        } else {
            line
        };
        if data.tspan {
            element.push_child(Element::new("tspan").with("x", number(x)).text(text));
        } else {
            element = element.text(text);
        }
        if aligned {
            text_height += line_height(data.font)?;
        }
        elements.push(element);
    }
    Ok(elements)
}

/// `Math.round` for the text's coordinates (see [`super::bounds::js_round`]).
fn js_round(value: f64) -> f64 {
    super::bounds::js_round(value)
}

/// The label of an actor, box or other shape (`byTspan` of `_drawTextCandidateFunc`): one `<text>`
/// per line, each centred on `x`, on a line centred vertically in the band. Every line is a
/// `tspan` shifted by its place among the lines.
#[must_use]
pub fn draw_label_by_tspan(
    content: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    class: &str,
    font: &Font,
) -> Vec<Element> {
    let lines = split_line_breaks(content);
    let size = font.size;
    let count = lines.len() as f64;
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let dy = index as f64 * size - (size * (count - 1.0)) / 2.0;
            let centre = x + width / 2.0;
            Element::new("text")
                .with("x", number(centre))
                .with("y", number(y))
                .with(
                    "style",
                    format!(
                        "text-anchor: middle; font-size: {}; font-weight: {}; font-family: {};",
                        font.size_css(),
                        number(font.weight),
                        font.family
                    ),
                )
                .child(
                    Element::new("tspan")
                        .with("x", number(centre))
                        .with("dy", number(dy))
                        .text(*line),
                )
                .with("y", number(y + height / 2.0))
                .with("dominant-baseline", "central")
                .with("alignment-baseline", "central")
                .with("class", class)
        })
        .collect()
}
