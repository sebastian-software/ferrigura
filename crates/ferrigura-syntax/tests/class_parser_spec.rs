//! Port of packages/mermaid/src/diagrams/class/parser/class.spec.js (26 lines). Test names follow
//! the spec's titles in snake_case.
//!
//! The spec checks that the parser accepts prototype names and backtick-quoted namespace names. It
//! asserts only that the text does not throw, so each test asserts only that the text parses.

use ferrigura_syntax::class::parse_grammar;

fn assert_parses(text: &str) {
    if let Err(error) = parse_grammar(text) {
        panic!("{text:?} does not parse: {error:?}");
    }
}

/// The two texts of the `it.each` body, for one prototype property name.
fn assert_prototype_property_parses(prop: &str) {
    assert_parses(&format!("classDiagram\nclass {prop}"));
    assert_parses(&format!("classDiagram\nnamespace {prop} {{\n\tclass A\n}}"));
}

#[test]
fn should_work_with_a_proto_property() {
    assert_prototype_property_parses("__proto__");
}

#[test]
fn should_work_with_a_constructor_property() {
    assert_prototype_property_parses("constructor");
}

#[test]
fn should_handle_backtick_quoted_namespace_names() {
    assert_parses("classDiagram\nnamespace `A::B` {\n\tclass `IPC::Sender`\n}");
}
