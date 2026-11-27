//! Text cleanup before lexing: the parts of `packages/mermaid/src/preprocess.ts` that decide what
//! the grammar sees.
//!
//! The front matter is read here, with the YAML reader of `crate::yaml` that the reference's
//! `js-yaml` corresponds to, so a front matter that does not parse is an error for every diagram.
//! Comments and directives are not removed here: the lexer treats them as hidden tokens, as the
//! grammars do.

use std::borrow::Cow;

use crate::ParseError;
use crate::text::{is_cr_lf, is_js_space, is_line_term, is_word};
use crate::yaml::{self, Yaml};

/// Text prepared for the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    /// The text after front matter removal, with CRLF and CR normalized to LF.
    pub(crate) text: String,
    /// The `title` of the front matter, if it is truthy and its string is not empty.
    pub(crate) title: Option<String>,
    /// The fields of the front matter's YAML mapping, in order. Empty when there is no front
    /// matter, or when its document is not a mapping, as `extractFrontMatter` treats it.
    pub(crate) front_matter: Vec<(String, Yaml)>,
    /// Number of lines removed with the front matter, so positions can be reported against the
    /// original input.
    pub(crate) line_offset: usize,
}

/// Normalizes CRLF and CR to LF without allocating when `\r` is absent.
fn normalize_line_breaks(raw: &str) -> Cow<'_, str> {
    if !raw.contains('\r') {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut copied = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            out.push_str(&raw[copied..i]);
            out.push('\n');
            if bytes.get(i + 1) == Some(&b'\n') {
                i += 2;
            } else {
                i += 1;
            }
            copied = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&raw[copied..]);
    Cow::Owned(out)
}

/// Whether `text` begins with optional horizontal JavaScript whitespace followed by `---`.
fn has_leading_front_matter_fence(text: &str) -> bool {
    let rest = text.trim_start_matches(|c: char| is_js_space(c) && !is_cr_lf(c));
    rest.starts_with("---")
}

/// Normalizes line breaks and quotes HTML attributes, then removes the front matter and reads it.
pub fn prepare(raw: &str) -> Result<Prepared, ParseError> {
    let line_norm = normalize_line_breaks(raw);
    let normalized = quote_html_attributes(&line_norm);
    if !has_leading_front_matter_fence(&normalized) {
        return Ok(Prepared {
            text: normalized.into_owned(),
            title: None,
            front_matter: Vec::new(),
            line_offset: 0,
        });
    }
    let chars: Vec<char> = normalized.chars().collect();
    let Some(front_matter) = match_front_matter(&chars) else {
        return Ok(Prepared {
            text: normalized.into_owned(),
            title: None,
            front_matter: Vec::new(),
            line_offset: 0,
        });
    };

    let removed = &chars[..front_matter.length];
    let fields = front_matter_fields(
        &front_matter.body,
        &front_matter.indent,
        front_matter.body_line_offset,
    )?;
    let title = fields
        .iter()
        .find(|(key, _)| key == "title")
        .and_then(|(_, value)| value.truthy_string())
        .filter(|title| !title.is_empty());
    Ok(Prepared {
        text: chars[front_matter.length..].iter().collect(),
        title,
        front_matter: fields,
        line_offset: removed.iter().filter(|&&c| c == '\n').count(),
    })
}

/// The fields of the front matter's YAML: the pairs of its mapping, or none when the document is
/// empty or is not a mapping.
fn front_matter_fields(
    raw_body: &str,
    indent: &str,
    body_line_offset: usize,
) -> Result<Vec<(String, Yaml)>, ParseError> {
    let body = dedent(raw_body, indent);
    match yaml::load(&body) {
        Ok(Some(Yaml::Mapping(pairs))) => Ok(pairs),
        Ok(_) => Ok(Vec::new()),
        Err(error) => {
            let line_index = error.line.saturating_sub(1);
            let bom_stripped = line_index == 0 && body.starts_with('\u{FEFF}');
            let yaml_body = body.strip_prefix('\u{FEFF}').unwrap_or(&body);
            let char_column = yaml_body
                .split('\n')
                .nth(line_index)
                .map_or(error.column, |line| {
                    utf16_offset_to_chars(line, error.column)
                })
                + usize::from(bom_stripped);
            let indent_offset = if !indent.is_empty()
                && raw_body
                    .split('\n')
                    .nth(line_index)
                    .is_some_and(|line| line.starts_with(indent))
            {
                indent.chars().count()
            } else {
                0
            };
            Err(ParseError {
                line: error.line + body_line_offset,
                column: char_column + indent_offset + 1,
                message: format!("invalid front matter: {}", error.message),
            })
        }
    }
}

/// Converts a 0-based UTF-16 code-unit offset on `line` into a 0-based Unicode scalar count.
fn utf16_offset_to_chars(line: &str, utf16_offset: usize) -> usize {
    let mut units = 0usize;
    let mut chars = 0usize;
    for ch in line.chars() {
        if units >= utf16_offset {
            break;
        }
        units += ch.len_utf16();
        chars += 1;
    }
    chars
}

/// Removes the captured indent from the start of each line that has it, as `extractFrontMatter`
/// does before it parses the YAML: js-yaml rejects indented documents.
fn dedent(body: &str, indent: &str) -> String {
    if indent.is_empty() {
        return body.to_owned();
    }
    body.split('\n')
        .map(|line| line.strip_prefix(indent).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `replace(/<(\w+)([^>]*)>/g, …)` with `="x"` changed to `='x'` inside the tag.
fn quote_html_attributes(text: &str) -> Cow<'_, str> {
    if !text.contains('<') || !text.contains('>') || !text.contains("=\"") {
        return Cow::Borrowed(text);
    }
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut out: Option<String> = None;
    let mut copied = 0;
    let mut i = 0;
    while i < n {
        let Some(rel_lt) = text[i..].find('<') else {
            break;
        };
        let lt = i + rel_lt;
        let mut name_end = lt + 1;
        while name_end < n && is_word(bytes[name_end] as char) {
            name_end += 1;
        }
        if name_end > lt + 1 {
            let Some(rel_gt) = text[name_end..].find('>') else {
                // No closing `>` exists anywhere in the rest of the text.
                break;
            };
            let close = name_end + rel_gt;
            let attrs = &text[name_end..close];
            if attrs.contains("=\"") {
                let quoted = quote_attributes(attrs);
                if quoted != attrs {
                    let buf = out.get_or_insert_with(|| String::with_capacity(text.len()));
                    buf.push_str(&text[copied..name_end]);
                    buf.push_str(&quoted);
                    copied = close;
                }
            }
            i = close + 1;
        } else {
            i = lt + 1;
        }
    }
    match out {
        Some(mut buf) => {
            buf.push_str(&text[copied..]);
            Cow::Owned(buf)
        }
        None => Cow::Borrowed(text),
    }
}

/// `replace(/="([^"]*)"/g, "='$1'")`.
fn quote_attributes(attributes: &str) -> String {
    let bytes = attributes.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut copied = 0;
    let mut i = 0;
    while i < n {
        let Some(rel) = attributes[i..].find("=\"") else {
            break;
        };
        let eq = i + rel;
        if let Some(rel_close) = attributes[eq + 2..].find('"') {
            let close = eq + 2 + rel_close;
            out.push_str(&attributes[copied..eq]);
            out.push_str("='");
            out.push_str(&attributes[eq + 2..close]);
            out.push('\'');
            copied = close + 1;
            i = close + 1;
        } else {
            // No closing `"` exists anywhere in the remainder of the tag.
            break;
        }
    }
    out.push_str(&attributes[copied..]);
    out
}

struct FrontMatter {
    indent: String,
    body: String,
    body_line_offset: usize,
    length: usize,
}

fn starts_at(text: &[char], at: usize, pattern: &[char]) -> bool {
    text.get(at..at + pattern.len()) == Some(pattern)
}

/// The index of the last line break in the whitespace run that starts at `from`.
fn last_line_break_in_run(text: &[char], from: usize) -> Option<usize> {
    let mut last = None;
    let mut i = from;
    while i < text.len() && is_js_space(text[i]) {
        if is_cr_lf(text[i]) {
            last = Some(i);
        }
        i += 1;
    }
    last
}

/// Port of `matchFrontMatter` in `packages/mermaid/src/diagram-api/regexes.ts`, which follows the
/// backtracking of `frontMatterRegex`: the longest opening that still leaves a closing fence, and
/// the first closing fence after the body.
/// The text after a front matter block at its start, as `detectType` strips it
/// (`detectType.ts` 15-18, `matchFrontMatter`): the block is only located, its YAML is not read.
pub(crate) fn strip_front_matter(text: &str) -> Cow<'_, str> {
    if !has_leading_front_matter_fence(text) {
        return Cow::Borrowed(text);
    }
    let chars: Vec<char> = text.chars().collect();
    match match_front_matter(&chars) {
        Some(front_matter) => Cow::Owned(chars[front_matter.length..].iter().collect()),
        None => Cow::Borrowed(text),
    }
}

