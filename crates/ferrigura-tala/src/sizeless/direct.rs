// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from pinned D2 TALA: internal/placement/direct.go and
// internal/layoutgraph/node.go.

use std::cmp::Ordering;
use std::collections::HashMap;

use super::sized_cost::{Bounds, opposite};
use super::{optimize_sized_phase, preferred_direction};
use crate::{Error, Graph, MAX_EDGES, MAX_NODES, NodeGeometry, valid_dimension};

const RIGHT: i32 = 2;
const BOTTOM: i32 = 4;
const LEFT: i32 = -2;
const TOP: i32 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Vote {
    direction: i32,
    count: usize,
}

/// Places a single unfixed, prepared rectangular component and anchors the
/// body's bounding box at the packing origin.
///
/// # Errors
/// Returns an error for unsupported topology, invalid geometry or exhausted placement work.
pub fn place_flat_component(
    graph: &Graph,
    seed: i64,
    obstacle_order: &[usize],
) -> Result<Vec<NodeGeometry>, Error> {
    let mut bodies = optimize_sized_phase(graph, seed, obstacle_order)?;
    direct_flat_component(graph, &mut bodies)?;
    let min_x = bodies
        .iter()
        .map(|body| body.x)
        .fold(f64::INFINITY, f64::min)
        .round();
    let min_y = bodies
        .iter()
        .map(|body| body.y)
        .fold(f64::INFINITY, f64::min)
        .round();
    for body in &mut bodies {
        body.x -= min_x;
        body.y -= min_y;
    }
    Ok(bodies)
}

/// Mirrors a placed flat rectangular component toward its prevailing
/// directed-edge orientations. Both axes reflect about the origin.
///
/// # Errors
/// Returns an error for unsupported topology or invalid placement geometry.
pub fn direct_flat_component(graph: &Graph, bodies: &mut [NodeGeometry]) -> Result<(), Error> {
    if bodies.len() > MAX_NODES || graph.edges.len() > MAX_EDGES {
        return Err(Error::WorkLimit);
    }
    if graph.nodes.iter().any(|node| {
        node.parent_id.is_some()
            || node.is_group
            || !matches!(node.shape.as_str(), "rect" | "rectangle" | "squareRect")
    }) || graph.edges.iter().any(|edge| edge.source == edge.target)
    {
        return Err(Error::UnsupportedGraph);
    }
    if graph.nodes.len() != bodies.len()
        || bodies.iter().any(|body| {
            !body.x.is_finite()
                || !body.y.is_finite()
                || !valid_dimension(body.width)
                || !valid_dimension(body.height)
        })
    {
        return Err(Error::InvalidGraph);
    }
    let positions: HashMap<_, _> = bodies
        .iter()
        .map(|body| (body.id.as_str(), Bounds::from(body)))
        .collect();
    if positions.len() != bodies.len()
        || graph
            .nodes
            .iter()
            .any(|node| !positions.contains_key(node.id.as_str()))
    {
        return Err(Error::InvalidGraph);
    }
    let mut counts: [Vote; 4] = [RIGHT, BOTTOM, LEFT, TOP].map(|direction| Vote {
        direction,
        count: 0,
    });
    for edge in &graph.edges {
        if edge.source_arrow == edge.target_arrow {
            continue;
        }
        let from = positions
            .get(edge.source.as_str())
            .ok_or(Error::InvalidGraph)?;
        let to = positions
            .get(edge.target.as_str())
            .ok_or(Error::InvalidGraph)?;
        let relative = if edge.source_arrow {
            to.direction_relative_to(*from)
        } else {
            from.direction_relative_to(*to)
        };
        if let Some(direction) = relative.map(opposite) {
            record_direction(&mut counts, direction);
        }
    }
    let (mirror_x, mirror_y) = mirror_axes(counts, preferred_direction(graph)?);
    for body in bodies {
        if mirror_x {
            body.x = -body.x - body.width + 0.0;
        }
        if mirror_y {
            body.y = -body.y - body.height + 0.0;
        }
    }
    Ok(())
}

fn record_direction([right, bottom, left, top]: &mut [Vote; 4], direction: i32) {
    match direction {
        RIGHT => right.count += 1,
        BOTTOM => bottom.count += 1,
        LEFT => left.count += 1,
        TOP => top.count += 1,
        -1 => {
            top.count += 1;
            left.count += 1;
        }
        1 => {
            top.count += 1;
            right.count += 1;
        }
        3 => {
            bottom.count += 1;
            right.count += 1;
        }
        -3 => {
            bottom.count += 1;
            left.count += 1;
        }
        _ => unreachable!("only compass directions are recorded"),
    }
}

fn compare_votes(a: Vote, b: Vote, preferred: Option<i32>) -> Ordering {
    b.count
        .cmp(&a.count)
        .then_with(|| (Some(b.direction) == preferred).cmp(&(Some(a.direction) == preferred)))
}

fn mirror_axes(mut votes: [Vote; 4], preferred: Option<i32>) -> (bool, bool) {
    votes.sort_by(|a, b| compare_votes(*a, *b, preferred));
    let [primary, second, third, last] = votes;
    let secondary = if second.direction == opposite(primary.direction) {
        third
    } else {
        second
    };
    let horizontal = |angle: i32| angle.abs() == RIGHT;
    let preferred_x = preferred
        .filter(|&angle| horizontal(angle))
        .unwrap_or(RIGHT);
    let preferred_y = preferred
        .filter(|&angle| !horizontal(angle))
        .unwrap_or(BOTTOM);
    let (mut mirror_x, mut mirror_y) = (false, false);
    for vote in [
        Some(primary),
        (secondary.count > last.count).then_some(secondary),
    ]
    .into_iter()
    .flatten()
    {
        if horizontal(vote.direction) {
            mirror_x = vote.direction != preferred_x;
        } else {
            mirror_y = vote.direction != preferred_y;
        }
    }
    (mirror_x, mirror_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_count_comparison_is_reflexive_and_stable() {
        let mut votes = [LEFT, TOP, RIGHT, BOTTOM].map(|direction| Vote {
            direction,
            count: 2,
        });
        for vote in votes {
            assert_eq!(compare_votes(vote, vote, Some(RIGHT)), Ordering::Equal);
        }
        votes.sort_by(|a, b| compare_votes(*a, *b, Some(RIGHT)));
        assert_eq!(votes.map(|vote| vote.direction), [RIGHT, LEFT, TOP, BOTTOM]);
    }

    #[test]
    fn direction_transforms_follow_primary_and_secondary_votes() {
        for (preferred, counts, expected) in [
            (Some(RIGHT), [0, 0, 0, 0], (false, false)),
            (Some(RIGHT), [1, 0, 0, 0], (false, false)),
            (Some(LEFT), [1, 0, 0, 0], (true, false)),
            (Some(TOP), [0, 1, 0, 0], (false, true)),
            (Some(BOTTOM), [0, 0, 0, 1], (false, true)),
            (Some(RIGHT), [0, 0, 2, 1], (true, true)),
            (Some(RIGHT), [1, 0, 1, 1], (false, true)),
            (Some(BOTTOM), [0, 1, 1, 0], (true, false)),
            (None, [0, 1, 1, 0], (true, false)),
        ] {
            let votes = std::array::from_fn(|index| Vote {
                direction: [RIGHT, BOTTOM, LEFT, TOP][index],
                count: counts[index],
            });
            assert_eq!(mirror_axes(votes, preferred), expected);
        }
    }
}
