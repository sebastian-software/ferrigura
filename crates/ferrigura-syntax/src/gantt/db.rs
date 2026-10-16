//! The gantt DB of `ganttDb.js` for one parse: the state the grammar's actions set, the tasks of
//! `addTask`, and the date compile of `getTasks` (`compileTasks`, `compileTask`, `checkTaskDates`,
//! `fixTaskDates`, `getStartDate`, `getEndDate`). The plan is in `docs-port/gantt/PLAN.md`,
//! sections 3 to 5; the reference's line numbers are in `docs-port/gantt/NOTES.md`, section 3.
//!
//! The DB keeps no state across parses (the reference's `taskDb` does, PLAN 4). Lookups by id use
//! the map of the last definition of each id, as `taskDb` does; the id `__proto__` is never stored,
//! because the reference's assignment to it is ignored.

use std::collections::{BTreeMap, HashMap};

use ferrigura_model::gantt::{GanttDiagram, GanttTask};

use super::date::{self, Time, Unit};
use super::url::sanitize_url;
use crate::sanitize::sanitize_text;
use crate::text::{is_js_space, js_trim, newline_then_space_to_newline, strip_leading_space};

/// A rejection of the diagram by the DB, with the offset of the statement it comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DbError {
    pub offset: usize,
    pub message: String,
}

/// The start of a task as its statement gives it (`parseData`).
#[derive(Debug, Clone, PartialEq, Eq)]
enum StartSpec {
    /// The end of the task before this one (the first field is missing).
    PrevTaskEnd,
    /// A start date, an `after` reference, or a timestamp.
    GetStartDate(String),
}

/// A task of the DB (`rawTask`), with the state the date compile sets.
#[derive(Debug, Clone)]
struct RawTask {
    id: String,
    /// The id of the task before this one; `None` for the first task, whose key is `undefined`.
    prev_task_id: Option<String>,
    start_spec: StartSpec,
    end_data: String,
    /// Offset of the statement, for the errors of the compile.
    offset: usize,
    section: String,
    task: String,
    order: i64,
    vert: bool,
    milestone: bool,
    active: bool,
    done: bool,
    crit: bool,
    classes: Vec<String>,
    manual_end_time: bool,
    processed: bool,
    /// `undefined` is `None`; an Invalid Date is `Some(NaN)`, which is truthy as the reference's
    /// Date object is.
    start: Option<Time>,
    end: Option<Time>,
    render_end: Option<Time>,
}

/// The state of one gantt diagram while it is parsed and compiled.
#[derive(Debug, Clone)]
pub(crate) struct Db {
    now: Time,
    title: String,
    acc_title: String,
    acc_descr: String,
    date_format: String,
    axis_format: String,
    tick_interval: Option<String>,
    today_marker: String,
    inclusive_end_dates: bool,
    top_axis: bool,
    weekday: String,
    weekend: String,
    includes: Vec<String>,
    excludes: Vec<String>,
    links: BTreeMap<String, String>,
    sections: Vec<String>,
    current_section: String,
    tasks: Vec<RawTask>,
    task_db: HashMap<String, usize>,
    last_task_id: Option<String>,
    last_order: i64,
    task_count: u64,
}

impl Db {
    /// An empty DB whose clock reads `now` (milliseconds since the epoch, as a JS number).
    pub(crate) fn new(now: Time) -> Self {
        Self {
            now,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            date_format: String::new(),
            axis_format: String::new(),
            tick_interval: None,
            today_marker: String::new(),
            inclusive_end_dates: false,
            top_axis: false,
            weekday: "sunday".to_owned(),
            weekend: "saturday".to_owned(),
            includes: Vec::new(),
            excludes: Vec::new(),
            links: BTreeMap::new(),
            sections: Vec::new(),
            current_section: String::new(),
            tasks: Vec::new(),
            task_db: HashMap::new(),
            last_task_id: None,
            last_order: 0,
            task_count: 0,
        }
    }

    /// `setDiagramTitle`: sanitised, not trimmed.
    pub(crate) fn set_diagram_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle` (commonDb): sanitised, then without leading blanks.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription` (commonDb): sanitised, then each line break and the blanks after it
    /// reduced to one line break.
    pub(crate) fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    pub(crate) fn set_date_format(&mut self, text: &str) {
        self.date_format = text.to_owned();
    }

    pub(crate) fn set_axis_format(&mut self, text: &str) {
        self.axis_format = text.to_owned();
    }

    pub(crate) fn set_tick_interval(&mut self, text: &str) {
        self.tick_interval = Some(text.to_owned());
    }

