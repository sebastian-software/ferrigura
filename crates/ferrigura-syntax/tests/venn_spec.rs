//! Port of `packages/mermaid/src/diagrams/venn/parser/venn.spec.ts`. The reference reads the DB
//! through `venn.parse`, so each test parses a text and checks the state the DB is left in, which
//! is the model. The texts are the spec's own, indentation included, and the test names follow
//! the spec's titles in snake_case.

use ferrigura_model::venn::{StyleEntry, Subset, TextNode, VennDiagram};
use ferrigura_syntax::venn::parse;

fn parsed(text: &str) -> VennDiagram {
    parse(text).expect("parses")
}

fn subset(sets: &[&str], size: f64, label: &str) -> Subset {
    Subset {
        sets: sets.iter().map(|set| (*set).to_owned()).collect(),
        size,
        label: label.to_owned(),
    }
}

fn text_node(sets: &[&str], id: &str, label: &str) -> TextNode {
    TextNode {
        sets: sets.iter().map(|set| (*set).to_owned()).collect(),
        id: id.to_owned(),
        label: label.to_owned(),
    }
}

fn style(targets: &[&str], properties: &[(&str, &str)]) -> StyleEntry {
    StyleEntry {
        targets: targets.iter().map(|target| (*target).to_owned()).collect(),
        styles: properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    }
}

#[test]
fn simple() {
    let diagram = parsed(
        r#"venn-beta
          title foo bar
          set A
          set B
          union A,B
      "#,
    );
    assert_eq!(diagram.title, "foo bar");
    assert_eq!(
        diagram.subsets,
        [
            subset(&["A"], 10.0, ""),
            subset(&["B"], 10.0, ""),
            subset(&["A", "B"], 2.5, ""),
        ]
    );
}

#[test]
fn with_bracket_labels() {
    let diagram = parsed(
        r#"venn-beta
          set A["Alpha"]
          set B["Beta"]
          union A,B["AB"]
      "#,
    );
    assert_eq!(
        diagram.subsets,
        [
            subset(&["A"], 10.0, "Alpha"),
            subset(&["B"], 10.0, "Beta"),
            subset(&["A", "B"], 2.5, "AB"),
        ]
    );
}

#[test]
fn with_unquoted_bracket_labels() {
    let diagram = parsed(
        r#"venn-beta
          set A[Alpha]
          set B[Beta]
          union A,B[AB]
      "#,
    );
    assert_eq!(
        diagram.subsets,
        [
            subset(&["A"], 10.0, "Alpha"),
            subset(&["B"], 10.0, "Beta"),
            subset(&["A", "B"], 2.5, "AB"),
        ]
    );
}

#[test]
fn with_size_suffix() {
    let diagram = parsed(
        r#"venn-beta
          set A:20
          set B:12
          union A,B:3
      "#,
    );
    assert_eq!(
        diagram.subsets,
        [
            subset(&["A"], 20.0, ""),
            subset(&["B"], 12.0, ""),
            subset(&["A", "B"], 3.0, ""),
        ]
    );
}

#[test]
fn with_bracket_label_and_size_suffix() {
    let diagram = parsed(
        r#"venn-beta
          title foo bar
          set A["Alpha"]:20
          set B["Beta"]:12
          set C["Gamma"]:30
          union A,B["AB"]:5.3
          union C,A,B:1
      "#,
    );
    assert_eq!(
        diagram.subsets,
        [
            subset(&["A"], 20.0, "Alpha"),
            subset(&["B"], 12.0, "Beta"),
            subset(&["C"], 30.0, "Gamma"),
            subset(&["A", "B"], 5.3, "AB"),
            subset(&["A", "B", "C"], 1.0, ""),
        ]
    );
}

#[test]
fn with_text_nodes_using_bracket_labels() {
    let diagram = parsed(
        r#"venn-beta
          set A
            text A1["foo bar"]
          set B
            text B1["hello, world"]
          union A,B
            text AB1["shared note"]
      "#,
    );
    assert_eq!(
        diagram.text_nodes,
        [
            text_node(&["A"], "A1", "foo bar"),
            text_node(&["B"], "B1", "hello, world"),
            text_node(&["A", "B"], "AB1", "shared note"),
        ]
    );
}

