//! The candidate paths of an edge in the crossing resolution (`candidatePathsFor` and its helpers, lines
//! 1835-2814 of `materializedGeometry.ts`): the orthogonal routes between the sides of the two endpoints, the
//! routes on the outer tracks, and the routes that keep the first departure of the edge.

use super::common::*;
use super::crossings::{ANCHOR, Ctx};
use crate::math::js_max;
use crate::math::js_min;
use crate::model::Point;

/// The sides, in the order the reference visits them.
const SIDES: [RectSide; 4] = [
    RectSide::Top,
    RectSide::Bottom,
    RectSide::Left,
    RectSide::Right,
];

/// `candidatePathsFor(edge)`: the candidate paths of an edge, with no dedup and no node test.
pub(super) fn candidate_paths_for(ctx: &Ctx, edge: usize) -> Vec<Vec<Point>> {
    let edge_ref = &ctx.view.edges[edge];
    let (Some(src_node), Some(dst_node)) = (
        ctx.info(edge_ref.start.as_deref()),
        ctx.info(edge_ref.end.as_deref()),
    ) else {
        return Vec::new();
    };

    let mut candidates = Vec::new();
    for src_side in SIDES {
        let src_port = port_for_rect_side(src_node, src_side);
        for dst_side in SIDES {
            let dst_port = port_for_rect_side(dst_node, dst_side);
            candidates.extend(candidates_for_sides(
                ctx, src_port, src_side, dst_port, dst_side,
            ));
        }
    }
    candidates.extend(terminal_preserving_outer_track_candidates(ctx, edge));
    candidates
}

/// `pushOrthogonalCandidate(candidates, points)`: the simplified path, when it is orthogonal.
fn push_orthogonal(candidates: &mut Vec<Vec<Point>>, points: &[Point]) {
    let candidate = simplified(points);
    if is_orthogonal_path(&candidate) {
        candidates.push(candidate);
    }
}

fn is_horizontal_side(side: RectSide) -> bool {
    matches!(side, RectSide::Left | RectSide::Right)
}

/// `localTrackForSameSide(src, side, dst)`.
fn local_track_for_same_side(src: Point, side: RectSide, dst: Point) -> f64 {
    match side {
        RectSide::Left => js_min(src.x, dst.x) - ANCHOR,
        RectSide::Right => js_max(src.x, dst.x) + ANCHOR,
        RectSide::Top => js_min(src.y, dst.y) - ANCHOR,
        RectSide::Bottom => js_max(src.y, dst.y) + ANCHOR,
    }
}

/// `addSameSideCandidates`: the routes along the tracks outside both ports, on their shared side.
fn add_same_side_candidates(
    candidates: &mut Vec<Vec<Point>>,
    ctx: &Ctx,
    src: Point,
    src_side: RectSide,
    dst: Point,
) {
    let outward = if matches!(src_side, RectSide::Left | RectSide::Top) {
        -1.0
    } else {
        1.0
    };
    let track_seeds = [
        local_track_for_same_side(src, src_side, dst),
        ctx.outside.of(src_side),
    ];
    for seed in track_seeds {
        for channel in 0..=2 {
            let track = seed + outward * ANCHOR * channel as f64;
            push_orthogonal(
                candidates,
                &build_same_side_track_path(src, src_side, dst, track),
            );
        }
    }
}

/// `addHorizontalToVerticalCandidates`.
fn add_horizontal_to_vertical_candidates(
    candidates: &mut Vec<Vec<Point>>,
    ctx: &Ctx,
    src: Point,
    src_side: RectSide,
    dst: Point,
    dst_side: RectSide,
) {
    for x_track in ctx.outward_tracks(src_side) {
        for y_track in ctx.outward_tracks(dst_side) {
            push_orthogonal(
                candidates,
                &[
                    src,
                    Point {
                        x: x_track,
                        y: src.y,
                    },
                    Point {
                        x: x_track,
                        y: y_track,
                    },
                    Point {
                        x: dst.x,
                        y: y_track,
                    },
                    dst,
                ],
            );
        }
    }
}

