//! The kanban renderer against the numbers and markup of the reference (`docs-port/kanban/PLAN.md`
//! and `NOTES.md` 6). The numbers are the ones Chromium gives for the fixtures of
//! `e2e/diagrams/kanban` and for the item probes of `tools/port/geometry/probes/kanban`, read with
//! `pnpm port:geometry kanban`; the markup is the reference's, quirks included.

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::theme::Theme;
use ferrigura_model::value::Value;
use ferrigura_render::kanban::{KanbanInput, render};
use ferrigura_render::theme::Themes;

/// The first fixture of `e2e/diagrams/kanban/pr-check`: three sections, one duplicate id.
const FIXTURE_ONE: &str = "kanban
  id1[Todo]
  docs[Create Documentation]
  docs[Create Blog about the new diagram]
";

/// Probe `p01`: two items under `Todo`, one under `In progress`, and an empty `Done`.
const ITEMS: &str = "kanban
  Todo
    Write the spec
    Review the plan
  In progress
    Build the parser
  Done
";

/// Renders a kanban with the redux colour theme and the default configuration, with the given
/// settings of the `kanban` section.
fn draw(text: &str, id: &str, kanban: &[(&str, Value)]) -> String {
    let diagram = ferrigura_syntax::kanban::parse(text).expect("the kanban parses");
    let mut value = defaults();
    for (key, setting) in kanban {
        value
            .get_mut("kanban")
            .and_then(Value::as_object_mut)
            .expect("the defaults hold a kanban section")
            .insert((*key).to_owned(), setting.clone());
    }
    let config = typed(&value).expect("the configuration is typed");
    let variables = Theme::ReduxColor.variables(None);
    render(&KanbanInput {
        diagram: &diagram,
        config: &config,
        themes: Themes::Single(&variables),
        id,
    })
    .expect("the kanban renders")
}

/// The value of the attribute `name` of the first element that starts with `<tag `, as written.
fn attribute<'a>(svg: &'a str, tag: &str, name: &str) -> Option<&'a str> {
    let start = svg.find(&format!("<{tag} "))?;
    let element = &svg[start..svg[start..].find('>')? + start];
    let needle = format!(" {name}=\"");
    let value_start = element.find(&needle)? + needle.len();
    let value_end = element[value_start..].find('"')? + value_start;
    Some(&element[value_start..value_end])
}

/// The `transform` of the element with the id `id`, as written.
fn transform_of<'a>(svg: &'a str, id: &str) -> Option<&'a str> {
    let start = svg.find(&format!("id=\"{id}\""))?;
    let element = &svg[start..svg[start..].find('>')? + start];
    let needle = " transform=\"";
    let value_start = element.find(needle)? + needle.len();
    let value_end = element[value_start..].find('"')? + value_start;
    Some(&element[value_start..value_end])
}

/// The numbers of a `viewBox` or of a `translate(…)`, as they are written: the words are left out.
fn numbers(text: &str) -> Vec<f64> {
    text.split([' ', ',', '(', ')'])
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// Asserts that two lists of numbers agree to a hundredth. The reference's `getBBox` values are
/// float32, so the numbers it writes carry the rounding of that type.
fn assert_close(actual: &[f64], expected: &[f64]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{actual:?} against {expected:?}"
    );
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() < 0.01, "{actual:?} against {expected:?}");
    }
}

fn view_box(svg: &str) -> Vec<f64> {
    numbers(attribute(svg, "svg", "viewBox").expect("the root has a viewBox"))
}

#[test]
fn a_fixture_board_has_the_reference_viewbox_and_size() {
    // Chromium gives `90 -310 630 77.4` for this fixture: three sections of 200 px, with the gaps of
    // `padding / 2`, and rectangles 50 px tall above a title band of 25 px.
    let svg = draw(FIXTURE_ONE, "kanban", &[]);
    assert_close(&view_box(&svg), &[90.0, -310.0, 630.0, 77.4]);
    assert!(
        svg.starts_with("<svg id=\"kanban\" width=\"100%\" xmlns=\"http://www.w3.org/2000/svg\" style=\"max-width: 630px;\""),
        "the root element takes the box's width plus the padding"
    );
    assert!(svg.contains("aria-roledescription=\"kanban\""));
}

#[test]
fn the_sections_are_side_by_side_and_numbered_from_one() {
    let svg = draw(FIXTURE_ONE, "kanban", &[]);
    // The first rectangle is at x 100 (its centre at 200 less half the width), the next at 305.
    assert!(
        svg.contains(
            "<g class=\"cluster undefined section-1\" id=\"kanban-id1\" data-look=\"classic\">"
        ),
        "the first section's group"
    );
    assert!(
        svg.contains("<rect style=\"\" rx=\"5\" ry=\"5\" x=\"305\" y=\"-300\" width=\"200\""),
        "the second section's rectangle"
    );
    // The duplicate id gives the third section the same DOM id as the second.
    assert!(svg.contains("<g class=\"cluster undefined section-3\" id=\"kanban-docs\""));
}

