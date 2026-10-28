//! Labels as lines of words, on the SVG path: a port of `markdownToLines` and `nonMarkdownToLines` in
//! `packages/mermaid/src/rendering-util/handle-markdown-text.ts`, with its `preprocessMarkdown` and
//! `dedent` from ts-dedent 2.2.0.
//!
//! The reference parses Markdown with marked and reads its tokens. This port lexes with a port of
//! marked's lexer (`marked.rs`) and reads its tokens the same way, so that the lines and words are
//! the reference's:
//!
//! - Text is taken as written: the reference's text tokens are not escaped, so `&amp;`, `&copy;`
//!   and `"` stay as they are in the source, and the SVG step decodes what it decodes. A text token,
//!   which holds the text between two tokens that the reference drops, is split at newlines and at
//!   single spaces. Each piece is a word, and an empty piece is no word.
//! - A newline starts a new line, a space separates words, and `&#39;` becomes an apostrophe.
//! - Strong and emphasis give their words their type; the innermost one wins.
//! - Inline HTML is one word, as written, with no splitting.
//! - Escapes, code spans, links, images, deletions, autolinks and line breaks are dropped, with
//!   the text inside them, and each one ends the text before it.
//! - A top-level token other than a paragraph is one word: its raw source, or the HTML it holds.
//!
//! `nonMarkdownToLines` splits on `\n`, a literal `\n`, and `<br>` tags, and each line gives the
//! words that `\s` and angle brackets leave, where a tag is one word.

use super::marked::{self, Block, Inline};

/// The style of a word, as the reference's `MarkdownWordType`: strong, emphasis or normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownWordType {
    /// Text outside strong and emphasis.
    Normal,
    /// Text inside `**` or `__`.
    Strong,
    /// Text inside `*` or `_`.
    Em,
}

/// A word of a label line: its text as the reference keeps it, before the SVG step decodes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownWord {
    /// The text of the word.
    pub content: String,
    /// The style of the word.
    pub kind: MarkdownWordType,
}

/// One line of a label: its words, left to right.
pub type MarkdownLine = Vec<MarkdownWord>;

/// `markdownToLines`: the lines and words of a label written in Markdown. The text is preprocessed
/// as `preprocessMarkdown` does (`<br/>` is a newline, blank lines go, and the common indentation
/// goes), then parsed.
///
/// A label that Ferromark cannot parse, such as one nested too deeply, has no Markdown structure to
/// keep, so it is read as plain text, which is what the reference's tokens would show for it.
#[must_use]
pub fn markdown_to_lines(markdown: &str) -> Vec<MarkdownLine> {
    let source = preprocess_markdown(markdown);
    let mut builder = Builder {
        lines: vec![Vec::new()],
    };
    for block in marked::lex(&source) {
        match block {
            Block::Paragraph(tokens) => builder.inline(&tokens, MarkdownWordType::Normal),
            Block::Html(text) | Block::Other(text) => {
                builder.push_word(&text, MarkdownWordType::Normal);
            }
        }
    }
    builder.lines
}

/// `nonMarkdownToLines`: the lines and words of a label written as plain text.
#[must_use]
pub fn non_markdown_to_lines(text: &str) -> Vec<MarkdownLine> {
    split_on_breaks(text)
        .into_iter()
        .map(|line| {
            plain_words(js_trim(line))
                .into_iter()
                .map(|content| MarkdownWord {
                    content,
                    kind: MarkdownWordType::Normal,
                })
                .collect()
        })
        .collect()
}

/// `preprocessMarkdown` in `handle-markdown-text.ts`: `<br/>` becomes a newline, runs of newlines
/// become one, and the common indentation goes, as ts-dedent removes it.
#[must_use]
pub fn preprocess_markdown(markdown: &str) -> String {
    let without_br = markdown.replace("<br/>", "\n");
    dedent(&collapse_newlines(&without_br))
}

/// `replace(/\n{2,}/g, '\n')`: a run of newlines becomes one newline.
fn collapse_newlines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut after_newline = false;
    for character in text.chars() {
        if character == '\n' {
            if !after_newline {
                out.push('\n');
            }
            after_newline = true;
        } else {
            after_newline = false;
            out.push(character);
        }
    }
    out
}

