//! Shared state and helpers of the materialized-geometry passes (`materializedGeometry.ts`).
//!
//! A pass reads the layout through a [`View`] (the nodes, the edges and `nodeByIdMap`) and edits the edge
//! points through [`Points`]. The reference assigns `edge.points` in place, and every later read of the pass
//! sees the assignment, so the reads go to the working copy in [`Points`] as well. The copy is written back
//! to the edges when the pass ends ([`edit_points`]); an edge that the pass did not touch keeps its points,
//! `undefined` included.

use std::cmp::Ordering;

use indexmap::IndexMap;
use serde_json::Value;

// The geometry of `geometry.ts` that the passes call. The functions are in `direction/geometry.rs`.
pub(super) use crate::direction::geometry::{
    EPS, OrthogonalSegment, RectEntry, RectSide, collect_node_rect_entries, count_orthogonal_bends,
    dedupe_consecutive_points, is_horizontal_segment, is_vertical_segment, js_sign_value,
    orthogonal_segments_for_points, orthogonal_segments_strictly_cross, overlap_length,
    rect_of_node_bounds, same_axis_segment_overlap_length, same_point, same_x, same_y,
    segment_hits_any_rect, simplify_polyline, truthy_text,
};
use crate::math::{js_max, js_min, js_sign_order};
use crate::model::{Direction, Edge, Extra, LayoutData, Node, Point, Rect};

/// `MIN_SHARED`: the shortest overlap of two rails that counts as shared.
pub(super) const MIN_SHARED: f64 = 8.0;

/// The replacement points of some edges, keyed by the index of the edge, in insertion order (`Map<Edge,
/// Point[]>` in the reference).
pub(super) type Replacements = IndexMap<usize, Vec<Point>>;

/// `Number(flag)`, for the arithmetic of the comparators.
pub(super) fn number(flag: bool) -> f64 {
    if flag { 1.0 } else { 0.0 }
}

/// `a || b` for numbers: `a` when it is truthy (neither `0`, `-0` nor NaN), otherwise `b()`.
pub(super) fn js_or(a: f64, b: impl FnOnce() -> f64) -> f64 {
    if a != 0.0 && !a.is_nan() { a } else { b() }
}

/// `Math.min(...values)`: `+Infinity` for no values, NaN when one of them is NaN.
pub(super) fn js_min_of(values: impl IntoIterator<Item = f64>) -> f64 {
    values.into_iter().fold(f64::INFINITY, js_min)
}

/// `Math.max(...values)`: `-Infinity` for no values, NaN when one of them is NaN.
pub(super) fn js_max_of(values: impl IntoIterator<Item = f64>) -> f64 {
    values.into_iter().fold(f64::NEG_INFINITY, js_max)
}

/// The order that a comparator returning `difference` gives to its elements.
pub(super) fn order_by_difference(difference: f64) -> Ordering {
    js_sign_order(difference)
}

/// The JavaScript truthiness of a value of the untyped `extra` maps.
fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// A boolean property that the model may keep untyped: its typed value, or else its truthiness in `extra`.
fn flag(typed: Option<bool>, extra: &Extra, key: &str) -> bool {
    typed.unwrap_or_else(|| extra.get(key).is_some_and(js_truthy))
}

/// `(edge as { isLayoutOnly?: boolean }).isLayoutOnly`, as a truth value.
pub(super) fn is_layout_only(edge: &Edge) -> bool {
    flag(edge.is_layout_only, &edge.extra, "isLayoutOnly")
}

/// `node.isGroup`, as a truth value.
pub(super) fn is_group(node: &Node) -> bool {
    flag(node.is_group, &node.extra, "isGroup")
}

/// `node.isEdgeLabel`, as a truth value.
pub(super) fn is_edge_label(node: &Node) -> bool {
    flag(node.is_edge_label, &node.extra, "isEdgeLabel")
}

/// `node.parentId` is truthy, that is a non-empty string or another truthy value.
pub(super) fn has_parent(node: &Node) -> bool {
    match &node.parent_id {
        Some(id) => !id.is_empty(),
        None => node.extra.get("parentId").is_some_and(js_truthy),
    }
}

