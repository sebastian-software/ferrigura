//! The elements of the chart, each written in the reference's order and with its attributes in the
//! order the reference sets them: the excluded days, the grids, the section rows, the bars and their
//! labels, the section labels, the today marker and the title (`ganttRenderer.js:213-222`).

use ferrigura_model::theme::{ThemeTokens, Token};
use ferrigura_text::FontStyle;

use super::axis::{Axis, Orient};
use super::format::utc_format;
use super::style::{self, Bar};
use super::text::label;
use super::time::{Base, Interval, TickCount};
use super::{Chart, Texts, dayjs};
use crate::RenderError;
use crate::svg::{escape, number};

/// The units of the custom tick interval, as the reference's regular expression names them.
const TICK_UNITS: [&str; 7] = [
    "millisecond",
    "second",
    "minute",
    "hour",
    "day",
    "week",
    "month",
];

/// The custom tick interval of a configuration: the count and the unit, from
/// `^([1-9]\d*)(millisecond|second|minute|hour|day|week|month)$` (`ganttRenderer.js:661`).
fn parse_tick_interval(text: &str) -> Option<(f64, &'static str)> {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || text.starts_with('0') {
        return None;
    }
    let unit = TICK_UNITS
        .iter()
        .copied()
        .find(|unit| *unit == &text[digits..])?;
    let every = text[..digits].parse::<f64>().ok()?;
    Some((every, unit))
}

/// `getEstimatedTickCount` (`ganttRenderer.js:627-637`): the number of ticks a custom interval gives
/// over a span.
fn estimated_ticks(min: f64, max: f64, every: f64, unit: &str) -> f64 {
    if every <= 0.0 || min > max {
        return f64::INFINITY;
    }
    let interval = every * dayjs::unit_milliseconds(unit).unwrap_or(f64::NAN);
    if interval.is_nan() || interval <= 0.0 {
        return f64::INFINITY;
    }
    ((max - min) / interval).ceil()
}

