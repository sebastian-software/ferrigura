//! The label anchoring of `postProcessSwimlaneLayout`: `direction/labelAnchoring.ts` of Mermaid 12.2.0.
//!
//! [`anchor_labels_to_polyline`] (`labelAnchoring.ts:83-459`) moves the label node of each labelled edge onto a
//! segment of the edge's polyline, away from the marker at each end, from the foreign nodes and edges, and from
//! the labels already placed, and it moves the label into the lane that contains it. It writes `x`, `y` and
//! `parentId` of the label node.
//!
//! The helpers are `markerClearanceRectFor` (`:18-63`), `normalizeRect` (`:65-72`) and
//! `labelOverlapsOwnMarker` (`:74-81`).
//!
//! The TypeScript reads the lanes and the foreign nodes once, before the loop, and it changes only the label
//! nodes in the loop. The port does the same: the obstacles are read into [`Obstacles`] first, and the label
//! nodes are written after each edge.

use indexmap::IndexMap;

use crate::direction::geometry::{
    EPS, dedupe_consecutive_points, inflate_rect, js_sign_value, js_sort_by, rect_contains_rect,
    rect_from_center_size, rect_of_node_bounds, rects_overlap, segment_bounds_overlap_rect,
    truthy_text,
};
use crate::math::{js_max, js_min};
use crate::model::{LayoutData, Node, Point, Rect};

/// `MARKER_CLEARANCE_LENGTH` (`labelAnchoring.ts:15`): the length of the marker's clearance along the segment.
const MARKER_CLEARANCE_LENGTH: f64 = 10.0;
/// `MARKER_CLEARANCE_HALF_WIDTH` (`labelAnchoring.ts:16`): the half width of the marker's clearance.
const MARKER_CLEARANCE_HALF_WIDTH: f64 = 7.0;
/// `LABEL_PLACEMENT_BUFFER` (`labelAnchoring.ts:148`).
const LABEL_PLACEMENT_BUFFER: f64 = 3.0;
/// `LABEL_LANE_MARGIN` (`labelAnchoring.ts:149`).
const LABEL_LANE_MARGIN: f64 = 1.0;
/// `LABEL_ENDPOINT_CLEARANCE` (`labelAnchoring.ts:152`).
const LABEL_ENDPOINT_CLEARANCE: f64 = 12.0;
/// `ALONG_SEGMENT_TS` (`labelAnchoring.ts:304`): the positions along a segment that are tried, in order.
const ALONG_SEGMENT_TS: [f64; 9] = [0.5, 0.25, 0.75, 0.05, 0.95, 0.15, 0.85, 0.1, 0.9];

/// A point of an anchor: the middle of a label on a segment.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Anchor {
    mid_x: f64,
    mid_y: f64,
}

/// A segment of a polyline that a label may sit on (`SegmentCandidate`, `labelAnchoring.ts:199-205`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct SegmentCandidate {
    /// The index of the first point of the segment in the polyline.
    idx: usize,
    length: f64,
    horizontal: bool,
    mid_x: f64,
    mid_y: f64,
}

/// A segment of another edge, which a label must not overlap (`SegmentLite`, `labelAnchoring.ts:88-92`).
#[derive(Debug, Clone, PartialEq)]
struct EdgeSegment {
    edge_id: String,
    p1: Point,
    p2: Point,
}

/// A lane: a top-level group, with its rectangle.
#[derive(Debug, Clone, PartialEq)]
struct LaneGroup {
    id: String,
    rect: Rect,
}

/// The obstacles of the label placement, read once from the layout before the loop of labels, and the labels
/// placed so far.
struct Obstacles {
    lane_groups: Vec<LaneGroup>,
    /// The rectangles of the nodes that are not groups and not edge labels, with their ids.
    foreign_nodes: Vec<(String, Rect)>,
    edge_segments: Vec<EdgeSegment>,
    /// The rectangle of each label placed so far, by label id.
    placed_labels: Vec<(String, Rect)>,
}

