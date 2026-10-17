//! The text of a box, drawn as SVG text in the lines the browser breaks the reference's HTML into
//! (decision 0004; `docs-port/eventmodeling/NOTES.md` 6.4.2).
//!
//! The reference's box text is `<b>name</b>`, then `<br/><br/>` and a `code` block with the data.
//! The browser lays it out in a table cell as wide as the box's text area (`foreignObject`):
//!
//! - the name lines are bold, 14 px, centred in the cell, one line for each `<br/>` of the name;
//! - the second `<br/>` leaves an empty line, the height of the cell's own font;
//! - the `code` block is in the mono face, left aligned at the cell's left edge, at most 430 px wide.
//!   It breaks only where its text has a break opportunity: the blanks of the data's newlines
//!   (`&nbsp;` does not break), and the `<br/>` of the wrapped data;
//! - the lines are one block, centred vertically in the cell when it is taller than they are.
//!
//! The line boxes are the font's text boxes (`ferrigura_text::text_box`), which is what the browser
//! reports for a character. The reference's entity placeholders and HTML entities are decoded as the
//! browser decodes them, so the drawn text is the text the reader sees; the measured text keeps them.

use ferrigura_syntax::decode_entities;
use ferrigura_text::{Face, FontFamily, FontStyle, FontWeight, face, text_box};

use super::layout::BoxText;
use crate::RenderError;
use crate::common::text::collapsed_width;
use crate::journey::text::{bold, regular, split_br};

/// The widest a code block is: `max-width` of the `code` element (`db.ts` 376-378).
const CODE_MAX_WIDTH: f64 = 430.0;

/// The mono face: the face of the `code` element, which the reference takes from the UA's
/// `monospace` and the harness pins to this face (`tools/port/geometry/common.ts`).
#[must_use]
pub fn mono() -> &'static Face {
    face(FontFamily::Mono, FontWeight::Normal, FontStyle::Normal)
}

/// The cell of a box's text: the `foreignObject` of the reference (`renderer.ts` 30-33).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// The left edge.
    pub x: f64,
    /// The top edge.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

/// Which face a line of the label is set in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// A line of the name: bold.
    Name,
    /// A line of the data: the mono face.
    Code,
}

/// One line of a box's text, as it is drawn: its text, and the position of its first glyph.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelLine {
    /// The face of the line.
    pub kind: LineKind,
    /// The text the reader sees, with the non-breaking spaces of the data.
    pub text: String,
    /// The start of the line.
    pub x: f64,
    /// The baseline of the line.
    pub y: f64,
}

/// A line of the browser's layout, before it has its place: its kind (`None` for an empty line,
/// which has no glyphs), its text, and the distances of its baseline from its top and its height.
struct Entry {
    kind: Option<LineKind>,
    text: String,
    ascent: f64,
    height: f64,
}

/// The browser's rule for a line with runs of two faces: the line box is as tall as the taller
/// ascent plus the taller descent of its runs, and the baseline is at the taller ascent.
fn merged(first: ferrigura_text::TextBox, second: ferrigura_text::TextBox) -> (f64, f64) {
    let ascent = first.ascent.max(second.ascent);
    let descent = first.descent.max(second.descent);
    (ascent, ascent + descent)
}

/// The text the reader sees of a piece of the reference's HTML: the tags of the sanitised data are
/// dropped (a tag draws nothing, its text does), then the placeholders are decoded (`decodeEntities`)
/// and the HTML entities.
#[must_use]
pub fn visible(html: &str) -> String {
    // The tags are read before the entities are decoded, so an escaped `&lt;b&gt;` stays text.
    html_entities(&decode_entities(&strip_tags(html)))
}

