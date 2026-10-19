//! The wardley renderer's output for the reference's rules and quirks (`docs-port/wardley/NOTES.md`,
//! section 6, and the probes of section 2.5 that the drawing shows). The geometry against the
//! reference is compared by `pnpm port:geometry wardley`; these tests pin the structure and the
//! arithmetic the comparison relies on, each with the rule of the reference it reproduces.
//!
//! The expected numbers are the reference's arithmetic on the default configuration: a canvas of
//! 900 by 600 px, a margin of 48 px, a node radius of 6 px, so that the plot is 804 by 504 px. A
//! percentage `p` of the evolution is `48 + p / 100 * 804` and of the visibility `552 - p / 100 * 504`.

use std::collections::BTreeMap;

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::wardley::render;
use ferrigura_syntax::wardley::parse;
use ferrigura_text::{FontFamily, FontStyle, FontWeight, Run, face, layout_width, measure_line};

/// The settings of a render: the `wardley-beta` section, and the group of the theme variables.
#[derive(Default)]
struct Setup {
    section: Vec<(&'static str, Value)>,
    group: Option<Value>,
    dark: bool,
}

/// Renders the text with the redux-color theme and the default configuration.
fn draw(text: &str) -> String {
    draw_with(text, &Setup::default())
}

/// Renders the text with the given settings.
fn draw_with(text: &str, setup: &Setup) -> String {
    let diagram = parse(text).expect("the diagram parses");
    let mut value = defaults();
    for (key, setting) in &setup.section {
        value
            .get_mut("wardley-beta")
            .and_then(Value::as_object_mut)
            .expect("the wardley-beta section")
            .insert((*key).to_owned(), setting.clone());
    }
    let config = typed(&value).expect("the configuration");
    let mut variables = if setup.dark {
        Theme::ReduxDarkColor.variables(None)
    } else {
        Theme::ReduxColor.variables(None)
    };
    if let Some(group) = &setup.group {
        variables
            .as_object_mut()
            .expect("the variables are an object")
            .insert("wardley".to_owned(), group.clone());
    }
    let tokens = ThemeTokens::from_value(&variables);
    let globals = Globals::of(&config);
    render(&Context {
        diagram: &diagram,
        section: &config.wardley_beta,
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: "chart",
    })
    .expect("the map renders")
}

/// The variables a test sets in the `wardley` group.
fn group(pairs: &[(&str, Value)]) -> Value {
    let map: BTreeMap<String, Value> = pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect();
    Value::Object(map)
}

/// The elements `<name ...>` of the markup, each with its start tag and its inner markup (empty for an
/// element that is written with `/>`). Nested elements of the same name are listed too.
fn elements<'a>(markup: &'a str, name: &str) -> Vec<(&'a str, &'a str)> {
    let open = format!("<{name} ");
    let close = format!("</{name}>");
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(relative) = markup[from..].find(&open) {
        let start = from + relative;
        let end = start + markup[start..].find('>').expect("a start tag ends");
        let tag = &markup[start..=end];
        from = end + 1;
        if tag.ends_with("/>") {
            found.push((tag, ""));
            continue;
        }
        // The matching close tag: the closes that come after the nested starts of the same name.
        let mut depth = 1;
        let mut scan = end + 1;
        let body_end = loop {
            let next_close = scan + markup[scan..].find(&close).expect("the element is closed");
            let next_open = markup[scan..]
                .find(&open)
                .map(|relative| scan + relative)
                .filter(|open_at| *open_at < next_close);
            match next_open {
                Some(open_at) => {
                    let nested_end =
                        open_at + markup[open_at..].find('>').expect("a start tag ends");
                    if !markup[open_at..=nested_end].ends_with("/>") {
                        depth += 1;
                    }
                    scan = nested_end + 1;
                }
                None => {
                    depth -= 1;
                    if depth == 0 {
                        break next_close;
                    }
                    scan = next_close + close.len();
                }
            }
        };
        found.push((tag, &markup[end + 1..body_end]));
    }
    found
}

/// The elements of the given name whose class is the given class.
fn with_class<'a>(markup: &'a str, name: &str, class: &str) -> Vec<(&'a str, &'a str)> {
    elements(markup, name)
        .into_iter()
        .filter(|(tag, _)| attr(tag, "class") == Some(class))
        .collect()
}

/// The elements of the given name that have no class.
fn plain<'a>(markup: &'a str, name: &str) -> Vec<(&'a str, &'a str)> {
    elements(markup, name)
        .into_iter()
        .filter(|(tag, _)| attr(tag, "class").is_none())
        .collect()
}

