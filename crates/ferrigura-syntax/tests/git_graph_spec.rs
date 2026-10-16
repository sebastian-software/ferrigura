//! Port of `packages/mermaid/src/diagrams/git/gitGraph.spec.ts`. Test names follow the spec's
//! titles in snake_case (`gitGraph` becomes `git_graph`). The reference's groups are comments.
//!
//! The reference reads the DB after `parser.parse`. Here each test parses with [`parse`] and reads
//! the returned diagram:
//! - `getCommits()` is `commits` (insertion order), `getBranches()` is `branches`,
//!   `getCurrentBranch()` is `current_branch`, and `getHead()` is the head of the current branch.
//! - `getBranchesAsObjArray()` is the branches sorted by `order` (a stable sort). The model keeps
//!   the creation order, so [`sorted_branch_names`] applies the sort.
//! - A reference id drawn from `Math.random` is deterministic here (PLAN section 7). Such a commit
//!   is checked for `autoId` and the shape `<seq>-<7 hex>`, not for a value.
//! - The `rejects.toThrow(...)` tests assert that [`parse`] fails with the exact DB message.
//! - The `it.skip` tests are ignored with the reason of the skip. The five `getConfig` tests are
//!   ignored, see the note above them.

use ferrigura_model::git_graph::{
    CommitType, Direction, GitGraphBranch, GitGraphCommit, GitGraphDiagram,
};
use ferrigura_syntax::git_graph::{Options, parse, parse_with_warnings};

fn parsed(text: &str) -> GitGraphDiagram {
    parse(text).expect("the gitGraph parses")
}

/// The message of the parse error of `text`. The test fails if the text parses.
fn rejected(text: &str) -> String {
    parse(text).expect_err("the gitGraph is rejected").message
}

fn branch<'a>(diagram: &'a GitGraphDiagram, name: &str) -> &'a GitGraphBranch {
    diagram
        .branches
        .iter()
        .find(|branch| branch.name == name)
        .unwrap_or_else(|| panic!("no branch {name}"))
}

/// `getHead()`: the head commit of the current branch.
fn head(diagram: &GitGraphDiagram) -> Option<&str> {
    branch(diagram, &diagram.current_branch).head.as_deref()
}

/// `getBranchesAsObjArray()`: the branch names sorted by their order, stably.
fn sorted_branch_names(diagram: &GitGraphDiagram) -> Vec<&str> {
    let mut branches: Vec<&GitGraphBranch> = diagram.branches.iter().collect();
    branches.sort_by(|left, right| left.order.total_cmp(&right.order));
    branches
        .into_iter()
        .map(|branch| branch.name.as_str())
        .collect()
}

/// A commit whose id the input does not give: `<seq>-<7 hex digits>`, flagged `autoId`.
fn assert_auto_id(commit: &GitGraphCommit) {
    assert!(commit.auto_id, "the id {} is generated", commit.id);
    let (seq, hex) = commit.id.split_once('-').expect("the id has a dash");
    assert_eq!(seq, commit.seq.to_string());
    assert_eq!(hex.len(), 7);
    assert!(
        hex.bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
}

// basic gitGraph

#[test]
fn should_handle_a_git_graph_definition() {
    let diagram = parsed("gitGraph:\n commit\n");

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
}

#[test]
fn should_handle_set_direction_top_to_bottom() {
    let diagram = parsed("gitGraph TB:\ncommit\n");

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::TB);
    assert_eq!(diagram.branches.len(), 1);
}

#[test]
fn should_handle_set_direction_bottom_to_top() {
    let diagram = parsed("gitGraph BT:\ncommit\n");

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::BT);
    assert_eq!(diagram.branches.len(), 1);
}

#[test]
fn should_checkout_a_branch() {
    let diagram = parsed("gitGraph:\nbranch new\ncheckout new\n");

    assert_eq!(diagram.commits.len(), 0);
    assert_eq!(diagram.current_branch, "new");
}

#[test]
fn should_switch_a_branch() {
    let diagram = parsed("gitGraph:\nbranch new\nswitch new\n");

    assert_eq!(diagram.commits.len(), 0);
    assert_eq!(diagram.current_branch, "new");
}

#[test]
fn should_add_commits_to_checked_out_branch() {
    let diagram = parsed("gitGraph:\nbranch new\ncheckout new\ncommit\ncommit\n");

    assert_eq!(diagram.commits.len(), 2);
    assert_eq!(diagram.current_branch, "new");
    let head_id = branch(&diagram, "new")
        .head
        .as_deref()
        .expect("the branch has a head");
    let head_commit = diagram
        .commits
        .iter()
        .find(|commit| commit.id == head_id)
        .expect("the head is a commit of the diagram");
    assert_eq!(head_commit.branch, "new");
}

