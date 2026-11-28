// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, internal/quality/{crossings,labels}.go.

use std::collections::HashMap;

use crate::Point;

use super::{Box, EvaluationError, LABEL_PADDING, PositionedEdge, Work};

pub(super) fn count_crossings(
    edges: &[PositionedEdge],
    work: &mut Work<'_>,
) -> Result<i64, EvaluationError> {
    let mut count = 0;
    for (index, edge) in edges.iter().enumerate() {
        work.step()?;
        for other in &edges[index + 1..] {
            work.step()?;
            for pair in edge.points.windows(2) {
                work.step()?;
                for other_pair in other.points.windows(2) {
                    work.step()?;
                    count += i64::from(is_crossing(pair[0], pair[1], other_pair[0], other_pair[1]));
                }
            }
        }
    }
    Ok(count)
}

fn is_crossing(a: Point, b: Point, c: Point, d: Point) -> bool {
    let abc = orientation(a, b, c);
    let abd = orientation(a, b, d);
    if equal_signs(abc, abd) {
        return false;
    }
    let cda = orientation(c, d, a);
    let cdb = orientation(c, d, b);
    if equal_signs(cda, cdb) {
        return false;
    }
    if abc != 0.0 && abd != 0.0 && cda != 0.0 && cdb != 0.0 {
        return true;
    }
    !(a.x == b.x && (a.x == c.x || a.x == d.x) || a.y == b.y && (a.y == c.y || a.y == d.y))
}

fn orientation(p: Point, q: Point, r: Point) -> f64 {
    let pq_x = q.x - p.x;
    let pq_y = q.y - p.y;
    let pr_x = r.x - p.x;
    let pr_y = r.y - p.y;
    pq_y * pr_x - pq_x * pr_y
}

fn equal_signs(a: f64, b: f64) -> bool {
    a > 0.0 && b > 0.0 || a == 0.0 && b == 0.0 || a < 0.0 && b < 0.0
}

pub(crate) fn box_crosses_line(bounds: Box, first: Point, second: Point, padding: f64) -> bool {
    let left = bounds.x - padding;
    let right = bounds.right() + padding;
    let top = bounds.y - padding;
    let bottom = bounds.bottom() + padding;
    let contains =
        |point: Point| left <= point.x && point.x <= right && top <= point.y && point.y <= bottom;
    if contains(first) || contains(second) {
        return true;
    }
    let corners = [
        Point { x: left, y: top },
        Point { x: right, y: top },
        Point {
            x: right,
            y: bottom,
        },
        Point { x: left, y: bottom },
    ];
    corners.into_iter().enumerate().any(|(index, corner)| {
        segments_intersect(corner, corners[(index + 1) % corners.len()], first, second)
    })
}

fn segments_intersect(a: Point, b: Point, c: Point, d: Point) -> bool {
    let abc = orientation(a, b, c);
    let abd = orientation(a, b, d);
    let cda = orientation(c, d, a);
    let cdb = orientation(c, d, b);
    if abc == 0.0 && abd == 0.0 && cda == 0.0 && cdb == 0.0 {
        let overlap = |first: f64, second: f64, third: f64, fourth: f64| {
            first.min(second) <= third.max(fourth) && third.min(fourth) <= first.max(second)
        };
        return overlap(a.x, b.x, c.x, d.x) && overlap(a.y, b.y, c.y, d.y);
    }
    let straddle =
        |first: f64, second: f64| first == 0.0 || second == 0.0 || (first < 0.0) != (second < 0.0);
    straddle(abc, abd) && straddle(cda, cdb)
}

const SHARED_SEGMENT_CLEARANCE: f64 = 2.5 * LABEL_PADDING;

#[derive(Clone, Copy)]
struct Span {
    start: f64,
    end: f64,
}

type Groups = HashMap<u64, (f64, Vec<Span>)>;

pub(super) fn shared_segment_boxes(
    edges: &[PositionedEdge],
    work: &mut Work<'_>,
) -> Result<Vec<Box>, EvaluationError> {
    work.step()?;
    let mut vertical = Groups::new();
    let mut horizontal = Groups::new();
    for edge in edges {
        work.step()?;
        for pair in edge.points.windows(2) {
            work.step()?;
            if pair[0].x == pair[1].x {
                add_span(&mut vertical, pair[0].x, pair[0].y, pair[1].y);
            } else if pair[0].y == pair[1].y {
                add_span(&mut horizontal, pair[0].y, pair[0].x, pair[1].x);
            }
        }
    }

    let mut boxes = Vec::new();
    for (groups, horizontal) in [(vertical, false), (horizontal, true)] {
        for (axis, mut spans) in groups.into_values() {
            work.step()?;
            if spans.len() < 2 {
                continue;
            }
            sort_spans(&mut spans, work)?;
            let mut previous = spans[0];
            let mut shared: Option<Span> = None;
            let mut add = |span: Span| {
                if horizontal {
                    boxes.push(Box {
                        x: span.start,
                        y: axis - SHARED_SEGMENT_CLEARANCE,
                        width: span.end - span.start,
                        height: 2.0 * SHARED_SEGMENT_CLEARANCE,
                    });
                } else {
                    boxes.push(Box {
                        x: axis - SHARED_SEGMENT_CLEARANCE,
                        y: span.start,
                        width: 2.0 * SHARED_SEGMENT_CLEARANCE,
                        height: span.end - span.start,
                    });
                }
            };
            for &span in &spans[1..] {
                work.step()?;
                if previous.end > span.start {
                    let end = previous.end.min(span.end);
                    if let Some(overlap) = &mut shared {
                        overlap.end = overlap.end.max(end);
                    } else {
                        shared = Some(Span {
                            start: span.start,
                            end,
                        });
                    }
                } else if let Some(overlap) = shared.take() {
                    add(overlap);
                }
                if span.end > previous.end {
                    previous = span;
                }
            }
            if let Some(overlap) = shared {
                add(overlap);
            }
        }
    }

    // Go accounts for the expansion from segments into padded label boxes.
    for _ in &boxes {
        work.step()?;
    }
    Ok(boxes)
}

fn add_span(groups: &mut Groups, axis: f64, one: f64, two: f64) {
    let key = if axis == 0.0 { 0 } else { axis.to_bits() };
    groups
        .entry(key)
        .or_insert_with(|| (axis, Vec::new()))
        .1
        .push(Span {
            start: one.min(two),
            end: one.max(two),
        });
}

fn sort_spans(spans: &mut Vec<Span>, work: &mut Work<'_>) -> Result<(), EvaluationError> {
    let mut buffer = vec![spans[0]; spans.len()];
    let mut width = 1;
    while width < spans.len() {
        for start in (0..spans.len()).step_by(width * 2) {
            let middle = (start + width).min(spans.len());
            let end = (start + 2 * width).min(spans.len());
            let mut left = start;
            let mut right = middle;
            for slot in &mut buffer[start..end] {
                work.step()?;
                let take_left = right == end
                    || left < middle
                        && (spans[left].start < spans[right].start
                            || (spans[left].start == spans[right].start
                                && spans[left].end <= spans[right].end));
                if take_left {
                    *slot = spans[left];
                    left += 1;
                } else {
                    *slot = spans[right];
                    right += 1;
                }
            }
        }
        std::mem::swap(spans, &mut buffer);
        width *= 2;
    }
    Ok(())
}
