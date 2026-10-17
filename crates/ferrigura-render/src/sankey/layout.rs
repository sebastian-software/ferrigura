//! The layout of `d3-sankey` 0.12.3 (`src/sankey.js` and `src/align.js`), as Mermaid runs it
//! (`sankeyRenderer.ts` 121-134): the nodes are put in columns by their depth, their breadths are
//! relaxed for six iterations, and each link gets its breadth. The order of the operations, the
//! sorts and their comparators, and the NaN behaviour of JavaScript are the library's. The research
//! is in `docs-port/sankey/NOTES.md` (section 6.5).
//!
//! The extent starts at `(0, 0)`, as Mermaid's `extent([[0, 0], [width, height]])` gives it. A
//! position that is NaN or infinite, which a zero or non-finite value gives, flows through the
//! arithmetic as it does in JavaScript, and the sorts see it as the library's comparators do.

use ferrigura_jsmath::pow;

use crate::common::d3::{math_max, math_min, min_by, sort_js, sum_by};

/// The x and y of the start of the extent. Mermaid's extent starts at the origin.
const X0: f64 = 0.0;
const Y0: f64 = 0.0;

/// The number of relaxation iterations (`sankey.js` 64).
const ITERATIONS: usize = 6;

/// The tolerance below which a collision is not resolved (`sankey.js` 305, 315).
const COLLISION_TOLERANCE: f64 = 1e-6;

/// `sankeyLeft`, `sankeyRight`, `sankeyCenter` and `sankeyJustify` (`align.js`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// `sankeyLeft`: the depth.
    Left,
    /// `sankeyRight`: the column counted from the sinks.
    Right,
    /// `sankeyCenter`: the depth, or next to the nearest target for a node without inputs.
    Center,
    /// `sankeyJustify`: the depth, and the last column for a node without outputs.
    Justify,
}

/// A link as the layout takes it: the indices of its ends and its value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkSpec {
    /// The index of the node the link starts at.
    pub source: usize,
    /// The index of the node the link ends at.
    pub target: usize,
    /// The value of the link. Any number, NaN and the infinities included.
    pub value: f64,
}

/// The settings Mermaid gives the generator (`sankeyRenderer.ts` 121-129).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The width of the extent (`nodeWidth`'s layout, the `width` option).
    pub width: f64,
    /// The height of the extent (the `height` option).
    pub height: f64,
    /// The width of a node, `dx`.
    pub node_width: f64,
    /// The padding between the nodes of a column, `dy`.
    pub node_padding: f64,
    /// The alignment of the nodes.
    pub align: Align,
}

/// A node after the layout. `value`, `depth`, `height` and `layer` are its counts, and the
/// coordinates are its box in the extent.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// `node.value`: the larger of the sums of its outgoing and incoming values.
    pub value: f64,
    /// `node.depth`: the longest path from a node with no input to this node, in links.
    pub depth: usize,
    /// `node.height`: the longest path from this node to a node with no output, in links.
    pub height: usize,
    /// `node.layer`: the column the alignment puts the node in.
    pub layer: usize,
    /// The left edge.
    pub x0: f64,
    /// The right edge.
    pub x1: f64,
    /// The top edge.
    pub y0: f64,
    /// The bottom edge.
    pub y1: f64,
    /// The indices of the outgoing links, in the order the layout leaves them.
    pub source_links: Vec<usize>,
    /// The indices of the incoming links, in the order the layout leaves them.
    pub target_links: Vec<usize>,
}

/// A link after the layout: its ends, its value, its width and the ends of its path.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    /// The index of the node the link starts at.
    pub source: usize,
    /// The index of the node the link ends at.
    pub target: usize,
    /// The value of the link.
    pub value: f64,
    /// `link.width`: the value times the scale of the column.
    pub width: f64,
    /// The y of the link at its source.
    pub y0: f64,
    /// The y of the link at its target.
    pub y1: f64,
}

