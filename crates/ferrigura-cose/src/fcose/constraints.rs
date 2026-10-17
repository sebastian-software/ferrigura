//! The constraint handling of cytoscape-fcose 2.2.0 on cose-base 2.2.0, as Mermaid's architecture
//! diagrams run it: `ConstraintHandler.handleConstraints` (`ConstraintHandler.js` 9-847), the
//! constraint variables of `CoSELayout.initConstraintVariables` (`CoSELayout.js` 324-587), and
//! `CoSELayout.updateDisplacements` (`CoSELayout.js` 588-801).
//!
//! Insertion order is part of the result. `JsMap` and `JsSet` are the JavaScript `Map` and `Set`:
//! their `forEach` and `for...of` visit the entries in insertion order, and the sums, the
//! first-match searches and the component numbering depend on it.
//!
//! A node id without an index reads as NaN, which is what `xCoords[undefined]` gives the source's
//! arithmetic, and a write to it does nothing, as a write to the array property "undefined" does.
//! Where the source dereferences a missing node (`idToNodeMap.get(id).getWidth()`) it throws, and
//! this port panics with the id.
//!
//! The transform of the draft (`ConstraintHandler.js` 438-609) runs when `randomize` is true: the
//! draft is fitted to the constrained nodes by the orthogonal Procrustes fit of `linalg.rs`, and
//! the relative placement of the largest component is applied on it.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::fcose::layout::{AlignmentConstraint, Layout, RelativeConstraint};
use crate::fcose::linalg::{dot_product, orthogonal_procrustes};
use crate::fcose::model::Model;
use crate::fcose::rng::Mulberry32;
use crate::geometry::{PointD, RectD};

/// `CoSEConstants.ENFORCE_CONSTRAINTS` (`CoSEConstants.js` 18). fcose's step 'all' keeps it true
/// (`cose.js` 233-239).
const ENFORCE_CONSTRAINTS: bool = true;

/// `CoSEConstants.RELAX_MOVEMENT_ON_CONSTRAINTS` (`CoSEConstants.js` 20). fcose does not set it.
const RELAX_MOVEMENT_ON_CONSTRAINTS: bool = true;

/// The name a relative-constraint error message gives.
const RELATIVE: &str = "relativePlacementConstraint";

/// One entry of fcose's `fixedNodeConstraint`, `{nodeId, position: {x, y}}`. Mermaid gives none
/// (`mod.rs` builds no fixed constraint), so `Layout.fixed_node_constraint` is `None` in every run.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FixedNodeConstraint {
    pub node_id: String,
    pub position: PointD,
}

/// The state `initConstraintVariables` sets on the layout (`this.idToNodeMap`, `this.fixedNodeSet`,
/// and the rest of `CoSELayout.js` 326-582) and `updateDisplacements` reads. Maps and sets keep
/// the source's insertion order.
#[derive(Clone, Debug, Default)]
pub(crate) struct ConstraintVars {
    /// `idToNodeMap`: every node, compounds included, by id (`CoSELayout.js` 326-335).
    id_to_node: HashMap<String, usize>,
    /// `fixedNodeSet` (`CoSELayout.js` 327, 358).
    fixed_node_set: JsSet,
    /// `dummyToNodeForVerticalAlignment` and `...Horizontal` (`CoSELayout.js` 379-380, 407-408).
    dummy_to_node_vertical: JsMap<Vec<String>>,
    dummy_to_node_horizontal: JsMap<Vec<String>>,
    /// `fixedNodesOnHorizontal` and `fixedNodesOnVertical` (`CoSELayout.js` 381-382).
    fixed_nodes_on_horizontal: JsSet,
    fixed_nodes_on_vertical: JsSet,
    /// `nodesInRelativeHorizontal` and `...Vertical` (`CoSELayout.js` 432-433), relaxed branch.
    nodes_in_relative_horizontal: Vec<String>,
    nodes_in_relative_vertical: Vec<String>,
    /// `nodeToRelativeConstraintMapHorizontal` and `...Vertical` (`CoSELayout.js` 434-435).
    node_to_relative_horizontal: JsMap<Vec<RelativeEdge>>,
    node_to_relative_vertical: JsMap<Vec<RelativeEdge>>,
    /// `nodeToTempPositionMapHorizontal` and `...Vertical` (`CoSELayout.js` 436-437).
    node_to_temp_position_horizontal: JsMap<f64>,
    node_to_temp_position_vertical: JsMap<f64>,
    /// `componentsOnHorizontal` and `fixedComponentsOnHorizontal` (and the vertical pair), the
    /// non-relaxed branch (`CoSELayout.js` 577-582).
    components_on_horizontal: Vec<Vec<String>>,
    fixed_components_on_horizontal: Vec<bool>,
    components_on_vertical: Vec<Vec<String>>,
    fixed_components_on_vertical: Vec<bool>,
}

/// `{right: id, gap}` or `{bottom: id, gap}` (`CoSELayout.js` 466, 493): the node that comes after
/// this one, on the horizontal or vertical axis.
#[derive(Clone, Debug)]
enum RelativeEdge {
    Forward {
        id: String,
        gap: f64,
    },
    /// `{left: id, gap}` or `{top: id, gap}` (`CoSELayout.js` 467, 494).
    Backward {
        id: String,
        gap: f64,
    },
}

/// `{id, gap, direction}` (`ConstraintHandler.js` 412-415, 423), one edge of a constraint graph.
#[derive(Clone, Debug)]
struct DagEdge {
    id: String,
    gap: f64,
    direction: Direction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Horizontal,
    Vertical,
}

/// A JavaScript `Map` with string keys. `set` on a present key keeps its place, and `keys` and
/// `iter` visit the entries in insertion order.
#[derive(Clone, Debug)]
struct JsMap<V> {
    entries: Vec<(String, V)>,
    index: HashMap<String, usize>,
}

impl<V> Default for JsMap<V> {
    fn default() -> Self {
        JsMap {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<V> JsMap<V> {
    fn set(&mut self, key: &str, value: V) {
        match self.index.get(key).copied() {
            Some(position) => self.entries[position].1 = value,
            None => {
                self.index.insert(key.to_string(), self.entries.len());
                self.entries.push((key.to_string(), value));
            }
        }
    }

    fn get(&self, key: &str) -> Option<&V> {
        self.index
            .get(key)
            .map(|&position| &self.entries[position].1)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let position = *self.index.get(key)?;
        Some(&mut self.entries[position].1)
    }

    fn has(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    /// `map.size`.
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(key, _)| key.as_str())
    }

    fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_str(), value))
    }
}

impl<T> JsMap<Vec<T>> {
    /// `if (map.has(k)) map.get(k).push(v); else map.set(k, [v]);`
    fn push_entry(&mut self, key: &str, value: T) {
        if self.has(key) {
            self.get_mut(key).expect("checked by has").push(value);
        } else {
            self.set(key, vec![value]);
        }
    }
}

/// A JavaScript `Set` of strings, in insertion order.
#[derive(Clone, Debug, Default)]
struct JsSet {
    items: Vec<String>,
    index: HashSet<String>,
}

impl JsSet {
    fn singleton(id: &str) -> JsSet {
        let mut set = JsSet::default();
        set.add(id);
        set
    }

    fn from_ids(ids: &[String]) -> JsSet {
        let mut set = JsSet::default();
        for id in ids {
            set.add(id);
        }
        set
    }

    fn add(&mut self, id: &str) {
        if self.index.insert(id.to_string()) {
            self.items.push(id.to_string());
        }
    }

    fn has(&self, id: &str) -> bool {
        self.index.contains(id)
    }

    fn iter(&self) -> std::slice::Iter<'_, String> {
        self.items.iter()
    }

    fn first(&self) -> Option<&String> {
        self.items.first()
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}

/// `if (x)` on a node id: a missing or empty id is falsy.
fn truthy(id: &Option<String>) -> bool {
    id.as_deref().is_some_and(|id| !id.is_empty())
}

/// A side of a relative constraint that the branch reads (`constraint.right`, `constraint.bottom`,
/// ...). A missing id is invalid input: the source would carry `undefined` into the graph, so the
/// port panics with the constraint's name instead.
fn required<'a>(id: &'a Option<String>, what: &str) -> &'a str {
    id.as_deref()
        .unwrap_or_else(|| panic!("{what} has a constraint without a node id"))
}

/// `idToNodeMap.get(id)` followed by a property read: the source throws for a missing node.
fn node_of(id_to_node: &HashMap<String, usize>, id: &str) -> usize {
    *id_to_node
        .get(id)
        .unwrap_or_else(|| panic!("a constraint names {id:?}, which is not a node of the layout"))
}