/// The value of an attribute of a start tag.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

/// The value of a numeric attribute.
fn number(tag: &str, name: &str) -> f64 {
    attr(tag, name)
        .unwrap_or_else(|| panic!("{name} in {tag}"))
        .parse()
        .unwrap_or_else(|_| panic!("{name} of {tag} is a number"))
}

/// Whether two numbers are the same within the rounding of the arithmetic.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// The position of the first occurrence of the text in the markup.
fn at(markup: &str, needle: &str) -> usize {
    markup
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} is in the map"))
}

/// The stylesheet of the output: the text of its `style` element.
fn style_of(markup: &str) -> String {
    let Some(start) = markup.find("<style>") else {
        return String::new();
    };
    let body = &markup[start + "<style>".len()..];
    body[..body.find("</style>").expect("the style is closed")].to_owned()
}

/// A single component on the map, with its default position: evolution 50 and visibility 50, which is
/// x 450 and y 300.
const ONE: &str = "wardley-beta\ncomponent A [0.5, 0.5]\n";

/// A pipeline of a parent at (450, 300) with two components at evolution 20 and 60, which both take
/// the parent's visibility (`[0.2]` and `[0.6]`).
const PIPELINE: &str = "wardley-beta\ncomponent Parent [0.5, 0.5]\npipeline Parent {\n  component Left [0.2]\n  component Right [0.6]\n}\n";

/// The reference's stylesheet is not written: the output carries the bundled face and no rule of the
/// `styles.ts` (NOTES 6.9). The `@font-face` rules are what the face needs to be drawn.
#[test]
fn the_stylesheet_of_the_reference_is_not_written() {
    let markup = draw(ONE);
    let style = style_of(&markup);
    assert!(style.contains("@font-face"), "{style}");
    assert!(!style.contains(".wardley-"), "{style}");
    assert!(!style.contains("#chart"), "{style}");
}

/// The groups of the map come in the reference's order (NOTES 6.3), with the three markers last.
#[test]
fn the_groups_of_the_map_are_in_the_reference_order() {
    let text = "wardley-beta\ntitle Map\ncomponent A [0.5, 0.5] (build)\ncomponent B [0.4, 0.2] (buy)\n\
                A -> B\nevolve A 0.8\naccelerator \"Go\" [0.2, 0.2]\ndeaccelerator \"Stop\" [0.3, 0.3]\n\
                note \"n\" [0.1, 0.1]\nannotations [0.1, 0.9]\nannotation 1,[0.5, 0.5] \"x\"\n";
    let mut setup = Setup::default();
    setup.section.push(("showGrid", Value::Bool(true)));
    let markup = draw_with(text, &setup);
    let order = [
        "class=\"wardley-background\"",
        "class=\"wardley-title\"",
        "class=\"wardley-axes\"",
        "class=\"wardley-stages\"",
        "class=\"wardley-grid\"",
        "class=\"wardley-links\"",
        "class=\"wardley-trends\"",
        "class=\"wardley-nodes\"",
        "class=\"wardley-annotations\"",
        "class=\"wardley-notes\"",
        "class=\"wardley-accelerators\"",
        "class=\"wardley-deaccelerators\"",
        "<defs>",
    ];
    let positions: Vec<usize> = order.iter().map(|needle| at(&markup, needle)).collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "{order:?} {positions:?}"
    );
}

/// The links, trends and nodes groups are written even when empty, and the groups that need a
/// statement are not written without one (NOTES 6.3).
#[test]
fn empty_groups_are_written_and_the_optional_ones_are_not() {
    let markup = draw(ONE);
    for class in [
        "wardley-links",
        "wardley-trends",
        "wardley-nodes",
        "wardley-axes",
        "wardley-stages",
    ] {
        assert!(markup.contains(&format!("class=\"{class}\"")), "{class}");
    }
    for class in [
        "wardley-title",
        "wardley-pipelines",
        "wardley-pipeline-links",
        "wardley-grid",
        "wardley-annotations",
        "wardley-notes",
        "wardley-accelerators",
        "wardley-deaccelerators",
    ] {
        assert!(!markup.contains(&format!("class=\"{class}\"")), "{class}");
    }
    assert!(markup.contains("<g class=\"wardley-trends\"/>"), "{markup}");
}

