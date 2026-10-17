//! cytoscape-fcose 2.2.0 on cose-base 2.2.0 and layout-base 2.0.1, ported from the JavaScript
//! sources: the layout Mermaid's architecture diagrams use (`layoutArchitecture`, with the options
//! of `architectureRenderer.ts` 478-510).
//!
//! **Shape of a run.** Mermaid calls `layout.run()` twice per render (`architectureRenderer.ts`
//! 582-601 and 513-580). Each run is one call of [`layout`] with the node positions the run starts
//! from, the mean ideal length of the edges and their elasticities, the constraints, the component
//! bounds that fcose relocates to, and the seed of `withSeededRandom`. The second run's input is
//! the first run's output and the bounds after the segment weights the renderer computes from that
//! output; those weights are the renderer's and stay outside the engine.
//!
//! **Path.** fcose turns constraints on whenever it receives an alignment object, which Mermaid
//! always does (`getAlignments` returns `{horizontal, vertical}`). Then `tile` and tree reduction
//! are off, the constraint handler runs, and the spring embedder runs in the incremental branch.
//! Packing is off (Mermaid registers no layout utilities), so fcose lays out the whole graph as one
//! component. `randomize: true` runs the spectral start (`spectral.rs`) before the layout, with the
//! same generator, and the constraint handler's transform (the orthogonal Procrustes fit of
//! `linalg.rs`). Not ported: tiling, tree reduction and the positions of hidden children (both need
//! a run with no constraints or a hidden child, and Mermaid gives neither), the packing of
//! components, `draft` quality, and the fixed-node constraint (Mermaid never gives one). See
//! `docs-port/architecture/NOTES.md`, section E, for the reasons.
//!
//! **Numbers.** `sin`, `cos`, `log` and `pow` are `ferrigura_jsmath`'s, which give Chromium's bits.
//! The generator of the random draws is `crate::fcose::rng`. Each expression keeps the operand
//! order of the source.
//!
//! **Output.** Each node's position is the centre of its rectangle after the relocation to the
//! component centre (`relocateComponent`). Mermaid writes positions for the nodes without children
//! only; a compound's position is cytoscape's, so the value returned for a compound is the
//! layout's rectangle and is not what the renderer draws.

pub(crate) mod constraints;
pub(crate) mod forces;
pub(crate) mod layout;
pub(crate) mod linalg;
pub(crate) mod model;
pub(crate) mod rng;
pub(crate) mod spectral;

use std::collections::{HashMap, HashSet};

use crate::fcose::layout::{
    AlignmentConstraint, Constants, Constraints, Layout, RelativeConstraint,
};
use crate::fcose::model::Model;
use crate::fcose::rng::Mulberry32;
use crate::fcose::spectral::{
    IdealEdgeLength, SpectralInput, SpectralNode, SpectralResult, spectral_layout,
};
use crate::geometry::RectD;

/// Why [`layout`] does not lay a run out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Two nodes have the same id, which cytoscape refuses.
    DuplicateNode(String),
    /// A node names a parent, or an edge names a node, that is not in the input.
    UnknownNode(String),
    /// A width, height, position, ideal length or elasticity that is not a finite number.
    NonFinite(String),
    /// The run needs a part that is not ported, or is not reproducible in Mermaid.
    Unsupported(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DuplicateNode(id) => write!(f, "two nodes have the id {id:?}"),
            Error::UnknownNode(id) => write!(f, "{id:?} is not a node of the input"),
            Error::NonFinite(id) => write!(f, "{id:?} has a value that is not a finite number"),
            Error::Unsupported(reason) => write!(f, "not ported: {reason}"),
        }
    }
}

impl std::error::Error for Error {}

/// A node of the input, as cytoscape holds it when fcose runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    /// The parent's id, or `None` for a top-level node.
    pub parent: Option<String>,
    /// `layoutDimensions` of the node (`w`, `h`) with `nodeDimensionsIncludeLabels: false`.
    pub width: f64,
    pub height: f64,
    /// `parseInt(css('padding'))`: 40 for a group, 0 for the others.
    pub padding: f64,
    /// `position()` at the start of the run. Only the top-level and leaf nodes read it (the
    /// non-random start of `processChildrenList`).
    pub x: f64,
    pub y: f64,
}

