//! The state DB of `stateDb.ts` as far as the parse holds it: `docTranslator` (327-375), `extract`
//! (249-302) with its setters, and the one error the reference raises while it extracts, which
//! comes from the render data. See `docs-port/state/PLAN.md`, sections 5.1 to 5.4.
//!
//! The reference runs `extract` twice (at the parse and at the draw), and the second run gives the
//! same states, relations, classes and links; the port runs it once.

use std::collections::HashMap;

use ferrigura_model::state::{
    ClassDef, Description, Link, Note, Relation, StateEntry, StateStmt, Statement, StyleStmt,
};

use crate::sanitize::sanitize_text;
use crate::text::js_trim;

/// The id of a trailing concurrency region. The reference draws it with `Math.random`; the port
/// writes this placeholder, and the exporter writes the same one for the reference's random id.
pub(crate) const GENERATED_ID: &str = "<generated>";

/// `docTranslator` for the root document, whose parent is `root`.
pub(crate) fn translate_root(document: &mut [Statement]) {
    for node in document {
        translate_statement("root", node, true);
    }
}

/// `docTranslator` for one statement: a relation translates its two states, a state its id and
/// its document.
fn translate_statement(parent: &str, node: &mut Statement, first: bool) {
    match node {
        Statement::Relation(relation) => {
            translate_state(parent, &mut relation.state1, true);
            translate_state(parent, &mut relation.state2, false);
        }
        Statement::State(state) => translate_state(parent, state, first),
        _ => {}
    }
}

/// `docTranslator` for a state: `[*]` becomes `<parent>_start` or `<parent>_end`, other ids are
/// trimmed, and a document is split at its dividers.
fn translate_state(parent: &str, state: &mut StateStmt, first: bool) {
    if state.id == "[*]" {
        state.id = format!("{parent}{}", if first { "_start" } else { "_end" });
        state.start = Some(first);
    } else {
        state.id = js_trim(&state.id).to_owned();
    }
    let Some(document) = state.doc.take() else {
        return;
    };

    // Each divider takes the statements before it; the statements after the last one become a
    // trailing region. Without a divider, or with an empty trailing list, the document stays.
    let mut split = Vec::new();
    let mut current = Vec::new();
    for statement in &document {
        match statement {
            Statement::State(divider) if divider.kind.as_deref() == Some("divider") => {
                let mut split_divider = divider.clone();
                split_divider.doc = Some(current.clone());
                split.push(Statement::State(split_divider));
                current.clear();
            }
            other => current.push(other.clone()),
        }
    }
    let document = if !split.is_empty() && !current.is_empty() {
        let mut trailing = StateStmt::new(GENERATED_ID, Some("divider"));
        trailing.doc = Some(current);
        split.push(Statement::State(trailing));
        split
    } else {
        document
    };

    state.doc = Some(document);
    let id = state.id.clone();
    for child in state.doc.iter_mut().flatten() {
        translate_statement(&id, child, true);
    }
}

/// The DB's states, relations, classes and links after the root document has been extracted.
#[derive(Debug, Default)]
pub(crate) struct Db {
    pub states: Vec<StateEntry>,
    state_index: HashMap<String, usize>,
    pub relations: Vec<Relation>,
    pub classes: Vec<ClassDef>,
    class_index: HashMap<String, usize>,
    pub links: Vec<Link>,
}

/// `extract`: each root statement, in order, goes to the DB. The root's notes are sanitised in
/// place, as `addState` does to the note object it is given.
pub(crate) fn extract(document: &mut [Statement]) -> Db {
    let mut db = Db::default();
    for item in document.iter_mut() {
        match item {
            Statement::State(state) => {
                if let Some(note) = state.note.as_mut() {
                    note.text = sanitize_text(&note.text);
                }
                let id = js_trim(&state.id).to_owned();
                let kind = state.kind.clone().unwrap_or_else(|| "default".to_owned());
                db.add_state(
                    &id,
                    &kind,
                    state.doc.as_deref(),
                    state.description.as_ref(),
                    state.note.as_ref(),
                    None,
                );
            }
            Statement::Relation(relation) => db.add_relation(
                &relation.state1,
                &relation.state2,
                relation.description.as_deref(),
            ),
            Statement::ClassDef(class_def) => {
                db.add_style_class(js_trim(&class_def.id), &class_def.classes);
            }
            Statement::Style(style) => db.handle_style_def(style),
            Statement::ApplyClass(apply) => {
                db.set_css_class(js_trim(&apply.id), &apply.style_class);
            }
            Statement::Click(click) => db.links.push(Link {
                id: click.id.id.clone(),
                url: click.url.clone(),
                tooltip: click.tooltip.clone(),
            }),
            _ => {}
        }
    }
    db
}

