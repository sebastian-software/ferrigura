//! The things drawn around the messages: notes (`drawNote`, `sequenceRenderer.ts:243-286`), the
//! blocks (`drawLoop`, `svgDraw.js:1519-1625`, with the wrapping of their titles,
//! `adjustLoopHeightForWrap`, `:916-934`), the `rect` backgrounds, the activations
//! (`drawActivation`, `svgDraw.js:1477-1518`) and the boxes around actors (`drawBox`,
//! `svgDraw.js:1445-1462`).

use std::collections::HashMap;

use super::Renderer;
use super::bounds::{get_max, js_round};
use super::model::{Activation, Block};
use super::text::{TextData, draw_label_by_tspan, draw_text, line_count, line_height};
use crate::RenderError;
use crate::common::Element;
use crate::common::draw::{RectData, draw_rect, get_note_rect};
use crate::svg::number;

/// What an item does with its block once its title is wrapped (`addLoopFn`).
pub enum BlockAction {
    /// Opens a block with the title (`newLoop`); a `rect` has a fill and no title.
    Open { fill: Option<String> },
    /// Adds a section to the innermost block (`addSectionToLoop`).
    Section,
}

/// The fill of a `rect` with no colour of its own: the theme's background, or grey
/// (`sequenceRenderer.ts:1190-1193`).
fn default_rect_fill(variables: &ferrigura_model::value::Value) -> String {
    let theme = crate::variables::text_of(variables, "rectBkgColor")
        .filter(|text| !text.is_empty())
        .or_else(|| {
            crate::variables::text_of(variables, "actorBkg").filter(|text| !text.is_empty())
        });
    theme.unwrap_or_else(|| "rgba(128, 128, 128, 0.5)".to_owned())
}