fn match_front_matter(text: &[char]) -> Option<FrontMatter> {
    let n = text.len();
    let mut cursor = 0;
    while cursor < n && is_js_space(text[cursor]) && !is_cr_lf(text[cursor]) {
        cursor += 1;
    }
    let indent: String = text[..cursor].iter().collect();
    let dashes = ['-', '-', '-'];
    if !starts_at(text, cursor, &dashes) {
        return None;
    }

    let mut opening_ends = Vec::new();
    let mut i = cursor + 3;
    while i < n && is_js_space(text[i]) {
        if is_cr_lf(text[i]) {
            opening_ends.push(i);
        }
        i += 1;
    }
    let first_opening_end = *opening_ends.first()?;

    let closing: Vec<char> = indent.chars().chain(dashes).collect();
    let closing_end = |i: usize| -> Option<usize> {
        if i < n && is_cr_lf(text[i]) && starts_at(text, i + 1, &closing) {
            last_line_break_in_run(text, i + 1 + closing.len())
        } else {
            None
        }
    };

    let mut last_closing_fence = None;
    for i in first_opening_end + 1..n {
        if closing_end(i).is_some() {
            last_closing_fence = Some(i);
        }
    }
    let last_closing_fence = last_closing_fence?;

    let mut opening_end = first_opening_end;
    for &candidate in &opening_ends {
        if candidate < last_closing_fence {
            opening_end = candidate;
        }
    }

    let body_start = opening_end + 1;
    let body_line_offset = text[..body_start].iter().filter(|&&c| is_cr_lf(c)).count();
    for i in body_start..=last_closing_fence {
        if let Some(match_end) = closing_end(i) {
            return Some(FrontMatter {
                indent,
                body: text[body_start..i].iter().collect(),
                body_line_offset,
                length: match_end + 1,
            });
        }
    }
    None
}

/// Byte length of the blanks at the start of `text`.
fn ws_len(text: &str) -> usize {
    text.char_indices()
        .find(|&(_, c)| !is_js_space(c))
        .map_or(text.len(), |(index, _)| index)
}

/// Length of a `%%{ … }%%` directive as `directiveRegex` in `regexes.ts` matches it, which is the
/// loose form the reference uses to find directives before detection. `text` starts with `%%{`.
pub(crate) fn loose_directive_len(text: &str) -> Option<usize> {
    let peek = |i: usize| text.get(i..).and_then(|rest| rest.chars().next());
    let skip_space = |mut i: usize| {
        while let Some(c) = peek(i).filter(|&c| is_js_space(c)) {
            i += c.len_utf8();
        }
        i
    };
    let skip_word = |mut i: usize| {
        while let Some(c) = peek(i).filter(|&c| is_word(c)) {
            i += c.len_utf8();
        }
        i
    };

    let mut i = skip_space(3);
    if !peek(i).is_some_and(is_word) {
        return None;
    }
    i = skip_word(i);
    i = skip_space(i);
    if peek(i) == Some(':') {
        i = skip_space(i + 1);
    }
    if peek(i).is_some_and(is_word) {
        i = skip_space(skip_word(i));
        return Some(if text[i..].starts_with("}%%") {
            i + 3
        } else {
            i
        });
    }
    // The free text `(?:(?!}%{2}).|\r?\n)*` stops before `}%%`, and before a character that `.`
    // does not match: U+2028, U+2029 and a bare CR. The `\s*` after it takes that character and the
    // blanks that follow, and the closing `}%%` is optional.
    let stop = text[i..]
        .char_indices()
        .find(|&(index, c)| {
            matches!(c, '\u{2028}' | '\u{2029}' | '\r') || text[i + index..].starts_with("}%%")
        })
        .map_or(text.len(), |(index, _)| i + index);
    let end = stop + ws_len(&text[stop..]);
    Some(if text[end..].starts_with("}%%") {
        end + 3
    } else {
        end
    })
}