#[test]
fn should_handle_commit_with_args() {
    let diagram = parsed("gitGraph:\ncommit \"a commit\"\n");

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.commits[0].message, "a commit");
    assert_eq!(diagram.current_branch, "main");
}

#[test]
#[ignore = "port: reset is not in the grammar (the reference skips this test)"]
fn should_reset_a_branch() {
    let diagram = parsed(
        "gitGraph:\ncommit\ncommit\nbranch newbranch\ncheckout newbranch\ncommit\nreset main\n",
    );

    assert_eq!(diagram.commits.len(), 3);
    assert_eq!(diagram.current_branch, "newbranch");
    assert_eq!(
        branch(&diagram, "newbranch").head,
        branch(&diagram, "main").head
    );
    assert_eq!(
        head(&diagram),
        branch(&diagram, "newbranch").head.as_deref()
    );
}

#[test]
#[ignore = "port: reset is not in the grammar (the reference skips this test)"]
fn reset_can_take_an_argument() {
    let diagram = parsed(
        "gitGraph:\ncommit\ncommit\nbranch newbranch\ncheckout newbranch\ncommit\nreset main^\n",
    );

    assert_eq!(diagram.commits.len(), 3);
    assert_eq!(diagram.current_branch, "newbranch");
}

#[test]
#[ignore = "port: the reference DB has no fast-forward merge, it always adds a merge commit (the reference skips this test)"]
fn should_handle_fast_forwardable_merges() {
    let diagram = parsed(
        "gitGraph:\ncommit\nbranch newbranch\ncheckout newbranch\ncommit\ncommit\ncheckout main\nmerge newbranch\n",
    );

    assert_eq!(diagram.commits.len(), 4);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(
        branch(&diagram, "newbranch").head,
        branch(&diagram, "main").head
    );
}

#[test]
fn should_handle_cases_when_merge_is_a_noop() {
    let diagram = parsed(
        "gitGraph:\ncommit\nbranch newbranch\ncheckout newbranch\ncommit\ncommit\nmerge main\n",
    );

    assert_eq!(diagram.commits.len(), 4);
    assert_eq!(diagram.current_branch, "newbranch");
    assert_ne!(
        branch(&diagram, "newbranch").head,
        branch(&diagram, "main").head
    );
    assert_eq!(
        head(&diagram),
        branch(&diagram, "newbranch").head.as_deref()
    );
}

#[test]
fn should_handle_merge_with_2_parents() {
    let diagram = parsed(
        "gitGraph:\ncommit\nbranch newbranch\ncheckout newbranch\ncommit\ncommit\ncheckout main\ncommit\nmerge newbranch\n",
    );

    assert_eq!(diagram.commits.len(), 5);
    assert_eq!(diagram.current_branch, "main");
    assert_ne!(
        branch(&diagram, "newbranch").head,
        branch(&diagram, "main").head
    );
    assert_eq!(head(&diagram), branch(&diagram, "main").head.as_deref());
}

#[test]
#[ignore = "port: the reference DB has no fast-forward merge, it always adds a merge commit (the reference skips this test)"]
fn should_handle_ff_merge_when_history_walk_has_two_parents_merge_commit() {
    let diagram = parsed(
        "gitGraph:\ncommit\nbranch newbranch\ncheckout newbranch\ncommit\ncommit\ncheckout main\ncommit\nmerge newbranch\ncommit\ncheckout newbranch\nmerge main\n",
    );

    assert_eq!(diagram.commits.len(), 7);
    assert_eq!(diagram.current_branch, "newbranch");
    assert_eq!(
        branch(&diagram, "newbranch").head,
        branch(&diagram, "main").head
    );
}

#[test]
fn should_generate_an_array_of_known_branches() {
    let diagram = parsed("gitGraph:\ncommit\nbranch b1\ncheckout b1\ncommit\ncommit\nbranch b2\n");

    assert_eq!(sorted_branch_names(&diagram), ["main", "b1", "b2"]);
}

// advanced gitGraphs: commits (id, msg, tag, type)