/// The first node of an alignment list (`list[0]`), for a dummy that a node of the list looks up.
/// Such a dummy holds at least that node, so the panic guards an invariant rather than an input.
fn first_id(ids: &[String]) -> &str {
    ids.first()
        .map(String::as_str)
        .unwrap_or_else(|| panic!("an alignment list is empty"))
}

/// `xCoords[nodeIndexes.get(id)]`. A node without an index reads as NaN, the `undefined` of the
/// source in its arithmetic.
fn coord(coords: &[f64], node_indexes: &HashMap<String, usize>, id: &str) -> f64 {
    node_indexes
        .get(id)
        .map_or(f64::NAN, |&index| coords[index])
}

/// `xCoords[nodeIndexes.get(id)] = value`: a node without an index is not written.
fn set_coord(coords: &mut [f64], node_indexes: &HashMap<String, usize>, id: &str, value: f64) {
    if let Some(&index) = node_indexes.get(id) {
        coords[index] = value;
    }
}

/// `xCoords[nodeIndexes.get(list[0])]`, for a dummy node's start position (`ConstraintHandler.js`
/// 717, 731). An empty list reads as `undefined`, which is NaN.
fn first_coord(list: &[String], coords: &[f64], node_indexes: &HashMap<String, usize>) -> f64 {
    list.first()
        .map_or(f64::NAN, |id| coord(coords, node_indexes, id))
}

/// `map.get(key)` read as a number: an absent key is `undefined`, which is NaN in the source's
/// arithmetic and comparisons.
fn value_or_nan(map: &JsMap<f64>, key: &str) -> f64 {
    map.get(key).copied().unwrap_or(f64::NAN)
}

/// `dummy.get(id) || id`: the dummy node of an alignment list, or the node itself.
fn mapped(dummies: &JsMap<String>, id: &str) -> String {
    dummies.get(id).cloned().unwrap_or_else(|| id.to_string())
}

/// The gap of a relative constraint. `handle_constraints` fills every gap before anything reads it
/// (`ConstraintHandler.js` 38-49), so the value is set; NaN is the `undefined` the source would
/// otherwise carry.
fn gap_of(constraint: &RelativeConstraint) -> f64 {
    constraint.gap.unwrap_or(f64::NAN)
}

/// `LNode.setCenter` (`LNode.js` 126-130).
fn set_center(rect: &mut RectD, cx: f64, cy: f64) {
    rect.x = cx - rect.width / 2.0;
    rect.y = cy - rect.height / 2.0;
}

/// `calculatePositionDiff` (`ConstraintHandler.js` 54-56).
fn calculate_position_diff(pos1: PointD, pos2: PointD) -> PointD {
    PointD::new(pos1.x - pos2.x, pos1.y - pos2.y)
}

/// `calculateAvgPosition` (`ConstraintHandler.js` 59-68): the mean of the set's coordinates.
fn calculate_avg_position(
    set: &JsSet,
    x_coords: &[f64],
    y_coords: &[f64],
    node_indexes: &HashMap<String, usize>,
) -> PointD {
    let mut x_sum = 0.0;
    let mut y_sum = 0.0;
    for node_id in set.iter() {
        x_sum += coord(x_coords, node_indexes, node_id);
        y_sum += coord(y_coords, node_indexes, node_id);
    }
    PointD::new(x_sum / set.len() as f64, y_sum / set.len() as f64)
}

/// `setUnion` (`ConstraintHandler.js` 77-83): a copy of `a`, then the new elements of `b`.
fn set_union(a: &JsSet, b: &JsSet) -> JsSet {
    let mut union = a.clone();
    for elem in b.iter() {
        union.add(elem);
    }
    union
}

/// `findComponents` (`ConstraintHandler.js` 317-347): the weakly connected components of an
/// undirected graph, numbered by the first key that starts each one.
fn find_components(graph: &JsMap<Vec<DagEdge>>) -> Vec<Vec<String>> {
    let mut components: Vec<Vec<String>> = Vec::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    for key in graph.keys() {
        if visited.contains(key) {
            continue;
        }
        let count = components.len();
        components.push(Vec::new());
        queue.push_back(key.to_string());
        visited.insert(key.to_string());
        components[count].push(key.to_string());
        while let Some(current) = queue.pop_front() {
            let neighbors = graph
                .get(&current)
                .expect("a neighbour is a key of the graph");
            for neighbor in neighbors {
                if !visited.contains(&neighbor.id) {
                    queue.push_back(neighbor.id.clone());
                    visited.insert(neighbor.id.clone());
                    components[count].push(neighbor.id.clone());
                }
            }
        }
    }
    components
}

/// `dagToUndirected` (`ConstraintHandler.js` 350-365): each edge in both directions, in key order.
fn dag_to_undirected(dag: &JsMap<Vec<DagEdge>>) -> JsMap<Vec<DagEdge>> {
    let mut undirected: JsMap<Vec<DagEdge>> = JsMap::default();
    for key in dag.keys() {
        undirected.set(key, Vec::new());
    }
    for (key, adjacents) in dag.iter() {
        for adjacent in adjacents {
            undirected
                .get_mut(key)
                .expect("every key was set above")
                .push(adjacent.clone());
            undirected
                .get_mut(&adjacent.id)
                .expect("every target is a key of the graph")
                .push(DagEdge {
                    id: key.to_string(),
                    gap: adjacent.gap,
                    direction: adjacent.direction,
                });
        }
    }
    undirected
}

/// `dagToReversed` (`ConstraintHandler.js` 368-382): each edge pointing back, so that a node's
/// list holds its predecessors.
fn dag_to_reversed(dag: &JsMap<Vec<DagEdge>>) -> JsMap<Vec<DagEdge>> {
    let mut reversed: JsMap<Vec<DagEdge>> = JsMap::default();
    for key in dag.keys() {
        reversed.set(key, Vec::new());
    }
    for (key, adjacents) in dag.iter() {
        for adjacent in adjacents {
            reversed
                .get_mut(&adjacent.id)
                .expect("every target is a key of the graph")
                .push(DagEdge {
                    id: key.to_string(),
                    gap: adjacent.gap,
                    direction: adjacent.direction,
                });
        }
    }
    reversed
}

