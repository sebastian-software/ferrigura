//! Placeholders for `packages/mermaid/src/diagrams/state/stateDiagram-colorIndex.spec.ts`. Every
//! test reads the colour slots from `getData().nodes`, which the port does not model here (the
//! data differential covers it), so the bodies are not ported. Each test keeps the spec's title,
//! in snake_case, and its own `#[ignore]` reason.

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn gives_each_composite_its_own_slot_in_containment_order() {
    // stateDiagram-colorIndex.spec.ts: "gives each composite its own slot, in containment order"
}

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn leaves_plain_states_unslotted_so_only_containers_are_painted() {
    // stateDiagram-colorIndex.spec.ts: "leaves plain states unslotted, so only containers are painted"
}

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn gives_every_concurrency_region_of_one_composite_the_same_slot() {
    // stateDiagram-colorIndex.spec.ts: "gives every concurrency region of one composite the same slot"
}

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn does_not_shift_later_composites_when_a_divider_is_added() {
    // stateDiagram-colorIndex.spec.ts: "does not shift later composites when a divider is added"
}

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn carries_a_styled_composite_s_opt_out_into_its_concurrency_regions() {
    // stateDiagram-colorIndex.spec.ts: "carries a styled composite's opt-out into its concurrency regions"
}

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn does_not_share_one_slot_between_two_separately_divided_composites() {
    // stateDiagram-colorIndex.spec.ts: "does not share one slot between two separately divided composites"
}

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn leaves_a_composite_with_its_own_classdef_unstamped_but_still_spends_its_slot() {
    // stateDiagram-colorIndex.spec.ts: "leaves a composite with its own classDef unstamped, but still spends its slot"
}

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn leaves_a_composite_with_its_own_style_statement_unstamped() {
    // stateDiagram-colorIndex.spec.ts: "leaves a composite with its own style statement unstamped"
}

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn spends_one_slot_on_a_composite_named_twice_so_the_cycle_does_not_skip() {
    // stateDiagram-colorIndex.spec.ts: "spends one slot on a composite named twice, so the cycle does not skip"
}
