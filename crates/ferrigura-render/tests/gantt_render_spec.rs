//! The gantt renderer's output: the order of its elements, the widths of the labels, the rows of the
//! compact layout, the today marker and the excluded days, as `ganttRenderer.js` writes them. The
//! geometry against the reference is compared by `pnpm port:geometry gantt`; these tests pin the
//! structure the comparison relies on.

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::gantt::{GanttSection, render};
use ferrigura_syntax::gantt::{Options, parse_with};

/// 2014-01-01T00:00Z, the first day of the diagrams below.
const JAN_1_2014: i64 = 1_388_534_400_000;

/// Renders a diagram with the redux-color theme and the given configuration, at `now`.
fn draw(text: &str, now: i64, gantt: &[(&str, Value)]) -> String {
    let diagram = parse_with(text, Options { now }).expect("the diagram parses");
    let mut value = defaults();
    for (key, setting) in gantt {
        value
            .get_mut("gantt")
            .and_then(Value::as_object_mut)
            .expect("the gantt section")
            .insert((*key).to_owned(), setting.clone());
    }
    let config = typed(&value).expect("the configuration");
    let variables = Theme::ReduxColor.variables(None);
    let tokens = ThemeTokens::from_value(&variables);
    render(&Context {
        diagram: &diagram,
        section: &GanttSection {
            config: &config.gantt,
            now,
        },
        globals: &Globals::of(&config),
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: "chart",
    })
    .expect("the diagram renders")
}

/// The drawn elements of the chart: what follows the style element.
fn body(svg: &str) -> &str {
    let end = svg.find("</style>").expect("a style element") + "</style>".len();
    &svg[end..]
}

#[test]
fn the_empty_group_comes_first_and_the_title_last() {
    let svg = draw(
        "gantt\n  dateFormat YYYY-MM-DD\n  title Plan\n  section A\n  one :a1, 2014-01-01, 1d\n",
        JAN_1_2014,
        &[],
    );
    assert!(body(&svg).starts_with("<g></g><g"), "{svg}");
    assert!(
        body(&svg).contains("<text x=\"600\" y=\"25\" class=\"titleText\""),
        "{svg}"
    );
    assert!(svg.ends_with(">Plan</text></svg>"), "{svg}");
}

#[test]
fn a_label_is_measured_without_the_blanks_at_its_end() {
    // The reference measures `Completed task` followed by twelve blanks as 76.03125 px: the browser
    // does not lay out trailing blanks (`getBBox` of the text, `width-` in the class).
    let svg = draw(
        "gantt\n  dateFormat YYYY-MM-DD\n  section A\n  Completed task            :done, des1, 2014-01-01, 1d\n",
        JAN_1_2014,
        &[],
    );
    assert!(
        svg.contains("class=\"taskText taskText0  doneText0 width-76.03125\""),
        "{svg}"
    );
    assert!(svg.contains(">Completed task            </text>"), "{svg}");
}

#[test]
fn compact_rows_hold_the_tasks_that_overlap_in_separate_rows() {
    // `a1` and `a2` overlap, so the section needs two rows; `a3` fits in the first row again.
    let svg = draw(
        "gantt\n  dateFormat YYYY-MM-DD\n  section S\n  a :a1, 2014-01-01, 3d\n  b :a2, 2014-01-02, 1d\n  c :a3, 2014-01-05, 1d\n",
        JAN_1_2014,
        &[("displayMode", Value::from("compact"))],
    );
    // Two rows of 24 px after the top padding of 50 px, twice: 2 * 50 + 2 * 24 = 148.
    assert!(svg.contains("viewBox=\"0 0 1200 148\""), "{svg}");
}

#[test]
fn the_today_marker_is_at_now() {
    // The domain starts at `now`, so the marker is at the left padding of 75 px, and it runs from
    // the title's margin to 25 px above the bottom of the 124 px chart.
    let svg = draw(
        "gantt\n  dateFormat YYYY-MM-DD\n  section S\n  a :a1, 2014-01-01, 1d\n",
        JAN_1_2014,
        &[],
    );
    assert!(
        body(&svg).contains(
            "<g class=\"today\"><line x1=\"75\" x2=\"75\" y1=\"25\" y2=\"99\" class=\"today\""
        ),
        "{svg}"
    );
}

#[test]
fn a_run_of_excluded_days_that_ends_the_span_is_not_drawn() {
    // The manual end (Sunday) is not moved past the weekend, so the excluded run of Saturday and
    // Sunday is still open when the span ends. The reference does not push an open run
    // (`ganttRenderer.js:580-590`), so no excluded range is drawn.
    let svg = draw(
        "gantt\n  dateFormat YYYY-MM-DD\n  excludes weekends\n  section S\n  a :a1, 2014-01-02, 2014-01-05\n",
        JAN_1_2014,
        &[],
    );
    assert!(!body(&svg).contains("exclude-range"), "{svg}");
    let excluded_midweek = draw(
        "gantt\n  dateFormat YYYY-MM-DD\n  excludes weekends\n  section S\n  a :a1, 2014-01-02, 2014-01-06\n  b :b1, 2014-01-06, 1d\n",
        JAN_1_2014,
        &[],
    );
    assert_eq!(
        body(&excluded_midweek)
            .matches("class=\"exclude-range\"")
            .count(),
        1,
        "{excluded_midweek}"
    );
}

#[test]
fn a_milestone_label_carries_the_milestone_class() {
    let svg = draw(
        "gantt\n  dateFormat YYYY-MM-DD\n  section S\n  Launch :milestone, m1, 2014-01-02, 0d\n",
        JAN_1_2014,
        &[],
    );
    assert!(
        body(&svg).contains("class=\"taskTextOutsideRight taskTextOutside0  milestoneText"),
        "{svg}"
    );
}
