//! The venn scanner: the rules of `venn.jison`'s lexer, in the order they are written. Jison takes
//! the first rule that matches at the current position, not the longest match, so the order below
//! is the grammar's. The one start condition beyond INITIAL is `bol`, which is inclusive: the rules
//! without a condition apply in both states, and the rules marked `<bol>` only in `bol`.
//!
//! The indent mode belongs to the grammar's actions (`setIndentMode`): the parser turns it on when a
//! `set` or `union` statement is reduced, and the scanner turns it off when a line begins with a
//! character that is not blank. See `docs-port/venn/PLAN.md`, quirks V1 to V3.

use crate::error::{ParseError, Source};
use crate::text::{is_cr_lf, is_js_space, is_word, js_trim};

/// The tokens of the grammar. Keywords and punctuation are unit variants; the others keep the text
/// the lexer matched, after the rule's own slicing where it has one.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Token {
    /// `[\n\r]+`, which also begins the `bol` state.
    Newline,
    /// The end of the text, after the final newline.
    Eof,
    /// `"title"\s[^#\n;]+`, with the whole match, the keyword and its first blank included.
    Title(String),
    /// `venn-beta`.
    Venn,
    /// `set`.
    Set,
    /// `union`.
    Union,
    /// `text`, when the lexer does not consume it as the text of an indented line.
    Text,
    /// `style`.
    Style,
    /// The `text` keyword of an indented line, returned before the keyword itself.
    IndentText,
    /// A bracket label, with its brackets removed (and trimmed, in the plain form).
    BracketLabel(String),
    /// A number as written, with its sign.
    Numeric(String),
    /// An identifier as written.
    Identifier(String),
    /// A quoted string as written, quotes included.
    Str(String),
    /// A colour as written: `#hex`, `rgb(…)` or `rgba(…)`.
    Colour(String),
    /// `,`.
    Comma,
    /// `:`.
    Colon,
}

/// A token and the byte offset where it starts, for the error positions.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Spanned {
    pub(super) token: Token,
    pub(super) offset: usize,
}

/// The scanner's state: the text as characters, the `bol` condition, the flag of the indented
/// `text` keyword, and the indent mode the grammar sets.
pub(super) struct Scanner<'a> {
    source: Source<'a>,
    chars: Vec<char>,
    /// The byte offset of each character, and the length of the text after the last one.
    offsets: Vec<usize>,
    pos: usize,
    bol: bool,
    consume_indent_text: bool,
    /// `yy.getIndentMode()`, written by the grammar's actions and by the rule for a line that
    /// begins with a non-blank character.
    pub(super) indent_mode: bool,
}

impl<'a> Scanner<'a> {
    pub(super) fn new(source: Source<'a>) -> Self {
        let text = source.text();
        let mut chars = Vec::with_capacity(text.len());
        let mut offsets = Vec::with_capacity(text.len() + 1);
        for (offset, c) in text.char_indices() {
            chars.push(c);
            offsets.push(offset);
        }
        offsets.push(text.len());
        Self {
            source,
            chars,
            offsets,
            pos: 0,
            bol: false,
            consume_indent_text: false,
            indent_mode: false,
        }
    }

    /// The next token. Skipped text is passed over, as Jison's `lex()` does, and the end of the
    /// text is `Eof` on every call after it is reached.
    pub(super) fn next_token(&mut self) -> Result<Spanned, ParseError> {
        loop {
            if self.pos >= self.chars.len() {
                return Ok(self.spanned(Token::Eof, self.pos));
            }
            if let Some(spanned) = self.step()? {
                return Ok(spanned);
            }
        }
    }

