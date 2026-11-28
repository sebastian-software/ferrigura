// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// The normalization algorithm is ported from D2 TALA.
// See NOTICE.md for the source revision and original copyright.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

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

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        for node in &self.nodes {
            min_x = min_x.min(node.top_left.x);
            min_y = min_y.min(node.top_left.y);
        }
        for edge in &self.edges {
            for point in edge.points.iter().chain(edge.label_top_left.iter()) {
                min_x = min_x.min(point.x.floor());
                min_y = min_y.min(point.y.floor());
            }
        }
        if !min_x.is_finite() || !min_y.is_finite() {
            return Ok(());
        }

        for node in &mut self.nodes {
            node.top_left.x -= min_x;
            node.top_left.y -= min_y;
        }
        for edge in &mut self.edges {
            for point in &mut edge.points {
                point.x -= min_x;
                point.y -= min_y;
            }
        }
        Ok(())
    }
}
