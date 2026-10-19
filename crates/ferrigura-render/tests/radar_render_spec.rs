//! Port of the renderer tests of `packages/mermaid/src/diagrams/radar/radar.spec.ts`: `relativeRadius`
//! (six tests), `closedRoundCurve` (three) and the two `draw` calls. Test names follow the spec's
//! titles in snake_case. The `draw` calls assert nothing in the reference, so the two below check the
//! SVG they write, as far as the spec's diagrams show it.

use ferrigura_model::config::{effective, typed};
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_render::context::{Context, Globals, ThemeView};
use ferrigura_render::radar::geometry::{Point, closed_round_curve, relative_radius};
use ferrigura_render::radar::render;

const ID: &str = "tst";

/// Renders a radar text with the redux colour theme, its configuration read the way the facade reads
/// it: the front matter and the directives of the text give the `radar` section.
fn draw(text: &str) -> String {
    let variables = Theme::ReduxColor.variables(None);
    let directive = ferrigura_syntax::config::read(text).expect("the directives read");
    let configured = effective(None, Some("radar"), &directive);
    let config = typed(&configured).expect("the configuration is typed");
    let diagram = ferrigura_syntax::radar::parse(text).expect("the radar parses");
    let tokens = ThemeTokens::from_value(&variables);
    let globals = Globals::of(&config);
    render(&Context {
        diagram: &diagram,
        section: &config.radar,
        globals: &globals,
        theme: ThemeView {
            light: &tokens,
            dark: None,
        },
        id: ID,
    })
    .expect("the radar renders")
}

mod renderer {
    use super::*;

    mod relative_radius {
        use super::*;

        #[test]
        fn should_calculate_relative_radius() {
            assert_eq!(relative_radius(5.0, 0.0, 10.0, 100.0), 50.0);
        }

        #[test]
        fn should_handle_min_value() {
            assert_eq!(relative_radius(0.0, 0.0, 10.0, 100.0), 0.0);
        }

        #[test]
        fn should_handle_max_value() {
            assert_eq!(relative_radius(10.0, 0.0, 10.0, 100.0), 100.0);
        }

        #[test]
        fn should_clip_values_below_min() {
            assert_eq!(relative_radius(-5.0, 0.0, 10.0, 100.0), 0.0);
        }

        #[test]
        fn should_clip_values_above_max() {
            assert_eq!(relative_radius(15.0, 0.0, 10.0, 100.0), 100.0);
        }

        #[test]
        fn should_handle_negative_min() {
            assert_eq!(relative_radius(5.0, -10.0, 10.0, 100.0), 75.0);
        }
    }

    mod closed_round_curve {
        use super::*;

        #[test]
        fn should_construct_a_polygon_if_tension_is_0() {
            let points = [
                Point { x: 0.0, y: 0.0 },
                Point { x: 100.0, y: 0.0 },
                Point { x: 100.0, y: 100.0 },
                Point { x: 0.0, y: 100.0 },
            ];
            assert_eq!(
                closed_round_curve(&points, 0.0),
                "M0,0 C0,0 100,0 100,0 C100,0 100,100 100,100 C100,100 0,100 0,100 C0,100 0,0 0,0 Z"
            );
        }

        #[test]
        fn should_construct_a_simple_round_curve() {
            let points = [Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 100.0 }];
            assert_eq!(
                closed_round_curve(&points, 0.5),
                "M0,0 C0,0 100,100 100,100 C100,100 0,0 0,0 Z"
            );
        }

        #[test]
        fn should_construct_a_closed_round_curve() {
            let points = [
                Point { x: 0.0, y: 0.0 },
                Point { x: 100.0, y: 0.0 },
                Point { x: 100.0, y: 100.0 },
                Point { x: 0.0, y: 100.0 },
            ];
            assert_eq!(
                closed_round_curve(&points, 0.5),
                "M0,0 C50,-50 50,-50 100,0 C150,50 150,50 100,100 C50,150 50,150 0,100 C-50,50 -50,50 0,0 Z"
            );
        }
    }

    mod draw {
        use super::*;

        #[test]
        fn should_draw_a_simple_radar_diagram() {
            let svg = draw("radar-beta\n        axis A,B,C\n        curve mycurve{1,2,3}");
            assert!(svg.starts_with("<svg id=\"tst\""), "{svg}");
            assert!(svg.contains("class=\"radarAxisLine\""), "{svg}");
            assert!(svg.contains("class=\"radarCurve-0\""), "{svg}");
            assert!(svg.ends_with("</svg>"), "{svg}");
        }

        #[test]
        fn should_draw_a_complex_radar_diagram() {
            let svg = draw(
                "radar-beta
        title Radar diagram
        accTitle: Radar accTitle
        accDescr: Radar accDescription
        axis A[\"Axis A\"], B[\"Axis B\"] ,C[\"Axis C\"]
        curve mycurve[\"My Curve\"]{1,2,3}
        curve mycurve2[\"My Curve 2\"]{ C: 1, A: 2, B: 3 }
        graticule polygon
        ",
            );
            assert!(svg.contains("<title id=\"chart-title-tst\">Radar accTitle</title>"));
            assert!(svg.contains("<desc id=\"chart-desc-tst\">Radar accDescription</desc>"));
            assert!(svg.contains("<polygon points=\""), "{svg}");
            assert!(svg.contains(">My Curve 2</text>"), "{svg}");
            assert!(svg.contains("class=\"radarTitle\" x=\"0\" y=\"-350\">Radar diagram</text>"));
        }
    }
}