/// The whitespace of JavaScript's `\s`, which is what `trim` and the plain-text splits use. It is
/// not Rust's `char::is_whitespace`: U+0085 is whitespace in Rust and not in JavaScript, and U+FEFF
/// is whitespace in JavaScript only.
fn is_js_space(character: char) -> bool {
    matches!(
        character,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

/// ts-dedent 2.2.0's `dedent` for a string: the trailing newline and the indentation after it go,
/// the common indentation of the lines after a newline goes, and a leading newline goes.
///
/// A line that starts without indentation counts as indented by zero, so the indentation goes only
/// when every line after a newline is indented.
fn dedent(text: &str) -> String {
    let mut text = text;
    if let Some(position) = text.rfind('\n') {
        let tail = &text[position + 1..];
        if tail
            .chars()
            .all(|character| character == ' ' || character == '\t')
        {
            let end = if position > 0 && text.as_bytes()[position - 1] == b'\r' {
                position - 1
            } else {
                position
            };
            text = &text[..end];
        }
    }

    let indents = indents_after_newlines(text);
    let mut dedented = match indents.iter().min() {
        Some(&minimum) if minimum > 0 => remove_indent_after_newlines(text, minimum),
        _ => text.to_owned(),
    };
    if let Some(rest) = dedented.strip_prefix("\r\n") {
        dedented = rest.to_owned();
    } else if let Some(rest) = dedented.strip_prefix('\n') {
        dedented = rest.to_owned();
    }
    dedented
}

/// The indentation after each newline, as ts-dedent's `/\n([\t ]+|(?!\s).)/g` matches count it: the
/// number of spaces and tabs, or zero when a newline is followed by a non-space character.
fn indents_after_newlines(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut indents = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\n' {
            index += 1;
            continue;
        }
        let after = index + 1;
        let mut end = after;
        while end < bytes.len() && (bytes[end] == b' ' || bytes[end] == b'\t') {
            end += 1;
        }
        if end > after {
            indents.push(end - after);
            index = end;
            continue;
        }
        match text[after..].chars().next() {
            Some(character) if !is_js_space(character) => {
                indents.push(0);
                index = after + character.len_utf8();
            }
            _ => index = after,
        }
    }
    indents
}

/// ts-dedent's `replace(/\n[\t ]{min}/g, '\n')`: removes `minimum` spaces or tabs after each newline
/// that has at least that many.
fn remove_indent_after_newlines(text: &str, minimum: usize) -> String {
    let mut pieces = text.split('\n');
    let mut out = String::with_capacity(text.len());
    if let Some(first) = pieces.next() {
        out.push_str(first);
    }
    for piece in pieces {
        out.push('\n');
        // Spaces and tabs are one byte each, so the byte index is a character boundary.
        let indent = piece
            .bytes()
            .take_while(|byte| *byte == b' ' || *byte == b'\t')
            .count();
        out.push_str(if indent >= minimum {
            &piece[minimum..]
        } else {
            piece
        });
    }
    out
}

/// Collects the lines of a document, as `markdownToLines`' `processNode` and its top-level loop do.
struct Builder {
    lines: Vec<MarkdownLine>,
}

impl Builder {
    /// The inline tokens of a paragraph, strong or emphasis: text gives words of the type around
    /// it, HTML one word, and every other token none.
    fn inline(&mut self, tokens: &[Inline], kind: MarkdownWordType) {
        for token in tokens {
            match token {
                Inline::Text(text) => self.text(text, kind),
                Inline::Strong(children) => self.inline(children, MarkdownWordType::Strong),
                Inline::Em(children) => self.inline(children, MarkdownWordType::Em),
                Inline::Html(text) => self.push_word(text, MarkdownWordType::Normal),
                Inline::Other => {}
            }
        }
    }

    /// The text of a token, split at newlines and at single spaces. A newline starts a new line,
    /// even when the piece before it is empty, as the reference does.
    fn text(&mut self, text: &str, kind: MarkdownWordType) {
        for (index, piece) in text.split('\n').enumerate() {
            if index > 0 {
                self.lines.push(Vec::new());
            }
            for word in piece.split(' ') {
                let content = word.replace("&#39;", "'");
                if !content.is_empty() {
                    self.current().push(MarkdownWord { content, kind });
                }
            }
        }
    }

    fn push_word(&mut self, content: &str, kind: MarkdownWordType) {
        self.current().push(MarkdownWord {
            content: content.to_owned(),
            kind,
        });
    }

