//! YAML as the TypeScript reference reads it: `js-yaml` 4's `load` with `JSON_SCHEMA`.
//!
//! The reference reads the front matter, the shape data of flowcharts and the other YAML of the
//! diagrams with this one configuration, so this module is shared by every parser that needs YAML.
//! The reader is a port of `js-yaml`'s own loader (`lib/loader.js` of the version the reference
//! installs), state for state: its indentation and tab rules, its flow collections, its block
//! scalars, its anchor transactions and its errors. `js-yaml` is not a YAML 1.2 reader in every
//! detail (a flow collection may continue at any indentation, a tab may separate a key from its
//! value, a `:` with no key is an error), so a general YAML parser does not agree with it, and the
//! differential run (`pnpm port:diff yaml`) compares the two over the inputs of `yaml-cases.ts`.
//!
//! As in `js-yaml`, a collection is an object while it is read: an alias to a collection that is
//! still open sees the entries it gets later. The finished document is converted to a tree; a
//! collection that contains itself has no tree, and `js-yaml`'s canonical form of it does not end,
//! so it is an error here too.
//!
//! The typing is the JSON schema's, as `js-yaml` gives it (`lib/type/null.js`, `bool.js`, `int.js`,
//! `float.js`, `schema/json.js`):
//!
//! - a plain scalar is typed by the schema: `null` (`~`, `null`, `Null`, `NULL`, or an empty node),
//!   booleans in the spellings `true`/`True`/`TRUE` and `false`/`False`/`FALSE`, integers (decimal,
//!   `0b`, `0x`, `0o`) and floats (decimal, `.inf`, `.nan`); anything else is a string;
//! - a quoted or block scalar is always a string;
//! - an explicit core tag (`!!str`, `!!seq`, ...) types the node, and an empty node takes the tag's
//!   empty value (`""`, `[]`, `{}`) or `null`; a tag the schema does not know is an error;
//! - mapping keys become strings, as `String(key)` makes them, and a repeated key is an error;
//! - a stream with more than one document is an error; an empty stream is no document.
//!
//! Numbers are `f64`, as in JavaScript, so `.inf` and `.nan` are numbers too. A lone UTF-16
//! surrogate (a `\u` escape of one) is not a character, and a Rust string cannot hold it: it is read
//! as U+FFFD, which is what the reference's output (UTF-8) writes for it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use ferrigura_model::value::Value;
use regex::Regex;

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

/// A YAML error. The position is the reader's own: `line` counts from 1, `column` from 0.
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
    // `loadDocuments`: a line break is added at the end when there is none, and a byte order mark
    // is removed.
    let mut input: Vec<u16> = text.encode_utf16().collect();
    if !input.is_empty() {
        if !matches!(input.last(), Some(&LF | &CR)) {
            input.push(LF);
        }
        if input.first() == Some(&0xFEFF) {
            input.remove(0);
        }
    }
    if let Some(position) = input.iter().position(|&unit| unit == 0) {
        let before = &input[..position];
        let line_start = before
            .iter()
            .rposition(|&unit| unit == LF)
            .map_or(0, |i| i + 1);
        return Err(YamlError {
            line: before.iter().filter(|&&unit| unit == LF).count() + 1,
            column: position - line_start,
            message: "null byte is not allowed in input".to_owned(),
        });
    }
    let mut state = State::new(input);
    while state.at(state.position) == SPACE {
        state.line_indent += 1;
        state.position += 1;
    }
    while state.position + 1 < state.length {
        state.read_document()?;
    }
    match state.documents.len() {
        0 => Ok(None),
        1 => {
            let document = state.documents.pop().unwrap_or(Node::Null);
            document
                .to_yaml()
                .map(Some)
                .ok_or_else(|| state.error(CYCLIC))
        }
        _ => Err(state.error("expected a single document in the stream, but found more")),
    }
}

const CYCLIC: &str = "a collection that contains itself has no tree";

const TAB: u16 = 0x09;
const LF: u16 = 0x0A;
const CR: u16 = 0x0D;
const SPACE: u16 = 0x20;

/// `MAX_DEPTH` of `js-yaml`: the nesting of nodes.
const MAX_DEPTH: usize = 100;

fn unit(c: u8) -> u16 {
    u16::from(c)
}

fn is_eol(c: u16) -> bool {
    c == LF || c == CR
}

fn is_whitespace(c: u16) -> bool {
    c == TAB || c == SPACE
}

fn is_ws_or_eol(c: u16) -> bool {
    is_whitespace(c) || is_eol(c)
}

fn is_flow_indicator(c: u16) -> bool {
    matches!(c, 0x2C | 0x5B | 0x5D | 0x7B | 0x7D)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    FlowIn,
    FlowOut,
    BlockIn,
    BlockOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Scalar,
    Sequence,
    Mapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chomping {
    Clip,
    Strip,
    Keep,
}

/// A node while it is read. A collection is shared, as `js-yaml`'s object is: the reader fills it
/// in place, and an alias to it gets the same object.
#[derive(Clone)]
enum Node {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Sequence(Rc<RefCell<Vec<Node>>>),
    Mapping(Rc<RefCell<Vec<(String, Node)>>>),
}

impl Node {
    /// The tree of a finished node, or `None` when a collection contains itself.
    fn to_yaml(&self) -> Option<Yaml> {
        self.to_yaml_on(&mut Vec::new())
    }

    /// `path` holds the collections being converted, outermost first.
    fn to_yaml_on(&self, path: &mut Vec<*const ()>) -> Option<Yaml> {
        Some(match self {
            Self::Null => Yaml::Null,
            Self::Bool(flag) => Yaml::Bool(*flag),
            Self::Number(number) => Yaml::Number(*number),
            Self::String(text) => Yaml::String(text.clone()),
            Self::Sequence(items) => {
                let id = Rc::as_ptr(items).cast::<()>();
                if path.contains(&id) {
                    return None;
                }
                path.push(id);
                let converted = items
                    .borrow()
                    .iter()
                    .map(|item| item.to_yaml_on(path))
                    .collect::<Option<Vec<_>>>()?;
                path.pop();
                Yaml::Sequence(converted)
            }
            Self::Mapping(pairs) => {
                let id = Rc::as_ptr(pairs).cast::<()>();
                if path.contains(&id) {
                    return None;
                }
                path.push(id);
                let converted = pairs
                    .borrow()
                    .iter()
                    .map(|(key, value)| value.to_yaml_on(path).map(|value| (key.clone(), value)))
                    .collect::<Option<Vec<_>>>()?;
                path.pop();
                Yaml::Mapping(converted)
            }
        })
    }

    fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

/// The types of the JSON schema, by the tag they answer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeId {
    Str,
    Seq,
    Map,
    Null,
    Bool,
    Int,
    Float,
}

impl TypeId {
    fn tag(self) -> &'static str {
        match self {
            Self::Str => "tag:yaml.org,2002:str",
            Self::Seq => "tag:yaml.org,2002:seq",
            Self::Map => "tag:yaml.org,2002:map",
            Self::Null => "tag:yaml.org,2002:null",
            Self::Bool => "tag:yaml.org,2002:bool",
            Self::Int => "tag:yaml.org,2002:int",
            Self::Float => "tag:yaml.org,2002:float",
        }
    }

    fn kind(self) -> Kind {
        match self {
            Self::Str | Self::Null | Self::Bool | Self::Int | Self::Float => Kind::Scalar,
            Self::Seq => Kind::Sequence,
            Self::Map => Kind::Mapping,
        }
    }
}

