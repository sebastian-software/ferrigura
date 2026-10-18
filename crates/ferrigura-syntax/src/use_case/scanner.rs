//! The scanner of the use case diagram, as `usecase.tokens.ts` lexes it. Chevrotain tries the
//! patterns of a mode in their written order and takes the first that matches, not the longest,
//! except that a token's `longer_alt` wins when it matches more text at the same offset. Whitespace
//! is skipped. Three modes: the default one, the JSON body after `json <id> @`, and the stereotype
//! after `<<`. The rule order of each mode is the one of the reference; see
//! `docs-port/usecase/PLAN.md`.

use crate::text::js_trim;

/// The kind of a token. The names follow the reference's token names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tok {
    /// The end of the input (not in the reference's list; Chevrotain's `EOF`).
    Eof,
    NewLine,
    Comment,
    MarkdownString,
    UnclosedMarkdownString,
    AccDescrBlock,
    AccTitleLine,
    AccDescrLine,
    JsonDeclarationStart,
    JsonObjectLiteral,
    UnclosedJsonObjectLiteral,
    StereotypeStart,
    StereotypeText,
    UnclosedStereotypeText,
    StereotypeEnd,
    Usecase,
    Actor,
    SystemBoundary,
    End,
    Direction,
    Td,
    Tb,
    Bt,
    Lr,
    Rl,
    Note,
    For,
    Json,
    ClassDef,
    Class,
    Style,
    Include,
    Extend,
    True,
    False,
    Generalization,
    DependencyArrow,
    ClassSeparator,
    ForwardSolid,
    BackwardSolid,
    ForwardCircle,
    BackwardCircle,
    ForwardCross,
    BackwardCross,
    MarkerlessSolid,
    MetadataStart,
    At,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    Comma,
    Colon,
    HashColor,
    PlainString,
    CssIdentifier,
    Identifier,
    NumberLiteral,
    CssEscapedComma,
    Dash,
    Dot,
    Percent,
    CssPunctuation,
    LabelPunctuation,
    LabelSymbol,
}

impl Tok {
    /// Whether the token is in the reference's `Word` category: the keywords and identifiers.
    #[must_use]
    pub(crate) fn is_word(self) -> bool {
        self == Self::Identifier || self.is_keyword()
    }

    /// Whether the token is one of the keywords (`Word` tokens that are not identifiers).
    #[must_use]
    pub(crate) fn is_keyword(self) -> bool {
        matches!(
            self,
            Self::Usecase
                | Self::Actor
                | Self::SystemBoundary
                | Self::End
                | Self::Direction
                | Self::Td
                | Self::Tb
                | Self::Bt
                | Self::Lr
                | Self::Rl
                | Self::Note
                | Self::For
                | Self::Json
                | Self::ClassDef
                | Self::Class
                | Self::Style
                | Self::Include
                | Self::Extend
                | Self::True
                | Self::False
        )
    }

    /// Whether the token is in the `LabelText` category, the tokens an unquoted label may hold.
    #[must_use]
    pub(crate) fn is_label_text(self) -> bool {
        self.is_word()
            || matches!(
                self,
                Self::NumberLiteral
                    | Self::At
                    | Self::Comma
                    | Self::Colon
                    | Self::HashColor
                    | Self::CssIdentifier
                    | Self::CssEscapedComma
                    | Self::Dash
                    | Self::Dot
                    | Self::Percent
                    | Self::CssPunctuation
                    | Self::LabelPunctuation
                    | Self::LabelSymbol
            )
    }
}

/// A token with its span in the scanned text: the byte offsets (for slicing) and the UTF-16
/// offsets (for the positions the reference reports, as JavaScript counts them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    /// The kind of the token.
    pub kind: Tok,
    /// The byte offset of the first byte of the token.
    pub start: usize,
    /// The byte offset just after the last byte of the token.
    pub end: usize,
    /// The UTF-16 offset of the first code unit of the token.
    pub u16_start: usize,
    /// The UTF-16 offset just after the last code unit of the token.
    pub u16_end: usize,
}

