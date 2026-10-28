//! Port of `packages/mermaid/src/diagrams/flowchart/parser/flow-huge.spec.js`, plus a check of
//! the parser's cost on the same input.
//!
//! The reference's test is skipped, and as written it could not pass: it draws 47,917 edges without
//! raising `maxEdges`, whose default is 500, so the reference throws at the 501st edge. The port
//! raises the limit for the input, which keeps the assertions of the spec. The reference's
//! skip also says the test takes minutes, because its edge bookkeeping is quadratic; the port
//! keeps the same input, not skipped, so a quadratic regression shows as a slow test.

use ferrigura_syntax::flowchart::Options;

/// The input of the reference's test: 47,917 edges between two vertices.
fn huge_input() -> String {
    let nodes = "A-->B;B-->A;".repeat(415) + "A-->B;";
    let block = nodes.repeat(57) + &"A-->B;B-->A;".repeat(275);
    format!("graph LR;{block}")
}

/// `maxEdges` raised above the input's edge count.
fn raised_limit() -> Options {
    Options { max_edges: 100_000 }
}

mod text {
    use super::*;

    #[test]
    #[ignore = "port: skipped upstream: takes minutes, and exceeds the default maxEdges as written"]
    fn it_should_handle_huge_diagrams() {
        let (diagram, _) = parse_with_warnings_for_test();
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
        assert_eq!(diagram.edges.len(), 47_917);
        assert_eq!(diagram.vertices.len(), 2);
    }

    /// Not in the reference: the same input and assertions, not skipped, so the parser's cost is
    /// checked on every run. There is no timing assertion.
    #[test]
    fn it_should_handle_huge_diagrams_in_linear_time() {
        let (diagram, _) = parse_with_warnings_for_test();
        assert_eq!(diagram.edges[0].edge_type, "arrow_point");
        assert_eq!(diagram.edges.len(), 47_917);
        assert_eq!(diagram.vertices.len(), 2);
    }

    fn parse_with_warnings_for_test() -> (ferrigura_model::flowchart::FlowchartDiagram, Vec<String>)
    {
        ferrigura_syntax::flowchart::parse_with_warnings(&huge_input(), raised_limit())
            .expect("parses")
    }
}
