//! The layout data of a state diagram: a port of `dataFetcher` (`diagrams/state/dataFetcher.ts`),
//! the label step of `stateDb.extract` (`stateDb.ts` 285-299) and `stateDb.getData` (`stateDb.ts`
//! 755-770). It fills the layout data of [`crate::unified::data`].
//!
//! The reference keeps `dataFetcher`'s state in module variables (`nodeDb`, `graphItemCount`, the
//! colour slots) and resets them at every `extract`. Here that state lives in one [`Fetcher`] per
//! call, so each call starts from the reset state. The reference's `extract` runs `dataFetcher`
//! on the root document a second time (`stateDb.ts` 277) and `draw` runs `extract` again; the
//! second pass gives the same nodes and edges, so the document is read once (`docs-port/state/
//! PLAN.md` section 2, step 6).
//!
//! The differential run (`pnpm port:diff state-data`) compares the result with the reference's
//! `getData()`.

use std::collections::HashMap;

use crate::context::Globals;
use crate::unified::Section;
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::state::{
    ClassDef, Description, StateDiagram, StateEntry, StateStmt, Statement,
};
use ferrigura_syntax::sanitize::sanitize_text;
use ferrigura_syntax::state::sanitize_state_id;

use crate::unified::data::{Edge, LayoutData, Node};

/// The id of the root of the document (`STMT_ROOT`). The root is not a node.
const ROOT: &str = "root";

/// The shape of a plain state (`SHAPE_STATE`, a `rect` with radii, see `nodes.ts`).
const SHAPE_STATE: &str = "rect";
/// The shape of a state with two or more descriptions (`SHAPE_STATE_WITH_DESC`).
const SHAPE_STATE_WITH_DESC: &str = "rectWithTitle";
/// The shape of a composite state (`SHAPE_GROUP`).
const SHAPE_GROUP: &str = "roundedWithTitle";
/// The shape of a concurrency region (`SHAPE_DIVIDER`).
const SHAPE_DIVIDER: &str = "divider";
/// The shape of a note group (`SHAPE_NOTEGROUP`).
const SHAPE_NOTEGROUP: &str = "noteGroup";

/// The corner radius of a state (`rx` and `ry` in `dataFetcher.ts` 360-361).
const STATE_RADIUS: f64 = 10.0;
/// The padding of a state (`padding: 8`, `dataFetcher.ts` 359).
const STATE_PADDING: f64 = 8.0;
/// The padding of a note group (`padding: 16`, `dataFetcher.ts` 415).
const NOTE_GROUP_PADDING: f64 = 16.0;

/// The label of a node while `dataFetcher` runs: one text, or the list of descriptions that
/// `extract` splits into the label and the rest.
#[derive(Debug, Clone, PartialEq)]
enum Label {
    /// A single text.
    Text(String),
    /// Two or more descriptions, flattened; the first is the label once `extract` is done.
    List(Vec<String>),
}

/// The node of `nodeDb`: what the first visit of a state set, and what later visits build on.
#[derive(Debug, Clone)]
struct Entry {
    /// The shape. `None` when the first visit had no type, which the reference leaves undefined.
    shape: Option<String>,
    /// The description, sanitised.
    description: Label,
    /// The class attribute: the classes of the state, then `statediagram-state`.
    css_classes: String,
    /// The styles of the state, from the DB.
    css_styles: Vec<String>,
    /// `group` for a composite, `None` for a state.
    kind: Option<String>,
    /// The direction of a composite.
    dir: Option<String>,
    /// The colour slot of a composite.
    color_index: Option<u32>,
}

/// A node that `dataFetcher` inserts, with the label it has before `extract` splits it.
#[derive(Debug, Clone)]
struct Pending {
    node: Node,
    label: Label,
}

