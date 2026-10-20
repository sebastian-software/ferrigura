//! The gantt diagram: a port of `diagrams/gantt/ganttRenderer.js` and `styles.js`. The research, with
//! the reference's line for every rule, is in `docs-port/gantt/NOTES.md`, section 6; the date rules
//! are in the parser (`ferrigura_syntax::gantt`).
//!
//! The chart is laid out in the reference's order: the dimensions, the time scale, the excluded
//! days, the grids, the section rows, the task bars and their labels, the section labels, the today
//! marker and the title. The elements come out in that document order, and every number is written
//! the way the reference's JavaScript writes it.

mod axis;
mod dayjs;
mod draw;
mod format;
mod scale;
mod style;
mod text;
mod time;

use ferrigura_model::config::GanttDiagramConfig;
use ferrigura_model::gantt::{GanttDiagram, GanttTask};
use ferrigura_text::{EmbeddedFamily, FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

use crate::RenderError;
use crate::a11y;
use crate::context::{Context, ThemeView};
use crate::svg::{escape, font_faces, number, xlink_declaration};

use scale::TimeScale;

/// What the gantt renderer reads besides its diagram and the globals: the gantt section of the
/// configuration, and the time of the render. The time is the facade's `RenderOptions::now`, or the
/// system clock: the today marker and the dates without a day read it, and it is not configuration.
#[derive(Debug, Clone, Copy)]
pub struct GanttSection<'a> {
    /// The gantt section of the configuration, in the diagram's scope.
    pub config: &'a GanttDiagramConfig,
    /// The current time in milliseconds since the epoch, for the today marker.
    pub now: i64,
}

/// The width of the chart when the configuration gives none: `ganttRenderer.js:101-103`, the
/// reference's own fallback for a container without a width. The reference takes the parent's
/// `offsetWidth`, which ferrigura has no equivalent of; the harness makes the reference's container
/// this wide too (`tools/port/geometry/common.ts`).
const DEFAULT_WIDTH: f64 = 1200.0;

/// The half pixel that d3-axis adds to crisp lines at a device pixel ratio of 1 (`axis.js:38`).
/// The reference's own offset depends on the device's pixel ratio; the port writes the ratio-1 value,
/// which the geometry harness renders at.
const AXIS_OFFSET: f64 = 0.5;

/// The most ticks a custom tick interval may give (`MAX_TICK_COUNT`, `ganttRenderer.js:31`).
const MAX_TICK_COUNT: f64 = 10_000.0;

/// Renders the diagram to an SVG document. The context carries the gantt section of the configuration
/// and the time of the render as its section, and the theme's tokens.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset, or if the reference would throw
/// while drawing (an excluded day that is not found, a custom tick interval with an unknown
/// weekday).
pub fn render(input: &Context<'_, GanttDiagram, GanttSection<'_>>) -> Result<String, RenderError> {
    let mut chart = Chart::new(input);
    let mut texts = Texts::default();
    let mut body = String::new();
    // `appendDivSvgG` leaves an empty group before the chart's own elements (`mermaidAPI.ts:418`).
    body.push_str("<g></g>");
    chart.exclude_group(&mut body)?;
    chart.grid(&mut body, &mut texts)?;
    chart.draw_rects(&mut body, &mut texts)?;
    chart.section_labels(&mut body, &mut texts)?;
    chart.today(&mut body);
    chart.title(&mut body, &mut texts);

    let runs = texts.runs();
    let families = embed_runs(&runs)?;
    let family = families
        .first()
        .map(|family: &EmbeddedFamily| family.css_family.clone())
        .unwrap_or_default();
    let faces = if families.is_empty() {
        String::new()
    } else {
        font_faces(&families)
    };
    let style = style::style_element(input.id, input.theme, &family, &faces);
    let title = input.diagram.acc_title.as_str();
    let description = input.diagram.acc_descr.as_str();
    let accessibility = a11y::children(input.id, title, description);
    let size = if chart.conf.use_max_width {
        format!(
            " width=\"100%\" style=\"max-width: {}px;\"",
            number(chart.w)
        )
    } else {
        format!(
            " height=\"{}\" width=\"{}\"",
            number(chart.h),
            number(chart.w)
        )
    };
    let link_namespace = xlink_declaration(chart.has_links());
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" id=\"{id}\"{size}{link_namespace} viewBox=\"0 0 {w} {h}\"{roles}>{accessibility}{style}{body}</svg>",
        id = escape(input.id),
        w = number(chart.w),
        h = number(chart.h),
        roles = a11y::root_attributes(input.id, "gantt", title, description),
    ))
}

/// The text the chart draws, with the font each run uses (regular, or italic for milestones), so
/// that the embedded fonts cover every glyph.
#[derive(Default)]
pub(super) struct Texts {
    runs: Vec<(String, FontStyle)>,
}

impl Texts {
    /// Records a run of text in the given style.
    pub(super) fn push(&mut self, text: &str, style: FontStyle) {
        self.runs.push((text.to_owned(), style));
    }