/// The label of one edge, and the polyline it sits on.
struct LabelFrame<'a> {
    pts: &'a [Point],
    label_id: &'a str,
    edge_id: &'a str,
    lw: f64,
    lh: f64,
    long_axis_horizontal: bool,
    first_idx: usize,
    last_idx: usize,
}

/// A place for the label: the lane that holds it, the anchor, and the rectangle of the label.
struct Placement {
    lane_id: String,
    anchor: Anchor,
    rect: Rect,
}

/// The point at `index` of a list, or `None` outside it. A negative index is outside, as `pts[-1]` is in
/// JavaScript.
fn point_at(points: &[Point], index: i64) -> Option<Point> {
    usize::try_from(index)
        .ok()
        .and_then(|index| points.get(index))
        .copied()
}

/// `markerClearanceRectFor(pts, atStart)` (`labelAnchoring.ts:18-63`): the rectangle that the marker at one end
/// of the polyline takes, along its first segment. `None` when the end has no segment of length at least `EPS`.
fn marker_clearance_rect_for(pts: &[Point], at_start: bool) -> Option<Rect> {
    let terminal_index: i64 = if at_start { 0 } else { pts.len() as i64 - 1 };
    let step: i64 = if at_start { 1 } else { -1 };
    let tip = point_at(pts, terminal_index)?;
    let inner = point_at(pts, terminal_index + step)?;

    let dx = inner.x - tip.x;
    let dy = inner.y - tip.y;
    let length = dx.abs() + dy.abs();
    if length < EPS {
        return None;
    }

    if dy.abs() <= EPS {
        let x2 = tip.x + js_sign_value(dx) * MARKER_CLEARANCE_LENGTH;
        return Some(Rect {
            left: js_min(tip.x, x2),
            right: js_max(tip.x, x2),
            top: tip.y - MARKER_CLEARANCE_HALF_WIDTH,
            bottom: tip.y + MARKER_CLEARANCE_HALF_WIDTH,
        });
    }

    if dx.abs() <= EPS {
        let y2 = tip.y + js_sign_value(dy) * MARKER_CLEARANCE_LENGTH;
        return Some(Rect {
            left: tip.x - MARKER_CLEARANCE_HALF_WIDTH,
            right: tip.x + MARKER_CLEARANCE_HALF_WIDTH,
            top: js_min(tip.y, y2),
            bottom: js_max(tip.y, y2),
        });
    }

    Some(Rect {
        left: js_min(tip.x, inner.x),
        right: js_max(tip.x, inner.x),
        top: js_min(tip.y, inner.y),
        bottom: js_max(tip.y, inner.y),
    })
}

/// `normalizeRect(rect)` (`labelAnchoring.ts:65-72`): the rectangle with its sides in order.
fn normalize_rect(rect: Rect) -> Rect {
    Rect {
        left: js_min(rect.left, rect.right),
        right: js_max(rect.left, rect.right),
        top: js_min(rect.top, rect.bottom),
        bottom: js_max(rect.top, rect.bottom),
    }
}

/// `labelOverlapsOwnMarker(rect, pts)` (`labelAnchoring.ts:74-81`): the label meets the marker at either end of
/// its own edge.
fn label_overlaps_own_marker(rect: Rect, pts: &[Point]) -> bool {
    let visible_pts = dedupe_consecutive_points(pts, EPS);
    [
        marker_clearance_rect_for(&visible_pts, true),
        marker_clearance_rect_for(&visible_pts, false),
    ]
    .into_iter()
    .flatten()
    .any(|marker| rects_overlap(rect, normalize_rect(marker)))
}

