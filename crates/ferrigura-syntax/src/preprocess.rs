//! Text cleanup before lexing: the parts of `packages/mermaid/src/preprocess.ts` that decide what
//! the grammar sees.
//!
//! The front matter is read here, with the YAML reader of `crate::yaml` that the reference's
//! `js-yaml` corresponds to, so a front matter that does not parse is an error for every diagram.
//! Comments and directives are not removed here: the lexer treats them as hidden tokens, as the
//! grammars do.

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

/// Normalizes line breaks and quotes HTML attributes, then removes the front matter and reads it.
pub fn prepare(raw: &str) -> Result<Prepared, ParseError> {
    let normalized = quote_html_attributes(&raw.replace("\r\n", "\n").replace('\r', "\n"));
    let chars: Vec<char> = normalized.chars().collect();
    let Some(front_matter) = match_front_matter(&chars) else {
        return Ok(Prepared {
            text: normalized,
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
fn quote_html_attributes(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        if chars[i] == '<' {
            let name_end = (i + 1..n).find(|&k| !is_word(chars[k])).unwrap_or(n);
            if name_end > i + 1
                && let Some(close) = (name_end..n).find(|&k| chars[k] == '>')
            {
                out.push('<');
                out.extend(&chars[i + 1..name_end]);
                out.push_str(&quote_attributes(&chars[name_end..close]));
                out.push('>');
                i = close + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `replace(/="([^"]*)"/g, "='$1'")`.
fn quote_attributes(attributes: &[char]) -> String {
    let n = attributes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        if attributes[i] == '='
            && attributes.get(i + 1) == Some(&'"')
            && let Some(close) = (i + 2..n).find(|&k| attributes[k] == '"')
        {
            out.push_str("='");
            out.extend(&attributes[i + 2..close]);
            out.push('\'');
            i = close + 1;
            continue;
        }
        out.push(attributes[i]);
        i += 1;
    }
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
pub(crate) fn strip_front_matter(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    match match_front_matter(&chars) {
        Some(front_matter) => chars[front_matter.length..].iter().collect(),
        None => text.to_owned(),
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
pub(crate) fn remove_directives(text: &str) -> String {
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
    out
}

/// Removes the comment lines, as `cleanupComments` does: `^\s*%%(?!{)[^\n]+\n?` with the
/// multi-line flag, then the leading whitespace of the text.
pub(crate) fn cleanup_comments(text: &str) -> String {
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
    out.push_str(&text[copied..]);
    out.trim_start_matches(is_js_space).to_owned()
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

/// `encodeEntities`: the semicolon that ends a `style` or `classDef` line with a color is dropped,
/// and `#name;` or `#123;` become placeholders that the renderer decodes.
pub(crate) fn encode_entities(text: &str) -> String {
    if !text.contains('#') {
        return text.to_owned();
    }
    let text = drop_color_semicolons(text, "style");
    let text = drop_color_semicolons(&text, "classDef");
    encode_hash_entities(&text)
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
    let Some(orig_line) = cleaned.split('\n').nth(target_line) else {
        return error;
    };
    if !orig_line.contains('#') {
        return error;
    }
    error.column = remap_encoded_line_column(orig_line, error.column);
    error
}

/// JavaScript line terminators that `.` in a regex does not cross (`\n`, `\r`, U+2028, U+2029).
const JS_LINE_TERMINATORS: [char; 4] = ['\n', '\r', '\u{2028}', '\u{2029}'];

/// Maps a 1-based column in `encode_entities(orig_line)` back to its 1-based column in `orig_line`.
fn remap_encoded_line_column(orig_line: &str, encoded_column: usize) -> usize {
    let mut chars: Vec<(char, usize)> = orig_line
        .chars()
        .enumerate()
        .map(|(col, ch)| (ch, col))
        .collect();
    for word in ["style", "classDef"] {
        let current: String = chars.iter().map(|&(c, _)| c).collect();
        let mut to_remove = Vec::new();
        let mut char_offset = 0usize;
        for segment in current.split_inclusive(JS_LINE_TERMINATORS) {
            let content = segment.strip_suffix(JS_LINE_TERMINATORS).unwrap_or(segment);
            if let Some(semi_byte) = color_semicolon_byte(content, word) {
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
                let prefix_chars = if name.bytes().all(|b| b.is_ascii_digit()) {
                    "ﬂ°°".chars().count()
                } else {
                    "ﬂ°".chars().count()
                };
                let replacement_chars = prefix_chars + name_len + "¶ß".chars().count();
                mapped_cols.extend(std::iter::repeat_n(orig_col, replacement_chars));
                // `#`, ASCII `name`, and `;` are all single-byte ASCII characters.
                char_idx += 1 + name_len + 1;
                byte_idx += 1 + name_len + 1;
                continue;
            }
        }
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
    text.replace("ﬂ°°", "&#")
        .replace("ﬂ°", "&")
        .replace("¶ß", ";")
}

/// Byte offset of the color-terminating semicolon to drop on a single line for `<word>.*:\S*#.*;`,
/// if any. Because the match is greedy from the first occurrence of `word` to the last `;` on the
/// line, each line is scanned at most once (O(line length)).
fn color_semicolon_byte(line: &str, word: &str) -> Option<usize> {
    let found = line.find(word)?;
    let after_word = found + word.len();
    let rel_semi = line[after_word..].rfind(';')?;
    let semicolon = after_word + rel_semi;
    has_colon_hash(&line[after_word..semicolon]).then_some(semicolon)
}

/// `<word>.*:\S*#.*;` with the semicolon removed: the last semicolon of each JavaScript line, when
/// a colon followed by non-blank text and a `#` come after the first occurrence of `word` on that
/// line.
fn drop_color_semicolons(text: &str, word: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive(JS_LINE_TERMINATORS) {
        let content = line.strip_suffix(JS_LINE_TERMINATORS).unwrap_or(line);
        if let Some(semicolon) = color_semicolon_byte(content, word) {
            out.push_str(&line[..semicolon]);
            out.push_str(&line[semicolon + 1..]);
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Whether `segment` has a colon followed by blank-free text and a `#`: `.*:\S*#.*`.
fn has_colon_hash(segment: &str) -> bool {
    let mut seen_colon = false;
    for c in segment.chars() {
        if c == ':' {
            seen_colon = true;
        } else if is_js_space(c) {
            seen_colon = false;
        } else if seen_colon && c == '#' {
            return true;
        }
    }
    false
}

/// `#\w+;` becomes `ﬂ°°<digits>¶ß` for a number and `ﬂ°<name>¶ß` for a name.
pub(crate) fn encode_hash_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(hash) = rest.find('#') {
        out.push_str(&rest[..hash]);
        let after = &rest[hash + 1..];
        let name_len = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        if name_len > 0 && after[name_len..].starts_with(';') {
            let name = &after[..name_len];
            if name.bytes().all(|b| b.is_ascii_digit()) {
                out.push_str("ﬂ°°");
            } else {
                out.push_str("ﬂ°");
            }
            out.push_str(name);
            out.push_str("¶ß");
            rest = &after[name_len + 1..];
        } else {
            out.push('#');
            rest = after;
        }
    }
    out.push_str(rest);
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
}
