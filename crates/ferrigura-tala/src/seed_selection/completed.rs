// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, d2talalayout/{result,result_validate,limits}.go.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::Point;

pub mod metadata;

const MAX_RESULT_COORDINATE: f64 = 1_000_000_000.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dimensions {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct CompletedNode {
    pub id: i64,
    pub top_left: Option<Point>,
    pub size: Dimensions,
    pub label_size: Option<Dimensions>,
}

#[derive(Clone, Copy, Debug)]
pub struct CompletedEdge<'a> {
    pub id: i64,
    pub source: Option<usize>,
    pub target: Option<usize>,
    pub points: &'a [Point],
    pub label_percentage: f64,
    pub label_size: Option<Dimensions>,
    pub source_arrowhead_label_size: Option<Dimensions>,
    pub target_arrowhead_label_size: Option<Dimensions>,
}

/// The solver result before any mutation of the renderer's graph. Endpoints
/// refer to indices in `nodes`, while node and edge identities survive
/// workspace reorderings and are checked independently.
#[derive(Debug)]
pub struct CompletedGraph<'a> {
    pub nodes: &'a [CompletedNode],
    pub edges: &'a [CompletedEdge<'a>],
}

/// Identities from the immutable seed input, before workspace graph rewrites.
#[derive(Debug)]
pub struct AuthoredTopology<'a> {
    pub node_ids: &'a [i64],
    pub edges: &'a [AuthoredEdge],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthoredEdge {
    pub id: i64,
    pub source: i64,
    pub target: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationError {
    Cancelled,
    DuplicateNode(i64),
    DuplicateEdge(i64),
    MissingNodePosition(i64),
    InvalidNodeGeometry {
        id: i64,
        field: &'static str,
    },
    ForeignEndpoint {
        id: i64,
        field: &'static str,
    },
    IncompleteRoute(i64),
    InvalidRoutePoint {
        id: i64,
        index: usize,
        axis: char,
    },
    InvalidEdgeGeometry {
        id: i64,
        field: &'static str,
    },
    NodeCount {
        expected: usize,
        actual: usize,
    },
    DuplicateAuthoredNode(i64),
    DuplicateAuthoredEdge(i64),
    UnexpectedNode(i64),
    UnexpectedEdge(i64),
    ChangedEndpoints {
        expected: AuthoredEdge,
        actual: AuthoredEdge,
    },
    MissingEdge(i64),
    ChangedGraphMetadata(&'static str),
    ChangedNodeMetadata {
        id: i64,
        field: &'static str,
    },
    ChangedEdgeMetadata {
        id: i64,
        field: &'static str,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("TALA seed result validation was cancelled"),
            Self::DuplicateNode(id) => write!(f, "node ID {id} is duplicated"),
            Self::DuplicateEdge(id) => write!(f, "edge ID {id} is duplicated"),
            Self::MissingNodePosition(id) => write!(f, "node {id} has no position"),
            Self::InvalidNodeGeometry { id, field } => {
                write!(f, "node {id}: {field} is outside the supported range")
            }
            Self::ForeignEndpoint { id, field } => {
                write!(f, "edge {id} has a {field} outside the result graph")
            }
            Self::IncompleteRoute(id) => write!(f, "edge {id} has an incomplete route"),
            Self::InvalidRoutePoint { id, index, axis } => {
                write!(
                    f,
                    "edge {id} point {index}: route {axis} is outside the supported range"
                )
            }
            Self::InvalidEdgeGeometry { id, field } => {
                write!(f, "edge {id}: {field} is outside the supported range")
            }
            Self::NodeCount { expected, actual } => {
                write!(f, "node count changed from {expected} to {actual}")
            }
            Self::DuplicateAuthoredNode(id) => write!(f, "input contains duplicate node ID {id}"),
            Self::DuplicateAuthoredEdge(id) => write!(f, "input contains duplicate edge ID {id}"),
            Self::UnexpectedNode(id) => write!(f, "result contains unexpected node ID {id}"),
            Self::UnexpectedEdge(id) => write!(f, "result contains unexpected edge ID {id}"),
            Self::ChangedEndpoints { expected, actual } => write!(
                f,
                "edge {} endpoints changed from {}->{} to {}->{}",
                actual.id, expected.source, expected.target, actual.source, actual.target
            ),
            Self::MissingEdge(id) => write!(f, "result is missing edge ID {id}"),
            Self::ChangedGraphMetadata(field) => write!(f, "{field} metadata changed"),
            Self::ChangedNodeMetadata { id, field } => write!(f, "node {id} {field} changed"),
            Self::ChangedEdgeMetadata { id, field } => write!(f, "edge {id} {field} changed"),
        }
    }
}