/// The layout of a graph: its nodes in index order and its links in input order.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The nodes, in the order they were given.
    pub nodes: Vec<Node>,
    /// The links, in the order they were given.
    pub links: Vec<Link>,
}

/// The layout found a cycle: `throw new Error("circular link")` (`sankey.js` 169, 187).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircularLink;

/// Lays out a graph of `node_count` nodes and the given links, as `sankey(graph)` does.
///
/// # Errors
///
/// Returns [`CircularLink`] when the links form a cycle, the one error the library throws.
pub fn sankey(
    node_count: usize,
    links: &[LinkSpec],
    settings: Settings,
) -> Result<Layout, CircularLink> {
    let mut graph = Sankey::new(node_count, links, settings);
    graph.compute_node_links(links);
    graph.compute_node_values();
    graph.compute_node_depths()?;
    graph.compute_node_heights()?;
    graph.compute_node_breadths();
    graph.compute_link_breadths();
    Ok(Layout {
        nodes: graph.nodes,
        links: graph.links,
    })
}

/// `d3-sankey`'s closure: the graph, and the settings the phases read. `py` is the padding
/// between the nodes of a column, which the breadth phase sets from `dy` and the column sizes.
struct Sankey {
    nodes: Vec<Node>,
    links: Vec<Link>,
    /// `x1` of the extent (`x0` is `X0`).
    x1: f64,
    /// `y1` of the extent (`y0` is `Y0`).
    y1: f64,
    /// `dx`: the width of a node.
    dx: f64,
    /// `dy`: the padding the settings give.
    dy: f64,
    /// `py`: the padding between the nodes of a column, as the breadth phase has set it.
    py: f64,
    align: Align,
}

impl Sankey {
    fn new(node_count: usize, links: &[LinkSpec], settings: Settings) -> Self {
        Self {
            nodes: (0..node_count)
                .map(|_| Node {
                    value: 0.0,
                    depth: 0,
                    height: 0,
                    layer: 0,
                    x0: 0.0,
                    x1: 0.0,
                    y0: 0.0,
                    y1: 0.0,
                    source_links: Vec::new(),
                    target_links: Vec::new(),
                })
                .collect(),
            links: links
                .iter()
                .map(|link| Link {
                    source: link.source,
                    target: link.target,
                    value: link.value,
                    width: 0.0,
                    y0: 0.0,
                    y1: 0.0,
                })
                .collect(),
            x1: settings.width,
            y1: settings.height,
            dx: settings.node_width,
            dy: settings.node_padding,
            py: settings.node_padding,
            align: settings.align,
        }
    }

    /// `computeNodeLinks` (`sankey.js` 126-147): the links of each node, in input order.
    fn compute_node_links(&mut self, links: &[LinkSpec]) {
        for (index, link) in links.iter().enumerate() {
            self.nodes[link.source].source_links.push(index);
            self.nodes[link.target].target_links.push(index);
        }
    }

    /// `computeNodeValues` (`sankey.js` 149-155): the larger of the two sums.
    fn compute_node_values(&mut self) {
        for index in 0..self.nodes.len() {
            let outgoing = sum_by(&self.nodes[index].source_links, |&link| {
                self.links[link].value
            });
            let incoming = sum_by(&self.nodes[index].target_links, |&link| {
                self.links[link].value
            });
            self.nodes[index].value = math_max(outgoing, incoming);
        }
    }

    /// `computeNodeDepths` (`sankey.js` 157-173): a breadth-first walk from every node, so that a
    /// node's depth is the last level it is reached at. A walk longer than the node count is a
    /// cycle.
    fn compute_node_depths(&mut self) -> Result<(), CircularLink> {
        let count = self.nodes.len();
        let mut current: Vec<usize> = (0..count).collect();
        let mut level = 0;
        while !current.is_empty() {
            let mut next = Vec::new();
            let mut queued = vec![false; count];
            for &node in &current {
                self.nodes[node].depth = level;
                for &link in &self.nodes[node].source_links {
                    let target = self.links[link].target;
                    if !queued[target] {
                        queued[target] = true;
                        next.push(target);
                    }
                }
            }
            level += 1;
            if level > count {
                return Err(CircularLink);
            }
            current = next;
        }
        Ok(())
    }

