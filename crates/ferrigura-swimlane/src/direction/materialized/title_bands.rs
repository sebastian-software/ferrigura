//! `liftTopLaneTitleBandsAboveRails` (`materializedGeometry.ts:720-842`) and
//! `shiftLeftLaneTitleBandsLeftOfRails` (`materializedGeometry.ts:848-984`). The title bands of the top-level
//! lanes are visual headers, not routing obstacles: a band that a rail crosses moves away from the rail, and
//! the lane grows by the same amount. The passes read the edge points and change only the node boxes.

use super::common::*;
use crate::math::{js_max, js_min};
use crate::model::{Direction, LayoutData, Node, Rect};

const CLEARANCE: f64 = 4.0;

/// A top-level lane with a valid title band (`LaneTitle` in the reference).
#[derive(Clone, Copy)]
struct LaneTitle {
    /// The index of the lane node.
    node: usize,
    rect: Rect,
}

/// The lane of a title band, when it is a top-level group whose direction is `accepts` allows, with a band
/// and a box of a positive size.
fn title_lane(
    view: &View,
    index: usize,
    layout: Direction,
    accepts: impl Fn(&str) -> bool,
    box_ok: impl Fn(&Node) -> bool,
    band_ok: impl Fn(Rect) -> bool,
) -> Option<LaneTitle> {
    let node = &view.nodes[index];
    if !is_group(node) || has_parent(node) {
        return None;
    }
    let direction = lane_direction(node, layout);
    if !accepts(&direction) {
        return None;
    }
    let rect = valid_title_rect(node)?;
    if !box_ok(node) || !band_ok(rect) {
        return None;
    }
    Some(LaneTitle { node: index, rect })
}

/// The `y` and `height` of a top-level lane are finite numbers, and the height is positive.
fn top_box_ok(node: &Node) -> bool {
    matches!(node.y, Some(y) if y.is_finite())
        && matches!(node.height, Some(height) if height.is_finite() && height > 0.0)
}

/// The `x` and `width` of a left lane are finite numbers, and the width is positive.
fn left_box_ok(node: &Node) -> bool {
    matches!(node.x, Some(x) if x.is_finite())
        && matches!(node.width, Some(width) if width.is_finite() && width > 0.0)
}

/// `topLaneTitleFor`: a band that is at least as wide as it is tall.
fn top_lane_title(view: &View, index: usize, layout: Direction) -> Option<LaneTitle> {
    title_lane(
        view,
        index,
        layout,
        |direction| !matches!(direction, "LR" | "RL" | "BT"),
        top_box_ok,
        |rect| {
            let title_width = rect.right - rect.left;
            let title_height = rect.bottom - rect.top;
            title_height > 0.0 && title_width >= title_height
        },
    )
}

/// `leftLaneTitleFor`: a band that is at least as tall as it is wide, in an `LR` layout.
fn left_lane_title(view: &View, index: usize, layout: Direction) -> Option<LaneTitle> {
    title_lane(
        view,
        index,
        layout,
        |direction| direction == "LR",
        left_box_ok,
        |rect| {
            let title_width = rect.right - rect.left;
            let title_height = rect.bottom - rect.top;
            title_width > 0.0 && title_height >= title_width
        },
    )
}

/// `horizontalSegmentIntersectsTitle(segment, rect)`: a horizontal rail crosses the band (a rail along the
/// band's edge counts).
fn horizontal_intersects(segment: &OrthogonalSegment, rect: Rect) -> bool {
    if !segment.horizontal {
        return false;
    }
    let y = segment.a.y;
    if y < rect.top - EPS || y > rect.bottom + EPS {
        return false;
    }
    overlap_length(segment.a.x, segment.b.x, rect.left, rect.right) >= MIN_SHARED
}

/// `verticalSegmentIntersectsTitle(segment, rect)`: a vertical rail crosses the band.
fn vertical_intersects(segment: &OrthogonalSegment, rect: Rect) -> bool {
    if !segment.vertical {
        return false;
    }
    let x = segment.a.x;
    if x < rect.left - EPS || x > rect.right + EPS {
        return false;
    }
    overlap_length(segment.a.y, segment.b.y, rect.top, rect.bottom) >= MIN_SHARED
}