/// `findAppropriatePositionForRelativePlacement` (`ConstraintHandler.js` 74-280). Each node of the
/// DAG gets the position its predecessors allow, fixed nodes stay where they are, and each
/// connected component is shifted to the centre it had. `coords` is the axis of the constraint
/// (`xCoords` for horizontal). `fixed_nodes` is `None` for the call with no fixed set
/// (`ConstraintHandler.js` 552-553, inside the transform block) and `Some` for the enforcing call.
/// `dummy_positions` is `None` for the same call, which passes no dummy positions either.
fn find_appropriate_position(
    coords: &[f64],
    node_indexes: &HashMap<String, usize>,
    graph: &JsMap<Vec<DagEdge>>,
    fixed_nodes: Option<&JsSet>,
    dummy_positions: Option<&JsMap<f64>>,
    component_sources: &[Vec<String>],
) -> JsMap<f64> {
    // The position of a node: its coordinate, or the dummy's start position (ConstraintHandler.js
    // 105, 133, 175, 252). Without dummy positions the source's `dummyPositions.get` throws for a
    // key that is not a node, and this port panics with the id.
    let position_of = |id: &str| -> f64 {
        match node_indexes.get(id) {
            Some(&index) => coords[index],
            None => match dummy_positions {
                Some(dummies) => value_or_nan(dummies, id),
                None => panic!("a constraint names {id:?}, which is not a node of the layout"),
            },
        }
    };

    // ConstraintHandler.js 86-95: the in-degree of every node.
    let mut in_degrees: JsMap<f64> = JsMap::default();
    for key in graph.keys() {
        in_degrees.set(key, 0.0);
    }
    for (_, adjacents) in graph.iter() {
        for adjacent in adjacents {
            let degree = value_or_nan(&in_degrees, &adjacent.id) + 1.0;
            in_degrees.set(&adjacent.id, degree);
        }
    }

    // ConstraintHandler.js 97-118. A source of a fixed set gets its position from the component
    // sources below, so it has no entry here.
    let mut position_map: JsMap<f64> = JsMap::default();
    let mut past_map: JsMap<JsSet> = JsMap::default();
    let mut queue: VecDeque<String> = VecDeque::new();
    for (key, &degree) in in_degrees.iter() {
        if degree == 0.0 {
            queue.push_back(key.to_string());
            if fixed_nodes.is_none() {
                position_map.set(key, position_of(key));
            }
        } else {
            position_map.set(key, f64::NEG_INFINITY);
        }
        if fixed_nodes.is_some() {
            past_map.set(key, JsSet::singleton(key));
        }
    }

    // ConstraintHandler.js 121-164: a component with a fixed node takes the mean of the fixed
    // nodes' positions; a component without one takes the mean of its sources.
    if let Some(fixed) = fixed_nodes {
        for component in component_sources {
            let fixed_ids: Vec<&String> = component.iter().filter(|id| fixed.has(id)).collect();
            if !fixed_ids.is_empty() {
                let mut position = 0.0;
                for fixed_id in &fixed_ids {
                    let value = position_of(fixed_id);
                    position_map.set(fixed_id, value);
                    position += value;
                }
                position /= fixed_ids.len() as f64;
                for node_id in component {
                    if !fixed.has(node_id) {
                        position_map.set(node_id, position);
                    }
                }
            } else {
                let mut position = 0.0;
                for node_id in component {
                    position += position_of(node_id);
                }
                position /= component.len() as f64;
                for node_id in component {
                    position_map.set(node_id, position);
                }
            }
        }
    }

    // ConstraintHandler.js 166-200: a breadth-first walk from the sources. A fixed node that the
    // walk would move is put back, and the nodes before it move by the same amount.
    while let Some(current) = queue.pop_front() {
        let neighbors = graph
            .get(&current)
            .expect("a queued node is a key of the graph");
        for neighbor in neighbors {
            if value_or_nan(&position_map, &neighbor.id)
                < value_or_nan(&position_map, &current) + neighbor.gap
            {
                if fixed_nodes.is_some_and(|fixed| fixed.has(&neighbor.id)) {
                    let fixed_position = position_of(&neighbor.id);
                    position_map.set(&neighbor.id, fixed_position);
                    if fixed_position < value_or_nan(&position_map, &current) + neighbor.gap {
                        let diff =
                            (value_or_nan(&position_map, &current) + neighbor.gap) - fixed_position;
                        let past = past_map
                            .get(&current)
                            .expect("past sets exist when fixed nodes are given");
                        for node_id in past.iter() {
                            let value = value_or_nan(&position_map, node_id) - diff;
                            position_map.set(node_id, value);
                        }
                    }
                } else {
                    let value = value_or_nan(&position_map, &current) + neighbor.gap;
                    position_map.set(&neighbor.id, value);
                }
            }
            let degree = value_or_nan(&in_degrees, &neighbor.id) - 1.0;
            in_degrees.set(&neighbor.id, degree);
            if degree == 0.0 {
                queue.push_back(neighbor.id.clone());
            }
            if fixed_nodes.is_some() {
                let union = set_union(
                    past_map
                        .get(&current)
                        .expect("past sets exist when fixed nodes are given"),
                    past_map
                        .get(&neighbor.id)
                        .expect("past sets exist when fixed nodes are given"),
                );
                past_map.set(&neighbor.id, union);
            }
        }
    }

    // ConstraintHandler.js 203-276: the sinks' past sets, merged where they share a node, are the
    // components that move as one. Each moves so that its span before the walk keeps its centre.
    if let Some(fixed) = fixed_nodes {
        let mut sink_nodes = JsSet::default();
        for (key, adjacents) in graph.iter() {
            if adjacents.is_empty() {
                sink_nodes.add(key);
            }
        }
        let mut components: Vec<JsSet> = Vec::new();
        for (key, past) in past_map.iter() {
            if !sink_nodes.has(key) {
                continue;
            }
            if past.iter().any(|node_id| fixed.has(node_id)) {
                continue;
            }
            let first = past.first().expect("a past set holds its own node");
            // The last component that holds the first node (ConstraintHandler.js 225-230).
            let exist_at = components
                .iter()
                .rposition(|component| component.has(first));
            match exist_at {
                None => components.push(past.clone()),
                Some(index) => {
                    for ele in past.iter() {
                        components[index].add(ele);
                    }
                }
            }
        }

        for component in &components {
            let mut min_before = f64::INFINITY;
            let mut min_after = f64::INFINITY;
            let mut max_before = f64::NEG_INFINITY;
            let mut max_after = f64::NEG_INFINITY;
            for node_id in component.iter() {
                let pos_before = position_of(node_id);
                let pos_after = value_or_nan(&position_map, node_id);
                if pos_before < min_before {
                    min_before = pos_before;
                }
                if pos_before > max_before {
                    max_before = pos_before;
                }
                if pos_after < min_after {
                    min_after = pos_after;
                }
                if pos_after > max_after {
                    max_after = pos_after;
                }
            }
            let diff = (min_before + max_before) / 2.0 - (min_after + max_after) / 2.0;
            for node_id in component.iter() {
                let value = value_or_nan(&position_map, node_id) + diff;
                position_map.set(node_id, value);
            }
        }
    }

    position_map
}

/// `constructComponents` (`CoSELayout.js` 540-575): the components of a directed subgraph, as
/// undirected, with the flag of whether one of their nodes is fixed.
fn construct_components(
    graph: &JsMap<Vec<String>>,
    fixed_nodes: &JsSet,
) -> (Vec<Vec<String>>, Vec<bool>) {
    let mut components: Vec<Vec<String>> = Vec::new();
    let mut is_fixed: Vec<bool> = Vec::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    for key in graph.keys() {
        if visited.contains(key) {
            continue;
        }
        let count = components.len();
        components.push(Vec::new());
        is_fixed.push(false);
        queue.push_back(key.to_string());
        visited.insert(key.to_string());
        components[count].push(key.to_string());
        while let Some(current) = queue.pop_front() {
            if fixed_nodes.has(&current) {
                is_fixed[count] = true;
            }
            let neighbors = graph
                .get(&current)
                .expect("a neighbour is a key of the subgraph");
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    queue.push_back(neighbor.clone());
                    visited.insert(neighbor.clone());
                    components[count].push(neighbor.clone());
                }
            }
        }
    }
    (components, is_fixed)
}

/// `calculateCompoundWeight` (`CoSELayout.js` 338-353): 100 for each fixed node below the compound.
fn calculate_compound_weight(model: &Model, fixed_node_set: &JsSet, compound: usize) -> f64 {
    let child = model.nodes[compound]
        .child
        .expect("calculateCompoundWeight is called on a compound");
    let mut weight = 0.0;
    for &node in &model.graphs[child].nodes {
        if model.nodes[node].child.is_none() {
            if fixed_node_set.has(&model.nodes[node].id) {
                weight += 100.0;
            }
        } else {
            weight += calculate_compound_weight(model, fixed_node_set, node);
        }
    }
    weight
}

/// The start position of a node in a relative constraint: the centre of its first actual node for
/// an alignment dummy, else the centre of the node (`CoSELayout.js` 448-452, 458-462, 476-480,
/// 486-490).
fn start_centre(
    model: &Model,
    id_to_node: &HashMap<String, usize>,
    dummy_to_node: &JsMap<Vec<String>>,
    id: &str,
    horizontal: bool,
) -> f64 {
    let node = match dummy_to_node.get(id) {
        Some(actual) => node_of(id_to_node, first_id(actual)),
        None => node_of(id_to_node, id),
    };
    if horizontal {
        model.centre_x(node)
    } else {
        model.centre_y(node)
    }
}

/// `coords[nodeIndexes.get(id)]` for a constraint's optional id. An absent id is `undefined` in the
/// source, which reads as NaN in its arithmetic (`ConstraintHandler.js` 291-294).
fn optional_coord(
    coords: &[f64],
    node_indexes: &HashMap<String, usize>,
    id: &Option<String>,
) -> f64 {
    id.as_deref()
        .map_or(f64::NAN, |id| coord(coords, node_indexes, id))
}

