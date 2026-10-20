//! Ports of the reference's swimlane specs that test the detection and the configuration of the type:
//!
//! - `diagrams/swimlanes/swimlanesDiagram.spec.ts`: the layout a swimlane diagram resolves to, with
//!   and without the user's settings (`layout_for`, the five tests of the spec).
//! - `mermaidAPI.spec.ts` "resolves swimlanes as its own diagram type", and
//!   `diagram-api/diagram-orchestration.spec.ts` "swimlane-beta TD;" as `swimlane`.
//! - `config.appearance.spec.ts`: the rows of its tables that the swimlane type takes (the redux
//!   colour theme and the neo look, the `elk` layout of the other types, and the appearance layers).
//!
//! The reference resolves a diagram's configuration with `setDiagramConfigScope(diagramType)` and
//! `getConfig()` after `mermaidAPI.parse(text)`. Here the same resolution is `effective`, with the
//! scope of the detected type and the directive of the text, as the configuration example does.
//! The parser's own decisions are in `swimlanes_parser_spec.rs`; the flowchart spec's "should accept
//! swimlane-beta as a graph keyword" is `should_accept_swimlane_beta_as_a_graph_keyword` in
//! `flowchart_flow_spec.rs`.

use ferrigura_model::config::{effective, typed};
use ferrigura_model::value::Value;
use ferrigura_syntax::{Diagram, DiagramType, config, detect, parse};
use serde_json::json;

const SWIMLANE: &str = "swimlane-beta TD\n  A --> B";
const FLOWCHART: &str = "flowchart TD\n  A --> B";

/// The configuration of `text` in the scope of its diagram type, with the settings given to
/// `initialize`: what `mermaidAPI.parse` and `getConfig()` give in that scope.
fn config_of(text: &str, site: Option<&Value>) -> Value {
    let kind = detect(text).expect("the text has a diagram type");
    let directive = config::read(text).expect("the directive reads");
    effective(site, Some(kind.config_key()), &directive)
}

/// The `layout` of the diagram's configuration, as `layoutFor` in the spec reads it.
fn layout_for(text: &str, site: Option<&Value>) -> Option<String> {
    config_of(text, site)["layout"].as_str().map(str::to_owned)
}

mod swimlanes_diagram_spec {
    use super::*;

    #[test]
    fn defaults_the_shared_flowchart_renderer_to_the_swimlane_layout() {
        assert_eq!(layout_for(SWIMLANE, None).as_deref(), Some("swimlane"));
    }

    #[test]
    fn leaves_plain_flowcharts_on_the_global_default_layout() {
        assert_eq!(layout_for(FLOWCHART, None).as_deref(), Some("elk"));
    }

    #[test]
    fn keeps_an_explicit_global_layout_override() {
        let site = Value::from(json!({ "layout": "elk" }));
        assert_eq!(layout_for(SWIMLANE, Some(&site)).as_deref(), Some("elk"));
    }

    #[test]
    fn keeps_a_diagram_scoped_layout_override() {
        // `init` used to override this too, so swimlanes could not be moved off the engine.
        let site = Value::from(json!({ "swimlane": { "layout": "elk" } }));
        assert_eq!(layout_for(SWIMLANE, Some(&site)).as_deref(), Some("elk"));
    }

    #[test]
    fn keeps_a_layout_set_in_the_diagram_frontmatter() {
        let text = format!("---\nconfig:\n  layout: elk\n---\n{SWIMLANE}");
        assert_eq!(layout_for(&text, None).as_deref(), Some("elk"));
    }
}

mod detection {
    use super::*;

    #[test]
    fn resolves_swimlanes_as_its_own_diagram_type() {
        // `mermaidAPI.spec.ts`: `swimlane-beta TD;A-->B;` is `swimlane`, and so is the orchestration's
        // `swimlane-beta TD;`. The configuration section is `swimlane`, as `getDiagramConfigKey` names it.
        assert_eq!(
            detect("swimlane-beta TD;A-->B;"),
            Some(DiagramType::Swimlanes)
        );
        assert_eq!(detect("swimlane-beta TD;"), Some(DiagramType::Swimlanes));
        assert_eq!(DiagramType::Swimlanes.config_key(), "swimlane");
    }

    #[test]
    fn parses_a_swimlane_text_to_the_swimlane_diagram() {
        assert!(matches!(parse(SWIMLANE), Ok(Diagram::Swimlanes(_))));
    }
}

mod appearance {
    use super::*;

    #[test]
    fn takes_the_redux_colour_theme_and_the_neo_look_when_nothing_is_configured() {
        let config = config_of(SWIMLANE, None);
        assert_eq!(config["theme"].as_str(), Some("redux-color"));
        assert_eq!(config["look"].as_str(), Some("neo"));
    }

    #[test]
    fn leaves_the_layout_on_elk_except_where_a_diagram_type_names_its_own() {
        assert_eq!(
            layout_for("classDiagram\n  class Duck", None).as_deref(),
            Some("elk")
        );
        assert_eq!(
            layout_for("pie\n  \"Dogs\" : 40", None).as_deref(),
            Some("elk")
        );
        // The one type that names its own is the swimlane, `layout: swimlane` in its section.
        assert_eq!(layout_for(SWIMLANE, None).as_deref(), Some("swimlane"));
    }

    #[test]
    fn a_global_look_applies_to_a_swimlane_diagram() {
        let site = Value::from(json!({ "look": "classic" }));
        assert_eq!(
            config_of(SWIMLANE, Some(&site))["look"].as_str(),
            Some("classic")
        );
    }

    #[test]
    fn a_diagram_scoped_look_beats_the_global_one_set_alongside_it() {
        let site = Value::from(json!({ "look": "classic", "swimlane": { "look": "handDrawn" } }));
        assert_eq!(
            config_of(SWIMLANE, Some(&site))["look"].as_str(),
            Some("handDrawn")
        );
    }

    #[test]
    fn frontmatter_beats_a_diagram_type_default() {
        let text = format!("---\nconfig:\n  look: classic\n---\n{SWIMLANE}");
        assert_eq!(config_of(&text, None)["look"].as_str(), Some("classic"));
    }

    #[test]
    fn the_typed_view_carries_the_swimlane_section_and_the_resolved_appearance() {
        // The facade reads the typed configuration of the diagram (`View::resolve`): the top level
        // holds the resolved appearance, and the `swimlane` section its own defaults (`config.schema.yaml`
        // 2315-2370).
        let typed = typed(&config_of(SWIMLANE, None)).expect("the configuration types");
        assert_eq!(typed.layout, "swimlane");
        assert_eq!(typed.look, "neo");
        assert_eq!(typed.theme, "redux-color");
        assert_eq!(typed.swimlane.layout, "swimlane");
        assert_eq!(typed.swimlane.look, "neo");
        assert_eq!(typed.swimlane.theme, "redux-color");
        assert!(typed.swimlane.ignore_cross_lane_edges);
        assert!(typed.swimlane.optimize_ranks_by_crossings);
        assert!(!typed.swimlane.automatic_lane_ordering);
        assert_eq!(
            typed.swimlane.line_hops,
            Some(Value::from("arc")),
            "the lane hop style is arc"
        );
    }

    #[test]
    fn a_directive_beats_a_diagram_type_default() {
        let text = format!("%%{{init: {{'look': 'classic'}}}}%%\n{SWIMLANE}");
        assert_eq!(config_of(&text, None)["look"].as_str(), Some("classic"));
    }
}
