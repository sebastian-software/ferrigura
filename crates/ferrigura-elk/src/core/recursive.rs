//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/RecursiveGraphLayoutEngine.java
//!
//! `RecursiveGraphLayoutEngine`: runs the algorithm of each level of the graph.
//!
//! Only the root level is laid out so far, with the layered algorithm.
//! Nested graphs and the other algorithms are reported as unsupported.

use crate::core::data::OptionValue;
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::error::LayoutError;
use crate::graph::ElkNode;
use crate::layered::driver;
use crate::layered::options;

/// `layout(layoutGraph, progressMonitor)`: the monitor's root task is "Recursive Graph Layout".
pub fn layout(
    root: &ElkNode,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    progress.begin(monitor, "Recursive Graph Layout", 1.0);
    if !root.children.is_empty() && root.children.iter().any(|child| !child.children.is_empty()) {
        return Err(LayoutError::Unsupported(
            "nested graphs (RecursiveGraphLayoutEngine)".to_string(),
        ));
    }
    let algorithm = match root.properties.value_or_default(options::ALGORITHM) {
        Some(OptionValue::Algorithm(name)) => name,
        _ => {
            return Err(LayoutError::Unsupported(
                "a root without elk.algorithm (LayoutAlgorithmResolver is not ported yet)"
                    .to_string(),
            ));
        }
    };
    if algorithm != "elk.layered" && algorithm != "org.eclipse.elk.layered" {
        return Err(LayoutError::Unsupported(format!("algorithm {algorithm}")));
    }
    let sub = progress
        .sub_task(monitor, 1.0)
        .ok_or_else(|| LayoutError::Unsupported("subtask of a finished monitor".to_string()))?;
    driver::layout(root, progress, sub)?;
    progress.done(monitor);
    Ok(())
}
