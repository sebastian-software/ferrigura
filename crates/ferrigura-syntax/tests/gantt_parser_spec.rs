//! Port of `packages/mermaid/src/diagrams/gantt/parser/gantt.spec.js`. Test names follow the spec's
//! titles. The reference's spies on the DB become outcomes of the model: the `clickable` class,
//! the link, the today marker and the excludes. The clock is fixed to 1010-10-10T00:00Z, the clock
//! of the differential run (`docs-port/gantt/PLAN.md`, section 5).

use ferrigura_model::gantt::{GanttDiagram, GanttTask};
use ferrigura_syntax::gantt::{Options, parse_with};

const NOW: i64 = -30_270_326_400_000;
const JAN_1_2014: i64 = 1_388_534_400_000;
const JAN_4_2014: i64 = 1_388_793_600_000;
const JAN_20_2024: i64 = 1_705_708_800_000;
const JAN_27_2024: i64 = 1_706_313_600_000;
const JAN_28_2024: i64 = 1_706_400_000_000;
const JAN_30_2024: i64 = 1_706_572_800_000;

fn parse(text: &str) -> Result<GanttDiagram, ferrigura_syntax::ParseError> {
    parse_with(text, Options { now: NOW })
}

fn parsed(text: &str) -> GanttDiagram {
    parse(text).unwrap_or_else(|error| panic!("the diagram should parse: {error}"))
}

fn task<'a>(diagram: &'a GanttDiagram, id: &str) -> &'a GanttTask {
    diagram
        .tasks
        .iter()
        .find(|task| task.id == id)
        .unwrap_or_else(|| panic!("no task {id}"))
}

#[test]
fn should_handle_a_date_format_definition() {
    assert!(parse("gantt\ndateFormat yyyy-mm-dd").is_ok());
}

#[test]
fn should_handle_a_inclusive_end_date_definition() {
    let diagram = parsed("gantt\ndateFormat yyyy-mm-dd\ninclusiveEndDates");
    assert!(diagram.inclusive_end_dates);
}

#[test]
fn should_enable_the_top_axis_with_the_topaxis_keyword() {
    let diagram = parsed("gantt\ndateFormat yyyy-mm-dd\ntopAxis");
    assert!(diagram.top_axis);
}

#[test]
fn should_handle_a_title_definition() {
    let plain = "gantt\ndateFormat yyyy-mm-dd\ntitle Adding gantt diagram functionality to mermaid";
    let semi = "gantt\ndateFormat yyyy-mm-dd\ntitle ;Gantt diagram titles support semicolons";
    let hash = "gantt\ndateFormat yyyy-mm-dd\ntitle #Gantt diagram titles support hashtags";
    assert!(parse(plain).is_ok());
    assert!(parse(semi).is_ok());
    assert!(parse(hash).is_ok());
}

#[test]
fn should_handle_an_excludes_definition() {
    let text = "gantt\ndateFormat yyyy-mm-dd\ntitle Adding gantt diagram functionality to mermaid\nexcludes weekdays 2019-02-01";
    assert!(parse(text).is_ok());
}

#[test]
fn should_concatenate_tokens_across_multiple_excludes_lines_issue_6270() {
    let text = [
        "gantt",
        "dateFormat DD-MM-YYYY",
        "excludes weekends",
        "%% week 7 is winter break",
        "excludes 10-02-2025 11-02-2025 12-02-2025",
        "%% workers holiday 1 maj",
        "excludes 01-05-2025",
    ]
    .join("\n");
    let diagram = parsed(&text);
    assert_eq!(
        diagram.excludes,
        [
            "weekends",
            "10-02-2025",
            "11-02-2025",
            "12-02-2025",
            "01-05-2025"
        ]
    );
}

#[test]
fn should_handle_a_today_marker_definition() {
    let text = "gantt\ndateFormat yyyy-mm-dd\ntitle Adding gantt diagram functionality to mermaid\nexcludes weekdays 2019-02-01\ntodayMarker off";
    let diagram = parsed(text);
    assert_eq!(diagram.today_marker, "off");
}

