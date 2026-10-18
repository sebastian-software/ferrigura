//! The box-drawing preprocessor, `packages/mermaid/src/diagrams/treeView/boxDrawingPreprocessor.ts`.
//!
//! It runs before the grammar, on the text the core gives it (`encodeEntities` and a line break).
//! When a content line of the diagram holds a box-drawing character, the tree lines are rewritten
//! as indentation (four spaces per depth), and the decoration lines are dropped. The text of every
//! other line is kept as it is. The result keeps a map from its line numbers to the original ones,
//! which the errors of the grammar are reported through.
//!
//! The reference's positions are UTF-16 offsets (`String.prototype.search` and `RegExp.index`), so
//! the column of a branch character counts an astral character as two, as this module does.

use std::collections::BTreeMap;

use crate::error::ParseError;
use crate::text::{is_js_space, js_trim};

/// The keyword line, compared after `trim()`.
const KEYWORD: &str = "treeView-beta";

/// Four spaces per depth level in the output.
const INDENT_UNIT: &str = "    ";

/// The result of the preprocessing: the text the grammar reads, and the map from its line numbers
/// (1-based) to the original ones. The map is empty when the text was not rewritten.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BoxDrawing {
    /// The text the grammar reads.
    pub text: String,
    /// Output line number to original line number, both 1-based.
    pub line_map: BTreeMap<usize, usize>,
}

impl BoxDrawing {
    /// The original line of an error at an output line, as `remapErrorLines` gives it: the line
    /// itself when the map has no entry for it, or when the text was not rewritten.
    #[doc(hidden)]
    #[must_use]
    pub fn original_line(&self, line: usize) -> usize {
        self.line_map.get(&line).copied().unwrap_or(line)
    }

    /// The error with its line moved to the original line. An error that is not in the rewritten
    /// text keeps its position.
    #[doc(hidden)]
    #[must_use]
    pub fn remap_error(&self, mut error: ParseError) -> ParseError {
        if !self.line_map.is_empty() {
            error.line = self.original_line(error.line);
        }
        error
    }
}

/// `isBoxDrawingFormat`: whether any of the lines holds a box-drawing character.
#[doc(hidden)]
#[must_use]
pub fn is_box_drawing_format(lines: &[&str]) -> bool {
    lines.iter().any(|line| line.chars().any(is_box_char))
}

/// Preprocesses the text of a treeView diagram, as `preprocessBoxDrawing` does.
///
/// # Errors
///
/// The two errors of the reference, with their messages: an empty node after a box-drawing prefix,
/// and an indented line without a box-drawing character in box mode. Both are reported at the line
/// of the input (`Line N: …`), as the reference throws them outside its parser.
#[doc(hidden)]
pub fn preprocess_box_drawing(input: &str) -> Result<BoxDrawing, ParseError> {
    let unchanged = || BoxDrawing {
        text: input.to_owned(),
        line_map: BTreeMap::new(),
    };
    let lines: Vec<&str> = input.split('\n').collect();
    let Some(keyword) = lines.iter().position(|line| js_trim(line) == KEYWORD) else {
        return Ok(unchanged());
    };

    // The content lines decide the mode: blanks, comments, metadata and decoration are skipped.
    let content: Vec<String> = lines[keyword + 1..]
        .iter()
        .filter(|line| !is_skipped_for_detection(line))
        .map(|line| line.replace('\t', INDENT_UNIT))
        .collect();
    if !is_box_drawing_format(&content.iter().map(String::as_str).collect::<Vec<_>>()) {
        return Ok(unchanged());
    }
    let segment_width = infer_segment_width(&content);

    let mut output = Output::default();
    for (index, line) in lines.iter().enumerate().take(keyword + 1) {
        output.keep((*line).to_owned(), index + 1);
    }

    for (index, line) in lines.iter().enumerate().skip(keyword + 1) {
        let original = index + 1;
        if js_trim(line).is_empty() || is_comment(line) || is_metadata(line) {
            output.keep((*line).to_owned(), original);
            continue;
        }
        if is_decoration(line) {
            continue;
        }

        let normalized = line.replace('\t', INDENT_UNIT);
        if let Some(branch) = normalized.find(is_branch_char) {
            // The depth is `Math.round(column / segmentWidth) + 1`, with the column in UTF-16 units.
            let column = utf16_len(&normalized[..branch]);
            let depth = js_round(column as f64 / segment_width as f64) + 1;
            let content = branch_content(&normalized, branch);
            if content.is_empty() {
                return Err(ParseError {
                    line: original,
                    column: 1,
                    message: format!(
                        "Line {original}: Empty node — expected a filename or directory name after the box-drawing prefix"
                    ),
                });
            }
            output.keep(format!("{}{content}", INDENT_UNIT.repeat(depth)), original);
        } else if normalized.chars().all(|c| is_js_space(c) || is_box_char(c)) {
            // A line of decoration only, with no branch: dropped.
            continue;
        } else if normalized.chars().any(is_box_char) {
            // A box character without a branch: content such as `Section ─ A.txt`, a root item.
            output.keep((*line).to_owned(), original);
        } else if normalized.starts_with(is_js_space) {
            return Err(ParseError {
                line: original,
                column: 1,
                message: format!(
                    "Line {original}: Unexpected indentation without box-drawing characters. In box-drawing format, use ├── or └── prefixes for indented nodes."
                ),
            });
        } else {
            output.keep((*line).to_owned(), original);
        }
    }

    Ok(output.finish())
}

