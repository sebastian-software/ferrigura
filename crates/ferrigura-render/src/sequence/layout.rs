//! The layout of a sequence diagram before anything is drawn: the widths and margins of the
//! actors (`getMaxMessageWidthPerActor`, `calculateActorMargins`, `addActorRenderingData`), the
//! geometry of notes and messages (`buildNoteModel`, `buildMessageModel`) and the widths of the
//! blocks (`calculateLoopBounds`). The reference runs them in this order, before the drawing loop
//! (`sequenceRenderer.ts:1057-1101`).
//!
//! Each function is a transcription of its reference function; the comments name the line.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use ferrigura_model::sequence::{CentralConnection, LineType, Placement};

use super::Renderer;
use super::actors::stack_height;
use super::bounds::{get_max, get_min};
use super::model::{Actor, Extent, MessageModel, NoteModel};
use super::text::{dimensions, wrap};
use crate::RenderError;

/// `Number.MAX_SAFE_INTEGER`, the start of a block's `from` (`calculateLoopBounds`).
const MAX_SAFE: f64 = 9_007_199_254_740_991.0;

/// The types whose geometry `buildMessageModel` computes: the arrows (`:2013-2033`).
#[must_use]
pub fn is_arrow(line_type: LineType) -> bool {
    matches!(
        line_type,
        LineType::SolidOpen
            | LineType::DottedOpen
            | LineType::Solid
            | LineType::SolidTop
            | LineType::SolidBottom
            | LineType::StickTop
            | LineType::StickBottom
            | LineType::SolidTopDotted
            | LineType::SolidBottomDotted
            | LineType::StickTopDotted
            | LineType::StickBottomDotted
            | LineType::SolidArrowTopReverse
            | LineType::SolidArrowBottomReverse
            | LineType::StickArrowTopReverse
            | LineType::StickArrowBottomReverse
            | LineType::SolidArrowTopReverseDotted
            | LineType::SolidArrowBottomReverseDotted
            | LineType::StickArrowTopReverseDotted
            | LineType::StickArrowBottomReverseDotted
            | LineType::Dotted
            | LineType::SolidCross
            | LineType::DottedCross
            | LineType::SolidPoint
            | LineType::DottedPoint
            | LineType::BidirectionalSolid
            | LineType::BidirectionalDotted
    )
}

/// `calculateCentralConnectionOffset` (`sequenceRenderer.ts:1820-1840`): the shift of the start of a
/// message that has a central connection.
fn central_offset(
    central: Option<CentralConnection>,
    line_type: LineType,
    is_arrow_to_right: bool,
) -> f64 {
    let mut offset = 0.0;
    let reverse_or_dual = matches!(
        central,
        Some(CentralConnection::Reverse | CentralConnection::Dual)
    );
    if reverse_or_dual {
        offset += 4.0;
    }
    if reverse_or_dual
        && matches!(
            line_type,
            LineType::BidirectionalSolid | LineType::BidirectionalDotted
        )
    {
        offset += if is_arrow_to_right { 0.0 } else { -6.0 };
    }
    offset
}

/// `NaN`-free truthiness of a number, as JavaScript's `if (x)` reads it.
fn truthy(value: f64) -> bool {
    value != 0.0 && !value.is_nan()
}

/// `a || b` for numbers: `a` unless it is zero or NaN.
fn or_default(value: f64, fallback: f64) -> f64 {
    if truthy(value) { value } else { fallback }
}

