//! Room beside a node in the default rendering: the geometry that the steps share which keep room
//! beside a node in the layout and set something into it afterwards (a state's note, a self-loop).
//! The steps are ferrigura's own and run around ELK's layout.

use std::collections::HashMap;

use ferrigura_elk::json::JsonNode;

use crate::unified::geom::{BBox, Point};

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

/// Takes the labels that kept room beside a node out of its node in ELK's result
/// (`elk_build::beside_label`); a group keeps its title.
pub(crate) fn clear_beside(node: &mut JsonNode) {
    node.labels
        .retain(|label| label.text.as_deref() != Some(crate::unified::elk_build::BESIDE_TEXT));
}
