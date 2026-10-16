//! YAML as the TypeScript reference reads it: `js-yaml`'s `load` with `JSON_SCHEMA`.
//!
//! The reference reads the front matter, the shape data of flowcharts and the other YAML of the
//! diagrams with this one configuration, so this module is shared by every parser that needs YAML.
//! The structure comes from `saphyr-parser`, a YAML 1.2 parser. The typing and the errors are ours,
//! following `js-yaml` 4 (`lib/type/null.js`, `bool.js`, `int.js`, `float.js`, `schema/json.js` and
//! `loader.js`), so that the differential run (`pnpm port:diff yaml`) can compare them:
//!
//! - a plain scalar is typed by the JSON schema: `null` (`~`, `null`, `Null`, `NULL`, or an empty
//!   node), booleans in the spellings `true`/`True`/`TRUE` and `false`/`False`/`FALSE`, integers
//!   (decimal, `0b`, `0x`, `0o`) and floats (decimal, `.inf`, `.nan`); anything else is a string;
//! - a quoted or block scalar is always a string;
//! - mapping keys become strings, as `String(key)` makes them, and a repeated key is an error;
//! - a document that is not the only one in the stream is an error; so is a tag other than the
//!   core ones, and a core tag on a node of the wrong kind or with a value it cannot resolve.
//!
//! Numbers are `f64`, as in JavaScript, so `.inf` and `.nan` are numbers too; the configuration
//! value of [`Yaml`] (`crate::value::Value`) keeps them as numbers as well.

use std::collections::HashMap;
use std::fmt;

use ferrigura_model::value::Value;
use regex::Regex;
use saphyr_parser::{Event, Parser, ScalarStyle, Span, Tag};

/// A YAML value as `js-yaml` with `JSON_SCHEMA` reads it.
#[derive(Debug, Clone, PartialEq)]
pub enum Yaml {
    /// `null`, `~`, or an empty node.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, including `.inf`, `-.inf` and `.nan`.
    Number(f64),
    /// A string.
    String(String),
    /// A sequence, in order.
    Sequence(Vec<Yaml>),
    /// A mapping, in the order of its keys. Keys are unique.
    Mapping(Vec<(String, Yaml)>),
}

/// A YAML error. Positions are as `saphyr-parser` reports them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlError {
    /// Line of the error.
    pub line: usize,
    /// Column of the error.
    pub column: usize,
    /// What went wrong.
    pub message: String,
}

impl fmt::Display for YamlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for YamlError {}

/// The configuration value of a YAML value. Numbers stay numbers, `.inf` and `.nan` included.
impl From<&Yaml> for Value {
    fn from(value: &Yaml) -> Self {
        match value {
            Yaml::Null => Value::Null,
            Yaml::Bool(flag) => Value::Bool(*flag),
            Yaml::Number(number) => Value::Number(*number),
            Yaml::String(text) => Value::String(text.clone()),
            Yaml::Sequence(items) => Value::Array(items.iter().map(Value::from).collect()),
            Yaml::Mapping(pairs) => Value::Object(
                pairs
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::from(value)))
                    .collect(),
            ),
        }
    }
}