impl Renderer<'_> {
    /// `getMaxMessageWidthPerActor` (`:1488-1587`): the widest message or note that each actor has
    /// to keep room for, keyed by the actor's key.
    pub(super) fn max_message_width_per_actor(
        &mut self,
    ) -> Result<BTreeMap<String, f64>, RenderError> {
        let mut max_width: BTreeMap<String, f64> = BTreeMap::new();
        let padding = self.conf.wrap_padding;
        let width_conf = self.conf.width;
        let layout = &self.layout;
        let conf = &self.conf;
        let memo = &mut self.memo;
        for msg in &layout.messages {
            let (Some(from), Some(to)) = (msg.from.as_deref(), msg.to.as_deref()) else {
                continue;
            };
            let (Some(from_index), Some(to_index)) =
                (layout.actor_index(from), layout.actor_index(to))
            else {
                continue;
            };
            let actor = &layout.actors[to_index];
            // The first actor with a message to its left needs no margin, nor the last one a note
            // to its right (`:1507-1516`).
            if msg.placement == Some(Placement::LeftOf) && actor.prev_actor.is_none() {
                continue;
            }
            if msg.placement == Some(Placement::RightOf) && actor.next_actor.is_none() {
                continue;
            }
            let is_note = msg.placement.is_some();
            let is_message = !is_note;
            let font = if is_note {
                &conf.note_font
            } else {
                &conf.message_font
            };
            let wrapped = if msg.wrap {
                Cow::Owned(wrap(memo, &msg.text, width_conf - 2.0 * padding, font)?)
            } else {
                Cow::Borrowed(msg.text.as_str())
            };
            let dims = dimensions(memo, &wrapped, font)?;
            let message_width = dims.width + 2.0 * padding;
            let next_matches = actor.next_actor.as_deref() == Some(from);
            let prev_matches = actor.prev_actor.as_deref() == Some(from);
            if is_message && next_matches {
                bump_max(&mut max_width, to, message_width);
            } else if is_message && prev_matches {
                bump_max(&mut max_width, from, message_width);
            } else if is_message && from == to {
                bump_max(&mut max_width, from, message_width / 2.0);
                bump_max(&mut max_width, to, message_width / 2.0);
            } else if msg.placement == Some(Placement::RightOf) {
                bump_max(&mut max_width, from, message_width);
            } else if msg.placement == Some(Placement::LeftOf) {
                if let Some(prev) = &actor.prev_actor {
                    bump_max(&mut max_width, prev, message_width);
                }
            } else if msg.placement == Some(Placement::Over) {
                if let Some(prev) = &actor.prev_actor {
                    bump_max(&mut max_width, prev, message_width / 2.0);
                }
                if actor.next_actor.is_some() {
                    bump_max(&mut max_width, from, message_width / 2.0);
                }
            }
            let _ = from_index;
        }
        Ok(max_width)
    }

    /// `calculateActorMargins` (`:1614-1699`): the size of every actor, the margin to the next one,
    /// the size of the boxes' titles. Returns the height of the header row.
    pub(super) fn calculate_actor_margins(
        &mut self,
        max_width: &BTreeMap<String, f64>,
    ) -> Result<f64, RenderError> {
        let padding = self.conf.wrap_padding;
        let width_conf = self.conf.width;
        let height_conf = self.conf.height;
        let neo = self.conf.neo;
        let mut max_height = 0.0_f64;
        {
            let actor_font = &self.conf.actor_font;
            let memo = &mut self.memo;
            for actor in &mut self.layout.actors {
                if actor.wrap {
                    actor.description = wrap(
                        memo,
                        &actor.description,
                        width_conf - 2.0 * padding,
                        actor_font,
                    )?;
                }
                let dims = dimensions(memo, &actor.description, actor_font)?;
                actor.width = if actor.wrap {
                    width_conf
                } else {
                    get_max(&[Some(width_conf), Some(dims.width + 2.0 * padding)])
                };
                if neo {
                    actor.actor_text_height = Some(dims.height);
                    actor.height = get_max(&[Some(stack_height(dims.height)), Some(height_conf)]);
                } else {
                    actor.height = if actor.wrap {
                        get_max(&[Some(dims.height), Some(height_conf)])
                    } else {
                        height_conf
                    };
                }
                max_height = get_max(&[Some(max_height), Some(actor.height)]);
            }
        }

        // The margin after each actor: wide enough for the widest message from it, and never less
        // than `actorMargin` (`:1648-1668`).
        for (key, message_width) in max_width {
            let Some(index) = self.layout.actor_index(key) else {
                continue;
            };
            let next_width = self.layout.actors[index]
                .next_actor
                .as_deref()
                .and_then(|next| self.layout.actor_index(next))
                .map(|next| self.layout.actors[next].width);
            let actor_width = self.conf.actor_margin;
            let actor = &mut self.layout.actors[index];
            let margin = match next_width {
                None => message_width + actor_width - actor.width / 2.0,
                Some(next) => message_width + actor_width - actor.width / 2.0 - next / 2.0,
            };
            actor.margin = Some(get_max(&[Some(margin), Some(actor_width)]));
        }

        // The boxes: the margin they add beside their actors, and the height of their titles
        // (`:1670-1698`).
        let mut max_box_height = 0.0_f64;
        {
            let message_font = &self.conf.message_font;
            let box_margin = self.conf.box_margin;
            let box_text_margin = self.conf.box_text_margin;
            let memo = &mut self.memo;
            let actors = &self.layout.actors;
            let actor_indices = &self.layout.actor_indices;
            for sequence_box in &mut self.layout.boxes {
                let mut total_width = 0.0;
                for key in &sequence_box.actor_keys {
                    if let Some(&index) = actor_indices.get(key) {
                        let actor = &actors[index];
                        total_width +=
                            actor.width + actor.margin.filter(|m| *m != 0.0).unwrap_or(0.0);
                    }
                }
                total_width += box_margin * 8.0;
                total_width -= 2.0 * box_text_margin;
                if sequence_box.wrap
                    && let Some(name) = sequence_box.name.as_deref()
                {
                    let wrapped = wrap(memo, name, total_width - 2.0 * padding, message_font)?;
                    sequence_box.name = Some(wrapped);
                }
                let name = sequence_box.name.as_deref().unwrap_or("");
                let dims = dimensions(memo, name, message_font)?;
                max_box_height = get_max(&[Some(dims.height), Some(max_box_height)]);
                let min_width = get_max(&[Some(total_width), Some(dims.width + 2.0 * padding)]);
                let mut margin = box_text_margin;
                if total_width < min_width {
                    margin += (min_width - total_width) / 2.0;
                }
                sequence_box.margin = margin;
            }
        }
        for sequence_box in &mut self.layout.boxes {
            sequence_box.text_max_height = max_box_height;
        }
        Ok(get_max(&[Some(max_height), Some(height_conf)]))
    }

    /// `buildNoteModel` (`:1701-1771`): the geometry of a note.
    pub(super) fn build_note_model(&mut self, msg_index: usize) -> Result<NoteModel, RenderError> {
        let layout = &self.layout;
        let msg = &layout.messages[msg_index];
        let from_index = msg
            .from
            .as_deref()
            .and_then(|key| layout.actor_index(key))
            .ok_or_else(missing_actor)?;
        let to_index = msg
            .to
            .as_deref()
            .and_then(|key| layout.actor_index(key))
            .ok_or_else(missing_actor)?;
        let from = &layout.actors[from_index];
        let to = &layout.actors[to_index];
        let conf = &self.conf;
        let note_font = &conf.note_font;
        let memo = &mut self.memo;
        let padding = conf.note_margin;
        let width_conf = conf.width;
        let actor_margin = conf.actor_margin;
        let should_wrap = msg.wrap && !msg.text.is_empty();
        let startx = from.x;
        let stopx = to.x;

        let measured_text = if should_wrap {
            Cow::Owned(wrap(memo, &msg.text, width_conf, note_font)?)
        } else {
            Cow::Borrowed(msg.text.as_str())
        };
        let mut text_dims = dimensions(memo, &measured_text, note_font)?;
        let width;
        let note_startx;
        if msg.placement == Some(Placement::RightOf) {
            width = if should_wrap {
                get_max(&[Some(width_conf), Some(text_dims.width)])
            } else {
                get_max(&[
                    Some(from.width / 2.0 + to.width / 2.0),
                    Some(text_dims.width + 2.0 * padding),
                ])
            };
            note_startx = startx + (from.width + actor_margin) / 2.0;
        } else if msg.placement == Some(Placement::LeftOf) {
            width = if should_wrap {
                get_max(&[Some(width_conf), Some(text_dims.width + 2.0 * padding)])
            } else {
                get_max(&[
                    Some(from.width / 2.0 + to.width / 2.0),
                    Some(text_dims.width + 2.0 * padding),
                ])
            };
            note_startx = startx - width + (from.width - actor_margin) / 2.0;
        } else if msg.to == msg.from {
            let self_text = if should_wrap {
                Cow::Owned(wrap(
                    memo,
                    &msg.text,
                    get_max(&[Some(width_conf), Some(from.width)]),
                    note_font,
                )?)
            } else {
                Cow::Borrowed(msg.text.as_str())
            };
            text_dims = dimensions(memo, &self_text, note_font)?;
            width = if should_wrap {
                get_max(&[Some(width_conf), Some(from.width)])
            } else {
                get_max(&[
                    Some(from.width),
                    Some(width_conf),
                    Some(text_dims.width + 2.0 * padding),
                ])
            };
            note_startx = startx + (from.width - width) / 2.0;
        } else {
            width = (startx + from.width / 2.0 - (stopx + to.width / 2.0)).abs() + actor_margin;
            note_startx = if startx < stopx {
                startx + from.width / 2.0 - actor_margin / 2.0
            } else {
                stopx + to.width / 2.0 - actor_margin / 2.0
            };
        }
        let message = if should_wrap {
            wrap(memo, &msg.text, width - 2.0 * conf.wrap_padding, note_font)?
        } else {
            msg.text.clone()
        };
        Ok(NoteModel {
            width,
            height: 0.0,
            startx: note_startx,
            stopx: 0.0,
            starty: 0.0,
            stopy: 0.0,
            message,
        })
    }

    /// The index of an actor the reference looks up with `actors.get(name)` and then reads; a
    /// missing actor is the TypeError the reference would throw.
    fn actor_of(&self, key: Option<&str>) -> Result<usize, RenderError> {
        key.and_then(|key| self.layout.actor_index(key))
            .ok_or_else(missing_actor)
    }

    /// `buildMessageModel` (`:1953-2032`): the geometry of a message, or `None` for the items that
    /// are not arrows (the reference returns `{}` for them). Wraps the message's text in place.
    pub(super) fn build_message_model(
        &mut self,
        msg_index: usize,
    ) -> Result<Option<MessageModel>, RenderError> {
        let (
            line_type,
            central,
            activate,
            should_wrap,
            same_actor,
            from_left,
            from_right,
            to_left,
            to_right,
        ) = {
            let msg = &self.layout.messages[msg_index];
            if !is_arrow(msg.line_type) {
                return Ok(None);
            }
            let from_key = msg.from.as_deref().unwrap_or("");
            let to_key = msg.to.as_deref().unwrap_or("");
            let (from_left, from_right) = self.activation_bounds(from_key)?;
            let (to_left, to_right) = self.activation_bounds(to_key)?;
            (
                msg.line_type,
                msg.central,
                msg.activate,
                msg.wrap,
                msg.from == msg.to,
                from_left,
                from_right,
                to_left,
                to_right,
            )
        };
        let neo = self.conf.neo;
        let activation_width = self.conf.activation_width;
        let padding = self.conf.wrap_padding;
        let conf_width = self.conf.width;

        let is_arrow_to_right = from_left <= to_left;
        let mut startx = if is_arrow_to_right {
            from_right
        } else {
            from_left
        };
        let mut stopx = if is_arrow_to_right { to_left } else { to_right };

        if neo {
            let offset = 3.0;
            if line_type != LineType::SolidOpen {
                stopx += if is_arrow_to_right { -offset } else { offset };
            }
            if matches!(
                line_type,
                LineType::BidirectionalSolid | LineType::BidirectionalDotted
            ) {
                startx += if is_arrow_to_right { offset } else { -offset };
            }
        }

        startx += central_offset(central, line_type, is_arrow_to_right);
        // The line width is counted, so the bounds of an activation are two pixels off.
        let is_arrow_to_activation = (to_left - to_right).abs() > 2.0;
        let adjust = |value: f64| if is_arrow_to_right { -value } else { value };

        if same_actor {
            // A self message: its stop is its start, as the reference's later checks rely on.
            stopx = startx;
        } else {
            if activate == Some(true) && !is_arrow_to_activation {
                stopx += adjust(activation_width / 2.0 - 1.0);
            }
            // The arrowhead is drawn past the line's end (refX), so the line stops three pixels
            // short, except for the open arrows and the stick and reverse shapes that do not take one.
            if !matches!(
                line_type,
                LineType::SolidOpen
                    | LineType::DottedOpen
                    | LineType::StickTop
                    | LineType::StickBottom
                    | LineType::StickTopDotted
                    | LineType::StickBottomDotted
                    | LineType::SolidArrowTopReverseDotted
                    | LineType::SolidArrowBottomReverseDotted
                    | LineType::StickArrowTopReverse
                    | LineType::StickArrowBottomReverse
                    | LineType::StickArrowTopReverseDotted
                    | LineType::StickArrowBottomReverseDotted
                    | LineType::SolidArrowTopReverse
                    | LineType::SolidArrowBottomReverse
            ) {
                stopx += adjust(3.0);
            }
            if matches!(
                line_type,
                LineType::BidirectionalSolid
                    | LineType::BidirectionalDotted
                    | LineType::SolidArrowTopReverseDotted
                    | LineType::SolidArrowBottomReverseDotted
                    | LineType::SolidArrowTopReverse
                    | LineType::SolidArrowBottomReverse
            ) {
                startx -= adjust(3.0);
            }
        }

        let all = [from_left, from_right, to_left, to_right];
        let bounded_width = (startx - stopx).abs();
        let message_font = &self.conf.message_font;
        let memo = &mut self.memo;
        let msg = &mut self.layout.messages[msg_index];
        if should_wrap && !msg.text.is_empty() {
            msg.text = wrap(
                memo,
                &msg.text,
                get_max(&[Some(bounded_width + 2.0 * padding), Some(conf_width)]),
                message_font,
            )?;
        }
        let dims = dimensions(memo, &msg.text, message_font)?;
        let text = msg.text.clone();
        let width = get_max(&[
            Some(if should_wrap {
                0.0
            } else {
                dims.width + 2.0 * padding
            }),
            Some(bounded_width + 2.0 * padding),
            Some(conf_width),
        ]);
        Ok(Some(MessageModel {
            width,
            height: 0.0,
            startx,
            stopx,
            starty: 0.0,
            stopy: 0.0,
            message: text,
            from_bounds: get_min(&all.map(Some)),
            to_bounds: get_max(&all.map(Some)),
            sequence_index: 0.0,
            sequence_visible: false,
        }))
    }

    /// `activationBounds(actor, actors)` (`:1569-1586`): the span an actor's activations take up,
    /// around its lifeline.
    fn activation_bounds(&self, key: &str) -> Result<(f64, f64), RenderError> {
        let index = self.actor_of(Some(key))?;
        let actor = &self.layout.actors[index];
        let centre = actor.x + actor.width / 2.0;
        let mut left = centre - 1.0;
        let mut right = centre + 1.0;
        for activation in self.bounds.activations.iter().filter(|a| a.actor == key) {
            if let Some(start) = activation.extent.startx {
                left = get_min(&[Some(left), Some(start)]);
            }
            if let Some(stop) = activation.extent.stopx {
                right = get_max(&[Some(right), Some(stop)]);
            }
        }
        Ok((left, right))
    }

    /// `calculateLoopBounds` (`:2034-2144`): the note and message geometry of every item, and the
    /// width of every block, keyed by the id of the item that opens or continues it. The open
    /// activations of this pass are dropped at the end, as the reference does.
    pub(super) fn calculate_loop_bounds(&mut self) -> Result<HashMap<String, f64>, RenderError> {
        let mut arena: Vec<LoopCalc> = Vec::new();
        let mut loops: HashMap<String, usize> = HashMap::new();
        let mut stack: Vec<usize> = Vec::new();
        let label_box_width = self.conf.label_box_width;
        let activation_width = self.conf.activation_width;

        for msg_index in 0..self.layout.messages.len() {
            let line_type = self.layout.messages[msg_index].line_type;
            let id = self.layout.messages[msg_index].id.clone();
            match line_type {
                LineType::LoopStart
                | LineType::AltStart
                | LineType::OptStart
                | LineType::ParStart
                | LineType::ParOverStart
                | LineType::CriticalStart
                | LineType::BreakStart => {
                    arena.push(LoopCalc {
                        id: id.clone(),
                        from: MAX_SAFE,
                        to: -MAX_SAFE,
                        width: 0.0,
                    });
                    stack.push(arena.len() - 1);
                }
                LineType::AltElse | LineType::ParAnd | LineType::CriticalOption => {
                    if !self.layout.messages[msg_index].text.is_empty() {
                        let current = stack.pop().ok_or_else(missing_block)?;
                        loops.insert(arena[current].id.clone(), current);
                        loops.insert(id.clone(), current);
                        stack.push(current);
                    }
                }
                LineType::LoopEnd
                | LineType::AltEnd
                | LineType::OptEnd
                | LineType::ParEnd
                | LineType::CriticalEnd
                | LineType::BreakEnd => {
                    let current = stack.pop().ok_or_else(missing_block)?;
                    loops.insert(arena[current].id.clone(), current);
                }
                LineType::ActiveStart => {
                    let key = self.layout.messages[msg_index]
                        .from
                        .clone()
                        .ok_or_else(missing_block)?;
                    let index = self.actor_of(Some(&key))?;
                    let actor = &self.layout.actors[index];
                    let stacked = self.bounds.activation_count(&key) as f64;
                    let x =
                        actor.x + actor.width / 2.0 + ((stacked - 1.0) * activation_width) / 2.0;
                    self.bounds.activations.push(super::model::Activation {
                        extent: Extent {
                            startx: Some(x),
                            starty: None,
                            stopx: Some(x + activation_width),
                            stopy: None,
                        },
                        actor: key,
                        anchored: 0,
                    });
                }
                LineType::ActiveEnd => {
                    let key = self.layout.messages[msg_index]
                        .from
                        .clone()
                        .unwrap_or_default();
                    // `splice(idx, 1).splice(0, 1)`: the removed activation is dropped, and an
                    // index of -1 removes the last one.
                    let _ = self.bounds.end_activation(&key);
                }
                _ => {}
            }

            let placement = self.layout.messages[msg_index].placement;
            if placement.is_some() {
                let note = self.build_note_model(msg_index)?;
                let (from, width) = (note.startx, note.width);
                self.layout.messages[msg_index].note = Some(note);
                for &open in &stack {
                    let current = &mut arena[open];
                    current.from = get_min(&[Some(current.from), Some(from)]);
                    current.to = get_max(&[Some(current.to), Some(from + width)]);
                    current.width =
                        get_max(&[Some(current.width), Some((current.from - current.to).abs())])
                            - label_box_width;
                }
            } else {
                let model = self.build_message_model(msg_index)?;
                let Some(model) = model else {
                    continue;
                };
                let (startx, stopx) = (model.startx, model.stopx);
                let model_width = model.width;
                self.layout.messages[msg_index].message_model = Some(model);
                if truthy(startx) && truthy(stopx) && !stack.is_empty() {
                    for &open in &stack {
                        if startx == stopx {
                            let msg = &self.layout.messages[msg_index];
                            let from_index = self.actor_of(msg.from.as_deref())?;
                            let to_index = self.actor_of(msg.to.as_deref())?;
                            let from = &self.layout.actors[from_index];
                            let to = &self.layout.actors[to_index];
                            let current = &mut arena[open];
                            current.from = get_min(&[
                                Some(from.x - model_width / 2.0),
                                Some(from.x - from.width / 2.0),
                                Some(current.from),
                            ]);
                            current.to = get_max(&[
                                Some(to.x + model_width / 2.0),
                                Some(to.x + from.width / 2.0),
                                Some(current.to),
                            ]);
                            current.width = get_max(&[
                                Some(current.width),
                                Some((current.to - current.from).abs()),
                            ]) - label_box_width;
                        } else {
                            let current = &mut arena[open];
                            current.from = get_min(&[Some(startx), Some(current.from)]);
                            current.to = get_max(&[Some(stopx), Some(current.to)]);
                            current.width = get_max(&[Some(current.width), Some(model_width)])
                                - label_box_width;
                        }
                    }
                }
            }
        }
        self.bounds.activations.clear();
        Ok(loops
            .into_iter()
            .map(|(id, index)| (id, arena[index].width))
            .collect())
    }

    /// `addActorRenderingData` (`:743-811`): places the actors in a row from the left, with their
    /// margins, the boxes around them, and the first row's height.
    pub(super) fn add_actor_rendering_data(&mut self) {
        let mut prev_width = 0.0_f64;
        let mut prev_margin = 0.0_f64;
        let mut prev_box: Option<usize> = None;
        let mut max_height = 0.0_f64;
        let margin_conf = self.conf.actor_margin;
        let box_margin = self.conf.box_margin;
        let width_conf = self.conf.width;
        let height_conf = self.conf.height;
        for key_index in 0..self.layout.actor_keys.len() {
            let Some(index) = self.layout.actor_index(&self.layout.actor_keys[key_index]) else {
                continue;
            };
            let actor_box = self.layout.actors[index].box_index;
            // The end of a box, when the actor is in another one (`:755-761`).
            if let Some(previous) = prev_box
                && Some(previous) != actor_box
            {
                self.bounds.boxes.push(previous);
                prev_margin += box_margin + self.layout.boxes[previous].margin;
            }
            // The start of a box (`:763-769`).
            if let Some(current) = actor_box
                && Some(current) != prev_box
            {
                let sequence_box = &mut self.layout.boxes[current];
                sequence_box.x = prev_width + prev_margin;
                sequence_box.y = 0.0;
                prev_margin += sequence_box.margin;
            }
            let actor: &mut Actor = &mut self.layout.actors[index];
            actor.width = get_max(&[Some(or_default(actor.width, width_conf)), Some(width_conf)]);
            actor.height = get_max(&[
                Some(or_default(actor.height, height_conf)),
                Some(height_conf),
            ]);
            actor.margin = Some(or_default(actor.margin.unwrap_or(0.0), margin_conf));
            max_height = get_max(&[Some(max_height), Some(actor.height)]);
            if self
                .layout
                .created
                .get(&actor.name)
                .is_some_and(|count| *count != 0)
            {
                prev_margin += actor.width / 2.0;
            }
            actor.x = prev_width + prev_margin;
            actor.starty = self.bounds.vertical_pos;
            let (x, width, height) = (actor.x, actor.width, actor.height);
            self.bounds.insert(box_margin, x, 0.0, x + width, height);

            let actor = &mut self.layout.actors[index];
            prev_width += actor.width + prev_margin;
            if let Some(current) = actor_box {
                let sequence_box = &mut self.layout.boxes[current];
                sequence_box.width = prev_width + sequence_box.margin - sequence_box.x;
            }
            prev_margin = actor.margin.unwrap_or(margin_conf);
            prev_box = actor_box;
        }
        // The end of the last box (`:806-808`).
        if let Some(previous) = prev_box {
            self.bounds.boxes.push(previous);
        }
        self.bounds.bump(max_height);
    }
}

