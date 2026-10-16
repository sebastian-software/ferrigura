//! Canonical model of a sequence diagram, as the sequence DB of `sequenceDb.ts` holds it after
//! parsing, less the notes list and the state that only rendering reads (see
//! `docs-port/sequence/PLAN.md`, section 7).
//!
//! Field names are the DB's names in camelCase. A field the DB leaves `undefined` is `None`, which
//! serialises as `null`. Lists keep the DB's order: actors in declaration order (a repeated
//! declaration keeps its place), messages in the order the DB applies them, boxes in opening order.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;
use crate::value::Value;

/// The kind of a message or control item, by the name of its `LINETYPE` value in `sequenceDb.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LineType {
    /// `SOLID` (0), the arrow `->>`.
    Solid,
    /// `DOTTED` (1), `-->>`.
    Dotted,
    /// `NOTE` (2).
    Note,
    /// `SOLID_CROSS` (3), `-x`.
    SolidCross,
    /// `DOTTED_CROSS` (4), `--x`.
    DottedCross,
    /// `SOLID_OPEN` (5), `->`.
    SolidOpen,
    /// `DOTTED_OPEN` (6), `-->`.
    DottedOpen,
    /// `LOOP_START` (10).
    LoopStart,
    /// `LOOP_END` (11).
    LoopEnd,
    /// `ALT_START` (12).
    AltStart,
    /// `ALT_ELSE` (13).
    AltElse,
    /// `ALT_END` (14).
    AltEnd,
    /// `OPT_START` (15).
    OptStart,
    /// `OPT_END` (16).
    OptEnd,
    /// `ACTIVE_START` (17).
    ActiveStart,
    /// `ACTIVE_END` (18).
    ActiveEnd,
    /// `PAR_START` (19).
    ParStart,
    /// `PAR_AND` (20).
    ParAnd,
    /// `PAR_END` (21).
    ParEnd,
    /// `RECT_START` (22).
    RectStart,
    /// `RECT_END` (23).
    RectEnd,
    /// `SOLID_POINT` (24), `-)`.
    SolidPoint,
    /// `DOTTED_POINT` (25), `--)`.
    DottedPoint,
    /// `AUTONUMBER` (26).
    Autonumber,
    /// `CRITICAL_START` (27).
    CriticalStart,
    /// `CRITICAL_OPTION` (28).
    CriticalOption,
    /// `CRITICAL_END` (29).
    CriticalEnd,
    /// `BREAK_START` (30).
    BreakStart,
    /// `BREAK_END` (31).
    BreakEnd,
    /// `PAR_OVER_START` (32).
    ParOverStart,
    /// `BIDIRECTIONAL_SOLID` (33), `<<->>`.
    BidirectionalSolid,
    /// `BIDIRECTIONAL_DOTTED` (34), `<<-->>`.
    BidirectionalDotted,
    /// `SOLID_TOP` (41), `-|\`.
    SolidTop,
    /// `SOLID_BOTTOM` (42), `-|/`.
    SolidBottom,
    /// `STICK_TOP` (43), `-\\`.
    StickTop,
    /// `STICK_BOTTOM` (44), `-//`.
    StickBottom,
    /// `SOLID_ARROW_TOP_REVERSE` (45), `/|-`.
    SolidArrowTopReverse,
    /// `SOLID_ARROW_BOTTOM_REVERSE` (46), `\|-`.
    SolidArrowBottomReverse,
    /// `STICK_ARROW_TOP_REVERSE` (47), `//-`.
    StickArrowTopReverse,
    /// `STICK_ARROW_BOTTOM_REVERSE` (48), `\\-`.
    StickArrowBottomReverse,
    /// `SOLID_TOP_DOTTED` (51), `--|\`.
    SolidTopDotted,
    /// `SOLID_BOTTOM_DOTTED` (52), `--|/`.
    SolidBottomDotted,
    /// `STICK_TOP_DOTTED` (53), `--\\`.
    StickTopDotted,
    /// `STICK_BOTTOM_DOTTED` (54), `--//`.
    StickBottomDotted,
    /// `SOLID_ARROW_TOP_REVERSE_DOTTED` (55), `/|--`.
    SolidArrowTopReverseDotted,
    /// `SOLID_ARROW_BOTTOM_REVERSE_DOTTED` (56), `\|--`.
    SolidArrowBottomReverseDotted,
    /// `STICK_ARROW_TOP_REVERSE_DOTTED` (57), `//--`.
    StickArrowTopReverseDotted,
    /// `STICK_ARROW_BOTTOM_REVERSE_DOTTED` (58), `\\--`.
    StickArrowBottomReverseDotted,
    /// `CENTRAL_CONNECTION` (59).
    CentralConnection,
    /// `CENTRAL_CONNECTION_REVERSE` (60).
    CentralConnectionReverse,
    /// `CENTRAL_CONNECTION_DUAL` (61).
    CentralConnectionDual,
}

/// Where a note is placed (`PLACEMENT` in `sequenceDb.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Placement {
    /// `left of` (0).
    LeftOf,
    /// `right of` (1).
    RightOf,
    /// `over` (2).
    Over,
}

