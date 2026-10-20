//! The quadrant chart renderer's output for the reference's quirks (`docs-port/quadrant/NOTES.md`,
//! sections 2.3, 6.2 and 6.6): the groups in their order, the points at the scale's digits, a
//! coordinate the scale cannot read, the NaN fill of the redux themes, the border styles, an empty
//! chart, an entity placeholder and the accessible texts. The geometry against the reference is
//! compared by `pnpm port:geometry quadrantChart`; these tests pin the structure it relies on.

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::quadrant_chart::render;
use ferrigura_syntax::quadrant_chart::parse;

/// The complete chart of `should-render-a-complete-quadrant-chart`, without the theme directive.
const COMPLETE: &str = "quadrantChart
  title Reach and engagement of campaigns
  x-axis Low Reach --> High Reach
  y-axis Low Engagement --> High Engagement
  quadrant-1 We should expand
  quadrant-2 Need to promote
  quadrant-3 Re-evaluate
  quadrant-4 May be improved
  Campaign A: [0.3, 0.6]
  Campaign B: [0.45, 0.23]
  Campaign C: [0.57, 0.69]
";

/// Renders a chart with the given `quadrantChart` settings and the redux-color theme.
fn draw_with(text: &str, quadrant_chart: &[(&str, Value)]) -> String {
    let diagram = parse(text).expect("the diagram parses");
    let mut value = defaults();
    for (key, setting) in quadrant_chart {
        value
            .get_mut("quadrantChart")
            .and_then(Value::as_object_mut)
            .expect("the quadrantChart section")
            .insert((*key).to_owned(), setting.clone());
    }
    let config = typed(&value).expect("the configuration");
    let variables = Theme::ReduxColor.variables(None);
    let tokens = ThemeTokens::from_value(&variables);
    let globals = Globals::of(&config);
    render(&Context {
        diagram: &diagram,
        section: &config.quadrant_chart,
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: "chart",
    })
    .expect("the diagram renders")
}

fn draw(text: &str) -> String {
    draw_with(text, &[])
}

/// The tags of the elements with the given name, in document order, as their markup.
fn tags<'a>(svg: &'a str, name: &str) -> Vec<&'a str> {
    let open = format!("<{name}");
    svg.match_indices(&open)
        .filter_map(|(start, _)| {
            let rest = &svg[start..];
            let end = rest.find('>')?;
            Some(&rest[..=end])
        })
        .collect()
}

/// The class names of the groups, in document order.
fn group_classes(svg: &str) -> Vec<String> {
    tags(svg, "g")
        .iter()
        .filter_map(|tag| {
            let start = tag.find("class=\"")? + "class=\"".len();
            let end = tag[start..].find('"')?;
            Some(tag[start..start + end].to_owned())
        })
        .collect()
}

#[test]
fn the_groups_of_the_chart_come_in_the_reference_order() {
    let svg = draw(COMPLETE);
    let classes = group_classes(&svg);
    let main = classes
        .iter()
        .position(|class| class == "main")
        .expect("the main group");
    assert_eq!(
        &classes[main + 1..],
        [
            "quadrants",
            "quadrant",
            "quadrant",
            "quadrant",
            "quadrant",
            "border",
            "data-points",
            "data-point",
            "data-point",
            "data-point",
            "labels",
            "label",
            "label",
            "label",
            "label",
            "title"
        ]
        .map(String::from)
    );
}

#[test]
fn a_point_is_placed_with_the_scale_digits_of_the_reference() {
    let svg = draw(COMPLETE);
    // `Campaign C: [0.57, 0.69]` is drawn at the digits d3's scale gives (the fixture's SVG).
    assert!(
        svg.contains("cx=\"295.47999999999996\" cy=\"176.44\""),
        "{svg}"
    );
    assert!(
        svg.contains("translate(295.47999999999996, 181.44)"),
        "{svg}"
    );
}

#[test]
fn a_coordinate_the_scale_cannot_read_drops_its_attribute_and_writes_undefined() {
    // `0,5` reads as NaN: the circle has no `cx`, and the label's transform has `undefined`.
    let svg = draw("quadrantChart\n  Comma: [0,5, 0.5]\n");
    let circles = tags(&svg, "circle");
    assert_eq!(circles.len(), 1);
    assert!(!circles[0].contains("cx="), "{}", circles[0]);
    // `0.5` on the y axis of the bare chart (5 to 495) is 250.
    assert!(circles[0].contains("cy=\"250\""), "{}", circles[0]);
    assert!(svg.contains("transform=\"translate(undefined, "), "{svg}");
}

#[test]
fn a_y_coordinate_the_scale_cannot_read_writes_nan_in_the_label() {
    // `0a5` reads as NaN on the y axis: the label is at `NaN`, and `cy` is dropped.
    let svg = draw("quadrantChart\n  Letter: [0.5, 0a5]\n");
    assert!(svg.contains(", NaN) rotate(0)\""), "{svg}");
    let circle = tags(&svg, "circle")[0];
    assert!(!circle.contains("cy="), "{circle}");
}

