//! What the tree view tests share: the render of a text with the redux colour theme and its
//! configuration read the way the facade reads it, a small reader of the SVG the render writes, and
//! the readings of its rows (the label, the icon, the description, the highlight).
//!
//! The reader covers the markup the renderers write: elements with attributes, text with the
//! character references the writer escapes, and the `<style>` element as raw text.

#![allow(
    dead_code,
    reason = "each test crate uses the part of the support it needs"
)]

use ferrigura_model::config::{TreeViewDiagramConfig, effective, typed, user_theme_variables};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::tree_view::render;

/// The id of the rendered SVG in the tests.
pub const ID: &str = "tree-view-test";

/// Renders a tree view text with the redux colour theme, its configuration read the way the facade
/// reads it: the front matter and the directives of the text, on top of `site` (what `initialize`
/// gets).
///
/// # Errors
///
/// Returns the render's error, when the render fails.
pub fn draw_configured(text: &str, site: Option<&Value>) -> Result<String, RenderError> {
    let directive = ferrigura_syntax::config::read(text).expect("the directives read");
    let configured = effective(site, Some("treeView"), &directive);
    let config = typed(&configured).expect("the configuration is typed");
    let diagram = ferrigura_syntax::tree_view::parse(text).expect("the tree parses");
    let overrides = user_theme_variables(site, &directive);
    let variables = Theme::ReduxColor.variables(Some(&overrides));
    let tokens = ThemeTokens::from_value(&variables);
    let globals = Globals::of(&config);
    render(&Context {
        diagram: &diagram,
        section: &config.tree_view,
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: ID,
    })
}

/// Renders a tree view text with a section of the configuration that the test builds, and the redux
/// colour theme. The globals are the defaults.
///
/// # Errors
///
/// Returns the render's error, when the render fails.
pub fn draw_section(text: &str, section: &TreeViewDiagramConfig) -> Result<String, RenderError> {
    let diagram = ferrigura_syntax::tree_view::parse(text).expect("the tree parses");
    let config = typed(&effective(None, Some("treeView"), &Value::empty_object()))
        .expect("the configuration is typed");
    let globals = Globals::of(&config);
    let tokens = ThemeTokens::from_value(&Theme::ReduxColor.variables(None));
    render(&Context {
        diagram: &diagram,
        section,
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: ID,
    })
}

/// Renders a tree view text with no site configuration.
///
/// # Errors
///
/// Returns the render's error, when the render fails.
pub fn draw(text: &str) -> Result<String, RenderError> {
    draw_configured(text, None)
}

/// Renders a text that must render, and reads its SVG.
///
/// # Panics
///
/// Panics when the render fails.
#[must_use]
pub fn svg_of(text: &str) -> Element {
    parse(&draw(text).expect("the tree view renders"))
}

/// Renders a text with a site configuration that must render, and reads its SVG.
///
/// # Panics
///
/// Panics when the render fails.
#[must_use]
pub fn svg_with(text: &str, site: &Value) -> Element {
    parse(&draw_configured(text, Some(site)).expect("the tree view renders"))
}

/// An element of the SVG that the reader built.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub tag: String,
    pub attributes: Vec<(String, String)>,
    /// The text the element holds itself. [`Element::text_content`] adds the text below it.
    pub text: String,
    pub children: Vec<Element>,
}

impl Element {
    fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_owned(),
            attributes: Vec::new(),
            text: String::new(),
            children: Vec::new(),
        }
    }

    /// The value of an attribute.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// The value of an attribute as a number.
    ///
    /// # Panics
    ///
    /// Panics when the attribute is missing or not a number.
    #[must_use]
    pub fn number(&self, name: &str) -> f64 {
        self.attr(name)
            .unwrap_or_else(|| panic!("<{}> has no {name}", self.tag))
            .parse()
            .unwrap_or_else(|_| panic!("{name} of <{}> is not a number", self.tag))
    }

    /// Whether the element has the class.
    #[must_use]
    pub fn has_class(&self, class: &str) -> bool {
        self.attr("class")
            .is_some_and(|classes| classes.split_whitespace().any(|name| name == class))
    }

    /// The first element below this one, in document order, with the tag and the class.
    #[must_use]
    pub fn find(&self, tag: &str, class: &str) -> Option<&Element> {
        self.children.iter().find_map(|child| {
            if child.tag == tag && child.has_class(class) {
                Some(child)
            } else {
                child.find(tag, class)
            }
        })
    }

    /// The text of the element and of the elements below it, in document order (`textContent`).
    #[must_use]
    pub fn text_content(&self) -> String {
        let mut text = self.text.clone();
        for child in &self.children {
            text.push_str(&child.text_content());
        }
        text
    }

    /// The `<style>` text of the root.
    #[must_use]
    pub fn style(&self) -> String {
        self.children
            .iter()
            .find(|child| child.tag == "style")
            .map(|child| child.text.clone())
            .unwrap_or_default()
    }

    /// The `<g class="tree-view">` of the root.
    #[must_use]
    pub fn tree(&self) -> &Element {
        self.find("g", "tree-view").expect("the tree view group")
    }
}

/// The icon a row shows, as the markup of the icon tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Folder,
    File,
    Unknown,
}

/// A row of the tree: its group's label, its icon and its description, and the highlight rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub label: Element,
    pub icon: Option<IconKind>,
    pub icon_group: Option<Element>,
    pub description: Option<Element>,
    pub highlight: Option<Element>,
}

