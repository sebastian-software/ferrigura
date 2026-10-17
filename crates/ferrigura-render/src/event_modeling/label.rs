//! The text of a box, drawn as SVG text in the lines the browser breaks the reference's HTML into
//! (decision 0004; `docs-port/eventmodeling/NOTES.md` 6.4.2).
//!
//! The reference's box text is `<b>name</b>`, then `<br/><br/>` and a `code` block with the data.
//! The browser lays it out in a table cell as wide as the box's text area (`foreignObject`):
//!
//! - the name lines are bold, 14 px, centred in the cell, one line for each `<br/>` of the name;
//! - the second `<br/>` leaves an empty line, the height of the cell's own font;
//! - the `code` block is in the mono face, left aligned at the cell's left edge, at most 430 px wide.
//!   Its runs are drawn in the weight and slant of their tags: `b` and `strong` are bold, `i` and
//!   `em` italic, and nested tags add up. It breaks only where its text has a break opportunity: the
//!   blanks of the data's newlines (`&nbsp;` does not break), and the `<br/>` of the wrapped data.
//!   A line is measured in the faces its runs are drawn in;
//! - the lines are one block, centred vertically in the cell when it is taller than they are.
//!
//! The line boxes are the font's text boxes (`ferrigura_text::text_box`), which is what the browser
//! reports for a character. The reference's entity placeholders and HTML entities are decoded as the
//! browser decodes them, so the drawn text is the text the reader sees; the measured text keeps them.

use ferrigura_syntax::decode_entities;
use ferrigura_text::{
    Face, FontFamily, FontStyle, FontWeight, TextBox, face, layout_width, measure, text_box,
};

use super::layout::BoxText;
use crate::RenderError;
use crate::common::text::collapsed_width;
use crate::journey::text::{bold, regular, split_br};

/// The widest a code block is: `max-width` of the `code` element (`db.ts` 376-378).
const CODE_MAX_WIDTH: f64 = 430.0;

/// The weight and slant of a run of a box's text. The data is HTML: `b` and `strong` make a run
/// bold, `i` and `em` italic, and nested tags add up (`db.ts` 349-378 builds the data as HTML, and
/// the reference's browser draws its tags). A tag of any other name draws nothing of its own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    /// Bold: `b` or `strong`.
    pub bold: bool,
    /// Italic: `i` or `em`.
    pub italic: bool,
}

/// The mono face of a style: the `code` element's family, in the weight and slant of the run.
#[must_use]
pub fn code_face(style: Style) -> &'static Face {
    face(
        FontFamily::Mono,
        if style.bold {
            FontWeight::Bold
        } else {
            FontWeight::Normal
        },
        if style.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
    )
}

/// The mono face of the code block itself, which is the strut of its lines.
#[must_use]
pub fn mono() -> &'static Face {
    code_face(Style::default())
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

/// Which face a line of the label is set in: the bold name in the sans face, or the code in the mono
/// face, in the weight and slant of each run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// A line of the name: bold.
    Name,
    /// A line of the data: the mono face.
    Code,
}

/// A run of a line: its text as the reader sees it (a run of blanks is one blank), and its style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRun {
    /// The text of the run.
    pub text: String,
    /// The style the run is drawn in.
    pub style: Style,
}

/// One line of a box's text, as it is drawn: its face, its runs, and the position of its first glyph.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelLine {
    /// The face of the line.
    pub kind: LineKind,
    /// The runs of the line, in order.
    pub runs: Vec<LineRun>,
    /// The start of the line.
    pub x: f64,
    /// The baseline of the line.
    pub y: f64,
}

/// A line of the browser's layout, before it has its place: its kind (`None` for an empty line,
/// which has no glyphs), its runs, and the distances of its baseline from its top and its height.
struct Entry {
    kind: Option<LineKind>,
    runs: Vec<LineRun>,
    ascent: f64,
    height: f64,
}

/// A character of a box's text with the style it is drawn in.
type Glyph = (char, Style);