/// `typeMap[kind || 'fallback'][tag]`: the type a tag names among those of a node's kind.
fn find_type(tag: &str, kind: Option<Kind>) -> Option<TypeId> {
    const SCALARS: [TypeId; 5] = [
        TypeId::Str,
        TypeId::Null,
        TypeId::Bool,
        TypeId::Int,
        TypeId::Float,
    ];
    const ALL: [TypeId; 7] = [
        TypeId::Str,
        TypeId::Seq,
        TypeId::Map,
        TypeId::Null,
        TypeId::Bool,
        TypeId::Int,
        TypeId::Float,
    ];
    let candidates: &[TypeId] = match kind {
        Some(Kind::Scalar) => &SCALARS,
        Some(Kind::Sequence) => &[TypeId::Seq],
        Some(Kind::Mapping) => &[TypeId::Map],
        None => &ALL,
    };
    candidates.iter().copied().find(|t| t.tag() == tag)
}

/// `resolve` then `construct` of a type for the data of a node: `None` when the type does not
/// accept the data. The string type and the collections take an empty node as their empty value.
fn construct(t: TypeId, data: &Node) -> Option<Node> {
    match t {
        TypeId::Str => Some(match data {
            Node::Null => Node::String(String::new()),
            other => other.clone(),
        }),
        TypeId::Seq => Some(match data {
            Node::Null => Node::Sequence(Rc::new(RefCell::new(Vec::new()))),
            other => other.clone(),
        }),
        TypeId::Map => Some(match data {
            Node::Null => Node::Mapping(Rc::new(RefCell::new(Vec::new()))),
            other => other.clone(),
        }),
        TypeId::Null => match data {
            Node::Null => Some(Node::Null),
            Node::String(text) if resolve_null(text) => Some(Node::Null),
            _ => None,
        },
        TypeId::Bool => match data {
            Node::String(text) => resolve_bool(text).map(Node::Bool),
            _ => None,
        },
        TypeId::Int => match data {
            Node::String(text) => resolve_integer(text).map(Node::Number),
            _ => None,
        },
        TypeId::Float => match data {
            Node::String(text) => resolve_float(text).map(Node::Number),
            _ => None,
        },
    }
}

/// The implicit types of a plain scalar that has no tag, in the order of the schema: the first one
/// that resolves the data gives the value and its tag.
fn implicit_resolve(data: &Node) -> Option<(TypeId, Node)> {
    [TypeId::Null, TypeId::Bool, TypeId::Int, TypeId::Float]
        .into_iter()
        .find_map(|t| construct(t, data).map(|value| (t, value)))
}

/// The state of `js-yaml`'s reader: the input in UTF-16 code units (so that columns and
/// indentation count as they do in JavaScript), and the position, the line and the node being read.
struct State {
    input: Vec<u16>,
    length: usize,
    position: usize,
    line: usize,
    line_start: usize,
    line_indent: isize,
    depth: usize,
    first_tab_in_line: Option<usize>,
    version_seen: bool,
    tag_map: HashMap<String, String>,
    anchor_map: HashMap<String, Node>,
    transactions: Vec<Vec<(String, Option<Node>)>>,
    documents: Vec<Node>,
    tag: Option<String>,
    anchor: Option<String>,
    kind: Option<Kind>,
    result: Node,
}

/// The reader's state, saved and restored by `js-yaml` when it reads a node twice.
#[derive(Clone)]
struct Snapshot {
    position: usize,
    line: usize,
    line_start: usize,
    line_indent: isize,
    first_tab_in_line: Option<usize>,
    tag: Option<String>,
    anchor: Option<String>,
    kind: Option<Kind>,
    result: Node,
}

type Res<T> = Result<T, YamlError>;

impl State {
    /// `input` ends with the line break that `load` adds; the terminating NUL is read past its end.
    fn new(mut input: Vec<u16>) -> Self {
        let length = input.len();
        input.push(0);
        Self {
            input,
            length,
            position: 0,
            line: 0,
            line_start: 0,
            line_indent: 0,
            depth: 0,
            first_tab_in_line: None,
            version_seen: false,
            tag_map: HashMap::new(),
            anchor_map: HashMap::new(),
            transactions: Vec::new(),
            documents: Vec::new(),
            tag: None,
            anchor: None,
            kind: None,
            result: Node::Null,
        }
    }

    /// The code unit at `index`; past the input it is the NUL that ends it.
    fn at(&self, index: usize) -> u16 {
        self.input.get(index).copied().unwrap_or(0)
    }

    fn ch(&self) -> u16 {
        self.at(self.position)
    }

    fn error(&self, message: &str) -> YamlError {
        YamlError {
            line: self.line + 1,
            column: self.position.saturating_sub(self.line_start),
            message: message.to_owned(),
        }
    }

    /// The string of captured code units. A lone surrogate becomes U+FFFD, as the output of the
    /// reference writes it.
    fn text(&self, units: &[u16]) -> String {
        String::from_utf16_lossy(units)
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            position: self.position,
            line: self.line,
            line_start: self.line_start,
            line_indent: self.line_indent,
            first_tab_in_line: self.first_tab_in_line,
            tag: self.tag.clone(),
            anchor: self.anchor.clone(),
            kind: self.kind,
            result: self.result.clone(),
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.position = snapshot.position;
        self.line = snapshot.line;
        self.line_start = snapshot.line_start;
        self.line_indent = snapshot.line_indent;
        self.first_tab_in_line = snapshot.first_tab_in_line;
        self.tag = snapshot.tag;
        self.anchor = snapshot.anchor;
        self.kind = snapshot.kind;
        self.result = snapshot.result;
    }

    /// `storeAnchor`: the first store in a transaction records what the name held before it.
    fn store_anchor(&mut self, name: &str, value: Node) {
        if let Some(transaction) = self.transactions.last_mut()
            && !transaction.iter().any(|(recorded, _)| recorded == name)
        {
            transaction.push((name.to_owned(), self.anchor_map.get(name).cloned()));
        }
        self.anchor_map.insert(name.to_owned(), value);
    }

    /// Stores the node that holds the anchor of the current node, if it has one.
    fn store_current_anchor(&mut self) {
        if let Some(name) = self.anchor.clone() {
            self.store_anchor(&name, self.result.clone());
        }
    }

    fn begin_transaction(&mut self) {
        self.transactions.push(Vec::new());
    }

    fn commit_transaction(&mut self) {
        let transaction = self.transactions.pop().unwrap_or_default();
        if let Some(parent) = self.transactions.last_mut() {
            for (name, previous) in transaction {
                if !parent.iter().any(|(recorded, _)| *recorded == name) {
                    parent.push((name, previous));
                }
            }
        }
    }

    fn rollback_transaction(&mut self) {
        let transaction = self.transactions.pop().unwrap_or_default();
        for (name, previous) in transaction.into_iter().rev() {
            match previous {
                Some(node) => {
                    self.anchor_map.insert(name, node);
                }
                None => {
                    self.anchor_map.remove(&name);
                }
            }
        }
    }

    /// `skipSeparationSpace`: the blanks, a comment when `allow_comments`, and the line breaks with
    /// the indentation of the next line. Returns the number of line breaks.
    fn skip_separation_space(&mut self, allow_comments: bool) -> usize {
        let mut line_breaks = 0;
        let mut ch = self.ch();
        while ch != 0 {
            while is_whitespace(ch) {
                if ch == TAB && self.first_tab_in_line.is_none() {
                    self.first_tab_in_line = Some(self.position);
                }
                self.position += 1;
                ch = self.ch();
            }
            if allow_comments && ch == unit(b'#') {
                loop {
                    self.position += 1;
                    ch = self.ch();
                    if ch == LF || ch == CR || ch == 0 {
                        break;
                    }
                }
            }
            if !is_eol(ch) {
                break;
            }
            self.break_line();
            ch = self.ch();
            line_breaks += 1;
            self.line_indent = 0;
            while ch == SPACE {
                self.line_indent += 1;
                self.position += 1;
                ch = self.ch();
            }
        }
        line_breaks
    }

    /// `readLineBreak` at a line break: LF, CR or CRLF.
    fn break_line(&mut self) {
        if self.ch() == LF {
            self.position += 1;
        } else {
            self.position += 1;
            if self.ch() == LF {
                self.position += 1;
            }
        }
        self.line += 1;
        self.line_start = self.position;
        self.first_tab_in_line = None;
    }