impl Token {
    /// The text of the token.
    #[must_use]
    pub fn image<'a>(&self, text: &'a str) -> &'a str {
        &text[self.start..self.end]
    }
}

/// The modes of the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Default,
    JsonBody,
    Stereotype,
}

/// Scans the text. A character that no pattern matches is a lexer error, reported at its byte
/// offset.
///
/// # Errors
///
/// Returns the byte offset of the first character that no token matches.
pub fn scan(text: &str) -> Result<Vec<Token>, usize> {
    let mut scanner = Scanner {
        text,
        pos: 0,
        u16_pos: 0,
        modes: vec![Mode::Default],
        tokens: Vec::new(),
    };
    while scanner.pos < text.len() {
        scanner.step()?;
    }
    scanner.push(Tok::Eof, 0);
    Ok(scanner.tokens)
}

/// What a matched pattern does to the mode stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Plain,
    Skip,
    Push(Mode),
    Pop,
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    u16_pos: usize,
    modes: Vec<Mode>,
    tokens: Vec<Token>,
}

impl Scanner<'_> {
    fn mode(&self) -> Mode {
        self.modes.last().copied().unwrap_or(Mode::Default)
    }

    /// Consumes `len` bytes as a token of `kind`, or skips them.
    fn push(&mut self, kind: Tok, len: usize) {
        let consumed = &self.text[self.pos..self.pos + len];
        let u16_len = consumed.encode_utf16().count();
        self.tokens.push(Token {
            kind,
            start: self.pos,
            end: self.pos + len,
            u16_start: self.u16_pos,
            u16_end: self.u16_pos + u16_len,
        });
        self.pos += len;
        self.u16_pos += u16_len;
    }

    fn skip(&mut self, len: usize) {
        let u16_len = self.text[self.pos..self.pos + len].encode_utf16().count();
        self.pos += len;
        self.u16_pos += u16_len;
    }

    /// Scans one token (or skipped run) at the current position.
    fn step(&mut self) -> Result<(), usize> {
        let found = match self.mode() {
            Mode::Default => self.default_match(),
            Mode::JsonBody => json_body_match(self.rest()),
            Mode::Stereotype => stereotype_match(self.text, self.pos),
        };
        let Some((kind, len, action)) = found else {
            return Err(self.pos);
        };
        if action == Action::Skip {
            self.skip(len);
            return Ok(());
        }
        self.push(kind, len);
        match action {
            Action::Push(mode) => self.modes.push(mode),
            Action::Pop => {
                self.modes.pop();
            }
            Action::Plain | Action::Skip => {}
        }
        Ok(())
    }

    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    /// The default mode: the list of `defaultModeTokens` in its order, with the longer alternative
    /// of the keywords and identifiers.
    fn default_match(&self) -> Option<(Tok, usize, Action)> {
        let text = self.text;
        let at = self.pos;
        let rest = self.rest();
        let plain = |kind, len: Option<usize>| len.map(|l| (kind, l, Action::Plain));
        if let Some(len) = blank_run_len(rest) {
            return Some((Tok::Eof, len, Action::Skip));
        }
        if let Some(len) = markdown_string_len(text, at) {
            return Some((Tok::MarkdownString, len, Action::Plain));
        }
        if rest.starts_with("\"`") {
            return Some((Tok::UnclosedMarkdownString, rest.len(), Action::Plain));
        }
        if let Some(len) = comment_len(text, at) {
            return Some((Tok::Comment, len, Action::Plain));
        }
        if let Some(len) = newline_len(rest) {
            return Some((Tok::NewLine, len, Action::Plain));
        }
        if let Some(len) = acc_descr_block_len(text, at) {
            return Some((Tok::AccDescrBlock, len, Action::Plain));
        }
        if let Some(len) = accessibility_line_len(text, at, "accTitle") {
            return Some((Tok::AccTitleLine, len, Action::Plain));
        }
        if let Some(len) = accessibility_line_len(text, at, "accDescr") {
            return Some((Tok::AccDescrLine, len, Action::Plain));
        }
        if let Some(len) = json_declaration_len(rest) {
            return Some((Tok::JsonDeclarationStart, len, Action::Push(Mode::JsonBody)));
        }
        if let Some(found) = keyword_match(rest) {
            return Some((found.0, found.1, Action::Plain));
        }
        if let Some(len) = literal_len(rest, "--|>") {
            return plain(Tok::Generalization, Some(len));
        }
        if let Some(len) = literal_len(rest, "..>") {
            return plain(Tok::DependencyArrow, Some(len));
        }
        if rest.starts_with("<<") {
            return Some((Tok::StereotypeStart, 2, Action::Push(Mode::Stereotype)));
        }
        if let Some(len) = literal_len(rest, ":::") {
            return plain(Tok::ClassSeparator, Some(len));
        }
        if let Some(len) = forward_solid_len(rest) {
            return plain(Tok::ForwardSolid, Some(len));
        }
        if let Some(len) = backward_solid_len(rest) {
            return plain(Tok::BackwardSolid, Some(len));
        }
        if let Some(len) = literal_len(rest, "--o") {
            return plain(Tok::ForwardCircle, Some(len));
        }
        if let Some(len) = literal_len(rest, "o--") {
            return plain(Tok::BackwardCircle, Some(len));
        }
        if let Some(len) = literal_len(rest, "--x") {
            return plain(Tok::ForwardCross, Some(len));
        }
        if let Some(len) = literal_len(rest, "x--") {
            return plain(Tok::BackwardCross, Some(len));
        }
        if let Some(len) = markerless_solid_len(rest) {
            return plain(Tok::MarkerlessSolid, Some(len));
        }
        self.default_punctuation(rest)
    }

    /// The single-character patterns and the strings, in their order.
    fn default_punctuation(&self, rest: &str) -> Option<(Tok, usize, Action)> {
        let single = |kind| Some((kind, 1, Action::Plain));
        let first = rest.as_bytes().first().copied()?;
        if let Some(len) = literal_len(rest, "@{") {
            return Some((Tok::MetadataStart, len, Action::Plain));
        }
        match first {
            b'@' => return single(Tok::At),
            b'{' => return single(Tok::LeftBrace),
            b'}' => return single(Tok::RightBrace),
            b'[' => return single(Tok::LeftBracket),
            b']' => return single(Tok::RightBracket),
            b'(' => return single(Tok::LeftParen),
            b')' => return single(Tok::RightParen),
            b',' => return single(Tok::Comma),
            b':' => return single(Tok::Colon),
            _ => {}
        }
        if let Some(len) = hash_color_len(rest) {
            return Some((Tok::HashColor, len, Action::Plain));
        }
        if let Some(len) = plain_string_len(rest) {
            return Some((Tok::PlainString, len, Action::Plain));
        }
        if let Some(len) = css_identifier_len(rest) {
            return Some((Tok::CssIdentifier, len, Action::Plain));
        }
        if is_word_byte(first) {
            return Some(identifier_or_number(rest));
        }
        if first == b'.'
            && let Some(len) = number_len(rest)
        {
            return Some((Tok::NumberLiteral, len, Action::Plain));
        }
        if let Some(len) = literal_len(rest, "\\,") {
            return Some((Tok::CssEscapedComma, len, Action::Plain));
        }
        match first {
            b'-' => single(Tok::Dash),
            b'.' => single(Tok::Dot),
            b'%' => single(Tok::Percent),
            _ if b"!#$&*+/=?^_|~".contains(&first) => single(Tok::CssPunctuation),
            _ if b";<>\\`".contains(&first) => single(Tok::LabelPunctuation),
            _ => label_symbol_len(rest).map(|len| (Tok::LabelSymbol, len, Action::Plain)),
        }
    }
}

