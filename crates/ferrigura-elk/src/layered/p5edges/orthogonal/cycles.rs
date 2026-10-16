//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p5edges/orthogonal/HyperEdgeCycleDetector.java
//!
//! Finds the dependencies to remove or reverse so that the dependency graph of the hyperedge
//! segments becomes acyclic, after Eades, Lin and Smyth (1993), with the linear ordering that
//! `computeLinearOrderingMarks` builds.
//!
//! Java's `Sets.newTreeSet(segments)` orders the unprocessed segments by `mark`. Marks are
//! distinct when the set is built, so a `BTreeMap` keyed by mark is the same set, and it removes
//! and iterates in the same order.

use std::collections::{BTreeMap, VecDeque};

use crate::java::random::Random;
use crate::layered::LayoutError;

use super::segment::{Arena, DependencyType};

/// `HyperEdgeCycleDetector.detectCycles`: the dependencies that point left against the linear
/// ordering. With `critical_only`, only critical dependencies are considered.
pub fn detect_cycles(
    arena: &mut Arena,
    segments: &[usize],
    critical_only: bool,
    random: &mut Option<Random>,
) -> Result<Vec<usize>, LayoutError> {
    let mut result = Vec::new();
    let mut sources = VecDeque::new();
    let mut sinks = VecDeque::new();

    initialize(arena, segments, &mut sources, &mut sinks, critical_only);
    compute_linear_ordering_marks(
        arena,
        segments,
        &mut sources,
        &mut sinks,
        critical_only,
        random,
    )?;

    // Process the edges that point left: those of zero weight are removed, the others reversed.
    for &source in segments {
        for dependency in arena.segments[source].outgoing_deps.clone() {
            let data = arena.dependencies[dependency];
            // Only consider critical dependencies, if required.
            if critical_only && data.kind != DependencyType::Critical {
                continue;
            }
            let Some(target) = data.target else { continue };
            if arena.segments[source].mark > arena.segments[target].mark {
                result.push(dependency);
            }
        }
    }
    Ok(result)
}

/// `initialize`: marks each segment from -1 down, sums the weights, and lists the sources (no
/// incoming weight) and the sinks (no outgoing weight).
fn initialize(
    arena: &mut Arena,
    segments: &[usize],
    sources: &mut VecDeque<usize>,
    sinks: &mut VecDeque<usize>,
    critical_only: bool,
) {
    let mut next_mark = -1;
    for &segment in segments {
        arena.segments[segment].mark = next_mark;
        next_mark -= 1;

        let critical_in_weight = sum_weights(arena, &arena.segments[segment].incoming_deps, true);
        let critical_out_weight = sum_weights(arena, &arena.segments[segment].outgoing_deps, true);

        // If we're only considering critical dependencies, we'll ignore the others.
        let mut in_weight = critical_in_weight;
        let mut out_weight = critical_out_weight;
        if !critical_only {
            // Just sum up everything.
            in_weight = sum_weights(arena, &arena.segments[segment].incoming_deps, false);
            out_weight = sum_weights(arena, &arena.segments[segment].outgoing_deps, false);
        }

        let data = &mut arena.segments[segment];
        data.in_weight = in_weight;
        data.critical_in_weight = critical_in_weight;
        data.out_weight = out_weight;
        data.critical_out_weight = critical_out_weight;

        // Add the segment to either sources or sinks if the corresponding weight is zero.
        if out_weight == 0 {
            sinks.push_back(segment);
        } else if in_weight == 0 {
            sources.push_back(segment);
        }
    }
}

/// The sum of the weights of the dependencies, or of the critical ones only.
fn sum_weights(arena: &Arena, dependencies: &[usize], critical_only: bool) -> i32 {
    dependencies
        .iter()
        .map(|&dependency| &arena.dependencies[dependency])
        .filter(|dependency| !critical_only || dependency.kind == DependencyType::Critical)
        .map(|dependency| dependency.weight)
        .sum()
}

