//! The event modeling renderer against the numbers of the reference (`docs-port/eventmodeling/PLAN.md`
//! section 6 and `NOTES.md` section 6). The numbers are the ones Chromium gives for
//! `e2e/diagrams/eventmodeling/pr-check/with-data-block-reference.mmd`, read with
//! `pnpm port:geometry eventmodeling`: the root's view box, the lanes, the boxes, and the lines of the
//! two box texts (the bold name and the code of the data).

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::theme::Theme;
use ferrigura_render::event_modeling::{EventModelingInput, render};
use ferrigura_render::theme::Themes;

/// The fixture `with-data-block-reference.mmd`, without its front matter.
const FIXTURE: &str = "eventmodeling

tf 01 ui CartScreen
tf 02 cmd AddItem { productId: 7 }
tf 03 evt ItemAdded [[ItemAddedData]]

data ItemAddedData
{
  productId: 7,
  quantity: 1
}
";

/// Renders the text with the redux colour theme and the default configuration.
fn draw(text: &str) -> String {
    let diagram = ferrigura_syntax::event_modeling::parse(text).expect("the diagram parses");
    let config = typed(&defaults()).expect("the configuration is typed");
    let variables = Theme::ReduxColor.variables(None);
    render(&EventModelingInput {
        diagram: &diagram,
        config: &config,
        themes: Themes::Single(&variables),
        id: "fx",
    })
    .expect("the diagram renders")
}

/// The value of attribute `name` of the element `which` (0-based) that starts with `<tag ` and
/// contains `marker` after it, as written. `marker` is the opening tag of the group the element is
/// in (for example `class="em-box"`), so `which` counts the groups.
fn attribute(svg: &str, marker: &str, which: usize, tag: &str, name: &str) -> String {
    let group = svg
        .match_indices(marker)
        .nth(which)
        .map(|(start, _)| start)
        .expect("the group is in the diagram");
    let rest = &svg[group..];
    let element = rest
        .find(&format!("<{tag} "))
        .map(|start| &rest[start..])
        .expect("the element is in the group");
    let element = &element[..element.find('>').expect("the element closes")];
    let needle = format!(" {name}=\"");
    let start = element
        .find(&needle)
        .unwrap_or_else(|| panic!("{tag} has no {name}: {element}"))
        + needle.len();
    let end = element[start..].find('"').expect("the value closes") + start;
    element[start..end].to_owned()
}

/// The attribute as a number, for the comparisons with the reference's numbers.
fn number(svg: &str, marker: &str, which: usize, tag: &str, name: &str) -> f64 {
    attribute(svg, marker, which, tag, name)
        .parse()
        .unwrap_or_else(|_| panic!("{name} is a number"))
}

/// The `tspan` lines of the label of the box `which`: each line's text and its position.
fn label_lines(svg: &str, which: usize) -> Vec<(String, f64, f64)> {
    let start = svg
        .match_indices("<g class=\"em-box\">")
        .nth(which)
        .map(|(start, _)| start)
        .expect("the box is in the diagram");
    let rest = &svg[start..];
    let end = rest.find("</switch>").expect("the label closes");
    let mut lines = Vec::new();
    let mut tail = &rest[..end];
    while let Some(open) = tail.find("<tspan ") {
        let after = &tail[open..];
        let close = after.find('>').expect("the tspan opens");
        let tag = &after[..close];
        let value = |name: &str| -> f64 {
            let needle = format!(" {name}=\"");
            let begin = tag.find(&needle).expect("the position is written") + needle.len();
            let finish = tag[begin..].find('"').expect("the value closes") + begin;
            tag[begin..finish]
                .parse()
                .expect("the position is a number")
        };
        let (x, y) = (value("x"), value("y"));
        let text_start = close + 1;
        let text_end = after[text_start..]
            .find("</tspan>")
            .expect("the line closes")
            + text_start;
        lines.push((after[text_start..text_end].to_owned(), x, y));
        tail = &after[text_end..];
    }
    lines
}