impl Yaml {
    /// `String(value)` as JavaScript gives it: arrays join their items with commas (an empty item
    /// is the empty string), and a mapping is `[object Object]`.
    #[must_use]
    pub fn js_string(&self) -> String {
        match self {
            Self::Null => "null".to_owned(),
            Self::Bool(flag) => flag.to_string(),
            Self::Number(number) => js_number_string(*number),
            Self::String(text) => text.clone(),
            Self::Sequence(items) => items
                .iter()
                .map(|item| {
                    if *item == Self::Null {
                        String::new()
                    } else {
                        item.js_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
            Self::Mapping(_) => "[object Object]".to_owned(),
        }
    }

    /// The string of a truthy value, as `if (value) String(value)` gives it; `None` for the falsy
    /// values: `null`, `false`, zero, `NaN` and the empty string.
    #[must_use]
    pub fn truthy_string(&self) -> Option<String> {
        let truthy = match self {
            Self::Null => false,
            Self::Bool(flag) => *flag,
            Self::Number(number) => *number != 0.0 && !number.is_nan(),
            Self::String(text) => !text.is_empty(),
            Self::Sequence(_) | Self::Mapping(_) => true,
        };
        truthy.then(|| self.js_string())
    }
}

/// Loads the single document of `text`, as `js-yaml`'s `load` does. `None` is a stream with no
/// document to read, which `load` returns as `undefined`.
pub fn load(text: &str) -> Result<Option<Yaml>, YamlError> {
    if let Some(position) = text.find('\0') {
        let before = &text[..position];
        return Err(YamlError {
            line: before.matches('\n').count() + 1,
            column: before
                .rsplit('\n')
                .next()
                .map_or(0, |line| line.chars().count())
                + 1,
            message: "null byte is not allowed in input".to_owned(),
        });
    }
    let mut source = text.to_owned();
    let root = loop {
        match read(&source) {
            Ok(root) => break root,
            Err(Unread::Builder(error)) => return Err(error),
            Err(Unread::Scanner(error)) => match lenient_indentation(&source, &error) {
                Some(reindented) => source = reindented,
                None => return Err(error),
            },
        }
    };
    Ok(match root {
        Some(root) => Some(root),
        None if document_is_read(text) => Some(Yaml::Null),
        None => None,
    })
}

/// Why `read` stopped: the scanner of `saphyr-parser` rejected the text, or the typing did.
enum Unread {
    Scanner(YamlError),
    Builder(YamlError),
}

fn read(text: &str) -> Result<Option<Yaml>, Unread> {
    let mut builder = Builder::default();
    for event in Parser::new_from_str(text) {
        let (event, span) = event.map_err(|error| {
            Unread::Scanner(YamlError {
                line: error.marker().line(),
                column: error.marker().col(),
                message: error.info().to_owned(),
            })
        })?;
        builder.event(event, &span).map_err(Unread::Builder)?;
    }
    Ok(builder.root)
}

/// The text re-indented where `js-yaml` accepts the indentation of a quoted scalar and
/// `saphyr-parser` does not, or `None`. The lines of a quoted scalar after its first may start at
/// any column, with tabs too: `readDoubleQuotedScalar` and `readSingleQuotedScalar` only warn about
/// a deficient indentation, and `load` sets no `onWarning`. Their leading blanks are not part of
/// the value in either reader, so they can be replaced.
///
/// For a tab in the indentation, `error` points at the tab, and the tabs of that line's leading
/// blanks become spaces. For a deficient indentation, it points at the opening quote, and every
/// later line of the scalar gets one column more than the quote, which no enclosing block needs to
/// exceed; a line of blanks only becomes empty. A line that `js-yaml` reads as a document marker
/// ends the scalar with an error there too, so that text is left as it is.
fn lenient_indentation(text: &str, error: &YamlError) -> Option<String> {
    let at = byte_offset(text, error.line, error.column)?;
    match error.message.as_str() {
        "tab cannot be used as indentation" => {
            let line_start = text[..at].rfind(['\n', '\r']).map_or(0, |i| i + 1);
            let blanks = text[line_start..]
                .find(|c| c != ' ' && c != '\t')
                .map_or(text.len(), |i| line_start + i);
            let spaces = " ".repeat(blanks - line_start);
            Some(format!(
                "{}{spaces}{}",
                &text[..line_start],
                &text[blanks..]
            ))
        }
        "invalid indentation in quoted scalar" => reindent_quoted(text, at, error.column + 1),
        _ => None,
    }
}

/// The byte offset of a `saphyr-parser` position: `line` counts from 1, with `\r\n`, `\n` and a
/// lone `\r` each ending a line, and `column` counts characters from 0.
fn byte_offset(text: &str, line: usize, column: usize) -> Option<usize> {
    let mut start = 0;
    for _ in 1..line {
        let rest = &text[start..];
        let end = rest.find(['\n', '\r'])?;
        start += end
            + if rest[end..].starts_with("\r\n") {
                2
            } else {
                1
            };
    }
    let (offset, _) = text[start..].char_indices().nth(column)?;
    Some(start + offset)
}

/// The text with the lines after the first of the quoted scalar that opens at byte `quote`
/// indented by `indent` spaces, or `None` when nothing changes or `js-yaml` rejects the scalar too.
fn reindent_quoted(text: &str, quote: usize, indent: usize) -> Option<String> {
    let single = match text[quote..].chars().next()? {
        '\'' => true,
        '"' => false,
        _ => return None,
    };
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..=quote]);
    let mut chars = text[quote + 1..].char_indices().peekable();
    let base = quote + 1;
    loop {
        let (i, c) = chars.next()?;
        match c {
            '\\' if !single => {
                out.push(c);
                if let Some(&(_, next)) = chars.peek()
                    && next != '\n'
                    && next != '\r'
                {
                    out.push(next);
                    chars.next();
                }
            }
            '\'' if single => {
                out.push(c);
                if chars.peek().map(|&(_, next)| next) != Some('\'') {
                    break;
                }
                out.push('\'');
                chars.next();
            }
            '"' if !single => {
                out.push(c);
                break;
            }
            '\n' | '\r' => {
                out.push(c);
                let mut line = base + i + 1;
                if c == '\r' && chars.peek().map(|&(_, next)| next) == Some('\n') {
                    out.push('\n');
                    chars.next();
                    line += 1;
                }
                let rest = &text[line..];
                if is_document_marker(rest) {
                    return None;
                }
                let blanks = rest.len() - rest.trim_start_matches([' ', '\t']).len();
                let after = &rest[blanks..];
                if !(after.is_empty() || after.starts_with(['\n', '\r'])) {
                    out.push_str(&" ".repeat(indent));
                }
                while chars.peek().is_some_and(|&(j, _)| base + j < line + blanks) {
                    chars.next();
                }
            }
            _ => out.push(c),
        }
    }
    out.push_str(&text[base + chars.peek().map_or(text.len() - base, |&(j, _)| j)..]);
    (out != text).then_some(out)
}

/// `testDocumentSeparator` of `js-yaml` at the start of a line: `---` or `...`, then a blank, a line
/// break or the end of the text.
fn is_document_marker(rest: &str) -> bool {
    (rest.starts_with("---") || rest.starts_with("..."))
        && rest[3..]
            .chars()
            .next()
            .is_none_or(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
}

/// Whether `js-yaml` reads a document from `text`. Its reader stops before the last character of
/// the input, which is `text` with a line break added when it has none, and without a byte order
/// mark. Leading spaces are skipped first. So an empty text, or spaces and one line break, has
/// no document, while a comment alone has one that is `null`.
fn document_is_read(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let mut units: Vec<u16> = text.encode_utf16().collect();
    if !matches!(units.last(), Some(0x0A | 0x0D)) {
        units.push(0x0A);
    }
    if units.first() == Some(&0xFEFF) {
        units.remove(0);
    }
    let leading_spaces = units.iter().take_while(|&&unit| unit == 0x20).count();
    leading_spaces + 1 < units.len()
}

#[derive(Default)]
struct Builder {
    documents: usize,
    root: Option<Yaml>,
    stack: Vec<Frame>,
    anchors: HashMap<usize, Yaml>,
}

enum Frame {
    Sequence {
        items: Vec<Yaml>,
        anchor: usize,
    },
    Mapping {
        pairs: Vec<(String, Yaml)>,
        key: Option<String>,
        anchor: usize,
    },
}

impl Builder {
    fn event(&mut self, event: Event<'_>, span: &Span) -> Result<(), YamlError> {
        match event {
            Event::DocumentStart(_) => {
                self.documents += 1;
                if self.documents > 1 {
                    return Err(error_at(
                        span,
                        "expected a single document in the stream, but found more",
                    ));
                }
            }
            Event::Scalar(text, style, anchor, tag) => {
                let value = scalar(&text, style, tag.as_deref())
                    .map_err(|message| error_at(span, &message))?;
                self.remember(anchor, &value);
                self.place(value, span)?;
            }
            Event::Alias(id) => {
                let value = self
                    .anchors
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| error_at(span, "unidentified alias"))?;
                self.place(value, span)?;
            }
            Event::SequenceStart(anchor, tag) => {
                collection_tag(tag.as_deref(), Kind::Sequence)
                    .map_err(|message| error_at(span, &message))?;
                self.stack.push(Frame::Sequence {
                    items: Vec::new(),
                    anchor,
                });
            }
            Event::SequenceEnd => match self.stack.pop() {
                Some(Frame::Sequence { items, anchor }) => {
                    let value = Yaml::Sequence(items);
                    self.remember(anchor, &value);
                    self.place(value, span)?;
                }
                _ => return Err(error_at(span, "unexpected end of a sequence")),
            },
            Event::MappingStart(anchor, tag) => {
                collection_tag(tag.as_deref(), Kind::Mapping)
                    .map_err(|message| error_at(span, &message))?;
                self.stack.push(Frame::Mapping {
                    pairs: Vec::new(),
                    key: None,
                    anchor,
                });
            }
            Event::MappingEnd => match self.stack.pop() {
                Some(Frame::Mapping {
                    pairs,
                    key: None,
                    anchor,
                }) => {
                    let value = Yaml::Mapping(pairs);
                    self.remember(anchor, &value);
                    self.place(value, span)?;
                }
                _ => return Err(error_at(span, "a mapping key has no value")),
            },
            _ => {}
        }
        Ok(())
    }

    fn remember(&mut self, anchor: usize, value: &Yaml) {
        if anchor != 0 {
            self.anchors.insert(anchor, value.clone());
        }
    }

    /// Puts a completed node where it belongs: the root, the next item of a sequence, or the key
    /// or value of a mapping.
    fn place(&mut self, value: Yaml, span: &Span) -> Result<(), YamlError> {
        match self.stack.last_mut() {
            None => {
                if self.root.is_some() {
                    return Err(error_at(
                        span,
                        "expected a single document in the stream, but found more",
                    ));
                }
                self.root = Some(value);
            }
            Some(Frame::Sequence { items, .. }) => items.push(value),
            Some(Frame::Mapping { pairs, key, .. }) => match key.take() {
                None => {
                    *key = Some(key_string(&value).map_err(|message| error_at(span, &message))?);
                }
                Some(name) => {
                    if pairs.iter().any(|(existing, _)| *existing == name) {
                        return Err(error_at(span, "duplicated mapping key"));
                    }
                    pairs.push((name, value));
                }
            },
        }
        Ok(())
    }
}

fn error_at(span: &Span, message: &str) -> YamlError {
    YamlError {
        line: span.start.line(),
        column: span.start.col(),
        message: message.to_owned(),
    }
}

/// The string a mapping key becomes, as `storeMappingPair` makes it with `String(keyNode)`.
fn key_string(value: &Yaml) -> Result<String, String> {
    match value {
        Yaml::Sequence(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                parts.push(match item {
                    Yaml::Sequence(_) => {
                        return Err("nested arrays are not supported inside keys".to_owned());
                    }
                    Yaml::Mapping(_) => "[object Object]".to_owned(),
                    other => plain_string(other),
                });
            }
            Ok(parts.join(","))
        }
        Yaml::Mapping(_) => Ok("[object Object]".to_owned()),
        other => Ok(plain_string(other)),
    }
}

