//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-lines.spec.js`. Test names follow the
//! spec's titles in snake_case. Where two titles are the same, the later test gets a `_2` suffix.
//!
//! The tests with `@{ … }` edge metadata are step 7 and stay ignored until that step lands.

use ferrigura_syntax::flowchart::parse;

mod lines_when_parsing {
    use super::*;

    #[test]
    fn should_handle_line_interpolation_default_definitions() {
        let diagram =
            parse("graph TD\nA-->B\nlinkStyle default interpolate basis").expect("parses");
        assert_eq!(diagram.default_interpolate.as_deref(), Some("basis"));
    }

    #[test]
    fn should_handle_line_interpolation_numbered_definitions() {
        let diagram = parse(
            "graph TD\nA-->B\nA-->C\nlinkStyle 0 interpolate basis\nlinkStyle 1 interpolate cardinal",
        )
        .expect("parses");
        assert_eq!(diagram.edges[0].interpolate.as_deref(), Some("basis"));
        assert_eq!(diagram.edges[1].interpolate.as_deref(), Some("cardinal"));
    }

    #[test]
    #[ignore = "port: shape data, step 7"]
    fn should_handle_edge_curve_properties_using_edge_id() {
        let diagram = parse(
            "graph TD\nA e1@-->B\nA uniqueName@-->C\ne1@{curve: basis}\nuniqueName@{curve: cardinal}",
        )
        .expect("parses");
        assert_eq!(diagram.edges[0].interpolate.as_deref(), Some("basis"));
        assert_eq!(diagram.edges[1].interpolate.as_deref(), Some("cardinal"));
    }

    // Uses `@{ … }` edge metadata, which is step 7.
    #[test]
    #[ignore = "port: shape data, step 7"]
    fn should_handle_edge_curve_properties_using_edge_id_but_without_overriding_default() {
        let diagram = parse(
            "graph TD\nA e1@-->B\nA-->C\nlinkStyle default interpolate linear\ne1@{curve: stepAfter}",
        )
        .expect("parses");
        assert_eq!(diagram.edges[0].interpolate.as_deref(), Some("stepAfter"));
        assert_eq!(diagram.default_interpolate.as_deref(), Some("linear"));
    }

    // Uses `@{ … }` edge metadata, which is step 7.
    #[test]
    #[ignore = "port: shape data, step 7"]
    fn should_handle_edge_curve_properties_using_edge_id_mixed_with_line_interpolation() {
        let diagram = parse(
            "graph TD\nA e1@-->B-->D\nA-->C e4@-->D-->E\nlinkStyle default interpolate linear\nlinkStyle 1 interpolate basis\ne1@{curve: monotoneX}\ne4@{curve: stepBefore}",
        )
        .expect("parses");
        assert_eq!(diagram.edges[0].interpolate.as_deref(), Some("monotoneX"));
        assert_eq!(diagram.edges[1].interpolate.as_deref(), Some("basis"));
        assert_eq!(diagram.default_interpolate.as_deref(), Some("linear"));
        assert_eq!(diagram.edges[3].interpolate.as_deref(), Some("stepBefore"));
        assert_eq!(diagram.default_interpolate.as_deref(), Some("linear"));
    }

    #[test]
    fn should_handle_line_interpolation_multi_numbered_definitions() {
        let diagram =
            parse("graph TD\nA-->B\nA-->C\nlinkStyle 0,1 interpolate basis").expect("parses");
        assert_eq!(diagram.edges[0].interpolate.as_deref(), Some("basis"));
        assert_eq!(diagram.edges[1].interpolate.as_deref(), Some("basis"));
    }

    #[test]
    fn should_handle_line_interpolation_default_with_style() {
        let diagram =
            parse("graph TD\nA-->B\nlinkStyle default interpolate basis stroke-width:1px;")
                .expect("parses");
        assert_eq!(diagram.default_interpolate.as_deref(), Some("basis"));
    }

    #[test]
    fn should_handle_line_interpolation_numbered_with_style() {
        let diagram = parse(
            "graph TD\nA-->B\nA-->C\nlinkStyle 0 interpolate basis stroke-width:1px;\nlinkStyle 1 interpolate cardinal stroke-width:1px;",
        )
        .expect("parses");
        assert_eq!(diagram.edges[0].interpolate.as_deref(), Some("basis"));
        assert_eq!(diagram.edges[1].interpolate.as_deref(), Some("cardinal"));
    }

    #[test]
    fn should_handle_line_interpolation_multi_numbered_with_style() {
        let diagram =
            parse("graph TD\nA-->B\nA-->C\nlinkStyle 0,1 interpolate basis stroke-width:1px;")
                .expect("parses");
        assert_eq!(diagram.edges[0].interpolate.as_deref(), Some("basis"));
        assert_eq!(diagram.edges[1].interpolate.as_deref(), Some("basis"));
    }

    mod it_should_handle_new_line_type_notation {
        use super::super::*;

        #[test]
        fn should_handle_regular_lines() {
            let diagram = parse("graph TD;A-->B;").expect("parses");
            assert_eq!(diagram.edges[0].stroke, "normal");
        }

        #[test]
        fn should_handle_dotted_lines() {
            let diagram = parse("graph TD;A-.->B;").expect("parses");
            assert_eq!(diagram.edges[0].stroke, "dotted");
        }

        // The spec gives this test the same title as the one above; the input is a thick line.
        #[test]
        fn should_handle_dotted_lines_2() {
            let diagram = parse("graph TD;A==>B;").expect("parses");
            assert_eq!(diagram.edges[0].stroke, "thick");
        }
    }
}