/// The lines of the output, with the map from their numbers to the original ones.
#[derive(Default)]
struct Output {
    lines: Vec<String>,
    line_map: BTreeMap<usize, usize>,
}

impl Output {
    /// Appends a line that came from the original line `original` (1-based).
    fn keep(&mut self, text: String, original: usize) {
        self.lines.push(text);
        self.line_map.insert(self.lines.len(), original);
    }

    fn finish(self) -> BoxDrawing {
        BoxDrawing {
            text: self.lines.join("\n"),
            line_map: self.line_map,
        }
    }
}

/// The lines that do not count when the mode is chosen: blank, comment, metadata and decoration.
fn is_skipped_for_detection(line: &str) -> bool {
    js_trim(line).is_empty() || is_comment(line) || is_metadata(line) || is_decoration(line)
}

/// `/^\s*%%/`.
fn is_comment(line: &str) -> bool {
    line.trim_start_matches(is_js_space).starts_with("%%")
}

/// `/^\s*(title[\t ]|accTitle[\t ]*:|accDescr[\t ]*[:{])/`: a header line the grammar reads.
fn is_metadata(line: &str) -> bool {
    let text = line.trim_start_matches(is_js_space);
    if let Some(after) = text.strip_prefix("title") {
        return after.starts_with(['\t', ' ']);
    }
    if let Some(after) = text.strip_prefix("accTitle") {
        return after.trim_start_matches(['\t', ' ']).starts_with(':');
    }
    if let Some(after) = text.strip_prefix("accDescr") {
        return after
            .trim_start_matches(['\t', ' '])
            .starts_with([':', '{']);
    }
    false
}

/// `/^[\s│┃]+$/`: a non-empty line of blanks and the two vertical bars.
fn is_decoration(line: &str) -> bool {
    !line.is_empty() && line.chars().all(|c| is_js_space(c) || c == '│' || c == '┃')
}

/// The box-drawing characters: `─━│┃└┗├┣`.
fn is_box_char(c: char) -> bool {
    matches!(c, '─' | '━' | '│' | '┃' | '└' | '┗' | '├' | '┣')
}

/// The branch characters: `└┗├┣`.
fn is_branch_char(c: char) -> bool {
    matches!(c, '└' | '┗' | '├' | '┣')
}

/// The dash characters of a prefix: `─━`.
fn is_dash_char(c: char) -> bool {
    matches!(c, '─' | '━')
}

/// `inferSegmentWidth`: the column of the first branch character that is not in column 0, found
/// line by line; 4 when there is none.
fn infer_segment_width(content: &[String]) -> usize {
    for line in content {
        if let Some(branch) = line.find(is_branch_char) {
            let column = utf16_len(&line[..branch]);
            if column > 0 {
                return column;
            }
        }
    }
    4
}

/// The text after a branch character: its dashes, then its spaces, with the end trimmed.
fn branch_content(line: &str, branch: usize) -> String {
    let mut rest = &line[branch..];
    let mut chars = rest.chars();
    // The branch itself is one character.
    chars.next();
    rest = chars.as_str();
    let after_dashes = rest.trim_start_matches(is_dash_char);
    let after_spaces = after_dashes.trim_start_matches(' ');
    after_spaces.trim_end_matches(is_js_space).to_owned()
}

/// `Math.round` for the non-negative quotients of this module: halves round up.
fn js_round(value: f64) -> usize {
    (value + 0.5).floor() as usize
}

/// The length of a string in UTF-16 code units, JavaScript's `length`.
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}
