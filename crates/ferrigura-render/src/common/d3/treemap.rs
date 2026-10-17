//! The treemap layout of `d3-hierarchy` 3.1.2 (`src/treemap`): `treemap()` with its paddings and
//! its rounding (`index.js`, `round.js`), and the default tile, `squarify` with the golden ratio
//! (`squarify.js`), which lays its rows with `dice` (`dice.js`) or `slice` (`slice.js`).
//!
//! The operations are d3's, in d3's order, because the rounding of a sum or a product decides the
//! last digit of a box, and the reference's boxes are the integers `Math.round` gives them. A value
//! of zero is falsy, as it is in JavaScript, and so is NaN; `Math.max` returns NaN when an argument
//! is NaN.

use super::hierarchy::{Hierarchy, Node};
use super::number::{js_max, js_round};

/// `phi = (1 + sqrt(5)) / 2`: the ratio squarify aims for (`squarify.js` 4), computed as the
/// reference computes it, so that the double is the same one.
#[must_use]
pub fn phi() -> f64 {
    (1.0 + 5.0_f64.sqrt()) / 2.0
}

/// The paddings of a node: its inner padding and the four sides of the padding around its children.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Padding {
    /// The inner padding, between siblings: half of it on each side (`paddingInner`).
    pub inner: f64,
    /// The padding above the children (`paddingTop`).
    pub top: f64,
    /// The padding to the right of the children (`paddingRight`).
    pub right: f64,
    /// The padding below the children (`paddingBottom`).
    pub bottom: f64,
    /// The padding to the left of the children (`paddingLeft`).
    pub left: f64,
}

/// A treemap layout over a box of `width` by `height` (`treemap().size([width, height])`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Treemap {
    /// The width of the root's box.
    pub width: f64,
    /// The height of the root's box.
    pub height: f64,
    /// Whether the boxes are rounded to integers (`treemap().round(true)`).
    pub round: bool,
}

impl Treemap {
    /// `treemap(root)` (`index.js` 9-28): the root takes the whole box, each node with children is
    /// positioned inside its own box (shrunk by its parent's half padding) and tiles its children,
    /// and the boxes are rounded when `round` is set.
    pub fn layout<'a, T>(
        &self,
        hierarchy: &mut Hierarchy<'a, T>,
        padding: impl Fn(&Node<'a, T>) -> Padding,
    ) {
        let order = hierarchy.each_before();
        let nodes = hierarchy.nodes_mut();
        nodes[0].x0 = 0.0;
        nodes[0].y0 = 0.0;
        nodes[0].x1 = self.width;
        nodes[0].y1 = self.height;
        // `paddingStack`: the half inner padding of the parent, by depth.
        let mut padding_stack = vec![0.0_f64];
        for &index in &order {
            position_node(nodes, index, &mut padding_stack, &padding);
        }
        if self.round {
            for &index in &order {
                let node = &mut nodes[index];
                node.x0 = js_round(node.x0);
                node.y0 = js_round(node.y0);
                node.x1 = js_round(node.x1);
                node.y1 = js_round(node.y1);
            }
        }
    }
}