/// The rows of the tree, in document order.
#[must_use]
pub fn rows(svg: &Element) -> Vec<Row> {
    svg.tree()
        .children
        .iter()
        .filter(|child| child.tag == "g" && child.attr("class").is_none())
        .map(|group| {
            let label = group
                .children
                .iter()
                .find(|child| child.tag == "text" && child.has_class("treeView-node-label"))
                .expect("every row has a label")
                .clone();
            let icon_group = group
                .children
                .iter()
                .find(|child| child.tag == "g" && child.has_class("treeView-node-icon"))
                .cloned();
            Row {
                label,
                icon: icon_group.as_ref().map(icon_kind),
                icon_group,
                description: group
                    .children
                    .iter()
                    .find(|child| {
                        child.tag == "text" && child.has_class("treeView-node-description")
                    })
                    .cloned(),
                highlight: group
                    .children
                    .iter()
                    .find(|child| child.tag == "rect" && child.has_class("treeView-highlight-bg"))
                    .cloned(),
            }
        })
        .collect()
}

/// The row of a label with the text, which must be one row of the tree.
///
/// # Panics
///
/// Panics when no row or more than one row has the text as its label.
#[must_use]
pub fn row_named(svg: &Element, name: &str) -> Row {
    let mut found = rows(svg)
        .into_iter()
        .filter(|row| row.label.text == name)
        .collect::<Vec<_>>();
    assert_eq!(found.len(), 1, "the rows labelled {name:?}");
    found.remove(0)
}

/// The lines of the tree: the connectors and the trunks, in document order.
#[must_use]
pub fn lines(svg: &Element) -> Vec<Element> {
    svg.tree()
        .children
        .iter()
        .filter(|child| child.tag == "line")
        .cloned()
        .collect()
}

/// The kind of the icon in an icon group, read from its markup.
///
/// # Panics
///
/// Panics when the markup is none of the three icons.
#[must_use]
pub fn icon_kind(group: &Element) -> IconKind {
    let svg = group.children.first().expect("an icon holds an svg");
    let path = svg.children.iter().find(|child| child.tag == "path");
    match path.and_then(|path| path.attr("d")) {
        Some(d) if d.starts_with("M10.59 4.59") => IconKind::Folder,
        Some(d) if d.starts_with("M6 2a2") => IconKind::File,
        Some(d) => panic!("an unknown built-in path {d:?}"),
        None => {
            assert!(
                svg.text_content().contains('?'),
                "the unknown icon draws its question mark"
            );
            IconKind::Unknown
        }
    }
}

/// The reader of the SVG: an element tree, with the text and the entities decoded.
#[must_use]
pub fn parse(svg: &str) -> Element {
    let bytes = svg.as_bytes();
    let mut stack = vec![Element::new("#document")];
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'<' {
            if svg[index..].starts_with("</") {
                let end = index + svg[index..].find('>').expect("a closing tag ends");
                let closing = svg[index + 2..end].trim().to_owned();
                index = end + 1;
                let top = stack.pop().expect("a tag is open");
                assert_eq!(top.tag, closing, "the closing tag matches");
                stack.last_mut().expect("a parent").children.push(top);
            } else {
                let (element, self_closing, next) = read_start_tag(svg, index);
                index = next;
                if self_closing {
                    stack.last_mut().expect("a parent").children.push(element);
                } else if element.tag == "style" {
                    let end = index + svg[index..].find("</style>").expect("the style ends");
                    let mut style = element;
                    style.text = svg[index..end].to_owned();
                    index = end + "</style>".len();
                    stack.last_mut().expect("a parent").children.push(style);
                } else {
                    stack.push(element);
                }
            }
        } else {
            let end = svg[index..]
                .find('<')
                .map_or(bytes.len(), |offset| index + offset);
            let text = decode(&svg[index..end]);
            if !text.trim().is_empty() {
                stack.last_mut().expect("a parent").text.push_str(&text);
            }
            index = end;
        }
    }
    let document = stack.pop().expect("the document");
    let mut root = document.children;
    assert_eq!(root.len(), 1, "one root element");
    root.remove(0)
}

/// The start tag at `start` (a `<`): its element with the attributes, whether it is self-closing,
/// and the index after it.
fn read_start_tag(svg: &str, start: usize) -> (Element, bool, usize) {
    let bytes = svg.as_bytes();
    let mut index = start + 1;
    while bytes[index] == b' ' {
        index += 1;
    }
    let name_start = index;
    while !matches!(bytes[index], b' ' | b'>' | b'/' | b'\n' | b'\t') {
        index += 1;
    }
    let mut element = Element::new(&svg[name_start..index]);
    loop {
        while matches!(bytes[index], b' ' | b'\n' | b'\t') {
            index += 1;
        }
        if bytes[index] == b'>' {
            return (element, false, index + 1);
        }
        if svg[index..].starts_with("/>") {
            return (element, true, index + 2);
        }
        let name_start = index;
        while bytes[index] != b'=' {
            index += 1;
        }
        let name = svg[name_start..index].to_owned();
        index += 2; // `="`
        let value_start = index;
        while bytes[index] != b'"' {
            index += 1;
        }
        element
            .attributes
            .push((name, decode(&svg[value_start..index])));
        index += 1;
    }
}

/// The text with the character references the writer uses decoded.
fn decode(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}
