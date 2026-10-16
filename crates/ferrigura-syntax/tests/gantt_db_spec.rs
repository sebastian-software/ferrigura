//! Port of `packages/mermaid/src/diagrams/gantt/ganttDb.spec.ts`. The DB calls of the spec become
//! diagram text, parsed with the fixed clock 1010-10-10T00:00Z (`docs-port/gantt/PLAN.md`, section
//! 5): `setDateFormat` is `dateFormat`, `addSection` is `section`, `addTask(name, data)` is
//! `name :data`, and `getTasks()` is the model's tasks. The spec's `Date` values are UTC here.
//! The log spies become outcomes: an unknown id falls back to today's midnight, and a milestone
//! without a duration ends where it starts.

use ferrigura_model::gantt::{GanttDiagram, GanttTask};
use ferrigura_syntax::ParseError;
use ferrigura_syntax::gantt::{Options, parse_with};

const NOW: i64 = -30_270_326_400_000;
const DAY: i64 = 86_400_000;

/// Days since the epoch of a proleptic Gregorian date, `month` from 1.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let year_of_era = y - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// A UTC instant, as the spec's `new Date(y, m, d[, h, mi, s])` under TZ=UTC.
fn at(year: i64, month: i64, day: i64) -> i64 {
    days_from_civil(year, month, day) * DAY
}

fn at_time(year: i64, month: i64, day: i64, hours: i64, minutes: i64) -> i64 {
    at(year, month, day) + hours * 3_600_000 + minutes * 60_000
}

fn parse(text: &str) -> Result<GanttDiagram, ParseError> {
    parse_with(text, Options { now: NOW })
}

fn parsed(text: &str) -> GanttDiagram {
    parse(text).unwrap_or_else(|error| panic!("the diagram should parse: {error}"))
}

/// Checks one task: its dates, order, id and text.
fn check(
    task: &GanttTask,
    start: Option<i64>,
    end: Option<i64>,
    order: Option<i64>,
    id: &str,
    text: &str,
) {
    assert_eq!(task.start_time, start, "start of {id}");
    assert_eq!(task.end_time, end, "end of {id}");
    if let Some(order) = order {
        assert_eq!(task.order, order, "order of {id}");
    }
    assert_eq!(task.id, id);
    assert_eq!(task.task, text);
}

#[test]
fn should_clear_get_tasks() {
    assert!(parsed("gantt").tasks.is_empty());
}

#[test]
fn should_clear_get_acc_title() {
    assert_eq!(
        parsed("gantt\ndateFormat YYYY-MM-DD\nsection s\nA:a1, 2014-01-01, 1d").acc_title,
        ""
    );
}

#[test]
fn should_clear_get_acc_description() {
    assert_eq!(parsed("gantt").acc_descr, "");
}

#[test]
fn should_clear_get_date_format() {
    assert_eq!(parsed("gantt").date_format, "");
}

#[test]
fn should_clear_get_axis_format() {
    assert_eq!(parsed("gantt").axis_format, "");
}

#[test]
fn should_clear_get_today_marker() {
    assert_eq!(parsed("gantt").today_marker, "");
}

#[test]
fn should_clear_get_excludes() {
    assert!(parsed("gantt").excludes.is_empty());
}

#[test]
fn should_clear_get_sections() {
    assert!(parsed("gantt").sections.is_empty());
}

#[test]
fn should_clear_ends_dates_are_inclusive() {
    assert!(!parsed("gantt").inclusive_end_dates);
}

#[test]
#[ignore = "port: display mode is configuration, not part of the model (PLAN section 6)"]
fn should_clear_get_display_mode() {}

#[test]
fn should_handle_fixed_dates() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2013-01-12");
    check(
        &d.tasks[0],
        Some(at(2013, 1, 1)),
        Some(at(2013, 1, 12)),
        None,
        "id1",
        "test1",
    );
}

#[test]
fn should_handle_duration_days_instead_of_fixed_date_to_determine_end_date() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2d");
    check(
        &d.tasks[0],
        Some(at(2013, 1, 1)),
        Some(at(2013, 1, 3)),
        None,
        "id1",
        "test1",
    );
}

#[test]
fn should_handle_duration_hours_instead_of_fixed_date_to_determine_end_date() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2h");
    check(
        &d.tasks[0],
        Some(at(2013, 1, 1)),
        Some(at_time(2013, 1, 1, 2, 0)),
        None,
        "id1",
        "test1",
    );
}

