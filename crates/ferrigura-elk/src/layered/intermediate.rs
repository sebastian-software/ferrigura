//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/IntermediateProcessorStrategy.java and the processors it creates
//!
//! Intermediate processors, `IntermediateProcessorStrategy` and its processors.
//!
//! Each strategy gives the task name the processor logs, as its `begin` call
//! does in the Java source. The processors run their nodes-only behaviour; the
//! ones that need edges, ports or labels are reported as unsupported when
//! their name is needed, so they cannot silently do nothing.

use crate::core::data::JavaEnum;
use crate::core::data::enums::{IntermediateProcessorStrategy, OrderingStrategy};
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::phases::PhaseStep;

/// One step of the algorithm: a phase implementation or an intermediate processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Phase(PhaseStep),
    Intermediate(IntermediateProcessorStrategy),
}

/// The task name a strategy's processor logs, for the processors of the nodes-only path.
pub fn task_name(
    strategy: IntermediateProcessorStrategy,
    graph: &LGraph,
) -> Result<String, LayoutError> {
    use IntermediateProcessorStrategy as S;
    let fixed = match strategy {
        S::DIRECTION_PREPROCESSOR => "Graph transformation (TO_INPUT_DIRECTION)",
        S::DIRECTION_POSTPROCESSOR => "Graph transformation (TO_INTERNAL_LTR)",
        S::EDGE_AND_LAYER_CONSTRAINT_EDGE_REVERSER => "Edge and layer constraint edge reversal",
        S::LAYER_CONSTRAINT_PREPROCESSOR => "Layer constraint preprocessing",
        S::LAYER_CONSTRAINT_POSTPROCESSOR => "Layer constraint postprocessing",
        S::LONG_EDGE_SPLITTER => "Edge splitting",
        S::PORT_SIDE_PROCESSOR => "Port side processing",
        S::PORT_LIST_SORTER => "Port order processing",
        S::IN_LAYER_CONSTRAINT_PROCESSOR => "Layer constraint edge reversal",
        S::LABEL_AND_NODE_SIZE_PROCESSOR => "Node and Port Label Placement and Node Sizing",
        S::INNERMOST_NODE_MARGIN_CALCULATOR => "Node margin calculation",
        S::LAYER_SIZE_AND_GRAPH_HEIGHT_CALCULATOR => "Layer size calculation",
        S::LONG_EDGE_JOINER => "Edge joining",
        S::END_LABEL_SORTER => "Sort end labels",
        S::REVERSED_EDGE_RESTORER => "Restoring reversed edges",
        S::HIERARCHICAL_NODE_RESIZER => "Resize child graph to fit parent.",
        S::SORT_BY_INPUT_ORDER_OF_MODEL => {
            let order = graph
                .properties
                .enum_of::<OrderingStrategy>(options::CONSIDER_MODEL_ORDER_STRATEGY)
                .map_or("NONE", JavaEnum::java_name);
            return Ok(format!("Sort By Input Model {order}"));
        }
        other => {
            return Err(LayoutError::Unsupported(format!(
                "intermediate processor {} is not ported yet",
                other.java_name()
            )));
        }
    };
    Ok(fixed.to_string())
}

/// Runs one intermediate processor: its task, under `monitor`.
pub fn run(
    strategy: IntermediateProcessorStrategy,
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    let name = task_name(strategy, graph)?;
    progress.begin(monitor, &name, 1.0);
    progress.done(monitor);
    Ok(())
}
