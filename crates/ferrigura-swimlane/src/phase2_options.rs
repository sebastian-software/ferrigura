//! The top lane of each node, and the order of the lanes (`phase2.options.ts`).
//!
//! Ported: `buildTopLaneMap` (23-50), `createTopLaneResolver` (52-55), `buildTopLaneOrder` (57-65) and
//! `resolveTopLaneOrder` (67-90). `resolveTopLaneOrder` takes a preferred order, which only the
//! non-default `automaticLaneOrdering` provides; the core refuses that option, so the default path always
//! passes none.

use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};

use crate::helpers::{Graph, NodeId};

/// For each node of the graph, its top lane: the id of the outermost group that holds it, or the id of
/// its own parent when the parent is not inside a group, or `None` for a node without a parent.
///
/// The map has an entry for every node of the graph and for each of their ancestors, as the reference's
/// cache does. A cycle of parents makes the reference recurse until the stack overflows. This port stops the
/// recursion at the node it re-enters, which counts as having no top lane for that one step.
pub(crate) fn build_top_lane_map(g: &Graph) -> IndexMap<NodeId, Option<NodeId>> {
    let mut cache: IndexMap<NodeId, Option<NodeId>> = IndexMap::new();
    let mut in_progress: HashSet<NodeId> = HashSet::new();
    for id in &g.nodes {
        resolve(g, &mut cache, &mut in_progress, id);
    }
    cache
}

fn resolve(
    g: &Graph,
    cache: &mut IndexMap<NodeId, Option<NodeId>>,
    in_progress: &mut HashSet<NodeId>,
    id: &str,
) -> Option<NodeId> {
    if let Some(lane) = cache.get(id) {
        return lane.clone();
    }
    let Some(node) = g.node_by_id.get(id) else {
        cache.insert(id.to_owned(), None);
        return None;
    };
    // `if (!parentId)`: an absent or empty parent id is no parent.
    let parent = match node.parent_id.as_deref() {
        Some(parent) if !parent.is_empty() => parent.to_owned(),
        _ => {
            cache.insert(id.to_owned(), None);
            return None;
        }
    };
    if !in_progress.insert(id.to_owned()) {
        return None;
    }
    let parent_lane = resolve(g, cache, in_progress, &parent);
    in_progress.remove(id);
    // `parentLane ?? parentId`.
    let lane = parent_lane.unwrap_or(parent);
    cache.insert(id.to_owned(), Some(lane.clone()));
    Some(lane)
}

/// `topLaneMap.get(id) ?? null`: the top lane of a node, looked up in a map from [`build_top_lane_map`].
/// A node that the map does not hold has no top lane.
pub(crate) fn top_lane_of(map: &IndexMap<NodeId, Option<NodeId>>, id: &str) -> Option<NodeId> {
    map.get(id).cloned().flatten()
}

/// `buildTopLaneOrder`: the top-level groups of the layout, in the reverse of their order, without
/// duplicates.
pub(crate) fn build_top_lane_order(g: &Graph) -> Vec<NodeId> {
    let unique: IndexSet<&NodeId> = g.top_level_groups.iter().collect();
    unique.into_iter().rev().cloned().collect()
}

/// `resolveTopLaneOrder`: the lane order, with the preferred lanes first where they are known lanes, then
/// the other lanes in the source order. With no preferred order, the source order.
pub(crate) fn resolve_top_lane_order(g: &Graph, preferred: Option<&[NodeId]>) -> Vec<NodeId> {
    let source_order = build_top_lane_order(g);
    let preferred = match preferred {
        Some(preferred) if !preferred.is_empty() => preferred,
        _ => return source_order,
    };

    let source_lane_ids: HashSet<&NodeId> = source_order.iter().collect();
    let mut seen: HashSet<&NodeId> = HashSet::new();
    let mut resolved: Vec<NodeId> = Vec::new();
    for lane_id in preferred {
        if !source_lane_ids.contains(lane_id) || seen.contains(lane_id) {
            continue;
        }
        seen.insert(lane_id);
        resolved.push(lane_id.clone());
    }
    for lane_id in &source_order {
        if seen.contains(lane_id) {
            continue;
        }
        resolved.push(lane_id.clone());
    }
    resolved
}
