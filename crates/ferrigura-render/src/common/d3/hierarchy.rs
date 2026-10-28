//! The hierarchy of `d3-hierarchy` 3.1.2 (`src/hierarchy`): the nodes of a tree with their depth,
//! value and children, `sum` (`sum.js`), `sort` (`sort.js`), and the traversals `eachBefore`
//! (`eachBefore.js`), `eachAfter`, `descendants` (the level order of `iterator.js`) and `leaves`
//! (`leaves.js`).
//!
//! The nodes live in one list and refer to each other by index. A node with no children is a leaf:
//! d3 leaves `children` undefined when the datum has none, and so does an empty list.

use std::cmp::Ordering;
use std::collections::VecDeque;

/// A node of a hierarchy: its datum, its place in the tree, and the values the layouts write.
#[derive(Debug, Clone)]
pub struct Node<'a, T> {
    /// The datum (`node.data`).
    pub data: &'a T,
    /// The index of the parent; `None` for the root.
    pub parent: Option<usize>,
    /// The indices of the children, in order; `None` for a leaf.
    pub children: Option<Vec<usize>>,
    /// The depth: 0 for the root.
    pub depth: usize,
    /// The value, set by [`Hierarchy::sum`].
    pub value: f64,
    /// The left edge of the box the layout gives the node.
    pub x0: f64,
    /// The top edge of the box.
    pub y0: f64,
    /// The right edge of the box.
    pub x1: f64,
    /// The bottom edge of the box.
    pub y1: f64,
}

/// A hierarchy: the nodes built from a root, with the root at index 0.
#[derive(Debug, Clone)]
pub struct Hierarchy<'a, T> {
    nodes: Vec<Node<'a, T>>,
}

