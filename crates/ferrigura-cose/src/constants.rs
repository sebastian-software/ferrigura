//! The constants the engine reads, with the values Mermaid's run gives them.
//!
//! cytoscape-cose-bilkent's `getUserOptions` (`index.js` 84-122) writes the layout-base and cose-base
//! constants from its options, and Mermaid passes `quality: 'proof'`, `animate: false` and the
//! library defaults for the rest (`cytoscape-setup.ts` 130-136). Where the library default differs
//! from the value after `getUserOptions`, this file has the value after it, and says so.

/// `LayoutConstants.WORLD_CENTER_X` and `WORLD_CENTER_Y` (`LayoutConstants.js`): where a forest is
/// placed before the final transform.
pub(crate) const WORLD_CENTER_X: f64 = 1200.0;
pub(crate) const WORLD_CENTER_Y: f64 = 900.0;

/// `LayoutConstants.WORLD_BOUNDARY`: node corners are clamped to it before a transform.
pub(crate) const WORLD_BOUNDARY: f64 = 1_000_000.0;

/// `LayoutConstants.DEFAULT_GRAPH_MARGIN`: the margin of the root graph (`LGraph.margin`). The
/// margin is read from the node above the root (`getParent().paddingLeft`), which has no padding,
/// so the default applies to every flat graph.
pub(crate) const GRAPH_MARGIN: f64 = 15.0;

/// `LGraph` estimated size of an empty graph (`EMPTY_COMPOUND_NODE_SIZE`).
pub(crate) const EMPTY_COMPOUND_NODE_SIZE: f64 = 40.0;

/// `FDLayoutConstants.DEFAULT_EDGE_LENGTH`, which is also `idealEdgeLength` (`index.js` 87-88).
pub(crate) const IDEAL_EDGE_LENGTH: f64 = 50.0;

/// `DEFAULT_SPRING_STRENGTH`, which is `edgeElasticity` (`index.js` 89-90): 0.45.
pub(crate) const SPRING_STRENGTH: f64 = 0.45;

/// `DEFAULT_REPULSION_STRENGTH`, which is `nodeRepulsion` (`index.js` 85-86): 4500.
pub(crate) const REPULSION_STRENGTH: f64 = 4500.0;

/// `DEFAULT_GRAVITY_STRENGTH`, which is `gravity` (`index.js` 93-94): 0.25, where layout-base has
/// 0.4. Only disconnected forests feel gravity.
pub(crate) const GRAVITY_STRENGTH: f64 = 0.25;

/// `DEFAULT_GRAVITY_RANGE_FACTOR`, which is `gravityRange` (`index.js` 97-98): 3.8.
pub(crate) const GRAVITY_RANGE_FACTOR: f64 = 3.8;

/// `MAX_ITERATIONS`, which is `numIter` (`index.js` 95-96): 2500. The spring embedder runs at least
/// `5 n` iterations, so this is the minimum.
pub(crate) const MAX_ITERATIONS: f64 = 2500.0;

/// `FDLayoutConstants.MAX_NODE_DISPLACEMENT`: 300 (`100 * 3`), the cap of one move per axis at
/// cooling factor 1.
pub(crate) const MAX_NODE_DISPLACEMENT: f64 = 300.0;

/// `FDLayoutConstants.MIN_REPULSION_DIST`: 5 (`DEFAULT_EDGE_LENGTH / 10`). Closer centres are pushed
/// apart as if they were this far.
pub(crate) const MIN_REPULSION_DIST: f64 = IDEAL_EDGE_LENGTH / 10.0;

/// `CONVERGENCE_CHECK_PERIOD`: the cooling and the convergence test run every 100 iterations.
pub(crate) const CONVERGENCE_CHECK_PERIOD: u64 = 100;

/// `GRID_CALCULATION_CHECK_PERIOD`: the repulsion grid is rebuilt when `iteration % 10 == 1`.
pub(crate) const GRID_CALCULATION_CHECK_PERIOD: u64 = 10;

/// `displacementThresholdPerNode` (`FDLayout` constructor): `3 * DEFAULT_EDGE_LENGTH / 100`. The
/// layout has converged when the total displacement is below this per node.
pub(crate) const DISPLACEMENT_THRESHOLD_PER_NODE: f64 = (3.0 * IDEAL_EDGE_LENGTH) / 100.0;

/// `FDLayoutConstants.COOLING_ADAPTATION_FACTOR`: the lowest cooling factor for large graphs.
pub(crate) const COOLING_ADAPTATION_FACTOR: f64 = 0.33;

/// `ADAPTATION_LOWER_NODE_LIMIT` and `ADAPTATION_UPPER_NODE_LIMIT`: the node counts between which
/// the cooling factor falls from 1 to `COOLING_ADAPTATION_FACTOR`.
pub(crate) const ADAPTATION_LOWER_NODE_LIMIT: f64 = 1000.0;
pub(crate) const ADAPTATION_UPPER_NODE_LIMIT: f64 = 5000.0;

/// `CoSEConstants.DEFAULT_COMPONENT_SEPERATION` (sic): the gap between the trees of a forest.
pub(crate) const COMPONENT_SEPARATION: f64 = 60.0;

/// `CoSEConstants.DEFAULT_RADIAL_SEPARATION`, which is `DEFAULT_EDGE_LENGTH`: the least gap between
/// two rings of a tree.
pub(crate) const RADIAL_SEPARATION: f64 = IDEAL_EDGE_LENGTH;
