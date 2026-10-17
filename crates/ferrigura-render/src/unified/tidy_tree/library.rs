//! The library `non-layered-tidy-tree-layout` 2.0.2 (`src/algorithm.js` and `src/helpers.js`): the
//! non-layered tidy tree of A. J. van der Ploeg, "Drawing Non-layered Tidy Trees in Linear Time",
//! transcribed from the library's contour walk with its threads and its list of left siblings (IYL).
//!
//! The library's objects are arena indices here, and the sequence of every operation is the
//! library's, so the positions are the library's bit for bit: the operands keep their order in each
//! sum and difference (`positionRoot`, `addChildSpacing`, `seperate`). The library is MIT licensed
//! (Copyright (c) 2019 Michael Wong); its source is vendored in
//! `tools/port/vendor/non-layered-tidy-tree-layout/`.

use super::TidyTreeError;

/// A node of the input tree, as the library's `treeData`: the size of its box, and its children.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeData {
    /// The width of the node's box, before the bounding box is added.
    pub width: f64,
    /// The height of the node's box, before the bounding box is added.
    pub height: f64,
    /// The children, in order.
    pub children: Vec<TreeData>,
}

/// Where the library puts a node: its breadth `x` and its depth `y`, with the gap removed
/// (`Layout.assignLayout`, `helpers.js` 106-125, and `removeBoundingBox`, 20-22).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// The start of the node in breadth (the library's `x` plus half the gap).
    pub x: f64,
    /// The depth of the node.
    pub y: f64,
}

/// The library's `Tree` (`algorithm.js` 1-22). The links between the nodes (the children, the
/// threads and the extreme nodes) are arena indices, where the library holds object references.
#[derive(Debug, Clone)]
struct Tree {
    /// The breadth of the box with the gap (`w`).
    w: f64,
    /// The depth of the box with the bottom padding (`h`).
    h: f64,
    /// The depth of the top of the box (`y`).
    y: f64,
    /// The children, in order (`c`; `cs` is its length).
    c: Vec<usize>,
    /// The breadth of the node after the second walk (`x`).
    x: f64,
    /// The preliminary breadth (`prelim`).
    prelim: f64,
    /// The modifier of the node's subtree (`mod`).
    modifier: f64,
    /// The shift of the children (`shift`).
    shift: f64,
    /// The change of the children (`change`).
    change: f64,
    /// The left thread (`tl`), null when the node has none.
    tl: Option<usize>,
    /// The right thread (`tr`), null when the node has none.
    tr: Option<usize>,
    /// The extreme left node of the subtree (`el`). Set by `set_extremes` after the children are
    /// laid out, before anything reads it; the node's own index is the value it starts with.
    el: usize,
    /// The extreme right node of the subtree (`er`), set the same way.
    er: usize,
    /// The sum of the modifiers at the extreme left node (`msel`).
    msel: f64,
    /// The sum of the modifiers at the extreme right node (`mser`).
    mser: f64,
}

/// An entry of the list of left siblings (`IYL`, `algorithm.js` 41-49): the index of a sibling, the
/// lowest bottom that is still visible of it, and the entry after it.
#[derive(Debug, Clone, Copy)]
struct Iyl {
    low_y: f64,
    index: usize,
    next: Option<usize>,
}

/// The arena of one layout: the trees in pre-order, and the entries of the list of left siblings.
struct Arena {
    trees: Vec<Tree>,
    iyl: Vec<Iyl>,
}

/// Lays out a tree as `Layout.layout` does with `BoundingBox(gap, bottom_padding)` (`helpers.js`
/// 36-42): the tree is converted (`convert`, 47-62), walked (`layout`, `algorithm.js` 203-206), and
/// every node gets its placement. The placements are in pre-order, the root first.
///
/// # Errors
///
/// Returns an error where the library throws: when the walk along the list of left siblings runs
/// past its end (`seperate`, `algorithm.js` 117-125).
pub fn layout(
    root: &TreeData,
    gap: f64,
    bottom_padding: f64,
) -> Result<Vec<Placement>, TidyTreeError> {
    let mut arena = Arena {
        trees: Vec::new(),
        iyl: Vec::new(),
    };
    let root_index = arena.convert(root, gap, bottom_padding, 0.0);
    arena.first_walk(root_index)?;
    arena.second_walk(root_index, 0.0);
    Ok(arena
        .trees
        .iter()
        .map(|tree| Placement {
            x: tree.x + gap / 2.0,
            y: tree.y,
        })
        .collect())
}