/// An edge of the input, with the ideal length and the elasticity the renderer's functions give it.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub ideal_length: f64,
    pub elasticity: f64,
}

/// `eles.boundingBox()` at the start of the run: fcose's component centre is
/// `(x1 + w / 2, y1 + h / 2)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub x1: f64,
    pub y1: f64,
    pub w: f64,
    pub h: f64,
}

/// `alignmentConstraint`: lists of node ids that share a row (`horizontal`) or a column
/// (`vertical`). An empty list is the same as no list.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Alignment {
    pub horizontal: Vec<Vec<String>>,
    pub vertical: Vec<Vec<String>>,
}

/// One `relativePlacementConstraint`: `left` and `right` (or `top` and `bottom`) with a gap.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Relative {
    pub left: Option<String>,
    pub right: Option<String>,
    pub top: Option<String>,
    pub bottom: Option<String>,
    pub gap: Option<f64>,
}

/// The options of one run that Mermaid sets (`architectureRenderer.ts` 478-510). The others are
/// fcose's defaults, fixed in `Constants` and in the layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub randomize: bool,
    /// `nodeSeparation` (read only by the spectral start).
    pub node_separation: f64,
    /// `numIter`.
    pub num_iter: f64,
}

/// One run of the layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    /// The nodes, in cytoscape's order.
    pub nodes: Vec<Node>,
    /// The edges, in cytoscape's order.
    pub edges: Vec<Edge>,
    /// `alignmentConstraint`, or `None` when the option is not given.
    pub alignment: Option<Alignment>,
    /// `relativePlacementConstraint`, or `None` when the option is not given or empty.
    pub relative: Vec<Relative>,
    /// The bounds of the elements before the run (the component centre).
    pub bounds: Bounds,
    /// The seed of `withSeededRandom`. Must be at least 1: Mermaid's seed 0 is not reproducible.
    pub seed: u32,
    pub options: Options,
}