#[test]
fn should_handle_a_git_graph_commit_with_no_params_get_auto_generated_read_only_id() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "");
    assert_auto_id(commit);
    assert!(commit.tags.is_empty());
    assert_eq!(commit.kind, CommitType::Normal);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_id_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit id:"1111"
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "");
    assert_eq!(commit.id, "1111");
    assert!(commit.tags.is_empty());
    assert_eq!(commit.kind, CommitType::Normal);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_tag_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit tag:"test"
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "");
    assert_auto_id(commit);
    assert_eq!(commit.tags, ["test"]);
    assert_eq!(commit.kind, CommitType::Normal);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_type_highlight_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit type: HIGHLIGHT
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "");
    assert_auto_id(commit);
    assert!(commit.tags.is_empty());
    assert_eq!(commit.kind, CommitType::Highlight);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_type_reverse_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit type: REVERSE
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "");
    assert_auto_id(commit);
    assert!(commit.tags.is_empty());
    assert_eq!(commit.kind, CommitType::Reverse);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_type_normal_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit type: NORMAL
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "");
    assert_auto_id(commit);
    assert!(commit.tags.is_empty());
    assert_eq!(commit.kind, CommitType::Normal);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_msg_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit "test commit"
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "test commit");
    assert_auto_id(commit);
    assert!(commit.tags.is_empty());
    assert_eq!(commit.kind, CommitType::Normal);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_msg_key_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit msg: "test commit"
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "test commit");
    assert_auto_id(commit);
    assert!(commit.tags.is_empty());
    assert_eq!(commit.kind, CommitType::Normal);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_id_tag_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit id:"1111" tag: "test tag"
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "");
    assert_eq!(commit.id, "1111");
    assert_eq!(commit.tags, ["test tag"]);
    assert_eq!(commit.kind, CommitType::Normal);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_type_tag_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit type:HIGHLIGHT tag: "test tag"
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "");
    assert_auto_id(commit);
    assert_eq!(commit.tags, ["test tag"]);
    assert_eq!(commit.kind, CommitType::Highlight);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_tag_and_type_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit tag: "test tag" type:HIGHLIGHT
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "");
    assert_auto_id(commit);
    assert_eq!(commit.tags, ["test tag"]);
    assert_eq!(commit.kind, CommitType::Highlight);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_id_type_and_tag_only() {
    let diagram = parsed(
        r#"gitGraph:
        commit id:"1111" type:REVERSE tag: "test tag"
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "");
    assert_eq!(commit.id, "1111");
    assert_eq!(commit.tags, ["test tag"]);
    assert_eq!(commit.kind, CommitType::Reverse);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_commit_id_type_tag_and_msg() {
    let diagram = parsed(
        r#"gitGraph:
        commit id:"1111" type:REVERSE tag: "test tag" msg:"test msg"
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "test msg");
    assert_eq!(commit.id, "1111");
    assert_eq!(commit.tags, ["test tag"]);
    assert_eq!(commit.kind, CommitType::Reverse);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_type_tag_msg_commit_id() {
    let diagram = parsed(
        r#"gitGraph:
        commit type:REVERSE tag: "test tag" msg: "test msg" id: "1111"

        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "test msg");
    assert_eq!(commit.id, "1111");
    assert_eq!(commit.tags, ["test tag"]);
    assert_eq!(commit.kind, CommitType::Reverse);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_tag_msg_commit_id_type() {
    let diagram = parsed(
        r#"gitGraph:
        commit  tag: "test tag" msg:"test msg" id:"1111" type:REVERSE
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "test msg");
    assert_eq!(commit.id, "1111");
    assert_eq!(commit.tags, ["test tag"]);
    assert_eq!(commit.kind, CommitType::Reverse);
}

#[test]
fn should_handle_a_git_graph_commit_with_custom_msg_commit_id_type_tag() {
    let diagram = parsed(
        r#"gitGraph:
        commit msg:"test msg" id:"1111" type:REVERSE tag: "test tag"
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
    let commit = &diagram.commits[0];
    assert_eq!(commit.message, "test msg");
    assert_eq!(commit.id, "1111");
    assert_eq!(commit.tags, ["test tag"]);
    assert_eq!(commit.kind, CommitType::Reverse);
}

// advanced gitGraphs: commits and branches

#[test]
fn should_handle_3_straight_commits() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        commit
        commit
        "#,
    );

    assert_eq!(diagram.commits.len(), 3);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 1);
}

#[test]
fn should_handle_new_branch_creation() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch testBranch
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "testBranch");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
}

