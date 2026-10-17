//! The legacy texts of the C4 renderer: the labels, types and descriptions of boundaries, and the
//! labels and techn of relations. They are measured by `calcC4ShapeTextWH` (`c4Renderer.ts:204-244`)
//! and drawn by `byTspan` (`svgDraw.ts:361-395`), the placement of C4, which reads no `textPlacement`.

use crate::common::text::{MeasureError, TextMemo, TextStyle, WrapStyle, split_line_breaks};
use crate::svg::{escape, number};
use ferrigura_syntax::decode_entities;

/// `wrapLabel`'s join string: the `<br/>` the wrapped lines are joined with (`utils.ts:539`).
const JOIN: &str = "<br/>";

/// A legacy text after measurement: the text as it is drawn (wrapped, when the wrap applies), its box,
/// and the `Y` the layout gives it (the top of the text, relative to its boundary or relation).
#[derive(Debug, Clone, PartialEq)]
pub struct Legacy {
    /// The text, with `<br/>` between the wrapped lines.
    pub text: String,
    /// The width of the text box.
    pub width: f64,
    /// The height of the text box.
    pub height: f64,
    /// The `Y` of the text, set by the layout before the text is drawn.
    pub y: f64,
}

/// `calcC4ShapeTextWH` on a text that is measured once: the wrapped text and its box when the wrap
/// applies (`textLimitWidth` is the width of the box), otherwise the widest line and the sum of the
/// line heights.
///
/// # Errors
///
/// Returns an error when a line has no glyphs (white space only), as the reference throws.
pub fn measure(
    memo: &mut TextMemo,
    text: &str,
    wrap: bool,
    style: &TextStyle,
    limit: f64,
) -> Result<Legacy, MeasureError> {
    if wrap {
        let wrap_style = WrapStyle {
            text: style.clone(),
            join_with: JOIN.to_owned(),
        };
        let wrapped = memo.wrap_label(text, limit, &wrap_style)?;
        let height = memo.height(&wrapped, style)?;
        return Ok(Legacy {
            text: wrapped,
            width: limit,
            height,
            y: 0.0,
        });
    }
    let mut width = 0.0_f64;
    let mut height = 0.0_f64;
    for line in split_line_breaks(text) {
        width = memo.width(line, style)?.max(width);
        height += memo.height(line, style)?;
    }
    Ok(Legacy {
        text: text.to_owned(),
        width,
        height,
        y: 0.0,
    })
}

/// The attributes that `byTspan` gives every text, besides its placement: the fill, and for the
/// techn the italic style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paint<'a> {
    /// The `fill` of the text.
    pub fill: &'a str,
    /// Whether the text is italic (`font-style="italic"`, the techn of a relation).
    pub italic: bool,
}

/// The font of a legacy text as `byTspan` writes it: the size in pixels, the CSS weight and the CSS
/// family of the embedded subset.
#[derive(Debug, Clone, Copy)]
pub struct Face<'a> {
    /// The size in pixels.
    pub size: f64,
    /// The CSS weight: `normal` or `bold`.
    pub weight: &'a str,
    /// The CSS family.
    pub family: &'a str,
}

/// `byTspan`: one `text` for each line of `content`, centred on `x + width / 2`, each with its
/// `tspan` moved up or down so that the block of lines is centred on `y`. The content is escaped and
/// the entity placeholders are decoded, as the reference's final SVG string is decoded (`mermaidAPI.ts`
/// 359).
#[must_use]
pub fn by_tspan(
    content: &str,
    x: f64,
    y: f64,
    width: f64,
    face: Face<'_>,
    paint: Paint<'_>,
) -> String {
    let lines = split_line_breaks(content);
    let count = lines.len() as f64;
    let mut out = String::new();
    for (index, line) in lines.iter().enumerate() {
        let dy = index as f64 * face.size - (face.size * (count - 1.0)) / 2.0;
        let italic = if paint.italic {
            " font-style=\"italic\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" dominant-baseline=\"middle\" fill=\"{fill}\"{italic} style=\"text-anchor: middle; font-size: {size}px; font-weight: {weight}; font-family: {family};\"><tspan dy=\"{dy}\" alignment-baseline=\"mathematical\">{text}</tspan></text>",
            x = number(x + width / 2.0),
            y = number(y),
            fill = escape(paint.fill),
            size = number(face.size),
            weight = face.weight,
            family = face.family,
            dy = number(dy),
            text = decode_entities(&escape(line)),
        ));
    }
    out
}

/// The lines of a legacy text, for the font subset (every line is set in the face of its weight).
#[must_use]
pub fn lines(content: &str) -> Vec<&str> {
    split_line_breaks(content)
}
