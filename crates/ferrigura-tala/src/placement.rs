// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2's TALA. See NOTICE.md for the original paths and revision.

use crate::{Graph, Node, NodeGeometry, Point};

const COMPONENT_GAP: f64 = 20.0;
const PLACEMENT_PADDING: f64 = 1000.0;
const TEMPORARY_OFFSET: f64 = 100_000.0;

// The reference's area sort preserves equal-area source order in the
// small-slice insertion path. This also bounds the greedy packing work.
const MAX_COMPONENTS: usize = 12;
const MAX_TILE_SIDE: f64 = 2000.0;

const ORIGIN: Point = Point { x: 0.0, y: 0.0 };

#[derive(Clone, Copy)]
struct Tile {
    top_left: Point,
    width: f64,
    height: f64,
}

impl Tile {
    fn with_position(self, top_left: Point) -> Self {
        Self { top_left, ..self }
    }

    fn rounded_corner(self) -> Point {
        Point {
            x: (self.top_left.x + self.width).round(),
            y: (self.top_left.y + self.height).round(),
        }
    }

    fn overlaps(self, other: Self, padding: f64) -> bool {
        let a = self.top_left;
        let b = other.top_left;
        a.x < b.x + other.width + padding
            && b.x < a.x + self.width + padding
            && a.y < b.y + other.height + padding
            && b.y < a.y + self.height + padding
    }
}

/// Places independent rectangular components in source order at the output boundary.
pub(crate) fn place(graph: &Graph) -> Option<Vec<NodeGeometry>> {
    if !graph.edges.is_empty()
        || graph.nodes.len() > MAX_COMPONENTS
        || !graph.nodes.iter().all(is_plain_tile)
    {
        return None;
    }

    let mut tiles: Vec<_> = graph
        .nodes
        .iter()
        .map(|node| Tile {
            top_left: ORIGIN,
            width: node.width,
            height: node.height,
        })
        .collect();
    if !pack_tiles(&mut tiles) {
        return None;
    }

    Some(
        graph
            .nodes
            .iter()
            .zip(tiles)
            .map(|(node, tile)| NodeGeometry {
                id: node.id.clone(),
                x: tile.top_left.x,
                y: tile.top_left.y,
                width: tile.width,
                height: tile.height,
            })
            .collect(),
    )
}

/// Computes the origins of two fitted frames in source order.
pub(crate) fn pack_frame_pair(first: &NodeGeometry, second: &NodeGeometry) -> Option<[Point; 2]> {
    let mut tiles = [first, second].map(|node| Tile {
        top_left: ORIGIN,
        width: node.width,
        height: node.height,
    });
    pack_tiles(&mut tiles).then(|| tiles.map(|tile| tile.top_left))
}

fn pack_tiles(tiles: &mut [Tile]) -> bool {
    if tiles.len() > 1 {
        combine_components(tiles);
        repack(tiles);
        for tile in &mut *tiles {
            tile.top_left.x += PLACEMENT_PADDING;
            tile.top_left.y += PLACEMENT_PADDING;
        }
        repack(tiles);
        let (top_left, _) = bounds(tiles);
        for tile in &mut *tiles {
            tile.top_left.x -= top_left.x;
            tile.top_left.y -= top_left.y;
        }
    }

    // Rounded component bounds can eliminate every initial candidate in the
    // upstream algorithm. Its final pass may then keep coincident bodies
    // because that result has the lowest footprint. Decline that geometry.
    !tiles.iter().enumerate().any(|(index, &tile)| {
        tiles[..index]
            .iter()
            .any(|&other| tile.overlaps(other, 0.0))
    })
}

fn is_plain_tile(node: &Node) -> bool {
    !node.is_group
        && node.parent_id.is_none()
        && matches!(node.shape.as_str(), "rect" | "rectangle" | "squareRect")
        && (1.0..=MAX_TILE_SIDE).contains(&node.width)
        && (1.0..=MAX_TILE_SIDE).contains(&node.height)
        && node.label_width.ceil() <= node.width
        && node.label_height.ceil() <= node.height
}

fn corners(top_left: Point, bottom_right: Point) -> [Point; 6] {
    let Point { x, y } = top_left;
    let Point {
        x: right,
        y: bottom,
    } = bottom_right;
    [
        Point { x: right, y },
        Point {
            x: right,
            y: bottom,
        },
        Point { x, y: bottom },
        Point {
            x: right + COMPONENT_GAP,
            y,
        },
        Point {
            x: right + COMPONENT_GAP,
            y: bottom,
        },
        Point {
            x,
            y: bottom + COMPONENT_GAP,
        },
    ]
}

fn footprint(width: f64, height: f64) -> f64 {
    let difference = width - height;
    width * height + (difference * difference) * 0.5
}

fn bounds(tiles: &[Tile]) -> (Point, Point) {
    let mut top_left = Point {
        x: f64::INFINITY,
        y: f64::INFINITY,
    };
    let mut bottom_right = Point {
        x: f64::NEG_INFINITY,
        y: f64::NEG_INFINITY,
    };
    for tile in tiles {
        top_left.x = top_left.x.min(tile.top_left.x);
        top_left.y = top_left.y.min(tile.top_left.y);
        let corner = tile.rounded_corner();
        bottom_right.x = bottom_right.x.max(corner.x);
        bottom_right.y = bottom_right.y.max(corner.y);
    }
    (top_left, bottom_right)
}

