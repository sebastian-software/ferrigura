// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde:
// d2talalayout/{patch,result_validate}.go, d2target/d2target.go,
// lib/label/label.go and lib/geo/{math,point,route,vector}.go.

use serde::{Deserialize, Serialize};

use crate::{EdgeGeometry, Error, Layout, NodeGeometry, Point};

const MAX_RESULT_COORDINATE: f64 = 1_000_000_000.0;
const LABEL_PADDING: f64 = 5.0;
const RENDER_STROKE_WIDTH: f64 = 2.0;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Anchor {
    InsideMiddleLeft,
    InsideMiddleCenter,
    InsideMiddleRight,
    OutsideTopLeft,
    OutsideTopCenter,
    OutsideTopRight,
    OutsideBottomLeft,
    OutsideBottomCenter,
    OutsideBottomRight,
    UnlockedTop,
    UnlockedMiddle,
    UnlockedBottom,
}

impl Anchor {
    pub(crate) fn mirrored(self) -> Self {
        match self {
            Self::InsideMiddleLeft => Self::InsideMiddleRight,
            Self::InsideMiddleRight => Self::InsideMiddleLeft,
            Self::OutsideTopLeft => Self::OutsideBottomRight,
            Self::OutsideTopCenter => Self::OutsideBottomCenter,
            Self::OutsideTopRight => Self::OutsideBottomLeft,
            Self::OutsideBottomLeft => Self::OutsideTopRight,
            Self::OutsideBottomCenter => Self::OutsideTopCenter,
            Self::OutsideBottomRight => Self::OutsideTopLeft,
            Self::UnlockedTop => Self::UnlockedBottom,
            Self::UnlockedBottom => Self::UnlockedTop,
            Self::InsideMiddleCenter | Self::UnlockedMiddle => self,
        }
    }