/// The stage labels are at the default stages' centres, and the stage dividers are black, with the
/// dashes and the opacity of the reference (NOTES 6.4).
#[test]
fn default_stages_are_of_equal_width_with_black_dividers() {
    let markup = draw(ONE);
    let labels = with_class(&markup, "text", "wardley-stage-label");
    let xs: Vec<f64> = labels.iter().map(|(tag, _)| number(tag, "x")).collect();
    assert_eq!(xs.len(), 4);
    for (x, expected) in xs.iter().zip([148.5, 349.5, 550.5, 751.5]) {
        assert!(close(*x, expected), "{x} {expected}");
    }
    for (tag, _) in &labels {
        assert_eq!(
            attr(tag, "font-size"),
            Some("10"),
            "the stage label is the axis size less two"
        );
    }
    let dividers = plain(&markup, "line");
    let dividers: Vec<_> = dividers
        .into_iter()
        .filter(|(tag, _)| attr(tag, "stroke-dasharray") == Some("5 5"))
        .collect();
    assert_eq!(dividers.len(), 3);
    for (tag, _) in dividers {
        assert_eq!(attr(tag, "stroke"), Some("#000"));
        assert_eq!(attr(tag, "opacity"), Some("0.8"));
    }
}

/// The dividers keep black whatever the theme says, while the axes take the theme's axis colour
/// (NOTES 6.4: a literal `#000`).
#[test]
fn the_stage_dividers_stay_black_under_a_theme_group() {
    let setup = Setup {
        group: Some(group(&[(
            "axisColor",
            Value::from(serde_json::json!("#ff0000")),
        )])),
        ..Setup::default()
    };
    let markup = draw_with(
        "wardley-beta\ncomponent A [0.5, 0.5]\nevolution A -> B -> C\n",
        &setup,
    );
    let axes = with_class(&markup, "g", "wardley-axes");
    assert_eq!(axes.len(), 1);
    assert!(
        markup.contains("stroke=\"#ff0000\""),
        "the axes take the axis colour"
    );
    for (tag, _) in elements(&markup, "line") {
        if attr(tag, "stroke-dasharray") == Some("5 5") {
            assert_eq!(attr(tag, "stroke"), Some("#000"));
        }
    }
}

/// A pipeline's parent moves to the top of the box before the trends are drawn, so the trend starts
/// at the moved position; its end is the statement's target at the node's own visibility (NOTES
/// 6.2 and 8.1, `pipeline-trend`).
#[test]
fn the_pipeline_parent_moves_and_its_trend_starts_at_the_moved_position() {
    let markup = draw(&format!("{PIPELINE}evolve Parent 0.8\n"));
    let boxes = with_class(&markup, "rect", "wardley-pipeline-box");
    assert_eq!(boxes.len(), 1);
    let (box_tag, _) = boxes[0];
    assert!(
        close(number(box_tag, "x"), 193.8),
        "the box is 15 px out at each side"
    );
    assert!(close(number(box_tag, "width"), 351.6));
    assert!(close(number(box_tag, "y"), 288.0));
    assert!(close(number(box_tag, "height"), 24.0));

    // The parent's square is centred at the middle of the components, 1.6 px above the box's top.
    let squares: Vec<_> = elements(&markup, "rect")
        .into_iter()
        .filter(|(tag, _)| attr(tag, "width") == Some("9.600000000000001"))
        .collect();
    assert_eq!(squares.len(), 1);
    let (square, _) = squares[0];
    assert!(close(number(square, "x") + 4.8, 369.6), "{square}");
    assert!(close(number(square, "y") + 4.8, 286.4), "{square}");

    let trends = with_class(&markup, "line", "wardley-trend");
    assert_eq!(trends.len(), 1);
    let (trend, _) = trends[0];
    assert!(close(number(trend, "x1"), 369.6), "{trend}");
    assert!(close(number(trend, "y1"), 286.4), "{trend}");
    let target_x = 48.0 + 0.8 * 804.0;
    let (dx, dy): (f64, f64) = (target_x - 369.6, 300.0 - 286.4);
    let distance = (dx * dx + dy * dy).sqrt();
    assert!(
        close(number(trend, "x2"), target_x - dx / distance * 8.0),
        "{trend}"
    );
    assert!(
        close(number(trend, "y2"), 300.0 - dy / distance * 8.0),
        "{trend}"
    );
    assert_eq!(attr(trend, "marker-end"), Some("url(#arrow-chart)"));
}

/// The components of a pipeline are joined in the order of their evolution, whatever order the text
/// gives them (NOTES 2.5 and 6.2).
#[test]
fn pipeline_components_are_joined_in_the_order_of_their_evolution() {
    let text = "wardley-beta\ncomponent Parent [0.5, 0.5]\npipeline Parent {\n  component Right [0.6]\n  component Left [0.2]\n}\n";
    let markup = draw(text);
    let links = with_class(&markup, "line", "wardley-pipeline-evolution-link");
    assert_eq!(links.len(), 1);
    let (tag, _) = links[0];
    assert!(close(number(tag, "x1"), 208.8), "{tag}");
    assert!(close(number(tag, "x2"), 530.4), "{tag}");
    assert_eq!(attr(tag, "stroke-dasharray"), Some("4 4"));
}

