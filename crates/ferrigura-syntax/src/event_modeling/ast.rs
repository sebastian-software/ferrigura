//! The syntax tree of `event-modeling.langium`: the statements of the text, with the values their
//! tokens carry. The reference reads Langium's AST; the DB reads these fields.
//!
//! Only the header texts are converted by the grammar's value rules, and those rules do not apply
//! to this grammar's terminals (`EM_TITLE` and the others are not the names the value converter
//! matches), so the header texts are the raw token texts, leading blanks included.

/// The whole diagram. The header texts are the last statement of each kind, as Langium assigns
/// them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventModel {
    /// The last `title` token's text.
    pub title: Option<String>,
    /// The last `accTitle:` token's text.
    pub acc_title: Option<String>,
    /// The last `accDescr` token's text.
    pub acc_descr: Option<String>,
    /// The `entity` statements, which the DB does not read.
    pub model_entities: Vec<ModelEntity>,
    /// The frames, in the order of the text.
    pub frames: Vec<Frame>,
    /// The `data` statements.
    pub data_entities: Vec<DataEntity>,
    /// The `note` statements, which the DB does not read.
    pub note_entities: Vec<NoteEntity>,
    /// The `gwt` statements, which the DB does not read.
    pub gwt_entities: Vec<Gwt>,
}

/// The kind of a frame: `tf`/`timeframe` or `rf`/`resetframe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    /// A time frame.
    Time,
    /// A reset frame.
    Reset,
}

/// A frame statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// The kind of the frame.
    pub kind: FrameKind,
    /// The frame number, as written.
    pub name: String,
    /// The entity type keyword, as written.
    pub model_entity_type: String,
    /// The qualified name, with the dots and without the blanks around them.
    pub entity_identifier: String,
    /// The numbers after each `->>`, as written.
    pub source_frames: Vec<String>,
    /// The name inside `[[ ]]`, as written.
    pub data_reference: Option<String>,
    /// The inline data token, quotes or braces included, with its optional type dropped.
    pub data_inline: Option<String>,
    /// The byte offset of the frame's keyword in the parsed text, for error positions.
    pub offset: usize,
}

/// An `entity` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntity {
    /// The qualified name.
    pub name: String,
}

/// A `data` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataEntity {
    /// The name of the block.
    pub name: String,
    /// The block token's text.
    pub data_block_value: String,
}

/// A `note` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEntity {
    /// The frame number the note is attached to, as written.
    pub source_frame: String,
    /// The block token's text.
    pub data_block_value: String,
}

/// A `gwt` statement: a frame and its given, when and then statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gwt {
    /// The frame number, as written.
    pub source_frame: String,
    /// The `given` statements.
    pub given: Vec<Statement>,
    /// The `when` statements, empty when the statement has none.
    pub when: Vec<Statement>,
    /// The `then` statements.
    pub then: Vec<Statement>,
}

/// One statement of a `gwt`: an entity type and an entity name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// The entity type keyword.
    pub model_entity_type: String,
    /// The entity name, as written.
    pub entity: String,
}