/// `addVerticalToHorizontalCandidates`.
fn add_vertical_to_horizontal_candidates(
    candidates: &mut Vec<Vec<Point>>,
    ctx: &Ctx,
    src: Point,
    src_side: RectSide,
    dst: Point,
    dst_side: RectSide,
) {
    for y_track in ctx.outward_tracks(src_side) {
        for x_track in ctx.outward_tracks(dst_side) {
            push_orthogonal(
                candidates,
                &[
                    src,
                    Point {
                        x: src.x,
                        y: y_track,
                    },
                    Point {
                        x: x_track,
                        y: y_track,
                    },
                    Point {
                        x: x_track,
                        y: dst.y,
                    },
                    dst,
                ],
            );
        }
    }
}

/// `addHorizontalPairCandidates`: two horizontal ports, joined by a track above or below.
fn add_horizontal_pair_candidates(
    candidates: &mut Vec<Vec<Point>>,
    ctx: &Ctx,
    src: Point,
    src_side: RectSide,
    dst: Point,
    dst_side: RectSide,
) {
    let mut y_tracks = ctx.outward_tracks(RectSide::Top);
    y_tracks.extend(ctx.outward_tracks(RectSide::Bottom));
    for src_track in ctx.outward_tracks(src_side) {
        for dst_track in ctx.outward_tracks(dst_side) {
            for &y_track in &y_tracks {
                push_orthogonal(
                    candidates,
                    &[
                        src,
                        Point {
                            x: src_track,
                            y: src.y,
                        },
                        Point {
                            x: src_track,
                            y: y_track,
                        },
                        Point {
                            x: dst_track,
                            y: y_track,
                        },
                        Point {
                            x: dst_track,
                            y: dst.y,
                        },
                        dst,
                    ],
                );
            }
        }
    }
}

/// `addVerticalPairCandidates`: two vertical ports, joined by a track to the left or right.
fn add_vertical_pair_candidates(
    candidates: &mut Vec<Vec<Point>>,
    ctx: &Ctx,
    src: Point,
    src_side: RectSide,
    dst: Point,
    dst_side: RectSide,
) {
    let mut x_tracks = ctx.outward_tracks(RectSide::Left);
    x_tracks.extend(ctx.outward_tracks(RectSide::Right));
    for src_track in ctx.outward_tracks(src_side) {
        for dst_track in ctx.outward_tracks(dst_side) {
            for &x_track in &x_tracks {
                push_orthogonal(
                    candidates,
                    &[
                        src,
                        Point {
                            x: src.x,
                            y: src_track,
                        },
                        Point {
                            x: x_track,
                            y: src_track,
                        },
                        Point {
                            x: x_track,
                            y: dst_track,
                        },
                        Point {
                            x: dst.x,
                            y: dst_track,
                        },
                        dst,
                    ],
                );
            }
        }
    }
}

/// `dedupeCandidatePaths(candidates)`: the first of each path, by its key, that has two points or more.
fn dedupe_candidate_paths(candidates: Vec<Vec<Point>>) -> Vec<Vec<Point>> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for candidate in candidates {
        let candidate = dedupe_consecutive_points(&candidate, EPS);
        let key = path_key(&candidate);
        if seen.contains(&key) || candidate.len() < 2 {
            continue;
        }
        seen.insert(key);
        result.push(candidate);
    }
    result
}

/// `buildCandidatesForSides(src, srcSide, dst, dstSide)`.
fn candidates_for_sides(
    ctx: &Ctx,
    src: Point,
    src_side: RectSide,
    dst: Point,
    dst_side: RectSide,
) -> Vec<Vec<Point>> {
    let mut candidates = Vec::new();
    if let Some(base) = build_orthogonal_port_path(src, src_side, dst, dst_side, ANCHOR, EPS) {
        push_orthogonal(&mut candidates, &base);
    }
    if src_side == dst_side {
        add_same_side_candidates(&mut candidates, ctx, src, src_side, dst);
    }

    let src_horizontal = is_horizontal_side(src_side);
    let dst_horizontal = is_horizontal_side(dst_side);
    if src_horizontal && !dst_horizontal {
        add_horizontal_to_vertical_candidates(&mut candidates, ctx, src, src_side, dst, dst_side);
    } else if !src_horizontal && dst_horizontal {
        add_vertical_to_horizontal_candidates(&mut candidates, ctx, src, src_side, dst, dst_side);
    } else if src_horizontal {
        add_horizontal_pair_candidates(&mut candidates, ctx, src, src_side, dst, dst_side);
    } else {
        add_vertical_pair_candidates(&mut candidates, ctx, src, src_side, dst, dst_side);
    }

    dedupe_candidate_paths(candidates)
}