#[test]
fn with_indented_text_nodes_no_label() {
    let diagram = parsed(
        r#"venn-beta
          set A["Frontend"]
            text A1
            text A2
          set B["Backend"]
            text B1
          union A,B["APIs"]
      "#,
    );
    assert_eq!(
        diagram.text_nodes,
        [
            text_node(&["A"], "A1", ""),
            text_node(&["A"], "A2", ""),
            text_node(&["B"], "B1", ""),
        ]
    );
}

#[test]
fn with_style_statement_for_single_set() {
    let diagram = parsed(
        r#"venn-beta
          set A
          set B
          union A,B
          style A fill:#ff6b6b
      "#,
    );
    assert_eq!(diagram.styles, [style(&["A"], &[("fill", "#ff6b6b")])]);
}

#[test]
fn with_style_statement_for_intersection() {
    let diagram = parsed(
        r#"venn-beta
          set A
          set B
          union A,B
          style A,B color:#333
      "#,
    );
    assert_eq!(diagram.styles, [style(&["A", "B"], &[("color", "#333")])]);
}

#[test]
fn with_style_statement_for_text_node() {
    let diagram = parsed(
        r#"venn-beta
          set A
            text A1["React"]
          style A1 color:red
      "#,
    );
    assert_eq!(diagram.styles, [style(&["A1"], &[("color", "red")])]);
}

#[test]
fn with_multiple_style_properties() {
    let diagram = parsed(
        r#"venn-beta
          set A
          set B
          union A,B
          style A fill:#ff6b6b, color:#333
      "#,
    );
    assert_eq!(
        diagram.styles,
        [style(&["A"], &[("fill", "#ff6b6b"), ("color", "#333")])]
    );
}

#[test]
fn style_with_rgb_rgba_colors() {
    let diagram = parsed(
        r#"venn-beta
          set A
          set B
          style A fill:rgb(255, 0, 128)
          style B fill:rgba(255, 0, 128, 0.5)
      "#,
    );
    assert_eq!(
        diagram.styles,
        [
            style(&["A"], &[("fill", "rgb(255, 0, 128)")]),
            style(&["B"], &[("fill", "rgba(255, 0, 128, 0.5)")]),
        ]
    );
}

#[test]
fn indent_mode_with_style_coexistence() {
    let diagram = parsed(
        r#"venn-beta
          set A["Frontend"]:20
            text A1["React"]
          set B["Backend"]:12
          union A,B["Shared"]:3
            text AB1["OpenAPI"]
          style A fill:#ff6b6b
          style A,B color:#333
          style A1 color:red
      "#,
    );
    assert_eq!(
        diagram.subsets,
        [
            subset(&["A"], 20.0, "Frontend"),
            subset(&["B"], 12.0, "Backend"),
            subset(&["A", "B"], 3.0, "Shared"),
        ]
    );
    assert_eq!(
        diagram.text_nodes,
        [
            text_node(&["A"], "A1", "React"),
            text_node(&["A", "B"], "AB1", "OpenAPI"),
        ]
    );
    assert_eq!(
        diagram.styles,
        [
            style(&["A"], &[("fill", "#ff6b6b")]),
            style(&["A", "B"], &[("color", "#333")]),
            style(&["A1"], &[("color", "red")]),
        ]
    );
}

#[test]
fn set_requires_single_identifier() {
    let error = parse("venn-beta\n        set A,B\n    ").expect_err("is rejected");
    assert_eq!(error.line, 2);
}

#[test]
fn union_requires_multiple_identifiers() {
    let error = parse(
        r#"venn-beta
        union A
    "#,
    )
    .expect_err("is rejected");
    assert_eq!(error.message, "union requires multiple identifiers");
}

#[test]
fn union_requires_known_identifiers() {
    let error = parse(
        r#"venn-beta
        set Foo
        union Foo,Buz
    "#,
    )
    .expect_err("is rejected");
    assert!(error.message.starts_with("unknown set identifier"));
}

#[test]
fn quoted_identifiers() {
    let diagram = parsed(
        r#"venn-beta
        set "Foo Bar"
        set Buz
        union "Foo Bar",Buz
    "#,
    );
    assert_eq!(
        diagram.subsets,
        [
            subset(&["Foo Bar"], 10.0, ""),
            subset(&["Buz"], 10.0, ""),
            subset(&["Buz", "Foo Bar"], 2.5, ""),
        ]
    );
}