    /// One rule of the lexer. `None` means the text was skipped, or the state changed without a
    /// token, and the caller lexes again.
    fn step(&mut self) -> Result<Option<Spanned>, ParseError> {
        let start = self.pos;
        let c = self.chars[start];

        // `\%\%(?!\{)[^\n]*`: a comment to the end of the line, but not a `%%{` directive. A
        // directive is taken by the comment rule of `\%\%[^\n]*` below, after the `bol` rules.
        if self.at(start, "%%") && self.chars.get(start + 2) != Some(&'{') {
            self.pos = self.line_end(start);
            return Ok(None);
        }
        // `[^\}]\%\%[^\n]*`: a character that is not `}`, then a comment. It eats that character
        // too, and a newline before the comment goes with it, so the next line does not begin in
        // `bol` through this newline. JavaScript's `[^\}]` matches one UTF-16 unit, so a character
        // outside the BMP does not match here.
        if c != '}' && c.len_utf16() == 1 && self.at(start + 1, "%%") {
            self.pos = self.line_end(start + 1);
            return Ok(None);
        }

        if self.bol {
            if is_blank(c) {
                let end = self.blank_end(start);
                // `<bol>[ \t]+(?=[\n\r])`: a line of blanks.
                if self.chars.get(end).is_some_and(|&next| is_cr_lf(next)) {
                    self.pos = end;
                    return Ok(None);
                }
                // `<bol>[ \t]+(?=text\b)`: with the indent mode on, the blanks are the indent of a
                // `text` line, which returns `INDENT_TEXT`; with it off, they are passed over.
                self.pos = end;
                if self.at_word_ci(end, "text") {
                    if self.indent_mode {
                        self.consume_indent_text = true;
                        self.bol = false;
                        return Ok(Some(self.spanned(Token::IndentText, start)));
                    }
                    return Ok(None);
                }
                // `<bol>[ \t]+`: leading blanks.
                return Ok(None);
            }
            // `<bol>[^ \t\n\r]`: a line that begins with another character clears the indent
            // mode, begins INITIAL and is put back, so it is lexed again.
            if !is_cr_lf(c) {
                self.indent_mode = false;
                self.bol = false;
                return Ok(None);
            }
        }

        // `[\n\r]+`: a newline, and the `bol` state for the next line.
        if is_cr_lf(c) {
            let mut end = start;
            while self.chars.get(end).is_some_and(|&next| is_cr_lf(next)) {
                end += 1;
            }
            self.pos = end;
            self.bol = true;
            return Ok(Some(self.spanned(Token::Newline, start)));
        }
        // `\%\%[^\n]*`: a comment, including a `%%{` directive that the rule above does not take.
        if self.at(start, "%%") {
            self.pos = self.line_end(start);
            return Ok(None);
        }
        // `[ \t]+`: skipped.
        if is_blank(c) {
            self.pos = self.blank_end(start);
            return Ok(None);
        }

        // `"title"\s[^#\n;]+`: the keyword, one blank, and the title up to `#`, `;` or a newline.
        if let Some(end) = self.title_end(start) {
            self.pos = end;
            return Ok(Some(
                self.spanned(Token::Title(self.text(start, end)), start),
            ));
        }
        for (word, token) in [
            ("venn-beta", Token::Venn),
            ("set", Token::Set),
            ("union", Token::Union),
        ] {
            if self.at_word_ci(start, word) {
                self.pos = start + word.len();
                return Ok(Some(self.spanned(token, start)));
            }
        }
        // `"text"`: with the indent text pending, the keyword is consumed and returns nothing.
        if self.at_word_ci(start, "text") {
            self.pos = start + 4;
            if self.consume_indent_text {
                self.consume_indent_text = false;
                return Ok(None);
            }
            return Ok(Some(self.spanned(Token::Text, start)));
        }
        if self.at_word_ci(start, "style") {
            self.pos = start + 5;
            return Ok(Some(self.spanned(Token::Style, start)));
        }

        // `\[\"[^\"]*\"\]`, with the quotes and brackets removed, then `\[[^\]\"]+\]`, trimmed.
        if let Some((end, label)) = self.quoted_bracket(start) {
            self.pos = end;
            return Ok(Some(self.spanned(Token::BracketLabel(label), start)));
        }
        if let Some((end, label)) = self.plain_bracket(start) {
            self.pos = end;
            return Ok(Some(self.spanned(Token::BracketLabel(label), start)));
        }

        if let Some(end) = self.numeric_end(start) {
            self.pos = end;
            return Ok(Some(
                self.spanned(Token::Numeric(self.text(start, end)), start),
            ));
        }
        if let Some(end) = self.colour_end(start) {
            self.pos = end;
            return Ok(Some(
                self.spanned(Token::Colour(self.text(start, end)), start),
            ));
        }
        if let Some(end) = self.identifier_end(start) {
            self.pos = end;
            return Ok(Some(
                self.spanned(Token::Identifier(self.text(start, end)), start),
            ));
        }
        if let Some(end) = self.string_end(start) {
            self.pos = end;
            return Ok(Some(self.spanned(Token::Str(self.text(start, end)), start)));
        }
        match c {
            ',' => {
                self.pos = start + 1;
                Ok(Some(self.spanned(Token::Comma, start)))
            }
            ':' => {
                self.pos = start + 1;
                Ok(Some(self.spanned(Token::Colon, start)))
            }
            _ => Err(self
                .source
                .error(self.offsets[start], "unexpected character")),
        }
    }

