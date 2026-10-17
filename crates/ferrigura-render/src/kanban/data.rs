//! `getData` of the kanban diagram: the layout data its renderer reads (`kanbanDb.ts:54-97`).
//!
//! The data is a flat list. Each section comes first, followed by the entries of its children, in
//! the order the DB holds them. A child is every node whose parent id is the section's id, so a
//! section id that the diagram repeats lists its children under each copy, and the renderer, which
//! filters the whole list by parent id, draws them once per copy (`kanbanRenderer.ts` 66-67,
//! NOTES 3.4). A section keeps its ticket and drops its priority, assignee, icon and classes; an
//! item keeps all of them. Both drop their classes, which the renderer does not read.

use ferrigura_elk::json::js_number_string;
use ferrigura_model::kanban::{KanbanDiagram, KanbanNode};
use ferrigura_model::value::Value;
use ferrigura_syntax::sanitize::sanitize_text;

/// The shape a data node is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `kanbanSection`: a column of the board.
    Section,
    /// `kanbanItem`: a card in a column.
    Item,
}

/// The `cssStyles` of an item: `['text-align: left']` (`kanbanDb.ts:83`).
const ITEM_CSS_STYLES: [&str; 1] = ["text-align: left"];

/// The corner radius of an item (`kanbanDb.ts:81-82`); a section has none in the data, and the
/// renderer sets 5 on it (`kanbanRenderer.ts:55-56`).
const CORNER_RADIUS: f64 = 5.0;

/// A node of the layout data: a section (`isGroup`) or an item.
#[derive(Debug, Clone, PartialEq)]
pub struct DataNode {
    /// The id, as the DB holds it. A repeated section id is repeated here.
    pub id: String,
    /// The id of the section an item belongs to (`parentId`). Absent for a section.
    pub parent_id: Option<String>,
    /// The label, sanitised a second time (`sanitizeText(label ?? '')`). Its type is markdown.
    pub label: String,
    /// `isGroup`: whether the node is a section.
    pub is_group: bool,
    /// The shape the node is drawn with.
    pub shape: Shape,
    /// The indentation of the node in the source.
    pub level: u32,
    /// The ticket of the node. Kept for sections and items alike.
    pub ticket: Option<String>,
    /// The priority, as the YAML value gave it. Items only: a section's is dropped.
    pub priority: Option<Value>,
    /// The assignee. Items only: a section's is dropped.
    pub assigned: Option<String>,
    /// The icon. Items only: a section's is dropped.
    pub icon: Option<String>,
    /// The look of the configuration (`look: conf.look`). Sections only.
    pub look: Option<String>,
    /// The corner radii (`rx`, `ry`). Items only; the renderer sets them on sections.
    pub corner_radius: Option<f64>,
    /// The declarations set on the node (`cssStyles`). Items only.
    pub css_styles: Vec<String>,
}

/// `getData` of the kanban DB: the nodes the renderer lays out, sections and their items in
/// order. `look` is the configuration's `look`.
#[must_use]
pub fn get_data(diagram: &KanbanDiagram, look: &str) -> Vec<DataNode> {
    let mut nodes = Vec::new();
    for section in diagram.nodes.iter().filter(|node| node.is_section) {
        nodes.push(section_node(section, look));
        // `nodes.filter((n) => n.parentId === section.id)`: the children of every node with this
        // id, which a repeated section id shares.
        for item in diagram
            .nodes
            .iter()
            .filter(|node| node.parent_id.as_deref() == Some(section.id.as_str()))
        {
            nodes.push(item_node(item, &section.id));
        }
    }
    nodes
}

/// A section's node (`kanbanDb.ts` 56-65): its label, its ticket and its look; no shape data beyond
/// the ticket reaches the layout.
fn section_node(section: &KanbanNode, look: &str) -> DataNode {
    DataNode {
        id: section.id.clone(),
        parent_id: None,
        label: sanitized_label(&section.label),
        is_group: true,
        shape: Shape::Section,
        level: section.level,
        ticket: section.ticket.clone(),
        priority: None,
        assigned: None,
        icon: None,
        look: Some(look.to_owned()),
        corner_radius: None,
        css_styles: Vec::new(),
    }
}

/// An item's node (`kanbanDb.ts` 73-92), a child of the section with the id `parent`.
fn item_node(item: &KanbanNode, parent: &str) -> DataNode {
    DataNode {
        id: item.id.clone(),
        parent_id: Some(parent.to_owned()),
        label: sanitized_label(&item.label),
        is_group: false,
        shape: Shape::Item,
        level: item.level,
        ticket: item.ticket.clone(),
        priority: item.priority.clone(),
        assigned: item.assigned.clone(),
        icon: item.icon.clone(),
        look: None,
        corner_radius: Some(CORNER_RADIUS),
        css_styles: ITEM_CSS_STYLES
            .iter()
            .map(|style| (*style).to_owned())
            .collect(),
    }
}

/// `sanitizeText(label ?? '')` on the label as the DB holds it.
fn sanitized_label(label: &Value) -> String {
    sanitize_text(&js_string(label))
}

/// `String(value)` for a label, with `null` as the empty string (`?? ''`): booleans and numbers as
/// JavaScript writes them, arrays joined with commas, and a mapping as `[object Object]`.
#[must_use]
pub fn js_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) if number.is_nan() => "NaN".to_owned(),
        Value::Number(number) if number.is_infinite() => if *number > 0.0 {
            "Infinity"
        } else {
            "-Infinity"
        }
        .to_owned(),
        Value::Number(number) => js_number_string(*number),
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}