/// The central connection a message carries (`centralConnection` in the DB).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CentralConnection {
    /// No connection, the DB's `0`.
    #[serde(rename = "NONE")]
    NoConnection,
    /// `()` on the receiving side (59).
    #[serde(rename = "CENTRAL_CONNECTION")]
    Central,
    /// `()` on the sending side, written before the arrow (60).
    #[serde(rename = "CENTRAL_CONNECTION_REVERSE")]
    Reverse,
    /// `()` on both sides (61).
    #[serde(rename = "CENTRAL_CONNECTION_DUAL")]
    Dual,
}

/// The sequence numbering of an `autonumber` item. Undefined numbers are `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Autonumber {
    /// The first number (`autonumber 5`), `None` when none was given.
    pub start: Option<f64>,
    /// The step (`autonumber 5 2`), `None` when none was given.
    pub step: Option<f64>,
    /// Whether the numbers are shown (`autonumber off` hides them).
    pub visible: bool,
}

/// The text of a message, or the numbering of an `autonumber` item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageText {
    /// The text, as the grammar gives it. Empty when there is none.
    Text(String),
    /// The numbering of an `autonumber` item.
    Autonumber(Autonumber),
}

/// An actor or participant, as the DB's `addActor` leaves it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceActor {
    /// The id the messages refer to.
    pub id: String,
    /// The name, which is the id unless an alias or a description replaced it.
    pub name: String,
    /// The description shown in the actor's box, the name when none was given.
    pub description: String,
    /// `participant`, `actor`, or the `type` of the metadata (`entity`, `database`, …).
    #[serde(rename = "type")]
    pub actor_type: String,
    /// Whether the label wraps.
    pub wrap: bool,
    /// Index into [`SequenceDiagram::boxes`] of the box the actor is in, `None` if none.
    #[serde(rename = "box")]
    pub box_index: Option<usize>,
    /// The actor declared before this one, as the DB records it. A repeated declaration with a
    /// description records itself (`docs-port/sequence/PLAN.md`, Q4).
    pub prev_actor: Option<String>,
    /// The actor declared after this one, `None` until one is declared.
    pub next_actor: Option<String>,
    /// The links of the actor, as the DB holds them.
    pub links: BTreeMap<String, Value>,
    /// The properties of the actor, as the DB holds them.
    pub properties: BTreeMap<String, Value>,
}

/// A `box` … `end` group of actors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceBox {
    /// The title of the box, `None` when the box has none.
    pub name: Option<String>,
    /// Whether the title wraps.
    pub wrap: bool,
    /// The colour as written, or `transparent` when the first word is not a colour.
    pub fill: String,
    /// The ids of the actors declared in the box, in order; a repeated id is kept as the DB keeps it.
    pub actor_keys: Vec<String>,
}

/// One item of the message list: a message, a note, a control item (`loop`, `activate`, …) or an
/// `autonumber` item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceMessage {
    /// The index of the item in the list, as a string.
    pub id: String,
    /// The sending actor; `None` for control items and `autonumber`.
    pub from: Option<String>,
    /// The receiving actor; `None` for control items and `autonumber`.
    pub to: Option<String>,
    /// The text, or the numbering of an `autonumber` item.
    pub message: MessageText,
    /// Whether the text wraps.
    pub wrap: bool,
    /// The kind of the item.
    #[serde(rename = "type")]
    pub line_type: LineType,
    /// Whether the message activates the receiver (`+`). `None` for notes and `autonumber`, which
    /// the DB gives no such field.
    pub activate: Option<bool>,
    /// The central connection of the message. `None` for notes and `autonumber`.
    pub central_connection: Option<CentralConnection>,
    /// Where a note is placed. `None` for everything but notes.
    pub placement: Option<Placement>,
}

/// A parsed sequence diagram: its header texts, its actors, boxes and message list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Title: the body's `title` when it is given (even blank), else the front matter's, else `""`.
    pub title: String,
    /// Accessible title from `accTitle`, empty if absent.
    pub acc_title: String,
    /// Accessible description from `accDescr`, empty if absent.
    pub acc_descr: String,
    /// The actors in the order they were first declared.
    pub actors: Vec<SequenceActor>,
    /// The boxes in the order they were opened.
    pub boxes: Vec<SequenceBox>,
    /// The message list, in the order the DB applies the items.
    pub messages: Vec<SequenceMessage>,
    /// Actors created by `create`, mapped to the number of messages before their creation.
    pub created_actors: BTreeMap<String, usize>,
    /// Actors destroyed by `destroy`, mapped to the number of messages before their destruction.
    pub destroyed_actors: BTreeMap<String, usize>,
}

impl SequenceDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            actors: Vec::new(),
            boxes: Vec::new(),
            messages: Vec::new(),
            created_actors: BTreeMap::new(),
            destroyed_actors: BTreeMap::new(),
        }
    }
}

impl Default for SequenceDiagram {
    fn default() -> Self {
        Self::new()
    }
}