    fn spanned(&self, token: Token, at: usize) -> Spanned {
        Spanned {
            token,
            offset: self.offsets[at.min(self.offsets.len() - 1)],
        }
    }

    /// The characters from `start` to `end`, as text.
    fn text(&self, start: usize, end: usize) -> String {
        self.chars[start..end].iter().collect()
    }

    /// Whether the characters at `at` are `word`, exactly.
    fn at(&self, at: usize, word: &str) -> bool {
        word.chars()
            .enumerate()
            .all(|(i, expected)| self.chars.get(at + i) == Some(&expected))
    }

    /// Whether the characters at `at` are `word`, ignoring the case of ASCII letters, as the `i`
    /// flag of Jison's regular expressions does.
    fn at_ci(&self, at: usize, word: &str) -> bool {
        word.chars().enumerate().all(|(i, expected)| {
            self.chars
                .get(at + i)
                .is_some_and(|actual| actual.eq_ignore_ascii_case(&expected))
        })
    }

    /// Whether `word` is at `at` and ends at a word boundary, as `word\b` does: the next character
    /// is not a word character, or there is none. Jison appends `\b` to a rule whose pattern ends in
    /// a word character, so `setA` is the identifier `setA`, not `set` and `A`. The keywords
    /// `venn-beta`, `set`, `union`, `text` and `style` all take this boundary.
    fn at_word_ci(&self, at: usize, word: &str) -> bool {
        self.at_ci(at, word)
            && !self
                .chars
                .get(at + word.len())
                .is_some_and(|&next| is_word(next))
    }

    fn blank_end(&self, mut at: usize) -> usize {
        while self.chars.get(at).is_some_and(|&c| is_blank(c)) {
            at += 1;
        }
        at
    }

    /// The position of the newline that ends the line of `at`, or the end of the text.
    fn line_end(&self, mut at: usize) -> usize {
        while self.chars.get(at).is_some_and(|&c| c != '\n') {
            at += 1;
        }
        at
    }

    /// `"title"\s[^#\n;]+` at `start`, as the end of the match.
    fn title_end(&self, start: usize) -> Option<usize> {
        if !self.at_ci(start, "title")
            || !self.chars.get(start + 5).is_some_and(|&c| is_js_space(c))
        {
            return None;
        }
        let mut end = start + 6;
        while self
            .chars
            .get(end)
            .is_some_and(|&c| !matches!(c, '#' | '\n' | ';'))
        {
            end += 1;
        }
        (end > start + 6).then_some(end)
    }

    /// `\[\"[^\"]*\"\]` at `start`: the end of the match and the label between the quotes.
    fn quoted_bracket(&self, start: usize) -> Option<(usize, String)> {
        if !self.at(start, "[\"") {
            return None;
        }
        let close = (start + 2..self.chars.len()).find(|&i| self.chars[i] == '"')?;
        if self.chars.get(close + 1) != Some(&']') {
            return None;
        }
        Some((close + 2, self.text(start + 2, close)))
    }

