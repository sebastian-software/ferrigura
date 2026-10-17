//! Canonical model of an event modeling diagram: the DB state after the parse, as
//! `docs-port/eventmodeling/NOTES.md` section 4 proposes, without the geometry and the box texts
//! (see `docs-port/canonical-model.md`, section "Event modeling").
//!
//! The lanes and the relations are the DB's, with frames as the boxes: a relation joins two frame
//! indices. The box sizes and positions and the measured box texts are the renderer's.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed event modeling diagram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventModelingDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The diagram title: the grammar's last `title` line, sanitised, else the front matter's.
    pub title: String,
    /// The accessibility title, sanitised, without its leading white space.
    pub acc_title: String,
    /// The accessibility description, sanitised, with the white space after each line break
    /// removed.
    pub acc_descr: String,
    /// The frames, in the order of the text. A frame is a box of the diagram.
    pub frames: Vec<Frame>,
    /// The data blocks, in the order of the text, with the value the grammar gives.
    pub data_entities: Vec<DataEntity>,
    /// The swimlanes that the frames use, in index order.
    pub lanes: Vec<Lane>,
    /// The relations between boxes, in the order the DB creates them.
    pub relations: Vec<Relation>,
}

/// A frame, with its entity, the lane it is drawn in, and the data it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    /// The frame number as written, so `01` and `1` are different frames.
    pub name: String,
    /// `time` for `tf`/`timeframe`, `reset` for `rf`/`resetframe`.
    pub kind: FrameKind,
    /// The entity type as written: `ui`, `cmd`, `evt` or one of the other keywords.
    pub model_entity_type: String,
    /// The entity identifier as the grammar gives it: the qualified name, dots included.
    pub entity_identifier: String,
    /// The part after the dot, when the identifier has exactly two parts; else the identifier.
    pub entity_name: String,
    /// The part before the dot, when the identifier has exactly two parts.
    pub namespace: Option<String>,
    /// The index of the lane the frame is drawn in.
    pub lane: usize,
    /// The source frame numbers as written, in the order of the text.
    pub source_frames: Vec<String>,
    /// The inline data as written, quotes and braces included, or `null`.
    pub data_inline: Option<String>,
    /// The name of the data block the frame refers to, as written, or `null`.
    pub data_reference: Option<String>,
    /// The data text the DB shows for the frame: the referenced block's text when the block exists,
    /// else the inline data's, cut and sanitised as `calculateTextProps` cuts and sanitises it,
    /// before the line breaks and the `&nbsp;` joins. `""` for a quoted inline value, which the
    /// reference shows as an empty code block; `null` when the frame shows no data.
    pub data: Option<String>,
}

/// The two kinds of frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameKind {
    /// A time frame (`tf`, `timeframe`).
    Time,
    /// A reset frame (`rf`, `resetframe`).
    Reset,
}

/// A data block: its name and the value the grammar gives, braces and line breaks included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEntity {
    /// The name of the block.
    pub name: String,
    /// The block's text, from its opening brace to its closing brace.
    pub data_block_value: String,
}

/// A swimlane: its index and label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lane {
    /// The index, which orders the lanes and is the lane's identity.
    pub index: usize,
    /// The label drawn for the lane.
    pub label: String,
}

/// A relation between two frames. The indices are positions in [`EventModelingDiagram::frames`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    /// The frame the relation starts from.
    pub source_frame: usize,
    /// The frame the relation points to.
    pub target_frame: usize,
}

impl EventModelingDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            frames: Vec::new(),
            data_entities: Vec::new(),
            lanes: Vec::new(),
            relations: Vec::new(),
        }
    }
}

impl Default for EventModelingDiagram {
    fn default() -> Self {
        Self::new()
    }
}