/// Removes the `%%{ … }%%` directives, as `removeDirectives` does.
pub(crate) fn remove_directives(text: &str) -> Cow<'_, str> {
    if !text.contains("%%{") {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("%%{") {
        out.push_str(&rest[..start]);
        match loose_directive_len(&rest[start..]) {
            Some(len) => rest = &rest[start + len..],
            None => {
                out.push_str("%%{");
                rest = &rest[start + 3..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// Removes the comment lines, as `cleanupComments` does: `^\s*%%(?!{)[^\n]+\n?` with the
/// multi-line flag, then the leading whitespace of the text.
pub(crate) fn cleanup_comments(text: &str) -> Cow<'_, str> {
    if !text.contains("%%") {
        return Cow::Borrowed(text.trim_start_matches(is_js_space));
    }
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut at = 0;
    loop {
        if let Some(end) = comment_match_end(text, at) {
            out.push_str(&text[copied..at]);
            copied = end;
            at = end;
            continue;
        }
        // The next position where `^` matches with the multi-line flag: after the next line
        // terminator, which is U+2028 and U+2029 as well as the line breaks.
        match text[at..].char_indices().find(|&(_, c)| is_line_term(c)) {
            Some((index, c)) => at += index + c.len_utf8(),
            None => break,
        }
    }
    if copied == 0 {
        return Cow::Borrowed(text.trim_start_matches(is_js_space));
    }
    out.push_str(&text[copied..]);
    let lead = out.len() - out.trim_start_matches(is_js_space).len();
    if lead > 0 {
        out.drain(..lead);
    }
    Cow::Owned(out)
}

/// The end of a comment match that starts at the line start `start`, if there is one.
fn comment_match_end(text: &str, start: usize) -> Option<usize> {
    let rest = &text[start..];
    let spaces = rest.len() - rest.trim_start_matches(is_js_space).len();
    let after = &rest[spaces..];
    let body = after.strip_prefix("%%")?;
    if body.starts_with('{') || body.is_empty() || body.starts_with('\n') {
        return None;
    }
    let line = body.find('\n').unwrap_or(body.len());
    let mut end = start + spaces + 2 + line;
    if text[end..].starts_with('\n') {
        end += 1;
    }
    Some(end)
}

/// Whether the diagram's first non-blank, non-comment line is a dialect whose `classDef`
/// declarations accept semicolon-separated CSS properties (`stateDiagram`). Flowchart dialects use
/// `;` as a statement separator instead.
fn allows_classdef_semicolon_css(text: &str) -> bool {
    for line in text.split(JS_LINE_TERMINATORS) {
        let trimmed = line.trim_start_matches(is_js_space);
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        return trimmed.strip_prefix("stateDiagram").is_some_and(|after| {
            after
                .chars()
                .next()
                .is_none_or(|c| is_js_space(c) || c == '-')
        });
    }
    false
}

/// Whether the diagram's first non-blank, non-comment line is `classDiagram` (or `classDiagram-v2`),
/// whose `class ... { ... }` bodies contain plain-text member rows whose literal `"` characters do
/// not open multiline quoted strings.
fn is_class_diagram_dialect(text: &str) -> bool {
    for line in text.split(JS_LINE_TERMINATORS) {
        let trimmed = line.trim_start_matches(is_js_space);
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        return trimmed.strip_prefix("classDiagram").is_some_and(|after| {
            after
                .chars()
                .next()
                .is_none_or(|c| is_js_space(c) || c == '-')
        });
    }
    false
}

/// Whether the diagram's first non-blank, non-comment line is a dialect whose unquoted single `:`
/// on non-`style`/non-`classDef` statements introduces a single-line plain label to end-of-line
/// (`classDiagram` / `stateDiagram`).
fn is_colon_label_dialect(text: &str) -> bool {
    for line in text.split(JS_LINE_TERMINATORS) {
        let trimmed = line.trim_start_matches(is_js_space);
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        return trimmed
            .strip_prefix("classDiagram")
            .or_else(|| trimmed.strip_prefix("stateDiagram"))
            .is_some_and(|after| {
                after
                    .chars()
                    .next()
                    .is_none_or(|c| is_js_space(c) || c == '-')
            });
    }
    false
}

/// Accessibility opener matched at the start of a line whose following `{` or `:` appears after
/// a line break (`accDescr\n{ ... }`, `accDescr\n: ...`, `accTitle\n: ...`).
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum PendingAcc {
    #[default]
    None,
    Descr,
    Title,
}

/// Whether `rest` (at the first non-whitespace token of a line outside quotes) starts an
/// accessibility keyword (`accDescr` or `accTitle`, case-insensitive in `stateDiagram`).
fn strip_acc_keyword<'a>(rest: &'a str, keyword: &str, ci: bool) -> Option<&'a str> {
    if ci {
        strip_ascii_ci_word(rest, keyword)
    } else {
        rest.strip_prefix(keyword)
    }
}

/// Result of classifying an accessibility keyword at the start of a line.
enum AccLineStart {
    /// `accDescr { ... }` opened on this line.
    DescrBlock,
    /// `accTitle:` or `accDescr:` opened on this line (single-line plain text).
    PlainLine,
    /// Keyword alone on the line; `{` or `:` may follow on a later line (`\s*` in lexers).
    Pending(PendingAcc),
}

fn classify_acc_line_start(rest: &str, ci: bool) -> Option<AccLineStart> {
    if let Some(after) = strip_acc_keyword(rest, "accDescr", ci) {
        let after_ws = trim_horizontal_ws(after);
        if after_ws.starts_with('{') {
            return Some(AccLineStart::DescrBlock);
        }
        if after_ws.starts_with(':') {
            return Some(AccLineStart::PlainLine);
        }
        if after_ws.is_empty()
            || after_ws
                .chars()
                .next()
                .is_some_and(|c| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
        {
            return Some(AccLineStart::Pending(PendingAcc::Descr));
        }
    }
    if let Some(after) = strip_acc_keyword(rest, "accTitle", ci) {
        let after_ws = trim_horizontal_ws(after);
        if after_ws.starts_with(':') {
            return Some(AccLineStart::PlainLine);
        }
        if after_ws.is_empty()
            || after_ws
                .chars()
                .next()
                .is_some_and(|c| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
        {
            return Some(AccLineStart::Pending(PendingAcc::Title));
        }
    }
    None
}

/// Whether `rest` starts a `style` or `classDef` statement (keyword followed by whitespace).
fn starts_style_or_classdef(rest: &str) -> bool {
    rest.strip_prefix("style")
        .or_else(|| rest.strip_prefix("classDef"))
        .is_some_and(|after| after.chars().next().is_some_and(is_js_space))
}

/// Whether `rest` starts a `classDef` statement (keyword followed by whitespace).
fn starts_classdef(rest: &str) -> bool {
    rest.strip_prefix("classDef")
        .is_some_and(|after| after.chars().next().is_some_and(is_js_space))
}

/// Whether `rest` starts a `class` declaration statement (`class` followed by whitespace).
fn starts_class_stmt(rest: &str) -> bool {
    rest.strip_prefix("class")
        .is_some_and(|after| after.chars().next().is_some_and(is_js_space))
}

/// Whether the `:` at byte offset `at` in `src` is a standalone single colon (not part of `::` or
/// `:::`).
fn is_single_colon(src: &str, at: usize) -> bool {
    let bytes = src.as_bytes();
    (at == 0 || bytes[at - 1] != b':') && bytes.get(at + 1) != Some(&b':')
}

/// Progress through a `stateDiagram` `note` header (`note left/right of <id>` or floating
/// `note "..." as <id>`), which may span line breaks because `state/scanner.rs` uses `\s+` after
/// `note` and `\s*` around `as`, `<id>`, and `:`.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum StateNoteHeader {
    #[default]
    None,
    /// Saw `note\s+` (or `note` at line end); awaiting `left of`, `right of`, or `"`.
    AfterNote,
    /// Saw `left of` / `right of`; awaiting the target state ID (`[^:\n\s\-]+`).
    AfterSideOf,
    /// Saw target ID; awaiting either `\s*:...` (single-line note) or any other token (block note).
    AfterTargetId,
    /// Inside the quoted text of a floating note (`note "..." as <id>`).
    FloatingText,
    /// Immediately after the closing `"` of a floating note; awaiting `\s*as\s*`.
    FloatingAfterQuote,
    /// After `as` in a floating note; awaiting the first non-whitespace token of the alias line
    /// (`[^\n]*` runs to end of line as plain text).
    FloatingAsId,
}

/// Byte length of the `stateDiagram-v2` or `stateDiagram` header keyword when it starts `rest`
/// (ASCII case-insensitive) and is followed by whitespace (`state/scanner.rs:810-816`).
fn match_state_diagram_header(rest: &str) -> Option<usize> {
    for kw in ["stateDiagram-v2", "stateDiagram"] {
        if let Some(after) = strip_ascii_ci_word(rest, kw)
            && after.chars().next().is_some_and(is_js_space)
        {
            return Some(kw.len());
        }
    }
    None
}

/// Whether `rest` starts with the `as` keyword (ASCII case-insensitive) followed by EOF or a
/// non-word character (`state/scanner.rs:756`).
fn starts_state_as_keyword(rest: &str) -> bool {
    strip_ascii_ci_word(rest, "as").is_some_and(|after| !after.chars().next().is_some_and(is_word))
}

/// Whether a `#` immediately after `before` begins a `stateDiagram` line comment (at line start
/// or immediately after whitespace, `;`, `{`, or `}`).
fn is_state_comment_boundary(before: &str) -> bool {
    before
        .chars()
        .next_back()
        .is_none_or(|prev| is_js_space(prev) || matches!(prev, ';' | '{' | '}'))
}

/// Whether `rest` starts with the `note` keyword (ASCII case-insensitive) followed by whitespace
/// or line end (since line-by-line scanners strip the trailing newline).
fn starts_state_note_keyword(rest: &str) -> bool {
    strip_ascii_ci_word(rest, "note")
        .is_some_and(|after| after.chars().next().is_none_or(is_js_space))
}

/// Byte length of `left <hws> of` or `right <hws> of` at the start of `rest` (ASCII
/// case-insensitive, followed by EOF or a non-word character).
fn match_state_note_side_of(rest: &str) -> Option<usize> {
    let after_side =
        strip_ascii_ci_word(rest, "left").or_else(|| strip_ascii_ci_word(rest, "right"))?;
    let after_side_ws = trim_horizontal_ws(after_side);
    if after_side_ws.len() == after_side.len() {
        return None;
    }
    let after_of = strip_ascii_ci_word(after_side_ws, "of")?;
    if after_of.chars().next().is_some_and(is_word) {
        return None;
    }
    Some(rest.len() - after_of.len())
}

/// Byte length of a state note target ID (`[^:\n\s\-]+`, truncated before `%%`) at the start of
/// `rest`.
fn state_note_target_id_len(rest: &str) -> usize {
    let raw = rest
        .find(|c: char| {
            matches!(c, ':' | '\n' | '\r' | '\u{2028}' | '\u{2029}' | '-') || is_js_space(c)
        })
        .unwrap_or(rest.len());
    rest[..raw].find("%%").unwrap_or(raw)
}

/// Byte length of `end <ws> note` at the start of `rest` (case-insensitive, followed by EOF or a
/// non-word character, matching `state/scanner.rs`), if `rest` closes a multiline state note.
fn ends_multiline_state_note_len(rest: &str) -> Option<usize> {
    let after_end = strip_ascii_ci_word(rest, "end")?;
    let after_end_ws = trim_horizontal_ws(after_end);
    if after_end_ws.len() == after_end.len() {
        return None;
    }
    let after_note = strip_ascii_ci_word(after_end_ws, "note")?;
    if after_note.chars().next().is_some_and(is_word) {
        return None;
    }
    Some(rest.len() - after_note.len())
}

fn trim_horizontal_ws(s: &str) -> &str {
    s.trim_start_matches(|c: char| {
        is_js_space(c) && !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
    })
}

fn strip_ascii_ci_word<'a>(s: &'a str, word: &str) -> Option<&'a str> {
    let head = s.get(..word.len())?;
    if head.eq_ignore_ascii_case(word) {
        Some(&s[word.len()..])
    } else {
        None
    }
}

/// Dialect flags computed once from the diagram header for entity encoding and error remapping.
#[derive(Clone, Copy)]
struct EncodeDialects {
    allow_classdef_css_semi: bool,
    colon_label_dialect: bool,
    class_diagram_dialect: bool,
    state_diagram_dialect: bool,
}

impl EncodeDialects {
    fn of(text: &str) -> Self {
        let state_diagram_dialect = allows_classdef_semicolon_css(text);
        Self {
            allow_classdef_css_semi: state_diagram_dialect,
            colon_label_dialect: is_colon_label_dialect(text),
            class_diagram_dialect: is_class_diagram_dialect(text),
            state_diagram_dialect,
        }
    }
}

/// Multiline scanning state carried across lines during entity preprocessing.
#[derive(Clone, Copy, Default)]
struct MultilineScanState {
    in_quotes: bool,
    in_backticks: bool,
    in_acc_descr_block: bool,
    pending_acc: PendingAcc,
    note_header: StateNoteHeader,
    in_note_block: bool,
    class_body_depth: usize,
}

/// `encodeEntities`: the semicolon that ends a `style` or `classDef` line with a color is dropped,
/// and `#name;` or `#123;` become placeholders that the renderer decodes.
pub(crate) fn encode_entities(text: &str) -> String {
    if !text.contains('#') {
        return text.to_owned();
    }
    let dialects = EncodeDialects::of(text);
    let text = drop_color_semicolons(text, "style", dialects);
    let text = drop_color_semicolons(&text, "classDef", dialects);
    encode_hash_entities(&text, dialects)
}

/// Maps a [`ParseError`](crate::error::ParseError) produced on `encode_entities(cleaned)` back to
/// its 1-based column in `cleaned`. `encode_entities` never adds or removes line breaks, so
/// `error.line` already identifies the exact line in `cleaned`.
pub(crate) fn remap_encoded_error(
    cleaned: &str,
    line_offset: usize,
    mut error: crate::error::ParseError,
) -> crate::error::ParseError {
    if !cleaned.contains('#') {
        return error;
    }
    let Some(target_line) = error.line.checked_sub(1 + line_offset) else {
        return error;
    };
    let dialects = EncodeDialects::of(cleaned);
    let mut state = MultilineScanState::default();
    let mut lines = cleaned.split('\n');
    for _ in 0..target_line {
        let Some(prev_line) = lines.next() else {
            return error;
        };
        advance_quote_state(prev_line, &mut state, dialects);
    }
    let Some(orig_line) = lines.next() else {
        return error;
    };
    if !orig_line.contains('#') {
        return error;
    }
    error.column = remap_encoded_line_column(orig_line, error.column, state, dialects);
    error
}

/// Advances the multiline double-quote, backtick, `accDescr { ... }`, state `note ... end note`,
/// and `class ... { ... }` body state across a single line.
fn advance_quote_state(line: &str, state: &mut MultilineScanState, dialects: EncodeDialects) {
    let mut at_line_start = true;
    let mut stmt_start = true;
    let mut in_style_stmt = false;
    let mut in_class_stmt = false;
    let mut prev_was_alnum = false;
    let mut skip_until = 0usize;
    for (at, ch) in line.char_indices() {
        if at < skip_until {
            prev_was_alnum = false;
            continue;
        }
        if state.note_header == StateNoteHeader::FloatingText {
            if ch == '"' {
                state.note_header = StateNoteHeader::FloatingAfterQuote;
            }
            prev_was_alnum = false;
            continue;
        }
        if state.note_header != StateNoteHeader::None {
            if is_js_space(ch) {
                prev_was_alnum = false;
                continue;
            }
            match state.note_header {
                StateNoteHeader::AfterNote => {
                    if ch == '"' {
                        state.note_header = StateNoteHeader::FloatingText;
                        at_line_start = false;
                        stmt_start = false;
                        prev_was_alnum = false;
                        continue;
                    }
                    if let Some(len) = match_state_note_side_of(&line[at..]) {
                        state.note_header = StateNoteHeader::AfterSideOf;
                        at_line_start = false;
                        stmt_start = false;
                        skip_until = at + len;
                        prev_was_alnum = false;
                        continue;
                    }
                    state.note_header = StateNoteHeader::None;
                }
                StateNoteHeader::AfterSideOf => {
                    let id_len = state_note_target_id_len(&line[at..]);
                    if id_len > 0 {
                        state.note_header = StateNoteHeader::AfterTargetId;
                        at_line_start = false;
                        stmt_start = false;
                        skip_until = at + id_len;
                        prev_was_alnum = false;
                        continue;
                    }
                    state.note_header = StateNoteHeader::None;
                }
                StateNoteHeader::AfterTargetId => {
                    state.note_header = StateNoteHeader::None;
                    if ch == ':' {
                        break;
                    }
                    state.in_note_block = true;
                }
                StateNoteHeader::FloatingAfterQuote => {
                    if starts_state_as_keyword(&line[at..]) {
                        state.note_header = StateNoteHeader::FloatingAsId;
                        at_line_start = false;
                        stmt_start = false;
                        skip_until = at + 2;
                        prev_was_alnum = false;
                        continue;
                    }
                    state.note_header = StateNoteHeader::None;
                }
                StateNoteHeader::FloatingAsId => {
                    state.note_header = StateNoteHeader::None;
                    break;
                }
                StateNoteHeader::None | StateNoteHeader::FloatingText => {}
            }
        }
        if state.in_note_block {
            if at_line_start {
                if is_js_space(ch) {
                    continue;
                }
                at_line_start = false;
                if let Some(len) = ends_multiline_state_note_len(&line[at..]) {
                    state.in_note_block = false;
                    stmt_start = true;
                    in_style_stmt = false;
                    in_class_stmt = false;
                    skip_until = at + len;
                    prev_was_alnum = false;
                    continue;
                }
            }
            break;
        }
        if state.in_acc_descr_block {
            if ch == '}' {
                state.in_acc_descr_block = false;
                stmt_start = true;
                in_style_stmt = false;
                in_class_stmt = false;
            }
            prev_was_alnum = false;
            continue;
        }
        if state.class_body_depth > 0 {
            if ch == '{' {
                state.class_body_depth += 1;
            } else if ch == '}' {
                state.class_body_depth -= 1;
                if state.class_body_depth == 0 {
                    stmt_start = true;
                    in_style_stmt = false;
                    in_class_stmt = false;
                }
            }
            prev_was_alnum = false;
            continue;
        }
        if state.in_quotes {
            if ch == '"' {
                state.in_quotes = false;
            }
            prev_was_alnum = false;
            continue;
        }
        if state.in_backticks {
            if ch == '`' {
                state.in_backticks = false;
            }
            prev_was_alnum = false;
            continue;
        }
        if at_line_start {
            if is_js_space(ch) {
                prev_was_alnum = false;
                continue;
            }
            at_line_start = false;
            if state.pending_acc != PendingAcc::None {
                let pending = state.pending_acc;
                state.pending_acc = PendingAcc::None;
                if pending == PendingAcc::Descr && ch == '{' {
                    if !line[at + 1..].contains('}') {
                        state.in_acc_descr_block = true;
                    }
                    break;
                }
                if ch == ':' {
                    break;
                }
            }
            let rest = &line[at..];
            if rest.starts_with("%%") {
                break;
            }
            match classify_acc_line_start(rest, dialects.state_diagram_dialect) {
                Some(AccLineStart::DescrBlock) => {
                    if let Some(brace) = rest.find('{')
                        && !rest[brace + 1..].contains('}')
                    {
                        state.in_acc_descr_block = true;
                    }
                    break;
                }
                Some(AccLineStart::PlainLine) => break,
                Some(AccLineStart::Pending(pending)) => {
                    state.pending_acc = pending;
                    break;
                }
                None => {}
            }
        }
        if ch == '%' && line.as_bytes().get(at + 1) == Some(&b'%') {
            break;
        }
        if dialects.state_diagram_dialect
            && !in_style_stmt
            && ch == '#'
            && is_state_comment_boundary(&line[..at])
        {
            break;
        }
        if ch == '"' && !prev_was_alnum {
            state.in_quotes = true;
            stmt_start = false;
            prev_was_alnum = false;
            continue;
        }
        if dialects.class_diagram_dialect && ch == '`' {
            state.in_backticks = true;
            stmt_start = false;
            prev_was_alnum = false;
            continue;
        }
        if dialects.state_diagram_dialect
            && !in_style_stmt
            && matches!(ch, 'n' | 'N')
            && !line[..at].chars().next_back().is_some_and(is_word)
            && starts_state_note_keyword(&line[at..])
        {
            state.note_header = StateNoteHeader::AfterNote;
            stmt_start = false;
            skip_until = at + 4;
            prev_was_alnum = false;
            continue;
        }
        prev_was_alnum = ch.is_ascii_alphanumeric();
        if ch == ';' {
            stmt_start = true;
            in_style_stmt = false;
            in_class_stmt = false;
            continue;
        }
        if dialects.state_diagram_dialect && !in_style_stmt && matches!(ch, '{' | '}') {
            stmt_start = true;
            in_style_stmt = false;
            in_class_stmt = false;
            continue;
        }
        if stmt_start {
            if is_js_space(ch) {
                continue;
            }
            let rest = &line[at..];
            if dialects.state_diagram_dialect
                && let Some(len) = match_state_diagram_header(rest)
            {
                skip_until = at + len;
                prev_was_alnum = false;
                continue;
            }
            stmt_start = false;
            match classify_acc_line_start(rest, dialects.state_diagram_dialect) {
                Some(AccLineStart::DescrBlock) => {
                    if let Some(brace) = rest.find('{')
                        && !rest[brace + 1..].contains('}')
                    {
                        state.in_acc_descr_block = true;
                    }
                    break;
                }
                Some(AccLineStart::PlainLine) => break,
                Some(AccLineStart::Pending(pending)) => {
                    state.pending_acc = pending;
                    break;
                }
                None => {}
            }
            in_style_stmt = starts_style_or_classdef(rest);
            in_class_stmt = dialects.class_diagram_dialect && starts_class_stmt(rest);
        }
        if dialects.class_diagram_dialect && in_class_stmt && ch == '{' {
            state.class_body_depth = 1;
            in_class_stmt = false;
            prev_was_alnum = false;
            continue;
        }
        if dialects.colon_label_dialect && !in_style_stmt && ch == ':' && is_single_colon(line, at)
        {
            break;
        }
    }
}

/// JavaScript line terminators that `.` in a regex does not cross (`\n`, `\r`, U+2028, U+2029).
const JS_LINE_TERMINATORS: [char; 4] = ['\n', '\r', '\u{2028}', '\u{2029}'];

/// Maps a 1-based column in `encode_entities(orig_line)` back to its 1-based column in `orig_line`,
/// given the multiline scan state carried in from preceding lines.
fn remap_encoded_line_column(
    orig_line: &str,
    encoded_column: usize,
    initial_state: MultilineScanState,
    dialects: EncodeDialects,
) -> usize {
    let mut chars: Vec<(char, usize)> = orig_line
        .chars()
        .enumerate()
        .map(|(col, ch)| (ch, col))
        .collect();
    for word in ["style", "classDef"] {
        let current: String = chars.iter().map(|&(c, _)| c).collect();
        let mut to_remove = Vec::new();
        let mut char_offset = 0usize;
        let mut state = initial_state;
        for segment in current.split_inclusive(JS_LINE_TERMINATORS) {
            let content = segment.strip_suffix(JS_LINE_TERMINATORS).unwrap_or(segment);
            if let Some(semi_byte) = color_semicolon_byte(content, word, &mut state, dialects) {
                to_remove.push(char_offset + content[..semi_byte].chars().count());
            }
            char_offset += segment.chars().count();
        }
        if !to_remove.is_empty() {
            let mut drop_iter = to_remove.into_iter().peekable();
            let mut idx = 0usize;
            chars.retain(|_| {
                let keep = if drop_iter.peek() == Some(&idx) {
                    drop_iter.next();
                    false
                } else {
                    true
                };
                idx += 1;
                keep
            });
        }
    }
    let current: String = chars.iter().map(|&(c, _)| c).collect();
    let mut mapped_cols: Vec<usize> = Vec::with_capacity(chars.len());
    let mut tracker = StyleStmtTracker::with_state(initial_state, dialects);
    let mut char_idx = 0usize;
    let mut byte_idx = 0usize;
    while char_idx < chars.len() {
        let (ch, orig_col) = chars[char_idx];
        if ch == '#' {
            let after = &current[byte_idx + 1..];
            let name_len = after
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(after.len());
            if name_len > 0 && after[name_len..].starts_with(';') {
                let name = &after[..name_len];
                if tracker.allows_hex_color(name) {
                    let rest_after_semi = &after[name_len + 1..];
                    let keep_len = if is_horizontal_ws_to_eol(rest_after_semi) {
                        1 + name_len
                    } else {
                        1 + name_len + 1
                    };
                    for &(_, c_col) in &chars[char_idx..char_idx + keep_len] {
                        mapped_cols.push(c_col);
                    }
                    tracker.after_hex_semicolon(rest_after_semi);
                    char_idx += 1 + name_len + 1;
                    byte_idx += 1 + name_len + 1;
                    continue;
                }
                let prefix_chars = if name.bytes().all(|b| b.is_ascii_digit()) {
                    "ﬂ°°".chars().count()
                } else {
                    "ﬂ°".chars().count()
                };
                let replacement_chars = prefix_chars + name_len + "¶ß".chars().count();
                mapped_cols.extend(std::iter::repeat_n(orig_col, replacement_chars));
                tracker.finish_entity();
                // `#`, ASCII `name`, and `;` are all single-byte ASCII characters.
                char_idx += 1 + name_len + 1;
                byte_idx += 1 + name_len + 1;
                continue;
            }
        }
        tracker.push(&current, byte_idx, ch);
        mapped_cols.push(orig_col);
        char_idx += 1;
        byte_idx += ch.len_utf8();
    }
    let target = encoded_column.saturating_sub(1);
    mapped_cols
        .get(target)
        .map_or_else(|| orig_line.chars().count() + 1, |&orig| orig + 1)
}

/// `decodeEntities` (`utils.ts:904-906`): the placeholders that `encode_entities` writes become
/// `&#`, `&` and `;` again. The reference decodes them in labels and in the whole SVG string once
/// it is rendered (`mermaidAPI.ts:359`), so a placeholder the user typed is decoded as well.
#[must_use]
pub fn decode_entities(text: &str) -> String {
    if !text.contains("ﬂ°") && !text.contains("¶ß") {
        return text.to_owned();
    }
    let mut out = if text.contains("ﬂ°°") {
        text.replace("ﬂ°°", "&#")
    } else {
        text.to_owned()
    };
    if out.contains("ﬂ°") {
        out = out.replace("ﬂ°", "&");
    }
    if out.contains("¶ß") {
        out = out.replace("¶ß", ";");
    }
    out
}

/// Whether the next semicolon-bounded statement in `rest` (which is already on a single line)
/// starts with a lowercase ASCII CSS property declaration (`<name>:` not followed by another `:`,
/// with at least two characters and no diagram edge/node operators inside that single statement),
/// such as `color:#abcdef` in `classDef late fill:#123456;color:#abcdef`. Bounds its scan to the
/// next `;` so single-line diagrams with many declarations stay strictly O(n).
fn starts_with_css_property(rest: &str) -> bool {
    let trimmed = rest.trim_start_matches(is_js_space);
    let mut letter_count = 0usize;
    let mut after_colon_start = None;
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_lowercase() {
            letter_count += 1;
        } else if ch == '-' && letter_count > 0 {
            continue;
        } else {
            if letter_count >= 2 && ch == ':' && !trimmed[idx + 1..].starts_with(':') {
                after_colon_start = Some(idx + 1);
            }
            break;
        }
    }
    let Some(after_idx) = after_colon_start else {
        return false;
    };
    let tail = &trimmed[after_idx..];
    let stmt_end = tail
        .find([';', '\n', '\r', '\u{2028}', '\u{2029}'])
        .unwrap_or(tail.len());
    let after_colon = &tail[..stmt_end];
    ![
        "-->", "---", "==>", "-.", "<--", "~~~", ":::", "o--", "x--", "--o", "--x", "[", "{",
    ]
    .iter()
    .any(|op| after_colon.contains(op))
}

