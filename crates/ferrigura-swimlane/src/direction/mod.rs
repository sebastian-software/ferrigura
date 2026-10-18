//! Geometry passes of `postProcessSwimlaneLayout` (`direction/*.ts`).
//!
//! Each pass takes the layout data and runs on it in place, as the TypeScript pass runs on the layout. The node
//! map of `postProcessing.ts` (`nodeByIdMap`) is built from the nodes by [`node_by_id_map`].

use indexmap::IndexMap;

use crate::model::Node;

pub(crate) mod detour_simplification;
pub(crate) mod endpoint_clip;
pub(crate) mod geometry;
pub(crate) mod label_anchoring;
pub(crate) mod lr_transform;
pub(crate) mod port_swap;
pub(crate) mod shared_track_nudging;
pub(crate) mod sibling_shared_face_routing;
pub(crate) mod terminal_stub;

/// `nodeByIdMap` of `postProcessing.ts`: the index in `nodes` of the node with each id, in the insertion order of a
/// JavaScript `Map`. A repeated id keeps the position of its first node and the index of its last one, as
/// `Map.prototype.set` does.
pub(crate) fn node_by_id_map(nodes: &[Node]) -> IndexMap<String, usize> {
    let mut map = IndexMap::new();
    for (index, node) in nodes.iter().enumerate() {
        map.insert(node.id.clone(), index);
    }
    map
}

/// The nodes of `nodeByIdMap`, in the order of the map: the node that each id names, the last one of a repeated id.
/// The TypeScript passes iterate `nodeByIdMap.values()`. The geometry helpers take a node slice, and this copy is the
/// slice of those nodes.
pub(crate) fn nodes_by_id(nodes: &[Node], node_by_id: &IndexMap<String, usize>) -> Vec<Node> {
    node_by_id
        .values()
        .map(|&index| nodes[index].clone())
        .collect()
}