    pub(crate) fn set_today_marker(&mut self, text: &str) {
        self.today_marker = text.to_owned();
    }

    pub(crate) fn enable_inclusive_end_dates(&mut self) {
        self.inclusive_end_dates = true;
    }

    pub(crate) fn enable_top_axis(&mut self) {
        self.top_axis = true;
    }

    /// `setIncludes` (`mergeTokens`): lower case, split on blanks and commas, de-duplicated.
    pub(crate) fn set_includes(&mut self, text: &str) {
        merge_tokens(&mut self.includes, text);
    }

    /// `setExcludes` (`mergeTokens`).
    pub(crate) fn set_excludes(&mut self, text: &str) {
        merge_tokens(&mut self.excludes, text);
    }

    pub(crate) fn set_weekday(&mut self, name: &str) {
        self.weekday = name.to_owned();
    }

    pub(crate) fn set_weekend(&mut self, name: &str) {
        self.weekend = name.to_owned();
    }

    /// `addSection`: the name is kept as written, and it is the section of the tasks that follow.
    pub(crate) fn add_section(&mut self, name: &str) {
        self.current_section = name.to_owned();
        self.sections.push(name.to_owned());
    }

    /// `addTask` (lines 596-630) with `parseData`: the tags, the fields and the id of the task.
    pub(crate) fn add_task(
        &mut self,
        descr: String,
        data: &str,
        offset: usize,
    ) -> Result<(), DbError> {
        let mut fields = split_task_data(data);
        let tags = take_tags(&mut fields).map_err(|message| DbError { offset, message })?;
        for field in &mut fields {
            *field = js_trim(field).to_owned();
        }

        let (id, start_spec, end_data) = match fields.as_slice() {
            [end] => {
                let id = self.generated_id();
                (id, StartSpec::PrevTaskEnd, end.clone())
            }
            [start, end] => {
                let id = self.generated_id();
                (id, StartSpec::GetStartDate(start.clone()), end.clone())
            }
            [id, start, end] => (
                id.clone(),
                StartSpec::GetStartDate(start.clone()),
                end.clone(),
            ),
            _ => {
                // Zero or four or more fields: the reference's `startTime` is undefined, and
                // `compileTask` reads its `type`, a TypeError in `getTasks`.
                return Err(DbError {
                    offset,
                    message: "the task has no start and end, or too many fields".to_owned(),
                });
            }
        };

        let order = if tags.vert {
            -1
        } else {
            let order = self.last_order;
            self.last_order += 1;
            order
        };
        let position = self.tasks.len();
        if id != "__proto__" {
            self.task_db.insert(id.clone(), position);
        }
        self.tasks.push(RawTask {
            id: id.clone(),
            prev_task_id: self.last_task_id.clone(),
            start_spec,
            end_data,
            offset,
            section: self.current_section.clone(),
            task: descr,
            order,
            vert: tags.vert,
            milestone: tags.milestone,
            active: tags.active,
            done: tags.done,
            crit: tags.crit,
            classes: Vec::new(),
            manual_end_time: false,
            processed: false,
            start: None,
            end: None,
            render_end: None,
        });
        self.last_task_id = Some(id);
        Ok(())
    }

    /// `parseId()` with no id: `task` and a counter that counts the tasks without one.
    fn generated_id(&mut self) -> String {
        self.task_count += 1;
        format!("task{}", self.task_count)
    }

    /// The task of an id, as `findTaskById` gives it (the last definition seen so far).
    fn find_task(&self, id: &str) -> Option<usize> {
        self.task_db.get(id).copied()
    }

    /// `setClickEvent` in strict mode: no callback is stored, the classes are set.
    pub(crate) fn set_click_event(&mut self, ids: &str) {
        self.set_class(ids, "clickable");
    }

    /// `setLink`: the sanitised address of each task that exists, and the `clickable` class.
    pub(crate) fn set_link(&mut self, ids: &str, url: &str) {
        let link = sanitize_url(url);
        for id in ids.split(',') {
            if self.find_task(id).is_some() {
                self.links.insert(id.to_owned(), link.clone());
            }
        }
        self.set_class(ids, "clickable");
    }

    /// `setClass`: adds the class to each task of the comma-separated ids that exists.
    fn set_class(&mut self, ids: &str, class: &str) {
        for id in ids.split(',') {
            if let Some(position) = self.find_task(id) {
                self.tasks[position].classes.push(class.to_owned());
            }
        }
    }

