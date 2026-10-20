//! The sequence diagram: a port of `diagrams/sequence/sequenceRenderer.ts`, `svgDraw.js`,
//! `styles.js` and `actorBands.ts`. The research is in `docs-port/sequence/NOTES.md`, section 5.
//!
//! The reference draws into one `<svg>` in three passes: the layout (`calculateActorMargins`,
//! `addActorRenderingData`, `calculateLoopBounds`), the message loop that bounds every item and
//! draws the notes and blocks, and the drawing of the actors' rows and the messages. The port
//! follows that order. The children of the root are kept in two lists, because d3's `lower()`
//! moves an element to the front of its parent: [`Renderer::lower`] puts an element in the front
//! list, and [`Renderer::append`] in the back list, which keeps the indices of its anchors.
//!
//! The modules: `conf` (the configuration), `model` (the state of one render), `bounds` (the
//! bounds of the layout), `layout` (the widths and the models), `actors` (the participants),
//! `messages`, `blocks` (notes, blocks, activations, boxes), `markers` (the defs), `text` (the
//! labels), `style` (the style sheet) and `icons` (the paths of the icons).

mod actors;
mod blocks;
mod bounds;
mod conf;
mod icons;
mod layout;
mod markers;
mod messages;
mod model;
mod style;
mod text;

use ferrigura_model::config::SequenceDiagramConfig;
use ferrigura_model::sequence::{LineType, SequenceDiagram};
use ferrigura_text::{EmbeddedFamily, FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

use self::blocks::BlockAction;
use self::bounds::{Bounds, js_round};
use self::conf::Conf;
use self::model::{Block, Layout, Message};
use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::TextMemo;
use crate::common::decode_entities;
use crate::common::viewbox::calculate_svg_size_attrs;
use crate::context::{Context, ThemeView};
use crate::svg::{number, xlink_declaration, xml_references};

/// The namespace of the root element (`mermaidAPI` writes it on every SVG).
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The state of one render (`sequenceRenderer.ts`'s module state, and the DB's sequence flag).
pub(super) struct Renderer<'a> {
    /// The id of the diagram.
    pub(super) id: &'a str,
    /// The theme the diagram draws with: the tokens of its light scheme, and the dark ones when
    /// both schemes are written.
    pub(super) theme: ThemeView<'a>,
    /// The resolved configuration.
    pub(super) conf: Conf,
    /// The text measurements of this render.
    pub(super) memo: TextMemo,
    /// The actors, boxes, messages and their layout.
    pub(super) layout: Layout,
    /// The bounds of the layout (`bounds`).
    pub(super) bounds: Bounds,
    /// The number of the last lifeline drawn (`actorCnt`, starting at `-1`).
    pub(super) actor_cnt: i64,
    /// Whether the sequence numbers are shown (`db.sequenceNumbersEnabled`).
    pub(super) sequence_numbers: bool,
    /// The children of the root that `lower()` moved to the front, in the order they were moved.
    front: Vec<Element>,
    /// The other children of the root, in order.
    pub(super) back: Vec<Element>,
    /// The size of the drawing, set when the drawing is done.
    view: Option<View>,
}

/// The size of the drawing, from the bounds of its content (`sequenceRenderer.ts:1433-1472`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct View {
    /// The left edge of the content, `box.startx`.
    startx: f64,
    /// The width of the drawing, with the margins.
    width: f64,
    /// The height of the drawing, with the margins.
    height: f64,
    /// The extra height above the drawing for the title (`extraVertForTitle`).
    title_extra: f64,
    /// The extra height below for the neo look's footers (`extraHeightForNeoActors`).
    neo_extra: f64,
}

impl<'a> Renderer<'a> {
    /// The renderer of one diagram, with the configuration applied (`bounds.init`).
    fn new(input: &Context<'a, SequenceDiagram, SequenceDiagramConfig>) -> Self {
        Self {
            id: input.id,
            theme: input.theme,
            conf: Conf::new(input.section, input.globals),
            memo: TextMemo::new(),
            layout: Layout::new(input.diagram),
            bounds: Bounds::default(),
            actor_cnt: -1,
            sequence_numbers: false,
            front: Vec::new(),
            back: Vec::new(),
            view: None,
        }
    }