    /// The runs, as the font embedding takes them.
    fn runs(&self) -> Vec<Run<'_>> {
        self.runs
            .iter()
            .map(|(text, style)| Run {
                text,
                face: face(FontFamily::Sans, FontWeight::Normal, *style),
            })
            .collect()
    }
}

/// The derived values of one chart: the dimensions, the time scale, the task order and the sections.
pub(super) struct Chart<'a> {
    /// The id of the SVG element.
    pub(super) id: &'a str,
    /// The theme the chart draws with.
    pub(super) theme: ThemeView<'a>,
    /// The time of the render, for the today marker.
    pub(super) now: i64,
    pub(super) model: &'a GanttDiagram,
    pub(super) conf: &'a GanttDiagramConfig,
    /// `w`: the width of the chart.
    pub(super) w: f64,
    /// `h`: the height of the chart.
    pub(super) h: f64,
    /// `gap`: the height of one row, `barHeight + barGap`.
    pub(super) gap: f64,
    /// The time scale of the chart (`timeScale`).
    pub(super) scale: TimeScale,
    /// The sections, in the order their first task has (`categories`).
    pub(super) categories: Vec<String>,
    /// `categoryHeights`: the rows of each section, in the reference's object key order.
    pub(super) heights: Vec<(String, f64)>,
    /// The start of each task, in epoch milliseconds (NaN when unresolved).
    pub(super) starts: Vec<f64>,
    /// The end of each task.
    pub(super) ends: Vec<f64>,
    /// `renderEndTime || endTime`: the end the bar is drawn to.
    pub(super) render_ends: Vec<f64>,
    /// `order` of each task: the row, which the compact layout rewrites.
    pub(super) orders: Vec<i64>,
    /// Whether each task is a vertical marker.
    pub(super) vert: Vec<bool>,
    /// The tasks in drawing order: sorted by start, with the vertical markers last in `drawRects`.
    pub(super) array: Vec<usize>,
    /// The number of tasks that are not vertical markers (`tasksWithoutVert.length`).
    pub(super) rows: usize,
}

impl<'a> Chart<'a> {
    /// The chart's values, as `draw` computes them before it writes anything.
    fn new(input: &Context<'a, GanttDiagram, GanttSection<'a>>) -> Self {
        let model = input.diagram;
        let conf = input.section.config;
        let tasks: &[GanttTask] = &model.tasks;
        let starts: Vec<f64> = tasks.iter().map(|task| time_of(task.start_time)).collect();
        let ends: Vec<f64> = tasks.iter().map(|task| time_of(task.end_time)).collect();
        let render_ends: Vec<f64> = tasks
            .iter()
            .map(|task| match task.render_end_time {
                Some(render_end) => render_end as f64,
                None => time_of(task.end_time),
            })
            .collect();
        let orders: Vec<i64> = tasks.iter().map(|task| task.order).collect();
        let vert: Vec<bool> = tasks.iter().map(|task| task.vert).collect();

        let without_vert: Vec<usize> = (0..tasks.len()).filter(|&index| !vert[index]).collect();
        let mut categories: Vec<String> = Vec::new();
        for &index in &without_vert {
            let section = &tasks[index].section;
            if !categories.contains(section) {
                categories.push(section.clone());
            }
        }

        let gap = conf.bar_height as f64 + conf.bar_gap as f64;
        let mut h = 2.0 * conf.top_padding as f64;
        let mut orders = orders;
        let mut heights: Vec<(String, f64)> = Vec::new();
        if conf.display_mode == "compact" {
            // Sections in the order of their first task, then the rows of each (`getMaxIntersections`).
            let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
            for &index in &without_vert {
                let section = &tasks[index].section;
                match groups.iter_mut().find(|(name, _)| name == section) {
                    Some((_, members)) => members.push(index),
                    None => groups.push((section.clone(), vec![index])),
                }
            }
            let mut intersections = 0.0;
            let mut ordered = Vec::new();
            for (name, members) in js_key_order(groups) {
                let height =
                    max_intersections(&members, intersections, &starts, &ends, &mut orders) + 1.0;
                intersections += height;
                h += height * gap;
                ordered.push((name, height));
            }
            heights = ordered;
        } else {
            h += without_vert.len() as f64 * gap;
            for category in &categories {
                let count = without_vert
                    .iter()
                    .filter(|&&index| &tasks[index].section == category)
                    .count();
                heights.push((category.clone(), count as f64));
            }
            heights = js_key_order(heights);
        }

        // The domain covers every task, markers included; `min` and `max` skip the missing times.
        let domain = [
            minimum(starts.iter().copied()),
            maximum(ends.iter().copied()),
        ];
        let width = match conf.use_width {
            Some(width) => width,
            None => DEFAULT_WIDTH,
        };
        let range_max = width - conf.left_padding as f64 - conf.right_padding as f64;
        let scale = TimeScale::new(domain, range_max);

        let mut array: Vec<usize> = (0..tasks.len()).collect();
        sort_js(&mut array, |a, b| compare_start(starts[a], starts[b]));

        Self {
            id: input.id,
            theme: input.theme,
            now: input.section.now,
            model,
            conf,
            w: width,
            h,
            gap,
            scale,
            categories,
            heights,
            starts,
            ends,
            render_ends,
            orders,
            vert,
            array,
            rows: without_vert.len(),
        }
    }

