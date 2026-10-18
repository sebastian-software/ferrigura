//! The post-processing passes against the reference corpus. Each pass runs on the state before each of its stages
//! and must reproduce the stage exactly. The state before a stage is the data of the stage before it in the case;
//! the stage before the first post-processing pass is `routed`, and the core applies its curve rule to that state
//! before the passes run.

mod support;

use ferrigura_swimlane::LayoutData;
use ferrigura_swimlane::testing::{layout_data_from_json, layout_data_to_json, passes};
use support::corpus::{Case, compare_snapshot, load_cases};

/// The core's curve rule (`layoutCore.ts:55`): an edge without a curve, or with `basis`, is rounded.
fn apply_curve_rule(data: &mut LayoutData) {
    for edge in &mut data.edges {
        if matches!(edge.curve.as_deref(), None | Some("") | Some("basis")) {
            edge.curve = Some("rounded".to_owned());
        }
    }
}

/// The layout data before the stage at `index` of the case.
fn state_before(case: &Case, index: usize) -> LayoutData {
    let (previous, data) = &case.stages[index - 1];
    let mut layout = layout_data_from_json(data)
        .unwrap_or_else(|error| panic!("{}: {previous}: {error}", case.name));
    if previous == "routed" {
        apply_curve_rule(&mut layout);
    }
    layout
}

/// Whether a stage name is the pass, run once (`post:<pass>`) or with its run number (`post:<pass>#k`).
fn is_stage_of(stage: &str, pass: &str) -> bool {
    let name = format!("post:{pass}");
    stage == name
        || stage
            .strip_prefix(&name)
            .is_some_and(|rest| rest.starts_with('#'))
}

/// Runs one pass before each of its stages in the corpus, and returns how many stages it checked. Every stage must
/// match exactly; the first differences are named in the failure.
fn check_pass(cases: &[Case], pass: &str, run: fn(&mut LayoutData)) -> usize {
    let mut checked = 0;
    let mut failures = Vec::new();
    for case in cases {
        for index in 1..case.stages.len() {
            let (stage, expected) = &case.stages[index];
            if !is_stage_of(stage, pass) {
                continue;
            }
            checked += 1;
            let mut actual = state_before(case, index);
            run(&mut actual);
            if let Err(difference) = compare_snapshot(expected, &layout_data_to_json(&actual)) {
                failures.push(format!("{} {stage}: {difference}", case.name));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{pass}: {} of {checked} stages differ; the first: {:#?}",
        failures.len(),
        failures.iter().take(5).collect::<Vec<_>>()
    );
    eprintln!("{pass}: {checked} stages match");
    checked
}

#[test]
fn nudge_shared_interior_subpaths_matches_every_stage() {
    let cases = load_cases();
    let checked = check_pass(
        &cases,
        "nudgeSharedInteriorSubpaths",
        passes::nudge_shared_interior_subpaths,
    );
    assert_eq!(checked, 16, "11 runs with #1 and 5 with #2 in the corpus");
}

#[test]
fn straighten_collinear_sibling_detours_matches_every_stage() {
    let cases = load_cases();
    let checked = check_pass(
        &cases,
        "straightenCollinearSiblingDetours",
        passes::straighten_collinear_sibling_detours,
    );
    assert_eq!(checked, 52, "the corpus has 52 stages of this pass");
}