    /// Moves an element to the front of the root (`selection.lower()`), where the reference's
    /// lowered groups go.
    pub(super) fn lower(&mut self, element: Element) {
        self.front.push(element);
    }

    /// Appends an element to the root (`selection.append`), and returns its index in the back list.
    pub(super) fn append(&mut self, element: Element) -> usize {
        self.back.push(element);
        self.back.len() - 1
    }

    /// The children of the root in document order: the lowered ones last-moved first, then the rest.
    fn children(&self) -> Vec<&Element> {
        self.front.iter().rev().chain(self.back.iter()).collect()
    }

    /// Sets `y2` on the lifeline of an actor (`fixLifeLineHeights`, `svgDraw.js:368-381`).
    fn set_lifeline_y2(&mut self, actor_cnt: i64, y2: f64) {
        let id = format!("actor{actor_cnt}");
        let mut done = false;
        let mut visit = |element: &mut Element| {
            if !done && element.get("id") == Some(id.as_str()) {
                element.set("y2", number(y2));
                done = true;
            }
        };
        for element in self.front.iter_mut().chain(self.back.iter_mut()) {
            element.visit_mut(&mut visit);
        }
    }

    /// `draw(text, id, version, diagObj)` (`sequenceRenderer.ts:1043-1172`).
    fn draw(&mut self, title: &str) -> Result<(), RenderError> {
        let max_widths = self.max_message_width_per_actor()?;
        self.conf.height = self.calculate_actor_margins(&max_widths)?;

        // The icons and the arrowheads are appended to the root before the messages.
        for element in markers::icons(self.id) {
            self.append(element);
        }
        let has_boxes = !self.layout.boxes.is_empty();
        let has_box_titles = self.layout.boxes.iter().any(|sequence_box| {
            sequence_box
                .name
                .as_deref()
                .is_some_and(|name| !name.is_empty())
        });
        if has_boxes {
            self.bounds.bump(self.conf.box_margin);
            if has_box_titles {
                let first = self.layout.boxes[0].text_max_height;
                self.bounds.bump(first);
            }
        }

        if self.conf.hide_unused_participants {
            let mut used = std::collections::HashSet::new();
            for message in &self.layout.messages {
                used.insert(message.from.clone());
                used.insert(message.to.clone());
            }
            self.layout
                .actor_keys
                .retain(|key| used.contains(&Some(key.clone())));
        }

        self.add_actor_rendering_data();
        let loop_widths = self.calculate_loop_bounds()?;

        for element in markers::arrow_markers(self.id) {
            self.append(element);
        }
        if self.conf.neo {
            let theme = self.conf.theme.clone();
            self.append(markers::drop_shadow(self.id, &theme));
        }

        let mut sequence_index = 1.0_f64;
        let mut sequence_step = 1.0_f64;
        let mut to_draw: Vec<(usize, f64)> = Vec::new();
        let mut backgrounds: Vec<Block> = Vec::new();
        let box_margin = self.conf.box_margin;
        let text_margin = self.conf.box_text_margin;
        let message_count = self.layout.messages.len();
        for index in 0..message_count {
            let msg = self.layout.messages[index].clone();
            match msg.line_type {
                LineType::Note => {
                    self.draw_note(index)?;
                }
                LineType::ActiveStart
                | LineType::CentralConnection
                | LineType::CentralConnectionReverse => {
                    self.new_activation_for(&msg)?;
                }
                LineType::ActiveEnd => {
                    self.active_end(&msg)?;
                }
                LineType::LoopStart
                | LineType::OptStart
                | LineType::AltStart
                | LineType::CriticalStart
                | LineType::BreakStart => {
                    self.adjust_loop_height_for_wrap(
                        &loop_widths,
                        index,
                        box_margin,
                        box_margin + text_margin,
                        BlockAction::Open { fill: None },
                    )?;
                }
                LineType::ParStart | LineType::ParOverStart => {
                    self.adjust_loop_height_for_wrap(
                        &loop_widths,
                        index,
                        box_margin,
                        box_margin + text_margin,
                        BlockAction::Open { fill: None },
                    )?;
                }
                LineType::RectStart => {
                    let text = self.layout.messages[index].text.clone();
                    let fill = if text.is_empty() {
                        self.rect_default_fill()
                    } else {
                        text
                    };
                    self.adjust_loop_height_for_wrap(
                        &loop_widths,
                        index,
                        box_margin,
                        box_margin,
                        BlockAction::Open { fill: Some(fill) },
                    )?;
                }
                LineType::RectEnd => {
                    let block = self.end_block()?;
                    backgrounds.push(block.clone());
                    self.bounds.bump(stop_y(&block) - self.bounds.vertical_pos);
                }
                LineType::AltElse | LineType::CriticalOption => {
                    self.adjust_loop_height_for_wrap(
                        &loop_widths,
                        index,
                        box_margin + text_margin,
                        box_margin,
                        BlockAction::Section,
                    )?;
                }
                LineType::ParAnd => {
                    self.adjust_loop_height_for_wrap(
                        &loop_widths,
                        index,
                        box_margin + text_margin,
                        box_margin,
                        BlockAction::Section,
                    )?;
                }
                LineType::LoopEnd => {
                    self.close_block(&msg.id, "loop")?;
                }
                LineType::OptEnd => {
                    self.close_block(&msg.id, "opt")?;
                }
                LineType::AltEnd => {
                    self.close_block(&msg.id, "alt")?;
                }
                LineType::ParEnd => {
                    self.close_block(&msg.id, "par")?;
                }
                LineType::CriticalEnd => {
                    self.close_block(&msg.id, "critical")?;
                }
                LineType::BreakEnd => {
                    self.close_block(&msg.id, "break")?;
                }
                LineType::Autonumber => {
                    if let Some(number_item) = &msg.autonumber {
                        if let Some(start) = number_item.start.filter(|value| is_truthy(*value)) {
                            sequence_index = start;
                        }
                        if let Some(step) = number_item.step.filter(|value| is_truthy(*value)) {
                            sequence_step = step;
                        }
                        self.sequence_numbers = number_item.visible;
                    }
                }
                _ => {
                    // A message: its bounds now, its drawing after the actors' row.
                    if let Some(mut model) = self.layout.messages[index].message_model.clone() {
                        model.starty = self.bounds.vertical_pos;
                        model.sequence_index = sequence_index;
                        model.sequence_visible = self.sequence_numbers;
                        // The reference logs a message it cannot bound and goes on.
                        if let Ok(line_start_y) = self.bound_message(&mut model) {
                            self.adjust_created_destroyed(index, &mut model, line_start_y);
                            self.layout.messages[index].message_model = Some(model);
                            to_draw.push((index, line_start_y));
                        }
                    }
                }
            }
            if is_sequence_line(msg.line_type) {
                // The reference rounds to two decimals, against floating-point error.
                sequence_index = js_round((sequence_index + sequence_step) * 100.0) / 100.0;
            }
        }

        self.draw_actors(false);
        for (index, line_start_y) in to_draw {
            let model = self.layout.messages[index]
                .message_model
                .clone()
                .unwrap_or_default();
            self.draw_message(index, &model, line_start_y)?;
        }
        if self.conf.mirror_actors {
            self.draw_actors(true);
        }
        for block in &backgrounds {
            self.draw_rect_background(block);
        }
        self.fix_lifeline_heights();

        for box_index in self.bounds.boxes.clone() {
            let (x, y, width) = {
                let sequence_box = &self.layout.boxes[box_index];
                (sequence_box.x, sequence_box.y, sequence_box.width)
            };
            let height = self.bounds.vertical_pos - y;
            self.bounds.insert(box_margin, x, y, x + width, height);
            let padding = box_margin * 2.0;
            let startx = x - padding;
            let starty = y - padding * 0.25;
            let sequence_box = &mut self.layout.boxes[box_index];
            sequence_box.startx = startx;
            sequence_box.starty = starty;
            sequence_box.stopx = startx + width + 2.0 * padding;
            sequence_box.stopy = starty + height + padding * 0.75;
            self.draw_box(box_index);
        }
        if has_boxes {
            self.bounds.bump(box_margin);
        }

        // The menus of the actors (`drawActorsPopup`), drawn after the boxes: a menu taller or
        // wider than the content makes the drawing that much bigger (`:1424-1441`).
        let (menu_height, menu_width) = self.draw_actors_popups()?;
        let data = self.bounds.data;
        let startx = data.startx.unwrap_or(0.0);
        let starty = data.starty.unwrap_or(0.0);
        let stopx = data.stopx.unwrap_or(0.0);
        let stopy = data.stopy.unwrap_or(0.0);
        let mut box_height = stopy - starty;
        if box_height < menu_height {
            box_height = menu_height;
        }
        let mut height = box_height + 2.0 * self.conf.diagram_margin_y;
        if self.conf.mirror_actors {
            height = height - self.conf.box_margin + self.conf.bottom_margin_adj;
        }
        let mut box_width = stopx - startx;
        if box_width < menu_width {
            box_width = menu_width;
        }
        let width = box_width + 2.0 * self.conf.diagram_margin_x;
        if !title.is_empty() {
            let title_element = Element::new("text")
                .with(
                    "x",
                    number((stopx - startx) / 2.0 - 2.0 * self.conf.diagram_margin_x),
                )
                .with("y", "-25")
                .text(title);
            self.append(title_element);
        }
        let title_extra = if title.is_empty() { 0.0 } else { 40.0 };
        let neo_extra = if !self.layout.actors.is_empty() && self.conf.neo {
            30.0
        } else {
            0.0
        };
        self.view = Some(View {
            startx,
            width,
            height,
            title_extra,
            neo_extra,
        });
        Ok(())
    }

