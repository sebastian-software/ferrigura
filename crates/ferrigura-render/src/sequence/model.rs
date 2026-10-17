//! The state one render works on: the actors, boxes, messages and blocks of the diagram, with the
//! fields the layout gives them (`sequenceRenderer.ts` writes them onto the DB's objects).
//!
//! The reference mutates the DB's records in place. The port copies what the layout needs into
//! these structs, in the DB's order, and keeps the parse data it reads unchanged in the model.

use std::collections::BTreeMap;

use ferrigura_model::sequence::{
    Autonumber, CentralConnection, LineType, Placement, SequenceDiagram,
};

use crate::common::draw::RectData;
use ferrigura_model::js_object::JsObject;
use ferrigura_model::value::Value;

/// A participant, with the layout the renderer gives it: `width`, `height`, `margin`, `x`,
/// `starty` and `stopy` (`addActorRenderingData`, `calculateActorMargins`).
#[derive(Debug, Clone)]
pub struct Actor {
    /// The key of the actor in the DB's map.
    pub id: String,
    /// The name, as written. `createdActors` and the rendering attributes use it.
    pub name: String,
    /// The text of the actor's label; wrapped when `wrap` is set (`calculateActorMargins`).
    pub description: String,
    /// The `type` of the actor (`participant`, `actor`, …). Empty when the type is not a string,
    /// which matches none of the shape cases, as the reference's switch matches none.
    pub actor_type: String,
    /// Whether the label wraps.
    pub wrap: bool,
    /// Index into [`Layout::boxes`] of the box the actor is in.
    pub box_index: Option<usize>,
    /// The actor declared before this one.
    pub prev_actor: Option<String>,
    /// The actor declared after this one.
    pub next_actor: Option<String>,
    /// The links (the popup menu's entries), in the key order the reference lists them in.
    pub links: JsObject,
    /// The properties (`class`, `icon`).
    pub properties: BTreeMap<String, Value>,
    /// `actor.width`.
    pub width: f64,
    /// `actor.height`: the row's height, raised to `conf.height`.
    pub height: f64,
    /// `actor.margin`, the space to the next actor. Absent until calculated.
    pub margin: Option<f64>,
    /// `actor.x`, the left edge.
    pub x: f64,
    /// `actor.starty`, the top of the header row.
    pub starty: f64,
    /// `actor.stopy`, the top of the footer row, once the destruction or the footer sets it.
    pub stopy: Option<f64>,
    /// `actor.actorCnt`, the number of the lifeline (`actorN`), set when the header is drawn.
    pub actor_cnt: Option<i64>,
    /// The height of the label block of the neo bands, `actorTextHeight`.
    pub actor_text_height: Option<f64>,
    /// The rectangle of a box-shaped participant (`actor.rectData`), which the menu of the actor is
    /// placed from. Absent for the shapes that set none.
    pub rect_data: Option<RectData>,
}

/// A box around actors (`box … end`): its title, its colour and its place.
#[derive(Debug, Clone)]
pub struct BoxState {
    /// The title, `None` when the box has none.
    pub name: Option<String>,
    /// Whether the title wraps.
    pub wrap: bool,
    /// The colour.
    pub fill: String,
    /// The actors in the box, in order.
    pub actor_keys: Vec<String>,
    /// The left edge of the box, before the padding.
    pub x: f64,
    /// The top edge of the box.
    pub y: f64,
    /// The width of the box (`box.width`).
    pub width: f64,
    /// The margin the box adds beside its actors (`box.margin`).
    pub margin: f64,
    /// The height of the tallest title (`box.textMaxHeight`).
    pub text_max_height: f64,
    /// The edges of the box's background, set by the diagram's drawing (`box.startx` …).
    pub startx: f64,
    /// See [`BoxState::startx`].
    pub starty: f64,
    /// See [`BoxState::startx`].
    pub stopx: f64,
    /// See [`BoxState::startx`].
    pub stopy: f64,
}

