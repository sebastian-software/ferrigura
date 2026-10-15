//! Port of `packages/mermaid/src/diagrams/info/info.spec.ts`.
//!
//! The reference asserts the exact Jison-style error text of the rejected input; this port
//! asserts only that parsing fails and where, as `docs-port/decisions/0002-hand-written-parsers.md`
//! decides.

use ferrigura_syntax::info::parse;

#[test]
fn should_handle_an_info_definition() {
    parse("info").expect("an info definition parses");
}

#[test]
fn should_handle_an_info_definition_with_show_info() {
    parse("info showInfo").expect("an info definition with showInfo parses");
}

#[test]
fn should_throw_because_of_unsupported_info_grammar() {
    let error = parse("info unsupported").expect_err("an unknown word is rejected");
    assert_eq!((error.line, error.column), (1, 6));
}

#[test]
fn should_throw_because_of_unsupported_info_grammar_2() {
    let error = parse("info unsupported").expect_err("an unknown word is rejected");
    assert_eq!((error.line, error.column), (1, 6));
}