/// `upperCaseDirection(node.direction ?? layoutDirection)`: the direction of a lane, in capitals. A node
/// without a `direction` takes the layout's; a `direction` that is not a string gives the empty string.
pub(super) fn lane_direction(node: &Node, layout: Direction) -> String {
    match node.extra.get("direction") {
        None | Some(Value::Null) => layout.as_str().to_owned(),
        Some(Value::String(text)) => text.to_uppercase(),
        Some(_) => String::new(),
    }
}

/// `value ?? 0`.
pub(super) fn or_zero(value: Option<f64>) -> f64 {
    value.unwrap_or(0.0)
}

/// `validTitleRect(node)`: the title band of a top-level group, when all four sides are finite and the band
/// has a positive width and height.
pub(super) fn valid_title_rect(node: &Node) -> Option<Rect> {
    let rect = node.group_title_rect?;
    let finite = rect.left.is_finite()
        && rect.right.is_finite()
        && rect.top.is_finite()
        && rect.bottom.is_finite();
    if !finite || rect.right <= rect.left || rect.bottom <= rect.top {
        return None;
    }
    Some(rect)
}

/// `orthogonallyAligned(a, b)`: the two points share a row or a column, within `EPS`.
pub(super) fn orthogonally_aligned(a: Point, b: Point) -> bool {
    same_x(a, b, EPS) || same_y(a, b, EPS)
}

/// The ids of the endpoints of an edge that are truthy, `[start, end].filter(Boolean)`.
pub(super) fn endpoint_ids(edge: &Edge) -> Vec<&str> {
    [edge.start.as_deref(), edge.end.as_deref()]
        .into_iter()
        .flatten()
        .filter(|id| !id.is_empty())
        .collect()
}

/// The layout as the passes read it: the nodes and the edges, and `nodeByIdMap`, the map from each id to
/// its node. Like the reference's `Map`, it keeps the first position of an id and the last node that has it.
pub(super) struct View<'a> {
    pub(super) nodes: &'a [Node],
    pub(super) edges: &'a [Edge],
    by_id: IndexMap<String, usize>,
}

impl<'a> View<'a> {
    pub(super) fn new(nodes: &'a [Node], edges: &'a [Edge]) -> Self {
        let mut by_id = IndexMap::new();
        for (index, node) in nodes.iter().enumerate() {
            by_id.insert(node.id.clone(), index);
        }
        View {
            nodes,
            edges,
            by_id,
        }
    }

    /// The index of the node of an id, `nodeByIdMap.get(id)` for a truthy id, `undefined` for a falsy one.
    pub(super) fn node_index(&self, id: Option<&str>) -> Option<usize> {
        let id = truthy_text(id)?;
        self.by_id.get(id).copied()
    }

    /// The node of an id, as [`View::node_index`].
    pub(super) fn node(&self, id: Option<&str>) -> Option<&'a Node> {
        let nodes: &'a [Node] = self.nodes;
        self.node_index(id).map(|index| &nodes[index])
    }

    /// `nodeByIdMap.has(id)` for a truthy id.
    pub(super) fn has_node(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// The indices of the nodes of `nodeByIdMap.values()`, in the map's order.
    pub(super) fn node_indices(&self) -> Vec<usize> {
        self.by_id.values().copied().collect()
    }

    /// The indices of the edges that are not layout-only, in order (`edges.filter(e => !e.isLayoutOnly)`).
    pub(super) fn visible(&self) -> Vec<usize> {
        (0..self.edges.len())
            .filter(|&index| !is_layout_only(&self.edges[index]))
            .collect()
    }

    /// `nodeByIdMap.values()`, as nodes, in the map's order.
    pub(super) fn node_values(&self) -> Vec<Node> {
        self.node_indices()
            .into_iter()
            .map(|index| self.nodes[index].clone())
            .collect()
    }
}

/// The points of the edges that a pass edits: a working copy of `edge.points`, with `None` where the edge has
/// no points.
pub(super) struct Points(Vec<Option<Vec<Point>>>);

impl Points {
    pub(super) fn new(edges: &[Edge]) -> Self {
        Points(edges.iter().map(|edge| edge.points.clone()).collect())
    }

