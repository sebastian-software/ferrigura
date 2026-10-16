//! The messages: their rows and bounds (`boundMessage`, `sequenceRenderer.ts:407-451`), the
//! adjustments of the actors they create and destroy (`adjustCreatedDestroyedData`, `:968-1036`),
//! and their drawing (`drawMessage`, `:461-741`) with the central connections and the sequence
//! numbers.
//!
//! Each message gets its bounds in the drawing loop, and is drawn after the actors' header row, so
//! that the arrowheads sit on top of the activations (`:1389-1391`).

use ferrigura_model::sequence::{CentralConnection, LineType};

use super::Renderer;
use super::bounds::get_max;
use super::model::MessageModel;
use super::text::{TextData, draw_text, line_count};
use crate::RenderError;
use crate::common::Element;
use crate::svg::number;

/// `SEQUENCE_NUMBER_RADIUS` (`sequenceRenderer.ts`, in `drawMessage`).
const SEQUENCE_NUMBER_RADIUS: f64 = 6.0;

/// `CENTRAL_CONNECTION_CIRCLE_OFFSET` (`sequenceRenderer.ts:310`).
const CENTRAL_CONNECTION_CIRCLE_OFFSET: f64 = 16.5;

/// The actor types drawn as a glyph: their creation and destruction arrows stop at the glyph's
/// width, not at the box's (`actorArray`, `sequenceRenderer.ts:968-973`).
fn is_glyph_actor(actor_type: &str) -> bool {
    matches!(actor_type, "actor" | "control" | "entity" | "database")
}

/// Whether a message carries a central connection (`hasCentralConnection`, `:1841-1847`).
fn has_central_connection(central: Option<CentralConnection>) -> bool {
    matches!(
        central,
        Some(CentralConnection::Central | CentralConnection::Reverse | CentralConnection::Dual)
    )
}

/// The dotted line types, whose line is dashed (`drawMessage`, `:548-568`).
#[must_use]
pub fn is_dotted(line_type: LineType) -> bool {
    matches!(
        line_type,
        LineType::Dotted
            | LineType::DottedCross
            | LineType::DottedPoint
            | LineType::DottedOpen
            | LineType::BidirectionalDotted
            | LineType::SolidTopDotted
            | LineType::SolidBottomDotted
            | LineType::StickTopDotted
            | LineType::StickBottomDotted
            | LineType::SolidArrowTopReverseDotted
            | LineType::SolidArrowBottomReverseDotted
            | LineType::StickArrowTopReverseDotted
            | LineType::StickArrowBottomReverseDotted
    )
}

/// The reverse arrows: their arrowhead is at the start (`isReverseArrowType`, `:1867-1888`).
#[must_use]
pub fn is_reverse(line_type: LineType) -> bool {
    matches!(
        line_type,
        LineType::SolidArrowTopReverse
            | LineType::SolidArrowTopReverseDotted
            | LineType::SolidArrowBottomReverse
            | LineType::SolidArrowBottomReverseDotted
            | LineType::StickArrowTopReverse
            | LineType::StickArrowTopReverseDotted
            | LineType::StickArrowBottomReverse
            | LineType::StickArrowBottomReverseDotted
    )
}

/// The bidirectional arrows (`isBidirectionalArrowType`, `:1894-1899`).
#[must_use]
pub fn is_bidirectional(line_type: LineType) -> bool {
    matches!(
        line_type,
        LineType::BidirectionalSolid | LineType::BidirectionalDotted
    )
}

/// The marker at the end of a message's line, by its type (`drawMessage`, `:584-644`).
#[must_use]
pub fn marker_end(line_type: LineType) -> Option<&'static str> {
    Some(match line_type {
        LineType::SolidTop | LineType::SolidTopDotted => "solidTopArrowHead",
        LineType::SolidBottom | LineType::SolidBottomDotted => "solidBottomArrowHead",
        LineType::StickTop | LineType::StickTopDotted => "stickTopArrowHead",
        LineType::StickBottom | LineType::StickBottomDotted => "stickBottomArrowHead",
        LineType::Solid | LineType::Dotted => "arrowhead",
        LineType::BidirectionalSolid | LineType::BidirectionalDotted => "arrowhead",
        LineType::SolidPoint | LineType::DottedPoint => "filled-head",
        LineType::SolidCross | LineType::DottedCross => "crosshead",
        _ => return None,
    })
}