// CombineSubgraphs first places components by descending unrounded area. Its
// candidate corners come from the rounded extent evaluated before translation.
fn combine_components(tiles: &mut [Tile]) {
    let mut order: Vec<_> = (0..tiles.len()).collect();
    order.sort_by(|&a, &b| {
        (tiles[b].width * tiles[b].height).total_cmp(&(tiles[a].width * tiles[a].height))
    });

    let mut placed = Vec::with_capacity(tiles.len());
    let mut candidates = Vec::with_capacity(tiles.len() * 6);
    let mut combined = ORIGIN;
    for index in order {
        let tile = tiles[index];
        let extent = tile.rounded_corner();
        let mut best_score = f64::INFINITY;
        let mut best_index = None;
        for (index, &candidate) in candidates.iter().enumerate() {
            if placed
                .iter()
                .any(|&other| tile.with_position(candidate).overlaps(other, COMPONENT_GAP))
            {
                continue;
            }
            let score = footprint(
                combined.x.max(candidate.x + extent.x),
                combined.y.max(candidate.y + extent.y),
            );
            if score < best_score {
                best_score = score;
                best_index = Some(index);
            }
        }

        // The reference removes a used candidate by moving the last candidate
        // into its place. Later ties depend on this order.
        let position = best_index.map_or(ORIGIN, |index| candidates.swap_remove(index));
        let corner = Point {
            x: position.x + extent.x,
            y: position.y + extent.y,
        };
        combined.x = combined.x.max(corner.x);
        combined.y = combined.y.max(corner.y);
        candidates.extend(corners(position, corner));
        tiles[index].top_left = position;
        placed.push(tiles[index]);
    }
}

fn include_tile_candidates(candidates: &mut Vec<Point>, occupied: &[Point], tile: Tile) {
    let Point { x, y } = tile.top_left;
    let right = x + tile.width;
    let bottom = y + tile.height;
    let near = [
        Point {
            x: right + COMPONENT_GAP,
            y,
        },
        Point {
            x,
            y: bottom + COMPONENT_GAP,
        },
        Point {
            x: right + COMPONENT_GAP,
            y: bottom,
        },
    ];
    for point in near
        .into_iter()
        .chain(corners(tile.top_left, tile.rounded_corner()))
    {
        if !occupied.contains(&point) && !candidates.contains(&point) {
            candidates.push(point);
        }
    }
}

fn packing_score(tiles: &[Tile]) -> f64 {
    let (top_left, bottom_right) = bounds(tiles);
    footprint(bottom_right.x - top_left.x, bottom_right.y - top_left.y)
}

// BinPack starts at the first source component. It tentatively repacks in
// source order and keeps the original when the footprint gets worse.
fn repack(tiles: &mut [Tile]) {
    let root = tiles[0].top_left;
    let mut placed = Vec::with_capacity(tiles.len());
    let mut occupied = Vec::with_capacity(tiles.len() - 1);
    placed.push(tiles[0]);
    for &tile in &tiles[1..] {
        let mut candidates = Vec::with_capacity(placed.len() * 9 + 5);
        candidates.push(root);
        for &other in &placed {
            include_tile_candidates(&mut candidates, &occupied, other);
        }

        let pending = tile.with_position(Point {
            x: tile.top_left.x + TEMPORARY_OFFSET,
            y: tile.top_left.y + TEMPORARY_OFFSET,
        });
        let corner = pending.rounded_corner();
        let (minimum, _) = bounds(&placed);
        let left = minimum.x - (corner.x - pending.top_left.x) - COMPONENT_GAP;
        let top = minimum.y - (corner.y - pending.top_left.y) - COMPONENT_GAP;

        // The top and left candidates are appended independently of the set
        // for right and bottom corners. Preserve their order before original.
        let include_original = !candidates.contains(&tile.top_left);
        candidates.extend([
            Point {
                x: minimum.x,
                y: top,
            },
            Point {
                x: left,
                y: minimum.y,
            },
            Point { x: left, y: top },
        ]);
        if include_original {
            candidates.push(tile.top_left);
        }

        let mut best_score = f64::INFINITY;
        let mut best = None;
        for candidate in candidates {
            // Preserve the reference's sequence of floating-point operations
            // when translating the pending tile back into the packing frame.
            let translated = pending.with_position(Point {
                x: pending.top_left.x + (candidate.x - pending.top_left.x),
                y: pending.top_left.y + (candidate.y - pending.top_left.y),
            });
            if placed
                .iter()
                .any(|&other| translated.overlaps(other, COMPONENT_GAP))
            {
                continue;
            }
            placed.push(translated);
            let score = packing_score(&placed);
            placed.pop();
            if score < best_score {
                best_score = score;
                best = Some((candidate, translated));
            }
        }
        let Some((chosen, translated)) = best else {
            return;
        };
        occupied.push(chosen);
        placed.push(translated);
    }
    if packing_score(tiles) >= packing_score(&placed) {
        tiles.copy_from_slice(&placed);
    }
}

#[cfg(test)]
mod tests;