/// `applyReflectionForRelativePlacement` (`ConstraintHandler.js` 284-314). A horizontal constraint
/// `{left, right}` votes on the sign of x, a vertical one `{top, bottom}` on the sign of y. A vote
/// is "reflect" when the draft already has the constraint's order, and a NaN difference is a vote
/// against. The draft is reflected on an axis when the votes against it are a majority, and on
/// both axes when both are.
fn apply_reflection_for_relative_placement(
    x_coords: &mut [f64],
    y_coords: &mut [f64],
    node_indexes: &HashMap<String, usize>,
    constraints: &[RelativeConstraint],
) {
    // ConstraintHandler.js 286-296: the votes.
    let mut reflect_on_y = 0;
    let mut not_reflect_on_y = 0;
    let mut reflect_on_x = 0;
    let mut not_reflect_on_x = 0;
    for constraint in constraints {
        if truthy(&constraint.left) {
            let diff = optional_coord(x_coords, node_indexes, &constraint.left)
                - optional_coord(x_coords, node_indexes, &constraint.right);
            if diff >= 0.0 {
                reflect_on_y += 1;
            } else {
                not_reflect_on_y += 1;
            }
        } else {
            let diff = optional_coord(y_coords, node_indexes, &constraint.top)
                - optional_coord(y_coords, node_indexes, &constraint.bottom);
            if diff >= 0.0 {
                reflect_on_x += 1;
            } else {
                not_reflect_on_x += 1;
            }
        }
    }

    // ConstraintHandler.js 298-313: `nodeIndexes.size` entries. `-1 * x` is `-x` for every value
    // a comparison can observe (only the sign of a NaN differs).
    let count = node_indexes.len();
    if reflect_on_y > not_reflect_on_y && reflect_on_x > not_reflect_on_x {
        for (x, y) in x_coords.iter_mut().zip(y_coords.iter_mut()).take(count) {
            *x = -*x;
            *y = -*y;
        }
    } else if reflect_on_y > not_reflect_on_y {
        for x in x_coords.iter_mut().take(count) {
            *x = -*x;
        }
    } else if reflect_on_x > not_reflect_on_x {
        for y in y_coords.iter_mut().take(count) {
            *y = -*y;
        }
    }
}

/// The constraints that the transform block reads, as `handleConstraints` holds them after the gap
/// fill and the DAG (`ConstraintHandler.js` 400-436).
struct TransformInputs<'a> {
    fixed_node_constraint: Option<&'a [FixedNodeConstraint]>,
    /// `fixedNodes` (`ConstraintHandler.js` 394, 400-404).
    fixed_nodes: &'a JsSet,
    alignment: Option<&'a AlignmentConstraint>,
    relative: Option<&'a [RelativeConstraint]>,
    dag: &'a JsMap<Vec<DagEdge>>,
}

/// The transform block of `ConstraintHandler.handleConstraints` (`ConstraintHandler.js` 434-435 and
/// 438-609). The draft is rotated and reflected to fit the constrained nodes: a Procrustes fit of
/// the draft to the targets of the fixed, alignment or relative constraints. It runs when
/// `randomize` is set (`TRANSFORM_ON_CONSTRAINT_HANDLING`, `cose.js` 233-237).
fn transform_draft(
    x_coords: &mut [f64],
    y_coords: &mut [f64],
    node_indexes: &HashMap<String, usize>,
    input: &TransformInputs<'_>,
) {
    // ConstraintHandler.js 434-435. With no relative list the DAG is empty, so the components are
    // empty as well.
    let dag_undirected = dag_to_undirected(input.dag);
    let components = find_components(&dag_undirected);

    // `targetMatrix` and `sourceMatrix` (ConstraintHandler.js 390-393): one row per node.
    let mut target: Vec<[f64; 2]> = Vec::new();
    let mut source: Vec<[f64; 2]> = Vec::new();
    let mut standard_transformation = false;
    let mut reflection_type = false;

    if let Some(fixed) = input.fixed_node_constraint.filter(|fixed| fixed.len() > 1) {
        // ConstraintHandler.js 440-446. Only the fixed nodes are fitted: the `else if` chain at 440,
        // 447 and 497 reads the alignment and relative lists only when this branch is not taken.
        for data in fixed {
            target.push([data.position.x, data.position.y]);
            source.push([
                coord(x_coords, node_indexes, &data.node_id),
                coord(y_coords, node_indexes, &data.node_id),
            ]);
        }
        standard_transformation = true;
    } else if let Some(alignment) = input.alignment {
        // ConstraintHandler.js 448-470. A vertical list moves to the x of its first fixed node, or
        // to the mean x of the list when it has none. The rows follow the list, repeats included.
        if let Some(vertical) = &alignment.vertical {
            for list in vertical {
                let set = JsSet::from_ids(list);
                let x_pos = match set.iter().find(|id| input.fixed_nodes.has(id)) {
                    Some(fixed_id) => coord(x_coords, node_indexes, fixed_id),
                    None => calculate_avg_position(&set, x_coords, y_coords, node_indexes).x,
                };
                for node_id in list {
                    target.push([x_pos, coord(y_coords, node_indexes, node_id)]);
                    source.push([
                        coord(x_coords, node_indexes, node_id),
                        coord(y_coords, node_indexes, node_id),
                    ]);
                }
            }
            standard_transformation = true;
        }
        // ConstraintHandler.js 471-492. The y of the first fixed node is read from `xCoords` (481),
        // as the source has it, so the port reads x here too.
        if let Some(horizontal) = &alignment.horizontal {
            for list in horizontal {
                let set = JsSet::from_ids(list);
                let y_pos = match set.iter().find(|id| input.fixed_nodes.has(id)) {
                    Some(fixed_id) => coord(x_coords, node_indexes, fixed_id),
                    None => calculate_avg_position(&set, x_coords, y_coords, node_indexes).y,
                };
                for node_id in list {
                    target.push([coord(x_coords, node_indexes, node_id), y_pos]);
                    source.push([
                        coord(x_coords, node_indexes, node_id),
                        coord(y_coords, node_indexes, node_id),
                    ]);
                }
            }
            standard_transformation = true;
        }
        // ConstraintHandler.js 493-495. The reflection follows the fit when the relative list is
        // there too.
        if input.relative.is_some() {
            reflection_type = true;
        }
    } else if let Some(relative) = input.relative {
        // ConstraintHandler.js 499-506: the first component of the largest size.
        let mut largest_size = 0;
        let mut largest_index = 0;
        for (index, component) in components.iter().enumerate() {
            if component.len() > largest_size {
                largest_size = component.len();
                largest_index = index;
            }
        }
        // ConstraintHandler.js 508-512. A largest component below half the DAG is not dominant:
        // the votes reflect the draft, and no fit is made.
        if (largest_size as f64) < (dag_undirected.len() as f64) / 2.0 {
            apply_reflection_for_relative_placement(x_coords, y_coords, node_indexes, relative);
        } else {
            // ConstraintHandler.js 513-550. The subgraphs of the largest component, and the
            // constraints it holds for the reflection.
            let component = components.get(largest_index).expect(
                "a relative list has a component: the source throws on components[0] otherwise",
            );
            let mut sub_horizontal: JsMap<Vec<DagEdge>> = JsMap::default();
            let mut sub_vertical: JsMap<Vec<DagEdge>> = JsMap::default();
            let mut constraints_in_largest: Vec<RelativeConstraint> = Vec::new();
            for node_id in component {
                let adjacents = input
                    .dag
                    .get(node_id)
                    .expect("every node of a component is a key of the DAG");
                for adjacent in adjacents {
                    if adjacent.direction == Direction::Horizontal {
                        sub_horizontal.push_entry(node_id, adjacent.clone());
                        if !sub_horizontal.has(&adjacent.id) {
                            sub_horizontal.set(&adjacent.id, Vec::new());
                        }
                        constraints_in_largest.push(RelativeConstraint {
                            left: Some(node_id.clone()),
                            right: Some(adjacent.id.clone()),
                            ..Default::default()
                        });
                    } else {
                        sub_vertical.push_entry(node_id, adjacent.clone());
                        if !sub_vertical.has(&adjacent.id) {
                            sub_vertical.set(&adjacent.id, Vec::new());
                        }
                        constraints_in_largest.push(RelativeConstraint {
                            top: Some(node_id.clone()),
                            bottom: Some(adjacent.id.clone()),
                            ..Default::default()
                        });
                    }
                }
            }
            apply_reflection_for_relative_placement(
                x_coords,
                y_coords,
                node_indexes,
                &constraints_in_largest,
            );

            // ConstraintHandler.js 552-553: the subgraphs are placed with no fixed set and no dummy
            // positions.
            let position_horizontal =
                find_appropriate_position(x_coords, node_indexes, &sub_horizontal, None, None, &[]);
            let position_vertical =
                find_appropriate_position(y_coords, node_indexes, &sub_vertical, None, None, &[]);

            // ConstraintHandler.js 556-571. The source is the draft; the target is the placed
            // position where the subgraph gives one, and the draft's own coordinate where it does not.
            for node_id in component {
                source.push([
                    coord(x_coords, node_indexes, node_id),
                    coord(y_coords, node_indexes, node_id),
                ]);
                let x_target = position_horizontal
                    .get(node_id)
                    .copied()
                    .unwrap_or_else(|| coord(x_coords, node_indexes, node_id));
                let y_target = position_vertical
                    .get(node_id)
                    .copied()
                    .unwrap_or_else(|| coord(y_coords, node_indexes, node_id));
                target.push([x_target, y_target]);
            }
            standard_transformation = true;
        }
    }

    // ConstraintHandler.js 578-593: the fit `T = V U'` (`linalg.rs`). An empty or non-finite
    // configuration is an error there, where the source throws; the port panics with it.
    if standard_transformation {
        let fit = orthogonal_procrustes(&target, &source).unwrap_or_else(|error| {
            panic!("the constraint transform (ConstraintHandler.js 578-593) cannot fit: {error}")
        });
        // ConstraintHandler.js 596-602: `[x, y] T` for each draft point. `point` is read before x is
        // written, as the source's `temp1` is.
        for (x, y) in x_coords
            .iter_mut()
            .zip(y_coords.iter_mut())
            .take(node_indexes.len())
        {
            let point = [*x, *y];
            *x = dot_product(&point, &[fit[0][0], fit[1][0]]);
            *y = dot_product(&point, &[fit[0][1], fit[1][1]]);
        }
        // ConstraintHandler.js 604-607. `reflectionType` is set only with a relative list.
        if reflection_type && let Some(relative) = input.relative {
            apply_reflection_for_relative_placement(x_coords, y_coords, node_indexes, relative);
        }
    }
}

