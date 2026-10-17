//! `executeTidyTreeLayout` (`layout.ts` 23-72) without its promise: the root's children are split
//! between a left and a right tree (`convertToDualTreeFormat`, 80-138), each tree is laid out by the
//! library with its boxes turned (`buildSubTree` 144-160, `convertNodeToTidyTreeTransposed` 166-184),
//! and the trees are placed around the root (`combineAndPositionTrees` 189-292,
//! `positionLeftTreeBidirectional` 298-326, `positionRightTreeBidirectional` 332-360).

use std::collections::{HashMap, HashSet};

use super::edges;
use super::library::{self, Placement, TreeData};
use super::{
    LayoutEdge, LayoutNode, PositionedNode, Section, TidyTreeError, TidyTreeLayout, js_max, js_min,
};

/// The gap between the nodes of a tree and the bottom padding under each node (`layout.ts` 38-39).
const GAP: f64 = 20.0;
const BOTTOM_PADDING: f64 = 40.0;

/// The `y` of the root: its `rootY` (0) plus 20 (`layout.ts` 261).
const ROOT_Y: f64 = 20.0;

/// The space between the root and the first level, beyond half the root's width (`layout.ts` 199).
const ROOT_SPACING: f64 = 30.0;

/// The adjacency of the data (`convertToDualTreeFormat`, 87-104): the nodes by id, the children of
/// each node in the order of the edges, and the root.
struct Adjacency<'a> {
    nodes: &'a [LayoutNode],
    /// The last node with each id (`nodeMap`).
    index: HashMap<&'a str, usize>,
    /// The children of each node that the edges name, in their order.
    children: HashMap<&'a str, Vec<&'a str>>,
    /// The root: the first node without a parent, else the first node (`layout.ts` 106-111).
    root: usize,
}

impl<'a> Adjacency<'a> {
    fn new(nodes: &'a [LayoutNode], edges: &'a [LayoutEdge]) -> Self {
        let mut index = HashMap::new();
        for (position, node) in nodes.iter().enumerate() {
            index.insert(node.id.as_str(), position);
        }
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut has_parent: HashSet<&str> = HashSet::new();
        for edge in edges {
            let (parent, child) = (edge.start.as_str(), edge.end.as_str());
            if !parent.is_empty() && !child.is_empty() {
                children.entry(parent).or_default().push(child);
                has_parent.insert(child);
            }
        }
        let root = nodes
            .iter()
            .position(|node| !has_parent.contains(node.id.as_str()))
            .unwrap_or(0);
        Self {
            nodes,
            index,
            children,
            root,
        }
    }

    /// The children of a node by id, in the order of the edges (empty when it has none).
    fn children_of(&self, id: &str) -> Vec<&'a str> {
        self.children.get(id).cloned().unwrap_or_default()
    }

    /// The children that the ids name, converted (`buildSubTree`, 144-160). A child that the nodes do
    /// not have is left out, and `originals` gets the node of each converted tree node, in pre-order.
    fn side(
        &self,
        ids: &[&str],
        originals: &mut Vec<usize>,
        path: &mut Vec<usize>,
    ) -> Result<Vec<TreeData>, TidyTreeError> {
        let mut converted = Vec::new();
        for id in ids {
            if let Some(&index) = self.index.get(id) {
                converted.push(self.transposed(index, originals, path)?);
            }
        }
        Ok(converted)
    }

    /// `convertNodeToTidyTreeTransposed` (166-184): the box turned by 90 degrees, so the library's
    /// breadth is the node's height and its depth the node's width.
    fn transposed(
        &self,
        index: usize,
        originals: &mut Vec<usize>,
        path: &mut Vec<usize>,
    ) -> Result<TreeData, TidyTreeError> {
        // The reference recurses without end on a cycle, and throws (a stack overflow).
        if path.contains(&index) {
            return Err(TidyTreeError::Cycle);
        }
        let node = &self.nodes[index];
        originals.push(index);
        path.push(index);
        let ids = self.children_of(&node.id);
        let children = self.side(&ids, originals, path)?;
        path.pop();
        Ok(TreeData {
            width: node.height,
            height: node.width,
            children,
        })
    }
}

/// One side of the root before the offset: the node, and its position in the tree the side
/// positions it in (`positionLeftTreeBidirectional`, `positionRightTreeBidirectional`).
#[derive(Debug, Clone, Copy)]
struct SideNode {
    index: usize,
    x: f64,
    y: f64,
}

