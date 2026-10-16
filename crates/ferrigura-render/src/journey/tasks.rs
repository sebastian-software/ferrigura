//! The sections and the tasks of a journey (`journeyRenderer.ts` 256-351; `svgDraw.js` 161-262).
//!
//! A section is a band over the tasks it holds, with its name. A task is a box with its score's
//! face, the circles of its actors, and its name, and a dashed line down from the box to the
//! bottom of the diagram. The bounds of everything drawn give the size of the diagram.

use ferrigura_model::config::JourneyDiagramConfig;
use ferrigura_model::journey::JourneyTask;

use super::face;
use super::legend::{Actors, actor_circle};
use super::text::{Cell, Label, Placement, label};
use crate::RenderError;
use crate::common::Element;
use crate::common::draw::{RectData, draw_rect, get_note_rect};
use crate::svg::number;

/// The number of colour types a diagram cycles through (`section-type-N`, `task-type-N`). The
/// reference takes it from the length of `sectionFills` when its module loads, which is the schema
/// default, whatever the configuration says (`journeyRenderer.ts` 281; `docs-port/journey/NOTES.md`
/// 7.4).
pub const SECTION_TYPES: usize = 7;

/// The deepest point of a task's line: `300 + 5 * 30`, the length of the descenders
/// (`journeyRenderer.ts` 349; `svgDraw.js` 204).
const MAX_HEIGHT: f64 = 300.0 + 5.0 * 30.0;

/// The top of a section band.
const SECTION_Y: f64 = 50.0;

/// The bounds of the drawing, as `bounds` of `journeyRenderer.ts` keeps them: the smallest start and
/// the largest stop of everything inserted. The reference's `updateBounds` runs over an empty list,
/// so the insertions are the only input.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Bounds {
    startx: Option<f64>,
    starty: Option<f64>,
    stopx: Option<f64>,
    stopy: Option<f64>,
}

impl Bounds {
    /// `insert(startx, starty, stopx, stopy)`: the corners in either order.
    pub fn insert(&mut self, startx: f64, starty: f64, stopx: f64, stopy: f64) {
        let (x0, x1) = (startx.min(stopx), startx.max(stopx));
        let (y0, y1) = (starty.min(stopy), starty.max(stopy));
        self.startx = Some(self.startx.map_or(x0, |old| old.min(x0)));
        self.starty = Some(self.starty.map_or(y0, |old| old.min(y0)));
        self.stopx = Some(self.stopx.map_or(x1, |old| old.max(x1)));
        self.stopy = Some(self.stopy.map_or(y1, |old| old.max(y1)));
    }

    /// The corners `(startx, starty, stopx, stopy)`; zero for a side nothing was inserted on.
    #[must_use]
    pub fn corners(&self) -> (f64, f64, f64, f64) {
        (
            self.startx.unwrap_or(0.0),
            self.starty.unwrap_or(0.0),
            self.stopx.unwrap_or(0.0),
            self.stopy.unwrap_or(0.0),
        )
    }
}

/// What the sections and tasks are drawn with.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// The journey's configuration.
    pub config: &'a JourneyDiagramConfig,
    /// The actors, with their colours and positions.
    pub actors: &'a Actors,
    /// The id of the SVG, which prefixes the id of each task line.
    pub id: &'a str,
    /// The fill of each colour type, as the theme's `fillType0` to `fillType7`.
    pub fills: &'a [String],
    /// The theme's text colour: the colour of the label text.
    pub text_colour: &'a str,
    /// The `textPlacement` of the configuration.
    pub placement: Placement,
    /// The font size of the root, in pixels.
    pub font_size: f64,
    /// The `taskFontSize` of the configuration, in pixels.
    pub task_font_size: f64,
    /// The left margin: the legend's width plus `leftMargin`.
    pub left_margin: f64,
}

/// Draws the sections and the tasks in order, and inserts each task into the bounds. The sections
/// and tasks are the elements of the root, in the order the reference appends them.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub fn draw(
    tasks: &[JourneyTask],
    ctx: &Context<'_>,
    bounds: &mut Bounds,
    texts: &mut Vec<String>,
) -> Result<Vec<Element>, RenderError> {
    let config = ctx.config;
    let width = config.width as f64;
    let margin_x = config.diagram_margin_x as f64;
    let task_y = config.height as f64 * 2.0 + config.diagram_margin_y as f64;
    let task_margin = config.task_margin;
    let mut out = Vec::new();
    // The section's colour type, its fill and its text colour, kept from one section to the next
    // as the reference keeps them. Before the first section, the reference's defaults apply.
    let mut last_section = "";
    let mut section_number = 0;
    let mut number_type = 0;
    let mut fill: &str = ctx.fills.first().map_or("", String::as_str);
    let mut colour = "black";
    // `taskCount` of `svgDraw.js`: the index of the task in the ids of the task lines.
    let mut task_count: i64 = -1;
    for (index, task) in tasks.iter().enumerate() {
        let x = index as f64 * task_margin + index as f64 * width + ctx.left_margin;
        if last_section != task.section.as_str() {
            number_type = section_number % SECTION_TYPES;
            fill = ctx.fills.get(number_type).map_or("", String::as_str);
            colour = "#fff";
            // The tasks of the section that follow this one, as the reference counts them.
            let count = tasks[index..]
                .iter()
                .take_while(|next| next.section == task.section)
                .count();
            out.push(section(
                &task.section,
                x,
                count,
                number_type,
                fill,
                ctx,
                texts,
            )?);
            last_section = task.section.as_str();
            section_number += 1;
        }
        task_count += 1;
        out.push(task_group(
            task,
            x,
            task_count,
            ctx,
            Appearance {
                fill,
                colour,
                number_type,
                task_y,
            },
            texts,
        )?);
        bounds.insert(x, task_y, x + margin_x + task_margin, MAX_HEIGHT);
    }
    Ok(out)
}

