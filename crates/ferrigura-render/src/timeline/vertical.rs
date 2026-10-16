//! The vertical timeline, `TD`: `timelineRendererVertical.ts`, the sections and their tasks down the
//! page, and the events of a task on the right of its axis (`docs-port/timeline/NOTES.md` 5.7).
//!
//! The reference's defects of this layout are reproduced, as the quirks of a port are
//! (`docs-port/M6-PLAN.md`, quirks): the nodes are named `undefined-node-N` (`drawNode` gets no
//! diagram id), the marker is `undefined-arrowhead`, and every arrow refers to `arrowhead`, which does
//! not exist, so no arrow is drawn (NOTES 5.12, R1 and R2). The gradient of the horizontal layout is
//! not drawn here, as the reference does not draw it (R12).

use ferrigura_model::timeline::{TimelineDiagram, TimelinePeriod};

use super::horizontal::TASK_PREPASS_TEXT;
use super::node::{self, NodeSpec};
use super::{Canvas, Context, line_box, title, translate, translated};
use crate::RenderError;
use crate::common::Element;
use crate::svg::number;

/// The width of a section header: `max(50, leftWidth + rightWidth - 2 * padding)` (line 80), which is
/// 580 for the reference's widths.
const SECTION_WIDTH: f64 = LEFT_WIDTH + RIGHT_WIDTH - NODE_PADDING * 2.0;

/// The width of a task (`NODE_WIDTH`).
const TASK_WIDTH: f64 = 200.0;

/// The width of an event (`EVENT_WIDTH`).
const EVENT_WIDTH: f64 = 300.0;

/// The padding of a node (`NODE_PADDING`).
const NODE_PADDING: f64 = 5.0;

/// The gap between an event and the next (`EVENT_SPACING`).
const EVENT_SPACING: f64 = 10.0;

/// The gap between a section header and its first task (`SECTION_TASK_GAP`).
const SECTION_TASK_GAP: f64 = 20.0;

/// The gap between a task and the axis (`TASK_AXIS_GAP`).
const TASK_AXIS_GAP: f64 = 20.0;

/// The gap between the tasks of a section (`TASK_VERTICAL_GAP`).
const TASK_VERTICAL_GAP: f64 = 30.0;

/// The distance from a task's axis to its events (`EVENT_AXIS_GAP`).
const EVENT_AXIS_GAP: f64 = 50.0;

/// The width of a task's column with the axis: its node and the gap (`NODE_TOTAL_WIDTH` plus the
/// gap, `timelineRendererVertical.ts` 73).
const LEFT_WIDTH: f64 = (TASK_WIDTH + NODE_PADDING * 2.0) + TASK_AXIS_GAP;

/// The width of the events column with its gap (`rightWidth`, line 74).
const RIGHT_WIDTH: f64 = (EVENT_WIDTH + NODE_PADDING * 2.0) + EVENT_AXIS_GAP;

/// Draws the vertical timeline into `canvas`, and leaves the axis at the front of the root (the
/// reference lowers it, line 237). The viewbox is set by the caller.
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

    // Lines 52-94 and 96-137: the largest section header and task, the tallest stack of events,
    // measured without drawing.
    let master_x = 50.0 + left;
    let content_top_y = 50.0;
    let axis_x = master_x + LEFT_WIDTH;
    let has_sections = !diagram.sections.is_empty();
    let timeline_x = if has_sections {
        axis_x
    } else {
        master_x + LEFT_WIDTH
    };

    let mut max_section_height = 0.0_f64;
    for section in &diagram.sections {
        let height = node::virtual_height(cx, section, SECTION_WIDTH, NODE_PADDING)?;
        max_section_height = max_section_height.max(height);
    }
    let mut max_task_height = 0.0_f64;
    let mut max_event_stack_height = 0.0_f64;
    for task in &tasks {
        // The reference measures the task object, `[object Object]`, not its text (see
        // `horizontal::TASK_PREPASS_TEXT`, and `timelineRendererVertical.ts` 100-111).
        let height = node::virtual_height(cx, TASK_PREPASS_TEXT, TASK_WIDTH, NODE_PADDING)?;
        max_task_height = max_task_height.max(height);
        let mut stack = 0.0;
        for event in &task.events {
            stack += node::virtual_height(cx, event, EVENT_WIDTH, NODE_PADDING)?;
        }
        if !task.events.is_empty() {
            stack += (task.events.len() - 1) as f64 * EVENT_SPACING;
        }
        max_event_stack_height = max_event_stack_height.max(stack);
    }
    let task_block_height = max_task_height.max(max_event_stack_height);
    let task_spacing = task_block_height + TASK_VERTICAL_GAP;

    if has_sections {
        // Lines 139-184: each section's header, and its tasks under it.
        let mut master_y = content_top_y;
        for (section_number, section) in diagram.sections.iter().enumerate() {
            let tasks_for: Vec<&TimelinePeriod> = tasks
                .iter()
                .copied()
                .filter(|task| task.section == *section)
                .collect();
            let header = node::draw(
                cx,
                canvas,
                NodeSpec {
                    descr: section,
                    width: SECTION_WIDTH,
                    padding: NODE_PADDING,
                    max_height: max_section_height,
                },
                section_number,
                false,
                None,
            )?;
            let section_x = timeline_x - LEFT_WIDTH;
            canvas.include(translated(header.bounds, section_x, master_y));
            canvas.body.push(
                Element::new("g")
                    .with("transform", translate(section_x, master_y))
                    .child(header.element),
            );
            let task_start_y = master_y + header.height + SECTION_TASK_GAP;
            if !tasks_for.is_empty() {
                draw_tasks(
                    cx,
                    canvas,
                    &tasks_for,
                    section_number,
                    timeline_x,
                    task_start_y,
                    max_task_height,
                    task_spacing,
                    false,
                )?;
            }
            let count = tasks_for.len();
            let section_height =
                header.height + SECTION_TASK_GAP + task_spacing * count.max(1) as f64
                    - if count > 0 {
                        TASK_VERTICAL_GAP * 2.0
                    } else {
                        0.0
                    };
            master_y += section_height;
        }
    } else {
        draw_tasks(
            cx,
            canvas,
            &tasks,
            0,
            timeline_x,
            content_top_y,
            max_task_height,
            task_spacing,
            true,
        )?;
    }

    // Lines 201-220: the title, centred over the box of the drawing so far, and the box with it.
    let before = canvas.bounds.unwrap_or_else(super::zero_box);
    if !diagram.title.is_empty() {
        let (element, bounds) = title(cx, before.width / 2.0 - left, &diagram.title)?;
        canvas.body.push(element);
        canvas.include(bounds);
    }
    let box_ = canvas.bounds.unwrap_or_else(super::zero_box);

    // Lines 222-236: the axis, from the top of the content to below all of it. The font size is the
    // configured one, so the offsets are 2 and 0.5 em, plus 20.
    let font_size = cx.node_font_size;
    let arrow_top_offset = font_size * 2.0;
    let arrow_bottom_padding = font_size * 0.5 + 20.0;
    let axis_top = content_top_y - arrow_top_offset;
    let axis_bottom = box_.y + box_.height + arrow_bottom_padding;
    // `marker-end="url(#arrowhead)"`: the reference's marker is `undefined-arrowhead`, so the axis has
    // no arrow (NOTES 5.12, R1).
    let axis = Element::new("line")
        .with("x1", number(timeline_x))
        .with("y1", number(axis_top))
        .with("x2", number(timeline_x))
        .with("y2", number(axis_bottom))
        .with("stroke-width", "4")
        .with("stroke", cx.primary.node_border.as_str())
        .with("marker-end", "url(#arrowhead)");
    canvas.include(line_box(timeline_x, axis_top, timeline_x, axis_bottom));
    canvas.lead = Some(Element::new("g").with("class", "lineWrapper").child(axis));
    Ok(())
}

