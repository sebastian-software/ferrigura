//! The colour slots of agentflow: a port of `diagrams/agentflow/colorSlots.ts`. A node takes a colour
//! by its kind, a fixed slot per kind; a container takes a slot from a counter over the declaration
//! order, above the kinds' slots, so a frame never has a node's colour (`colorSlots.ts` 1-40).

use crate::unified::data::Node;

/// The kinds of a node, in the order of their palette slots: `tool` is the first colour, `task` the
/// second, and so on (`KIND_SLOT`, `colorSlots.ts` 25-37).
pub const KINDS: [&str; 7] = [
    "tool",
    "task",
    "decision",
    "input",
    "refdoc",
    "connector",
    "action",
];

/// The number of kinds, which the containers' slots start above (`KIND_COUNT`).
pub const KIND_COUNT: usize = KINDS.len();

/// The class that marks a node's kind, which the stylesheet's rules match (`kindClass`).
#[must_use]
pub fn kind_class(kind: &str) -> String {
    format!("af-kind-{kind}")
}

/// The palette slot of a kind: its index in [`KINDS`].
#[must_use]
pub fn kind_slot(kind: &str) -> Option<usize> {
    KINDS.iter().position(|known| *known == kind)
}

/// How many slots the containers cycle through, given the palette's length: at least one
/// (`containerSlotCount`, `colorSlots.ts` 43-46).
#[must_use]
pub fn container_slot_count(palette_len: usize) -> usize {
    palette_len.saturating_sub(KIND_COUNT).max(1)
}

/// The palette slot of the `n`th container (`containerSlot`, `colorSlots.ts` 56-62): the kinds' count
/// plus the counter, wrapped to the palette. A palette of no colours gives the raw slot.
#[must_use]
pub fn container_slot(n: usize, palette_len: usize) -> usize {
    let slot = KIND_COUNT + (n % container_slot_count(palette_len));
    if palette_len > 0 {
        slot % palette_len
    } else {
        slot
    }
}

/// `assignColorSlots` (`colorSlots.ts` 64-119): every container and collapsed container takes its slot
/// from `order` (the declaration order), and a node whose kind is one of the kinds gets the kind's class.
/// `kind_of` gives the kind of a node by its id: `connector` for a connector, else the vertex's kind.
/// A container the order does not name takes the next number after the named ones.
pub fn assign<'k>(
    nodes: &mut [Node],
    palette_len: usize,
    order: &std::collections::HashMap<&str, usize>,
    kind_of: impl Fn(&str) -> Option<&'k str>,
) {
    let mut fallback = order.len();
    for node in nodes.iter_mut() {
        // A collapsed container is drawn as a leaf, and it keeps its slot like an expanded one.
        if node.is_group || node.shape == "collapsedGroup" {
            let n = order.get(node.id.as_str()).copied().unwrap_or_else(|| {
                let next = fallback;
                fallback += 1;
                next
            });
            node.color_index = Some(container_slot(n, palette_len) as u32);
            continue;
        }
        let Some(kind) = kind_of(&node.id) else {
            continue;
        };
        if kind_slot(kind).is_some() {
            let classes = format!("{} {}", node.css_classes, kind_class(kind));
            node.css_classes = classes.split_whitespace().collect::<Vec<_>>().join(" ");
        }
    }
}