    fn fraction_and_normal(self, percentage: f64) -> (f64, f64) {
        match self {
            Self::InsideMiddleLeft => (0.25, 0.0),
            Self::InsideMiddleCenter => (0.5, 0.0),
            Self::InsideMiddleRight => (0.75, 0.0),
            Self::OutsideTopLeft => (0.25, -1.0),
            Self::OutsideTopCenter => (0.5, -1.0),
            Self::OutsideTopRight => (0.75, -1.0),
            Self::OutsideBottomLeft => (0.25, 1.0),
            Self::OutsideBottomCenter => (0.5, 1.0),
            Self::OutsideBottomRight => (0.75, 1.0),
            Self::UnlockedTop => (percentage, -1.0),
            Self::UnlockedMiddle => (percentage, 0.0),
            Self::UnlockedBottom => (percentage, 1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct RoutedLabel {
    pub anchor: Anchor,
    pub percentage: f64,
    pub width: f64,
    pub height: f64,
}

impl RoutedLabel {
    /// Finds the label box on an already routed edge. Coordinates are rounded by
    /// the label library after the half-size is subtracted from the center.
    pub(crate) fn top_left(self, points: &[Point], stroke_width: f64) -> Option<Point> {
        let (fraction, normal) = self.anchor.fraction_and_normal(self.percentage);
        let total: f64 = segments(points).map(|(_, _, length)| length).sum();
        let mut remaining = fraction * total;
        let mut last = None;
        for (from, to, length) in segments(points) {
            if remaining <= length {
                return Some(self.top_left_on_segment(
                    from,
                    to,
                    length,
                    remaining / length,
                    normal,
                    stroke_width,
                ));
            }
            remaining -= length;
            last = Some((from, to, length));
        }
        // The same summation order as Go can leave a residual at the endpoint.
        last.map(|(from, to, length)| {
            self.top_left_on_segment(
                from,
                to,
                length,
                1.0 + remaining / length,
                normal,
                stroke_width,
            )
        })
    }

    fn top_left_on_segment(
        self,
        from: Point,
        to: Point,
        length: f64,
        fraction: f64,
        normal: f64,
        stroke_width: f64,
    ) -> Point {
        let mut x = from.x * (1.0 - fraction) + to.x * fraction;
        let mut y = from.y * (1.0 - fraction) + to.y * fraction;
        if normal != 0.0 {
            let offset_x = stroke_width / 2.0 + LABEL_PADDING + self.width / 2.0;
            let offset_y = stroke_width / 2.0 + LABEL_PADDING + self.height / 2.0;
            x += (from.y - to.y) / length * normal * offset_x;
            y += (to.x - from.x) / length * normal * offset_y;
        }
        Point {
            x: round_label_coordinate(x - self.width / 2.0),
            y: round_label_coordinate(y - self.height / 2.0),
        }
    }
}

fn segments(points: &[Point]) -> impl Iterator<Item = (Point, Point, f64)> + '_ {
    points.windows(2).filter_map(|pair| {
        let [from, to] = [pair[0], pair[1]];
        let x = to.x - from.x;
        let y = to.y - from.y;
        let length = if x == 0.0 {
            y.abs()
        } else if y == 0.0 {
            x.abs()
        } else {
            (x * x + y * y).sqrt()
        };
        (length != 0.0).then_some((from, to, length))
    })
}

fn round_label_coordinate(value: f64) -> f64 {
    let rounded = (f64::from((value * 10_000.0) as f32) / 10_000.0).round();
    if rounded == 0.0 { 0.0 } else { rounded }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutedEdge {
    pub id: String,
    pub points: Vec<Point>,
    pub label: Option<RoutedLabel>,
    pub reversed_for_layout: bool,
}

impl RoutedEdge {
    pub(crate) fn into_geometry(mut self) -> Result<EdgeGeometry, Error> {
        if self.points.iter().any(|point| {
            point.x.abs() > MAX_RESULT_COORDINATE
                || point.y.abs() > MAX_RESULT_COORDINATE
                || !point.x.is_finite()
                || !point.y.is_finite()
        }) || self.label.is_some_and(|label| {
            !(0.0..=1.0).contains(&label.percentage)
                || !(0.0..=MAX_RESULT_COORDINATE).contains(&label.width)
                || !(0.0..=MAX_RESULT_COORDINATE).contains(&label.height)
        }) {
            return Err(Error::InvalidGraph);
        }
        if self.reversed_for_layout {
            self.points.reverse();
            if let Some(label) = &mut self.label {
                label.anchor = label.anchor.mirrored();
                label.percentage = 1.0 - label.percentage;
            }
        }
        self.points.dedup();
        if self.points.len() < 2 {
            return Err(Error::InvalidGraph);
        }
        let label = match self.label {
            Some(mut label) => {
                label.width = label.width.trunc();
                label.height = label.height.trunc();
                let point = label
                    .top_left(&self.points, RENDER_STROKE_WIDTH)
                    .ok_or(Error::InvalidGraph)?;
                Some(Point {
                    x: point.x + label.width / 2.0,
                    y: point.y + label.height / 2.0,
                })
            }
            None => None,
        };
        Ok(EdgeGeometry {
            id: self.id,
            points: self.points,
            label,
        })
    }
}

/// Applies the authored route and label projection to completed polyline edges.
///
/// # Errors
/// Returns an error when a route or its caption is invalid or degenerate.
pub fn project(edges: Vec<RoutedEdge>) -> Result<Vec<EdgeGeometry>, Error> {
    edges.into_iter().map(RoutedEdge::into_geometry).collect()
}

/// Normalizes finished boxes and routes, then applies their authored traversal and label anchors.
/// # Errors
/// Returns an error if a route or its caption cannot be projected into Ferrigura geometry.
pub fn finalize(mut nodes: Vec<NodeGeometry>, mut edges: Vec<RoutedEdge>) -> Result<Layout, Error> {
    crate::normalize::normalize_routed(&mut nodes, &mut edges)?;
    Ok(Layout {
        nodes,
        edges: project(edges)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Case {
        input: RoutedEdge,
        expected: EdgeGeometry,
    }

    #[test]
    fn projects_the_pinned_go_winner_in_authored_order() {
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("route_output/fixtures.json")).unwrap();
        for case in cases {
            let actual = case.input.into_geometry().unwrap();
            assert_eq!(actual, case.expected);
        }
    }

    #[test]
    fn removes_zero_length_legs_without_removing_a_later_visit() {
        let a = Point { x: 10.0, y: 20.0 };
        let b = Point { x: 10.0, y: 40.0 };
        let c = Point { x: 50.0, y: 40.0 };
        for (reversed_for_layout, expected) in [(false, [a, b, a, c]), (true, [c, a, b, a])] {
            let route = RoutedEdge {
                id: "route".into(),
                points: vec![a, a, b, b, a, c, c],
                label: None,
                reversed_for_layout,
            };
            assert_eq!(route.into_geometry().unwrap().points, expected);
        }

        let empty = RoutedEdge {
            id: "empty after projection".into(),
            points: vec![a, a],
            label: None,
            reversed_for_layout: false,
        };
        assert_eq!(empty.into_geometry().unwrap_err(), Error::InvalidGraph);
    }
}
