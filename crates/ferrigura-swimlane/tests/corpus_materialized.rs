//! The materialized-geometry passes of `postProcessSwimlaneLayout` against the reference corpus, one pass at a
//! time.
//!
//! For each case, the state before a present `post:<pass>` stage is the data of the stage before it in the list.
//! For the first post stage that is `routed`, with the curve rule of the core applied (`layoutCore.ts`, the loop
//! after the routing: `curve` becomes `rounded` where it is absent or `basis`). The pass runs on that state, and
//! its result is compared with the stage exactly (`support::corpus::compare_snapshot`).
//!
//! Only the present stages are checked. A pass with no stage of its own changed nothing, and the full chain test
//! covers the cases where a pass is absent.

mod support;

use std::collections::BTreeMap;

use ferrigura_swimlane::testing::{
    collapse_redundant_rectangular_doglegs, layout_data_from_json, layout_data_to_json,
    lift_obstacle_hugging_same_side_rails, lift_top_lane_title_bands_above_rails,
    reassign_crossing_external_rail_channels, resolve_rendered_orthogonal_crossings,
    separate_shared_rendered_terminal_lanes, shift_left_lane_title_bands_left_of_rails,
    shortcut_redundant_orthogonal_jogs, swap_destination_terminal_tails_to_reduce_crossings,
};
use ferrigura_swimlane::{Direction, LayoutData};
use support::corpus::{Case, compare_snapshot, load_cases};

/// A ported pass, run on a layout in the direction of the case.
type PassFn = fn(&mut LayoutData, Direction);

/// The passes of this file, by the name of their stage (without `post:` and the `#k` of a repeat). A pass
/// joins the list when its stages match the reference.
const PORTED: &[&str] = &[
    "separateSharedRenderedTerminalLanes",
    "collapseRedundantRectangularDoglegs",
    "liftObstacleHuggingSameSideRails",
    "swapDestinationTerminalTailsToReduceCrossings",
    "reassignCrossingExternalRailChannels",
    "resolveRenderedOrthogonalCrossings",
    "shortcutRedundantOrthogonalJogs",
    "liftTopLaneTitleBandsAboveRails",
    "shiftLeftLaneTitleBandsLeftOfRails",
];

/// The runner of a ported pass, or `None` for a pass that this file does not check.
fn ported(name: &str) -> Option<PassFn> {
    match name {
        "separateSharedRenderedTerminalLanes" => Some(run_separate_shared_rendered_terminal_lanes),
        "collapseRedundantRectangularDoglegs" => Some(run_collapse_redundant_rectangular_doglegs),
        "liftObstacleHuggingSameSideRails" => Some(run_lift_obstacle_hugging_same_side_rails),
        "swapDestinationTerminalTailsToReduceCrossings" => {
            Some(run_swap_destination_terminal_tails_to_reduce_crossings)
        }
        "reassignCrossingExternalRailChannels" => {
            Some(run_reassign_crossing_external_rail_channels)
        }
        "resolveRenderedOrthogonalCrossings" => Some(run_resolve_rendered_orthogonal_crossings),
        "shortcutRedundantOrthogonalJogs" => Some(run_shortcut_redundant_orthogonal_jogs),
        "liftTopLaneTitleBandsAboveRails" => Some(run_lift_top_lane_title_bands_above_rails),
        "shiftLeftLaneTitleBandsLeftOfRails" => Some(run_shift_left_lane_title_bands_left_of_rails),
        _ => None,
    }
}

fn run_separate_shared_rendered_terminal_lanes(layout: &mut LayoutData, _direction: Direction) {
    separate_shared_rendered_terminal_lanes(layout);
}

fn run_collapse_redundant_rectangular_doglegs(layout: &mut LayoutData, _direction: Direction) {
    collapse_redundant_rectangular_doglegs(layout);
}

fn run_lift_obstacle_hugging_same_side_rails(layout: &mut LayoutData, _direction: Direction) {
    lift_obstacle_hugging_same_side_rails(layout);
}