/// `ConstraintHandler.handleConstraints` (`ConstraintHandler.js` 9-847): the nodes are moved to
/// satisfy the fixed, alignment and relative constraints, in the coordinates of the centres.
pub(crate) fn handle_constraints(layout: &mut Layout) {
    // ConstraintHandler.js 19-35: the simple nodes, in `getAllNodes` order, with their index.
    let all_nodes = layout.model.all_nodes();
    let mut id_to_node: HashMap<String, usize> = HashMap::new();
    let mut node_indexes: HashMap<String, usize> = HashMap::new();
    let mut x_coords: Vec<f64> = Vec::new();
    let mut y_coords: Vec<f64> = Vec::new();
    for &node in &all_nodes {
        if layout.model.nodes[node].child.is_none() {
            let id = layout.model.nodes[node].id.clone();
            node_indexes.insert(id.clone(), x_coords.len());
            x_coords.push(layout.model.centre_x(node));
            y_coords.push(layout.model.centre_y(node));
            id_to_node.insert(id, node);
        }
    }

    // ConstraintHandler.js 38-49: a gap that is undefined or NaN becomes the default edge length
    // plus the half widths (left and right) or half heights (top and bottom) of the two nodes. A
    // gap of 0 stays, since `!0 && 0 != 0` is false.
    let default_edge_length = layout.k.default_edge_length;
    if let Some(relative) = layout.constraints.relative.as_mut() {
        for constraint in relative.iter_mut() {
            if constraint.gap.is_none_or(f64::is_nan) {
                let gap = if truthy(&constraint.left) {
                    let left = node_of(&id_to_node, required(&constraint.left, RELATIVE));
                    let right = node_of(&id_to_node, required(&constraint.right, RELATIVE));
                    default_edge_length
                        + layout.model.nodes[left].rect.width / 2.0
                        + layout.model.nodes[right].rect.width / 2.0
                } else {
                    let top = node_of(&id_to_node, required(&constraint.top, RELATIVE));
                    let bottom = node_of(&id_to_node, required(&constraint.bottom, RELATIVE));
                    default_edge_length
                        + layout.model.nodes[top].rect.height / 2.0
                        + layout.model.nodes[bottom].rect.height / 2.0
                };
                constraint.gap = Some(gap);
            }
        }
    }

    // The constraints as the handler reads them, after the gap fill above. The gaps written back
    // here are the ones `init_constraint_variables` reads later.
    let fixed_node_constraint = layout.fixed_node_constraint.clone();
    let alignment = layout.constraints.alignment.clone();
    let relative = layout.constraints.relative.clone();

    // ConstraintHandler.js 400-404
    let mut fixed_nodes = JsSet::default();
    if let Some(fixed) = &fixed_node_constraint {
        for data in fixed {
            fixed_nodes.add(&data.node_id);
        }
    }

    // ConstraintHandler.js 407-432: the DAG of the relative constraints.
    let mut dag: JsMap<Vec<DagEdge>> = JsMap::default();
    if let Some(relative) = &relative {
        for constraint in relative {
            let gap = gap_of(constraint);
            if truthy(&constraint.left) {
                let left = required(&constraint.left, RELATIVE);
                let right = required(&constraint.right, RELATIVE);
                dag.push_entry(
                    left,
                    DagEdge {
                        id: right.to_string(),
                        gap,
                        direction: Direction::Horizontal,
                    },
                );
                if !dag.has(right) {
                    dag.set(right, Vec::new());
                }
            } else {
                let top = required(&constraint.top, RELATIVE);
                let bottom = required(&constraint.bottom, RELATIVE);
                dag.push_entry(
                    top,
                    DagEdge {
                        id: bottom.to_string(),
                        gap,
                        direction: Direction::Vertical,
                    },
                );
                if !dag.has(bottom) {
                    dag.set(bottom, Vec::new());
                }
            }
        }
    }

    // ConstraintHandler.js 434-435 and 438-609: the transform of the draft, when `randomize` is set.
    // It runs before the enforcement, as the source does.
    if layout.randomize {
        let input = TransformInputs {
            fixed_node_constraint: fixed_node_constraint.as_deref(),
            fixed_nodes: &fixed_nodes,
            alignment: alignment.as_ref(),
            relative: relative.as_deref(),
            dag: &dag,
        };
        transform_draft(&mut x_coords, &mut y_coords, &node_indexes, &input);
    }

    if ENFORCE_CONSTRAINTS {
        // ConstraintHandler.js 616-640: the fixed nodes are moved, with all the nodes, by the mean
        // of their offsets from their positions, and then put exactly on their positions.
        if let Some(fixed) = fixed_node_constraint
            .as_ref()
            .filter(|fixed| !fixed.is_empty())
        {
            let mut translation = PointD::new(0.0, 0.0);
            for data in fixed {
                let pos_in_theory = PointD::new(
                    coord(&x_coords, &node_indexes, &data.node_id),
                    coord(&y_coords, &node_indexes, &data.node_id),
                );
                let pos_diff = calculate_position_diff(data.position, pos_in_theory);
                translation.x += pos_diff.x;
                translation.y += pos_diff.y;
            }
            translation.x /= fixed.len() as f64;
            translation.y /= fixed.len() as f64;
            for x in x_coords.iter_mut() {
                *x += translation.x;
            }
            for y in y_coords.iter_mut() {
                *y += translation.y;
            }
            for data in fixed {
                set_coord(&mut x_coords, &node_indexes, &data.node_id, data.position.x);
                set_coord(&mut y_coords, &node_indexes, &data.node_id, data.position.y);
            }
        }

        // ConstraintHandler.js 644-685: each alignment list takes the x (vertical lists) or y
        // (horizontal lists) of its first fixed node, or the mean of the list when none is fixed.
        // Fixed nodes are not moved.
        if let Some(alignment) = &alignment {
            if let Some(vertical) = &alignment.vertical {
                for list in vertical {
                    let set = JsSet::from_ids(list);
                    let x_pos = match set.iter().find(|id| fixed_nodes.has(id)) {
                        Some(fixed_id) => coord(&x_coords, &node_indexes, fixed_id),
                        None => calculate_avg_position(&set, &x_coords, &y_coords, &node_indexes).x,
                    };
                    for node_id in set.iter() {
                        if !fixed_nodes.has(node_id) {
                            set_coord(&mut x_coords, &node_indexes, node_id, x_pos);
                        }
                    }
                }
            }
            if let Some(horizontal) = &alignment.horizontal {
                for list in horizontal {
                    let set = JsSet::from_ids(list);
                    let y_pos = match set.iter().find(|id| fixed_nodes.has(id)) {
                        Some(fixed_id) => coord(&y_coords, &node_indexes, fixed_id),
                        None => calculate_avg_position(&set, &x_coords, &y_coords, &node_indexes).y,
                    };
                    for node_id in set.iter() {
                        if !fixed_nodes.has(node_id) {
                            set_coord(&mut y_coords, &node_indexes, node_id, y_pos);
                        }
                    }
                }
            }
        }

        // ConstraintHandler.js 689-835
        if relative.is_some() {
            enforce_relative_placement(
                &dag,
                alignment.as_ref(),
                &fixed_nodes,
                &node_indexes,
                &mut x_coords,
                &mut y_coords,
            );
        }
    }

    // ConstraintHandler.js 840-845: every simple node takes its computed centre.
    for &node in &all_nodes {
        if layout.model.nodes[node].child.is_none() {
            let cx = coord(&x_coords, &node_indexes, &layout.model.nodes[node].id);
            let cy = coord(&y_coords, &node_indexes, &layout.model.nodes[node].id);
            set_center(&mut layout.model.nodes[node].rect, cx, cy);
        }
    }
}