#[test]
fn should_handle_duration_minutes_instead_of_fixed_date_to_determine_end_date() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2m");
    check(
        &d.tasks[0],
        Some(at(2013, 1, 1)),
        Some(at_time(2013, 1, 1, 0, 2)),
        None,
        "id1",
        "test1",
    );
}

#[test]
fn should_handle_duration_seconds_instead_of_fixed_date_to_determine_end_date() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2s");
    check(
        &d.tasks[0],
        Some(at(2013, 1, 1)),
        Some(at_time(2013, 1, 1, 0, 0) + 2_000),
        None,
        "id1",
        "test1",
    );
}

#[test]
fn should_handle_duration_weeks_instead_of_fixed_date_to_determine_end_date() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2w");
    check(
        &d.tasks[0],
        Some(at(2013, 1, 1)),
        Some(at(2013, 1, 15)),
        None,
        "id1",
        "test1",
    );
}

#[test]
fn should_handle_fixed_dates_without_id() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:2013-01-01,2013-01-12");
    check(
        &d.tasks[0],
        Some(at(2013, 1, 1)),
        Some(at(2013, 1, 12)),
        None,
        "task1",
        "test1",
    );
}

#[test]
fn should_handle_duration_instead_of_a_fixed_date_to_determine_end_date_without_id() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:2013-01-01,4d");
    check(
        &d.tasks[0],
        Some(at(2013, 1, 1)),
        Some(at(2013, 1, 5)),
        None,
        "task1",
        "test1",
    );
}

#[test]
fn should_handle_a_task_after_a_fixed_duration_task_with_an_end_date_of_two_weeks() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2w\ntest2:id2,after id1,1d",
    );
    check(
        &d.tasks[1],
        Some(at(2013, 1, 15)),
        d.tasks[1].end_time,
        None,
        "id2",
        "test2",
    );
}

#[test]
fn should_handle_relative_start_from_an_unknown_id_as_today() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2w\ntest2:id2,after id3,1d",
    );
    check(
        &d.tasks[1],
        Some(NOW),
        d.tasks[1].end_time,
        None,
        "id2",
        "test2",
    );
}

#[test]
fn should_handle_relative_start_without_id_after_a_task() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2w\ntest2:after id1,1d",
    );
    check(
        &d.tasks[1],
        Some(at(2013, 1, 15)),
        d.tasks[1].end_time,
        None,
        "task1",
        "test2",
    );
}

#[test]
fn should_handle_a_fixed_end_date_after_a_task_without_id() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2w\ntest2:after id1,2013-01-26",
    );
    check(
        &d.tasks[1],
        Some(at(2013, 1, 15)),
        Some(at(2013, 1, 26)),
        None,
        "task1",
        "test2",
    );
}

#[test]
fn should_handle_a_duration_after_a_task_without_id() {
    let d =
        parsed("gantt\ndateFormat YYYY-MM-DD\nsection testa1\ntest1:id1,2013-01-01,2w\ntest2:2d");
    check(
        &d.tasks[1],
        Some(at(2013, 1, 15)),
        Some(at(2013, 1, 17)),
        None,
        "task1",
        "test2",
    );
}

#[test]
fn should_handle_milliseconds() {
    let d = parsed(
        "gantt\ndateFormat x\nsection testa1\ntest1:id1,0,20ms\ntest2:id2,after id1,5ms\nsection testa2\ntest3:id3,20,10ms\ntest4:id4,after id3,0.005s",
    );
    assert_eq!(
        (d.tasks[0].start_time, d.tasks[0].end_time),
        (Some(0), Some(20))
    );
    assert_eq!(
        (d.tasks[1].start_time, d.tasks[1].end_time),
        (Some(20), Some(25))
    );
    assert_eq!(
        (d.tasks[2].start_time, d.tasks[2].end_time),
        (Some(20), Some(30))
    );
    assert_eq!(
        (d.tasks[3].start_time, d.tasks[3].end_time),
        (Some(30), Some(35))
    );
}

#[test]
fn should_handle_relative_start_date_based_on_id_regardless_of_sections() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nsection sec1\ntest1:id1,2013-01-01,2w\ntest2:id2,after id3,1d\nsection sec2\ntest3:id3,after id1,2d",
    );
    check(
        &d.tasks[1],
        Some(at(2013, 1, 17)),
        Some(at(2013, 1, 18)),
        None,
        "id2",
        "test2",
    );
    check(
        &d.tasks[2],
        Some(at(2013, 1, 15)),
        Some(at(2013, 1, 17)),
        None,
        "id3",
        "test3",
    );
}