/// `String(value)` of a scalar, as JavaScript prints it.
fn plain_string(value: &Yaml) -> String {
    match value {
        Yaml::Null => "null".to_owned(),
        Yaml::Bool(flag) => flag.to_string(),
        Yaml::Number(number) => js_number_string(*number),
        Yaml::String(text) => text.clone(),
        Yaml::Sequence(_) | Yaml::Mapping(_) => String::new(),
    }
}

/// `Number.prototype.toString` as ECMAScript defines it for the shortest round-trip digits.
fn js_number_string(number: f64) -> String {
    if number.is_nan() {
        return "NaN".to_owned();
    }
    if number.is_infinite() {
        return if number > 0.0 {
            "Infinity"
        } else {
            "-Infinity"
        }
        .to_owned();
    }
    if number == 0.0 {
        return "0".to_owned();
    }
    let scientific = format!("{:e}", number.abs());
    let (mantissa, exponent) = scientific
        .split_once('e')
        .unwrap_or((scientific.as_str(), "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let length = digits.len() as i32;
    let point = exponent + 1;
    let body = if length <= point && point <= 21 {
        format!("{digits}{}", "0".repeat((point - length) as usize))
    } else if 0 < point && point <= 21 {
        let (whole, fraction) = digits.split_at(point as usize);
        format!("{whole}.{fraction}")
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else {
        let (first, rest) = digits.split_at(1);
        let rest = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let sign = if point > 0 { "+" } else { "-" };
        format!("{first}{rest}e{sign}{}", (point - 1).abs())
    };
    if number < 0.0 {
        format!("-{body}")
    } else {
        body
    }
}

/// The core tags the schema resolves, and the non-specific `!`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CoreTag {
    Str,
    Int,
    Float,
    Bool,
    Null,
    Seq,
    Map,
    NonSpecific,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Sequence,
    Mapping,
}

const CORE_PREFIX: &str = "tag:yaml.org,2002:";

/// The core tag of a node, if the node has one. A tag outside the core set is an error, as
/// `js-yaml` reports it.
fn core_tag(tag: Option<&Tag>) -> Result<Option<CoreTag>, String> {
    let Some(tag) = tag else {
        return Ok(None);
    };
    let full = if tag.handle == CORE_PREFIX {
        format!("{CORE_PREFIX}{}", tag.suffix)
    } else if tag.handle == "!" && tag.suffix.is_empty() {
        "!".to_owned()
    } else if tag.handle.is_empty() {
        tag.suffix.clone()
    } else {
        format!("{}{}", tag.handle, tag.suffix)
    };
    let core = match full.as_str() {
        "!" => CoreTag::NonSpecific,
        "tag:yaml.org,2002:str" => CoreTag::Str,
        "tag:yaml.org,2002:int" => CoreTag::Int,
        "tag:yaml.org,2002:float" => CoreTag::Float,
        "tag:yaml.org,2002:bool" => CoreTag::Bool,
        "tag:yaml.org,2002:null" => CoreTag::Null,
        "tag:yaml.org,2002:seq" => CoreTag::Seq,
        "tag:yaml.org,2002:map" => CoreTag::Map,
        other => return Err(format!("unknown tag !<{other}>")),
    };
    Ok(Some(core))
}

/// Checks the tag of a collection: none, the non-specific one, or the core tag of its kind.
fn collection_tag(tag: Option<&Tag>, kind: Kind) -> Result<(), String> {
    match core_tag(tag)? {
        None | Some(CoreTag::NonSpecific) => Ok(()),
        Some(CoreTag::Seq) if kind == Kind::Sequence => Ok(()),
        Some(CoreTag::Map) if kind == Kind::Mapping => Ok(()),
        Some(_) => Err("unacceptable node kind for the tag of a collection".to_owned()),
    }
}

/// The value of a scalar: typed by its tag if it has a core one, otherwise by the schema when the
/// scalar is plain, and as a string when it is quoted or a block scalar.
fn scalar(text: &str, style: ScalarStyle, tag: Option<&Tag>) -> Result<Yaml, String> {
    let plain = style == ScalarStyle::Plain;
    match core_tag(tag)? {
        None => Ok(if plain {
            implicit(text)
        } else {
            Yaml::String(text.to_owned())
        }),
        // The non-specific tag `!` makes any scalar a string.
        Some(CoreTag::NonSpecific) => Ok(Yaml::String(text.to_owned())),
        Some(CoreTag::Str) => Ok(Yaml::String(text.to_owned())),
        Some(CoreTag::Int) => resolve_integer(text)
            .map(Yaml::Number)
            .ok_or_else(|| "cannot resolve a node with !<tag:yaml.org,2002:int> explicit tag".to_owned()),
        Some(CoreTag::Float) => resolve_float(text)
            .map(Yaml::Number)
            .ok_or_else(|| "cannot resolve a node with !<tag:yaml.org,2002:float> explicit tag".to_owned()),
        Some(CoreTag::Bool) => resolve_bool(text)
            .map(Yaml::Bool)
            .ok_or_else(|| "cannot resolve a node with !<tag:yaml.org,2002:bool> explicit tag".to_owned()),
        Some(CoreTag::Null) => {
            if resolve_null(text) {
                Ok(Yaml::Null)
            } else {
                Err("cannot resolve a node with !<tag:yaml.org,2002:null> explicit tag".to_owned())
            }
        }
        Some(CoreTag::Seq | CoreTag::Map) => {
            Err("unacceptable node kind for the tag of a collection; it should be a sequence or a mapping, not a scalar".to_owned())
        }
    }
}

/// The schema's typing of a plain scalar: null, boolean, integer, float, and otherwise a string.
/// An empty plain node is null, as `js-yaml` reads a key with no value.
fn implicit(text: &str) -> Yaml {
    if text.is_empty() || resolve_null(text) {
        return Yaml::Null;
    }
    if let Some(flag) = resolve_bool(text) {
        return Yaml::Bool(flag);
    }
    if let Some(number) = resolve_integer(text).or_else(|| resolve_float(text)) {
        return Yaml::Number(number);
    }
    Yaml::String(text.to_owned())
}

fn resolve_null(text: &str) -> bool {
    matches!(text, "~" | "null" | "Null" | "NULL")
}

fn resolve_bool(text: &str) -> Option<bool> {
    match text {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

/// The schema's integer: an optional sign, then decimal digits, or `0b`, `0x`, `0o` with digits
/// of their base. `None` when the text is not one, or when its value is not finite.
fn resolve_integer(text: &str) -> Option<f64> {
    let (sign, body) = match text.as_bytes().first() {
        Some(b'-') => (-1.0, &text[1..]),
        Some(b'+') => (1.0, &text[1..]),
        _ => (1.0, text),
    };
    if body == "0" {
        return Some(0.0);
    }
    let (radix, digits) = match body.as_bytes() {
        [b'0', b'b', ..] => (2, &body[2..]),
        [b'0', b'x', ..] => (16, &body[2..]),
        [b'0', b'o', ..] => (8, &body[2..]),
        _ => (10, body),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let value = if radix == 10 {
        digits.parse::<f64>().ok()?
    } else {
        digits.chars().fold(0.0, |acc, c| {
            acc * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
        })
    };
    value.is_finite().then_some(sign * value)
}

/// The schema's float pattern, including `.inf` and `.nan`. A decimal that overflows to infinity is
/// not a float, as in `js-yaml`.
fn resolve_float(text: &str) -> Option<f64> {
    static FLOAT: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static SPECIAL: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let float = FLOAT.get_or_init(|| {
        Regex::new(
            r"^(?:[-+]?[0-9]+(?:\.[0-9]*)?(?:[eE][-+]?[0-9]+)?|\.[0-9]+(?:[eE][-+]?[0-9]+)?|[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN))$",
        )
        .unwrap_or_else(|_| unreachable!("a fixed pattern"))
    });
    let special = SPECIAL.get_or_init(|| {
        Regex::new(r"^(?:[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN))$")
            .unwrap_or_else(|_| unreachable!("a fixed pattern"))
    });
    if !float.is_match(text) {
        return None;
    }
    if let Ok(value) = text.parse::<f64>()
        && value.is_finite()
    {
        return Some(value);
    }
    if !special.is_match(text) {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    let negative = lower.starts_with('-');
    let name = lower.trim_start_matches(['-', '+']);
    if name == ".nan" {
        Some(f64::NAN)
    } else if negative {
        Some(f64::NEG_INFINITY)
    } else {
        Some(f64::INFINITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> Yaml {
        load(text).expect("loads").expect("has a document")
    }

    #[test]
    fn plain_scalars_follow_the_json_schema() {
        assert_eq!(value("a: ~"), Yaml::Mapping(vec![("a".into(), Yaml::Null)]));
        assert_eq!(
            value("a: TRUE"),
            Yaml::Mapping(vec![("a".into(), Yaml::Bool(true))])
        );
        assert_eq!(
            value("a: tRUE"),
            Yaml::Mapping(vec![("a".into(), Yaml::String("tRUE".into()))])
        );
        assert_eq!(
            value("a: 0x1F"),
            Yaml::Mapping(vec![("a".into(), Yaml::Number(31.0))])
        );
        assert_eq!(
            value("a: -0o17"),
            Yaml::Mapping(vec![("a".into(), Yaml::Number(-15.0))])
        );
        assert_eq!(
            value("a: 007"),
            Yaml::Mapping(vec![("a".into(), Yaml::Number(7.0))])
        );
        assert_eq!(
            value("a: .5"),
            Yaml::Mapping(vec![("a".into(), Yaml::Number(0.5))])
        );
        assert_eq!(
            value("a: 1e5"),
            Yaml::Mapping(vec![("a".into(), Yaml::Number(100000.0))])
        );
        assert_eq!(
            value("a: 1_000"),
            Yaml::Mapping(vec![("a".into(), Yaml::String("1_000".into()))])
        );
        assert_eq!(
            value("a: -.5"),
            Yaml::Mapping(vec![("a".into(), Yaml::String("-.5".into()))])
        );
        assert_eq!(
            value("a: 1e400"),
            Yaml::Mapping(vec![("a".into(), Yaml::String("1e400".into()))])
        );
    }

    #[test]
    fn infinity_and_nan_are_numbers() {
        let Yaml::Mapping(pairs) = value("a: .inf\nb: -.Inf\nc: .NAN") else {
            panic!("a mapping");
        };
        assert_eq!(pairs[0].1, Yaml::Number(f64::INFINITY));
        assert_eq!(pairs[1].1, Yaml::Number(f64::NEG_INFINITY));
        assert!(matches!(pairs[2].1, Yaml::Number(number) if number.is_nan()));
        assert_eq!(Value::from(&pairs[0].1), Value::Number(f64::INFINITY));
    }

    #[test]
    fn quoted_and_block_scalars_are_strings() {
        assert_eq!(
            value("a: 'true'"),
            Yaml::Mapping(vec![("a".into(), Yaml::String("true".into()))])
        );
        assert_eq!(
            value("a: |\n  x\n  y\n"),
            Yaml::Mapping(vec![("a".into(), Yaml::String("x\ny\n".into()))])
        );
        assert_eq!(
            value("a: >\n  x\n  y\n"),
            Yaml::Mapping(vec![("a".into(), Yaml::String("x y\n".into()))])
        );
    }

    #[test]
    fn quoted_lines_may_have_any_indentation() {
        let inner = |text: &str| match value(text) {
            Yaml::Mapping(pairs) => pairs[0].1.clone(),
            other => other,
        };
        let pair = |value: &str| Yaml::Mapping(vec![("b".into(), Yaml::String(value.into()))]);
        assert_eq!(inner("a:\n  b: \"x\ny\""), pair("x y"));
        assert_eq!(inner("a:\n  b: \"x\n\ty\""), pair("x y"));
        assert_eq!(inner("a:\n  b: 'x\n y ''z'''"), pair("x y 'z'"));
        assert_eq!(inner("a:\n  b: \"x\n   \n y\n  \""), pair("x\ny "));
        assert_eq!(inner("a:\n  b: \"x \\\ny\""), pair("x y"));
        assert!(load("a:\n  b: \"x\n---\ny\"").is_err());
        assert!(load("a:\n  b: \"x\ny").is_err());
    }

    #[test]
    fn keys_become_strings() {
        assert_eq!(
            value("1.0: a"),
            Yaml::Mapping(vec![("1".into(), Yaml::String("a".into()))])
        );
        assert_eq!(
            value("null: a"),
            Yaml::Mapping(vec![("null".into(), Yaml::String("a".into()))])
        );
        assert_eq!(
            value("[a, b]: c"),
            Yaml::Mapping(vec![("a,b".into(), Yaml::String("c".into()))])
        );
    }

    #[test]
    fn a_repeated_key_is_an_error() {
        assert!(load("a: 1\na: 2").is_err());
    }

    #[test]
    fn two_documents_are_an_error() {
        assert!(load("--- 1\n--- 2").is_err());
    }

    #[test]
    fn an_empty_stream_is_no_document() {
        assert_eq!(load("").expect("loads"), None);
    }

    #[test]
    fn core_tags_convert_and_others_are_errors() {
        assert_eq!(
            value("a: !!str 1"),
            Yaml::Mapping(vec![("a".into(), Yaml::String("1".into()))])
        );
        assert_eq!(
            value("a: !!int \"1\""),
            Yaml::Mapping(vec![("a".into(), Yaml::Number(1.0))])
        );
        assert!(load("a: !!int x").is_err());
        assert!(load("a: !!binary aGk=").is_err());
        assert!(load("a: !local 1").is_err());
    }

    #[test]
    fn aliases_copy_the_anchored_value() {
        assert_eq!(
            value("a: &x 1\nb: *x"),
            Yaml::Mapping(vec![
                ("a".into(), Yaml::Number(1.0)),
                ("b".into(), Yaml::Number(1.0))
            ])
        );
    }

    #[test]
    fn numbers_print_as_javascript_does() {
        assert_eq!(js_number_string(1.5), "1.5");
        assert_eq!(js_number_string(1e21), "1e+21");
        assert_eq!(js_number_string(1e-7), "1e-7");
        assert_eq!(js_number_string(-0.0), "0");
        assert_eq!(js_number_string(100.0), "100");
    }
}