/// `bumpMax` of `getMaxMessageWidthPerActor`: the larger of the value so far and the new one.
fn bump_max(map: &mut BTreeMap<String, f64>, key: &str, value: f64) {
    let entry = map.entry(key.to_owned()).or_insert(0.0);
    *entry = get_max(&[Some(*entry), Some(value)]);
}

/// The error when a message references an undeclared actor.
fn missing_actor() -> RenderError {
    RenderError::Layout("a message refers to an actor that is not declared".to_owned())
}

/// The error of a block that is closed with nothing open, which the reference throws on.
fn missing_block() -> RenderError {
    RenderError::Layout("a block is closed before it is opened".to_owned())
}

/// A block of the `calculateLoopBounds` pass: the reference's `{id, msg, from, to, width}`.
#[derive(Debug, Clone)]
struct LoopCalc {
    id: String,
    from: f64,
    to: f64,
    width: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_central_offset_follows_the_connection_and_the_direction() {
        assert_eq!(
            central_offset(Some(CentralConnection::Central), LineType::Solid, true),
            0.0
        );
        assert_eq!(
            central_offset(Some(CentralConnection::Reverse), LineType::Solid, true),
            4.0
        );
        assert_eq!(
            central_offset(
                Some(CentralConnection::Dual),
                LineType::BidirectionalSolid,
                false
            ),
            -2.0
        );
    }

    #[test]
    fn a_tie_in_a_max_keeps_the_first_value() {
        let mut map = BTreeMap::new();
        bump_max(&mut map, "A", 3.0);
        bump_max(&mut map, "A", 1.0);
        assert_eq!(map.get("A"), Some(&3.0));
    }
}