impl Db {
    fn state_mut(&mut self, id: &str) -> Option<&mut StateEntry> {
        let index = *self.state_index.get(id)?;
        self.states.get_mut(index)
    }

    /// `addState`: a new state takes the type, the document and the note; an existing one keeps
    /// its type and takes the document only when it has none. The description, the note and the
    /// classes are then added in that order.
    pub(crate) fn add_state(
        &mut self,
        id: &str,
        kind: &str,
        document: Option<&[Statement]>,
        description: Option<&Description>,
        note: Option<&Note>,
        classes: Option<&[String]>,
    ) {
        let id = js_trim(id).to_owned();
        if let Some(state) = self.state_mut(&id) {
            if state.doc.is_none() {
                state.doc = document.map(<[Statement]>::to_vec);
            }
        } else {
            self.state_index.insert(id.clone(), self.states.len());
            self.states.push(StateEntry {
                id: id.clone(),
                kind: kind.to_owned(),
                descriptions: Vec::new(),
                doc: document.map(<[Statement]>::to_vec),
                note: None,
                classes: Vec::new(),
                styles: Vec::new(),
                text_styles: Vec::new(),
            });
        }

        match description {
            Some(Description::Text(text)) if !text.is_empty() => {
                self.add_description(&id, js_trim(text));
            }
            Some(Description::List(parts)) => {
                for part in parts {
                    self.add_description(&id, js_trim(part));
                }
            }
            _ => {}
        }

        if let Some(note) = note
            && let Some(state) = self.state_mut(&id)
        {
            state.note = Some(note.clone());
        }

        for class in classes.unwrap_or_default() {
            self.set_css_class(&id, js_trim(class));
        }
    }

    /// `addDescription`: a leading colon goes, the rest is trimmed, and the text is sanitised.
    fn add_description(&mut self, id: &str, text: &str) {
        let text = match text.strip_prefix(':') {
            Some(rest) => js_trim(rest),
            None => text,
        };
        let sanitized = sanitize_text(text);
        if let Some(state) = self.state_mut(id) {
            state.descriptions.push(sanitized);
        }
    }

    /// `addRelationObjs`: both states are added with their own type, document, description, note
    /// and classes; the relation keeps the sanitised label.
    pub(crate) fn add_relation(
        &mut self,
        state1: &StateStmt,
        state2: &StateStmt,
        label: Option<&str>,
    ) {
        let id1 = js_trim(&state1.id).to_owned();
        let id2 = js_trim(&state2.id).to_owned();
        self.add_state(
            &id1,
            state1.kind.as_deref().unwrap_or("default"),
            state1.doc.as_deref(),
            state1.description.as_ref(),
            state1.note.as_ref(),
            state1.classes.as_deref(),
        );
        self.add_state(
            &id2,
            state2.kind.as_deref().unwrap_or("default"),
            state2.doc.as_deref(),
            state2.description.as_ref(),
            state2.note.as_ref(),
            state2.classes.as_deref(),
        );
        self.relations.push(Relation {
            id1,
            id2,
            relation_title: sanitize_text(label.unwrap_or("")),
        });
    }

    /// `setCssClass`: each comma-separated id gets the class, and is created when it is missing.
    pub(crate) fn set_css_class(&mut self, ids: &str, class: &str) {
        for part in ids.split(',') {
            let id = js_trim(part).to_owned();
            if self.state_mut(&id).is_none() {
                self.add_state(&id, "default", None, None, None, None);
            }
            if let Some(state) = self.state_mut(&id) {
                state.classes.push(class.to_owned());
            }
        }
    }

    /// `handleStyleDef`: the styles of each listed state are replaced by the split list, with
    /// every `;` removed; a missing state is created.
    fn handle_style_def(&mut self, style: &StyleStmt) {
        let styles: Vec<String> = style
            .style_class
            .split(',')
            .map(|part| js_trim(&part.replace(';', "")).to_owned())
            .collect();
        for part in js_trim(&style.id).split(',') {
            let id = js_trim(part).to_owned();
            if self.state_mut(&id).is_none() {
                self.add_state(&id, "default", None, None, None, None);
            }
            if let Some(state) = self.state_mut(&id) {
                state.styles = styles.clone();
            }
        }
    }

    /// `addStyleClass`: a class is created once; each non-empty comma-separated attribute is added
    /// with its first `;` removed. An attribute that mentions `color` also goes to the text styles,
    /// with `fill` renamed `bgFill` and then `color` renamed `fill`.
    pub(crate) fn add_style_class(&mut self, id: &str, attributes: &str) {
        if !self.class_index.contains_key(id) {
            self.class_index.insert(id.to_owned(), self.classes.len());
            self.classes.push(ClassDef {
                id: id.to_owned(),
                styles: Vec::new(),
                text_styles: Vec::new(),
            });
        }
        if attributes.is_empty() {
            return;
        }
        let Some(&index) = self.class_index.get(id) else {
            return;
        };
        let class = &mut self.classes[index];
        for attribute in attributes.split(',') {
            let fixed = js_trim(&remove_first_semicolon(attribute)).to_owned();
            if attribute.contains("color") {
                let renamed = fixed
                    .replacen("fill", "bgFill", 1)
                    .replacen("color", "fill", 1);
                class.text_styles.push(renamed);
            }
            class.styles.push(fixed);
        }
    }
}