/// The colours and the position of a task's box.
#[derive(Debug, Clone, Copy)]
struct Appearance<'a> {
    fill: &'a str,
    colour: &'a str,
    number_type: usize,
    task_y: f64,
}

/// A section: the band over its tasks, with its name in the band (`svgDraw.js` 161-189).
fn section(
    name: &str,
    x: f64,
    count: usize,
    number_type: usize,
    fill: &str,
    ctx: &Context<'_>,
    texts: &mut Vec<String>,
) -> Result<Element, RenderError> {
    let config = ctx.config;
    let width =
        config.width as f64 * count as f64 + config.diagram_margin_x as f64 * (count as f64 - 1.0);
    let height = config.height as f64;
    let class = format!("journey-section section-type-{number_type}");
    let rect = RectData {
        x,
        y: SECTION_Y,
        fill: Some(fill.to_owned()),
        width,
        height,
        rx: Some(3.0),
        ry: Some(3.0),
        class: Some(class.clone()),
        ..get_note_rect()
    };
    let mut group = Element::new("g").child(draw_rect(&rect));
    texts.push(name.to_owned());
    let label_elements = label(
        ctx.placement,
        &Label {
            text: name,
            cell: Cell {
                x,
                y: SECTION_Y,
                width,
                height,
            },
            class: &class,
            colour: "#fff",
            text_colour: ctx.text_colour,
            font_size: ctx.font_size,
            task_font_size: ctx.task_font_size,
        },
    )?;
    for element in label_elements {
        group = group.child(element);
    }
    Ok(group)
}

/// A task: its line down to the bottom, its face, its box, the circles of its actors and its
/// name (`svgDraw.js` 200-262). The box is in the colour of the task's section.
fn task_group(
    task: &JourneyTask,
    x: f64,
    task_count: i64,
    ctx: &Context<'_>,
    appearance: Appearance<'_>,
    texts: &mut Vec<String>,
) -> Result<Element, RenderError> {
    let config = ctx.config;
    let width = config.width as f64;
    let height = config.height as f64;
    let task_y = appearance.task_y;
    let centre = x + width / 2.0;
    let mut group = Element::new("g").child(
        Element::new("line")
            .with("id", format!("{}-task{task_count}", ctx.id))
            .with("x1", number(centre))
            .with("y1", number(task_y))
            .with("x2", number(centre))
            .with("y2", number(MAX_HEIGHT))
            .with("class", "task-line")
            .with("stroke-width", "1px")
            .with("stroke-dasharray", "4 2")
            .with("stroke", "#666"),
    );
    for element in face::draw(centre, 300.0 + (5.0 - task.score) * 30.0, task.score) {
        group = group.child(element);
    }
    let rect = RectData {
        x,
        y: task_y,
        fill: Some(appearance.fill.to_owned()),
        width,
        height,
        rx: Some(3.0),
        ry: Some(3.0),
        class: Some(format!("task task-type-{}", appearance.number_type)),
        ..get_note_rect()
    };
    group = group.child(draw_rect(&rect));
    // A circle for each actor of the task, in order; an actor named twice gets two circles.
    let mut x_position = x + 14.0;
    for person in &task.people {
        if let Some(actor) = ctx.actors.get(person) {
            group = group.child(actor_circle(
                x_position,
                task_y,
                7.0,
                &actor.colour,
                actor.position,
                Some(person),
            ));
        }
        x_position += 10.0;
    }
    texts.push(task.task.clone());
    let label_elements = label(
        ctx.placement,
        &Label {
            text: &task.task,
            cell: Cell {
                x,
                y: task_y,
                width,
                height,
            },
            class: "task",
            colour: appearance.colour,
            text_colour: ctx.text_colour,
            font_size: ctx.font_size,
            task_font_size: ctx.task_font_size,
        },
    )?;
    for element in label_elements {
        group = group.child(element);
    }
    Ok(group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bounds_keep_the_smallest_start_and_the_largest_stop() {
        let mut bounds = Bounds::default();
        assert_eq!(bounds.corners(), (0.0, 0.0, 0.0, 0.0));
        bounds.insert(0.0, 0.0, 455.0, 100.0);
        bounds.insert(150.0, 110.0, 250.0, 450.0);
        assert_eq!(bounds.corners(), (0.0, 0.0, 455.0, 450.0));
    }

    #[test]
    fn the_bounds_take_their_corners_in_either_order() {
        let mut bounds = Bounds::default();
        bounds.insert(300.0, 450.0, 150.0, 110.0);
        assert_eq!(bounds.corners(), (150.0, 110.0, 300.0, 450.0));
    }
}
