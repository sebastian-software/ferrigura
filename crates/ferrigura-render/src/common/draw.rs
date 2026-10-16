//! The drawing helpers of `packages/mermaid/src/diagrams/common/svgDrawCommon.ts`: `drawRect`,
//! `drawBackgroundRect`, `drawText`, `drawImage`, `drawEmbeddedImage`, `getNoteRect` and
//! `getTextObj`. Each returns the element it writes, with the attributes the reference sets, in the
//! order it sets them; a caller that appends it to a group places it where the reference does.
//!
//! Not ported: `createTooltip` appends a `div` to the document body, not to the SVG, and it is only
//! called by the parsers of flowchart, class, state and agentflow, which show the tooltip of a
//! click. Clicks are dropped (`docs-port/decisions/0001`), so the tooltip is not drawn.

use crate::svg::number;

use super::Element;
use super::text::split_line_breaks;
use super::url::sanitize_url;

/// `RectData` (`commonTypes.ts`): what `drawRect` writes. An absent value is not written, as the
/// reference's `undefined` is not, and an attribute that is falsy in JavaScript (an empty name, a
/// zero corner radius, an empty class) is not written either.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RectData {
    /// The left edge.
    pub x: f64,
    /// The top edge.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
    /// The fill colour, if set.
    pub fill: Option<String>,
    /// The stroke colour, if set.
    pub stroke: Option<String>,
    /// The `name` attribute, written when it is not empty.
    pub name: Option<String>,
    /// The horizontal corner radius, written when it is not zero.
    pub rx: Option<f64>,
    /// The vertical corner radius, written when it is not zero.
    pub ry: Option<f64>,
    /// Further attributes, in order. An attribute that is already set keeps its place.
    pub attrs: Vec<(String, String)>,
    /// The class, written when it is not empty.
    pub class: Option<String>,
}

/// `Bound` (`commonTypes.ts`): the corners of a background rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct Bound {
    /// The left edge.
    pub startx: f64,
    /// The right edge.
    pub stopx: f64,
    /// The top edge.
    pub starty: f64,
    /// The bottom edge.
    pub stopy: f64,
    /// The fill colour.
    pub fill: String,
    /// The stroke colour.
    pub stroke: String,
}

/// `TextData` (`commonTypes.ts`): what `drawText` writes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextData {
    /// The x of the text; the line is drawn at `x + 2 * textMargin`.
    pub x: f64,
    /// The y of the baseline.
    pub y: f64,
    /// The `text-anchor` of the text, if set and not empty.
    pub anchor: Option<String>,
    /// The text. Its `<br>` tags are drawn as spaces.
    pub text: String,
    /// The margin: twice it is added to `x` for the line.
    pub text_margin: f64,
    /// The class of the text, if set and not empty. The default is `legend`.
    pub class: Option<String>,
}

/// `TextObject` (`commonTypes.ts`): the defaults of a text that the caller then changes. The fields
/// are the reference's, including the `style` that `getTextObj` sets to `#666`.
#[derive(Debug, Clone, PartialEq)]
pub struct TextObject {
    /// The x of the text.
    pub x: f64,
    /// The y of the text.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
    /// The fill, if set.
    pub fill: Option<String>,
    /// The anchor, if set.
    pub anchor: Option<String>,
    /// The `text-anchor` of the text.
    pub text_anchor: String,
    /// The `style` the reference sets (a colour, `#666`).
    pub style: String,
    /// The text margin.
    pub text_margin: f64,
    /// The horizontal corner radius.
    pub rx: f64,
    /// The vertical corner radius.
    pub ry: f64,
    /// Whether the text is drawn with a `tspan`.
    pub tspan: bool,
    /// The vertical alignment, if set.
    pub valign: Option<String>,
}

/// `drawRect(element, rectData)`: a `<rect>` with the attributes the reference sets, in its order:
/// `x`, `y`, `fill`, `stroke`, `width`, `height`, then `name`, `rx`, `ry`, the further attributes
/// and `class`.
#[must_use]
pub fn draw_rect(rect: &RectData) -> Element {
    let mut element = Element::new("rect")
        .with("x", number(rect.x))
        .with("y", number(rect.y));
    if let Some(fill) = &rect.fill {
        element.set("fill", fill.as_str());
    }
    if let Some(stroke) = &rect.stroke {
        element.set("stroke", stroke.as_str());
    }
    element.set("width", number(rect.width));
    element.set("height", number(rect.height));
    if let Some(name) = non_empty(rect.name.as_deref()) {
        element.set("name", name);
    }
    if let Some(rx) = non_zero(rect.rx) {
        element.set("rx", number(rx));
    }
    if let Some(ry) = non_zero(rect.ry) {
        element.set("ry", number(ry));
    }
    for (name, value) in &rect.attrs {
        element.set(name, value.as_str());
    }
    if let Some(class) = non_empty(rect.class.as_deref()) {
        element.set("class", class);
    }
    element
}

