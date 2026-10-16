//! The in-source renderer tests of `packages/mermaid/src/diagrams/git/gitGraphRenderer.ts` (lines
//! 1095 to 1489), as ignored stubs. The renderer and its geometry are M6's later step (PLAN
//! section 1, NOTES section 5), so each test is ignored and has an empty body: the names keep the
//! reference's titles for the port that follows.
//!
//! The expected values are the reference's, kept in the comments, for the geometry port:
//! - `setBranchPosition` with a stubbed 10-wide box gives 90 (LR, the step is 50 + 40 for the
//!   rotated label); with the widths 34.9921875 and 56.421875 (TB) it gives 107.49609375 and
//!   225.70703125.
//! - `getCommitPosition` for the commits of the fixture: TB and LR give the same numbers, for
//!   example `commitA` at `{ x: 107.49609375, y: 90, posWithOffset: 90 }`.
//! - `getCommitClassType`: `commit-normal`, `commit-merge` for the merge and `commit-cherry-pick`
//!   for the two cherry-picks.

/// `drawText` needs the DOM to measure the label (renderer 1098-1104).
#[test]
#[ignore = "port: renderer, M6 geometry"]
fn should_draw_text() {}

mod branch_position {
    /// `setBranchPosition` in LR, two branches (renderer 1119-1128).
    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_set_branch_positions_lr_with_two_branches() {}

    /// `setBranchPosition` in TB, two branches (renderer 1130-1143).
    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_set_branch_positions_tb_with_two_branches() {}
}

mod commit_position_tb {
    /// `getCommitPosition` for each commit of the fixture in TB (renderer 1250-1270).
    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_zero() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_a() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_b() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_m() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_c() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit5_8928ea0() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_d() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit7_ed848ba() {}
}

mod commit_position_lr {
    /// `getCommitPosition` for each commit of the fixture in LR (renderer 1271-1291).
    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_zero() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_a() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_b() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_m() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_c() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit5_8928ea0() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit_d() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_position_for_commit_commit7_ed848ba() {}
}

mod class_type {
    /// `getCommitClassType` for each commit of the fixture (renderer 1292-1310).
    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_class_type_for_commit_commit_zero() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_class_type_for_commit_commit_a() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_class_type_for_commit_commit_b() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_class_type_for_commit_commit_m() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_class_type_for_commit_commit_c() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_class_type_for_commit_commit5_8928ea0() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_class_type_for_commit_commit_d() {}

    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_give_the_correct_class_type_for_commit_commit7_ed848ba() {}
}

mod bt_parallel {
    /// `setCommitPosition` and `calculateCommitPosition` for the BT parallel fixture (renderer
    /// 1443-1459).
    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_get_the_correct_commit_position_and_current_position() {}

    /// `setParallelBTPos` for the same fixture (renderer 1461-1476).
    #[test]
    #[ignore = "port: renderer, M6 geometry"]
    fn should_get_the_correct_commit_position_after_parallel_commits() {}
}

/// `findClosestParent` by the direction of the diagram (renderer 1478-1488).
#[test]
#[ignore = "port: renderer, M6 geometry"]
fn add() {}