impl<'a, T> Hierarchy<'a, T> {
    /// `hierarchy(data, children)` (`hierarchy/index.js` 15-43): the nodes of the tree below `root`,
    /// where `children` gives the children of a datum. A `None` or an empty list makes a leaf.
    #[must_use]
    pub fn new(root: &'a T, children: fn(&'a T) -> Option<&'a [T]>) -> Self {
        let mut nodes = vec![Node {
            data: root,
            parent: None,
            children: None,
            depth: 0,
            value: 0.0,
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
        }];
        let mut stack = vec![0_usize];
        while let Some(index) = stack.pop() {
            let data = nodes[index].data;
            let Some(kids) = children(data).filter(|kids| !kids.is_empty()) else {
                continue;
            };
            let depth = nodes[index].depth + 1;
            let mut ids = Vec::with_capacity(kids.len());
            for kid in kids {
                ids.push(nodes.len());
                nodes.push(Node {
                    data: kid,
                    parent: Some(index),
                    children: None,
                    depth,
                    value: 0.0,
                    x0: 0.0,
                    y0: 0.0,
                    x1: 0.0,
                    y1: 0.0,
                });
            }
            // The children are visited in order, so they are pushed in reverse.
            stack.extend(ids.iter().rev().copied());
            nodes[index].children = Some(ids);
        }
        Self { nodes }
    }

    /// The nodes, in the order they were built (parents before their children).
    #[must_use]
    pub fn nodes(&self) -> &[Node<'a, T>] {
        &self.nodes
    }

    /// The nodes, mutable, for a layout that writes their boxes.
    pub fn nodes_mut(&mut self) -> &mut [Node<'a, T>] {
        &mut self.nodes
    }

    /// `hierarchy.sum(value)` (`sum.js` 1-9): each node's value is its own value, `+value(data) || 0`,
    /// plus its children's values, added from the last child to the first, as d3 adds them, so that
    /// the rounding of the sum is d3's.
    pub fn sum(&mut self, value: impl Fn(&T) -> f64) {
        for index in (0..self.nodes.len()).rev() {
            let own = value(self.nodes[index].data);
            let mut sum = if own.is_nan() { 0.0 } else { own };
            if let Some(children) = &self.nodes[index].children {
                for &child in children.iter().rev() {
                    sum += self.nodes[child].value;
                }
            }
            self.nodes[index].value = sum;
        }
    }

    /// `hierarchy.sort(compare)` (`sort.js`): the children of each node are sorted with the
    /// comparator, which returns a difference as JavaScript's does. A comparison that gives NaN is
    /// zero, and the sort is stable.
    pub fn sort(&mut self, compare: impl Fn(&Node<'a, T>, &Node<'a, T>) -> f64) {
        for index in 0..self.nodes.len() {
            let Some(children) = self.nodes[index].children.clone() else {
                continue;
            };
            let mut sorted = children;
            sorted.sort_by(|&a, &b| {
                let difference = compare(&self.nodes[a], &self.nodes[b]);
                if difference < 0.0 {
                    Ordering::Less
                } else if difference > 0.0 {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            });
            self.nodes[index].children = Some(sorted);
        }
    }

    /// `hierarchy.eachBefore()` (`eachBefore.js`): the indices in pre-order, each node before its
    /// descendants and the children in order.
    #[must_use]
    pub fn each_before(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.nodes.len());
        let mut stack = vec![0_usize];
        while let Some(index) = stack.pop() {
            order.push(index);
            if let Some(children) = &self.nodes[index].children {
                stack.extend(children.iter().rev().copied());
            }
        }
        order
    }

    /// `hierarchy.descendants()` (`iterator.js`): every node in level order, the children of each
    /// level in order.
    #[must_use]
    pub fn descendants(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.nodes.len());
        let mut queue = VecDeque::from([0_usize]);
        while let Some(index) = queue.pop_front() {
            order.push(index);
            if let Some(children) = &self.nodes[index].children {
                queue.extend(children.iter().copied());
            }
        }
        order
    }

    /// `hierarchy.leaves()` (`leaves.js`): the nodes without children, in pre-order.
    #[must_use]
    pub fn leaves(&self) -> Vec<usize> {
        self.each_before()
            .into_iter()
            .filter(|&index| self.nodes[index].children.is_none())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test tree: a name, a value for a leaf, and children for a section.
    #[derive(Debug)]
    struct Item {
        value: Option<f64>,
        children: Option<Vec<Item>>,
    }

    fn kids(item: &Item) -> Option<&[Item]> {
        item.children.as_deref()
    }

    fn leaf(value: f64) -> Item {
        Item {
            value: Some(value),
            children: None,
        }
    }

    fn section(children: Vec<Item>) -> Item {
        Item {
            value: None,
            children: Some(children),
        }
    }

    #[test]
    fn a_section_with_no_children_is_a_leaf() {
        let root = section(vec![section(Vec::new()), leaf(2.0)]);
        let hierarchy = Hierarchy::new(&root, kids);
        assert_eq!(hierarchy.leaves().len(), 2);
        assert!(hierarchy.nodes()[1].children.is_none());
    }

    #[test]
    fn the_sum_adds_the_children_and_a_missing_or_nan_value_is_zero() {
        let root = section(vec![leaf(2.0), section(vec![leaf(3.0), leaf(f64::NAN)])]);
        let mut hierarchy = Hierarchy::new(&root, kids);
        hierarchy.sum(|item| item.value.unwrap_or(0.0));
        assert_eq!(hierarchy.nodes()[0].value, 5.0);
        assert_eq!(hierarchy.nodes()[2].value, 3.0);
    }

    #[test]
    fn the_sort_is_descending_and_stable() {
        let root = section(vec![leaf(1.0), leaf(5.0), leaf(5.0), leaf(3.0)]);
        let mut hierarchy = Hierarchy::new(&root, kids);
        hierarchy.sum(|item| item.value.unwrap_or(0.0));
        hierarchy.sort(|a, b| b.value - a.value);
        let values: Vec<f64> = hierarchy.nodes()[0]
            .children
            .as_ref()
            .expect("children")
            .iter()
            .map(|&index| hierarchy.nodes()[index].value)
            .collect();
        assert_eq!(values, [5.0, 5.0, 3.0, 1.0]);
    }

    #[test]
    fn the_traversals_are_pre_order_and_level_order() {
        let root = section(vec![section(vec![leaf(1.0)]), leaf(2.0)]);
        let hierarchy = Hierarchy::new(&root, kids);
        // The nodes are 0 (root), 1 (section), 2 (the second leaf) and 3 (the leaf in the section).
        assert_eq!(hierarchy.each_before(), [0, 1, 3, 2]);
        assert_eq!(hierarchy.descendants(), [0, 1, 2, 3]);
        assert_eq!(hierarchy.leaves(), [3, 2]);
    }
}