/// A note's geometry: `noteModel` in `buildNoteModel`, completed by `drawNote`.
#[derive(Debug, Clone, Default)]
pub struct NoteModel {
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
    /// The left edge.
    pub startx: f64,
    /// The right edge.
    pub stopx: f64,
    /// The top edge.
    pub starty: f64,
    /// The bottom edge.
    pub stopy: f64,
    /// The text, wrapped when the note wraps.
    pub message: String,
}

/// A message's geometry: `msgModel` in `buildMessageModel`, completed by `boundMessage`.
#[derive(Debug, Clone, Default)]
pub struct MessageModel {
    /// The width, for the bounds of the loops.
    pub width: f64,
    /// The height.
    pub height: f64,
    /// The start of the arrow.
    pub startx: f64,
    /// The end of the arrow.
    pub stopx: f64,
    /// The top of the message's row.
    pub starty: f64,
    /// The bottom of the message's row.
    pub stopy: f64,
    /// The text, wrapped when the message wraps.
    pub message: String,
    /// The left edge of the actors involved.
    pub from_bounds: f64,
    /// The right edge of the actors involved.
    pub to_bounds: f64,
    /// The sequence number shown with the message.
    pub sequence_index: f64,
    /// Whether the sequence number is shown for this message.
    pub sequence_visible: bool,
}

/// One item of the message list, with the layout the renderer gives it.
#[derive(Debug, Clone)]
pub struct Message {
    /// The index of the item, as a string (`msg.id`).
    pub id: String,
    /// The sending actor.
    pub from: Option<String>,
    /// The receiving actor.
    pub to: Option<String>,
    /// The text (`msg.message`). A block's title, or a message's text. Wrapped in place as the
    /// reference wraps it.
    pub text: String,
    /// The numbering of an `autonumber` item.
    pub autonumber: Option<Autonumber>,
    /// Whether the text wraps. Set to true for a block title that is wrapped.
    pub wrap: bool,
    /// The kind of the item.
    pub line_type: LineType,
    /// Whether the message activates the receiver.
    pub activate: Option<bool>,
    /// The central connection of the message.
    pub central: Option<CentralConnection>,
    /// Where a note is placed. `Some` marks a note.
    pub placement: Option<Placement>,
    /// The width of a block, set when its title wraps (`msg.width`).
    pub width: Option<f64>,
    /// The note's geometry, for a note.
    pub note: Option<NoteModel>,
    /// The message's geometry, for a message.
    pub message_model: Option<MessageModel>,
}

/// The four edges of a box in the bounds. A missing edge is `undefined` in the reference.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Extent {
    /// The left edge.
    pub startx: Option<f64>,
    /// The right edge.
    pub stopx: Option<f64>,
    /// The top edge.
    pub starty: Option<f64>,
    /// The bottom edge.
    pub stopy: Option<f64>,
}

/// A section of a block: the y of its divider line.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// The y of the divider.
    pub y: f64,
}

/// A block (`loop`, `alt`, `opt`, `par`, `critical`, `break`) or a `rect`, as `createLoop` makes it.
#[derive(Debug, Clone, Default)]
pub struct Block {
    /// The edges, updated as the content grows (`bounds.sequenceItems`).
    pub extent: Extent,
    /// The title (`title.message`); absent for a `rect`.
    pub title: Option<String>,
    /// The height, rounded at the end.
    pub height: f64,
    /// The fill of a `rect`.
    pub fill: Option<String>,
    /// The dividers of the sections, in order.
    pub sections: Vec<Section>,
    /// The title of each section (the `else`, `and`, `option` text), in order.
    pub section_titles: Vec<String>,
}

/// An open activation (`bounds.activations`), with the group it is drawn into.
#[derive(Debug, Clone)]
pub struct Activation {
    /// The edges, updated as the content grows.
    pub extent: Extent,
    /// The actor it is on.
    pub actor: String,
    /// The index of the `g` that holds the activation, in the diagram's back list.
    pub anchored: usize,
}