/// The state of one `dataFetcher` run: the DB's states and classes, the nodes and edges it builds,
/// and the counters that the reference keeps in module variables.
struct Fetcher<'a> {
    states: HashMap<&'a str, &'a StateEntry>,
    classes: HashMap<&'a str, &'a ClassDef>,
    look: String,
    /// `flowchart.padding`, which the note nodes take (`dataFetcher.ts` 400).
    note_padding: f64,
    node_db: HashMap<String, Entry>,
    graph_item_count: u64,
    next_color_index: u32,
    container_color_index: HashMap<String, Option<u32>>,
    nodes: Vec<Pending>,
    edges: Vec<Edge>,
}

/// The layout data of a state diagram: its nodes and edges as `getData()` gives them, with the
/// layout's direction (`stateDb.ts` 755-770). The configuration is read as `getData` reads it.
#[must_use]
pub fn get_data(diagram: &StateDiagram, section: &Section<'_>, globals: &Globals) -> LayoutData {
    let mut fetcher = Fetcher {
        states: diagram
            .states
            .iter()
            .map(|state| (state.id.as_str(), state))
            .collect(),
        classes: diagram
            .classes
            .iter()
            .map(|class| (class.id.as_str(), class))
            .collect(),
        look: globals.look.clone(),
        note_padding: section.flowchart.padding,
        node_db: HashMap::new(),
        graph_item_count: 0,
        next_color_index: 0,
        container_color_index: HashMap::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    // `extract` runs `dataFetcher` on the root, whose children get `altFlag` inverted from the
    // `true` it is given (`dataFetcher.ts` 271-275, 475).
    fetcher.setup_doc(None, &diagram.document, false);

    let mut nodes: Vec<Node> = fetcher.nodes.into_iter().map(finish).collect();
    // `getData` writes the label's wrapping width on every node and the minimum width on the
    // non-groups (`stateDb.ts` 755-762).
    for node in &mut nodes {
        node.wrapping_width = Some(section.state.wrapping_width);
        if !node.is_group {
            node.min_width = Some(section.state.min_node_width);
        }
    }

    LayoutData {
        nodes,
        edges: fetcher.edges,
        direction: diagram.layout_direction.clone(),
        default_style: None,
        default_interpolate: None,
        diagram_type: String::new(),
    }
}

/// The label step of `extract`: a node whose label is a list takes its first entry as the label
/// and the rest as its descriptions (`stateDb.ts` 289-299). The error for a group with a second
/// description is raised by the parser, so it cannot reach here.
fn finish(pending: Pending) -> Node {
    let Pending { mut node, label } = pending;
    match label {
        Label::Text(text) => node.label = text,
        Label::List(list) => {
            let mut rest = list;
            node.label = if rest.is_empty() {
                String::new()
            } else {
                rest.remove(0)
            };
            node.description = Some(rest);
        }
    }
    node
}

/// The direction of a document: the last `dir` statement in it, `TB` when there is none
/// (`getDir`, `dataFetcher.ts` 40-50, with the default of `DEFAULT_NESTED_DOC_DIR`).
fn get_dir(doc: &[Statement]) -> String {
    doc.iter()
        .rev()
        .find_map(|statement| match statement {
            Statement::Dir(dir) => Some(dir.value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "TB".to_owned())
}

/// Whether a description is truthy in the reference: a non-empty string, or any list.
fn description_is_truthy(description: &Description) -> bool {
    match description {
        Description::Text(text) => !text.is_empty(),
        Description::List(_) => true,
    }
}

/// The descriptions of a statement, as the list `push` and `[..]` add to a node's description.
/// The reference flattens them one level when it sanitises (`sanitizeTextOrArray`), which is
/// the same as adding them flat here.
fn parts(description: &Description) -> Vec<String> {
    match description {
        Description::Text(text) => vec![text.clone()],
        Description::List(list) => list.clone(),
    }
}

/// The length the reference reads with `.length` on a string or an array, for the one case
/// `dataFetcher` tests it (`newNode.description?.length === 1`).
fn label_len(label: &Label) -> usize {
    match label {
        Label::Text(text) => text.encode_utf16().count(),
        Label::List(list) => list.len(),
    }
}

impl Fetcher<'_> {
    /// `setupDoc` (`dataFetcher.ts` 111-176): each state of the document goes through
    /// [`Fetcher::fetch`], each relation visits its two states and then adds its edge.
    fn setup_doc(&mut self, parent: Option<&StateStmt>, doc: &[Statement], alt: bool) {
        for statement in doc {
            match statement {
                Statement::State(state) => self.fetch(parent, state, alt),
                Statement::Relation(relation) => {
                    self.fetch(parent, &relation.state1, alt);
                    self.fetch(parent, &relation.state2, alt);
                    let is_neo = self.look == "neo";
                    self.edges.push(Edge {
                        id: format!("edge{}", self.graph_item_count),
                        start: relation.state1.id.clone(),
                        end: relation.state2.id.clone(),
                        arrowhead: Some("normal".to_owned()),
                        arrow_type_end: if is_neo {
                            "arrow_barb_neo".to_owned()
                        } else {
                            "arrow_barb".to_owned()
                        },
                        style: vec!["fill:none".to_owned()],
                        label: sanitize_text(relation.description.as_deref().unwrap_or("")),
                        arrowhead_style: "fill: #333".to_owned(),
                        labelpos: "c".to_owned(),
                        label_type: LabelType::Markdown,
                        thickness: "normal".to_owned(),
                        classes: "transition".to_owned(),
                        look: self.look.clone(),
                        ..Edge::default()
                    });
                    self.graph_item_count += 1;
                }
                // The other statements are not states: `dataFetcher` ignores them.
                _ => {}
            }
        }
    }

    /// `dataFetcher` (`dataFetcher.ts` 241-477) for one state: its node in `nodeDb`, its
    /// description and composite shape, its note, and its children.
    fn fetch(&mut self, parent: Option<&StateStmt>, item: &StateStmt, alt: bool) {
        let item_id = item.id.as_str();
        let db_state = self.states.get(item_id).copied();
        let class_str = db_state
            .map(|state| state.classes.join(" "))
            .unwrap_or_default();
        let style = db_state
            .map(|state| state.styles.clone())
            .unwrap_or_default();
        // A styled container opts out of the palette (`dataFetcher.ts` 253-262).
        let user_styled = !class_str.trim().is_empty() || !style.is_empty();

        if item_id != ROOT {
            // The shape of a first visit: a start or an end, then the statement's type, which
            // replaces the shape even when it is missing (`dataFetcher.ts` 272-281).
            let mut shape = Some(SHAPE_STATE.to_owned());
            match item.start {
                Some(true) => shape = Some("stateStart".to_owned()),
                Some(false) => shape = Some("stateEnd".to_owned()),
                None => {}
            }
            if item.kind.as_deref() != Some("default") {
                shape = item.kind.clone();
            }

            if !self.node_db.contains_key(item_id) {
                self.node_db.insert(
                    item_id.to_owned(),
                    Entry {
                        shape,
                        description: Label::Text(sanitize_state_id(item_id)),
                        css_classes: format!("{class_str} statediagram-state"),
                        css_styles: style.clone(),
                        kind: None,
                        dir: None,
                        color_index: None,
                    },
                );
            }

            // The description is taken from the statement, and the shape follows the number of
            // descriptions (`dataFetcher.ts` 299-330).
            let mut entry = self
                .node_db
                .remove(item_id)
                .expect("the entry was inserted above");
            if let Some(description) = item
                .description
                .as_ref()
                .filter(|d| description_is_truthy(d))
            {
                add_description(&mut entry, item_id, description);
            }
            if label_len(&entry.description) == 1
                && entry.shape.as_deref() == Some(SHAPE_STATE_WITH_DESC)
            {
                entry.shape = Some(
                    if entry.kind.as_deref() == Some("group") {
                        SHAPE_GROUP
                    } else {
                        SHAPE_STATE
                    }
                    .to_owned(),
                );
            }

            // A composite: the first statement with a `doc` makes the node a group (`dataFetcher.ts`
            // 333-346). A styled container still spends its colour slot.
            if entry.kind.is_none() && item.doc.is_some() {
                let doc = item.doc.as_deref().unwrap_or_default();
                entry.kind = Some("group".to_owned());
                entry.dir = Some(get_dir(doc));
                let shape = if item.kind.as_deref() == Some(SHAPE_DIVIDER) {
                    SHAPE_DIVIDER
                } else {
                    SHAPE_GROUP
                };
                entry.shape = Some(shape.to_owned());
                entry.color_index = self.color_slot(shape, item_id, parent, user_styled);
                entry.css_classes = format!(
                    "{} statediagram-cluster {}",
                    entry.css_classes,
                    if alt { "statediagram-cluster-alt" } else { "" }
                );
            }

            let count = self.graph_item_count;
            let is_divider = entry.shape.as_deref() == Some(SHAPE_DIVIDER);
            let label = if is_divider {
                // A region's label is empty (`dataFetcher.ts` 371-373).
                Label::Text(String::new())
            } else {
                entry.description.clone()
            };
            let node = Node {
                id: item_id.to_owned(),
                dom_id: format!("state-{item_id}-{count}"),
                label_type: LabelType::Markdown,
                parent_id: parent
                    .filter(|parent| parent.id != ROOT)
                    .map(|parent| parent.id.clone()),
                padding: STATE_PADDING,
                css_styles: entry.css_styles.clone(),
                css_classes: entry.css_classes.clone(),
                dir: entry.dir.clone(),
                look: self.look.clone(),
                is_group: entry.kind.as_deref() == Some("group"),
                // The reference leaves an undefined shape undefined, and the renderer names it
                // in its error (`nodes.ts` 31-35); an empty string stands for it here.
                shape: entry.shape.clone().unwrap_or_default(),
                color_index: entry.color_index,
                rx: Some(STATE_RADIUS),
                ry: Some(STATE_RADIUS),
                node_type: entry.kind.clone(),
                center_label: Some(true),
                ..Node::default()
            };
            let node_data = Pending { node, label };

            if let Some(note) = &item.note {
                let note_id = format!("{item_id}----note-{count}");
                let parent_node_id = format!("{item_id}----parent");
                // The note is a leaf inside the note group, which holds the state's note and
                // the state's own class attribute (`dataFetcher.ts` 379-428).
                let note_data = Pending {
                    node: Node {
                        id: note_id.clone(),
                        dom_id: format!("state-{item_id}----note-{count}"),
                        label_type: LabelType::Markdown,
                        parent_id: Some(parent_node_id.clone()),
                        padding: self.note_padding,
                        css_classes: "statediagram-note".to_owned(),
                        look: self.look.clone(),
                        shape: "note".to_owned(),
                        node_type: Some("node".to_owned()),
                        position: Some(note.position.clone()),
                        ..Node::default()
                    },
                    label: Label::Text(note.text.clone()),
                };
                let group_data = Pending {
                    node: Node {
                        id: parent_node_id.clone(),
                        dom_id: format!("state-{item_id}----parent-{count}"),
                        parent_id: None,
                        padding: NOTE_GROUP_PADDING,
                        css_classes: entry.css_classes.clone(),
                        look: self.look.clone(),
                        shape: SHAPE_NOTEGROUP.to_owned(),
                        is_group: true,
                        node_type: Some("group".to_owned()),
                        position: Some(note.position.clone()),
                        ..Node::default()
                    },
                    label: Label::Text(note.text.clone()),
                };
                self.graph_item_count += 1;
                self.insert_or_update(group_data);
                self.insert_or_update(note_data);
                self.insert_or_update(node_data);

                let (from, to) = if note.position == "left of" {
                    (note_id, item_id.to_owned())
                } else {
                    (item_id.to_owned(), note_id)
                };
                self.edges.push(Edge {
                    id: format!("{from}-{to}"),
                    start: from,
                    end: to,
                    arrowhead: Some("none".to_owned()),
                    arrow_type_end: String::new(),
                    style: vec!["fill:none".to_owned()],
                    classes: "transition note-edge".to_owned(),
                    // The dashes are declared on the edge, as `insertEdge` writes them under
                    // `look: neo` (`dataFetcher.ts` 430-441).
                    pattern: "dashed".to_owned(),
                    arrowhead_style: "fill: #333".to_owned(),
                    labelpos: "c".to_owned(),
                    label_type: LabelType::Markdown,
                    thickness: "normal".to_owned(),
                    look: self.look.clone(),
                    ..Edge::default()
                });
            } else {
                self.insert_or_update(node_data);
            }
            self.node_db.insert(item_id.to_owned(), entry);
        }
        if let Some(doc) = &item.doc {
            self.setup_doc(Some(item), doc, !alt);
        }
    }

    /// The colour slot of a container (`colorSlotFor`, `dataFetcher.ts` 75-94). A region takes
    /// its parent's slot; any other container takes the next one. A styled container records no
    /// slot, so its regions inherit that.
    fn color_slot(
        &mut self,
        shape: &str,
        item_id: &str,
        parent: Option<&StateStmt>,
        user_styled: bool,
    ) -> Option<u32> {
        if shape == SHAPE_DIVIDER {
            let inherited = parent
                .and_then(|parent| self.container_color_index.get(&parent.id))
                .copied();
            if let Some(inherited) = inherited {
                self.container_color_index
                    .insert(item_id.to_owned(), inherited);
                return inherited;
            }
        }
        let slot = self.next_color_index;
        self.next_color_index += 1;
        let effective = if user_styled { None } else { Some(slot) };
        self.container_color_index
            .insert(item_id.to_owned(), effective);
        effective
    }

    /// `insertOrUpdateNode` (`dataFetcher.ts` 153-173): a node with an empty id, or one of the
    /// two placeholder ids, is not inserted. The compiled styles come from the node's classes. A
    /// node that is already there takes the new values (`Object.assign`), except the parent and
    /// the note side, which a visit without them leaves as they are.
    fn insert_or_update(&mut self, mut data: Pending) {
        let id = data.node.id.as_str();
        if id.is_empty() || id == "</join></fork>" || id == "</choice>" {
            return;
        }
        if !data.node.css_classes.is_empty() {
            let mut compiled = Vec::new();
            for class in data.node.css_classes.split(' ') {
                if let Some(definition) = self.classes.get(class) {
                    compiled.extend(definition.styles.iter().cloned());
                }
            }
            data.node.css_compiled_styles = compiled;
        }
        match self.nodes.iter_mut().find(|node| node.node.id == id) {
            Some(existing) => {
                let parent_id = data
                    .node
                    .parent_id
                    .take()
                    .or(existing.node.parent_id.take());
                let position = data.node.position.take().or(existing.node.position.take());
                existing.node = Node {
                    parent_id,
                    position,
                    ..data.node
                };
                existing.label = data.label;
            }
            None => self.nodes.push(data),
        }
    }
}

/// `addDescription` as `dataFetcher` does it (`dataFetcher.ts` 299-330): the first description
/// is the state's label, a second one makes the state a `rectWithTitle` with the descriptions as
/// rows, and each later one is added to them. The whole list is sanitised after each change.
fn add_description(entry: &mut Entry, item_id: &str, description: &Description) {
    let next = match std::mem::replace(&mut entry.description, Label::Text(String::new())) {
        Label::List(mut list) => {
            entry.shape = Some(SHAPE_STATE_WITH_DESC.to_owned());
            list.extend(parts(description));
            Label::List(list)
        }
        Label::Text(text) if !text.is_empty() => {
            entry.shape = Some(SHAPE_STATE_WITH_DESC.to_owned());
            if text == item_id {
                // A label that is the id itself is replaced by the first description.
                Label::List(parts(description))
            } else {
                let mut list = vec![text];
                list.extend(parts(description));
                Label::List(list)
            }
        }
        Label::Text(_) => {
            entry.shape = Some(SHAPE_STATE.to_owned());
            match description {
                Description::Text(text) => Label::Text(text.clone()),
                Description::List(list) => Label::List(list.clone()),
            }
        }
    };
    entry.description = match next {
        Label::Text(text) => Label::Text(sanitize_text(&text)),
        Label::List(list) => Label::List(list.iter().map(|text| sanitize_text(text)).collect()),
    };
}
