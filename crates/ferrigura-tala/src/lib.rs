// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The experimental Ferrigura port of D2's TALA engine.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

mod connected;
mod hierarchy;
#[doc(hidden)]
pub mod normalize;
mod placement;
mod routing;
#[doc(hidden)]
pub mod simplify;

/// Input measured by the renderer. Positions in the output share one absolute coordinate system.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    pub direction: Option<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub parent_id: Option<String>,
    pub direction: Option<String>,
    pub is_group: bool,
    pub shape: String,
    pub width: f64,
    pub height: f64,
    pub label: String,
    pub label_width: f64,
    pub label_height: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_arrow: bool,
    pub target_arrow: bool,
    pub label: String,
    pub label_width: f64,
    pub label_height: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NodeGeometry {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EdgeGeometry {
    pub id: String,
    pub points: Vec<Point>,
    pub label: Option<Point>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Layout {
    pub nodes: Vec<NodeGeometry>,
    pub edges: Vec<EdgeGeometry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    UnsupportedGraph,
    InvalidGraph,
    WorkLimit,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedGraph => formatter.write_str(
                "this graph is outside the experimental TALA port; select ELK for this diagram",
            ),
            Self::InvalidGraph => formatter.write_str("invalid measured graph for TALA"),
            Self::WorkLimit => {
                formatter.write_str("experimental TALA input exceeds its work limit")
            }
        }
    }
}

impl std::error::Error for Error {}

// This limit keeps each graph admitted by this experimental stage small. Broader topology
// will need its own solver work counters rather than increasing these limits alone.
const MAX_NODES: usize = 32;
const MAX_EDGES: usize = 32;

fn valid_direction(direction: Option<&str>) -> bool {
    matches!(
        direction,
        None | Some("" | "TB" | "TD" | "BT" | "LR" | "RL")
    )
}

fn valid_dimension(value: f64) -> bool {
    (0.0..=4_096.0).contains(&value)
}

/// Runs the native layout covered by this build's differential corpus.
///
/// # Errors
/// Returns an error for invalid measurement, graphs over the pilot work limit, and topology
/// whose placement or routing has not yet been ported.
pub fn layout(graph: &Graph) -> Result<Layout, Error> {
    if graph.nodes.len() > MAX_NODES || graph.edges.len() > MAX_EDGES {
        return Err(Error::WorkLimit);
    }

    if !valid_direction(graph.direction.as_deref()) {
        return Err(Error::InvalidGraph);
    }

    let mut node_ids = HashSet::with_capacity(graph.nodes.len());
    for node in &graph.nodes {
        if !node.is_group && !matches!(node.shape.as_str(), "squareRect" | "rect" | "rectangle") {
            return Err(Error::UnsupportedGraph);
        }
        if node.id.is_empty()
            || !node_ids.insert(node.id.as_str())
            || !valid_direction(node.direction.as_deref())
            || !node.is_group && (node.width == 0.0 || node.height == 0.0)
            || !valid_dimension(node.width)
            || !valid_dimension(node.height)
            || !valid_dimension(node.label_width)
            || !valid_dimension(node.label_height)
        {
            return Err(Error::InvalidGraph);
        }
    }
    let mut edge_ids = HashSet::with_capacity(graph.edges.len());
    for edge in &graph.edges {
        if edge.id.is_empty()
            || !edge_ids.insert(edge.id.as_str())
            || !node_ids.contains(edge.source.as_str())
            || !node_ids.contains(edge.target.as_str())
            || !valid_dimension(edge.label_width)
            || !valid_dimension(edge.label_height)
        {
            return Err(Error::InvalidGraph);
        }
    }

    if graph
        .nodes
        .iter()
        .any(|node| node.is_group || node.parent_id.is_some())
    {
        let layout = hierarchy::layout(graph).ok_or(Error::UnsupportedGraph)?;
        check_bounds(&layout.nodes)?;
        return Ok(layout);
    }

    if let [node] = graph.nodes.as_slice()
        && !graph.edges.is_empty()
    {
        let mut geometry = NodeGeometry {
            id: node.id.clone(),
            x: 0.0,
            y: 0.0,
            width: node.width,
            height: node.height,
        };
        let edges = match graph.edges.as_slice() {
            [edge] => vec![routing::isolated_rectangle_loop(&mut geometry, edge)],
            edges if edges.iter().all(|edge| edge.label.is_empty()) => {
                routing::isolated_unlabeled_loops(&mut geometry, edges)
            }
            _ => return Err(Error::UnsupportedGraph),
        };
        return Ok(Layout {
            nodes: vec![geometry],
            edges,
        });
    }
    if !graph.edges.is_empty() {
        return connected::layout(graph).ok_or(Error::UnsupportedGraph);
    }
    let nodes = placement::place(graph).ok_or(Error::UnsupportedGraph)?;
    check_bounds(&nodes)?;
    Ok(Layout {
        nodes,
        edges: Vec::new(),
    })
}

fn check_bounds(nodes: &[NodeGeometry]) -> Result<(), Error> {
    if nodes
        .iter()
        .any(|node| node.x + node.width > 30_000.0 || node.y + node.height > 30_000.0)
    {
        return Err(Error::WorkLimit);
    }
    Ok(())
}
