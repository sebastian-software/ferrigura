//! Room beside a node in the design mode: the geometry that the steps share which give a node a
//! box larger than itself in the layout and set something into the room around it afterwards (a
//! state's note, a self-loop). The steps are ferrigura's own and run around ELK's layout.

use std::collections::HashMap;

use ferrigura_elk::json::{JsonNode, JsonPoint};

use crate::unified::elk_geometry::find_common_ancestor;
use crate::unified::geom::{BBox, Point};

/// How far a point may be off a side to count as on it.
const TOLERANCE: f64 = 0.5;

/// Moves the end of an edge from the side of a node's box in the layout onto the node's own
/// outline, along the edge's last part, which must leave the side straight. `None` where the end
/// is not on a side of the box, leaves it at a slant, or misses the node.
pub(crate) fn onto_outline(point: Point, next: Point, laid: BBox, real: BBox) -> Option<Point> {
    let near = |a: f64, b: f64| (a - b).abs() <= TOLERANCE;
    let within =
        |value: f64, from: f64, to: f64| value >= from - TOLERANCE && value <= to + TOLERANCE;
    if near(point.y, laid.y) || near(point.y, laid.bottom()) {
        if !near(point.x, next.x) || !within(point.x, real.x, real.right()) {
            return None;
        }
        let y = if near(point.y, laid.y) {
            real.y
        } else {
            real.bottom()
        };
        return Some(Point::new(point.x, y));
    }
    if near(point.x, laid.x) || near(point.x, laid.right()) {
        if !near(point.y, next.y) || !within(point.y, real.y, real.bottom()) {
            return None;
        }
        let x = if near(point.x, laid.x) {
            real.x
        } else {
            real.right()
        };
        return Some(Point::new(x, point.y));
    }
    None
}

/// The absolute box of every node of ELK's result, with the id of its parent.
pub(crate) fn absolute_boxes(
    node: &JsonNode,
    origin: Point,
    boxes: &mut HashMap<String, (BBox, String)>,
) {
    for child in &node.children {
        let at = Point::new(
            origin.x + child.x.unwrap_or(0.0),
            origin.y + child.y.unwrap_or(0.0),
        );
        boxes.insert(
            child.id.clone(),
            (
                BBox::new(
                    at.x,
                    at.y,
                    child.width.unwrap_or(0.0),
                    child.height.unwrap_or(0.0),
                ),
                node.id.clone(),
            ),
        );
        absolute_boxes(child, at, boxes);
    }
}

/// The node of ELK's result with the id, at any depth below `node`.
pub(crate) fn find_mut<'a>(node: &'a mut JsonNode, id: &str) -> Option<&'a mut JsonNode> {
    for child in &mut node.children {
        if child.id == id {
            return Some(child);
        }
        if let Some(found) = find_mut(child, id) {
            return Some(found);
        }
    }
    None
}

/// The box grown by `by` on every side.
pub(crate) fn grown(bbox: BBox, by: f64) -> BBox {
    BBox::new(
        bbox.x - by,
        bbox.y - by,
        bbox.width + 2.0 * by,
        bbox.height + 2.0 * by,
    )
}

/// Whether two boxes share more than an edge.
pub(crate) fn overlaps(a: BBox, b: BBox) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

/// Whether `outer` holds all of `inner`.
pub(crate) fn contains(outer: BBox, inner: BBox) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.right() <= outer.right()
        && inner.bottom() <= outer.bottom()
}

/// Whether the segment from `a` to `b` passes through the inside of the box.
pub(crate) fn crosses(a: Point, b: Point, bbox: BBox) -> bool {
    // Liang-Barsky clipping of the segment against the box.
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (mut low, mut high) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, a.x - bbox.x),
        (dx, bbox.right() - a.x),
        (-dy, a.y - bbox.y),
        (dy, bbox.bottom() - a.y),
    ] {
        if p == 0.0 {
            if q <= 0.0 {
                return false;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            low = low.max(t);
        } else {
            high = high.min(t);
        }
        if low >= high {
            return false;
        }
    }
    true
}

/// Moves the ends of the edges of ELK's result that meet a node of `own` from the side of the
/// node's box in the layout onto its own box, along their last part (see [`onto_outline`]). `own`
/// gives each such node its box in the layout and its own box, both absolute; `boxes` are the
/// absolute boxes of [`absolute_boxes`]. Where an end does not leave its node's box straight from a
/// side, the result is that node's id, with the graph unchanged.
pub(crate) fn ends_onto_outlines(
    graph: &mut JsonNode,
    boxes: &HashMap<String, (BBox, String)>,
    own: &HashMap<&str, (BBox, BBox)>,
    parent_by_id: &HashMap<String, String>,
) -> Result<(), String> {
    let origin = |id: Option<&String>| {
        id.and_then(|id| boxes.get(id))
            .map_or_else(Point::default, |(bbox, _)| Point::new(bbox.x, bbox.y))
    };
    let mut ends: Vec<(usize, bool, JsonPoint)> = Vec::new();
    for (position, edge) in graph.edges.iter().enumerate() {
        let (Some(source), Some(target)) = (edge.sources.first(), edge.targets.first()) else {
            continue;
        };
        let Some(section) = edge.sections.first() else {
            continue;
        };
        let offset = origin(find_common_ancestor(source, target, parent_by_id).as_ref());
        let absolute = |point: Option<JsonPoint>| {
            point.map(|point| Point::new(point.x + offset.x, point.y + offset.y))
        };
        let start = absolute(section.start_point);
        let end = absolute(section.end_point);
        let first_bend = absolute(section.bend_points.first().copied()).or(end);
        let last_bend = absolute(section.bend_points.last().copied()).or(start);
        for (at_start, id, point, next) in [
            (true, source, start, first_bend),
            (false, target, end, last_bend),
        ] {
            let Some(&(laid, real)) = own.get(id.as_str()) else {
                continue;
            };
            let (Some(point), Some(next)) = (point, next) else {
                return Err(id.clone());
            };
            let Some(moved) = onto_outline(point, next, laid, real) else {
                return Err(id.clone());
            };
            ends.push((
                position,
                at_start,
                JsonPoint {
                    x: moved.x - offset.x,
                    y: moved.y - offset.y,
                },
            ));
        }
    }
    for (position, at_start, point) in ends {
        let section = &mut graph.edges[position].sections[0];
        if at_start {
            section.start_point = Some(point);
        } else {
            section.end_point = Some(point);
        }
    }
    Ok(())
}