    fn current(&mut self) -> &mut MarkdownLine {
        if self.lines.is_empty() {
            self.lines.push(Vec::new());
        }
        let last = self.lines.len() - 1;
        &mut self.lines[last]
    }
}

/// The plain-text split of `nonMarkdownToLines`: the pieces between `\\n` (a backslash and an `n`),
/// newlines and `<br>` tags, which `gi` matches in any case.
fn split_on_breaks(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < text.len() {
        let rest = &text[index..];
        let length = if rest.starts_with("\\n") {
            Some(2)
        } else if rest.starts_with('\n') {
            Some(1)
        } else {
            br_tag_length(rest, true)
        };
        match length {
            Some(length) => {
                pieces.push(&text[start..index]);
                index += length;
                start = index;
            }
            None => index += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    pieces.push(&text[start..]);
    pieces
}

/// The length of a `<br\s*\/?>` tag at the start of `text`, or `None`. The name matches in any
/// case when `ignore_case` is set, as the plain-text split's `i` flag does, and in lower case only
/// otherwise, as `createText`'s `<br>` rewrite does.
pub(crate) fn br_tag_length(text: &str, ignore_case: bool) -> Option<usize> {
    let name = text.strip_prefix('<')?.get(..2)?;
    let is_name = if ignore_case {
        name.eq_ignore_ascii_case("br")
    } else {
        name == "br"
    };
    if !is_name {
        return None;
    }
    let rest = &text[3..];
    let rest = rest.trim_start_matches(is_js_space);
    let rest = rest.strip_prefix('/').unwrap_or(rest);
    let rest = rest.strip_prefix('>')?;
    Some(text.len() - rest.len())
}

/// The words of one plain-text line: a `<...>` span (with spaces inside it) is one word, and any
/// other run of characters that are neither whitespace nor angle brackets is one word. A lone `<`
/// or `>` is dropped, as the regular expression `/<[^>]+>|[^\s<>]+/g` drops it.
fn plain_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut index = 0;
    while index < line.len() {
        let rest = &line[index..];
        let first = rest.chars().next().unwrap_or_default();
        if first == '<' {
            match rest[1..].find('>') {
                Some(close) if close >= 1 => {
                    words.push(rest[..close + 2].to_owned());
                    index += close + 2;
                }
                _ => index += 1,
            }
        } else if is_js_space(first) || first == '>' {
            index += first.len_utf8();
        } else {
            let end = rest
                .find(|character: char| {
                    is_js_space(character) || character == '<' || character == '>'
                })
                .unwrap_or(rest.len());
            words.push(rest[..end].to_owned());
            index += end;
        }
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(content: &str, kind: MarkdownWordType) -> MarkdownWord {
        MarkdownWord {
            content: content.to_owned(),
            kind,
        }
    }

    fn normal(content: &str) -> MarkdownWord {
        word(content, MarkdownWordType::Normal)
    }

    fn strong(content: &str) -> MarkdownWord {
        word(content, MarkdownWordType::Strong)
    }

    fn em(content: &str) -> MarkdownWord {
        word(content, MarkdownWordType::Em)
    }

    #[test]
    fn markdown_to_lines_basic_test() {
        let input = "This is regular text
Here is a new line
There is some words **with a bold** section
Here is a line *with an italic* section";
        let expected = vec![
            vec![
                normal("This"),
                normal("is"),
                normal("regular"),
                normal("text"),
            ],
            vec![
                normal("Here"),
                normal("is"),
                normal("a"),
                normal("new"),
                normal("line"),
            ],
            vec![
                normal("There"),
                normal("is"),
                normal("some"),
                normal("words"),
                strong("with"),
                strong("a"),
                strong("bold"),
                normal("section"),
            ],
            vec![
                normal("Here"),
                normal("is"),
                normal("a"),
                normal("line"),
                em("with"),
                em("an"),
                em("italic"),
                normal("section"),
            ],
        ];
        assert_eq!(markdown_to_lines(input), expected);
    }

    #[test]
    fn markdown_to_lines_empty_input() {
        assert_eq!(markdown_to_lines(""), vec![Vec::<MarkdownWord>::new()]);
    }

    #[test]
    fn markdown_to_lines_no_formatting() {
        let input = "This is a simple test
with no formatting";
        let expected = vec![
            vec![
                normal("This"),
                normal("is"),
                normal("a"),
                normal("simple"),
                normal("test"),
            ],
            vec![normal("with"), normal("no"), normal("formatting")],
        ];
        assert_eq!(markdown_to_lines(input), expected);
    }

    #[test]
    fn markdown_to_lines_only_bold_formatting() {
        let expected = vec![vec![
            normal("This"),
            normal("is"),
            normal("a"),
            strong("bold"),
            normal("test"),
        ]];
        assert_eq!(markdown_to_lines("This is a **bold** test"), expected);
    }

    #[test]
    fn markdown_to_lines_paragraph_1() {
        let input = "**Start** with
    a second line";
        let expected = vec![
            vec![strong("Start"), normal("with")],
            vec![normal("a"), normal("second"), normal("line")],
        ];
        assert_eq!(markdown_to_lines(input), expected);
    }

    #[test]
    fn markdown_to_lines_paragraph() {
        let input = "**Start** with
    a second line";
        let expected = vec![
            vec![strong("Start"), normal("with")],
            vec![normal("a"), normal("second"), normal("line")],
        ];
        assert_eq!(markdown_to_lines(input), expected);
    }

    #[test]
    fn markdown_to_lines_only_italic_formatting() {
        let expected = vec![vec![
            normal("This"),
            normal("is"),
            normal("an"),
            em("italic"),
            normal("test"),
        ]];
        assert_eq!(markdown_to_lines("This is an *italic* test"), expected);
    }

    #[test]
    fn markdown_to_lines_mixed_formatting() {
        let expected = vec![vec![
            em("Italic"),
            normal("and"),
            strong("bold"),
            normal("formatting"),
        ]];
        assert_eq!(
            markdown_to_lines("*Italic* and **bold** formatting"),
            expected
        );

        let expected = vec![vec![
            em("Italic"),
            em("with"),
            em("space"),
            normal("and"),
            strong("bold"),
            strong("ws"),
            normal("formatting"),
        ]];
        assert_eq!(
            markdown_to_lines("*Italic with space* and **bold ws** formatting"),
            expected
        );
    }

    #[test]
    fn markdown_to_lines_mixed_formatting_across_lines() {
        let input = "The dog in **the** hog... a *very long text* about it
Word!";
        let expected = vec![
            vec![
                normal("The"),
                normal("dog"),
                normal("in"),
                strong("the"),
                normal("hog..."),
                normal("a"),
                em("very"),
                em("long"),
                em("text"),
                normal("about"),
                normal("it"),
            ],
            vec![normal("Word!")],
        ];
        assert_eq!(markdown_to_lines(input), expected);
    }

    #[test]
    #[ignore = "port: skipped in the reference too (test.skip); markdownAutoWrap false is broken for htmlLabels false"]
    fn markdown_to_lines_no_auto_wrapping() {}

    #[test]
    fn non_markdown_to_lines_splits_text_by_newlines_preserves_svg_tags_does_not_process_markdown()
    {
        // Splits by newlines (\n, \\n, <br>) and preserves SVG tags
        assert_eq!(
            non_markdown_to_lines("Line 1\\nLine 2<br/>Text with <strong>tag</strong>"),
            vec![
                vec![normal("Line"), normal("1")],
                vec![normal("Line"), normal("2")],
                vec![
                    normal("Text"),
                    normal("with"),
                    normal("<strong>"),
                    normal("tag"),
                    normal("</strong>"),
                ],
            ]
        );

        // Markdown syntax is NOT processed - stays as plain text
        assert_eq!(
            non_markdown_to_lines("**bold** and *italic*"),
            vec![vec![normal("**bold**"), normal("and"), normal("*italic*"),]]
        );
    }

    #[test]
    fn plain_text_keeps_empty_lines_and_trims_each_line() {
        assert_eq!(
            non_markdown_to_lines("a<br/><br/>b"),
            vec![vec![normal("a")], vec![], vec![normal("b")]]
        );
        assert_eq!(non_markdown_to_lines(""), vec![Vec::<MarkdownWord>::new()]);
        assert_eq!(
            non_markdown_to_lines("x < y and a > b"),
            vec![vec![normal("x"), normal("< y and a >"), normal("b"),]]
        );
    }

    #[test]
    fn dedent_removes_the_common_indentation_only_when_every_line_has_it() {
        assert_eq!(dedent("a\n    b\n    c"), "a\nb\nc");
        assert_eq!(dedent("a\n  b\nc"), "a\n  b\nc");
        assert_eq!(dedent("\nabc\n  "), "abc");
    }
}