/// The identifier of the `\w+` pattern, with its `longer_alt` number: the longer one wins.
fn identifier_or_number(rest: &str) -> (Tok, usize, Action) {
    let identifier = word_len(rest);
    match number_len(rest) {
        Some(number) if number > identifier => (Tok::NumberLiteral, number, Action::Plain),
        _ => (Tok::Identifier, identifier, Action::Plain),
    }
}

/// The keyword patterns of the default mode in their order, with their `longer_alt` identifier.
fn keyword_match(rest: &str) -> Option<(Tok, usize)> {
    const KEYWORDS: [(&str, Tok, bool); 20] = [
        ("usecase-beta", Tok::Usecase, false),
        ("actor", Tok::Actor, false),
        ("systemBoundary", Tok::SystemBoundary, false),
        ("end", Tok::End, false),
        ("direction", Tok::Direction, false),
        ("TD", Tok::Td, false),
        ("TB", Tok::Tb, false),
        ("BT", Tok::Bt, false),
        ("LR", Tok::Lr, false),
        ("RL", Tok::Rl, false),
        ("note", Tok::Note, false),
        ("for", Tok::For, false),
        ("json", Tok::Json, false),
        ("classDef", Tok::ClassDef, false),
        ("class", Tok::Class, false),
        ("style", Tok::Style, false),
        ("include", Tok::Include, true),
        ("extend", Tok::Extend, true),
        ("true", Tok::True, false),
        ("false", Tok::False, false),
    ];
    for (word, kind, ignore_case) in KEYWORDS {
        let matches = if ignore_case {
            rest.len() >= word.len()
                && rest.as_bytes()[..word.len()].eq_ignore_ascii_case(word.as_bytes())
        } else {
            rest.starts_with(word)
        };
        if matches {
            let identifier = word_len(rest);
            return Some(if identifier > word.len() {
                (Tok::Identifier, identifier)
            } else {
                (kind, word.len())
            });
        }
    }
    None
}

