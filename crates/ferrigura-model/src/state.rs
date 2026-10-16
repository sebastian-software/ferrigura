//! Canonical model of a state diagram, as the state DB of `stateDb.ts` holds it after parsing, less
//! what the input does not contain or only rendering needs (see `docs-port/state/PLAN.md`, sections
//! 5 and 6).
//!
//! The document is the root statement tree after the reference's `docTranslator`: each statement
//! is written with the keys its JavaScript object has, and only those (`undefined` is left out).
//! Statements are internally tagged by `stmt`, except that a string statement stays a JSON string,
//! as the reference keeps the values of `accTitle`, `hide empty description` and similar lines in
//! the document.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed state diagram: the header texts, the directions, the root document, and the DB's
/// states, relations, classes and links.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Title from the front matter, sanitised; empty if there is none.
    pub title: String,
    /// Accessible title from `accTitle`, sanitised, without leading blanks; empty if absent.
    pub acc_title: String,
    /// Accessible description from `accDescr`, sanitised, with the indentation after each line
    /// break removed; empty if absent.
    pub acc_descr: String,
    /// The DB's `getDirection()`: the first `dir` statement of the root document, `TB` if none.
    pub direction: String,
    /// The direction the layout uses (`getData().direction`): the last `dir` statement of the
    /// root document, `TB` if none. It differs from `direction` when the document gives several.
    pub layout_direction: String,
    /// The root statements, after `docTranslator`.
    pub document: Vec<Statement>,
    /// The states the root statements create, in the order of their first mention.
    pub states: Vec<StateEntry>,
    /// The relations of the root document, in source order.
    pub relations: Vec<Relation>,
    /// The class definitions, in the order they are first named.
    pub classes: Vec<ClassDef>,
    /// One entry per `click` statement of the root document, in source order.
    pub links: Vec<Link>,
}

/// A statement of a document. A string is a statement the DB ignores but the document keeps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Statement {
    /// A state, a divider, a fork, a join, a choice, a note, a start or an end.
    State(StateStmt),
    /// A transition between two states.
    Relation(RelationStmt),
    /// A direction statement.
    Dir(DirStmt),
    /// A `classDef` line.
    ClassDef(ClassDefStmt),
    /// A `style` line.
    Style(StyleStmt),
    /// A `class` line.
    ApplyClass(ApplyClassStmt),
    /// A `click` line.
    Click(ClickStmt),
    /// A statement the DB does not use: `scale`, `hide empty description`, an accessibility text,
    /// a composite's bare name, or a floating note.
    Text(String),
}

/// A state statement as the parser builds it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateStmt {
    /// Always `"state"`.
    pub stmt: String,
    /// The id, trimmed for the states of the document (`docTranslator`).
    pub id: String,
    /// `default`, `fork`, `join`, `choice` or `divider`. Absent for a note statement.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The description: a string, or the list of two descriptions of `state "x" as S:y`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
    /// The classes given with `:::`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// The statements of a composite state, after the dividers were split off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<Vec<Statement>>,
    /// The note of a state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<Note>,
    /// Set for `[*]` only: `true` for the start, `false` for the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<bool>,
}

/// A description as the grammar gives it: a single string, or the two parts of an aliased state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Description {
    /// One description.
    Text(String),
    /// The description and the part after the colon in the id.
    List(Vec<String>),
}

/// A note of a state: the position as written (`left of` or `right of`, in any case), and its
/// text, trimmed and sanitised.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// The position as the source writes it, trimmed.
    pub position: String,
    /// The text, trimmed and sanitised.
    pub text: String,
}

/// A transition between two states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationStmt {
    /// Always `"relation"`.
    pub stmt: String,
    /// The source state.
    pub state1: StateStmt,
    /// The target state.
    pub state2: StateStmt,
    /// The label, if one is given after a colon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A `direction` statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirStmt {
    /// Always `"dir"`.
    pub stmt: String,
    /// `TB`, `BT`, `RL` or `LR`.
    pub value: String,
}

/// A `classDef` statement, before it reaches the DB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassDefStmt {
    /// Always `"classDef"`.
    pub stmt: String,
    /// The class name, trimmed.
    pub id: String,
    /// The style list as written, trimmed.
    pub classes: String,
}

/// A `style` statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleStmt {
    /// Always `"style"`.
    pub stmt: String,
    /// The state ids, as written, trimmed.
    pub id: String,
    /// The style list as written, trimmed.
    #[serde(rename = "styleClass")]
    pub style_class: String,
}

/// A `class` statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplyClassStmt {
    /// Always `"applyClass"`.
    pub stmt: String,
    /// The state ids, as written, trimmed.
    pub id: String,
    /// The class names, as written, trimmed.
    #[serde(rename = "styleClass")]
    pub style_class: String,
}

/// A `click` statement. The id is the state statement of the clicked id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClickStmt {
    /// Always `"click"`.
    pub stmt: String,
    /// The clicked state.
    pub id: StateStmt,
    /// The address, quotes included, as written.
    pub url: String,
    /// The tooltip, quotes included, as written; empty with `href`.
    pub tooltip: String,
}

/// A state of the DB: the root document's state, with its descriptions, classes and styles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateEntry {
    /// The id, trimmed.
    pub id: String,
    /// `default`, `fork`, `join`, `choice` or `divider`: the first type the state was given.
    #[serde(rename = "type")]
    pub kind: String,
    /// The descriptions, each sanitised, in the order they were added.
    pub descriptions: Vec<String>,
    /// The composite's statements, when the state has a `doc`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<Vec<Statement>>,
    /// The note, sanitised.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<Note>,
    /// The class names applied to the state, in order.
    pub classes: Vec<String>,
    /// The styles of the state, from its last `style` statement.
    pub styles: Vec<String>,
    /// The text styles of the state. Empty in every parse: the reference never fills them here.
    pub text_styles: Vec<String>,
}

/// A transition of the DB's root document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    /// The id of the source state.
    pub id1: String,
    /// The id of the target state.
    pub id2: String,
    /// The label, sanitised; empty when there is none.
    pub relation_title: String,
}

/// A class definition of the DB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassDef {
    /// The class name.
    pub id: String,
    /// The style attributes, one per comma-separated part, with the first `;` of a part removed.
    pub styles: Vec<String>,
    /// The attributes that mention `color`, with `fill` and `color` renamed to `bgFill` and `fill`.
    pub text_styles: Vec<String>,
}

/// A link of the DB: the id of the clicked state, its address and its tooltip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    /// The id of the clicked state.
    pub id: String,
    /// The address, quotes included.
    pub url: String,
    /// The tooltip, quotes included.
    pub tooltip: String,
}

impl StateDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`] and the default directions.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            direction: "TB".to_owned(),
            layout_direction: "TB".to_owned(),
            document: Vec::new(),
            states: Vec::new(),
            relations: Vec::new(),
            classes: Vec::new(),
            links: Vec::new(),
        }
    }
}

impl Default for StateDiagram {
    fn default() -> Self {
        Self::new()
    }
}

impl StateStmt {
    /// A state statement with the given id and type, and no other key.
    #[must_use]
    pub fn new(id: impl Into<String>, kind: Option<&str>) -> Self {
        Self {
            stmt: "state".to_owned(),
            id: id.into(),
            kind: kind.map(str::to_owned),
            description: None,
            classes: None,
            doc: None,
            note: None,
            start: None,
        }
    }
}
