//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/{p1cycles,p2layers,p3order,p4nodes,p5edges}/
//!
//! The five phases and their implementations, `LayeredPhases` and the strategy
//! enums that create them (`CycleBreakingStrategy`, `LayeringStrategy`, and so on).
//!
//! Each implementation names its processor configuration: the processors it
//! adds before or after phases. Those tables are the static
//! `*_PROCESSING_CONFIGURATION` constants of the Java classes.

use crate::core::alg::LayoutProcessorConfiguration;
use crate::core::data::JavaEnum;
use crate::core::data::enums::{
    CrossingMinimizationStrategy, CycleBreakingStrategy, EdgeRouting, GraphProperties,
    IntermediateProcessorStrategy as S, LayeringStrategy, NodePlacementStrategy,
};
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::p1cycles;
use crate::layered::p2layers;
use crate::layered::p3order;
use crate::layered::p5edges;

/// Phase index of `LayeredPhases.P1_CYCLE_BREAKING` and the others, by ordinal.
pub const P1: usize = 0;
pub const P2: usize = 1;
pub const P3: usize = 2;
pub const P4: usize = 3;
pub const P5: usize = 4;
pub const PHASES: usize = 5;

/// The configuration type all phases add to.
pub type Config = LayoutProcessorConfiguration<S>;

/// One phase implementation, chosen from the strategy options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseStep {
    CycleBreaking(CycleBreakingStrategy),
    Layering(LayeringStrategy),
    CrossingMinimization(CrossingMinimizationStrategy),
    NodePlacement(NodePlacementStrategy),
    EdgeRouting(EdgeRouting),
}

impl PhaseStep {
    /// The task name the phase logs, where its `begin` call has a fixed string.
    pub fn task_name(self, _graph: &LGraph) -> Result<String, LayoutError> {
        let name = match self {
            PhaseStep::CycleBreaking(CycleBreakingStrategy::DEPTH_FIRST) => {
                "Depth-first cycle removal"
            }
            PhaseStep::CycleBreaking(CycleBreakingStrategy::GREEDY) => "Greedy cycle removal",
            PhaseStep::CycleBreaking(CycleBreakingStrategy::MODEL_ORDER) => {
                "Model order cycle breaking"
            }
            PhaseStep::CycleBreaking(CycleBreakingStrategy::INTERACTIVE) => {
                "Interactive cycle breaking"
            }
            PhaseStep::Layering(LayeringStrategy::NETWORK_SIMPLEX) => "Network simplex layering",
            PhaseStep::Layering(LayeringStrategy::LONGEST_PATH) => "Longest path layering",
            PhaseStep::Layering(LayeringStrategy::LONGEST_PATH_SOURCE) => {
                "Longest path to source layering"
            }
            PhaseStep::Layering(LayeringStrategy::COFFMAN_GRAHAM) => "Coffman-Graham Layering",
            PhaseStep::Layering(LayeringStrategy::MIN_WIDTH) => "MinWidth layering",
            PhaseStep::Layering(LayeringStrategy::STRETCH_WIDTH) => "StretchWidth layering",
            PhaseStep::Layering(LayeringStrategy::INTERACTIVE) => "Interactive node layering",
            PhaseStep::CrossingMinimization(CrossingMinimizationStrategy::LAYER_SWEEP) => {
                return Ok(format!("Minimize Crossings {}", CROSS_MIN_TYPE));
            }
            PhaseStep::CrossingMinimization(other) => {
                return Err(LayoutError::Unsupported(format!(
                    "crossing minimization {} is not ported yet",
                    other.java_name()
                )));
            }
            PhaseStep::NodePlacement(NodePlacementStrategy::BRANDES_KOEPF) => {
                "Brandes & Koepf node placement"
            }
            PhaseStep::NodePlacement(other) => {
                return Err(LayoutError::Unsupported(format!(
                    "node placement {} is not ported yet",
                    other.java_name()
                )));
            }
            PhaseStep::EdgeRouting(EdgeRouting::ORTHOGONAL | EdgeRouting::UNDEFINED) => {
                "Orthogonal edge routing"
            }
            other => {
                return Err(LayoutError::Unsupported(format!(
                    "phase {other:?} is not ported yet"
                )));
            }
        };
        Ok(name.to_string())
    }

