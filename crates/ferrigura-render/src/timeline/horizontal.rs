//! The horizontal timeline: `timelineRenderer.ts`, the sections, tasks and events from left to right
//! (`docs-port/timeline/NOTES.md` 5.6). The numbers are the reference's: 50 for the top, 150 for the
//! width of a task and an event, and so on; each of them is named where it is used.

use ferrigura_model::timeline::{TimelineDiagram, TimelinePeriod};

use super::node::{self, NodeSpec};
use super::{Canvas, Context, line_box, title, translate, translated};
use crate::RenderError;
use crate::common::Element;
use crate::svg::number;

/// The width the text of a task or an event is wrapped to (`timelineRenderer.ts` 108, 125).
const NODE_WIDTH: f64 = 150.0;

/// The padding of a node (`timelineRenderer.ts` 109, 126).
const NODE_PADDING: f64 = 20.0;

/// The least height of an event (`timelineRenderer.ts` 127).
const EVENT_MIN_HEIGHT: f64 = 50.0;

/// The text the task pre-pass measures. `timelineRenderer.ts` 103-114 gives `getVirtualNodeHeight` the
/// task object as `descr`, not `task.task`, and `.text(object)` writes its string form. So the largest
/// task height comes from this text, not from the tasks (NOTES 5.12 does not list it; the reference
/// is the only source). The vertical layout has the same pre-pass (`timelineRendererVertical.ts`
/// 100-111).
pub(super) const TASK_PREPASS_TEXT: &str = "[object Object]";

/// The distance of a task from the next, in x (`timelineRenderer.ts` 333).
const TASK_SPACING: f64 = 200.0;

/// Draws the horizontal timeline into `canvas`: sections, tasks, their events and lines, the title
/// and the axis. The viewbox is set by the caller, from the box of the canvas.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read.
pub(super) fn draw(
    cx: &Context<'_>,
    canvas: &mut Canvas,
    diagram: &TimelineDiagram,
) -> Result<(), RenderError> {
    let left = cx.left_margin;
    let tasks: Vec<&TimelinePeriod> = diagram.periods.iter().collect();

    // The pre-pass (lines 82-136): the largest heights of the section headers and of the tasks, and
    // the longest stack of events, measured without drawing.
    let mut max_section_height = 0.0_f64;
    for section in &diagram.sections {
        let height = node::virtual_height(cx, section, NODE_WIDTH, NODE_PADDING)?;
        max_section_height = max_section_height.max(height + 20.0);
    }
    let mut max_task_height = 0.0_f64;
    let mut max_event_line_length = 0.0_f64;
    for task in &tasks {
        let height = node::virtual_height(cx, TASK_PREPASS_TEXT, NODE_WIDTH, NODE_PADDING)?;
        max_task_height = max_task_height.max(height + 20.0);
        let mut line_length = 0.0;
        for event in &task.events {
            line_length += node::virtual_height(cx, event, NODE_WIDTH, NODE_PADDING)?;
        }
        if !task.events.is_empty() {
            line_length += (task.events.len() - 1) as f64 * 10.0;
        }
        max_event_line_length = max_event_line_length.max(line_length);
    }

    let has_sections = !diagram.sections.is_empty();
    if has_sections {
        // Lines 141-185: each section's header, then its tasks under it, one column per task.
        let mut master_x = 50.0 + left;
        let section_begin_y = 50.0;
        for (section_number, section) in diagram.sections.iter().enumerate() {
            let tasks_for: Vec<&TimelinePeriod> = tasks
                .iter()
                .copied()
                .filter(|task| task.section == *section)
                .collect();
            let columns = tasks_for.len().max(1) as f64;
            let header = node::draw(
                cx,
                canvas,
                NodeSpec {
                    descr: section,
                    width: TASK_SPACING * columns - 50.0,
                    padding: NODE_PADDING,
                    max_height: max_section_height,
                },
                section_number,
                false,
                Some(cx.svg_id),
            )?;
            canvas.include(translated(header.bounds, master_x, section_begin_y));
            canvas.body.push(
                Element::new("g")
                    .with("transform", translate(master_x, section_begin_y))
                    .child(header.element),
            );
            let master_y = section_begin_y + max_section_height + 50.0;
            if !tasks_for.is_empty() {
                draw_tasks(
                    cx,
                    canvas,
                    &tasks_for,
                    section_number,
                    master_x,
                    master_y,
                    max_task_height,
                    max_event_line_length,
                    false,
                )?;
            }
            master_x += TASK_SPACING * columns;
        }
    } else {
        // Lines 186-203: no sections, so every task is drawn in one row, in a colour of its own.
        draw_tasks(
            cx,
            canvas,
            &tasks,
            0,
            50.0 + left,
            50.0,
            max_task_height,
            max_event_line_length,
            true,
        )?;
    }

    // Lines 205-231: the title, centred over the box of the drawing so far, and the axis below it.
    let before = canvas.bounds.unwrap_or_else(super::zero_box);
    if !diagram.title.is_empty() {
        let x = if cx.neo {
            before.x * 2.0 + left
        } else {
            before.width / 2.0 - left
        };
        let (element, bounds) = title(cx, x, &diagram.title)?;
        canvas.body.push(element);
        canvas.include(bounds);
    }
    let depth_y = if has_sections {
        max_section_height + max_task_height + 150.0
    } else {
        max_task_height + 100.0
    };
    let axis = Element::new("line")
        .with("x1", number(left))
        .with("y1", number(depth_y))
        .with("x2", number(before.width + 3.0 * left))
        .with("y2", number(depth_y))
        .with("stroke-width", "4")
        .with("stroke", cx.primary.node_border.as_str())
        .with("marker-end", format!("url(#{}-arrowhead)", cx.svg_id));
    canvas.include(line_box(left, depth_y, before.width + 3.0 * left, depth_y));
    canvas
        .body
        .push(Element::new("g").with("class", "lineWrapper").child(axis));

    // Lines 234-257: the gradient of the neo look, in `defs` after the drop shadow.
    if cx.gradient {
        canvas.defs.push(gradient(cx));
    }
    Ok(())
}

