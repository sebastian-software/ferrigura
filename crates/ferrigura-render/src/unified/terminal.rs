//! The terminal labels of an edge (the cardinalities at its ends): their measure, the ELK labels that
//! reserve room for them, the passes that move them after the layout, and their paint. A port of
//! `rendering-util/rendering-elements/edges.js` (`insertEdgeLabel`, `recordTerminalLabelSize`,
//! `positionEdgeLabel`), `rendering-util/layout-algorithms/elk/render.ts` (`terminalElkLabels`, the
//! passes after `addEdgesToElkGraph`'s layout, lines 2085-2364) and `layout-algorithms/common/index.ts`
//! (`terminalLabelTranslate`). The research is in `docs-port/class/NOTES.md`, section 5.5.
//!
//! Only the class relations have terminal labels: the class DB gives `startLabelRight` and
//! `endLabelLeft`. The other two keys are the reference's and are kept for the passes.

use super::data::{Edge, Node, Terminal, TerminalKey};
use super::geom::{BBox, Point};
use super::label::{self, Anchor, LabelSpec};
use crate::RenderError;
use ferrigura_jsmath::{atan2, cos, hypot, sin};
use ferrigura_model::flowchart::LabelType;

/// The clearance a marker needs beside an end: its paths span 12 px (`TERMINAL_LABEL_MARKER_CLEARANCE`,
/// `elk/render.ts` 1145).
const MARKER_CLEARANCE: f64 = 6.0;

/// The width a terminal label wraps at: `createLabel`'s default (`createText`, 200).
const WRAP_WIDTH: f64 = 200.0;

/// The font size of a terminal label: `.edgeTerminals { font-size: 11px }` in the class styles
/// (`styles.js` 218-221). The label is measured in it, so the ELK label is sized in it.
const TERMINAL_FONT_SIZE: f64 = 11.0;

/// The space a slid label keeps off a group's frame (`TERMINAL_LABEL_FRAME_CLEARANCE`, `render.ts` 2268).
const FRAME_CLEARANCE: f64 = 2.0;

/// The terminal labels of an edge, measured as `insertEdgeLabel` creates them: the start-right text,
/// then the end-left text, each only when it is not empty (`edges.js` 139-223). A diagram whose
/// stylesheet sets no terminal font size passes the theme's size instead.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn measure(edge: &Edge) -> Result<Vec<Terminal>, RenderError> {
    let candidates = [
        (TerminalKey::StartRight, edge.start_label_right.as_deref()),
        (TerminalKey::EndLeft, edge.end_label_left.as_deref()),
    ];
    let mut terminals = Vec::new();
    for (key, text) in candidates {
        let Some(text) = text.filter(|text| !text.is_empty()) else {
            continue;
        };
        let spec = LabelSpec {
            text,
            label_type: LabelType::Markdown,
            width: WRAP_WIDTH,
            is_node: false,
            add_background: false,
            anchor: Anchor::Middle,
            wrap_in_style: false,
            styles: &[],
            font_size: TERMINAL_FONT_SIZE,
            title: false,
        };
        let label = label::layout(&spec)?;
        // `recordTerminalLabelSize` keeps the label's `getBBox`, which the layout reads unpadded.
        let bbox = label.bbox.browser();
        terminals.push(Terminal {
            key,
            text: text.to_owned(),
            label,
            width: bbox.width,
            height: bbox.height,
            centre: None,
        });
    }
    Ok(terminals)
}

/// The marker of the end a terminal label sits at: the start's or the end's arrow type.
fn marker_of(edge: &Edge, key: TerminalKey) -> &str {
    if key.at_start() {
        &edge.arrow_type_start
    } else {
        &edge.arrow_type_end
    }
}

/// The padding around a terminal label's size that ELK reserves: the marker's clearance, when the
/// end has a marker (`terminalElkLabels`, `render.ts` 1148-1170).
#[must_use]
pub fn elk_padding(edge: &Edge, key: TerminalKey) -> f64 {
    let marker = marker_of(edge, key);
    if marker.is_empty() || marker == "none" {
        0.0
    } else {
        MARKER_CLEARANCE
    }
}

/// The box of a label of `size` centred on `centre`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box4 {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