fn run_swap_destination_terminal_tails_to_reduce_crossings(
    layout: &mut LayoutData,
    _direction: Direction,
) {
    swap_destination_terminal_tails_to_reduce_crossings(layout);
}

fn run_reassign_crossing_external_rail_channels(layout: &mut LayoutData, _direction: Direction) {
    reassign_crossing_external_rail_channels(layout);
}

fn run_resolve_rendered_orthogonal_crossings(layout: &mut LayoutData, _direction: Direction) {
    resolve_rendered_orthogonal_crossings(layout);
}

fn run_shortcut_redundant_orthogonal_jogs(layout: &mut LayoutData, _direction: Direction) {
    shortcut_redundant_orthogonal_jogs(layout);
}

fn run_lift_top_lane_title_bands_above_rails(layout: &mut LayoutData, direction: Direction) {
    lift_top_lane_title_bands_above_rails(layout, direction);
}

fn run_shift_left_lane_title_bands_left_of_rails(layout: &mut LayoutData, direction: Direction) {
    shift_left_lane_title_bands_left_of_rails(layout, direction);
}

/// The state before the first post stage: the routed layout with the curve rule of the core applied.
fn routed_state(routed: &serde_json::Value) -> LayoutData {
    let mut layout = layout_data_from_json(routed).expect("the routed stage reads as a layout");
    for edge in &mut layout.edges {
        if matches!(edge.curve.as_deref(), None | Some("") | Some("basis")) {
            edge.curve = Some("rounded".to_owned());
        }
    }
    layout
}

/// The name of a post stage without its prefix and its repeat count: `post:liftTopLaneTitleBandsAboveRails#2`
/// is `liftTopLaneTitleBandsAboveRails`.
fn pass_name(stage: &str) -> Option<&str> {
    let name = stage.strip_prefix("post:")?;
    Some(name.split('#').next().unwrap_or(name))
}

#[derive(Default)]
struct Tally {
    present: usize,
    matched: usize,
    first_failure: Option<String>,
}

/// Runs every present stage of one case through the ported passes, and records the outcome per pass.
fn check_case(case: &Case, tally: &mut BTreeMap<String, Tally>) {
    let direction = Direction::parse(&case.returned_direction)
        .unwrap_or_else(|| panic!("{}: unknown direction", case.name));
    let mut before: Option<LayoutData> = None;
    for (stage, data) in &case.stages {
        if stage == "routed" {
            before = Some(routed_state(data));
            continue;
        }
        let Some(name) = pass_name(stage) else {
            continue;
        };
        let state = before
            .clone()
            .unwrap_or_else(|| panic!("{}: a post stage without routed", case.name));
        if let Some(run) = ported(name) {
            let entry = tally.entry(name.to_owned()).or_default();
            entry.present += 1;
            let mut after = state;
            run(&mut after, direction);
            match compare_snapshot(data, &layout_data_to_json(&after)) {
                Ok(()) => entry.matched += 1,
                Err(message) => {
                    if entry.first_failure.is_none() {
                        entry.first_failure = Some(format!("{} {stage}: {message}", case.name));
                    }
                }
            }
        }
        before = Some(layout_data_from_json(data).expect("a post stage reads as a layout"));
    }
}

#[test]
fn materialized_passes_match_the_reference_corpus() {
    let cases = load_cases();
    let mut tally: BTreeMap<String, Tally> = BTreeMap::new();
    for case in &cases {
        check_case(case, &mut tally);
    }

    eprintln!("materialized passes against {} cases:", cases.len());
    for (name, entry) in &tally {
        eprintln!(
            "  {name}: {} of {} present stages match",
            entry.matched, entry.present
        );
    }

    let failures: Vec<&str> = tally
        .values()
        .filter_map(|entry| entry.first_failure.as_deref())
        .collect();
    assert!(
        failures.is_empty(),
        "first failure of each pass:\n{}",
        failures.join("\n")
    );
    for name in PORTED {
        assert!(
            tally.get(*name).is_some_and(|entry| entry.present > 0),
            "{name} has no present stage in the corpus"
        );
    }
}