/// `computeLinearOrderingMarks`: sinks get the marks below the base, sources the marks above
/// it, and the segments with the highest outflow are taken at random when neither is left.
fn compute_linear_ordering_marks(
    arena: &mut Arena,
    segments: &[usize],
    sources: &mut VecDeque<usize>,
    sinks: &mut VecDeque<usize>,
    critical_only: bool,
    random: &mut Option<Random>,
) -> Result<(), LayoutError> {
    // Ordered by mark, as Java's TreeSet is; the marks are distinct here.
    let mut unprocessed: BTreeMap<i32, usize> = segments
        .iter()
        .map(|&segment| (arena.segments[segment].mark, segment))
        .collect();
    let mut max_segments: Vec<usize> = Vec::new();

    // Sinks get marks below `mark_base` and sources marks above it. Sink marks are offset later,
    // so they end up above the source marks; this keeps the order in which sinks were found.
    let mark_base = segments.len() as i32;
    let mut next_sink_mark = mark_base - 1;
    let mut next_source_mark = mark_base + 1;

    while !unprocessed.is_empty() {
        while let Some(sink) = sinks.pop_front() {
            unprocessed.remove(&arena.segments[sink].mark);
            arena.segments[sink].mark = next_sink_mark;
            next_sink_mark -= 1;
            update_neighbors(arena, sink, sources, sinks, critical_only);
        }

        while let Some(source) = sources.pop_front() {
            unprocessed.remove(&arena.segments[source].mark);
            arena.segments[source].mark = next_source_mark;
            next_source_mark += 1;
            update_neighbors(arena, source, sources, sinks, critical_only);
        }

        // Neither sources nor sinks are left: take the segments with the highest out flow.
        // With both kinds of dependencies, a segment with a critical outgoing and no critical
        // incoming dependency is taken alone, so critical dependencies keep pointing right.
        let mut max_outflow = i32::MIN;
        for &segment in unprocessed.values() {
            let data = &arena.segments[segment];
            if !critical_only && data.critical_out_weight > 0 && data.critical_in_weight <= 0 {
                max_segments.clear();
                max_segments.push(segment);
                break;
            }

            let outflow = data.out_weight - data.in_weight;
            if outflow >= max_outflow {
                if outflow > max_outflow {
                    max_segments.clear();
                    max_outflow = outflow;
                }
                max_segments.push(segment);
            }
        }

        // Several segments may have the maximal out flow; one is chosen at random, which may
        // yield new sources and sinks.
        if !max_segments.is_empty() {
            let pick = next_int(random, max_segments.len())?;
            let max_node = max_segments[pick];
            unprocessed.remove(&arena.segments[max_node].mark);
            arena.segments[max_node].mark = next_source_mark;
            next_source_mark += 1;
            update_neighbors(arena, max_node, sources, sinks, critical_only);
            max_segments.clear();
        }
    }

    // Shift the marks below the base so that sinks have higher marks than sources.
    let shift_base = segments.len() as i32 + 1;
    for &node in segments {
        if arena.segments[node].mark < mark_base {
            arena.segments[node].mark += shift_base;
        }
    }
    Ok(())
}

/// `updateNeighbors`: simulates the removal of `node`, lowering the weights of its neighbours
/// that are still unprocessed, and queues the ones that became sources or sinks.
fn update_neighbors(
    arena: &mut Arena,
    node: usize,
    sources: &mut VecDeque<usize>,
    sinks: &mut VecDeque<usize>,
    critical_only: bool,
) {
    // Process the following nodes.
    for dependency in arena.segments[node].outgoing_deps.clone() {
        let data = arena.dependencies[dependency];
        if critical_only && data.kind != DependencyType::Critical {
            continue;
        }
        let Some(target) = data.target else { continue };
        if arena.segments[target].mark < 0 && data.weight > 0 {
            // Remove weight (and possibly critical weight) from the target.
            let target_data = &mut arena.segments[target];
            target_data.in_weight -= data.weight;
            if data.kind == DependencyType::Critical {
                target_data.critical_in_weight -= data.weight;
            }
            if target_data.in_weight <= 0 && target_data.out_weight > 0 {
                sources.push_back(target);
            }
        }
    }

    // Process the preceding nodes.
    for dependency in arena.segments[node].incoming_deps.clone() {
        let data = arena.dependencies[dependency];
        if critical_only && data.kind != DependencyType::Critical {
            continue;
        }
        let Some(source) = data.source else { continue };
        if arena.segments[source].mark < 0 && data.weight > 0 {
            // Remove weight (and possibly critical weight) from the source.
            let source_data = &mut arena.segments[source];
            source_data.out_weight -= data.weight;
            if data.kind == DependencyType::Critical {
                source_data.critical_out_weight -= data.weight;
            }
            if source_data.out_weight <= 0 && source_data.in_weight > 0 {
                sinks.push_back(source);
            }
        }
    }
}

/// `random.nextInt(bound)`. A graph without a random source fails here; Java would fail with a
/// null pointer at the same point, so the port reports it as unsupported.
fn next_int(random: &mut Option<Random>, bound: usize) -> Result<usize, LayoutError> {
    let random = random.as_mut().ok_or_else(|| {
        LayoutError::Unsupported("edge routing without a random source".to_string())
    })?;
    Ok(random.next_int_bounded(bound as i32) as usize)
}