    /// `computeNodeHeights` (`sankey.js` 175-191): the same walk along the incoming links.
    fn compute_node_heights(&mut self) -> Result<(), CircularLink> {
        let count = self.nodes.len();
        let mut current: Vec<usize> = (0..count).collect();
        let mut level = 0;
        while !current.is_empty() {
            let mut next = Vec::new();
            let mut queued = vec![false; count];
            for &node in &current {
                self.nodes[node].height = level;
                for &link in &self.nodes[node].target_links {
                    let source = self.links[link].source;
                    if !queued[source] {
                        queued[source] = true;
                        next.push(source);
                    }
                }
            }
            level += 1;
            if level > count {
                return Err(CircularLink);
            }
            current = next;
        }
        Ok(())
    }

    /// `align` of `align.js`: the column of a node, before it is clamped to the columns. `n` is the
    /// number of columns, which `justify` and `right` count from.
    fn align_of(&self, node: usize, n: usize) -> f64 {
        let subject = &self.nodes[node];
        match self.align {
            Align::Left => subject.depth as f64,
            Align::Right => (n as f64) - 1.0 - subject.height as f64,
            Align::Justify => {
                if subject.source_links.is_empty() {
                    (n as f64) - 1.0
                } else {
                    subject.depth as f64
                }
            }
            Align::Center => {
                if !subject.target_links.is_empty() {
                    subject.depth as f64
                } else if !subject.source_links.is_empty() {
                    // `min(node.sourceLinks, targetDepth) - 1`: one column before the nearest target.
                    min_by(&subject.source_links, |&link| {
                        self.nodes[self.links[link].target].depth as f64
                    })
                    .map_or(f64::NAN, |depth| depth - 1.0)
                } else {
                    0.0
                }
            }
        }
    }

    /// `computeNodeLayers` (`sankey.js` 193-209): the x of every node from its column, and the
    /// nodes of each column in node order.
    fn compute_node_layers(&mut self) -> Vec<Vec<usize>> {
        let max_depth = self.nodes.iter().map(|node| node.depth).max().unwrap_or(0);
        let x = max_depth + 1;
        let kx = (self.x1 - X0 - self.dx) / (x as f64 - 1.0);
        let mut columns: Vec<Vec<usize>> = vec![Vec::new(); x];
        for index in 0..self.nodes.len() {
            let aligned = self.align_of(index, x).floor();
            let layer = math_max(0.0, math_min((x - 1) as f64, aligned)) as usize;
            let node = &mut self.nodes[index];
            node.layer = layer;
            node.x0 = X0 + layer as f64 * kx;
            node.x1 = node.x0 + self.dx;
            columns[layer].push(index);
        }
        columns
    }

    /// `computeNodeBreadths` (`sankey.js` 233-243): the columns, the padding, the first placement,
    /// and the six relaxation iterations.
    fn compute_node_breadths(&mut self) {
        let mut columns = self.compute_node_layers();
        let longest = columns.iter().map(Vec::len).max().unwrap_or(0) as f64;
        self.py = math_min(self.dy, (self.y1 - Y0) / (longest - 1.0));
        self.initialize_node_breadths(&columns);
        for iteration in 0..ITERATIONS {
            let alpha = pow(0.99, iteration as f64);
            let beta = math_max(1.0 - alpha, (iteration as f64 + 1.0) / ITERATIONS as f64);
            self.relax_right_to_left(&mut columns, alpha, beta);
            self.relax_left_to_right(&mut columns, alpha, beta);
        }
    }

