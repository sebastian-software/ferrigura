//! Canonical model of a class diagram, as the class DB of `classDb.ts` holds it after parsing, less
//! what the input does not contain or only rendering needs (see `docs-port/class/PLAN.md`, section
//! 5).
//!
//! Lists keep the DB's order: classes, relations, notes, interfaces, namespaces and style classes in
//! the order they are created. A field the DB leaves `undefined` is `None` (serialised as `null`).

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed class diagram: its header texts, its direction, and the DB's classes, relations, notes,
/// interfaces, namespaces and style classes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Title from the front matter, sanitised as `setDiagramTitle` does; empty if none.
    pub title: String,
    /// Accessible title from `accTitle`, sanitised, empty if absent.
    pub acc_title: String,
    /// Accessible description from `accDescr`, sanitised, empty if absent.
    pub acc_descr: String,
    /// Direction as the DB keeps it: `TB` by default, then the last `direction` statement.
    pub direction: String,
    /// Classes in the order the DB creates them.
    pub classes: Vec<ClassNode>,
    /// Relations in the order they are added.
    pub relations: Vec<ClassRelation>,
    /// Notes in the order they are added; the id is `note<index>`.
    pub notes: Vec<ClassNote>,
    /// Lollipop interfaces in the order they are added.
    pub interfaces: Vec<ClassInterface>,
    /// Namespaces in the order the DB creates them, ancestors before their descendants.
    pub namespaces: Vec<ClassNamespace>,
    /// Style classes from `classDef`, in the order they are first named.
    pub style_classes: Vec<ClassStyle>,
}

impl ClassDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`] and the default direction.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            direction: "TB".to_owned(),
            classes: Vec::new(),
            relations: Vec::new(),
            notes: Vec::new(),
            interfaces: Vec::new(),
            namespaces: Vec::new(),
            style_classes: Vec::new(),
        }
    }

    /// The class with this id, if the diagram has one.
    #[must_use]
    pub fn class(&self, id: &str) -> Option<&ClassNode> {
        self.classes.iter().find(|class| class.id == id)
    }

    /// The namespace with this id, if the diagram has one.
    #[must_use]
    pub fn namespace(&self, id: &str) -> Option<&ClassNamespace> {
        self.namespaces.iter().find(|namespace| namespace.id == id)
    }
}

impl Default for ClassDiagram {
    fn default() -> Self {
        Self::new()
    }
}

/// A class: its name, its generic type and label, its members and methods, and its styling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassNode {
    /// The class name without its generic type; sanitised.
    pub id: String,
    /// The generic type between the first two `~`, sanitised; empty if none.
    #[serde(rename = "type")]
    pub class_type: String,
    /// The label: the name, or the label given in brackets.
    pub label: String,
    /// The text the class box shows: the label, then the generic type (`&lt;T&gt;` when the class
    /// is added, `<T>` when a label is set).
    pub text: String,
    /// The CSS classes: `default`, then one space and each class applied to it.
    pub css_classes: String,
    /// Annotations in the order they are given (`<<x>>`), sanitised by the member rule.
    pub annotations: Vec<String>,
    /// Attributes in source order.
    pub members: Vec<ClassMember>,
    /// Methods in source order.
    pub methods: Vec<ClassMember>,
    /// Styles from `style`, `classDef` and `cssStyle`, in order.
    pub styles: Vec<String>,
    /// The namespace the class belongs to, `None` outside namespaces.
    pub parent: Option<String>,
    /// The link, trimmed and passed through `sanitizeUrl` unless `securityLevel` is `loose`.
    pub link: Option<String>,
    /// The target of the link, `_blank` by default, `_top` under `sandbox`.
    pub link_target: Option<String>,
    /// The tooltip, sanitised.
    pub tooltip: Option<String>,
    /// `true` once a callback is set on the class; `None` otherwise. The callback itself is not kept.
    pub have_callback: Option<bool>,
}

/// A member of a class: an attribute or a method, as `ClassMember` parses it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassMember {
    /// `method` when the text has a `)` after its first character, `attribute` otherwise.
    pub member_type: MemberType,
    /// `#`, `+`, `~`, `-`, or empty when the text starts with none of them.
    pub visibility: String,
    /// `$` (static) or `*` (abstract), or empty.
    pub classifier: String,
    /// The name, with the parameters and the return type removed.
    pub id: String,
    /// The parameters of a method, trimmed; `None` for an attribute.
    pub parameters: Option<String>,
    /// The return type of a method, trimmed, without its classifier; `None` for an attribute.
    pub return_type: Option<String>,
    /// The DB's `text`, which the renderer shows: a backslash and the visibility, the name, and `(params) : type` for a
    /// method, with `<` and `>` written as `&lt;` and `&gt;`.
    pub text: String,
}

