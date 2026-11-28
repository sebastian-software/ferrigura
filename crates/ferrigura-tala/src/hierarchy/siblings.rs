// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from TALA's recursive container fitting and bin packing.
// See NOTICE.md for the original revision.

use crate::{Graph, Layout, NodeGeometry, placement};

use super::{CONTAINER_PADDING, fitted_frame, is_fitted_group, is_rectangular_leaf, node_geometry};

pub(super) fn layout(
    graph: &Graph,
    children: &[Vec<usize>],
    [first, second]: [usize; 2],
    outer: Option<usize>,
) -> Option<Layout> {
    if outer.is_some_and(|outer| !is_fitted_group(&graph.nodes[outer])) {
        return None;
    }
    let branches = [
        unary_branch(graph, children, first)?,
        unary_branch(graph, children, second)?,
    ];
    // A cycle cannot belong to either rooted branch. Reject any unvisited
    // nodes, including disconnected cycles and additional components.
    if branches[0].len() + branches[1].len() + usize::from(outer.is_some()) != graph.nodes.len() {
        return None;
    }

    let mut nodes: Vec<_> = graph.nodes.iter().map(node_geometry).collect();
    for branch in &branches {
        fit_branch(graph, &mut nodes, branch);
    }
    let positions = placement::pack_frame_pair(&nodes[first], &nodes[second])?;
    let padding = if outer.is_some() {
        CONTAINER_PADDING
    } else {
        0.0
    };
    let mut width: f64 = 0.0;
    let mut height: f64 = 0.0;
    for (branch, position) in branches.iter().zip(positions) {
        let frame = &nodes[branch[0]];
        width = width.max(position.x + frame.width);
        height = height.max(position.y + frame.height);

        let mut offset = padding;
        for &index in branch {
            nodes[index].x = position.x + offset;
            nodes[index].y = position.y + offset;
            offset += CONTAINER_PADDING;
        }
    }
    if let Some(outer) = outer {
        (nodes[outer].width, nodes[outer].height) =
            fitted_frame(&graph.nodes[outer], width, height);
    }

    Some(Layout {
        nodes,
        edges: Vec::new(),
    })
}

fn unary_branch(graph: &Graph, children: &[Vec<usize>], mut index: usize) -> Option<Vec<usize>> {
    let mut branch = Vec::new();
    loop {
        let node = &graph.nodes[index];
        match children[index].as_slice() {
            [child] if is_fitted_group(node) => {
                branch.push(index);
                index = *child;
            }
            [] if !branch.is_empty() && is_rectangular_leaf(node) => {
                branch.push(index);
                return Some(branch);
            }
            _ => return None,
        }
    }
}

fn fit_branch(graph: &Graph, geometry: &mut [NodeGeometry], branch: &[usize]) {
    if let Some((&leaf, groups)) = branch.split_last() {
        let mut size = (geometry[leaf].width, geometry[leaf].height);
        for &group in groups.iter().rev() {
            size = fitted_frame(&graph.nodes[group], size.0, size.1);
            (geometry[group].width, geometry[group].height) = size;
        }
    }
}
