//! The kanban item: a port of `kanbanItem` (`packages/mermaid/src/rendering-util/rendering-elements/shapes/kanbanItem.ts`
//! 9-163), the shape the kanban renderer inserts for each item (`kanbanRenderer.ts` 65-84).
//!
//! The item is a rectangle with three labels: the title, the ticket on the left under it and the
//! assignee on the right. The title and the ticket are laid out start-anchored at the left edge of
//! the rectangle plus the padding, and the assignee ends at its right edge. The height is the
//! title's height plus 20, plus half the taller of the ticket and the assignee, so a long ticket
//! can take the group's box beyond the rectangle; the renderer places the next item from the box of
//! the whole group (`kanbanRenderer.ts` 81-84), so that box is what this shape returns.
//!
//! The group's class is `node undefined`: `getData` drops the classes of an item, and the class
//! string is `node` and the classes (`getNodeClasses`, `shapes/util.ts` 249-250). An item has no
//! `data-look`, as `insertNode` writes it from the look of the node, which the item does not have.

use ferrigura_model::value::Value;

use crate::svg::{escape, number};
use crate::unified::geom::BBox;
use crate::unified::label::LabelLayout;

/// `labelPaddingX`: the gap from the rectangle's right edge to the assignee (`kanbanItem.ts` 98-105).
const LABEL_PADDING_X: f64 = 10.0;

/// `labelPaddingY`: the rectangle is the title's height plus twice this (`kanbanItem.ts` 84-89).
const LABEL_PADDING_Y: f64 = 10.0;

/// The colours an item takes from the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPaint<'a> {
    /// The fill of the rectangle (`options.background`).
    pub background: &'a str,
    /// The stroke of the rectangle (`options.nodeBorder`).
    pub node_border: &'a str,
    /// The colour of the labels (`.label`, `textColor`).
    pub text: &'a str,
}

/// What an item draws.
#[derive(Debug, Clone, Copy)]
pub struct KanbanItem<'a> {
    /// The id of the item's group: `domId`, the diagram id and the item id.
    pub dom_id: &'a str,
    /// The title, laid out at the item's label width.
    pub title: &'a LabelLayout,
    /// The ticket, laid out at the item's label width; empty when the item has none.
    pub ticket: &'a LabelLayout,
    /// The assignee, laid out at the item's label width; empty when the item has none.
    pub assigned: &'a LabelLayout,
    /// The width of the rectangle (`node.width`, the section width less 1.5 times the padding).
    pub width: f64,
    /// The padding of the labels (`node.padding || 10`).
    pub padding: f64,
    /// The corner radii of the rectangle (`rx`, `ry`).
    pub corner_radius: f64,
    /// The priority from the data, when the item has one.
    pub priority: Option<&'a Value>,
    /// The style of the labels: `text-align: left` in the reference, as `!important`.
    pub label_style: &'a str,
    /// The link of the ticket, when the configuration gives a ticket base URL and the item has a
    /// ticket (`kanbanItem.ts` 46-53). The ticket's label goes inside the link.
    pub ticket_link: Option<&'a str>,
    /// The colours from the theme.
    pub paint: ItemPaint<'a>,
}

/// A drawn item: the box of its group, and the group's markup at the origin of the group.
#[derive(Debug, Clone, PartialEq)]
pub struct KanbanItemShape {
    /// The box of the group in its own coordinates: the rectangle, and the labels at their places.
    pub bbox: BBox,
    dom_id: String,
    inner: String,
}

impl KanbanItemShape {
    /// The item's `<g>`, placed at `(x, y)` as `positionNode` writes it.
    #[must_use]
    pub fn group(&self, x: f64, y: f64) -> String {
        format!(
            "<g class=\"node undefined\" id=\"{}\" transform=\"translate({}, {})\">{}</g>",
            escape(&self.dom_id),
            number(x),
            number(y),
            self.inner,
        )
    }
}