    /// `edge.points ?? []`, without the deduplication.
    pub(super) fn raw(&self, edge: usize) -> &[Point] {
        self.0[edge].as_deref().unwrap_or(&[])
    }

    /// `edge.points = points`.
    pub(super) fn set(&mut self, edge: usize, points: Vec<Point>) {
        self.0[edge] = Some(points);
    }

    fn write(self, edges: &mut [Edge]) {
        for (edge, points) in edges.iter_mut().zip(self.0) {
            edge.points = points;
        }
    }
}

/// Runs a pass that edits the edge points: `pass` reads the layout through the view and the points through
/// the working copy, and the edited points are written back when it returns.
pub(super) fn edit_points(data: &mut LayoutData, pass: impl FnOnce(&View, &mut Points)) {
    let mut points = Points::new(&data.edges);
    {
        let view = View::new(&data.nodes, &data.edges);
        pass(&view, &mut points);
    }
    points.write(&mut data.edges);
}

/// `replacementPointsFor(edge, replacements)`: the deduplicated points of an edge, or of its replacement
/// when it has one (`dedupeConsecutivePoints(replacements.get(edge) ?? edge.points ?? [])`).
pub(super) fn points_of(points: &Points, replacements: &Replacements, edge: usize) -> Vec<Point> {
    let raw: &[Point] = match replacements.get(&edge) {
        Some(replacement) => replacement,
        None => points.raw(edge),
    };
    dedupe_consecutive_points(raw, EPS)
}

/// `segmentsFor(points)`: the orthogonal segments of a path, with the default epsilon.
pub(super) fn segments_for(points: &[Point]) -> Vec<OrthogonalSegment> {
    orthogonal_segments_for_points(points, EPS)
}

/// The number of strict crossings between two sets of segments: each crossing pair counts once.
pub(super) fn crossings_between(first: &[OrthogonalSegment], second: &[OrthogonalSegment]) -> f64 {
    let mut count = 0.0;
    for a in first {
        for b in second {
            if orthogonal_segments_strictly_cross(a.a, a.b, b.a, b.b, EPS) {
                count += 1.0;
            }
        }
    }
    count
}

/// `strictCrossingCount`: the number of strict crossings between the visible edges, where an edge has the
/// points of `replacements` when it is in them.
pub(super) fn strict_crossing_count(
    visible: &[usize],
    points: &Points,
    replacements: &Replacements,
) -> f64 {
    let segments: Vec<Vec<OrthogonalSegment>> = visible
        .iter()
        .map(|&edge| segments_for(&points_of(points, replacements, edge)))
        .collect();
    let mut count = 0.0;
    for i in 0..segments.len() {
        for j in (i + 1)..segments.len() {
            count += crossings_between(&segments[i], &segments[j]);
        }
    }
    count
}

/// `totalBends`: the orthogonal bends of the visible edges, with the replacements.
pub(super) fn total_bends(visible: &[usize], points: &Points, replacements: &Replacements) -> f64 {
    visible.iter().fold(0.0, |sum, &edge| {
        sum + count_orthogonal_bends(&points_of(points, replacements, edge), EPS) as f64
    })
}

/// `collectNodeRectEntries(nodeByIdMap.values())`: the rectangles of the real nodes and of the edge labels.
pub(super) fn node_rect_entries(view: &View) -> (Vec<RectEntry>, Vec<RectEntry>) {
    collect_node_rect_entries(&view.node_values())
}

/// The key of a path for the dedup of the candidates: `x.toFixed(3),y.toFixed(3)` of each point, joined by `|`.
pub(super) fn path_key(points: &[Point]) -> String {
    points
        .iter()
        .map(|point| {
            format!(
                "{},{}",
                crate::js::to_fixed(point.x, 3),
                crate::js::to_fixed(point.y, 3)
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// `simplifyPolyline(dedupeConsecutivePoints(points))`.
pub(super) fn simplified(points: &[Point]) -> Vec<Point> {
    simplify_polyline(&dedupe_consecutive_points(points, EPS))
}

/// `segmentsFor(path).length === path.length - 1`: every consecutive pair is a horizontal or vertical segment.
pub(super) fn is_orthogonal_path(path: &[Point]) -> bool {
    segments_for(path).len() + 1 == path.len()
}
