//! Port of `packages/mermaid/src/diagrams/state/parser/state-style.spec.js` (24 tests). Test names
//! follow the spec's titles in snake_case; a title that repeats after normalising gets `_2`, `_3`, …
//! in source order. The spec's `describe` blocks are the `mod`s here.
//!
//! Each test parses the spec's text with `parse_grammar`, which runs the grammar and the DB alone,
//! as the reference's `stateDiagram.parser.parse` does.
//!
//! The three tests that read `getData()` (the `cssStyles` of the nodes after a `style` statement)
//! are ignored: the render data is not ported yet. Their bodies are comments and their names are
//! kept.

use ferrigura_model::state::{ClassDef, RelationStmt, StateDiagram, StateEntry, Statement};
use ferrigura_syntax::state::parse_grammar;

/// The state with the given id, if the diagram has one.
fn find_state<'a>(diagram: &'a StateDiagram, id: &str) -> Option<&'a StateEntry> {
    diagram.states.iter().find(|state| state.id == id)
}

/// The state with the given id. Panics when there is none.
fn state<'a>(diagram: &'a StateDiagram, id: &str) -> &'a StateEntry {
    find_state(diagram, id).unwrap_or_else(|| panic!("no state with id {id}"))
}

/// The class with the given name. Panics when there is none.
fn class<'a>(diagram: &'a StateDiagram, id: &str) -> &'a ClassDef {
    diagram
        .classes
        .iter()
        .find(|class| class.id == id)
        .unwrap_or_else(|| panic!("no class with id {id}"))
}

/// The statements of a composite state. Panics when the state has none.
fn composite_doc<'a>(diagram: &'a StateDiagram, id: &str) -> &'a [Statement] {
    state(diagram, id)
        .doc
        .as_deref()
        .unwrap_or_else(|| panic!("state {id} has no doc"))
}

/// The relation statement at `index` of a document. Panics when that statement is another kind.
fn relation_at(doc: &[Statement], index: usize) -> &RelationStmt {
    match &doc[index] {
        Statement::Relation(relation) => relation,
        other => panic!("statement {index} is not a relation: {other:?}"),
    }
}

/// Whether a document has a statement with this `id`. The spec's `d.id` finds state, style and
/// class statements, which carry an id; other statements have none.
fn doc_has_id(doc: &[Statement], id: &str) -> bool {
    doc.iter().any(|statement| match statement {
        Statement::State(stmt) => stmt.id == id,
        Statement::Style(stmt) => stmt.id == id,
        Statement::ApplyClass(stmt) => stmt.id == id,
        _ => false,
    })
}

mod defining_class_def {
    use super::*;

    #[test]
    fn has_classdef_as_a_keyword_an_id_and_can_set_a_css_style_attribute() {
        let diagram = parse_grammar("stateDiagram-v2\n classDef exampleClass background:#bbb;")
            .expect("parses");
        assert_eq!(class(&diagram, "exampleClass").styles.len(), 1);
        assert_eq!(class(&diagram, "exampleClass").styles[0], "background:#bbb");
    }

    #[test]
    fn can_define_multiple_attributes_separated_by_commas() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n classDef exampleClass background:#bbb, font-weight:bold, font-style:italic;",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exampleClass").styles.len(), 3);
        assert_eq!(class(&diagram, "exampleClass").styles[0], "background:#bbb");
        assert_eq!(
            class(&diagram, "exampleClass").styles[1],
            "font-weight:bold"
        );
        assert_eq!(
            class(&diagram, "exampleClass").styles[2],
            "font-style:italic"
        );
    }

    // need to look at what the lexer is doing
    #[test]
    fn an_attribute_can_have_a_dot_in_the_style() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n classDef exampleStyleClass background:#bbb,border:1.5px solid red;",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exampleStyleClass").styles.len(), 2);
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[0],
            "background:#bbb"
        );
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[1],
            "border:1.5px solid red"
        );
    }

    #[test]
    fn an_attribute_can_have_a_space_in_the_style() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n classDef exampleStyleClass background:  #bbb,border:1.5px solid red;",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exampleStyleClass").styles.len(), 2);
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[0],
            "background:  #bbb"
        );
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[1],
            "border:1.5px solid red"
        );
    }

    #[test]
    fn can_have_proto_or_constructor_as_a_class_name() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n classDef __proto__ background:#bbb,border:1.5px solid red;\n classDef constructor background:#bbb,border:1.5px solid red;",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "__proto__").styles.len(), 2);
        assert_eq!(class(&diagram, "constructor").styles.len(), 2);
    }
}

