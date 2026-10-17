//! The mindmap DB (`mindmapDb.ts`): the nodes the parser adds, in the order it adds them. The DB
//! keeps every node in one list, with the parent each node was attached to, and the model is the
//! tree of that list. The calls and their rules are in `docs-port/mindmap/PLAN.md`, section 3.

use ferrigura_model::mindmap::{MindmapDiagram, MindmapNode};
use ferrigura_model::value::Value;

use crate::sanitize::sanitize_text;
use crate::text::js_number;

/// `nodeType.DEFAULT`, also `NO_BORDER`.
pub(crate) const DEFAULT: u8 = 0;
/// `nodeType.ROUNDED_RECT`, written `(` … `)`.
pub(crate) const ROUNDED_RECT: u8 = 1;
/// `nodeType.RECT`, written `[` … `]`.
pub(crate) const RECT: u8 = 2;
/// `nodeType.CIRCLE`, written `((` … `))`.
pub(crate) const CIRCLE: u8 = 3;
/// `nodeType.CLOUD`, written `)` … `(`.
pub(crate) const CLOUD: u8 = 4;
/// `nodeType.BANG`, written `))` … `((`.
pub(crate) const BANG: u8 = 5;
/// `nodeType.HEXAGON`, written `{{` … `}}`.
pub(crate) const HEXAGON: u8 = 6;

/// The settings the DB reads when it adds a node: `mindmap.padding` and `mindmap.maxNodeWidth`, as
/// the configuration of the text gives them, with the reference's defaults for a missing or null one.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Settings {
    pub(crate) padding: Value,
    pub(crate) max_node_width: Value,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            padding: Value::Number(10.0),
            max_node_width: Value::Number(200.0),
        }
    }
}

/// A node as the DB holds it, with the index of its parent's entry and of its children's.
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    id: u32,
    node_id: String,
    descr: String,
    node_type: u8,
    level: u32,
    width: Value,
    padding: Value,
    icon: String,
    class: String,
    children: Vec<usize>,
}

/// The DB of a mindmap, filled by the parser's calls.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Db {
    entries: Vec<Entry>,
    /// The level of the first node: the levels of the others are counted from it.
    base_level: i64,
    settings: Settings,
}

impl Db {
    pub(crate) fn new(settings: Settings) -> Self {
        Self {
            entries: Vec::new(),
            base_level: 0,
            settings,
        }
    }

    /// `addNode(level, id, descr, type)`. The first node is the root; every other node is attached
    /// to the nearest earlier node with a smaller level. A node that finds no parent is an error,
    /// as is a second node at the root's level.
    pub(crate) fn add_node(
        &mut self,
        level: i64,
        id: &str,
        descr: &str,
        node_type: u8,
    ) -> Result<(), String> {
        let is_root = self.entries.is_empty();
        let level = if is_root {
            self.base_level = level;
            0
        } else {
            level - self.base_level
        };
        // `padding *= 2` for the shapes that double it: JavaScript's `ToNumber`, then the product.
        let padding = if matches!(node_type, ROUNDED_RECT | RECT | HEXAGON) {
            Value::Number(to_number(&self.settings.padding) * 2.0)
        } else {
            self.settings.padding.clone()
        };
        let node_id = sanitize_text(id);
        let descr = sanitize_text(descr);
        // `getParent(level)`: the last node with a smaller level.
        let parent = self
            .entries
            .iter()
            .rposition(|entry| i64::from(entry.level) < level);
        if parent.is_none() && !is_root {
            return Err(format!(
                "There can be only one root. No parent could be found for (\"{descr}\")"
            ));
        }
        let index = self.entries.len();
        self.entries.push(Entry {
            id: u32::try_from(index).unwrap_or(u32::MAX),
            node_id,
            descr,
            node_type,
            level: u32::try_from(level).unwrap_or(0),
            width: self.settings.max_node_width.clone(),
            padding,
            icon: String::new(),
            class: String::new(),
            children: Vec::new(),
        });
        if let Some(parent) = parent.and_then(|parent| self.entries.get_mut(parent)) {
            parent.children.push(index);
        }
        Ok(())
    }

    /// `decorateNode({icon})`: sets the icon of the last node added. With no node it throws, as the
    /// reference does on `undefined`, when the icon is not empty.
    pub(crate) fn decorate_icon(&mut self, icon: &str) -> Result<(), String> {
        if icon.is_empty() {
            return Ok(());
        }
        let text = sanitize_text(icon);
        let last = self.last_entry()?;
        last.icon = text;
        Ok(())
    }

    /// `decorateNode({class})`: sets the class of the last node added. Throws as `decorate_icon`.
    pub(crate) fn decorate_class(&mut self, class: &str) -> Result<(), String> {
        if class.is_empty() {
            return Ok(());
        }
        let text = sanitize_text(class);
        let last = self.last_entry()?;
        last.class = text;
        Ok(())
    }

    fn last_entry(&mut self) -> Result<&mut Entry, String> {
        self.entries
            .last_mut()
            .ok_or_else(|| "Cannot set properties of undefined".to_owned())
    }

    /// The model: the tree from the first node, or none when no node was added.
    pub(crate) fn into_model(self) -> MindmapDiagram {
        let root = if self.entries.is_empty() {
            None
        } else {
            Some(build(&self.entries, 0))
        };
        MindmapDiagram {
            root,
            ..MindmapDiagram::new()
        }
    }
}

fn build(entries: &[Entry], index: usize) -> MindmapNode {
    let entry = &entries[index];
    MindmapNode {
        id: entry.id,
        node_id: entry.node_id.clone(),
        descr: entry.descr.clone(),
        node_type: entry.node_type,
        level: entry.level,
        width: entry.width.clone(),
        padding: entry.padding.clone(),
        icon: entry.icon.clone(),
        class: entry.class.clone(),
        children: entry
            .children
            .iter()
            .map(|&child| build(entries, child))
            .collect(),
    }
}

/// JavaScript's `ToNumber` for a configuration value: `null` is 0, a boolean 1 or 0, a string is
/// read by `Number()`, and an array or an object goes through its string form.
fn to_number(value: &Value) -> f64 {
    match value {
        Value::Null => 0.0,
        Value::Bool(flag) => f64::from(u8::from(*flag)),
        Value::Number(number) => *number,
        Value::String(text) => js_number(text),
        Value::Array(_) | Value::Object(_) => js_number(&to_js_string(value)),
    }
}

/// JavaScript's `String()` for an array or an object: an array joins its elements with commas, an
/// element that is `null` gives nothing, and an object is `[object Object]`.
fn to_js_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number_to_string(*number),
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(to_js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// JavaScript's `String()` for a number, for the cases a configuration can produce.
fn number_to_string(number: f64) -> String {
    if number.is_nan() {
        "NaN".to_owned()
    } else if number.is_infinite() {
        let sign = if number > 0.0 { "" } else { "-" };
        format!("{sign}Infinity")
    } else if number == 0.0 {
        "0".to_owned()
    } else {
        number.to_string()
    }
}

/// `getType(start, end)`: the shape of a node from the token that opens its text and the token
/// that closes it. A `(` closed by `)` is rounded; a `(` closed by anything else is a cloud.
pub(crate) fn node_type(start: &str, end: &str) -> u8 {
    match start {
        "[" => RECT,
        "(" => {
            if end == ")" {
                ROUNDED_RECT
            } else {
                CLOUD
            }
        }
        "((" => CIRCLE,
        ")" => CLOUD,
        "))" => BANG,
        "{{" => HEXAGON,
        _ => DEFAULT,
    }
}