    /// Whether any task has a link, so that the root declares the `xlink` namespace.
    fn has_links(&self) -> bool {
        self.model
            .tasks
            .iter()
            .any(|task| self.model.links.contains_key(&task.id))
    }

    /// The index of a section in `categories` (`categories.entries()`), if it is one.
    pub(super) fn section_index(&self, section: &str) -> Option<usize> {
        self.categories
            .iter()
            .position(|category| category == section)
    }

    /// `secNum` of a task: its section's index modulo `numberSectionStyles`, or 0 for a section
    /// that is not a category.
    pub(super) fn section_number(&self, task: usize) -> usize {
        let styles = self.conf.number_section_styles.max(1) as usize;
        self.section_index(&self.model.tasks[task].section)
            .map_or(0, |index| index % styles)
    }

    /// `textWidth` of a task's label: the width of its text in the sans face at the font size.
    pub(super) fn label_width(&self, plain: &str) -> Result<f64, RenderError> {
        text::width(plain, self.conf.font_size as f64)
    }
}

/// The time of an optional epoch-millisecond value: NaN when it is missing (`undefined` in the
/// reference, which then computes NaN).
pub(super) fn time_of(value: Option<i64>) -> f64 {
    value.map_or(f64::NAN, |time| time as f64)
}

/// `Math.min` over the times that are numbers: d3's `min` skips NaN and undefined values; with none
/// left the result is undefined, which the scale reads as NaN.
fn minimum(values: impl Iterator<Item = f64>) -> f64 {
    values
        .filter(|value| !value.is_nan())
        .reduce(f64::min)
        .unwrap_or(f64::NAN)
}

/// The largest of the times that are numbers (see [`minimum`]).
fn maximum(values: impl Iterator<Item = f64>) -> f64 {
    values
        .filter(|value| !value.is_nan())
        .reduce(f64::max)
        .unwrap_or(f64::NAN)
}

/// `taskCompare`: the sign of the start difference, zero when either start is not a number.
fn compare_start(a: f64, b: f64) -> f64 {
    if a > b {
        1.0
    } else if a < b {
        -1.0
    } else {
        0.0
    }
}

/// `Array.prototype.sort` with a comparator that returns a number: a stable sort that keeps the
/// order of equal items. An insertion sort, so an inconsistent comparator (with NaN) cannot break it.
pub(super) fn sort_js<T: Copy>(items: &mut [T], compare: impl Fn(T, T) -> f64) {
    for index in 1..items.len() {
        let mut position = index;
        while position > 0 && compare(items[position - 1], items[position]) > 0.0 {
            items.swap(position - 1, position);
            position -= 1;
        }
    }
}

/// `getMaxIntersections` (`ganttRenderer.js:59-77`): places the tasks of one section in rows, so
/// that no two that overlap share a row, and returns the largest row index. The rows are numbered
/// from `offset`, and each task's `order` is set to its row.
fn max_intersections(
    members: &[usize],
    offset: f64,
    starts: &[f64],
    ends: &[f64],
    orders: &mut [i64],
) -> f64 {
    let mut timeline = vec![f64::NEG_INFINITY; members.len()];
    let mut sorted = members.to_vec();
    sort_js(&mut sorted, |a, b| {
        let by_start = starts[a] - starts[b];
        if by_start.is_nan() || by_start == 0.0 {
            (orders[a] - orders[b]) as f64
        } else {
            by_start
        }
    });
    let mut largest = 0.0_f64;
    for &element in &sorted {
        for (row, slot) in timeline.iter_mut().enumerate() {
            if starts[element] >= *slot {
                *slot = ends[element];
                orders[element] = row as i64 + offset as i64;
                largest = largest.max(row as f64);
                break;
            }
        }
    }
    largest
}

/// The keys of an object in the order `Object.keys` gives them: the array indices first, in
/// ascending order, then the other keys in the order they were added.
pub(super) fn js_key_order<T>(entries: Vec<(String, T)>) -> Vec<(String, T)> {
    let (mut indices, others): (Vec<_>, Vec<_>) = entries
        .into_iter()
        .partition(|(key, _)| array_index(key).is_some());
    indices.sort_by_key(|(key, _)| array_index(key).unwrap_or(u64::MAX));
    indices.extend(others);
    indices
}

/// The value of a key that is an array index (a canonical integer below 2^32 - 1), if it is one.
fn array_index(key: &str) -> Option<u64> {
    let canonical = key == "0" || (!key.starts_with('0') && !key.is_empty());
    if !canonical || !key.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    key.parse::<u64>()
        .ok()
        .filter(|index| *index < 4_294_967_295)
}
