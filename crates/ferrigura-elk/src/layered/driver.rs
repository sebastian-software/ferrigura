//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/ElkLayered.java
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/LayeredLayoutProvider.java
//!
//! `LayeredLayoutProvider` and `ElkLayered`: the driver of the layered algorithm.

use crate::core::data::enums::HierarchyHandling;
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::error::LayoutError;
use crate::graph::ElkNode;
use crate::layered::configurator;
use crate::layered::importer;
use crate::layered::intermediate::{self, Step};
use crate::layered::lgraph::LGraph;
use crate::layered::options;

/// `LayeredLayoutProvider.layout(elkgraph, monitor)` for one root graph.
pub fn layout(
    root: &ElkNode,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    let mut graph = importer::import_root(root)?;
    let hierarchy = root
        .properties
        .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING);
    if hierarchy != Some(HierarchyHandling::INCLUDE_CHILDREN) {
        return Err(LayoutError::Unsupported(
            "a root without INCLUDE_CHILDREN (doLayout and the components processor are not ported yet)"
                .to_string(),
        ));
    }
    do_compound_layout(&mut graph, progress, monitor)
}

/// `ElkLayered.doCompoundLayout`.
fn do_compound_layout(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    progress.begin(monitor, "Layered layout", 2.0);
    let pre = sub_task(progress, monitor)?;
    progress.begin(pre, "Compound graph preprocessor", 1.0);
    progress.done(pre);
    let hierarchical = sub_task(progress, monitor)?;
    hierarchical_layout(graph, progress, hierarchical)?;
    let post = sub_task(progress, monitor)?;
    progress.begin(post, "Compound graph postprocessor", 1.0);
    progress.done(post);
    progress.done(monitor);
    Ok(())
}

/// `ElkLayered.hierarchicalLayout` for a root without nested graphs.
fn hierarchical_layout(
    graph: &mut LGraph,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    configurator::prepare(graph)?;
    let steps: Vec<Step> = graph.processors.clone();
    progress.begin(monitor, "Recursive hierarchical layout", steps.len() as f64);
    for step in steps {
        let sub = sub_task(progress, monitor)?;
        match step {
            Step::Intermediate(strategy) => intermediate::run(strategy, graph, progress, sub)?,
            Step::Phase(phase) => phase.run(graph, progress, sub)?,
        }
    }
    progress.done(monitor);
    Ok(())
}

fn sub_task(progress: &mut ProgressMonitor, monitor: Monitor) -> Result<Monitor, LayoutError> {
    progress
        .sub_task(monitor, 1.0)
        .ok_or_else(|| LayoutError::Unsupported("subtask of a finished monitor".to_string()))
}
