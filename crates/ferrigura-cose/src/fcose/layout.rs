//! The driver of cose-base 2.2.0's `CoSELayout` for the incremental path that fcose takes, with
//! layout-base 2.0.1's `Layout.runLayout` and `FDLayout` set-up around it: the parameters, the
//! classic layout (gravity set, inclusion depths, estimated sizes, ideal lengths, constraints),
//! the spring embedder (`tick`, convergence, cooling), and the move of every node.
//!
//! What fcose does not reach is not here: tiling and tree reduction (fcose turns `tile` off and
//! `TREE_REDUCTION_ON_INCREMENTAL` off whenever a constraint object exists, and Mermaid always
//! passes one), `positionNodesRandomly` and the radial start (both need `incremental` false),
//! the pruned-node growth, the final transform (`doPostLayout` does not transform an incremental
//! layout), and animation. `mod.rs` refuses an input without constraints, which is the one case
//! that would reach tiling.

use crate::fcose::constraints::{ConstraintVars, FixedNodeConstraint};
use crate::fcose::model::{Model, SIMPLE_NODE_SIZE};
use crate::fcose::rng::Mulberry32;
use crate::geometry::{PointD, js_max, sign};

/// Constants of one run (`FDLayoutConstants`, `CoSEConstants` and `LayoutConstants` after fcose's
/// `coseLayout` has set them, `cose.js` 180-236). They are per run: fcose overwrites the module
/// constants on every run, so nothing carries from one run to the next.
#[derive(Clone, Debug)]
pub(crate) struct Constants {
    /// `DEFAULT_EDGE_LENGTH`: the mean of the edges' ideal lengths (`cose.js` 152).
    pub default_edge_length: f64,
    /// `MIN_REPULSION_DIST`: `DEFAULT_EDGE_LENGTH / 10` (`cose.js` 158).
    pub min_repulsion_dist: f64,
    /// `DEFAULT_GRAVITY_STRENGTH` (the `gravity` option, 0.25), and the compound's (1.0).
    pub gravity_strength: f64,
    pub compound_gravity_strength: f64,
    /// `DEFAULT_GRAVITY_RANGE_FACTOR` (3.8) and `DEFAULT_COMPOUND_GRAVITY_RANGE_FACTOR` (1.5).
    pub gravity_range_factor: f64,
    pub compound_gravity_range_factor: f64,
    /// `MAX_ITERATIONS` (`numIter`).
    pub max_iterations: f64,
    /// `PER_LEVEL_IDEAL_EDGE_LENGTH_FACTOR` (`nestingFactor`, 0.1).
    pub nesting_factor: f64,
    /// `DEFAULT_COOLING_FACTOR_INCREMENTAL` (`initialEnergyOnIncremental`, 0.3).
    pub initial_cooling_factor: f64,
    /// `MAX_NODE_DISPLACEMENT_INCREMENTAL`: 100 (the incremental branch of `initSpringEmbedder`).
    pub max_node_displacement_incremental: f64,
}

impl Constants {
    /// The constants fcose sets for the options of Mermaid, given the mean ideal length.
    pub(crate) fn new(default_edge_length: f64, num_iter: f64) -> Constants {
        Constants {
            default_edge_length,
            min_repulsion_dist: default_edge_length / 10.0,
            gravity_strength: 0.25,
            compound_gravity_strength: 1.0,
            gravity_range_factor: 3.8,
            compound_gravity_range_factor: 1.5,
            max_iterations: num_iter,
            nesting_factor: 0.1,
            initial_cooling_factor: 0.3,
            max_node_displacement_incremental: 100.0,
        }
    }
}