/// `positionNode` (`index.js` 29-51): the node's box, shrunk by the half padding of its parent, and
/// for a node with children, the tile of its children inside the box less its own padding.
fn position_node<'a, T>(
    nodes: &mut [Node<'a, T>],
    index: usize,
    padding_stack: &mut Vec<f64>,
    padding: &impl Fn(&Node<'a, T>) -> Padding,
) {
    let depth = nodes[index].depth;
    let p = stack_value(padding_stack, depth);
    let mut x0 = nodes[index].x0 + p;
    let mut y0 = nodes[index].y0 + p;
    let mut x1 = nodes[index].x1 - p;
    let mut y1 = nodes[index].y1 - p;
    if x1 < x0 {
        x0 = (x0 + x1) / 2.0;
        x1 = x0;
    }
    if y1 < y0 {
        y0 = (y0 + y1) / 2.0;
        y1 = y0;
    }
    nodes[index].x0 = x0;
    nodes[index].y0 = y0;
    nodes[index].x1 = x1;
    nodes[index].y1 = y1;

    if nodes[index].children.is_some() {
        let own = padding(&nodes[index]);
        let half = own.inner / 2.0;
        set_stack_value(padding_stack, depth + 1, half);
        let mut x0 = x0 + (own.left - half);
        let mut y0 = y0 + (own.top - half);
        let mut x1 = x1 - (own.right - half);
        let mut y1 = y1 - (own.bottom - half);
        if x1 < x0 {
            x0 = (x0 + x1) / 2.0;
            x1 = x0;
        }
        if y1 < y0 {
            y0 = (y0 + y1) / 2.0;
            y1 = y0;
        }
        squarify(nodes, index, x0, y0, x1, y1);
    }
}

/// `paddingStack[depth]`. A depth the stack does not hold has no padding.
fn stack_value(stack: &[f64], depth: usize) -> f64 {
    stack.get(depth).copied().unwrap_or(0.0)
}

/// `paddingStack[depth] = value`, growing the stack as JavaScript's array does.
fn set_stack_value(stack: &mut Vec<f64>, depth: usize, value: f64) {
    if stack.len() <= depth {
        stack.resize(depth + 1, 0.0);
    }
    stack[depth] = value;
}

/// `squarify(parent, x0, y0, x1, y1)` with the golden ratio (`squarify.js` 6-53).
pub fn squarify<T>(nodes: &mut [Node<'_, T>], parent: usize, x0: f64, y0: f64, x1: f64, y1: f64) {
    squarify_ratio(phi(), nodes, parent, x0, y0, x1, y1);
}

/// `squarifyRatio(ratio, parent, x0, y0, x1, y1)` (`squarify.js` 6-53): the children of `parent`
/// are laid in rows, each row a strip along the shorter side of the box, and a row grows while its
/// worst aspect ratio does not get worse.
fn squarify_ratio<T>(
    ratio: f64,
    nodes: &mut [Node<'_, T>],
    parent: usize,
    mut x0: f64,
    mut y0: f64,
    x1: f64,
    y1: f64,
) {
    let children = nodes[parent].children.clone().unwrap_or_default();
    let n = children.len();
    let mut value = nodes[parent].value;
    let mut i0 = 0;
    let mut i1 = 0;
    while i0 < n {
        let dx = x1 - x0;
        let dy = y1 - y0;

        // Find the next non-empty node.
        let mut sum_value;
        loop {
            sum_value = nodes[children[i1]].value;
            i1 += 1;
            if !is_truthy(sum_value) && i1 < n {
                continue;
            }
            break;
        }
        let mut min_value = sum_value;
        let mut max_value = sum_value;
        let alpha = js_max(dy / dx, dx / dy) / (value * ratio);
        let mut beta = sum_value * sum_value * alpha;
        let mut min_ratio = js_max(max_value / beta, beta / min_value);

        // Keep adding nodes while the aspect ratio maintains or improves.
        while i1 < n {
            let node_value = nodes[children[i1]].value;
            sum_value += node_value;
            if node_value < min_value {
                min_value = node_value;
            }
            if node_value > max_value {
                max_value = node_value;
            }
            beta = sum_value * sum_value * alpha;
            let new_ratio = js_max(max_value / beta, beta / min_value);
            if new_ratio > min_ratio {
                sum_value -= node_value;
                break;
            }
            min_ratio = new_ratio;
            i1 += 1;
        }

        // Position the row along the shorter side of the box, as `dice` (wider than tall) or
        // `slice` lays it. The new edge is computed before the call, as `value ? y0 += … : y1`.
        let row = children[i0..i1].to_vec();
        let dice = dx < dy;
        if dice {
            let next = if is_truthy(value) {
                y0 + dy * sum_value / value
            } else {
                y1
            };
            dice_row(nodes, &row, sum_value, x0, y0, x1, next);
            if is_truthy(value) {
                y0 = next;
            }
        } else {
            let next = if is_truthy(value) {
                x0 + dx * sum_value / value
            } else {
                x1
            };
            slice_row(nodes, &row, sum_value, x0, y0, next, y1);
            if is_truthy(value) {
                x0 = next;
            }
        }
        value -= sum_value;
        i0 = i1;
    }
}

/// `dice(row, x0, y0, x1, y1)` (`dice.js`): the nodes of a row side by side, from left to right.
fn dice_row<T>(
    nodes: &mut [Node<'_, T>],
    row: &[usize],
    row_value: f64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) {
    let k = if is_truthy(row_value) {
        (x1 - x0) / row_value
    } else {
        row_value
    };
    let mut x = x0;
    for &child in row {
        nodes[child].y0 = y0;
        nodes[child].y1 = y1;
        nodes[child].x0 = x;
        x += nodes[child].value * k;
        nodes[child].x1 = x;
    }
}

/// `slice(row, x0, y0, x1, y1)` (`slice.js`): the nodes of a row one above the other, from the top.
fn slice_row<T>(
    nodes: &mut [Node<'_, T>],
    row: &[usize],
    row_value: f64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) {
    let k = if is_truthy(row_value) {
        (y1 - y0) / row_value
    } else {
        row_value
    };
    let mut y = y0;
    for &child in row {
        nodes[child].x0 = x0;
        nodes[child].x1 = x1;
        nodes[child].y0 = y;
        y += nodes[child].value * k;
        nodes[child].y1 = y;
    }
}

/// A number that JavaScript treats as true: not zero, and not NaN.
fn is_truthy(value: f64) -> bool {
    value != 0.0 && !value.is_nan()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Item {
        value: f64,
        children: Option<Vec<Item>>,
    }

    fn kids(item: &Item) -> Option<&[Item]> {
        item.children.as_deref()
    }

    fn no_padding(_: &Node<'_, Item>) -> Padding {
        Padding {
            inner: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    #[test]
    fn two_leaves_of_equal_value_split_the_box_in_half() {
        let root = Item {
            value: 0.0,
            children: Some(vec![
                Item {
                    value: 1.0,
                    children: None,
                },
                Item {
                    value: 1.0,
                    children: None,
                },
            ]),
        };
        let mut hierarchy = Hierarchy::new(&root, kids);
        hierarchy.sum(|item| item.value);
        hierarchy.sort(|a, b| b.value - a.value);
        let treemap = Treemap {
            width: 100.0,
            height: 40.0,
            round: true,
        };
        treemap.layout(&mut hierarchy, no_padding);
        let boxes: Vec<(f64, f64, f64, f64)> = hierarchy.nodes()[1..]
            .iter()
            .map(|node| (node.x0, node.y0, node.x1, node.y1))
            .collect();
        // Sides of 40 and 20: the box is wider than tall, so the children are side by side.
        assert_eq!(boxes, [(0.0, 0.0, 50.0, 40.0), (50.0, 0.0, 100.0, 40.0)]);
    }

    #[test]
    fn a_zero_value_takes_no_room_and_the_empty_root_is_its_box() {
        let root = Item {
            value: 0.0,
            children: Some(Vec::new()),
        };
        let mut hierarchy = Hierarchy::new(&root, kids);
        hierarchy.sum(|item| item.value);
        let treemap = Treemap {
            width: 1000.0,
            height: 400.0,
            round: true,
        };
        treemap.layout(&mut hierarchy, no_padding);
        let root_box = &hierarchy.nodes()[0];
        assert_eq!(
            (root_box.x0, root_box.y0, root_box.x1, root_box.y1),
            (0.0, 0.0, 1000.0, 400.0)
        );
    }
}
