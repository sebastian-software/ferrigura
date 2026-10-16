//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/RecursiveGraphLayoutEngine.java
//!
//! `RecursiveGraphLayoutEngine`: runs the algorithm of each level of the graph. The children of a
//! level are laid out before the level, so that each child's size is known to the level above.
//!
//! A level is laid out by the layered algorithm. A child whose hierarchy handling is
//! `SEPARATE_CHILDREN` is a level of its own. Under a level whose hierarchy handling is
//! `INCLUDE_CHILDREN`, the other descendants join the level's compound layout, with their children.
//! The hierarchy handling `INHERIT` takes the value of the parent, and the root's value is
//! `SEPARATE_CHILDREN`.
//!
//! Not ported: the layout algorithm resolver (each level must name `elk.layered`), inside self
//! loops, topdown layout and the scaling factors.

use std::collections::VecDeque;

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
        // The descendants of a compound level, breadth first: those that stop the hierarchy are
        // laid out on their own, first, and the children of the others join this level.
        let mut queue: VecDeque<Vec<usize>> = (0..node.children.len()).map(|i| vec![i]).collect();
        while let Some(path) = queue.pop_front() {
            let (parent_properties, parent_hierarchy, child_chain) = {
                let (parent, child) = parent_and_child(node, &path)?;
                let parent_hierarchy = parent
                    .properties
                    .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING);
                (
                    parent.properties.clone(),
                    parent_hierarchy,
                    chain_of(chain, node, &path, child),
                )
            };
            let child = child_at_mut(node, &path)?;
            let child_hierarchy = resolve_hierarchy(&mut child.properties, parent_hierarchy);
            // A child that names another algorithm is laid out on its own as well, and refused there.
            let other_algorithm = child.properties.has(options::ALGORITHM)
                && require_layered(&child.properties).is_err();
            if child_hierarchy == HierarchyHandling::SEPARATE_CHILDREN || other_algorithm {
                layout_recursively(
                    child,
                    Some(&parent_properties),
                    parent_hierarchy,
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
            } else {
                for index in 0..child.children.len() {
                    let mut child_path = path.clone();
                    child_path.push(index);
                    queue.push_back(child_path);
                }
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

/// The node at `path` below `node`, with its parent.
fn parent_and_child<'a>(
    node: &'a ElkNode,
    path: &[usize],
) -> Result<(&'a ElkNode, &'a ElkNode), LayoutError> {
    let mut parent = node;
    let Some((&last, ancestors)) = path.split_last() else {
        return Err(LayoutError::Format("an empty node path".to_string()));
    };
    for &index in ancestors {
        parent = parent.children.get(index).ok_or_else(outside)?;
    }
    let child = parent.children.get(last).ok_or_else(outside)?;
    Ok((parent, child))
}

/// The node at `path` below `node`, to change.
fn child_at_mut<'a>(node: &'a mut ElkNode, path: &[usize]) -> Result<&'a mut ElkNode, LayoutError> {
    let mut current = node;
    for &index in path {
        current = current.children.get_mut(index).ok_or_else(outside)?;
    }
    Ok(current)
}

fn outside() -> LayoutError {
    LayoutError::Format("a node path outside the graph".to_string())
}

/// The designation prefix of the node `child` at `path` below `node`: `chain`, then the ids of the
/// nodes on the path.
fn chain_of(chain: &str, node: &ElkNode, path: &[usize], child: &ElkNode) -> String {
    let mut result = chain.to_string();
    let mut current = node;
    for &index in &path[..path.len().saturating_sub(1)] {
        if let Some(next) = current.children.get(index) {
            result.push('.');
            result.push_str(&next.id);
            current = next;
        }
    }
    result.push('.');
    result.push_str(&child.id);
    result
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
