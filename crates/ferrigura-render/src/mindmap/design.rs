//! The layout of a mindmap in the default rendering (`docs/diagrams/mindmap.md#default-look`,
//! `docs/decisions/0023-the-default-rendering.md`): a tree around its center, drawn as a person
//! draws one.
//!
//! The root stands in the middle. Its branches go to both sides, the first ones on the right from top
//! to bottom and the rest on the left from bottom to top, so that the text's order runs clockwise
//! around the root; the split is the one that gives both sides the most even height. Each side is a
//! tree that grows outwards: a node's children stand in one column beyond it, in the order of the
//! text, from top to bottom, their inner sides on one line, and the node stands in the middle between
//! its first and its last child. Every subtree takes the room of its box, and two siblings stand a
//! constant gap apart, wider between the root's branches than inside a branch, so that each branch
//! reads as one block. A line leaves the middle of its parent's outer side and enters the middle of
//! its child's inner side, level at both ends, so that the lines of a node fan out of one point.
//!
//! The layout reads only the sizes of the nodes and the edges: it is deterministic (decision 0018),
//! draws no random numbers and runs no engine. `compat` never runs it; it keeps
//! cose-bilkent (`ferrigura-cose`).

use std::collections::HashMap;

use crate::unified::data::LayoutData;
use crate::unified::geom::Point;

/// The gap between the root's outer side and its branches' inner sides.
pub const ROOT_GAP: f64 = 64.0;

/// The steepest a line from the root to a branch runs, as its rise over the root's gap: a side with
/// many branches stands further out, so that their lines fan out instead of running together.
pub const ROOT_SLOPE: f64 = 3.0;

/// The gap between a node's outer side and its children's inner sides, below the root.
pub const LEVEL_GAP: f64 = 40.0;

/// The gap between the boxes of two branches of the root on one side.
pub const BRANCH_GAP: f64 = 24.0;

/// The gap between the boxes of two siblings inside a branch.
pub const SIBLING_GAP: f64 = 10.0;

/// How far across its gap a line runs level out of each end, as a share of the gap. Less than half,
/// so that the middle part of the line is not vertical and `fixCorners` finds no corner in it.
pub const LINE_REACH: f64 = 0.4;

/// How much more padding the root has than its shape gives it.
pub const ROOT_PADDING: f64 = 5.0;

/// How much less padding a node below a branch has than its shape gives it.
pub const LEAF_PADDING: f64 = 6.0;

/// The least padding of a node below a branch.
pub const MIN_PADDING: f64 = 6.0;

/// Gives the nodes the padding of their depth, before they are measured: the root more than its
/// shape gives it, the root's branches what their shapes give them, and the nodes below a branch
/// less, so that a node's size says how far from the center it stands.
pub fn pad_by_depth(data: &mut LayoutData) {
    let index = indices(data);
    let mut depths = vec![0_usize; data.nodes.len()];
    // The edges are in pre-order, so a parent's depth is known before its children's.
    for edge in &data.edges {
        if let (Some(&parent), Some(&child)) =
            (index.get(edge.start.as_str()), index.get(edge.end.as_str()))
        {
            depths[child] = depths[parent] + 1;
        }
    }
    for (node, depth) in data.nodes.iter_mut().zip(depths) {
        node.padding = match depth {
            0 => node.padding + ROOT_PADDING,
            1 => node.padding,
            _ => (node.padding - LEAF_PADDING).max(MIN_PADDING),
        };
    }
}

/// A subtree laid out on its own, from the top of its box: its height, the offset of its node's
/// center from its top, and the tops of its children relative to its top.
#[derive(Debug, Clone, Default)]
struct Block {
    height: f64,
    anchor: f64,
    tops: Vec<f64>,
}

/// The tree of a mindmap's layout data: the root and each node's children in the order of the text,
/// as indices into the nodes, and the nodes' measured sizes.
struct Tree {
    root: usize,
    children: Vec<Vec<usize>>,
    sizes: Vec<(f64, f64)>,
}

impl Tree {
    /// The tree of the data, or `None` when its edges do not form one tree.
    fn of(data: &LayoutData) -> Option<Self> {
        let index = indices(data);
        let mut children = vec![Vec::new(); data.nodes.len()];
        let mut has_parent = vec![false; data.nodes.len()];
        // The DB generates the edges in pre-order, so each node's children come in the order of the
        // text.
        for edge in &data.edges {
            let (Some(&parent), Some(&child)) =
                (index.get(edge.start.as_str()), index.get(edge.end.as_str()))
            else {
                return None;
            };
            if has_parent[child] || parent == child {
                return None;
            }
            has_parent[child] = true;
            children[parent].push(child);
        }
        let mut roots = has_parent.iter().enumerate().filter(|(_, has)| !**has);
        let (root, _) = roots.next()?;
        if roots.next().is_some() {
            return None;
        }
        let sizes = data
            .nodes
            .iter()
            .map(|node| (node.width.unwrap_or(0.0), node.height.unwrap_or(0.0)))
            .collect();
        Some(Self {
            root,
            children,
            sizes,
        })
    }