impl Renderer<'_> {
    /// The theme's default fill of a `rect` (see [`default_rect_fill`]).
    pub(super) fn rect_default_fill(&self) -> String {
        default_rect_fill(self.themes.primary())
    }

    /// `drawNote` (`:243-286`): a note with its text, in the note's place in the row.
    pub(super) fn draw_note(&mut self, msg_index: usize) -> Result<(), RenderError> {
        let msg = self.layout.messages[msg_index].clone();
        let Some(mut note) = msg.note.clone() else {
            return Ok(());
        };
        let box_margin = self.conf.box_margin;
        let note_margin = self.conf.note_margin;
        self.bounds.bump(box_margin);
        note.height = box_margin;
        note.starty = self.bounds.vertical_pos;
        let width = if note.width != 0.0 && !note.width.is_nan() {
            note.width
        } else {
            self.conf.width
        };
        let font = self.conf.note_font.clone();
        let anchor = self.conf.note_align.clone();
        let text_data = TextData {
            text: &note.message,
            x: note.startx,
            y: note.starty,
            width: Some(width),
            anchor: Some(&anchor),
            valign: Some("center"),
            text_margin: Some(note_margin),
            dy: Some("1em"),
            class: "noteText",
            tspan: true,
            font: &font,
        };
        let lines = draw_text(&text_data)?;
        // The height of the note: the text's lines, rounded, and a margin above and below.
        let mut text_height = 0.0;
        for _ in 0..line_count(&note.message) {
            text_height += line_height(&font)?;
        }
        let text_height = js_round(text_height);
        let rect = {
            let mut rect: RectData = get_note_rect();
            rect.x = note.startx;
            rect.y = note.starty;
            rect.width = width;
            rect.height = text_height + 2.0 * note_margin;
            rect.class = Some("note".to_owned());
            rect
        };
        let mut rect_element = draw_rect(&rect);
        if self.conf.neo {
            rect_element.set("data-look", "neo");
        }
        let mut group = Element::new("g")
            .with("data-et", "note")
            .with("data-id", format!("i{}", msg.id))
            .child(rect_element);
        for element in lines {
            group.push_child(element);
        }
        self.append(group);

        note.height += text_height + 2.0 * note_margin;
        self.bounds.bump(text_height + 2.0 * note_margin);
        note.stopy = note.starty + text_height + 2.0 * note_margin;
        note.stopx = note.startx + width;
        self.bounds.insert(box_margin, note.startx, note.starty, note.stopx, note.stopy);
        Ok(())
    }

    /// `drawLoop` (`svgDraw.js:1519-1619`): a block's frame, its label, its title and its section
    /// titles. The block's height is set from its final extent.
    pub(super) fn draw_loop(
        &mut self,
        block: &mut Block,
        label: &str,
        end_id: &str,
    ) -> Result<(), RenderError> {
        let box_margin = self.conf.box_margin;
        let box_text_margin = self.conf.box_text_margin;
        let font = self.conf.message_font.clone();
        let startx = block.extent.startx.unwrap_or(f64::NAN);
        let stopx = block.extent.stopx.unwrap_or(f64::NAN);
        let starty = block.extent.starty.unwrap_or(f64::NAN);
        let stopy = block.extent.stopy.unwrap_or(f64::NAN);

        let mut group = Element::new("g")
            .with("data-et", "control-structure")
            .with("data-id", format!("i{end_id}"));
        let frame = |x1: f64, y1: f64, x2: f64, y2: f64| {
            Element::new("line")
                .with("x1", number(x1))
                .with("y1", number(y1))
                .with("x2", number(x2))
                .with("y2", number(y2))
                .with("class", "loopLine")
        };
        group.push_child(frame(startx, starty, stopx, starty));
        group.push_child(frame(stopx, starty, stopx, stopy));
        group.push_child(frame(startx, stopy, stopx, stopy));
        group.push_child(frame(startx, starty, startx, stopy));
        for section in &block.sections {
            group.push_child(frame(startx, section.y, stopx, section.y).with("style", "stroke-dasharray: 3, 3;"));
        }

        // The label box: a polygon with its corner cut, then the label in it.
        let label_width = get_max(&[Some(self.conf.label_box_width), Some(50.0)]);
        let label_height = {
            let height = self.conf.label_box_height + if self.conf.neo { 15.0 } else { 0.0 };
            if height == 0.0 || height.is_nan() { 20.0 } else { height }
        };
        let label_x = startx;
        let label_y = starty;
        let cut = 7.0;
        let points = format!(
            "{},{} {},{} {},{} {},{} {},{}",
            number(label_x),
            number(label_y),
            number(label_x + label_width),
            number(label_y),
            number(label_x + label_width),
            number(label_y + label_height - cut),
            number(label_x + label_width - cut * 1.2),
            number(label_y + label_height),
            number(label_x),
            number(label_y + label_height),
        );
        group.push_child(
            Element::new("polygon")
                .with("points", points)
                .with("class", "labelBox"),
        );
        let label_text = TextData {
            text: label,
            x: label_x,
            y: label_y + label_height / 2.0,
            width: Some(label_width),
            anchor: Some("middle"),
            valign: Some("middle"),
            text_margin: Some(box_text_margin),
            dy: None,
            class: "labelText",
            tspan: false,
            font: &font,
        };
        for element in draw_text(&label_text)? {
            group.push_child(element);
        }

        // The title: centred in the block after its label, with no width to anchor it by.
        let title = block.title.clone().unwrap_or_default();
        let title_text = TextData {
            text: &title,
            x: startx + label_width / 2.0 + (stopx - startx) / 2.0,
            y: starty + box_margin + box_text_margin,
            width: None,
            anchor: Some("middle"),
            valign: Some("middle"),
            text_margin: Some(box_text_margin),
            dy: None,
            class: "loopText",
            tspan: true,
            font: &font,
        };
        for element in draw_text(&title_text)? {
            group.push_child(element);
        }

        // The section titles, each under its divider.
        for (index, section_title) in block.section_titles.clone().iter().enumerate() {
            if section_title.is_empty() {
                continue;
            }
            let Some(section) = block.sections.get(index) else {
                continue;
            };
            let section_text = TextData {
                text: section_title,
                x: startx + (stopx - startx) / 2.0,
                y: section.y + box_margin + box_text_margin,
                width: None,
                anchor: Some("middle"),
                valign: Some("middle"),
                text_margin: Some(box_text_margin),
                dy: None,
                class: "sectionTitle",
                tspan: false,
                font: &font,
            };
            for element in draw_text(&section_text)? {
                group.push_child(element);
            }
        }
        block.height = js_round(stopy - starty);
        self.append(group);
        Ok(())
    }

    /// `adjustLoopHeightForWrap` (`:916-934`): a block's title is wrapped to the block's width,
    /// the row is moved down by the pre-margin, the title's height is reserved, and the block is
    /// opened or a section is added with the title.
    pub(super) fn adjust_loop_height_for_wrap(
        &mut self,
        loop_widths: &HashMap<String, f64>,
        msg_index: usize,
        pre_margin: f64,
        post_margin: f64,
        action: BlockAction,
    ) -> Result<(), RenderError> {
        let padding = self.conf.wrap_padding;
        let label_box_height = self.conf.label_box_height;
        self.bounds.bump(pre_margin);
        let mut height_adjust = post_margin;
        let id = self.layout.messages[msg_index].id.clone();
        let text = self.layout.messages[msg_index].text.clone();
        if !id.is_empty() && !text.is_empty() {
            if let Some(&loop_width) = loop_widths.get(&id) {
                let font = self.conf.message_font.clone();
                let wrapped = super::text::wrap(
                    &mut self.memo,
                    &format!("[{text}]"),
                    loop_width - 2.0 * padding,
                    &font,
                )?;
                {
                    let msg = &mut self.layout.messages[msg_index];
                    msg.text = wrapped.clone();
                    msg.width = Some(loop_width);
                    msg.wrap = true;
                }
                let dims = super::text::dimensions(&mut self.memo, &wrapped, &font)?;
                let total_offset = get_max(&[Some(dims.height), Some(label_box_height)]);
                height_adjust = post_margin + total_offset;
            }
        }
        let msg = self.layout.messages[msg_index].clone();
        match action {
            BlockAction::Open { fill } => {
                self.bounds
                    .new_block(Some(msg.text.clone()), msg.wrap, msg.width, fill);
            }
            BlockAction::Section => self.bounds.add_section(msg.text.clone()),
        }
        self.bounds.bump(height_adjust);
        Ok(())
    }

    /// `drawBackgroundRect` of a `rect` block (`svgDrawCommon.ts:54-66`): a rectangle behind the
    /// content, lowered to the back of the diagram.
    pub(super) fn draw_rect_background(&mut self, block: &Block) {
        let rect = RectData {
            x: block.extent.startx.unwrap_or(f64::NAN),
            y: block.extent.starty.unwrap_or(f64::NAN),
            width: block.extent.stopx.unwrap_or(f64::NAN) - block.extent.startx.unwrap_or(f64::NAN),
            height: block.extent.stopy.unwrap_or(f64::NAN) - block.extent.starty.unwrap_or(f64::NAN),
            fill: block.fill.clone(),
            class: Some("rect".to_owned()),
            ..RectData::default()
        };
        self.lower(draw_rect(&rect));
    }

    /// `drawActivation` (`svgDraw.js:1477-1518`): the activation's rectangle, in the group that was
    /// anchored when it opened, with the class of its stack depth.
    pub(super) fn draw_activation(&mut self, activation: &Activation, vertical_pos: f64, stacked: usize) {
        let mut rect = get_note_rect();
        rect.x = activation.extent.startx.unwrap_or(f64::NAN);
        rect.y = activation.extent.starty.unwrap_or(f64::NAN);
        rect.class = Some(format!("activation{}", stacked % 3));
        rect.width = activation.extent.stopx.unwrap_or(f64::NAN) - rect.x;
        rect.height = vertical_pos - rect.y;
        let mut element = draw_rect(&rect);
        if self.conf.neo {
            element.set("data-look", "neo");
        }
        if self.uses_palettes() {
            let (bkg, border) = self.palettes();
            let place = self.layout.actor_index(&activation.actor).map_or(0, |index| self.place_of(index));
            let stroke = super::actors::palette_color(&border, place);
            let fill = super::actors::palette_color(&bkg, place)
                .or_else(|| crate::variables::text_of(self.themes.primary(), "mainBkg"));
            if let Some(style) = super::actors::style_text(&[("stroke", stroke), ("fill", fill)]) {
                element.set("style", style);
            }
        }
        if let Some(anchor) = self.back.get_mut(activation.anchored) {
            anchor.push_child(element);
        }
    }

    /// `drawBox` (`svgDraw.js:1445-1462`): the box behind a group of actors, with its title, and
    /// lowered to the back of the diagram.
    pub(super) fn draw_box(&mut self, box_index: usize) {
        let sequence_box = self.layout.boxes[box_index].clone();
        let mut group = Element::new("g");
        let rect = RectData {
            x: sequence_box.startx,
            y: sequence_box.starty,
            width: sequence_box.stopx - sequence_box.startx,
            height: sequence_box.stopy - sequence_box.starty,
            fill: Some(sequence_box.fill.clone()),
            stroke: Some("rgb(0,0,0, 0.5)".to_owned()),
            class: Some("rect".to_owned()),
            ..RectData::default()
        };
        group.push_child(draw_rect(&rect));
        if let Some(name) = sequence_box.name.as_deref().filter(|name| !name.is_empty()) {
            let label = draw_label_by_tspan(
                name,
                sequence_box.x,
                sequence_box.y + self.conf.box_text_margin + sequence_box.text_max_height / 2.0,
                sequence_box.width,
                0.0,
                "text",
                &self.conf.actor_font,
            );
            for element in label {
                group.push_child(element);
            }
        }
        self.lower(group);
    }
}