/// A link from a pipeline's component to the pipeline's parent is not drawn (`wardleyRenderer.ts`
/// 396-401), while the other links are.
#[test]
fn a_link_from_a_pipeline_component_to_its_parent_is_not_drawn() {
    let markup = draw(&format!("{PIPELINE}Left -> Parent\nParent -> Right\n"));
    assert_eq!(with_class(&markup, "line", "wardley-link").len(), 1);
}

/// A link to a name that no node has is not drawn (`wardleyRenderer.ts` 390-393).
#[test]
fn a_link_to_a_name_without_a_node_is_not_drawn() {
    let markup = draw(&format!("{ONE}A -> Nowhere\n"));
    assert!(with_class(&markup, "line", "wardley-link").is_empty());
}

/// A link whose two ends are at one point divides by zero, and the reference writes `NaN` (NOTES
/// 6.4, `nan-links`).
#[test]
fn a_link_between_coincident_ends_writes_nan() {
    let markup = draw("wardley-beta\ncomponent A [0.5, 0.5]\ncomponent B [0.5, 0.5]\nA -> B\n");
    let links = with_class(&markup, "line", "wardley-link");
    assert_eq!(links.len(), 1);
    assert_eq!(attr(links[0].0, "x1"), Some("NaN"));
    assert_eq!(attr(links[0].0, "y2"), Some("NaN"));
}

/// A trend into the node's own position stays finite, since the reference falls back to the target
/// at zero length (NOTES 6.4, `trend-into-same-x`).
#[test]
fn a_trend_into_its_own_position_stays_finite() {
    let markup = draw("wardley-beta\ncomponent A [0.5, 0.5]\nevolve A 0.5\n");
    let trends = with_class(&markup, "line", "wardley-trend");
    assert_eq!(trends.len(), 1);
    assert_eq!(attr(trends[0].0, "x2"), Some("450"));
    assert_eq!(attr(trends[0].0, "y2"), Some("300"));
    assert!(!markup.contains("NaN"));
}

/// A label on a link that runs right to left is turned over: its angle is taken as the reference
/// takes it, and 180 degrees is 360 (NOTES 6.2, `wardleyRenderer.ts` 523-527).
#[test]
fn a_label_that_would_read_upside_down_is_turned_over() {
    let markup =
        draw("wardley-beta\ncomponent A [0.5, 0.8]\ncomponent B [0.5, 0.2]\nA +'lbl'> B\n");
    let labels = with_class(&markup, "text", "wardley-link-label");
    assert_eq!(labels.len(), 1);
    assert_eq!(
        attr(labels[0].0, "transform").map(|t| t.starts_with("rotate(360 ")),
        Some(true)
    );
}

/// A flow draws its arrow at the end or the start of the line, and a plain link has none (NOTES
/// 6.4, `marker-end` and `marker-start`).
#[test]
fn the_markers_of_a_link_follow_its_flow() {
    let end = "url(#link-arrow-end-chart)";
    let start = "url(#link-arrow-start-chart)";
    let cases = [
        ("+>", Some(end), None),
        ("+<", None, Some(start)),
        ("+<>", Some(end), Some(start)),
    ];
    for (arrow, marker_end, marker_start) in cases {
        let markup = draw(&format!("{ONE}component B [0.5, 0.2]\nA {arrow} B\n"));
        let links = with_class(&markup, "line", "wardley-link");
        assert_eq!(links.len(), 1, "{arrow}");
        assert_eq!(attr(links[0].0, "marker-end"), marker_end, "{arrow}");
        assert_eq!(attr(links[0].0, "marker-start"), marker_start, "{arrow}");
    }
}

/// The marker ids and the references to them carry the diagram's id, which keeps several maps on
/// one page apart (NOTES 6.3).
#[test]
fn the_marker_ids_carry_the_diagram_id() {
    let text = format!("{ONE}component B [0.5, 0.2]\nA +<> B\nevolve A 0.8\n");
    let diagram = parse(&text).expect("the diagram parses");
    let variables = Theme::ReduxColor.variables(None);
    let tokens = ThemeTokens::from_value(&variables);
    let config = typed(&defaults()).expect("the configuration");
    let globals = Globals::of(&config);
    let markup = render(&Context {
        diagram: &diagram,
        section: &config.wardley_beta,
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: "map-7",
    })
    .expect("the map renders");
    for id in [
        "arrow-map-7",
        "link-arrow-end-map-7",
        "link-arrow-start-map-7",
    ] {
        assert!(markup.contains(&format!("<marker id=\"{id}\"")), "{id}");
    }
    assert!(markup.contains("url(#link-arrow-end-map-7)"));
    assert!(markup.contains("url(#link-arrow-start-map-7)"));
    assert!(markup.contains("marker-end=\"url(#arrow-map-7)\""));
}

