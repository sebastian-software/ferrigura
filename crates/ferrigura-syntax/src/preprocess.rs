//! Text cleanup before lexing: the parts of `packages/mermaid/src/preprocess.ts` that decide what
//! the grammar sees.
//!
//! Only the front matter title is read here; the front matter `config` block is read by
//! `crate::config`, which needs the whole YAML document. Comments and directives are not removed
//! here: the lexer treats them as hidden tokens, as the grammars do.

use crate::text::{is_cr_lf, is_js_space, is_word};

/// Text prepared for the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Prepared {
    /// The text after front matter removal, with CRLF and CR normalised to LF.
    pub text: String,
    /// The `title` of the front matter, if it has a non-empty one.
    pub title: Option<String>,
    /// The YAML body of the front matter, dedented as `extractFrontMatter` does, if there is one.
    pub front_matter: Option<String>,
    /// Number of lines removed with the front matter, so positions can be reported against the
    /// original input.
    pub line_offset: usize,
}

/// Normalises line breaks and quotes HTML attributes, then removes the front matter.
pub(crate) fn prepare(raw: &str) -> Prepared {
    let normalized = quote_html_attributes(&raw.replace("\r\n", "\n").replace('\r', "\n"));
    let chars: Vec<char> = normalized.chars().collect();
    match match_front_matter(&chars) {
        Some(front_matter) => {
            let removed = &chars[..front_matter.length];
            Prepared {
                text: chars[front_matter.length..].iter().collect(),
                title: front_matter_title(&front_matter.body, &front_matter.indent),
                front_matter: Some(dedent(&front_matter.body, &front_matter.indent)),
                line_offset: removed.iter().filter(|&&c| c == '\n').count(),
            }
        }
        None => Prepared {
            text: normalized,
            title: None,
            front_matter: None,
            line_offset: 0,
        },
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

/// The `title` key of the front matter body, when it is a non-empty scalar.
///
/// Only the first top-level `title:` line is read, as a plain or quoted scalar. The reference
/// reads the whole YAML document with `js-yaml`; documents that rely on other YAML features in
/// `title` are a known gap until the front matter is ported in full.
fn front_matter_title(body: &str, indent: &str) -> Option<String> {
    for line in body.split('\n') {
        let line = line.strip_prefix(indent).unwrap_or(line);
        if let Some(value) = line.strip_prefix("title:") {
            return yaml_scalar(value.trim());
        }
    }
    None
}

fn yaml_scalar(value: &str) -> Option<String> {
    let text = if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        unescape_double_quoted(&value[1..value.len() - 1])
    } else if value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'') {
        value[1..value.len() - 1].replace("''", "'")
    } else {
        if matches!(value, "" | "null" | "~") {
            return None;
        }
        match value.find(" #") {
            Some(comment) => value[..comment].trim_end().to_owned(),
            None => value.to_owned(),
        }
    };
    (!text.is_empty()).then_some(text)
}

fn unescape_double_quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
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
    Some(match text[i..].find("}%%") {
        Some(close) => i + close + 3,
        None => text.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_front_matter_and_reads_its_title() {
        let prepared = prepare("---\ntitle: Pets\nconfig:\n  theme: dark\n---\npie\n\"a\": 1");
        assert_eq!(prepared.text, "pie\n\"a\": 1");
        assert_eq!(prepared.title.as_deref(), Some("Pets"));
        assert_eq!(prepared.line_offset, 5);
    }

    #[test]
    fn normalises_line_breaks_and_quotes_attributes() {
        let prepared = prepare("pie\r\n\"<b class=\"x\">a</b>\": 1\r");
        assert_eq!(prepared.text, "pie\n\"<b class='x'>a</b>\": 1\n");
    }

    #[test]
    fn loose_directive_ends_at_its_closing_fence() {
        assert_eq!(loose_directive_len("%%{init: {}}%%\npie"), Some(14));
        assert_eq!(loose_directive_len("%%{init: a b}%%"), Some(11));
        assert_eq!(loose_directive_len("%%{ }%%"), None);
    }
}
