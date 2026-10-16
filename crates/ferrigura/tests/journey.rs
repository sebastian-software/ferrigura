//! User journeys through the public entry point: the sizes of the standard chart, the legend, the
//! quirks the reference keeps, and the lines of a label. The geometry is checked against the
//! reference by `pnpm port:geometry journey`.

use ferrigura::render;
use ferrigura_model::value::Value;

fn svg(text: &str) -> String {
    render(text, &Value::empty_object()).expect("the journey renders")
}

/// The standard chart of the fixtures: two sections, five tasks, two actors (`should-render-a-user-journey-chart`).
const CHART: &str = "journey\n  title My working day\n  section Go to work\n  Make tea: 5: Me\n  Go upstairs: 3: Me\n  Do work: 1: Me, Cat\n  section Go home\n  Go downstairs: 5: Me\n  Sit down: 3: Me\n";

#[test]
fn should_size_the_standard_chart_as_the_reference_does() {
    // The reference's own root for this chart: a view box of 1300 by 540 with the title, and a
    // height of 565 (`NOTES.md` 7.3).
    let out = svg(CHART);
    assert!(out.contains("viewBox=\"0 -25 1300 540\""), "{out}");
    assert!(out.contains("height=\"565\""), "{out}");
    assert!(out.contains("style=\"max-width: 1300px;\""), "{out}");
    assert!(out.contains("role=\"graphics-document document\""), "{out}");
    assert!(out.contains("aria-roledescription=\"journey\""), "{out}");
}

#[test]
fn should_draw_the_title_in_bold_at_the_left_margin_and_the_activity_line_at_the_bottom() {
    let out = svg(CHART);
    assert!(
        out.contains("<text x=\"150\" font-size=\"4ex\" font-weight=\"bold\" y=\"25\" fill=\"\">My working day</text>"),
        "{out}"
    );
    assert!(
        out.contains(
            "<line x1=\"150\" y1=\"200\" x2=\"1146\" y2=\"200\" stroke-width=\"4\" stroke=\"black\""
        ),
        "{out}"
    );
}

#[test]
fn should_list_the_actors_in_the_order_of_their_names_with_array_indices_first() {
    // `10` and `2` are array indices: `Object.keys` lists them first, in numeric order. The
    // positions (and so the colours) follow the sorted names: `10` is 0, `2` is 1.
    let out = svg("journey\n  section S\n  Task: 5: Zed, 10, 2\n");
    // The first circle of each actor is its legend entry, which comes before the task circles.
    let two = out
        .find("class=\"actor-1\" fill=")
        .expect("the actor 2 has a circle");
    let ten = out
        .find("class=\"actor-0\" fill=")
        .expect("the actor 10 has a circle");
    let zed = out
        .find("class=\"actor-2\" fill=")
        .expect("the actor Zed has a circle");
    assert!(two < ten && ten < zed, "{out}");
    assert!(out.contains("<tspan x=\"50\">2</tspan>"), "{out}");
    assert!(out.contains("<tspan x=\"50\">10</tspan>"), "{out}");
}

#[test]
fn should_keep_the_prototype_name_out_of_the_legend_but_draw_its_circle() {
    // The reference's actors object cannot hold `__proto__` as an own key: its circles on the
    // tasks are drawn, with the colour and position of the actor, but it has no legend entry.
    let out = svg("journey\n  section S\n  Task: 5: __proto__, Ann\n");
    assert!(out.contains("<title>__proto__</title>"), "{out}");
    assert!(!out.contains("<tspan x=\"50\">__proto__</tspan>"), "{out}");
    assert!(out.contains("<tspan x=\"50\">Ann</tspan>"), "{out}");
}

#[test]
fn should_break_a_long_task_name_on_the_lines_the_browser_breaks_the_html_on() {
    // The reference's label of this name, in a 150 px cell (`NOTES.md` 7.6.1).
    let out = svg(
        "journey\n  section S\n  This is a very long task name that should wrap across several lines in the box: 5: Me\n",
    );
    for line in [
        "This is a very long task",
        "name that should wrap",
        "across several lines in",
        "the box",
    ] {
        assert!(out.contains(&format!(">{line}</tspan>")), "{line}: {out}");
    }
}

#[test]
fn should_draw_a_label_as_svg_text_in_a_switch_and_not_as_html() {
    let out = svg(CHART);
    assert!(!out.contains("foreignObject"), "{out}");
    assert!(out.contains("<switch><text class=\"label\""), "{out}");
}

#[test]
fn should_write_the_accessibility_title_and_description_first_and_refer_to_them() {
    let out = svg(
        "journey\n  accTitle: A title\n  accDescr: A description\n  section S\n  Task: 5: Me\n",
    );
    let title = out.find("<title id=").expect("an accessible title");
    let description = out.find("<desc id=").expect("an accessible description");
    let style = out.find("<style>").expect("a style");
    assert!(title < description && description < style, "{out}");
    assert!(out.contains("aria-labelledby=\""), "{out}");
    assert!(out.contains("aria-describedby=\""), "{out}");
}
