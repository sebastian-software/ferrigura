//! Port of `packages/mermaid/src/diagrams/state/stateDiagram-colorIndex.spec.ts`: the container
//! colour slots that `getData()` gives the composite states and their concurrency regions. The
//! spec reads `getData().nodes` after the grammar and the DB have parsed the text, as
//! [`parse_grammar`] does, so the texts are the spec's and the expectations are its own.
//!
//! The titles are the spec's, in snake_case.

use std::collections::HashMap;

use ferrigura_model::config::MermaidConfig;
use ferrigura_render::context::Globals;
use ferrigura_render::state::data::get_data;
use ferrigura_render::unified::Section;
use ferrigura_render::unified::data::Node;
use ferrigura_syntax::state::parse_grammar;

/// The nodes of the text's layout data.
fn nodes(text: &str) -> Vec<Node> {
    let diagram = parse_grammar(text).expect("parses");
    get_data(
        &diagram,
        &Section::of(&MermaidConfig::default()),
        &Globals::of(&MermaidConfig::default()),
    )
    .nodes
}

/// The colour slot of each node, by id.
fn slots(text: &str) -> HashMap<String, Option<u32>> {
    nodes(text)
        .into_iter()
        .map(|node| (node.id, node.color_index))
        .collect()
}

/// The concurrency regions: the nodes of the `divider` shape.
fn regions(nodes: &[Node]) -> Vec<&Node> {
    nodes
        .iter()
        .filter(|node| node.shape == "divider")
        .collect()
}

/// The slot of a node that is a state of the text.
fn slot(map: &HashMap<String, Option<u32>>, id: &str) -> Option<u32> {
    *map.get(id).expect("the node is in the layout data")
}

#[test]
fn gives_each_composite_its_own_slot_in_containment_order() {
    let by_id = slots(
        "stateDiagram-v2
      [*] --> Boot
      state Boot {
        [*] --> Firmware
        state Kernel {
          [*] --> Scheduler
          state Drivers {
            [*] --> Probe
          }
        }
      }
      Boot --> Running
      state Running {
        [*] --> Serving
      }
    ",
    );
    // Pre-order: a composite is numbered before the composites it contains, so depth is what
    // separates the colours rather than declaration order across the whole file.
    assert_eq!(slot(&by_id, "Boot"), Some(0));
    assert_eq!(slot(&by_id, "Kernel"), Some(1));
    assert_eq!(slot(&by_id, "Drivers"), Some(2));
    assert_eq!(slot(&by_id, "Running"), Some(3));
}

#[test]
fn leaves_plain_states_unslotted_so_only_containers_are_painted() {
    let by_id = slots(
        "stateDiagram-v2
      state Outer {
        [*] --> Inner
        Inner --> Done
      }
    ",
    );
    assert_eq!(slot(&by_id, "Outer"), Some(0));
    assert_eq!(slot(&by_id, "Inner"), None);
    assert_eq!(slot(&by_id, "Done"), None);
}

#[test]
fn gives_every_concurrency_region_of_one_composite_the_same_slot() {
    let all = nodes(
        "stateDiagram-v2
      state Active {
        [*] --> NumLockOff
        --
        [*] --> CapsLockOff
        --
        [*] --> ScrollLockOff
      }
    ",
    );
    let by_id: HashMap<String, Option<u32>> = all
        .iter()
        .map(|node| (node.id.clone(), node.color_index))
        .collect();
    let region_slots: Vec<Option<u32>> =
        regions(&all).iter().map(|node| node.color_index).collect();

    // Counted, because two `--` produce three regions: one per separator plus the trailing
    // remainder. Asserting only on the distinct values would pass if a region went missing.
    assert_eq!(region_slots.len(), 3);
    assert!(region_slots.windows(2).all(|pair| pair[0] == pair[1]));
    // ...and it is the composite's own slot, not a fresh one.
    assert_eq!(slot(&by_id, "Active"), Some(0));
    assert_eq!(region_slots[0], Some(0));
}

