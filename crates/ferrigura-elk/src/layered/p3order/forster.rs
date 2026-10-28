//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/ForsterConstraintResolver.java
//!
//! `ForsterConstraintResolver`: once the barycenter heuristic has sorted a layer, the nodes that
//! must come before others (`IN_LAYER_SUCCESSOR_CONSTRAINTS`) and the layout units of the north/south
//! port dummies (`IN_LAYER_LAYOUT_UNIT`) are put in order. Groups of nodes that violate a constraint
//! merge into one group with the mean of their barycenters, until no constraint is violated; the
//! layer then takes the groups' order and each node its group's barycenter.
//!
//! A group of one node has no weight and no degree in the Java (its constructor sets neither), so a
//! merged group always takes the mean of its two parts' barycenters. Each call starts from the
//! groups of one node that `initAtNodeLevel` made, whose state the Java resets before it reads it;
//! a fresh arena per call keeps that. `IN_LAYER_SUCCESSOR_CONSTRAINTS_BETWEEN_NON_DUMMIES` is set
//! only by the semi-interactive processor, which is not ported, so the pass between normal nodes
//! does not run.

use std::collections::HashMap;

use crate::core::data::enums::NodeType;
use crate::layered::LayoutError;
use crate::layered::p3order::{Layout, Net};

use super::barycenter::BarycenterState;

/// The resolver's `layoutUnits`: the nodes of each layout unit, by the node that names the unit, in
/// the order of the layers when the sweep starts (`LinkedHashMultimap`).
pub(super) struct ForsterConstraintResolver {
    layout_units: HashMap<usize, Vec<usize>>,
}

/// `ConstraintGroup`. Groups are kept in an arena and named by their index in it.
struct ConstraintGroup {
    nodes: Vec<usize>,
    summed_weight: f64,
    degree: i32,
    /// `outgoingConstraints`, `None` for the Java's `null`.
    outgoing: Option<Vec<usize>>,
    /// `incomingConstraints`, `None` for the Java's `null`.
    incoming: Option<Vec<usize>>,
    incoming_count: i32,
}

impl ConstraintGroup {
    fn single(node: usize) -> Self {
        ConstraintGroup {
            nodes: vec![node],
            summed_weight: 0.0,
            degree: 0,
            outgoing: None,
            incoming: None,
            incoming_count: 0,
        }
    }

    fn has_outgoing(&self) -> bool {
        self.outgoing.as_ref().is_some_and(|list| !list.is_empty())
    }

    fn has_incoming(&self) -> bool {
        self.incoming.as_ref().is_some_and(|list| !list.is_empty())
    }
}

/// The groups of one `processConstraints` call, with the barycenter states they read and write.
struct Groups<'s> {
    arena: Vec<ConstraintGroup>,
    /// `constraintGroups[layer][node]`: the group of one node, made when first asked for.
    of_node: HashMap<usize, usize>,
    states: &'s mut [BarycenterState],
}