    /// `getTasks` (lines 169-181): `compileTasks` once, then up to ten more passes while a task is
    /// unprocessed.
    pub(crate) fn get_tasks(&mut self) -> Result<(), DbError> {
        let mut all_processed = self.compile_tasks()?;
        let mut passes = 0;
        while !all_processed && passes < 10 {
            all_processed = self.compile_tasks()?;
            passes += 1;
        }
        Ok(())
    }

    /// `compileTasks` (lines 658-704): every task in order; the result is whether all are processed.
    fn compile_tasks(&mut self) -> Result<bool, DbError> {
        let mut all_processed = true;
        for position in 0..self.tasks.len() {
            self.compile_task(position)?;
            all_processed = all_processed && self.tasks[position].processed;
        }
        Ok(all_processed)
    }

    /// `compileTask` (lines 659-695). Every pass recomputes the start and the end from the raw
    /// data, and the other tasks are read as they stand.
    fn compile_task(&mut self, position: usize) -> Result<bool, DbError> {
        let offset = self.tasks[position].offset;
        match self.tasks[position].start_spec.clone() {
            StartSpec::PrevTaskEnd => {
                let key = self.tasks[position]
                    .prev_task_id
                    .clone()
                    .unwrap_or_else(|| "undefined".to_owned());
                let Some(previous) = self.find_task(&key) else {
                    // `findTaskById(...).endTime` reads a property of undefined.
                    return Err(DbError {
                        offset,
                        message: "the task before this one is not defined".to_owned(),
                    });
                };
                self.tasks[position].start = self.tasks[previous].end;
            }
            StartSpec::GetStartDate(text) => {
                if let Some(start) = self.get_start_date(&text, offset)? {
                    self.tasks[position].start = Some(start);
                }
            }
        }

        if let Some(start) = self.tasks[position].start {
            let end_data = self.tasks[position].end_data.clone();
            let end = self.get_end_date(start, &end_data, offset)?;
            self.tasks[position].end = end;
            if end.is_some() {
                self.tasks[position].processed = true;
                self.tasks[position].manual_end_time =
                    date::strict_parse(&end_data, "YYYY-MM-DD", self.now)
                        .ok()
                        .flatten()
                        .is_some();
                self.check_task_dates(position)?;
            }
        }
        Ok(self.tasks[position].processed)
    }

    /// `getStartDate` (lines 305-375).
    fn get_start_date(&self, text: &str, offset: usize) -> Result<Option<Time>, DbError> {
        let text = js_trim(text);
        let format = self.date_format.trim();

        // A timestamp in milliseconds for the formats `x` and `X`.
        if (format == "x" || format == "X")
            && !text.is_empty()
            && text.bytes().all(|b| b.is_ascii_digit())
        {
            let value: f64 = text.parse().unwrap_or(f64::NAN);
            return Ok(Some(date::time_clip(value)));
        }

        // `after` takes the latest end of the tasks it names; with none found, today.
        if let Some(ids) = reference_ids(text, "after") {
            let mut latest: Option<usize> = None;
            for id in ids.split(' ').filter(|id| !id.is_empty()) {
                let Some(task) = self.find_task(id) else {
                    continue;
                };
                match latest {
                    None => latest = Some(task),
                    Some(current) => {
                        if let (Some(end), Some(best)) =
                            (self.tasks[task].end, self.tasks[current].end)
                            && end > best
                        {
                            latest = Some(task);
                        }
                    }
                }
            }
            return Ok(match latest {
                Some(task) => self.tasks[task].end,
                None => Some(date::start_of_day(self.now)),
            });
        }

        if let Some(time) = strict(text, format, self.now, offset)? {
            return Ok(Some(time));
        }
        // The legacy fallback of `new Date(str)`, with the reference's range check.
        let time = date::legacy_parse(text);
        let year = date::full_year(time);
        if time.is_nan() || year < -10000.0 || year > 10000.0 {
            return Err(DbError {
                offset,
                message: format!("Invalid date:{text}"),
            });
        }
        Ok(Some(time))
    }