/// A dashed link keeps the dash of its attribute, `6 6`, since the reference's stylesheet that
/// would change it is not written (NOTES 6.9).
#[test]
fn a_dashed_link_keeps_its_six_six_dash() {
    let markup = draw(&format!("{ONE}component B [0.5, 0.2]\nA -.-> B\n"));
    let links = with_class(&markup, "line", "wardley-link wardley-link--dashed");
    assert_eq!(links.len(), 1);
    assert_eq!(attr(links[0].0, "stroke-dasharray"), Some("6 6"));
}

/// The inertia mark stands further out when the component has a sourcing overlay: its offset is the
/// radius and fifteen more, and the radius and ten more again (`wardleyRenderer.ts` 750-755).
#[test]
fn the_inertia_mark_stands_further_out_with_a_strategy() {
    let plain = draw("wardley-beta\ncomponent A [0.5, 0.5] (inertia)\n");
    let with_overlay = draw("wardley-beta\ncomponent A [0.5, 0.5] (build) (inertia)\n");
    let inertia = |markup: &str| {
        let lines = with_class(markup, "line", "wardley-inertia");
        assert_eq!(lines.len(), 1);
        lines[0].0.to_owned()
    };
    let plain_tag = inertia(&plain);
    let overlay_tag = inertia(&with_overlay);
    assert!(close(number(&plain_tag, "x1"), 471.0), "{plain_tag}");
    assert!(close(number(&overlay_tag, "x1"), 487.0), "{overlay_tag}");
    assert!(close(number(&overlay_tag, "y1"), 294.0), "{overlay_tag}");
    assert!(close(number(&overlay_tag, "y2"), 306.0), "{overlay_tag}");
    assert_eq!(attr(&overlay_tag, "stroke-width"), Some("6"));
}

/// A component's label is offset to the right and up by the configured offset, and by ten more with a
/// strategy when the statement gives no offset (`wardleyRenderer.ts` 787-808).
#[test]
fn a_label_gets_ten_more_with_a_strategy_unless_it_has_an_offset() {
    let labelled = |text: &str| {
        let markup = draw(text);
        let labels = with_class(&markup, "text", "wardley-node-label");
        assert_eq!(labels.len(), 1);
        (number(labels[0].0, "x"), number(labels[0].0, "y"))
    };
    let (x, y) = labelled("wardley-beta\ncomponent A [0.5, 0.5] (buy)\n");
    assert!(close(x, 468.0) && close(y, 282.0), "{x} {y}");
    let (x, y) = labelled("wardley-beta\ncomponent A [0.5, 0.5] label [-60, 35] (buy)\n");
    assert!(close(x, 390.0) && close(y, 335.0), "{x} {y}");
}

/// An anchor has no circle, and its label is centred on it in bold black, three pixels above it
/// (`wardleyRenderer.ts` 784-823).
#[test]
fn an_anchor_is_centred_with_a_bold_black_label() {
    let markup = draw("wardley-beta\nanchor Business [0.95, 0.63]\n");
    let node = with_class(&markup, "g", "wardley-node wardley-node--anchor");
    assert_eq!(node.len(), 1);
    assert!(!node[0].1.contains("<circle"), "an anchor has no circle");
    let labels = with_class(&markup, "text", "wardley-node-label");
    let (tag, text) = labels[0];
    assert_eq!(text, "Business");
    assert!(close(number(tag, "x"), 48.0 + 0.63 * 804.0), "{tag}");
    assert!(close(number(tag, "y"), 552.0 - 0.95 * 504.0 - 3.0), "{tag}");
    assert_eq!(attr(tag, "fill"), Some("#000"));
    assert_eq!(attr(tag, "font-weight"), Some("bold"));
    assert_eq!(attr(tag, "text-anchor"), Some("middle"));
    assert_eq!(attr(tag, "dominant-baseline"), Some("middle"));
}