    /// `endLoop()` of a block that must be open: the reference's `pop` of an empty stack is an
    /// error when the block is read.
    fn end_block(&mut self) -> Result<Block, RenderError> {
        self.bounds
            .end_block()
            .ok_or_else(|| RenderError::Layout("a block is closed before it is opened".to_owned()))
    }

    /// The end of a block (`endLoop`, `drawLoop`, `bump`): drawn with its label, then the row
    /// moves past its bottom edge.
    fn close_block(&mut self, end_id: &str, label: &str) -> Result<(), RenderError> {
        let mut block = self.end_block()?;
        self.draw_loop(&mut block, label, end_id)?;
        let stop = stop_y(&block);
        self.bounds.bump(stop - self.bounds.vertical_pos);
        Ok(())
    }

    /// `newActivation` for an activation item (`sequenceRenderer.ts:1155-1163`): the activation
    /// opens on its actor, in a group of its own at the back of the root.
    fn new_activation_for(&mut self, msg: &Message) -> Result<(), RenderError> {
        let key = msg.from.clone().unwrap_or_default();
        let index = self.layout.actor_index(&key).ok_or_else(|| {
            RenderError::Layout("an activation names an undeclared actor".to_owned())
        })?;
        let stacked = self.bounds.activation_count(&key) as f64;
        let actor = &self.layout.actors[index];
        let x = actor.x + actor.width / 2.0 + ((stacked - 1.0) * self.conf.activation_width) / 2.0;
        let anchored = self.append(Element::new("g"));
        let width = self.conf.activation_width;
        self.bounds.new_activation(&key, x, width, anchored);
        Ok(())
    }