/// The point `t` of the way along a segment (`anchorAtT`, `labelAnchoring.ts:305-312`).
fn anchor_at_t(pts: &[Point], seg: &SegmentCandidate, t: f64) -> Anchor {
    let a = pts[seg.idx];
    let b = pts[seg.idx + 1];
    Anchor {
        mid_x: a.x + (b.x - a.x) * t,
        mid_y: a.y + (b.y - a.y) * t,
    }
}

/// `clamp(value, min, max)` (`labelAnchoring.ts:313-314`): `Math.min(max, Math.max(min, value))`.
fn clamp(value: f64, min: f64, max: f64) -> f64 {
    js_min(max, js_max(min, value))
}

/// `pointInsideRectInclusive(point, rect)` (`labelAnchoring.ts:315-322`): the anchor is inside the rectangle,
/// borders included, with the `EPS` tolerance.
fn point_inside_rect_inclusive(anchor: Anchor, rect: Rect) -> bool {
    (rect.left - EPS..=rect.right + EPS).contains(&anchor.mid_x)
        && (rect.top - EPS..=rect.bottom + EPS).contains(&anchor.mid_y)
}

/// `distanceAlongSegment(seg, anchor, endpoint)` (`labelAnchoring.ts:358-365`).
fn distance_along_segment(seg: &SegmentCandidate, anchor: Anchor, endpoint: Point) -> f64 {
    if seg.horizontal {
        (anchor.mid_x - endpoint.x).abs()
    } else {
        (anchor.mid_y - endpoint.y).abs()
    }
}

/// `labelClearsTerminalEndpoints(seg, anchor)` (`labelAnchoring.ts:366-385`): on the first and the last segment,
/// the label keeps its distance from the endpoint of the edge.
fn label_clears_terminal_endpoints(
    frame: &LabelFrame<'_>,
    seg: &SegmentCandidate,
    anchor: Anchor,
) -> bool {
    let label_half_extent = if seg.horizontal {
        frame.lw / 2.0
    } else {
        frame.lh / 2.0
    };
    let required_distance = label_half_extent + LABEL_ENDPOINT_CLEARANCE;
    if seg.idx == frame.first_idx {
        let start = frame.pts[seg.idx];
        if distance_along_segment(seg, anchor, start) + EPS < required_distance {
            return false;
        }
    }
    if seg.idx == frame.last_idx {
        let end = frame.pts[seg.idx + 1];
        if distance_along_segment(seg, anchor, end) + EPS < required_distance {
            return false;
        }
    }
    true
}

/// `rankSegments(pool)` (`labelAnchoring.ts:271-285`): the segments on the label's long axis first, then those
/// long enough for the label, then the longest. The sort is stable.
fn rank_segments(pool: &[SegmentCandidate], frame: &LabelFrame<'_>) -> Vec<SegmentCandidate> {
    let mut ranked = pool.to_vec();
    js_sort_by(&mut ranked, |a, b| {
        let a_long_axis = a.horizontal == frame.long_axis_horizontal;
        let b_long_axis = b.horizontal == frame.long_axis_horizontal;
        if a_long_axis != b_long_axis {
            return if a_long_axis { -1.0 } else { 1.0 };
        }
        let a_extent = if a.horizontal { frame.lw } else { frame.lh };
        let b_extent = if b.horizontal { frame.lw } else { frame.lh };
        let a_fits = a.length >= a_extent + 2.0;
        let b_fits = b.length >= b_extent + 2.0;
        if a_fits != b_fits {
            return if a_fits { -1.0 } else { 1.0 };
        }
        b.length - a.length
    });
    ranked
}

impl Obstacles {
    /// `findContainingLane(rect)` (`labelAnchoring.ts:185-192`): the first lane that holds the rectangle.
    fn find_containing_lane(&self, rect: Rect) -> Option<&str> {
        self.lane_groups
            .iter()
            .find(|lane| rect_contains_rect(lane.rect, rect))
            .map(|lane| lane.id.as_str())
    }