/// The sourcing overlays keep their own fills: grey for a bought component, a dark grey for an
/// outsourced one, and a very light grey with a black edge for a built one (NOTES 6.4).
#[test]
fn the_sourcing_overlays_keep_their_own_fills() {
    let fill_of = |decorator: &str, name: &str| {
        let markup = draw(&format!(
            "wardley-beta\ncomponent A [0.5, 0.5] ({decorator})\n"
        ));
        let circles = with_class(&markup, "circle", name);
        assert_eq!(circles.len(), 1, "{decorator}");
        (
            attr(circles[0].0, "fill").map(str::to_owned),
            attr(circles[0].0, "stroke").map(str::to_owned),
        )
    };
    assert_eq!(
        fill_of("outsource", "wardley-outsource-overlay")
            .0
            .as_deref(),
        Some("#666")
    );
    assert_eq!(
        fill_of("buy", "wardley-buy-overlay").0.as_deref(),
        Some("#ccc")
    );
    let (fill, stroke) = fill_of("build", "wardley-build-overlay");
    assert_eq!(fill.as_deref(), Some("#eee"));
    assert_eq!(stroke.as_deref(), Some("#000"));
}

/// A market component has its overlay, three lines joining three small circles, and no circle of its
/// own (`wardleyRenderer.ts` 632-729).
#[test]
fn a_market_component_has_its_symbol_and_no_plain_circle() {
    let markup = draw("wardley-beta\ncomponent A [0.5, 0.5] (market)\n");
    assert_eq!(
        with_class(&markup, "circle", "wardley-market-overlay").len(),
        1
    );
    assert_eq!(with_class(&markup, "line", "wardley-market-line").len(), 3);
    let dots = with_class(&markup, "circle", "wardley-market-dot");
    assert_eq!(dots.len(), 3);
    for (tag, _) in &dots {
        assert_eq!(attr(tag, "stroke-width"), Some("2"));
    }
    let node = with_class(&markup, "g", "wardley-node wardley-node--component");
    let plain = elements(node[0].1, "circle")
        .into_iter()
        .filter(|(tag, _)| attr(tag, "class").is_none())
        .count();
    assert_eq!(plain, 0, "a market component is drawn by its symbol only");
}

/// The sizes of the map come from the `size` statement, which overrides the configuration (NOTES
/// 6.8).
#[test]
fn the_size_statement_overrides_the_configured_size() {
    let markup = draw("wardley-beta\nsize [1100, 800]\ncomponent A [0.5, 0.5]\n");
    assert!(markup.contains("viewBox=\"0 0 1100 800\""), "{markup}");
    assert!(markup.contains("style=\"max-width: 1100px;\""), "{markup}");
}

/// Without `useMaxWidth`, the root takes its size as `height` and `width` attributes and no style
/// (NOTES 6.8, `maxwidth-false`).
#[test]
fn without_use_max_width_the_root_takes_its_size_as_attributes() {
    let setup = Setup {
        section: vec![("useMaxWidth", Value::Bool(false))],
        ..Setup::default()
    };
    let markup = draw_with(ONE, &setup);
    let root = elements(&markup, "svg")[0].0;
    assert_eq!(attr(root, "width"), Some("900"));
    assert_eq!(attr(root, "height"), Some("600"));
    assert_eq!(attr(root, "style"), None);
}

/// The title is set at 1.05 times the axis size, which the reference writes as
/// `12.600000000000001` (NOTES 6.4).
#[test]
fn the_title_is_one_point_oh_five_times_the_axis_size() {
    let markup = draw("wardley-beta\ntitle Tea\ncomponent A [0.5, 0.5]\n");
    let titles = with_class(&markup, "text", "wardley-title");
    assert_eq!(titles.len(), 1);
    assert_eq!(attr(titles[0].0, "font-size"), Some("12.600000000000001"));
    assert_eq!(titles[0].1, "Tea");
}

/// An accelerator is an arrow to the right, written as the reference's template writes it, with the
/// template's line breaks and indentation (NOTES 6.4, `wardleyRenderer.ts` 992-1001).
#[test]
fn an_accelerator_path_is_the_templates_bytes() {
    let markup = draw("wardley-beta\naccelerator \"Speed\" [0.5, 0.5]\n");
    let paths = plain(&markup, "path");
    let path = paths
        .iter()
        .map(|(tag, _)| attr(tag, "d").expect("a path has d"))
        .next()
        .expect("the arrow is drawn");
    assert_eq!(
        path,
        "\n        M 450 285\n        L 490 285\n        L 490 277\n        L 510 300\n        L 490 323\n        L 490 315\n        L 450 315\n        Z\n      "
    );
    assert!(
        markup.contains(">Speed</text>"),
        "the arrow's name is below it"
    );
}