    /// `activeEnd` (`sequenceRenderer.ts:1121-1138`): closes the last activation of the actor,
    /// draws it down to the row, and bounds it.
    fn active_end(&mut self, msg: &Message) -> Result<(), RenderError> {
        let key = msg.from.clone().unwrap_or_default();
        let mut vertical_pos = self.bounds.vertical_pos;
        let mut activation = self
            .bounds
            .end_activation(&key)
            .ok_or_else(|| RenderError::Layout("an activation ends with none open".to_owned()))?;
        let starty = activation.extent.starty.unwrap_or(f64::NAN);
        if starty + 18.0 > vertical_pos {
            activation.extent.starty = Some(vertical_pos - 6.0);
            vertical_pos += 12.0;
        }
        let stacked = self.bounds.activation_count(&key);
        self.draw_activation(&activation, vertical_pos, stacked);
        self.bounds.insert(
            self.conf.box_margin,
            activation.extent.startx.unwrap_or(f64::NAN),
            vertical_pos - 10.0,
            activation.extent.stopx.unwrap_or(f64::NAN),
            vertical_pos,
        );
        Ok(())
    }

    /// `svgDraw.fixLifeLineHeights` (`svgDraw.js:368-381`): the lifelines stop at the footer's top
    /// (mirrored) or below the row's bottom.
    fn fix_lifeline_heights(&mut self) {
        let mirror = self.conf.mirror_actors;
        let keys = self.layout.actor_keys.clone();
        for key in keys {
            let Some(index) = self.layout.actor_index(&key) else {
                continue;
            };
            let actor = self.layout.actors[index].clone();
            let Some(cnt) = actor.actor_cnt else {
                continue;
            };
            let stop = actor.stopy.unwrap_or(0.0);
            if !mirror && actor.stopy.is_some_and(is_truthy) {
                self.set_lifeline_y2(cnt, stop + actor.height / 2.0);
            } else if mirror {
                self.set_lifeline_y2(cnt, stop);
            }
        }
    }