#[test]
fn should_handle_a_section_definition() {
    let base = "gantt\ndateFormat yyyy-mm-dd\ntitle Adding gantt diagram functionality to mermaid\nexcludes weekdays 2019-02-01\n";
    assert!(parse(&format!("{base}section Documentation")).is_ok());
    assert!(parse(&format!("{base}section ;Documentation")).is_ok());
    assert!(parse(&format!("{base}section #Documentation")).is_ok());
}

#[test]
fn should_handle_multiline_section_titles_with_different_line_breaks() {
    let text = "gantt\ndateFormat yyyy-mm-dd\ntitle Adding gantt diagram functionality to mermaid\nexcludes weekdays 2019-02-01\nsection Line1<br>Line2<br/>Line3</br />Line4<br\t/>Line5";
    assert!(parse(text).is_ok());
}

#[test]
fn should_handle_a_task_definition() {
    let base = "gantt\ndateFormat YYYY-MM-DD\ntitle Adding gantt diagram functionality to mermaid\nsection Documentation\n";
    for line in [
        "Design jison grammar:des1, 2014-01-01, 2014-01-04",
        ";Design jison grammar:des1, 2014-01-01, 2014-01-04",
        "#Design jison grammar:des1, 2014-01-01, 2014-01-04",
    ] {
        assert!(parse(&format!("{base}{line}")).is_ok());
    }
    let diagram = parsed(&format!(
        "{base}Design jison grammar:des1, 2014-01-01, 2014-01-04"
    ));
    let first = &diagram.tasks[0];
    assert_eq!(first.start_time, Some(JAN_1_2014));
    assert_eq!(first.end_time, Some(JAN_4_2014));
    assert_eq!(first.id, "des1");
    assert_eq!(first.task, "Design jison grammar");
}

#[test]
fn should_handle_a_task_with_start_end_time_relative_to_other_tasks() {
    let text = "gantt\ndateFormat YYYY-MM-DD\ntitle Adding gantt diagram functionality to mermaid\nsection Documentation\ntask A: a, 2024-01-27, 2024-01-28\ntask B: b, after a, 2024-01-30\ntask C: c, 2024-01-20, until a\ntask D: d, after c, until b";
    let diagram = parsed(text);
    let a = &diagram.tasks[0];
    assert_eq!(
        (a.start_time, a.end_time),
        (Some(JAN_27_2024), Some(JAN_28_2024))
    );
    assert_eq!((a.id.as_str(), a.task.as_str()), ("a", "task A"));
    let b = &diagram.tasks[1];
    assert_eq!(
        (b.start_time, b.end_time),
        (Some(JAN_28_2024), Some(JAN_30_2024))
    );
    assert_eq!((b.id.as_str(), b.task.as_str()), ("b", "task B"));
    let c = &diagram.tasks[2];
    assert_eq!(
        (c.start_time, c.end_time),
        (Some(JAN_20_2024), Some(JAN_27_2024))
    );
    assert_eq!((c.id.as_str(), c.task.as_str()), ("c", "task C"));
    let d = &diagram.tasks[3];
    assert_eq!(
        (d.start_time, d.end_time),
        (Some(JAN_27_2024), Some(JAN_28_2024))
    );
    assert_eq!((d.id.as_str(), d.task.as_str()), ("d", "task D"));
}

#[test]
fn should_handle_a_task_with_tags_milestone() {
    assert_task_tags("milestone", true, false, false, false);
}

#[test]
fn should_handle_a_task_with_tags_done() {
    assert_task_tags("done", false, true, false, false);
}

#[test]
fn should_handle_a_task_with_tags_crit() {
    assert_task_tags("crit", false, false, true, false);
}

#[test]
fn should_handle_a_task_with_tags_active() {
    assert_task_tags("active", false, false, false, true);
}

#[test]
fn should_handle_a_task_with_tags_crit_milestone_done() {
    assert_task_tags("crit,milestone,done", true, true, true, false);
}

/// The tags of a task: `(milestone, done, crit, active)`.
fn assert_task_tags(tags: &str, milestone: bool, done: bool, crit: bool, active: bool) {
    let text = format!(
        "gantt\ndateFormat YYYY-MM-DD\ntitle Adding gantt diagram functionality to mermaid\nsection Documentation\ntest task:{tags}, 2014-01-01, 2014-01-04"
    );
    let diagram = parsed(&text);
    let first = &diagram.tasks[0];
    assert_eq!(first.milestone, milestone, "milestone of {tags}");
    assert_eq!(first.done, done, "done of {tags}");
    assert_eq!(first.crit, crit, "crit of {tags}");
    assert_eq!(first.active, active, "active of {tags}");
}