/// Byte offset of the color-terminating semicolon to drop at end-of-line for a `style` or
/// `classDef` declaration (`<word> .*:\S*#.*;`), if any. Only matches `word` followed by whitespace
/// at the start of an unquoted statement (line start or after an unquoted `;`), skips non-hex
/// entities like `#44;` inside CSS values (such as `rgb(255#44;0#44;0);`), and preserves interior
/// semicolons between semicolon-separated CSS properties on `stateDiagram` `classDef` lines.
fn color_semicolon_byte(
    line: &str,
    word: &str,
    state: &mut MultilineScanState,
    dialects: EncodeDialects,
) -> Option<usize> {
    let mut at_line_start = true;
    let mut stmt_start = true;
    let mut in_target_stmt = false;
    let mut in_style_or_classdef = false;
    let mut in_class_stmt = false;
    let mut seen_colon = false;
    let mut seen_colon_hash = false;
    let mut prev_was_alnum = false;
    let mut skip_until = 0usize;
    let mut last_semi = None;
    let mut chars = line.char_indices();

    while let Some((at, ch)) = chars.next() {
        if at < skip_until {
            prev_was_alnum = false;
            continue;
        }
        if state.note_header == StateNoteHeader::FloatingText {
            if ch == '"' {
                state.note_header = StateNoteHeader::FloatingAfterQuote;
            }
            prev_was_alnum = false;
            continue;
        }
        if state.note_header != StateNoteHeader::None {
            if is_js_space(ch) {
                prev_was_alnum = false;
                continue;
            }
            match state.note_header {
                StateNoteHeader::AfterNote => {
                    if ch == '"' {
                        state.note_header = StateNoteHeader::FloatingText;
                        at_line_start = false;
                        stmt_start = false;
                        prev_was_alnum = false;
                        continue;
                    }
                    if let Some(len) = match_state_note_side_of(&line[at..]) {
                        state.note_header = StateNoteHeader::AfterSideOf;
                        at_line_start = false;
                        stmt_start = false;
                        skip_until = at + len;
                        prev_was_alnum = false;
                        continue;
                    }
                    state.note_header = StateNoteHeader::None;
                }
                StateNoteHeader::AfterSideOf => {
                    let id_len = state_note_target_id_len(&line[at..]);
                    if id_len > 0 {
                        state.note_header = StateNoteHeader::AfterTargetId;
                        at_line_start = false;
                        stmt_start = false;
                        skip_until = at + id_len;
                        prev_was_alnum = false;
                        continue;
                    }
                    state.note_header = StateNoteHeader::None;
                }
                StateNoteHeader::AfterTargetId => {
                    state.note_header = StateNoteHeader::None;
                    if ch == ':' {
                        break;
                    }
                    state.in_note_block = true;
                }
                StateNoteHeader::FloatingAfterQuote => {
                    if starts_state_as_keyword(&line[at..]) {
                        state.note_header = StateNoteHeader::FloatingAsId;
                        at_line_start = false;
                        stmt_start = false;
                        skip_until = at + 2;
                        prev_was_alnum = false;
                        continue;
                    }
                    state.note_header = StateNoteHeader::None;
                }
                StateNoteHeader::FloatingAsId => {
                    state.note_header = StateNoteHeader::None;
                    break;
                }
                StateNoteHeader::None | StateNoteHeader::FloatingText => {}
            }
        }
        if state.in_note_block {
            if at_line_start {
                if is_js_space(ch) {
                    continue;
                }
                at_line_start = false;
                if let Some(len) = ends_multiline_state_note_len(&line[at..]) {
                    state.in_note_block = false;
                    stmt_start = true;
                    in_target_stmt = false;
                    in_style_or_classdef = false;
                    in_class_stmt = false;
                    seen_colon = false;
                    seen_colon_hash = false;
                    skip_until = at + len;
                    prev_was_alnum = false;
                    continue;
                }
            }
            break;
        }
        if state.in_acc_descr_block {
            if ch == '}' {
                state.in_acc_descr_block = false;
                stmt_start = true;
                in_style_or_classdef = false;
                in_class_stmt = false;
            }
            prev_was_alnum = false;
            continue;
        }
        if state.class_body_depth > 0 {
            if ch == '{' {
                state.class_body_depth += 1;
            } else if ch == '}' {
                state.class_body_depth -= 1;
                if state.class_body_depth == 0 {
                    stmt_start = true;
                    in_style_or_classdef = false;
                    in_class_stmt = false;
                }
            }
            prev_was_alnum = false;
            continue;
        }
        if state.in_quotes {
            if ch == '"' {
                state.in_quotes = false;
            }
            prev_was_alnum = false;
            continue;
        }
        if state.in_backticks {
            if ch == '`' {
                state.in_backticks = false;
            }
            prev_was_alnum = false;
            continue;
        }
        if at_line_start {
            if is_js_space(ch) {
                prev_was_alnum = false;
                continue;
            }
            at_line_start = false;
            if state.pending_acc != PendingAcc::None {
                let pending = state.pending_acc;
                state.pending_acc = PendingAcc::None;
                if pending == PendingAcc::Descr && ch == '{' {
                    if !line[at + 1..].contains('}') {
                        state.in_acc_descr_block = true;
                    }
                    break;
                }
                if ch == ':' {
                    break;
                }
            }
            let rest = &line[at..];
            if rest.starts_with("%%") {
                break;
            }
            match classify_acc_line_start(rest, dialects.state_diagram_dialect) {
                Some(AccLineStart::DescrBlock) => {
                    if let Some(brace) = rest.find('{')
                        && !rest[brace + 1..].contains('}')
                    {
                        state.in_acc_descr_block = true;
                    }
                    break;
                }
                Some(AccLineStart::PlainLine) => break,
                Some(AccLineStart::Pending(pending)) => {
                    state.pending_acc = pending;
                    break;
                }
                None => {}
            }
        }
        if ch == '%' && line.as_bytes().get(at + 1) == Some(&b'%') {
            break;
        }
        if dialects.state_diagram_dialect
            && !in_style_or_classdef
            && ch == '#'
            && is_state_comment_boundary(&line[..at])
        {
            break;
        }
        if ch == '"' && !prev_was_alnum {
            state.in_quotes = true;
            stmt_start = false;
            prev_was_alnum = false;
            if in_target_stmt {
                seen_colon = false;
            }
            continue;
        }
        if dialects.class_diagram_dialect && ch == '`' {
            state.in_backticks = true;
            stmt_start = false;
            prev_was_alnum = false;
            if in_target_stmt {
                seen_colon = false;
            }
            continue;
        }
        if dialects.state_diagram_dialect
            && !in_style_or_classdef
            && matches!(ch, 'n' | 'N')
            && !line[..at].chars().next_back().is_some_and(is_word)
            && starts_state_note_keyword(&line[at..])
        {
            state.note_header = StateNoteHeader::AfterNote;
            stmt_start = false;
            skip_until = at + 4;
            prev_was_alnum = false;
            continue;
        }
        prev_was_alnum = ch.is_ascii_alphanumeric();
        if ch == ';' {
            let after = &line[at + 1..];
            if in_target_stmt && seen_colon_hash && is_horizontal_ws_to_eol(after) {
                last_semi = Some(at);
            }
            if in_target_stmt
                && dialects.allow_classdef_css_semi
                && word == "classDef"
                && starts_with_css_property(after)
            {
                // Preserve interior property separators on `stateDiagram` `classDef` lines and
                // keep the statement active for any trailing color or EOL terminator.
                seen_colon = false;
                continue;
            }
            stmt_start = true;
            in_target_stmt = false;
            in_style_or_classdef = false;
            in_class_stmt = false;
            seen_colon = false;
            seen_colon_hash = false;
            continue;
        }
        if dialects.state_diagram_dialect && !in_style_or_classdef && matches!(ch, '{' | '}') {
            stmt_start = true;
            in_target_stmt = false;
            in_style_or_classdef = false;
            in_class_stmt = false;
            seen_colon = false;
            seen_colon_hash = false;
            continue;
        }
        if stmt_start {
            if is_js_space(ch) {
                continue;
            }
            let rest = &line[at..];
            if dialects.state_diagram_dialect
                && let Some(len) = match_state_diagram_header(rest)
            {
                skip_until = at + len;
                prev_was_alnum = false;
                continue;
            }
            stmt_start = false;
            match classify_acc_line_start(rest, dialects.state_diagram_dialect) {
                Some(AccLineStart::DescrBlock) => {
                    if let Some(brace) = rest.find('{')
                        && !rest[brace + 1..].contains('}')
                    {
                        state.in_acc_descr_block = true;
                    }
                    break;
                }
                Some(AccLineStart::PlainLine) => break,
                Some(AccLineStart::Pending(pending)) => {
                    state.pending_acc = pending;
                    break;
                }
                None => {}
            }
            in_style_or_classdef = starts_style_or_classdef(rest);
            in_class_stmt = dialects.class_diagram_dialect && starts_class_stmt(rest);
            if let Some(after) = rest.strip_prefix(word)
                && after.chars().next().is_some_and(is_js_space)
            {
                in_target_stmt = true;
            }
            continue;
        }
        if dialects.class_diagram_dialect && in_class_stmt && ch == '{' {
            state.class_body_depth = 1;
            in_class_stmt = false;
            prev_was_alnum = false;
            continue;
        }
        if dialects.colon_label_dialect
            && !in_style_or_classdef
            && ch == ':'
            && is_single_colon(line, at)
        {
            break;
        }
        if in_target_stmt {
            if ch == ':' {
                seen_colon = true;
            } else if is_js_space(ch) {
                seen_colon = false;
            } else if ch == '#' {
                if seen_colon {
                    seen_colon_hash = true;
                }
                let after = &line[at + 1..];
                let name_len = after
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(after.len());
                if name_len > 0 && after[name_len..].starts_with(';') {
                    let name = &after[..name_len];
                    let is_hex_color = matches!(name.len(), 3 | 4 | 6 | 8)
                        && name.bytes().all(|b| b.is_ascii_hexdigit());
                    if !is_hex_color {
                        for _ in 0..=name_len {
                            let _ = chars.next();
                        }
                        prev_was_alnum = false;
                    }
                }
            }
        }
    }
    last_semi
}

