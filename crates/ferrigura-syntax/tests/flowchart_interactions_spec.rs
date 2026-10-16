//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-interactions.spec.js`. Test names
//! follow the spec's titles in snake_case.
//!
//! Every test here is an interaction (`click`, `href`), so all are ignored until step 6. The spec's
//! spy assertions become outcome assertions on the model, as `docs-port/flowchart/PLAN.md` section
//! 10.2 decides: the link, its target, the tooltip. A click callback is not in the model (PLAN 13.4),
//! so the callback tests check only that the text parses.

use ferrigura_model::flowchart::{FlowVertex, FlowchartDiagram};
use ferrigura_syntax::flowchart::parse;

fn vertex<'a>(diagram: &'a FlowchartDiagram, id: &str) -> &'a FlowVertex {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex {id}"))
}

fn has_tooltip(diagram: &FlowchartDiagram, id: &str, tooltip: &str) -> bool {
    diagram
        .tooltips
        .iter()
        .any(|entry| entry.id == id && entry.tooltip == tooltip)
}

mod interactions_when_parsing {
    use super::*;

    #[test]
    fn should_be_possible_to_use_click_to_a_callback() {
        parse("graph TD\nA-->B\nclick A callback").expect("parses");
    }

    #[test]
    fn should_be_possible_to_use_click_to_a_click_and_call_callback() {
        parse("graph TD\nA-->B\nclick A call callback()").expect("parses");
    }

    #[test]
    fn should_be_possible_to_use_click_to_a_callback_with_tooltip() {
        let diagram = parse("graph TD\nA-->B\nclick A callback \"tooltip\"").expect("parses");

        assert!(has_tooltip(&diagram, "A", "tooltip"));
    }

    #[test]
    fn should_be_possible_to_use_click_to_a_click_and_call_callback_with_tooltip() {
        let diagram =
            parse("graph TD\nA-->B\nclick A call callback() \"tooltip\"").expect("parses");

        assert!(has_tooltip(&diagram, "A", "tooltip"));
    }

    #[test]
    fn should_be_possible_to_use_click_to_a_callback_with_an_arbitrary_number_of_args() {
        parse("graph TD\nA-->B\nclick A call callback(\"test0\", test1, test2)").expect("parses");
    }

    #[test]
    fn should_handle_interaction_click_to_a_link() {
        let diagram = parse("graph TD\nA-->B\nclick A \"click.html\"").expect("parses");

        assert_eq!(vertex(&diagram, "A").link.as_deref(), Some("click.html"));
    }

    #[test]
    fn should_handle_interaction_click_to_a_click_and_href_link() {
        let diagram = parse("graph TD\nA-->B\nclick A href \"click.html\"").expect("parses");

        assert_eq!(vertex(&diagram, "A").link.as_deref(), Some("click.html"));
    }

    #[test]
    fn should_handle_interaction_click_to_a_link_with_tooltip() {
        let diagram = parse("graph TD\nA-->B\nclick A \"click.html\" \"tooltip\"").expect("parses");

        assert_eq!(vertex(&diagram, "A").link.as_deref(), Some("click.html"));
        assert!(has_tooltip(&diagram, "A", "tooltip"));
    }

    #[test]
    fn should_handle_interaction_click_to_a_click_and_href_link_with_tooltip() {
        let diagram =
            parse("graph TD\nA-->B\nclick A href \"click.html\" \"tooltip\"").expect("parses");

        assert_eq!(vertex(&diagram, "A").link.as_deref(), Some("click.html"));
        assert!(has_tooltip(&diagram, "A", "tooltip"));
    }

    #[test]
    fn should_handle_interaction_click_to_a_link_with_target() {
        let diagram = parse("graph TD\nA-->B\nclick A \"click.html\" _blank").expect("parses");

        assert_eq!(vertex(&diagram, "A").link.as_deref(), Some("click.html"));
        assert_eq!(vertex(&diagram, "A").link_target.as_deref(), Some("_blank"));
    }

    #[test]
    fn should_handle_interaction_click_to_a_click_and_href_link_with_target() {
        let diagram = parse("graph TD\nA-->B\nclick A href \"click.html\" _blank").expect("parses");

        assert_eq!(vertex(&diagram, "A").link.as_deref(), Some("click.html"));
        assert_eq!(vertex(&diagram, "A").link_target.as_deref(), Some("_blank"));
    }

    #[test]
    fn should_handle_interaction_click_to_a_link_with_tooltip_and_target() {
        let diagram =
            parse("graph TD\nA-->B\nclick A \"click.html\" \"tooltip\" _blank").expect("parses");

        assert_eq!(vertex(&diagram, "A").link.as_deref(), Some("click.html"));
        assert_eq!(vertex(&diagram, "A").link_target.as_deref(), Some("_blank"));
        assert!(has_tooltip(&diagram, "A", "tooltip"));
    }

    #[test]
    fn should_handle_interaction_click_to_a_click_and_href_link_with_tooltip_and_target() {
        let diagram = parse("graph TD\nA-->B\nclick A href \"click.html\" \"tooltip\" _blank")
            .expect("parses");

        assert_eq!(vertex(&diagram, "A").link.as_deref(), Some("click.html"));
        assert_eq!(vertex(&diagram, "A").link_target.as_deref(), Some("_blank"));
        assert!(has_tooltip(&diagram, "A", "tooltip"));
    }
}