/// `addVerticalDepartureOuterTrackCandidates(candidates, first, departure, dstNode)`.
fn add_vertical_departure_outer_track_candidates(
    candidates: &mut Vec<Vec<Point>>,
    ctx: &Ctx,
    first: Point,
    departure: Point,
    dst_node: &NodeBoundsInfo,
) {
    let mut external_x = ctx.outward_tracks(RectSide::Left);
    external_x.extend(ctx.outward_tracks(RectSide::Right));
    let mut external_y = ctx.outward_tracks(RectSide::Top);
    external_y.extend(ctx.outward_tracks(RectSide::Bottom));
    for side in SIDES {
        let dst = port_for_rect_side(dst_node, side);
        let target_y_tracks = if matches!(side, RectSide::Top | RectSide::Bottom) {
            ctx.outward_tracks(side)
        } else {
            external_y.clone()
        };
        for &track in &external_x {
            push_orthogonal(
                candidates,
                &[
                    first,
                    departure,
                    Point {
                        x: track,
                        y: departure.y,
                    },
                    Point { x: track, y: dst.y },
                    dst,
                ],
            );
            for &target_track in &target_y_tracks {
                push_orthogonal(
                    candidates,
                    &[
                        first,
                        departure,
                        Point {
                            x: track,
                            y: departure.y,
                        },
                        Point {
                            x: track,
                            y: target_track,
                        },
                        Point {
                            x: dst.x,
                            y: target_track,
                        },
                        dst,
                    ],
                );
            }
        }
    }
}

/// `addHorizontalDepartureOuterTrackCandidates(candidates, first, departure, dstNode)`.
fn add_horizontal_departure_outer_track_candidates(
    candidates: &mut Vec<Vec<Point>>,
    ctx: &Ctx,
    first: Point,
    departure: Point,
    dst_node: &NodeBoundsInfo,
) {
    let mut external_x = ctx.outward_tracks(RectSide::Left);
    external_x.extend(ctx.outward_tracks(RectSide::Right));
    let mut external_y = ctx.outward_tracks(RectSide::Top);
    external_y.extend(ctx.outward_tracks(RectSide::Bottom));
    for side in SIDES {
        let dst = port_for_rect_side(dst_node, side);
        let target_x_tracks = if matches!(side, RectSide::Left | RectSide::Right) {
            ctx.outward_tracks(side)
        } else {
            external_x.clone()
        };
        for &track in &external_y {
            push_orthogonal(
                candidates,
                &[
                    first,
                    departure,
                    Point {
                        x: departure.x,
                        y: track,
                    },
                    Point { x: dst.x, y: track },
                    dst,
                ],
            );
            for &target_track in &target_x_tracks {
                push_orthogonal(
                    candidates,
                    &[
                        first,
                        departure,
                        Point {
                            x: departure.x,
                            y: track,
                        },
                        Point {
                            x: target_track,
                            y: track,
                        },
                        Point {
                            x: target_track,
                            y: dst.y,
                        },
                        dst,
                    ],
                );
            }
        }
    }
}

/// `terminalPreservingOuterTrackCandidates(edge)`: the routes that keep the first departure of the edge and
/// leave through an outer track.
fn terminal_preserving_outer_track_candidates(ctx: &Ctx, edge: usize) -> Vec<Vec<Point>> {
    let edge_ref = &ctx.view.edges[edge];
    let Some(dst_node) = ctx.info(edge_ref.end.as_deref()) else {
        return Vec::new();
    };
    if truthy_text(edge_ref.start.as_deref()).is_none() {
        return Vec::new();
    }

    // The current points of the edge: an earlier iteration of the pass may have replaced them.
    let points = ctx.pts(&Replacements::new(), edge);
    if points.len() < 4 {
        return Vec::new();
    }

    let first = points[0];
    let departure = points[1];
    let mut candidates = Vec::new();
    if is_vertical_segment(first, departure, EPS) {
        add_vertical_departure_outer_track_candidates(
            &mut candidates,
            ctx,
            first,
            departure,
            dst_node,
        );
    } else if is_horizontal_segment(first, departure, EPS) {
        add_horizontal_departure_outer_track_candidates(
            &mut candidates,
            ctx,
            first,
            departure,
            dst_node,
        );
    }
    candidates
}