#[test]
fn should_allow_quoted_branch_names() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch "branch"
        checkout "branch"
        commit
        checkout main
        merge "branch"
        "#,
    );

    assert_eq!(diagram.commits.len(), 3);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
    let commits = &diagram.commits;
    assert_eq!(commits[0].branch, "main");
    assert_eq!(commits[1].branch, "branch");
    assert_eq!(commits[2].branch, "main");
    assert_eq!(sorted_branch_names(&diagram), ["main", "branch"]);
}

#[test]
fn should_allow_characters_in_branch_names() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch azAZ_-./test
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "azAZ_-./test");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
}

#[test]
fn should_allow_branch_names_starting_with_numbers() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        %% branch names starting with numbers are not recommended, but are supported by git
        branch 1.0.1
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "1.0.1");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
}

#[test]
fn should_allow_branch_names_starting_with_unusual_prefixes() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        %% branch names starting with numbers are not recommended, but are supported by git
        branch branch01
        branch checkout02
        branch cherry-pick03
        branch branch/example-branch
        branch merge/test_merge
        %% single character branch name
        branch A
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "A");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 7);
    let names: Vec<&str> = diagram
        .branches
        .iter()
        .map(|branch| branch.name.as_str())
        .collect();
    for expected in [
        "branch01",
        "checkout02",
        "cherry-pick03",
        "branch/example-branch",
        "merge/test_merge",
        "A",
    ] {
        assert!(names.contains(&expected), "missing branch {expected}");
    }
}

#[test]
fn should_handle_new_branch_checkout() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch testBranch
        checkout testBranch
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "testBranch");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
}

#[test]
fn should_handle_new_branch_checkout_with_order() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch test1 order: 3
        branch test2 order: 2
        branch test3 order: 1
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "test3");
    assert_eq!(diagram.branches.len(), 4);
    assert_eq!(
        sorted_branch_names(&diagram),
        ["main", "test3", "test2", "test1"]
    );
}

#[test]
fn should_handle_new_branch_checkout_with_and_without_order() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch test1 order: 1
        branch test2
        branch test3
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "test3");
    assert_eq!(diagram.branches.len(), 4);
    assert_eq!(
        sorted_branch_names(&diagram),
        ["main", "test2", "test3", "test1"]
    );
}

#[test]
fn should_handle_new_branch_checkout_commit() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch testBranch
        checkout testBranch
        commit
        "#,
    );

    assert_eq!(diagram.commits.len(), 2);
    assert_eq!(diagram.current_branch, "testBranch");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
    let commits = &diagram.commits;
    assert_eq!(commits[0].branch, "main");
    assert!(commits[0].parents.is_empty());
    assert_eq!(commits[1].branch, "testBranch");
    assert_eq!(commits[1].parents, [commits[0].id.as_str()]);
}

#[test]
fn should_handle_new_branch_checkout_commit_and_merge() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch testBranch
        checkout testBranch
        commit
        commit
        checkout main
        merge testBranch
        "#,
    );

    assert_eq!(diagram.commits.len(), 4);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
    let commits = &diagram.commits;
    assert_eq!(commits[0].branch, "main");
    assert!(commits[0].parents.is_empty());
    assert_eq!(commits[1].branch, "testBranch");
    assert_eq!(commits[1].parents, [commits[0].id.as_str()]);
    assert_eq!(commits[2].branch, "testBranch");
    assert_eq!(commits[2].parents, [commits[1].id.as_str()]);
    assert_eq!(commits[3].branch, "main");
    assert_eq!(
        commits[3].parents,
        [commits[0].id.clone(), commits[2].id.clone()]
    );
    assert_eq!(sorted_branch_names(&diagram), ["main", "testBranch"]);
}

#[test]
fn should_handle_new_branch_switch() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch testBranch
        switch testBranch
        "#,
    );

    assert_eq!(diagram.commits.len(), 1);
    assert_eq!(diagram.current_branch, "testBranch");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
}

#[test]
fn should_handle_new_branch_switch_commit() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch testBranch
        switch testBranch
        commit
        "#,
    );

    assert_eq!(diagram.commits.len(), 2);
    assert_eq!(diagram.current_branch, "testBranch");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
    let commits = &diagram.commits;
    assert_eq!(commits[0].branch, "main");
    assert!(commits[0].parents.is_empty());
    assert_eq!(commits[1].branch, "testBranch");
    assert_eq!(commits[1].parents, [commits[0].id.as_str()]);
}

