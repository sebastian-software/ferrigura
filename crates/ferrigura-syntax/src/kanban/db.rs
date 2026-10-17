//! The kanban DB, `kanbanDb.ts`: the nodes the grammar adds, their level-based placement, the shape
//! data and the decorations. See `docs-port/kanban/PLAN.md`, sections 3 and 4.
//!
//! The DB keeps one list. A section is a node that `getSection` found no parent for; the reference
//! keeps the same objects in its `sections` list, so the flag [`DbNode::is_section`] is that list.

use ferrigura_model::kanban::{KanbanDiagram, KanbanNode};
use ferrigura_model::value::Value;

use crate::sanitize::sanitize_text;
use crate::yaml::{self, Yaml};

/// A DB error: the reference throws it, and the parse is rejected with its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DbError(pub(crate) String);

/// One node of the DB object (`addNode`, `kanbanDb.ts:109-116`), with the fields the shape data and
/// the decorations add. The width and padding of the reference come from the configuration and are
/// not kept.
#[derive(Debug, Clone, PartialEq)]
struct DbNode {
    id: String,
    level: u32,
    label: Value,
    parent_id: Option<String>,
    is_section: bool,
    icon: Option<String>,
    assigned: Option<String>,
    ticket: Option<String>,
    priority: Option<Value>,
    css_classes: Option<String>,
}

/// The DB state the grammar builds.
#[derive(Debug, Default)]
pub(crate) struct KanbanDb {
    nodes: Vec<DbNode>,
    /// The counter of the `kbn<n>` ids (`kanbanDb.ts:12`, `cnt`).
    counter: u32,
}

impl KanbanDb {
    /// `addNode(level, id, descr, type, shapeData)` (`kanbanDb.ts:99-165`). The shape data is the
    /// text the grammar concatenated, `None` when the node has none.
    pub(crate) fn add_node(
        &mut self,
        level: u32,
        id: &str,
        descr: &str,
        shape_data: Option<&str>,
    ) -> Result<(), DbError> {
        // `sanitizeText(id) || 'kbn' + cnt++`: the counter moves only for an id that sanitises to
        // nothing.
        let sanitized_id = sanitize_text(id);
        let id = if sanitized_id.is_empty() {
            let counted = format!("kbn{}", self.counter);
            self.counter += 1;
            counted
        } else {
            sanitized_id
        };
        let mut node = DbNode {
            id,
            level,
            label: Value::String(sanitize_text(descr)),
            parent_id: None,
            is_section: false,
            icon: None,
            assigned: None,
            ticket: None,
            priority: None,
            css_classes: None,
        };
        if let Some(text) = shape_data {
            apply_shape_data(&mut node, text)?;
        }
        match self.get_section(level)? {
            Some(section) => node.parent_id = Some(self.nodes[section].id.clone()),
            None => node.is_section = true,
        }
        self.nodes.push(node);
        Ok(())
    }

    /// `getSection(level)` (`kanbanDb.ts:25-48`): the index of the section the node joins, or `None`
    /// when the node is a section. Every node that exists is checked against the first node's level,
    /// so the error appears one line late (quirk 8).
    fn get_section(&self, level: u32) -> Result<Option<usize>, DbError> {
        let Some(first) = self.nodes.first() else {
            return Ok(None);
        };
        let section_level = first.level;
        let mut last_section = None;
        for (index, node) in self.nodes.iter().enumerate().rev() {
            if node.level == section_level && last_section.is_none() {
                last_section = Some(index);
            }
            if node.level < section_level {
                return Err(DbError(format!(
                    "Items without section detected, found section (\"{}\")",
                    label_text(&node.label)
                )));
            }
        }
        match last_section {
            Some(section) if level != self.nodes[section].level => Ok(Some(section)),
            _ => Ok(None),
        }
    }

    /// `decorateNode({icon})` (`kanbanDb.ts:202-214`): the icon goes to the last node, whatever its
    /// level. Without a node the reference fails on the property assignment.
    pub(crate) fn decorate_icon(&mut self, icon: &str) -> Result<(), DbError> {
        let node = self.last_node()?;
        node.icon = Some(sanitize_text(icon));
        Ok(())
    }

