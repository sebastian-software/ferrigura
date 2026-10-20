//! The parser decisions of the swimlane diagram, each with its probe in the reference
//! (`docs-port/swimlanes/PLAN.md`, section 2). The reference has no parser spec of its own for the
//! type: its text goes through the flowchart's grammar (`flow.jison`, 116), and the lanes are the
//! flowchart's subgraphs. The expected values are the reference's own outputs for the same texts,
//! read from `tools/port/export-swimlanes.ts`, and the differential run (`pnpm port:diff swimlanes`)
//! compares every model on the whole collection.

use ferrigura_model::flowchart::FlowchartDiagram;
use ferrigura_syntax::swimlanes::parse;
use ferrigura_syntax::{Diagram, flowchart};

/// The lane titles and members, in the order the DB holds them.
fn lanes(diagram: &FlowchartDiagram) -> Vec<(&str, &str, Vec<&str>)> {
    diagram
        .subgraphs
        .iter()
        .map(|lane| {
            (
                lane.id.as_str(),
                lane.title.as_str(),
                lane.nodes.iter().map(String::as_str).collect(),
            )
        })
        .collect()
}

#[test]
fn the_header_is_a_graph_keyword_with_its_direction() {
    // `flow.jison` 116 and the `dir` state; `TD` is stored as `TB` (`flowDb.ts`, `setDirection`).
    assert_eq!(parse("swimlane-beta LR\n  a-->b").unwrap().direction, "LR");
    assert_eq!(parse("swimlane-beta TD;A-->B;").unwrap().direction, "TB");
    assert_eq!(parse("swimlane-beta\n  a-->b").unwrap().direction, "TB");
    assert_eq!(parse("swimlane-beta >\n  a-->b").unwrap().direction, "LR");
    assert_eq!(parse("swimlane-beta BR\n  a-->b").unwrap().direction, "BR");
}

#[test]
fn the_model_is_the_flowchart_model_of_the_same_text() {
    // The reference's swimlane DB is the flowchart's (`swimlanesDiagram.ts`, `createFlowDiagram`),
    // so the two parsers give the same model for the same text.
    let text =
        "swimlane-beta LR\n  subgraph A [Lane A]\n    a1[Start] --> a2(Next)\n  end\n  a2 --> b";
    assert_eq!(
        parse(text).ok(),
        flowchart::parse(text).ok(),
        "the swimlane model is the flowchart's"
    );
}

#[test]
fn a_lane_is_a_subgraph_with_its_title_and_its_members_in_chain_order() {
    // A chain's later nodes come first in a subgraph's `nodes` (canonical-model.md, Flowchart).
    let diagram = parse(
        "swimlane-beta LR\n  subgraph A [Lane A]\n    a1[Start] --> a2(Next)\n  end\n  subgraph B [Lane B]\n    b1{Check} --> b2((Done))\n  end\n  a2 --> b1",
    )
    .expect("parses");
    assert_eq!(
        lanes(&diagram),
        vec![
            ("A", "Lane A", vec!["a2", "a1"]),
            ("B", "Lane B", vec!["b2", "b1"])
        ]
    );
    assert_eq!(diagram.edges.len(), 3);
}

#[test]
fn a_nested_lane_closes_first_and_its_parent_lists_it() {
    let diagram = parse("swimlane-beta TD\n  subgraph A\n    subgraph B\n      x\n    end\n  end")
        .expect("parses");
    assert_eq!(
        lanes(&diagram),
        vec![("B", "B", vec!["x"]), ("A", "A", vec!["B"])]
    );
    assert!(diagram.edges.is_empty());
}

#[test]
fn a_repeated_lane_id_merges_into_its_first_position() {
    let diagram = parse("swimlane-beta TD\n  subgraph A\n    a\n  end\n  subgraph A\n    b\n  end")
        .expect("parses");
    assert_eq!(lanes(&diagram), vec![("A", "A", vec!["a", "b"])]);
}

#[test]
fn a_node_outside_every_lane_is_a_vertex_in_no_lane() {
    // The layout puts such a node in a synthesised lane (`helpers.ts`); the DB has no such lane.
    let diagram = parse("swimlane-beta TD\n  loose\n  subgraph A\n    a\n  end").expect("parses");
    assert_eq!(
        diagram
            .vertices
            .iter()
            .map(|vertex| vertex.id.as_str())
            .collect::<Vec<_>>(),
        vec!["loose", "a"]
    );
    assert_eq!(lanes(&diagram), vec![("A", "A", vec!["a"])]);
}

#[test]
fn a_lane_without_an_id_and_an_unclosed_lane_are_rejected() {
    // The reference throws a TypeError for the first (`subgraph` with no id, `flowDb.ts`) and a
    // parse error for the second; ferrigura's messages are its own (decision 0002).
    assert!(parse("swimlane-beta TD\n  subgraph\n  a\n  end").is_err());
    assert!(parse("swimlane-beta TD\n  subgraph A\n  a --> b").is_err());
    assert!(parse("swimlane-beta TD\n  end").is_err());
}

#[test]
fn more_than_500_edges_are_rejected() {
    // `maxEdges` is 500 (`flowDb.ts:330`); the 501st edge is an error on both sides.
    let edges = vec!["a-->b"; 500].join("\n");
    assert!(parse(&format!("swimlane-beta TD\n{edges}")).is_ok());
    let more = vec!["a-->b"; 501].join("\n");
    assert!(parse(&format!("swimlane-beta TD\n{more}")).is_err());
}

#[test]
fn a_link_without_an_end_and_a_bad_shape_are_rejected() {
    assert!(parse("swimlane-beta TD\n  a -->").is_err());
    assert!(parse("swimlane-beta TD\n  a@{ shape: foo } --> b").is_err());
}

#[test]
fn the_swimlane_keyword_is_only_a_header_at_the_start_of_the_text() {
    // The `swimlane-beta` token is a graph word in the flowchart's scanner (`flowchart/scanner.rs`),
    // as it is in `flow.jison`; the detector, not the parser, decides the diagram type.
    assert!(parse("swimlane-beta TD\n  swimlane-beta --> b").is_err());
}

#[test]
fn the_detected_swimlane_text_gives_the_swimlane_variant() {
    assert!(matches!(
        ferrigura_syntax::parse("swimlane-beta TD\n  a-->b"),
        Ok(Diagram::Swimlanes(_))
    ));
}