/// The centre of a node after the run, in the order of `Input::nodes`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Lays out one run of fcose. Returns one position per input node, in order.
///
/// # Errors
///
/// See [`Error`]: duplicate or unknown ids, parents that are not nodes, non-finite numbers, a
/// missing alignment (the tiling path), and `randomize: true` (the spectral start is not ported).
pub fn layout(input: &Input) -> Result<Vec<Position>, Error> {
    if input.seed == 0 {
        return Err(Error::Unsupported(
            "seed 0 means the real Math.random in Mermaid, which is not reproducible",
        ));
    }
    if input.nodes.is_empty() {
        return Ok(Vec::new());
    }
    if input.alignment.is_none() {
        return Err(Error::Unsupported(
            "a run with no alignment constraint reaches the tiling and tree reduction, which are not ported",
        ));
    }
    // Every id a constraint names must be a node: the constraint code reads them by id (the source
    // throws for a missing node), so the check is here, where the error can be returned.
    let known: HashSet<&str> = input.nodes.iter().map(|node| node.id.as_str()).collect();
    let mut named: Vec<&str> = Vec::new();
    if let Some(alignment) = &input.alignment {
        for list in alignment.horizontal.iter().chain(&alignment.vertical) {
            named.extend(list.iter().map(String::as_str));
        }
    }
    for relative in &input.relative {
        for id in [
            &relative.left,
            &relative.right,
            &relative.top,
            &relative.bottom,
        ]
        .into_iter()
        .flatten()
        {
            named.push(id.as_str());
        }
    }
    if let Some(id) = named.into_iter().find(|id| !known.contains(id)) {
        return Err(Error::UnknownNode(id.to_string()));
    }
    let numbers_finite = |values: &[f64]| values.iter().all(|v| v.is_finite());
    for node in &input.nodes {
        if !numbers_finite(&[node.width, node.height, node.padding, node.x, node.y]) {
            return Err(Error::NonFinite(node.id.clone()));
        }
    }
    for edge in &input.edges {
        if !numbers_finite(&[edge.ideal_length, edge.elasticity]) {
            return Err(Error::NonFinite(edge.id.clone()));
        }
    }
    if !numbers_finite(&[
        input.bounds.x1,
        input.bounds.y1,
        input.bounds.w,
        input.bounds.h,
    ]) {
        return Err(Error::NonFinite("bounds".to_string()));
    }

    // `processChildrenList` and `processEdges` (cose.js 48-161) build the model.
    let mut model = Model::new();
    let mut by_id: HashMap<&str, usize> = HashMap::with_capacity(input.nodes.len());
    for (index, node) in input.nodes.iter().enumerate() {
        if by_id.insert(node.id.as_str(), index).is_some() {
            return Err(Error::DuplicateNode(node.id.clone()));
        }
    }
    let mut parent_of: Vec<Option<usize>> = Vec::with_capacity(input.nodes.len());
    for node in &input.nodes {
        let parent = match &node.parent {
            None => None,
            Some(id) => Some(
                *by_id
                    .get(id.as_str())
                    .ok_or_else(|| Error::UnknownNode(id.clone()))?,
            ),
        };
        parent_of.push(parent);
    }
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); input.nodes.len()];
    let mut top_level: Vec<usize> = Vec::new();
    for (index, parent) in parent_of.iter().enumerate() {
        match parent {
            None => top_level.push(index),
            Some(p) => children[*p].push(index),
        }
    }
    // One generator serves the spectral start and the layout: fcose wraps the whole run in
    // `withSeededRandom` (architectureRenderer.ts 582-601), so the draws continue across the two.
    let mut rng = rng::Mulberry32::new(input.seed);
    let spectral = if input.options.randomize {
        Some(spectral_start(input, &by_id, &parent_of, &mut rng)?)
    } else {
        None
    };
    let leaves_below: Vec<Vec<usize>> = (0..input.nodes.len())
        .map(|i| leaves_below(i, &children))
        .collect();
    let start = Start {
        spectral: spectral.as_ref(),
        leaves_below: &leaves_below,
    };

    // `model_node[i]`: the arena index of input node `i`.
    let mut model_node: Vec<usize> = vec![0; input.nodes.len()];
    add_children(
        &mut model,
        0,
        &top_level,
        input,
        &children,
        &start,
        &mut model_node,
    );

    // `processEdges` (cose.js 132-161): self loops and parallel edges are dropped, the ideal
    // lengths are summed in the order of the edges, and the mean becomes `DEFAULT_EDGE_LENGTH`.
    let mut total = 0.0;
    let mut count = 0usize;
    for edge in &input.edges {
        let source = *by_id
            .get(edge.source.as_str())
            .ok_or_else(|| Error::UnknownNode(edge.source.clone()))?;
        let target = *by_id
            .get(edge.target.as_str())
            .ok_or_else(|| Error::UnknownNode(edge.target.clone()))?;
        if source == target {
            continue;
        }
        let (s, t) = (model_node[source], model_node[target]);
        if !model.edges_between(s, t).is_empty() {
            continue;
        }
        let e = model.add_edge(s, t);
        model.edges[e].ideal_length = edge.ideal_length;
        model.edges[e].edge_elasticity = edge.elasticity;
        total += edge.ideal_length;
        count += 1;
    }
    let default_edge_length = if count > 0 {
        total / count as f64
    } else {
        // A function gives no value when there is no edge (cose.js 152-156).
        50.0
    };

    let constraints = Constraints {
        alignment: input
            .alignment
            .as_ref()
            .map(|alignment| AlignmentConstraint {
                horizontal: non_empty(&alignment.horizontal),
                vertical: non_empty(&alignment.vertical),
            }),
        relative: if input.relative.is_empty() {
            None
        } else {
            Some(
                input
                    .relative
                    .iter()
                    .map(|r| RelativeConstraint {
                        left: r.left.clone(),
                        right: r.right.clone(),
                        top: r.top.clone(),
                        bottom: r.bottom.clone(),
                        gap: r.gap,
                    })
                    .collect(),
            )
        },
    };

    let k = Constants::new(default_edge_length, input.options.num_iter);
    let mut layout = Layout::new(model, k, input.options.randomize, rng, constraints);
    layout.run()?;

    // `relocateComponent` (auxiliary.js 136-198, the non-draft branch): the bounding box of every
    // node's rectangle is moved so that its centre is the centre of the elements before the run.
    // It walks `Object.keys(idToLNode)`, and `idToLNode` is a plain object: assigning the key
    // `__proto__` sets its prototype instead of adding a key (cose.js 102-104 for the assignment,
    // auxiliary.js 143-186 for the walk), so a node with that id is missed by the walk, and it is
    // not moved either. Mermaid's spec inputs use that id (architecture.spec.ts 209 and 219).
    let component_x = input.bounds.x1 + input.bounds.w / 2.0;
    let component_y = input.bounds.y1 + input.bounds.h / 2.0;
    let own_key: Vec<bool> = layout
        .model
        .nodes
        .iter()
        .map(|node| node.id != "__proto__")
        .collect();
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (index, node) in layout.model.nodes.iter().enumerate() {
        if !own_key[index] {
            continue;
        }
        let rect = node.rect;
        if rect.x < min_x {
            min_x = rect.x;
        }
        if rect.right() > max_x {
            max_x = rect.right();
        }
        if rect.y < min_y {
            min_y = rect.y;
        }
        if rect.bottom() > max_y {
            max_y = rect.bottom();
        }
    }
    let diff_x = component_x - (max_x + min_x) / 2.0;
    let diff_y = component_y - (max_y + min_y) / 2.0;
    for (index, node) in layout.model.nodes.iter_mut().enumerate() {
        if !own_key[index] {
            continue;
        }
        let cx = node.rect.center_x() + diff_x;
        let cy = node.rect.center_y() + diff_y;
        node.rect.x = cx - node.rect.width / 2.0;
        node.rect.y = cy - node.rect.height / 2.0;
    }

    Ok(model_node
        .iter()
        .map(|&m| {
            let rect: RectD = layout.model.nodes[m].rect;
            Position {
                x: rect.center_x(),
                y: rect.center_y(),
            }
        })
        .collect())
}