    /// `overlapsPlacedLabel(labelId, rect)` (`labelAnchoring.ts:194-197`).
    fn overlaps_placed_label(&self, label_id: &str, rect: Rect) -> bool {
        self.placed_labels
            .iter()
            .any(|(placed_id, placed)| placed_id != label_id && rects_overlap(rect, *placed))
    }

    /// `labelOverlapsForeignNode(labelId, rect)` (`labelAnchoring.ts:154-165`), with the buffer.
    fn overlaps_foreign_node(&self, label_id: &str, rect: Rect) -> bool {
        let buffered = inflate_rect(rect, LABEL_PLACEMENT_BUFFER);
        self.foreign_nodes
            .iter()
            .any(|(node_id, node_rect)| node_id != label_id && rects_overlap(buffered, *node_rect))
    }

    /// `labelOverlapsForeignEdge(edgeId, rect)` (`labelAnchoring.ts:167-178`): a segment of another edge meets
    /// the buffered rectangle.
    fn overlaps_foreign_edge(&self, edge_id: &str, rect: Rect) -> bool {
        let buffered = inflate_rect(rect, LABEL_PLACEMENT_BUFFER);
        self.edge_segments.iter().any(|segment| {
            segment.edge_id != edge_id
                && segment_bounds_overlap_rect(segment.p1, segment.p2, buffered, 0.0)
        })
    }

    /// `labelOverlapsAnything(labelId, edgeId, rect)` (`labelAnchoring.ts:180-181`).
    fn overlaps_anything(&self, label_id: &str, edge_id: &str, rect: Rect) -> bool {
        self.overlaps_foreign_node(label_id, rect) || self.overlaps_foreign_edge(edge_id, rect)
    }

    /// `placementForAnchor(anchor)` (`labelAnchoring.ts:323-357`): the label centred on the anchor, when a lane
    /// holds it; otherwise the label moved inside the lane that holds the anchor, with the anchor still inside
    /// the label.
    fn placement_for_anchor(&self, frame: &LabelFrame<'_>, anchor: Anchor) -> Option<Placement> {
        let centered_rect = rect_from_center_size(anchor.mid_x, anchor.mid_y, frame.lw, frame.lh);
        if let Some(lane_id) = self.find_containing_lane(centered_rect) {
            return Some(Placement {
                lane_id: lane_id.to_owned(),
                anchor,
                rect: centered_rect,
            });
        }

        // The anchor is near a lane border: the label box moves inside the lane, and the anchor must stay
        // inside that box.
        let containing = self
            .lane_groups
            .iter()
            .find(|lane| point_inside_rect_inclusive(anchor, lane.rect))?;
        let min_x = containing.rect.left + frame.lw / 2.0 + LABEL_LANE_MARGIN;
        let max_x = containing.rect.right - frame.lw / 2.0 - LABEL_LANE_MARGIN;
        let min_y = containing.rect.top + frame.lh / 2.0 + LABEL_LANE_MARGIN;
        let max_y = containing.rect.bottom - frame.lh / 2.0 - LABEL_LANE_MARGIN;
        if min_x > max_x || min_y > max_y {
            return None;
        }

        let clamped_anchor = Anchor {
            mid_x: clamp(anchor.mid_x, min_x, max_x),
            mid_y: clamp(anchor.mid_y, min_y, max_y),
        };
        let clamped_rect = rect_from_center_size(
            clamped_anchor.mid_x,
            clamped_anchor.mid_y,
            frame.lw,
            frame.lh,
        );
        if point_inside_rect_inclusive(anchor, clamped_rect) {
            Some(Placement {
                lane_id: containing.id.clone(),
                anchor: clamped_anchor,
                rect: clamped_rect,
            })
        } else {
            None
        }
    }