impl Arena {
    /// `updateIYL` (`algorithm.js` 51-58): removes the siblings that the new subtree hides, and
    /// prepends the new one. Returns the index of the new entry.
    fn update_iyl(&mut self, min_y: f64, index: usize, ih: Option<usize>) -> usize {
        let mut head = ih;
        while let Some(current) = head {
            if min_y >= self.iyl[current].low_y {
                head = self.iyl[current].next;
            } else {
                break;
            }
        }
        self.iyl.push(Iyl {
            low_y: min_y,
            index,
            next: head,
        });
        self.iyl.len() - 1
    }

    /// `Layout.convert` (`helpers.js` 47-62): one tree per node, with the bounding box added, and the
    /// depth of each node, which is its parent's depth plus the parent's height. A node takes its
    /// index before its children, so the indices are in pre-order.
    fn convert(&mut self, data: &TreeData, gap: f64, padding: f64, y: f64) -> usize {
        let index = self.trees.len();
        let w = data.width + gap;
        let h = data.height + padding;
        self.trees.push(Tree {
            w,
            h,
            y,
            c: Vec::new(),
            x: 0.0,
            prelim: 0.0,
            modifier: 0.0,
            shift: 0.0,
            change: 0.0,
            tl: None,
            tr: None,
            el: index,
            er: index,
            msel: 0.0,
            mser: 0.0,
        });
        let children: Vec<usize> = data
            .children
            .iter()
            .map(|child| self.convert(child, gap, padding, y + h))
            .collect();
        self.trees[index].c = children;
        index
    }

    /// `bottom` (`algorithm.js` 37-39): the depth of the bottom of a tree's box.
    fn bottom(&self, tree: usize) -> f64 {
        self.trees[tree].y + self.trees[tree].h
    }

    /// `nextLeftContour` (`algorithm.js` 78-80).
    fn next_left_contour(&self, tree: usize) -> Option<usize> {
        let node = &self.trees[tree];
        if node.c.is_empty() {
            node.tl
        } else {
            Some(node.c[0])
        }
    }

    /// `nextRightContour` (`algorithm.js` 82-84).
    fn next_right_contour(&self, tree: usize) -> Option<usize> {
        let node = &self.trees[tree];
        if node.c.is_empty() {
            node.tr
        } else {
            Some(node.c[node.c.len() - 1])
        }
    }

    /// `setExtremes` (`algorithm.js` 24-35).
    fn set_extremes(&mut self, tree: usize) {
        if self.trees[tree].c.is_empty() {
            let node = &mut self.trees[tree];
            node.el = tree;
            node.er = tree;
            node.msel = 0.0;
            node.mser = 0.0;
        } else {
            let first = self.trees[tree].c[0];
            let last = self.trees[tree].c[self.trees[tree].c.len() - 1];
            let (el, msel) = (self.trees[first].el, self.trees[first].msel);
            let (er, mser) = (self.trees[last].er, self.trees[last].mser);
            let node = &mut self.trees[tree];
            node.el = el;
            node.msel = msel;
            node.er = er;
            node.mser = mser;
        }
    }

    /// `setLeftThread` (`algorithm.js` 86-97): the left thread of the new subtree's left contour
    /// goes to the extreme left node of the left siblings, and the extreme node moves to the new
    /// subtree.
    fn set_left_thread(&mut self, tree: usize, i: usize, cl: usize, modsumcl: f64) {
        let first = self.trees[tree].c[0];
        let li = self.trees[first].el;
        self.trees[li].tl = Some(cl);
        // Change the modifier so that the sum of modifiers after following the thread is correct.
        let diff = (modsumcl - self.trees[cl].modifier) - self.trees[first].msel;
        self.trees[li].modifier += diff;
        // Change the preliminary x so that the node does not move.
        self.trees[li].prelim -= diff;
        // Update the extreme node and its sum of modifiers.
        let subtree = self.trees[tree].c[i];
        self.trees[first].el = self.trees[subtree].el;
        self.trees[first].msel = self.trees[subtree].msel;
    }

    /// `setRightThread` (`algorithm.js` 99-108), symmetric to `set_left_thread`.
    fn set_right_thread(&mut self, tree: usize, i: usize, sr: usize, modsumsr: f64) {
        let subtree = self.trees[tree].c[i];
        let ri = self.trees[subtree].er;
        self.trees[ri].tr = Some(sr);
        let diff = (modsumsr - self.trees[sr].modifier) - self.trees[subtree].mser;
        self.trees[ri].modifier += diff;
        self.trees[ri].prelim -= diff;
        let previous = self.trees[tree].c[i - 1];
        self.trees[subtree].er = self.trees[previous].er;
        self.trees[subtree].mser = self.trees[previous].mser;
    }

    /// `distributeExtra` (`algorithm.js` 60-68): the distance a subtree moves is shared between the
    /// siblings in between.
    fn distribute_extra(&mut self, tree: usize, i: usize, si: usize, distance: f64) {
        // Are there intermediate children? `si` is always a sibling before `i`.
        if si + 1 != i {
            let nr = i as f64 - si as f64;
            let next = self.trees[tree].c[si + 1];
            self.trees[next].shift += distance / nr;
            let subtree = self.trees[tree].c[i];
            self.trees[subtree].shift -= distance / nr;
            self.trees[subtree].change -= distance - distance / nr;
        }
    }