    fn read_line_break(&mut self) -> Res<()> {
        if !is_eol(self.ch()) {
            return Err(self.error("a line break is expected"));
        }
        self.break_line();
        Ok(())
    }

    /// `testDocumentSeparator`: `---` or `...` followed by a blank, a line break or the end.
    fn test_document_separator(&self) -> bool {
        let ch = self.ch();
        if (ch == unit(b'-') || ch == unit(b'.'))
            && ch == self.at(self.position + 1)
            && ch == self.at(self.position + 2)
        {
            let following = self.at(self.position + 3);
            return following == 0 || is_ws_or_eol(following);
        }
        false
    }

    /// `captureSegment`: appends the code units `start..end`. A quoted scalar may hold any character
    /// from a space on; a plain or block scalar may not hold a non-printable one.
    fn capture(&self, out: &mut Vec<u16>, start: usize, end: usize, check_json: bool) -> Res<()> {
        if start >= end {
            return Ok(());
        }
        let segment = &self.input[start..end];
        if check_json {
            if segment.iter().any(|&c| c != TAB && c < 0x20) {
                return Err(self.error("expected valid JSON character"));
            }
        } else if has_non_printable(segment) {
            return Err(self.error("the stream contains non-printable characters"));
        }
        out.extend_from_slice(segment);
        Ok(())
    }

    /// `readDocument`: the directives, the document's node, and the end of the document.
    fn read_document(&mut self) -> Res<()> {
        self.version_seen = false;
        self.tag_map.clear();
        self.anchor_map.clear();
        let mut has_directives = false;
        while self.ch() != 0 {
            self.skip_separation_space(true);
            let ch = self.ch();
            if self.line_indent > 0 || ch != unit(b'%') {
                break;
            }
            has_directives = true;
            self.position += 1;
            let mut ch = self.ch();
            let name_start = self.position;
            while ch != 0 && !is_ws_or_eol(ch) {
                self.position += 1;
                ch = self.ch();
            }
            let name = String::from_utf16_lossy(&self.input[name_start..self.position]);
            if name.is_empty() {
                return Err(
                    self.error("directive name must not be less than one character in length")
                );
            }
            let mut args: Vec<String> = Vec::new();
            while ch != 0 {
                while is_whitespace(ch) {
                    self.position += 1;
                    ch = self.ch();
                }
                if ch == unit(b'#') {
                    loop {
                        self.position += 1;
                        ch = self.ch();
                        if ch == 0 || is_eol(ch) {
                            break;
                        }
                    }
                    break;
                }
                if is_eol(ch) {
                    break;
                }
                let arg_start = self.position;
                while ch != 0 && !is_ws_or_eol(ch) {
                    self.position += 1;
                    ch = self.ch();
                }
                args.push(String::from_utf16_lossy(
                    &self.input[arg_start..self.position],
                ));
            }
            if ch != 0 {
                self.read_line_break()?;
            }
            match name.as_str() {
                "YAML" => self.yaml_directive(&args)?,
                "TAG" => self.tag_directive(&args)?,
                // An unknown directive is a warning in `js-yaml`.
                _ => {}
            }
        }

        self.skip_separation_space(true);
        if self.line_indent == 0
            && self.at(self.position) == unit(b'-')
            && self.at(self.position + 1) == unit(b'-')
            && self.at(self.position + 2) == unit(b'-')
        {
            self.position += 3;
            self.skip_separation_space(true);
        } else if has_directives {
            return Err(self.error("directives end mark is expected"));
        }

        self.compose_node(self.line_indent - 1, Context::BlockOut, false, true)?;
        self.skip_separation_space(true);
        let document = std::mem::replace(&mut self.result, Node::Null);
        self.documents.push(document);

        if self.position == self.line_start && self.test_document_separator() {
            if self.ch() == unit(b'.') {
                self.position += 3;
                self.skip_separation_space(true);
            }
            return Ok(());
        }
        if self.position + 1 < self.length {
            return Err(self.error("end of the stream or a document separator is expected"));
        }
        Ok(())
    }

    /// The `%YAML` directive: one version argument, of major version 1.
    fn yaml_directive(&mut self, args: &[String]) -> Res<()> {
        if self.version_seen {
            return Err(self.error("duplication of %YAML directive"));
        }
        let [version] = args else {
            return Err(self.error("YAML directive accepts exactly one argument"));
        };
        let Some((major, _minor)) = split_version(version) else {
            return Err(self.error("ill-formed argument of the YAML directive"));
        };
        if major.trim_start_matches('0') != "1" {
            return Err(self.error("unacceptable YAML version of the document"));
        }
        self.version_seen = true;
        Ok(())
    }

    /// The `%TAG` directive: a handle and a prefix, which is percent-decoded.
    fn tag_directive(&mut self, args: &[String]) -> Res<()> {
        let [handle, prefix] = args else {
            return Err(self.error("TAG directive accepts exactly two arguments"));
        };
        if !is_tag_handle(handle) {
            return Err(self.error("ill-formed tag handle (first argument) of the TAG directive"));
        }
        if self.tag_map.contains_key(handle) {
            return Err(self.error(&format!(
                "there is a previously declared suffix for \"{handle}\" tag handle"
            )));
        }
        let units: Vec<u16> = prefix.encode_utf16().collect();
        if !is_tag_uri(&units) {
            return Err(self.error("ill-formed tag prefix (second argument) of the TAG directive"));
        }
        let decoded = decode_uri_component(&units)
            .ok_or_else(|| self.error(&format!("tag prefix is malformed: {prefix}")))?;
        self.tag_map.insert(handle.clone(), decoded);
        Ok(())
    }

