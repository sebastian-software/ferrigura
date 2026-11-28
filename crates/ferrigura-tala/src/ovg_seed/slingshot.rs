// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/slingshot.go and internal/layoutgraph/node.go.

use std::collections::HashSet;

use crate::{Error, NodeGeometry, Point};

use super::{LinkedVisibility, Vertex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    TopLeft,
    Top,
    TopRight,
    Left,
    Overlap,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Orientation {
    /// Finds the orientation of the first rectangle relative to the second.
    #[must_use]
    pub fn between(source: &NodeGeometry, target: &NodeGeometry) -> Self {
        let left = source.x + source.width < target.x;
        let right = target.x + target.width < source.x;
        let top = source.y + source.height < target.y;
        let bottom = target.y + target.height < source.y;
        match (top, bottom, left, right) {
            (true, _, true, _) => Self::TopLeft,
            (true, _, _, true) => Self::TopRight,
            (true, _, _, _) => Self::Top,
            (_, true, true, _) => Self::BottomLeft,
            (_, true, _, true) => Self::BottomRight,
            (_, true, _, _) => Self::Bottom,
            (_, _, _, true) => Self::Right,
            (_, _, true, _) => Self::Left,
            _ => Self::Overlap,
        }
    }

    #[must_use]
    pub fn diagonal(self) -> Option<Diagonal> {
        match self {
            Self::TopLeft => Some(Diagonal::TopLeft),
            Self::TopRight => Some(Diagonal::TopRight),
            Self::BottomLeft => Some(Diagonal::BottomLeft),
            Self::BottomRight => Some(Diagonal::BottomRight),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Diagonal {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Diagonal {
    pub(super) fn opposite(self) -> Self {
        match self {
            Self::TopLeft => Self::BottomRight,
            Self::TopRight => Self::BottomLeft,
            Self::BottomLeft => Self::TopRight,
            Self::BottomRight => Self::TopLeft,
        }
    }

    fn left(self) -> bool {
        matches!(self, Self::TopLeft | Self::BottomLeft)
    }

    fn top(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopRight)
    }

    /// Finds the orientation preference using incident neighbors on each end
    /// in their reference geometry. Repeated edges contribute separately.
    #[must_use]
    pub fn prefer_launching_vertically(
        self,
        source_id: i64,
        target_id: i64,
        source_neighbors: impl IntoIterator<Item = Orientation>,
        target_neighbors: impl IntoIterator<Item = Orientation>,
    ) -> (bool, bool) {
        use Orientation::{Bottom, BottomLeft, BottomRight, Left, Right, Top, TopLeft, TopRight};
        let (vertical_source, vertical_target, horizontal_source, horizontal_target) = match self {
            Self::TopLeft => (
                [BottomLeft, Bottom],
                [Left, BottomLeft],
                [TopRight, Right],
                [Top, TopRight],
            ),
            Self::TopRight => (
                [BottomRight, Bottom],
                [Right, BottomRight],
                [TopLeft, Left],
                [Top, TopLeft],
            ),
            Self::BottomLeft => (
                [TopLeft, Top],
                [Left, TopLeft],
                [BottomRight, Right],
                [Bottom, BottomRight],
            ),
            Self::BottomRight => (
                [TopRight, Top],
                [Right, TopRight],
                [BottomLeft, Left],
                [Bottom, BottomLeft],
            ),
        };
        let mut vertical = 0;
        let mut horizontal = 0;
        for direction in source_neighbors {
            vertical += usize::from(vertical_source.contains(&direction));
            horizontal += usize::from(horizontal_source.contains(&direction));
        }
        for direction in target_neighbors {
            vertical += usize::from(vertical_target.contains(&direction));
            horizontal += usize::from(horizontal_target.contains(&direction));
        }
        if vertical == horizontal {
            (source_id < target_id, false)
        } else {
            (vertical < horizontal, true)
        }
    }

    #[must_use]
    pub fn flight(self, vertical_launch: bool) -> Flight {
        let positive = if vertical_launch {
            self.top()
        } else {
            self.left()
        };
        let (launch_side, landing_side) = if vertical_launch {
            (
                if self.top() { 4 } else { 1 },
                if self.left() { 2 } else { 8 },
            )
        } else {
            (
                if self.left() { 8 } else { 2 },
                if self.top() { 1 } else { 4 },
            )
        };
        Flight {
            vertical_launch,
            positive,
            launch_side,
            landing_side,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Flight {
    vertical_launch: bool,
    positive: bool,
    launch_side: u8,
    landing_side: u8,
}

impl Flight {
    fn axes(self, point: Point) -> [f64; 2] {
        if self.vertical_launch {
            [point.y, point.x]
        } else {
            [point.x, point.y]
        }
    }

    fn bounds(self, target: &NodeGeometry) -> [f64; 2] {
        if self.vertical_launch {
            [target.y, target.y + target.height]
        } else {
            [target.x, target.x + target.width]
        }
    }

    #[must_use]
    pub fn on_flight_plan(self, from: Point, to: Point) -> bool {
        let [progress, line] = self.axes(from);
        let [next, next_line] = self.axes(to);
        line == next_line
            && if self.positive {
                next > progress
            } else {
                next < progress
            }
    }

    #[must_use]
    pub fn undershot(self, point: Point, target: &NodeGeometry) -> bool {
        let [first, last] = self.bounds(target);
        let [progress, _] = self.axes(point);
        if self.positive {
            progress < first
        } else {
            progress > last
        }
    }

    #[must_use]
    pub fn overshot(self, point: Point, target: &NodeGeometry, l_shape: bool) -> bool {
        let [first, last] = self.bounds(target);
        let [progress, _] = self.axes(point);
        match (self.positive, l_shape) {
            (true, true) => progress > last,
            (true, false) => progress > first,
            (false, true) => progress < first,
            (false, false) => progress < last,
        }
    }

    /// Finds launch candidates in the original port-slot order.
    pub fn launch_ports(self, adjacency: &VisibilityAdjacency<'_>, owner: usize) -> Vec<usize> {
        ports_with_side(adjacency.graph, owner, self.launch_side)
    }

    /// Finds landing candidates in the original port-slot order.
    pub fn landing_ports(self, adjacency: &VisibilityAdjacency<'_>, owner: usize) -> Vec<usize> {
        ports_with_side(adjacency.graph, owner, self.landing_side)
    }
}

fn ports_with_side(graph: &LinkedVisibility, owner: usize, side: u8) -> Vec<usize> {
    let ports = &graph.ports_by_node[owner];
    let mut seen = HashSet::with_capacity(ports.len());
    let mut result = Vec::new();
    for &index in ports {
        if !seen.insert(index) {
            continue;
        }
        if graph.vertices[index]
            .owners
            .iter()
            .any(|metadata| metadata.node == owner && metadata.sides & side != 0)
        {
            result.push(index);
        }
    }
    result
}

/// The visibility graph with incident links in their reference insertion order.
pub struct VisibilityAdjacency<'a> {
    graph: &'a LinkedVisibility,
    incidents: Vec<Vec<Incident>>,
}

#[derive(Clone, Copy)]
pub struct Incident {
    pub link: usize,
    pub neighbor: usize,
}

impl<'a> VisibilityAdjacency<'a> {
    /// Builds ordered adjacency for a single visibility graph.
    ///
    /// # Errors
    /// Returns an invalid graph error for nonexistent link endpoints.
    pub fn new(graph: &'a LinkedVisibility) -> Result<Self, Error> {
        let mut incidents = vec![Vec::new(); graph.vertices.len()];
        for (link, &[from, to]) in graph.links.iter().enumerate() {
            incidents
                .get_mut(from)
                .ok_or(Error::InvalidGraph)?
                .push(Incident { link, neighbor: to });
            incidents
                .get_mut(to)
                .ok_or(Error::InvalidGraph)?
                .push(Incident {
                    link,
                    neighbor: from,
                });
        }
        Ok(Self { graph, incidents })
    }

    #[must_use]
    pub fn vertex(&self, index: usize) -> &Vertex {
        &self.graph.vertices[index]
    }

    #[must_use]
    pub fn incidents(&self, index: usize) -> &[Incident] {
        &self.incidents[index]
    }

    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.graph.vertices.len()
    }

    /// Follows the first forward axis-aligned neighbor at each vertex. The
    /// returned path includes the source and stops short of the destination.
    ///
    /// # Errors
    /// Returns an invalid graph error for nonexistent vertices and a work
    /// limit if traversing the path exceeds the remaining budget.
    pub fn fill_path(
        &self,
        source: usize,
        target: usize,
        work: &mut usize,
    ) -> Result<Option<Vec<usize>>, Error> {
        let from = self
            .graph
            .vertices
            .get(source)
            .ok_or(Error::InvalidGraph)?
            .point;
        let to = self
            .graph
            .vertices
            .get(target)
            .ok_or(Error::InvalidGraph)?
            .point;
        let mut path = vec![source];
        let mut current = source;
        loop {
            step(work)?;
            let point = self.vertex(current).point;
            let mut next = current;
            for &Incident {
                neighbor: adjacent, ..
            } in self.incidents(current)
            {
                step(work)?;
                let adjacent_point = self.vertex(adjacent).point;
                let on_path = if from.x == to.x && adjacent_point.x == point.x {
                    from.y < to.y && point.y < adjacent_point.y
                        || from.y > to.y && point.y > adjacent_point.y
                } else if from.y == to.y && adjacent_point.y == point.y {
                    from.x < to.x && point.x < adjacent_point.x
                        || from.x > to.x && point.x > adjacent_point.x
                } else {
                    false
                };
                if on_path {
                    next = adjacent;
                    break;
                }
            }
            if next == current {
                return Ok(None);
            }
            if next == target {
                return Ok(Some(path));
            }
            path.push(next);
            current = next;
        }
    }
}

pub(super) fn step(work: &mut usize) -> Result<(), Error> {
    *work = work.checked_sub(1).ok_or(Error::WorkLimit)?;
    Ok(())
}

#[cfg(test)]
mod tests;