    /// `moveSubtree` (`algorithm.js` 70-76): moves the subtree `i` to the right by `distance`, by
    /// changing its modifiers.
    fn move_subtree(&mut self, tree: usize, i: usize, si: usize, distance: f64) {
        let subtree = self.trees[tree].c[i];
        self.trees[subtree].modifier += distance;
        self.trees[subtree].msel += distance;
        self.trees[subtree].mser += distance;
        self.distribute_extra(tree, i, si, distance);
    }

    /// `seperate` (`algorithm.js` 110-151): moves the subtree `i` right until its left contour clears
    /// the right contour of the subtrees on its left, and sets the threads where one contour ends
    /// before the other. `ih_start` is the list of left siblings at that point.
    ///
    /// # Errors
    ///
    /// Returns an error where the library throws: when the walk along the list runs past its end.
    fn seperate(&mut self, tree: usize, i: usize, ih_start: usize) -> Result<(), TidyTreeError> {
        let mut ih = Some(ih_start);
        // The right contour node of the left siblings and its sum of modifiers.
        let mut sr = Some(self.trees[tree].c[i - 1]);
        let mut mssr = self.trees[self.trees[tree].c[i - 1]].modifier;
        // The left contour node of the right siblings and its sum of modifiers.
        let mut cl = Some(self.trees[tree].c[i]);
        let mut mscl = self.trees[self.trees[tree].c[i]].modifier;

        while let (Some(sr_node), Some(cl_node)) = (sr, cl) {
            let current = ih.ok_or(TidyTreeError::SiblingListExhausted)?;
            if self.bottom(sr_node) > self.iyl[current].low_y {
                ih = self.iyl[current].next;
            }
            // How far to the left of the right side of sr is the left side of cl.
            let distance = mssr + self.trees[sr_node].prelim + self.trees[sr_node].w
                - (mscl + self.trees[cl_node].prelim);
            if distance > 0.0 {
                mscl += distance;
                let current = ih.ok_or(TidyTreeError::SiblingListExhausted)?;
                let si = self.iyl[current].index;
                self.move_subtree(tree, i, si, distance);
            }

            let sy = self.bottom(sr_node);
            let cy = self.bottom(cl_node);
            if sy <= cy {
                sr = self.next_right_contour(sr_node);
                if let Some(next) = sr {
                    mssr += self.trees[next].modifier;
                }
            }
            if sy >= cy {
                cl = self.next_left_contour(cl_node);
                if let Some(next) = cl {
                    mscl += self.trees[next].modifier;
                }
            }
        }

        // The current subtree is taller than the left siblings, or the reverse.
        match (sr, cl) {
            (None, Some(cl_node)) => self.set_left_thread(tree, i, cl_node, mscl),
            (Some(sr_node), None) => self.set_right_thread(tree, i, sr_node, mssr),
            _ => {}
        }
        Ok(())
    }

    /// `positionRoot` (`algorithm.js` 153-163): the node is centred over its first and last child,
    /// with their modifiers.
    fn position_root(&mut self, tree: usize) {
        let first = self.trees[tree].c[0];
        let last = self.trees[tree].c[self.trees[tree].c.len() - 1];
        let (first_prelim, first_mod) = (self.trees[first].prelim, self.trees[first].modifier);
        let (last_mod, last_prelim, last_w) = (
            self.trees[last].modifier,
            self.trees[last].prelim,
            self.trees[last].w,
        );
        self.trees[tree].prelim = (first_prelim + first_mod + last_mod + last_prelim + last_w)
            / 2.0
            - self.trees[tree].w / 2.0;
    }

    /// `firstWalk` (`algorithm.js` 165-181): the bottom-up walk that places the children of every
    /// node with `seperate`.
    fn first_walk(&mut self, tree: usize) -> Result<(), TidyTreeError> {
        if self.trees[tree].c.is_empty() {
            self.set_extremes(tree);
            return Ok(());
        }

        let first = self.trees[tree].c[0];
        self.first_walk(first)?;
        let first_low = self.bottom(self.trees[first].el);
        let mut ih = self.update_iyl(first_low, 0, None);
        for i in 1..self.trees[tree].c.len() {
            let child = self.trees[tree].c[i];
            self.first_walk(child)?;
            let min_y = self.bottom(self.trees[child].er);
            self.seperate(tree, i, ih)?;
            ih = self.update_iyl(min_y, i, Some(ih));
        }
        self.position_root(tree);
        self.set_extremes(tree);
        Ok(())
    }