impl Chart<'_> {
    /// The tokens of the theme: the light scheme, which gives the presentation attributes.
    fn tokens(&self) -> &ThemeTokens {
        self.theme.light
    }

    /// The presentation attribute of a colour token.
    fn colour(&self, name: &str, token: &Token) -> String {
        style::attribute(name, token)
    }

    /// The `excludes` rectangles (`drawExcludeDays`, `ganttRenderer.js:543-616`): one per run of
    /// excluded days, drawn in a group that is written whenever `excludes` or `includes` is given.
    pub(super) fn exclude_group(&self, out: &mut String) -> Result<(), RenderError> {
        let model = self.model;
        if model.excludes.is_empty() && model.includes.is_empty() {
            return Ok(());
        }
        // `minTime` and `maxTime`: the first task sets each, the others move them.
        let mut min_time: Option<f64> = None;
        let mut max_time: Option<f64> = None;
        for index in 0..self.starts.len() {
            let (start, end) = (self.starts[index], self.ends[index]);
            if min_time.is_none_or(|min| start < min) {
                min_time = Some(start);
            }
            if max_time.is_none_or(|max| end > max) {
                max_time = Some(end);
            }
        }
        let (Some(min_time), Some(max_time)) = (min_time, max_time) else {
            return Ok(());
        };
        if dayjs::years_between(max_time, min_time) > 5.0 {
            // The reference warns and draws nothing for a span of more than five years.
            return Ok(());
        }

        let mut ranges: Vec<(f64, f64)> = Vec::new();
        let mut open: Option<(f64, f64)> = None;
        let mut day = min_time;
        while day <= max_time {
            let invalid = ferrigura_syntax::gantt::is_invalid_date(model, day)
                .map_err(RenderError::Layout)?;
            if invalid {
                open = Some(match open {
                    None => (day, day),
                    Some((start, _)) => (start, day),
                });
            } else if let Some(range) = open.take() {
                ranges.push(range);
            }
            day = dayjs::add_days(day, 1.0);
        }
        // A run still open at the end of the span is not pushed (`ganttRenderer.js:580-590`): the
        // reference drops an excluded run that reaches the last day, and so does the port.

        let scale = &self.scale;
        let sp = self.conf.left_padding as f64;
        let gl = self.conf.grid_line_start_padding as f64;
        let tp = self.conf.top_padding as f64;
        let fill = self.colour("fill", &self.tokens().exclude_bkg_color);
        let mut out_group = String::from("<g>");
        for (index, (start, end)) in ranges.iter().copied().enumerate() {
            let x = scale.apply(dayjs::start_of_day(start)) + sp;
            let width =
                scale.apply(dayjs::end_of_day(end)) - scale.apply(dayjs::start_of_day(start));
            let origin = scale.apply(start) + sp + 0.5 * (scale.apply(end) - scale.apply(start));
            let row = index as f64 * self.gap + 0.5 * self.h;
            out_group.push_str(&format!(
                "<rect id=\"{}-exclude-{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" transform-origin=\"{}px {}px\" class=\"exclude-range\"{fill}></rect>",
                escape(self.id),
                escape(&dayjs::format_date(start)),
                number(x),
                number(gl),
                number(width),
                number(self.h - tp - gl),
                number(origin),
                number(row),
            ));
        }
        out_group.push_str("</g>");
        out.push_str(&out_group);
        Ok(())
    }

    /// The values of an axis: the scale's ticks, or the custom interval's instants when the
    /// configuration gives one that the reference would draw (`makeGrid`, `ganttRenderer.js:657-713`).
    fn tick_values(&self, custom: Option<(f64, &str)>) -> Result<Vec<f64>, RenderError> {
        let Some((every, unit)) = custom else {
            return Ok(self.scale.ticks(TickCount::Count(10.0)));
        };
        let [min, max] = self.scale.domain();
        if estimated_ticks(min, max, every, unit) > super::MAX_TICK_COUNT {
            return Ok(self.scale.ticks(TickCount::Count(10.0)));
        }
        let base = match unit {
            "millisecond" => Base::Millisecond,
            "second" => Base::Second,
            "minute" => Base::Minute,
            "hour" => Base::Hour,
            "day" => Base::Day,
            "month" => Base::Month,
            _ => Base::Week(self.weekday_number()?),
        };
        Ok(match Interval::every(base, every) {
            Some(interval) => self.scale.ticks(TickCount::Interval(interval)),
            None => self.scale.ticks(TickCount::Count(10.0)),
        })
    }

    /// The day number of the `weekday` that weeks start on (`mapWeekdayToTimeFunction`). A name the
    /// reference does not map throws when it reads the function, so the render fails.
    fn weekday_number(&self) -> Result<u8, RenderError> {
        let name = if self.model.weekday.is_empty() {
            self.conf.weekday.as_str()
        } else {
            self.model.weekday.as_str()
        };
        match name {
            "sunday" => Ok(0),
            "monday" => Ok(1),
            "tuesday" => Ok(2),
            "wednesday" => Ok(3),
            "thursday" => Ok(4),
            "friday" => Ok(5),
            "saturday" => Ok(6),
            other => Err(RenderError::Layout(format!(
                "weekday {other} has no tick function"
            ))),
        }
    }

    /// The custom tick interval the diagram or the configuration gives (`db.getTickInterval() ||
    /// conf.tickInterval`), if it matches the reference's pattern.
    fn custom_tick(&self) -> Option<(f64, &'static str)> {
        let given = self
            .model
            .tick_interval
            .as_deref()
            .filter(|text| !text.is_empty())
            .or_else(|| {
                self.conf
                    .tick_interval
                    .as_deref()
                    .filter(|text| !text.is_empty())
            })?;
        parse_tick_interval(given)
    }

    /// The grid lines and their labels: the bottom axis, and the top one when the diagram or the
    /// configuration asks for it (`makeGrid`).
    pub(super) fn grid(&self, out: &mut String, texts: &mut Texts) -> Result<(), RenderError> {
        let model = self.model;
        let conf = self.conf;
        let axis_format = if !model.axis_format.is_empty() {
            model.axis_format.as_str()
        } else if model.date_format == "D" {
            "%d"
        } else {
            conf.axis_format.as_str()
        };
        let sp = conf.left_padding as f64;
        let tp = conf.top_padding as f64;
        let gl = conf.grid_line_start_padding as f64;
        let tick_size = -self.h + tp + gl;
        let text_fill = self.tokens().text_color.text().unwrap_or_default();
        let custom = self.custom_tick();

        let bottom_values = self.tick_values(custom)?;
        let bottom = Axis {
            orient: Orient::Bottom,
            scale: &self.scale,
            tick_size,
            offset: super::AXIS_OFFSET,
            values: bottom_values,
            format: axis_format,
            text_fill: &text_fill,
        };
        out.push_str(&bottom.group(&format!(
            "translate({}, {})",
            number(sp),
            number(self.h - 50.0)
        )));
        for &value in &bottom.values {
            texts.push(&utc_format(axis_format, value), FontStyle::Normal);
        }

        if model.top_axis || conf.top_axis {
            let top_values = self.tick_values(custom)?;
            let top = Axis {
                orient: Orient::Top,
                scale: &self.scale,
                tick_size,
                offset: super::AXIS_OFFSET,
                values: top_values,
                format: axis_format,
                text_fill: &text_fill,
            };
            out.push_str(&top.group(&format!("translate({}, {})", number(sp), number(tp))));
            for &value in &top.values {
                texts.push(&utc_format(axis_format, value), FontStyle::Normal);
            }
        }
        Ok(())
    }

    /// The section rows and the task bars with their labels (`drawRects`, `ganttRenderer.js:238-388`
    /// and `391-506`). Sorts the tasks so that the vertical markers come last, as the reference does
    /// in place.
    pub(super) fn draw_rects(
        &mut self,
        out: &mut String,
        texts: &mut Texts,
    ) -> Result<(), RenderError> {
        let vert = self.vert.clone();
        self.array.sort_by_key(|&index| vert[index]);
        let without: Vec<usize> = self
            .array
            .iter()
            .copied()
            .filter(|&index| !self.vert[index])
            .collect();

        // One background row per order, taken from the first task of that order.
        let mut seen = std::collections::HashSet::new();
        let mut background = String::from("<g>");
        let gap = self.gap;
        let tp = self.conf.top_padding as f64;
        let rows_width = self.w - self.conf.right_padding as f64 / 2.0;
        for &index in &without {
            if !seen.insert(self.orders[index]) {
                continue;
            }
            background.push_str(&format!(
                "<rect x=\"0\" y=\"{}\" width=\"{}\" height=\"{}\" class=\"section section{}\"{}></rect>",
                number(self.orders[index] as f64 * gap + tp - 2.0),
                number(rows_width),
                number(gap),
                self.section_number(index),
                self.section_fill_attribute(index),
            ));
        }
        background.push_str("</g>");
        out.push_str(&background);

        // The bars and the labels: all rectangles first, then all texts, in the same group.
        let mut parts: Vec<(usize, String, String)> = Vec::with_capacity(self.array.len());
        for &index in &self.array {
            let rect = self.bar(index);
            let text = self.label_element(index, texts)?;
            parts.push((index, rect, text));
        }
        let mut group = String::from("<g>");
        for (index, rect, _) in &parts {
            if !self.is_linked(*index) {
                group.push_str(rect);
            }
        }
        for (index, _, text) in &parts {
            if !self.is_linked(*index) {
                group.push_str(text);
            }
        }
        // A task with a link is wrapped in an anchor at the end of the group, as the sandbox
        // branch wraps it (`ganttRenderer.js:508-531`).
        for (index, rect, text) in &parts {
            if let Some(url) = self.model.links.get(&self.model.tasks[*index].id) {
                group.push_str(&format!(
                    "<a xlink:href=\"{}\" target=\"_top\">{rect}{text}</a>",
                    escape(url)
                ));
            }
        }
        group.push_str("</g>");
        out.push_str(&group);
        Ok(())
    }

    /// Whether a task has a link, which wraps it in an anchor.
    fn is_linked(&self, index: usize) -> bool {
        self.model.links.contains_key(&self.model.tasks[index].id)
    }

    /// The presentation fill of a section row (`.section0`, `.section1`, ...), if the rule names it.
    fn section_fill_attribute(&self, index: usize) -> String {
        let section = self.section_number(index);
        style::section_token(self.tokens(), section)
            .filter(|_| {
                self.section_index(&self.model.tasks[index].section)
                    .is_some()
            })
            .map(|token| self.colour("fill", token))
            .unwrap_or_default()
    }

    /// One task's bar (`rect`), with its attributes in the reference's order.
    fn bar(&self, index: usize) -> String {
        let task = &self.model.tasks[index];
        let conf = self.conf;
        let scale = &self.scale;
        let sp = conf.left_padding as f64;
        let bh = conf.bar_height as f64;
        let tp = conf.top_padding as f64;
        let gl = conf.grid_line_start_padding as f64;
        let order = self.orders[index] as f64;
        let (start, end) = (
            scale.apply(self.starts[index]),
            scale.apply(self.ends[index]),
        );
        let render_end = scale.apply(self.render_ends[index]);
        let x = if task.milestone {
            start + sp + 0.5 * (end - start) - 0.5 * bh
        } else {
            start + sp
        };
        let y = if task.vert { gl } else { order * self.gap + tp };
        let width = if task.milestone {
            bh
        } else if task.vert {
            0.08 * bh
        } else {
            render_end - start
        };
        let height = if task.vert {
            self.rows as f64 * self.gap + bh * 2.0
        } else {
            bh
        };
        let origin = start + sp + 0.5 * (end - start);
        let section = self.section_number(index);
        let state = bar_state(task.active, task.done, task.crit);
        let (fill, stroke) = style::bar_tokens(self.tokens(), state, section, task.vert);
        let mut attributes = String::new();
        if let Some(token) = fill {
            attributes.push_str(&self.colour("fill", token));
        }
        if let Some(token) = stroke {
            attributes.push_str(&self.colour("stroke", token));
        }
        let class = bar_class(task, section, state_classes(task));
        format!(
            "<rect id=\"{}-{}\" rx=\"3\" ry=\"3\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" transform-origin=\"{}px {}px\" class=\"{}\"{}></rect>",
            escape(self.id),
            escape(&task.id),
            number(x),
            number(y),
            number(width),
            number(height),
            number(origin),
            number(order * self.gap + tp + 0.5 * bh),
            escape(&class),
            attributes,
        )
    }

    /// One task's label (`text`), measured as the reference measures it: the width is the text's
    /// width in the sans face at the font size, before the element has its class.
    fn label_element(&self, index: usize, texts: &mut Texts) -> Result<String, RenderError> {
        let task = &self.model.tasks[index];
        let conf = self.conf;
        let scale = &self.scale;
        let sp = conf.left_padding as f64;
        let bh = conf.bar_height as f64;
        let tp = conf.top_padding as f64;
        let gl = conf.grid_line_start_padding as f64;
        let lp = conf.left_padding as f64;
        let font = conf.font_size as f64;
        let w = self.w;
        let text = label(&task.task);
        let text_width = self.label_width(&text.plain)?;
        let (start, end) = (
            scale.apply(self.starts[index]),
            scale.apply(self.ends[index]),
        );
        let render_end = scale.apply(self.render_ends[index]);
        let order = self.orders[index] as f64;

        // The label's x uses the bar's end to the render end, and the start shifted for a milestone.
        let mut start_x = start;
        let mut end_x = render_end;
        if task.milestone {
            start_x += 0.5 * (end - start) - 0.5 * bh;
            end_x = start_x + bh;
        }
        let x = if task.vert {
            start + sp
        } else if text_width > end_x - start_x {
            if end_x + text_width + 1.5 * lp > w {
                start_x + sp - 5.0
            } else {
                end_x + sp + 5.0
            }
        } else {
            (end_x - start_x) / 2.0 + start_x + sp
        };
        let y = if task.vert {
            gl + self.rows as f64 * self.gap + 60.0
        } else {
            order * self.gap + bh / 2.0 + (font / 2.0 - 2.0) + tp
        };

        let class = self.label_class(index, text_width);
        let class_tokens: Vec<&str> = class.split_whitespace().collect();
        let fill = style::text_token(self.tokens(), &class_tokens)
            .map(|token| self.colour("fill", token))
            .unwrap_or_default();
        let style = if task.milestone {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };
        texts.push(&text.plain, style);
        Ok(format!(
            "<text id=\"{}-{}-text\" font-size=\"{}\" x=\"{}\" y=\"{}\" class=\"{}\"{}>{}</text>",
            escape(self.id),
            escape(&task.id),
            number(font),
            number(x),
            number(y),
            escape(&class),
            fill,
            text.xml,
        ))
    }

    /// The class of a task's label: where the text sits relative to its bar, and which state it
    /// has. The reference's values are kept, including the leading and doubled blanks that its
    /// sanitiser trims at the ends only.
    fn label_class(&self, index: usize, text_width: f64) -> String {
        let task = &self.model.tasks[index];
        let scale = &self.scale;
        let bh = self.conf.bar_height as f64;
        let start = scale.apply(self.starts[index]);
        let mut end = scale.apply(self.ends[index]);
        if task.milestone {
            end = start + bh;
        }
        let section = self.section_number(index);
        let mut task_type = String::new();
        if task.active {
            task_type = if task.crit {
                format!("activeCritText{section}")
            } else {
                format!("activeText{section}")
            };
        }
        if task.done {
            let kind = if task.crit {
                "doneCritText"
            } else {
                "doneText"
            };
            task_type = format!("{task_type} {kind}{section}");
        } else if task.crit {
            task_type = format!("{task_type} critText{section}");
        }
        if task.milestone {
            task_type.push_str(" milestoneText");
        }
        if task.vert {
            task_type.push_str(" vertText");
        }
        let classes = task.classes.join(" ");
        let width = number(text_width);
        let class = if text_width > end - start {
            if end + text_width + 1.5 * self.conf.left_padding as f64 > self.w {
                format!("{classes} taskTextOutsideLeft taskTextOutside{section} {task_type}")
            } else {
                format!(
                    "{classes} taskTextOutsideRight taskTextOutside{section} {task_type} width-{width}"
                )
            }
        } else {
            format!("{classes} taskText taskText{section} {task_type} width-{width}")
        };
        class.trim().to_owned()
    }

    /// The section labels (`vertLabels`, `ganttRenderer.js:799-848`), one per section, centred on
    /// the section's rows.
    pub(super) fn section_labels(
        &self,
        out: &mut String,
        texts: &mut Texts,
    ) -> Result<(), RenderError> {
        let gap = self.gap;
        let tp = self.conf.top_padding as f64;
        let styles = self.conf.number_section_styles.max(1) as usize;
        let font = self.conf.section_font_size as f64;
        let mut group = String::from("<g>");
        let mut before = 0.0;
        for (name, count) in &self.heights {
            let rows = super::text::line_breaks(name);
            let dy = -((rows.len() - 1) as f64) / 2.0;
            let y = count * gap / 2.0 + before * gap + tp;
            let class = match self.section_index(name) {
                Some(category) => format!("sectionTitle sectionTitle{}", category % styles),
                None => "sectionTitle".to_owned(),
            };
            // `.sectionTitle0` to `.sectionTitle3` colour the title; a later section has no rule.
            let fill = match self.section_index(name) {
                Some(category) if category % styles < 4 => {
                    self.colour("fill", style::title_token(self.tokens()))
                }
                _ => String::new(),
            };
            let mut spans = String::new();
            for (row_index, row) in rows.iter().enumerate() {
                let row_label = label(row);
                let dy_attribute = if row_index > 0 { " dy=\"1em\"" } else { "" };
                spans.push_str(&format!(
                    "<tspan alignment-baseline=\"central\" x=\"10\"{dy_attribute}>{}</tspan>",
                    row_label.xml
                ));
                texts.push(&row_label.plain, FontStyle::Normal);
            }
            group.push_str(&format!(
                "<text dy=\"{}em\" x=\"10\" y=\"{}\" font-size=\"{}\" class=\"{}\"{fill}>{spans}</text>",
                number(dy),
                number(y),
                number(font),
                escape(&class),
            ));
            before += count;
        }
        group.push_str("</g>");
        out.push_str(&group);
        Ok(())
    }

    /// The today marker (`drawToday`, `ganttRenderer.js:856-876`): a line at `now`, unless the
    /// marker is `off`.
    pub(super) fn today(&self, out: &mut String) {
        let marker = self.model.today_marker.as_str();
        if marker == "off" {
            return;
        }
        let sp = self.conf.left_padding as f64;
        let x = self.scale.apply(self.now as f64) + sp;
        let title_top = self.conf.title_top_margin as f64;
        let style = if marker.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", escape(&marker.replace(',', ";")))
        };
        out.push_str(&format!(
            "<g class=\"today\"><line x1=\"{x}\" x2=\"{x}\" y1=\"{}\" y2=\"{}\" class=\"today\"{}{style}></line></g>",
            number(title_top),
            number(self.h - title_top),
            self.colour("stroke", &self.tokens().today_line_color),
            x = number(x),
        ));
    }

    /// The title of the diagram, centred at the top (`ganttRenderer.js:190-195`). It is written
    /// even when the diagram has none.
    pub(super) fn title(&self, out: &mut String, texts: &mut Texts) {
        let title = label(&self.model.title);
        texts.push(&title.plain, FontStyle::Normal);
        let fill = self.colour("fill", style::title_token(self.tokens()));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" class=\"titleText\"{fill}>{}</text>",
            number(self.w / 2.0),
            number(self.conf.title_top_margin as f64),
            title.xml,
        ));
    }
}