    /// `tryPool(pool)` (`labelAnchoring.ts:386-412`): the first position, along the ranked segments, that is
    /// free of the own marker, the placed labels, and the foreign nodes and edges.
    fn try_pool(
        &self,
        frame: &LabelFrame<'_>,
        pool: &[SegmentCandidate],
    ) -> Option<(String, Anchor)> {
        for seg in rank_segments(pool, frame) {
            for t in ALONG_SEGMENT_TS {
                let anchor = anchor_at_t(frame.pts, &seg, t);
                if !label_clears_terminal_endpoints(frame, &seg, anchor) {
                    continue;
                }
                let Some(placement) = self.placement_for_anchor(frame, anchor) else {
                    continue;
                };
                if label_overlaps_own_marker(placement.rect, frame.pts) {
                    continue;
                }
                if self.overlaps_placed_label(frame.label_id, placement.rect) {
                    continue;
                }
                if !self.overlaps_anything(frame.label_id, frame.edge_id, placement.rect) {
                    return Some((placement.lane_id, placement.anchor));
                }
            }
        }
        None
    }

    /// `findLaneContainingFallback(pool, requireEndpointClearance, allowForeignEdgeOverlap)`
    /// (`labelAnchoring.ts:414-437`): the midpoint of each ranked segment, with fewer tests than [`Self::try_pool`].
    fn find_lane_containing_fallback(
        &self,
        frame: &LabelFrame<'_>,
        pool: &[SegmentCandidate],
        require_endpoint_clearance: bool,
        allow_foreign_edge_overlap: bool,
    ) -> Option<(String, Anchor)> {
        for seg in rank_segments(pool, frame) {
            let anchor = Anchor {
                mid_x: seg.mid_x,
                mid_y: seg.mid_y,
            };
            if require_endpoint_clearance && !label_clears_terminal_endpoints(frame, &seg, anchor) {
                continue;
            }
            let Some(placement) = self.placement_for_anchor(frame, anchor) else {
                continue;
            };
            if label_overlaps_own_marker(placement.rect, frame.pts)
                || self.overlaps_placed_label(frame.label_id, placement.rect)
                || self.overlaps_foreign_node(frame.label_id, placement.rect)
                || (!allow_foreign_edge_overlap
                    && self.overlaps_foreign_edge(frame.edge_id, placement.rect))
            {
                continue;
            }
            return Some((placement.lane_id, placement.anchor));
        }
        None
    }
}

