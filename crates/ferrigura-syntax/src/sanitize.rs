//! The DOM step of the reference's `sanitizeText` (`packages/mermaid/src/diagrams/common/common.ts`):
//! DOMPurify 3.4.12, which the package pins. On the strict path, which a diagram takes by default,
//! the reference runs it twice: `removeScript` with DOMPurify's defaults, then again on that output
//! with `{ FORBID_TAGS: ['style'] }`, so a `style` is kept by the first pass and removed by the
//! second. The flowchart DB runs it on the text of a node, an edge and a subgraph title
//! (`flowDb.ts`), and the node label runs it again when it is drawn (`shapes/util.ts`).
//!
//! The text is HTML to DOMPurify, so the output is HTML too: `<` that does not start a tag becomes
//! `&lt;`, `&` becomes `&amp;`, `>` becomes `&gt;`, an entity is written as its character, an
//! element that is not allowed is removed with its content (or its content is kept, for most),
//! and attributes are written in double quotes. A plain-text label then shows that HTML as text.
//! Text without `<` is returned as it is, which is DOMPurify's own exit for such input.
//!
//! The reproduced subset, with the DOMPurify behaviour it follows:
//!
//! - Tokens: start and end tags, attributes (quoted, unquoted or valueless; names lower-cased; the
//!   first of two equal names wins), comments, bogus comments and the doctype, which are dropped.
//!   Raw text for `script`, `style`, `xmp`, `iframe`, `noembed` and `noframes`, RCDATA for `title`
//!   and `textarea`, and `plaintext` take their content as the HTML parser does. `</br>` is `<br>`.
//!   `<image>` in HTML is `<img>`. CR and CR LF are LF.
//! - Tree: void elements close at once, and an end tag closes the nearest open element of its name.
//!   A `<head>` element before any body text (`<title>`, `<meta>`, ...) is dropped with its
//!   content, and so is `<noscript>` there; the whitespace that starts the text is kept, as
//!   DOMPurify keeps it.
//! - Elements: the namespace of the HTML, SVG and MathML elements is the parser's (an `<svg>` opens
//!   the SVG namespace; there are no integration points). An element outside DOMPurify's allowed
//!   list, or a `style` in the second pass, is removed and its children are kept, unless DOMPurify
//!   forbids the content (`FORBID_CONTENTS`, such as `script` and `title`); an element in the wrong
//!   namespace is removed with its content (`_checkValidNamespace`).
//! - Attributes: a name must be in DOMPurify's allowed list, or `data-*` or `aria-*`; a value that
//!   is not one of the URI-safe attributes must match `IS_ALLOWED_URI` after the whitespace is
//!   removed (a `data:` URI is allowed for `img`, `audio`, `video`, `source`, `image` and `track`),
//!   and a value that is not empty must do so. The value is trimmed unless the name is `value`. Values
//!   with `-->` or `</style` and the like are removed, as is `for` outside `label` and `output`.
//! - The anchor hooks (`setupDompurifyHooks`): on an HTML `a`, a `target` is copied to
//!   `data-temp-href-target` before DOMPurify checks the attributes, and read back after it. The
//!   `target` is then appended with the value that survived, the temporary attribute is removed,
//!   and `rel="noopener"` is set when the value is `_blank`, in place if `rel` was kept and
//!   appended otherwise. A `target` whose value DOMPurify refuses is lost, with no `rel`. An `a`
//!   in `svg` or `math` has the tag name `a`, not `A`, so the hooks do not reach it. A
//!   `data-temp-href-target` written in the source is read back as a `target` when the element has
//!   none, so `<a data-temp-href-target="_blank">` gets `rel="noopener"` too.
//! - Serialisation: `&`, no-break space and `<`, `>` in text; `&`, no-break space and `"` in
//!   attribute values; a HTML void element has no end tag; the content of a raw-text element is
//!   written as it is.
//!
//! Not reproduced, and where the result can differ: the element-level implied end tags (`p`, `li`,
//! table parts), the adoption agency for misnested formatting elements, foster parenting, the
//! breakout of `<b>` and other HTML tags out of `<svg>`, the case adjustment of SVG and MathML
//! element and attribute names, the DOM-clobbering check of `id` and `name`, the mXSS checks,
//! `dompurifyConfig`, and the `loose` and `htmlLabels: false` paths of `sanitizeMore`, which run one
//! pass and reach the anchor hooks only when an earlier strict call in the process has installed
//! them. The named character references are the full HTML5 table (`entities.rs`). The corpora in
//! the tests hold the reference's output for the cases the subset covers: `DOMPURIFY_CORPUS` for
//! the cases without an anchor, `HOOKED_CORPUS` for the anchors.

use crate::entities;
use crate::text::js_trim;

/// The namespace of an element, as far as labels can reach it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Namespace {
    Html,
    Svg,
    MathMl,
}

/// A node of the parsed text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Text(String),
    Element(Element),
}

/// An element of the parsed text: its lower-case name, namespace, attributes in source order, and
/// children.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Element {
    name: String,
    namespace: Namespace,
    attributes: Vec<(String, String)>,
    children: Vec<Node>,
}

/// The element a node is put into, for the namespace check.
#[derive(Debug, Clone, Copy)]
struct Parent<'a> {
    namespace: Namespace,
    name: &'a str,
}

/// The `body` that DOMPurify parses into, whose children are the output.
const BODY: Parent<'static> = Parent {
    namespace: Namespace::Html,
    name: "body",
};

/// `sanitizeText` on the strict path of `sanitizeMore`, on the subset described in the module
/// documentation: DOMPurify with its defaults (`removeScript`), then DOMPurify with
/// `{ FORBID_TAGS: ['style'] }` on that output. Text without `<` is returned as it is.
#[must_use]
pub fn sanitize_text(text: &str) -> String {
    dompurify(&dompurify(text, false), true)
}

/// One DOMPurify pass over `text` on the subset: `forbid_style` is the `FORBID_TAGS: ['style']` of
/// the second pass. The pass runs the anchor hooks, as the reference installs them before its first
/// pass. Text without `<` is returned as it is, which is DOMPurify's own exit.
fn dompurify(text: &str, forbid_style: bool) -> String {
    if !text.contains('<') {
        return text.to_owned();
    }
    let normalized: Vec<char> = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .collect();
    let body = parse(&normalized);
    let kept = sanitize_children(body, BODY, forbid_style);
    let mut out = String::with_capacity(text.len());
    serialize(&kept, false, &mut out);
    out
}

/// `sanitizeText` for a configuration without `securityLevel`, as the gitGraph section is: the
/// reference takes the escape branch of `sanitizeMore`, which turns `<br>` into a placeholder,
/// escapes `<`, `>` and `=`, puts the breaks back as `<br/>`, and then runs DOMPurify. The
/// result is therefore not the DOMPurify output alone: `a=b` gives `a&equals;b`. The empty text
/// is returned as it is. The escape branch runs one pass, with `{ FORBID_TAGS: ['style'] }`.
#[must_use]
pub fn sanitize_section(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let placeholder = break_to_placeholder(text);
    let escaped = placeholder
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('=', "&equals;");
    dompurify(&escaped.replace("#br#", "<br/>"), true)
}

/// `replace(/<\/?br\s*\/?>/gi, '#br#')`: a line break tag, in any case and with JavaScript blanks,
/// becomes the placeholder `#br#`.
fn break_to_placeholder(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if let Some(end) = line_break_tag_end(&chars, i) {
            out.push_str("#br#");
            i = end;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// The index after a `<br>` tag that starts at `start`, if one does.
fn line_break_tag_end(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start) != Some(&'<') {
        return None;
    }
    let mut i = start + 1;
    if chars.get(i) == Some(&'/') {
        i += 1;
    }
    let is_br = chars.get(i).is_some_and(|c| c.eq_ignore_ascii_case(&'b'))
        && chars
            .get(i + 1)
            .is_some_and(|c| c.eq_ignore_ascii_case(&'r'));
    if !is_br {
        return None;
    }
    i += 2;
    while chars.get(i).is_some_and(|&c| crate::text::is_js_space(c)) {
        i += 1;
    }
    if chars.get(i) == Some(&'/') {
        i += 1;
    }
    (chars.get(i) == Some(&'>')).then_some(i + 1)
}

/// The elements that void in HTML: they have no content and no end tag.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "img", "input",
    "keygen", "link", "meta", "param", "source", "track", "wbr",
];

/// The elements whose content is raw text, which the parser does not read as markup.
const RAW_TEXT_ELEMENTS: &[&str] = &["iframe", "noembed", "noframes", "script", "style", "xmp"];

/// The elements whose content is text with character references but no markup.
const RCDATA_ELEMENTS: &[&str] = &["textarea", "title"];

/// The elements whose content is raw text in the serialisation, as HTML writes a raw-text parent.
const RAW_TEXT_PARENTS: &[&str] = &[
    "iframe",
    "noembed",
    "noframes",
    "noscript",
    "plaintext",
    "script",
    "style",
    "xmp",
];

/// The elements the parser puts in `head` when they come before any body text, and which the
/// output then drops: the void ones alone, and the others with their content (read as raw text,
/// which is an approximation for `template`, whose content is markup).
const HEAD_VOID: &[&str] = &["base", "basefont", "bgsound", "link", "meta"];
const HEAD_RAW: &[&str] = &["noframes", "script", "style", "template", "title"];

/// The HTML parser's tree for `chars`: the body's children, before any sanitising.
fn parse(chars: &[char]) -> Vec<Node> {
    // The whitespace that starts the text is kept whatever it precedes, as DOMPurify puts it back
    // in front of the body (`leadingWhitespace` in `_initDocument`).
    let lead = chars
        .iter()
        .take_while(|c| matches!(c, '\n' | '\t' | ' '))
        .count();
    let mut parser = Parser {
        chars: &chars[lead..],
        position: 0,
        open: Vec::new(),
        root: Vec::new(),
        in_head: true,
    };
    if lead > 0 {
        parser.root.push(Node::Text(chars[..lead].iter().collect()));
    }
    while parser.position < parser.chars.len() {
        parser.step();
    }
    parser.finish()
}

