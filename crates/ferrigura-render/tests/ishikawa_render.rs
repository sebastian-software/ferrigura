//! The ishikawa renderer against the numbers of the reference (`docs-port/ishikawa/NOTES.md`
//! section 6), which Chromium gives for the fixtures of `e2e/diagrams/ishikawa` and the probes of
//! `tools/port/geometry/probes/ishikawa`, read with `pnpm port:geometry ishikawa`. The numbers are
//! the reference's and are checked there; these tests keep the port's own arithmetic in view.

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::ishikawa::render;

/// Fixture 1 (`pr-check/1-should-render-a-simple-ishikawa-diagram`): one pair of causes, each with a
/// sub-cause.
const FIXTURE_ONE: &str = "ishikawa-beta
  Blurry Photo
  Process
      Out of focus
  User
      Shaky hands
";

/// Probe `p11`: a head of one word and one cause with no children.
const SHORT_HEAD: &str = "ishikawa-beta
  Bug
  Code
";

/// Renders the diagram with the redux colour theme and the default configuration. `look` replaces
/// the look of the configuration when given.
fn draw(text: &str, id: &str, look: Option<&str>) -> Result<String, RenderError> {
    let diagram = ferrigura_syntax::ishikawa::parse(text).expect("the ishikawa parses");
    let mut value = defaults();
    if let Some(look) = look {
        value
            .as_object_mut()
            .expect("the defaults are an object")
            .insert("look".to_owned(), Value::from(look));
    }
    let config = typed(&value).expect("the configuration is typed");
    let variables = Theme::ReduxColor.variables(None);
    let tokens = ThemeTokens::from_value(&variables);
    let globals = Globals::of(&config);
    render(&Context {
        diagram: &diagram,
        section: &config.ishikawa,
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id,
    })
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

/// The numbers of a `viewBox` or of a `translate(…)`, as they are written: the words are left out.
fn numbers(text: &str) -> Vec<f64> {
    text.split([' ', ',', '(', ')'])
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// Asserts that two lists of numbers agree to a hundredth, the rounding of the reference's float32
/// boxes.
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

/// The number of occurrences of `needle` in `svg`.
fn count(svg: &str, needle: &str) -> usize {
    svg.matches(needle).count()
}

#[test]
fn fixture_one_has_the_reference_viewbox_and_head() {
    // `NOTES.md` 6.3: the box of the drawing is `[-144.5373, -20.0983, 216.5373, 538.134]`, and the
    // viewBox pads it by the diagram padding of 20 on each side.
    let svg = draw(FIXTURE_ONE, "ref-1", None).expect("fixture one renders");
    assert_close(&view_box(&svg), &[-164.5373, -40.0983, 256.5373, 578.134]);
    assert!(
        svg.contains("d=\"M 0 -53.8 L 0 53.8 Q 144 0 0 -53.8 Z\""),
        "the head's shape"
    );
    assert!(
        svg.contains("transform=\"translate(0,250)\""),
        "the head sits on the spine"
    );
    assert!(
        svg.contains("width=\"100%\" style=\"max-width: 256.5"),
        "useMaxWidth is the default"
    );
}

#[test]
fn the_spine_ends_at_the_leftmost_text_of_the_last_pair() {
    // `NOTES.md` 6.2: the spine starts at `spineX`, the leftmost text of the last pair. In fixture
    // one the text `Shaky hands` is the leftmost, at -67.3966 and anchored at its end.
    let svg = draw(FIXTURE_ONE, "ref-1", None).expect("fixture one renders");
    let spine = attribute(&svg, "line", "x1").expect("the spine is drawn");
    assert!(
        (spine.parse::<f64>().unwrap() + 144.5373).abs() < 0.01,
        "{spine}"
    );
}

#[test]
fn every_cause_is_a_branch_and_every_descendant_a_bone() {
    let svg = draw(FIXTURE_ONE, "ref-1", None).expect("fixture one renders");
    // Two causes give two branch lines, each with its marker; two sub-causes give two bones.
    assert_eq!(count(&svg, "class=\"ishikawa-branch\""), 2);
    assert_eq!(
        count(&svg, "marker-start=\"url(#ishikawa-arrow-ref-1)\""),
        4
    );
    assert_eq!(count(&svg, "class=\"ishikawa-sub-group\""), 2);
    assert_eq!(count(&svg, "class=\"ishikawa-pair\""), 1);
    assert_eq!(count(&svg, "class=\"ishikawa-label-group\""), 2);
}

#[test]
fn a_root_without_causes_has_no_pairs_and_a_zero_length_spine() {
    // Probe-like case of fixture 5: the spine is a point at the head's foot.
    let svg = draw("ishikawa-beta\n  Root only\n", "ref-5", None).expect("renders");
    assert_eq!(count(&svg, "class=\"ishikawa-pair\""), 0);
    assert!(svg.contains("<line class=\"ishikawa-spine\" x1=\"0\" y1=\"250\" x2=\"0\" y2=\"250\""));
}

#[test]
fn a_head_of_one_word_takes_the_minimum_shape() {
    // `max(60, width + 6)` and `max(40, 2 * height + 40)`: a single line of 17 px gives 74 px, so
    // the shape is 60 wide and 74 tall, and its curve reaches 72 (1.2 times the width).
    let svg = draw(SHORT_HEAD, "ref-11", None).expect("renders");
    assert!(
        svg.contains("d=\"M 0 -37 L 0 37 Q 144 0 0 -37 Z\""),
        "{svg}"
    );
}

#[test]
fn the_angle_of_the_causes_uses_v8s_cosine_and_sine() {
    // The first branch runs from the spine at (-20, 250) by `cos(82°) * 250` to the left.
    let svg = draw(FIXTURE_ONE, "ref-1", None).expect("fixture one renders");
    let first = svg.find("class=\"ishikawa-branch\"").expect("a branch");
    let element = &svg[first..svg[first..].find("/>").unwrap() + first];
    assert!(element.contains("x2=\"-54.79327524001642\""), "{element}");
    assert!(element.contains("y2=\"2.432982814607442\""), "{element}");
}

#[test]
fn a_handdrawn_look_draws_the_classic_geometry() {
    // roughjs is not ported; the other types draw the classic geometry for this look as well.
    let classic = draw(FIXTURE_ONE, "ref-6", None).expect("fixture one renders");
    let hand_drawn =
        draw(FIXTURE_ONE, "ref-6", Some("handDrawn")).expect("the look falls back to classic");
    assert_eq!(hand_drawn, classic);
}

#[test]
fn the_style_sheet_scopes_every_rule_to_the_diagram() {
    let svg = draw(FIXTURE_ONE, "ref-1", None).expect("renders");
    let sheet = &svg[svg.find("<style>").unwrap()..svg.find("</style>").unwrap()];
    // Every rule of the diagram starts with its id, and the labels take the embedded face.
    assert!(sheet.contains("#ref-1 .ishikawa .ishikawa-head-label{font-weight:600;"));
    assert!(sheet.contains("#ref-1 .ishikawa .ishikawa-label.down{dominant-baseline:hanging;}"));
    assert!(sheet.contains("#ref-1{--mainBkg:#ffffff;}"));
}
