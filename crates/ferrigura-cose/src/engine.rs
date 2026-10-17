//! The spring embedder of cose-base's `CoSELayout` and layout-base's `FDLayout` on the tree path:
//! the run (`runLayout`), the parameters, the set-up of the embedder, one iteration (`tick`), the
//! cooling and the convergence test. The forces are in `forces.rs`.
//!
//! The growth of pruned nodes and the gravity of compounds are not here: a tree has no pruned
//! nodes (`reduceTrees` is not run for a forest), and gravity of a flat graph applies only to a
//! disconnected forest, which `classic_layout` handles.

use crate::constants::{
    ADAPTATION_LOWER_NODE_LIMIT, ADAPTATION_UPPER_NODE_LIMIT, CONVERGENCE_CHECK_PERIOD,
    COOLING_ADAPTATION_FACTOR, DISPLACEMENT_THRESHOLD_PER_NODE, IDEAL_EDGE_LENGTH, MAX_ITERATIONS,
    MAX_NODE_DISPLACEMENT,
};
use crate::error::Error;
use crate::geometry::{PointD, js_max};
use crate::graph::LGraph;

/// The state of one layout run.
pub(crate) struct Engine {
    pub graph: LGraph,
    /// `FDLayout.grid`: the cells of the repulsion grid, each the nodes it covers.
    pub grid: Vec<Vec<Vec<usize>>>,
    pub total_iterations: u64,
    pub max_iterations: f64,
    pub total_displacement: f64,
    pub old_total_displacement: f64,
    pub total_displacement_threshold: f64,
    pub cooling_factor: f64,
    pub initial_cooling_factor: f64,
    pub cooling_cycle: f64,
    pub max_cooling_cycle: f64,
    pub final_temperature: f64,
    pub max_node_displacement: f64,
    pub repulsion_range: f64,
    /// The nodes that gravity acts on: those of a disconnected graph (`calculateNodesToApplyGravitationTo`).
    pub nodes_with_gravity: Vec<usize>,
}

impl Engine {
    fn new(graph: LGraph) -> Engine {
        Engine {
            graph,
            grid: Vec::new(),
            total_iterations: 0,
            max_iterations: MAX_ITERATIONS,
            total_displacement: 0.0,
            old_total_displacement: 0.0,
            total_displacement_threshold: 0.0,
            cooling_factor: 0.5,
            initial_cooling_factor: 0.5,
            cooling_cycle: 0.0,
            max_cooling_cycle: 0.0,
            final_temperature: 0.0,
            max_node_displacement: MAX_NODE_DISPLACEMENT,
            repulsion_range: 0.0,
            nodes_with_gravity: Vec::new(),
        }
    }

    /// `CoSELayout.initParameters` and `FDLayout.initParameters` for a layout that is not a
    /// sub-layout. The cooling schedule is set here, from `maxIterations` before the embedder
    /// raises it (`initSpringEmbedder`), as the source does.
    fn init_parameters(&mut self) {
        self.total_iterations = 0;
        self.max_cooling_cycle = self.max_iterations / CONVERGENCE_CHECK_PERIOD as f64;
        self.final_temperature = CONVERGENCE_CHECK_PERIOD as f64 / self.max_iterations;
    }

    /// `CoSELayout.classicLayout` (`CoSELayout.js` 84-120) for a graph that is a forest: the
    /// gravity set, the ideal lengths, the radial start, then the embedder. A graph that is not a
    /// forest is an error, as the random path is not ported.
    fn classic_layout(&mut self) -> Result<(), Error> {
        let n = self.graph.nodes.len();
        self.nodes_with_gravity = if self.graph.is_connected() {
            Vec::new()
        } else {
            (0..n).collect()
        };
        self.graph.calc_estimated_size();
        for edge in &mut self.graph.edges {
            edge.ideal_length = IDEAL_EDGE_LENGTH;
        }
        let forest = self.graph.flat_forest();
        if forest.is_empty() {
            return Err(Error::NotAForest);
        }
        self.graph
            .position_nodes_radially(&forest)
            .ok_or(Error::NotAForest)?;
        self.init_spring_embedder();
        self.run_spring_embedder();
        Ok(())
    }