/// The visible edges' rails, as the segments of their deduplicated points.
fn visible_segments(view: &View) -> Vec<OrthogonalSegment> {
    let mut segments = Vec::new();
    for edge in view.visible() {
        let points = view.edges[edge].points.as_deref().unwrap_or(&[]);
        segments.extend(segments_for(&dedupe_consecutive_points(points, EPS)));
    }
    segments
}

/// `liftTopLaneTitleBandsAboveRails(edges, nodeByIdMap, layoutDirection)`.
pub fn lift_top_lane_title_bands_above_rails(data: &mut LayoutData, layout: Direction) {
    let Some((top_delta, lanes)) = top_band_lift(&View::new(&data.nodes, &data.edges), layout)
    else {
        return;
    };
    for lane in lanes {
        let node = &mut data.nodes[lane.node];
        let (Some(y), Some(height)) = (node.y, node.height) else {
            continue;
        };
        node.y = Some(y - top_delta / 2.0);
        node.height = Some(height + top_delta);
        node.group_title_rect = Some(Rect {
            top: lane.rect.top - top_delta,
            bottom: lane.rect.bottom - top_delta,
            ..lane.rect
        });
    }
}

/// The distance the title bands move up, and the lanes that move, or `None` when nothing moves.
fn top_band_lift(view: &View, layout: Direction) -> Option<(f64, Vec<LaneTitle>)> {
    let lanes: Vec<LaneTitle> = view
        .node_indices()
        .into_iter()
        .filter_map(|index| top_lane_title(view, index, layout))
        .collect();
    if lanes.is_empty() {
        return None;
    }

    let mut top_delta = 0.0;
    for segment in visible_segments(view) {
        for lane in &lanes {
            if !horizontal_intersects(&segment, lane.rect) {
                continue;
            }
            top_delta = js_max(top_delta, lane.rect.bottom - segment.a.y + CLEARANCE);
        }
    }

    if top_delta <= EPS {
        return None;
    }
    Some((top_delta, lanes))
}

/// `shiftLeftLaneTitleBandsLeftOfRails(edges, nodeByIdMap, layoutDirection)`.
pub fn shift_left_lane_title_bands_left_of_rails(data: &mut LayoutData, layout: Direction) {
    let Some((left_delta, lanes)) = left_band_shift(&View::new(&data.nodes, &data.edges), layout)
    else {
        return;
    };
    for lane in lanes {
        let node = &mut data.nodes[lane.node];
        let (Some(x), Some(width)) = (node.x, node.width) else {
            continue;
        };
        node.x = Some(x - left_delta / 2.0);
        node.width = Some(width + left_delta);
        node.group_title_rect = Some(Rect {
            left: lane.rect.left - left_delta,
            right: lane.rect.right - left_delta,
            ..lane.rect
        });
    }
}

/// The distance the title bands move left, and the lanes that move, or `None` when nothing moves.
fn left_band_shift(view: &View, layout: Direction) -> Option<(f64, Vec<LaneTitle>)> {
    let lanes: Vec<LaneTitle> = view
        .node_indices()
        .into_iter()
        .filter_map(|index| left_lane_title(view, index, layout))
        .collect();
    if lanes.is_empty() {
        return None;
    }

    let mut left_delta = 0.0;
    for segment in visible_segments(view) {
        for lane in &lanes {
            if vertical_intersects(&segment, lane.rect) {
                left_delta = js_max(left_delta, lane.rect.right - segment.a.x + CLEARANCE);
            } else if horizontal_intersects(&segment, lane.rect) {
                let segment_left = js_min(segment.a.x, segment.b.x);
                left_delta = js_max(left_delta, lane.rect.right - segment_left + CLEARANCE);
            }
        }
    }

    if left_delta <= EPS {
        return None;
    }
    Some((left_delta, lanes))
}