#[test]
fn should_handle_relative_end_date_based_on_id_regardless_of_sections() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nsection sec1\ntask1:id1,2013-01-01,until id3\nsection sec2\ntask2:id2,2013-01-10,until id3\ntask3:id3,2013-02-01,2d",
    );
    check(
        &d.tasks[0],
        Some(at(2013, 1, 1)),
        Some(at(2013, 2, 1)),
        None,
        "id1",
        "task1",
    );
    check(
        &d.tasks[1],
        Some(at(2013, 1, 10)),
        Some(at(2013, 2, 1)),
        None,
        "id2",
        "task2",
    );
}

#[test]
fn should_handle_relative_start_date_based_on_multiple_id() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nsection sec1\ntask1:id1,after id2 id3 id4,1d\ntask2:id2,2013-01-01,1d\ntask3:id3,2013-02-01,3d\ntask4:id4,2013-02-01,2d",
    );
    assert_eq!(d.tasks[0].end_time, Some(at(2013, 2, 5)));
    assert_eq!(d.tasks[0].id, "id1");
    assert_eq!(d.tasks[0].task, "task1");
}

#[test]
fn should_handle_relative_end_date_based_on_multiple_id() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nsection sec1\ntask1:id1,2013-01-01,until id2 id3 id4\ntask2:id2,2013-01-11,1d\ntask3:id3,2013-02-10,1d\ntask4:id4,2013-02-12,1d",
    );
    assert_eq!(d.tasks[0].end_time, Some(at(2013, 1, 11)));
    assert_eq!(d.tasks[0].id, "id1");
    assert_eq!(d.tasks[0].task, "task1");
}

#[test]
fn should_ignore_weekends() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nexcludes weekends 2019-02-06,friday\nsection weekends skip test\ntest1:id1,2019-02-01,1d\ntest2:id2,after id1,2d\ntest3:id3,after id2,7d\ntest4:id4,2019-02-01,2019-02-20\ntest5:id5,after id4,1d\nsection full ending task on last day\ntest6:id6,2019-02-13,2d\ntest7:id7,after id6,1d",
    );
    let t = &d.tasks;
    check(
        &t[0],
        Some(at(2019, 2, 1)),
        Some(at(2019, 2, 4)),
        None,
        "id1",
        "test1",
    );
    assert_eq!(t[0].render_end_time, Some(at(2019, 2, 2)));
    check(
        &t[1],
        Some(at(2019, 2, 4)),
        Some(at(2019, 2, 7)),
        None,
        "id2",
        "test2",
    );
    assert_eq!(t[1].render_end_time, Some(at(2019, 2, 6)));
    check(
        &t[2],
        Some(at(2019, 2, 7)),
        Some(at(2019, 2, 20)),
        None,
        "id3",
        "test3",
    );
    assert_eq!(t[2].render_end_time, Some(at(2019, 2, 20)));
    check(
        &t[3],
        Some(at(2019, 2, 1)),
        Some(at(2019, 2, 20)),
        None,
        "id4",
        "test4",
    );
    assert_eq!(t[3].render_end_time, None, "a fixed end is not moved");
    check(
        &t[4],
        Some(at(2019, 2, 20)),
        Some(at(2019, 2, 21)),
        None,
        "id5",
        "test5",
    );
    assert_eq!(t[4].render_end_time, Some(at(2019, 2, 21)));
    check(
        &t[5],
        Some(at(2019, 2, 13)),
        Some(at(2019, 2, 18)),
        None,
        "id6",
        "test6",
    );
    assert_eq!(t[5].render_end_time, Some(at(2019, 2, 15)));
    check(
        &t[6],
        Some(at(2019, 2, 18)),
        Some(at(2019, 2, 19)),
        None,
        "id7",
        "test7",
    );
}

#[test]
fn should_ignore_weekends_starting_on_friday() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nexcludes weekends\nweekend friday\nsection friday-saturday weekends skip test\ntest1:id1,2024-02-28, 3d",
    );
    check(
        &d.tasks[0],
        Some(at(2024, 2, 28)),
        Some(at(2024, 3, 4)),
        None,
        "id1",
        "test1",
    );
}

#[test]
fn should_not_infinite_loop_when_excluding_everything() {
    let all = "gantt\ndateFormat YYYY-MM-DD\nexcludes weekends,monday,tuesday,wednesday,thursday,friday\nweekend saturday\nsection weekends skip test\ntest1:id1,2019-02-01,7d";
    let error = parse(all).expect_err("every day is excluded");
    assert!(
        error.message.contains("Failed to find a valid date"),
        "{}",
        error.message
    );
    let allowed = "gantt\ndateFormat YYYY-MM-DD\nexcludes weekends,monday,tuesday,wednesday,thursday\nweekend saturday\nsection weekends skip test\ntest1:id1,2019-02-01,7d";
    assert!(parse(allowed).is_ok());
}