/// The state of the tree builder: the open elements, innermost last, and the children of the
/// body so far.
struct Parser<'a> {
    chars: &'a [char],
    position: usize,
    open: Vec<Element>,
    root: Vec<Node>,
    /// Whether the parser is still before the body: only head elements, whitespace and comments
    /// have come so far.
    in_head: bool,
}

/// A start tag, read: its lower-case name, its attributes, and whether it ends in `/>`.
struct StartTag {
    name: String,
    attributes: Vec<(String, String)>,
    self_closing: bool,
}

impl Parser<'_> {
    fn finish(mut self) -> Vec<Node> {
        while !self.open.is_empty() {
            self.close_innermost();
        }
        self.root
    }

    fn current_namespace(&self) -> Namespace {
        self.open
            .last()
            .map_or(Namespace::Html, |element| element.namespace)
    }

    fn children(&mut self) -> &mut Vec<Node> {
        match self.open.last_mut() {
            Some(element) => &mut element.children,
            None => &mut self.root,
        }
    }

    fn close_innermost(&mut self) {
        if let Some(element) = self.open.pop() {
            self.children().push(Node::Element(element));
        }
    }

    fn insert(&mut self, element: Element) {
        self.children().push(Node::Element(element));
    }

    /// Adds text to the innermost open element, merging it with the text before.
    fn text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let children = self.children();
        if let Some(Node::Text(previous)) = children.last_mut() {
            previous.push_str(text);
        } else {
            children.push(Node::Text(text.to_owned()));
        }
    }

    /// Leaves the head: the body starts with the next node.
    fn enter_body(&mut self) {
        self.in_head = false;
    }

    fn step(&mut self) {
        let c = self.chars[self.position];
        match c {
            '<' => self.markup(),
            '&' => {
                let (decoded, length) = match reference_at(self.chars, self.position, false) {
                    Some(reference) => reference,
                    None => ("&".to_owned(), 1),
                };
                self.position += length;
                self.character_text(&decoded);
            }
            _ => {
                self.position += 1;
                self.character_text(&c.to_string());
            }
        }
    }

    /// Text in the body: whitespace in the head is dropped (the head's own whitespace), anything
    /// else leaves the head.
    fn character_text(&mut self, text: &str) {
        if self.in_head {
            if text
                .chars()
                .all(|c| matches!(c, '\n' | '\t' | ' ' | '\u{c}' | '\r'))
            {
                return;
            }
            self.enter_body();
        }
        self.text(text);
    }

    /// Reads what starts with `<` at the position: a tag, a comment or declaration, or the `<` as
    /// text when no tag starts there.
    fn markup(&mut self) {
        let rest = &self.chars[self.position..];
        match rest.get(1) {
            Some('!') => self.skip_declaration(),
            Some('?') => self.skip_bogus_comment(),
            Some('/') => match rest.get(2) {
                Some(c) if c.is_ascii_alphabetic() => self.end_tag(),
                Some('>') => self.position += 3,
                None => {
                    self.position += 2;
                    self.character_text("</");
                }
                Some(_) => self.skip_bogus_comment(),
            },
            Some(c) if c.is_ascii_alphabetic() => self.start_tag(),
            _ => {
                self.position += 1;
                self.character_text("<");
            }
        }
    }

    /// Skips to just past the next `>`, or to the end.
    fn skip_past_gt(&mut self) {
        let end = self.chars[self.position..]
            .iter()
            .position(|&c| c == '>')
            .map_or(self.chars.len(), |offset| self.position + offset + 1);
        self.position = end;
    }

    fn skip_bogus_comment(&mut self) {
        self.skip_past_gt();
    }

    /// A comment (`<!--`), the doctype, or a bogus comment (`<!...>`); none of them reaches the
    /// output.
    fn skip_declaration(&mut self) {
        let body = self.position + 2;
        if self
            .chars
            .get(body..)
            .is_some_and(|rest| rest.starts_with(&['-', '-']))
        {
            let text = body + 2;
            if self.chars.get(text) == Some(&'>') {
                self.position = text + 1;
            } else if self
                .chars
                .get(text..)
                .is_some_and(|rest| rest.starts_with(&['-', '>']))
            {
                self.position = text + 2;
            } else {
                let end = find_comment_end(self.chars, text);
                self.position = end;
            }
        } else {
            self.skip_past_gt();
        }
    }

    fn start_tag(&mut self) {
        let Some(tag) = self.read_start_tag() else {
            // The input ends inside the tag, which then never closes.
            self.position = self.chars.len();
            return;
        };
        let mut name = tag.name;
        let mut namespace = match name.as_str() {
            "svg" => Namespace::Svg,
            "math" => Namespace::MathMl,
            _ => self.current_namespace(),
        };
        let leaves_foreign_content = namespace != Namespace::Html
            && (FOREIGN_BREAKOUT.contains(&name.as_str())
                || (name == "font"
                    && tag.attributes.iter().any(|(attribute, _)| {
                        matches!(attribute.as_str(), "color" | "face" | "size")
                    })));
        if leaves_foreign_content {
            // The parser closes the foreign elements open above the HTML one, and reads the tag
            // in HTML (the integration points are not reproduced).
            while self
                .open
                .last()
                .is_some_and(|element| element.namespace != Namespace::Html)
            {
                self.close_innermost();
            }
            namespace = Namespace::Html;
        }
        if namespace == Namespace::Html && name == "image" {
            name = "img".to_owned();
        }
        if namespace == Namespace::Html && matches!(name.as_str(), "html" | "head" | "body") {
            // The parser merges their attributes into the document, which the output does not hold.
            return;
        }
        if self.in_head && namespace == Namespace::Html {
            if HEAD_VOID.contains(&name.as_str()) {
                return;
            }
            if HEAD_RAW.contains(&name.as_str()) {
                let _ = self.raw_content(&name, false);
                return;
            }
            if name == "noscript" {
                self.enter_body();
                return;
            }
        }
        self.enter_body();
        let is_void = namespace == Namespace::Html && VOID_ELEMENTS.contains(&name.as_str());
        let is_raw = namespace == Namespace::Html
            && (RAW_TEXT_ELEMENTS.contains(&name.as_str())
                || RCDATA_ELEMENTS.contains(&name.as_str()));
        let mut element = Element {
            name,
            namespace,
            attributes: tag.attributes,
            children: Vec::new(),
        };
        if is_void || (namespace != Namespace::Html && tag.self_closing) {
            self.insert(element);
        } else if is_raw {
            // The content is text up to the element's end tag (RCDATA also decodes references).
            let rcdata = RCDATA_ELEMENTS.contains(&element.name.as_str());
            let content = self.raw_content(&element.name.clone(), rcdata);
            if !content.is_empty() {
                element.children.push(Node::Text(content));
            }
            self.insert(element);
        } else if namespace == Namespace::Html && element.name == "plaintext" {
            let content: String = self.chars[self.position..].iter().collect();
            self.position = self.chars.len();
            if !content.is_empty() {
                element.children.push(Node::Text(content));
            }
            self.insert(element);
        } else {
            self.open.push(element);
        }
    }

    /// The text up to the end tag of `name`, which is read too. With `decode`, character references
    /// in it are decoded (RCDATA).
    fn raw_content(&mut self, name: &str, decode: bool) -> String {
        let start = self.position;
        let name_chars: Vec<char> = name.chars().collect();
        let mut end = self.chars.len();
        let mut index = start;
        while index + 2 < self.chars.len() {
            if self.chars[index] == '<'
                && self.chars[index + 1] == '/'
                && self.chars[index + 2..].len() >= name_chars.len()
                && self.chars[index + 2..index + 2 + name_chars.len()]
                    .iter()
                    .zip(&name_chars)
                    .all(|(a, b)| a.to_ascii_lowercase() == *b)
            {
                let after = index + 2 + name_chars.len();
                let closes = self
                    .chars
                    .get(after)
                    .is_none_or(|c| matches!(c, '\n' | '\t' | '\u{c}' | ' ' | '/' | '>'));
                if closes {
                    end = index;
                    break;
                }
            }
            index += 1;
        }
        let raw: String = self.chars[start..end].iter().collect();
        self.position = end;
        if end < self.chars.len() {
            self.skip_past_gt();
        }
        if decode { decode_references(&raw) } else { raw }
    }

    /// An end tag: it closes the nearest open element of its name, and does nothing when none is open.
    fn end_tag(&mut self) {
        // The tag reader takes the name after the character at the position, so it starts at the
        // `/` of `</name`.
        self.position += 1;
        let Some(tag) = self.read_start_tag() else {
            self.position = self.chars.len();
            return;
        };
        let name = tag.name;
        if name == "br" {
            // `</br>` is read as `<br>`.
            self.insert(Element {
                name,
                namespace: Namespace::Html,
                attributes: Vec::new(),
                children: Vec::new(),
            });
            return;
        }
        if self.in_head || matches!(name.as_str(), "html" | "head" | "body") {
            return;
        }
        if let Some(index) = self.open.iter().rposition(|element| element.name == name) {
            while self.open.len() > index {
                self.close_innermost();
            }
        }
    }

    /// Reads a tag's name and attributes from the `<` at the position, and moves past its `>`.
    /// `None` when the input ends before the `>`.
    fn read_start_tag(&mut self) -> Option<StartTag> {
        let chars = self.chars;
        let mut index = self.position + 1;
        let name_start = index;
        while index < chars.len()
            && !matches!(chars[index], '\n' | '\t' | '\u{c}' | '\r' | ' ' | '/' | '>')
        {
            index += 1;
        }
        let name: String = chars[name_start..index]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();
        let mut attributes: Vec<(String, String)> = Vec::new();
        let mut self_closing = false;
        loop {
            while index < chars.len() && is_html_space(chars[index]) {
                index += 1;
            }
            let Some(&c) = chars.get(index) else {
                self.position = chars.len();
                return None;
            };
            match c {
                '>' => {
                    index += 1;
                    break;
                }
                '/' => {
                    if chars.get(index + 1) == Some(&'>') {
                        self_closing = true;
                        index += 2;
                        break;
                    }
                    index += 1;
                    continue;
                }
                _ => {}
            }
            // The first character of a name is taken even when it is `=`.
            let attribute_start = index;
            index += 1;
            while index < chars.len()
                && !matches!(
                    chars[index],
                    '\n' | '\t' | '\u{c}' | '\r' | ' ' | '/' | '>' | '='
                )
            {
                index += 1;
            }
            let attribute_name: String = chars[attribute_start..index]
                .iter()
                .collect::<String>()
                .to_ascii_lowercase();
            let mut after_name = index;
            while after_name < chars.len() && is_html_space(chars[after_name]) {
                after_name += 1;
            }
            let mut value = String::new();
            if chars.get(after_name) == Some(&'=') {
                index = after_name + 1;
                while index < chars.len() && is_html_space(chars[index]) {
                    index += 1;
                }
                match chars.get(index) {
                    None => {
                        self.position = chars.len();
                        return None;
                    }
                    Some(&quote @ ('"' | '\'')) => {
                        let value_start = index + 1;
                        let mut value_end = value_start;
                        while value_end < chars.len() && chars[value_end] != quote {
                            value_end += 1;
                        }
                        if value_end >= chars.len() {
                            self.position = chars.len();
                            return None;
                        }
                        value = decode_attribute(&chars[value_start..value_end]);
                        index = value_end + 1;
                    }
                    Some(&'>') => {}
                    Some(_) => {
                        let value_start = index;
                        while index < chars.len()
                            && !matches!(chars[index], '\n' | '\t' | '\u{c}' | '\r' | ' ' | '>')
                        {
                            index += 1;
                        }
                        value = decode_attribute(&chars[value_start..index]);
                    }
                }
            }
            if !attributes
                .iter()
                .any(|(existing, _)| *existing == attribute_name)
            {
                attributes.push((attribute_name, value));
            }
        }
        self.position = index;
        Some(StartTag {
            name,
            attributes,
            self_closing,
        })
    }
}

