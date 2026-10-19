//! The participants: their rows (`drawActors`, `svgDraw.js:813-849`), their shapes under the two
//! looks, and their lifelines. A participant of an unknown type draws nothing, as the reference's
//! switch falls through (`drawActor`, `svgDraw.js:1361-1404`).
//!
//! Two DOM shapes are used, as the reference builds them. Participant, queue, database and
//! collections draw inside one lowered group that holds the lifeline and an inner group (header)
//! or the shape itself (footer). Boundary, control, entity and the stick figure put the lifeline
//! in a lowered group of its own, always present, and draw the shape in a sibling group.
//!
//! Under `neo` the glyph band of [`header_band`] and [`footer_band`] fixes the vertical geometry
//! (`actorBands.ts`). Under `classic` each shape keeps its legacy coordinates, and the height it
//! reports is measured on the shape (`getBBox`); the same geometry gives those heights here.

use ferrigura_model::value::Value;

use super::Renderer;
use super::bounds::get_max;
use super::model::Actor;
use super::text::draw_label_by_tspan;
use crate::RenderError;
use crate::common::Element;
use crate::common::draw::{RectData, draw_embedded_image, draw_image, draw_rect, get_note_rect};
use crate::svg::number;

/// `ACTOR_TYPE_WIDTH` (`svgDraw.js:17`).
pub const ACTOR_TYPE_WIDTH: f64 = 18.0 * 2.0;

/// `GLYPH_BAND_HEIGHT` (`actorBands.ts:41`).
pub const GLYPH_BAND_HEIGHT: f64 = 44.0;

/// `GLYPH_LABEL_GAP` (`actorBands.ts:44`).
pub const GLYPH_LABEL_GAP: f64 = 6.0;

/// `LABEL_LIFELINE_GAP` (`actorBands.ts:47`).
pub const LABEL_LIFELINE_GAP: f64 = 6.0;

/// The height of the stick figure's glyph, `ACTOR_GLYPH_BOTTOM - ACTOR_GLYPH_TOP` (`svgDraw.js:20-22`).
const ACTOR_GLYPH_HEIGHT: f64 = 60.0 - -5.0;

/// The stack an icon-family actor needs above the datum: the glyph, a gap, the label and a gap
/// (`actorStackHeight`, `actorBands.ts:85`).
#[must_use]
pub fn stack_height(text_height: f64) -> f64 {
    GLYPH_BAND_HEIGHT + GLYPH_LABEL_GAP + text_height + LABEL_LIFELINE_GAP
}

/// The bands of one participant under `neo` (`headerBands` and `footerBands`, `actorBands.ts`).
#[derive(Debug, Clone, Copy)]
pub struct Band {
    /// The datum: where the lifelines start (the header's row bottom).
    pub lifeline_start_y: f64,
    /// The bottom of the glyph band.
    pub glyph_bottom_y: f64,
    /// The vertical centre of the label block.
    pub label_centre_y: f64,
}

/// `headerBands(actorY, rowHeight, textHeight)` (`actorBands.ts:100-108`).
#[must_use]
pub fn header_band(actor_y: f64, row_height: f64, text_height: f64) -> Band {
    let lifeline_start_y = actor_y + row_height;
    Band {
        lifeline_start_y,
        glyph_bottom_y: lifeline_start_y - LABEL_LIFELINE_GAP - text_height - GLYPH_LABEL_GAP,
        label_centre_y: lifeline_start_y - LABEL_LIFELINE_GAP - text_height / 2.0,
    }
}

/// `footerBands(actorY, textHeight)` (`actorBands.ts:111-120`).
#[must_use]
pub fn footer_band(actor_y: f64, text_height: f64) -> Band {
    let glyph_bottom_y = actor_y + LABEL_LIFELINE_GAP + GLYPH_BAND_HEIGHT;
    Band {
        lifeline_start_y: actor_y,
        glyph_bottom_y,
        label_centre_y: glyph_bottom_y + GLYPH_LABEL_GAP + text_height / 2.0,
    }
}

/// The colour of the `index`th actor from a palette, cycling through it. `None` for an empty
/// palette, so that the stylesheet decides (`paletteColor`, `svgDraw.js:94`).
#[must_use]
pub fn palette_color(palette: &[String], index: usize) -> Option<String> {
    if palette.is_empty() {
        None
    } else {
        Some(palette[index % palette.len()].clone())
    }
}

/// The `style` attribute d3 writes for the properties given: `name: value;` for each one that is
/// set, and nothing when none is.
#[must_use]
pub fn style_text(properties: &[(&str, Option<String>)]) -> Option<String> {
    let text: String = properties
        .iter()
        .filter_map(|(name, value)| value.as_ref().map(|value| format!("{name}: {value};")))
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then_some(text)
}

/// The lifeline of an actor: a `line` from its centre down to `y2 = 2000`, which
/// `fixLifeLineHeights` clips later (`svgDraw.js:400-412`).
fn lifeline(actor_cnt: i64, centre: f64, centre_y: f64, name: &str) -> Element {
    Element::new("line")
        .with("id", format!("actor{actor_cnt}"))
        .with("x1", number(centre))
        .with("y1", number(centre_y))
        .with("x2", number(centre))
        .with("y2", "2000")
        .with("class", "actor-line 200")
        .with("stroke-width", "0.5px")
        .with("stroke", "#999")
        .with("name", name)
        .with("data-et", "life-line")
        .with("data-id", name)
}