/// `drawBackgroundRect(element, bounds)`: a `rect` with class `rect`. The reference moves it to be
/// the first child of its group (`lower()`), so the caller inserts it before the other children.
#[must_use]
pub fn draw_background_rect(bounds: &Bound) -> Element {
    draw_rect(&RectData {
        x: bounds.startx,
        y: bounds.starty,
        width: bounds.stopx - bounds.startx,
        height: bounds.stopy - bounds.starty,
        fill: Some(bounds.fill.clone()),
        stroke: Some(bounds.stroke.clone()),
        class: Some("rect".to_owned()),
        ..RectData::default()
    })
}

/// `drawText(element, textData)`: a `<text>` with class `legend` (or the given class), its
/// `text-anchor` as a style when it is set, and one `<tspan>` at `x + 2 * textMargin`. The `<br>`
/// tags of the text are drawn as spaces.
#[must_use]
pub fn draw_text(data: &TextData) -> Element {
    let text = split_line_breaks(&data.text).join(" ");
    let mut element = Element::new("text")
        .with("x", number(data.x))
        .with("y", number(data.y))
        .with("class", "legend");
    if let Some(anchor) = non_empty(data.anchor.as_deref()) {
        element.set("style", format!("text-anchor: {anchor};"));
    }
    if let Some(class) = non_empty(data.class.as_deref()) {
        element.set("class", class);
    }
    element.child(
        Element::new("tspan")
            .with("x", number(data.x + data.text_margin * 2.0))
            .text(text),
    )
}

/// `drawImage(elem, x, y, link)`: an `<image>` whose `xlink:href` is the sanitised link.
#[must_use]
pub fn draw_image(x: f64, y: f64, link: &str) -> Element {
    Element::new("image")
        .with("x", number(x))
        .with("y", number(y))
        .with("xlink:href", sanitize_url(link))
}

/// `drawEmbeddedImage(element, x, y, link)`: a `<use>` of the element with the id `link`, which is
/// sanitised and written as `#` followed by the result.
#[must_use]
pub fn draw_embedded_image(x: f64, y: f64, link: &str) -> Element {
    Element::new("use")
        .with("x", number(x))
        .with("y", number(y))
        .with("xlink:href", format!("#{}", sanitize_url(link)))
}

/// `getNoteRect()`: the rectangle of a note, at the origin, 100 by 100, in the note colours. The
/// reference also carries an `anchor` of `start`, which `drawRect` does not read, so it is not kept.
#[must_use]
pub fn get_note_rect() -> RectData {
    RectData {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
        fill: Some("#EDF2AE".to_owned()),
        stroke: Some("#666".to_owned()),
        name: None,
        rx: Some(0.0),
        ry: Some(0.0),
        attrs: Vec::new(),
        class: None,
    }
}

/// `getTextObj()`: the defaults of a text object (see [`TextObject`]).
#[must_use]
pub fn get_text_obj() -> TextObject {
    TextObject {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
        fill: None,
        anchor: None,
        text_anchor: "start".to_owned(),
        style: "#666".to_owned(),
        text_margin: 0.0,
        rx: 0.0,
        ry: 0.0,
        tspan: true,
        valign: None,
    }
}

/// The text of an optional value, when it is not empty (JavaScript's truthiness for a string).
fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|text| !text.is_empty())
}

/// The number of an optional value, when it is not zero and not NaN (JavaScript's truthiness).
fn non_zero(value: Option<f64>) -> Option<f64> {
    value.filter(|number| *number != 0.0 && !number.is_nan())
}

#[cfg(test)]
mod tests {
    use super::*;

    // The expected markup is the reference's, as the probe (`drawing` in the fixture) read it in
    // Chromium. The probe writes empty elements as `<x/>`, as this module does.

    #[test]
    fn a_plain_rect_has_its_position_colours_and_size() {
        let rect = RectData {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
            fill: Some("#fff".to_owned()),
            stroke: Some("#000".to_owned()),
            ..RectData::default()
        };
        assert_eq!(
            draw_rect(&rect).render(),
            "<rect x=\"10\" y=\"20\" fill=\"#fff\" stroke=\"#000\" width=\"100\" height=\"50\"/>"
        );
    }

    #[test]
    fn every_optional_attribute_keeps_the_reference_order_and_an_attribute_is_replaced_in_place() {
        let rect = RectData {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: Some("red".to_owned()),
            stroke: Some("none".to_owned()),
            name: Some("n1".to_owned()),
            rx: Some(5.0),
            ry: Some(5.0),
            attrs: vec![
                ("data-x".to_owned(), "1".to_owned()),
                ("fill".to_owned(), "blue".to_owned()),
            ],
            class: Some("rect".to_owned()),
        };
        assert_eq!(
            draw_rect(&rect).render(),
            "<rect x=\"0\" y=\"0\" fill=\"blue\" stroke=\"none\" width=\"10\" height=\"10\" name=\"n1\" rx=\"5\" ry=\"5\" data-x=\"1\" class=\"rect\"/>"
        );
    }