#[test]
fn should_handle_new_branch_switch_commit_and_merge() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch testBranch
        switch testBranch
        commit
        commit
        switch main
        merge testBranch
        "#,
    );

    assert_eq!(diagram.commits.len(), 4);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
    let commits = &diagram.commits;
    assert_eq!(commits[0].branch, "main");
    assert!(commits[0].parents.is_empty());
    assert_eq!(commits[1].branch, "testBranch");
    assert_eq!(commits[1].parents, [commits[0].id.as_str()]);
    assert_eq!(commits[2].branch, "testBranch");
    assert_eq!(commits[2].parents, [commits[1].id.as_str()]);
    assert_eq!(commits[3].branch, "main");
    assert_eq!(
        commits[3].parents,
        [commits[0].id.clone(), commits[2].id.clone()]
    );
    assert_eq!(sorted_branch_names(&diagram), ["main", "testBranch"]);
}

#[test]
fn should_handle_merge_tags() {
    let diagram = parsed(
        r#"gitGraph:
        commit
        branch testBranch
        checkout testBranch
        commit
        checkout main
        merge testBranch tag: "merge-tag"
        "#,
    );

    assert_eq!(diagram.commits.len(), 3);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);
    assert_eq!(diagram.branches.len(), 2);
    let commits = &diagram.commits;
    assert_eq!(commits[0].branch, "main");
    assert!(commits[0].parents.is_empty());
    assert_eq!(commits[1].branch, "testBranch");
    assert_eq!(commits[1].parents, [commits[0].id.as_str()]);
    assert_eq!(commits[2].branch, "main");
    assert_eq!(
        commits[2].parents,
        [commits[0].id.clone(), commits[1].id.clone()]
    );
    assert_eq!(commits[2].tags, ["merge-tag"]);
    assert_eq!(sorted_branch_names(&diagram), ["main", "testBranch"]);
}

#[test]
fn should_handle_merge_with_custom_ids_tags_and_type() {
    let diagram = parsed(
        r#"gitGraph:
          commit
          branch testBranch
          checkout testBranch
          commit
          checkout main
          %% Merge Tag and ID
          merge testBranch tag: "merge-tag" id: "2-222"
          branch testBranch2
          checkout testBranch2
          commit
          checkout main
          %% Merge ID and Tag (reverse order)
          merge testBranch2 id: "4-444" tag: "merge-tag2" type:HIGHLIGHT
          branch testBranch3
          checkout testBranch3
          commit
          checkout main
          %% just Merge ID
          merge testBranch3 id: "6-666"
        "#,
    );

    assert_eq!(diagram.commits.len(), 7);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.direction, Direction::LR);

    // The order of these commits is the order they were created in.
    let commits = &diagram.commits;
    let main_commit = &commits[0];
    let test_branch_commit = &commits[1];
    let test_branch_merge = &commits[2];
    let test_branch2_commit = &commits[3];
    let test_branch2_merge = &commits[4];
    let test_branch3_commit = &commits[5];
    let test_branch3_merge = &commits[6];

    assert_eq!(main_commit.branch, "main");
    assert!(main_commit.parents.is_empty());

    assert_eq!(test_branch_commit.branch, "testBranch");
    assert_eq!(test_branch_commit.parents, [main_commit.id.as_str()]);

    assert_eq!(test_branch_merge.branch, "main");
    assert_eq!(
        test_branch_merge.parents,
        [main_commit.id.clone(), test_branch_commit.id.clone()]
    );
    assert_eq!(test_branch_merge.tags, ["merge-tag"]);
    assert_eq!(test_branch_merge.id, "2-222");

    assert_eq!(test_branch2_merge.branch, "main");
    assert_eq!(
        test_branch2_merge.parents,
        [test_branch_merge.id.clone(), test_branch2_commit.id.clone()]
    );
    assert_eq!(test_branch2_merge.tags, ["merge-tag2"]);
    assert_eq!(test_branch2_merge.id, "4-444");
    assert_eq!(test_branch2_merge.custom_type, Some(CommitType::Highlight));
    assert_eq!(test_branch2_merge.custom_id, Some(true));

    assert_eq!(test_branch3_merge.branch, "main");
    assert_eq!(
        test_branch3_merge.parents,
        [
            test_branch2_merge.id.clone(),
            test_branch3_commit.id.clone()
        ]
    );
    assert_eq!(test_branch3_merge.id, "6-666");

    assert_eq!(
        sorted_branch_names(&diagram),
        ["main", "testBranch", "testBranch2", "testBranch3"]
    );
}

// cherry-picks