/// `CoSELayout` `constraints`: the alignment object and the relative list, as fcose passes them
/// after its own sanitising (`fcose/index.js` 113-134 and `cose.js` 210-215). An empty list is
/// `None`, as fcose sets it to `undefined`. `alignment` is `Some` whenever the object was given,
/// which is what makes fcose's `Object.keys(constraints).length > 0` true.
#[derive(Clone, Debug, Default)]
pub(crate) struct Constraints {
    pub alignment: Option<AlignmentConstraint>,
    pub relative: Option<Vec<RelativeConstraint>>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct AlignmentConstraint {
    pub horizontal: Option<Vec<Vec<String>>>,
    pub vertical: Option<Vec<Vec<String>>>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RelativeConstraint {
    pub left: Option<String>,
    pub right: Option<String>,
    pub top: Option<String>,
    pub bottom: Option<String>,
    pub gap: Option<f64>,
}

impl Constraints {
    /// `Object.keys(this.constraints).length > 0`.
    pub(crate) fn any(&self) -> bool {
        self.alignment.is_some() || self.relative.is_some()
    }
}

/// The state of one layout run: the model, the parameters and the counters of the spring
/// embedder. Field names follow the source's properties.
pub(crate) struct Layout {
    pub model: Model,
    pub k: Constants,
    /// `options.randomize`. With `false` the spectral start is not run, and the constraint
    /// handler does not transform the draft (`TRANSFORM_ON_CONSTRAINT_HANDLING`).
    pub randomize: bool,
    /// The generator of `withSeededRandom` for this run.
    pub rng: Mulberry32,
    pub constraints: Constraints,
    /// fcose's `fixedNodeConstraint` (`cose.js` 166-168). Mermaid gives none, so it stays `None`.
    pub fixed_node_constraint: Option<Vec<FixedNodeConstraint>>,
    /// The state `initConstraintVariables` sets for `updateDisplacements` (`constraints.rs`).
    pub constraint_vars: ConstraintVars,
    /// `allNodesToApplyGravitation`: the nodes of unconnected graphs.
    pub nodes_with_gravity: Vec<usize>,

    // FDLayout.
    /// `totalIterations`. It counts `tick` calls, so it is an integer kept as a double.
    pub total_iterations: f64,
    pub max_iterations: f64,
    pub total_displacement: f64,
    pub old_total_displacement: f64,
    pub total_displacement_threshold: f64,
    pub displacement_threshold_per_node: f64,
    pub cooling_factor: f64,
    pub initial_cooling_factor: f64,
    pub max_node_displacement: f64,
    pub repulsion_range: f64,
    /// `grid[i][j]`: the nodes in each repulsion cell.
    pub grid: Vec<Vec<Vec<usize>>>,

    // CoSELayout.
    pub ideal_edge_length: f64,
    pub gravity_constant: f64,
    pub compound_gravity_constant: f64,
    pub gravity_range_factor: f64,
    pub compound_gravity_range_factor: f64,
    pub level: f64,
    pub cooling_cycle: f64,
    pub max_cooling_cycle: f64,
    pub final_temperature: f64,
    pub cooling_adjuster: f64,
}

impl Layout {
    pub(crate) fn new(
        model: Model,
        k: Constants,
        randomize: bool,
        rng: Mulberry32,
        constraints: Constraints,
    ) -> Layout {
        // `CoSELayout.initParameters` (CoSELayout.js 51-82) and `FDLayout.initParameters`
        // (FDLayout.js 29-38): the ideal length is the mean, floored at 10.
        let ideal_edge_length = if k.default_edge_length < 10.0 {
            10.0
        } else {
            k.default_edge_length
        };
        let max_iterations = k.max_iterations;
        let initial_cooling_factor = k.initial_cooling_factor;
        Layout {
            model,
            randomize,
            rng,
            constraints,
            fixed_node_constraint: None,
            constraint_vars: ConstraintVars::default(),
            nodes_with_gravity: Vec::new(),
            total_iterations: 0.0,
            max_iterations,
            total_displacement: 0.0,
            old_total_displacement: 0.0,
            total_displacement_threshold: 0.0,
            displacement_threshold_per_node: (3.0 * k.default_edge_length) / 100.0,
            cooling_factor: initial_cooling_factor,
            initial_cooling_factor,
            max_node_displacement: 0.0,
            repulsion_range: 0.0,
            grid: Vec::new(),
            ideal_edge_length,
            gravity_constant: k.gravity_strength,
            compound_gravity_constant: k.compound_gravity_strength,
            gravity_range_factor: k.gravity_range_factor,
            compound_gravity_range_factor: k.compound_gravity_range_factor,
            level: 0.0,
            cooling_cycle: 0.0,
            max_cooling_cycle: 0.0,
            final_temperature: 0.0,
            cooling_adjuster: 1.0,
            k,
        }
    }

    /// `Layout.runLayout` for a layout that is not a sub-layout, with `incremental` true and
    /// `quality` 'proof'. Returns without changes when the root has no nodes (`checkLayoutSuccess`).
    pub(crate) fn run(&mut self) -> Result<(), crate::fcose::Error> {
        let root = self.model.root();
        if self.model.graphs[root].nodes.is_empty() {
            return Ok(());
        }
        // `Layout.initParameters` (`Layout.js` 258-270) sets the quality, the incremental flag and
        // the leaf-size flag; `FDLayout.initParameters` (`FDLayout.js` 29-38) resets the counters.
        self.total_iterations = 0.0;
        self.classic_layout()
    }

    /// `CoSELayout.layout` (`CoSELayout.js` 94-104): the level is 0, then the classic layout.
    fn classic_layout(&mut self) -> Result<(), crate::fcose::Error> {
        // `calculateNodesToApplyGravitationTo` (`CoSELayout.js` 802-822): every graph is tested for
        // connectivity, and the nodes of the unconnected ones feel gravity.
        let mut nodes_with_gravity = Vec::new();
        for graph in 0..self.model.graphs.len() {
            self.model.update_connected(graph);
            if !self.model.graphs[graph].is_connected {
                nodes_with_gravity.extend_from_slice(&self.model.graphs[graph].nodes);
            }
        }
        self.nodes_with_gravity = nodes_with_gravity;

        self.calc_no_of_children_for_all_nodes();
        self.model.calc_lowest_common_ancestors();
        self.model.calc_inclusion_tree_depths(0, 1.0);
        self.model.calc_graph_estimated_size(0);
        self.calc_ideal_edge_lengths();

        // The incremental branch of `classicLayout` (`CoSELayout.js` 138-146) reduces trees only
        // when TREE_REDUCTION_ON_INCREMENTAL is set, which fcose clears whenever a constraint
        // exists. Tiling (`tilingPreLayout`) is off for the same reason.
        if self.constraints.any() {
            crate::fcose::constraints::handle_constraints(self);
            crate::fcose::constraints::init_constraint_variables(self);
        } else {
            return Err(crate::fcose::Error::Unsupported(
                "a layout without constraints reaches the tiling and tree reduction, which are not ported",
            ));
        }
        self.init_spring_embedder();
        self.run_spring_embedder();
        Ok(())
    }

    /// `calcNoOfChildrenForAllNodes` (`FDLayout.js` 391-401).
    fn calc_no_of_children_for_all_nodes(&mut self) {
        for node in self.model.all_nodes() {
            self.model.nodes[node].no_of_children = self.model.no_of_children(node);
        }
    }

    /// `FDLayout.calcIdealEdgeLengths` (`FDLayout.js` 40-78) for the inter-graph edges (the other
    /// edges keep their ideal length), with the smart ideal length on (`CoSEConstants`).
    fn calc_ideal_edge_lengths(&mut self) {
        for edge in self.model.all_edges() {
            let original = self.model.edges[edge].ideal_length;
            if !self.model.edges[edge].inter_graph {
                continue;
            }
            let source = self.model.edges[edge].source;
            let target = self.model.edges[edge].target;
            let source_in_lca = self.model.edges[edge].source_in_lca;
            let target_in_lca = self.model.edges[edge].target_in_lca;
            let size_of_source_in_lca = self.model.nodes[source_in_lca].estimated_size;
            let size_of_target_in_lca = self.model.nodes[target_in_lca].estimated_size;
            // `useSmartIdealEdgeLengthCalculation` is true (FDLayoutConstants.js 22).
            self.model.edges[edge].ideal_length +=
                size_of_source_in_lca + size_of_target_in_lca - 2.0 * SIMPLE_NODE_SIZE;
            let lca = self.model.edges[edge]
                .lca
                .expect("every edge has a lowest common ancestor");
            let lca_depth = self.model.graph_inclusion_depth(lca);
            let depth_sum = self.model.nodes[source].inclusion_tree_depth
                + self.model.nodes[target].inclusion_tree_depth
                - 2.0 * lca_depth;
            self.model.edges[edge].ideal_length += original * self.k.nesting_factor * depth_sum;
        }
    }

    /// `FDLayout.initSpringEmbedder` (`FDLayout.js` 80-110) for the incremental branch, then
    /// `CoSELayout.initSpringEmbedder` (`CoSELayout.js` 84-92).
    fn init_spring_embedder(&mut self) {
        let s = self.model.all_nodes().len() as f64;
        let lower = 1000.0;
        let upper = 5000.0;
        let adaptation = 0.33;
        if s > lower {
            self.cooling_factor = js_max(
                self.cooling_factor * adaptation,
                self.cooling_factor
                    - (s - lower) / (upper - lower) * self.cooling_factor * (1.0 - adaptation),
            );
        }
        self.max_node_displacement = self.k.max_node_displacement_incremental;
        self.max_iterations = js_max(
            self.model.all_nodes().len() as f64 * 5.0,
            self.max_iterations,
        );
        self.displacement_threshold_per_node = (3.0 * self.k.default_edge_length) / 100.0;
        self.total_displacement_threshold =
            self.displacement_threshold_per_node * self.model.all_nodes().len() as f64;
        // `calcRepulsionRange` of CoSELayout (CoSELayout.js 1061-1063).
        self.repulsion_range = 2.0 * (self.level + 1.0) * self.ideal_edge_length;
        // CoSELayout.initSpringEmbedder: the cooling schedule of quality 'proof'.
        self.cooling_cycle = 0.0;
        self.max_cooling_cycle = self.max_iterations / 100.0;
        self.final_temperature = 0.04;
        self.cooling_adjuster = 1.0;
    }

    /// `runSpringEmbedder` (`CoSELayout.js` 278-296), animation off: tick until the layout ends,
    /// then the bounds.
    fn run_spring_embedder(&mut self) {
        loop {
            if self.tick() {
                break;
            }
        }
        self.model.update_bounds();
    }

    /// `CoSELayout.tick` (`CoSELayout.js` 163-258) without the growth phase (no pruned nodes).
    /// Returns true when the layout has ended.
    fn tick(&mut self) -> bool {
        self.total_iterations += 1.0;
        if self.total_iterations == self.max_iterations {
            return true;
        }
        if self.total_iterations % 100.0 == 0.0 {
            if self.is_converged() {
                return true;
            }
            self.cooling_cycle += 1.0;
            // The adjuster stays 1 for quality 'proof' (only 0 and 1 change it).
            let exponent = ferrigura_jsmath::log(
                100.0 * (self.initial_cooling_factor - self.final_temperature),
            ) / ferrigura_jsmath::log(self.max_cooling_cycle);
            self.cooling_factor = js_max(
                self.initial_cooling_factor
                    - ferrigura_jsmath::pow(self.cooling_cycle, exponent) / 100.0
                        * self.cooling_adjuster,
                self.final_temperature,
            );
        }
        self.total_displacement = 0.0;
        self.model.update_bounds();
        self.calc_spring_forces();
        self.calc_repulsion_forces(true, false);
        self.calc_gravitational_forces();
        self.move_nodes();
        false
    }

    /// `FDLayout.isConverged` (`FDLayout.js` 358-373).
    fn is_converged(&mut self) -> bool {
        let mut oscillating = false;
        if self.total_iterations > self.max_iterations / 3.0 {
            oscillating = (self.total_displacement - self.old_total_displacement).abs() < 2.0;
        }
        let converged = self.total_displacement < self.total_displacement_threshold;
        self.old_total_displacement = self.total_displacement;
        converged || oscillating
    }

    /// `CoSELayout.moveNodes` (`CoSELayout.js` 298-322): every node computes its displacement,
    /// the constraints adjust them when there are any, then every node moves.
    fn move_nodes(&mut self) {
        let nodes = self.model.all_nodes();
        for &node in &nodes {
            self.calculate_displacement(node);
        }
        if self.constraints.any() {
            crate::fcose::constraints::update_displacements(self);
        }
        for &node in &nodes {
            self.move_node(node);
        }
    }

    /// `CoSENode.calculateDisplacement` (`CoSENode.js` 13-49).
    fn calculate_displacement(&mut self, node: usize) {
        let cooling = self.cooling_factor;
        let max_displacement = self.max_node_displacement;
        let fixed = self.model.nodes[node].fixed_node_weight;
        let n = &mut self.model.nodes[node];
        let weight = match (n.child.is_some(), fixed) {
            (true, Some(w)) if w != 0.0 && !w.is_nan() => w,
            _ => n.no_of_children,
        };
        n.displacement.x += cooling * (n.spring.x + n.repulsion.x + n.gravitation.x) / weight;
        n.displacement.y += cooling * (n.spring.y + n.repulsion.y + n.gravitation.y) / weight;
        if n.displacement.x.abs() > cooling * max_displacement {
            n.displacement.x = cooling * max_displacement * sign(n.displacement.x);
        }
        if n.displacement.y.abs() > cooling * max_displacement {
            n.displacement.y = cooling * max_displacement * sign(n.displacement.y);
        }
        let (dx, dy) = (n.displacement.x, n.displacement.y);
        if let Some(child) = n.child.filter(|&c| !self.model.graphs[c].nodes.is_empty()) {
            self.propagate_displacement(child, dx, dy);
        }
    }

    /// `CoSENode.propogateDisplacementToChildren` (`CoSENode.js` 51-68): the leaves below a
    /// compound get its displacement added.
    fn propagate_displacement(&mut self, graph: usize, dx: f64, dy: f64) {
        let nodes = self.model.graphs[graph].nodes.clone();
        for node in nodes {
            match self.model.nodes[node].child {
                None => {
                    self.model.nodes[node].displacement.x += dx;
                    self.model.nodes[node].displacement.y += dy;
                }
                Some(child) => self.propagate_displacement(child, dx, dy),
            }
        }
    }

    /// `CoSENode.move` (`CoSENode.js` 70-90): a leaf (or a compound with no node) moves by its
    /// displacement; every force and displacement is reset.
    fn move_node(&mut self, node: usize) {
        let movable = match self.model.nodes[node].child {
            None => true,
            Some(child) => self.model.graphs[child].nodes.is_empty(),
        };
        if movable {
            let n = &mut self.model.nodes[node];
            n.rect.x += n.displacement.x;
            n.rect.y += n.displacement.y;
            self.total_displacement += self.model.nodes[node].displacement.x.abs()
                + self.model.nodes[node].displacement.y.abs();
        }
        let n = &mut self.model.nodes[node];
        n.spring = PointD::default();
        n.repulsion = PointD::default();
        n.gravitation = PointD::default();
        n.displacement = PointD::default();
    }
}