const CALLBACK_HEADER: &str = "gantt\ndateFormat  YYYY-MM-DD\nsection Clickable\nVisit mermaidjs           :active, cl1, 2014-01-07, 3d\nCalling a callback        :cl2, after cl1, 3d\n\nclick cl1 href \"https://mermaidjs.github.io/\"\n";

#[test]
fn should_parse_callback_specifier_with_no_args() {
    let diagram = parsed(&format!(
        "{CALLBACK_HEADER}click cl2 call ganttTestClick()\n"
    ));
    assert_eq!(task(&diagram, "cl2").classes, ["clickable"]);
    assert_eq!(
        diagram.links.get("cl1").map(String::as_str),
        Some("https://mermaidjs.github.io/")
    );
}

#[test]
fn should_parse_callback_specifier_with_arbitrary_number_of_args() {
    let diagram = parsed(&format!(
        "{CALLBACK_HEADER}click cl2 call ganttTestClick(\"test0\", test1, test2)\n"
    ));
    assert_eq!(task(&diagram, "cl2").classes, ["clickable"]);
}

#[test]
fn should_allow_for_a_accessibility_title_and_description_accdescr() {
    let text = "gantt\n   accTitle: Gantt Diagram\n   accDescr: Tasks for Q4\n   dateFormat  YYYY-MM-DD\n   section Section\n   A task :a1, 2014-01-01, 30d\n";
    let diagram = parsed(text);
    assert_eq!(diagram.acc_title, "Gantt Diagram");
    assert_eq!(diagram.acc_descr, "Tasks for Q4");
}

#[test]
fn should_allow_for_a_accessibility_title_and_multiline_description_accdescr() {
    let text = "gantt\n   accTitle: Gantt Diagram\n   accDescr {\n     Tasks for Q4 row1\nrow2\n   }\n   dateFormat  YYYY-MM-DD\n   section Section\n   A task :a1, 2014-01-01, 30d\n";
    let diagram = parsed(text);
    assert_eq!(diagram.acc_title, "Gantt Diagram");
    assert_eq!(diagram.acc_descr, "Tasks for Q4 row1\nrow2");
}

#[test]
fn should_allow_for_setting_the_starting_weekday_to_monday() {
    assert_eq!(parsed("gantt\nweekday monday").weekday, "monday");
}

#[test]
fn should_allow_for_setting_the_starting_weekday_to_tuesday() {
    assert_eq!(parsed("gantt\nweekday tuesday").weekday, "tuesday");
}

#[test]
fn should_allow_for_setting_the_starting_weekday_to_wednesday() {
    assert_eq!(parsed("gantt\nweekday wednesday").weekday, "wednesday");
}

#[test]
fn should_allow_for_setting_the_starting_weekday_to_thursday() {
    assert_eq!(parsed("gantt\nweekday thursday").weekday, "thursday");
}

#[test]
fn should_allow_for_setting_the_starting_weekday_to_friday() {
    assert_eq!(parsed("gantt\nweekday friday").weekday, "friday");
}

#[test]
fn should_allow_for_setting_the_starting_weekday_to_saturday() {
    assert_eq!(parsed("gantt\nweekday saturday").weekday, "saturday");
}

#[test]
fn should_allow_for_setting_the_starting_weekday_to_sunday() {
    assert_eq!(parsed("gantt\nweekday sunday").weekday, "sunday");
}

#[test]
fn should_allow_for_a_link_to_proto_id() {
    let text = "gantt\n    dateFormat YYYY-MM-DD\n    section Section\n    A task :__proto__, 2024-10-01, 3d\n    click __proto__ href \"https://mermaid.js.org/\"\n    ";
    assert!(parse(text).is_ok());
}

#[test]
fn should_allow_for_a_link_to_constructor_id() {
    let text = "gantt\n    dateFormat YYYY-MM-DD\n    section Section\n    A task :constructor, 2024-10-01, 3d\n    click constructor href \"https://mermaid.js.org/\"\n    ";
    assert!(parse(text).is_ok());
}