/// `[\t ]+` at the start: the length of the run of tabs and spaces, if there is one.
fn blank_run_len(rest: &str) -> Option<usize> {
    let len = rest
        .bytes()
        .take_while(|b| *b == b' ' || *b == b'\t')
        .count();
    (len > 0).then_some(len)
}

/// The length of `literal` if `rest` starts with it.
fn literal_len(rest: &str, literal: &str) -> Option<usize> {
    rest.starts_with(literal).then_some(literal.len())
}

/// The length of the run of ASCII word characters `[A-Za-z0-9_]` at the start.
fn word_len(rest: &str) -> usize {
    rest.bytes().take_while(|b| is_word_byte(*b)).count()
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The length of the run of ASCII digits at the start.
fn digits_len(bytes: &[u8]) -> usize {
    bytes.iter().take_while(|b| b.is_ascii_digit()).count()
}

/// `(?:\d+\.\d+|\d+|\.\d+)(?:[A-Za-z]+)?`: the length of the number at the start, if any.
fn number_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let whole = digits_len(bytes);
    let core = if whole > 0 {
        let fraction = bytes.get(whole + 1..).map_or(0, digits_len);
        if bytes.get(whole) == Some(&b'.') && fraction > 0 {
            whole + 1 + fraction
        } else {
            whole
        }
    } else if bytes.first() == Some(&b'.') {
        let fraction = digits_len(&bytes[1..]);
        if fraction == 0 {
            return None;
        }
        1 + fraction
    } else {
        return None;
    };
    let letters = bytes[core..]
        .iter()
        .take_while(|b| b.is_ascii_alphabetic())
        .count();
    Some(core + letters)
}

