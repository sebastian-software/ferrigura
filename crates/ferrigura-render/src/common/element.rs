//! An SVG element written the way the reference's d3 calls write it.
//!
//! Mermaid sets attributes with d3's `attr` and `style`: a name that is set again keeps the place it
//! had and takes the new value. Attribute order does not change the geometry, but the shared helpers
//! keep it, so the markup reads as the reference's does and the tests can compare it.

use crate::svg::escape;

/// What an element holds between its tags.
#[derive(Debug, Clone, PartialEq)]
enum Content {
    /// Nothing: the element is written with `/>`.
    Empty,
    /// Text, escaped on output.
    Text(String),
    /// Text the caller has escaped already, written as it is (a `<style>` element).
    Raw(String),
}

/// An SVG element: its tag, its attributes in the order they were set, its text and its children.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    tag: &'static str,
    attributes: Vec<(String, String)>,
    content: Content,
    children: Vec<Element>,
}

impl Element {
    /// A new element with no attributes, text or children.
    #[must_use]
    pub fn new(tag: &'static str) -> Self {
        Self {
            tag,
            attributes: Vec::new(),
            content: Content::Empty,
            children: Vec::new(),
        }
    }

    /// Sets an attribute, as a builder: see [`Element::set`].
    #[must_use]
    pub fn with(mut self, name: &str, value: impl Into<String>) -> Self {
        self.set(name, value);
        self
    }

    /// Sets an attribute. A name that is already set keeps its place and takes the new value.
    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        let value = value.into();
        match self.attributes.iter_mut().find(|(key, _)| key == name) {
            Some(entry) => entry.1 = value,
            None => self.attributes.push((name.to_owned(), value)),
        }
    }

    /// Removes an attribute, as d3 does when it is given `undefined`.
    pub fn remove(&mut self, name: &str) {
        self.attributes.retain(|(key, _)| key != name);
    }

    /// The value of an attribute, if it is set.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Sets the text of the element, escaped on output.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.content = Content::Text(text.into());
        self
    }

    /// Sets text that is written as it is. The caller escapes it for XML, as a `<style>` element
    /// takes its rules.
    #[must_use]
    pub fn raw_text(mut self, text: impl Into<String>) -> Self {
        self.content = Content::Raw(text.into());
        self
    }

    /// Appends a child element.
    #[must_use]
    pub fn child(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }

    /// The children of the element, in order.
    #[must_use]
    pub fn children(&self) -> &[Element] {
        &self.children
    }

    /// The markup of the element: the attributes in order, then the text and the children. An
    /// element with neither text nor children is written with `/>`.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = format!("<{}", self.tag);
        for (name, value) in &self.attributes {
            out.push_str(&format!(" {name}=\"{}\"", escape(value)));
        }
        if matches!(self.content, Content::Empty) && self.children.is_empty() {
            out.push_str("/>");
            return out;
        }
        out.push('>');
        match &self.content {
            Content::Empty => {}
            Content::Text(text) => out.push_str(&escape(text)),
            Content::Raw(text) => out.push_str(text),
        }
        for child in &self.children {
            out.push_str(&child.render());
        }
        out.push_str(&format!("</{}>", self.tag));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_set_again_keeps_its_place() {
        let element = Element::new("rect")
            .with("x", "1")
            .with("fill", "red")
            .with("x", "2")
            .with("y", "3");
        assert_eq!(element.render(), "<rect x=\"2\" fill=\"red\" y=\"3\"/>");
    }

    #[test]
    fn a_removed_attribute_is_not_written() {
        let mut element = Element::new("rect").with("fill", "red").with("x", "1");
        element.remove("fill");
        assert_eq!(element.render(), "<rect x=\"1\"/>");
    }

    #[test]
    fn text_is_escaped_and_raw_text_is_not() {
        let text = Element::new("text").text("a<b & c");
        assert_eq!(text.render(), "<text>a&lt;b &amp; c</text>");
        let style = Element::new("style").raw_text("a>b{}");
        assert_eq!(style.render(), "<style>a>b{}</style>");
    }

    #[test]
    fn children_follow_the_text() {
        let group = Element::new("g").child(Element::new("rect").with("x", "1"));
        assert_eq!(group.render(), "<g><rect x=\"1\"/></g>");
    }
}