mod applying_class_to_states {
    use super::*;

    #[test]
    fn can_apply_a_class_to_a_state() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n\nclassDef exampleStyleClass background:#bbb,border:1px solid red;\na --> b class a exampleStyleClass",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exampleStyleClass").styles.len(), 2);
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[0],
            "background:#bbb"
        );
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[1],
            "border:1px solid red"
        );

        let state_a = state(&diagram, "a");
        assert_eq!(state_a.classes.len(), 1);
        assert_eq!(state_a.classes[0], "exampleStyleClass");
    }

    #[test]
    fn can_be_applied_to_a_state_with_an_id_containing() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n\nclassDef exampleStyleClass background:#bbb,border:1px solid red;\na_a --> b_b\nclass a_a exampleStyleClass",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exampleStyleClass").styles.len(), 2);
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[0],
            "background:#bbb"
        );
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[1],
            "border:1px solid red"
        );

        let state_a_a = state(&diagram, "a_a");
        assert_eq!(state_a_a.classes.len(), 1);
        assert_eq!(state_a_a.classes[0], "exampleStyleClass");
    }

    // `::: syntax`
    #[test]
    fn can_be_applied_to_a_state_using_syntax() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n\nclassDef exampleStyleClass background:#bbb,border:1px solid red;\na --> b:::exampleStyleClass\n",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exampleStyleClass").styles.len(), 2);
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[0],
            "background:#bbb"
        );
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[1],
            "border:1px solid red"
        );

        assert_eq!(state(&diagram, "b").classes[0], "exampleStyleClass");
    }

    #[test]
    fn can_be_applied_to_a_state() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n\nclassDef exampleStyleClass background:#bbb,border:1px solid red;\n[*]:::exampleStyleClass --> b\n",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exampleStyleClass").styles.len(), 2);
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[0],
            "background:#bbb"
        );
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[1],
            "border:1px solid red"
        );

        assert_eq!(
            state(&diagram, "root_start").classes[0],
            "exampleStyleClass"
        );
    }

    #[test]
    fn can_be_applied_to_a_comma_separated_list_of_states() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n\nclassDef exampleStyleClass background:#bbb,border:1px solid red;\na-->b\nclass a,b exampleStyleClass",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exampleStyleClass").styles.len(), 2);
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[0],
            "background:#bbb"
        );
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[1],
            "border:1px solid red"
        );
        assert_eq!(state(&diagram, "a").classes[0], "exampleStyleClass");
        assert_eq!(state(&diagram, "b").classes[0], "exampleStyleClass");
    }

    #[test]
    fn a_comma_separated_list_of_states_may_or_may_not_have_spaces_after_commas() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n\nclassDef exampleStyleClass background:#bbb,border:1px solid red;\na-->b\nclass a,b,c, d, e exampleStyleClass",
        )
        .expect("parses");
        assert_eq!(class(&diagram, "exampleStyleClass").styles.len(), 2);
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[0],
            "background:#bbb"
        );
        assert_eq!(
            class(&diagram, "exampleStyleClass").styles[1],
            "border:1px solid red"
        );

        for id in ["a", "b", "c", "d", "e"] {
            assert_eq!(state(&diagram, id).classes[0], "exampleStyleClass");
        }
    }

    // `::: syntax inside composite states`
    #[test]
    fn can_be_applied_to_a_state_inside_a_composite_state() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n\nclassDef exampleStyleClass background:#bbb,border:1px solid red;\nstate A {\n  a:::exampleStyleClass --> b\n}\n",
        )
        .expect("parses");
        let relation = relation_at(composite_doc(&diagram, "A"), 0);
        let classes = relation
            .state1
            .classes
            .as_ref()
            .expect("state1 has classes");
        assert_eq!(classes[0], "exampleStyleClass");
    }

    #[test]
    fn can_be_applied_to_a_state_inside_a_composite_state_2() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n\nclassDef exampleStyleClass background:#bbb,border:1px solid red;\nstate A {\n  [*]:::exampleStyleClass --> a\n}\n",
        )
        .expect("parses");
        let relation = relation_at(composite_doc(&diagram, "A"), 0);
        let classes = relation
            .state1
            .classes
            .as_ref()
            .expect("state1 has classes");
        assert_eq!(classes[0], "exampleStyleClass");
    }
}