/// The state of a bar, from its flags, as the reference's class string takes them.
fn bar_state(active: bool, done: bool, crit: bool) -> Bar {
    if active {
        if crit { Bar::ActiveCrit } else { Bar::Active }
    } else if done {
        if crit { Bar::DoneCrit } else { Bar::Done }
    } else if crit {
        Bar::Crit
    } else {
        Bar::Task
    }
}

/// The classes a bar's state adds to its own name (`taskClass`), before the section number.
fn state_classes(task: &ferrigura_model::gantt::GanttTask) -> String {
    let mut task_class = String::new();
    if task.active {
        if task.crit {
            task_class.push_str(" activeCrit");
        } else {
            task_class = " active".to_owned();
        }
    } else if task.done {
        task_class = if task.crit {
            " doneCrit".to_owned()
        } else {
            " done".to_owned()
        };
    } else if task.crit {
        task_class.push_str(" crit");
    }
    if task_class.is_empty() {
        task_class = " task".to_owned();
    }
    if task.milestone {
        task_class = format!(" milestone {task_class}");
    }
    if task.vert {
        task_class = format!(" vert {task_class}");
    }
    task_class
}

/// The class of a bar: `task` and its state, the section number, and the diagram's classes
/// (`ganttRenderer.js:338-388`). Trimmed, as the sanitiser leaves it.
fn bar_class(task: &ferrigura_model::gantt::GanttTask, section: usize, state: String) -> String {
    format!("task{state}{section} {}", task.classes.join(" "))
        .trim()
        .to_owned()
}
