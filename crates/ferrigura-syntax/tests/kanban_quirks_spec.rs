//! The quirks of the kanban grammar and DB that the port reproduces, each as a direct assertion.
//! The list and its reasons are in `docs-port/kanban/PLAN.md`, section 2.5 and 3.6. The
//! differential run (`pnpm port:diff kanban`) checks the same behaviour against the reference.

use ferrigura_model::kanban::KanbanNode;
use ferrigura_model::value::Value;
use ferrigura_syntax::kanban::parse;

/// The node with the given id, which must exist once.
fn node<'a>(nodes: &'a [KanbanNode], id: &str) -> &'a KanbanNode {
    let found: Vec<&KanbanNode> = nodes.iter().filter(|node| node.id == id).collect();
    assert_eq!(found.len(), 1, "one node with the id {id:?}");
    found[0]
}

#[test]
fn the_keyword_needs_a_word_boundary_so_kanbanx_is_rejected() {
    assert!(parse("kanbanX\n  a\n").is_err());
}

#[test]
fn a_hyphen_after_the_keyword_is_a_section_named_by_the_rest() {
    let diagram = parse("kanban-x\n  a\n").expect("parses");
    assert_eq!(diagram.nodes.len(), 2);
    assert_eq!(diagram.nodes[0].id, "-x");
    assert!(diagram.nodes[0].is_section);
    assert_eq!(diagram.nodes[0].level, 0);
    assert_eq!(diagram.nodes[1].parent_id.as_deref(), Some("-x"));
}

#[test]
fn a_comment_after_a_plain_id_is_part_of_the_id() {
    let diagram = parse("kanban\n  a %% note\n").expect("parses");
    assert_eq!(diagram.nodes[0].id, "a %% note");
    assert_eq!(
        diagram.nodes[0].label,
        Value::String("a %% note".to_owned())
    );
}

#[test]
fn a_comment_after_a_bracket_form_is_removed() {
    let diagram = parse("kanban\n  a[x] %% note\n").expect("parses");
    assert_eq!(diagram.nodes[0].id, "a");
    assert_eq!(diagram.nodes[0].label, Value::String("x".to_owned()));
}

#[test]
fn ids_keep_their_trailing_blanks() {
    let diagram = parse("kanban\n  Todo  \n").expect("parses");
    assert_eq!(diagram.nodes[0].id, "Todo  ");
    assert_eq!(diagram.nodes[0].label, Value::String("Todo  ".to_owned()));
}

#[test]
fn a_caret_in_shape_data_is_a_lexical_error() {
    assert!(parse("kanban\n  a@{ label: x^y }\n").is_err());
    assert!(parse("kanban\n  a@{ label: 'x^y' }\n").is_err());
}

#[test]
fn single_quotes_are_ordinary_in_shape_data() {
    let diagram = parse("kanban\n  a@{ label: 'xy' }\n").expect("parses");
    assert_eq!(diagram.nodes[0].label, Value::String("xy".to_owned()));
}

#[test]
fn a_decoration_before_any_node_is_rejected() {
    assert!(parse("kanban\n  ::icon(x)\n").is_err());
    assert!(parse("kanban\n:::x\n").is_err());
}

#[test]
fn a_decoration_goes_to_the_last_node_whatever_its_level() {
    let diagram = parse("kanban\n  root\n    child\n    ::icon(q)\n").expect("parses");
    assert_eq!(node(&diagram.nodes, "root").icon, None);
    assert_eq!(node(&diagram.nodes, "child").icon.as_deref(), Some("q"));
}

#[test]
fn numeric_and_boolean_values_keep_their_type() {
    let diagram = parse(
        "kanban\n  a@{ priority: 1 }\n  b@{ priority: 0 }\n  c@{ label: 5 }\n  d@{ ticket: 12 }\n  e@{ priority: true }\n",
    )
    .expect("parses");
    assert_eq!(node(&diagram.nodes, "a").priority, Some(Value::Number(1.0)));
    assert_eq!(node(&diagram.nodes, "b").priority, None);
    assert_eq!(node(&diagram.nodes, "c").label, Value::Number(5.0));
    assert_eq!(node(&diagram.nodes, "d").ticket.as_deref(), Some("12"));
    assert_eq!(node(&diagram.nodes, "e").priority, Some(Value::Bool(true)));
}

#[test]
fn a_lowercase_shape_is_accepted_and_not_stored_while_a_capitalised_one_is_rejected() {
    let diagram = parse("kanban\n  a@{ shape: rect }\n").expect("parses");
    assert_eq!(diagram.nodes[0].id, "a");
    assert!(parse("kanban\n  a@{ shape: Rect }\n").is_err());
    assert!(parse("kanban\n  a@{ shape: kanbanItem }\n").is_err());
    assert!(parse("kanban\n  a@{ shape: 1 }\n").is_err());
}

#[test]
fn a_bare_keyword_without_a_newline_is_rejected_and_one_with_a_newline_is_empty() {
    assert!(parse("kanban").is_err());
    assert!(parse("kanban\n").expect("parses").nodes.is_empty());
    assert!(parse("kanban\n\n").expect("parses").nodes.is_empty());
}

#[test]
fn ids_that_sanitise_to_nothing_take_the_counter() {
    let diagram = parse("kanban\n  <script>x</script>\n  <script>y</script>\n").expect("parses");
    assert_eq!(diagram.nodes[0].id, "kbn0");
    assert_eq!(diagram.nodes[0].label, Value::String(String::new()));
    assert_eq!(diagram.nodes[1].id, "kbn1");
}

#[test]
fn an_id_keeps_its_markup_after_sanitising() {
    let diagram = parse("kanban\n  x<b>y</b>[z]\n").expect("parses");
    assert_eq!(diagram.nodes[0].id, "x<b>y</b>");
    assert_eq!(diagram.nodes[0].label, Value::String("z".to_owned()));
}

#[test]
fn an_at_sign_outside_shape_data_is_a_lexical_error() {
    assert!(parse("kanban\n  a@b\n").is_err());
}

#[test]
fn a_shape_data_block_that_is_not_closed_is_rejected() {
    assert!(parse("kanban\n  a@{ label: 'x' \n").is_err());
}

#[test]
fn a_line_break_in_a_shape_data_string_becomes_a_br() {
    let diagram = parse("kanban\n  a@{ label: \"two\n    lines\" }\n").expect("parses");
    assert_eq!(
        diagram.nodes[0].label,
        Value::String("two<br/>lines".to_owned())
    );
}

#[test]
fn the_level_check_looks_one_node_behind() {
    // A last line shallower than the first node becomes a child; the error is for the next line.
    let accepted = parse("kanban\n    root\n      child\n  last\n").expect("parses");
    assert_eq!(
        node(&accepted.nodes, "last").parent_id.as_deref(),
        Some("root")
    );
    let error = parse("kanban\n    root\n      child\n  last\n    more\n").expect_err("rejects");
    assert_eq!(error.line, 5);
}