impl Groups<'_> {
    /// `groupOf(node)`.
    fn group_of(&mut self, node: usize) -> usize {
        if let Some(&group) = self.of_node.get(&node) {
            return group;
        }
        self.arena.push(ConstraintGroup::single(node));
        let group = self.arena.len() - 1;
        self.of_node.insert(node, group);
        group
    }

    /// `getBarycenter()`: the barycenter of the group's first node.
    fn barycenter(&self, group: usize) -> Option<f64> {
        self.arena[group]
            .nodes
            .first()
            .and_then(|&node| self.states[node].barycenter)
    }

    /// `getBarycenter()` where the Java unboxes it, which fails for `null`.
    fn known_barycenter(&self, group: usize) -> Result<f64, LayoutError> {
        self.barycenter(group).ok_or_else(|| {
            LayoutError::Format(
                "a constraint group without a barycenter (ForsterConstraintResolver)".to_string(),
            )
        })
    }

    /// `setBarycenter`: every node of the group takes the value.
    fn set_barycenter(&mut self, group: usize, value: Option<f64>) {
        for index in 0..self.arena[group].nodes.len() {
            let node = self.arena[group].nodes[index];
            self.states[node].barycenter = value;
        }
    }

    /// `getOutgoingConstraints().add(target)`, counting the target's incoming constraint.
    fn add_constraint(&mut self, from: usize, to: usize) {
        self.arena[from]
            .outgoing
            .get_or_insert_with(Vec::new)
            .push(to);
        self.arena[to].incoming_count += 1;
    }

    /// `new ConstraintGroup(nodeGroup1, nodeGroup2)`: the concatenation of the two groups, with
    /// their merged successors and the barycenter derived from theirs.
    fn merge(&mut self, first: usize, second: usize) -> usize {
        let mut nodes = self.arena[first].nodes.clone();
        nodes.extend(self.arena[second].nodes.iter().copied());
        let outgoing = if let Some(first_outgoing) = self.arena[first].outgoing.clone() {
            let mut outgoing = first_outgoing;
            remove_first(&mut outgoing, second);
            if let Some(second_outgoing) = self.arena[second].outgoing.clone() {
                for candidate in second_outgoing {
                    if candidate == first {
                        continue;
                    } else if outgoing.contains(&candidate) {
                        // Both groups were predecessors of the candidate.
                        self.arena[candidate].incoming_count -= 1;
                    } else {
                        outgoing.push(candidate);
                    }
                }
            }
            Some(outgoing)
        } else if let Some(second_outgoing) = self.arena[second].outgoing.clone() {
            let mut outgoing = second_outgoing;
            remove_first(&mut outgoing, first);
            Some(outgoing)
        } else {
            None
        };
        let summed_weight = self.arena[first].summed_weight + self.arena[second].summed_weight;
        let degree = self.arena[first].degree + self.arena[second].degree;
        let (first_barycenter, second_barycenter) =
            (self.barycenter(first), self.barycenter(second));
        self.arena.push(ConstraintGroup {
            nodes,
            summed_weight,
            degree,
            outgoing,
            incoming: None,
            incoming_count: 0,
        });
        let merged = self.arena.len() - 1;
        let barycenter = if degree > 0 {
            Some(summed_weight / f64::from(degree))
        } else {
            match (first_barycenter, second_barycenter) {
                (Some(first), Some(second)) => Some((first + second) / 2.0),
                (Some(first), None) => Some(first),
                (None, Some(second)) => Some(second),
                (None, None) => None,
            }
        };
        if barycenter.is_some() {
            self.set_barycenter(merged, barycenter);
        }
        merged
    }
}

impl ForsterConstraintResolver {
    /// `new ForsterConstraintResolver` with its `initAtNodeLevel` calls: the members of each layout
    /// unit, in the order of the layers.
    pub(super) fn new(net: &Net, layout: &Layout) -> Self {
        let mut layout_units: HashMap<usize, Vec<usize>> = HashMap::new();
        for &node in layout.order.iter().flatten() {
            if let Some(unit) = net.graph().nodes[node].in_layer_layout_unit {
                let members = layout_units.entry(unit).or_default();
                if !members.contains(&node) {
                    members.push(node);
                }
            }
        }
        ForsterConstraintResolver { layout_units }
    }

    /// `processConstraints(nodes)`: the layer's nodes, sorted by barycenter, in the order that keeps
    /// the constraints; each node takes its group's barycenter.
    pub(super) fn process_constraints(
        &self,
        net: &Net,
        nodes: &mut Vec<usize>,
        states: &mut [BarycenterState],
    ) -> Result<(), LayoutError> {
        let mut groups = Groups {
            arena: Vec::with_capacity(nodes.len()),
            of_node: HashMap::with_capacity(nodes.len()),
            states,
        };
        let mut list: Vec<usize> = nodes.iter().map(|&node| groups.group_of(node)).collect();
        self.build_constraints_graph(net, &mut groups, &list);
        while let Some((first, second)) = find_violated_constraint(&mut groups, &list)? {
            handle_violated_constraint(&mut groups, first, second, &mut list)?;
        }
        nodes.clear();
        for group in list {
            let barycenter = groups.barycenter(group);
            for index in 0..groups.arena[group].nodes.len() {
                let node = groups.arena[group].nodes[index];
                nodes.push(node);
                groups.states[node].barycenter = barycenter;
            }
        }
        Ok(())
    }