/// A property of the actor, when it is a non-empty string (`actor.properties?.class`).
fn property_text(actor: &Actor, key: &str) -> Option<String> {
    match actor.properties.get(key) {
        Some(Value::String(text)) if !text.is_empty() => Some(text.clone()),
        _ => None,
    }
}

impl Renderer<'_> {
    /// The background and border palettes of the theme (`bkgColorArray`, `borderColorArray`).
    pub(super) fn palettes(&self) -> (Vec<String>, Vec<String>) {
        let tokens = self.theme.light;
        (
            tokens.bkg_color_array.texts(),
            tokens.border_color_array.texts(),
        )
    }

    /// Whether the theme takes its actor colours from the palettes (`COLOR_THEMES`).
    pub(super) fn uses_palettes(&self) -> bool {
        matches!(self.conf.theme.as_str(), "redux-color" | "redux-dark-color")
    }

    /// The stroke and fill of an actor's shape: the palette colour of its place under the palette
    /// themes, else the theme's own (`actorBorder`, `actorBkg`).
    fn shape_style(&self, index: usize, fill: bool) -> Vec<(&'static str, Option<String>)> {
        let mut style = Vec::with_capacity(2);
        if self.uses_palettes() {
            let (bkg, border) = self.palettes();
            let place = self.place_of(index);
            style.push(("stroke", palette_color(&border, place)));
            if fill {
                style.push(("fill", palette_color(&bkg, place)));
            }
        } else {
            style.push(("stroke", self.theme.light.actor_border.text()));
            if fill {
                style.push(("fill", self.theme.light.actor_bkg.text()));
            }
        }
        style
    }

    /// The place of an actor among the actors drawn (`actorIndexMap`, the index in the actor keys).
    pub(super) fn place_of(&self, index: usize) -> usize {
        let name = &self.layout.actors[index].name;
        self.layout
            .actor_keys
            .iter()
            .filter_map(|key| self.layout.actor_index(key))
            .position(|other| self.layout.actors[other].name == *name)
            .unwrap_or(0)
    }

    /// `drawActors` (`svgDraw.js` callers, `sequenceRenderer.ts:813-849`): the header row, or the
    /// footer row after the messages, which first moves the row down by two box margins.
    pub(super) fn draw_actors(&mut self, footer: bool) {
        let keys = self.layout.actor_keys.clone();
        if !footer {
            for key in &keys {
                if let Some(index) = self.layout.actor_index(key) {
                    self.draw_actor(index, false);
                }
            }
            return;
        }
        let box_margin = self.conf.box_margin;
        self.bounds.bump(box_margin * 2.0);
        let mut max_height = 0.0_f64;
        for key in &keys {
            let Some(index) = self.layout.actor_index(key) else {
                continue;
            };
            if self.layout.actors[index].stopy.is_none() {
                self.layout.actors[index].stopy = Some(self.bounds.vertical_pos);
            }
            if let Some(height) = self.draw_actor(index, true) {
                max_height = get_max(&[Some(max_height), Some(height)]);
            }
        }
        self.bounds.bump(max_height + box_margin);
    }

    /// `drawActor` (`svgDraw.js:1361-1404`): the shape of one actor in its row. Returns the
    /// actor's height, or `None` for an unknown type, which draws nothing.
    pub(super) fn draw_actor(&mut self, index: usize, footer: bool) -> Option<f64> {
        let actor_type = self.layout.actors[index].actor_type.clone();
        Some(match actor_type.as_str() {
            "actor" => self.draw_stick_figure(index, footer),
            "participant" => self.draw_participant(index, footer),
            "boundary" => self.draw_boundary(index, footer),
            "control" => self.draw_control(index, footer),
            "entity" => self.draw_entity(index, footer),
            "database" => self.draw_database(index, footer),
            "collections" => self.draw_collections(index, footer),
            "queue" => self.draw_queue(index, footer),
            _ => return None,
        })
    }

    /// The top of an actor's row: `starty`, or `stopy` for the footer.
    fn row_top(&self, index: usize, footer: bool) -> f64 {
        let actor = &self.layout.actors[index];
        if footer {
            actor.stopy.unwrap_or(0.0)
        } else {
            actor.starty
        }
    }

    /// The bands of an actor under `neo`, or `None` under `classic`.
    fn bands(&self, index: usize, footer: bool, actor_y: f64) -> Option<Band> {
        if !self.conf.neo {
            return None;
        }
        let actor = &self.layout.actors[index];
        let text_height = actor.actor_text_height.unwrap_or(0.0);
        Some(if footer {
            footer_band(actor_y, text_height)
        } else {
            header_band(actor_y, actor.height, text_height)
        })
    }

    /// `actorCnt++` and the number of the lifeline the actor gets (`svgDraw.js:366`, `:400-402`).
    fn next_lifeline(&mut self, index: usize) -> i64 {
        self.actor_cnt += 1;
        self.layout.actors[index].actor_cnt = Some(self.actor_cnt);
        self.actor_cnt
    }

    /// The rectangle of a box-shaped participant: a note rectangle with the actor's class, and the
    /// grey fill unless the actor has a class of its own (`svgDraw.js:406-416`, `:527-537`).
    fn actor_rect(&self, actor: &Actor, actor_y: f64, footer: bool) -> RectData {
        let mut rect = get_note_rect();
        let mut class = "actor".to_owned();
        match property_text(actor, "class") {
            Some(custom) => class = custom,
            None => rect.fill = Some("#eaeaea".to_owned()),
        }
        class.push_str(if footer {
            " actor-bottom"
        } else {
            " actor-top"
        });
        rect.x = actor.x;
        rect.y = actor_y;
        rect.width = actor.width;
        rect.height = actor.height;
        rect.class = Some(class);
        rect.name = Some(actor.name.clone());
        rect
    }

    /// The icon of an actor (`properties.icon`): an embedded element for `@id`, else an image.
    fn icon(&self, actor: &Actor, x: f64, y: f64, width: f64) -> Option<Element> {
        let source = property_text(actor, "icon")?;
        let source = source.trim();
        let (ix, iy) = (x + width - 20.0, y + 10.0);
        match source.strip_prefix('@') {
            // A `use` is not in the sanitiser's allow-list, so a sanitised diagram loses it.
            Some(_) if self.conf.sanitized => None,
            Some(id) => Some(draw_embedded_image(ix, iy, id)),
            None => Some(draw_image(ix, iy, source)),
        }
    }

    /// The label of an actor, placed as `byTspan` places it, in its class.
    fn label(
        &self,
        actor: &Actor,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        class: &str,
    ) -> Vec<Element> {
        draw_label_by_tspan(
            &actor.description,
            x,
            y,
            width,
            height,
            class,
            &self.conf.actor_font,
        )
    }

    /// Places a participant of the first shape: the lowered group of the lifeline and the inner
    /// group (header), or the group itself (footer). `group` carries the attributes the reference
    /// sets on it; `children` are the shape's elements.
    fn place_grouped(
        &mut self,
        footer: bool,
        lifeline: Element,
        mut group: Element,
        children: Vec<Element>,
    ) {
        for child in children {
            group.push_child(child);
        }
        if footer {
            self.lower(group);
        } else {
            let outer = Element::new("g").child(lifeline).child(group);
            self.lower(outer);
        }
    }

    /// `drawActorTypeParticipant` (`svgDraw.js:391-506`): a box with the label in it.
    fn draw_participant(&mut self, index: usize, footer: bool) -> f64 {
        let actor = self.layout.actors[index].clone();
        let actor_y = self.row_top(index, footer);
        let neo = self.conf.neo;
        let rect_source = self.actor_rect(&actor, actor_y, footer);
        let mut rect = rect_source.clone();
        rect.rx = Some(if neo { 6.0 } else { 3.0 });
        rect.ry = rect.rx;
        let mut group = Element::new("g");
        let mut lifeline_element = Element::new("g");
        if !footer {
            let cnt = self.next_lifeline(index);
            let centre = actor.x + actor.width / 2.0;
            lifeline_element = lifeline(cnt, centre, actor_y + actor.height, &actor.name);
            group = Element::new("g").with("id", format!("root-{cnt}"));
            if neo {
                group.set("data-look", "neo");
            }
        }
        self.layout.actors[index].rect_data = Some(rect.clone());
        let mut shape = draw_rect(&rect);
        if neo {
            shape.set("data-look", "neo");
        }
        if let Some(style) =
            style_text(&self.shape_style(index, true)).filter(|_| self.uses_palettes())
        {
            shape.set("style", style);
        }
        if neo {
            shape.set("filter", format!("url(#{}-drop-shadow)", self.id));
        }
        let mut children = vec![shape];
        children.extend(self.icon(&actor, rect.x, rect.y, rect.width));
        children.extend(self.label(
            &actor,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            "actor actor-box",
        ));
        if !footer {
            group.set("data-et", "participant");
            group.set("data-type", "participant");
            group.set("data-id", actor.name.clone());
        }
        self.place_grouped(footer, lifeline_element, group, children);
        // The height is the row's under `neo`; under `classic` the rectangle's own height.
        self.layout.actors[index].height
    }

    /// `drawActorTypeCollections` (`svgDraw.js:516-639`): a box with a copy stacked behind it.
    fn draw_collections(&mut self, index: usize, footer: bool) -> f64 {
        let actor = self.layout.actors[index].clone();
        let actor_y = self.row_top(index, footer);
        let neo = self.conf.neo;
        let mut rect = self.actor_rect(&actor, actor_y, footer);
        // The stacked copy sits six pixels up and to the left. Under `neo` the main box stops six
        // pixels above the datum, so that the copy does not cross it (`:577-580`).
        let offset = 6.0;
        if neo {
            rect.height = actor.height - offset;
        }
        let shadow = RectData {
            x: rect.x - offset,
            y: rect.y + offset,
            class: Some("actor".to_owned()),
            ..rect.clone()
        };
        let mut lifeline_element = Element::new("g");
        let mut group = Element::new("g");
        if !footer {
            let cnt = self.next_lifeline(index);
            let centre = actor.x + actor.width / 2.0;
            lifeline_element = lifeline(cnt, centre, actor_y + actor.height, &actor.name);
            group = Element::new("g").with("id", format!("root-{cnt}"));
            if neo {
                group.set("data-look", "neo");
            }
        }
        self.layout.actors[index].rect_data = Some(rect.clone());
        let mut main = draw_rect(&rect);
        let mut stacked = draw_rect(&shadow);
        // `svgDraw.drawRect` marks every rectangle of the neo look.
        if neo {
            main.set("data-look", "neo");
            stacked.set("data-look", "neo");
        }
        if let Some(style) =
            style_text(&self.shape_style(index, true)).filter(|_| self.uses_palettes())
        {
            main.set("style", style.clone());
            stacked.set("style", style);
        }
        if neo {
            group.set("filter", format!("url(#{}-drop-shadow)", self.id));
        }
        let mut children = vec![main, stacked];
        children.extend(self.icon(&actor, rect.x, rect.y, rect.width));
        children.extend(self.label(
            &actor,
            rect.x - offset,
            rect.y + offset,
            rect.width,
            rect.height,
            "actor actor-box",
        ));
        if !footer {
            group.set("data-et", "participant");
            group.set("data-type", "collections");
            group.set("data-id", actor.name.clone());
        }
        self.place_grouped(footer, lifeline_element, group, children);
        // The main box is the row's height (`getBBox` of its rectangle), so it is unchanged.
        self.layout.actors[index].height
    }

    /// `drawActorTypeQueue` (`svgDraw.js:641-779`): a cylinder lying on its side.
    fn draw_queue(&mut self, index: usize, footer: bool) -> f64 {
        let actor = self.layout.actors[index].clone();
        let actor_y = self.row_top(index, footer);
        let neo = self.conf.neo;
        let rect = self.actor_rect(&actor, actor_y, footer);
        let mut lifeline_element = Element::new("g");
        let mut group = Element::new("g");
        if !footer {
            let cnt = self.next_lifeline(index);
            let centre = actor.x + actor.width / 2.0;
            lifeline_element = lifeline(cnt, centre, actor_y + actor.height, &actor.name);
            group = Element::new("g").with("id", format!("root-{cnt}"));
            if neo {
                group.set("data-look", "neo");
            }
        }
        group.set("class", rect.class.clone().unwrap_or_default());
        self.layout.actors[index].rect_data = Some(rect.clone());

        // The cylinder: its ends are half ellipses of radius `ry = h/2` and `rx = ry/(2.5 + h/50)`
        // (`:702-703`).
        let h = rect.height;
        let ry = h / 2.0;
        let rx = ry / (2.5 + h / 50.0);
        let body = format!(
            "M {},{}\n    a {},{} 0 0 0 0,{}\n    h {}\n    a {},{} 0 0 0 0,-{}\n    Z\n  ",
            number(rect.x),
            number(rect.y + ry),
            number(rx),
            number(ry),
            number(h),
            number(rect.width - 2.0 * rx),
            number(rx),
            number(ry),
            number(h),
        );
        let arc = format!(
            "M {},{}\n      a {},{} 0 0 0 0,{}",
            number(rect.x),
            number(rect.y + ry),
            number(rx),
            number(ry),
            number(h),
        );
        let mut cylinder_group = Element::new("g").child(Element::new("path").with("d", body));
        let mut cylinder_arc = Element::new("g").child(Element::new("path").with("d", arc));
        cylinder_group.set(
            "transform",
            format!("translate({}, {})", number(rx), number(-(h / 2.0))),
        );
        cylinder_arc.set(
            "transform",
            format!(
                "translate({}, {})",
                number(rect.width - rx),
                number(-(h / 2.0))
            ),
        );
        if neo {
            cylinder_group.set("filter", format!("url(#{}-drop-shadow)", self.id));
        }
        if let Some(style) =
            style_text(&self.shape_style(index, true)).filter(|_| self.uses_palettes())
        {
            cylinder_group.set("style", style.clone());
            cylinder_arc.set("style", style);
        }
        let mut children = vec![cylinder_group, cylinder_arc];
        children.extend(self.icon(&actor, rect.x, rect.y, rect.width));
        children.extend(self.label(
            &actor,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            "actor actor-box",
        ));
        if !footer {
            group.set("data-et", "participant");
            group.set("data-type", "queue");
            group.set("data-id", actor.name.clone());
        }
        self.place_grouped(footer, lifeline_element, group, children);
        // The cylinder's path is as high as the rectangle (`getBBox` of its last path, `h`).
        self.layout.actors[index].height
    }

    /// `drawActorTypeDatabase` (`svgDraw.js:999-1128`): a cylinder standing on the datum.
    fn draw_database(&mut self, index: usize, footer: bool) -> f64 {
        let actor = self.layout.actors[index].clone();
        let actor_y = self.row_top(index, footer);
        let band = self.bands(index, footer, actor_y);
        let neo = self.conf.neo;
        let rect = self.actor_rect(&actor, actor_y, footer);
        let mut lifeline_element = Element::new("g");
        let mut group = Element::new("g");
        if !footer {
            let cnt = self.next_lifeline(index);
            let centre = actor.x + actor.width / 2.0;
            let centre_y = match band {
                Some(band) => band.lifeline_start_y,
                None => actor_y + actor.height + 2.0 * self.conf.box_text_margin,
            };
            lifeline_element = lifeline(cnt, centre, centre_y, &actor.name);
            group = Element::new("g").with("id", format!("root-{cnt}"));
            if neo {
                group.set("data-look", "neo");
            }
        }
        // The width is a third of the row's. Under `neo` the height is the band's, and the icon
        // stands on the glyph band's bottom edge; under `classic` it is the width's third.
        let w = rect.width / 3.0;
        let rx = w / 2.0;
        let ry = rx / (2.5 + w / 50.0);
        let h = if band.is_some() {
            GLYPH_BAND_HEIGHT
        } else {
            rect.width / 3.0
        };
        let cylinder_y = match band {
            Some(band) => band.glyph_bottom_y - h - ry,
            None => rect.y,
        };
        let d = format!(
            "\n  M {},{}\n  a {},{} 0 0 0 {},0\n  a {},{} 0 0 0 -{},0\n  l 0,{}\n  a {},{} 0 0 0 {},0\n  l 0,-{}\n",
            number(rect.x),
            number(cylinder_y + ry),
            number(rx),
            number(ry),
            number(w),
            number(rx),
            number(ry),
            number(w),
            number(h - 2.0 * ry),
            number(rx),
            number(ry),
            number(w),
            number(h - 2.0 * ry),
        );
        self.layout.actors[index].rect_data = Some(rect.clone());
        let mut cylinder_group =
            Element::new("g").with("class", rect.class.clone().unwrap_or_default());
        cylinder_group.push_child(Element::new("path").with("d", d));
        if neo {
            cylinder_group.set("filter", format!("url(#{}-drop-shadow)", self.id));
        }
        if self.uses_palettes() {
            if let Some(style) = style_text(&self.shape_style(index, true)) {
                cylinder_group.set("style", style);
            }
        } else {
            cylinder_group.set(
                "style",
                format!(
                    "stroke: {};",
                    self.theme.light.actor_border.text().unwrap_or_default()
                ),
            );
        }
        cylinder_group.set(
            "transform",
            format!("translate({}, {})", number(w), number(ry)),
        );
        let label_y = match band {
            Some(band) => band.label_centre_y - rect.height / 2.0,
            None => rect.y + 35.0,
        };
        let mut children = vec![cylinder_group];
        children.extend(self.label(
            &actor,
            rect.x,
            label_y,
            rect.width,
            rect.height,
            "actor actor-box",
        ));
        if !footer {
            group.set("data-et", "participant");
            group.set("data-type", "database");
            group.set("data-id", actor.name.clone());
        }
        self.place_grouped(footer, lifeline_element, group, children);
        if band.is_none() {
            // Classic: the cylinder's own height and the label box's (`:1113-1119`).
            let height = h + self.conf.label_box_height;
            self.layout.actors[index].height = height;
        }
        self.layout.actors[index].height
    }

    /// Places a shape that has its lifeline in a lowered group of its own, always present (the
    /// empty group of a footer included), and its shape in a sibling group.
    fn place_beside(&mut self, lifeline_element: Option<Element>) {
        let mut line = Element::new("g");
        if let Some(lifeline_element) = lifeline_element {
            line.push_child(lifeline_element);
        }
        self.lower(line);
    }

    /// `drawActorTypeBoundary` (`svgDraw.js:1130-1238`): a circle with a torso and arms.
    fn draw_boundary(&mut self, index: usize, footer: bool) -> f64 {
        let actor = self.layout.actors[index].clone();
        let actor_y = self.row_top(index, footer);
        let band = self.bands(index, footer, actor_y);
        let neo = self.conf.neo;
        let radius = 22.0;
        let rect = self.actor_rect(&actor, actor_y, footer);
        let row_height = rect.height;
        let icon_centre_y = match band {
            Some(band) => band.glyph_bottom_y - radius,
            None => actor_y + 12.0,
        };
        let mut lifeline_element = None;
        let cnt = if footer {
            self.actor_cnt
        } else {
            self.next_lifeline(index)
        };
        if !footer {
            let centre = actor.x + actor.width / 2.0;
            let centre_y = match band {
                Some(band) => band.lifeline_start_y,
                None => actor_y + 80.0,
            };
            lifeline_element = Some(lifeline(cnt, centre, centre_y, &actor.name));
        }
        self.place_beside(lifeline_element);
        let centre = actor.x + actor.width / 2.0;
        let mut act = Element::new("g")
            .with(
                "class",
                format!(
                    "actor-man {}",
                    if footer { "actor-bottom" } else { "actor-top" }
                ),
            )
            .with("name", actor.name.clone());
        act.push_child(
            Element::new("line")
                .with("id", format!("actor-man-torso{cnt}"))
                .with("x1", number(centre - radius * 2.5))
                .with("y1", number(icon_centre_y))
                .with("x2", number(centre - 15.0))
                .with("y2", number(icon_centre_y)),
        );
        act.push_child(
            Element::new("line")
                .with("id", format!("actor-man-arms{cnt}"))
                .with("x1", number(centre - radius * 2.5))
                .with("y1", number(icon_centre_y - 10.0))
                .with("x2", number(centre - radius * 2.5))
                .with("y2", number(icon_centre_y + 10.0)),
        );
        act.push_child(
            Element::new("circle")
                .with("cx", number(centre))
                .with("cy", number(icon_centre_y))
                .with("r", number(radius)),
        );
        if neo {
            act.set("filter", format!("url(#{}-drop-shadow)", self.id));
        }
        if self.uses_palettes() {
            if let Some(style) = style_text(&self.shape_style(index, true)) {
                act.set("style", style);
            }
        } else {
            act.set(
                "style",
                format!(
                    "stroke: {};",
                    self.theme.light.actor_border.text().unwrap_or_default()
                ),
            );
        }
        if band.is_none() {
            // Classic: the circle's own height and the label box's (`:1200-1206`).
            self.layout.actors[index].height = 2.0 * radius + self.conf.label_box_height;
        }
        let label_y = match band {
            Some(band) => band.label_centre_y - row_height / 2.0,
            None => actor_y + 15.0,
        };
        let label_rect_height = row_height;
        let label = self.label(
            &actor,
            rect.x,
            label_y,
            rect.width,
            label_rect_height,
            "actor actor-man",
        );
        for child in label {
            act.push_child(child);
        }
        if band.is_none() {
            // The legacy nudge; under the band model the circle is placed exactly.
            act.set(
                "transform",
                format!("translate(0,{})", number(radius / 2.0 + 10.0)),
            );
        }
        if !footer {
            act.set("data-et", "participant");
            act.set("data-type", "boundary");
            act.set("data-id", actor.name.clone());
        }
        self.append_shape(act);
        self.layout.actors[index].height
    }

    /// Adds a shape group after the others, as a sibling of the lowered lifeline group.
    fn append_shape(&mut self, element: Element) {
        self.append(element);
    }

    /// `drawActorTypeControl` (`svgDraw.js:781-889`): a circle with a looping arrow on it.
    fn draw_control(&mut self, index: usize, footer: bool) -> f64 {
        let actor = self.layout.actors[index].clone();
        let actor_y = self.row_top(index, footer);
        let band = self.bands(index, footer, actor_y);
        let neo = self.conf.neo;
        let rect = self.actor_rect(&actor, actor_y, footer);
        let row_height = rect.height;
        let centre = actor.x + actor.width / 2.0;
        let centre_y = band.map_or(actor_y + 75.0, |band| band.lifeline_start_y);
        let cx = centre;
        let cy = match band {
            Some(band) => band.glyph_bottom_y - 22.0,
            None => actor_y + 32.0,
        };
        let r = 22.0;
        let mut lifeline_element = None;
        let cnt = if footer {
            self.actor_cnt
        } else {
            self.next_lifeline(index)
        };
        if !footer {
            lifeline_element = Some(lifeline(cnt, centre, centre_y, &actor.name));
        }
        self.place_beside(lifeline_element);

        let marker = Element::new("defs").child(
            Element::new("marker")
                .with("id", format!("{}-filled-head-control", self.id))
                .with("refX", "11")
                .with("refY", "5.8")
                .with("markerWidth", "20")
                .with("markerHeight", "28")
                .with("orient", "172.5")
                .with("stroke-width", "1.2")
                .child(
                    Element::new("path").with("d", "M 14.4 5.6 L 7.2 10.4 L 8.8 5.6 L 7.2 0.8 Z"),
                ),
        );
        let mut act = Element::new("g")
            .with(
                "class",
                format!(
                    "actor-man {}",
                    if footer { "actor-bottom" } else { "actor-top" }
                ),
            )
            .with("name", actor.name.clone())
            .child(marker);
        let filter = if neo {
            format!("url(#{}-drop-shadow)", self.id)
        } else {
            String::new()
        };
        act.push_child(
            Element::new("circle")
                .with("cx", number(cx))
                .with("cy", number(cy))
                .with("r", number(r))
                .with("filter", filter),
        );
        act.push_child(
            Element::new("line")
                .with(
                    "marker-end",
                    format!("url(#{}-filled-head-control)", self.id),
                )
                .with(
                    "transform",
                    format!("translate({}, {})", number(cx), number(cy - r)),
                ),
        );
        if self.uses_palettes() {
            if let Some(style) = style_text(&self.shape_style(index, true)) {
                act.set("style", style);
            }
        } else {
            let (stroke, fill) = (
                self.theme.light.actor_border.text(),
                self.theme.light.actor_bkg.text(),
            );
            act.set(
                "style",
                style_text(&[("stroke", stroke), ("fill", fill)]).unwrap_or_default(),
            );
        }
        if band.is_none() {
            // Classic: the arrow's extent is the circle's, and the label box is added twice.
            self.layout.actors[index].height = 2.0 * r + 2.0 * self.conf.label_box_height;
        }
        let label_y = match band {
            Some(band) => band.label_centre_y - row_height / 2.0,
            None => rect.y + r + if footer { 5.0 } else { 12.0 },
        };
        let label = self.label(
            &actor,
            rect.x,
            label_y,
            rect.width,
            row_height,
            "actor actor-man",
        );
        for child in label {
            act.push_child(child);
        }
        if !footer {
            act.set("data-et", "participant");
            act.set("data-type", "control");
            act.set("data-id", actor.name.clone());
        }
        self.append_shape(act);
        self.layout.actors[index].height
    }

    /// `drawActorTypeEntity` (`svgDraw.js:891-997`): a circle with a bar under it.
    fn draw_entity(&mut self, index: usize, footer: bool) -> f64 {
        let actor = self.layout.actors[index].clone();
        let actor_y = self.row_top(index, footer);
        let band = self.bands(index, footer, actor_y);
        let neo = self.conf.neo;
        let rect = self.actor_rect(&actor, actor_y, footer);
        let row_height = rect.height;
        let centre = actor.x + actor.width / 2.0;
        let centre_y = band.map_or(actor_y + 75.0, |band| band.lifeline_start_y);
        let cx = centre;
        let cy = match band {
            Some(band) => band.glyph_bottom_y - 22.0,
            None => actor_y + if footer { 10.0 } else { 25.0 },
        };
        let r = 22.0;
        let mut act = Element::new("g")
            .with(
                "class",
                format!(
                    "actor {}",
                    if footer { "actor-bottom" } else { "actor-top" }
                ),
            )
            .with("name", actor.name.clone());
        act.push_child(
            Element::new("circle")
                .with("cx", number(cx))
                .with("cy", number(cy))
                .with("r", number(r))
                .with("width", number(actor.width))
                .with("height", number(row_height)),
        );
        act.push_child(
            Element::new("line")
                .with("x1", number(cx - r))
                .with("x2", number(cx + r))
                .with("y1", number(cy + r))
                .with("y2", number(cy + r))
                .with("stroke-width", "2"),
        );
        if neo {
            act.set("filter", format!("url(#{}-drop-shadow)", self.id));
        }
        if self.uses_palettes()
            && let Some(style) = style_text(&self.shape_style(index, true))
        {
            act.set("style", style);
        }
        if band.is_none() {
            // Classic: the circle's extent plus the label box's height (`:936-937`).
            self.layout.actors[index].height = 2.0 * r + self.conf.label_box_height;
        }
        let mut lifeline_element = None;
        let cnt = if footer {
            self.actor_cnt
        } else {
            self.next_lifeline(index)
        };
        if !footer {
            lifeline_element = Some(lifeline(cnt, centre, centre_y, &actor.name));
        }
        self.place_beside(lifeline_element);
        let label_y = match band {
            Some(band) => band.label_centre_y - row_height / 2.0,
            None => rect.y + if footer { 15.0 } else { 30.0 },
        };
        let label = self.label(
            &actor,
            rect.x,
            label_y,
            rect.width,
            row_height,
            "actor actor-man",
        );
        for child in label {
            act.push_child(child);
        }
        if band.is_none() {
            // The legacy nudge; under the band model the circle is placed exactly.
            let nudge = if footer { r } else { r / 2.0 - 5.0 };
            act.set("transform", format!("translate(0, {})", number(nudge)));
        }
        if !footer {
            act.set("data-et", "participant");
            act.set("data-type", "entity");
            act.set("data-id", actor.name.clone());
        }
        self.append_shape(act);
        self.layout.actors[index].height
    }

    /// `drawActorTypeActor` (`svgDraw.js:1240-1359`): a stick figure.
    fn draw_stick_figure(&mut self, index: usize, footer: bool) -> f64 {
        let actor = self.layout.actors[index].clone();
        let actor_y = self.row_top(index, footer);
        let band = self.bands(index, footer, actor_y);
        let centre = actor.x + actor.width / 2.0;
        let centre_y = band.map_or(actor_y + 80.0, |band| band.lifeline_start_y);
        let mut lifeline_element = None;
        let cnt = if footer {
            self.actor_cnt
        } else {
            self.next_lifeline(index)
        };
        if !footer {
            lifeline_element = Some(lifeline(cnt, centre, centre_y, &actor.name));
        }
        self.place_beside(lifeline_element);
        let mut act = Element::new("g")
            .with(
                "class",
                format!(
                    "actor-man {}",
                    if footer { "actor-bottom" } else { "actor-top" }
                ),
            )
            .with("name", actor.name.clone());
        if !footer {
            act.set("data-et", "participant");
            act.set("data-type", "actor");
            act.set("data-id", actor.name.clone());
        }
        // Under the band model the figure is scaled to the glyph band, its feet on the band's
        // bottom edge; `classic` draws the legacy coordinates.
        let scale = band.map_or(1.0, |_| GLYPH_BAND_HEIGHT / ACTOR_GLYPH_HEIGHT);
        let gy = |offset: f64| match band {
            Some(band) => band.glyph_bottom_y - (60.0 - offset) * scale,
            None => actor_y + offset,
        };
        let gx = |offset: f64| centre + offset * scale;
        let half = ACTOR_TYPE_WIDTH / 2.0;
        act.push_child(
            Element::new("line")
                .with("id", format!("actor-man-torso{cnt}"))
                .with("x1", number(centre))
                .with("y1", number(gy(25.0)))
                .with("x2", number(centre))
                .with("y2", number(gy(45.0))),
        );
        act.push_child(
            Element::new("line")
                .with("id", format!("actor-man-arms{cnt}"))
                .with("x1", number(gx(-half)))
                .with("y1", number(gy(33.0)))
                .with("x2", number(gx(half)))
                .with("y2", number(gy(33.0))),
        );
        act.push_child(
            Element::new("line")
                .with("x1", number(gx(-half)))
                .with("y1", number(gy(60.0)))
                .with("x2", number(centre))
                .with("y2", number(gy(45.0))),
        );
        act.push_child(
            Element::new("line")
                .with("x1", number(centre))
                .with("y1", number(gy(45.0)))
                .with("x2", number(gx(half - 2.0)))
                .with("y2", number(gy(60.0))),
        );
        act.push_child(
            Element::new("circle")
                .with("cx", number(centre))
                .with("cy", number(gy(10.0)))
                .with("r", number(15.0 * scale))
                .with("width", number(actor.width))
                .with("height", number(actor.height)),
        );
        if band.is_none() {
            // Classic reports the glyph's fixed extent (`:1324-1327`).
            self.layout.actors[index].height = ACTOR_GLYPH_HEIGHT;
        }
        let actor = self.layout.actors[index].clone();
        if self.uses_palettes() {
            if let Some(style) = style_text(&self.shape_style(index, true)) {
                act.set("style", style);
            }
        } else {
            act.set(
                "style",
                format!(
                    "stroke: {};",
                    self.theme.light.actor_border.text().unwrap_or_default()
                ),
            );
        }
        let label_y = match band {
            Some(band) => band.label_centre_y - actor.height / 2.0,
            None => actor_y + 35.0,
        };
        let label = self.label(
            &actor,
            actor.x,
            label_y,
            actor.width,
            actor.height,
            "actor actor-man",
        );
        for child in label {
            act.push_child(child);
        }
        self.append_shape(act);
        self.layout.actors[index].height
    }
}