/// The HTML whitespace that separates the parts of a tag.
fn is_html_space(c: char) -> bool {
    matches!(c, '\n' | '\t' | '\u{c}' | '\r' | ' ')
}

/// The end of a comment body that starts at `from`: after `-->` or `--!>`, or the end of the input.
fn find_comment_end(chars: &[char], from: usize) -> usize {
    let mut index = from;
    while index < chars.len() {
        if chars[index] == '-' && chars.get(index + 1) == Some(&'-') {
            if chars.get(index + 2) == Some(&'>') {
                return index + 3;
            }
            if chars.get(index + 2) == Some(&'!') && chars.get(index + 3) == Some(&'>') {
                return index + 4;
            }
        }
        index += 1;
    }
    chars.len()
}

fn decode_attribute(chars: &[char]) -> String {
    let mut out = String::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '&'
            && let Some((decoded, length)) = reference_at(chars, index, true)
        {
            out.push_str(&decoded);
            index += length;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    out
}

/// The text with its character references decoded, as the HTML parser reads the text state.
pub(crate) fn decode_references(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '&'
            && let Some((decoded, length)) = reference_at(&chars, index, false)
        {
            out.push_str(&decoded);
            index += length;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    out
}

/// The character reference at `start`, which is a `&`: its decoded text and its length in
/// characters, or `None` when it is not a reference.
fn reference_at(chars: &[char], start: usize, in_attribute: bool) -> Option<(String, usize)> {
    if chars.get(start + 1) == Some(&'#') {
        let mut index = start + 2;
        let hex = matches!(chars.get(index), Some('x' | 'X'));
        if hex {
            index += 1;
        }
        let digits_start = index;
        while index < chars.len()
            && if hex {
                chars[index].is_ascii_hexdigit()
            } else {
                chars[index].is_ascii_digit()
            }
        {
            index += 1;
        }
        if index == digits_start {
            return None;
        }
        let base = if hex { 16 } else { 10 };
        let mut value: u32 = 0;
        for c in &chars[digits_start..index] {
            let digit = c.to_digit(base).unwrap_or(0);
            value = value
                .saturating_mul(base)
                .saturating_add(digit)
                .min(0x11_0000);
        }
        if chars.get(index) == Some(&';') {
            index += 1;
        }
        return Some((numeric_character(value).to_string(), index - start));
    }

    let mut end = start + 1;
    while end < chars.len() && chars[end].is_ascii_alphanumeric() {
        end += 1;
    }
    let name: String = chars[start + 1..end].iter().collect();
    if chars.get(end) == Some(&';') {
        let value = named_character(&name)?;
        return Some((value.to_owned(), end + 1 - start));
    }
    // Without `;`, only the legacy names are decoded, and a legacy name may be the start of a longer
    // run of letters, as the browser reads it.
    for length in (1..=name.len()).rev() {
        let prefix = &name[..length];
        if !entities::is_legacy(prefix) {
            continue;
        }
        let next = chars.get(start + 1 + length);
        if in_attribute && (next == Some(&'=') || next.is_some_and(|c| c.is_ascii_alphanumeric())) {
            return None;
        }
        let value = named_character(prefix)?;
        return Some((value.to_owned(), 1 + length));
    }
    None
}

/// The value of a named reference written without its `;`, from the full HTML5 table.
fn named_character(name: &str) -> Option<&'static str> {
    entities::named(name)
}

/// The character a numeric reference stands for, as the HTML parser reads it: NUL, surrogates and
/// values above U+10FFFF become U+FFFD, and the C1 controls take the windows-1252 characters.
fn numeric_character(value: u32) -> char {
    const WINDOWS_1252: [u32; 32] = [
        0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160,
        0x2039, 0x0152, 0x8D, 0x017D, 0x8F, 0x90, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013,
        0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x9D, 0x017E, 0x0178,
    ];
    let replaced = match value {
        0 | 0xD800..=0xDFFF => 0xFFFD,
        0x80..=0x9F => WINDOWS_1252[(value - 0x80) as usize],
        other => other,
    };
    char::from_u32(replaced).unwrap_or('\u{FFFD}')
}

/// DOMPurify's `_sanitizeElements` and `_sanitizeAttributes` over a list of nodes, whose parent is
/// `parent`: the nodes that are kept, with their attributes filtered. `forbid_style` is the
/// `FORBID_TAGS: ['style']` of the second pass.
fn sanitize_children(nodes: Vec<Node>, parent: Parent<'_>, forbid_style: bool) -> Vec<Node> {
    let mut kept = Vec::with_capacity(nodes.len());
    for node in nodes {
        let element = match node {
            Node::Text(text) => {
                kept.push(Node::Text(text));
                continue;
            }
            Node::Element(element) => element,
        };
        let forbidden = forbid_style && element.name == "style";
        if forbidden || !is_allowed_element(&element.name) {
            // `_sanitizeDisallowedNode`: the children move up unless the content is forbidden.
            if !FORBID_CONTENTS.contains(&element.name.as_str()) {
                kept.extend(sanitize_children(element.children, parent, forbid_style));
            }
            continue;
        }
        if !namespace_allowed(element.namespace, &element.name, parent) {
            continue;
        }
        let attributes = if element.namespace == Namespace::Html && element.name == "a" {
            sanitize_anchor_attributes(element.attributes)
        } else {
            sanitize_attributes(&element.name, element.attributes)
        };
        let children = sanitize_children(
            element.children,
            Parent {
                namespace: element.namespace,
                name: &element.name,
            },
            forbid_style,
        );
        kept.push(Node::Element(Element {
            name: element.name,
            namespace: element.namespace,
            attributes,
            children,
        }));
    }
    kept
}

/// The attribute that `setupDompurifyHooks` (`common.ts`) keeps a `target` in while DOMPurify runs.
const TEMPORARY_ATTRIBUTE: &str = "data-temp-href-target";

/// The two anchor hooks of `setupDompurifyHooks` for an HTML `a`, around `_sanitizeAttributes`.
/// `beforeSanitizeAttributes` copies the `target` (the first one: the parser keeps it) to the
/// temporary attribute, which replaces one already there in place. DOMPurify then checks that as a
/// data attribute, and drops the `target` itself, which is not allowed. `afterSanitizeAttributes`
/// reads the temporary attribute back if it survived, appends `target` with its value, removes the
/// temporary attribute, and sets `rel` to `noopener` for `_blank`, in place when `rel` was kept.
fn sanitize_anchor_attributes(mut attributes: Vec<(String, String)>) -> Vec<(String, String)> {
    if let Some(target) = attributes
        .iter()
        .find(|(name, _)| name == "target")
        .map(|(_, value)| value.clone())
    {
        set_attribute(&mut attributes, TEMPORARY_ATTRIBUTE, target);
    }
    let mut kept = sanitize_attributes("a", attributes);
    let Some(position) = kept
        .iter()
        .position(|(name, _)| name == TEMPORARY_ATTRIBUTE)
    else {
        return kept;
    };
    let (_, target) = kept.remove(position);
    let blank = target == "_blank";
    kept.push(("target".to_owned(), target));
    if blank {
        set_attribute(&mut kept, "rel", "noopener".to_owned());
    }
    kept
}

/// `setAttribute`: the value of an attribute that is there is replaced in place, and one that is
/// not is appended.
fn set_attribute(attributes: &mut Vec<(String, String)>, name: &str, value: String) {
    match attributes.iter_mut().find(|(existing, _)| existing == name) {
        Some(entry) => entry.1 = value,
        None => attributes.push((name.to_owned(), value)),
    }
}

/// `ALLOWED_TAGS`: the union of the HTML, SVG and MathML lists.
fn is_allowed_element(name: &str) -> bool {
    HTML_ELEMENTS.contains(&name) || SVG_ELEMENTS.contains(&name) || MATHML_ELEMENTS.contains(&name)
}

/// `_checkValidNamespace`: whether an element of `namespace` may sit under `parent`.
fn namespace_allowed(namespace: Namespace, name: &str, parent: Parent<'_>) -> bool {
    match namespace {
        Namespace::Html => {
            if parent.namespace == Namespace::Svg && parent.name != "annotation-xml" {
                return false;
            }
            if parent.namespace == Namespace::MathMl
                && !MATHML_TEXT_INTEGRATION.contains(&parent.name)
            {
                return false;
            }
            !in_mathml(name) && (COMMON_SVG_AND_HTML.contains(&name) || !in_svg(name))
        }
        Namespace::Svg => match parent.namespace {
            Namespace::Html => name == "svg",
            Namespace::MathMl => {
                name == "svg"
                    && (parent.name == "annotation-xml"
                        || MATHML_TEXT_INTEGRATION.contains(&parent.name))
            }
            Namespace::Svg => in_svg(name),
        },
        Namespace::MathMl => match parent.namespace {
            Namespace::Html => name == "math",
            Namespace::Svg => name == "math" && parent.name == "annotation-xml",
            Namespace::MathMl => in_mathml(name),
        },
    }
}

/// The MathML text integration points, and the elements that DOMPurify lists in both namespaces.
const MATHML_TEXT_INTEGRATION: &[&str] = &["mi", "mo", "mn", "ms", "mtext"];
const COMMON_SVG_AND_HTML: &[&str] = &["title", "style", "font", "a", "script"];

/// `ALL_SVG_TAGS`: the SVG elements, allowed or not.
fn in_svg(name: &str) -> bool {
    SVG_ELEMENTS.contains(&name) || SVG_DISALLOWED_ELEMENTS.contains(&name)
}

/// `ALL_MATHML_TAGS`: the MathML elements, allowed or not.
fn in_mathml(name: &str) -> bool {
    MATHML_ELEMENTS.contains(&name) || MATHML_DISALLOWED_ELEMENTS.contains(&name)
}

/// The start tags that leave foreign content (SVG and MathML) for HTML, as the parser's "in foreign
/// content" rule gives them. A `font` leaves it only with a `color`, `face` or `size` attribute.
const FOREIGN_BREAKOUT: &[&str] = &[
    "b",
    "big",
    "blockquote",
    "body",
    "br",
    "center",
    "code",
    "dd",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "hr",
    "i",
    "img",
    "li",
    "listing",
    "menu",
    "meta",
    "nobr",
    "ol",
    "p",
    "pre",
    "ruby",
    "s",
    "small",
    "span",
    "strong",
    "strike",
    "sub",
    "sup",
    "table",
    "tt",
    "u",
    "ul",
    "var",
];

/// `_sanitizeAttributes` for one element: the attributes that `_isValidAttribute` keeps, with the
/// values trimmed (except `value`).
fn sanitize_attributes(tag: &str, attributes: Vec<(String, String)>) -> Vec<(String, String)> {
    attributes
        .into_iter()
        .filter_map(|(name, raw)| {
            let value = if name == "value" {
                raw
            } else {
                js_trim(&raw).to_owned()
            };
            if has_unsafe_markup(&value) {
                return None;
            }
            if name == "attributename" && value.contains("href") {
                return None;
            }
            if name == "patchsrc" || (name == "for" && tag != "label" && tag != "output") {
                return None;
            }
            attribute_is_valid(tag, &name, &value).then_some((name, value))
        })
        .collect()
}

/// `SAFE_FOR_XML` on an attribute value: a comment end, a `]>`, or the end of a raw-text element.
fn has_unsafe_markup(value: &str) -> bool {
    if value.contains("-->") || value.contains("--!>") || value.contains("]>") {
        return true;
    }
    let lower = value.to_ascii_lowercase();
    [
        "style", "script", "title", "xmp", "textarea", "noscript", "iframe", "noembed", "noframes",
    ]
    .iter()
    .any(|name| lower.contains(&format!("</{name}")))
}

/// `_isValidAttribute`, for an attribute name and value that are already trimmed.
fn attribute_is_valid(tag: &str, name: &str, value: &str) -> bool {
    if is_data_attribute(name) || is_aria_attribute(name) {
        return true;
    }
    if !ALLOWED_ATTRIBUTES.contains(&name) {
        return false;
    }
    if URI_SAFE_ATTRIBUTES.contains(&name) {
        return true;
    }
    let stripped: String = value.chars().filter(|&c| !is_uri_whitespace(c)).collect();
    if is_allowed_uri(&stripped) {
        return true;
    }
    if matches!(name, "src" | "xlink:href" | "href")
        && tag != "script"
        && value.starts_with("data:")
        && DATA_URI_TAGS.contains(&tag)
    {
        return true;
    }
    value.is_empty()
}

/// `^data-[\-\w.·-￿]+$`.
fn is_data_attribute(name: &str) -> bool {
    name.strip_prefix("data-").is_some_and(|rest| {
        !rest.is_empty()
            && rest.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') || c as u32 >= 0xB7
            })
    })
}