#[test]
fn should_merge_tokens_across_multiple_set_excludes_calls_issue_6270() {
    let d = parsed("gantt\nexcludes weekends\nexcludes 2019-02-06\nexcludes friday, monday");
    assert_eq!(d.excludes, ["weekends", "2019-02-06", "friday", "monday"]);
}

#[test]
fn should_dedupe_tokens_across_multiple_set_excludes_calls_issue_6270() {
    let d = parsed("gantt\nexcludes weekends,2019-02-06\nexcludes weekends 2019-02-07");
    assert_eq!(d.excludes, ["weekends", "2019-02-06", "2019-02-07"]);
}

#[test]
fn should_merge_tokens_across_multiple_set_includes_calls_issue_6270() {
    let d = parsed("gantt\nincludes 2019-02-06\nincludes 2019-02-07,2019-02-08");
    assert_eq!(d.includes, ["2019-02-06", "2019-02-07", "2019-02-08"]);
}

#[test]
fn should_maintain_the_order_in_which_tasks_are_created() {
    let text = [
        "gantt",
        "accTitle: Project Execution",
        "dateFormat YYYY-MM-DD",
        "section section A section",
        "Completed task:done,    des1, 2014-01-06,2014-01-08",
        "Active task:active,  des2, 2014-01-09, 3d",
        "Future task:des3, after des2, 5d",
        "Future task2:des4, after des3, 5d",
        "section section Critical tasks",
        "Completed task in the critical line:crit, done, 2014-01-06,24h",
        "Implement parser and jison:crit, done, after des1, 2d",
        "Create tests for parser:crit, active, 3d",
        "Future task in critical line:crit, 5d",
        "Create tests for renderer:2d",
        "Add to mermaid:1d",
        "section section Documentation",
        "Describe gantt syntax:active, a1, after des1, 3d",
        "Add gantt diagram to demo page:after a1  , 20h",
        "Add another diagram to demo page:doc1, after a1  , 48h",
        "section section Last section",
        "Describe gantt syntax:after doc1, 3d",
        "Add gantt diagram to demo page:20h",
        "Add another diagram to demo page:48h",
    ]
    .join("\n");
    let d = parsed(&text);
    let t = &d.tasks;
    let expected: [(i64, i64, i64, &str, &str); 16] = [
        (at(2014, 1, 6), at(2014, 1, 8), 0, "des1", "Completed task"),
        (at(2014, 1, 9), at(2014, 1, 12), 1, "des2", "Active task"),
        (at(2014, 1, 12), at(2014, 1, 17), 2, "des3", "Future task"),
        (at(2014, 1, 17), at(2014, 1, 22), 3, "des4", "Future task2"),
        (
            at(2014, 1, 6),
            at(2014, 1, 7),
            4,
            "task1",
            "Completed task in the critical line",
        ),
        (
            at(2014, 1, 8),
            at(2014, 1, 10),
            5,
            "task2",
            "Implement parser and jison",
        ),
        (
            at(2014, 1, 10),
            at(2014, 1, 13),
            6,
            "task3",
            "Create tests for parser",
        ),
        (
            at(2014, 1, 13),
            at(2014, 1, 18),
            7,
            "task4",
            "Future task in critical line",
        ),
        (
            at(2014, 1, 18),
            at(2014, 1, 20),
            8,
            "task5",
            "Create tests for renderer",
        ),
        (
            at(2014, 1, 20),
            at(2014, 1, 21),
            9,
            "task6",
            "Add to mermaid",
        ),
        (
            at(2014, 1, 8),
            at(2014, 1, 11),
            10,
            "a1",
            "Describe gantt syntax",
        ),
        (
            at(2014, 1, 11),
            at_time(2014, 1, 11, 20, 0),
            11,
            "task7",
            "Add gantt diagram to demo page",
        ),
        (
            at(2014, 1, 11),
            at(2014, 1, 13),
            12,
            "doc1",
            "Add another diagram to demo page",
        ),
        (
            at(2014, 1, 13),
            at(2014, 1, 16),
            13,
            "task8",
            "Describe gantt syntax",
        ),
        (
            at(2014, 1, 16),
            at_time(2014, 1, 16, 20, 0),
            14,
            "task9",
            "Add gantt diagram to demo page",
        ),
        (
            at_time(2014, 1, 16, 20, 0),
            at_time(2014, 1, 18, 20, 0),
            15,
            "task10",
            "Add another diagram to demo page",
        ),
    ];
    for (index, (start, end, order, id, task)) in expected.into_iter().enumerate() {
        check(&t[index], Some(start), Some(end), Some(order), id, task);
    }
}