/// The relative placement of `ConstraintHandler.js` 689-835. Each axis is a DAG whose alignment
/// lists are single dummy nodes ("dummy" + i). Each axis is placed by
/// `find_appropriate_position`, and every node takes the position of its dummy or itself.
fn enforce_relative_placement(
    dag: &JsMap<Vec<DagEdge>>,
    alignment: Option<&AlignmentConstraint>,
    fixed_nodes: &JsSet,
    node_indexes: &HashMap<String, usize>,
    x_coords: &mut [f64],
    y_coords: &mut [f64],
) {
    // ConstraintHandler.js 690-697
    let mut node_to_dummy_vertical: JsMap<String> = JsMap::default();
    let mut node_to_dummy_horizontal: JsMap<String> = JsMap::default();
    let mut dummy_to_node_vertical: JsMap<Vec<String>> = JsMap::default();
    let mut dummy_to_node_horizontal: JsMap<Vec<String>> = JsMap::default();
    let mut dummy_positions_vertical: JsMap<f64> = JsMap::default();
    let mut dummy_positions_horizontal: JsMap<f64> = JsMap::default();
    let mut fixed_nodes_on_horizontal = JsSet::default();
    let mut fixed_nodes_on_vertical = JsSet::default();

    // ConstraintHandler.js 700-703
    for node_id in fixed_nodes.iter() {
        fixed_nodes_on_horizontal.add(node_id);
        fixed_nodes_on_vertical.add(node_id);
    }

    // ConstraintHandler.js 705-734. A dummy is fixed on the axis of the list it belongs to, as the
    // source has it: a vertical list's fixed dummy goes into the horizontal set (709-711), and a
    // horizontal list's into the vertical set (727-729).
    if let Some(alignment) = alignment {
        if let Some(vertical) = &alignment.vertical {
            for (i, list) in vertical.iter().enumerate() {
                let dummy = format!("dummy{i}");
                dummy_to_node_vertical.set(&dummy, Vec::new());
                for node_id in list {
                    node_to_dummy_vertical.set(node_id, dummy.clone());
                    dummy_to_node_vertical
                        .get_mut(&dummy)
                        .expect("set just above")
                        .push(node_id.clone());
                    if fixed_nodes.has(node_id) {
                        fixed_nodes_on_horizontal.add(&dummy);
                    }
                }
                dummy_positions_vertical.set(&dummy, first_coord(list, x_coords, node_indexes));
            }
        }
        if let Some(horizontal) = &alignment.horizontal {
            for (i, list) in horizontal.iter().enumerate() {
                let dummy = format!("dummy{i}");
                dummy_to_node_horizontal.set(&dummy, Vec::new());
                for node_id in list {
                    node_to_dummy_horizontal.set(node_id, dummy.clone());
                    dummy_to_node_horizontal
                        .get_mut(&dummy)
                        .expect("set just above")
                        .push(node_id.clone());
                    if fixed_nodes.has(node_id) {
                        fixed_nodes_on_vertical.add(&dummy);
                    }
                }
                dummy_positions_horizontal.set(&dummy, first_coord(list, y_coords, node_indexes));
            }
        }
    }

    // ConstraintHandler.js 737-781: the DAG of each axis, with the endpoints replaced by their
    // dummies. The source is the dummy of its node on the axis that the dummy belongs to.
    let mut dag_on_horizontal: JsMap<Vec<DagEdge>> = JsMap::default();
    let mut dag_on_vertical: JsMap<Vec<DagEdge>> = JsMap::default();
    for node_id in dag.keys() {
        let adjacents = dag.get(node_id).expect("every key has a list");
        for adjacent in adjacents {
            if adjacent.direction == Direction::Horizontal {
                let source_id = mapped(&node_to_dummy_vertical, node_id);
                let target = match node_to_dummy_vertical.get(&adjacent.id) {
                    Some(dummy) => DagEdge {
                        id: dummy.clone(),
                        gap: adjacent.gap,
                        direction: adjacent.direction,
                    },
                    None => adjacent.clone(),
                };
                dag_on_horizontal.push_entry(&source_id, target.clone());
                if !dag_on_horizontal.has(&target.id) {
                    dag_on_horizontal.set(&target.id, Vec::new());
                }
            } else {
                let source_id = mapped(&node_to_dummy_horizontal, node_id);
                let target = match node_to_dummy_horizontal.get(&adjacent.id) {
                    Some(dummy) => DagEdge {
                        id: dummy.clone(),
                        gap: adjacent.gap,
                        direction: adjacent.direction,
                    },
                    None => adjacent.clone(),
                };
                dag_on_vertical.push_entry(&source_id, target.clone());
                if !dag_on_vertical.has(&target.id) {
                    dag_on_vertical.set(&target.id, Vec::new());
                }
            }
        }
    }

    // ConstraintHandler.js 784-809: the components of each axis, and the sources of each.
    let undirected_on_horizontal = dag_to_undirected(&dag_on_horizontal);
    let undirected_on_vertical = dag_to_undirected(&dag_on_vertical);
    let components_on_horizontal = find_components(&undirected_on_horizontal);
    let components_on_vertical = find_components(&undirected_on_vertical);
    let reversed_on_horizontal = dag_to_reversed(&dag_on_horizontal);
    let reversed_on_vertical = dag_to_reversed(&dag_on_vertical);
    let component_sources_on_horizontal =
        component_sources(&components_on_horizontal, &reversed_on_horizontal);
    let component_sources_on_vertical =
        component_sources(&components_on_vertical, &reversed_on_vertical);

    // ConstraintHandler.js 812-813
    let position_map_horizontal = find_appropriate_position(
        x_coords,
        node_indexes,
        &dag_on_horizontal,
        Some(&fixed_nodes_on_horizontal),
        Some(&dummy_positions_vertical),
        &component_sources_on_horizontal,
    );
    let position_map_vertical = find_appropriate_position(
        y_coords,
        node_indexes,
        &dag_on_vertical,
        Some(&fixed_nodes_on_vertical),
        Some(&dummy_positions_horizontal),
        &component_sources_on_vertical,
    );

    // ConstraintHandler.js 816-835: a dummy writes its position to all of its nodes.
    for (key, position) in position_map_horizontal.iter() {
        match dummy_to_node_vertical.get(key) {
            Some(nodes) => {
                for node_id in nodes {
                    set_coord(x_coords, node_indexes, node_id, *position);
                }
            }
            None => set_coord(x_coords, node_indexes, key, *position),
        }
    }
    for (key, position) in position_map_vertical.iter() {
        match dummy_to_node_horizontal.get(key) {
            Some(nodes) => {
                for node_id in nodes {
                    set_coord(y_coords, node_indexes, node_id, *position);
                }
            }
            None => set_coord(y_coords, node_indexes, key, *position),
        }
    }
}

/// The sources (nodes without predecessors) of each component, in the component's order
/// (`ConstraintHandler.js` 790-808).
fn component_sources(
    components: &[Vec<String>],
    reversed: &JsMap<Vec<DagEdge>>,
) -> Vec<Vec<String>> {
    components
        .iter()
        .map(|component| {
            component
                .iter()
                .filter(|node_id| {
                    reversed
                        .get(node_id)
                        .expect("every node of a component is a key of the DAG")
                        .is_empty()
                })
                .cloned()
                .collect()
        })
        .collect()
}

/// `CoSELayout.prototype.shuffle` (`CoSELayout.js` 421-430): from the last index down to two
/// thirds of the length, each index swaps with a random index at or below it.
fn shuffle(array: &mut [String], rng: &mut Mulberry32) {
    let bound = 2.0 * array.len() as f64 / 3.0;
    let mut i = array.len() as i64 - 1;
    while (i as f64) >= bound {
        let j = (rng.next_f64() * (i + 1) as f64).floor() as usize;
        array.swap(i as usize, j);
        i -= 1;
    }
}