    /// The root element: its size and view box, its accessibility texts, its style, the empty group
    /// `mermaidAPI` appends, and the drawing (`mermaidAPI.ts`, `setupGraphViewbox.js`).
    fn write(&self, input: &Context<'_, SequenceDiagram, SequenceDiagramConfig>) -> String {
        let diagram = input.diagram;
        let view = self.view.unwrap_or(View {
            startx: 0.0,
            width: 0.0,
            height: 0.0,
            title_extra: 0.0,
            neo_extra: 0.0,
        });
        // The lowered groups come first, then the style and the empty group `mermaidAPI` appends
        // before the drawing, then the rest (the reference's order in the DOM, `sequenceRenderer`).
        let mut content = String::new();
        for element in self.front.iter().rev() {
            content.push_str(&element.render());
        }
        let mut rest = String::new();
        for element in &self.back {
            rest.push_str(&element.render());
        }
        let mut text = String::new();
        for element in self.children() {
            element.collect_text(&mut text);
        }
        let (families, font_family) = self.embedded_fonts(&text);
        // The root's font size is the theme's `fontSize` (the base styles read it), not the text's.
        let font_size = input
            .theme
            .light
            .font_size
            .text()
            .unwrap_or_else(|| "16px".to_owned());
        let style = style::style_sheet(
            input.id,
            input.theme,
            &font_family,
            &font_size,
            &families,
            self.conf.neo,
        );
        let size = calculate_svg_size_attrs(view.height, view.width, self.conf.use_max_width);
        // An actor's menu links and icons write `xlink:href`.
        let links = self.children().iter().any(|element| element.uses_xlink());
        let mut open = format!(
            "<svg xmlns=\"{SVG_NAMESPACE}\"{} id=\"{}\"",
            xlink_declaration(links),
            crate::svg::escape(input.id)
        );
        for (name, value) in size {
            open.push_str(&format!(" {name}=\"{}\"", crate::svg::escape(&value)));
        }
        let view_box = format!(
            "{} -{} {} {}",
            number(view.startx - self.conf.diagram_margin_x),
            number(self.conf.diagram_margin_y + view.title_extra),
            number(view.width),
            number(view.height + view.title_extra + view.neo_extra),
        );
        open.push_str(&format!(" viewBox=\"{view_box}\""));
        open.push_str(&a11y::root_attributes(
            input.id,
            "sequence",
            &diagram.acc_title,
            &diagram.acc_descr,
        ));
        open.push('>');
        let accessibility = a11y::children(input.id, &diagram.acc_title, &diagram.acc_descr);
        let svg = format!("{open}{accessibility}{content}<style>{style}</style><g/>{rest}</svg>");
        // The entities the parser encoded are decoded in the whole document, as the reference
        // does after rendering (`mermaidAPI.ts:359`), and the references XML does not define are
        // rewritten.
        xml_references(&decode_entities(&svg))
    }