/// `<word> .*:\S*#.*;` with the color-terminating semicolon removed on each JavaScript line.
fn drop_color_semicolons(text: &str, word: &str, dialects: EncodeDialects) -> String {
    let mut out = String::with_capacity(text.len());
    let mut state = MultilineScanState::default();
    for line in text.split_inclusive(JS_LINE_TERMINATORS) {
        let content = line.strip_suffix(JS_LINE_TERMINATORS).unwrap_or(line);
        if let Some(semicolon) = color_semicolon_byte(content, word, &mut state, dialects) {
            out.push_str(&line[..semicolon]);
            out.push_str(&line[semicolon + 1..]);
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Whether `rest` contains only horizontal whitespace up to the next JS line terminator or EOF.
/// Stops at the first non-whitespace character so repeated semicolon-separated statements stay O(n).
fn is_horizontal_ws_to_eol(rest: &str) -> bool {
    for ch in rest.chars() {
        if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            return true;
        }
        if !is_js_space(ch) {
            return false;
        }
    }
    true
}

/// Classification of the current semicolon-delimited statement on a line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StmtKind {
    Start,
    Style,
    Other,
}

/// Single-pass O(n) forward tracker for whether the current `#<hex>;` sits inside an unquoted
/// `style` or `classDef` declaration value (after `:` and a CSS token separator), resetting
/// statement state on every line terminator and unquoted `;` while preserving quote context across
/// multiline quoted labels and ignoring literal quotes inside `accDescr { ... }` blocks,
/// `class ... { ... }` member blocks, and single-line `:` labels in `classDiagram` / `stateDiagram`.
struct StyleStmtTracker {
    kind: StmtKind,
    in_classdef: bool,
    in_class_stmt: bool,
    seen_colon: bool,
    last_allows_hex: bool,
    at_line_start: bool,
    in_plain_line: bool,
    note_end_until: usize,
    state: MultilineScanState,
    prev_was_alnum: bool,
    dialects: EncodeDialects,
}

impl StyleStmtTracker {
    fn new(dialects: EncodeDialects) -> Self {
        Self::with_state(MultilineScanState::default(), dialects)
    }

    fn with_state(state: MultilineScanState, dialects: EncodeDialects) -> Self {
        Self {
            kind: StmtKind::Start,
            in_classdef: false,
            in_class_stmt: false,
            seen_colon: false,
            last_allows_hex: false,
            at_line_start: true,
            in_plain_line: false,
            note_end_until: 0,
            state,
            prev_was_alnum: false,
            dialects,
        }
    }

    fn reset(&mut self) {
        self.kind = StmtKind::Start;
        self.in_classdef = false;
        self.in_class_stmt = false;
        self.seen_colon = false;
        self.last_allows_hex = false;
    }

    fn after_hex_semicolon(&mut self, rest_after_semi: &str) {
        if self.dialects.allow_classdef_css_semi
            && self.in_classdef
            && starts_with_css_property(rest_after_semi)
        {
            self.seen_colon = false;
            self.last_allows_hex = false;
        } else {
            self.reset();
        }
    }

    fn reset_line(&mut self) {
        self.reset();
        self.at_line_start = true;
        self.in_plain_line = false;
        self.note_end_until = 0;
        self.prev_was_alnum = false;
    }

    fn finish_entity(&mut self) {
        match self.state.note_header {
            StateNoteHeader::AfterNote | StateNoteHeader::FloatingAfterQuote => {
                self.state.note_header = StateNoteHeader::None;
            }
            StateNoteHeader::AfterTargetId => {
                self.state.note_header = StateNoteHeader::None;
                self.state.in_note_block = true;
            }
            StateNoteHeader::FloatingAsId => {
                self.state.note_header = StateNoteHeader::None;
                self.in_plain_line = true;
            }
            StateNoteHeader::None
            | StateNoteHeader::AfterSideOf
            | StateNoteHeader::FloatingText => {}
        }
        if self.kind == StmtKind::Start {
            self.kind = StmtKind::Other;
        }
        self.at_line_start = false;
        self.last_allows_hex = false;
        self.prev_was_alnum = false;
    }

    fn push(&mut self, src: &str, at: usize, ch: char) {
        if at < self.note_end_until {
            if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                self.reset_line();
            } else {
                self.prev_was_alnum = false;
            }
            return;
        }
        if self.state.note_header == StateNoteHeader::FloatingText {
            if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                self.reset_line();
            } else if ch == '"' {
                self.state.note_header = StateNoteHeader::FloatingAfterQuote;
            }
            self.prev_was_alnum = false;
            return;
        }
        if self.state.note_header != StateNoteHeader::None {
            if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                self.reset_line();
                return;
            }
            if is_js_space(ch) {
                self.prev_was_alnum = false;
                return;
            }
            match self.state.note_header {
                StateNoteHeader::AfterNote => {
                    if ch == '"' {
                        self.state.note_header = StateNoteHeader::FloatingText;
                        self.at_line_start = false;
                        self.kind = StmtKind::Other;
                        self.prev_was_alnum = false;
                        return;
                    }
                    if let Some(len) = match_state_note_side_of(&src[at..]) {
                        self.state.note_header = StateNoteHeader::AfterSideOf;
                        self.at_line_start = false;
                        self.kind = StmtKind::Other;
                        self.note_end_until = at + len;
                        self.prev_was_alnum = false;
                        return;
                    }
                    self.state.note_header = StateNoteHeader::None;
                }
                StateNoteHeader::AfterSideOf => {
                    let id_len = state_note_target_id_len(&src[at..]);
                    if id_len > 0 {
                        self.state.note_header = StateNoteHeader::AfterTargetId;
                        self.at_line_start = false;
                        self.kind = StmtKind::Other;
                        self.note_end_until = at + id_len;
                        self.prev_was_alnum = false;
                        return;
                    }
                    self.state.note_header = StateNoteHeader::None;
                }
                StateNoteHeader::AfterTargetId => {
                    self.state.note_header = StateNoteHeader::None;
                    if ch == ':' {
                        self.in_plain_line = true;
                        self.kind = StmtKind::Other;
                        self.prev_was_alnum = false;
                        return;
                    }
                    self.state.in_note_block = true;
                }
                StateNoteHeader::FloatingAfterQuote => {
                    if starts_state_as_keyword(&src[at..]) {
                        self.state.note_header = StateNoteHeader::FloatingAsId;
                        self.at_line_start = false;
                        self.kind = StmtKind::Other;
                        self.note_end_until = at + 2;
                        self.prev_was_alnum = false;
                        return;
                    }
                    self.state.note_header = StateNoteHeader::None;
                }
                StateNoteHeader::FloatingAsId => {
                    self.state.note_header = StateNoteHeader::None;
                    self.in_plain_line = true;
                    self.kind = StmtKind::Other;
                    self.prev_was_alnum = false;
                    return;
                }
                StateNoteHeader::None | StateNoteHeader::FloatingText => {}
            }
        }
        if self.state.in_note_block {
            if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                self.reset_line();
            } else if self.at_line_start {
                if is_js_space(ch) {
                    return;
                }
                self.at_line_start = false;
                if let Some(len) = ends_multiline_state_note_len(&src[at..]) {
                    self.state.in_note_block = false;
                    self.reset();
                    self.note_end_until = at + len;
                }
            }
            self.prev_was_alnum = false;
            return;
        }
        if self.state.in_acc_descr_block {
            if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                self.reset_line();
            } else if ch == '}' {
                self.state.in_acc_descr_block = false;
                self.reset();
            }
            self.prev_was_alnum = false;
            return;
        }
        if self.state.class_body_depth > 0 {
            if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                self.reset_line();
            } else if ch == '{' {
                self.state.class_body_depth += 1;
            } else if ch == '}' {
                self.state.class_body_depth -= 1;
                if self.state.class_body_depth == 0 {
                    self.reset();
                }
            }
            self.prev_was_alnum = false;
            return;
        }
        if self.state.in_quotes {
            if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                self.reset_line();
            } else if ch == '"' {
                self.state.in_quotes = false;
            }
            self.prev_was_alnum = false;
            return;
        }
        if self.state.in_backticks {
            if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                self.reset_line();
            } else if ch == '`' {
                self.state.in_backticks = false;
            }
            self.prev_was_alnum = false;
            return;
        }
        if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            self.reset_line();
            return;
        }
        if self.in_plain_line {
            self.prev_was_alnum = false;
            return;
        }
        if self.at_line_start {
            if is_js_space(ch) {
                self.prev_was_alnum = false;
                return;
            }
            self.at_line_start = false;
            if self.state.pending_acc != PendingAcc::None {
                let pending = self.state.pending_acc;
                self.state.pending_acc = PendingAcc::None;
                if pending == PendingAcc::Descr && ch == '{' {
                    self.state.in_acc_descr_block = true;
                    self.kind = StmtKind::Other;
                    self.prev_was_alnum = false;
                    return;
                }
                if ch == ':' {
                    self.in_plain_line = true;
                    self.kind = StmtKind::Other;
                    self.prev_was_alnum = false;
                    return;
                }
            }
            let rest = &src[at..];
            if rest.starts_with("%%") {
                self.in_plain_line = true;
                self.kind = StmtKind::Other;
                self.prev_was_alnum = false;
                return;
            }
            match classify_acc_line_start(rest, self.dialects.state_diagram_dialect) {
                Some(AccLineStart::DescrBlock) => {
                    self.state.in_acc_descr_block = true;
                    self.kind = StmtKind::Other;
                    self.prev_was_alnum = false;
                    return;
                }
                Some(AccLineStart::PlainLine) => {
                    self.in_plain_line = true;
                    self.kind = StmtKind::Other;
                    self.prev_was_alnum = false;
                    return;
                }
                Some(AccLineStart::Pending(pending)) => {
                    self.state.pending_acc = pending;
                    self.in_plain_line = true;
                    self.kind = StmtKind::Other;
                    self.prev_was_alnum = false;
                    return;
                }
                None => {}
            }
        }
        if ch == '%' && src.as_bytes().get(at + 1) == Some(&b'%') {
            self.in_plain_line = true;
            self.prev_was_alnum = false;
            return;
        }
        if self.dialects.state_diagram_dialect
            && self.kind != StmtKind::Style
            && ch == '#'
            && is_state_comment_boundary(&src[..at])
        {
            self.in_plain_line = true;
            self.prev_was_alnum = false;
            return;
        }
        if ch == '"' && !self.prev_was_alnum {
            self.state.in_quotes = true;
            if self.kind == StmtKind::Start {
                self.kind = StmtKind::Other;
            }
            self.last_allows_hex = false;
            self.prev_was_alnum = false;
            return;
        }
        if self.dialects.class_diagram_dialect && ch == '`' {
            self.state.in_backticks = true;
            if self.kind == StmtKind::Start {
                self.kind = StmtKind::Other;
            }
            self.last_allows_hex = false;
            self.prev_was_alnum = false;
            return;
        }
        if self.dialects.state_diagram_dialect
            && self.kind != StmtKind::Style
            && matches!(ch, 'n' | 'N')
            && !src[..at].chars().next_back().is_some_and(is_word)
            && starts_state_note_keyword(&src[at..])
        {
            self.state.note_header = StateNoteHeader::AfterNote;
            self.kind = StmtKind::Other;
            self.note_end_until = at + 4;
            self.prev_was_alnum = false;
            return;
        }
        self.prev_was_alnum = ch.is_ascii_alphanumeric();
        if ch == ';' {
            if self.dialects.allow_classdef_css_semi
                && self.in_classdef
                && starts_with_css_property(&src[at + 1..])
            {
                self.seen_colon = false;
                self.last_allows_hex = false;
                return;
            }
            self.reset();
            return;
        }
        if self.dialects.state_diagram_dialect
            && self.kind != StmtKind::Style
            && matches!(ch, '{' | '}')
        {
            self.reset();
            return;
        }
        if self.kind == StmtKind::Start {
            if is_js_space(ch) {
                return;
            }
            let rest = &src[at..];
            if self.dialects.state_diagram_dialect
                && let Some(len) = match_state_diagram_header(rest)
            {
                self.note_end_until = at + len;
                self.prev_was_alnum = false;
                return;
            }
            match classify_acc_line_start(rest, self.dialects.state_diagram_dialect) {
                Some(AccLineStart::DescrBlock) => {
                    self.state.in_acc_descr_block = true;
                    self.kind = StmtKind::Other;
                    self.prev_was_alnum = false;
                    return;
                }
                Some(AccLineStart::PlainLine) => {
                    self.in_plain_line = true;
                    self.kind = StmtKind::Other;
                    self.prev_was_alnum = false;
                    return;
                }
                Some(AccLineStart::Pending(pending)) => {
                    self.state.pending_acc = pending;
                    self.in_plain_line = true;
                    self.kind = StmtKind::Other;
                    self.prev_was_alnum = false;
                    return;
                }
                None => {}
            }
            if starts_style_or_classdef(rest) {
                self.kind = StmtKind::Style;
                self.in_classdef = starts_classdef(rest);
            } else {
                self.kind = StmtKind::Other;
                self.in_class_stmt = self.dialects.class_diagram_dialect && starts_class_stmt(rest);
            }
        }
        if self.dialects.class_diagram_dialect && self.in_class_stmt && ch == '{' {
            self.state.class_body_depth = 1;
            self.in_class_stmt = false;
            self.prev_was_alnum = false;
            return;
        }
        if self.dialects.colon_label_dialect
            && self.kind == StmtKind::Other
            && ch == ':'
            && is_single_colon(src, at)
        {
            self.in_plain_line = true;
            self.prev_was_alnum = false;
            return;
        }
        if self.kind == StmtKind::Style {
            if ch == ':' {
                self.seen_colon = true;
            }
            self.last_allows_hex = ch == ':' || ch == ',' || ch == '(' || is_js_space(ch);
        }
    }

    fn allows_hex_color(&self, name: &str) -> bool {
        !self.state.in_quotes
            && !self.state.in_backticks
            && !self.in_plain_line
            && !self.state.in_acc_descr_block
            && self.state.note_header == StateNoteHeader::None
            && !self.state.in_note_block
            && self.state.class_body_depth == 0
            && self.kind == StmtKind::Style
            && self.seen_colon
            && self.last_allows_hex
            && matches!(name.len(), 3 | 4 | 6 | 8)
            && name.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

/// `#\w+;` becomes `ﬂ°°<digits>¶ß` for a number and `ﬂ°<name>¶ß` for a name, except for CSS hex
/// colors inside the current `style` / `classDef` statement.
fn encode_hash_entities(text: &str, dialects: EncodeDialects) -> String {
    let mut out = String::with_capacity(text.len());
    let mut tracker = StyleStmtTracker::new(dialects);
    let mut cursor = 0;
    let mut chars = text.char_indices();
    while let Some((at, ch)) = chars.next() {
        if ch == '#' {
            let after = &text[at + 1..];
            let name_len = after
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(after.len());
            if name_len > 0 && after[name_len..].starts_with(';') {
                let name = &after[..name_len];
                out.push_str(&text[cursor..at]);
                let end = at + 1 + name_len + 1;
                if tracker.allows_hex_color(name) {
                    let rest_after_semi = &after[name_len + 1..];
                    out.push('#');
                    out.push_str(name);
                    if !is_horizontal_ws_to_eol(rest_after_semi) {
                        out.push(';');
                    }
                    tracker.after_hex_semicolon(rest_after_semi);
                } else {
                    if name.bytes().all(|b| b.is_ascii_digit()) {
                        out.push_str("ﬂ°°");
                    } else {
                        out.push_str("ﬂ°");
                    }
                    out.push_str(name);
                    out.push_str("¶ß");
                    tracker.finish_entity();
                }
                cursor = end;
                for _ in 0..=name_len {
                    let _ = chars.next();
                }
                continue;
            }
        }
        tracker.push(text, at, ch);
    }
    out.push_str(&text[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_front_matter_and_reads_its_title() {
        let prepared = prepare("---\ntitle: Pets\nconfig:\n  theme: dark\n---\npie\n\"a\": 1")
            .expect("prepares");
        assert_eq!(prepared.text, "pie\n\"a\": 1");
        assert_eq!(prepared.title.as_deref(), Some("Pets"));
        assert_eq!(prepared.line_offset, 5);
    }

    #[test]
    fn normalizes_line_breaks_and_quotes_attributes() {
        let prepared = prepare("pie\r\n\"<b class=\"x\">a</b>\": 1\r").expect("prepares");
        assert_eq!(prepared.text, "pie\n\"<b class='x'>a</b>\": 1\n");
    }

    #[test]
    fn loose_directive_ends_at_its_closing_fence() {
        assert_eq!(loose_directive_len("%%{init: {}}%%\npie"), Some(14));
        assert_eq!(loose_directive_len("%%{init: a b}%%"), Some(11));
        assert_eq!(loose_directive_len("%%{ }%%"), None);
    }

    #[test]
    fn decoding_restores_the_references_the_encoding_replaced() {
        let encoded = encode_entities("a #9829; b #amp; c #x;y");
        assert_eq!(encoded, "a ﬂ°°9829¶ß b ﬂ°amp¶ß c ﬂ°x¶ßy");
        assert_eq!(decode_entities(&encoded), "a &#9829; b &amp; c &x;y");
    }

    #[test]
    fn state_classdef_keeps_interior_semicolons_and_hex_colors() {
        assert_eq!(
            encode_entities(
                "stateDiagram-v2\n  classDef hot stroke:#abc,stroke-width:2px;opacity:0.5;\n"
            ),
            "stateDiagram-v2\n  classDef hot stroke:#abc,stroke-width:2px;opacity:0.5\n"
        );
        assert_eq!(
            encode_entities("stateDiagram-v2\n  classDef hot fill:#abc;color:#def;opacity:0.5;\n"),
            "stateDiagram-v2\n  classDef hot fill:#abc;color:#def;opacity:0.5\n"
        );
    }

    #[test]
    fn class_member_quotes_do_not_leak_into_later_styles() {
        let input =
            "classDiagram\n  class A {\n    +char quote = '\"'\n  }\n  style A fill:#abc;\n";
        assert_eq!(
            encode_entities(input),
            "classDiagram\n  class A {\n    +char quote = '\"'\n  }\n  style A fill:#abc\n"
        );
    }

    #[test]
    fn state_midline_block_note_and_same_line_end_note_preserve_styles() {
        let input = "stateDiagram-v2\n  A --> B note right of A\n    \"draft\n  end note style A fill:#abc;\n";
        assert_eq!(
            encode_entities(input),
            "stateDiagram-v2\n  A --> B note right of A\n    \"draft\n  end note style A fill:#abc\n"
        );
        let semi =
            "stateDiagram-v2\n  note right of A\n    \"draft\n  end note; style A fill:#abc;\n";
        assert_eq!(
            encode_entities(semi),
            "stateDiagram-v2\n  note right of A\n    \"draft\n  end note; style A fill:#abc\n"
        );
    }

    #[test]
    fn class_backtick_names_do_not_leak_braces_or_quotes_into_later_styles() {
        let brace_input = "classDiagram\n  class `A {`\n  class B\n  style B fill:#abc;\n";
        assert_eq!(
            encode_entities(brace_input),
            "classDiagram\n  class `A {`\n  class B\n  style B fill:#abc\n"
        );
        let quote_input = "classDiagram\n  class `A \"x`\n  class B\n  style B fill:#abc;\n";
        assert_eq!(
            encode_entities(quote_input),
            "classDiagram\n  class `A \"x`\n  class B\n  style B fill:#abc\n"
        );
    }

    #[test]
    fn state_note_headers_split_across_lines_preserve_later_styles() {
        for input in [
            "stateDiagram-v2\n  A --> B\n  note\n    left of A\n    \"draft\n  end note\n  style A fill:#abc;\n",
            "stateDiagram-v2\n  A --> B\n  note right of\n    A\n    \"draft\n  end note\n  style A fill:#abc;\n",
            "stateDiagram-v2\n  A --> B\n  note\n    left of A\n    : \"single\n  style A fill:#abc;\n",
            "stateDiagram-v2\n  A --> B\n  note\n    \"floating\" as N\n  style A fill:#abc;\n",
        ] {
            let expected = input.replace("fill:#abc;", "fill:#abc");
            assert_eq!(encode_entities(input), expected, "input={input:?}");
        }
    }

    #[test]
    fn state_header_line_styles_and_floating_note_alias_quotes_preserve_hex_colors() {
        for input in [
            "stateDiagram-v2 classDef hot fill:#abc;\n  A:::hot\n",
            "stateDiagram style A fill:#abc;\n  A --> B\n",
            "stateDiagram-v2\n  A --> B\n  note \"foo\" as Some \"odd text\n  style A fill:#abc;\n",
            "stateDiagram-v2\n  A --> B\n  note \"foo\" as\n    Some \"odd text\n  style A fill:#abc;\n",
            "stateDiagram-v2\n  state S {\n    A --> B\n  } style C fill:#abc;\n",
            "stateDiagram-v2\n  state S { style A fill:#abc;\n  }\n",
            "stateDiagram-v2\n  A --> B\n  # draft \"only one quote\n  style A fill:#abc;\n",
            "stateDiagram-v2\n  A --> B # \"inline draft\n  style A fill:#abc;\n",
        ] {
            let expected = input.replace("fill:#abc;", "fill:#abc");
            assert_eq!(encode_entities(input), expected, "input={input:?}");
        }
    }
}