    /// `addChildSpacing` (`algorithm.js` 183-191): adds the shifts of the children to their modifiers.
    fn add_child_spacing(&mut self, tree: usize) {
        let mut d = 0.0;
        let mut modsumdelta = 0.0;
        for i in 0..self.trees[tree].c.len() {
            let child = self.trees[tree].c[i];
            d += self.trees[child].shift;
            modsumdelta += d + self.trees[child].change;
            self.trees[child].modifier += modsumdelta;
        }
    }

    /// `secondWalk` (`algorithm.js` 193-201): the top-down walk that gives every node its breadth.
    fn second_walk(&mut self, tree: usize, modsum: f64) {
        let modsum = modsum + self.trees[tree].modifier;
        // Absolute (not relative) horizontal coordinates.
        self.trees[tree].x = self.trees[tree].prelim + modsum;
        self.add_child_spacing(tree);
        for i in 0..self.trees[tree].c.len() {
            let child = self.trees[tree].c[i];
            self.second_walk(child, modsum);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(width: f64, height: f64) -> TreeData {
        TreeData {
            width,
            height,
            children: Vec::new(),
        }
    }

    fn node(width: f64, height: f64, children: Vec<TreeData>) -> TreeData {
        TreeData {
            width,
            height,
            children,
        }
    }

    fn placed(placements: &[Placement]) -> Vec<(f64, f64)> {
        placements.iter().map(|p| (p.x, p.y)).collect()
    }

    // The known answers of the library's own tests (`test/tree-layout.test.js`, with no bounding
    // box: the gap and the padding are zero, so a placement is the library's `x` and `y`).

    #[test]
    fn one_node() {
        let tree = leaf(10.0, 5.0);
        let placements = layout(&tree, 0.0, 0.0).expect("layout");
        assert_eq!(placed(&placements), vec![(0.0, 0.0)]);
    }

    #[test]
    fn two_nodes() {
        let tree = node(10.0, 4.0, vec![leaf(20.0, 10.0)]);
        let placements = layout(&tree, 0.0, 0.0).expect("layout");
        assert_eq!(placed(&placements), vec![(5.0, 0.0), (0.0, 4.0)]);
    }

    #[test]
    fn three_nodes() {
        let tree = node(40.0, 10.0, vec![leaf(10.0, 30.0), leaf(20.0, 10.0)]);
        let placements = layout(&tree, 0.0, 0.0).expect("layout");
        assert_eq!(
            placed(&placements),
            vec![(-5.0, 0.0), (0.0, 10.0), (10.0, 10.0)]
        );
    }

    #[test]
    fn reflection_of_the_tree() {
        // n1 with children n2 (which has n7), n3, n4, n5 and n6; pre-order is n1 n2 n7 n3 n4 n5 n6.
        let tree = node(
            10.0,
            10.0,
            vec![
                node(10.0, 10.0, vec![leaf(150.0, 10.0)]),
                leaf(10.0, 10.0),
                leaf(10.0, 10.0),
                leaf(10.0, 10.0),
                leaf(10.0, 20.0),
            ],
        );
        let placements = layout(&tree, 0.0, 0.0).expect("layout");
        assert_eq!(
            placed(&placements),
            vec![
                (110.0, 0.0),
                (70.0, 10.0),
                (0.0, 20.0),
                (90.0, 10.0),
                (110.0, 10.0),
                (130.0, 10.0),
                (150.0, 10.0),
            ]
        );
    }

    // `test/helpers.test.js`: the `Layout` class with `BoundingBox(10, 10)`. The library's
    // `data.x` is the placement's x, and its children follow in pre-order.
    #[test]
    fn layout_class_with_a_bounding_box() {
        let tree = node(
            10.0,
            10.0,
            vec![
                node(10.0, 10.0, vec![leaf(150.0, 10.0)]),
                leaf(10.0, 10.0),
                leaf(10.0, 10.0),
                leaf(10.0, 10.0),
                leaf(10.0, 20.0),
            ],
        );
        let placements = layout(&tree, 10.0, 10.0).expect("layout");
        assert_eq!(
            placed(&placements),
            vec![
                (120.0, 0.0),
                (75.0, 20.0),
                (5.0, 40.0),
                (97.5, 20.0),
                (120.0, 20.0),
                (142.5, 20.0),
                (165.0, 20.0),
            ]
        );
    }

    // `test/helpers.test.js`: a big root over a small child, with no bounding box.
    #[test]
    fn big_root_small_child() {
        let tree = node(100.0, 50.0, vec![leaf(50.0, 50.0)]);
        let placements = layout(&tree, 0.0, 0.0).expect("layout");
        assert_eq!(placed(&placements), vec![(-25.0, 0.0), (0.0, 50.0)]);
    }
}