    /// `composeNode`: one node, with its properties, at the indentation the context allows.
    fn compose_node(
        &mut self,
        parent_indent: isize,
        context: Context,
        allow_to_seek: bool,
        allow_compact: bool,
    ) -> Res<bool> {
        if self.depth >= MAX_DEPTH {
            return Err(self.error("nesting exceeded maxDepth (100)"));
        }
        self.depth += 1;
        self.tag = None;
        self.anchor = None;
        self.kind = None;
        self.result = Node::Null;

        let allow_block_styles = matches!(context, Context::BlockOut | Context::BlockIn);
        let allow_block_scalars = allow_block_styles;
        let mut allow_block_collections = allow_block_styles;
        let mut at_new_line = false;
        let mut indent_status = 1;
        let mut has_content = false;
        let mut property_start: Option<Snapshot> = None;

        if allow_to_seek && self.skip_separation_space(true) > 0 {
            at_new_line = true;
            indent_status = indent_relation(self.line_indent, parent_indent);
        }

        if indent_status == 1 {
            loop {
                let ch = self.ch();
                let property_state = self.snapshot();
                // A duplicate property after a line break is the first key of a nested mapping.
                if at_new_line
                    && ((ch == unit(b'!') && self.tag.is_some())
                        || (ch == unit(b'&') && self.anchor.is_some()))
                {
                    break;
                }
                if !self.read_tag_property()? && !self.read_anchor_property()? {
                    break;
                }
                if property_start.is_none() {
                    property_start = Some(property_state);
                }
                if self.skip_separation_space(true) > 0 {
                    at_new_line = true;
                    allow_block_collections = allow_block_styles;
                    indent_status = indent_relation(self.line_indent, parent_indent);
                } else {
                    allow_block_collections = false;
                }
            }
        }

        if allow_block_collections {
            allow_block_collections = at_new_line || allow_compact;
        }

        if indent_status == 1 || context == Context::BlockOut {
            let flow_indent = if matches!(context, Context::FlowIn | Context::FlowOut) {
                parent_indent
            } else {
                parent_indent + 1
            };
            let block_indent = self.position as isize - self.line_start as isize;

            if indent_status == 1 {
                let block = allow_block_collections
                    && (self.read_block_sequence(block_indent)?
                        || self.read_block_mapping(block_indent, flow_indent)?);
                if block || self.read_flow_collection(flow_indent)? {
                    has_content = true;
                } else {
                    let ch = self.ch();
                    if let Some(start) = property_start.clone()
                        && allow_block_styles
                        && !allow_block_collections
                        && ch != unit(b'|')
                        && ch != unit(b'>')
                        && self.try_read_block_mapping_from_property(start, flow_indent)?
                    {
                        has_content = true;
                    } else if (allow_block_scalars && self.read_block_scalar(flow_indent)?)
                        || self.read_single_quoted()?
                        || self.read_double_quoted()?
                    {
                        has_content = true;
                    } else if self.read_alias()? {
                        has_content = true;
                        if self.tag.is_some() || self.anchor.is_some() {
                            return Err(self.error("alias node should not have any properties"));
                        }
                    } else if self.read_plain_scalar(flow_indent, context == Context::FlowIn)? {
                        has_content = true;
                        if self.tag.is_none() {
                            self.tag = Some("?".to_owned());
                        }
                    }
                    self.store_current_anchor();
                }
            } else if indent_status == 0 {
                // A block sequence may have the indentation of its parent.
                has_content = allow_block_collections && self.read_block_sequence(block_indent)?;
            }
        }

        match self.tag.clone() {
            None => self.store_current_anchor(),
            Some(tag) if tag == "?" => {
                // The non-specific tag of a plain scalar: only a scalar can take it.
                if !self.result.is_null() && self.kind != Some(Kind::Scalar) {
                    return Err(
                        self.error("unacceptable node kind for !<?> tag; it should be \"scalar\"")
                    );
                }
                if let Some((t, value)) = implicit_resolve(&self.result) {
                    self.result = value;
                    self.tag = Some(t.tag().to_owned());
                    self.store_current_anchor();
                }
            }
            // The non-specific tag `!` leaves the node as it is.
            Some(tag) if tag == "!" => {}
            Some(tag) => {
                let Some(t) = find_type(&tag, self.kind) else {
                    return Err(self.error(&format!("unknown tag !<{tag}>")));
                };
                if !self.result.is_null() && Some(t.kind()) != self.kind {
                    return Err(self.error(&format!(
                        "unacceptable node kind for !<{tag}> tag; it should be \"{:?}\"",
                        t.kind()
                    )));
                }
                let Some(value) = construct(t, &self.result) else {
                    return Err(
                        self.error(&format!("cannot resolve a node with !<{tag}> explicit tag"))
                    );
                };
                self.result = value;
                self.store_current_anchor();
            }
        }

        self.depth -= 1;
        Ok(self.tag.is_some() || self.anchor.is_some() || has_content)
    }

    /// `tryReadBlockMappingFromProperty`: reads the properties again as part of a block mapping's
    /// first key, when that is what they start.
    fn try_read_block_mapping_from_property(
        &mut self,
        start: Snapshot,
        flow_indent: isize,
    ) -> Res<bool> {
        let node_indent = start.position as isize - start.line_start as isize;
        let fallback = self.snapshot();
        self.begin_transaction();
        self.restore(start);
        self.tag = None;
        self.anchor = None;
        self.kind = None;
        self.result = Node::Null;
        if self.read_block_mapping(node_indent, flow_indent)? && self.kind == Some(Kind::Mapping) {
            self.commit_transaction();
            return Ok(true);
        }
        self.rollback_transaction();
        self.restore(fallback);
        Ok(false)
    }

    /// `readTagProperty`.
    fn read_tag_property(&mut self) -> Res<bool> {
        if self.ch() != unit(b'!') {
            return Ok(false);
        }
        if self.tag.is_some() {
            return Err(self.error("duplication of a tag property"));
        }
        self.position += 1;
        let mut ch = self.ch();
        let mut is_verbatim = false;
        let mut is_named = false;
        let mut tag_handle: Vec<u16> = vec![unit(b'!')];
        if ch == unit(b'<') {
            is_verbatim = true;
            self.position += 1;
            ch = self.ch();
        } else if ch == unit(b'!') {
            is_named = true;
            tag_handle = vec![unit(b'!'), unit(b'!')];
            self.position += 1;
            ch = self.ch();
        }

        let mut name_start = self.position;
        let tag_name: Vec<u16>;
        if is_verbatim {
            loop {
                self.position += 1;
                ch = self.ch();
                if ch == 0 || ch == unit(b'>') {
                    break;
                }
            }
            if self.position >= self.length {
                return Err(self.error("unexpected end of the stream within a verbatim tag"));
            }
            tag_name = self.input[name_start..self.position].to_vec();
            self.position += 1;
        } else {
            while ch != 0 && !is_ws_or_eol(ch) {
                if ch == unit(b'!') {
                    if is_named {
                        return Err(self.error("tag suffix cannot contain exclamation marks"));
                    }
                    tag_handle = self.input[name_start - 1..self.position + 1].to_vec();
                    if !is_tag_handle(&String::from_utf16_lossy(&tag_handle)) {
                        return Err(self.error("named tag handle cannot contain such characters"));
                    }
                    is_named = true;
                    name_start = self.position + 1;
                }
                self.position += 1;
                ch = self.ch();
            }
            tag_name = self.input[name_start..self.position].to_vec();
            if tag_name.iter().any(|&c| is_flow_indicator(c)) {
                return Err(self.error("tag suffix cannot contain flow indicator characters"));
            }
        }

        if !tag_name.is_empty() && !is_tag_uri(&tag_name) {
            return Err(self.error(&format!(
                "tag name cannot contain such characters: {}",
                String::from_utf16_lossy(&tag_name)
            )));
        }
        let name = decode_uri_component(&tag_name).ok_or_else(|| {
            self.error(&format!(
                "tag name is malformed: {}",
                String::from_utf16_lossy(&tag_name)
            ))
        })?;
        let handle = String::from_utf16_lossy(&tag_handle);
        self.tag = Some(if is_verbatim {
            name
        } else if let Some(prefix) = self.tag_map.get(&handle) {
            format!("{prefix}{name}")
        } else if handle == "!" {
            format!("!{name}")
        } else if handle == "!!" {
            format!("tag:yaml.org,2002:{name}")
        } else {
            return Err(self.error(&format!("undeclared tag handle \"{handle}\"")));
        });
        Ok(true)
    }

    /// `readAnchorProperty`.
    fn read_anchor_property(&mut self) -> Res<bool> {
        if self.ch() != unit(b'&') {
            return Ok(false);
        }
        if self.anchor.is_some() {
            return Err(self.error("duplication of an anchor property"));
        }
        self.position += 1;
        let start = self.position;
        let mut ch = self.ch();
        while ch != 0 && !is_ws_or_eol(ch) && !is_flow_indicator(ch) {
            self.position += 1;
            ch = self.ch();
        }
        if self.position == start {
            return Err(self.error("name of an anchor node must contain at least one character"));
        }
        self.anchor = Some(String::from_utf16_lossy(&self.input[start..self.position]));
        Ok(true)
    }

    /// `readAlias`: the node that the anchor names, which may be a collection still being read.
    fn read_alias(&mut self) -> Res<bool> {
        if self.ch() != unit(b'*') {
            return Ok(false);
        }
        self.position += 1;
        let start = self.position;
        let mut ch = self.ch();
        while ch != 0 && !is_ws_or_eol(ch) && !is_flow_indicator(ch) {
            self.position += 1;
            ch = self.ch();
        }
        if self.position == start {
            return Err(self.error("name of an alias node must contain at least one character"));
        }
        let name = String::from_utf16_lossy(&self.input[start..self.position]);
        let Some(node) = self.anchor_map.get(&name) else {
            return Err(self.error(&format!("unidentified alias \"{name}\"")));
        };
        self.result = node.clone();
        self.skip_separation_space(true);
        Ok(true)
    }

