//! The ishikawa DB's behaviour, through `parse`, as `ishikawaDb.ts` and the common DB give it:
//! the placement of causes by their level, the title, and the sanitising of the texts. See
//! `docs-port/ishikawa/PLAN.md`, section 3.
//!
//! The DB has no public surface, so each test drives `parse`. The reference's `clear` runs before
//! every parse (`Diagram.fromText`), so each parse here starts from an empty DB.

use ferrigura_model::ishikawa::{IshikawaDiagram, IshikawaNode};
use ferrigura_syntax::ishikawa::parse;

fn parsed(text: &str) -> IshikawaDiagram {
    parse(text).unwrap_or_else(|error| panic!("{text:?} should parse: {error}"))
}

/// The tree as text: a node's text, then its children in brackets, separated by spaces.
fn shape(node: &IshikawaNode) -> String {
    if node.children.is_empty() {
        return node.text.clone();
    }
    let children: Vec<String> = node.children.iter().map(shape).collect();
    format!("{}({})", node.text, children.join(" "))
}

fn tree_of(text: &str) -> String {
    parsed(text).root.as_ref().map(shape).unwrap_or_default()
}

#[test]
fn should_set_the_title_to_the_head_text() {
    assert_eq!(
        parsed("ishikawa\nBlurry Photo\n  a\n").title,
        "Blurry Photo"
    );
}

#[test]
fn should_keep_the_front_matter_title_when_there_is_no_head() {
    let diagram = parsed("---\ntitle: FM\n---\nishikawa\n");
    assert_eq!(diagram.title, "FM");
    assert!(diagram.root.is_none());
}

#[test]
fn should_replace_the_front_matter_title_with_the_head_text() {
    assert_eq!(
        parsed("---\ntitle: FM\n---\nishikawa\nRoot D\n").title,
        "Root D"
    );
}

#[test]
fn should_set_an_empty_title_for_a_head_that_sanitises_to_nothing() {
    let diagram = parsed("---\ntitle: FM\n---\nishikawa\n<script></script>\n");
    assert_eq!(diagram.title, "");
    assert_eq!(diagram.root.as_ref().map(shape), Some(String::new()));
}

#[test]
fn should_start_each_parse_from_an_empty_db() {
    parsed("ishikawa\nRoot A\n  x\n");
    let diagram = parsed("ishikawa\n");
    assert_eq!(diagram.title, "");
    assert!(diagram.root.is_none());
}

#[test]
fn should_place_causes_by_their_level_with_pops() {
    assert_eq!(
        tree_of("ishikawa\nP\n  a\n    x\n      y\n  b\n    z\nQ\n    r\n"),
        "P(a(x(y)) b(z) Q(r))"
    );
}

#[test]
fn should_take_the_base_level_from_the_first_cause() {
    assert_eq!(tree_of("ishikawa\nP\n a\n  b\n c\n"), "P(a(b) c)");
}

#[test]
fn should_keep_the_order_of_the_causes() {
    assert_eq!(tree_of("ishikawa\nP\n  c\n  a\n  b\n"), "P(c a b)");
}

#[test]
fn should_sanitise_the_head_and_the_causes() {
    assert_eq!(
        tree_of("ishikawa\nP & <b>x</b>\n  <script>x</script>y\n"),
        "P &amp; <b>x</b>(y)"
    );
}

#[test]
fn should_serialise_a_missing_root_as_null() {
    let value = serde_json::to_value(parsed("ishikawa\n")).expect("serialises");
    assert!(value["root"].is_null());
    assert_eq!(value["version"], 1);
    assert_eq!(value["accTitle"], "");
    assert_eq!(value["accDescr"], "");
}

#[test]
fn should_serialise_a_node_with_its_children() {
    let value = serde_json::to_value(parsed("ishikawa\nP\n  a\n")).expect("serialises");
    assert_eq!(value["root"]["text"], "P");
    assert_eq!(value["root"]["children"][0]["text"], "a");
    assert_eq!(
        value["root"]["children"][0]["children"],
        serde_json::json!([])
    );
}