impl std::error::Error for ValidationError {}

impl CompletedGraph<'_> {
    /// Validates raw node and route geometry before scoring. Raw measurements
    /// and label percentages here precede any conversion for the renderer.
    ///
    /// # Errors
    /// Rejects cancellation or the first invalid record in solver order.
    pub fn validate(&self, cancelled: &dyn Fn() -> bool) -> Result<(), ValidationError> {
        check_cancelled(cancelled)?;
        let mut nodes = HashSet::with_capacity(self.nodes.len());
        for node in self.nodes {
            check_cancelled(cancelled)?;
            if !nodes.insert(node.id) {
                return Err(ValidationError::DuplicateNode(node.id));
            }
            validate_node(*node)?;
        }

        let mut edges = HashSet::with_capacity(self.edges.len());
        for edge in self.edges {
            check_cancelled(cancelled)?;
            if !edges.insert(edge.id) {
                return Err(ValidationError::DuplicateEdge(edge.id));
            }
            self.endpoint(edge.id, edge.source, "source")?;
            self.endpoint(edge.id, edge.target, "destination")?;
            validate_edge(*edge, cancelled)?;
        }
        Ok(())
    }

    /// Validates geometry, then checks identities and endpoints against the
    /// authored input. `sequence_edges` comes from grouping's derivation of
    /// legal sequence transitions; returned IDs are transitions actually
    /// absent from the candidate and must be omitted when it is applied.
    ///
    /// # Errors
    /// Rejects cancellation, invalid geometry, or a changed input record.
    pub fn validate_topology(
        &self,
        expected: &AuthoredTopology<'_>,
        mut sequence_edges: HashSet<i64>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<HashSet<i64>, ValidationError> {
        self.validate(cancelled)?;
        if expected.node_ids.len() != self.nodes.len() {
            return Err(ValidationError::NodeCount {
                expected: expected.node_ids.len(),
                actual: self.nodes.len(),
            });
        }
        let mut expected_nodes = HashSet::with_capacity(expected.node_ids.len());
        for &id in expected.node_ids {
            check_cancelled(cancelled)?;
            if !expected_nodes.insert(id) {
                return Err(ValidationError::DuplicateAuthoredNode(id));
            }
        }
        for node in self.nodes {
            check_cancelled(cancelled)?;
            if !expected_nodes.contains(&node.id) {
                return Err(ValidationError::UnexpectedNode(node.id));
            }
        }
        let mut expected_edges = HashMap::with_capacity(expected.edges.len());
        for edge in expected.edges {
            check_cancelled(cancelled)?;
            if expected_edges.insert(edge.id, *edge).is_some() {
                return Err(ValidationError::DuplicateAuthoredEdge(edge.id));
            }
        }
        let mut actual_edges = HashSet::with_capacity(self.edges.len());
        for edge in self.edges {
            check_cancelled(cancelled)?;
            let expected = expected_edges
                .get(&edge.id)
                .ok_or(ValidationError::UnexpectedEdge(edge.id))?;
            let actual = AuthoredEdge {
                id: edge.id,
                source: self.endpoint(edge.id, edge.source, "source")?.id,
                target: self.endpoint(edge.id, edge.target, "destination")?.id,
            };
            if actual != *expected {
                return Err(ValidationError::ChangedEndpoints {
                    expected: *expected,
                    actual,
                });
            }
            actual_edges.insert(edge.id);
            sequence_edges.remove(&edge.id);
        }
        for edge in expected.edges {
            check_cancelled(cancelled)?;
            if !actual_edges.contains(&edge.id) && !sequence_edges.contains(&edge.id) {
                return Err(ValidationError::MissingEdge(edge.id));
            }
        }
        Ok(sequence_edges)
    }

    fn endpoint(
        &self,
        id: i64,
        index: Option<usize>,
        field: &'static str,
    ) -> Result<&CompletedNode, ValidationError> {
        index
            .and_then(|index| self.nodes.get(index))
            .ok_or(ValidationError::ForeignEndpoint { id, field })
    }
}

