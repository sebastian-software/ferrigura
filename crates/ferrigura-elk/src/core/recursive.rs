//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/RecursiveGraphLayoutEngine.java
//!
//! `RecursiveGraphLayoutEngine`: runs the algorithm of each level of the graph. The children of a
//! level are laid out before the level, so that each child's size is known to the level above.
//!
//! A level is laid out by the layered algorithm. A child whose hierarchy handling is
//! `SEPARATE_CHILDREN` is a level of its own; a child whose children would join the level above
//! (a nested graph inside a compound layout) is not ported yet. The hierarchy handling `INHERIT`
//! takes the value of the parent, and the root's value is `SEPARATE_CHILDREN`.
//!
//! Not ported: the layout algorithm resolver (each level must name `elk.layered`), inside self
//! loops, topdown layout and the scaling factors.

use crate::core::data::enums::HierarchyHandling;
use crate::core::data::property::PropertyHolder;
use crate::core::data::{JavaEnum, OptionValue};
use crate::core::util::progress::{Monitor, ProgressMonitor};
use crate::error::LayoutError;
use crate::graph::ElkNode;
use crate::layered::driver;
use crate::layered::options;

/// `layout(layoutGraph, progressMonitor)`: the monitor's root task is "Recursive Graph Layout".
pub fn layout(
    root: &mut ElkNode,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    progress.begin(monitor, "Recursive Graph Layout", 1.0);
    let chain = root.id.clone();
    layout_recursively(root, None, None, &chain, progress, monitor)?;
    progress.done(monitor);
    Ok(())
}

/// `layoutRecursively`: lays out the children of `node` that are laid out on their own, then the
/// level of `node`. `parent` holds the options and the hierarchy handling of the node's parent, and
/// `chain` is the node's designation prefix. A node without children has no level to lay out.
fn layout_recursively(
    node: &mut ElkNode,
    parent: Option<&PropertyHolder>,
    parent_hierarchy: Option<HierarchyHandling>,
    chain: &str,
    progress: &mut ProgressMonitor,
    monitor: Monitor,
) -> Result<(), LayoutError> {
    if node.properties.bool_of(options::NO_LAYOUT) || node.children.is_empty() {
        return Ok(());
    }
    let hierarchy = resolve_hierarchy(&mut node.properties, parent_hierarchy);
    require_layered(&node.properties)?;
    if hierarchy == HierarchyHandling::INCLUDE_CHILDREN {
        // The children of a compound level: those that stop the hierarchy are laid out on their
        // own, first, and the others would join this level.
        for child in node.children.iter_mut() {
            let child_hierarchy = resolve_hierarchy(&mut child.properties, Some(hierarchy));
            if child_hierarchy == HierarchyHandling::SEPARATE_CHILDREN {
                let child_chain = format!("{chain}.{}", child.id);
                layout_recursively(
                    child,
                    Some(&node.properties),
                    Some(hierarchy),
                    &child_chain,
                    progress,
                    monitor,
                )?;
                // The child is laid out now: the level above includes it as a plain node.
                child.properties.set(
                    options::HIERARCHY_HANDLING,
                    Some(OptionValue::Enum {
                        class: "HierarchyHandling",
                        name: HierarchyHandling::SEPARATE_CHILDREN.java_name().to_string(),
                    }),
                );
            } else if !child.children.is_empty() {
                return Err(LayoutError::Unsupported(format!(
                    "nested graph in {} inside a compound layout",
                    child.id
                )));
            }
        }
    } else {
        for child in node.children.iter_mut() {
            let child_chain = format!("{chain}.{}", child.id);
            layout_recursively(
                child,
                Some(&node.properties),
                Some(hierarchy),
                &child_chain,
                progress,
                monitor,
            )?;
        }
    }
    // `executeAlgorithm(layoutNode, ...)`: the level is laid out with its own children.
    let sub = progress
        .sub_task(monitor, 1.0)
        .ok_or_else(|| LayoutError::Unsupported("subtask of a finished monitor".to_string()))?;
    driver::layout(node, parent, chain, progress, sub)
}

/// `evaluateHierarchyHandlingInheritance`: the node's hierarchy handling, with INHERIT replaced by
/// the parent's value, or by SEPARATE_CHILDREN for a root. The value is written back to the node.
fn resolve_hierarchy(
    properties: &mut PropertyHolder,
    parent: Option<HierarchyHandling>,
) -> HierarchyHandling {
    let own = properties.enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING);
    let resolved = match own {
        Some(HierarchyHandling::INHERIT) | None => {
            parent.unwrap_or(HierarchyHandling::SEPARATE_CHILDREN)
        }
        Some(value) => value,
    };
    properties.set(
        options::HIERARCHY_HANDLING,
        Some(OptionValue::Enum {
            class: "HierarchyHandling",
            name: resolved.java_name().to_string(),
        }),
    );
    resolved
}

/// The algorithm of a level, which must be the layered one. The resolver that picks other
/// algorithms is not ported.
fn require_layered(properties: &PropertyHolder) -> Result<(), LayoutError> {
    match properties.value_or_default(options::ALGORITHM) {
        Some(OptionValue::Algorithm(name))
            if name == "elk.layered" || name == "org.eclipse.elk.layered" =>
        {
            Ok(())
        }
        Some(OptionValue::Algorithm(name)) => {
            Err(LayoutError::Unsupported(format!("algorithm {name}")))
        }
        _ => Err(LayoutError::Unsupported(
            "a level without elk.algorithm (LayoutAlgorithmResolver is not ported yet)".to_string(),
        )),
    }
}