/// The ascent and the height of a line that holds the given boxes: the tallest ascent and the deepest
/// descent of its strut and its runs, as the browser sets the baseline of a line with runs in several
/// faces.
fn line_box(boxes: &[TextBox]) -> (f64, f64) {
    let ascent = boxes.iter().map(|text| text.ascent).fold(0.0, f64::max);
    let descent = boxes.iter().map(|text| text.descent).fold(0.0, f64::max);
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

/// The characters of a piece of the reference's HTML with the style of each. The weight and slant
/// of a tag apply to the text up to its end tag, a `<` that starts no tag stays, and the entities are
/// decoded in each run of text (`visible`).
#[must_use]
pub fn styled(html: &str) -> Vec<Glyph> {
    let mut out = Vec::new();
    let mut bold = 0_usize;
    let mut italic = 0_usize;
    let mut rest = html;
    loop {
        let style = Style {
            bold: bold > 0,
            italic: italic > 0,
        };
        let Some(start) = rest.find('<') else {
            push_text(rest, style, &mut out);
            return out;
        };
        let after = &rest[start + 1..];
        let is_tag = after
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '/');
        match after.find('>').filter(|_| is_tag) {
            Some(end) => {
                push_text(&rest[..start], style, &mut out);
                apply_tag(&after[..end], &mut bold, &mut italic);
                rest = &after[end + 1..];
            }
            None => {
                push_text(&rest[..=start], style, &mut out);
                rest = after;
            }
        }
    }
}

/// The text of a run, decoded, with its style.
fn push_text(text: &str, style: Style, out: &mut Vec<Glyph>) {
    out.extend(
        html_entities(&decode_entities(text))
            .chars()
            .map(|character| (character, style)),
    );
}

/// Counts a tag that changes the weight or the slant: an opening tag adds one, its end tag takes one
/// away. The other tags are not counted.
fn apply_tag(tag: &str, bold: &mut usize, italic: &mut usize) {
    let closing = tag.starts_with('/');
    let name = tag
        .trim_start_matches('/')
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    let counter = match name.as_str() {
        "b" | "strong" => bold,
        "i" | "em" => italic,
        _ => return,
    };
    if closing {
        *counter = counter.saturating_sub(1);
    } else {
        *counter += 1;
    }
}