fn non_empty(lists: &[Vec<String>]) -> Option<Vec<Vec<String>>> {
    if lists.is_empty() {
        None
    } else {
        Some(lists.to_vec())
    }
}

/// `spectralLayout(options)` (index.js 161-164) on every node and edge of the run. Mermaid's
/// options are the renderer's `nodeSeparation`, and fcose's defaults (`sampleSize` 25, `piTol`
/// 1e-7, greedy sampling), with `idealEdgeLength` a function (architectureRenderer.ts 478-510).
fn spectral_start(
    input: &Input,
    by_id: &HashMap<&str, usize>,
    parent_of: &[Option<usize>],
    rng: &mut Mulberry32,
) -> Result<SpectralResult, Error> {
    let nodes = input
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| SpectralNode {
            id: node.id.clone(),
            parent: parent_of[index],
            x: node.x,
            y: node.y,
        })
        .collect();
    let mut edges = Vec::with_capacity(input.edges.len());
    for edge in &input.edges {
        let source = *by_id
            .get(edge.source.as_str())
            .ok_or_else(|| Error::UnknownNode(edge.source.clone()))?;
        let target = *by_id
            .get(edge.target.as_str())
            .ok_or_else(|| Error::UnknownNode(edge.target.clone()))?;
        edges.push((source, target));
    }
    let spectral = SpectralInput {
        nodes,
        edges,
        node_separation: input.options.node_separation,
        sample_size: 25,
        pi_tol: 0.000_000_1,
        greedy_sampling: true,
        full_layout: true,
        ideal_edge_length: IdealEdgeLength::Function,
    };
    spectral_layout(&spectral, rng)
}

/// The childless nodes below each node, in cytoscape's order (`descendants().not(":parent")`,
/// fcose's `calcBoundingBox`, auxiliary.js 200-205). The order does not change the bounding box.
fn leaves_below(node: usize, children: &[Vec<usize>]) -> Vec<usize> {
    let mut leaves = Vec::new();
    let mut stack: Vec<usize> = children[node].clone();
    while let Some(child) = stack.pop() {
        if children[child].is_empty() {
            leaves.push(child);
        } else {
            stack.extend(children[child].iter().copied());
        }
    }
    leaves
}