/// The state of a diagram: the DB's header texts and lists, with the layout's fields.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The actors, in the DB's order.
    pub actors: Vec<Actor>,
    /// The actors' keys in order, as `getActorKeys` gives them.
    pub actor_keys: Vec<String>,
    /// The boxes, in the order they were opened.
    pub boxes: Vec<BoxState>,
    /// The messages, in the order they were applied.
    pub messages: Vec<Message>,
    /// The actors created by a message, with the message count at the time.
    pub created: BTreeMap<String, usize>,
    /// The actors destroyed by a message, with the message count at the time.
    pub destroyed: BTreeMap<String, usize>,
}

impl Layout {
    /// The layout state of a parsed diagram, before any layout is done.
    #[must_use]
    pub fn new(diagram: &SequenceDiagram) -> Self {
        let actors = diagram
            .actors
            .iter()
            .map(|actor| Actor {
                id: actor.id.clone(),
                name: actor.name.clone(),
                description: value_text(&actor.description),
                actor_type: match &actor.actor_type {
                    Value::String(text) => text.clone(),
                    _ => String::new(),
                },
                wrap: actor.wrap,
                box_index: actor.box_index,
                prev_actor: actor.prev_actor.clone(),
                next_actor: actor.next_actor.clone(),
                links: actor.links.clone(),
                properties: actor.properties.clone(),
                width: 0.0,
                height: 0.0,
                margin: None,
                x: 0.0,
                starty: 0.0,
                stopy: None,
                actor_cnt: None,
                actor_text_height: None,
                rect_data: None,
            })
            .collect::<Vec<_>>();
        let boxes = diagram
            .boxes
            .iter()
            .map(|sequence_box| BoxState {
                name: sequence_box.name.clone(),
                wrap: sequence_box.wrap,
                fill: sequence_box.fill.clone(),
                actor_keys: sequence_box.actor_keys.clone(),
                x: 0.0,
                y: 0.0,
                width: 0.0,
                margin: 0.0,
                text_max_height: 0.0,
                startx: 0.0,
                starty: 0.0,
                stopx: 0.0,
                stopy: 0.0,
            })
            .collect();
        let messages = diagram
            .messages
            .iter()
            .map(|message| Message {
                id: message.id.clone(),
                from: message.from.clone(),
                to: message.to.clone(),
                text: match &message.message {
                    ferrigura_model::sequence::MessageText::Text(text) => text.clone(),
                    ferrigura_model::sequence::MessageText::Autonumber(_) => String::new(),
                },
                autonumber: match &message.message {
                    ferrigura_model::sequence::MessageText::Autonumber(number) => {
                        Some(number.clone())
                    }
                    ferrigura_model::sequence::MessageText::Text(_) => None,
                },
                wrap: message.wrap,
                line_type: message.line_type,
                activate: message.activate,
                central: message.central_connection,
                placement: message.placement,
                width: None,
                note: None,
                message_model: None,
            })
            .collect();
        Self {
            actor_keys: actors
                .iter()
                .map(|actor: &Actor| actor.id.clone())
                .collect(),
            actors,
            boxes,
            messages,
            created: diagram
                .created_actors
                .iter()
                .map(|(id, count)| (id.clone(), *count))
                .collect(),
            destroyed: diagram
                .destroyed_actors
                .iter()
                .map(|(id, count)| (id.clone(), *count))
                .collect(),
        }
    }

    /// The index of the actor with the key `id`.
    #[must_use]
    pub fn actor_index(&self, id: &str) -> Option<usize> {
        self.actors.iter().position(|actor| actor.id == id)
    }
}

/// The text of a configuration value, as a string: a string as it is, a number as JavaScript
/// writes it, anything else as nothing (the reference's text functions get such a value only in
/// unusual metadata).
#[must_use]
pub fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => crate::svg::number(*number),
        Value::Bool(flag) => flag.to_string(),
        _ => String::new(),
    }
}