/// A deaccelerator is the accelerator's mirror image, pointing left (`wardleyRenderer.ts` 1037-1046).
#[test]
fn a_deaccelerator_points_left() {
    let markup = draw("wardley-beta\ndeaccelerator \"Stop\" [0.5, 0.5]\n");
    let path = elements(&markup, "path")
        .into_iter()
        .map(|(tag, _)| attr(tag, "d").expect("a path has d").to_owned())
        .next()
        .expect("the arrow is drawn");
    assert_eq!(
        path,
        "\n        M 510 285\n        L 470 285\n        L 470 277\n        L 450 300\n        L 470 323\n        L 470 315\n        L 510 315\n        Z\n      "
    );
}

/// The redux themes give no `wardley` group, so the colours are the renderer's own fallbacks, and the
/// text takes `primaryTextColor` and the background `background` (NOTES 6.10).
#[test]
fn the_redux_theme_falls_back_to_the_renderer_colours() {
    let variables = Theme::ReduxColor.variables(None);
    let background = variables
        .get("background")
        .and_then(Value::as_str)
        .expect("a background")
        .to_owned();
    let text = variables
        .get("primaryTextColor")
        .and_then(Value::as_str)
        .expect("a text colour")
        .to_owned();
    let markup = draw(&format!("{ONE}title T\n"));
    let rect = with_class(&markup, "rect", "wardley-background");
    assert_eq!(attr(rect[0].0, "fill"), Some(background.as_str()));
    let titles = with_class(&markup, "text", "wardley-title");
    assert_eq!(attr(titles[0].0, "fill"), Some(text.as_str()));
    let nodes = plain(&markup, "circle");
    let node = nodes
        .iter()
        .find(|(tag, _)| attr(tag, "r") == Some("6"))
        .expect("a node circle");
    assert_eq!(attr(node.0, "fill"), Some("#fff"));
    assert_eq!(attr(node.0, "stroke"), Some("#000"));
}

/// A `wardley` group of the theme variables gives the colours it names (NOTES 6.10).
#[test]
fn a_wardley_group_of_the_theme_overrides_the_colours() {
    let setup = Setup {
        group: Some(group(&[(
            "componentFill",
            Value::from(serde_json::json!("#abcdef")),
        )])),
        ..Setup::default()
    };
    let markup = draw_with(ONE, &setup);
    let node = plain(&markup, "circle");
    let node = node
        .iter()
        .find(|(tag, _)| attr(tag, "r") == Some("6"))
        .expect("a node circle");
    assert_eq!(attr(node.0, "fill"), Some("#abcdef"));
}

/// A colour that is not text is written as JavaScript writes it, and `null` takes the fallback
/// (`??` of `wardleyRenderer.ts` 35-48).
#[test]
fn a_colour_that_is_not_text_is_written_as_javascript_writes_it() {
    let setup = Setup {
        group: Some(group(&[
            ("axisColor", Value::Bool(true)),
            ("componentStroke", Value::Null),
        ])),
        ..Setup::default()
    };
    let markup = draw_with(ONE, &setup);
    let axes = with_class(&markup, "g", "wardley-axes");
    assert!(axes[0].1.contains("stroke=\"true\""), "{}", axes[0].1);
    let node = plain(&markup, "circle");
    let node = node
        .iter()
        .find(|(tag, _)| attr(tag, "r") == Some("6"))
        .expect("a node circle");
    assert_eq!(attr(node.0, "stroke"), Some("#000"));
}

/// The grid is drawn when the configuration asks for it, as three dotted lines of each direction
/// at a quarter, a half and three quarters of the plot (NOTES 6.4).
#[test]
fn the_grid_is_drawn_only_when_asked() {
    assert!(!draw(ONE).contains("wardley-grid"));
    let setup = Setup {
        section: vec![("showGrid", Value::Bool(true))],
        ..Setup::default()
    };
    let markup = draw_with(ONE, &setup);
    let lines = plain(&markup, "line");
    let dotted: Vec<_> = lines
        .into_iter()
        .filter(|(tag, _)| attr(tag, "stroke-dasharray") == Some("2 6"))
        .collect();
    assert_eq!(dotted.len(), 6);
    assert!(close(number(dotted[0].0, "x1"), 249.0));
}

/// The annotation lines in the box are sorted by their numbers, not by the text (NOTES 6.4).
#[test]
fn the_annotation_lines_are_sorted_by_their_numbers() {
    let markup = draw(
        "wardley-beta\nannotations [0.1, 0.9]\nannotation 2,[0.5, 0.5] \"Second\"\nannotation 1,[0.6, 0.65] \"First\"\n",
    );
    let first = markup.find(">1. First<").expect("the first line");
    let second = markup.find(">2. Second<").expect("the second line");
    assert!(first < second);
}

