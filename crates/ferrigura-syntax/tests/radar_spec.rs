//! Port of `packages/mermaid/src/diagrams/radar/radar.spec.ts`: the parser and DB tests, through
//! the public parse. Test names follow the reference's `it` titles in snake_case, and its
//! `describe` groups are comments.
//!
//! The renderer's checks (`relativeRadius`, `closedRoundCurve` and the two `draw` calls) are in
//! `crates/ferrigura-render/tests/radar_render_spec.rs`. The configuration and theme tests parse
//! only, as the reference's do.

use ferrigura_model::radar::{Graticule, RadarAxis, RadarCurve, RadarDiagram};
use ferrigura_syntax::radar::parse;

fn parsed(text: &str) -> RadarDiagram {
    parse(text).expect("the diagram parses")
}

fn axis(name: &str, label: &str) -> RadarAxis {
    RadarAxis {
        name: name.into(),
        label: label.into(),
    }
}

mod radar_diagram_definitions {
    use super::*;

    #[test]
    fn should_handle_a_simple_radar_definition() {
        parsed("radar-beta\n    axis A,B,C\n    curve mycurve{1,2,3}");
    }

    #[test]
    fn should_handle_diagram_with_data_and_title() {
        let diagram = parsed(
            "radar-beta
    title Radar diagram
    accTitle: Radar accTitle
    accDescr: Radar accDescription
    axis A[\"Axis A\"], B[\"Axis B\"] ,C[\"Axis C\"]
    curve mycurve[\"My Curve\"]{1,2,3}
    ",
        );
        assert_eq!(diagram.title, "Radar diagram");
        assert_eq!(diagram.acc_title, "Radar accTitle");
        assert_eq!(diagram.acc_descr, "Radar accDescription");
        assert_eq!(
            diagram.axes,
            vec![
                axis("A", "Axis A"),
                axis("B", "Axis B"),
                axis("C", "Axis C"),
            ]
        );
        assert_eq!(
            diagram.curves,
            vec![RadarCurve {
                name: "mycurve".into(),
                label: "My Curve".into(),
                entries: vec![1.0, 2.0, 3.0],
            }]
        );
        assert_eq!(diagram.options.graticule, Graticule::Circle);
        assert_eq!(diagram.options.max, None);
        assert_eq!(diagram.options.min, 0.0);
        assert!(diagram.options.show_legend);
        assert_eq!(diagram.options.ticks, 5.0);
    }

    #[test]
    fn should_handle_a_radar_diagram_with_options() {
        let diagram = parsed(
            "radar-beta
    ticks 10
    showLegend false
    graticule polygon
    min 1
    max 10
    ",
        );
        let options = diagram.options;
        assert_eq!(options.graticule, Graticule::Polygon);
        assert_eq!(options.max, Some(10.0));
        assert_eq!(options.min, 1.0);
        assert!(!options.show_legend);
        assert_eq!(options.ticks, 10.0);
    }

    #[test]
    fn should_handle_curve_with_detailed_data_in_any_order() {
        let diagram = parsed("radar-beta\n    axis A,B,C\n    curve mycurve{ C: 3, A: 1, B: 2 }");
        assert_eq!(
            diagram.curves,
            vec![RadarCurve {
                name: "mycurve".into(),
                label: "mycurve".into(),
                entries: vec![1.0, 2.0, 3.0],
            }]
        );
    }

    #[test]
    fn should_handle_radar_diagram_with_comments() {
        parsed(
            "radar-beta
    %% This is a comment
    axis A,B,C
    %% This is another comment
    curve mycurve{1,2,3}
    ",
        );
    }

    #[test]
    fn should_handle_radar_diagram_with_config_override() {
        parsed(
            "
    %%{init: {'radar': {'marginTop': 80, 'axisLabelFactor': 1.25}}}%%
    radar-beta
    axis A,B,C
    curve mycurve{1,2,3}
    ",
        );
    }

    #[test]
    fn should_parse_radar_diagram_with_theme_override() {
        parsed(
            "
    %%{init: { \"theme\": \"base\", \"themeVariables\": {'fontSize': 80, 'cScale0': '#123456' }}}%%
    radar-beta:
    axis A,B,C
    curve mycurve{1,2,3}
    ",
        );
    }

    #[test]
    fn should_handle_radar_diagram_with_radar_style_override() {
        parsed(
            "
    %%{init: { \"theme\": \"base\", \"themeVariables\": {'fontSize': 10, 'radar': { 'axisColor': '#FF0000' }}}}%%
    radar-beta
    axis A,B,C
    curve mycurve{1,2,3}
    ",
        );
    }

    #[test]
    fn should_limit_the_number_of_ticks_to_max_ticks() {
        // Otherwise, user can set millions of ticks, which causes DoS.
        for (ticks, expected) in [(12.0, 12.0), (32.0, 32.0), (33.0, 32.0)] {
            let diagram = parsed(&format!("radar-beta\n      ticks {ticks}\n      "));
            assert_eq!(diagram.options.ticks, expected, "ticks {ticks}");
        }
    }
}