    /// `decorateNode({class})`: the classes go to the last node, whatever its level.
    pub(crate) fn decorate_class(&mut self, class: &str) -> Result<(), DbError> {
        let node = self.last_node()?;
        node.css_classes = Some(sanitize_text(class));
        Ok(())
    }

    fn last_node(&mut self) -> Result<&mut DbNode, DbError> {
        self.nodes.last_mut().ok_or_else(|| {
            DbError("Cannot set properties of undefined (decorating before any node)".to_owned())
        })
    }

    /// The model of the DB: every node in the order it was added.
    pub(crate) fn into_model(self) -> KanbanDiagram {
        let mut diagram = KanbanDiagram::new();
        diagram.nodes = self
            .nodes
            .into_iter()
            .map(|node| KanbanNode {
                id: node.id,
                level: node.level,
                label: node.label,
                parent_id: node.parent_id,
                is_section: node.is_section,
                icon: node.icon,
                assigned: node.assigned,
                ticket: node.ticket,
                priority: node.priority,
                css_classes: node.css_classes,
            })
            .collect();
        diagram
    }
}

/// The text of a label for an error message, as `String` gives it.
fn label_text(label: &Value) -> String {
    match label {
        Value::String(text) => text.clone(),
        _ => String::new(),
    }
}

/// The shape data (`kanbanDb.ts:118-155`): YAML in braces when it has no line break, as it is, with
/// a line break added, and then the fields of the mapping. Only the fields the DB keeps are set.
fn apply_shape_data(node: &mut DbNode, text: &str) -> Result<(), DbError> {
    let source = if text.contains('\n') {
        format!("{text}\n")
    } else {
        format!("{{\n{text}\n}}")
    };
    let document = yaml::load(&source).map_err(|error| DbError(error.message))?;
    // `doc.shape` throws for an empty document and for `null`; every other scalar and a sequence
    // have no shape and none of the fields, so they change nothing.
    let pairs = match document {
        None | Some(Yaml::Null) => {
            return Err(DbError(
                "Cannot read properties of null (reading 'shape')".to_owned(),
            ));
        }
        Some(Yaml::Mapping(pairs)) => pairs,
        Some(_) => return Ok(()),
    };
    let field = |name: &str| {
        pairs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
            .filter(|value| truthy(value))
    };

    // The shape is checked, and never stored: the only shape the DB would store is `kanbanItem`,
    // and that name fails the lowercase check first (quirk 7).
    if let Some(shape) = field("shape") {
        match shape {
            Yaml::String(name) if name == &name.to_lowercase() && !name.contains('_') => {}
            Yaml::String(name) => {
                return Err(DbError(format!(
                    "No such shape: {name}. Shape names should be lowercase."
                )));
            }
            _ => {
                return Err(DbError(
                    "doc.shape.toLowerCase is not a function".to_owned(),
                ));
            }
        }
    }
    if let Some(label) = field("label") {
        node.label = Value::from(label);
    }
    if let Some(icon) = field("icon") {
        node.icon = Some(icon.js_string());
    }
    if let Some(assigned) = field("assigned") {
        node.assigned = Some(assigned.js_string());
    }
    if let Some(ticket) = field("ticket") {
        node.ticket = Some(ticket.js_string());
    }
    if let Some(priority) = field("priority") {
        node.priority = Some(Value::from(priority));
    }
    Ok(())
}

/// JavaScript's truthiness of a YAML value: `null`, `false`, zero, `NaN` and the empty string are
/// falsy; arrays and objects are always truthy.
fn truthy(value: &Yaml) -> bool {
    match value {
        Yaml::Null => false,
        Yaml::Bool(flag) => *flag,
        Yaml::Number(number) => *number != 0.0 && !number.is_nan(),
        Yaml::String(text) => !text.is_empty(),
        Yaml::Sequence(_) | Yaml::Mapping(_) => true,
    }
}