/// Where the nodes start: the positions of the input (`randomize: false`, cose.js 85-87), or the
/// spectral coordinates (`randomize: true`, cose.js 56-72).
struct Start<'a> {
    spectral: Option<&'a SpectralResult>,
    leaves_below: &'a [Vec<usize>],
}

/// `processChildrenList` (cose.js 48-130). Each child becomes a node of its parent's graph; a child
/// with children gets a child graph, and its children are added to it, in order. A leaf's
/// rectangle is centred on its start position, and a compound's is the bounding box of its leaves
/// (`calcBoundingBox`, auxiliary.js 200-245), which the run then replaces by the children's bounds.
fn add_children(
    model: &mut Model,
    graph: usize,
    children: &[usize],
    input: &Input,
    all_children: &[Vec<usize>],
    start: &Start<'_>,
    model_node: &mut [usize],
) {
    for &child in children {
        let node = &input.nodes[child];
        let (mut x, mut y, width, height) = if all_children[child].is_empty() {
            match start.spectral {
                None => (
                    node.x - node.width / 2.0,
                    node.y - node.height / 2.0,
                    node.width,
                    node.height,
                ),
                Some(result) => {
                    let index = result.node_index[child];
                    let cx = index.map_or(f64::NAN, |i| result.x[i]);
                    let cy = index.map_or(f64::NAN, |i| result.y[i]);
                    (
                        cx - node.width / 2.0,
                        cy - node.height / 2.0,
                        node.width,
                        node.height,
                    )
                }
            }
        } else {
            match start.spectral {
                None => (
                    node.x - node.width / 2.0,
                    node.y - node.height / 2.0,
                    node.width,
                    node.height,
                ),
                Some(result) => {
                    let bounds = bounding_box(&start.leaves_below[child], input, result);
                    (bounds.0, bounds.1, bounds.2, bounds.3)
                }
            }
        };
        // `if (isNaN(theNode.rect.x)) theNode.rect.x = 0` (cose.js 115-121).
        if x.is_nan() {
            x = 0.0;
        }
        if y.is_nan() {
            y = 0.0;
        }
        let rect = RectD::new(x, y, width, height);
        let id = model.add_node(graph, &node.id, rect, Some(node.padding));
        model_node[child] = id;
        if !all_children[child].is_empty() {
            let child_graph = model.add_child_graph(id);
            add_children(
                model,
                child_graph,
                &all_children[child],
                input,
                all_children,
                start,
                model_node,
            );
        }
    }
}

/// `calcBoundingBox` (auxiliary.js 200-245) over the leaves below a compound: the leaves' extent
/// around their spectral centres, with `node.width()` and `node.height()` as the leaves' style
/// sizes. Returns `(left, top, width, height)`.
///
/// `node.width()` is cytoscape's style width, not the layout dimensions: `outerWidth()` is
/// `width + border + 2 * padding` (`collection/dimensions/width-height.mjs` 45-60), and the
/// architecture style sets no border, so a leaf's style width is its layout width less twice its
/// padding (a childless group has the default width under its padding).
fn bounding_box(leaves: &[usize], input: &Input, result: &SpectralResult) -> (f64, f64, f64, f64) {
    let mut left = 9_007_199_254_740_991.0_f64;
    let mut right = -9_007_199_254_740_991.0_f64;
    let mut top = 9_007_199_254_740_991.0_f64;
    let mut bottom = -9_007_199_254_740_991.0_f64;
    for &leaf in leaves {
        let node = &input.nodes[leaf];
        let style_width = node.width - 2.0 * node.padding;
        let style_height = node.height - 2.0 * node.padding;
        let index = result.node_index[leaf];
        let cx = index.map_or(f64::NAN, |i| result.x[i]);
        let cy = index.map_or(f64::NAN, |i| result.y[i]);
        let node_left = cx - style_width / 2.0;
        let node_right = cx + style_width / 2.0;
        let node_top = cy - style_height / 2.0;
        let node_bottom = cy + style_height / 2.0;
        if left > node_left {
            left = node_left;
        }
        if right < node_right {
            right = node_right;
        }
        if top > node_top {
            top = node_top;
        }
        if bottom < node_bottom {
            bottom = node_bottom;
        }
    }
    (left, top, right - left, bottom - top)
}
