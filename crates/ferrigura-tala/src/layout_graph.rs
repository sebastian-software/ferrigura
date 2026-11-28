// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Ordered solver input translated from Ferrigura's measured flowchart graph.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::{Error, Graph, Node};

const MAX_NODES: usize = 10_000;
const MAX_EDGES: usize = 50_000;
const MAX_DEPTH: usize = 256;
const MAX_MEASUREMENT: f64 = 1_000_000.0;
const FIRST_SPILL_ID: i64 = 1 << 32;
const LABEL_PADDING: f64 = 5.0;

mod prescale;
pub(crate) use prescale::port_dimensions;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum Direction {
    Top,
    Right,
    Bottom,
    Left,
}

impl Direction {
    fn parse(value: Option<&str>) -> Result<Option<Self>, Error> {
        let Some(value) = value.filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        let direction = match value.to_ascii_uppercase().as_str() {
            "TB" | "TD" => Self::Bottom,
            "BT" => Self::Top,
            "LR" => Self::Right,
            "RL" => Self::Left,
            _ => return Err(Error::InvalidGraph),
        };
        Ok(Some(direction))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum Shape {
    Square,
    Diamond,
    Circle,
    Oval,
    Cylinder,
    Hexagon,
}

impl Shape {
    pub(crate) fn parse(node: &Node) -> Result<Self, Error> {
        if node.is_group {
            return Ok(Self::Square);
        }
        match node.shape.as_str() {
            "" | "rect" | "rectangle" | "square" | "squareRect" | "rounded" | "round-rect"
            | "roundedRect" => Ok(Self::Square),
            "diam" | "diamond" | "question" => Ok(Self::Diamond),
            "circle" | "circ" => Ok(Self::Circle),
            "stadium" | "oval" | "pill" => Ok(Self::Oval),
            "cylinder" | "cyl" | "db" => Ok(Self::Cylinder),
            "hexagon" | "hex" => Ok(Self::Hexagon),
            _ => Err(Error::UnsupportedGraph),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct Label {
    pub text: String,
    pub width: f64,
    pub height: f64,
}

impl Label {
    fn measured(text: &str, width: f64, height: f64) -> Option<Self> {
        (!text.is_empty()).then(|| Self {
            text: text.to_owned(),
            width: width.ceil(),
            height: height.ceil(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LayoutNode {
    pub id: i64,
    pub input_index: usize,
    pub absolute_id: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub incident_edges: Vec<usize>,
    pub direction: Option<Direction>,
    pub shape: Shape,
    pub width: f64,
    pub height: f64,
    pub label: Option<Label>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LayoutEdge {
    pub id: i64,
    pub absolute_id: String,
    pub from: usize,
    pub to: usize,
    pub source_arrow: bool,
    pub target_arrow: bool,
    pub reversed_for_layout: bool,
    pub min_width: f64,
    pub min_height: f64,
    pub label: Option<Label>,
}

/// An owned graph. Nodes use D2 depth-first order and edges keep input order.
/// Each seed can own a clone of this state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct LayoutGraph {
    pub direction: Option<Direction>,
    pub roots: Vec<usize>,
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
}

impl LayoutGraph {
    /// Reserves space for measured shapes and the port demand of incident edges.
    fn prescale(&mut self) {
        prescale::apply(self);
    }

    /// Translates the measured graph into the state supplied to each seed.
    ///
    /// # Errors
    /// Rejects malformed measurements and topology, unsupported measured shapes,
    /// or input beyond the source engine's bounds.
    pub(crate) fn translate(graph: &Graph) -> Result<Self, Error> {
        if graph.nodes.len() > MAX_NODES || graph.edges.len() > MAX_EDGES {
            return Err(Error::WorkLimit);
        }
        let direction = Direction::parse(graph.direction.as_deref())?;
        let topology = Topology::new(graph)?;
        let object_keys: Vec<_> = graph
            .nodes
            .iter()
            .map(|node| object_key(&node.id))
            .collect();
        let mut nodes = Vec::<LayoutNode>::with_capacity(graph.nodes.len());
        let mut paths = Vec::<Vec<&str>>::with_capacity(graph.nodes.len());
        let mut slots = vec![0; graph.nodes.len()];
        let mut roots = Vec::new();

        for input_index in topology.preorder()? {
            let node = &graph.nodes[input_index];
            let slot = nodes.len();
            slots[input_index] = slot;
            let parent = topology.parents[input_index].map(|index| slots[index]);
            let mut path = parent.map_or_else(Vec::new, |index| paths[index].clone());
            path.push(&object_keys[input_index]);
            let absolute_id = path.join(".");
            paths.push(path);
            if let Some(parent) = parent {
                nodes[parent].children.push(slot);
            } else {
                roots.push(slot);
            }
            let width = group_side(node.is_group, node.width, node.label_width);
            let height = group_side(node.is_group, node.height, node.label_height);
            nodes.push(LayoutNode {
                id: 0,
                input_index,
                absolute_id,
                parent,
                children: Vec::new(),
                incident_edges: Vec::new(),
                direction: Direction::parse(node.direction.as_deref())?,
                shape: Shape::parse(node)?,
                width,
                height,
                label: Label::measured(&node.label, node.label_width, node.label_height),
            });
        }
        let ids = allocate_ids(nodes.iter().map(|node| node.absolute_id.as_bytes()))?;
        for (node, id) in nodes.iter_mut().zip(ids) {
            node.id = id;
        }

        let mut edge_ids = HashSet::with_capacity(graph.edges.len());
        let mut edges = Vec::with_capacity(graph.edges.len());
        for (index, edge) in graph.edges.iter().enumerate() {
            if edge.id.is_empty()
                || !edge_ids.insert(edge.id.as_str())
                || !valid_measurement(edge.label_width)
                || !valid_measurement(edge.label_height)
            {
                return Err(Error::InvalidGraph);
            }
            let source = slots[*topology
                .indices
                .get(edge.source.as_str())
                .ok_or(Error::InvalidGraph)?];
            let target = slots[*topology
                .indices
                .get(edge.target.as_str())
                .ok_or(Error::InvalidGraph)?];
            let absolute_id = edge_key(
                &paths[source],
                &paths[target],
                edge.source_arrow,
                edge.target_arrow,
                index,
            );
            let reversed_for_layout = edge.source_arrow && !edge.target_arrow;
            let (from, to, source_arrow, target_arrow) = if reversed_for_layout {
                (target, source, false, true)
            } else {
                (source, target, edge.source_arrow, edge.target_arrow)
            };
            nodes[from].incident_edges.push(index);
            if from != to {
                nodes[to].incident_edges.push(index);
            }
            edges.push(LayoutEdge {
                id: 0,
                absolute_id,
                from,
                to,
                source_arrow,
                target_arrow,
                reversed_for_layout,
                min_width: edge.label_width.ceil() + 2.0 * LABEL_PADDING,
                min_height: edge.label_height.ceil() + 2.0 * LABEL_PADDING,
                label: Label::measured(&edge.label, edge.label_width, edge.label_height),
            });
        }
        let ids = allocate_ids(edges.iter().map(|edge| edge.absolute_id.as_bytes()))?;
        for (edge, id) in edges.iter_mut().zip(ids) {
            edge.id = id;
        }
        Ok(Self {
            direction,
            roots,
            nodes,
            edges,
        })
    }
}

/// Captures the translated graph or its first placement stage for parity replay.
///
/// # Errors
/// Returns the same errors as measured input translation.
pub fn checkpoint(graph: &Graph, prescale: bool) -> Result<impl Serialize, Error> {
    let mut graph = LayoutGraph::translate(graph)?;
    if prescale {
        graph.prescale();
    }
    Ok(graph)
}

struct Topology<'a> {
    indices: HashMap<&'a str, usize>,
    parents: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
    roots: Vec<usize>,
}

impl<'a> Topology<'a> {
    fn new(graph: &'a Graph) -> Result<Self, Error> {
        let mut indices = HashMap::with_capacity(graph.nodes.len());
        for (index, node) in graph.nodes.iter().enumerate() {
            if node.id.is_empty()
                || indices.insert(node.id.as_str(), index).is_some()
                || !node.is_group && (node.width == 0.0 || node.height == 0.0)
                || ![node.width, node.height, node.label_width, node.label_height]
                    .into_iter()
                    .all(valid_measurement)
            {
                return Err(Error::InvalidGraph);
            }
        }

        let mut parents = Vec::with_capacity(graph.nodes.len());
        let mut children = vec![Vec::new(); graph.nodes.len()];
        let mut roots = Vec::new();
        for (index, node) in graph.nodes.iter().enumerate() {
            let parent = if let Some(id) = node.parent_id.as_deref().filter(|id| !id.is_empty()) {
                let parent = *indices.get(id).ok_or(Error::InvalidGraph)?;
                children[parent].push(index);
                Some(parent)
            } else {
                roots.push(index);
                None
            };
            parents.push(parent);
        }
        Ok(Self {
            indices,
            parents,
            children,
            roots,
        })
    }

    fn preorder(&self) -> Result<Vec<usize>, Error> {
        let mut stack: Vec<_> = self.roots.iter().rev().map(|&index| (index, 1)).collect();
        let mut order = Vec::with_capacity(self.parents.len());
        while let Some((index, depth)) = stack.pop() {
            if depth > MAX_DEPTH {
                return Err(Error::WorkLimit);
            }
            order.push(index);
            stack.extend(
                self.children[index]
                    .iter()
                    .rev()
                    .map(|&child| (child, depth + 1)),
            );
        }
        // A cycle cannot be reached from a root because each node has one parent.
        if order.len() != self.parents.len() {
            return Err(Error::InvalidGraph);
        }
        Ok(order)
    }
}

fn valid_measurement(value: f64) -> bool {
    (0.0..=MAX_MEASUREMENT).contains(&value)
}

fn group_side(is_group: bool, size: f64, label_size: f64) -> f64 {
    if is_group {
        size.max(label_size.ceil()).max(1.0)
    } else {
        size
    }
}

fn object_key(id: &str) -> String {
    let mut key = String::with_capacity(1 + id.len() * 2);
    key.push('n');
    for byte in id.bytes() {
        write!(key, "{byte:02x}").expect("writing to a string cannot fail");
    }
    key
}

fn edge_key(
    source: &[&str],
    target: &[&str],
    source_arrow: bool,
    target_arrow: bool,
    index: usize,
) -> String {
    let prefix = source
        .iter()
        .take(source.len().min(target.len()) - 1)
        .zip(target)
        .take_while(|(source, target)| source == target)
        .count();
    let arrow = match (source_arrow, target_arrow) {
        (false, false) => "--",
        (true, false) => "<-",
        (false, true) => "->",
        (true, true) => "<->",
    };
    let mut key = String::new();
    for component in &source[..prefix] {
        key.push_str(component);
        key.push('.');
    }
    write!(
        key,
        "({} {arrow} {})[{index}]",
        source[prefix..].join("."),
        target[prefix..].join(".")
    )
    .expect("writing to a string cannot fail");
    key
}

fn fnv32(bytes: &[u8]) -> u32 {
    bytes.iter().fold(2_166_136_261, |hash, &byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    })
}

fn allocate_ids<'a>(keys: impl IntoIterator<Item = &'a [u8]>) -> Result<Vec<i64>, Error> {
    let mut seen = HashSet::new();
    let mut buckets = HashMap::new();
    let mut hashed = Vec::new();
    for key in keys {
        if !seen.insert(key) {
            return Err(Error::InvalidGraph);
        }
        let hash = fnv32(key);
        *buckets.entry(hash).or_insert(0) += 1;
        hashed.push((hash, key));
    }
    let mut ids = vec![0; hashed.len()];
    let mut spills = Vec::new();
    for (index, (hash, key)) in hashed.into_iter().enumerate() {
        if hash != 0 && buckets[&hash] == 1 {
            ids[index] = i64::from(hash);
        } else {
            spills.push((hash, key, index));
        }
    }
    spills.sort_unstable();
    for (id, (_, _, index)) in (FIRST_SPILL_ID..).zip(spills) {
        ids[index] = id;
    }
    Ok(ids)
}

#[cfg(test)]
mod tests;