    /// `\[[^\]\"]+\]` at `start`: the end of the match and the label between the brackets, trimmed.
    fn plain_bracket(&self, start: usize) -> Option<(usize, String)> {
        if self.chars.get(start) != Some(&'[') {
            return None;
        }
        let mut end = start + 1;
        while self.chars.get(end).is_some_and(|&c| c != ']' && c != '"') {
            end += 1;
        }
        if end == start + 1 || self.chars.get(end) != Some(&']') {
            return None;
        }
        Some((end + 1, js_trim(&self.text(start + 1, end)).to_owned()))
    }

    /// `[+-]?(\d+(\.\d+)?|\.\d+)` at `start`, as the end of the match.
    fn numeric_end(&self, start: usize) -> Option<usize> {
        let mut at = start;
        if matches!(self.chars.get(at), Some('+' | '-')) {
            at += 1;
        }
        let digits_end = self.digits_end(at);
        if digits_end > at {
            // `\d+(\.\d+)?`: the fraction is taken only with a digit after its dot.
            if self.chars.get(digits_end) == Some(&'.') {
                let fraction_end = self.digits_end(digits_end + 1);
                if fraction_end > digits_end + 1 {
                    return Some(fraction_end);
                }
            }
            return Some(digits_end);
        }
        // `\.\d+`
        if self.chars.get(at) == Some(&'.') {
            let fraction_end = self.digits_end(at + 1);
            if fraction_end > at + 1 {
                return Some(fraction_end);
            }
        }
        None
    }

    fn digits_end(&self, mut at: usize) -> usize {
        while self.chars.get(at).is_some_and(char::is_ascii_digit) {
            at += 1;
        }
        at
    }

    /// `#[0-9a-fA-F]{3,8}`, `rgba(…)` or `rgb(…)` at `start`, as the end of the match. The first
    /// alternative that matches is taken: `#` with three to eight hex digits (greedy), then `rgba`
    /// with four numbers, then `rgb` with three.
    fn colour_end(&self, start: usize) -> Option<usize> {
        if self.chars.get(start) == Some(&'#') {
            let digits = (start + 1..start + 9)
                .take_while(|&i| self.chars.get(i).is_some_and(char::is_ascii_hexdigit))
                .count();
            return (digits >= 3).then_some(start + 1 + digits);
        }
        if self.at_ci(start, "rgba(") {
            return self.function_end(start + 5, 4);
        }
        if self.at_ci(start, "rgb(") {
            return self.function_end(start + 4, 3);
        }
        None
    }

    /// The rest of `rgba\(\s*[0-9.]+\s*[,]…\)` after its open parenthesis, with `count` numbers.
    fn function_end(&self, mut at: usize, count: usize) -> Option<usize> {
        for index in 0..count {
            at = self.blank_run_end(at);
            let mut number_end = at;
            while self
                .chars
                .get(number_end)
                .is_some_and(|&c| c.is_ascii_digit() || c == '.')
            {
                number_end += 1;
            }
            if number_end == at {
                return None;
            }
            at = self.blank_run_end(number_end);
            if index + 1 < count {
                if self.chars.get(at) != Some(&',') {
                    return None;
                }
                at += 1;
            }
        }
        (self.chars.get(at) == Some(&')')).then_some(at + 1)
    }

    /// The end of a run of JavaScript's `\s`, from `at`.
    fn blank_run_end(&self, mut at: usize) -> usize {
        while self.chars.get(at).is_some_and(|&c| is_js_space(c)) {
            at += 1;
        }
        at
    }

    /// `[A-Za-z_][A-Za-z0-9\-_]*` at `start`, as the end of the match.
    fn identifier_end(&self, start: usize) -> Option<usize> {
        if !self
            .chars
            .get(start)
            .is_some_and(|&c| c.is_ascii_alphabetic() || c == '_')
        {
            return None;
        }
        let mut end = start + 1;
        while self
            .chars
            .get(end)
            .is_some_and(|&c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            end += 1;
        }
        Some(end)
    }

    /// `\"[^\"]*\"` at `start`, as the end of the match.
    fn string_end(&self, start: usize) -> Option<usize> {
        if self.chars.get(start) != Some(&'"') {
            return None;
        }
        (start + 1..self.chars.len())
            .find(|&i| self.chars[i] == '"')
            .map(|close| close + 1)
    }
}

/// `[ \t]`.
fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}