/// `"`…`"` with no quote in between and no line break (Markdown string, `matchMarkdownString`).
fn markdown_string_len(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(at..at + 2)? != b"\"`" {
        return None;
    }
    (at + 2..bytes.len().saturating_sub(1))
        .find(|&index| bytes[index] == b'`' && bytes[index + 1] == b'"')
        .map(|index| index + 2 - at)
}

/// Whether the text from the start of the line to `at` holds only blanks (`isIndentedLineStart`).
fn is_indented_line_start(text: &str, at: usize) -> bool {
    text.as_bytes()[..at]
        .iter()
        .rev()
        .find(|b| !matches!(b, b' ' | b'\t'))
        .is_none_or(|b| matches!(b, b'\n' | b'\r'))
}

/// `%%` and the rest of the line, when `%%` is the first token of its line (`matchComment`).
fn comment_len(text: &str, at: usize) -> Option<usize> {
    if !text[at..].starts_with("%%") || !is_indented_line_start(text, at) {
        return None;
    }
    let rest = &text[at..];
    Some(rest.find(['\n', '\r']).unwrap_or(rest.len()))
}

/// A line break: `\r\n`, `\n` or `\r`.
fn newline_len(rest: &str) -> Option<usize> {
    if rest.starts_with("\r\n") {
        Some(2)
    } else if rest.starts_with(['\n', '\r']) {
        Some(1)
    } else {
        None
    }
}

/// `accDescr [\t ]* {` up to the first `}` after it, at the start of an indented line
/// (`matchAccDescrBlock`).
fn acc_descr_block_len(text: &str, at: usize) -> Option<usize> {
    if !is_indented_line_start(text, at) {
        return None;
    }
    let rest = text[at..].strip_prefix("accDescr")?;
    let blanks = rest
        .bytes()
        .take_while(|b| matches!(b, b' ' | b'\t'))
        .count();
    if rest.as_bytes().get(blanks) != Some(&b'{') {
        return None;
    }
    let opening = "accDescr".len() + blanks + 1;
    let close = text[at + opening..].find('}')?;
    Some(opening + close + 1)
}

/// `accTitle[\t ]*:[^\n\r]*` (or `accDescr`) at the start of an indented line.
fn accessibility_line_len(text: &str, at: usize, keyword: &str) -> Option<usize> {
    if !is_indented_line_start(text, at) {
        return None;
    }
    let rest = text[at..].strip_prefix(keyword)?;
    let blanks = rest
        .bytes()
        .take_while(|b| matches!(b, b' ' | b'\t'))
        .count();
    if rest.as_bytes().get(blanks) != Some(&b':') {
        return None;
    }
    let line = &rest[blanks + 1..];
    Some(keyword.len() + blanks + 1 + line.find(['\n', '\r']).unwrap_or(line.len()))
}

/// `json[\t ]+\w+[\t ]*@[\t ]*(?={)`: the declaration of a JSON table, up to its `{`.
fn json_declaration_len(rest: &str) -> Option<usize> {
    let after_json = rest.strip_prefix("json")?;
    let blanks = after_json
        .bytes()
        .take_while(|b| matches!(b, b' ' | b'\t'))
        .count();
    if blanks == 0 {
        return None;
    }
    let after_blanks = &after_json[blanks..];
    let id = word_len(after_blanks);
    if id == 0 {
        return None;
    }
    let after_id = &after_blanks[id..];
    let gap = after_id
        .bytes()
        .take_while(|b| matches!(b, b' ' | b'\t'))
        .count();
    let after_gap = &after_id[gap..];
    let at_sign = after_gap.strip_prefix('@')?;
    let trailing = at_sign
        .bytes()
        .take_while(|b| matches!(b, b' ' | b'\t'))
        .count();
    if at_sign.as_bytes().get(trailing) != Some(&b'{') {
        return None;
    }
    Some(rest.len() - at_sign.len() + trailing)
}

/// `--+>`: two or more dashes followed by `>`.
fn forward_solid_len(rest: &str) -> Option<usize> {
    let dashes = rest.bytes().take_while(|b| *b == b'-').count();
    (dashes >= 2 && rest.as_bytes().get(dashes) == Some(&b'>')).then_some(dashes + 1)
}