/// `attrib.replace(/([^;]*);/, '$1')`: the first semicolon is removed, and no other.
fn remove_first_semicolon(text: &str) -> String {
    match text.find(';') {
        Some(index) => format!("{}{}", &text[..index], &text[index + 1..]),
        None => text.to_owned(),
    }
}

/// `dataFetcher`'s description handling, followed by the check in `extract` (lines 288-301). The
/// DB does not hold the nodes, so this follows the descriptions the render data gives each state
/// (`dataFetcher.ts` 241-370): the first description turns the id into a list, the later ones are
/// appended, and a group (a state with a document) that ends with two or more descriptions makes
/// `extract` throw. Returns the message of that error.
pub(crate) fn group_label_error(document: &[Statement]) -> Option<String> {
    let mut fetch = Fetch::default();
    fetch.visit_document(document);
    fetch.order.iter().find_map(|id| {
        let node = fetch.nodes.get(id)?;
        match &node.description {
            NodeText::List(labels) if node.group && labels.len() >= 2 => Some(format!(
                "Group nodes can only have label. Remove the additional description for node [{id}]"
            )),
            _ => None,
        }
    })
}

/// The label of a node: the sanitised id, one text, or a list (the first entry is the label).
#[derive(Debug, Clone)]
enum NodeText {
    Text(String),
    List(Vec<String>),
}

#[derive(Debug)]
struct Node {
    description: NodeText,
    group: bool,
}

#[derive(Debug, Default)]
struct Fetch {
    nodes: HashMap<String, Node>,
    /// The ids in the order `insertOrUpdateNode` first inserts them.
    order: Vec<String>,
}

impl Fetch {
    fn visit_document(&mut self, document: &[Statement]) {
        for item in document {
            match item {
                Statement::State(state) => self.visit_state(state),
                Statement::Relation(relation) => {
                    self.visit_state(&relation.state1);
                    self.visit_state(&relation.state2);
                }
                _ => {}
            }
        }
    }

    /// `dataFetcher`: a state named `root` is not a node, but its document is still visited.
    fn visit_state(&mut self, state: &StateStmt) {
        if state.id != "root" {
            self.visit_node(state);
        }
        if let Some(document) = &state.doc {
            self.visit_document(document);
        }
    }

    fn visit_node(&mut self, state: &StateStmt) {
        let id = state.id.as_str();
        let is_new = !self.nodes.contains_key(id);
        let node = self.nodes.entry(id.to_owned()).or_insert_with(|| Node {
            description: NodeText::Text(sanitize_text(id)),
            group: false,
        });
        if let Some(description) = &state.description
            && is_present(description)
        {
            update_description(node, id, description);
        }
        if state.doc.is_some() {
            node.group = true;
        }
        let is_node = !id.is_empty() && !matches!(id, "</join></fork>" | "</choice>");
        if is_new && is_node {
            self.order.push(id.to_owned());
        }
    }
}

/// Whether a description is truthy in JavaScript: a non-empty string, or a list.
fn is_present(description: &Description) -> bool {
    match description {
        Description::Text(text) => !text.is_empty(),
        Description::List(_) => true,
    }
}

/// The description update of `dataFetcher` for one visit, with `sanitizeTextOrArray` applied after
/// it, which flattens the list and sanitises each text.
fn update_description(node: &mut Node, id: &str, description: &Description) {
    let parts: Vec<String> = match description {
        Description::Text(text) => vec![text.clone()],
        Description::List(parts) => parts.clone(),
    };
    let next = match &node.description {
        NodeText::List(labels) => {
            let mut labels = labels.clone();
            labels.extend(parts);
            NodeText::List(labels)
        }
        NodeText::Text(current) if !current.is_empty() => {
            if current == id {
                NodeText::List(parts)
            } else {
                let mut labels = vec![current.clone()];
                labels.extend(parts);
                NodeText::List(labels)
            }
        }
        NodeText::Text(_) => match description {
            Description::Text(text) => NodeText::Text(text.clone()),
            Description::List(list) => NodeText::List(list.clone()),
        },
    };
    node.description = match next {
        NodeText::Text(text) => NodeText::Text(sanitize_text(&text)),
        NodeText::List(list) => {
            NodeText::List(list.iter().map(|text| sanitize_text(text)).collect())
        }
    };
}
