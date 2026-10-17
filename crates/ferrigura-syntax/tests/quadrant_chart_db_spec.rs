//! Port of `packages/mermaid/src/diagrams/quadrant-chart/quadrantDb.spec.ts`. The reference tests
//! its style parser through the DB's `parseStyles`, which the Rust DB does not expose, so each test
//! drives `parse` with the text that makes the grammar pass the styles to the DB: a `classDef`
//! line, or a point. The model keeps the styles the DB sets, so the spec's expected objects are the
//! styles of the class or the point.
//!
//! The spec's last test spies on the reference's `getConfig` call. The port's sanitiser takes no
//! configuration, so there is no call to observe; it is kept as an ignored test with that reason.

use ferrigura_model::quadrant_chart::QuadrantStyle;
use ferrigura_syntax::quadrant_chart::parse;

/// The style of the class named `hot` in the model, which the spec's `classDef` line defines.
fn class_style(text: &str) -> QuadrantStyle {
    let diagram = parse(text).expect("parses");
    diagram
        .classes
        .into_iter()
        .find(|(name, _)| name == "hot")
        .expect("a class named hot")
        .1
}

#[test]
fn should_parse_the_styles_array_and_return_a_styles_object() {
    let style = class_style(
        "quadrantChart\nclassDef hot radius: 10, color: #ff0000, stroke-color: #ff00ff, stroke-width: 10px",
    );
    assert_eq!(
        style,
        QuadrantStyle {
            radius: Some(10.0),
            color: Some("#ff0000".to_owned()),
            stroke_color: Some("#ff00ff".to_owned()),
            stroke_width: Some("10px".to_owned()),
        }
    );
}

#[test]
fn should_throw_an_error_for_non_supported_style_name() {
    let error = parse("quadrantChart\nclassDef hot testname: value").expect_err("rejected");
    assert!(
        error
            .message
            .contains("style named testname is not supported."),
        "{}",
        error.message
    );
}

#[test]
fn should_return_an_empty_styles_object_for_an_empty_input_array() {
    // The grammar never passes an empty list of styles: a class needs one, and a point without
    // styles passes none. The point's style is therefore the empty object.
    let diagram = parse("quadrantChart\nP: [0.5, 0.5]").expect("parses");
    assert_eq!(diagram.points[0].style, QuadrantStyle::default());
}

#[test]
fn should_throw_an_error_for_non_supported_style_value() {
    let cases = [
        (
            "quadrantChart\nclassDef hot radius: f",
            "value for radius f is invalid, please use a valid number",
        ),
        (
            "quadrantChart\nclassDef hot color: ffaa",
            "value for color ffaa is invalid, please use a valid hex code",
        ),
        (
            "quadrantChart\nclassDef hot stroke-color: #f677779",
            "value for stroke-color #f677779 is invalid, please use a valid hex code",
        ),
        (
            "quadrantChart\nclassDef hot stroke-width: 30",
            "value for stroke-width 30 is invalid, please use a valid number of pixels (eg. 10px)",
        ),
    ];
    for (text, message) in cases {
        let error = parse(text).expect_err("rejected");
        assert_eq!(error.message, message);
    }
}

#[test]
#[ignore = "the reference's spec spies on its getConfig call at sanitising time; the port's sanitiser takes no configuration (docs-port/quadrant/PLAN.md, section 6)"]
fn should_call_get_config_at_sanitization_time_not_at_module_load_time() {}
