//! The geometry passes of `postProcessSwimlaneLayout` against the reference corpus, one pass at a time.
//!
//! For each case, the state before a present `post:<pass>` stage is the data of the stage before it in the
//! list. For the first post stage that is `routed`, with the curve rule of the core applied (`layoutCore.ts`,
//! the loop after the routing: `curve` becomes `rounded` where it is absent or `basis`). The pass runs on that
//! state, and its result is compared with the stage exactly (`support::corpus::compare_snapshot`).
//!
//! Only the present stages are checked. A pass with no stage of its own changed nothing, and the full chain
//! test covers the cases where a pass is absent.

mod support;

use std::collections::BTreeMap;

use ferrigura_swimlane::testing::{
    apply_bt_direction_transform, apply_lr_direction_transform,
    clip_edge_endpoints_to_node_boundaries, layout_data_from_json, layout_data_to_json,
    orthogonalize_and_simplify_edges, prepare_edge_endpoints_for_renderer,
};
use ferrigura_swimlane::{Direction, LayoutData};
use support::corpus::{Case, compare_snapshot, load_cases};

/// A ported pass, run on a layout in the direction of the case.
type PassFn = fn(&mut LayoutData, Direction);

/// The passes of this file, by the name of their stage (without `post:` and the `#k` of a repeat).
const PORTED: &[&str] = &[
    "applyLrDirectionTransform",
    "applyBtDirectionTransform",
    "orthogonalizePolyline+simplifyPolyline",
    "clipEdgeEndpointsToNodeBoundaries",
    "prepareEdgeEndpointsForRenderer",
];

/// The runner of a ported pass, or `None` for a pass that this file does not check.
fn ported(name: &str) -> Option<PassFn> {
    match name {
        "applyLrDirectionTransform" => Some(run_lr_direction_transform),
        "applyBtDirectionTransform" => Some(run_bt_direction_transform),
        "orthogonalizePolyline+simplifyPolyline" => Some(run_orthogonalize_and_simplify),
        "clipEdgeEndpointsToNodeBoundaries" => Some(run_clip_edge_endpoints),
        "prepareEdgeEndpointsForRenderer" => Some(run_prepare_edge_endpoints),
        _ => None,
    }
}

fn run_lr_direction_transform(layout: &mut LayoutData, direction: Direction) {
    apply_lr_direction_transform(layout, direction);
}

fn run_bt_direction_transform(layout: &mut LayoutData, _direction: Direction) {
    apply_bt_direction_transform(layout);
}

fn run_clip_edge_endpoints(layout: &mut LayoutData, _direction: Direction) {
    clip_edge_endpoints_to_node_boundaries(layout);
}

fn run_prepare_edge_endpoints(layout: &mut LayoutData, _direction: Direction) {
    prepare_edge_endpoints_for_renderer(layout);
}

fn run_orthogonalize_and_simplify(layout: &mut LayoutData, _direction: Direction) {
    orthogonalize_and_simplify_edges(layout);
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

/// The name of a post stage without its prefix and its repeat count: `post:anchorLabelsToPolyline#2` is
/// `anchorLabelsToPolyline`.
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
fn geometry_passes_match_the_reference_corpus() {
    let cases = load_cases();
    let mut tally: BTreeMap<String, Tally> = BTreeMap::new();
    for case in &cases {
        check_case(case, &mut tally);
    }

    eprintln!("geometry passes against {} cases:", cases.len());
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
