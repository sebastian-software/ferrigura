//! Cycle removal (`phase1.cycles.ts`, `removeCycles_DFS`, lines 11-81).
//!
//! The depth-first search of the reference is recursive. This port runs the same search with an explicit
//! stack, so a long chain of nodes cannot overflow the thread's stack. The order of the visits and of the
//! recorded back edges is the same.

use std::collections::{HashMap, HashSet};

use crate::helpers::{EdgeRef, Graph, NodeId};
use crate::js::locale_compare;
use crate::phase0::{build_in_degree_map, normalize_graph};

/// The result of [`remove_cycles_dfs`]: the acyclic graph, and the edges that were reversed, in their
/// original orientation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CycleRemoval {
    /// `acyclic`.
    pub acyclic: Graph,
    /// `reversed`.
    pub reversed: Vec<EdgeRef>,
}

const WHITE: u8 = 0;
const GRAY: u8 = 1;
const BLACK: u8 = 2;

/// `removeCycles_DFS`: reverses the back edges of a depth-first search. The search starts from the nodes
/// of in-degree zero, in the order of the nodes, then from the rest, in the same order. Each node's edges
/// are visited in order of destination, then of id, both by `localeCompare`.
pub(crate) fn remove_cycles_dfs(g: &Graph) -> CycleRemoval {
    let gn = normalize_graph(g);

    let index_of: HashMap<&str, usize> = gn
        .nodes
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();

    // The out-edges of each node, as indices into `gn.edges`, sorted by (dst, id).
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); gn.nodes.len()];
    for (edge_index, e) in gn.edges.iter().enumerate() {
        adj[index_of[e.src.as_str()]].push(edge_index);
    }
    for list in &mut adj {
        list.sort_by(|&a, &b| {
            let (ea, eb) = (&gn.edges[a], &gn.edges[b]);
            if ea.dst == eb.dst {
                locale_compare(&ea.id, &eb.id)
            } else {
                locale_compare(&ea.dst, &eb.dst)
            }
        });
    }

    let mut color = vec![WHITE; gn.nodes.len()];
    let mut reversed_edges: Vec<usize> = Vec::new();

    let indeg = build_in_degree_map(&gn);
    let is_source = |id: &NodeId| indeg.get(id).copied().unwrap_or(0) == 0;
    let seeds: Vec<usize> = (0..gn.nodes.len())
        .filter(|&v| is_source(&gn.nodes[v]))
        .chain((0..gn.nodes.len()).filter(|&v| !is_source(&gn.nodes[v])))
        .collect();

    for seed in seeds {
        if color[seed] != WHITE {
            continue;
        }
        // Each frame is a node and the position of its next out-edge.
        let mut stack: Vec<(usize, usize)> = vec![(seed, 0)];
        color[seed] = GRAY;
        while let Some(frame) = stack.last_mut() {
            let (u, next) = *frame;
            if next == adj[u].len() {
                color[u] = BLACK;
                stack.pop();
                continue;
            }
            frame.1 += 1;
            let edge_index = adj[u][next];
            let v = index_of[gn.edges[edge_index].dst.as_str()];
            match color[v] {
                WHITE => {
                    color[v] = GRAY;
                    stack.push((v, 0));
                }
                GRAY => reversed_edges.push(edge_index),
                _ => {}
            }
        }
    }

    let reversed: Vec<EdgeRef> = reversed_edges
        .iter()
        .map(|&edge_index| gn.edges[edge_index].clone())
        .collect();

    // Every edge with the key of a reversed edge is reversed, as the reference does with its key set.
    let key = |e: &EdgeRef| format!("{}:{}->{}", e.id, e.src, e.dst);
    let to_reverse: HashSet<String> = reversed.iter().map(key).collect();
    let acyclic_edges: Vec<EdgeRef> = gn
        .edges
        .iter()
        .map(|e| {
            if to_reverse.contains(&key(e)) {
                EdgeRef {
                    id: e.id.clone(),
                    src: e.dst.clone(),
                    dst: e.src.clone(),
                    origin: e.origin.clone(),
                }
            } else {
                e.clone()
            }
        })
        .collect();

    let acyclic = Graph {
        nodes: gn.nodes.clone(),
        edges: acyclic_edges,
        node_by_id: gn.node_by_id.clone(),
        top_level_groups: gn.top_level_groups.clone(),
    };
    CycleRemoval { acyclic, reversed }
}