/// `labelBox` (`render.ts` 1850).
fn label_box(centre: Point, width: f64, height: f64) -> Box4 {
    Box4 {
        x1: centre.x - width / 2.0,
        y1: centre.y - height / 2.0,
        x2: centre.x + width / 2.0,
        y2: centre.y + height / 2.0,
    }
}

/// `boxesOverlap` (`render.ts` 2178).
fn boxes_overlap(a: Box4, b: Box4) -> bool {
    a.x1 < b.x2 && b.x1 < a.x2 && a.y1 < b.y2 && b.y1 < a.y2
}

/// `segmentHitsBox` (`render.ts` 1826-1848): a Liang-Barsky clip of the segment against the box,
/// with the same comparisons, so that a segment that only touches the box does not hit it.
fn segment_hits_box(a: Point, b: Point, bounds: Box4) -> bool {
    let mut t0 = 0.0_f64;
    let mut t1 = 1.0_f64;
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    for (p, q) in [
        (-dx, a.x - bounds.x1),
        (dx, bounds.x2 - a.x),
        (-dy, a.y - bounds.y1),
        (dy, bounds.y2 - a.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else if p < 0.0 {
            t0 = t0.max(q / p);
        } else {
            t1 = t1.min(q / p);
        }
    }
    t0 < t1
}

/// The four sides of a box, as segments.
fn frame_segments(bounds: Box4) -> [(Point, Point); 4] {
    let corner = |x: f64, y: f64| Point::new(x, y);
    [
        (corner(bounds.x1, bounds.y1), corner(bounds.x2, bounds.y1)),
        (corner(bounds.x2, bounds.y1), corner(bounds.x2, bounds.y2)),
        (corner(bounds.x2, bounds.y2), corner(bounds.x1, bounds.y2)),
        (corner(bounds.x1, bounds.y2), corner(bounds.x1, bounds.y1)),
    ]
}

/// What a terminal label must not touch: the boxes of the nodes, the centre labels, the frames of
/// the groups, the other edges and their terminal labels (`terminalLabelObstacles`, `render.ts`
/// 2179-2242). The nodes and frames are fixed while the labels move, so they are taken once; the
/// edges are read as they are at each check.
struct Obstacles {
    /// The boxes of the placed nodes (not groups).
    nodes: Vec<Box4>,
    /// The boxes of the centre labels.
    centre_labels: Vec<Box4>,
    /// The sides of the placed groups' frames.
    frames: Vec<(Point, Point)>,
}

impl Obstacles {
    fn new(edges: &[Edge], nodes: &[Node]) -> Self {
        let boxed = |node: &Node| {
            label_box(
                Point::new(node.x.unwrap_or(0.0), node.y.unwrap_or(0.0)),
                node.width.unwrap_or(0.0),
                node.height.unwrap_or(0.0),
            )
        };
        let placed = nodes
            .iter()
            .filter(|node| node.x.is_some() && node.y.is_some());
        let nodes_boxes = placed
            .clone()
            .filter(|node| !node.is_group)
            .map(boxed)
            .collect();
        // A group is hollow, so only its frame is in the way.
        let frames = placed
            .filter(|node| node.is_group)
            .map(boxed)
            .flat_map(frame_segments)
            .collect();
        // A centre label counts when it has text and a size (`edge.label && edge.width && …`).
        let centre_labels = edges
            .iter()
            .filter(|edge| {
                !edge.label.is_empty()
                    && edge.width.unwrap_or(0.0) != 0.0
                    && edge.height.unwrap_or(0.0) != 0.0
            })
            .filter_map(|edge| {
                let centre = Point::new(edge.x?, edge.y?);
                Some(label_box(centre, edge.width?, edge.height?))
            })
            .collect();
        Self {
            nodes: nodes_boxes,
            centre_labels,
            frames,
        }
    }

    /// `hitsFrame`: the box crosses a frame's side, padded by `clearance`.
    fn hits_frame(&self, bounds: Box4, clearance: f64) -> bool {
        let padded = Box4 {
            x1: bounds.x1 - clearance,
            y1: bounds.y1 - clearance,
            x2: bounds.x2 + clearance,
            y2: bounds.y2 + clearance,
        };
        self.frames
            .iter()
            .any(|&(a, b)| segment_hits_box(a, b, padded))
    }

    /// `blocked`: the box touches a node, a centre label, a frame, another edge's route, or another
    /// terminal label. `edge` and `key` are the label that moves; `segment` is its own route segment,
    /// which is not an obstacle to it.
    fn blocked(
        &self,
        edges: &[Edge],
        bounds: Box4,
        edge: usize,
        key: TerminalKey,
        segment: usize,
        clearance: f64,
    ) -> bool {
        if self.nodes.iter().any(|node| boxes_overlap(bounds, *node)) {
            return true;
        }
        if self
            .centre_labels
            .iter()
            .any(|label| boxes_overlap(bounds, *label))
        {
            return true;
        }
        if self.hits_frame(bounds, clearance) {
            return true;
        }
        let route_hit = edges.iter().enumerate().any(|(index, other)| {
            other.points.iter().enumerate().any(|(i, point)| {
                i + 1 < other.points.len()
                    && !(index == edge && i == segment)
                    && segment_hits_box(*point, other.points[i + 1], bounds)
            })
        });
        if route_hit {
            return true;
        }
        edges.iter().enumerate().any(|(index, other)| {
            other.terminals.iter().any(|terminal| {
                terminal.centre.is_some_and(|centre| {
                    !(index == edge && terminal.key == key)
                        && boxes_overlap(bounds, label_box(centre, terminal.width, terminal.height))
                })
            })
        })
    }
}

/// A placed terminal label with the end segment it sits beside.
struct Placed {
    /// The edge's index.
    edge: usize,
    /// The label's key.
    key: TerminalKey,
    /// Its centre.
    centre: Point,
    /// Its size.
    width: f64,
    height: f64,
    /// The index of its route segment.
    segment: usize,
    /// The segment's two ends.
    from: Point,
    to: Point,
    /// The segment's length.
    length: f64,
}

/// Every terminal label that has a centre and sits beside a route of at least two points with a
/// length (`placedTerminalLabels`, `render.ts` 2246-2268).
fn placed(edges: &[Edge]) -> Vec<Placed> {
    let mut out = Vec::new();
    for (index, edge) in edges.iter().enumerate() {
        let points = &edge.points;
        if points.len() < 2 {
            continue;
        }
        for terminal in &edge.terminals {
            let Some(centre) = terminal.centre else {
                continue;
            };
            let segment = if terminal.key.at_start() {
                0
            } else {
                points.len() - 2
            };
            let (from, to) = (points[segment], points[segment + 1]);
            let length = hypot(to.x - from.x, to.y - from.y);
            if length == 0.0 {
                continue;
            }
            out.push(Placed {
                edge: index,
                key: terminal.key,
                centre,
                width: terminal.width,
                height: terminal.height,
                segment,
                from,
                to,
                length,
            });
        }
    }
    out
}

/// Sets the centre of the terminal label `key` of `edge`.
fn set_centre(edges: &mut [Edge], edge: usize, key: TerminalKey, centre: Point) {
    if let Some(terminal) = edges[edge].terminals.iter_mut().find(|t| t.key == key) {
        terminal.centre = Some(centre);
    }
}

/// `followMovedEndpoints` (`render.ts` 2345-2364): a label moves with its endpoint, which the
/// clipping and straightening passes may have slid along the node's side.
pub fn follow_moved_endpoints(edge: &mut Edge) {
    let (Some((start, end)), Some(&first), Some(&last)) =
        (edge.ports, edge.points.first(), edge.points.last())
    else {
        return;
    };
    let moved_start = Point::new(first.x - start.x, first.y - start.y);
    let moved_end = Point::new(last.x - end.x, last.y - end.y);
    for terminal in &mut edge.terminals {
        if let Some(centre) = terminal.centre {
            let delta = if terminal.key.at_start() {
                moved_start
            } else {
                moved_end
            };
            terminal.centre = Some(Point::new(centre.x + delta.x, centre.y + delta.y));
        }
    }
}

/// `slideTerminalLabelsOffFrames` (`render.ts` 2275-2312): a label that crosses a group's frame is
/// slid along its end segment, by the shortest distance, until it clears the frames.
pub fn slide_off_frames(edges: &mut [Edge], nodes: &[Node]) {
    let obstacles = Obstacles::new(edges, nodes);
    for item in placed(edges) {
        let size = (item.width, item.height);
        if !obstacles.hits_frame(label_box(item.centre, size.0, size.1), 0.0) {
            continue;
        }
        // The unit direction from the far point of the segment towards its end.
        let (far, end) = if item.key.at_start() {
            (item.to, item.from)
        } else {
            (item.from, item.to)
        };
        let dir = Point::new((end.x - far.x) / item.length, (end.y - far.y) / item.length);
        let half_extent = (dir.x.abs() * size.0 + dir.y.abs() * size.1) / 2.0;
        // Where the centre sits along the segment: 0 at its end, -length at the far point.
        let along = |c: Point| (c.x - end.x) * dir.x + (c.y - end.y) * dir.y;
        let on_segment =
            |c: Point| along(c) + half_extent <= 0.0 && along(c) - half_extent >= -item.length;
        let mut step = 1.0;
        while step <= item.length {
            let found = [step, -step]
                .into_iter()
                .map(|shift| {
                    Point::new(item.centre.x + shift * dir.x, item.centre.y + shift * dir.y)
                })
                .find(|moved| {
                    on_segment(*moved)
                        && !obstacles.blocked(
                            edges,
                            label_box(*moved, size.0, size.1),
                            item.edge,
                            item.key,
                            item.segment,
                            FRAME_CLEARANCE,
                        )
                });
            if let Some(found) = found {
                set_centre(edges, item.edge, item.key, found);
                break;
            }
            step += 1.0;
        }
    }
}

/// `putTerminalLabelsOnTheirSide` (`render.ts` 2322-2338): a label on the wrong side of its end
/// segment is mirrored across it, onto free space only.
pub fn put_on_their_side(edges: &mut [Edge], nodes: &[Node]) {
    let obstacles = Obstacles::new(edges, nodes);
    for item in placed(edges) {
        // The unit normal to the right of the direction of travel (y points down).
        let normal = Point::new(
            -(item.to.y - item.from.y) / item.length,
            (item.to.x - item.from.x) / item.length,
        );
        let offset =
            (item.centre.x - item.from.x) * normal.x + (item.centre.y - item.from.y) * normal.y;
        if offset == 0.0 || (offset > 0.0) == item.key.on_right() {
            continue;
        }
        let mirrored = Point::new(
            item.centre.x - 2.0 * offset * normal.x,
            item.centre.y - 2.0 * offset * normal.y,
        );
        if !obstacles.blocked(
            edges,
            label_box(mirrored, item.width, item.height),
            item.edge,
            item.key,
            item.segment,
            0.0,
        ) {
            set_centre(edges, item.edge, item.key, mirrored);
        }
    }
}

/// The centre of a terminal label for painting (`terminalLabelTranslate`, `common/index.ts`
/// 364-378): the layout's centre when it placed the label, else the position along the path.
#[must_use]
pub fn translate(edge: &Edge, terminal: &Terminal) -> Point {
    if let Some(centre) = terminal.centre {
        return centre;
    }
    if edge.points.is_empty() {
        return Point::new(edge.x.unwrap_or(0.0), edge.y.unwrap_or(0.0));
    }
    let arrow = marker_of(edge, terminal.key);
    let size = if arrow.is_empty() || arrow == "none" {
        0.0
    } else {
        10.0
    };
    calc_position(size, terminal.key, &edge.points)
}

/// `calcTerminalLabelPosition` (`utils.ts` 378-410): the point beside the path, a set distance from
/// its end, on the side the key names.
fn calc_position(marker_size: f64, key: TerminalKey, points: &[Point]) -> Point {
    let mut points = points.to_vec();
    if !key.at_start() {
        points.reverse();
    }
    let Some(&first) = points.first() else {
        return Point::new(0.0, 0.0);
    };
    // Traverse 25 plus the marker size along the path to find the point the label is beside.
    let centre = calculate_point(&points, 25.0 + marker_size).unwrap_or(first);
    let d = 10.0 + marker_size * 0.5;
    let angle = atan2(first.y - centre.y, first.x - centre.x);
    let mid = Point::new((first.x + centre.x) / 2.0, (first.y + centre.y) / 2.0);
    match key {
        TerminalKey::StartLeft => Point::new(
            sin(angle + std::f64::consts::PI) * d + mid.x,
            -cos(angle + std::f64::consts::PI) * d + mid.y,
        ),
        TerminalKey::EndRight => Point::new(
            sin(angle - std::f64::consts::PI) * d + mid.x,
            -cos(angle - std::f64::consts::PI) * d + mid.y,
        ),
        TerminalKey::EndLeft | TerminalKey::StartRight => {
            Point::new(sin(angle) * d + mid.x, -cos(angle) * d + mid.y)
        }
    }
}

/// `calculatePoint` (`utils.ts` 336-363): the point a distance along the polyline, rounded to 1e-5
/// as the reference rounds it. `None` where the reference throws.
fn calculate_point(points: &[Point], distance: f64) -> Option<Point> {
    let mut previous: Option<Point> = None;
    let mut remaining = distance;
    for &point in points {
        if let Some(prev) = previous {
            let length = hypot(point.x - prev.x, point.y - prev.y);
            if length == 0.0 {
                return Some(prev);
            }
            if length < remaining {
                remaining -= length;
            } else {
                let ratio = remaining / length;
                if ratio <= 0.0 {
                    return Some(prev);
                }
                if ratio >= 1.0 {
                    return Some(point);
                }
                return Some(Point::new(
                    round5((1.0 - ratio) * prev.x + ratio * point.x),
                    round5((1.0 - ratio) * prev.y + ratio * point.y),
                ));
            }
        }
        previous = Some(point);
    }
    None
}

/// `roundNumber(value, 5)`: the reference rounds a computed point to five decimals.
fn round5(value: f64) -> f64 {
    (value * 1e5).round() / 1e5
}

/// The paint of a terminal label (`positionEdgeLabel`, `edges.js` 326-373; `common/index.ts`
/// `positionRenderedEdgeLabel`): its `edgeTerminals` group at the label's centre, and its inner group
/// centring the text on that point (`computeLabelTransform`). Returns the markup and the box it
/// covers, in the diagram's coordinates.
#[must_use]
pub fn paint(edge: &Edge, terminal: &Terminal) -> (String, BBox) {
    let centre = translate(edge, terminal);
    let bbox = terminal.label.bbox;
    let (inner_x, inner_y) = (-(bbox.x + bbox.width / 2.0), -(bbox.y + bbox.height / 2.0));
    let svg = format!(
        "<g class=\"edgeTerminals\" transform=\"translate({}, {})\"><g class=\"inner\" transform=\"translate({}, {})\">{}</g></g>",
        crate::svg::number(centre.x),
        crate::svg::number(centre.y),
        crate::svg::number(inner_x),
        crate::svg::number(inner_y),
        terminal.label.svg(""),
    );
    (svg, bbox.translate(inner_x + centre.x, inner_y + centre.y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_start_label_without_a_marker_sits_ten_pixels_beside_its_end_on_the_left() {
        // `calcTerminalLabelPosition` (`utils.ts` 378-410): the point 25 along the path, the middle of
        // the first segment, and an offset of 10 (no marker) to the left of the direction of travel.
        let path = [Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
        let at = calc_position(0.0, TerminalKey::StartLeft, &path);
        assert!((at.x - 12.5).abs() < 1e-9, "{at:?}");
        assert!((at.y + 10.0).abs() < 1e-9, "{at:?}");
    }

    #[test]
    fn a_point_is_found_by_its_distance_along_the_path() {
        let path = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ];
        assert_eq!(calculate_point(&path, 15.0), Some(Point::new(10.0, 5.0)));
        assert_eq!(calculate_point(&path, 40.0), None);
    }

    #[test]
    fn a_segment_beside_a_box_misses_it_and_one_through_it_hits_it() {
        // `segmentHitsBox` (`render.ts` 1826-1848): a segment on the box's own side is a hit, as the
        // comparisons are strict only outside the box.
        let bounds = Box4 {
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 10.0,
        };
        assert!(!segment_hits_box(
            Point::new(11.0, -5.0),
            Point::new(11.0, 15.0),
            bounds
        ));
        assert!(segment_hits_box(
            Point::new(5.0, -5.0),
            Point::new(5.0, 15.0),
            bounds
        ));
        assert!(segment_hits_box(
            Point::new(10.0, -5.0),
            Point::new(10.0, 5.0),
            bounds
        ));
    }
}
