//! The event modeling DB, `db.ts`: the lanes of the frames and the relations between them, as
//! `getState` computes them, without the geometry (`calculateX`, the sizes and the box texts).
//!
//! The lane and the relation rules depend only on the frames' entity types, namespaces and order,
//! so the port keeps those. `docs-port/eventmodeling/NOTES.md`, sections 3.4 to 3.7, has the
//! reference's lines, and the quirks: a namespaced frame always gets a new lane, and a reset frame
//! with no sources gets no inferred relation.

use std::collections::BTreeMap;

use ferrigura_model::event_modeling::{Lane, Relation};

use super::ast::{DataEntity, EventModel, Frame, FrameKind};
use crate::sanitize::sanitize_text;

/// The label of the lane of UI, processor entities without a namespace (`db.ts`, `labelUiAutomation`).
const LABEL_UI: &str = "UI/Automation";
/// The label prefix of a namespaced UI, processor lane (`labelUiAutomationPrefix`).
const PREFIX_UI: &str = "UI/A: ";
/// The label of the lane of command and read model entities (`labelCommandReadModel`).
const LABEL_COMMAND_READ_MODEL: &str = "Command/Read Model";
/// The label prefix of a namespaced command, read model lane (`labelCommandReadModelPrefix`).
const PREFIX_COMMAND_READ_MODEL: &str = "C/RM: ";
/// The label of the lane of events without a namespace (`labelEvents`).
const LABEL_EVENTS: &str = "Events";
/// The label prefix of a namespaced event lane (`labelEventsPrefix`).
const PREFIX_EVENTS: &str = "Stream: ";

/// The DB state after the frames are placed.
#[derive(Debug, Default)]
pub(crate) struct State {
    /// The lanes, in index order.
    pub(crate) lanes: Vec<Lane>,
    /// The lane index of each frame, in the order of the text (each frame is one box).
    pub(crate) lane_of_frame: Vec<usize>,
    /// The relations, in the order the DB creates them.
    pub(crate) relations: Vec<Relation>,
}

/// Places every frame and creates its relations, in the order of `getState`: each frame is placed,
/// then its relations are created from its explicit sources or, when it has none, from the nearest
/// earlier box in another lane.
pub(crate) fn state(ast: &EventModel) -> State {
    // The lanes by index. Their labels are set when a lane is created and never change.
    let mut lanes: BTreeMap<usize, String> = BTreeMap::new();
    let mut lane_of_frame: Vec<usize> = Vec::with_capacity(ast.frames.len());
    let mut relations = Vec::new();

    for (index, frame) in ast.frames.iter().enumerate() {
        let (lane, label) = swimlane_of(frame, &lanes);
        lanes.entry(lane).or_insert(label);
        lane_of_frame.push(lane);

        if frame.source_frames.is_empty() {
            // `isFirstFrame` has no relation, and a reset frame has no inferred one.
            if index == 0 || frame.kind == FrameKind::Reset {
                continue;
            }
            let source = (0..index)
                .rev()
                .find(|&earlier| lane_of_frame[earlier] != lane);
            if let Some(source_frame) = source {
                relations.push(Relation {
                    source_frame,
                    target_frame: index,
                });
            }
        } else {
            // The sources are the frames whose names are listed, in the order of the text. A
            // frame that comes later has no box yet, so it gives no relation.
            for (source_frame, source) in ast.frames.iter().enumerate() {
                let listed = frame.source_frames.contains(&source.name);
                if listed && source_frame <= index {
                    relations.push(Relation {
                        source_frame,
                        target_frame: index,
                    });
                }
            }
        }
    }

    let lanes = lanes
        .into_iter()
        .map(|(index, label)| Lane { index, label })
        .collect();
    State {
        lanes,
        lane_of_frame,
        relations,
    }
}

/// `calculateSwimlaneProps`: the lane of a frame and the label of that lane if it is created now.
/// A namespaced frame takes the next free index of its range; the reference also looks for an
/// existing lane with the same namespace, but its lanes never carry a namespace (`db.ts:457-464`),
/// so that lookup never finds one and is not ported.
fn swimlane_of(frame: &Frame, lanes: &BTreeMap<usize, String>) -> (usize, String) {
    let namespace = namespace(&frame.entity_identifier);
    match frame.model_entity_type.as_str() {
        "ui" | "pcr" | "processor" => match namespace {
            Some(namespace) => (
                next_available_index(lanes, 0, 100),
                format!("{PREFIX_UI}{namespace}"),
            ),
            None => (0, LABEL_UI.to_owned()),
        },
        "rmo" | "readmodel" | "cmd" | "command" => match namespace {
            Some(namespace) => (
                next_available_index(lanes, 100, 200),
                format!("{PREFIX_COMMAND_READ_MODEL}{namespace}"),
            ),
            None => (100, LABEL_COMMAND_READ_MODEL.to_owned()),
        },
        _ => match namespace {
            Some(namespace) => (
                next_available_index(lanes, 200, 300),
                format!("{PREFIX_EVENTS}{namespace}"),
            ),
            None => (200, LABEL_EVENTS.to_owned()),
        },
    }
}