/// Whether a character is white space that a browser collapses in a line of text: the blank, the tab,
/// the line feed, the return and the form feed (CSS `white-space: normal`). A non-breaking space is
/// not one of them.
fn is_collapsible(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

/// The runs of a line as the browser draws them: a run of collapsible white space is one blank, in
/// the style of its first character, and the blanks at the ends are not drawn. Characters of one
/// style next to each other form one run.
#[must_use]
pub fn grouped(glyphs: &[Glyph]) -> Vec<LineRun> {
    let mut runs: Vec<LineRun> = Vec::new();
    let mut started = false;
    let mut blank: Option<Style> = None;
    for &(character, style) in glyphs {
        if is_collapsible(character) {
            blank.get_or_insert(style);
            continue;
        }
        if let Some(blank_style) = blank.take().filter(|_| started) {
            push_glyph(&mut runs, ' ', blank_style);
        }
        started = true;
        push_glyph(&mut runs, character, style);
    }
    runs
}

fn push_glyph(runs: &mut Vec<LineRun>, character: char, style: Style) {
    match runs.last_mut() {
        Some(run) if run.style == style => run.text.push(character),
        _ => runs.push(LineRun {
            text: character.to_string(),
            style,
        }),
    }
}

/// The width of a line of glyphs as the browser lays its runs out: each run is measured in the mono
/// face of its style, its advance is snapped to the layout unit, and the runs are summed.
///
/// # Errors
///
/// Returns an error if the mono faces cannot be read.
pub fn line_width(glyphs: &[Glyph], size: f64) -> Result<f64, RenderError> {
    let mut total = 0.0;
    for run in grouped(glyphs) {
        total += layout_width(measure(code_face(run.style), &run.text, size)?);
    }
    Ok(total)
}

/// The pieces of a code line the browser can break between: a piece ends after a run of collapsible
/// white space, and after a hyphen that is followed by a letter (UAX 14, as `journey::text` breaks a
/// label). The pieces keep their characters as written, so a newline stays in the line it is in.
fn pieces(glyphs: &[Glyph]) -> Vec<Vec<Glyph>> {
    let mut pieces = Vec::new();
    let mut current: Vec<Glyph> = Vec::new();
    for (index, &glyph) in glyphs.iter().enumerate() {
        current.push(glyph);
        let next = glyphs.get(index + 1).map(|&(character, _)| character);
        let ends_run = is_collapsible(glyph.0) && !next.is_some_and(is_collapsible);
        let hyphen = glyph.0 == '-' && next.is_some_and(char::is_alphabetic);
        if ends_run || hyphen {
            pieces.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        pieces.push(current);
    }
    pieces
}

/// The lines a code text breaks into in a cell `width` wide, at the font size `size`: each line takes
/// the next piece while it fits, and the first piece always goes on a line. The width of a line is
/// measured in the faces its runs are drawn in.
///
/// # Errors
///
/// Returns an error if the mono faces cannot be read.
fn break_code(glyphs: &[Glyph], width: f64, size: f64) -> Result<Vec<Vec<Glyph>>, RenderError> {
    let mut lines = Vec::new();
    let mut current: Vec<Glyph> = Vec::new();
    for piece in pieces(glyphs) {
        let candidate: Vec<Glyph> = current.iter().chain(&piece).copied().collect();
        if current.is_empty() || line_width(&candidate, size)? <= width {
            current = candidate;
        } else {
            lines.push(current);
            current = piece;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    Ok(lines)
}

/// The widest piece of a code text: the least width its cell can take, since a piece has no break
/// inside it.
///
/// # Errors
///
/// Returns an error if the mono faces cannot be read.
fn widest_piece(glyphs: &[Glyph], size: f64) -> Result<f64, RenderError> {
    let mut widest = 0.0_f64;
    for piece in pieces(glyphs) {
        widest = widest.max(line_width(&piece, size)?);
    }
    Ok(widest)
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

/// A position rounded down to the layout unit of 1/64 px, as Chromium keeps it.
fn floor_layout_unit(length: f64) -> f64 {
    (length * 64.0 + 1e-6).floor() / 64.0
}

/// The lines of a box's text in its cell, at the font size `size` (the root's, 14 px in the redux
/// theme).
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or shaped.
pub fn lines_of(text: &BoxText, cell: Cell, size: f64) -> Result<Vec<LabelLine>, RenderError> {
    let bold_face = bold();
    // The strut of a name line is the span's font, the regular face; the strut of a code line is the
    // code block's, the mono face.
    let strut = text_box(regular(), size)?;
    let mono_strut = text_box(mono(), size)?;
    let name_box = text_box(bold_face, size)?;
    let (name_ascent, name_height) = line_box(&[strut, name_box]);
    let blank = |text: TextBox| Entry {
        kind: None,
        runs: Vec::new(),
        ascent: text.ascent,
        height: text.height,
    };

    // The name's lines: each `<br/>` ends one, and `visible` decodes the placeholders.
    let name_pieces: Vec<String> = split_br(&text.name)
        .iter()
        .map(|piece| visible(piece))
        .collect();
    let mut entries: Vec<Entry> = name_pieces
        .iter()
        .map(|piece| Entry {
            kind: Some(LineKind::Name),
            runs: vec![LineRun {
                text: piece.clone(),
                style: Style {
                    bold: true,
                    italic: false,
                },
            }],
            ascent: name_ascent,
            height: name_height,
        })
        .collect();

    // The data: the `<br/>` that ends the name is followed by an empty line, the strut's height, and
    // the code block's lines follow. A trailing `<br/>` ends the last line and adds none. Each segment
    // is kept with whether its source is empty: an empty source is a line of its own.
    let mut segments: Vec<(bool, Vec<Glyph>)> = Vec::new();
    if let Some(html) = &text.data {
        entries.push(blank(strut));
        if !html.is_empty() {
            let mut pieces = split_br(html);
            if pieces.len() > 1 && pieces.last().is_some_and(String::is_empty) {
                pieces.pop();
            }
            segments = pieces
                .iter()
                .map(|piece| (piece.is_empty(), styled(piece)))
                .collect();
        }
    }

    // The min-content width of the table cell: its widest unbreakable piece. A code block is capped
    // at 430 px, its own max-width, and the cell grows to hold the widest of its lines.
    let mut name_widest = 0.0_f64;
    for piece in &name_pieces {
        name_widest = name_widest.max(collapsed_width(bold_face, piece, size)?);
    }
    let mut code_widest = 0.0_f64;
    for (_, glyphs) in &segments {
        code_widest = code_widest.max(widest_piece(glyphs, size)?);
    }
    let cell_width = cell
        .width
        .max(name_widest.max(code_widest.min(CODE_MAX_WIDTH)));
    let break_width = cell_width.min(CODE_MAX_WIDTH);

    for (empty_source, glyphs) in &segments {
        let lines = if *empty_source {
            Vec::new()
        } else {
            break_code(glyphs, break_width, size)?
        };
        if lines.is_empty() {
            entries.push(blank(mono_strut));
            continue;
        }
        for line in lines {
            let runs = grouped(&line);
            if runs.is_empty() {
                entries.push(blank(mono_strut));
                continue;
            }
            // A line is as tall as its strut and its runs' faces: a bold or italic run may be taller.
            let mut boxes = vec![mono_strut];
            for run in &runs {
                boxes.push(text_box(code_face(run.style), size)?);
            }
            let (ascent, height) = line_box(&boxes);
            entries.push(Entry {
                kind: Some(LineKind::Code),
                runs,
                ascent,
                height,
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
                    let width = collapsed_width(bold_face, &entry.runs[0].text, size)?;
                    left + floor_layout_unit((cell_width - width) / 2.0)
                }
                LineKind::Code => left,
            };
            lines.push(LabelLine {
                kind,
                runs: entry.runs,
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

    fn bold_style() -> Style {
        Style {
            bold: true,
            italic: false,
        }
    }

    fn italic_style() -> Style {
        Style {
            bold: false,
            italic: true,
        }
    }

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
    fn b_and_strong_are_bold_i_and_em_are_italic_and_nesting_adds_up() {
        let glyphs = styled("<b>x</b>y<strong>z</strong>");
        assert_eq!(
            glyphs,
            [
                ('x', bold_style()),
                ('y', Style::default()),
                ('z', bold_style())
            ]
        );
        let nested = styled("<i><b>x</b>y</i>z");
        assert_eq!(
            nested,
            [
                (
                    'x',
                    Style {
                        bold: true,
                        italic: true
                    }
                ),
                ('y', italic_style()),
                ('z', Style::default())
            ]
        );
        // An unknown tag draws nothing of its own, and a stray `<` stays.
        assert_eq!(
            styled("<u>a</u> < b"),
            [
                ('a', Style::default()),
                (' ', Style::default()),
                ('<', Style::default()),
                (' ', Style::default()),
                ('b', Style::default())
            ]
        );
    }

    #[test]
    fn a_run_of_white_space_is_one_blank_in_the_style_of_its_first_character() {
        let runs = grouped(&styled("a\n\nb c"));
        assert_eq!(
            runs,
            [LineRun {
                text: "a b c".to_owned(),
                style: Style::default(),
            }]
        );
        // The collapsed newline is in the bold run, so its blank is bold too.
        let runs = grouped(&styled("<b>a\n</b>\u{a0}q"));
        assert_eq!(
            runs,
            [
                LineRun {
                    text: "a ".to_owned(),
                    style: bold_style(),
                },
                LineRun {
                    text: "\u{a0}q".to_owned(),
                    style: Style::default(),
                }
            ]
        );
    }

    #[test]
    fn a_code_line_breaks_at_a_newline_only_when_it_does_not_fit() {
        // The blank joins, `&nbsp;`, do not break; the newline is a blank that does.
        let text = styled("productId:&nbsp;7,\n&nbsp;quantity:&nbsp;1");
        assert_eq!(break_code(&text, 500.0, 14.0).unwrap().len(), 1);
        let lines = break_code(&text, 60.0, 14.0).unwrap();
        let texts: Vec<String> = lines
            .iter()
            .map(|line| grouped(line).iter().map(|run| run.text.clone()).collect())
            .collect();
        assert_eq!(texts, ["productId:\u{a0}7,", "\u{a0}quantity:\u{a0}1"]);
    }

    #[test]
    fn a_bold_run_is_measured_in_the_bold_face() {
        // The width of a line is the sum of its runs, each in its own face and snapped.
        let plain_width = line_width(&styled("abc"), 14.0).unwrap();
        let bold_width = line_width(&styled("<b>abc</b>"), 14.0).unwrap();
        let sum = layout_width(measure(code_face(bold_style()), "abc", 14.0).unwrap());
        assert_eq!(bold_width, sum);
        assert!(plain_width > 0.0);
    }

    #[test]
    fn a_placeholder_decodes_to_the_character_it_names() {
        assert_eq!(visible("ﬂ°quot¶ß"), "\"");
        assert_eq!(visible("x ﬂ°amp¶ß y"), "x & y");
    }
}
