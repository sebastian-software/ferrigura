//! Text cleanup before lexing: the parts of `packages/mermaid/src/preprocess.ts` that decide what
//! the grammar sees.
//!
//! The front matter is read here, with the YAML reader of `crate::yaml` that the reference's
//! `js-yaml` corresponds to, so a front matter that does not parse is an error for every diagram.
//! Comments and directives are not removed here: the lexer treats them as hidden tokens, as the
//! grammars do.

use crate::ParseError;
use crate::text::{is_cr_lf, is_js_space, is_word};
use crate::yaml::{self, Yaml};

/// Text prepared for the lexer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Prepared {
    /// The text after front matter removal, with CRLF and CR normalised to LF.
    pub text: String,
    /// The `title` of the front matter, if it is truthy and its string is not empty.
    pub title: Option<String>,
    /// The fields of the front matter's YAML mapping, in order. Empty when there is no front
    /// matter, or when its document is not a mapping, as `extractFrontMatter` treats it.
    pub front_matter: Vec<(String, Yaml)>,
    /// Number of lines removed with the front matter, so positions can be reported against the
    /// original input.
    pub line_offset: usize,
}

/// Normalises line breaks and quotes HTML attributes, then removes the front matter and reads it.
pub(crate) fn prepare(raw: &str) -> Result<Prepared, ParseError> {
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
    let fields = front_matter_fields(&dedent(&front_matter.body, &front_matter.indent))?;
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
fn front_matter_fields(body: &str) -> Result<Vec<(String, Yaml)>, ParseError> {
    match yaml::load(body) {
        Ok(Some(Yaml::Mapping(pairs))) => Ok(pairs),
        Ok(_) => Ok(Vec::new()),
        Err(error) => Err(ParseError {
            // The body starts on the line after the opening fence.
            line: error.line + 2,
            column: error.column + 1,
            message: format!("invalid front matter: {}", error.message),
        }),
    }
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
    for i in body_start..=last_closing_fence {
        if let Some(match_end) = closing_end(i) {
            return Some(FrontMatter {
                indent,
                body: text[body_start..i].iter().collect(),
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
    let mut line_start = 0;
    while line_start <= text.len() {
        if let Some(end) = comment_match_end(text, line_start) {
            out.push_str(&text[copied..line_start]);
            copied = end;
            line_start = end;
            continue;
        }
        match text[line_start..].find('\n') {
            Some(newline) => line_start += newline + 1,
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

/// `encodeEntities`: the semicolon that ends a `style` or `classDef` line with a colour is dropped,
/// and `#name;` or `#123;` become placeholders that the renderer decodes.
pub(crate) fn encode_entities(text: &str) -> String {
    let text = drop_colour_semicolons(text, "style");
    let text = drop_colour_semicolons(&text, "classDef");
    encode_hash_entities(&text)
}

/// `decodeEntities` (`utils.ts:904-906`): the placeholders that [`encode_entities`] writes become
/// `&#`, `&` and `;` again. The reference decodes them in labels and in the whole SVG string once
/// it is rendered (`mermaidAPI.ts:359`), so a placeholder the user typed is decoded as well.
#[must_use]
pub fn decode_entities(text: &str) -> String {
    text.replace("ﬂ°°", "&#")
        .replace("ﬂ°", "&")
        .replace("¶ß", ";")
}

/// `<word>.*:\S*#.*;` with the semicolon removed: the last semicolon of the line, when a colon
/// followed by non-blank text and a `#` come after the word on that line.
fn drop_colour_semicolons(text: &str, word: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    while let Some(found) = text[pos..].find(word) {
        let start = pos + found;
        let after_word = start + word.len();
        let line_end = text[after_word..]
            .find('\n')
            .map_or(text.len(), |newline| after_word + newline);
        let last_semicolon = text[after_word..line_end]
            .rfind(';')
            .map(|i| after_word + i);
        match last_semicolon {
            Some(semicolon) if has_colon_hash(&text[after_word..semicolon]) => {
                out.push_str(&text[pos..semicolon]);
                pos = semicolon + 1;
            }
            _ => {
                out.push_str(&text[pos..after_word]);
                pos = after_word;
            }
        }
    }
    out.push_str(&text[pos..]);
    out
}

/// Whether `segment` has a colon followed by blank-free text and a `#`: `.*:\S*#.*`.
fn has_colon_hash(segment: &str) -> bool {
    segment.char_indices().any(|(colon, c)| {
        c == ':'
            && segment[colon + 1..]
                .char_indices()
                .take_while(|&(_, c)| !is_js_space(c))
                .any(|(_, c)| c == '#')
    })
}

/// `#\w+;` becomes `ﬂ°°<digits>¶ß` for a number and `ﬂ°<name>¶ß` for a name.
fn encode_hash_entities(text: &str) -> String {
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
    fn normalises_line_breaks_and_quotes_attributes() {
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