#[test]
fn should_support_cherry_picking_commits() {
    let diagram = parsed(
        r#"gitGraph
        commit id: "ZERO"
        branch develop
        commit id:"A"
        checkout main
        cherry-pick id:"A"
        "#,
    );

    let cherry_pick = &diagram.commits[2];
    assert_eq!(cherry_pick.tags, ["cherry-pick:A"]);
    assert_eq!(cherry_pick.branch, "main");
}

#[test]
fn should_support_cherry_picking_commits_with_custom_tag() {
    let diagram = parsed(
        r#"gitGraph
        commit id: "ZERO"
        branch develop
        commit id:"A"
        checkout main
        cherry-pick id:"A" tag:"MyTag"
        "#,
    );

    let cherry_pick = &diagram.commits[2];
    assert_eq!(cherry_pick.tags, ["MyTag"]);
    assert_eq!(cherry_pick.branch, "main");
}

#[test]
fn should_support_cherry_picking_commits_with_no_tag() {
    let diagram = parsed(
        r#"gitGraph
        commit id: "ZERO"
        branch develop
        commit id:"A"
        checkout main
        cherry-pick id:"A" tag:""
        "#,
    );

    let cherry_pick = &diagram.commits[2];
    assert!(cherry_pick.tags.is_empty());
    assert_eq!(cherry_pick.branch, "main");
}

#[test]
fn should_support_cherry_picking_of_merge_commits() {
    let diagram = parsed(
        r#"gitGraph
        commit id: "ZERO"
        branch feature
        branch release
        checkout feature
        commit id: "A"
        commit id: "B"
        checkout main
        merge feature id: "M"
        checkout release
        cherry-pick id: "M" parent:"B"
        "#,
    );

    let cherry_pick = &diagram.commits[4];
    assert_eq!(cherry_pick.tags, ["cherry-pick:M|parent:B"]);
    assert_eq!(cherry_pick.branch, "release");
}

#[test]
fn should_support_cherry_picking_of_merge_commits_with_tag() {
    let diagram = parsed(
        r#"gitGraph
        commit id: "ZERO"
        branch feature
        branch release
        checkout feature
        commit id: "A"
        commit id: "B"
        checkout main
        merge feature id: "M"
        checkout release
        cherry-pick id: "M" parent:"ZERO" tag: "v1.0"
        "#,
    );

    let cherry_pick = &diagram.commits[4];
    assert_eq!(cherry_pick.tags, ["v1.0"]);
    assert_eq!(cherry_pick.branch, "release");
}

#[test]
fn should_support_cherry_picking_of_merge_commits_with_additional_commit() {
    let diagram = parsed(
        r#"gitGraph
        commit id: "ZERO"
        branch feature
        branch release
        checkout feature
        commit id: "A"
        commit id: "B"
        checkout main
        merge feature id: "M"
        checkout release
        commit id: "C"
        cherry-pick id: "M" tag: "v2.1:ZERO" parent:"ZERO"
        commit id: "D"
        "#,
    );

    let cherry_pick = &diagram.commits[5];
    assert_eq!(cherry_pick.tags, ["v2.1:ZERO"]);
    assert_eq!(cherry_pick.branch, "release");
}

#[test]
fn should_support_cherry_picking_of_merge_commits_with_empty_tag() {
    let diagram = parsed(
        r#"gitGraph
        commit id: "ZERO"
        branch feature
        branch release
        checkout feature
        commit id: "A"
        commit id: "B"
        checkout main
        merge feature id: "M"
        checkout release
        commit id: "C"
        cherry-pick id:"M" parent: "ZERO" tag:""
        commit id: "D"
        cherry-pick id:"M" tag:"" parent: "B"
        "#,
    );

    let cherry_pick = &diagram.commits[5];
    let cherry_pick2 = &diagram.commits[7];
    assert!(cherry_pick.tags.is_empty());
    assert!(cherry_pick2.tags.is_empty());
    assert_eq!(cherry_pick.branch, "release");
}

#[test]
fn should_fail_cherry_picking_of_merge_commits_if_the_parent_of_merge_commits_is_not_specified() {
    let message = rejected(
        r#"gitGraph
        commit id: "ZERO"
        branch feature
        branch release
        checkout feature
        commit id: "A"
        commit id: "B"
        checkout main
        merge feature id: "M"
        checkout release
        commit id: "C"
        cherry-pick id:"M"
        "#,
    );

    assert_eq!(
        message,
        "Incorrect usage of cherry-pick: If the source commit is a merge commit, an immediate parent commit must be specified."
    );
}