    /// The blocks of a node's subtree below the root, children stacked [`SIBLING_GAP`] apart.
    fn block(&self, node: usize, blocks: &mut [Block]) {
        for &child in &self.children[node] {
            self.block(child, blocks);
        }
        blocks[node] = stack(
            &self.children[node],
            self.sizes[node].1,
            SIBLING_GAP,
            blocks,
        );
    }
}

fn indices(data: &LayoutData) -> HashMap<&str, usize> {
    data.nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect()
}

/// Stacks the blocks of `children` from top to bottom with `gap` between them, and places a node of
/// `height` in the middle between the centers of the first and the last child. The block covers both
/// the children and the node.
fn stack(children: &[usize], height: f64, gap: f64, blocks: &[Block]) -> Block {
    let (Some(&first), Some(&last)) = (children.first(), children.last()) else {
        return Block {
            height,
            anchor: height / 2.0,
            tops: Vec::new(),
        };
    };
    let mut tops = Vec::with_capacity(children.len());
    let mut y = 0.0;
    for (position, &child) in children.iter().enumerate() {
        if position > 0 {
            y += gap;
        }
        tops.push(y);
        y += blocks[child].height;
    }
    let first_center = tops[0] + blocks[first].anchor;
    let last_center = tops[tops.len() - 1] + blocks[last].anchor;
    let center = (first_center + last_center) / 2.0;
    let top = (center - height / 2.0).min(0.0);
    let bottom = (center + height / 2.0).max(y);
    for t in &mut tops {
        *t -= top;
    }
    Block {
        height: bottom - top,
        anchor: center - top,
        tops,
    }
}

/// The height of one side of the root that holds `branches`.
fn side_height(branches: &[usize], blocks: &[Block]) -> f64 {
    let gaps = branches.len().saturating_sub(1) as f64 * BRANCH_GAP;
    branches.iter().map(|&b| blocks[b].height).sum::<f64>() + gaps
}

/// How many of the root's branches, in the order of the text, go to the right: the split that makes
/// the taller side the least tall, and of those the one that puts more on the right.
fn split(branches: &[usize], blocks: &[Block]) -> usize {
    let mut best = (f64::INFINITY, branches.len());
    for count in (0..=branches.len()).rev() {
        let (right, left) = branches.split_at(count);
        let tallest = side_height(right, blocks).max(side_height(left, blocks));
        if tallest < best.0 - 1e-9 {
            best = (tallest, count);
        }
    }
    best.1
}

/// Lays the mindmap out around its root: every node takes its center, and every edge the four points
/// of a line from its parent's outer side to its child's inner side, level at both ends. Returns
/// `false`, leaving the data as it was, when the edges do not form one tree.
pub fn lay_out(data: &mut LayoutData) -> bool {
    let Some(tree) = Tree::of(data) else {
        return false;
    };
    let mut blocks = vec![Block::default(); data.nodes.len()];
    let branches = &tree.children[tree.root];
    for &branch in branches {
        tree.block(branch, &mut blocks);
    }
    let count = split(branches, &blocks);
    let right: Vec<usize> = branches[..count].to_vec();
    // The left side reads from bottom to top, so that the text's order runs clockwise.
    let left: Vec<usize> = branches[count..].iter().rev().copied().collect();

    let mut placed = Placed {
        centers: vec![None; data.nodes.len()],
        sides: vec![1.0; data.nodes.len()],
    };
    placed.centers[tree.root] = Some(Point::new(0.0, 0.0));
    let (root_width, root_height) = tree.sizes[tree.root];
    // Each side's branches, from top to bottom, with the top of each one's block relative to the
    // root's center: the root stands in the middle between the first and the last branch of a side.
    let sides: Vec<(f64, Vec<(usize, f64)>)> = [(1.0, &right), (-1.0, &left)]
        .into_iter()
        .map(|(side, list)| {
            let block = stack(list, root_height, BRANCH_GAP, &blocks);
            let tops = list
                .iter()
                .zip(&block.tops)
                .map(|(&branch, &top)| (branch, top - block.anchor))
                .collect();
            (side, tops)
        })
        .collect();
    // The branches stand far enough out that no line from the root runs steeper than
    // [`ROOT_SLOPE`], and both sides stand at the same distance.
    let rise = sides
        .iter()
        .flat_map(|(_, tops)| tops)
        .map(|&(branch, top)| (top + blocks[branch].anchor).abs())
        .fold(0.0, f64::max);
    let gap = ROOT_GAP.max(rise / ROOT_SLOPE);
    for (side, tops) in &sides {
        let near = side * (root_width / 2.0 + gap);
        for &(branch, top) in tops {
            placed.place(&tree, &blocks, branch, top, near, *side);
        }
    }

    for (node, center) in data.nodes.iter_mut().zip(&placed.centers) {
        if let Some(center) = center {
            node.x = Some(center.x);
            node.y = Some(center.y);
        }
    }
    let index = indices(data);
    let mut lines = Vec::with_capacity(data.edges.len());
    for edge in &data.edges {
        let parent = index[edge.start.as_str()];
        let child = index[edge.end.as_str()];
        let (Some(from), Some(to)) = (placed.centers[parent], placed.centers[child]) else {
            lines.push(Vec::new());
            continue;
        };
        let side = placed.sides[child];
        let end = Point::new(to.x - side * tree.sizes[child].0 / 2.0, to.y);
        if parent == tree.root {
            lines.push(branch_line(from, tree.sizes[parent], end));
        } else {
            let start = Point::new(from.x + side * tree.sizes[parent].0 / 2.0, from.y);
            lines.push(line(start, end));
        }
    }
    for (edge, points) in data.edges.iter_mut().zip(lines) {
        edge.points = points;
    }
    true
}

