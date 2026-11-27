//! The state DB of `stateDb.ts` as far as the parse holds it: `docTranslator` (327-375), `extract`
//! (249-302) with its setters, and the one error the reference raises while it extracts, which
//! comes from the render data. See
//! `docs/diagrams/state.md#parsing`.
//!
//! The reference runs `extract` twice (at the parse and at the draw), and the second run gives the
//! same states, relations, classes and links; the port runs it once.

use std::collections::HashMap;

use ferrigura_model::state::{
    ClassDef, Description, Link, Note, Relation, StateEntry, StateStmt, Statement, StyleStmt,
};

use crate::sanitize::sanitize_text;
use crate::text::js_trim;

/// The id of the `number`-th trailing concurrency region, counted from 1 in the order the reference
/// creates them (`docTranslator` splits a composite before it translates the composite's children).
/// The reference draws the id with `Math.random`; the port writes this placeholder, and the
/// exporters number the reference's random ids in the same order, so the two agree. The numbers
/// keep two trailing regions apart: each one is a state of its own.
pub(crate) fn generated_id(number: usize) -> String {
    format!("<generated:{number}>")
}

/// The text of a state id as the reference's `sanitizeText` leaves it, with the placeholders of
/// the trailing regions kept as they are. The reference's random id is plain text (an alphanumeric
/// run), which sanitizing leaves alone, so the label of `<generated:k>_start` is the id itself; the
/// placeholder would be taken for an HTML tag otherwise.
pub fn sanitize_state_id(id: &str) -> String {
    const PREFIX: &str = "<generated:";
    let mut out = String::new();
    let mut rest = id;
    while let Some(start) = rest.find(PREFIX) {
        let after = &rest[start + PREFIX.len()..];
        let Some(end) = after.find('>') else { break };
        let number = &after[..end];
        if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
            break;
        }
        out.push_str(&sanitize_text(&rest[..start]));
        out.push_str(&rest[start..start + PREFIX.len() + end + 1]);
        rest = &after[end + 1..];
    }
    out.push_str(&sanitize_text(rest));
    out
}

enum TranslateTask<'a> {
    Statement {
        parent: String,
        node: &'a mut Statement,
        first: bool,
    },
    State {
        parent: String,
        state: &'a mut StateStmt,
        first: bool,
    },
}

/// `docTranslator` for the root document, whose parent is `root`. `regions` counts the trailing
/// regions the translation has created, over both passes of the parse.
pub(crate) fn translate_root(document: &mut [Statement], regions: &mut usize) {
    let mut stack: Vec<TranslateTask<'_>> = document
        .iter_mut()
        .rev()
        .map(|node| TranslateTask::Statement {
            parent: "root".to_owned(),
            node,
            first: true,
        })
        .collect();
    while let Some(task) = stack.pop() {
        match task {
            TranslateTask::Statement {
                parent,
                node,
                first,
            } => match node {
                Statement::Relation(relation) => {
                    stack.push(TranslateTask::State {
                        parent: parent.clone(),
                        state: &mut relation.state2,
                        first: false,
                    });
                    stack.push(TranslateTask::State {
                        parent,
                        state: &mut relation.state1,
                        first: true,
                    });
                }
                Statement::State(state) => {
                    stack.push(TranslateTask::State {
                        parent,
                        state,
                        first,
                    });
                }
                _ => {}
            },
            TranslateTask::State {
                parent,
                state,
                first,
            } => {
                if state.id == "[*]" {
                    state.id = format!("{parent}{}", if first { "_start" } else { "_end" });
                    state.start = Some(first);
                } else {
                    state.id = js_trim(&state.id).to_owned();
                }
                let Some(document) = state.doc.take() else {
                    continue;
                };

                // Each divider takes the statements before it; the statements after the last one
                // become a trailing region. Without a divider, or with an empty trailing list, the
                // document stays.
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
                    let mut trailing = StateStmt::new(generated_id(*regions + 1), Some("divider"));
                    *regions += 1;
                    trailing.doc = Some(current);
                    split.push(Statement::State(trailing));
                    split
                } else {
                    document
                };

                state.doc = Some(document);
                let id = state.id.clone();
                if let Some(children) = &mut state.doc {
                    for child in children.iter_mut().rev() {
                        stack.push(TranslateTask::Statement {
                            parent: id.clone(),
                            node: child,
                            first: true,
                        });
                    }
                }
            }
        }
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

/// The ids of the root states, as they are before the next translation pass.
pub(crate) fn root_state_ids(document: &[Statement]) -> Vec<Option<String>> {
    document
        .iter()
        .map(|item| match item {
            Statement::State(state) => Some(state.id.clone()),
            _ => None,
        })
        .collect()
}

/// `extract`: each root statement, in order, goes to the DB. The DB is keyed by `ids`, the root
/// state ids of the first translation pass. The root's notes are sanitized in place, as `addState`
/// does to the note object it is given.
pub(crate) fn extract(document: &mut [Statement], ids: &[Option<String>]) -> Db {
    let mut db = Db::default();
    for (index, item) in document.iter_mut().enumerate() {
        match item {
            Statement::State(state) => {
                if let Some(note) = state.note.as_mut() {
                    note.text = sanitize_text(&note.text);
                }
                let id = js_trim(
                    ids.get(index)
                        .and_then(Option::as_deref)
                        .unwrap_or(&state.id),
                )
                .to_owned();
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

    /// `addDescription`: a leading colon goes, the rest is trimmed, and the text is sanitized.
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
    /// and classes; the relation keeps the sanitized label.
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

/// The label of a node: the sanitized id, one text, or a list (the first entry is the label).
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
        enum VisitTask<'a> {
            Statement(&'a Statement),
            State(&'a StateStmt),
        }

        let mut stack: Vec<VisitTask<'_>> =
            document.iter().rev().map(VisitTask::Statement).collect();
        while let Some(task) = stack.pop() {
            match task {
                VisitTask::Statement(Statement::State(state)) => {
                    stack.push(VisitTask::State(state));
                }
                VisitTask::Statement(Statement::Relation(relation)) => {
                    stack.push(VisitTask::State(&relation.state2));
                    stack.push(VisitTask::State(&relation.state1));
                }
                VisitTask::Statement(_) => {}
                VisitTask::State(state) => {
                    // `dataFetcher`: a state named `root` is not a node, but its document is still
                    // visited.
                    if state.id != "root" {
                        self.visit_node(state);
                    }
                    if let Some(document) = &state.doc {
                        for item in document.iter().rev() {
                            stack.push(VisitTask::Statement(item));
                        }
                    }
                }
            }
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
/// it, which flattens the list and sanitizes each text.
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
