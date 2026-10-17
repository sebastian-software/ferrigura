//! cytoscape-cose-bilkent 4.1.0 on cose-base 1.0.3 and layout-base 1.0.2, ported from the
//! JavaScript sources. This is the layout Mermaid's mindmaps use by default (`layout: cose-bilkent`).
//!
//! **Scope.** Mermaid gives the engine trees only: a mindmap has one root, and every other node
//! hangs off it. cose-base lays a tree out by its radial path: the centre of the tree is found by
//! peeling leaves, the nodes are placed on rings around it, and the spring embedder starts from
//! there. That path draws no random numbers, so its output depends on the input alone.
//!
//! **Not ported.** The random path of cose-base is unreachable from Mermaid and is not ported:
//! `positionNodesRandomly`, `reduceTrees`, the pruned-node growth, and the draws of `RandomSeed` and
//! `Math.random`. A graph with a cycle is [`Error::NotAForest`]. Two or more nodes with no edge are
//! [`Error::TilingNotPorted`], because cose-base then groups them under a dummy compound, the
//! tiling path (a mindmap has one root, so it never has two). Compounds, the incremental mode and
//! the tiling are not in the input at all.
//!
//! **Edge endpoints.** The layout gives node centres. cytoscape also writes the edge endpoints
//! (`rscratch`: start, mid and end points, cytoscape 24690-24741 and 27575-27584), for the 30 by 30
//! default node style. They belong to the mindmap renderer and are not here; see
//! `docs-port/cose/NOTES.md` section 5.2. The corpus under `tests/corpus` records them, so the renderer
//! can check its own port against them.
//!
//! **Numbers.** `sin`, `cos`, `log` and `pow` are the defaults of `ferrigura_jsmath`, which give
//! Chromium's results bit for bit. Each expression keeps the operand order of the source, so the
//! output agrees with the reference exactly, not within a tolerance. Positions are the centres of
//! the nodes after the final transform to the origin.
//!
//! **Input.** Initial positions are not read. Mermaid passes zero for every node, and the radial
//! start sets every centre, so the initial position has no effect. Self loops and parallel edges
//! are dropped, as cytoscape-cose-bilkent's adapter drops them.
//!
//! Licences and sources are in `NOTICE.md`: the crate is MIT, as its sources are.

#![forbid(unsafe_code)]

mod constants;
mod engine;
mod error;
mod forces;
mod forest;
mod geometry;
mod graph;

pub use error::Error;

/// A node of the graph: its id, and the size of its box, as Mermaid measured it.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    pub width: f64,
    pub height: f64,
}

/// An edge from the node `source` to the node `target`. `id` is carried for the caller; the layout
/// does not read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
}

/// The centre of a node after the layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub id: String,
    pub x: f64,
    pub y: f64,
}

/// Lays out a graph the way cytoscape-cose-bilkent does for Mermaid's mindmaps. The result has one
/// position per node, in the order of `nodes`.
///
/// # Errors
///
/// See [`Error`]: a graph that is not a forest, two or more nodes with no edge, duplicate or unknown
/// ids, and sizes that are not finite numbers.
pub fn layout(nodes: &[Node], edges: &[Edge]) -> Result<Vec<Position>, Error> {
    let graph = graph::LGraph::build(nodes, edges)?;
    let centres = engine::lay_out(graph)?;
    Ok(nodes
        .iter()
        .zip(centres)
        .map(|(node, centre)| Position {
            id: node.id.clone(),
            x: centre.x,
            y: centre.y,
        })
        .collect())
}
