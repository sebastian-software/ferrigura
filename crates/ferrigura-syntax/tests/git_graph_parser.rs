//! Port of `packages/parser/tests/gitGraph.test.ts`: the syntax tree of the gitGraph grammar.
//! Test names follow the reference's `it` titles in snake_case. The reference's `describe` groups
//! are comments.
//!
//! The reference reads Langium's AST. Here [`parse_ast`] returns the syntax tree with the same
//! statements and values, and the tests read it through the statement kinds of
//! `ferrigura_syntax::git_graph::ast`.
//!
//! Three tests rely on Langium's error recovery (the malformed properties and the one-line
//! cherry-pick). The reference's entry point rejects those inputs too (PLAN section 9, NOTES 2.8),
//! so they assert that [`parse_ast`] fails, at the line of the statement. The two error tests
//! assert the same.

use ferrigura_model::git_graph::CommitType;
use ferrigura_syntax::git_graph::ast::{
    Branch, Checkout, CherryPick, Commit, GitGraph, Merge, Statement, StatementKind,
};
use ferrigura_syntax::git_graph::parse_ast;

fn parsed(text: &str) -> GitGraph {
    parse_ast(text).expect("the gitGraph parses")
}

fn commit_of(statement: &Statement) -> &Commit {
    match &statement.kind {
        StatementKind::Commit(commit) => commit,
        other => panic!("expected a commit, found {other:?}"),
    }
}

fn branch_of(statement: &Statement) -> &Branch {
    match &statement.kind {
        StatementKind::Branch(branch) => branch,
        other => panic!("expected a branch, found {other:?}"),
    }
}

fn merge_of(statement: &Statement) -> &Merge {
    match &statement.kind {
        StatementKind::Merge(merge) => merge,
        other => panic!("expected a merge, found {other:?}"),
    }
}

fn checkout_of(statement: &Statement) -> &Checkout {
    match &statement.kind {
        StatementKind::Checkout(checkout) => checkout,
        other => panic!("expected a checkout, found {other:?}"),
    }
}

fn cherry_pick_of(statement: &Statement) -> &CherryPick {
    match &statement.kind {
        StatementKind::CherryPick(cherry_pick) => cherry_pick,
        other => panic!("expected a cherry-pick, found {other:?}"),
    }
}

// Parsing Commit Statements

#[test]
fn should_parse_a_simple_commit() {
    let result = parsed("gitGraph\n  commit\n");

    assert!(matches!(
        result.statements[0].kind,
        StatementKind::Commit(_)
    ));
}

#[test]
fn should_parse_multiple_commits() {
    let result = parsed("gitGraph\n  commit\n  commit\n  commit\n");

    assert_eq!(result.statements.len(), 3);
}

#[test]
fn should_parse_commits_with_all_properties() {
    let result = parsed("gitGraph\n  commit id:\"1\" msg:\"Fix bug\" tag:\"v1.2\" type:NORMAL\n");
    let commit = commit_of(&result.statements[0]);

    assert_eq!(commit.id.as_deref(), Some("1"));
    assert_eq!(commit.message.as_deref(), Some("Fix bug"));
    assert_eq!(commit.tags, ["v1.2"]);
    assert_eq!(commit.kind, Some(CommitType::Normal));
}

#[test]
fn should_handle_commit_messages_with_special_characters() {
    let result = parsed("gitGraph\n  commit msg:\"Fix issue #123: Handle errors\"\n");
    let commit = commit_of(&result.statements[0]);

    assert_eq!(
        commit.message.as_deref(),
        Some("Fix issue #123: Handle errors")
    );
}

#[test]
fn should_parse_commits_with_only_a_message_and_no_other_properties() {
    let result = parsed("gitGraph\n  commit msg:\"Initial release\"\n");
    let commit = commit_of(&result.statements[0]);

    assert_eq!(commit.message.as_deref(), Some("Initial release"));
    assert_eq!(commit.id, None);
    assert_eq!(commit.kind, None);
}