    /// The embedded font faces of the text the diagram draws, and the family name the style
    /// refers to. The regular face is embedded; the bold one as well when a font is heavier.
    fn embedded_fonts(&self, text: &str) -> (Vec<EmbeddedFamily>, String) {
        let regular = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
        let heavy = [
            &self.conf.actor_font,
            &self.conf.note_font,
            &self.conf.message_font,
        ]
        .iter()
        .any(|font| font.weight > 500.0);
        let mut runs = vec![Run {
            text,
            face: regular,
        }];
        if heavy {
            let bold = face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal);
            runs.push(Run { text, face: bold });
        }
        match embed_runs(&runs) {
            Ok(families) => {
                let name = families.first().map_or_else(
                    || self.conf.message_font.family.clone(),
                    |family| family.css_family.clone(),
                );
                (families, name)
            }
            Err(_) => (Vec::new(), self.conf.message_font.family.clone()),
        }
    }
}

/// Whether a line type increments the sequence number after it (`sequenceRenderer.ts:1358-1378`).
fn is_sequence_line(line_type: LineType) -> bool {
    use ferrigura_model::sequence::LineType as L;
    matches!(
        line_type,
        L::SolidOpen
            | L::DottedOpen
            | L::Solid
            | L::SolidTop
            | L::SolidBottom
            | L::StickTop
            | L::StickBottom
            | L::SolidTopDotted
            | L::SolidBottomDotted
            | L::StickTopDotted
            | L::StickBottomDotted
            | L::SolidArrowTopReverse
            | L::SolidArrowBottomReverse
            | L::StickArrowTopReverse
            | L::StickArrowBottomReverse
            | L::SolidArrowTopReverseDotted
            | L::SolidArrowBottomReverseDotted
            | L::StickArrowTopReverseDotted
            | L::StickArrowBottomReverseDotted
            | L::Dotted
            | L::SolidCross
            | L::DottedCross
            | L::SolidPoint
            | L::DottedPoint
            | L::BidirectionalSolid
            | L::BidirectionalDotted
    )
}

/// JavaScript's truthiness of a number.
fn is_truthy(value: f64) -> bool {
    value != 0.0 && !value.is_nan()
}

/// The bottom edge of a block, `NaN` when it has none (as the reference's `undefined`).
fn stop_y(block: &Block) -> f64 {
    block.extent.stopy.unwrap_or(f64::NAN)
}

/// Renders the diagram to an SVG document. The context carries the sequence section of the
/// configuration as its section, and the theme's tokens.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read, or if the layout meets a state the reference
/// reports as an error (an activation that ends with none open, for example).
pub fn render(
    input: &Context<'_, SequenceDiagram, SequenceDiagramConfig>,
) -> Result<String, RenderError> {
    let mut renderer = Renderer::new(input);
    renderer.draw(&input.diagram.title)?;
    Ok(renderer.write(input))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sequence_number_follows_a_line_type_only() {
        assert!(is_sequence_line(LineType::Solid));
        assert!(!is_sequence_line(LineType::Note));
    }
}