/// The reference's view box: the boxes and lanes, padded by 30 on each side.
#[test]
fn the_view_box_is_the_reference_s() {
    let svg = draw(FIXTURE);
    let view_box = attribute(&svg, "<svg", 0, "svg", "viewBox");
    assert_eq!(view_box, "-30 -30 953.6666870117188 490");
}

/// The lanes stand at 0, 140 and 280; the first two are 130 high, the third, which holds the tallest
/// box, 150 (`db.ts` 489-495, 523-536).
#[test]
fn the_lanes_are_stacked_with_a_gap_of_ten() {
    let svg = draw(FIXTURE);
    let tops: Vec<f64> = (0..3)
        .map(|lane| number(&svg, "<g class=\"em-swimlane\">", lane, "rect", "y"))
        .collect();
    let heights: Vec<f64> = (0..3)
        .map(|lane| number(&svg, "<g class=\"em-swimlane\">", lane, "rect", "height"))
        .collect();
    assert_eq!(tops, [0.0, 140.0, 280.0]);
    assert_eq!(heights, [130.0, 130.0, 150.0]);
}

/// The boxes, as the reference places them: the first at 250; a box in another lane than the box
/// before it starts 80 px left of that box's `r` less 90 plus 10 (`calculateX`), which is 80 px left
/// of its right edge plus 10.
#[test]
fn the_boxes_are_placed_as_the_reference_places_them() {
    let svg = draw(FIXTURE);
    let boxes: Vec<(f64, f64, f64, f64)> = (0..3)
        .map(|which| {
            let marker = "<g class=\"em-box\">";
            (
                number(&svg, marker, which, "rect", "x"),
                number(&svg, marker, which, "rect", "y"),
                number(&svg, marker, which, "rect", "width"),
                number(&svg, marker, which, "rect", "height"),
            )
        })
        .collect();
    assert_eq!(boxes[0], (250.0, 15.0, 186.0, 100.0));
    assert!((boxes[1].0 - 366.0).abs() < 0.05, "{:?}", boxes[1]);
    assert!((boxes[1].1 - 155.0).abs() < 0.05, "{:?}", boxes[1]);
    assert!((boxes[1].2 - 265.66663).abs() < 0.05, "{:?}", boxes[1]);
    assert!((boxes[2].0 - 561.66669).abs() < 0.05, "{:?}", boxes[2]);
    assert_eq!(boxes[2].3, 120.0);
}

/// The name of a box is bold and centred in the cell: `ItemAdded` starts at 678, and its baseline is
/// 13 px below the top of its line; the data starts at the cell's left edge, snapped to the layout
/// unit of 1/64 px (`label.rs`).
#[test]
fn a_name_is_centred_and_a_data_block_starts_at_the_cell_s_left() {
    let svg = draw(FIXTURE);
    let name = label_lines(&svg, 2);
    assert_eq!(name[0].0, "ItemAdded");
    assert!((name[0].1 - 678.0).abs() < 0.05, "{name:?}");
    assert!((name[0].2 - 342.0).abs() < 0.05, "{name:?}");

    let data = &name[1];
    assert!((data.1 - 571.65625).abs() < 0.05, "{name:?}");
    assert!((data.2 - 377.0).abs() < 0.05, "{name:?}");
}

/// The newline of the data block is drawn as a blank, as the reference's HTML lays it out
/// (`label::drawn`): the line keeps its non-breaking spaces and its one blank.
#[test]
fn a_newline_of_the_data_is_one_blank() {
    let svg = draw(FIXTURE);
    let data = &label_lines(&svg, 2)[1].0;
    assert_eq!(data, "productId:\u{a0}7, \u{a0}quantity:\u{a0}1");
}

/// The label is SVG text (decision 0004): no `foreignObject` is written, and the switch holds the
/// text of the box.
#[test]
fn the_box_text_is_svg_text_and_not_a_foreign_object() {
    let svg = draw(FIXTURE);
    assert!(!svg.contains("foreignObject"));
    assert_eq!(svg.matches("<switch>").count(), 3);
}