    /// `readBlockSequence`: `- ` entries at `node_indent`.
    fn read_block_sequence(&mut self, node_indent: isize) -> Res<bool> {
        let saved_tag = self.tag.clone();
        let saved_anchor = self.anchor.clone();
        let mut detected = false;

        // A leading tab means the line is not a block collection.
        if self.first_tab_in_line.is_some() {
            return Ok(false);
        }
        let items: Rc<RefCell<Vec<Node>>> = Rc::new(RefCell::new(Vec::new()));
        if let Some(name) = self.anchor.clone() {
            self.store_anchor(&name, Node::Sequence(Rc::clone(&items)));
        }

        let mut ch = self.ch();
        while ch != 0 {
            if let Some(tab) = self.first_tab_in_line {
                self.position = tab;
                return Err(self.error("tab characters must not be used in indentation"));
            }
            if ch != unit(b'-') || !is_ws_or_eol(self.at(self.position + 1)) {
                break;
            }
            detected = true;
            self.position += 1;

            if self.skip_separation_space(true) > 0 && self.line_indent <= node_indent {
                items.borrow_mut().push(Node::Null);
                ch = self.ch();
                continue;
            }

            let line = self.line;
            self.compose_node(node_indent, Context::BlockIn, false, true)?;
            let item = std::mem::replace(&mut self.result, Node::Null);
            items.borrow_mut().push(item);
            self.skip_separation_space(true);
            ch = self.ch();

            if (self.line == line || self.line_indent > node_indent) && ch != 0 {
                return Err(self.error("bad indentation of a sequence entry"));
            } else if self.line_indent < node_indent {
                break;
            }
        }

        if detected {
            self.tag = saved_tag;
            self.anchor = saved_anchor;
            self.kind = Some(Kind::Sequence);
            self.result = Node::Sequence(items);
        }
        Ok(detected)
    }

    /// `readBlockMapping`: the pairs of a block mapping at `node_indent`, with explicit (`?`) and
    /// implicit keys.
    fn read_block_mapping(&mut self, node_indent: isize, flow_indent: isize) -> Res<bool> {
        let saved_tag = self.tag.clone();
        let saved_anchor = self.anchor.clone();
        let mut at_explicit_key = false;
        let mut detected = false;
        // Set on every path that reaches the composing of a key or a value below.
        let mut allow_compact: bool;

        if self.first_tab_in_line.is_some() {
            return Ok(false);
        }
        let pairs: Rc<RefCell<Vec<(String, Node)>>> = Rc::new(RefCell::new(Vec::new()));
        if let Some(name) = self.anchor.clone() {
            self.store_anchor(&name, Node::Mapping(Rc::clone(&pairs)));
        }
        let mut key_node = Node::Null;
        let mut value_node = Node::Null;

        let mut ch = self.ch();
        while ch != 0 {
            if !at_explicit_key && let Some(tab) = self.first_tab_in_line {
                self.position = tab;
                return Err(self.error("tab characters must not be used in indentation"));
            }

            let following = self.at(self.position + 1);
            let line = self.line;

            if (ch == unit(b'?') || ch == unit(b':')) && is_ws_or_eol(following) {
                if ch == unit(b'?') {
                    if at_explicit_key {
                        self.store_pair(&pairs, &key_node, Node::Null)?;
                        key_node = Node::Null;
                        value_node = Node::Null;
                    }
                    detected = true;
                    at_explicit_key = true;
                    allow_compact = true;
                } else if at_explicit_key {
                    at_explicit_key = false;
                    allow_compact = true;
                } else {
                    return Err(self.error(
                        "incomplete explicit mapping pair; a key node is missed; or followed by a non-tabulated empty line",
                    ));
                }
                self.position += 1;
                ch = following;
            } else {
                if !self.compose_node(flow_indent, Context::FlowOut, false, true)? {
                    break;
                }
                if self.line == line {
                    ch = self.ch();
                    while is_whitespace(ch) {
                        self.position += 1;
                        ch = self.ch();
                    }
                    if ch == unit(b':') {
                        self.position += 1;
                        ch = self.ch();
                        if !is_ws_or_eol(ch) {
                            return Err(self.error(
                                "a whitespace character is expected after the key-value separator within a block mapping",
                            ));
                        }
                        if at_explicit_key {
                            self.store_pair(&pairs, &key_node, Node::Null)?;
                            value_node = Node::Null;
                        }
                        detected = true;
                        at_explicit_key = false;
                        allow_compact = false;
                        key_node = self.result.clone();
                    } else if detected {
                        return Err(
                            self.error("can not read an implicit mapping pair; a colon is missed")
                        );
                    } else {
                        // A plain node, not a mapping: its result is the node's.
                        self.tag = saved_tag;
                        self.anchor = saved_anchor;
                        return Ok(true);
                    }
                } else if detected {
                    return Err(self.error(
                        "can not read a block mapping entry; a multiline key may not be an implicit key",
                    ));
                } else {
                    self.tag = saved_tag;
                    self.anchor = saved_anchor;
                    return Ok(true);
                }
            }

            if self.line == line || self.line_indent > node_indent {
                if self.compose_node(node_indent, Context::BlockOut, true, allow_compact)? {
                    let node = std::mem::replace(&mut self.result, Node::Null);
                    if at_explicit_key {
                        key_node = node;
                    } else {
                        value_node = node;
                    }
                }
                if !at_explicit_key {
                    self.store_pair(&pairs, &key_node, value_node)?;
                    key_node = Node::Null;
                    value_node = Node::Null;
                }
                self.skip_separation_space(true);
                ch = self.ch();
            }

            if (self.line == line || self.line_indent > node_indent) && ch != 0 {
                return Err(self.error("bad indentation of a mapping entry"));
            } else if self.line_indent < node_indent {
                break;
            }
        }

        if at_explicit_key {
            self.store_pair(&pairs, &key_node, Node::Null)?;
        }
        if detected {
            self.tag = saved_tag;
            self.anchor = saved_anchor;
            self.kind = Some(Kind::Mapping);
            self.result = Node::Mapping(pairs);
        }
        Ok(detected)
    }

    /// `storeMappingPair`: the key as a string, and the pair if the key is new.
    fn store_pair(
        &self,
        pairs: &Rc<RefCell<Vec<(String, Node)>>>,
        key: &Node,
        value: Node,
    ) -> Res<()> {
        let name = key_string(key).map_err(|message| self.error(message))?;
        if pairs.borrow().iter().any(|(existing, _)| *existing == name) {
            return Err(self.error("duplicated mapping key"));
        }
        pairs.borrow_mut().push((name, value));
        Ok(())
    }