#[test]
fn does_not_shift_later_composites_when_a_divider_is_added() {
    let without_divider = slots(
        "stateDiagram-v2
      state First {
        [*] --> A
      }
      state Second {
        [*] --> B
      }
    ",
    );
    let with_divider = slots(
        "stateDiagram-v2
      state First {
        [*] --> A
        --
        [*] --> C
      }
      state Second {
        [*] --> B
      }
    ",
    );

    assert_eq!(
        slot(&without_divider, "First"),
        slot(&with_divider, "First")
    );
    assert_eq!(
        slot(&without_divider, "Second"),
        slot(&with_divider, "Second")
    );
    assert_eq!(slot(&with_divider, "Second"), Some(1));
}

#[test]
fn carries_a_styled_composite_s_opt_out_into_its_concurrency_regions() {
    let all = nodes(
        "stateDiagram-v2
      classDef pinned fill:#111827,stroke:#F59E0B
      state Active {
        [*] --> A
        --
        [*] --> B
      }
      class Active pinned
    ",
    );
    let by_id: HashMap<String, Option<u32>> = all
        .iter()
        .map(|node| (node.id.clone(), node.color_index))
        .collect();
    assert_eq!(slot(&by_id, "Active"), None);
    let region_nodes = regions(&all);
    assert_eq!(region_nodes.len(), 2);
    assert!(
        region_nodes
            .iter()
            .all(|region| region.color_index.is_none())
    );
}

#[test]
fn does_not_share_one_slot_between_two_separately_divided_composites() {
    let all = nodes(
        "stateDiagram-v2
      state First {
        [*] --> A
        --
        [*] --> B
      }
      state Second {
        [*] --> C
        --
        [*] --> D
      }
    ",
    );
    let by_id: HashMap<String, Option<u32>> = all
        .iter()
        .map(|node| (node.id.clone(), node.color_index))
        .collect();
    let region_nodes = regions(&all);

    assert_ne!(slot(&by_id, "First"), slot(&by_id, "Second"));
    assert_eq!(region_nodes.len(), 4);
    // Two composites, two regions each: two distinct region colours, not one and not four.
    let distinct: std::collections::BTreeSet<Option<u32>> =
        region_nodes.iter().map(|node| node.color_index).collect();
    assert_eq!(distinct.len(), 2);
    // And the pairing is by parent: each composite's regions carry that composite's own slot.
    for (composite, parent_slot) in [("First", "First"), ("Second", "Second")] {
        let own: std::collections::BTreeSet<Option<u32>> = region_nodes
            .iter()
            .filter(|node| node.parent_id.as_deref() == Some(composite))
            .map(|node| node.color_index)
            .collect();
        assert_eq!(own.len(), 1, "the regions of {composite}");
        assert_eq!(own.iter().next().copied(), Some(slot(&by_id, parent_slot)));
    }
}

#[test]
fn leaves_a_composite_with_its_own_classdef_unstamped_but_still_spends_its_slot() {
    let by_id = slots(
        "stateDiagram-v2
      classDef pinned fill:#111827,stroke:#F59E0B
      state First {
        [*] --> A
      }
      state Second {
        [*] --> B
      }
      state Third {
        [*] --> C
      }
      class Second pinned
    ",
    );
    assert_eq!(slot(&by_id, "First"), Some(0));
    // Unstamped, so no `[data-color-id]` rule can match it: the author's class is the only thing
    // painting the container.
    assert_eq!(slot(&by_id, "Second"), None);
    // The slot is still consumed: `Third` keeps the colour it would have had anyway.
    assert_eq!(slot(&by_id, "Third"), Some(2));
}

#[test]
fn leaves_a_composite_with_its_own_style_statement_unstamped() {
    let by_id = slots(
        "stateDiagram-v2
      state First {
        [*] --> A
      }
      state Second {
        [*] --> B
      }
      style Second fill:#111827
    ",
    );
    assert_eq!(slot(&by_id, "First"), Some(0));
    assert_eq!(slot(&by_id, "Second"), None);
}

#[test]
fn spends_one_slot_on_a_composite_named_twice_so_the_cycle_does_not_skip() {
    let by_id = slots(
        "stateDiagram-v2
      [*] --> Loop
      state Loop {
        [*] --> Spin
      }
      Loop --> Loop
      Loop --> Exit
      state Exit {
        [*] --> Bye
      }
    ",
    );
    assert_eq!(slot(&by_id, "Loop"), Some(0));
    assert_eq!(slot(&by_id, "Exit"), Some(1));
}