/// `^aria-[\-\w]+$`.
fn is_aria_attribute(name: &str) -> bool {
    name.strip_prefix("aria-").is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    })
}

/// The characters that `ATTR_WHITESPACE` removes before the URI test.
fn is_uri_whitespace(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{20}' | '\u{a0}' | '\u{1680}' | '\u{180e}' | '\u{2000}'..='\u{2029}' | '\u{205f}' | '\u{3000}')
}

/// `IS_ALLOWED_URI`: a known scheme, a value that does not start with an ASCII letter, or a run of
/// scheme characters that ends the value or is not followed by `:`.
fn is_allowed_uri(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    const SCHEMES: &[&str] = &[
        "ftp:", "ftps:", "http:", "https:", "mailto:", "tel:", "callto:", "sms:", "cid:", "xmpp:",
        "matrix:",
    ];
    if SCHEMES.iter().any(|scheme| lower.starts_with(scheme)) {
        return true;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return true;
    }
    let run = value
        .chars()
        .take_while(|c| c.is_ascii_alphabetic() || matches!(c, '+' | '.' | '-'))
        .count();
    run > 0 && value.chars().nth(run) != Some(':')
}

/// The attributes whose values are written with `&`, no-break space and `"` escaped, and the text
/// with `&`, no-break space, `<` and `>` escaped, as HTML serialisation writes them.
fn serialize(nodes: &[Node], raw_text: bool, out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(text) => {
                if raw_text {
                    out.push_str(text);
                } else {
                    for c in text.chars() {
                        match c {
                            '&' => out.push_str("&amp;"),
                            '\u{a0}' => out.push_str("&nbsp;"),
                            '<' => out.push_str("&lt;"),
                            '>' => out.push_str("&gt;"),
                            other => out.push(other),
                        }
                    }
                }
            }
            Node::Element(element) => {
                out.push('<');
                out.push_str(&element.name);
                for (name, value) in &element.attributes {
                    out.push(' ');
                    out.push_str(name);
                    out.push_str("=\"");
                    for c in value.chars() {
                        match c {
                            '&' => out.push_str("&amp;"),
                            '\u{a0}' => out.push_str("&nbsp;"),
                            '"' => out.push_str("&quot;"),
                            other => out.push(other),
                        }
                    }
                    out.push('"');
                }
                out.push('>');
                if element.namespace == Namespace::Html
                    && VOID_ELEMENTS.contains(&element.name.as_str())
                {
                    continue;
                }
                let raw = element.namespace == Namespace::Html
                    && RAW_TEXT_PARENTS.contains(&element.name.as_str());
                serialize(&element.children, raw, out);
                out.push_str("</");
                out.push_str(&element.name);
                out.push('>');
            }
        }
    }
}