/// The tasks of one section (or of the whole timeline), from the given column `master_x`, with the
/// first row at `master_y` (`drawTasks`, lines 270-341).
#[allow(clippy::too_many_arguments)]
fn draw_tasks(
    cx: &Context<'_>,
    canvas: &mut Canvas,
    tasks: &[&TimelinePeriod],
    section_colour: usize,
    master_x: f64,
    master_y: f64,
    max_task_height: f64,
    max_event_line_length: f64,
    is_without_sections: bool,
) -> Result<(), RenderError> {
    let mut colour = section_colour;
    let mut max_height = max_task_height;
    let mut x = master_x;
    for task in tasks {
        let node = node::draw(
            cx,
            canvas,
            NodeSpec {
                descr: &task.period,
                width: NODE_WIDTH,
                padding: NODE_PADDING,
                max_height,
            },
            colour,
            false,
            Some(cx.svg_id),
        )?;
        canvas.include(translated(node.bounds, x, master_y));
        canvas.body.push(
            Element::new("g")
                .with("class", "taskWrapper")
                .with("transform", translate(x, master_y))
                .child(node.element),
        );
        max_height = max_height.max(node.height);

        // The line of the task is written before its events, and drawn once they are placed, since its
        // length depends on the tallest task of the timeline, not on this one. The events start 200
        // below the row (lines 310-318).
        let line_index = canvas.body.len();
        canvas.body.push(Element::new("g"));
        draw_events(cx, canvas, &task.events, colour, x, master_y + 100.0)?;

        // Every task has a dashed line, even without events (`if (task.events)` is always true,
        // NOTES 5.12 R3): it runs from the foot of the task to below its events.
        let line_x = x + 190.0 / 2.0;
        let line_top = master_y + max_height;
        let line_bottom = master_y + max_height + 100.0 + max_event_line_length + 100.0;
        let line = Element::new("line")
            .with("x1", number(line_x))
            .with("y1", number(line_top))
            .with("x2", number(line_x))
            .with("y2", number(line_bottom))
            .with("stroke-width", "2")
            .with("stroke", cx.primary.node_border.as_str())
            .with("marker-end", format!("url(#{}-arrowhead)", cx.svg_id))
            .with("stroke-dasharray", "5,5");
        canvas.include(line_box(line_x, line_top, line_x, line_bottom));
        canvas.body[line_index] = Element::new("g").with("class", "lineWrapper").child(line);

        x += TASK_SPACING;
        if is_without_sections && !cx.disable_multicolor {
            colour += 1;
        }
    }
    Ok(())
}

/// The events of one task, down from `start_y` (`drawEvents`, lines 343-380). Each event is at least
/// 50 high and the next one starts 10 below it.
fn draw_events(
    cx: &Context<'_>,
    canvas: &mut Canvas,
    events: &[String],
    colour: usize,
    x: f64,
    start_y: f64,
) -> Result<(), RenderError> {
    let mut y = start_y + 100.0;
    for event in events {
        let node = node::draw(
            cx,
            canvas,
            NodeSpec {
                descr: event,
                width: NODE_WIDTH,
                padding: NODE_PADDING,
                max_height: EVENT_MIN_HEIGHT,
            },
            colour,
            true,
            Some(cx.svg_id),
        )?;
        canvas.include(translated(node.bounds, x, y));
        canvas.body.push(
            Element::new("g")
                .with("class", "eventWrapper")
                .with("transform", translate(x, y))
                .child(node.element),
        );
        y = y + 10.0 + node.height;
    }
    Ok(())
}

/// The gradient of the neo look, from the start to the stop colour of the theme (`timelineRenderer.ts`
/// 237-256).
fn gradient(cx: &Context<'_>) -> Element {
    Element::new("linearGradient")
        .with("id", format!("{}-gradient", cx.svg_id))
        .with("gradientUnits", "objectBoundingBox")
        .with("x1", "0%")
        .with("y1", "0%")
        .with("x2", "100%")
        .with("y2", "0%")
        .child(
            Element::new("stop")
                .with("offset", "0%")
                .with("stop-color", cx.gradient_start.as_str())
                .with("stop-opacity", "1"),
        )
        .child(
            Element::new("stop")
                .with("offset", "100%")
                .with("stop-color", cx.gradient_stop.as_str())
                .with("stop-opacity", "1"),
        )
}