/// The HTML entities of the text the reference writes: the ones the sanitiser and the blank join
/// produce, and the numeric references. An entity that is not known stays as it is.
fn html_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let decoded = tail.find(';').filter(|&end| end <= 10).and_then(|end| {
            let name = &tail[1..end];
            let character = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                _ => name
                    .strip_prefix('#')
                    .and_then(|number| match number.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok(),
                        None => number.parse::<u32>().ok(),
                    })
                    .and_then(char::from_u32),
            };
            character.map(|character| (character, end + 1))
        });
        match decoded {
            Some((character, length)) => {
                out.push(character);
                rest = &tail[length..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The text without its tags: every `<…>` that starts with a letter or a slash is a tag, and it
/// draws nothing. A `<` that starts no tag stays.
fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        let is_tag = after
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '/');
        match after.find('>').filter(|_| is_tag) {
            Some(end) => {
                out.push_str(&rest[..start]);
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[..=start]);
                rest = &rest[start + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Whether a character is white space that a browser collapses in a line of text: the blank, the
/// tab, the line feed, the return and the form feed (CSS `white-space: normal`). A non-breaking
/// space is not one of them.
fn is_collapsible(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

/// The pieces of a code text, as the browser can break it, in the text as written: a piece ends
/// after a run of collapsible white space, and after a hyphen that is followed by a letter (UAX 14,
/// as `journey::text` breaks a label). The pieces keep the characters the reference's HTML keeps,
/// so a newline stays in the line it is in.
fn raw_pieces(text: &str) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    let mut pieces = Vec::new();
    let mut current = String::new();
    for (index, &character) in characters.iter().enumerate() {
        current.push(character);
        let next = characters.get(index + 1).copied();
        let ends_run = is_collapsible(character) && !next.is_some_and(is_collapsible);
        let hyphen = character == '-' && next.is_some_and(char::is_alphabetic);
        if ends_run || hyphen {
            pieces.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        pieces.push(current);
    }
    pieces
}

/// The text of a line without the white space at its ends: a line break's white space hangs, and
/// the browser draws none of it.
fn trim_collapsible(text: &str) -> &str {
    text.trim_matches(is_collapsible)
}

/// The text of a line as it is drawn: a run of collapsible white space is one blank, as the browser
/// lays it out (the reference's HTML collapses the newline of a data block to a blank). SVG text
/// draws a line feed with a glyph of its own, so the blank is written instead.
#[must_use]
pub fn drawn(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_run = false;
    for character in line.chars() {
        if is_collapsible(character) {
            if !in_run {
                out.push(' ');
            }
            in_run = true;
        } else {
            out.push(character);
            in_run = false;
        }
    }
    out
}

/// The lines a code text breaks into in a cell `width` wide, in the mono face at `size`: each line
/// takes the next piece while it fits, the first piece always goes on a line, and the text of a line
/// is as written (`raw_pieces`), so a newline inside a line stays in the line's text.
///
/// # Errors
///
/// Returns an error if the mono face cannot be read.
fn break_code(text: &str, width: f64, size: f64) -> Result<Vec<String>, RenderError> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for piece in raw_pieces(text) {
        let candidate = format!("{current}{piece}");
        if current.is_empty()
            || collapsed_width(mono(), trim_collapsible(&candidate), size)? <= width
        {
            current = candidate;
        } else {
            lines.push(trim_collapsible(&current).to_owned());
            current = piece;
        }
    }
    if !current.is_empty() {
        lines.push(trim_collapsible(&current).to_owned());
    }
    Ok(lines.iter().map(|line| drawn(line)).collect())
}

/// The widest piece of a code text, in the mono face: the least width its cell can take, since a
/// piece has no break inside it.
fn widest_piece(text: &str, size: f64) -> Result<f64, RenderError> {
    let mut widest = 0.0_f64;
    for piece in raw_pieces(text) {
        widest = widest.max(collapsed_width(mono(), trim_collapsible(&piece), size)?);
    }
    Ok(widest)
}

/// A position rounded down to the layout unit of 1/64 px, as Chromium keeps it.
fn floor_layout_unit(length: f64) -> f64 {
    (length * 64.0 + 1e-6).floor() / 64.0
}

/// The lines of a box's text in its cell, at the font size `size` (the root's, 14 px in the
/// redux theme).
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or shaped.
pub fn lines_of(text: &BoxText, cell: Cell, size: f64) -> Result<Vec<LabelLine>, RenderError> {
    let bold = bold();
    let mono = mono();
    // The strut of a line is the span's font, the regular face; the code block's is the mono face.
    let strut = text_box(regular(), size)?;
    let name_metrics = text_box(bold, size)?;
    let mono_metrics = text_box(mono, size)?;
    let (name_ascent, name_height) = merged(strut, name_metrics);

    // The name's lines: each `<br/>` ends one, and `visible` decodes the placeholders.
    let name_pieces: Vec<String> = split_br(&text.name)
        .iter()
        .map(|piece| visible(piece))
        .collect();
    let mut entries = Vec::new();
    for piece in &name_pieces {
        entries.push(Entry {
            kind: Some(LineKind::Name),
            text: piece.clone(),
            ascent: name_ascent,
            height: name_height,
        });
    }

    // The data: the `<br/>` that ends the name is followed by an empty line, the strut's height,
    // and the code block's lines follow. A trailing `<br/>` ends the last line and adds none.
    let mut code_segments: Vec<String> = Vec::new();
    if let Some(html) = &text.data {
        entries.push(Entry {
            kind: None,
            text: String::new(),
            ascent: strut.ascent,
            height: strut.height,
        });
        if !html.is_empty() {
            let mut segments = split_br(html);
            if segments.len() > 1 && segments.last().is_some_and(String::is_empty) {
                segments.pop();
            }
            code_segments = segments.iter().map(|segment| visible(segment)).collect();
        }
    }

    // The min-content width of the table cell: its widest unbreakable piece. A code block is
    // capped at 430 px, its own max-width, and the cell grows to hold the widest of its lines.
    let mut name_widest = 0.0_f64;
    for piece in &name_pieces {
        name_widest = name_widest.max(collapsed_width(bold, piece, size)?);
    }
    let mut code_widest = 0.0_f64;
    for segment in &code_segments {
        code_widest = code_widest.max(widest_piece(segment, size)?);
    }
    let cell_width = cell
        .width
        .max(name_widest.max(code_widest.min(CODE_MAX_WIDTH)));
    let break_width = cell_width.min(CODE_MAX_WIDTH);

    for segment in &code_segments {
        if segment.is_empty() {
            entries.push(Entry {
                kind: None,
                text: String::new(),
                ascent: mono_metrics.ascent,
                height: mono_metrics.height,
            });
            continue;
        }
        let lines = break_code(segment, break_width, size)?;
        if lines.is_empty() {
            entries.push(Entry {
                kind: None,
                text: String::new(),
                ascent: mono_metrics.ascent,
                height: mono_metrics.height,
            });
        }
        for line in lines {
            entries.push(Entry {
                kind: Some(LineKind::Code),
                text: line,
                ascent: mono_metrics.ascent,
                height: mono_metrics.height,
            });
        }
    }

    // The block is centred vertically in the cell when it is shorter than the cell, and starts at
    // the cell's top when it is taller.
    let block: f64 = entries.iter().map(|entry| entry.height).sum();
    let mut top = if block < cell.height {
        cell.y + (cell.height - block) / 2.0
    } else {
        cell.y
    };

    // The cell's origin is a layout position too: the browser keeps it to 1/64 px, rounded down.
    let left = floor_layout_unit(cell.x);
    let mut lines = Vec::new();
    for entry in entries {
        let baseline = top + entry.ascent;
        if let Some(kind) = entry.kind {
            let x = match kind {
                LineKind::Name => {
                    let width = collapsed_width(bold, &entry.text, size)?;
                    left + floor_layout_unit((cell_width - width) / 2.0)
                }
                LineKind::Code => left,
            };
            lines.push(LabelLine {
                kind,
                text: entry.text,
                x,
                y: baseline,
            });
        }
        top += entry.height;
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_entities_decode_as_the_browser_decodes_them() {
        assert_eq!(html_entities("a&amp;b&lt;c&gt;"), "a&b<c>");
        assert_eq!(html_entities("productId:&nbsp;7"), "productId:\u{a0}7");
        assert_eq!(html_entities("&#65;&#x42;"), "AB");
        assert_eq!(html_entities("a & b &unknown; &"), "a & b &unknown; &");
    }

    #[test]
    fn tags_draw_nothing_and_a_stray_angle_bracket_stays() {
        assert_eq!(strip_tags("<b>x</b> y"), "x y");
        assert_eq!(strip_tags("a < b"), "a < b");
        assert_eq!(strip_tags("<span style=\"a\u{a0}b\">t</span>"), "t");
    }

    #[test]
    fn a_run_of_white_space_is_one_piece_and_one_blank_when_drawn() {
        assert_eq!(raw_pieces("a\n\nb c"), ["a\n\n", "b ", "c"]);
        assert_eq!(drawn("7,\n\u{a0}q"), "7, \u{a0}q");
        assert_eq!(drawn("a\n\t b"), "a b");
        assert_eq!(drawn("x\u{a0}y"), "x\u{a0}y");
    }

    #[test]
    fn a_code_line_breaks_at_a_newline_only_when_it_does_not_fit() {
        // The blank joins, `&nbsp;`, do not break; the newline is a blank that does.
        let text = "productId:\u{a0}7,\n\u{a0}quantity:\u{a0}1";
        assert_eq!(break_code(text, 500.0, 14.0).unwrap().len(), 1);
        let lines = break_code(text, 60.0, 14.0).unwrap();
        assert_eq!(lines, ["productId:\u{a0}7,", "\u{a0}quantity:\u{a0}1"]);
    }

    #[test]
    fn a_placeholder_decodes_to_the_character_it_names() {
        assert_eq!(visible("ﬂ°quot¶ß"), "\"");
        assert_eq!(visible("x ﬂ°amp¶ß y"), "x & y");
    }
}
