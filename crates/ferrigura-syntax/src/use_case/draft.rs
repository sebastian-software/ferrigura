//! The drafts the syntax tree hands to the model builder (`usecaseModelBuilder.ts` 29-150). The
//! visitor makes them in the order of the statements; the builder checks the whole diagram in
//! `finalize`. Spans are UTF-16 code units of the prepared text, as the reference's are, so the
//! AST needs no conversion.

use ferrigura_model::use_case::{GraphStatement, LabelType, RelationshipType, Span, UseCaseShape};

/// A place in the prepared text: its span, and the 1-based line and column of its start, for the
/// error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Location {
    pub(crate) span: Span,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

/// A label: its text as the grammar gives it, its kind and its span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Label {
    pub(crate) text: String,
    pub(crate) kind: LabelType,
    pub(crate) span: Span,
}

/// The value of a metadata property: a string (an identifier or a quoted string, without the
/// quotes of a PLAIN_STRING the visitor removes, see `usecase.visitor.ts`) or a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MetadataValue {
    Text(String),
    Bool(bool),
}

/// One `key: value` of a metadata block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MetadataProperty {
    pub(crate) key: String,
    pub(crate) value: MetadataValue,
    pub(crate) span: Span,
    pub(crate) key_span: Span,
    pub(crate) value_span: Span,
    pub(crate) location: Location,
}

/// A metadata block `@{ … }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Metadata {
    pub(crate) properties: Vec<MetadataProperty>,
    pub(crate) location: Location,
}

/// The kind of an element draft: an actor or a use case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElementKind {
    Actor,
    UseCase,
}

/// An actor or a use case as declared (`addElement`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Element {
    pub(crate) id: String,
    pub(crate) kind: ElementKind,
    pub(crate) label: Label,
    pub(crate) location: Location,
    /// Whether the id was generated from a quoted label.
    pub(crate) generated: bool,
    pub(crate) parent_id: Option<String>,
    pub(crate) parent_location: Option<Location>,
    pub(crate) shape: Option<UseCaseShape>,
    pub(crate) metadata: Option<Metadata>,
    pub(crate) stereotype: Option<String>,
    pub(crate) stereotype_span: Option<Span>,
    pub(crate) classes: Vec<String>,
}

/// A system boundary as declared (`addBoundary`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Boundary {
    pub(crate) id: String,
    pub(crate) label: Label,
    pub(crate) location: Location,
    pub(crate) generated: bool,
    pub(crate) classes: Vec<String>,
}

/// A JSON table as declared (`addJson`): the object and its key order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JsonDraft {
    pub(crate) id: String,
    pub(crate) value: ferrigura_model::value::Value,
    pub(crate) property_order: std::collections::BTreeMap<String, Vec<String>>,
    pub(crate) location: Location,
    pub(crate) classes: Vec<String>,
}

/// An end of a relationship: the id, the label the reference gives it, and what is known of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Endpoint {
    pub(crate) id: String,
    pub(crate) label: Label,
    pub(crate) location: Location,
    pub(crate) generated: bool,
    /// The endpoint is declared by this reference (`actor A` or `UC1` with a glyph form).
    pub(crate) declaration: bool,
    /// The endpoint carries classes (`:::`) on this reference.
    pub(crate) classes_on_reference: bool,
}

/// A relationship as written (`addRelationship`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelationshipDraft {
    pub(crate) source: Endpoint,
    pub(crate) target: Endpoint,
    pub(crate) location: Location,
    pub(crate) explicit_id: Option<String>,
    pub(crate) explicit_id_location: Option<Location>,
    pub(crate) relationship_type: RelationshipType,
    pub(crate) arrow_type: u8,
    pub(crate) label: Option<Label>,
    pub(crate) minlen: u32,
}

/// A note (`addNote`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoteDraft {
    pub(crate) target: String,
    pub(crate) target_location: Location,
    pub(crate) label: Label,
    pub(crate) location: Location,
}

/// A metadata assignment to an actor, a use case, a boundary or an edge (`addMetadataAssignment`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MetadataAssignment {
    pub(crate) target: String,
    pub(crate) target_location: Location,
    pub(crate) metadata: Metadata,
    pub(crate) statement: GraphStatement,
}

/// A class definition (`addClassDef`): the ids and the styles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClassDefDraft {
    pub(crate) ids: Vec<String>,
    pub(crate) styles: Vec<String>,
}

/// A class assignment (`addClassAssignment`): the targets, each with its location, and the classes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClassAssignment {
    pub(crate) targets: Vec<(String, Location)>,
    pub(crate) classes: Vec<String>,
}

/// A style assignment (`addStyleAssignment`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StyleAssignment {
    pub(crate) target: String,
    pub(crate) location: Location,
    pub(crate) styles: Vec<String>,
}

/// The kind of an id the DB knows (`UsecaseSymbolKind`): the builder's `symbols` map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SymbolKind {
    Actor,
    UseCase,
    Boundary,
    Json,
    Edge,
}
