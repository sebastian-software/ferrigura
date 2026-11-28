// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// The normalization algorithm is ported from D2 TALA.
// See NOTICE.md for the source revision and original copyright.

use serde::{Deserialize, Serialize};

pub use crate::Point;
use crate::route_output::{LAYOUT_STROKE_WIDTH, RoutedEdge, RoutedLabel};
use crate::{Error, NodeGeometry};

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    pub id: i64,
    pub top_left: Point,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Edge {
    pub id: i64,
    pub points: Vec<Point>,
    pub label_top_left: Option<Point>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_label: Option<RoutedLabel>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Geometry {
    pub fixed: bool,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Geometry {
    /// Translates the unpinned graph to TALA's origin. The source derives its label
    /// positions from translated paths. The captured label anchor carries the
    /// input used to compute the origin.
    ///
    /// Returns an error for fixed layouts, which use a different origin policy.
    pub fn normalize(&mut self) -> Result<(), &'static str> {
        if self.fixed {
            return Err("fixed positions are outside the TALA pilot");
        }

        let labels = self
            .edges
            .iter()
            .map(|edge| match edge.route_label {
                Some(label) => normalization_label(label, &edge.points)
                    .map(Some)
                    .map_err(|_| "labeled route has no nonzero leg"),
                None => Ok(edge.label_top_left),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let node_positions = self.nodes.iter().map(|node| node.top_left);
        let route_positions = self
            .edges
            .iter()
            .flat_map(|edge| edge.points.iter().copied())
            .chain(labels.into_iter().flatten());
        let Some(origin) = origin(node_positions, route_positions) else {
            return Ok(());
        };

        for node in &mut self.nodes {
            node.top_left.x -= origin.x;
            node.top_left.y -= origin.y;
        }
        for edge in &mut self.edges {
            shift(&mut edge.points, origin);
        }
        Ok(())
    }
}

/// Normalizes finished, unpinned routes and nodes before authored projection.
/// Go includes route-label boxes measured with the solver stroke width in the
/// origin. The renderer computes its actual label center after translation.
///
/// # Errors
/// Returns an error if a labeled route has no nonzero leg.
pub fn normalize_routed(nodes: &mut [NodeGeometry], edges: &mut [RoutedEdge]) -> Result<(), Error> {
    let labels = edges
        .iter()
        .filter_map(|edge| edge.label.map(|label| (label, &edge.points)))
        .map(|(label, points)| normalization_label(label, points))
        .collect::<Result<Vec<_>, _>>()?;
    let node_positions = nodes.iter().map(|node| Point {
        x: node.x,
        y: node.y,
    });
    let route_positions = edges
        .iter()
        .flat_map(|edge| edge.points.iter().copied())
        .chain(labels);
    if let Some(origin) = origin(node_positions, route_positions) {
        for node in nodes {
            node.x -= origin.x;
            node.y -= origin.y;
        }
        for edge in edges {
            shift(&mut edge.points, origin);
        }
    }
    Ok(())
}

fn normalization_label(label: RoutedLabel, points: &[Point]) -> Result<Point, Error> {
    label
        .top_left(points, LAYOUT_STROKE_WIDTH)
        .ok_or(Error::InvalidGraph)
}

fn origin(
    nodes: impl Iterator<Item = Point>,
    routes: impl Iterator<Item = Point>,
) -> Option<Point> {
    let routes = routes.map(|point| Point {
        x: point.x.floor(),
        y: point.y.floor(),
    });
    let minimum = nodes.chain(routes).fold(
        Point {
            x: f64::INFINITY,
            y: f64::INFINITY,
        },
        |minimum, point| Point {
            x: minimum.x.min(point.x),
            y: minimum.y.min(point.y),
        },
    );
    (minimum.x.is_finite() && minimum.y.is_finite()).then_some(minimum)
}

fn shift(points: &mut [Point], origin: Point) {
    for point in points {
        point.x -= origin.x;
        point.y -= origin.y;
    }
}