    /// `readFlowCollection`: `[...]` or `{...}`, over as many lines as it takes.
    fn read_flow_collection(&mut self, node_indent: isize) -> Res<bool> {
        let saved_tag = self.tag.clone();
        let saved_anchor = self.anchor.clone();
        let (terminator, is_mapping) = match self.ch() {
            c if c == unit(b'[') => (unit(b']'), false),
            c if c == unit(b'{') => (unit(b'}'), true),
            _ => return Ok(false),
        };
        let items: Rc<RefCell<Vec<Node>>> = Rc::new(RefCell::new(Vec::new()));
        let pairs: Rc<RefCell<Vec<(String, Node)>>> = Rc::new(RefCell::new(Vec::new()));
        if let Some(name) = self.anchor.clone() {
            let node = if is_mapping {
                Node::Mapping(Rc::clone(&pairs))
            } else {
                Node::Sequence(Rc::clone(&items))
            };
            self.store_anchor(&name, node);
        }

        self.position += 1;
        let mut ch = self.ch();
        let mut read_next = true;

        while ch != 0 {
            self.skip_separation_space(true);
            ch = self.ch();

            if ch == terminator {
                self.position += 1;
                self.tag = saved_tag;
                self.anchor = saved_anchor;
                self.kind = Some(if is_mapping {
                    Kind::Mapping
                } else {
                    Kind::Sequence
                });
                self.result = if is_mapping {
                    Node::Mapping(pairs)
                } else {
                    Node::Sequence(items)
                };
                return Ok(true);
            } else if !read_next {
                return Err(self.error("missed comma between flow collection entries"));
            } else if ch == unit(b',') {
                return Err(self.error("expected the node content, but found ','"));
            }

            let mut is_pair = false;
            let mut is_explicit_pair = false;
            if ch == unit(b'?') && is_ws_or_eol(self.at(self.position + 1)) {
                is_pair = true;
                is_explicit_pair = true;
                self.position += 1;
                self.skip_separation_space(true);
            }

            let line = self.line;
            self.compose_node(node_indent, Context::FlowIn, false, true)?;
            let key_node = std::mem::replace(&mut self.result, Node::Null);
            self.skip_separation_space(true);
            ch = self.ch();

            let mut value_node = Node::Null;
            if (is_explicit_pair || self.line == line) && ch == unit(b':') {
                is_pair = true;
                self.position += 1;
                self.skip_separation_space(true);
                self.compose_node(node_indent, Context::FlowIn, false, true)?;
                value_node = std::mem::replace(&mut self.result, Node::Null);
            }

            if is_mapping {
                self.store_pair(&pairs, &key_node, value_node)?;
            } else if is_pair {
                // A pair in a sequence is a mapping of its own.
                let single: Rc<RefCell<Vec<(String, Node)>>> = Rc::new(RefCell::new(Vec::new()));
                self.store_pair(&single, &key_node, value_node)?;
                items.borrow_mut().push(Node::Mapping(single));
            } else {
                items.borrow_mut().push(key_node);
            }

            self.skip_separation_space(true);
            ch = self.ch();
            if ch == unit(b',') {
                read_next = true;
                self.position += 1;
                ch = self.ch();
            } else {
                read_next = false;
            }
        }

        Err(self.error("unexpected end of the stream within a flow collection"))
    }

    /// `readBlockScalar`: `|` (literal) or `>` (folded) with its header.
    fn read_block_scalar(&mut self, node_indent: isize) -> Res<bool> {
        let folding = match self.ch() {
            c if c == unit(b'|') => false,
            c if c == unit(b'>') => true,
            _ => return Ok(false),
        };
        self.kind = Some(Kind::Scalar);
        let mut out: Vec<u16> = Vec::new();
        let mut chomping = Chomping::Clip;
        let mut did_read = false;
        let mut detected_indent = false;
        let mut text_indent = node_indent;
        let mut empty_lines: usize = 0;
        let mut at_more_indented = false;

        let mut ch = self.ch();
        while ch != 0 {
            self.position += 1;
            ch = self.ch();
            if ch == unit(b'+') || ch == unit(b'-') {
                if chomping != Chomping::Clip {
                    return Err(self.error("repeat of a chomping mode identifier"));
                }
                chomping = if ch == unit(b'+') {
                    Chomping::Keep
                } else {
                    Chomping::Strip
                };
            } else if (unit(b'0')..=unit(b'9')).contains(&ch) {
                let digit = isize::from(u8::try_from(ch - unit(b'0')).unwrap_or(0));
                if digit == 0 {
                    return Err(self.error(
                        "bad explicit indentation width of a block scalar; it cannot be less than one",
                    ));
                }
                if detected_indent {
                    return Err(self.error("repeat of an indentation width identifier"));
                }
                text_indent = node_indent + digit - 1;
                detected_indent = true;
            } else {
                break;
            }
        }

        if is_whitespace(ch) {
            loop {
                self.position += 1;
                ch = self.ch();
                if !is_whitespace(ch) {
                    break;
                }
            }
            if ch == unit(b'#') {
                loop {
                    self.position += 1;
                    ch = self.ch();
                    if is_eol(ch) || ch == 0 {
                        break;
                    }
                }
            }
        }

        while ch != 0 {
            self.read_line_break()?;
            self.line_indent = 0;
            ch = self.ch();

            while (!detected_indent || self.line_indent < text_indent) && ch == SPACE {
                self.line_indent += 1;
                self.position += 1;
                ch = self.ch();
            }

            if !detected_indent && self.line_indent > text_indent {
                text_indent = self.line_indent;
            }

            if is_eol(ch) {
                empty_lines += 1;
                continue;
            }

            if !detected_indent && text_indent == 0 {
                return Err(self.error("missing indentation for block scalar"));
            }

            // The end of the scalar: the chomping decides the trailing line breaks.
            if self.line_indent < text_indent {
                match chomping {
                    Chomping::Keep => {
                        push_line_breaks(
                            &mut out,
                            if did_read {
                                1 + empty_lines
                            } else {
                                empty_lines
                            },
                        );
                    }
                    Chomping::Clip => {
                        if did_read {
                            out.push(LF);
                        }
                    }
                    Chomping::Strip => {}
                }
                break;
            }

            if folding {
                if is_whitespace(ch) {
                    // A more-indented line is not folded.
                    at_more_indented = true;
                    push_line_breaks(
                        &mut out,
                        if did_read {
                            1 + empty_lines
                        } else {
                            empty_lines
                        },
                    );
                } else if at_more_indented {
                    at_more_indented = false;
                    push_line_breaks(&mut out, empty_lines + 1);
                } else if empty_lines == 0 {
                    if did_read {
                        out.push(SPACE);
                    }
                } else {
                    push_line_breaks(&mut out, empty_lines);
                }
            } else {
                push_line_breaks(
                    &mut out,
                    if did_read {
                        1 + empty_lines
                    } else {
                        empty_lines
                    },
                );
            }

            did_read = true;
            detected_indent = true;
            empty_lines = 0;
            let start = self.position;
            while !is_eol(ch) && ch != 0 {
                self.position += 1;
                ch = self.ch();
            }
            self.capture(&mut out, start, self.position, false)?;
        }

        self.result = Node::String(self.text(&out));
        Ok(true)
    }

    /// `readSingleQuotedScalar`.
    fn read_single_quoted(&mut self) -> Res<bool> {
        if self.ch() != unit(b'\'') {
            return Ok(false);
        }
        self.kind = Some(Kind::Scalar);
        self.position += 1;
        let mut out: Vec<u16> = Vec::new();
        let mut capture_start = self.position;
        let mut capture_end = self.position;

        loop {
            let mut ch = self.ch();
            if ch == 0 {
                break;
            }
            if ch == unit(b'\'') {
                self.capture(&mut out, capture_start, self.position, true)?;
                self.position += 1;
                ch = self.ch();
                if ch == unit(b'\'') {
                    capture_start = self.position;
                    self.position += 1;
                    capture_end = self.position;
                } else {
                    self.result = Node::String(self.text(&out));
                    return Ok(true);
                }
            } else if is_eol(ch) {
                self.capture(&mut out, capture_start, capture_end, true)?;
                let breaks = self.skip_separation_space(false);
                write_folded_lines(&mut out, breaks);
                capture_start = self.position;
                capture_end = self.position;
            } else if self.position == self.line_start && self.test_document_separator() {
                return Err(
                    self.error("unexpected end of the document within a single quoted scalar")
                );
            } else {
                self.position += 1;
                if !is_whitespace(ch) {
                    capture_end = self.position;
                }
            }
        }
        Err(self.error("unexpected end of the stream within a single quoted scalar"))
    }