/// The centers of the placed nodes, and the side of the root each one grows to.
struct Placed {
    centers: Vec<Option<Point>>,
    sides: Vec<f64>,
}

impl Placed {
    /// Places the subtree of `node` with its block's top at `top` and its node's inner side at
    /// `near`, growing towards `side` (1 to the right, -1 to the left).
    fn place(
        &mut self,
        tree: &Tree,
        blocks: &[Block],
        node: usize,
        top: f64,
        near: f64,
        side: f64,
    ) {
        let width = tree.sizes[node].0;
        let block = &blocks[node];
        self.centers[node] = Some(Point::new(near + side * width / 2.0, top + block.anchor));
        self.sides[node] = side;
        let far = near + side * (width + LEVEL_GAP);
        for (&child, &child_top) in tree.children[node].iter().zip(&block.tops) {
            self.place(tree, blocks, child, top + child_top, far, side);
        }
    }
}

/// The points of a line from `start` to `end` that the `basis` curve draws level at both ends: the
/// two inner points stand [`LINE_REACH`] of the way across from each end, at the heights of the
/// ends. Where both ends are at one height the line is straight.
#[must_use]
pub fn line(start: Point, end: Point) -> Vec<Point> {
    let reach = (end.x - start.x) * LINE_REACH;
    vec![
        start,
        Point::new(start.x + reach, start.y),
        Point::new(end.x - reach, end.y),
        end,
    ]
}

/// The points of a branch's line from the root to `end`: it starts in the root's center and leaves
/// the root's box, `size`, where the straight line from the center to `end` does, so that the
/// branches of a side leave the root at different places in the order they stand in, and it runs
/// level into the branch's inner side.
#[must_use]
pub fn branch_line(center: Point, size: (f64, f64), end: Point) -> Vec<Point> {
    let (dx, dy) = (end.x - center.x, end.y - center.y);
    // The share of the way to `end` at which the line leaves the box.
    let exit = [(dx, size.0), (dy, size.1)]
        .into_iter()
        .filter(|(d, _)| *d != 0.0)
        .map(|(d, length)| length / 2.0 / d.abs())
        .fold(1.0, f64::min);
    let reach = dx * LINE_REACH;
    vec![
        center,
        Point::new(center.x + dx * exit, center.y + dy * exit),
        Point::new(end.x - reach, end.y),
        end,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(height: f64) -> Block {
        Block {
            height,
            anchor: height / 2.0,
            tops: Vec::new(),
        }
    }

    #[test]
    fn the_split_evens_the_sides_and_favors_the_right() {
        // Two equal branches: one on each side.
        let blocks = vec![leaf(40.0), leaf(40.0)];
        assert_eq!(split(&[0, 1], &blocks), 1);
        // Three equal branches: two on the right.
        let blocks = vec![leaf(40.0), leaf(40.0), leaf(40.0)];
        assert_eq!(split(&[0, 1, 2], &blocks), 2);
        // A tall first branch stands alone on its side.
        let blocks = vec![leaf(200.0), leaf(40.0), leaf(40.0), leaf(40.0)];
        assert_eq!(split(&[0, 1, 2, 3], &blocks), 1);
        // One branch goes right.
        assert_eq!(split(&[0], &[leaf(40.0)]), 1);
    }

    #[test]
    fn a_node_taller_than_its_children_keeps_its_block() {
        let blocks = vec![leaf(10.0), leaf(10.0)];
        let block = stack(&[0, 1], 60.0, SIBLING_GAP, &blocks);
        assert_eq!(block.height, 60.0);
        assert_eq!(block.anchor, 30.0);
        assert_eq!(block.tops, vec![15.0, 35.0]);
    }

    #[test]
    fn a_line_has_no_corner_for_fix_corners() {
        let points = line(Point::new(0.0, 0.0), Point::new(40.0, 30.0));
        assert_eq!(points.len(), 4);
        // The middle part is neither level nor upright.
        assert!(points[1].x < points[2].x && points[1].y < points[2].y);
        assert_eq!(crate::unified::path::fix_corners(&points), points);
    }
}
