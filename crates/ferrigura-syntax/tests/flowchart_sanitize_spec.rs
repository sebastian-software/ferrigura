//! The text that the reference sanitises at parse time (`flowDb.ts`: node text at 201, edge text at
//! 301, subgraph titles at 736), with DOMPurify's output as the expected value. The `@{ label }`
//! text is not sanitised, as the reference does not sanitise it (`flowDb.ts` 245-248).

use ferrigura_model::flowchart::FlowchartDiagram;
use ferrigura_syntax::flowchart::parse_grammar;

fn parse(text: &str) -> FlowchartDiagram {
    parse_grammar(text).expect("the diagram parses")
}

fn vertex_text(diagram: &FlowchartDiagram, id: &str) -> String {
    diagram
        .vertices
        .iter()
        .find(|vertex| vertex.id == id)
        .unwrap_or_else(|| panic!("no vertex {id}"))
        .text
        .clone()
}

/// `nightly-check/v2/03-should-render-raw-strings`: `["<11"]` is text, so DOMPurify writes its `<` as
/// `&lt;`, and the label is drawn as `<11`.
#[test]
fn a_bracket_label_with_a_less_than_is_sanitised() {
    let diagram = parse("flowchart TB\n  v10[\"<11\"] --> fine[\"No bug\"]");
    assert_eq!(vertex_text(&diagram, "v10"), "&lt;11");
    assert_eq!(vertex_text(&diagram, "fine"), "No bug");
}

/// `visual-check/5b-angle-brackets`: `>`, `&&` and `<` in a plain label become entities.
#[test]
fn a_bracket_label_with_comparison_operators_is_sanitised() {
    let diagram = parse("flowchart TD\n  a[\"Plain text:\n  5 > 3 && 2 < 4\"]");
    assert_eq!(
        vertex_text(&diagram, "a"),
        "Plain text:\n  5 &gt; 3 &amp;&amp; 2 &lt; 4"
    );
}

/// `visual-check/4023-should-render-html-labels-with-images`: an image tag keeps its source in
/// double quotes.
#[test]
fn a_bracket_label_with_an_image_keeps_the_image_in_double_quotes() {
    let diagram = parse("flowchart TD\n  B[<img src='https://mermaid.js.org/mermaid-logo.svg'>]");
    assert_eq!(
        vertex_text(&diagram, "B"),
        "<img src=\"https://mermaid.js.org/mermaid-logo.svg\">"
    );
}

/// The quotes are stripped after the sanitising, as `flowDb.ts` does it.
#[test]
fn a_quoted_label_is_sanitised_and_then_unquoted() {
    let diagram = parse("flowchart TD\n  A[\"x < y\"]");
    assert_eq!(vertex_text(&diagram, "A"), "x &lt; y");
}

#[test]
fn an_edge_text_is_sanitised() {
    let diagram = parse("flowchart TD\n  A -- \"a < b\" --> B");
    assert_eq!(diagram.edges.len(), 1);
    assert_eq!(diagram.edges[0].text, "a &lt; b");
}

#[test]
fn a_subgraph_title_is_sanitised_then_trimmed() {
    let diagram = parse("flowchart TD\n  subgraph s1 [\"<b>x</b> & y\"]\n    A\n  end");
    assert_eq!(diagram.subgraphs.len(), 1);
    assert_eq!(diagram.subgraphs[0].title, "<b>x</b> &amp; y");
}

/// `@{ label: ... }` is taken as it is written, with no sanitising.
#[test]
fn a_shape_data_label_is_not_sanitised() {
    let diagram = parse("flowchart TB\n  D@{ label: \"<11\" }");
    assert_eq!(vertex_text(&diagram, "D"), "<11");
}
