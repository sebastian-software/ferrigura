//! Geometry passes of `postProcessSwimlaneLayout` (`direction/*.ts`).
//!
//! Each pass takes the layout data and runs on it in place, as the TypeScript pass runs on the layout. The node
//! map of `postProcessing.ts` (`nodeByIdMap`) is built from the nodes by [`node_by_id_map`].

use indexmap::IndexMap;

use crate::model::Node;

pub(crate) mod geometry_stopgap;
pub(crate) mod port_swap;
pub(crate) mod shared_track_nudging;
pub(crate) mod sibling_shared_face_routing;

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