/// Lays the tree out: `executeTidyTreeLayout` (`layout.ts` 23-72). `nodes` are in the order of the
/// diagram, and `edges` in the order of their parents' children.
pub(super) fn layout(
    nodes: &[LayoutNode],
    edges: &[LayoutEdge],
) -> Result<TidyTreeLayout, TidyTreeError> {
    if nodes.is_empty() {
        return Err(TidyTreeError::NoNodes);
    }
    let adjacency = Adjacency::new(nodes, edges);
    let root = &nodes[adjacency.root];
    let root_children = adjacency.children_of(&root.id);

    // The children alternate: the first, the third, ... go left, the second, the fourth, ... right
    // (`layout.ts` 120-130). The position counts the children the nodes do not have, as the reference does.
    let left_ids: Vec<&str> = root_children.iter().step_by(2).copied().collect();
    let right_ids: Vec<&str> = root_children.iter().skip(1).step_by(2).copied().collect();
    let left = place_side(&adjacency, &left_ids)?;
    let right = place_side(&adjacency, &right_ids)?;

    // The trees are placed `treeSpacing` away from the root, in their direction (199, 204, 207).
    // The depth is the library's `y` and the breadth its `x` (`layout.ts` 311-316 and 345-350).
    let tree_spacing = root.width / 2.0 + ROOT_SPACING;
    let left: Vec<SideNode> = left
        .into_iter()
        .map(|(index, placement)| SideNode {
            index,
            x: (0.0 - tree_spacing) - placement.y,
            y: 0.0 + placement.x,
        })
        .collect();
    let right: Vec<SideNode> = right
        .into_iter()
        .map(|(index, placement)| SideNode {
            index,
            x: (0.0 + tree_spacing) + placement.y,
            y: 0.0 + placement.x,
        })
        .collect();

    let left_offset = -first_level_centre(nodes, &left, Section::Left);
    let right_offset = -first_level_centre(nodes, &right, Section::Right);

    let mut positioned = Vec::with_capacity(1 + left.len() + right.len());
    positioned.push(PositionedNode {
        id: root.id.clone(),
        x: 0.0,
        y: ROOT_Y,
        section: Section::Root,
        width: root.width,
        height: root.height,
        shape: root.shape.clone(),
    });
    for (side, offset, section) in [
        (&left, left_offset, Section::Left),
        (&right, right_offset, Section::Right),
    ] {
        for side_node in side {
            let node = &nodes[side_node.index];
            // `node.x - width / 2` and `node.y + offset + height / 2` (`layout.ts` 268-286), in that order.
            let x = if section == Section::Left {
                side_node.x - node.width / 2.0
            } else {
                side_node.x + node.width / 2.0
            };
            let y = side_node.y + offset + node.height / 2.0;
            positioned.push(PositionedNode {
                id: node.id.clone(),
                x,
                y,
                section,
                width: node.width,
                height: node.height,
                shape: node.shape.clone(),
            });
        }
    }

    let edge_positions = edges::positions(edges, &positioned);
    Ok(TidyTreeLayout {
        nodes: positioned,
        edges: edge_positions,
    })
}

/// Lays one side out with the library (`layout.ts` 144-160 and 49, 54): the side's node indices,
/// each with its placement, in pre-order. A side without children is empty.
fn place_side(
    adjacency: &Adjacency,
    ids: &[&str],
) -> Result<Vec<(usize, Placement)>, TidyTreeError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut originals = Vec::new();
    let mut path = Vec::new();
    let children = adjacency.side(ids, &mut originals, &mut path)?;
    // The virtual root has a box of 1 by 1 (`layout.ts` 151-152), and the library returns it first.
    let virtual_root = TreeData {
        width: 1.0,
        height: 1.0,
        children,
    };
    let placements = library::layout(&virtual_root, GAP, BOTTOM_PADDING)?;
    Ok(originals
        .into_iter()
        .zip(placements.into_iter().skip(1))
        .collect())
}

/// The offset that centres a side's first level, computed the reference's way (`layout.ts`
/// 216-256). The first level is the nodes at the extreme `x` nearest the root; their centre is taken
/// between their tops and bottoms, and the tops are the library's breadth, so the centre is off by
/// the half height of the nodes (Q1).
fn first_level_centre(nodes: &[LayoutNode], side: &[SideNode], section: Section) -> f64 {
    if side.is_empty() {
        return 0.0;
    }
    // The first level is the largest x on the left and the smallest on the right (`sort` in 220-222
    // and 238-240), and the nodes at exactly that x (`===`).
    let first_x = if section == Section::Left {
        side.iter()
            .map(|node| node.x)
            .fold(f64::NEG_INFINITY, js_max)
    } else {
        side.iter().map(|node| node.x).fold(f64::INFINITY, js_min)
    };
    let level: Vec<&SideNode> = side.iter().filter(|node| node.x == first_x).collect();
    if level.is_empty() {
        return 0.0;
    }
    let top_min = level
        .iter()
        .map(|node| node.y - nodes[node.index].height / 2.0)
        .fold(f64::INFINITY, js_min);
    let bottom_max = level
        .iter()
        .map(|node| node.y + nodes[node.index].height / 2.0)
        .fold(f64::NEG_INFINITY, js_max);
    (top_min + bottom_max) / 2.0
}