/// Whether a member is a method or an attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberType {
    /// A member with a `)` after its first character.
    Method,
    /// Any other member.
    Attribute,
}

/// A relation between two classes: the two ends, the line, and the labels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassRelation {
    /// The start class (or interface) id.
    pub id1: String,
    /// The end class (or interface) id.
    pub id2: String,
    /// The cardinality at the start, sanitised and trimmed; `none` when none is given.
    pub relation_title1: String,
    /// The cardinality at the end, sanitised and trimmed; `none` when none is given.
    pub relation_title2: String,
    /// The label of the relation, `None` without a label.
    pub title: Option<String>,
    /// The marker at the start of the line.
    pub type1: RelationEnd,
    /// The marker at the end of the line.
    pub type2: RelationEnd,
    /// Solid or dotted line.
    pub line_type: LineType,
}

/// The marker at one end of a relation, by the name of the DB's `getArrowMarker`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationEnd {
    /// No marker.
    None,
    /// A hollow diamond (`o`).
    Aggregation,
    /// An arrow head of `<|` or `|>`.
    Extension,
    /// A filled diamond (`*`).
    Composition,
    /// An open arrow head (`<` or `>`).
    Dependency,
    /// A circle (`()`).
    Lollipop,
}

/// The line of a relation: `--` or `..`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineType {
    /// `--`.
    Solid,
    /// `..`.
    Dotted,
}

/// A note, as `addNote` keeps it. The text is not sanitised here; the renderer sanitises it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassNote {
    /// `note` followed by the index.
    pub id: String,
    /// The class the note is for, as written; `None` for a note without a class.
    pub class: Option<String>,
    /// The text of the note, as written between the quotes.
    pub text: String,
    /// The position of the note in the order the notes are added.
    pub index: usize,
    /// The namespace the note belongs to, `None` outside namespaces.
    pub parent: Option<String>,
}

/// A lollipop interface: the label of the end that is not a class, and the class it belongs to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassInterface {
    /// `interface` followed by the index.
    pub id: String,
    /// The label, as written for the end that is not a class.
    pub label: String,
    /// The class the interface belongs to.
    pub class_id: String,
}

/// A namespace: its label, its place in the tree, and the classes and notes declared in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassNamespace {
    /// The qualified name, parts joined with `.`.
    pub id: String,
    /// The label: the name, or the label given in brackets for the namespace itself.
    pub label: String,
    /// The enclosing namespace, `None` at the top.
    pub parent: Option<String>,
    /// Whether the namespace was declared, as opposed to created as the ancestor of a dotted name.
    pub explicit: bool,
    /// The classes declared directly in the namespace, in order, without repeats.
    pub classes: Vec<String>,
    /// The notes declared directly in the namespace, in order, without repeats.
    pub notes: Vec<String>,
}

/// A style class from `classDef`: its name and its declarations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassStyle {
    /// The name of the style class.
    pub id: String,
    /// The declarations, each as written in its `classDef` statement, in order.
    pub styles: Vec<String>,
}