mod comments_parsing {
    use super::*;

    #[test]
    fn working_inside_states() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n\n[*] --> Moving\nMoving --> Still\nMoving --> Crash\nstate Moving {\n%% comment inside state\nslow  --> fast\n}\n",
        )
        .expect("parses");
        assert_eq!(composite_doc(&diagram, "Moving").len(), 1);
    }

    #[test]
    fn should_handle_comments_correctly() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n%%initial_comment\n[*] --> Moving %%inline_comment\n%%final_comment\n",
        )
        .expect("parses");
        assert_eq!(diagram.states.len(), 2);
        assert!(find_state(&diagram, "%%initial_comment").is_none());
        assert!(find_state(&diagram, "%%inline_comment").is_none());
        assert!(find_state(&diagram, "%%final_comment").is_none());
    }

    #[test]
    fn should_handle_comments_correctly_inside_states() {
        let diagram = parse_grammar(
            "stateDiagram-v2\nstate Moving {\n%%comment_inside_state\nslow  --> fast %%inline_comment\n}\n",
        )
        .expect("parses");
        let moving_doc = composite_doc(&diagram, "Moving");
        assert!(!doc_has_id(moving_doc, "%%comment_inside_state"));
        assert!(!doc_has_id(moving_doc, "%%inline_comment"));
    }

    #[test]
    fn should_handle_comments_correctly_after_a_blank_line() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n[*] --> Moving\n\n%% comment after a blank line\nMoving --> Still\n",
        )
        .expect("parses");
        assert_eq!(diagram.states.len(), 3);
    }

    #[test]
    fn should_parse_single_as_normal_syntax_not_a_comment() {
        let diagram = parse_grammar("stateDiagram-v2\n% not a comment\nMoving --> Still %inline\n")
            .expect("parses");
        assert_eq!(diagram.states.len(), 7);
        assert!(find_state(&diagram, "%inline").is_some());
    }

    #[test]
    fn should_skip_multiple_consecutive_comment_lines() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n%% first comment\n%% second comment\n%% third comment\n[*] --> Moving\n",
        )
        .expect("parses");
        assert_eq!(diagram.states.len(), 2);
    }

    #[test]
    fn transitions_should_be_preserved_when_separated_by_a_comment() {
        let diagram = parse_grammar(
            "stateDiagram-v2\nMoving --> Crash\n%% comment between\nMoving --> Still\n",
        )
        .expect("parses");
        assert_eq!(diagram.states.len(), 3);
        assert_eq!(diagram.relations.len(), 2);
        assert_eq!(diagram.relations[0].id1, "Moving");
        assert_eq!(diagram.relations[0].id2, "Crash");
        assert_eq!(diagram.relations[1].id1, "Moving");
        assert_eq!(diagram.relations[1].id2, "Still");
    }

    #[test]
    fn should_correctly_handle_inline_comments_in_state_syntax() {
        let diagram = parse_grammar(
            "stateDiagram-v2\n[*] --> Moving%% this is a comment\nMoving --> Still%Active\n",
        )
        .expect("parses");
        assert!(find_state(&diagram, "Moving").is_some());
        assert!(find_state(&diagram, "Still%Active").is_some());
    }
}

mod defining_style {
    // The spec reads `getData().nodes[..].cssStyles` after each `style` line below. That is render
    // data, so the expectations are not ported here.

    #[test]
    #[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
    fn has_style_as_a_keyword_an_id_and_can_set_a_css_style_attribute() {
        // getData: nodes[0].cssStyles after `style id1 background:#bbb`.
    }

    #[test]
    #[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
    fn has_style_as_a_keyword_an_id_and_can_set_a_css_style_attribute_2() {
        // getData: nodes[0] and nodes[1].cssStyles after `style id1,id2 background:#bbb`.
    }

    #[test]
    #[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
    fn can_define_multiple_attributes_separated_by_commas_2() {
        // getData: nodes[0] and nodes[1].cssStyles after `style id1,id2` with three attributes.
    }
}
