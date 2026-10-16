//! Canonical model of a gantt diagram.
//!
//! The model is the state of the reference's gantt DB after `getTasks()`, less what the input does
//! not contain or only rendering needs. Dates are epoch milliseconds in UTC; `null` is an
//! unresolved value (the reference's `undefined`) or an Invalid Date (its `NaN`). The rules are in
//! `docs-port/canonical-model.md` and `docs-port/gantt/PLAN.md`, section 6.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed gantt diagram: the header statements and the tasks in the order the DB keeps them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GanttDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Diagram title, sanitised. The grammar's `title` wins when it is non-empty; otherwise the
    /// front matter's title is kept, and otherwise it is empty.
    pub title: String,
    /// Accessible title from `accTitle`, sanitised, without leading blanks.
    pub acc_title: String,
    /// Accessible description from `accDescr`, sanitised, with blanks after line breaks removed.
    pub acc_descr: String,
    /// The `dateFormat` statement, as written after its keyword (blanks included).
    pub date_format: String,
    /// The `axisFormat` statement, as written after its keyword (blanks included).
    pub axis_format: String,
    /// The `tickInterval` statement, as written; `null` when the statement is absent.
    pub tick_interval: Option<String>,
    /// The `todayMarker` statement, as written; `off` disables the marker.
    pub today_marker: String,
    /// Whether `topAxis` was given.
    pub top_axis: bool,
    /// Whether `inclusiveEndDates` was given.
    pub inclusive_end_dates: bool,
    /// The first day of the week set by `weekday`, in lower case; `sunday` by default.
    pub weekday: String,
    /// The first weekend day set by `weekend`, in lower case; `saturday` by default.
    pub weekend: String,
    /// The dates and day names that `includes` lists, lower case, de-duplicated, first seen first.
    pub includes: Vec<String>,
    /// The dates, day names and `weekends` that `excludes` lists, lower case, de-duplicated.
    pub excludes: Vec<String>,
    /// The section names in the order they were given, duplicates kept.
    pub sections: Vec<String>,
    /// Task id to the sanitised URL of its `click … href` statement, for tasks that exist.
    pub links: std::collections::BTreeMap<String, String>,
    /// The tasks in the order the DB keeps them: the order of their statements.
    pub tasks: Vec<GanttTask>,
}

/// One task of a gantt diagram, with its dates resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GanttTask {
    /// The task id, given or generated (`task1`, `task2`, …).
    pub id: String,
    /// The task text as written, untrimmed and unsanitised.
    pub task: String,
    /// The section the task is in; empty before the first `section`.
    pub section: String,
    /// The position of the task among the non-vertical tasks; `-1` for a vertical task.
    pub order: i64,
    /// Whether the task is a vertical marker (`vert`).
    pub vert: bool,
    /// Whether the task is a milestone.
    pub milestone: bool,
    /// Whether the task is `active`.
    pub active: bool,
    /// Whether the task is `done`.
    pub done: bool,
    /// Whether the task is `crit`.
    pub crit: bool,
    /// The classes the task has, in order; `clickable` comes from `click`.
    pub classes: Vec<String>,
    /// Whether the end was given as a `YYYY-MM-DD` date, so that `excludes` do not move it.
    pub manual_end_time: bool,
    /// The start, in epoch milliseconds; `null` when unresolved.
    pub start_time: Option<i64>,
    /// The end, in epoch milliseconds; `null` when unresolved. With `excludes`, the end moved past
    /// the excluded days.
    pub end_time: Option<i64>,
    /// The end the bar is drawn to: the last working day before the excluded tail; `null` when
    /// `excludes` do not apply.
    pub render_end_time: Option<i64>,
}

impl GanttDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`] and the DB's defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            date_format: String::new(),
            axis_format: String::new(),
            tick_interval: None,
            today_marker: String::new(),
            top_axis: false,
            inclusive_end_dates: false,
            weekday: "sunday".to_owned(),
            weekend: "saturday".to_owned(),
            includes: Vec::new(),
            excludes: Vec::new(),
            sections: Vec::new(),
            links: std::collections::BTreeMap::new(),
            tasks: Vec::new(),
        }
    }
}

impl Default for GanttDiagram {
    fn default() -> Self {
        Self::new()
    }
}