    #[test]
    fn zero_radii_and_absent_colours_are_not_written() {
        let rect = RectData {
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
            fill: Some("none".to_owned()),
            stroke: Some("none".to_owned()),
            rx: Some(0.0),
            ry: Some(0.0),
            ..RectData::default()
        };
        assert_eq!(
            draw_rect(&rect).render(),
            "<rect x=\"1\" y=\"2\" fill=\"none\" stroke=\"none\" width=\"3\" height=\"4\"/>"
        );
        let bare = RectData {
            x: 1.5,
            y: -2.0,
            width: 0.1,
            height: 2.0,
            ..RectData::default()
        };
        assert_eq!(
            draw_rect(&bare).render(),
            "<rect x=\"1.5\" y=\"-2\" width=\"0.1\" height=\"2\"/>"
        );
    }

    #[test]
    fn a_background_rect_takes_the_bounds_and_the_rect_class() {
        let bounds = Bound {
            startx: 5.0,
            stopx: 105.0,
            starty: 6.0,
            stopy: 56.0,
            fill: "#eee".to_owned(),
            stroke: "none".to_owned(),
        };
        assert_eq!(
            draw_background_rect(&bounds).render(),
            "<rect x=\"5\" y=\"6\" fill=\"#eee\" stroke=\"none\" width=\"100\" height=\"50\" class=\"rect\"/>"
        );
    }

    #[test]
    fn a_text_is_centred_by_its_anchor_and_its_breaks_are_spaces() {
        let text = TextData {
            x: 10.0,
            y: 20.0,
            anchor: Some("middle".to_owned()),
            text: "a<br/>b".to_owned(),
            text_margin: 5.0,
            class: None,
        };
        assert_eq!(
            draw_text(&text).render(),
            "<text x=\"10\" y=\"20\" class=\"legend\" style=\"text-anchor: middle;\"><tspan x=\"20\">a b</tspan></text>"
        );
    }

    #[test]
    fn a_text_takes_its_class_and_escapes_its_markup() {
        let text = TextData {
            x: 0.0,
            y: 0.0,
            anchor: Some("start".to_owned()),
            text: "Hi <b>there</b>".to_owned(),
            text_margin: 0.0,
            class: Some("legend-x".to_owned()),
        };
        assert_eq!(
            draw_text(&text).render(),
            "<text x=\"0\" y=\"0\" class=\"legend-x\" style=\"text-anchor: start;\"><tspan x=\"0\">Hi &lt;b&gt;there&lt;/b&gt;</tspan></text>"
        );
    }

    #[test]
    fn a_text_without_an_anchor_has_no_style() {
        let text = TextData {
            x: 3.0,
            y: 4.0,
            anchor: None,
            text: "x".to_owned(),
            text_margin: 2.0,
            class: None,
        };
        assert_eq!(
            draw_text(&text).render(),
            "<text x=\"3\" y=\"4\" class=\"legend\"><tspan x=\"7\">x</tspan></text>"
        );
    }

    #[test]
    fn an_image_has_its_link_sanitised() {
        assert_eq!(
            draw_image(1.0, 2.0, "https://example.com/a.png?x=1&y=2").render(),
            "<image x=\"1\" y=\"2\" xlink:href=\"https://example.com/a.png?x=1&amp;y=2\"/>"
        );
        assert_eq!(
            draw_image(1.0, 2.0, "javascript:alert(1)").render(),
            "<image x=\"1\" y=\"2\" xlink:href=\"about:blank\"/>"
        );
        assert_eq!(
            draw_image(1.0, 2.0, "pics/a b.png").render(),
            "<image x=\"1\" y=\"2\" xlink:href=\"pics/a b.png\"/>"
        );
    }

    #[test]
    fn an_embedded_image_refers_to_an_element_by_its_id() {
        assert_eq!(
            draw_embedded_image(3.0, 4.0, "my-id").render(),
            "<use x=\"3\" y=\"4\" xlink:href=\"#my-id\"/>"
        );
        assert_eq!(
            draw_embedded_image(3.0, 4.0, "javascript:alert(1)").render(),
            "<use x=\"3\" y=\"4\" xlink:href=\"#about:blank\"/>"
        );
    }

    #[test]
    fn the_note_rect_and_the_text_object_have_the_reference_defaults() {
        let note = get_note_rect();
        assert_eq!(
            (note.x, note.y, note.width, note.height),
            (0.0, 0.0, 100.0, 100.0)
        );
        assert_eq!(note.fill.as_deref(), Some("#EDF2AE"));
        assert_eq!(note.stroke.as_deref(), Some("#666"));
        let object = get_text_obj();
        assert_eq!(object.text_anchor, "start");
        assert_eq!(object.style, "#666");
        assert!(object.tspan);
    }
}