    /// `initializeNodeBreadths` (`sankey.js` 211-231): one scale for every column, the nodes stacked
    /// from the top with the padding, and each column centred in the extent.
    fn initialize_node_breadths(&mut self, columns: &[Vec<usize>]) {
        let py = self.py;
        let ky = min_by(columns, |column| {
            (self.y1 - Y0 - (column.len() as f64 - 1.0) * py)
                / sum_by(column, |&node| self.nodes[node].value)
        })
        // `min` of nothing is `undefined`, which the multiplications read as NaN.
        .unwrap_or(f64::NAN);
        for column in columns {
            let mut y = Y0;
            for &node in column {
                self.nodes[node].y0 = y;
                self.nodes[node].y1 = y + self.nodes[node].value * ky;
                y = self.nodes[node].y1 + py;
                for &link in &self.nodes[node].source_links {
                    self.links[link].width = self.links[link].value * ky;
                }
            }
            let centre = (self.y1 - y + py) / (column.len() as f64 + 1.0);
            for (position, &node) in column.iter().enumerate() {
                let shift = centre * (position as f64 + 1.0);
                self.nodes[node].y0 += shift;
                self.nodes[node].y1 += shift;
            }
            for &node in column {
                self.sort_source_links(node);
                self.sort_target_links(node);
            }
        }
    }

    /// `relaxRightToLeft` (`sankey.js` 269-289): each column, from the one before the last, moves
    /// its nodes towards the weighted position of their targets, then resolves the collisions.
    fn relax_right_to_left(&mut self, columns: &mut [Vec<usize>], alpha: f64, beta: f64) {
        let count = columns.len();
        for index in (0..count.saturating_sub(1)).rev() {
            let mut column = std::mem::take(&mut columns[index]);
            for &source in &column {
                let mut y = 0.0;
                let mut w = 0.0;
                for &link in &self.nodes[source].source_links {
                    let target = self.links[link].target;
                    let v = self.links[link].value
                        * (self.nodes[target].layer as f64 - self.nodes[source].layer as f64);
                    y += self.source_top(source, target) * v;
                    w += v;
                }
                if w.is_nan() || w <= 0.0 {
                    continue;
                }
                let shift = (y / w - self.nodes[source].y0) * alpha;
                self.nodes[source].y0 += shift;
                self.nodes[source].y1 += shift;
                self.reorder_node_links(source);
            }
            sort_js(&mut column, |a, b| self.ascending_breadth(a, b));
            self.resolve_collisions(&column, beta);
            columns[index] = column;
        }
    }

    /// `relaxLeftToRight` (`sankey.js` 246-266): each column after the first moves its nodes towards
    /// the weighted position of their sources, then resolves the collisions.
    fn relax_left_to_right(&mut self, columns: &mut [Vec<usize>], alpha: f64, beta: f64) {
        for slot in columns.iter_mut().skip(1) {
            let mut column = std::mem::take(slot);
            for &target in &column {
                let mut y = 0.0;
                let mut w = 0.0;
                for &link in &self.nodes[target].target_links {
                    let source = self.links[link].source;
                    let v = self.links[link].value
                        * (self.nodes[target].layer as f64 - self.nodes[source].layer as f64);
                    y += self.target_top(source, target) * v;
                    w += v;
                }
                if w.is_nan() || w <= 0.0 {
                    continue;
                }
                let shift = (y / w - self.nodes[target].y0) * alpha;
                self.nodes[target].y0 += shift;
                self.nodes[target].y1 += shift;
                self.reorder_node_links(target);
            }
            sort_js(&mut column, |a, b| self.ascending_breadth(a, b));
            self.resolve_collisions(&column, beta);
            *slot = column;
        }
    }