#[test]
fn should_fail_cherry_picking_of_merge_commits_when_the_parent_provided_is_not_an_immediate_parent_of_cherry_picked_commit()
 {
    let message = rejected(
        r#"gitGraph
        commit id: "ZERO"
        branch feature
        branch release
        checkout feature
        commit id: "A"
        commit id: "B"
        checkout main
        merge feature id: "M"
        checkout release
        commit id: "C"
        cherry-pick id:"M" parent: "A"
        "#,
    );

    assert_eq!(
        message,
        "Invalid operation: The specified parent commit is not an immediate parent of the cherry-picked commit."
    );
}

// errors of the DB

#[test]
fn should_throw_error_when_try_to_branch_existing_branch_main() {
    let message = rejected(
        r#"gitGraph
        commit
        branch testBranch
        commit
        branch main
        commit
        checkout main
        merge testBranch
        "#,
    );

    assert_eq!(
        message,
        "Trying to create an existing branch. (Help: Either use a new name if you want create a new branch or try using \"checkout main\")"
    );
}

#[test]
fn should_throw_error_when_try_to_branch_existing_branch_test_branch() {
    let message = rejected(
        r#"gitGraph
        commit
        branch testBranch
        commit
        branch testBranch
        commit
        checkout main
        merge testBranch
        "#,
    );

    assert_eq!(
        message,
        "Trying to create an existing branch. (Help: Either use a new name if you want create a new branch or try using \"checkout testBranch\")"
    );
}

#[test]
fn should_throw_error_when_try_to_checkout_unknown_branch_test_branch() {
    let message = rejected(
        r#"gitGraph
        commit
        checkout testBranch
        commit
        branch testBranch
        commit
        checkout main
        merge testBranch
        "#,
    );

    assert_eq!(
        message,
        "Trying to checkout branch which is not yet created. (Help try using \"branch testBranch\")"
    );
}

#[test]
fn should_throw_error_when_trying_to_merge_when_current_branch_has_no_commits() {
    let message = rejected(
        r#"gitGraph
        merge testBranch
        commit
        checkout testBranch
        commit
        branch testBranch
        commit
        checkout main
        merge testBranch
        "#,
    );

    assert_eq!(
        message,
        "Incorrect usage of \"merge\". Current branch (main)has no commits"
    );
}

#[test]
fn should_throw_error_when_trying_to_merge_unknown_branch() {
    let message = rejected(
        r#"gitGraph
        commit
        merge testBranch
        commit
        checkout testBranch
        commit
        branch testBranch
        commit
        checkout main
        merge testBranch
        "#,
    );

    assert_eq!(
        message,
        "Incorrect usage of \"merge\". Branch to be merged (testBranch) does not exist"
    );
}

#[test]
fn should_throw_error_when_trying_to_merge_branch_to_itself() {
    let message = rejected(
        r#"gitGraph
        commit
        branch testBranch
        merge testBranch
        "#,
    );

    assert_eq!(
        message,
        "Incorrect usage of \"merge\". Cannot merge a branch to itself"
    );
}

#[test]
fn should_throw_error_when_using_existing_id_as_merge_id() {
    let message = rejected(
        r#"gitGraph
        commit id: "1-111"
        branch testBranch
        commit id: "2-222"
        commit id: "3-333"
        checkout main
        merge testBranch id: "1-111"
        "#,
    );

    assert_eq!(
        message,
        "Incorrect usage of \"merge\". Commit with id:1-111 already exists, use different custom id"
    );
}

#[test]
fn should_throw_error_when_trying_to_merge_branches_having_same_heads() {
    let message = rejected(
        r#"gitGraph
        commit
        branch testBranch
        checkout main
        merge testBranch
        "#,
    );

    assert_eq!(
        message,
        "Incorrect usage of \"merge\". Both branches have same head"
    );
}

#[test]
fn should_throw_error_when_trying_to_merge_branch_which_has_no_commits() {
    let message = rejected(
        r#"gitGraph
        branch test1

        checkout main
        commit
        merge test1
        "#,
    );

    assert_eq!(
        message,
        "Incorrect usage of \"merge\". Branch to be merged (test1) has no commits"
    );
}

// accessibility

#[test]
fn should_handle_a_title_and_a_description_acc_descr() {
    let diagram = parsed(
        r#"gitGraph:
          accTitle: This is a title
          accDescr: This is a description
        commit
        "#,
    );

    assert_eq!(diagram.acc_title, "This is a title");
    assert_eq!(diagram.acc_descr, "This is a description");
}

