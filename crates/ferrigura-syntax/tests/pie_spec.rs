//! Port of `packages/mermaid/src/diagrams/pie/pie.spec.ts`. Test names follow the spec's titles.
//!
//! The renderer spec (`pieRenderer.spec.ts`) checks SVG and is deferred to M2.

use ferrigura_model::pie::PieDiagram;
use ferrigura_syntax::pie::parse;

fn value_of(diagram: &PieDiagram, label: &str) -> Option<f64> {
    diagram
        .sections
        .iter()
        .find(|section| section.label == label)
        .map(|section| section.value)
}

mod parse_spec {
    use super::*;

    #[test]
    fn should_handle_very_simple_pie() {
        let diagram = parse(
            r#"pie
      "ash": 100
      "#,
        )
        .expect("parses");
        assert_eq!(value_of(&diagram, "ash"), Some(100.0));
    }

    #[test]
    fn should_handle_simple_pie() {
        let diagram = parse(
            r#"pie
      "ash" : 60
      "bat" : 40
      "#,
        )
        .expect("parses");
        assert_eq!(value_of(&diagram, "ash"), Some(60.0));
        assert_eq!(value_of(&diagram, "bat"), Some(40.0));
    }

    #[test]
    fn should_handle_simple_pie_with_show_data() {
        let diagram = parse(
            r#"pie showData
      "ash" : 60
      "bat" : 40
      "#,
        )
        .expect("parses");
        assert!(diagram.show_data);
        assert_eq!(value_of(&diagram, "ash"), Some(60.0));
        assert_eq!(value_of(&diagram, "bat"), Some(40.0));
    }

    #[test]
    fn should_handle_simple_pie_with_comments() {
        let diagram = parse(
            r#"pie
      %% comments
      "ash" : 60
      "bat" : 40
      "#,
        )
        .expect("parses");
        assert_eq!(value_of(&diagram, "ash"), Some(60.0));
        assert_eq!(value_of(&diagram, "bat"), Some(40.0));
    }

    #[test]
    fn should_handle_simple_pie_with_a_title() {
        let diagram = parse(
            r#"pie title a 60/40 pie
      "ash" : 60
      "bat" : 40
      "#,
        )
        .expect("parses");
        assert_eq!(diagram.title, "a 60/40 pie");
        assert_eq!(value_of(&diagram, "ash"), Some(60.0));
        assert_eq!(value_of(&diagram, "bat"), Some(40.0));
    }

    #[test]
    fn should_handle_simple_pie_with_an_acc_title_acc_title() {
        let diagram = parse(
            r#"pie title a neat chart
      accTitle: a neat acc title
      "ash" : 60
      "bat" : 40
      "#,
        )
        .expect("parses");
        assert_eq!(diagram.title, "a neat chart");
        assert_eq!(diagram.acc_title, "a neat acc title");
        assert_eq!(value_of(&diagram, "ash"), Some(60.0));
        assert_eq!(value_of(&diagram, "bat"), Some(40.0));
    }

    #[test]
    fn should_handle_simple_pie_with_an_acc_description_acc_descr() {
        let diagram = parse(
            r#"pie title a neat chart
      accDescr: a neat description
      "ash" : 60
      "bat" : 40
      "#,
        )
        .expect("parses");
        assert_eq!(diagram.title, "a neat chart");
        assert_eq!(diagram.acc_descr, "a neat description");
        assert_eq!(value_of(&diagram, "ash"), Some(60.0));
        assert_eq!(value_of(&diagram, "bat"), Some(40.0));
    }

    #[test]
    fn should_handle_simple_pie_with_a_multiline_acc_description_acc_descr() {
        let diagram = parse(
            r#"pie title a neat chart
      accDescr {
        a neat description
        on multiple lines
      }
      "ash" : 60
      "bat" : 40
    "#,
        )
        .expect("parses");
        assert_eq!(diagram.title, "a neat chart");
        assert_eq!(diagram.acc_descr, "a neat description\non multiple lines");
        assert_eq!(value_of(&diagram, "ash"), Some(60.0));
        assert_eq!(value_of(&diagram, "bat"), Some(40.0));
    }

    #[test]
    fn should_handle_simple_pie_with_positive_decimal() {
        let diagram = parse(
            r#"pie
      "ash" : 60.67
      "bat" : 40
      "#,
        )
        .expect("parses");
        assert_eq!(value_of(&diagram, "ash"), Some(60.67));
        assert_eq!(value_of(&diagram, "bat"), Some(40.0));
    }

    #[test]
    fn should_handle_simple_pie_with_negative_decimal() {
        let error = parse(
            r#"pie
        "ash" : -60.67
        "bat" : 40.12
        "#,
        )
        .expect_err("a negative value is rejected");
        assert_eq!(error.line, 2);
    }

    #[test]
    fn should_handle_simple_pie_with_zero_slice_value() {
        let diagram = parse(
            r#"pie title Default text position: Animal adoption
        accTitle: simple pie char demo
        accDescr: pie chart with 3 sections: dogs, cats, rats. Most are dogs.
         "dogs" : 0
        "rats" : 40.12
      "#,
        )
        .expect("parses");
        assert_eq!(value_of(&diagram, "dogs"), Some(0.0));
        assert_eq!(value_of(&diagram, "rats"), Some(40.12));
    }

    #[test]
    fn should_handle_simple_pie_with_negative_slice_value() {
        // The reference names the value in its message; here only the failure and its position
        // are asserted, as `docs-port/decisions/0002-hand-written-parsers.md` decides.
        let error = parse(
            r#"pie title Default text position: Animal adoption
        accTitle: simple pie char demo
        accDescr: pie chart with 3 sections: dogs, cats, rats. Most are dogs.
         "dogs" : -60.67
        "rats" : 40.12
    "#,
        )
        .expect_err("a negative value is rejected");
        assert_eq!(error.line, 4);
    }

    #[test]
    fn should_handle_unsafe_properties() {
        let diagram = parse(
            r#"pie title Unsafe props test
        "__proto__" : 386
        "constructor" : 85
        "prototype" : 15"#,
        )
        .expect("parses");
        let labels: Vec<&str> = diagram
            .sections
            .iter()
            .map(|section| section.label.as_str())
            .collect();
        assert_eq!(labels, ["__proto__", "constructor", "prototype"]);
    }
}

mod config {
    use ferrigura_model::config::{PieDiagramConfig, defaults};

    // `setConfig` and `resetConfig` are `it.todo` in the reference, and their bodies are commented
    // out: nothing is asserted there, and the pie DB has no `setConfig` to call.

    #[test]
    #[ignore = "port: it.todo in the reference with its body commented out; the pie DB has no setConfig"]
    fn set_config() {}

    #[test]
    fn get_config() {
        // The reference checks that the DB's config is the pie section of the defaults. That section
        // is the schema's pie defaults, plus `useWidth` from `defaultConfig.ts`.
        let pie = PieDiagramConfig::default();
        assert_eq!(pie.text_position, 0.75);
        assert_eq!(pie.donut_hole, 0.0);
        assert_eq!(pie.legend_position, "right");
        assert_eq!(pie.highlight_slice, "");
        let mut expected = serde_json::to_value(&pie).expect("the pie config serialises");
        expected["useWidth"] = serde_json::json!(984);
        assert_eq!(defaults()["pie"], expected);
    }

    #[test]
    #[ignore = "port: it.todo in the reference with its body commented out; the pie DB has no resetConfig"]
    fn reset_config() {}
}