    /// `readDoubleQuotedScalar`, with its escapes.
    fn read_double_quoted(&mut self) -> Res<bool> {
        if self.ch() != unit(b'"') {
            return Ok(false);
        }
        self.kind = Some(Kind::Scalar);
        self.position += 1;
        let mut out: Vec<u16> = Vec::new();
        let mut capture_start = self.position;
        let mut capture_end = self.position;

        loop {
            let mut ch = self.ch();
            if ch == 0 {
                break;
            }
            if ch == unit(b'"') {
                self.capture(&mut out, capture_start, self.position, true)?;
                self.position += 1;
                self.result = Node::String(self.text(&out));
                return Ok(true);
            } else if ch == unit(b'\\') {
                self.capture(&mut out, capture_start, self.position, true)?;
                self.position += 1;
                ch = self.ch();
                if is_eol(ch) {
                    // An escaped line break joins the lines, with no fold.
                    self.skip_separation_space(false);
                } else if let Some(escaped) = simple_escape(ch) {
                    out.push(escaped);
                    self.position += 1;
                } else if let Some(digits) = escaped_hex_len(ch) {
                    let mut value: i32 = 0;
                    for _ in 0..digits {
                        self.position += 1;
                        let digit = from_hex(self.ch())
                            .ok_or_else(|| self.error("expected hexadecimal character"))?;
                        value = value.wrapping_shl(4).wrapping_add(digit);
                    }
                    out.extend(code_units(value));
                    self.position += 1;
                } else {
                    return Err(self.error("unknown escape sequence"));
                }
                capture_start = self.position;
                capture_end = self.position;
            } else if is_eol(ch) {
                self.capture(&mut out, capture_start, capture_end, true)?;
                let breaks = self.skip_separation_space(false);
                write_folded_lines(&mut out, breaks);
                capture_start = self.position;
                capture_end = self.position;
            } else if self.position == self.line_start && self.test_document_separator() {
                return Err(
                    self.error("unexpected end of the document within a double quoted scalar")
                );
            } else {
                self.position += 1;
                if !is_whitespace(ch) {
                    capture_end = self.position;
                }
            }
        }
        Err(self.error("unexpected end of the stream within a double quoted scalar"))
    }

    /// `readPlainScalar`: a scalar that may run over several lines, as far as its indentation goes.
    fn read_plain_scalar(&mut self, node_indent: isize, within_flow: bool) -> Res<bool> {
        let saved_kind = self.kind;
        let saved_result = self.result.clone();
        let mut ch = self.ch();

        if is_ws_or_eol(ch)
            || is_flow_indicator(ch)
            || matches!(
                ch,
                0x23 | 0x26 | 0x2A | 0x21 | 0x7C | 0x3E | 0x27 | 0x22 | 0x25 | 0x40 | 0x60
            )
        {
            return Ok(false);
        }
        if ch == unit(b'?') || ch == unit(b'-') {
            let following = self.at(self.position + 1);
            if is_ws_or_eol(following) || (within_flow && is_flow_indicator(following)) {
                return Ok(false);
            }
        }

        self.kind = Some(Kind::Scalar);
        let mut out: Vec<u16> = Vec::new();
        let mut capture_start = self.position;
        let mut capture_end = self.position;
        let mut has_pending_content = false;
        // The line of the last line break that was skipped, for the folding of the lines.
        let mut pending_line = self.line;

        while ch != 0 {
            if ch == unit(b':') {
                let following = self.at(self.position + 1);
                if is_ws_or_eol(following) || (within_flow && is_flow_indicator(following)) {
                    break;
                }
            } else if ch == unit(b'#') {
                let preceding = if self.position == 0 {
                    0
                } else {
                    self.input[self.position - 1]
                };
                if is_ws_or_eol(preceding) {
                    break;
                }
            } else if (self.position == self.line_start && self.test_document_separator())
                || (within_flow && is_flow_indicator(ch))
            {
                break;
            } else if is_eol(ch) {
                let (line, line_start, line_indent) =
                    (self.line, self.line_start, self.line_indent);
                pending_line = line;
                self.skip_separation_space(false);

                if self.line_indent >= node_indent {
                    has_pending_content = true;
                    ch = self.ch();
                    continue;
                }
                // The next line belongs to another node: the scalar ends where it did.
                self.position = capture_end;
                self.line = line;
                self.line_start = line_start;
                self.line_indent = line_indent;
                break;
            }

            if has_pending_content {
                self.capture(&mut out, capture_start, capture_end, false)?;
                write_folded_lines(&mut out, self.line - pending_line);
                capture_start = self.position;
                capture_end = self.position;
                has_pending_content = false;
            }

            if !is_whitespace(ch) {
                capture_end = self.position + 1;
            }
            self.position += 1;
            ch = self.ch();
        }

        self.capture(&mut out, capture_start, capture_end, false)?;
        if !out.is_empty() {
            self.result = Node::String(self.text(&out));
            return Ok(true);
        }
        self.kind = saved_kind;
        self.result = saved_result;
        Ok(false)
    }
}

/// `composeNode`'s indentation of a node against its parent: 1 for deeper, 0 for equal, -1 for
/// shallower.
fn indent_relation(line_indent: isize, parent_indent: isize) -> i8 {
    match line_indent.cmp(&parent_indent) {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Less => -1,
    }
}

/// `writeFoldedLines`: one line break folds to a space, more are kept but the first.
fn write_folded_lines(out: &mut Vec<u16>, breaks: usize) {
    match breaks {
        0 => {}
        1 => out.push(SPACE),
        many => push_line_breaks(out, many - 1),
    }
}

fn push_line_breaks(out: &mut Vec<u16>, count: usize) {
    out.extend(std::iter::repeat_n(LF, count));
}

/// `PATTERN_NON_PRINTABLE`, with a surrogate pair as one character.
fn has_non_printable(units: &[u16]) -> bool {
    for (i, &c) in units.iter().enumerate() {
        if matches!(
            c,
            0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F..=0x84 | 0x86..=0x9F | 0xFFFE | 0xFFFF
        ) {
            return true;
        }
        if (0xD800..=0xDBFF).contains(&c) {
            if !units
                .get(i + 1)
                .is_some_and(|next| (0xDC00..=0xDFFF).contains(next))
            {
                return true;
            }
        } else if (0xDC00..=0xDFFF).contains(&c)
            && (i == 0 || !(0xD800..=0xDBFF).contains(&units[i - 1]))
        {
            return true;
        }
    }
    false
}

/// `simpleEscapeSequence`: the escapes that take one character.
fn simple_escape(c: u16) -> Option<u16> {
    Some(match c {
        0x30 => 0x00,
        0x61 => 0x07,
        0x62 => 0x08,
        0x74 | 0x09 => 0x09,
        0x6E => 0x0A,
        0x76 => 0x0B,
        0x66 => 0x0C,
        0x72 => 0x0D,
        0x65 => 0x1B,
        0x20 => 0x20,
        0x22 => 0x22,
        0x2F => 0x2F,
        0x5C => 0x5C,
        0x4E => 0x85,
        0x5F => 0xA0,
        0x4C => 0x2028,
        0x50 => 0x2029,
        _ => return None,
    })
}

/// `escapedHexLen`: the digits of `\x`, `\u` and `\U`.
fn escaped_hex_len(c: u16) -> Option<usize> {
    match c {
        0x78 => Some(2),
        0x75 => Some(4),
        0x55 => Some(8),
        _ => None,
    }
}

fn from_hex(c: u16) -> Option<i32> {
    match c {
        0x30..=0x39 => Some(i32::from(c - 0x30)),
        0x41..=0x46 | 0x61..=0x66 => Some(i32::from((c | 0x20) - 0x61 + 10)),
        _ => None,
    }
}

/// `charFromCodepoint` and `String.fromCharCode`: the code units of an escaped value. A value above
/// the BMP is a surrogate pair computed as `js-yaml` computes it, with each unit taken modulo 2^16.
fn code_units(value: i32) -> Vec<u16> {
    if value <= 0xFFFF {
        return vec![value.rem_euclid(0x1_0000) as u16];
    }
    let offset = i64::from(value) - 0x1_0000;
    vec![
        ((offset >> 10) + 0xD800).rem_euclid(0x1_0000) as u16,
        ((offset & 0x3FF) + 0xDC00) as u16,
    ]
}

