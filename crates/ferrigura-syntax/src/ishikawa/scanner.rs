//! The ishikawa scanner: the rules of `ishikawa.jison`, in the grammar's order. Jison takes the first
//! rule that matches, not the longest, so the order below is the order of the file. See
//! `docs-port/ishikawa/PLAN.md`, section 2, for the rule table and the quirks `I1` to `I26`.
//!
//! The scanner reads the text the reference's `Diagram.fromText` hands the parser: the cleaned
//! text with its line break added, and entities encoded. The grammar has no start condition, so
//! there is one state.

use crate::text::{is_js_space, is_line_term, is_word};

/// The tokens of the grammar. `Text` keeps the rest of the line as the rule matched it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// `ishikawa` or `ishikawa-beta`, in any letter case.
    Ishikawa,
    /// Blanks that end in a line break, or a `%%` comment: `SPACELINE`.
    SpaceLine,
    /// A line break: `NL`. A run of line breaks is one token.
    Nl,
    /// Blanks that end no line, with their length in characters: `SPACELIST`. Every JavaScript
    /// white space character is in the Basic Multilingual Plane, so a character is one UTF-16 unit.
    SpaceList(usize),
    /// The rest of the line from a character that is not blank, to the line break: `TEXT`.
    Text(String),
    /// The end of the text, which is the grammar's `EOF`.
    Eof,
}

/// A token with the byte offset where it starts in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) token: Token,
    pub(crate) start: usize,
}

/// Reads the whole text. The result ends with the one `Eof` token.
pub(crate) fn scan(text: &str) -> Vec<Spanned> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    loop {
        let rest = &text[pos..];
        if rest.is_empty() {
            tokens.push(Spanned {
                token: Token::Eof,
                start: pos,
            });
            return tokens;
        }
        let (token, len) = next_rule(rest);
        tokens.push(Spanned { token, start: pos });
        pos += len;
    }
}

/// The first rule of `ishikawa.jison` that matches at the start of `rest`, with its match length.
/// `rest` is not empty, and every rule that can match it consumes at least one byte: the last one
/// takes the rest of the line, which starts with a character that is not a line break or blank.
fn next_rule(rest: &str) -> (Token, usize) {
    if let Some(len) = comment_len(rest) {
        return (Token::SpaceLine, len);
    }
    if let Some(len) = header_len(rest) {
        return (Token::Ishikawa, len);
    }
    if let Some(len) = space_line_len(rest) {
        return (Token::SpaceLine, len);
    }
    let newlines = rest.len() - rest.trim_start_matches('\n').len();
    if newlines > 0 {
        return (Token::Nl, newlines);
    }
    let blanks = space_run(rest);
    if blanks > 0 {
        return (Token::SpaceList(rest[..blanks].chars().count()), blanks);
    }
    let len = rest.find('\n').unwrap_or(rest.len());
    (Token::Text(rest[..len].to_owned()), len)
}

/// The byte length of the run of JavaScript white space at the start of `text`, line breaks
/// included.
fn space_run(text: &str) -> usize {
    text.len() - text.trim_start_matches(is_js_space).len()
}

/// `\s*%%.*`: the blanks and line breaks before a `%%`, then the rest of that line. The `.` does not
/// match a line terminator, so the comment ends where JavaScript's `.` does.
fn comment_len(rest: &str) -> Option<usize> {
    let spaces = space_run(rest);
    let after = rest[spaces..].strip_prefix("%%")?;
    let body = after.find(is_line_term).unwrap_or(after.len());
    Some(spaces + 2 + body)
}

/// `ishikawa-beta\b`, then `ishikawa\b`, both in any letter case. The `\b` is the boundary of
/// JavaScript: the next character is not a word character (ASCII letters, digits and `_`). So
/// `ishikawaEffect` is not the header, and `ishikawa-betafoo` is the header and the text `-betafoo`.
fn header_len(rest: &str) -> Option<usize> {
    ["ishikawa-beta", "ishikawa"]
        .into_iter()
        .find_map(|literal| {
            let head = rest.get(..literal.len())?;
            let after = &rest[literal.len()..];
            (head.eq_ignore_ascii_case(literal) && !after.chars().next().is_some_and(is_word))
                .then_some(literal.len())
        })
}

/// `[\s]+[\n]`: the longest run of blanks that ends in a line break, where the break is not the
/// first character. Blanks with no such break, such as the indentation of a line, match nothing.
fn space_line_len(rest: &str) -> Option<usize> {
    let blanks = &rest[..space_run(rest)];
    blanks
        .rfind('\n')
        .filter(|&newline| newline > 0)
        .map(|newline| newline + 1)
}

#[cfg(test)]
mod tests {
    use super::{Token, scan};

    fn tokens(text: &str) -> Vec<Token> {
        scan(text)
            .into_iter()
            .map(|spanned| spanned.token)
            .collect()
    }

    /// I2: the text `ishikawa` plus its line break, plus the line break the parser adds, is a
    /// SPACELINE, not an NL: the rule `[\s]+[\n]` takes two line breaks first.
    #[test]
    fn a_header_followed_by_two_line_breaks_is_a_space_line() {
        assert_eq!(
            tokens("ishikawa\n\n"),
            [Token::Ishikawa, Token::SpaceLine, Token::Eof]
        );
    }

    /// I1: the header and one line break are an NL, which needs a statement after it.
    #[test]
    fn a_single_line_break_is_an_nl() {
        assert_eq!(
            tokens("ishikawa\n"),
            [Token::Ishikawa, Token::Nl, Token::Eof]
        );
    }

    /// I3: the `\b` after the header keyword: `ishikawaEffect` is one text.
    #[test]
    fn a_header_glued_to_a_word_is_text() {
        assert_eq!(
            tokens("ishikawaEffect\n"),
            [
                Token::Text("ishikawaEffect".to_owned()),
                Token::Nl,
                Token::Eof
            ]
        );
    }

    /// I5: `ishikawa-beta` needs a boundary after it, so `ishikawa` is the header of
    /// `ishikawa-betafoo`, and `-betafoo` is the text.
    #[test]
    fn a_glued_beta_keyword_is_the_plain_header_and_text() {
        assert_eq!(
            tokens("ishikawa-betafoo\n"),
            [
                Token::Ishikawa,
                Token::Text("-betafoo".to_owned()),
                Token::Nl,
                Token::Eof
            ]
        );
    }

    /// Indentation is a SPACELIST with its length in characters, and the text follows it.
    #[test]
    fn indentation_is_a_space_list_of_its_length() {
        assert_eq!(
            tokens("  a\n"),
            [
                Token::SpaceList(2),
                Token::Text("a".to_owned()),
                Token::Nl,
                Token::Eof
            ]
        );
    }
}