#[test]
fn a_point_takes_the_redux_nan_fill_and_stroke() {
    // The redux themes give the point fill `hsl(240, 90%, NaN%)` (`theme-redux-color.js` 350-352).
    let svg = draw(COMPLETE);
    let circle = tags(&svg, "circle")[0];
    assert!(circle.contains("fill=\"hsl(240, 90%, NaN%)\""), "{circle}");
    assert!(
        circle.contains("stroke=\"hsl(240, 90%, NaN%)\""),
        "{circle}"
    );
    assert!(circle.contains("stroke-width=\"0px\""), "{circle}");
}

#[test]
fn the_borders_carry_their_stroke_as_the_browser_writes_it() {
    // The style of a border is a CSS declaration: `#181818` is `rgb(24, 24, 24)` and the width is
    // a number (the reference's fixture).
    let svg = draw(COMPLETE);
    let lines = tags(&svg, "line");
    assert_eq!(lines.len(), 6);
    assert!(
        lines[0].contains("style=\"stroke: rgb(24, 24, 24); stroke-width: 2;\""),
        "{}",
        lines[0]
    );
    assert!(
        lines[4].contains("style=\"stroke: rgb(24, 24, 24); stroke-width: 1;\""),
        "{}",
        lines[4]
    );
}

#[test]
fn a_bare_chart_writes_its_empty_texts_and_groups() {
    // `quadrantChart` alone: the four quadrant texts are empty and centred, and the point, label
    // and title groups are empty.
    let svg = draw("quadrantChart\n");
    let texts = tags(&svg, "text");
    assert_eq!(texts.len(), 4);
    assert!(
        texts
            .iter()
            .all(|tag| tag.contains("dominant-baseline=\"middle\""))
    );
    assert!(svg.contains("<g class=\"title\"/>"), "{svg}");
    assert!(svg.contains("<g class=\"labels\"/>"), "{svg}");
    assert!(svg.contains("<g class=\"data-points\"/>"), "{svg}");
}

#[test]
fn an_entity_placeholder_is_written_as_an_entity_and_the_font_covers_its_character() {
    // `#35;` is a placeholder in the model; the whole-SVG pass writes it as the entity (`&#35;`).
    let svg = draw("quadrantChart\n  quadrant-2 #35; hash\n");
    assert!(svg.contains(">&#35; hash</text>"), "{svg}");
    assert!(!svg.contains("ﬂ°°"), "{svg}");
    assert!(svg.contains("@font-face"), "{svg}");
}

#[test]
fn the_accessible_title_and_description_come_first_in_the_root() {
    let svg = draw("quadrantChart\n  accTitle: Campaigns\n  accDescr: The reach\n");
    assert!(
        svg.contains("aria-labelledby=\"chart-title-chart\""),
        "{svg}"
    );
    assert!(
        svg.contains("aria-describedby=\"chart-desc-chart\""),
        "{svg}"
    );
    let title = svg.find("<title ").expect("a title element");
    let desc = svg.find("<desc ").expect("a desc element");
    let style = svg.find("<style>").expect("a style element");
    assert!(title < desc && desc < style, "{svg}");
}

#[test]
fn a_chart_without_max_width_has_its_size_in_pixels() {
    let svg = draw_with(
        "quadrantChart\n",
        &[
            ("chartWidth", Value::from(serde_json::json!(320))),
            ("chartHeight", Value::from(serde_json::json!(240))),
            ("useMaxWidth", Value::from(serde_json::json!(false))),
        ],
    );
    assert!(
        svg.starts_with("<svg id=\"chart\" width=\"320\" xmlns=\"http://www.w3.org/2000/svg\" height=\"240\" viewBox=\"0 0 320 240\""),
        "{svg}"
    );
}

#[test]
fn a_text_outside_the_bundled_faces_still_renders() {
    // The text is drawn, and the font subset takes what the faces cover.
    let svg = draw("quadrantChart\n  中文点: [0.5, 0.5]\n");
    assert!(svg.contains("中文点"), "{svg}");
}

#[test]
fn the_style_sheet_writes_each_font_face_once() {
    // The faces come with the base sheet, before the theme's rules, as in the other diagrams.
    let svg = draw(COMPLETE);
    let start = svg.find("<style>").expect("a style sheet") + "<style>".len();
    let sheet = &svg[start..start + svg[start..].find("</style>").expect("its end")];
    let faces: Vec<&str> = sheet
        .match_indices("@font-face")
        .map(|(at, _)| {
            let rule = &sheet[at..];
            &rule[..=rule.find('}').expect("the rule's end")]
        })
        .collect();
    assert!(!faces.is_empty(), "{sheet}");
    let mut distinct = faces.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), faces.len(), "{sheet}");
    assert!(sheet.starts_with("@font-face"), "{sheet}");
}