#[test]
fn should_handle_a_title_and_a_multiline_description_acc_descr() {
    let diagram = parsed(
        r#"gitGraph:
          accTitle: This is a title
          accDescr {
            This is a description
            using multiple lines
          }
        commit
        "#,
    );

    assert_eq!(diagram.acc_title, "This is a title");
    assert_eq!(
        diagram.acc_descr,
        "This is a description\nusing multiple lines"
    );
}

// unsafe properties: the reference's loop over `__proto__` and `constructor`, the names without
// their underscores in the test names

#[test]
fn should_work_with_custom_commit_id_or_branch_name_proto() {
    let prop = "__proto__";
    let diagram = parsed(&format!(
        "gitGraph\n        commit id:\"{prop}\"\n        branch {prop}\n        checkout {prop}\n        commit\n        checkout main\n        merge {prop}\n        "
    ));

    assert_eq!(diagram.commits.len(), 3);
    assert_eq!(diagram.commits[0].id, prop);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.branches.len(), 2);
    assert_eq!(sorted_branch_names(&diagram)[1], prop);
}

#[test]
fn should_work_with_custom_commit_id_or_branch_name_constructor() {
    let prop = "constructor";
    let diagram = parsed(&format!(
        "gitGraph\n        commit id:\"{prop}\"\n        branch {prop}\n        checkout {prop}\n        commit\n        checkout main\n        merge {prop}\n        "
    ));

    assert_eq!(diagram.commits.len(), 3);
    assert_eq!(diagram.commits[0].id, prop);
    assert_eq!(diagram.current_branch, "main");
    assert_eq!(diagram.branches.len(), 2);
    assert_eq!(sorted_branch_names(&diagram)[1], prop);
}

// warnings

#[test]
fn should_log_a_warning_when_two_commits_have_the_same_id() {
    let text = r#"gitGraph
     commit id:"initial commit"
     commit id:"work on first release"
     commit id:"design freeze from here"
     branch v1-rc
     checkout v1-rc
     commit id:"bugfix 1"
     commit id:"bigfix 2" tag:"v1.0.1"
     branch FORK-v1.0-MDR
     checkout FORK-v1.0-MDR
     commit id:"working on MDR"
     checkout v1-rc
     commit id:"minor design changes for MDR" tag:"v1.0.2"
     checkout FORK-v1.0-MDR
     merge v1-rc
     checkout main
     commit id:"new feature for v1.1…"
     checkout FORK-v1.0-MDR
     commit id:"working on MDR"
     commit id:"finishing MDR"
     branch v1.0-MDR
     checkout v1.0-MDR
     commit id:"brush up release" tag:"v1.0.2-MDR"
     checkout v1-rc
     commit id:"bugfix without MDR"
     checkout main
     commit id:"work on v1.1"
  "#;

    let (_, warnings) =
        parse_with_warnings(text, &Options::default()).expect("the gitGraph parses");

    assert!(
        warnings
            .iter()
            .any(|warning| warning.as_str() == "Commit ID working on MDR already exists"),
        "the warnings are {warnings:?}"
    );
}

// gitGraph config directives
//
// The reference's `db.getConfig()` returns the gitGraph section of the configuration, with the
// renderer's keys (`showBranches`, `showCommitLabel`, `rotateCommitLabel`, `parallelCommits`). The
// model has no configuration: `options_of` reads the two keys the DB reads (`mainBranchName` and
// `mainBranchOrder`), and the four renderer keys belong to the renderer, which is not ported in M6.

#[test]
#[ignore = "port: getConfig is the gitGraph section of the configuration; its renderer keys are not in options_of"]
fn should_expose_get_config_method() {}

#[test]
#[ignore = "port: getConfig is the gitGraph section of the configuration; its renderer keys are not in options_of"]
fn should_return_config_with_show_branches_property() {}

#[test]
#[ignore = "port: getConfig is the gitGraph section of the configuration; its renderer keys are not in options_of"]
fn should_return_config_with_show_commit_label_property() {}

#[test]
#[ignore = "port: getConfig is the gitGraph section of the configuration; its renderer keys are not in options_of"]
fn should_return_config_with_rotate_commit_label_property() {}

#[test]
#[ignore = "port: getConfig is the gitGraph section of the configuration; its renderer keys are not in options_of"]
fn should_return_config_with_parallel_commits_property() {}