#[test]
fn items_stack_below_the_title_band_at_the_reference_positions() {
    // Probe p01 (Chromium): the first item of `Todo` is 37 px tall and centred at -256.5, below the
    // 25 px title band; the next one is 5 px lower than its bottom edge.
    let svg = draw(ITEMS, "kanban", &[]);
    let place = |id: &str| numbers(transform_of(&svg, id).expect("the item is drawn"));
    assert_close(&place("kanban-Write the spec"), &[200.0, -256.5]);
    assert_close(&place("kanban-Review the plan"), &[200.0, -214.5]);
    assert_close(&place("kanban-Build the parser"), &[405.0, -256.5]);
    assert_close(&view_box(&svg), &[90.0, -310.0, 630.0, 134.0]);
    assert!(svg.contains("<g class=\"node undefined\" id=\"kanban-Write the spec\""));
    assert!(svg.contains(
        "<rect class=\"basic label-container __APA__\" style=\"\" rx=\"5\" ry=\"5\" x=\"-92.5\" y=\"-18.5\" width=\"185\" height=\"37\""
    ));
}

#[test]
fn a_wrapped_ticket_takes_the_item_box_below_the_rectangle() {
    // A three-line ticket makes the group's box taller than the rectangle, so the next item is placed
    // from that box (probe p04: Chromium gives -242.6 for the first item).
    let text = "kanban
  Todo
    Short title@{ ticket: 'A ticket name that is long enough to wrap onto more than one line in the card', assigned: 'Someone with a rather long assignee name that wraps too' }
    Medium title with more words to wrap@{ assigned: 'K.Sveidqvist' }
";
    let svg = draw(text, "kanban", &[]);
    let first = numbers(transform_of(&svg, "kanban-Short title").expect("the item is drawn"));
    assert_close(&first, &[200.0, -242.6]);
}

#[test]
fn an_item_with_an_icon_has_a_background_on_its_title() {
    // With an icon the reference gives the title a background, which makes the group 4 px taller:
    // the item is 41 px tall (probe p11).
    let text = "kanban
  Todo
    Item with icon
      ::icon(fa fa-book)
    Plain
";
    let svg = draw(text, "kanban", &[]);
    assert!(
        svg.contains("height=\"41\""),
        "the item with an icon is 41 px tall"
    );
    assert!(svg.contains("<rect class=\"background\" style=\"stroke: none\" x=\""));
    assert!(
        svg.contains("height=\"21\"></rect>"),
        "the background is the title's box plus 2 px each side"
    );
}

#[test]
fn a_priority_draws_a_bar_and_a_priority_without_a_colour_draws_none() {
    let text = "kanban
  Backlog
    High item@{ priority: 'High' }
    Medium item@{ priority: 'Medium' }
    Numeric item@{ priority: 1 }
";
    let svg = draw(text, "kanban", &[]);
    assert!(
        svg.contains("<line x1=\"-90.5\" y1=\"-16.5\" x2=\"-90.5\" y2=\"16.5\" stroke-width=\"4\" stroke=\"orange\""),
        "the high bar is orange"
    );
    // `Medium` and the number have a bar with no stroke colour (`colorFromPriority` gives none).
    assert_eq!(svg.matches("stroke-width=\"4\"></line>").count(), 2);
}

#[test]
fn a_ticket_links_its_label_when_the_configuration_gives_a_base_url() {
    let text = "kanban
  Todo
    Item with ticket@{ ticket: MC-2037 }
";
    let linked = draw(
        text,
        "kanban",
        &[(
            "ticketBaseUrl",
            Value::from("https://tickets.example.com/browse/#TICKET#"),
        )],
    );
    // The link is the first child after the rectangle, and it holds the ticket's label. The
    // reference's output has no `target`: its sanitiser drops it.
    assert!(
        linked.contains("<a class=\"kanban-ticket-link\" xlink:href=\"https://tickets.example.com/browse/MC-2037\"><g class=\"label\""),
        "the ticket's label is inside the link"
    );
    // The style sheet always has the rule of the link; the markup has the anchor only with a base URL.
    let plain = draw(text, "kanban", &[]);
    assert!(!plain.contains("<a class=\"kanban-ticket-link\""));
}

#[test]
fn an_empty_board_has_the_padding_as_its_box() {
    // `kanban` followed by a newline: no sections, so the box is empty and the viewBox is the padding.
    let svg = draw("kanban\n", "kanban", &[]);
    assert_close(&view_box(&svg), &[-10.0, -10.0, 20.0, 20.0]);
    assert!(svg.contains("style=\"max-width: 20px;\""));
}

#[test]
fn a_section_width_of_zero_is_the_default() {
    let zero = draw(
        FIXTURE_ONE,
        "kanban",
        &[("sectionWidth", Value::Number(0.0))],
    );
    let default = draw(FIXTURE_ONE, "kanban", &[]);
    assert_eq!(zero, default);
}

#[test]
fn the_style_sheet_scopes_every_rule_to_the_diagram() {
    let svg = draw(FIXTURE_ONE, "kanban", &[]);
    let start = svg.find("<style>").expect("a style element") + "<style>".len();
    let end = start
        + svg[start..]
            .find("</style>")
            .expect("the style element is closed");
    let style = &svg[start..end];
    assert!(style.contains("#kanban .section-1 rect"));
    assert!(style.contains("#kanban .node rect"));
    assert!(style.contains("#kanban .kanban-ticket-link"));
    assert!(style.contains("--kanbanFill0:"));
}