/// `CoSELayout.prototype.initConstraintVariables` (`CoSELayout.js` 324-587): the maps, the fixed
/// sets and the per-axis components that `updateDisplacements` and the spring embedder read.
pub(crate) fn init_constraint_variables(layout: &mut Layout) {
    let all_nodes = layout.model.all_nodes();
    let mut vars = ConstraintVars::default();

    // CoSELayout.js 325-335: idToNodeMap holds every node, compounds included.
    for &node in &all_nodes {
        vars.id_to_node
            .insert(layout.model.nodes[node].id.clone(), node);
    }

    // CoSELayout.js 357-374: a compound's fixedNodeWeight counts 100 per fixed node below it.
    if let Some(fixed) = &layout.fixed_node_constraint {
        for data in fixed {
            vars.fixed_node_set.add(&data.node_id);
        }
        for &node in &all_nodes {
            if layout.model.nodes[node].child.is_some() {
                let weight = calculate_compound_weight(&layout.model, &vars.fixed_node_set, node);
                if weight > 0.0 {
                    layout.model.nodes[node].fixed_node_weight = Some(weight);
                }
            }
        }
    }

    if let Some(relative) = &layout.constraints.relative {
        // CoSELayout.js 377-382
        let mut node_to_dummy_vertical: JsMap<String> = JsMap::default();
        let mut node_to_dummy_horizontal: JsMap<String> = JsMap::default();
        for node_id in vars.fixed_node_set.iter() {
            vars.fixed_nodes_on_horizontal.add(node_id);
            vars.fixed_nodes_on_vertical.add(node_id);
        }

        // CoSELayout.js 390-416
        if let Some(alignment) = &layout.constraints.alignment {
            if let Some(vertical) = &alignment.vertical {
                for (i, list) in vertical.iter().enumerate() {
                    let dummy = format!("dummy{i}");
                    vars.dummy_to_node_vertical.set(&dummy, Vec::new());
                    for node_id in list {
                        node_to_dummy_vertical.set(node_id, dummy.clone());
                        vars.dummy_to_node_vertical
                            .get_mut(&dummy)
                            .expect("set just above")
                            .push(node_id.clone());
                        if vars.fixed_node_set.has(node_id) {
                            vars.fixed_nodes_on_horizontal.add(&dummy);
                        }
                    }
                }
            }
            if let Some(horizontal) = &alignment.horizontal {
                for (i, list) in horizontal.iter().enumerate() {
                    let dummy = format!("dummy{i}");
                    vars.dummy_to_node_horizontal.set(&dummy, Vec::new());
                    for node_id in list {
                        node_to_dummy_horizontal.set(node_id, dummy.clone());
                        vars.dummy_to_node_horizontal
                            .get_mut(&dummy)
                            .expect("set just above")
                            .push(node_id.clone());
                        if vars.fixed_node_set.has(node_id) {
                            vars.fixed_nodes_on_vertical.add(&dummy);
                        }
                    }
                }
            }
        }

        if RELAX_MOVEMENT_ON_CONSTRAINTS {
            // CoSELayout.js 419-496: each node of a relative constraint, with its neighbours and
            // its start position.
            for constraint in relative {
                let gap = gap_of(constraint);
                if truthy(&constraint.left) {
                    let left = mapped(
                        &node_to_dummy_vertical,
                        required(&constraint.left, RELATIVE),
                    );
                    let right = mapped(
                        &node_to_dummy_vertical,
                        required(&constraint.right, RELATIVE),
                    );
                    if !vars.nodes_in_relative_horizontal.contains(&left) {
                        let start = start_centre(
                            &layout.model,
                            &vars.id_to_node,
                            &vars.dummy_to_node_vertical,
                            &left,
                            true,
                        );
                        vars.nodes_in_relative_horizontal.push(left.clone());
                        vars.node_to_relative_horizontal.set(&left, Vec::new());
                        vars.node_to_temp_position_horizontal.set(&left, start);
                    }
                    if !vars.nodes_in_relative_horizontal.contains(&right) {
                        let start = start_centre(
                            &layout.model,
                            &vars.id_to_node,
                            &vars.dummy_to_node_vertical,
                            &right,
                            true,
                        );
                        vars.nodes_in_relative_horizontal.push(right.clone());
                        vars.node_to_relative_horizontal.set(&right, Vec::new());
                        vars.node_to_temp_position_horizontal.set(&right, start);
                    }
                    vars.node_to_relative_horizontal
                        .get_mut(&left)
                        .expect("set just above")
                        .push(RelativeEdge::Forward {
                            id: right.clone(),
                            gap,
                        });
                    vars.node_to_relative_horizontal
                        .get_mut(&right)
                        .expect("set just above")
                        .push(RelativeEdge::Backward {
                            id: left.clone(),
                            gap,
                        });
                } else {
                    let top = mapped(
                        &node_to_dummy_horizontal,
                        required(&constraint.top, RELATIVE),
                    );
                    let bottom = mapped(
                        &node_to_dummy_horizontal,
                        required(&constraint.bottom, RELATIVE),
                    );
                    if !vars.nodes_in_relative_vertical.contains(&top) {
                        let start = start_centre(
                            &layout.model,
                            &vars.id_to_node,
                            &vars.dummy_to_node_horizontal,
                            &top,
                            false,
                        );
                        vars.nodes_in_relative_vertical.push(top.clone());
                        vars.node_to_relative_vertical.set(&top, Vec::new());
                        vars.node_to_temp_position_vertical.set(&top, start);
                    }
                    if !vars.nodes_in_relative_vertical.contains(&bottom) {
                        let start = start_centre(
                            &layout.model,
                            &vars.id_to_node,
                            &vars.dummy_to_node_horizontal,
                            &bottom,
                            false,
                        );
                        vars.nodes_in_relative_vertical.push(bottom.clone());
                        vars.node_to_relative_vertical.set(&bottom, Vec::new());
                        vars.node_to_temp_position_vertical.set(&bottom, start);
                    }
                    vars.node_to_relative_vertical
                        .get_mut(&top)
                        .expect("set just above")
                        .push(RelativeEdge::Forward {
                            id: bottom.clone(),
                            gap,
                        });
                    vars.node_to_relative_vertical
                        .get_mut(&bottom)
                        .expect("set just above")
                        .push(RelativeEdge::Backward { id: top, gap });
                }
            }
        } else {
            // CoSELayout.js 499-582: the subgraphs of the two axes, and their components.
            let mut sub_graph_horizontal: JsMap<Vec<String>> = JsMap::default();
            let mut sub_graph_vertical: JsMap<Vec<String>> = JsMap::default();
            for constraint in relative {
                if truthy(&constraint.left) {
                    let left = mapped(
                        &node_to_dummy_vertical,
                        required(&constraint.left, RELATIVE),
                    );
                    let right = mapped(
                        &node_to_dummy_vertical,
                        required(&constraint.right, RELATIVE),
                    );
                    sub_graph_horizontal.push_entry(&left, right.clone());
                    sub_graph_horizontal.push_entry(&right, left);
                } else {
                    let top = mapped(
                        &node_to_dummy_horizontal,
                        required(&constraint.top, RELATIVE),
                    );
                    let bottom = mapped(
                        &node_to_dummy_horizontal,
                        required(&constraint.bottom, RELATIVE),
                    );
                    sub_graph_vertical.push_entry(&top, bottom.clone());
                    sub_graph_vertical.push_entry(&bottom, top);
                }
            }
            let (components, fixed) =
                construct_components(&sub_graph_horizontal, &vars.fixed_nodes_on_horizontal);
            vars.components_on_horizontal = components;
            vars.fixed_components_on_horizontal = fixed;
            let (components, fixed) =
                construct_components(&sub_graph_vertical, &vars.fixed_nodes_on_vertical);
            vars.components_on_vertical = components;
            vars.fixed_components_on_vertical = fixed;
        }
    }

    layout.constraint_vars = vars;
}