/// `anchorLabelsToPolyline(edges, nodeByIdMap)` (`labelAnchoring.ts:83-459`): for each labelled edge that is not
/// layout-only and has a polyline, the label node takes a place on the middle segments of the polyline, or the
/// midpoint of a segment, that clears the markers, the placed labels and the foreign nodes and edges, and the
/// label joins the lane that holds it. An edge with no place keeps its label node as it was.
pub fn anchor_labels_to_polyline(layout: &mut LayoutData) {
    let LayoutData { nodes, edges, .. } = layout;

    // `nodeByIdMap`: the node of each id, at the position of its first node. A later node with the same id
    // takes the value.
    let mut node_by_id: IndexMap<String, usize> = IndexMap::new();
    for (index, node) in nodes.iter().enumerate() {
        node_by_id.insert(node.id.clone(), index);
    }

    let mut edge_segments = Vec::new();
    for other in edges.iter() {
        if other.is_layout_only == Some(true) {
            continue;
        }
        let Some(pts) = other.points.as_deref() else {
            continue;
        };
        if pts.len() < 2 {
            continue;
        }
        for pair in pts.windows(2) {
            edge_segments.push(EdgeSegment {
                edge_id: other.id.clone(),
                p1: pair[0],
                p2: pair[1],
            });
        }
    }

    let mut lane_groups = Vec::new();
    let mut foreign_nodes = Vec::new();
    for &index in node_by_id.values() {
        let node: &Node = &nodes[index];
        let is_group = node.is_group == Some(true);
        let has_parent = truthy_text(node.parent_id.as_deref()).is_some();
        if is_group && !has_parent {
            if let Some(rect) = rect_of_node_bounds(node) {
                lane_groups.push(LaneGroup {
                    id: node.id.clone(),
                    rect,
                });
            }
            continue;
        }
        if is_group || node.is_edge_label == Some(true) {
            continue;
        }
        if let Some(rect) = rect_of_node_bounds(node) {
            foreign_nodes.push((node.id.clone(), rect));
        }
    }

    let mut obstacles = Obstacles {
        lane_groups,
        foreign_nodes,
        edge_segments,
        placed_labels: Vec::new(),
    };

    for edge in edges.iter() {
        if edge.is_layout_only == Some(true) {
            continue;
        }
        let Some(label_id) = truthy_text(edge.label_node_id.as_deref()) else {
            continue;
        };
        let Some(&label_index) = node_by_id.get(label_id) else {
            continue;
        };
        let Some(pts) = edge.points.as_deref() else {
            continue;
        };
        if pts.len() < 2 {
            continue;
        }
        let label_node = &nodes[label_index];
        let lw = label_node.width.unwrap_or(0.0);
        let lh = label_node.height.unwrap_or(0.0);
        if lw <= 0.0 || lh <= 0.0 {
            continue;
        }

        // Every segment that is not a zero length and not diagonal, with its orientation.
        let mut segments: Vec<SegmentCandidate> = Vec::new();
        for (idx, pair) in pts.windows(2).enumerate() {
            let (a, b) = (pair[0], pair[1]);
            let dx = (a.x - b.x).abs();
            let dy = (a.y - b.y).abs();
            if dx < EPS && dy < EPS {
                continue;
            }
            if dx >= EPS && dy >= EPS {
                continue;
            }
            segments.push(SegmentCandidate {
                idx,
                length: dx + dy,
                horizontal: dx >= EPS,
                mid_x: (a.x + b.x) / 2.0,
                mid_y: (a.y + b.y) / 2.0,
            });
        }
        if segments.is_empty() {
            continue;
        }

        // The middle segments, first; any segment when there are fewer than three.
        let segment_count = segments.len();
        let middle_segments: Vec<SegmentCandidate> = if segment_count >= 3 {
            segments
                .iter()
                .filter(|seg| seg.idx > 0 && seg.idx < segment_count - 1)
                .copied()
                .collect()
        } else {
            segments.clone()
        };
        let pool_base: Vec<SegmentCandidate> = if middle_segments.is_empty() {
            segments.clone()
        } else {
            middle_segments
        };

        let frame = LabelFrame {
            pts,
            label_id,
            edge_id: edge.id.as_str(),
            lw,
            lh,
            long_axis_horizontal: lw >= lh,
            first_idx: segments[0].idx,
            last_idx: segments[segment_count - 1].idx,
        };

        let chosen = obstacles
            .try_pool(&frame, &pool_base)
            .or_else(|| {
                if pool_base.len() < segment_count {
                    obstacles.try_pool(&frame, &segments)
                } else {
                    None
                }
            })
            .or_else(|| obstacles.find_lane_containing_fallback(&frame, &segments, true, false))
            .or_else(|| obstacles.find_lane_containing_fallback(&frame, &segments, false, false))
            .or_else(|| obstacles.find_lane_containing_fallback(&frame, &segments, false, true));

        if let Some((lane_id, anchor)) = chosen {
            let label = &mut nodes[label_index];
            label.x = Some(anchor.mid_x);
            label.y = Some(anchor.mid_y);
            label.parent_id = Some(lane_id);
            let chosen_rect = rect_from_center_size(anchor.mid_x, anchor.mid_y, lw, lh);
            match obstacles
                .placed_labels
                .iter()
                .position(|(placed_id, _)| placed_id == label_id)
            {
                Some(prior) => obstacles.placed_labels[prior] = (label_id.to_owned(), chosen_rect),
                None => obstacles
                    .placed_labels
                    .push((label_id.to_owned(), chosen_rect)),
            }
        }
    }
}