#[test]
fn should_work_when_end_date_is_the_31st() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nsection Task endTime is on the 31st day of the month\ntest1:id1,2019-09-30,11d\ntest2:id2,after id1,20d",
    );
    check(
        &d.tasks[0],
        Some(at(2019, 9, 30)),
        Some(at(2019, 10, 11)),
        None,
        "id1",
        "test1",
    );
    check(
        &d.tasks[1],
        Some(at(2019, 10, 11)),
        Some(at(2019, 10, 31)),
        None,
        "id2",
        "test2",
    );
    assert_eq!(d.tasks[1].render_end_time, None);
}

#[test]
#[ignore = "port: the reference's test needs TZ=America/Los_Angeles; the port reads UTC only (PLAN section 5)"]
fn should_add_1_day_even_on_days_with_25_hours() {}

#[test]
fn should_automatically_add_one_day_to_all_end_dates() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\ninclusiveEndDates\ntest1:id1,2019-02-01,1d\ntest2:id2,2019-02-01,2019-02-03",
    );
    check(
        &d.tasks[0],
        Some(at(2019, 2, 1)),
        Some(at(2019, 2, 2)),
        None,
        "id1",
        "test1",
    );
    check(
        &d.tasks[1],
        Some(at(2019, 2, 1)),
        Some(at(2019, 2, 4)),
        None,
        "id2",
        "test2",
    );
    assert_eq!(
        d.tasks[1].render_end_time, None,
        "a fixed end has no render end"
    );
    assert!(d.tasks[1].manual_end_time);
}

#[test]
fn should_hide_today_marker() {
    assert_eq!(parsed("gantt\ntodayMarker off").today_marker, "off");
}

#[test]
fn should_style_today_marker() {
    let text = "gantt\ntodayMarker stoke:stroke-width:5px,stroke:#00f,opacity:0.5";
    assert_eq!(
        parsed(text).today_marker,
        "stoke:stroke-width:5px,stroke:#00f,opacity:0.5"
    );
}

#[test]
fn should_reject_dates_with_ridiculous_years() {
    let error = parse("gantt\ndateFormat YYYYMMDD\ntest1:id1,202304,1d").expect_err("out of range");
    assert_eq!(error.message, "Invalid date:202304");
}

#[test]
fn should_handle_seconds_only_format_with_valid_numeric_values_issue_5496() {
    let d = parsed("gantt\ndateFormat ss\nsection Network Request\nRTT:rtt, 0, 20");
    assert_eq!(d.tasks.len(), 1);
    assert_eq!(d.tasks[0].task, "RTT");
    assert_eq!(d.tasks[0].id, "rtt");
}

#[test]
fn should_handle_dates_with_year_typo_like_202_instead_of_2024_issue_5496() {
    let d = parsed(
        "gantt\ndateFormat YYYY-MM-DD\nsection Vacation\nLondon Trip 1:2024-12-01, 7d\nLondon Trip 2:202-12-01, 7d",
    );
    assert_eq!(d.tasks.len(), 2);
    assert_eq!(d.tasks[0].start_time, Some(at(2024, 12, 1)));
    // The fallback reads the year 202, as V8 does.
    assert_eq!(d.tasks[1].start_time, Some(-55_763_856_000_000));
}

#[test]
fn should_warn_about_unknown_ids_in_an_after_statement_issue_4121() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\ntask1:id1,2013-01-01,2d\ntask2:id2,after m1,1d");
    // The warning is not part of the model; the start falls back to today.
    assert_eq!(d.tasks[1].start_time, Some(NOW));
}

#[test]
fn should_warn_about_unknown_ids_in_an_until_statement() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\ntask1:id1,2013-01-01,until m1");
    assert_eq!(d.tasks[0].end_time, Some(NOW));
}

#[test]
fn should_not_warn_when_every_referenced_id_exists() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\ntask1:id1,2013-01-01,2d\ntask2:id2,after id1,1d");
    assert_eq!(d.tasks[1].start_time, Some(at(2013, 1, 3)));
}

#[test]
fn should_warn_when_a_milestone_is_declared_without_a_duration_issue_4121() {
    let d = parsed("gantt\ndateFormat YYYY-MM-DD\nM1:milestone, 2023-01-01, m1");
    assert_eq!(d.tasks[0].end_time, d.tasks[0].start_time);
}