/// The box is placed where the statement puts it, then clamped to the margins of the plot: its
/// height is one line of 16 px for each annotation, the padding and half the text box (NOTES 6.5).
#[test]
fn the_annotation_box_is_clamped_to_the_margins() {
    let markup = draw("wardley-beta\nannotations [0.05, 0]\nannotation 1,[0.5, 0.5] \"Text\"\n");
    let boxes = plain(&markup, "rect");
    let rect = boxes
        .iter()
        .find(|(tag, _)| attr(tag, "rx") == Some("4"))
        .expect("the box is drawn");
    // The box is 16 + 20 + 13 / 2 high, so its top is at most 552 - 42.5 = 509.5.
    assert!(close(number(rect.0, "y"), 509.5), "{}", rect.0);
    assert!(close(number(rect.0, "x"), 48.0), "{}", rect.0);
}

/// The box is as wide as its widest line's advance and a buffer of 105 px on top of the padding, as
/// the reference writes it (`wardleyRenderer.ts` 925).
#[test]
fn the_annotation_box_is_its_widest_line_plus_the_buffer() {
    let text = "Alternative algos in research";
    let line = format!("1. {text}");
    let runs = [Run {
        text: &line,
        face: face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
    }];
    let advance = layout_width(measure_line(&runs, 11.0).expect("the line measures"));
    let markup = draw(&format!(
        "wardley-beta\nannotations [0.1, 0.9]\nannotation 1,[0.5, 0.5] \"{text}\"\n"
    ));
    let boxes = plain(&markup, "rect");
    let rect = boxes
        .iter()
        .find(|(tag, _)| attr(tag, "rx") == Some("4"))
        .expect("the box is drawn");
    let expected = advance + 20.0 + 105.0;
    assert!(
        (number(rect.0, "width") - expected).abs() < 1e-6,
        "{} vs {expected}",
        number(rect.0, "width")
    );
}

/// An annotation with no text gives the box group and no box (NOTES 6.4, `annotation-empty-text`).
#[test]
fn an_annotation_without_text_leaves_the_box_group_empty() {
    let markup = draw("wardley-beta\nannotations [0.5, 0.5]\nannotation 1,[0.5, 0.5] \"\"\n");
    assert!(
        markup.contains("<g class=\"wardley-annotations-box\"/>"),
        "{markup}"
    );
}

/// A number too large for a float is `Infinity`, and the reference writes it so in the circle
/// (`canonical-model.md`, "Numbers").
#[test]
fn an_annotation_number_too_large_for_a_float_is_written_as_infinity() {
    let digits = "9".repeat(400);
    let markup = draw(&format!(
        "wardley-beta\nannotation {digits},[0.5, 0.5] \"Big\"\n"
    ));
    let numbers = plain(&markup, "text");
    assert!(
        numbers.iter().any(|(_, text)| *text == "Infinity"),
        "{markup}"
    );
}

/// The label of a note is set in the reference's `#` placeholder as the markup's entity, which the
/// browser reads as the character (NOTES 6.4, the core's decoding of the whole SVG).
#[test]
fn a_placeholder_in_a_label_is_written_as_the_entity() {
    let markup = draw("wardley-beta\nnote \"A #35; B\" [0.5, 0.5]\n");
    assert!(markup.contains(">A &#35; B</text>"), "{markup}");
}

/// The notes are bold text at their positions (NOTES 6.4).
#[test]
fn a_note_is_bold_text_at_its_position() {
    let markup = draw("wardley-beta\nnote \"Watch\" [0.5, 0.5]\n");
    let notes = plain(&markup, "text");
    let note = notes
        .iter()
        .find(|(_, text)| *text == "Watch")
        .expect("the note");
    assert_eq!(attr(note.0, "font-weight"), Some("bold"));
    assert!(close(number(note.0, "x"), 450.0));
    assert!(close(number(note.0, "y"), 300.0));
}

/// Both weights of the bundled family are embedded, so that the bold title is drawn in the face the
/// reference measures (`text.rs`, `Drawn`).
#[test]
fn the_bold_face_is_embedded_with_the_regular_one() {
    let style = style_of(&draw("wardley-beta\ntitle Tea\ncomponent A [0.5, 0.5]\n"));
    assert!(style.contains("font-weight:400"), "{style}");
    assert!(style.contains("font-weight:700"), "{style}");
}