#[test]
fn should_ignore_malformed_properties_and_not_break_parsing() {
    // Langium recovers from `oops:"ignored"`; the reference's entry point, which the port follows,
    // rejects the input.
    let error =
        parse_ast("gitGraph\n  commit id:\"2\" msg:\"Malformed commit\" oops:\"ignored\"\n")
            .expect_err("the malformed property is rejected");

    assert_eq!(error.line, 2);
}

#[test]
fn should_parse_multiple_commits_with_different_types() {
    let result = parsed("gitGraph\n  commit type:NORMAL\n  commit type:REVERSE\n");
    let commit1 = commit_of(&result.statements[0]);
    let commit2 = commit_of(&result.statements[1]);

    assert_eq!(commit1.kind, Some(CommitType::Normal));
    assert_eq!(commit2.kind, Some(CommitType::Reverse));
}

// Parsing Branch Statements

#[test]
fn should_parse_a_branch_with_a_simple_name() {
    let result = parsed("gitGraph\n commit\n commit\n branch master\n");
    let branch = branch_of(&result.statements[2]);

    assert_eq!(branch.name, "master");
}

#[test]
fn should_parse_a_branch_name_starting_with_numbers() {
    let result = parsed("gitGraph\n commit\n branch 1.0.1\n");
    let branch = branch_of(&result.statements[1]);

    assert_eq!(branch.name, "1.0.1");
}

#[test]
fn should_parse_a_branch_with_an_order_property() {
    let result = parsed("gitGraph\n commit\n  branch feature order:1\n");
    let branch = branch_of(&result.statements[1]);

    assert_eq!(branch.name, "feature");
    assert_eq!(branch.order, Some(1.0));
}

#[test]
fn should_handle_branch_names_with_special_characters() {
    let result = parsed("gitGraph\n  branch feature/test-branch\n");
    let branch = branch_of(&result.statements[0]);

    assert_eq!(branch.name, "feature/test-branch");
}

#[test]
fn should_parse_branches_with_hyphens_and_underscores() {
    let result = parsed("gitGraph\n  branch my-feature_branch\n");
    let branch = branch_of(&result.statements[0]);

    assert_eq!(branch.name, "my-feature_branch");
}

#[test]
fn should_correctly_handle_branch_without_order_property() {
    let result = parsed("gitGraph\n  branch feature\n");
    let branch = branch_of(&result.statements[0]);

    assert_eq!(branch.name, "feature");
    assert_eq!(branch.order, None);
}

// Parsing Merge Statements

#[test]
fn should_parse_a_merge_with_a_branch_name() {
    let result = parsed("gitGraph\n merge master\n");
    let merge = merge_of(&result.statements[0]);

    assert_eq!(merge.branch, "master");
}

#[test]
fn should_handle_merges_with_additional_properties() {
    let result = parsed("gitGraph\n  merge feature id:\"m1\" tag:\"release\" type:HIGHLIGHT\n");
    let merge = merge_of(&result.statements[0]);

    assert_eq!(merge.branch, "feature");
    assert_eq!(merge.id.as_deref(), Some("m1"));
    assert_eq!(merge.tags, ["release"]);
    assert_eq!(merge.kind, Some(CommitType::Highlight));
}

#[test]
fn should_parse_merge_without_any_properties() {
    let result = parsed("gitGraph\n  merge feature\n");
    let merge = merge_of(&result.statements[0]);

    assert_eq!(merge.branch, "feature");
}

#[test]
fn should_ignore_malformed_properties_in_merge_statements() {
    // Langium recovers from `random:"ignored"`; the reference's entry point rejects the input.
    let error = parse_ast("gitGraph\n  merge feature random:\"ignored\"\n")
        .expect_err("the malformed property is rejected");

    assert_eq!(error.line, 2);
}

// Parsing Checkout Statements