impl Renderer<'_> {
    /// `drawActorsPopup` (`sequenceRenderer.ts:1505-1520`): the menus of the actors, as the
    /// reference lays them out. Returns the height and the width the menus need. The menus are
    /// drawn hidden (`display: none`), as the reference does; the click that shows them is a
    /// JavaScript handler, which is not drawn (decision 0001).
    pub(super) fn draw_actors_popups(&mut self) -> Result<(f64, f64), RenderError> {
        let mut max_height = 0.0_f64;
        let mut max_width = 0.0_f64;
        let keys = self.layout.actor_keys.clone();
        for key in &keys {
            let Some(index) = self.layout.actor_index(key) else {
                continue;
            };
            let min_width = self.required_popup_width(index)?;
            let (height, width) = self.draw_popup(index, min_width)?;
            max_height = get_max(&[Some(max_height), Some(height)]);
            let actor_x = self.layout.actors[index].x;
            max_width = get_max(&[Some(max_width), Some(width + actor_x)]);
        }
        Ok((max_height, max_width))
    }

    /// `getRequiredPopupWidth` (`sequenceRenderer.ts:1592-1602`): the width the widest link label
    /// needs, with the padding of the menu.
    fn required_popup_width(&mut self, index: usize) -> Result<f64, RenderError> {
        let actor = self.layout.actors[index].clone();
        let font = self.conf.actor_font.clone();
        let padding = self.conf.wrap_padding;
        let margin = self.conf.box_margin;
        let mut required = 0.0_f64;
        for key in actor.links.keys() {
            let dims = super::text::dimensions(&mut self.memo, key, &font)?;
            let label_width = dims.width + 2.0 * padding + 2.0 * margin;
            if required < label_width {
                required = label_width;
            }
        }
        Ok(required)
    }

    /// `drawPopup` (`svgDraw.js:79-116`): the hidden menu of an actor with links. Returns the height
    /// and the width of the menu; an actor without links draws nothing and returns zeros.
    fn draw_popup(&mut self, index: usize, min_width: f64) -> Result<(f64, f64), RenderError> {
        let actor = self.layout.actors[index].clone();
        if actor.links.is_empty() {
            return Ok((0.0, 0.0));
        }
        // The reference reads the actor's rectangle, which only the box-shaped participants set.
        let rect = actor.rect_data.clone().ok_or_else(|| {
            RenderError::Layout(format!(
                "the menu of {} needs a box, and its type draws none",
                actor.name
            ))
        })?;
        let display = if self.conf.force_menus {
            "block !important"
        } else {
            "none"
        };
        let cnt = actor
            .actor_cnt
            .map_or_else(|| "undefined".to_owned(), |cnt| cnt.to_string());
        let class_suffix = rect
            .class
            .as_ref()
            .map(|class| format!(" {class}"))
            .unwrap_or_default();
        let menu_width = if rect.width > min_width {
            rect.width
        } else {
            min_width
        };
        let mut panel = Element::new("rect")
            .with("class", format!("actorPopupMenuPanel{class_suffix}"))
            .with("x", number(rect.x))
            .with("y", number(rect.height));
        if let Some(fill) = &rect.fill {
            panel.set("fill", fill.as_str());
        }
        if let Some(stroke) = &rect.stroke {
            panel.set("stroke", stroke.as_str());
        }
        panel.set("width", number(menu_width));
        panel.set("height", number(rect.height));
        panel.set("rx", number(rect.rx.unwrap_or(0.0)));
        panel.set("ry", number(rect.ry.unwrap_or(0.0)));
        let mut group = Element::new("g")
            .with("id", format!("actor{cnt}_popup"))
            .with("class", "actorPopupMenu")
            .with("display", display);
        let mut link_y = 20.0_f64;
        let font = self.conf.actor_font.clone();
        let mut panel_children = Vec::new();
        for (key, value) in &actor.links {
            let target = super::model::value_text(value);
            let mut link = Element::new("a")
                .with("xlink:href", ferrigura_syntax::url::sanitize_url(&target))
                .with("target", "_blank");
            for element in draw_menu_item_text(
                key,
                rect.x + 10.0,
                rect.height + link_y,
                menu_width,
                20.0,
                &font,
            ) {
                link.push_child(element);
            }
            panel_children.push(link);
            link_y += 30.0;
        }
        panel.set("height", number(link_y));
        group.push_child(panel);
        for child in panel_children {
            group.push_child(child);
        }
        self.append(group);
        Ok((rect.height + link_y, menu_width))
    }
}

/// The text of a menu item (`_drawMenuItemTextCandidateFunc`, `byTspan`): one line per `<br>`
/// piece, starting at `x`, centred on the item's `height` as the reference centres it.
fn draw_menu_item_text(
    content: &str,
    x: f64,
    y: f64,
    _width: f64,
    height: f64,
    font: &super::conf::Font,
) -> Vec<Element> {
    let lines = crate::common::text::split_line_breaks(content);
    let size = font.size;
    let count = lines.len() as f64;
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let dy = index as f64 * size - (size * (count - 1.0)) / 2.0;
            let mut text = Element::new("text")
                .with("x", number(x))
                .with("y", number(y))
                .with(
                    "style",
                    format!(
                        "text-anchor: start; font-size: {}; font-weight: {}; font-family: {};",
                        font.size_css(),
                        number(font.weight),
                        font.family
                    ),
                )
                .child(
                    Element::new("tspan")
                        .with("x", number(x))
                        .with("dy", number(dy))
                        .text(*line),
                );
            text.set("y", number(y + height / 2.0));
            text.set("dominant-baseline", "central");
            text.set("alignment-baseline", "central");
            text.set("class", "actor");
            text
        })
        .collect()
}