/// `findNextAvailableIndex`: one more than the largest lane index strictly between the bounds, or
/// than the lower bound when there is none.
fn next_available_index(lanes: &BTreeMap<usize, String>, min: usize, max: usize) -> usize {
    lanes
        .keys()
        .copied()
        .filter(|&index| index > min && index < max)
        .max()
        .unwrap_or(min)
        + 1
}

/// `extractNamespace`: the part before the dot, when the identifier has exactly two parts.
#[must_use]
pub(crate) fn namespace(entity_identifier: &str) -> Option<String> {
    let parts: Vec<&str> = entity_identifier.split('.').collect();
    (parts.len() == 2).then(|| parts[0].to_owned())
}

/// `extractName`: the part after the dot, when the identifier has exactly two parts; else the
/// whole identifier, dots and all.
#[must_use]
pub(crate) fn entity_name(entity_identifier: &str) -> String {
    let parts: Vec<&str> = entity_identifier.split('.').collect();
    if parts.len() == 2 {
        parts[1].to_owned()
    } else {
        entity_identifier.to_owned()
    }
}

/// The data text the DB shows for a frame, as `calculateTextProps` builds it before the line breaks
/// (`db.ts:349-372`): the referenced block's text when the block exists, otherwise the inline
/// data's. The text is cut as the DB cuts it, then sanitised.
#[must_use]
pub(crate) fn data_text(frame: &Frame, data_entities: &[DataEntity]) -> Option<String> {
    let block = frame
        .data_reference
        .as_ref()
        .and_then(|name| data_entities.iter().find(|entity| entity.name == *name));
    match (block, &frame.data_inline) {
        (Some(entity), _) => Some(sanitize_text(&block_data_cut(&entity.data_block_value))),
        (None, Some(inline)) => Some(sanitize_text(&inline_data_cut(inline))),
        (None, None) => None,
    }
}

/// The inline data without its braces, as the DB cuts it: the text after the first `{`, then the
/// text before the character before the last `}`. Without a `{` the cut starts at the beginning,
/// and without a `}` it ends before the start, so a quoted value is empty.
fn inline_data_cut(text: &str) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let rest = js_substring(&units, index_of(&units, "{") + 1, units.len() as i64);
    let end = last_index_of(rest, "}") - 1;
    String::from_utf16_lossy(js_substring(rest, 0, end))
}

/// The data block without its opening line, as the DB cuts it: the text after the first `{` and a
/// line break (the position is one when there is none, which cuts the brace only), then the text
/// before the character before the last `}`.
fn block_data_cut(text: &str) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let rest = js_substring(&units, index_of(&units, "{\n") + 2, units.len() as i64);
    let end = last_index_of(rest, "}") - 1;
    String::from_utf16_lossy(js_substring(rest, 0, end))
}

/// `indexOf` of a one-or-two character needle in UTF-16 units, or `-1`.
fn index_of(units: &[u16], needle: &str) -> i64 {
    let needle: Vec<u16> = needle.encode_utf16().collect();
    units
        .windows(needle.len())
        .position(|window| window == needle.as_slice())
        .map_or(-1, |index| index as i64)
}

/// `lastIndexOf` of a one-character needle in UTF-16 units, or `-1`.
fn last_index_of(units: &[u16], needle: &str) -> i64 {
    let needle = needle.encode_utf16().next().unwrap_or_default();
    units
        .iter()
        .rposition(|&unit| unit == needle)
        .map_or(-1, |index| index as i64)
}

/// `substring(start, end)` in UTF-16 units: bounds below zero or above the length are clamped, and
/// bounds in the wrong order are swapped, as JavaScript does.
fn js_substring(units: &[u16], start: i64, end: i64) -> &[u16] {
    let length = units.len() as i64;
    let start = start.clamp(0, length);
    let end = end.clamp(0, length);
    let (from, to) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    &units[from as usize..to as usize]
}