    /// `resolveCollisions` (`sankey.js` 291-298): the column is pushed apart from its middle node,
    /// upwards and downwards, and then kept inside the extent.
    fn resolve_collisions(&mut self, column: &[usize], alpha: f64) {
        let middle = column.len() >> 1;
        let subject = column[middle];
        let above = self.nodes[subject].y0 - self.py;
        self.resolve_bottom_to_top(column, above, middle as isize - 1, alpha);
        let below = self.nodes[subject].y1 + self.py;
        self.resolve_top_to_bottom(column, below, middle + 1, alpha);
        self.resolve_bottom_to_top(column, self.y1, column.len() as isize - 1, alpha);
        self.resolve_top_to_bottom(column, Y0, 0, alpha);
    }

    /// `resolveCollisionsTopToBottom` (`sankey.js` 301-308): the nodes from `start` down are pushed
    /// down where they overlap the one above.
    fn resolve_top_to_bottom(&mut self, column: &[usize], mut y: f64, start: usize, alpha: f64) {
        for &node in &column[start..] {
            let shift = (y - self.nodes[node].y0) * alpha;
            if shift > COLLISION_TOLERANCE {
                self.nodes[node].y0 += shift;
                self.nodes[node].y1 += shift;
            }
            y = self.nodes[node].y1 + self.py;
        }
    }

    /// `resolveCollisionsBottomToTop` (`sankey.js` 311-318): the nodes from `start` up are pushed up
    /// where they overlap the one below.
    fn resolve_bottom_to_top(&mut self, column: &[usize], mut y: f64, start: isize, alpha: f64) {
        let mut position = start;
        while position >= 0 {
            let node = column[position as usize];
            let shift = (self.nodes[node].y1 - y) * alpha;
            if shift > COLLISION_TOLERANCE {
                self.nodes[node].y0 -= shift;
                self.nodes[node].y1 -= shift;
            }
            y = self.nodes[node].y0 - self.py;
            position -= 1;
        }
    }

    /// `reorderNodeLinks` (`sankey.js` 320-329): the links of the neighbours of a node are sorted
    /// by the breadth of their other ends.
    fn reorder_node_links(&mut self, node: usize) {
        for link in self.nodes[node].target_links.clone() {
            let source = self.links[link].source;
            self.sort_source_links(source);
        }
        for link in self.nodes[node].source_links.clone() {
            let target = self.links[link].target;
            self.sort_target_links(target);
        }
    }

    /// `sourceLinks.sort(ascendingTargetBreadth)` of a node, as `reorderLinks` (`sankey.js` 331-338)
    /// and `reorderNodeLinks` (320-329) make it.
    fn sort_source_links(&mut self, node: usize) {
        let mut links = std::mem::take(&mut self.nodes[node].source_links);
        sort_js(&mut links, |a, b| self.ascending_target_breadth(a, b));
        self.nodes[node].source_links = links;
    }

    /// `targetLinks.sort(ascendingSourceBreadth)` of a node.
    fn sort_target_links(&mut self, node: usize) {
        let mut links = std::mem::take(&mut self.nodes[node].target_links);
        sort_js(&mut links, |a, b| self.ascending_source_breadth(a, b));
        self.nodes[node].target_links = links;
    }

    /// `targetTop` (`sankey.js` 341-352): the `y0` a node would need so that its link to `target` is
    /// ideal.
    fn target_top(&self, source: usize, target: usize) -> f64 {
        let subject = &self.nodes[source];
        let mut y = subject.y0 - ((subject.source_links.len() as f64 - 1.0) * self.py) / 2.0;
        for &link in &subject.source_links {
            if self.links[link].target == target {
                break;
            }
            y += self.links[link].width + self.py;
        }
        for &link in &self.nodes[target].target_links {
            if self.links[link].source == source {
                break;
            }
            y -= self.links[link].width;
        }
        y
    }

    /// `sourceTop` (`sankey.js` 355-366): the `y0` the source needs so that its link to `target` is
    /// ideal.
    fn source_top(&self, source: usize, target: usize) -> f64 {
        let subject = &self.nodes[target];
        let mut y = subject.y0 - ((subject.target_links.len() as f64 - 1.0) * self.py) / 2.0;
        for &link in &subject.target_links {
            if self.links[link].source == source {
                break;
            }
            y += self.links[link].width + self.py;
        }
        for &link in &self.nodes[source].source_links {
            if self.links[link].target == target {
                break;
            }
            y -= self.links[link].width;
        }
        y
    }