    /// Runs the phase's task under `monitor`. Only the nodes-only behaviour exists so far.
    pub fn run(
        self,
        graph: &mut LGraph,
        progress: &mut ProgressMonitor,
        monitor: Monitor,
    ) -> Result<(), LayoutError> {
        if self == PhaseStep::NodePlacement(NodePlacementStrategy::BRANDES_KOEPF) {
            return crate::layered::p4nodes::bk::place(graph, progress, monitor);
        }
        let name = self.task_name(graph)?;
        progress.begin(monitor, &name, 1.0);
        if self == PhaseStep::Layering(LayeringStrategy::NETWORK_SIMPLEX) {
            p2layers::network_simplex_layering(graph, progress, monitor)?;
        }
        if let PhaseStep::CycleBreaking(strategy) = self {
            cycle_breaking(graph, strategy)?;
        }
        if matches!(self, PhaseStep::CrossingMinimization(_)) {
            p3order::sweep(graph)?;
        }
        if matches!(self, PhaseStep::EdgeRouting(_)) {
            p5edges::route_nodes_only(graph)?;
        }
        progress.done(monitor);
        Ok(())
    }

    /// The processors this phase adds, `getLayoutProcessorConfiguration(graph)`.
    pub fn configuration(self, graph: &LGraph) -> Result<Option<Config>, LayoutError> {
        let mut config = Config::new(PHASES);
        match self {
            PhaseStep::CycleBreaking(CycleBreakingStrategy::INTERACTIVE) => {
                config
                    .add_before(P1, S::INTERACTIVE_EXTERNAL_PORT_POSITIONER)
                    .add_after(P5, S::REVERSED_EDGE_RESTORER);
            }
            PhaseStep::CycleBreaking(_) => {
                config.add_after(P5, S::REVERSED_EDGE_RESTORER);
            }
            PhaseStep::Layering(LayeringStrategy::INTERACTIVE) => {
                config
                    .add_before(P1, S::INTERACTIVE_EXTERNAL_PORT_POSITIONER)
                    .add_before(P2, S::LAYER_CONSTRAINT_PREPROCESSOR)
                    .add_before(P3, S::LAYER_CONSTRAINT_POSTPROCESSOR);
            }
            PhaseStep::Layering(_) => {
                config
                    .add_before(P1, S::EDGE_AND_LAYER_CONSTRAINT_EDGE_REVERSER)
                    .add_before(P2, S::LAYER_CONSTRAINT_PREPROCESSOR)
                    .add_before(P3, S::LAYER_CONSTRAINT_POSTPROCESSOR);
            }
            PhaseStep::CrossingMinimization(_) => {
                config
                    .add_before(P3, S::LONG_EDGE_SPLITTER)
                    .add_before(P3, S::PORT_LIST_SORTER)
                    .add_before(P4, S::IN_LAYER_CONSTRAINT_PROCESSOR)
                    .add_after(P5, S::LONG_EDGE_JOINER);
            }
            PhaseStep::NodePlacement(_) => {
                // Hierarchical ports add one processor, only for graphs with external ports.
                if graph
                    .graph_properties
                    .contains(&GraphProperties::EXTERNAL_PORTS)
                {
                    return Err(LayoutError::Unsupported(
                        "graphs with external ports are not ported yet".to_string(),
                    ));
                }
                return Ok(None);
            }
            PhaseStep::EdgeRouting(_) => {
                // The router adds processors for hyperedges, self loops, labels and ports.
                // Graphs with any of those are not ported yet; see `GraphConfigurator`.
                return Ok(None);
            }
        }
        Ok(Some(config))
    }
}

/// The cycle breaking of phase 1. Without edges there are no cycles, whatever the strategy;
/// with edges only the depth-first breaker is ported.
fn cycle_breaking(graph: &mut LGraph, strategy: CycleBreakingStrategy) -> Result<(), LayoutError> {
    if graph.edges.is_empty() {
        return Ok(());
    }
    match strategy {
        CycleBreakingStrategy::DEPTH_FIRST => {
            p1cycles::break_cycles(graph);
            Ok(())
        }
        other => Err(LayoutError::Unsupported(format!(
            "cycle breaking {} is not ported yet",
            other.java_name()
        ))),
    }
}

/// `crossMinType` of `LayerSweepCrossingMinimizer` for graphs without hierarchical
/// ports; the other heuristics need options Mermaid never sets.
const CROSS_MIN_TYPE: &str = "BARYCENTER";