    /// `getEndDate` (lines 409-466), without its warnings. `None` is an `undefined` result, which
    /// happens only for an `until` whose tasks have no start yet.
    fn get_end_date(&self, prev: Time, text: &str, offset: usize) -> Result<Option<Time>, DbError> {
        let text = js_trim(text);
        let format = self.date_format.trim();

        // `until` takes the earliest start of the tasks it names; with none found, today.
        if let Some(ids) = reference_ids(text, "until") {
            let mut earliest: Option<usize> = None;
            for id in ids.split(' ').filter(|id| !id.is_empty()) {
                let Some(task) = self.find_task(id) else {
                    continue;
                };
                match earliest {
                    None => earliest = Some(task),
                    Some(current) => {
                        if let (Some(start), Some(best)) =
                            (self.tasks[task].start, self.tasks[current].start)
                            && start < best
                        {
                            earliest = Some(task);
                        }
                    }
                }
            }
            return Ok(match earliest {
                Some(task) => self.tasks[task].start,
                None => Some(date::start_of_day(self.now)),
            });
        }

        if let Some(time) = strict(text, format, self.now, offset)? {
            let time = if self.inclusive_end_dates {
                date::add(time, 1.0, Unit::Day)
            } else {
                time
            };
            return Ok(Some(time));
        }

        // A duration from the start; an unparsable one leaves the end at the start.
        Ok(match date::parse_duration(text) {
            None => Some(prev),
            Some((value, unit)) => {
                let end = date::add(prev, value, unit);
                Some(if end.is_nan() { prev } else { end })
            }
        })
    }

    /// `checkTaskDates` (lines 227-254): with `excludes`, the end moves past the excluded days.
    fn check_task_dates(&mut self, position: usize) -> Result<(), DbError> {
        if self.excludes.is_empty() || self.tasks[position].manual_end_time {
            return Ok(());
        }
        let (Some(start), Some(end)) = (self.tasks[position].start, self.tasks[position].end)
        else {
            return Ok(());
        };
        // The start day itself is never tested.
        let start = date::add(start, 1.0, Unit::Day);
        let offset = self.tasks[position].offset;
        let (fixed_end, render_end) = self.fix_task_dates(start, end, offset)?;
        self.tasks[position].end = Some(fixed_end);
        self.tasks[position].render_end = render_end;
        Ok(())
    }

    /// `fixTaskDates` (lines 268-288): the end moves past each excluded day; the render end is the
    /// last working day before the excluded tail.
    fn fix_task_dates(
        &self,
        mut start: Time,
        mut end: Time,
        offset: usize,
    ) -> Result<(Time, Option<Time>), DbError> {
        let mut invalid = false;
        let mut render_end = None;
        let max_end = date::add(end, 10000.0, Unit::Day);
        while start <= end {
            if !invalid {
                render_end = Some(end);
            }
            invalid = self.is_invalid_date(start, offset)?;
            if invalid {
                end = date::add(end, 1.0, Unit::Day);
                if end > max_end {
                    return Err(DbError {
                        offset,
                        message: "Failed to find a valid date that was not excluded by `excludes` after 10,000 iterations."
                            .to_owned(),
                    });
                }
            }
            start = date::add(start, 1.0, Unit::Day);
        }
        Ok((end, render_end))
    }

    /// `isInvalidDate` of the DB (see [`invalid_date`]).
    fn is_invalid_date(&self, date: Time, offset: usize) -> Result<bool, DbError> {
        invalid_date(
            &self.date_format,
            &self.includes,
            &self.excludes,
            &self.weekend,
            date,
            offset,
        )
    }

    /// The model of the DB after `getTasks`. Dates are the epoch milliseconds, and an unresolved
    /// or Invalid Date is `null`.
    pub(crate) fn into_model(self) -> GanttDiagram {
        let mut diagram = GanttDiagram::new();
        diagram.title = self.title;
        diagram.acc_title = self.acc_title;
        diagram.acc_descr = self.acc_descr;
        diagram.date_format = self.date_format;
        diagram.axis_format = self.axis_format;
        diagram.tick_interval = self.tick_interval;
        diagram.today_marker = self.today_marker;
        diagram.top_axis = self.top_axis;
        diagram.inclusive_end_dates = self.inclusive_end_dates;
        diagram.weekday = self.weekday;
        diagram.weekend = self.weekend;
        diagram.includes = self.includes;
        diagram.excludes = self.excludes;
        diagram.sections = self.sections;
        diagram.links = self.links;
        diagram.tasks = self
            .tasks
            .into_iter()
            .map(|task| GanttTask {
                id: task.id,
                task: task.task,
                section: task.section,
                order: task.order,
                vert: task.vert,
                milestone: task.milestone,
                active: task.active,
                done: task.done,
                crit: task.crit,
                classes: task.classes,
                manual_end_time: task.manual_end_time,
                start_time: model_time(task.start),
                end_time: model_time(task.end),
                render_end_time: model_time(task.render_end),
            })
            .collect();
        diagram
    }
}