/// `CoSELayout.prototype.updateDisplacements` (`CoSELayout.js` 588-800): the displacements of the
/// constrained nodes are averaged over their alignments and their components, and relaxed to the
/// relative gaps, before the nodes move.
pub(crate) fn update_displacements(layout: &mut Layout) {
    let total_iterations = layout.total_iterations;
    let model = &mut layout.model;
    let vars = &mut layout.constraint_vars;
    let rng = &mut layout.rng;
    let constraints = &layout.constraints;
    let fixed_node_constraint = &layout.fixed_node_constraint;

    // CoSELayout.js 590-596: a fixed node does not move.
    if let Some(fixed) = fixed_node_constraint {
        for data in fixed {
            let node = node_of(&vars.id_to_node, &data.node_id);
            model.nodes[node].displacement = PointD::new(0.0, 0.0);
        }
    }

    // CoSELayout.js 598-632: the nodes of an alignment list share their mean displacement; a list
    // with a fixed node gets zero.
    if let Some(alignment) = &constraints.alignment {
        if let Some(vertical) = &alignment.vertical {
            for list in vertical {
                let mut total = 0.0;
                for node_id in list {
                    if vars.fixed_node_set.has(node_id) {
                        total = 0.0;
                        break;
                    }
                    total += model.nodes[node_of(&vars.id_to_node, node_id)]
                        .displacement
                        .x;
                }
                let average = total / list.len() as f64;
                for node_id in list {
                    model.nodes[node_of(&vars.id_to_node, node_id)]
                        .displacement
                        .x = average;
                }
            }
        }
        if let Some(horizontal) = &alignment.horizontal {
            for list in horizontal {
                let mut total = 0.0;
                for node_id in list {
                    if vars.fixed_node_set.has(node_id) {
                        total = 0.0;
                        break;
                    }
                    total += model.nodes[node_of(&vars.id_to_node, node_id)]
                        .displacement
                        .y;
                }
                let average = total / list.len() as f64;
                for node_id in list {
                    model.nodes[node_of(&vars.id_to_node, node_id)]
                        .displacement
                        .y = average;
                }
            }
        }
    }

    if constraints.relative.is_some() {
        if RELAX_MOVEMENT_ON_CONSTRAINTS {
            // CoSELayout.js 639-642: the order of each list is reshuffled every tenth iteration.
            if total_iterations % 10.0 == 0.0 {
                shuffle(&mut vars.nodes_in_relative_horizontal, rng);
                shuffle(&mut vars.nodes_in_relative_vertical, rng);
            }

            // CoSELayout.js 644-677
            for node_id in &vars.nodes_in_relative_horizontal {
                if vars.fixed_nodes_on_horizontal.has(node_id) {
                    continue;
                }
                let mut displacement = match vars.dummy_to_node_vertical.get(node_id) {
                    Some(actual) => {
                        model.nodes[node_of(&vars.id_to_node, first_id(actual))]
                            .displacement
                            .x
                    }
                    None => {
                        model.nodes[node_of(&vars.id_to_node, node_id)]
                            .displacement
                            .x
                    }
                };
                for edge in vars
                    .node_to_relative_horizontal
                    .get(node_id)
                    .expect("every relative node has a constraint list")
                {
                    match edge {
                        RelativeEdge::Forward { id, gap } => {
                            let diff = (value_or_nan(&vars.node_to_temp_position_horizontal, id)
                                - value_or_nan(&vars.node_to_temp_position_horizontal, node_id))
                                - displacement;
                            if diff < *gap {
                                displacement -= *gap - diff;
                            }
                        }
                        RelativeEdge::Backward { id, gap } => {
                            let diff =
                                (value_or_nan(&vars.node_to_temp_position_horizontal, node_id)
                                    - value_or_nan(&vars.node_to_temp_position_horizontal, id))
                                    + displacement;
                            if diff < *gap {
                                displacement += *gap - diff;
                            }
                        }
                    }
                }
                let temp =
                    value_or_nan(&vars.node_to_temp_position_horizontal, node_id) + displacement;
                vars.node_to_temp_position_horizontal.set(node_id, temp);
                match vars.dummy_to_node_vertical.get(node_id) {
                    Some(actual) => {
                        for member in actual {
                            model.nodes[node_of(&vars.id_to_node, member)]
                                .displacement
                                .x = displacement;
                        }
                    }
                    None => {
                        model.nodes[node_of(&vars.id_to_node, node_id)]
                            .displacement
                            .x = displacement;
                    }
                }
            }

            // CoSELayout.js 679-711. The source tests `fixedNodesOnHorizontal` here, not the
            // vertical set, and the port keeps that.
            for node_id in &vars.nodes_in_relative_vertical {
                if vars.fixed_nodes_on_horizontal.has(node_id) {
                    continue;
                }
                let mut displacement = match vars.dummy_to_node_horizontal.get(node_id) {
                    Some(actual) => {
                        model.nodes[node_of(&vars.id_to_node, first_id(actual))]
                            .displacement
                            .y
                    }
                    None => {
                        model.nodes[node_of(&vars.id_to_node, node_id)]
                            .displacement
                            .y
                    }
                };
                for edge in vars
                    .node_to_relative_vertical
                    .get(node_id)
                    .expect("every relative node has a constraint list")
                {
                    match edge {
                        RelativeEdge::Forward { id, gap } => {
                            let diff = (value_or_nan(&vars.node_to_temp_position_vertical, id)
                                - value_or_nan(&vars.node_to_temp_position_vertical, node_id))
                                - displacement;
                            if diff < *gap {
                                displacement -= *gap - diff;
                            }
                        }
                        RelativeEdge::Backward { id, gap } => {
                            let diff =
                                (value_or_nan(&vars.node_to_temp_position_vertical, node_id)
                                    - value_or_nan(&vars.node_to_temp_position_vertical, id))
                                    + displacement;
                            if diff < *gap {
                                displacement += *gap - diff;
                            }
                        }
                    }
                }
                let temp =
                    value_or_nan(&vars.node_to_temp_position_vertical, node_id) + displacement;
                vars.node_to_temp_position_vertical.set(node_id, temp);
                match vars.dummy_to_node_horizontal.get(node_id) {
                    Some(actual) => {
                        for member in actual {
                            model.nodes[node_of(&vars.id_to_node, member)]
                                .displacement
                                .y = displacement;
                        }
                    }
                    None => {
                        model.nodes[node_of(&vars.id_to_node, node_id)]
                            .displacement
                            .y = displacement;
                    }
                }
            }
        } else {
            // CoSELayout.js 714-797: every component takes the mean displacement of its nodes, or
            // zero when it has a fixed node.
            for (component, &fixed) in vars
                .components_on_horizontal
                .iter()
                .zip(vars.fixed_components_on_horizontal.iter())
            {
                if fixed {
                    for member in component {
                        match vars.dummy_to_node_vertical.get(member) {
                            Some(actual) => {
                                for node_id in actual {
                                    model.nodes[node_of(&vars.id_to_node, node_id)]
                                        .displacement
                                        .x = 0.0;
                                }
                            }
                            None => {
                                model.nodes[node_of(&vars.id_to_node, member)]
                                    .displacement
                                    .x = 0.0;
                            }
                        }
                    }
                } else {
                    let mut sum = 0.0;
                    let mut count = 0.0;
                    for member in component {
                        match vars.dummy_to_node_vertical.get(member) {
                            Some(actual) => {
                                sum += actual.len() as f64
                                    * model.nodes[node_of(&vars.id_to_node, first_id(actual))]
                                        .displacement
                                        .x;
                                count += actual.len() as f64;
                            }
                            None => {
                                sum += model.nodes[node_of(&vars.id_to_node, member)]
                                    .displacement
                                    .x;
                                count += 1.0;
                            }
                        }
                    }
                    let average = sum / count;
                    for member in component {
                        match vars.dummy_to_node_vertical.get(member) {
                            Some(actual) => {
                                for node_id in actual {
                                    model.nodes[node_of(&vars.id_to_node, node_id)]
                                        .displacement
                                        .x = average;
                                }
                            }
                            None => {
                                model.nodes[node_of(&vars.id_to_node, member)]
                                    .displacement
                                    .x = average;
                            }
                        }
                    }
                }
            }

            for (component, &fixed) in vars
                .components_on_vertical
                .iter()
                .zip(vars.fixed_components_on_vertical.iter())
            {
                if fixed {
                    for member in component {
                        match vars.dummy_to_node_horizontal.get(member) {
                            Some(actual) => {
                                for node_id in actual {
                                    model.nodes[node_of(&vars.id_to_node, node_id)]
                                        .displacement
                                        .y = 0.0;
                                }
                            }
                            None => {
                                model.nodes[node_of(&vars.id_to_node, member)]
                                    .displacement
                                    .y = 0.0;
                            }
                        }
                    }
                } else {
                    let mut sum = 0.0;
                    let mut count = 0.0;
                    for member in component {
                        match vars.dummy_to_node_horizontal.get(member) {
                            Some(actual) => {
                                sum += actual.len() as f64
                                    * model.nodes[node_of(&vars.id_to_node, first_id(actual))]
                                        .displacement
                                        .y;
                                count += actual.len() as f64;
                            }
                            None => {
                                sum += model.nodes[node_of(&vars.id_to_node, member)]
                                    .displacement
                                    .y;
                                count += 1.0;
                            }
                        }
                    }
                    let average = sum / count;
                    for member in component {
                        match vars.dummy_to_node_horizontal.get(member) {
                            Some(actual) => {
                                for node_id in actual {
                                    model.nodes[node_of(&vars.id_to_node, node_id)]
                                        .displacement
                                        .y = average;
                                }
                            }
                            None => {
                                model.nodes[node_of(&vars.id_to_node, member)]
                                    .displacement
                                    .y = average;
                            }
                        }
                    }
                }
            }
        }
    }
}
