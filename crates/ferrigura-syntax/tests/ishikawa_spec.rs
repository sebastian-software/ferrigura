//! Port of `packages/mermaid/src/diagrams/ishikawa/ishikawa.spec.ts`. Test names follow the spec's
//! titles in snake_case. The spec runs the grammar on the text as it is, and the port runs the whole
//! `Diagram.fromText` path; the three texts have no front matter, directive or comment, so both read
//! the same text.

use ferrigura_model::ishikawa::{IshikawaDiagram, IshikawaNode};
use ferrigura_syntax::ishikawa::parse;

fn parsed(text: &str) -> IshikawaDiagram {
    parse(text).expect("parses")
}

fn root_of(diagram: &IshikawaDiagram) -> &IshikawaNode {
    diagram.root.as_ref().expect("a root")
}

#[test]
fn should_parse_a_basic_ishikawa_hierarchy() {
    let diagram = parsed(
        "ishikawa-beta\n    Blurry Photo\n        Process\n            Out of focus\n        User\n            Shaky hands\n",
    );
    let root = root_of(&diagram);
    assert_eq!(root.text, "Blurry Photo");
    assert_eq!(root.children.len(), 2);
    assert_eq!(root.children[0].text, "Process");
    assert_eq!(root.children[0].children[0].text, "Out of focus");
    assert_eq!(root.children[1].text, "User");
    assert_eq!(root.children[1].children[0].text, "Shaky hands");
}

#[test]
fn should_support_an_unindented_root_with_nested_causes() {
    let diagram = parsed("ishikawa-beta\nProblem\nCause A\n  Subcause A1\nCause B\n");
    let root = root_of(&diagram);
    assert_eq!(root.text, "Problem");
    assert_eq!(root.children.len(), 2);
    assert_eq!(root.children[0].text, "Cause A");
    assert_eq!(root.children[0].children[0].text, "Subcause A1");
    assert_eq!(root.children[1].text, "Cause B");
}

#[test]
fn should_handle_effect_indented_more_than_causes() {
    let diagram = parsed("ishikawa-beta\n    Problem\nCause A\n  Subcause A1\nCause B\n");
    let root = root_of(&diagram);
    assert_eq!(root.text, "Problem");
    assert_eq!(root.children.len(), 2);
    assert_eq!(root.children[0].text, "Cause A");
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].children[0].text, "Subcause A1");
    assert_eq!(root.children[1].text, "Cause B");
}