fn check_cancelled(cancelled: &dyn Fn() -> bool) -> Result<(), ValidationError> {
    if cancelled() {
        return Err(ValidationError::Cancelled);
    }
    Ok(())
}

fn validate_node(node: CompletedNode) -> Result<(), ValidationError> {
    let invalid = |field| ValidationError::InvalidNodeGeometry { id: node.id, field };
    let position = node
        .top_left
        .ok_or(ValidationError::MissingNodePosition(node.id))?;
    for (value, field) in [(position.x, "node x"), (position.y, "node y")] {
        if !valid_coordinate(value) {
            return Err(invalid(field));
        }
    }
    for (value, field) in [
        (node.size.width, "node width"),
        (node.size.height, "node height"),
    ] {
        if !valid_dimension(value, false) {
            return Err(invalid(field));
        }
    }
    if !(position.x + node.size.width).is_finite() || !(position.y + node.size.height).is_finite() {
        return Err(invalid("node bounds"));
    }
    validate_label(node.label_size, "node label width", "node label height").map_err(invalid)
}

fn validate_edge(
    edge: CompletedEdge<'_>,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), ValidationError> {
    if edge.points.len() < 2 {
        return Err(ValidationError::IncompleteRoute(edge.id));
    }
    for (index, point) in edge.points.iter().enumerate() {
        if index % 256 == 0 {
            check_cancelled(cancelled)?;
        }
        for (value, axis) in [(point.x, 'x'), (point.y, 'y')] {
            if !valid_coordinate(value) {
                return Err(ValidationError::InvalidRoutePoint {
                    id: edge.id,
                    index,
                    axis,
                });
            }
        }
    }
    let invalid = |field| ValidationError::InvalidEdgeGeometry { id: edge.id, field };
    if !(0.0..=1.0).contains(&edge.label_percentage) {
        return Err(invalid("label percentage"));
    }
    for (size, width, height) in [
        (edge.label_size, "label width", "label height"),
        (
            edge.source_arrowhead_label_size,
            "source arrowhead label width",
            "source arrowhead label height",
        ),
        (
            edge.target_arrowhead_label_size,
            "target arrowhead label width",
            "target arrowhead label height",
        ),
    ] {
        validate_label(size, width, height).map_err(invalid)?;
    }
    Ok(())
}

fn validate_label(
    size: Option<Dimensions>,
    width: &'static str,
    height: &'static str,
) -> Result<(), &'static str> {
    if let Some(size) = size {
        if !valid_dimension(size.width, true) {
            return Err(width);
        }
        if !valid_dimension(size.height, true) {
            return Err(height);
        }
    }
    Ok(())
}

fn valid_coordinate(value: f64) -> bool {
    value.abs() <= MAX_RESULT_COORDINATE
}

fn valid_dimension(value: f64, zero_allowed: bool) -> bool {
    (0.0..=MAX_RESULT_COORDINATE).contains(&value) && (zero_allowed || value != 0.0)
}

#[cfg(test)]
mod tests;