/// The marker at the start of a message's line, by its type (`drawMessage`, `:584-644`).
#[must_use]
pub fn marker_start(line_type: LineType) -> Option<&'static str> {
    Some(match line_type {
        LineType::SolidArrowTopReverse | LineType::SolidArrowTopReverseDotted => {
            "solidBottomArrowHead"
        }
        LineType::SolidArrowBottomReverse | LineType::SolidArrowBottomReverseDotted => {
            "solidTopArrowHead"
        }
        LineType::StickArrowTopReverse | LineType::StickArrowTopReverseDotted => {
            "stickBottomArrowHead"
        }
        LineType::StickArrowBottomReverse | LineType::StickArrowBottomReverseDotted => {
            "stickTopArrowHead"
        }
        LineType::BidirectionalSolid | LineType::BidirectionalDotted => "arrowhead",
        _ => return None,
    })
}

impl Renderer<'_> {
    /// `boundMessage` (`:407-451`): reserves the message's row and returns the y of its line.
    pub(super) fn bound_message(&mut self, model: &mut MessageModel) -> Result<f64, RenderError> {
        let box_margin = self.conf.box_margin;
        self.bounds.bump(10.0);
        let lines = line_count(&model.message);
        let font = self.conf.message_font.clone();
        let dims = super::text::dimensions(&mut self.memo, &model.message, &font)?;
        let line_height = dims.height / lines as f64;
        model.height += line_height;
        self.bounds.bump(line_height);

        let mut total_offset = dims.height - 10.0;
        let text_width = dims.width;
        let line_start_y;
        if model.startx == model.stopx {
            // A self message: its loop hangs below the line, unless it draws right angles.
            let mut start = self.bounds.vertical_pos + total_offset;
            if !self.conf.right_angles {
                total_offset += box_margin;
                start = self.bounds.vertical_pos + total_offset;
            }
            line_start_y = start;
            total_offset += 30.0;
            let dx = get_max(&[Some(text_width / 2.0), Some(self.conf.width / 2.0)]);
            let top = self.bounds.vertical_pos - 10.0 + total_offset;
            let bottom = self.bounds.vertical_pos + 30.0 + total_offset;
            self.bounds
                .insert(box_margin, model.startx - dx, top, model.stopx + dx, bottom);
        } else {
            total_offset += box_margin;
            line_start_y = self.bounds.vertical_pos + total_offset;
            self.bounds.insert(
                box_margin,
                model.startx,
                line_start_y - 10.0,
                model.stopx,
                line_start_y,
            );
        }
        self.bounds.bump(total_offset);
        model.height += total_offset;
        model.stopy = model.starty + model.height;
        self.bounds.insert(
            box_margin,
            model.from_bounds,
            model.starty,
            model.to_bounds,
            model.stopy,
        );
        Ok(line_start_y)
    }

    /// `adjustCreatedDestroyedData` (`:968-1036`): an actor that a message creates or destroys is
    /// moved to the message's row, and the message's arrow stops at its edge.
    pub(super) fn adjust_created_destroyed(
        &mut self,
        msg_index: usize,
        model: &mut MessageModel,
        line_start_y: f64,
    ) {
        let msg_from = self.layout.messages[msg_index]
            .from
            .clone()
            .unwrap_or_default();
        let msg_to = self.layout.messages[msg_index]
            .to
            .clone()
            .unwrap_or_default();
        let created_to = self.layout.created.get(&msg_to).copied();
        let destroyed_from = self.layout.destroyed.get(&msg_from).copied();
        let destroyed_to = self.layout.destroyed.get(&msg_to).copied();
        if created_to == Some(msg_index) {
            // A created actor: its receiver end moves in, and its top is on the message's row.
            let Some(actor_index) = self.layout.actor_index(&msg_to) else {
                return;
            };
            let actor = self.layout.actors[actor_index].clone();
            let adjustment = if is_glyph_actor(&actor.actor_type) {
                super::actors::ACTOR_TYPE_WIDTH / 2.0 + 3.0
            } else {
                actor.width / 2.0 + 3.0
            };
            self.receiver_adjustment(&msg_from, &actor, model, adjustment);
            self.layout.actors[actor_index].starty = line_start_y - actor.height / 2.0;
            self.bounds.bump(actor.height / 2.0);
        } else if destroyed_from == Some(msg_index) {
            // A destroyed sender: its end is drawn at the message's row.
            let Some(actor_index) = self.layout.actor_index(&msg_from) else {
                return;
            };
            let actor = self.layout.actors[actor_index].clone();
            if self.conf.mirror_actors {
                let adjustment = if is_glyph_actor(&actor.actor_type) {
                    super::actors::ACTOR_TYPE_WIDTH / 2.0
                } else {
                    actor.width / 2.0
                };
                self.sender_adjustment(&msg_to, &actor, model, adjustment);
            }
            self.layout.actors[actor_index].stopy = Some(line_start_y - actor.height / 2.0);
            self.bounds.bump(actor.height / 2.0);
        } else if destroyed_to == Some(msg_index) {
            // A destroyed receiver.
            let Some(actor_index) = self.layout.actor_index(&msg_to) else {
                return;
            };
            let actor = self.layout.actors[actor_index].clone();
            if self.conf.mirror_actors {
                let adjustment = if is_glyph_actor(&actor.actor_type) {
                    super::actors::ACTOR_TYPE_WIDTH / 2.0 + 3.0
                } else {
                    actor.width / 2.0 + 3.0
                };
                self.receiver_adjustment(&msg_from, &actor, model, adjustment);
            }
            self.layout.actors[actor_index].stopy = Some(line_start_y - actor.height / 2.0);
            self.bounds.bump(actor.height / 2.0);
        }
    }

    /// `receiverAdjustment` (`:983-1000`): the arrow's end moves in by the adjustment, and the
    /// bounds cover the arrow down to the actor's lower edge.
    fn receiver_adjustment(
        &mut self,
        msg_from: &str,
        actor: &super::model::Actor,
        model: &mut MessageModel,
        adjustment: f64,
    ) {
        let box_margin = self.conf.box_margin;
        let note_margin = self.conf.note_margin;
        let from_x = self
            .layout
            .actor_index(msg_from)
            .map_or(0.0, |index| self.layout.actors[index].x);
        let below = model.stopy + actor.height / 2.0 + note_margin;
        if actor.x < from_x {
            self.bounds.insert(
                box_margin,
                model.stopx - adjustment,
                model.starty,
                model.startx,
                below,
            );
            model.stopx += adjustment;
        } else {
            self.bounds.insert(
                box_margin,
                model.startx,
                model.starty,
                model.stopx + adjustment,
                below,
            );
            model.stopx -= adjustment;
        }
    }

    /// `senderAdjustment` (`:1002-1019`): the arrow's start moves in by the adjustment.
    fn sender_adjustment(
        &mut self,
        msg_to: &str,
        actor: &super::model::Actor,
        model: &mut MessageModel,
        adjustment: f64,
    ) {
        let box_margin = self.conf.box_margin;
        let note_margin = self.conf.note_margin;
        let to_x = self
            .layout
            .actor_index(msg_to)
            .map_or(0.0, |index| self.layout.actors[index].x);
        let below = model.stopy + actor.height / 2.0 + note_margin;
        if actor.x < to_x {
            self.bounds.insert(
                box_margin,
                model.startx - adjustment,
                model.starty,
                model.stopx,
                below,
            );
            model.startx += adjustment;
        } else {
            self.bounds.insert(
                box_margin,
                model.stopx,
                model.starty,
                model.startx + adjustment,
                below,
            );
            model.startx -= adjustment;
        }
    }

    /// `drawMessage` (`:461-741`): the label, the line or the self loop with its markers, the
    /// central connection and the sequence number.
    pub(super) fn draw_message(
        &mut self,
        msg_index: usize,
        model: &MessageModel,
        line_start_y: f64,
    ) -> Result<(), RenderError> {
        let msg = self.layout.messages[msg_index].clone();
        let font = self.conf.message_font.clone();
        let text_dims = super::text::dimensions(&mut self.memo, &model.message, &font)?;
        let (startx, stopx, starty) = (model.startx, model.stopx, model.starty);
        let anchor = self.conf.message_align.clone();

        // The label (`drawText` with a margin of the wrap padding).
        let label = TextData {
            text: &model.message,
            x: startx.min(stopx),
            y: starty + 10.0,
            width: Some((stopx - startx).abs()),
            anchor: Some(&anchor),
            valign: Some("center"),
            text_margin: Some(self.conf.wrap_padding),
            dy: Some("1em"),
            class: "messageText",
            tspan: false,
            font: &font,
        };
        for element in draw_text(&label)? {
            self.append(element);
        }

        let line_type = msg.line_type;
        let central_on = has_central_connection(msg.central);
        let autonumber_on = model.sequence_visible || self.conf.show_sequence_numbers;
        let self_message = startx == stopx;
        let is_reverse_type = is_reverse(line_type);
        let bidirectional = is_bidirectional(line_type);

        // The line: a path that loops back for a message to the same actor, else a straight line.
        let mut line = if self_message {
            let line_start_x = startx
                + if autonumber_on && (is_reverse_type || bidirectional) {
                    10.0
                } else {
                    0.0
                };
            let d = if self.conf.right_angles {
                format!(
                    "M  {},{} H {} V {} H {}",
                    number(line_start_x),
                    number(line_start_y),
                    number(
                        startx
                            + get_max(&[Some(self.conf.width / 2.0), Some(text_dims.width / 2.0)])
                    ),
                    number(line_start_y + 25.0),
                    number(startx),
                )
            } else {
                format!(
                    "M {},{} C {},{} {},{} {},{}",
                    number(line_start_x),
                    number(line_start_y),
                    number(line_start_x + 60.0),
                    number(line_start_y - 10.0),
                    number(startx + 60.0),
                    number(line_start_y + 30.0),
                    number(startx),
                    number(line_start_y + 20.0),
                )
            };
            Element::new("path").with("d", d)
        } else {
            Element::new("line")
                .with("x1", number(startx))
                .with("y1", number(line_start_y))
                .with("x2", number(stopx))
                .with("y2", number(line_start_y))
        };

        // The central connection is drawn after the line, for either shape (`:513`, `:537`).
        let central_group = if central_on {
            Some(self.central_connection(&msg, model, line_start_y))
        } else {
            None
        };

        // The attributes of the line, in the reference's order.
        if is_dotted(line_type) {
            line.set("style", "stroke-dasharray: 3, 3;");
            line.set("class", "messageLine1");
        } else {
            line.set("class", "messageLine0");
        }
        line.set("data-et", "message");
        line.set("data-id", format!("i{}", msg.id));
        line.set("data-from", msg.from.clone().unwrap_or_default());
        line.set("data-to", msg.to.clone().unwrap_or_default());
        line.set("stroke-width", "2");
        line.set("stroke", "none");
        line.set(
            "style",
            match line.get("style") {
                Some(dash) => format!("{dash} fill: none;"),
                None => "fill: none;".to_owned(),
            },
        );
        if let Some(name) = marker_end(line_type) {
            line.set("marker-end", format!("url(#{}-{name})", self.id));
        }
        if let Some(name) = marker_start(line_type) {
            line.set("marker-start", format!("url(#{id}-{name})", id = self.id));
        }

        // The sequence number: the line is shortened to make room for it.
        let number_marker = if autonumber_on {
            Some(self.sequence_number_adjust(&msg, model, &mut line, line_start_y))
        } else {
            None
        };

        self.append(line);
        if let Some(group) = central_group {
            self.append(group);
        }
        if let Some((marker_line, number_text)) = number_marker {
            self.append(marker_line);
            self.append(number_text);
        }
        Ok(())
    }

    /// The sequence number of a message and the attributes it changes on the line (`:700-739`).
    /// Returns the marker line and the number's text, which the caller appends after the line.
    fn sequence_number_adjust(
        &self,
        msg: &super::model::Message,
        model: &MessageModel,
        line: &mut Element,
        line_start_y: f64,
    ) -> (Element, Element) {
        let (startx, stopx) = (model.startx, model.stopx);
        let reverse = is_reverse(msg.line_type);
        let bidirectional = is_bidirectional(msg.line_type);
        let central_shift = matches!(
            msg.central,
            Some(CentralConnection::Dual | CentralConnection::Reverse)
        );
        let central_any = has_central_connection(msg.central);
        let mut line_start_x = startx;
        let mut line_stop_x;
        if bidirectional {
            if startx < stopx {
                line_start_x = startx + SEQUENCE_NUMBER_RADIUS * 2.0;
            } else {
                line_start_x =
                    startx - SEQUENCE_NUMBER_RADIUS + if central_any { -5.0 } else { 0.0 };
                line_start_x += if central_shift { -7.5 } else { 0.0 };
            }
            line.set("x1", number(line_start_x));
        } else if reverse {
            if stopx > startx {
                line_stop_x = stopx - 2.0 * SEQUENCE_NUMBER_RADIUS;
            } else {
                line_stop_x = stopx - SEQUENCE_NUMBER_RADIUS;
                line_start_x += if central_shift { -7.5 } else { 0.0 };
            }
            line_stop_x += if central_any { 15.0 } else { 0.0 };
            line.set("x2", number(line_stop_x));
            line.set("x1", number(line_start_x));
        } else {
            line.set("x1", number(startx + SEQUENCE_NUMBER_RADIUS));
        }

        // The number's place: beside the actor the arrow starts from, or the one it points to.
        let is_self = startx == stopx;
        let is_left_to_right = startx <= stopx;
        let autonumber_x = if is_self {
            model.from_bounds + 1.0
        } else if reverse {
            if is_left_to_right {
                model.to_bounds - 1.0
            } else {
                model.from_bounds + 1.0
            }
        } else if is_left_to_right {
            model.from_bounds + 1.0
        } else {
            model.to_bounds - 1.0
        };
        let digits = number(model.sequence_index).len();
        let font_size = if digits > 5 {
            "7px"
        } else if digits > 3 {
            "9px"
        } else {
            "12px"
        };
        let marker_line = Element::new("line")
            .with("x1", number(autonumber_x))
            .with("y1", number(line_start_y))
            .with("x2", number(autonumber_x))
            .with("y2", number(line_start_y))
            .with("stroke-width", "0")
            .with("marker-start", format!("url(#{}-sequencenumber)", self.id));
        let text = Element::new("text")
            .with("x", number(autonumber_x))
            .with("y", number(line_start_y + 4.0))
            .with("font-family", "sans-serif")
            .with("font-size", font_size)
            .with("text-anchor", "middle")
            .with("class", "sequenceNumber")
            .text(number(model.sequence_index));
        (marker_line, text)
    }

    /// `drawCentralConnection` (`:288-374`): the circles at the ends of a message with a central
    /// connection, in a group of their own.
    fn central_connection(
        &self,
        msg: &super::model::Message,
        model: &MessageModel,
        line_start_y: f64,
    ) -> Element {
        let from = msg
            .from
            .as_deref()
            .and_then(|key| self.layout.actor_index(key))
            .map(|index| self.layout.actors[index].clone());
        let to = msg
            .to
            .as_deref()
            .and_then(|key| self.layout.actor_index(key))
            .map(|index| self.layout.actors[index].clone());
        let (Some(from), Some(to)) = (from, to) else {
            return Element::new("g");
        };
        let mut from_centre = from.x + from.width / 2.0;
        let mut to_centre = to.x + to.width / 2.0;
        let left_to_right = from_centre <= to_centre;
        let reverse = is_reverse(msg.line_type);
        let offset_for = |is_reverse_side: bool| {
            let base = if left_to_right {
                CENTRAL_CONNECTION_CIRCLE_OFFSET
            } else {
                -CENTRAL_CONNECTION_CIRCLE_OFFSET
            };
            if is_reverse_side { -base } else { base }
        };
        if model.sequence_visible {
            match msg.central {
                Some(CentralConnection::Central) => {
                    if reverse {
                        to_centre += offset_for(true);
                    }
                }
                Some(CentralConnection::Reverse) => {
                    if !reverse {
                        from_centre += offset_for(false);
                    }
                }
                Some(CentralConnection::Dual) => {
                    if reverse {
                        to_centre += offset_for(true);
                    } else {
                        from_centre += offset_for(false);
                    }
                }
                _ => {}
            }
        }
        let circle = |cx: f64| {
            Element::new("circle")
                .with("cx", number(cx))
                .with("cy", number(line_start_y))
                .with("r", "5")
                .with("width", "10")
                .with("height", "10")
        };
        let mut group = Element::new("g");
        match msg.central {
            Some(CentralConnection::Central) => group.push_child(circle(to_centre)),
            Some(CentralConnection::Reverse) => group.push_child(circle(from_centre)),
            Some(CentralConnection::Dual) => {
                group.push_child(circle(from_centre));
                group.push_child(circle(to_centre));
            }
            _ => {}
        }
        group
    }
}