    /// `buildConstraintsGraph(groups, false)`: the successor constraints of each node, and between
    /// the layout units of consecutive normal nodes, each node of the earlier unit before each node of
    /// the later one.
    fn build_constraints_graph(&self, net: &Net, groups: &mut Groups, list: &[usize]) {
        for &group in list {
            groups.arena[group].outgoing = None;
            groups.arena[group].incoming_count = 0;
        }
        let mut last_non_dummy: Option<usize> = None;
        for &group in list {
            let node = groups.arena[group].nodes[0];
            for &successor in &net.graph().nodes[node].in_layer_successor_constraints {
                let target = groups.group_of(successor);
                groups.add_constraint(group, target);
            }
            if net.node_type(node) == NodeType::NORMAL {
                if let Some(last) = last_non_dummy {
                    for &last_unit_node in self.units(last) {
                        for &current_unit_node in self.units(node) {
                            let from = groups.group_of(last_unit_node);
                            let to = groups.group_of(current_unit_node);
                            groups.add_constraint(from, to);
                        }
                    }
                }
                last_non_dummy = Some(node);
            }
        }
    }

    /// `layoutUnits.get(node)`.
    fn units(&self, node: usize) -> &[usize] {
        self.layout_units.get(&node).map_or(&[], Vec::as_slice)
    }
}

/// `findViolatedConstraint`: walks the constraint graph from the groups without predecessors, in
/// the order of the list; a group whose predecessor has a larger barycenter (or an equal one and a
/// later place) violates a constraint.
fn find_violated_constraint(
    groups: &mut Groups,
    list: &[usize],
) -> Result<Option<(usize, usize)>, LayoutError> {
    let mut active: Option<Vec<usize>> = None;
    for &group in list {
        groups.arena[group].incoming = None;
        if groups.arena[group].has_outgoing() && groups.arena[group].incoming_count == 0 {
            active.get_or_insert_with(Vec::new).push(group);
        }
    }
    let Some(mut active) = active else {
        return Ok(None);
    };
    let index_of = |group: usize| -> i64 {
        list.iter()
            .position(|&found| found == group)
            .map_or(-1, |index| index as i64)
    };
    while !active.is_empty() {
        let group = active.remove(0);
        if groups.arena[group].has_incoming() {
            let incoming = groups.arena[group].incoming.clone().unwrap_or_default();
            let barycenter = groups.known_barycenter(group)?;
            for predecessor in incoming {
                let predecessor_barycenter = groups.known_barycenter(predecessor)?;
                // `floatValue()` on both sides of the equality.
                if predecessor_barycenter as f32 == barycenter as f32 {
                    if index_of(predecessor) > index_of(group) {
                        return Ok(Some((predecessor, group)));
                    }
                } else if predecessor_barycenter > barycenter {
                    return Ok(Some((predecessor, group)));
                }
            }
        }
        let outgoing = groups.arena[group].outgoing.clone().unwrap_or_default();
        for successor in outgoing {
            let target = &mut groups.arena[successor];
            let incoming = target.incoming.get_or_insert_with(Vec::new);
            incoming.insert(0, group);
            if target.incoming_count == incoming.len() as i32 {
                active.push(successor);
            }
        }
    }
    Ok(None)
}

/// `handleViolatedConstraint`: the two groups merge; the new group takes the place before the first
/// group with a larger barycenter, and the constraints that pointed to either part point to it.
fn handle_violated_constraint(
    groups: &mut Groups,
    first: usize,
    second: usize,
    list: &mut Vec<usize>,
) -> Result<(), LayoutError> {
    let merged = groups.merge(first, second);
    let merged_barycenter = groups.known_barycenter(merged)?;
    let mut already_inserted = false;
    let mut index = 0;
    while index < list.len() {
        let group = list[index];
        if group == first || group == second {
            list.remove(index);
            continue;
        } else if !already_inserted && groups.known_barycenter(group)? > merged_barycenter {
            // The iterator steps back and adds before the group, which it then visits again.
            list.insert(index, merged);
            already_inserted = true;
        } else if groups.arena[group].has_outgoing() {
            let outgoing = groups.arena[group].outgoing.get_or_insert_with(Vec::new);
            let first_removed = remove_first(outgoing, first);
            let second_removed = remove_first(outgoing, second);
            if first_removed || second_removed {
                outgoing.push(merged);
                groups.arena[merged].incoming_count += 1;
            }
        }
        index += 1;
    }
    if !already_inserted {
        list.push(merged);
    }
    Ok(())
}

/// `List.remove(Object)`: the first occurrence leaves the list; whether there was one.
fn remove_first(list: &mut Vec<usize>, item: usize) -> bool {
    match list.iter().position(|&found| found == item) {
        Some(index) => {
            list.remove(index);
            true
        }
        None => false,
    }
}