/// The string a mapping key becomes, as `storeMappingPair` makes it with `String(keyNode)`: an
/// array joins its items, and an object is `[object Object]`.
fn key_string(key: &Node) -> Result<String, &'static str> {
    Ok(match key {
        Node::Null => "null".to_owned(),
        Node::Bool(flag) => flag.to_string(),
        Node::Number(number) => js_number_string(*number),
        Node::String(text) => text.clone(),
        Node::Mapping(_) => "[object Object]".to_owned(),
        Node::Sequence(items) => {
            let mut parts = Vec::new();
            for item in items.borrow().iter() {
                parts.push(match item {
                    Node::Sequence(_) => {
                        return Err("nested arrays are not supported inside keys");
                    }
                    Node::Null => String::new(),
                    Node::Mapping(_) => "[object Object]".to_owned(),
                    other => key_string(other)?,
                });
            }
            parts.join(",")
        }
    })
}

/// The version of a `%YAML` directive: two runs of ASCII digits around a dot.
fn split_version(text: &str) -> Option<(&str, &str)> {
    let (major, minor) = text.split_once('.')?;
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    (digits(major) && digits(minor)).then_some((major, minor))
}

/// `PATTERN_TAG_HANDLE`: `!`, `!!`, or `!` around a word of letters, digits and dashes.
fn is_tag_handle(text: &str) -> bool {
    match text {
        "!" | "!!" => true,
        _ => {
            let Some(inner) = text
                .strip_prefix('!')
                .and_then(|rest| rest.strip_suffix('!'))
            else {
                return false;
            };
            !inner.is_empty()
                && inner
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        }
    }
}

/// `PATTERN_TAG_URI`: a first character that is not a flow indicator, then percent escapes and the
/// characters of a URI.
fn is_tag_uri(units: &[u16]) -> bool {
    let Some((&first, rest)) = units.split_first() else {
        return false;
    };
    if is_flow_indicator(first) {
        return false;
    }
    let mut i = 0;
    while i < rest.len() {
        if rest[i] == unit(b'%') {
            if !(rest.get(i + 1).is_some_and(|&c| hex_digit(c).is_some())
                && rest.get(i + 2).is_some_and(|&c| hex_digit(c).is_some()))
            {
                return false;
            }
            i += 3;
        } else if rest[i] < 0x80 && is_uri_char(rest[i] as u8) {
            i += 1;
        } else {
            return false;
        }
    }
    true
}

fn is_uri_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"-#;/?:@&=+$,_.!~*'()[]".contains(&b)
}

fn hex_digit(c: u16) -> Option<u8> {
    u8::try_from(c)
        .ok()
        .and_then(|b| (b as char).to_digit(16))
        .and_then(|digit| u8::try_from(digit).ok())
}

/// `decodeURIComponent`: the percent escapes as UTF-8 bytes, or `None` where JavaScript throws.
fn decode_uri_component(units: &[u16]) -> Option<String> {
    let mut bytes: Vec<u8> = Vec::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        if units[i] == unit(b'%') {
            let high = hex_digit(*units.get(i + 1)?)?;
            let low = hex_digit(*units.get(i + 2)?)?;
            bytes.push(high * 16 + low);
            i += 3;
        } else {
            let width = if (0xD800..=0xDBFF).contains(&units[i]) {
                2
            } else {
                1
            };
            let text = String::from_utf16(units.get(i..i + width)?).ok()?;
            bytes.extend_from_slice(text.as_bytes());
            i += width;
        }
    }
    String::from_utf8(bytes).ok()
}

/// The schema's null spellings.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> Yaml {
        load(text).expect("loads").expect("has a document")
    }

    fn pair(key: &str, value: Yaml) -> Yaml {
        Yaml::Mapping(vec![(key.into(), value)])
    }

    #[test]
    fn plain_scalars_follow_the_json_schema() {
        assert_eq!(value("a: ~"), pair("a", Yaml::Null));
        assert_eq!(value("a: TRUE"), pair("a", Yaml::Bool(true)));
        assert_eq!(value("a: tRUE"), pair("a", Yaml::String("tRUE".into())));
        assert_eq!(value("a: 0x1F"), pair("a", Yaml::Number(31.0)));
        assert_eq!(value("a: -0o17"), pair("a", Yaml::Number(-15.0)));
        assert_eq!(value("a: 007"), pair("a", Yaml::Number(7.0)));
        assert_eq!(value("a: .5"), pair("a", Yaml::Number(0.5)));
        assert_eq!(value("a: 1e5"), pair("a", Yaml::Number(100000.0)));
        assert_eq!(value("a: 1_000"), pair("a", Yaml::String("1_000".into())));
        assert_eq!(value("a: -.5"), pair("a", Yaml::String("-.5".into())));
        assert_eq!(value("a: 1e400"), pair("a", Yaml::String("1e400".into())));
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
        assert_eq!(value("a: 'true'"), pair("a", Yaml::String("true".into())));
        assert_eq!(
            value("a: |\n  x\n  y\n"),
            pair("a", Yaml::String("x\ny\n".into()))
        );
        assert_eq!(
            value("a: >\n  x\n  y\n"),
            pair("a", Yaml::String("x y\n".into()))
        );
    }

    #[test]
    fn quoted_lines_may_have_any_indentation() {
        let inner = |text: &str| match value(text) {
            Yaml::Mapping(pairs) => pairs[0].1.clone(),
            other => other,
        };
        let inner_pair = |value: &str| pair("b", Yaml::String(value.into()));
        assert_eq!(inner("a:\n  b: \"x\ny\""), inner_pair("x y"));
        assert_eq!(inner("a:\n  b: \"x\n\ty\""), inner_pair("x y"));
        assert_eq!(inner("a:\n  b: 'x\n y ''z'''"), inner_pair("x y 'z'"));
        assert_eq!(inner("a:\n  b: \"x\n   \n y\n  \""), inner_pair("x\ny "));
        assert_eq!(inner("a:\n  b: \"x \\\ny\""), inner_pair("x y"));
        assert!(load("a:\n  b: \"x\n---\ny\"").is_err());
        assert!(load("a:\n  b: \"x\ny").is_err());
    }

    #[test]
    fn keys_become_strings() {
        assert_eq!(value("1.0: a"), pair("1", Yaml::String("a".into())));
        assert_eq!(value("null: a"), pair("null", Yaml::String("a".into())));
        assert_eq!(value("[a, b]: c"), pair("a,b", Yaml::String("c".into())));
        assert_eq!(value("[~]: c"), pair("", Yaml::String("c".into())));
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
        assert_eq!(load("  \n").expect("loads"), None);
        assert_eq!(load("# only a comment").expect("loads"), Some(Yaml::Null));
    }

    #[test]
    fn core_tags_convert_and_others_are_errors() {
        assert_eq!(value("a: !!str 1"), pair("a", Yaml::String("1".into())));
        assert_eq!(value("a: !!int \"1\""), pair("a", Yaml::Number(1.0)));
        assert_eq!(value("a: !!map"), pair("a", Yaml::Mapping(vec![])));
        assert_eq!(value("a: !!seq"), pair("a", Yaml::Sequence(vec![])));
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
    fn a_collection_that_contains_itself_has_no_tree() {
        assert!(load("a: &x [*x]").is_err());
        // The alias is not stored in the collection, so the mapping is a tree.
        assert_eq!(
            value("&x\n  *x : 1"),
            pair("[object Object]", Yaml::Number(1.0))
        );
    }

    #[test]
    fn a_colon_without_a_key_is_an_error() {
        assert!(load(":").is_err());
        assert!(load(": >-\n  a").is_err());
        assert!(load("a: 1\n: b").is_err());
        assert_eq!(
            value("? \n: a"),
            Yaml::Mapping(vec![("null".into(), Yaml::String("a".into()))])
        );
    }

    #[test]
    fn a_tab_separates_a_key_from_its_value() {
        assert_eq!(value("a:\tb"), pair("a", Yaml::String("b".into())));
    }

    #[test]
    fn flow_collections_may_continue_at_any_indentation() {
        assert_eq!(
            value("a: [1,\n2]"),
            pair(
                "a",
                Yaml::Sequence(vec![Yaml::Number(1.0), Yaml::Number(2.0)])
            )
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