/// A strict parse of the DB; a throw of the reference is a rejection at the task's statement.
fn strict(text: &str, format: &str, now: Time, offset: usize) -> Result<Option<Time>, DbError> {
    date::strict_parse(text, format, now).map_err(|thrown| DbError {
        offset,
        message: thrown.0,
    })
}

/// `dayjs(time).format(format)`; a throw of the reference is a rejection at the task's statement.
fn shown(time: Time, format: &str, offset: usize) -> Result<String, DbError> {
    date::format(time, format).map_err(|thrown| DbError {
        offset,
        message: thrown.0,
    })
}

/// `isInvalidDate` (`ganttDb.js:183-201`): whether the day that starts at `date` is excluded. The
/// includes win; then the weekend rule, the day name, and the date or the format's text decide. The
/// renderer uses it for the excluded ranges (`ganttRenderer.js:575`).
pub(crate) fn invalid_date(
    date_format: &str,
    includes: &[String],
    excludes: &[String],
    weekend: &str,
    date: Time,
    offset: usize,
) -> Result<bool, DbError> {
    let formatted = shown(date, date_format.trim(), offset)?;
    let date_only = shown(date, "YYYY-MM-DD", offset)?;
    if includes.contains(&formatted) || includes.contains(&date_only) {
        return Ok(false);
    }
    if excludes.iter().any(|item| item == "weekends") {
        let weekend_start = match weekend {
            "friday" => Some(5),
            "saturday" => Some(6),
            _ => None,
        };
        if let Some(start) = weekend_start {
            let day = date::iso_weekday(date);
            if day == start || day == start + 1 {
                return Ok(true);
            }
        }
    }
    let day_name = shown(date, "dddd", offset)?.to_lowercase();
    if excludes.contains(&day_name) {
        return Ok(true);
    }
    Ok(excludes.contains(&formatted) || excludes.contains(&date_only))
}

/// A date of the DB as a model value: integral milliseconds, or `null` for `undefined` and NaN.
fn model_time(time: Option<Time>) -> Option<i64> {
    time.filter(|value| value.is_finite())
        .map(|value| value as i64)
}

/// `mergeTokens` (lines 133-139): the tokens of `text`, lower case and split on blanks and commas,
/// appended to `list` in first-seen order without duplicates.
fn merge_tokens(list: &mut Vec<String>, text: &str) {
    for token in text
        .to_lowercase()
        .split(|c: char| is_js_space(c) || c == ',')
        .filter(|token| !token.is_empty())
    {
        if !list.iter().any(|item| item == token) {
            list.push(token.to_owned());
        }
    }
}

/// `after` or `until` followed by its task ids (`^after\s+(?<ids>[\d\w- ]+)`): the ids, if the
/// text starts with the keyword, one whitespace character at least, and an id character.
fn reference_ids<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(keyword)?;
    let spaces = rest.len() - rest.trim_start_matches(is_js_space).len();
    if spaces == 0 {
        return None;
    }
    let ids = &rest[spaces..];
    let len = ids
        .bytes()
        .take_while(|&b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b' ')
        .count();
    (len > 0).then(|| &ids[..len])
}

/// The data of a task statement, as `parseData` splits it: the leading colon is removed, then the
/// data is split on commas.
fn split_task_data(data: &str) -> Vec<String> {
    let body = data.strip_prefix(':').unwrap_or(data);
    body.split(',').map(str::to_owned).collect()
}

/// The tags of a task (`active`, `done`, `crit`, `milestone`, `vert`).
#[derive(Debug, Default, Clone, Copy)]
struct Tags {
    active: bool,
    done: bool,
    crit: bool,
    milestone: bool,
    vert: bool,
}

/// `getTaskTags` (lines 885-899): removes the tags at the start of the fields, in any order. The
/// reference reads the first field again after the last one is removed, which is a TypeError: a
/// task with only tags is rejected.
fn take_tags(fields: &mut Vec<String>) -> Result<Tags, String> {
    const NAMES: [&str; 5] = ["active", "done", "crit", "milestone", "vert"];
    let mut tags = Tags::default();
    let mut match_found = true;
    while match_found {
        match_found = false;
        for name in NAMES {
            let Some(first) = fields.first() else {
                return Err("the task has only tags and no text".to_owned());
            };
            if js_trim(first) == name {
                match name {
                    "active" => tags.active = true,
                    "done" => tags.done = true,
                    "crit" => tags.crit = true,
                    "milestone" => tags.milestone = true,
                    _ => tags.vert = true,
                }
                fields.remove(0);
                match_found = true;
            }
        }
    }
    Ok(tags)
}