    /// `ascendingBreadth` (`sankey.js` 13-15): the difference of the `y0` of two nodes, which is NaN
    /// when either is.
    fn ascending_breadth(&self, a: usize, b: usize) -> f64 {
        self.nodes[a].y0 - self.nodes[b].y0
    }

    /// `ascendingSourceBreadth` (`sankey.js` 5-7): the links by the breadth of their sources, then
    /// by index. A difference that is zero or NaN is "falsy", so the index decides, as `||` does.
    fn ascending_source_breadth(&self, a: usize, b: usize) -> f64 {
        let by_breadth = self.ascending_breadth(self.links[a].source, self.links[b].source);
        if truthy(by_breadth) {
            by_breadth
        } else {
            a as f64 - b as f64
        }
    }

    /// `ascendingTargetBreadth` (`sankey.js` 9-11): the links by the breadth of their targets, then
    /// by index.
    fn ascending_target_breadth(&self, a: usize, b: usize) -> f64 {
        let by_breadth = self.ascending_breadth(self.links[a].target, self.links[b].target);
        if truthy(by_breadth) {
            by_breadth
        } else {
            a as f64 - b as f64
        }
    }

    /// `computeLinkBreadths` (`sankey.js` 39-52): each link's `y0` at its source and `y1` at its
    /// target, stacked from the node's top.
    fn compute_link_breadths(&mut self) {
        for node in 0..self.nodes.len() {
            let mut y0 = self.nodes[node].y0;
            let mut y1 = y0;
            for &link in &self.nodes[node].source_links {
                self.links[link].y0 = y0 + self.links[link].width / 2.0;
                y0 += self.links[link].width;
            }
            for &link in &self.nodes[node].target_links {
                self.links[link].y1 = y1 + self.links[link].width / 2.0;
                y1 += self.links[link].width;
            }
        }
    }
}

/// The JavaScript truthiness of a number: not zero and not NaN.
fn truthy(value: f64) -> bool {
    value != 0.0 && !value.is_nan()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(width: f64, height: f64, align: Align) -> Settings {
        Settings {
            width,
            height,
            node_width: 10.0,
            node_padding: 27.0,
            align,
        }
    }

    #[test]
    fn a_cycle_is_the_circular_link_error() {
        let links = [
            LinkSpec {
                source: 0,
                target: 1,
                value: 1.0,
            },
            LinkSpec {
                source: 1,
                target: 0,
                value: 1.0,
            },
        ];
        assert_eq!(
            sankey(2, &links, settings(600.0, 400.0, Align::Justify)),
            Err(CircularLink)
        );
    }

    #[test]
    fn a_zero_value_column_gives_nan_positions() {
        // Every column sums to zero, so the scale is infinite and each position is NaN.
        let links = [LinkSpec {
            source: 0,
            target: 1,
            value: 0.0,
        }];
        let layout = sankey(2, &links, settings(600.0, 400.0, Align::Justify)).expect("lays out");
        assert!(layout.nodes[0].y0.is_nan());
        assert!(layout.nodes[1].y1.is_nan());
    }

    #[test]
    fn a_single_link_fills_the_height() {
        let links = [LinkSpec {
            source: 0,
            target: 1,
            value: 10.0,
        }];
        let layout = sankey(2, &links, settings(600.0, 400.0, Align::Justify)).expect("lays out");
        assert_eq!(layout.nodes[0].y0, 0.0);
        assert_eq!(layout.nodes[0].y1, 400.0);
        assert_eq!(layout.nodes[1].x1, 600.0);
        assert_eq!(layout.links[0].y0, 200.0);
        assert_eq!(layout.links[0].y1, 200.0);
    }
}