/// The tasks of one section (or of the whole timeline) down from `start_y`, with the events on the
/// right of the axis (`drawTasks`, lines 248-296). The task's column is 230 left of the axis.
#[allow(clippy::too_many_arguments)]
fn draw_tasks(
    cx: &Context<'_>,
    canvas: &mut Canvas,
    tasks: &[&TimelinePeriod],
    section_colour: usize,
    timeline_x: f64,
    start_y: f64,
    max_task_height: f64,
    task_spacing: f64,
    is_without_sections: bool,
) -> Result<(), RenderError> {
    let mut colour = section_colour;
    let mut max_height = max_task_height;
    let mut y = start_y;
    for task in tasks {
        let node = node::draw(
            cx,
            canvas,
            NodeSpec {
                descr: &task.period,
                width: TASK_WIDTH,
                padding: NODE_PADDING,
                max_height,
            },
            colour,
            false,
            None,
        )?;
        // The task's box ends 20 left of the axis (line 278).
        let task_x = timeline_x - TASK_AXIS_GAP - node.width;
        canvas.include(translated(node.bounds, task_x, y));
        canvas.body.push(
            Element::new("g")
                .with("class", "taskWrapper")
                .with("transform", translate(task_x, y))
                .child(node.element),
        );
        max_height = max_height.max(node.height);

        if !task.events.is_empty() {
            draw_events(
                cx,
                canvas,
                &task.events,
                colour,
                timeline_x,
                timeline_x + EVENT_AXIS_GAP,
                y,
            )?;
        }

        y += task_spacing;
        if is_without_sections && !cx.disable_multicolor {
            colour += 1;
        }
    }
    Ok(())
}

/// The events of one task, stacked down from `start_y` on the right of the axis, each with a dashed
/// arrow from the axis to its text (`drawEvents`, lines 298-344).
fn draw_events(
    cx: &Context<'_>,
    canvas: &mut Canvas,
    events: &[String],
    colour: usize,
    axis_x: f64,
    events_x: f64,
    start_y: f64,
) -> Result<(), RenderError> {
    let mut current_y = start_y;
    for event in events {
        let node = node::draw(
            cx,
            canvas,
            NodeSpec {
                descr: event,
                width: EVENT_WIDTH,
                padding: NODE_PADDING,
                max_height: 0.0,
            },
            colour,
            false,
            None,
        )?;
        canvas.include(translated(node.bounds, events_x, current_y));
        canvas.body.push(
            Element::new("g")
                .with("class", "eventWrapper")
                .with("transform", translate(events_x, current_y))
                .child(node.element),
        );

        // The dashed arrow sits at the middle of the event's text.
        let line_y = current_y + node.height / 2.0;
        let line = Element::new("line")
            .with("x1", number(axis_x))
            .with("y1", number(line_y))
            .with("x2", number(events_x))
            .with("y2", number(line_y))
            .with("stroke-width", "2")
            .with("stroke", cx.primary.node_border.as_str())
            .with("marker-end", "url(#arrowhead)")
            .with("stroke-dasharray", "5,5");
        canvas.include(line_box(axis_x, line_y, events_x, line_y));
        canvas
            .body
            .push(Element::new("g").with("class", "lineWrapper").child(line));

        current_y = current_y + node.height + EVENT_SPACING;
    }
    Ok(())
}