// The tables are DOMPurify 3.4.12's own lists (packages/mermaid/node_modules/dompurify/dist/purify.cjs.js).
/// The HTML elements DOMPurify allows (`html$1`).
const HTML_ELEMENTS: &[&str] = &[
    "a",
    "abbr",
    "acronym",
    "address",
    "area",
    "article",
    "aside",
    "audio",
    "b",
    "bdi",
    "bdo",
    "big",
    "blink",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "center",
    "cite",
    "code",
    "col",
    "colgroup",
    "content",
    "data",
    "datalist",
    "dd",
    "decorator",
    "del",
    "details",
    "dfn",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "element",
    "em",
    "fieldset",
    "figcaption",
    "figure",
    "font",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "i",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "main",
    "map",
    "mark",
    "marquee",
    "menu",
    "menuitem",
    "meter",
    "nav",
    "nobr",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "picture",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "search",
    "section",
    "select",
    "shadow",
    "slot",
    "small",
    "source",
    "spacer",
    "span",
    "strike",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "tr",
    "track",
    "tt",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

/// The SVG elements DOMPurify allows (`svg$1` and `svgFilters`).
const SVG_ELEMENTS: &[&str] = &[
    "a",
    "altglyph",
    "altglyphdef",
    "altglyphitem",
    "animatecolor",
    "animatemotion",
    "animatetransform",
    "circle",
    "clippath",
    "defs",
    "desc",
    "ellipse",
    "enterkeyhint",
    "exportparts",
    "feBlend",
    "feColorMatrix",
    "feComponentTransfer",
    "feComposite",
    "feConvolveMatrix",
    "feDiffuseLighting",
    "feDisplacementMap",
    "feDistantLight",
    "feDropShadow",
    "feFlood",
    "feFuncA",
    "feFuncB",
    "feFuncG",
    "feFuncR",
    "feGaussianBlur",
    "feImage",
    "feMerge",
    "feMergeNode",
    "feMorphology",
    "feOffset",
    "fePointLight",
    "feSpecularLighting",
    "feSpotLight",
    "feTile",
    "feTurbulence",
    "filter",
    "font",
    "g",
    "glyph",
    "glyphref",
    "hkern",
    "image",
    "inputmode",
    "line",
    "lineargradient",
    "marker",
    "mask",
    "metadata",
    "mpath",
    "part",
    "path",
    "pattern",
    "polygon",
    "polyline",
    "radialgradient",
    "rect",
    "stop",
    "style",
    "svg",
    "switch",
    "symbol",
    "text",
    "textpath",
    "title",
    "tref",
    "tspan",
    "view",
    "vkern",
];

/// The SVG elements DOMPurify knows and disallows (`svgDisallowed`); with `SVG_ELEMENTS` they are the SVG names a namespace check accepts.
const SVG_DISALLOWED_ELEMENTS: &[&str] = &[
    "animate",
    "color-profile",
    "cursor",
    "discard",
    "font-face",
    "font-face-format",
    "font-face-name",
    "font-face-src",
    "font-face-uri",
    "foreignobject",
    "hatch",
    "hatchpath",
    "mesh",
    "meshgradient",
    "meshpatch",
    "meshrow",
    "missing-glyph",
    "script",
    "set",
    "solidcolor",
    "unknown",
    "use",
];

/// The MathML elements DOMPurify allows (`mathMl$1`).
const MATHML_ELEMENTS: &[&str] = &[
    "math",
    "menclose",
    "merror",
    "mfenced",
    "mfrac",
    "mglyph",
    "mi",
    "mlabeledtr",
    "mmultiscripts",
    "mn",
    "mo",
    "mover",
    "mpadded",
    "mphantom",
    "mprescripts",
    "mroot",
    "mrow",
    "ms",
    "mspace",
    "msqrt",
    "mstyle",
    "msub",
    "msubsup",
    "msup",
    "mtable",
    "mtd",
    "mtext",
    "mtr",
    "munder",
    "munderover",
];

/// The MathML elements DOMPurify knows and disallows (`mathMlDisallowed`).
const MATHML_DISALLOWED_ELEMENTS: &[&str] = &[
    "annotation",
    "annotation-xml",
    "maction",
    "maligngroup",
    "malignmark",
    "mlongdiv",
    "mprescripts",
    "mscarries",
    "mscarry",
    "msgroup",
    "msline",
    "msrow",
    "mstack",
    "none",
    "semantics",
];

/// The attributes DOMPurify allows: the HTML, SVG, MathML and XML lists together (`DEFAULT_ALLOWED_ATTR`).
const ALLOWED_ATTRIBUTES: &[&str] = &[
    "accent",
    "accent-height",
    "accentunder",
    "accept",
    "accumulate",
    "action",
    "additive",
    "align",
    "alignment-baseline",
    "alt",
    "amplitude",
    "ascent",
    "attributename",
    "attributetype",
    "autocapitalize",
    "autocomplete",
    "autopictureinpicture",
    "autoplay",
    "azimuth",
    "background",
    "basefrequency",
    "baseline-shift",
    "begin",
    "bevelled",
    "bgcolor",
    "bias",
    "border",
    "by",
    "capture",
    "cellpadding",
    "cellspacing",
    "checked",
    "cite",
    "class",
    "clear",
    "clip",
    "clip-path",
    "clip-rule",
    "clippathunits",
    "close",
    "color",
    "color-interpolation",
    "color-interpolation-filters",
    "color-profile",
    "color-rendering",
    "cols",
    "colspan",
    "columnalign",
    "columnlines",
    "columnspacing",
    "columnspan",
    "command",
    "commandfor",
    "controls",
    "controlslist",
    "coords",
    "crossorigin",
    "cx",
    "cy",
    "d",
    "datetime",
    "decoding",
    "default",
    "denomalign",
    "depth",
    "diffuseconstant",
    "dir",
    "direction",
    "disabled",
    "disablepictureinpicture",
    "disableremoteplayback",
    "display",
    "displaystyle",
    "divisor",
    "dominant-baseline",
    "download",
    "draggable",
    "dur",
    "dx",
    "dy",
    "edgemode",
    "elevation",
    "encoding",
    "enctype",
    "end",
    "enterkeyhint",
    "exponent",
    "exportparts",
    "face",
    "fence",
    "fill",
    "fill-opacity",
    "fill-rule",
    "filter",
    "filterunits",
    "flood-color",
    "flood-opacity",
    "font-family",
    "font-size",
    "font-size-adjust",
    "font-stretch",
    "font-style",
    "font-variant",
    "font-weight",
    "for",
    "frame",
    "fx",
    "fy",
    "g1",
    "g2",
    "glyph-name",
    "glyphref",
    "gradienttransform",
    "gradientunits",
    "headers",
    "height",
    "hidden",
    "high",
    "href",
    "hreflang",
    "id",
    "image-rendering",
    "in",
    "in2",
    "inert",
    "inputmode",
    "integrity",
    "intercept",
    "ismap",
    "k",
    "k1",
    "k2",
    "k3",
    "k4",
    "kernelmatrix",
    "kernelunitlength",
    "kerning",
    "keypoints",
    "keysplines",
    "keytimes",
    "kind",
    "label",
    "lang",
    "largeop",
    "length",
    "lengthadjust",
    "letter-spacing",
    "lighting-color",
    "linethickness",
    "list",
    "loading",
    "local",
    "loop",
    "low",
    "lquote",
    "lspace",
    "marker-end",
    "marker-mid",
    "marker-start",
    "markerheight",
    "markerunits",
    "markerwidth",
    "mask",
    "mask-type",
    "maskcontentunits",
    "maskunits",
    "mathbackground",
    "mathcolor",
    "mathsize",
    "mathvariant",
    "max",
    "maxlength",
    "maxsize",
    "media",
    "method",
    "min",
    "minlength",
    "minsize",
    "mode",
    "movablelimits",
    "multiple",
    "muted",
    "name",
    "nonce",
    "noshade",
    "notation",
    "novalidate",
    "nowrap",
    "numalign",
    "numoctaves",
    "offset",
    "opacity",
    "open",
    "operator",
    "optimum",
    "order",
    "orient",
    "orientation",
    "origin",
    "overflow",
    "paint-order",
    "part",
    "path",
    "pathlength",
    "pattern",
    "patterncontentunits",
    "patterntransform",
    "patternunits",
    "placeholder",
    "playsinline",
    "points",
    "popover",
    "popovertarget",
    "popovertargetaction",
    "poster",
    "preload",
    "preservealpha",
    "preserveaspectratio",
    "primitiveunits",
    "pubdate",
    "r",
    "radiogroup",
    "radius",
    "readonly",
    "refx",
    "refy",
    "rel",
    "repeatcount",
    "repeatdur",
    "required",
    "restart",
    "result",
    "rev",
    "reversed",
    "role",
    "rotate",
    "rowalign",
    "rowlines",
    "rows",
    "rowspacing",
    "rowspan",
    "rquote",
    "rspace",
    "rx",
    "ry",
    "scale",
    "scope",
    "scriptlevel",
    "scriptminsize",
    "scriptsizemultiplier",
    "seed",
    "selected",
    "selection",
    "separator",
    "separators",
    "shape",
    "shape-rendering",
    "size",
    "sizes",
    "slope",
    "slot",
    "span",
    "specularconstant",
    "specularexponent",
    "spellcheck",
    "spreadmethod",
    "src",
    "srclang",
    "srcset",
    "start",
    "startoffset",
    "stddeviation",
    "step",
    "stitchtiles",
    "stop-color",
    "stop-opacity",
    "stretchy",
    "stroke",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-miterlimit",
    "stroke-opacity",
    "stroke-width",
    "style",
    "subscriptshift",
    "summary",
    "supscriptshift",
    "surfacescale",
    "symmetric",
    "systemlanguage",
    "tabindex",
    "tablevalues",
    "targetx",
    "targety",
    "text-anchor",
    "text-decoration",
    "text-orientation",
    "text-rendering",
    "textlength",
    "title",
    "transform",
    "transform-origin",
    "translate",
    "type",
    "u1",
    "u2",
    "unicode",
    "usemap",
    "valign",
    "value",
    "values",
    "version",
    "vert-adv-y",
    "vert-origin-x",
    "vert-origin-y",
    "viewbox",
    "visibility",
    "voffset",
    "width",
    "word-spacing",
    "wrap",
    "writing-mode",
    "x",
    "x1",
    "x2",
    "xchannelselector",
    "xlink:href",
    "xlink:title",
    "xml:id",
    "xml:space",
    "xmlns",
    "xmlns:xlink",
    "y",
    "y1",
    "y2",
    "ychannelselector",
    "z",
    "zoomandpan",
];

/// Elements whose content DOMPurify drops with them (`DEFAULT_FORBID_CONTENTS`).
const FORBID_CONTENTS: &[&str] = &[
    "annotation-xml",
    "audio",
    "colgroup",
    "desc",
    "foreignobject",
    "head",
    "iframe",
    "math",
    "mi",
    "mn",
    "mo",
    "ms",
    "mtext",
    "noembed",
    "noframes",
    "noscript",
    "plaintext",
    "script",
    "selectedcontent",
    "style",
    "svg",
    "template",
    "thead",
    "title",
    "video",
    "xmp",
];

/// Elements whose `src` and `href` may be a `data:` URI (`DEFAULT_DATA_URI_TAGS`).
const DATA_URI_TAGS: &[&str] = &["audio", "image", "img", "source", "track", "video"];

/// Attributes whose value is not checked against the URI pattern (`DEFAULT_URI_SAFE_ATTRIBUTES`).
const URI_SAFE_ATTRIBUTES: &[&str] = &[
    "alt",
    "class",
    "for",
    "id",
    "label",
    "name",
    "pattern",
    "placeholder",
    "role",
    "style",
    "summary",
    "title",
    "value",
    "xmlns",
];

#[cfg(test)]
mod tests {
    use super::{dompurify, sanitize_text};

    /// The reference's `sanitizeText` on the strict path (DOMPurify run twice, see the module
    /// documentation), with jsdom, for each input in this order: the outputs the port must give, for
    /// the cases the subset covers that have no anchor with a `target` (inputs with a `<`). One pass
    /// with `{ FORBID_TAGS: ["style"] }` gives the same outputs for these. The anchors are in
    /// `HOOKED_CORPUS`.
    const DOMPURIFY_CORPUS: &[(&str, &str)] = &[
        ("<11", "&lt;11"),
        ("5 > 3 && 2 < 4", "5 &gt; 3 &amp;&amp; 2 &lt; 4"),
        (
            "**Plain text**:\n  5 > 3 && 2 < 4",
            "**Plain text**:\n  5 &gt; 3 &amp;&amp; 2 &lt; 4",
        ),
        ("<**1**1", "&lt;**1**1"),
        (
            "<img src='https://mermaid.js.org/mermaid-logo.svg'>",
            "<img src=\"https://mermaid.js.org/mermaid-logo.svg\">",
        ),
        (
            "<img src='https://mermaid.js.org/mermaid-logo.svg'> some text",
            "<img src=\"https://mermaid.js.org/mermaid-logo.svg\"> some text",
        ),
        (
            "<img src=\"https://mermaid.js.org/mermaid-logo.svg\"> more text <img src='https://mermaid.js.org/mermaid-logo.svg'>",
            "<img src=\"https://mermaid.js.org/mermaid-logo.svg\"> more text <img src=\"https://mermaid.js.org/mermaid-logo.svg\">",
        ),
        ("a<br>b", "a<br>b"),
        ("a<br/>b", "a<br>b"),
        ("a</br>b", "a<br>b"),
        ("<b>bold</b> text", "<b>bold</b> text"),
        ("<script>alert(1)</script>x", "x"),
        ("<style>p{color:red}</style>y", "y"),
        ("<foo>unknown <i>kept</i></foo>", "unknown <i>kept</i>"),
        ("x<!-- comment -->y", "xy"),
        ("<a href=\"javascript:alert(1)\">link</a>", "<a>link</a>"),
        (
            "<a href=\"https://example.com/x?a=1&b=2\">x</a>",
            "<a href=\"https://example.com/x?a=1&amp;b=2\">x</a>",
        ),
        (
            "<img src=\"x.png\" onerror=\"alert(1)\">",
            "<img src=\"x.png\">",
        ),
        (
            "<span data-x=\"1\" aria-label=\"q\" class=\"c\" style=\"color:red\">s</span>",
            "<span data-x=\"1\" aria-label=\"q\" class=\"c\" style=\"color:red\">s</span>",
        ),
        ("AT&T <b>x</b>", "AT&amp;T <b>x</b>"),
        (
            "&lt;b&gt; and &amp; <i>y</i>",
            "&lt;b&gt; and &amp; <i>y</i>",
        ),
        ("&#39;quoted&#39; <b>x</b>", "'quoted' <b>x</b>"),
        ("&copy; 2024 <b>x</b>", "© 2024 <b>x</b>"),
        ("<circle r=\"5\"></circle>", ""),
        (
            "<svg><circle r=\"5\"/></svg>",
            "<svg><circle r=\"5\"></circle></svg>",
        ),
        (
            "<svg><script>x</script><rect width=\"1\"/></svg>",
            "<svg><rect width=\"1\"></rect></svg>",
        ),
        ("<title>t</title>x", "x"),
        ("<div>a<p>b</p></div>", "<div>a<p>b</p></div>"),
        ("<br>", "<br>"),
        ("  <b>lead</b>", "  <b>lead</b>"),
        ("a\nb <i>c</i>", "a\nb <i>c</i>"),
        ("x < y and a > b <b>z</b>", "x &lt; y and a &gt; b <b>z</b>"),
        ("<3 hearts <b>x</b>", "&lt;3 hearts <b>x</b>"),
        ("a <= b <b>x</b>", "a &lt;= b <b>x</b>"),
        ("<img src=x alt=y>", "<img src=\"x\" alt=\"y\">"),
        ("<img src='  https://x  '>", "<img src=\"https://x\">"),
        (
            "<img src='data:image/png;base64,AAA'>",
            "<img src=\"data:image/png;base64,AAA\">",
        ),
        ("<p>para</p>", "<p>para</p>"),
        ("<textarea>x</textarea>", "<textarea>x</textarea>"),
        ("<xmp><b>x</b></xmp> tail", " tail"),
        ("<iframe src='x'></iframe>after", "after"),
        (
            "<select><option>a</option></select>",
            "<select><option>a</option></select>",
        ),
        ("<b title='it&apos;s'>x</b>", "<b title=\"it's\">x</b>"),
        ("<b title='a\"b'>x</b>", "<b title=\"a&quot;b\">x</b>"),
        ("<img alt='  spaced  '>", "<img alt=\"spaced\">"),
        ("<b><b>x</b></b>", "<b><b>x</b></b>"),
        ("x<br >y", "x<br>y"),
        ("<b\n>x</b>", "<b>x</b>"),
        ("<1>", "&lt;1&gt;"),
        ("<!DOCTYPE html><b>x</b>", "<b>x</b>"),
        ("<?php echo 1 ?>x", "x"),
        ("</b>x", "x"),
        ("a <b>unclosed", "a <b>unclosed</b>"),
        ("<a href=' javascript:alert(1)'>x</a>", "<a>x</a>"),
        ("<b>&nbsp;x</b>", "<b>&nbsp;x</b>"),
        ("x&nbsp;<b>y</b>", "x&nbsp;<b>y</b>"),
        (
            "<b>&unknown; &#x41; &#x80; &#0;</b>",
            "<b>&amp;unknown; A € �</b>",
        ),
        ("<b>a\r\nb</b>", "<b>a\nb</b>"),
        (
            "<p class=\"a\">x</p><p>y</p>",
            "<p class=\"a\">x</p><p>y</p>",
        ),
        (
            "<img src=\"https://a.b/c.svg\"/> <b>tail</b>",
            "<img src=\"https://a.b/c.svg\"> <b>tail</b>",
        ),
        (
            "<label for=\"x\">l</label><b for=\"y\">z</b>",
            "<label for=\"x\">l</label><b>z</b>",
        ),
        (
            "<b>&AMP;&amp&lt&gt;&quot;&QUOT;&nbsp;&copy;&reg;&apos;</b>",
            "<b>&amp;&amp;&lt;&gt;\"\"&nbsp;©®'</b>",
        ),
        (
            "<b>&copy&ampx&lt2 &amp=1 <i>k</i></b>",
            "<b>©&amp;x&lt;2 &amp;=1 <i>k</i></b>",
        ),
        (
            "<b title=\"&amp=1&amp;2 &copy\">t</b>",
            "<b title=\"&amp;amp=1&amp;2 ©\">t</b>",
        ),
        (
            "<b>&bull;&cent;&deg;&euro;&hellip;&laquo;&larr;&ldquo;&lsquo;&mdash;&micro;&middot;&ndash;&para;&plusmn;&pound;&raquo;&rarr;&rdquo;&rsquo;&sect;&times;&trade;&uarr;&darr;&harr;&yen;</b>",
            "<b>•¢°€…«←“‘—µ·–¶±£»→”’§×™↑↓↔¥</b>",
        ),
        (
            "<b>&#128512; &#x1F600; &#xD800; &#x110000; &#150;</b>",
            "<b>😀 😀 � � –</b>",
        ),
        (
            "<b>&#; &#x; &#xZ; &zzz; &amp</b>",
            "<b>&amp;#; &amp;#x; &amp;#xZ; &amp;zzz; &amp;</b>",
        ),
        (
            "<b a=1 a=2 b c='x' d=&quot;y&quot;>z</b>",
            "<b d=\"&quot;y&quot;\">z</b>",
        ),
        ("<b =x>z</b>", "<b>z</b>"),
        ("<b/>tail", "<b>tail</b>"),
        ("<svg><b>x</b></svg>", "<svg></svg><b>x</b>"),
        (
            "<svg><a href=\"https://e.x\">l</a></svg>",
            "<svg><a href=\"https://e.x\">l</a></svg>",
        ),
        ("<math><mi>x</mi></math>", "<math><mi>x</mi></math>"),
        ("<meta charset=\"x\"><b>m</b>", "<b>m</b>"),
        ("<link rel=\"x\"> <b>l</b>", "<b>l</b>"),
        ("  <title>head</title> <b>t</b>", "  <b>t</b>"),
        ("<noscript>ns</noscript><b>n</b>", "ns<b>n</b>"),
        ("<template><b>tt</b></template>", ""),
        ("<image src=\"a.png\">", "<img src=\"a.png\">"),
        ("<html><body><b>h</b></body></html>", "<b>h</b>"),
        ("<plaintext><b>p</b>", ""),
        (
            "<textarea><b>q</b></textarea>",
            "<textarea>&lt;b&gt;q&lt;/b&gt;</textarea>",
        ),
        ("<noframes><b>x</b></noframes><i>y</i>", "<i>y</i>"),
        ("<b>x</b><br><b>y</b>", "<b>x</b><br><b>y</b>"),
        ("<script>a</scr", ""),
        (
            "<b>x</b><template><b>tt</b></template>",
            "<b>x</b><template><b>tt</b></template>",
        ),
        ("<template>a</template><b>t</b>", "<b>t</b>"),
    ];

    /// The reference's `sanitizeText` on the strict path, with its anchor hooks: the first pass
    /// installs them, so every strict call has them. The inputs are the anchors (an HTML `a` with a
    /// `target`, and the values, names and namespaces that the hooks leave alone), in this order.
    const HOOKED_CORPUS: &[(&str, &str)] = &[
        (
            "<a target=\"_blank\" href=\"x\">t</a>",
            "<a href=\"x\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a href=\"x\" target=\"_blank\">t</a>",
            "<a href=\"x\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" rel=\"nofollow\" href=\"x\">t</a>",
            "<a rel=\"noopener\" href=\"x\" target=\"_blank\">t</a>",
        ),
        (
            "<a rel=\"nofollow\" target=\"_blank\" href=\"x\">t</a>",
            "<a rel=\"noopener\" href=\"x\" target=\"_blank\">t</a>",
        ),
        ("<a target=\"_self\">t</a>", "<a target=\"_self\">t</a>"),
        ("<a target=\"_top\">t</a>", "<a target=\"_top\">t</a>"),
        ("<a target=\"frame1\">t</a>", "<a target=\"frame1\">t</a>"),
        ("<a target=\"\">t</a>", "<a target=\"\">t</a>"),
        (
            "<a target=\"  _blank  \">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        ("<a target=\"_BLANK\">t</a>", "<a target=\"_BLANK\">t</a>"),
        (
            "<A TARGET=\"_blank\">t</A>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a Target=\"_blank\">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" target=\"_self\">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        ("<a target=\"a&amp;b\">t</a>", "<a target=\"a&amp;b\">t</a>"),
        (
            "<a target=\"a&quot;b\">t</a>",
            "<a target=\"a&quot;b\">t</a>",
        ),
        ("<a target='a\"b'>t</a>", "<a target=\"a&quot;b\">t</a>"),
        ("<a target=\"a&#39;b\">t</a>", "<a target=\"a'b\">t</a>"),
        ("<b target=\"_blank\">x</b>", "<b>x</b>"),
        ("<span target=\"_blank\">x</span>", "<span>x</span>"),
        (
            "<svg><a target=\"_blank\" href=\"x\">t</a></svg>",
            "<svg><a href=\"x\">t</a></svg>",
        ),
        (
            "<svg><a href=\"x\" target=\"_blank\">t</a></svg>",
            "<svg><a href=\"x\">t</a></svg>",
        ),
        (
            "<a target=\"_blank\" class=\"c\" href=\"x\" title=\"t\" id=\"i\">t</a>",
            "<a class=\"c\" href=\"x\" title=\"t\" id=\"i\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" href=\"javascript:alert(1)\">x</a>",
            "<a rel=\"noopener\" target=\"_blank\">x</a>",
        ),
        (
            "<a target=\"_blank\" onclick=\"x()\">x</a>",
            "<a rel=\"noopener\" target=\"_blank\">x</a>",
        ),
        (
            "<a target=\"_blank\" data-x=\"1\">x</a>",
            "<a data-x=\"1\" rel=\"noopener\" target=\"_blank\">x</a>",
        ),
        ("<a target=\"x-->y\">t</a>", "<a>t</a>"),
        ("<a target=\"</style>\">t</a>", "<a>t</a>"),
        (
            "<a target=\"_blank\">unclosed",
            "<a rel=\"noopener\" target=\"_blank\">unclosed</a>",
        ),
        (
            "<a target=\"_blank\" rel=\"\">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" aria-label=\"q\">t</a>",
            "<a aria-label=\"q\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" href=\"x\" rel=\"noopener\">t</a>",
            "<a href=\"x\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" style=\"color:red\">t</a>",
            "<a style=\"color:red\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" href=\"x\">a</a> and <a target=\"_self\" href=\"y\">b</a>",
            "<a href=\"x\" rel=\"noopener\" target=\"_blank\">a</a> and <a href=\"y\" target=\"_self\">b</a>",
        ),
        (
            "<a target=\"_blank \">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=_blank>t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        ("<a target>t</a>", "<a target=\"\">t</a>"),
        ("<a target=\" \">t</a>", "<a target=\"\">t</a>"),
        ("<math><a target=\"_blank\">x</a></math>", "<math></math>"),
        (
            "<a target=\"_blank\"><svg><a target=\"_blank\">x</a></svg></a>",
            "<a rel=\"noopener\" target=\"_blank\"><svg><a>x</a></svg></a>",
        ),
        (
            "<a target=\"_blank\" href=\"x\" rel>t</a>",
            "<a href=\"x\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" REL=\"x\" href=\"x\">t</a>",
            "<a rel=\"noopener\" href=\"x\" target=\"_blank\">t</a>",
        ),
        (
            "<a rel=\"javascript:x\" target=\"_blank\">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a data-temp-href-target=\"zz\">x</a>",
            "<a target=\"zz\">x</a>",
        ),
        (
            "<a data-temp-href-target=\"_blank\">x</a>",
            "<a rel=\"noopener\" target=\"_blank\">x</a>",
        ),
        (
            "<a target=\"_blank\" data-temp-href-target=\"zz\">x</a>",
            "<a rel=\"noopener\" target=\"_blank\">x</a>",
        ),
        (
            "<a DATA-TEMP-HREF-TARGET=\"_blank\">x</a>",
            "<a rel=\"noopener\" target=\"_blank\">x</a>",
        ),
        (
            "<b data-temp-href-target=\"_blank\">x</b>",
            "<b data-temp-href-target=\"_blank\">x</b>",
        ),
        (
            "<a target=\"_blank\" href=\"  https://x  \">t</a>",
            "<a href=\"https://x\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\"><img src=\"x.png\"></a>",
            "<a rel=\"noopener\" target=\"_blank\"><img src=\"x.png\"></a>",
        ),
        (
            "<a target=\"_blank\" id=\"i\" name=\"n\">t</a>",
            "<a id=\"i\" name=\"n\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" href=\"x\" target=\"\">t</a>",
            "<a href=\"x\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" value=\" v \">t</a>",
            "<a value=\" v \" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\"></a>",
            "<a rel=\"noopener\" target=\"_blank\"></a>",
        ),
        (
            "<a target=\"_blank\" style=\"x\" href=\"javascript:1\" class=\"c\" target=\"_self\">t</a>",
            "<a style=\"x\" class=\"c\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        ("x<style>p{color:red}</style>y", "xy"),
        (
            "x<style>p{color:red}</style>y <a target=\"_blank\">z</a>",
            "xy <a rel=\"noopener\" target=\"_blank\">z</a>",
        ),
        (
            "<a target=\"_blank\"><style>q</style></a>",
            "<a rel=\"noopener\" target=\"_blank\"></a>",
        ),
        (
            "<svg><style>x</style><a target=\"_blank\">t</a></svg>",
            "<svg><a>t</a></svg>",
        ),
        (
            "<a rel=\"x\" target=\"_blank\" href=\"javascript:1\" class=\"c\">t</a>",
            "<a rel=\"noopener\" class=\"c\" target=\"_blank\">t</a>",
        ),
        (
            "<a data-temp-href-target=\"a&amp;b\">x</a>",
            "<a target=\"a&amp;b\">x</a>",
        ),
        (
            "<a target=\"_blank\" aria-label=\"x\" data-x=\"1\" title=\"t\">t</a>",
            "<a aria-label=\"x\" data-x=\"1\" title=\"t\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<b><a target=\"_blank\">t</a></b>",
            "<b><a rel=\"noopener\" target=\"_blank\">t</a></b>",
        ),
        (
            "<a target=\"_blank\" rel=\"a b\">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"  _blank  \" rel=\"nofollow\">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a data-temp-href-target=\"_blank\" target=\"_self\">x</a>",
            "<a target=\"_self\">x</a>",
        ),
        (
            "<a data-temp-href-target=\" _blank \">x</a>",
            "<a rel=\"noopener\" target=\"_blank\">x</a>",
        ),
        (
            "<a target=\"_blank\" data-temp-href-target=\"x-->y\">x</a>",
            "<a rel=\"noopener\" target=\"_blank\">x</a>",
        ),
        ("<a data-temp-href-target=\"x-->y\">x</a>", "<a>x</a>"),
        ("<a target=\"x&lt;y\">t</a>", "<a target=\"x<y\">t</a>"),
        (
            "<a target=\"&nbsp;_blank\">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank&nbsp;\">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\n\">t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target='_blank'>t</a>",
            "<a rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" href=\"x\"><img src=\"y.png\" alt=\"i\"></a> after",
            "<a href=\"x\" rel=\"noopener\" target=\"_blank\"><img src=\"y.png\" alt=\"i\"></a> after",
        ),
        (
            "text <a target=\"_blank\" href=\"https://e.x/?a=1&amp;b=2\">link</a> more",
            "text <a href=\"https://e.x/?a=1&amp;b=2\" rel=\"noopener\" target=\"_blank\">link</a> more",
        ),
        (
            "<a target=\"_blank\" href=\"x\" rel=\"noopener\"><b>bold</b></a>",
            "<a href=\"x\" rel=\"noopener\" target=\"_blank\"><b>bold</b></a>",
        ),
        (
            "<a href=\"x\"></a><a target=\"_blank\" class=\"k\"></a>",
            "<a href=\"x\"></a><a class=\"k\" rel=\"noopener\" target=\"_blank\"></a>",
        ),
        (
            "<a target=\"_blank\" class=\"a\"/>x",
            "<a class=\"a\" rel=\"noopener\" target=\"_blank\">x</a>",
        ),
        (
            "<a target=\"_blank\" title=\"a'b\">t</a>",
            "<a title=\"a'b\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" data-foo=\"x\" style=\"c\" href=\"https://q\" id=\"z\">t</a>",
            "<a data-foo=\"x\" style=\"c\" href=\"https://q\" id=\"z\" rel=\"noopener\" target=\"_blank\">t</a>",
        ),
        (
            "<a target=\"_blank\" rel=\"noopener\" data-x=\"1\">t</a>",
            "<a rel=\"noopener\" data-x=\"1\" target=\"_blank\">t</a>",
        ),
        ("<a target=\"_BLANK \">t</a>", "<a target=\"_BLANK\">t</a>"),
        ("<a target=\"blank\">t</a>", "<a target=\"blank\">t</a>"),
    ];

    #[test]
    fn the_subset_matches_dompurify_on_the_corpus() {
        for (input, expected) in DOMPURIFY_CORPUS {
            assert_eq!(sanitize_text(input), *expected, "input {input:?}");
        }
    }

    /// A `<` that starts no tag is text, and DOMPurify writes it as `&lt;`: the `<11` label of
    /// `nightly-check/v2/03-should-render-raw-strings`, which the reference draws as `<11`.
    #[test]
    fn the_anchor_hooks_match_the_reference_on_the_hooked_corpus() {
        for (input, expected) in HOOKED_CORPUS {
            assert_eq!(sanitize_text(input), *expected, "input {input:?}");
        }
    }

    /// The hooks append `target`, then `rel` for `_blank`, to a first pass; the second pass finds
    /// `rel` before `target` and sets it in place, which is the order the reference gives.
    #[test]
    fn a_blank_target_is_written_after_the_attributes_with_rel_noopener_before_it() {
        assert_eq!(
            sanitize_text("<a target=\"_blank\" href=\"x\">t</a>"),
            "<a href=\"x\" rel=\"noopener\" target=\"_blank\">t</a>"
        );
        assert_eq!(
            sanitize_text("<a target=\"_top\" href=\"x\">t</a>"),
            "<a href=\"x\" target=\"_top\">t</a>"
        );
    }

    /// A `target` whose value DOMPurify refuses leaves the temporary attribute out, so the hooks set
    /// neither `target` nor `rel`.
    #[test]
    fn a_target_that_dompurify_refuses_is_lost_with_its_rel() {
        assert_eq!(sanitize_text("<a target=\"x-->y\">t</a>"), "<a>t</a>");
    }

    /// `common.spec.ts`, `should detect unsecured target attribute, if value is _blank then generate
    /// a secured link`: `removeScript` is the first pass alone, which appends `target`, then `rel`.
    /// The strict `sanitizeText` runs the second pass on that, and `rel` comes first.
    #[test]
    fn the_first_pass_alone_appends_target_and_then_rel() {
        assert_eq!(
            dompurify(
                "<a href=\"https://mermaid.js.org/\" target=\"_blank\">note about mermaid</a>",
                false
            ),
            "<a href=\"https://mermaid.js.org/\" target=\"_blank\" rel=\"noopener\">note about mermaid</a>"
        );
        assert_eq!(
            sanitize_text(
                "<a href=\"https://mermaid.js.org/\" target=\"_blank\">note about mermaid</a>"
            ),
            "<a href=\"https://mermaid.js.org/\" rel=\"noopener\" target=\"_blank\">note about mermaid</a>"
        );
    }

    #[test]
    fn a_less_than_that_starts_no_tag_is_written_as_an_entity() {
        assert_eq!(sanitize_text("<11"), "&lt;11");
        assert_eq!(sanitize_text("<**1**1"), "&lt;**1**1");
        assert_eq!(sanitize_text("a <= b <b>x</b>"), "a &lt;= b <b>x</b>");
    }

    /// `visual-check/5b-angle-brackets`: the `>`, `&` and `<` of the text are written as entities,
    /// and a plain-text label then decodes them.
    #[test]
    fn greater_than_ampersand_and_less_than_in_text_are_written_as_entities() {
        assert_eq!(
            sanitize_text("5 > 3 && 2 < 4 <b>x</b>"),
            "5 &gt; 3 &amp;&amp; 2 &lt; 4 <b>x</b>"
        );
    }

    /// Without a `<`, DOMPurify returns the text as it is, so `>` and `&` stay as written.
    #[test]
    fn text_without_a_less_than_is_returned_as_it_is() {
        assert_eq!(sanitize_text("5 > 3 && 2"), "5 > 3 && 2");
        assert_eq!(sanitize_text(""), "");
    }

    /// `visual-check/4023-should-render-html-labels-with-images`: the `<img>` of a label is kept,
    /// with its attributes in double quotes, so its text is one with `"` and not `'`.
    #[test]
    fn an_image_keeps_its_source_in_double_quotes() {
        assert_eq!(
            sanitize_text("<img src='https://mermaid.js.org/mermaid-logo.svg'> some text"),
            "<img src=\"https://mermaid.js.org/mermaid-logo.svg\"> some text"
        );
    }

    #[test]
    fn an_unknown_element_loses_its_tags_and_keeps_its_content() {
        assert_eq!(
            sanitize_text("<foo>unknown <i>kept</i></foo>"),
            "unknown <i>kept</i>"
        );
    }

    #[test]
    fn script_and_style_are_removed_with_their_content() {
        assert_eq!(sanitize_text("<script>alert(1)</script>x"), "x");
        assert_eq!(sanitize_text("<style>p{color:red}</style>y"), "y");
    }

    /// `circle` is an SVG element, so an HTML `circle` is removed with its content, as
    /// `_checkValidNamespace` does, while an `<svg>` keeps its own.
    #[test]
    fn an_svg_element_outside_svg_is_removed_with_its_content() {
        assert_eq!(sanitize_text("<circle r=\"5\"></circle>"), "");
        assert_eq!(
            sanitize_text("<svg><circle r=\"5\"/></svg>"),
            "<svg><circle r=\"5\"></circle></svg>"
        );
    }

    #[test]
    fn a_link_to_a_script_loses_its_href_and_an_event_handler_is_removed() {
        assert_eq!(
            sanitize_text("<a href=\"javascript:alert(1)\">link</a>"),
            "<a>link</a>"
        );
        assert_eq!(
            sanitize_text("<img src=\"x.png\" onerror=\"alert(1)\">"),
            "<img src=\"x.png\">"
        );
    }

    #[test]
    fn data_and_aria_attributes_are_kept_and_values_are_trimmed() {
        assert_eq!(
            sanitize_text("<span data-x=\"1\" aria-label=\"q\">s</span>"),
            "<span data-x=\"1\" aria-label=\"q\">s</span>"
        );
        assert_eq!(
            sanitize_text("<img alt='  spaced  '>"),
            "<img alt=\"spaced\">"
        );
    }

    #[test]
    fn comments_declarations_and_processing_instructions_are_removed() {
        assert_eq!(sanitize_text("x<!-- comment -->y"), "xy");
        assert_eq!(sanitize_text("<!DOCTYPE html><b>x</b>"), "<b>x</b>");
        assert_eq!(sanitize_text("<?php echo 1 ?>x"), "x");
    }

    /// A head element before the text is put in `head` by the parser, so it is dropped.
    #[test]
    fn a_head_element_before_the_text_is_dropped_and_the_leading_whitespace_is_kept() {
        assert_eq!(sanitize_text("<title>t</title>x"), "x");
        assert_eq!(
            sanitize_text("  <title>head</title> <b>t</b>"),
            "  <b>t</b>"
        );
    }

    #[test]
    fn character_references_are_decoded_and_written_back_as_their_characters() {
        assert_eq!(
            sanitize_text("&#39;quoted&#39; <b>x</b>"),
            "'quoted' <b>x</b>"
        );
        assert_eq!(sanitize_text("&copy; 2024 <b>x</b>"), "© 2024 <b>x</b>");
        assert_eq!(sanitize_text("<b>&#x80;&#0;</b>"), "<b>€�</b>");
        assert_eq!(sanitize_text("AT&T <b>x</b>"), "AT&amp;T <b>x</b>");
    }

    /// The full HTML5 table decodes the names the subset did not have, including one with two code
    /// points, and writes each back as its characters.
    #[test]
    fn names_of_the_full_html5_table_are_decoded() {
        assert_eq!(sanitize_text("<i>a &hearts; b</i>"), "<i>a \u{2665} b</i>");
        assert_eq!(
            sanitize_text("<i>&NotEqualTilde;</i>"),
            "<i>\u{2242}\u{338}</i>"
        );
        assert_eq!(
            sanitize_text("<i>&rarr;&larr;</i>"),
            "<i>\u{2192}\u{2190}</i>"
        );
    }

    /// Without its `;` a name is decoded only when it is one of the legacy names a browser reads
    /// that way; `&hearts` stays as written, and so does a name the table does not have.
    #[test]
    fn a_name_without_its_semicolon_is_decoded_only_when_it_is_legacy() {
        assert_eq!(
            sanitize_text("<i>&eacute x &copy y &hearts z &nope w</i>"),
            "<i>\u{e9} x \u{a9} y &amp;hearts z &amp;nope w</i>"
        );
    }

    /// A `&nbsp;` is written back as an entity, and the plain-text label then shows it as text.
    #[test]
    fn a_no_break_space_is_written_as_an_entity() {
        assert_eq!(sanitize_text("<b>\u{a0}x</b>"), "<b>&nbsp;x</b>");
    }

    #[test]
    fn a_line_break_tag_is_kept_and_a_closing_br_is_read_as_one() {
        assert_eq!(sanitize_text("a</br>b"), "a<br>b");
        assert_eq!(sanitize_text("a<br/>b"), "a<br>b");
    }

    /// `sanitizeText` with the gitGraph section, checked against DOMPurify 3.4.12 with jsdom and the
    /// escape branch of `sanitizeMore` (`packages/mermaid/src/diagrams/common/common.ts`), on the
    /// inputs `docs-port/gitgraph/NOTES.md` section 2.6 lists.
    #[test]
    fn the_section_escapes_angle_brackets_and_equals_before_dompurify() {
        use super::sanitize_section;
        assert_eq!(sanitize_section("AT&T"), "AT&T");
        assert_eq!(sanitize_section("<b>x</b>"), "&lt;b&gt;x&lt;/b&gt;");
        assert_eq!(sanitize_section("a=b"), "a&equals;b");
        assert_eq!(sanitize_section("a<br>b"), "a<br>b");
        assert_eq!(sanitize_section("a<br/>b"), "a<br>b");
        assert_eq!(sanitize_section("a <br /> b"), "a <br> b");
        assert_eq!(sanitize_section("<BR>x"), "<br>x");
        assert_eq!(sanitize_section("x<y"), "x&lt;y");
        assert_eq!(sanitize_section("<tag>"), "&lt;tag&gt;");
        assert_eq!(sanitize_section("a&b<c>"), "a&b&lt;c&gt;");
        assert_eq!(sanitize_section("#br#"), "<br>");
        assert_eq!(
            sanitize_section("<script>x</script>"),
            "&lt;script&gt;x&lt;/script&gt;"
        );
        assert_eq!(
            sanitize_section("a&lt;b<i>q</i>"),
            "a&lt;b&lt;i&gt;q&lt;/i&gt;"
        );
        assert_eq!(sanitize_section("a<"), "a&lt;");
        assert_eq!(sanitize_section(""), "");
    }

    /// The title path: `removeScript` and then the `style` rule, which give the same text as one
    /// DOMPurify pass on these inputs (checked as above).
    #[test]
    fn the_strict_path_drops_an_unfinished_tag_and_keeps_equals() {
        assert_eq!(sanitize_text("a<b"), "a");
        assert_eq!(sanitize_text("x<y"), "x");
        assert_eq!(sanitize_text("a=b &amp; c"), "a=b &amp; c");
        assert_eq!(sanitize_text("<tag>"), "");
        assert_eq!(sanitize_text("<b>a"), "<b>a</b>");
    }
}