    /// `FDLayout.initSpringEmbedder` (`FDLayout.js` 82-111) for a non-incremental layout, and
    /// `CoSELayout.calcRepulsionRange` at level 0.
    fn init_spring_embedder(&mut self) {
        let s = self.graph.nodes.len() as f64;
        self.cooling_factor = if s > ADAPTATION_LOWER_NODE_LIMIT {
            js_max(
                COOLING_ADAPTATION_FACTOR,
                1.0 - (s - ADAPTATION_LOWER_NODE_LIMIT)
                    / (ADAPTATION_UPPER_NODE_LIMIT - ADAPTATION_LOWER_NODE_LIMIT)
                    * (1.0 - COOLING_ADAPTATION_FACTOR),
            )
        } else {
            1.0
        };
        self.initial_cooling_factor = self.cooling_factor;
        self.max_node_displacement = MAX_NODE_DISPLACEMENT;
        self.max_iterations = js_max(s * 5.0, self.max_iterations);
        self.total_displacement_threshold = DISPLACEMENT_THRESHOLD_PER_NODE * s;
        // calcRepulsionRange: 2 * (level + 1) * idealEdgeLength, with level 0.
        self.repulsion_range = 2.0 * (0.0 + 1.0) * IDEAL_EDGE_LENGTH;
    }

    /// `CoSELayout.runSpringEmbedder` (`CoSELayout.js` 213-225): iterate until `tick` ends the
    /// layout, then update the bounds.
    fn run_spring_embedder(&mut self) {
        while !self.tick() {}
        self.graph.update_bounds();
    }

    /// `CoSELayout.tick` (`CoSELayout.js` 122-195) for a tree: one iteration. Returns true when the
    /// layout is over. The growth phase is not entered, because a tree has no pruned nodes.
    fn tick(&mut self) -> bool {
        self.total_iterations += 1;
        if self.total_iterations as f64 == self.max_iterations {
            return true;
        }
        if self
            .total_iterations
            .is_multiple_of(CONVERGENCE_CHECK_PERIOD)
        {
            if self.is_converged() {
                return true;
            }
            self.cooling_cycle += 1.0;
            // The cooling adjuster stays 1 for quality 'proof' (CoSELayout.js 177-184 only change it
            // for quality 0 and 1).
            let exponent = ferrigura_jsmath::log(
                100.0 * (self.initial_cooling_factor - self.final_temperature),
            ) / ferrigura_jsmath::log(self.max_cooling_cycle);
            self.cooling_factor = js_max(
                self.initial_cooling_factor
                    - ferrigura_jsmath::pow(self.cooling_cycle, exponent) / 100.0,
                self.final_temperature,
            );
        }
        self.total_displacement = 0.0;
        self.graph.update_bounds();
        self.calc_spring_forces();
        self.calc_repulsion_forces(true, false);
        self.calc_gravitational_forces();
        self.move_nodes();
        false
    }

    /// `FDLayout.isConverged` (`FDLayout.js` 358-373): the total displacement is below the
    /// threshold, or it has barely changed since the last test after a third of the iterations.
    fn is_converged(&mut self) -> bool {
        let mut oscillating = false;
        if self.total_iterations as f64 > self.max_iterations / 3.0 {
            oscillating = (self.total_displacement - self.old_total_displacement).abs() < 2.0;
        }
        let converged = self.total_displacement < self.total_displacement_threshold;
        self.old_total_displacement = self.total_displacement;
        converged || oscillating
    }
}

/// Lays out a flat graph and returns the centre of each node, in the order of the nodes
/// (`_CoSELayout.prototype.run`, `index.js` 124-156 and 262-283, with `runLayout`'s final
/// transform). An empty graph has no positions.
pub(crate) fn lay_out(graph: LGraph) -> Result<Vec<PointD>, Error> {
    if graph.nodes.is_empty() {
        return Ok(Vec::new());
    }
    // tilingPreLayout (CoSELayout.js 753-758) runs before the parameters are set. It groups nodes
    // with no edge only when there are two or more of them; a single one is left alone.
    if graph.isolated_count() >= 2 {
        return Err(Error::TilingNotPorted);
    }
    let mut engine = Engine::new(graph);
    engine.init_parameters();
    engine.classic_layout()?;
    // doPostLayout: the transform to the origin, with the margin.
    engine.graph.transform_to(PointD::new(0.0, 0.0));
    Ok((0..engine.graph.nodes.len())
        .map(|node| engine.graph.centre(node))
        .collect())
}