/// Draws an item (`kanbanItem`).
#[must_use]
pub fn kanban_item(item: &KanbanItem<'_>) -> KanbanItemShape {
    // The box of each label's group, as `getBBox` returns it (float32): the text, and the background
    // rectangle when the label has one (`labelHelper` gives the title one for an item with an icon).
    let title = label_group_box(item.title);
    let ticket = label_group_box(item.ticket);
    let assigned = label_group_box(item.assigned);

    let total_width = item.width;
    // `heightAdj` (`kanbanItem.ts` 84-86): half the taller of the ticket and the assignee.
    let height_adjust = ticket.height.max(assigned.height) / 2.0;
    // `max(bbox.height + 2 * labelPaddingY, node.height || 0) + heightAdj`: the reference's node has
    // no height before the shape sets it, so the maximum is the title's box.
    let total_height = (title.height + LABEL_PADDING_Y * 2.0) + height_adjust;
    let rect_x = -total_width / 2.0;
    let rect_y = -total_height / 2.0;

    let left = item.padding - total_width / 2.0;
    let title_at = (left, -height_adjust - title.height / 2.0);
    let ticket_at = (left, -height_adjust + title.height / 2.0);
    let assigned_at = (
        item.padding + total_width / 2.0 - assigned.width - 2.0 * LABEL_PADDING_X,
        -height_adjust + title.height / 2.0,
    );

    // The group's box: the rectangle, and each label's box moved by its transform. A label without
    // ink adds nothing (`getBBox` leaves it out).
    let mut bbox = BBox::new(rect_x, rect_y, total_width, total_height);
    for (label, at) in [
        (item.title, title_at),
        (item.ticket, ticket_at),
        (item.assigned, assigned_at),
    ] {
        let label_box = label_group_box(label);
        if !label_box.is_empty() {
            bbox = bbox.union(label_box.translate(at.0, at.1));
        }
    }

    // The ticket's label is the first label after the rectangle when it is linked: the link is
    // inserted before the title (`kanbanItem.ts` 46-53), and the ticket's label goes into it.
    let mut inner = String::new();
    inner.push_str(&rectangle(item, rect_x, rect_y, total_width, total_height));
    match item.ticket_link {
        Some(link) => {
            // `target` is not in the markup: the sanitiser of the reference's output drops it.
            inner.push_str(&format!(
                "<a class=\"kanban-ticket-link\" xlink:href=\"{}\">{}</a>",
                escape(link),
                label_group(item.ticket, ticket_at, item),
            ));
            inner.push_str(&label_group(item.title, title_at, item));
        }
        None => {
            inner.push_str(&label_group(item.title, title_at, item));
            inner.push_str(&label_group(item.ticket, ticket_at, item));
        }
    }
    inner.push_str(&label_group(item.assigned, assigned_at, item));
    if let Some(priority) = item.priority.filter(|priority| is_truthy(priority)) {
        inner.push_str(&priority_line(
            priority,
            rect_x,
            rect_y,
            total_height,
            item.corner_radius,
        ));
    }

    KanbanItemShape {
        bbox,
        dom_id: item.dom_id.to_owned(),
        inner,
    }
}

/// The box `getBBox` gives a label's group: the text's box, with the background rectangle, which is
/// the text's box padded by 2 on every side, when the label has one (`createFormattedText`, 182-196).
#[must_use]
pub fn label_group_box(label: &LabelLayout) -> BBox {
    let text = label.bbox.browser();
    match label.background {
        Some(background) if text.is_empty() => background.browser(),
        Some(background) => text.union(background.browser()),
        None => text,
    }
}

/// The rectangle (`kanbanItem.ts` 127-135): the first child of the group.
fn rectangle(item: &KanbanItem<'_>, x: f64, y: f64, width: f64, height: f64) -> String {
    format!(
        "<rect class=\"basic label-container __APA__\" style=\"\" rx=\"{rx}\" ry=\"{rx}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\"></rect>",
        number(x),
        number(y),
        number(width),
        number(height),
        escape(item.paint.background),
        escape(item.paint.node_border),
        rx = number(item.corner_radius),
    )
}

/// One label's group (`labelHelper` and `insertLabel`): an empty background rectangle, then the
/// label, moved to its place by the transform the shape sets (`kanbanItem.ts` 90-105).
fn label_group(label: &LabelLayout, at: (f64, f64), item: &KanbanItem<'_>) -> String {
    format!(
        "<g class=\"label\" style=\"{style}\" transform=\"translate({}, {})\" fill=\"{}\"><rect></rect>{}</g>",
        number(at.0),
        number(at.1),
        escape(item.paint.text),
        label.svg(item.label_style),
        style = escape(item.label_style),
    )
}

/// The priority line (`kanbanItem.ts` 137-152): a bar two pixels inside the left edge, its stroke
/// the colour of the priority. A priority without a colour draws no stroke attribute.
fn priority_line(priority: &Value, x: f64, y: f64, height: f64, corner_radius: f64) -> String {
    let line_x = x + 2.0;
    let inset = (corner_radius / 2.0).floor();
    let stroke = colour_from_priority(priority)
        .map_or_else(String::new, |colour| format!(" stroke=\"{colour}\""));
    format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke-width=\"4\"{stroke}></line>",
        number(line_x),
        number(y + inset),
        number(line_x),
        number(y + height - inset),
        stroke = stroke,
    )
}

/// `colorFromPriority` (`kanbanItem.ts` 9-22): the colour of each named priority. `Medium` and any
/// other value have none, as the reference returns `null` or nothing for them.
#[must_use]
pub fn colour_from_priority(priority: &Value) -> Option<&'static str> {
    match priority.as_str()? {
        "Very High" => Some("red"),
        "High" => Some("orange"),
        "Low" => Some("blue"),
        "Very Low" => Some("lightblue"),
        _ => None,
    }
}

/// JavaScript's truthiness of a value: the falsy ones are `null`, `false`, zero, `NaN` and the empty
/// string.
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => *number != 0.0 && !number.is_nan(),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}