#[test]
fn should_parse_a_checkout_to_a_named_branch() {
    let result =
        parsed("gitGraph\n commit id:\"1\"\n branch develop\n branch fun\n checkout develop\n");
    let checkout = checkout_of(&result.statements[3]);

    assert_eq!(checkout.branch, "develop");
}

#[test]
fn should_parse_checkout_to_branches_with_complex_names() {
    let result = parsed("gitGraph\n  checkout hotfix-123\n");
    let checkout = checkout_of(&result.statements[0]);

    assert_eq!(checkout.branch, "hotfix-123");
}

#[test]
fn should_parse_checkouts_with_hyphens_and_numbers() {
    let result = parsed("gitGraph\n  checkout release-2021\n");
    let checkout = checkout_of(&result.statements[0]);

    assert_eq!(checkout.branch, "release-2021");
}

// Parsing CherryPicking Statements

#[test]
fn should_parse_cherry_picking_with_a_commit_id() {
    // Langium recovers from the second `commit` on the same line; the reference's entry point
    // rejects the input, and so does the port.
    let error =
        parse_ast("gitGraph\n commit id:\"123\" commit id:\"321\" cherry-pick id:\"123\"\n")
            .expect_err("the second commit on the line is rejected");

    assert_eq!(error.line, 2);
}

#[test]
fn should_parse_cherry_picking_with_multiple_properties() {
    let result = parsed("gitGraph\n  cherry-pick id:\"123\" tag:\"urgent\" parent:\"100\"\n");
    let cherry_pick = cherry_pick_of(&result.statements[0]);

    assert_eq!(cherry_pick.id.as_deref(), Some("123"));
    assert_eq!(cherry_pick.tags, ["urgent"]);
    assert_eq!(cherry_pick.parent.as_deref(), Some("100"));
}

// Parsing with Accessibility Titles and Descriptions

#[test]
fn should_parse_accessibility_titles() {
    let result = parsed("gitGraph\n  accTitle: Accessible Graph\n  commit\n");

    assert_eq!(result.acc_title, "Accessible Graph");
}

#[test]
fn should_parse_multiline_accessibility_descriptions() {
    let result = parsed(
        "gitGraph\n  accDescr {\n    Detailed description\n    across multiple lines\n  }\n  commit\n",
    );

    assert_eq!(
        result.acc_descr,
        "Detailed description\nacross multiple lines"
    );
}

// Integration Tests

#[test]
fn should_correctly_parse_a_complex_graph_with_various_elements() {
    let result = parsed(
        r#"
        gitGraph TB:
        accTitle: Complex Example
        commit id:"init" type:NORMAL
        branch feature
        commit id:"feat1" msg:"Add feature"
        checkout main
        merge feature tag:"v1.0"
        cherry-pick id:"feat1" tag:"critical fix"
      "#,
    );

    assert_eq!(result.acc_title, "Complex Example");
    assert!(matches!(
        result.statements[0].kind,
        StatementKind::Commit(_)
    ));
    assert!(matches!(
        result.statements[1].kind,
        StatementKind::Branch(_)
    ));
    assert!(matches!(
        result.statements[2].kind,
        StatementKind::Commit(_)
    ));
    assert!(matches!(
        result.statements[3].kind,
        StatementKind::Checkout(_)
    ));
    assert!(matches!(result.statements[4].kind, StatementKind::Merge(_)));
    assert!(matches!(
        result.statements[5].kind,
        StatementKind::CherryPick(_)
    ));
}

// Error Handling for Invalid Syntax

#[test]
fn should_report_errors_for_unknown_properties_in_commit() {
    let error = parse_ast("gitGraph\n  commit unknown:\"oops\"\n")
        .expect_err("the unknown property is rejected");

    assert_eq!(error.line, 2);
}

#[test]
fn should_report_errors_for_invalid_branch_order() {
    let error = parse_ast("gitGraph\n  branch feature order:xyz\n")
        .expect_err("the order that is not a number is rejected");

    assert_eq!(error.line, 2);
}