/// `<--+`: a `<` followed by two or more dashes.
fn backward_solid_len(rest: &str) -> Option<usize> {
    let dashes = rest.get(1..)?.bytes().take_while(|b| *b == b'-').count();
    (rest.starts_with('<') && dashes >= 2).then_some(1 + dashes)
}

/// `--+`: two or more dashes.
fn markerless_solid_len(rest: &str) -> Option<usize> {
    let dashes = rest.bytes().take_while(|b| *b == b'-').count();
    (dashes >= 2).then_some(dashes)
}

/// `#[\dA-Fa-f]+`.
fn hash_color_len(rest: &str) -> Option<usize> {
    let digits = rest
        .get(1..)?
        .bytes()
        .take_while(|b| b.is_ascii_hexdigit())
        .count();
    (rest.starts_with('#') && digits > 0).then_some(1 + digits)
}

/// `"[^\n\r"]*"` or `'[^\n\r']*'`.
fn plain_string_len(rest: &str) -> Option<usize> {
    let quote = match rest.as_bytes().first()? {
        b'"' => b'"',
        b'\'' => b'\'',
        _ => return None,
    };
    let body = &rest.as_bytes()[1..];
    let end = body
        .iter()
        .position(|b| *b == quote || *b == b'\n' || *b == b'\r')?;
    (body[end] == quote).then_some(end + 2)
}

/// `[A-Z_a-z]\w*(?:-\w+)+`: a name with at least one dash-separated part. The first alternative of
/// the reference (`--` …) can never match, since `--` is matched by the marker-less arrow first.
fn css_identifier_len(rest: &str) -> Option<usize> {
    let first = *rest.as_bytes().first()?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let mut len = word_len(rest);
    let mut groups = 0;
    while rest.as_bytes().get(len) == Some(&b'-') {
        let part = word_len(&rest[len + 1..]);
        if part == 0 {
            break;
        }
        len += 1 + part;
        groups += 1;
    }
    (groups > 0).then_some(len)
}

/// `[^\t\n\r !-~]+`: a run of characters outside printable ASCII.
fn label_symbol_len(rest: &str) -> Option<usize> {
    let len: usize = rest
        .chars()
        .take_while(|c| !matches!(c, '\t' | '\n' | '\r' | ' ') && !('!'..='~').contains(c))
        .map(char::len_utf8)
        .sum();
    (len > 0).then_some(len)
}

/// The JSON body: the object from its `{` to the matching `}`, or
/// the rest of the input when it never closes (`matchJsonObject` and `UnclosedJsonObjectLiteral`).
fn json_body_match(rest: &str) -> Option<(Tok, usize, Action)> {
    if !rest.starts_with('{') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some((Tok::JsonObjectLiteral, index + 1, Action::Pop));
                }
            }
            _ => {}
        }
    }
    Some((Tok::UnclosedJsonObjectLiteral, rest.len(), Action::Pop))
}

/// The stereotype mode: `>>` ends it; the text up to `>>` on one line that is not blank is the
/// text; otherwise the rest of the line is an unclosed text (`StereotypeText`, `UnclosedStereotypeText`).
fn stereotype_match(text: &str, at: usize) -> Option<(Tok, usize, Action)> {
    let rest = &text[at..];
    if rest.starts_with(">>") {
        return Some((Tok::StereotypeEnd, 2, Action::Pop));
    }
    if let Some(end) = rest.find(">>") {
        let image = &rest[..end];
        if !image.contains(['\n', '\r']) && !js_trim(image).is_empty() {
            return Some((Tok::StereotypeText, end, Action::Plain));
        }
    }
    let len = rest.find(['\n', '\r']).unwrap_or(rest.len());
    (len > 0).then_some((Tok::UnclosedStereotypeText, len, Action::Pop))
}
