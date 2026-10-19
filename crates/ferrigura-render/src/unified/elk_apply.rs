//! What the reference does with the elkjs result: a port of `applyElkLayoutResult`,
//! `applyElkNodePositions`, `evenGroupFrames`, `applyElkEdgeLayout` and their helpers in
//! `packages/mermaid/src/rendering-util/layout-algorithms/elk/render.ts` (lines 1325 to 2938 of that
//! file), with `elk/geometry.ts` ([`super::elk_geometry`]) and `elk/find-common-ancestor.ts`. See
//! `docs-port/flowchart/render-notes/elk-apply.md`.
//!
//! Afterwards every node has its centre and size, every group its frame, and every edge its points
//! (clipped at the outlines, in absolute coordinates), its curve and its label centre. The nodes are
//! in the reference's paint order (`orderNodesForElkPaint`).
//!
//! The reference keeps a second copy of every node (`nodeDb`) for the post-processing, with the
//! absolute top-left `offset`, the centre and the ELK size. That copy is [`DbNode`] here; the layout
//! node in `data` gets the same centre and size as the reference writes them back.
//!
//! The terminal labels of the edges (their centres, `followMovedEndpoints`,
//! `slideTerminalLabelsOffFrames`, `putTerminalLabelsOnTheirSide`) are in [`super::terminal`]. The
//! flowchart has none, so these passes never change a flowchart.

use std::collections::{HashMap, HashSet};

use super::data::{Edge, LayoutData, Node};
use super::elk_build::ElkState;
use super::elk_geometry::{
    NodeOutline, Side, compute_node_intersection, find_common_ancestor, js_hypot, js_sign,
    on_border, outline_attach_point, outside_node, replace_endpoint,
};
use super::elk_straighten::{project_label_onto_straightened_run, straighten_edge_terminals};
use super::geom::{CenterBox, Point};
use super::shapes::Intersect;
use super::terminal;
use ferrigura_elk::json::{JsonEdge, JsonEdgeSection, JsonNode, JsonPoint};
use ferrigura_jsmath::pow;

/// Padding between a subgraph frame and its children (`SUBGRAPH_PADDING`).
const SUBGRAPH_PADDING: f64 = 24.0;

/// Margin reserved at the ends of each side of a node (`PORTS_SURROUNDING_MARGIN`). A side shorter
/// than twice this has no usable anchor span.
const PORTS_SURROUNDING_MARGIN: f64 = 12.0;

/// The shortest final marker segment (`MIN_END_MARKER_SEGMENT_LENGTH`).
const MIN_END_MARKER_SEGMENT_LENGTH: f64 = 8.0;

/// The gap kept between two opposite-direction edges' labels, beyond their half-widths
/// (`OPPOSITE_LABEL_MIN_GAP`).
const OPPOSITE_LABEL_MIN_GAP: f64 = 4.0;

/// Applies the elkjs result to the layout data.
///
/// `intersects` holds the outline of every leaf node by id. Groups have none during layout, as in
/// the reference, whose cluster intersect is set only when the cluster is painted. `straighten_edges`
/// is `elk.straightenEdges`.
pub fn apply(
    data: &mut LayoutData,
    output: &JsonNode,
    state: &ElkState,
    intersects: &HashMap<String, Intersect>,
    straighten_edges: bool,
) {
    let crossing = |id: &str, node: CenterBox, probe: Point| {
        intersects.get(id).map(|shape| shape.intersect(node, probe))
    };
    apply_with_crossing(data, output, state, &crossing, straighten_edges);
}

/// The crossing of a leaf's outline: where the ray from the leaf's centre through a probe point
/// leaves the outline, or `None` for a node without one. The leaf is given by its id and its box.
pub type Crossing<'a> = &'a dyn Fn(&str, CenterBox, Point) -> Option<Point>;

/// [`apply`], with the leaves' outlines given as a function instead of the shapes' intersects. The
/// differential test passes the reference's `intersect-rect` for rectangular leaves until the other
/// shapes' intersects are ported; [`apply`] passes the shapes' own.
pub fn apply_with_crossing(
    data: &mut LayoutData,
    output: &JsonNode,
    state: &ElkState,
    crossing: Crossing<'_>,
    straighten: bool,
) {
    let mut layout = Layout::new(data, state, crossing);
    layout.apply_node_positions(data, &output.children, Point::default());
    layout.even_group_frames(data, &output.children, output);
    layout.apply_edge_layout(data, output, straighten);
    order_nodes_for_elk_paint(&mut data.nodes);
}

/// A node as the post-processing sees it (`nodeDb` entry in the reference).
#[derive(Debug, Clone, Default)]
struct DbNode {
    /// Whether the node is a subgraph (`isGroup`).
    is_group: bool,
    /// The measured title width (`labelData.width`), 0 for a leaf.
    label_width: f64,
    /// The width the title needs, `labelData.width + padding`, or 0 when no title is painted
    /// (`groupTitleWidth`).
    title_width: f64,
    /// Whether `pos` has been set by [`Layout::apply_node_positions`] (`offset` exists).
    positioned: bool,
    /// The absolute top-left corner of the node (`offset.posX`, `offset.posY`).
    pos: Point,
    /// Where ELK placed the container, kept once a frame has moved (`elkOrigin`).
    elk_origin: Option<Point>,
    /// The centre (`node.x`, `node.y`).
    centre: Point,
    /// The size ELK gave the node (`node.width`, `node.height`).
    width: f64,
    /// See `width`.
    height: f64,
}

/// The outline of a leaf: the crossing function, applied at the node's box as it is now. A leaf
/// without a shape gives no crossing, which is what a missing `node.intersect` does in the
/// reference: every use then falls through to the same path.
struct LeafOutline<'a> {
    source: Crossing<'a>,
    id: String,
    node: CenterBox,
}

impl NodeOutline for LeafOutline<'_> {
    fn crossing(&self, probe: Point) -> Option<Point> {
        (self.source)(&self.id, self.node, probe)
    }
}

/// The state of one application: the copies of the nodes, and the links the post-processing reads.
struct Layout<'a> {
    nodes: HashMap<String, DbNode>,
    /// The position of each node in `LayoutData::nodes`, to write the layout node back.
    index: HashMap<String, usize>,
    state: &'a ElkState,
    crossing: Crossing<'a>,
}

impl<'a> Layout<'a> {
    /// Copies the layout nodes into the node table.
    fn new(data: &LayoutData, state: &'a ElkState, crossing: Crossing<'a>) -> Self {
        let mut nodes = HashMap::new();
        let mut index = HashMap::new();
        for (position, node) in data.nodes.iter().enumerate() {
            let label_width = state
                .label_data
                .get(&node.id)
                .map_or(0.0, |label| label.width);
            // `groupTitleWidth`: the title and its padding, for a shape that paints a title.
            let title_width = if cluster_paints_title(&node.shape) {
                label_width + node.padding
            } else {
                0.0
            };
            nodes.insert(
                node.id.clone(),
                DbNode {
                    is_group: node.is_group,
                    label_width,
                    title_width,
                    width: node.width.unwrap_or(0.0),
                    height: node.height.unwrap_or(0.0),
                    ..DbNode::default()
                },
            );
            index.insert(node.id.clone(), position);
        }
        Self {
            nodes,
            index,
            state,
            crossing,
        }
    }

    /// Whether the node is a subgraph in the table (`isGroup`).
    fn is_group(&self, id: &str) -> bool {
        self.nodes.get(id).is_some_and(|node| node.is_group)
    }

    /// `applyElkNodePositions`: absolute top-left corners, centres, and the ELK sizes, depth first.
    fn apply_node_positions(
        &mut self,
        data: &mut LayoutData,
        children: &[JsonNode],
        relative: Point,
    ) {
        for child in children {
            let pos = Point::new(
                child.x.unwrap_or(0.0) + relative.x,
                child.y.unwrap_or(0.0) + relative.y,
            );
            let width = child.width.unwrap_or(0.0);
            let height = child.height.unwrap_or(0.0);
            let is_group = self.is_group(&child.id);

            let node = self.nodes.entry(child.id.clone()).or_default();
            node.pos = pos;
            node.positioned = true;
            node.centre = Point::new(pos.x + width / 2.0, pos.y + height / 2.0);
            node.width = width;
            node.height = height;
            let (centre, label_width) = (node.centre, node.label_width);

            if let Some(&position) = self.index.get(&child.id) {
                let layout = &mut data.nodes[position];
                layout.x = Some(centre.x);
                layout.y = Some(centre.y);
                layout.width = Some(if is_group {
                    width.max(label_width)
                } else {
                    width
                });
                layout.height = Some(height);
            }

            if is_group {
                self.apply_node_positions(data, &child.children, pos);
            }
        }
    }

    /// `evenGroupFrames`: pulls each group's frame in around its contents, deepest groups first.
    ///
    /// The frame is only ever pulled in, except for the title floor, which may widen it up to the
    /// width ELK reserved. The top stays where ELK put it, since it carries the title strip.
    fn even_group_frames(
        &mut self,
        data: &mut LayoutData,
        children: &[JsonNode],
        graph: &JsonNode,
    ) {
        for elk_node in children {
            if !self.is_group(&elk_node.id) {
                continue;
            }
            self.even_group_frames(data, &elk_node.children, graph);

            let boxes: Vec<(Point, f64, f64)> = elk_node
                .children
                .iter()
                .filter_map(|child| self.nodes.get(&child.id))
                .filter(|child| child.positioned && truthy(child.width) && truthy(child.height))
                .map(|child| (child.pos, child.width, child.height))
                .collect();
            let Some(group) = self.nodes.get(&elk_node.id) else {
                continue;
            };
            if !group.positioned || boxes.is_empty() {
                continue;
            }
            let origin = group.pos;
            let group_width = group.width;
            let group_height = group.height;
            let title_width = group.title_width;

            let descendants = collect_descendant_ids(elk_node);
            let lane = self.internal_edge_points(graph, &descendants, &elk_node.id);
            let xs: Vec<f64> = boxes
                .iter()
                .flat_map(|(pos, width, _)| [pos.x, pos.x + width])
                .chain(lane.iter().map(|p| p.x))
                .collect();
            let ys: Vec<f64> = boxes
                .iter()
                .flat_map(|(pos, _, height)| [pos.y, pos.y + height])
                .chain(lane.iter().map(|p| p.y))
                .collect();
            let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
            let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);

            // Only ever pull a frame in: ELK sized it to hold everything it put there.
            let left = origin.x.max(min_x - SUBGRAPH_PADDING);
            let right = (origin.x + group_width).min(max_x + SUBGRAPH_PADDING);
            let bottom = (origin.y + group_height).min(max_y + SUBGRAPH_PADDING);
            let top = origin.y;

            // The title floor reserved before routing; still never past ELK's own frame.
            let mut x = left;
            let mut width = right - left;
            if width < title_width {
                x -= (title_width - width) / 2.0;
                width = title_width;
                let original_right = origin.x + group_width;
                x = origin.x.max(x.min(original_right - width));
                width = width.min(group_width);
            }
            let height = bottom - top;
            if height <= 0.0 || width <= 0.0 {
                continue;
            }

            // Edge sections resolve against the origin ELK chose, so keep it for `calc_offset`.
            let Some(group) = self.nodes.get_mut(&elk_node.id) else {
                continue;
            };
            group.elk_origin.get_or_insert(origin);
            group.pos.x = x;
            group.width = width;
            group.height = height;
            group.centre = Point::new(x + width / 2.0, top + height / 2.0);
            let centre = group.centre;

            if let Some(&position) = self.index.get(&elk_node.id) {
                let layout = &mut data.nodes[position];
                layout.x = Some(centre.x);
                layout.y = Some(centre.y);
                layout.width = Some(width);
                layout.height = Some(height);
            }
        }
    }

    /// The absolute points of the edges that run inside the group, and of those that end on it
    /// (`internalEdgePoints`). They widen the lane the frame must keep clear.
    fn internal_edge_points(
        &self,
        graph: &JsonNode,
        descendants: &HashSet<String>,
        group_id: &str,
    ) -> Vec<Point> {
        let mut points = Vec::new();
        for edge in &graph.edges {
            let (source, target) = elk_ends(edge);
            let is_internal = descendants.contains(&source) && descendants.contains(&target);
            let attaches_start = source == group_id;
            let attaches_end = target == group_id;
            if !is_internal && !attaches_start && !attaches_end {
                continue;
            }
            let offset = self.calc_offset(&source, &target);
            for section in &edge.sections {
                let picked: Vec<Option<JsonPoint>> = if is_internal {
                    let mut all = vec![section.start_point];
                    all.extend(section.bend_points.iter().copied().map(Some));
                    all.push(section.end_point);
                    all
                } else {
                    vec![
                        if attaches_start {
                            section.start_point
                        } else {
                            None
                        },
                        if attaches_end {
                            section.end_point
                        } else {
                            None
                        },
                    ]
                };
                points.extend(
                    picked
                        .into_iter()
                        .flatten()
                        .map(|p| Point::new(p.x + offset.x, p.y + offset.y)),
                );
            }
        }
        points
    }

    /// `applyElkEdgeLayout`: the alignment pre-pass, then every edge in ELK's order, then the
    /// straightening, and the separation of opposite labels.
    fn apply_edge_layout(&mut self, data: &mut LayoutData, output: &JsonNode, straighten: bool) {
        let edge_index: HashMap<String, usize> = data
            .edges
            .iter()
            .enumerate()
            .map(|(position, edge)| (edge.id.clone(), position))
            .collect();

        // Alignment pre-pass: small nodes move onto their routed lines before any edge is built.
        let mut aligned = HashSet::new();
        for edge in &output.edges {
            if edge.sections.is_empty() {
                continue;
            }
            let (start, end) = elk_ends(edge);
            if !self.nodes.contains_key(&start) || !self.nodes.contains_key(&end) {
                continue;
            }
            let offset = self.calc_offset(&start, &end);
            let section = &edge.sections[0];
            let from = section.start_point.unwrap_or_default();
            let to = section.end_point.unwrap_or_default();
            self.align_degenerate_node(
                data,
                &start,
                Point::new(from.x + offset.x, from.y + offset.y),
                &mut aligned,
            );
            self.align_degenerate_node(
                data,
                &end,
                Point::new(to.x + offset.x, to.y + offset.y),
                &mut aligned,
            );
        }

        for edge in &output.edges {
            let Some(&position) = edge_index.get(&edge.id) else {
                continue;
            };

            // A reversed edge was laid out from its target to its source: its ends are swapped back
            // here, and its route is reversed once built.
            let reversed = self.state.reversed_edges.contains(&edge.id);
            let (mut start, mut end) = elk_ends(edge);
            if reversed {
                std::mem::swap(&mut start, &mut end);
            }
            if !self.nodes.contains_key(&start) || !self.nodes.contains_key(&end) {
                continue;
            }

            if edge.sections.is_empty() {
                // ELK routed nothing: a straight line between the centres, clipped to the outlines.
                let from = self.refresh_centre(&start);
                let to = self.refresh_centre(&end);
                let straight = self.sanitize(vec![from, to], &start, &end);
                let layout = &mut data.edges[position];
                layout.points = straight;
                layout.curve = Some("linear".to_string());
                if let (Some(first), Some(last)) = (layout.points.first(), layout.points.last()) {
                    let midpoint = Point::new((first.x + last.x) / 2.0, (first.y + last.y) / 2.0);
                    layout.x = Some(midpoint.x);
                    layout.y = Some(midpoint.y);
                }
                continue;
            }

            let offset = self.calc_offset(&start, &end);
            let mut points = create_points_from_section(&edge.sections[0], offset);
            if reversed {
                points.reverse();
            }
            // The ports are the ELK route's own ends, before the node centres join it (`elkPorts`).
            let ports = match (points.first(), points.last()) {
                (Some(&first), Some(&last)) => Some((first, last)),
                _ => None,
            };
            let start_centre = self.refresh_centre(&start);
            let end_centre = self.refresh_centre(&end);
            points.insert(0, start_centre);
            points.push(end_centre);

            let clipped = self.sanitize(points, &start, &end);
            let start_bounds = self.bounds_for(&start);
            let end_bounds = self.bounds_for(&end);
            let layout = &data.edges[position];
            let end_offset = marker_path_offset(&layout.arrow_type_end);
            let start_offset = marker_path_offset(&layout.arrow_type_start);
            let points = ensure_start_marker_segment_length(
                ensure_end_marker_segment_length(clipped, end_bounds, end_offset),
                start_bounds,
                start_offset,
            );

            let layout = &mut data.edges[position];
            layout.points = points;
            layout.curve = Some("rounded".to_string());
            if let Some(label) = edge.labels.first() {
                layout.x =
                    Some(label.x.unwrap_or(0.0) + offset.x + label.width.unwrap_or(0.0) / 2.0);
                layout.y =
                    Some(label.y.unwrap_or(0.0) + offset.y + label.height.unwrap_or(0.0) / 2.0);
            }
            // The terminal labels follow the centre label in ELK's list, in the order they were
            // given. A label at the origin is left unplaced (`render.ts` 2085-2098).
            let mut placed = false;
            for (index, terminal) in layout.terminals.iter_mut().enumerate() {
                let Some(label) = edge.labels.get(index + 1) else {
                    continue;
                };
                let (x, y) = (label.x.unwrap_or(0.0), label.y.unwrap_or(0.0));
                if x == 0.0 && y == 0.0 {
                    continue;
                }
                terminal.centre = Some(Point::new(
                    x + offset.x + label.width.unwrap_or(0.0) / 2.0,
                    y + offset.y + label.height.unwrap_or(0.0) / 2.0,
                ));
                placed = true;
            }
            if placed {
                layout.ports = ports;
            }
        }

        if straighten {
            for (position, runs) in straighten_edge_terminals(&mut data.edges) {
                let edge = &mut data.edges[position];
                let (Some(x), Some(y)) = (edge.x, edge.y) else {
                    continue;
                };
                if let Some(projected) =
                    project_label_onto_straightened_run(Point::new(x, y), &runs)
                {
                    edge.x = Some(projected.x);
                    edge.y = Some(projected.y);
                }
            }
        }
        // `render.ts` 2111-2116: the terminal labels move with their endpoints, slide off the frames
        // and go to their side, before the labels of opposite edges are separated.
        for edge in &mut data.edges {
            terminal::follow_moved_endpoints(edge);
        }
        terminal::slide_off_frames(&mut data.edges, &data.nodes);
        terminal::put_on_their_side(&mut data.edges, &data.nodes);
        separate_opposite_edge_labels(&mut data.edges);
    }

    /// Recomputes a node's centre from its absolute corner and ELK size, and returns it.
    fn refresh_centre(&mut self, id: &str) -> Point {
        match self.nodes.get_mut(id) {
            Some(node) => {
                node.centre = Point::new(
                    node.pos.x + node.width / 2.0,
                    node.pos.y + node.height / 2.0,
                );
                node.centre
            }
            None => Point::default(),
        }
    }

    /// `alignDegenerateNodeToAnchor`: a node whose side is too short for ELK's port spread is moved
    /// along that side onto the anchor's line, so the edge can stay as ELK routed it. The first
    /// anchor on a node wins.
    fn align_degenerate_node(
        &mut self,
        data: &mut LayoutData,
        id: &str,
        anchor: Point,
        aligned: &mut HashSet<String>,
    ) {
        let Some(node) = self.nodes.get_mut(id) else {
            return;
        };
        let width = node.width;
        let height = node.height;
        let top = node.pos.y;
        let bottom = top + height;
        // ELK puts the anchor on the border; the slack absorbs floating-point error only.
        let tol = 0.5;
        let along_width = (anchor.y - top).abs() <= tol || (anchor.y - bottom).abs() <= tol;
        let side = if along_width { width } else { height };
        if side >= 2.0 * PORTS_SURROUNDING_MARGIN {
            return;
        }
        if aligned.contains(id) {
            return;
        }
        aligned.insert(id.to_string());

        let delta = if along_width {
            anchor.x - (node.pos.x + width / 2.0)
        } else {
            anchor.y - (node.pos.y + height / 2.0)
        };
        if delta.abs() < 0.01 {
            return;
        }
        if along_width {
            node.pos.x += delta;
            node.centre.x = node.pos.x + width / 2.0;
        } else {
            node.pos.y += delta;
            node.centre.y = node.pos.y + height / 2.0;
        }
        let layout_centre = Point::new(node.pos.x + width / 2.0, node.pos.y + height / 2.0);

        if let Some(&position) = self.index.get(id) {
            let layout = &mut data.nodes[position];
            layout.x = Some(layout_centre.x);
            layout.y = Some(layout_centre.y);
        }
    }

    /// The box of a node for the clipping: the centre and the size, with the title's width for a
    /// subgraph (`boundsFor`).
    fn bounds_for(&self, id: &str) -> CenterBox {
        let node = self.nodes.get(id).cloned().unwrap_or_default();
        let width = if node.is_group {
            node.width.max(node.title_width)
        } else {
            node.width
        };
        CenterBox::new(
            node.pos.x + node.width / 2.0,
            node.pos.y + node.height / 2.0,
            width,
            node.height,
        )
    }

    /// The outline of a leaf, or `None` for a subgraph.
    fn outline(&self, id: &str) -> Option<LeafOutline<'a>> {
        let node = self.nodes.get(id)?;
        if node.is_group {
            return None;
        }
        Some(LeafOutline {
            source: self.crossing,
            id: id.to_string(),
            node: CenterBox::new(node.centre.x, node.centre.y, node.width, node.height),
        })
    }

    /// The offset of a common ancestor's container, against which an edge's section is given
    /// (`calcOffset`). The root gives nothing.
    fn calc_offset(&self, source: &str, target: &str) -> Point {
        let Some(ancestor) = find_common_ancestor(source, target, &self.state.parent_by_id) else {
            return Point::default();
        };
        self.nodes
            .get(&ancestor)
            .and_then(|node| node.elk_origin.or(node.positioned.then_some(node.pos)))
            .unwrap_or_default()
    }

    /// The candidate border point of one end and whether the first (or last) point is the centre
    /// that ELK's routing leaves in (`getCandidateBorderPoint`).
    fn candidate_border_point(&self, points: &[Point], id: &str, side: Side) -> (Point, bool) {
        let centre = self
            .nodes
            .get(id)
            .map_or_else(Point::default, |node| node.centre);
        if points.is_empty() {
            return (centre, true);
        }
        match side {
            Side::Start => {
                let first = points[0];
                let centre_approx = is_centre_approx(first, centre);
                let candidate = if centre_approx && points.len() > 1 {
                    points[1]
                } else {
                    first
                };
                (candidate, centre_approx)
            }
            Side::End => {
                let last = points[points.len() - 1];
                let centre_approx = is_centre_approx(last, centre);
                let candidate = if centre_approx && points.len() > 1 {
                    points[points.len() - 2]
                } else {
                    last
                };
                (candidate, centre_approx)
            }
        }
    }

    /// `sanitizeElkEdgePoints`: clips the points at the two ends, against the frame of a subgraph
    /// or the outline of a leaf, and removes the consecutive duplicates.
    fn sanitize(&self, points: Vec<Point>, start: &str, end: &str) -> Vec<Point> {
        let mut prev = points;
        let start_bounds = self.bounds_for(start);
        let end_bounds = self.bounds_for(end);
        let start_is_group = self.is_group(start);
        let end_is_group = self.is_group(end);

        let (start_candidate, start_centre_approx) =
            self.candidate_border_point(&prev, start, Side::Start);
        let (end_candidate, end_centre_approx) = self.candidate_border_point(&prev, end, Side::End);

        let mut skip_start = start_is_group && on_border(start_bounds, start_candidate, 0.5);
        let mut skip_end = end_is_group && on_border(end_bounds, end_candidate, 0.5);

        drop_auto_centre_point(&mut prev, Side::Start, skip_start && start_centre_approx);
        drop_auto_centre_point(&mut prev, Side::End, skip_end && end_centre_approx);

        // A frame that moved takes its interior terminals out and is intersected at the crossing.
        if start_is_group && !skip_start {
            skip_start = clip_group_endpoint(&mut prev, start_bounds, Side::Start);
        }
        if end_is_group && !skip_end {
            skip_end = clip_group_endpoint(&mut prev, end_bounds, Side::End);
        }

        let mut clipped = if skip_start || skip_end {
            if !skip_start {
                self.apply_start_intersection_if_needed(&mut prev, start, start_bounds);
            }
            if !skip_end {
                self.apply_end_intersection_if_needed(&mut prev, end, end_bounds);
            }
            prev.clone()
        } else {
            self.cutter2(start, end, &prev)
        };

        if clipped.len() < 2 || clipped.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            let cleaned: Vec<Point> = prev
                .iter()
                .copied()
                .filter(|p| p.x.is_finite() && p.y.is_finite())
                .collect();
            clipped = if cleaned.len() >= 2 { cleaned } else { prev };
        }
        dedupe_consecutive_points(&clipped)
    }

    /// Intersects the start with the outline at the first point outside the box
    /// (`applyStartIntersectionIfNeeded`).
    fn apply_start_intersection_if_needed(
        &self,
        points: &mut Vec<Point>,
        id: &str,
        bounds: CenterBox,
    ) {
        let Some(index) = points.iter().position(|p| outside_node(bounds, *p)) else {
            return;
        };
        let outline = self.outline(id);
        let outline_ref = outline.as_ref().map(|o| o as &dyn NodeOutline);
        let centre = points[0];
        let intersection = compute_node_intersection(outline_ref, bounds, points[index], centre);
        replace_endpoint(points, Side::Start, intersection, 0.1);
    }

    /// Intersects the end with the outline at the last point outside the box
    /// (`applyEndIntersectionIfNeeded`).
    fn apply_end_intersection_if_needed(
        &self,
        points: &mut Vec<Point>,
        id: &str,
        bounds: CenterBox,
    ) {
        let Some(index) = points.iter().rposition(|p| outside_node(bounds, *p)) else {
            return;
        };
        let outline = self.outline(id);
        let outline_ref = outline.as_ref().map(|o| o as &dyn NodeOutline);
        let centre = points[points.len() - 1];
        let intersection = compute_node_intersection(outline_ref, bounds, points[index], centre);
        replace_endpoint(points, Side::End, intersection, 0.1);
    }

    /// The attachment on the departure axis of the node at `port_index`, one step along the route
    /// (`attachAlongDepartureAxis`). A subgraph gives none.
    fn attach_along_departure_axis(
        &self,
        id: &str,
        bounds: CenterBox,
        points: &[Point],
        port_index: usize,
        step: isize,
    ) -> Option<Point> {
        if self.is_group(id) {
            return None;
        }
        let port = *points.get(port_index)?;
        let next_index = usize::try_from(port_index as isize + step).ok()?;
        let next = *points.get(next_index)?;
        let outline = self.outline(id);
        outline_attach_point(
            outline.as_ref().map(|o| o as &dyn NodeOutline),
            bounds,
            port,
            next,
        )
    }

    /// `cutter2`: moves each end of the route onto its node's outline, preferring the departure
    /// axis, and trims a last point that lies within 2px of the one before.
    fn cutter2(&self, start: &str, end: &str, original: &[Point]) -> Vec<Point> {
        if original.is_empty() {
            return Vec::new();
        }
        let start_bounds = self.bounds_for(start);
        let end_bounds = self.bounds_for(end);
        let mut points = original.to_vec();
        let start_centre = points[0];
        let end_centre = points[points.len() - 1];

        if let Some(index) = points.iter().position(|p| outside_node(start_bounds, *p)) {
            let outline = self.outline(start);
            let start_intersection = self
                .attach_along_departure_axis(start, start_bounds, &points, index, 1)
                .unwrap_or_else(|| {
                    compute_node_intersection(
                        outline.as_ref().map(|o| o as &dyn NodeOutline),
                        start_bounds,
                        points[index],
                        start_centre,
                    )
                });
            replace_endpoint(&mut points, Side::Start, start_intersection, 0.1);
        }

        let mut outside_for_end = points.iter().rposition(|p| outside_node(end_bounds, *p));
        if outside_for_end.is_none() && points.len() > 1 {
            outside_for_end = Some(points.len() - 2);
        }
        if let Some(index) = outside_for_end {
            let outline = self.outline(end);
            let end_intersection = self
                .attach_along_departure_axis(end, end_bounds, &points, index, -1)
                .unwrap_or_else(|| {
                    compute_node_intersection(
                        outline.as_ref().map(|o| o as &dyn NodeOutline),
                        end_bounds,
                        points[index],
                        end_centre,
                    )
                });
            replace_endpoint(&mut points, Side::End, end_intersection, 0.1);
        }

        if points.len() > 1 {
            let last = points[points.len() - 1];
            let second_last = points[points.len() - 2];
            let distance =
                (pow(last.x - second_last.x, 2.0) + pow(last.y - second_last.y, 2.0)).sqrt();
            if distance < 2.0 {
                points.pop();
            }
        }
        points
    }
}

/// Whether a value is a number that JavaScript would call truthy.
fn truthy(value: f64) -> bool {
    value != 0.0 && !value.is_nan()
}

/// Whether a cluster shape paints its label as a title strip (`clusterPaintsTitle`). A missing
/// shape is `rect`.
fn cluster_paints_title(shape: &str) -> bool {
    let shape = if shape.is_empty() { "rect" } else { shape };
    !matches!(shape, "noteGroup" | "divider")
}

/// The two ends of an ELK edge, in ELK's own orientation: the sources and targets it was given.
fn elk_ends(edge: &JsonEdge) -> (String, String) {
    let start = edge
        .sources
        .first()
        .cloned()
        .or_else(|| edge.source.clone())
        .unwrap_or_default();
    let end = edge
        .targets
        .first()
        .cloned()
        .or_else(|| edge.target.clone())
        .unwrap_or_default();
    (start, end)
}

/// The ids of every node inside `elk_node`, at any depth (`collectDescendantIds`).
fn collect_descendant_ids(elk_node: &JsonNode) -> HashSet<String> {
    fn collect(node: &JsonNode, into: &mut HashSet<String>) {
        for child in &node.children {
            into.insert(child.id.clone());
            collect(child, into);
        }
    }
    let mut ids = HashSet::new();
    collect(elk_node, &mut ids);
    ids
}

/// The points of a section, shifted by the container's offset: start, bends, end
/// (`createEdgePointsFromSection`).
fn create_points_from_section(section: &JsonEdgeSection, offset: Point) -> Vec<Point> {
    let source = section.start_point.unwrap_or_default();
    let target = section.end_point.unwrap_or_default();
    let mut points = Vec::with_capacity(section.bend_points.len() + 2);
    points.push(Point::new(source.x + offset.x, source.y + offset.y));
    points.extend(
        section
            .bend_points
            .iter()
            .map(|bend| Point::new(bend.x + offset.x, bend.y + offset.y)),
    );
    points.push(Point::new(target.x + offset.x, target.y + offset.y));
    points
}

/// Whether a point is at the centre of a node, within `1e-6` (`isCenterApprox`).
fn is_centre_approx(point: Point, centre: Point) -> bool {
    (point.x - centre.x).abs() < 1e-6 && (point.y - centre.y).abs() < 1e-6
}

/// Removes the auto-generated centre point at one end, when asked (`dropAutoCenterPoint`).
fn drop_auto_centre_point(points: &mut Vec<Point>, side: Side, do_drop: bool) {
    if !do_drop || points.is_empty() {
        return;
    }
    match side {
        Side::Start => {
            points.remove(0);
        }
        Side::End => {
            points.pop();
        }
    }
}

/// Clips the route at a subgraph's frame: the interior points are replaced by the crossing with
/// the frame (`clipGroupEndpoint`). Returns `false` when the route does not cross the frame at that
/// end.
fn clip_group_endpoint(points: &mut Vec<Point>, bounds: CenterBox, side: Side) -> bool {
    let step: isize = match side {
        Side::Start => 1,
        Side::End => -1,
    };
    let len = points.len() as isize;
    let terminal = match side {
        Side::Start => 0,
        Side::End => len - 1,
    };
    let mut index = terminal;
    while index >= 0 && index < len && !outside_node(bounds, points[index as usize]) {
        index += step;
    }
    if index == terminal || index < 0 || index >= len {
        return false;
    }

    let outside = points[index as usize];
    let inside = points[(index - step) as usize];
    let dx = outside.x - inside.x;
    let dy = outside.y - inside.y;
    // The first side reached from the last interior point is the crossing, corners included.
    let tx = if dx == 0.0 {
        f64::INFINITY
    } else {
        (bounds.x + (js_sign(dx) * bounds.width) / 2.0 - inside.x) / dx
    };
    let ty = if dy == 0.0 {
        f64::INFINITY
    } else {
        (bounds.y + (js_sign(dy) * bounds.height) / 2.0 - inside.y) / dy
    };
    let t = tx.min(ty);
    let crossing = Point::new(inside.x + t * dx, inside.y + t * dy);
    match side {
        Side::Start => {
            points.splice(0..index as usize, std::iter::once(crossing));
        }
        Side::End => {
            let from = (index + 1) as usize;
            points.splice(from.., std::iter::once(crossing));
        }
    }
    true
}

/// Removes each point that repeats the one before it, within `1e-6` (`dedupeConsecutivePoints`).
fn dedupe_consecutive_points(points: &[Point]) -> Vec<Point> {
    points
        .iter()
        .enumerate()
        .filter(|(index, point)| {
            *index == 0 || {
                let prev = points[index - 1];
                (point.x - prev.x).abs() > 1e-6 || (point.y - prev.y).abs() > 1e-6
            }
        })
        .map(|(_, point)| *point)
        .collect()
}

/// `markerPathOffset`: how far a path is pulled back along its last segment for this marker. The
/// drawing's table and the neo dash table are both read, as the reference does.
#[must_use]
pub fn marker_path_offset(arrow_type: &str) -> f64 {
    let drawn: f64 = match arrow_type {
        "aggregation" | "extension" | "composition" => 17.25,
        "dependency" => 6.0,
        "lollipop" => 13.5,
        "arrow_point" => 4.0,
        "arrow_barb_neo" => 5.5,
        _ => 0.0,
    };
    let dashed: f64 = match arrow_type {
        "arrow_point" => 4.0,
        "arrow_cross" | "arrow_circle" => 12.5,
        _ => 0.0,
    };
    drawn.max(dashed)
}

/// Shortens a route whose last segment is too short for its marker, by dropping the point that the
/// route enters the node's border at (`ensureEndMarkerSegmentLength`).
#[must_use]
pub fn ensure_end_marker_segment_length(
    points: Vec<Point>,
    end_bounds: CenterBox,
    marker_offset: f64,
) -> Vec<Point> {
    if marker_offset <= 0.0 || points.len() < 3 {
        return points;
    }
    let len = points.len();
    let end = points[len - 1];
    let entry = points[len - 2];
    let segment_length = js_hypot(end.x - entry.x, end.y - entry.y);
    if segment_length >= MIN_END_MARKER_SEGMENT_LENGTH.max(2.0 * marker_offset) {
        return points;
    }
    if !on_border(end_bounds, entry, 1.0) {
        return points;
    }
    let mut adjusted = points[..len - 2].to_vec();
    adjusted.push(end);
    adjusted
}

/// The mirror of [`ensure_end_marker_segment_length`] at the start of the route
/// (`ensureStartMarkerSegmentLength`).
#[must_use]
pub fn ensure_start_marker_segment_length(
    points: Vec<Point>,
    start_bounds: CenterBox,
    marker_offset: f64,
) -> Vec<Point> {
    if marker_offset <= 0.0 || points.len() < 3 {
        return points;
    }
    let start = points[0];
    let exit = points[1];
    let segment_length = js_hypot(exit.x - start.x, exit.y - start.y);
    if segment_length >= MIN_END_MARKER_SEGMENT_LENGTH.max(2.0 * marker_offset) {
        return points;
    }
    if !on_border(start_bounds, exit, 1.0) {
        return points;
    }
    let mut adjusted = vec![start];
    adjusted.extend_from_slice(&points[2..]);
    adjusted
}

/// Pushes apart the labels of two edges between the same pair of nodes when they would overlap
/// (`separateOppositeEdgeLabels`). Only pairs of exactly two labelled edges are considered, and
/// only when their labels are at about the same height. The direction is not checked (R:2141).
pub fn separate_opposite_edge_labels(edges: &mut [Edge]) {
    let mut buckets: Vec<Vec<usize>> = Vec::new();
    let mut bucket_of: HashMap<String, usize> = HashMap::new();
    for (index, edge) in edges.iter().enumerate() {
        if edge.x.is_none()
            || edge.y.is_none()
            || edge.label.is_empty()
            || !truthy_option(edge.width)
        {
            continue;
        }
        let mut pair = [edge.start.as_str(), edge.end.as_str()];
        pair.sort_unstable();
        let key = format!("{}\u{0}{}", pair[0], pair[1]);
        match bucket_of.get(&key) {
            Some(&bucket) => buckets[bucket].push(index),
            None => {
                bucket_of.insert(key, buckets.len());
                buckets.push(vec![index]);
            }
        }
    }

    for bucket in buckets {
        let (a, b) = match bucket.as_slice() {
            [a, b] => (*a, *b),
            _ => continue,
        };
        let (a_x, b_x) = (edges[a].x.unwrap_or(0.0), edges[b].x.unwrap_or(0.0));
        let (a_y, b_y) = (edges[a].y.unwrap_or(0.0), edges[b].y.unwrap_or(0.0));
        let (a_w, b_w) = (edges[a].width.unwrap_or(0.0), edges[b].width.unwrap_or(0.0));
        let max_height = edges[a]
            .height
            .unwrap_or(0.0)
            .max(edges[b].height.unwrap_or(0.0));
        // Labels drawn at meaningfully different heights do not compete for the same space.
        if (a_y - b_y).abs() > max_height {
            continue;
        }
        let required = a_w / 2.0 + b_w / 2.0 + OPPOSITE_LABEL_MIN_GAP;
        let current = b_x - a_x;
        let deficit = required - current.abs();
        if deficit <= 0.0 {
            continue;
        }
        let sign = match js_sign(current) {
            value if value == 0.0 || value.is_nan() => 1.0,
            value => value,
        };
        edges[a].x = Some(a_x - (sign * deficit) / 2.0);
        edges[b].x = Some(b_x + (sign * deficit) / 2.0);
    }
}

/// `Option` form of [`truthy`].
fn truthy_option(value: Option<f64>) -> bool {
    value.is_some_and(truthy)
}

/// The paint order: subgraphs first, the shallowest first, then the other nodes in their order
/// (`orderNodesForElkPaint`). The sort is stable, as the reference's is.
fn order_nodes_for_elk_paint(nodes: &mut [Node]) {
    let by_id: HashMap<&str, &Node> = nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let keys: HashMap<String, (u8, usize)> = nodes
        .iter()
        .map(|node| {
            let key = if node.is_group {
                (0, group_depth(node, &by_id))
            } else {
                (1, 0)
            };
            (node.id.clone(), key)
        })
        .collect();
    nodes.sort_by_key(|node| keys.get(&node.id).copied().unwrap_or((1, 0)));
}

/// How many subgraphs contain the node (`getGroupDepth`).
fn group_depth(node: &Node, by_id: &HashMap<&str, &Node>) -> usize {
    let mut depth = 0;
    let mut visited = HashSet::new();
    let mut parent = node.parent_id.as_deref();
    while let Some(id) = parent {
        if !visited.insert(id) {
            break;
        }
        let Some(group) = by_id.get(id) else {
            break;
        };
        if !group.is_group {
            break;
        }
        depth += 1;
        parent = group.parent_id.as_deref();
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn marker_offsets_read_both_tables() {
        // render.spec.ts: the guard uses the offsets of the drawing and of the dash table (D1).
        assert_eq!(marker_path_offset("arrow_point"), 4.0);
        assert_eq!(marker_path_offset("arrow_cross"), 12.5);
        assert_eq!(marker_path_offset("arrow_circle"), 12.5);
        assert_eq!(marker_path_offset("extension"), 17.25);
        assert_eq!(marker_path_offset("arrow_barb"), 0.0);
        assert_eq!(marker_path_offset("none"), 0.0);
    }

    #[test]
    fn cluster_titles_are_painted_except_for_note_groups_and_dividers() {
        // clusters.js:823-830 (`UNTITLED_CLUSTER_SHAPES`)
        assert!(cluster_paints_title("rect"));
        assert!(cluster_paints_title(""));
        assert!(!cluster_paints_title("noteGroup"));
        assert!(!cluster_paints_title("divider"));
    }

    // render.spec.ts: "ensureEndMarkerSegmentLength"

    fn circle_bounds() -> CenterBox {
        CenterBox::new(138.88020833333334, 607.4296875, 140.265625, 140.265625)
    }

    #[test]
    fn removes_the_target_bbox_entry_point_when_the_final_marker_segment_is_too_short() {
        // render.spec.ts: "removes the target bbox entry point when the final marker segment is too short"
        let points = vec![
            p(162.2578125, 497.296875),
            p(162.2578125, 537.296875),
            p(161.05815095468608, 540.8958596359416),
        ];
        assert_eq!(
            ensure_end_marker_segment_length(points.clone(), circle_bounds(), 4.0),
            vec![points[0], points[2]]
        );
    }

    #[test]
    fn keeps_real_bends_that_are_not_on_the_target_bounds() {
        // render.spec.ts: "keeps real bends that are not on the target bounds"
        let points = vec![p(120.0, 500.0), p(130.0, 532.0), p(132.0, 535.0)];
        assert_eq!(
            ensure_end_marker_segment_length(points.clone(), circle_bounds(), 4.0),
            points
        );
    }

    #[test]
    fn keeps_target_entry_segments_that_already_have_marker_runway() {
        // render.spec.ts: "keeps target entry segments that already have marker runway"
        let points = vec![
            p(162.2578125, 497.296875),
            p(162.2578125, 537.296875),
            p(162.2578125, 550.0),
        ];
        assert_eq!(
            ensure_end_marker_segment_length(points.clone(), circle_bounds(), 4.0),
            points
        );
    }

    #[test]
    fn removes_the_stub_an_off_centre_port_leaves_outside_an_ellipse_for_a_wide_marker() {
        // render.spec.ts: "removes the stub an off-centre port leaves outside an ellipse for a wide marker"
        let ellipse_bounds = CenterBox::new(408.0, 190.0, 160.0, 58.0);
        let points = vec![p(308.0, 199.0), p(328.0, 199.0), p(337.54, 199.0)];
        assert_eq!(
            ensure_end_marker_segment_length(points.clone(), ellipse_bounds, 17.25),
            vec![points[0], points[2]]
        );
    }

    // render.spec.ts: "ensureStartMarkerSegmentLength"

    #[test]
    fn removes_the_source_bbox_exit_point_when_the_first_marker_segment_is_too_short() {
        // render.spec.ts: "removes the source bbox exit point when the first marker segment is too short"
        let ellipse_bounds = CenterBox::new(408.0, 190.0, 160.0, 58.0);
        let points = vec![
            p(478.46, 199.0),
            p(488.0, 199.0),
            p(508.0, 199.0),
            p(508.0, 260.0),
        ];
        assert_eq!(
            ensure_start_marker_segment_length(points.clone(), ellipse_bounds, 17.25),
            vec![points[0], points[2], points[3]]
        );
    }

    #[test]
    fn keeps_real_bends_that_are_not_on_the_source_bounds_at_the_start() {
        // render.spec.ts: "keeps real bends that are not on the source bounds"
        let ellipse_bounds = CenterBox::new(408.0, 190.0, 160.0, 58.0);
        let points = vec![p(478.46, 199.0), p(484.0, 203.0), p(508.0, 260.0)];
        assert_eq!(
            ensure_start_marker_segment_length(points.clone(), ellipse_bounds, 17.25),
            points
        );
    }

    #[test]
    fn keeps_source_exit_segments_that_already_have_marker_runway() {
        // render.spec.ts: "keeps source exit segments that already have marker runway"
        let ellipse_bounds = CenterBox::new(408.0, 190.0, 160.0, 58.0);
        let points = vec![p(448.0, 190.0), p(488.0, 190.0), p(508.0, 260.0)];
        assert_eq!(
            ensure_start_marker_segment_length(points.clone(), ellipse_bounds, 4.0),
            points
        );
    }

    // render.spec.ts: "separateOppositeEdgeLabels"

    /// A labelled edge from `start` to `end`, its label centred at `centre` and sized `size`.
    fn labelled(
        id: &str,
        ends: (&str, &str),
        label: &str,
        centre: (f64, f64),
        size: (f64, f64),
    ) -> Edge {
        Edge {
            id: id.to_string(),
            start: ends.0.to_string(),
            end: ends.1.to_string(),
            label: label.to_string(),
            x: Some(centre.0),
            y: Some(centre.1),
            width: Some(size.0),
            height: Some(size.1),
            ..Edge::default()
        }
    }

    #[test]
    fn pulls_a_wide_overlapping_label_pair_apart_symmetrically() {
        // render.spec.ts: "pulls a wide, overlapping label pair apart symmetrically"
        let mut edges = vec![
            labelled(
                "e9",
                ("ScreenDimmed", "ScreenOn"),
                "touch",
                (492.933, 489.5),
                (34.25, 21.0),
            ),
            labelled(
                "e8",
                ("ScreenOn", "ScreenDimmed"),
                "idle 30s",
                (531.067, 489.5),
                (48.266, 21.0),
            ),
        ];
        separate_opposite_edge_labels(&mut edges);

        let (touch, idle) = (edges[0].x.unwrap(), edges[1].x.unwrap());
        let required = 34.25 / 2.0 + 48.266 / 2.0 + 4.0;
        assert!((idle - touch - required).abs() < 1e-5);
        assert!(((touch + idle) / 2.0 - (492.933 + 531.067) / 2.0).abs() < 0.05);
    }

    #[test]
    fn leaves_a_short_already_clear_label_pair_untouched() {
        // render.spec.ts: "leaves a short, already-clear label pair untouched"
        let mut edges = vec![
            labelled(
                "e6",
                ("Paused", "Playing"),
                "play",
                (284.933, 489.5),
                (25.6875, 21.0),
            ),
            labelled(
                "e5",
                ("Playing", "Paused"),
                "pause",
                (323.067, 489.5),
                (38.15625, 21.0),
            ),
        ];
        separate_opposite_edge_labels(&mut edges);
        assert_eq!(edges[0].x, Some(284.933));
        assert_eq!(edges[1].x, Some(323.067));
    }

    #[test]
    fn ignores_edges_between_the_same_pair_at_meaningfully_different_heights() {
        // render.spec.ts: "ignores edges between the same pair at meaningfully different heights"
        let mut edges = vec![
            labelled(
                "e1",
                ("A", "B"),
                "wide label one",
                (100.0, 0.0),
                (80.0, 21.0),
            ),
            labelled(
                "e2",
                ("B", "A"),
                "wide label two",
                (110.0, 200.0),
                (80.0, 21.0),
            ),
        ];
        separate_opposite_edge_labels(&mut edges);
        assert_eq!(edges[0].x, Some(100.0));
        assert_eq!(edges[1].x, Some(110.0));
    }

    #[test]
    fn ignores_a_node_pair_with_more_than_two_labelled_edges() {
        // render.spec.ts: "ignores a node pair with more than two labelled edges"
        let mut edges = vec![
            labelled("e1", ("A", "B"), "one", (100.0, 0.0), (80.0, 21.0)),
            labelled("e2", ("B", "A"), "two", (110.0, 0.0), (80.0, 21.0)),
            labelled("e3", ("A", "B"), "three", (105.0, 0.0), (80.0, 21.0)),
        ];
        separate_opposite_edge_labels(&mut edges);
        assert_eq!(
            [edges[0].x, edges[1].x, edges[2].x],
            [Some(100.0), Some(110.0), Some(105.0)]
        );
    }

    #[test]
    fn dedupe_drops_points_that_repeat_the_one_before() {
        // The reference's dedupe keeps the first point and compares each point with its predecessor.
        let points = [p(0.0, 0.0), p(0.0, 0.0), p(1.0, 0.0), p(1.0, 0.0000001)];
        assert_eq!(
            dedupe_consecutive_points(&points),
            vec![p(0.0, 0.0), p(1.0, 0.0)]
        );
    }

    #[test]
    fn create_points_from_section_shifts_every_point_by_the_offset() {
        // render.ts:2431-2445
        let section = JsonEdgeSection {
            start_point: Some(JsonPoint { x: 1.0, y: 2.0 }),
            end_point: Some(JsonPoint { x: 9.0, y: 8.0 }),
            bend_points: vec![JsonPoint { x: 5.0, y: 2.0 }],
            ..JsonEdgeSection::default()
        };
        assert_eq!(
            create_points_from_section(&section, p(10.0, 20.0)),
            vec![p(11.0, 22.0), p(15.0, 22.0), p(19.0, 28.0)]
        );
    }

    #[test]
    fn js_helpers_treat_zero_and_nan_as_the_reference_does() {
        // separateOppositeEdgeLabels: `Math.sign(current) || 1`, and `!edge.width`.
        assert!(!truthy_option(Some(0.0)));
        assert!(!truthy_option(None));
        assert!(truthy_option(Some(-3.0)));
    }

    use crate::unified::elk_build::LabelData;
    use ferrigura_elk::json::JsonLabel;

    /// `intersect-rect.js`, transcribed: the test double for a rectangular leaf.
    fn rect_crossing(node: CenterBox, point: Point) -> Point {
        let dx = point.x - node.x;
        let dy = point.y - node.y;
        let mut w = node.width / 2.0;
        let mut h = node.height / 2.0;
        let (sx, sy);
        if dy.abs() * w > dx.abs() * h {
            // The intersection is on the top or bottom of the box.
            if dy < 0.0 {
                h = -h;
            }
            sx = if dy == 0.0 { 0.0 } else { (h * dx) / dy };
            sy = h;
        } else {
            // The intersection is on the left or right of the box.
            if dx < 0.0 {
                w = -w;
            }
            sx = w;
            sy = if dx == 0.0 { 0.0 } else { (w * dy) / dx };
        }
        Point::new(node.x + sx, node.y + sy)
    }

    /// `diamond-intersections` in the reference's spec: a diamond whose outline runs through the
    /// midpoints of its box sides, with the half-diagonal `half`.
    fn diamond_crossing(node: CenterBox, probe: Point, half: f64) -> Point {
        let dx = probe.x - node.x;
        let dy = probe.y - node.y;
        if dx == 0.0 && dy == 0.0 {
            return Point::new(node.x, node.y);
        }
        let t = half / (dx.abs() + dy.abs());
        Point::new(node.x + dx * t, node.y + dy * t)
    }

    /// The test crossing: leaves named `D` are diamonds of half-diagonal 50, the others rectangles.
    fn scene_crossing(id: &str, node: CenterBox, probe: Point) -> Option<Point> {
        Some(if id == "D" {
            diamond_crossing(node, probe, 50.0)
        } else {
            rect_crossing(node, probe)
        })
    }

    fn node(id: &str, parent: Option<&str>, is_group: bool, width: f64, height: f64) -> Node {
        Node {
            id: id.to_string(),
            parent_id: parent.map(str::to_string),
            is_group,
            shape: "rect".to_string(),
            width: Some(width),
            height: Some(height),
            ..Node::default()
        }
    }

    fn layout_edge(id: &str, start: &str, end: &str) -> Edge {
        Edge {
            id: id.to_string(),
            start: start.to_string(),
            end: end.to_string(),
            arrow_type_start: "none".to_string(),
            arrow_type_end: "arrow_point".to_string(),
            ..Edge::default()
        }
    }

    /// An ELK node at `(x, y)` relative to its parent, with its children.
    fn elk(id: &str, x: f64, y: f64, width: f64, height: f64, children: Vec<JsonNode>) -> JsonNode {
        JsonNode {
            id: id.to_string(),
            x: Some(x),
            y: Some(y),
            width: Some(width),
            height: Some(height),
            children,
            ..JsonNode::default()
        }
    }

    fn point(x: f64, y: f64) -> JsonPoint {
        JsonPoint { x, y }
    }

    fn section(start: (f64, f64), bends: &[(f64, f64)], end: (f64, f64)) -> JsonEdgeSection {
        JsonEdgeSection {
            start_point: Some(point(start.0, start.1)),
            end_point: Some(point(end.0, end.1)),
            bend_points: bends.iter().map(|&(x, y)| point(x, y)).collect(),
            ..JsonEdgeSection::default()
        }
    }

    /// An ELK edge from `source` to `target`, with one section and a label at the origin.
    fn elk_edge(id: &str, source: &str, target: &str, sections: Vec<JsonEdgeSection>) -> JsonEdge {
        JsonEdge {
            id: id.to_string(),
            sources: vec![source.to_string()],
            targets: vec![target.to_string()],
            sections,
            labels: vec![JsonLabel {
                width: Some(0.0),
                height: Some(0.0),
                ..JsonLabel::default()
            }],
            ..JsonEdge::default()
        }
    }

    fn root(children: Vec<JsonNode>, edges: Vec<JsonEdge>) -> JsonNode {
        JsonNode {
            id: "root".to_string(),
            children,
            edges,
            ..JsonNode::default()
        }
    }

    /// Applies a scene with rectangular leaves, the test crossing, and the given parent links.
    fn run(
        data: &mut LayoutData,
        output: &JsonNode,
        parents: &[(&str, &str)],
        title_widths: &[(&str, f64)],
    ) {
        let state = ElkState {
            parent_by_id: parents
                .iter()
                .map(|(child, parent)| (child.to_string(), parent.to_string()))
                .collect(),
            label_data: title_widths
                .iter()
                .map(|(id, width)| {
                    (
                        id.to_string(),
                        LabelData {
                            width: *width,
                            height: 0.0,
                            wrapping_width: 0.0,
                        },
                    )
                })
                .collect(),
            ..ElkState::default()
        };
        apply_with_crossing(data, output, &state, &scene_crossing, false);
    }

    fn assert_points(actual: &[Point], expected: &[(f64, f64)]) {
        assert_eq!(actual.len(), expected.len(), "{actual:?}");
        for (point, (x, y)) in actual.iter().zip(expected) {
            assert!(
                (point.x - x).abs() < 1e-3 && (point.y - y).abs() < 1e-3,
                "{actual:?} != {expected:?}"
            );
        }
    }

    // Own tests: a two-node edge, an edge into a subgraph, a diamond departure, a self loop, a
    // reversed feedback edge. The leaves are rectangles unless named otherwise.

    #[test]
    fn a_two_node_edge_is_clipped_at_both_borders() {
        // Node A (0,0,100x50), node B (0,100,100x50). ELK's section runs from A's bottom to B's top.
        // render.ts 2046-2078: the centres are added to both ends, then `sanitize` clips them. For a
        // rectangle the port is already on the outline (`outlineAttachPoint` returns it), so both
        // clipped points are the ports.
        let output = root(
            vec![
                elk("A", 0.0, 0.0, 100.0, 50.0, vec![]),
                elk("B", 0.0, 100.0, 100.0, 50.0, vec![]),
            ],
            vec![elk_edge(
                "e",
                "A",
                "B",
                vec![section((50.0, 50.0), &[], (50.0, 100.0))],
            )],
        );
        let mut data = LayoutData {
            nodes: vec![
                node("A", None, false, 100.0, 50.0),
                node("B", None, false, 100.0, 50.0),
            ],
            edges: vec![layout_edge("e", "A", "B")],
            ..LayoutData::default()
        };
        run(&mut data, &output, &[], &[]);

        assert_points(&data.edges[0].points, &[(50.0, 50.0), (50.0, 100.0)]);
        assert_eq!(data.edges[0].curve.as_deref(), Some("rounded"));
        assert_eq!(data.nodes[1].x, Some(50.0));
        assert_eq!(data.nodes[1].y, Some(125.0));
    }

    #[test]
    fn a_reversed_edge_is_routed_from_its_own_source() {
        // render.ts 1992-1997 and 2048-2050: the ELK edge runs from B to A because it was reversed,
        // and the route is turned round, so it reads A to B as the edge is declared.
        let output = root(
            vec![
                elk("A", 0.0, 0.0, 100.0, 50.0, vec![]),
                elk("B", 0.0, 100.0, 100.0, 50.0, vec![]),
            ],
            vec![elk_edge(
                "e",
                "B",
                "A",
                vec![section((50.0, 100.0), &[], (50.0, 50.0))],
            )],
        );
        let mut data = LayoutData {
            nodes: vec![
                node("A", None, false, 100.0, 50.0),
                node("B", None, false, 100.0, 50.0),
            ],
            edges: vec![layout_edge("e", "A", "B")],
            ..LayoutData::default()
        };
        let state_reversed = ElkState {
            reversed_edges: HashSet::from(["e".to_string()]),
            ..ElkState::default()
        };
        apply_with_crossing(&mut data, &output, &state_reversed, &scene_crossing, false);

        assert_points(&data.edges[0].points, &[(50.0, 50.0), (50.0, 100.0)]);
    }

    #[test]
    fn an_edge_into_a_subgraph_stops_on_its_frame() {
        // Subgraph G (0,0,200x150), leaf M (0,200,100x50) below it. The section runs from M's top
        // to G's bottom. `sanitize` (render.ts 2504-2508): the candidate at G's bottom is on the
        // frame, so the auto-centre point is dropped and the frame is the endpoint; the leaf end is
        // clipped by the rectangle intersect.
        let output = root(
            vec![
                elk("G", 0.0, 0.0, 200.0, 150.0, vec![]),
                elk("M", 0.0, 200.0, 100.0, 50.0, vec![]),
            ],
            vec![elk_edge(
                "e",
                "M",
                "G",
                vec![section((50.0, 200.0), &[], (100.0, 150.0))],
            )],
        );
        let mut data = LayoutData {
            nodes: vec![
                node("G", None, true, 200.0, 150.0),
                node("M", None, false, 100.0, 50.0),
            ],
            edges: vec![layout_edge("e", "M", "G")],
            ..LayoutData::default()
        };
        run(&mut data, &output, &[], &[]);

        assert_points(&data.edges[0].points, &[(50.0, 200.0), (100.0, 150.0)]);
    }

    #[test]
    fn an_edge_departing_a_diamond_slides_along_its_departure_axis() {
        // Leaf A (0,0,100x50) above a diamond D centred at (50,125), half-diagonal 50, so its box is
        // (0,75)-(100,175). ELK's section leaves A at x = 40 and enters D's box at (40, 75). The
        // ELK route is (50,25) (A's centre), (40,50), (40,75), (50,125) (D's centre). The diamond's
        // outline at x = 40 is at y = 125 - (50 - 10) = 85, found by `outlineAttachPoint` along the
        // departure axis (render.ts 2920-2936, elk/geometry.ts 224-274), so the stub ends there.
        let output = root(
            vec![
                elk("A", 0.0, 0.0, 100.0, 50.0, vec![]),
                elk("D", 0.0, 75.0, 100.0, 100.0, vec![]),
            ],
            vec![elk_edge(
                "e",
                "A",
                "D",
                vec![section((40.0, 50.0), &[], (40.0, 75.0))],
            )],
        );
        let mut data = LayoutData {
            nodes: vec![
                node("A", None, false, 100.0, 50.0),
                node("D", None, false, 100.0, 100.0),
            ],
            edges: vec![layout_edge("e", "A", "D")],
            ..LayoutData::default()
        };
        run(&mut data, &output, &[], &[]);

        assert_points(
            &data.edges[0].points,
            &[(40.0, 50.0), (40.0, 75.0), (40.0, 85.0)],
        );
    }

    #[test]
    fn a_self_loop_keeps_its_bends_and_starts_at_the_border() {
        // Leaf A (0,0,100x50) with a loop out of its right side and back in at (100,40). The loop's
        // ends are the ports, its centre points are dropped by `cutter2`, and the loop is not
        // shortened by the marker guard (its last segment is 30 long).
        let output = root(
            vec![elk("A", 0.0, 0.0, 100.0, 50.0, vec![])],
            vec![elk_edge(
                "e",
                "A",
                "A",
                vec![section(
                    (100.0, 20.0),
                    &[(130.0, 20.0), (130.0, 40.0)],
                    (100.0, 40.0),
                )],
            )],
        );
        let mut data = LayoutData {
            nodes: vec![node("A", None, false, 100.0, 50.0)],
            edges: vec![layout_edge("e", "A", "A")],
            ..LayoutData::default()
        };
        run(&mut data, &output, &[], &[]);

        assert_points(
            &data.edges[0].points,
            &[(100.0, 20.0), (130.0, 20.0), (130.0, 40.0), (100.0, 40.0)],
        );
    }

    // render.spec.ts: "evenGroupFrames". Each scene is applied through `apply`; the spec's nodeDb
    // entries are the nodes' ELK boxes, given relative to their parent.

    /// A box as `(x, y, width, height)`.
    type Box4 = (f64, f64, f64, f64);

    /// Applies one group `g` at the root with the given children (absolute boxes), and returns the
    /// data, so the frame can be read back.
    fn group_scene(group: Box4, title: f64, children: &[(&str, Box4)]) -> LayoutData {
        let kids: Vec<JsonNode> = children
            .iter()
            .map(|(id, (x, y, w, h))| elk(id, x - group.0, y - group.1, *w, *h, vec![]))
            .collect();
        let output = root(
            vec![elk("g", group.0, group.1, group.2, group.3, kids)],
            vec![],
        );
        let mut nodes = vec![node("g", None, true, group.2, group.3)];
        for (id, _) in children {
            nodes.push(node(id, Some("g"), false, 0.0, 0.0));
        }
        let mut data = LayoutData {
            nodes,
            ..LayoutData::default()
        };
        let parents: Vec<(&str, &str)> = children.iter().map(|(id, _)| (*id, "g")).collect();
        run(&mut data, &output, &parents, &[("g", title)]);
        data
    }

    #[test]
    fn pulls_a_frame_in_to_even_padding_when_a_routing_lane_inflated_one_side() {
        // Nodes 100 wide at 24 from the left, the frame 200 wide. The frame is pulled to 24 on each
        // side: 148 wide and 148 high, its top left where ELK put it.
        let data = group_scene(
            (0.0, 0.0, 200.0, 148.0),
            0.0,
            &[("n0", (24.0, 48.0, 100.0, 76.0))],
        );
        let g = &data.nodes[0];
        assert_eq!(g.x, Some(74.0));
        assert_eq!(g.y, Some(74.0));
        assert_eq!(g.width, Some(148.0));
        assert_eq!(g.height, Some(148.0));
    }

    #[test]
    fn leaves_a_frame_alone_when_its_padding_is_already_even() {
        let data = group_scene(
            (0.0, 0.0, 148.0, 148.0),
            0.0,
            &[("n0", (24.0, 48.0, 100.0, 76.0))],
        );
        assert_eq!(data.nodes[0].width, Some(148.0));
        assert_eq!(data.nodes[0].height, Some(148.0));
    }

    #[test]
    fn never_squeezes_a_frame_narrower_than_its_own_title() {
        // The title is 200 wide, so the floor wins; the frame is clamped to what ELK gave.
        let data = group_scene(
            (0.0, 0.0, 300.0, 148.0),
            200.0,
            &[("n0", (24.0, 48.0, 40.0, 76.0))],
        );
        let g = &data.nodes[0];
        assert_eq!(g.width, Some(200.0));
        // The centre is (100, 74): the frame starts at 0.
        assert_eq!(g.x, Some(100.0));
    }

    #[test]
    fn never_grows_a_frame_beyond_what_elk_sized_it_to() {
        // 10 - 24 would put the left edge at -14 and 110 + 24 the right at 134; both are clamped
        // back to the frame ELK gave.
        let data = group_scene(
            (0.0, 0.0, 120.0, 148.0),
            0.0,
            &[("n", (10.0, 48.0, 100.0, 76.0))],
        );
        assert_eq!(data.nodes[0].width, Some(120.0));
        assert_eq!(data.nodes[0].x, Some(60.0));
    }

    #[test]
    fn paints_the_clamped_frame_not_the_wider_title_into_the_layout_node() {
        // A 100-wide frame under a 200-wide title: the clamp refuses to widen the frame, so the
        // layout node reports the clamped width as well.
        let data = group_scene(
            (0.0, 0.0, 100.0, 148.0),
            200.0,
            &[("n", (10.0, 48.0, 40.0, 76.0))],
        );
        assert_eq!(data.nodes[0].width, Some(100.0));
    }

    #[test]
    fn still_reports_the_honoured_title_floor_when_elk_left_room_for_it() {
        let data = group_scene(
            (0.0, 0.0, 300.0, 148.0),
            200.0,
            &[("n", (24.0, 48.0, 40.0, 76.0))],
        );
        assert_eq!(data.nodes[0].width, Some(200.0));
    }

    #[test]
    fn skips_a_group_with_no_children_rather_than_collapsing_it() {
        let output = root(vec![elk("g", 0.0, 0.0, 200.0, 100.0, vec![])], vec![]);
        let mut data = LayoutData {
            nodes: vec![node("g", None, true, 200.0, 100.0)],
            ..LayoutData::default()
        };
        run(&mut data, &output, &[], &[("g", 0.0)]);
        assert_eq!(data.nodes[0].width, Some(200.0));
        assert_eq!(data.nodes[0].height, Some(100.0));
    }

    #[test]
    fn measures_a_parent_against_children_it_has_already_pulled_in() {
        // Outer 300x220, inner 200x148 at (24,48), leaf 100x76 at (48,96). The inner frame is pulled
        // to 148; the outer one, deepest-first, then measures the tightened inner box.
        let output = root(
            vec![elk(
                "outer",
                0.0,
                0.0,
                300.0,
                220.0,
                vec![elk(
                    "inner",
                    24.0,
                    48.0,
                    200.0,
                    148.0,
                    vec![elk("leaf", 24.0, 48.0, 100.0, 76.0, vec![])],
                )],
            )],
            vec![],
        );
        let mut data = LayoutData {
            nodes: vec![
                node("outer", None, true, 300.0, 220.0),
                node("inner", Some("outer"), true, 200.0, 148.0),
                node("leaf", Some("inner"), false, 100.0, 76.0),
            ],
            ..LayoutData::default()
        };
        run(
            &mut data,
            &output,
            &[("inner", "outer"), ("leaf", "inner")],
            &[],
        );
        assert_eq!(data.nodes[1].width, Some(148.0));
        assert_eq!(data.nodes[0].width, Some(196.0));
    }

    #[test]
    fn keeps_a_frame_around_a_lane_belonging_to_the_groups_own_interior() {
        // Group P holds C (with a leaf) and a sibling. The edge from the leaf to the sibling runs
        // around C, so the lane is inside P but outside C. C is pulled in to 148; P keeps the lane,
        // reaching x = 350 + 24 = 374.
        let output = root(
            vec![elk(
                "P",
                0.0,
                0.0,
                400.0,
                220.0,
                vec![
                    elk(
                        "C",
                        24.0,
                        48.0,
                        200.0,
                        148.0,
                        vec![elk("leaf", 24.0, 48.0, 100.0, 76.0, vec![])],
                    ),
                    elk("sib", 260.0, 96.0, 60.0, 76.0, vec![]),
                ],
            )],
            vec![elk_edge(
                "e",
                "leaf",
                "sib",
                vec![section(
                    (148.0, 134.0),
                    &[(350.0, 134.0), (350.0, 60.0)],
                    (260.0, 134.0),
                )],
            )],
        );
        let mut data = LayoutData {
            nodes: vec![
                node("P", None, true, 400.0, 220.0),
                node("C", Some("P"), true, 200.0, 148.0),
                node("leaf", Some("C"), false, 100.0, 76.0),
                node("sib", Some("P"), false, 60.0, 76.0),
            ],
            ..LayoutData::default()
        };
        run(
            &mut data,
            &output,
            &[("C", "P"), ("leaf", "C"), ("sib", "P")],
            &[],
        );
        assert_eq!(data.nodes[1].width, Some(148.0));
        // P spans 0 to 374, so its centre is 187.
        assert_eq!(data.nodes[0].width, Some(374.0));
        assert_eq!(data.nodes[0].x, Some(187.0));
    }

    #[test]
    fn never_pulls_a_frame_past_the_anchor_of_an_edge_that_terminates_on_the_group() {
        // An edge from outside ends on G at x = 12. Its end point is a lane point, so the left side is
        // max(0, 12 - 24) = 0, and the right side is the leaf's right edge plus padding, 160 + 24 =
        // 184. The frame is therefore 184 wide, not pulled past the anchor.
        let output = root(
            vec![elk(
                "g",
                0.0,
                0.0,
                200.0,
                148.0,
                vec![elk("n", 60.0, 48.0, 100.0, 76.0, vec![])],
            )],
            vec![elk_edge(
                "e",
                "ext",
                "g",
                vec![section((500.0, -40.0), &[(500.0, -10.0)], (12.0, 0.0))],
            )],
        );
        let mut data = LayoutData {
            nodes: vec![
                node("g", None, true, 200.0, 148.0),
                node("n", Some("g"), false, 100.0, 76.0),
            ],
            ..LayoutData::default()
        };
        run(&mut data, &output, &[("n", "g")], &[]);
        assert_eq!(data.nodes[0].width, Some(184.0));
        assert_eq!(data.nodes[0].x, Some(92.0));
    }

    #[test]
    fn leaves_elks_own_origin_readable_after_moving_a_frame() {
        // The frame moves in to 40 - 24 = 16. `calcOffset` still reads ELK's origin (0), which an
        // edge section inside the group is relative to.
        let output = root(
            vec![elk(
                "g",
                0.0,
                0.0,
                200.0,
                148.0,
                vec![elk("n", 40.0, 48.0, 100.0, 76.0, vec![])],
            )],
            vec![],
        );
        let state = ElkState {
            parent_by_id: HashMap::from([("n".to_string(), "g".to_string())]),
            ..ElkState::default()
        };
        let mut data = LayoutData {
            nodes: vec![
                node("g", None, true, 200.0, 148.0),
                node("n", Some("g"), false, 100.0, 76.0),
            ],
            ..LayoutData::default()
        };
        let mut layout = Layout::new(&data, &state, &scene_crossing);
        layout.apply_node_positions(&mut data, &output.children, Point::default());
        layout.even_group_frames(&mut data, &output.children, &output);
        assert_eq!(layout.nodes["g"].pos.x, 16.0);
        assert_eq!(layout.calc_offset("n", "n"), Point::new(0.0, 0.0));
    }

    // render.spec.ts: "group title padding and edge attachment", the `sanitizeElkEdgePoints` cases.
    // A leaf without an intersect is a leaf whose crossing gives nothing, as in the spec.

    fn no_outline(_: &str, _: CenterBox, _: Point) -> Option<Point> {
        None
    }

    /// A layout whose nodes are given by their centre and size; a subgraph is a group.
    fn clip_layout<'a>(
        state: &'a ElkState,
        crossing: Crossing<'a>,
        nodes: &[(&str, bool, Point, f64, f64)],
    ) -> Layout<'a> {
        let nodes = nodes
            .iter()
            .map(|&(id, is_group, centre, width, height)| {
                (
                    id.to_string(),
                    DbNode {
                        is_group,
                        pos: Point::new(centre.x - width / 2.0, centre.y - height / 2.0),
                        positioned: true,
                        centre,
                        width,
                        height,
                        ..DbNode::default()
                    },
                )
            })
            .collect();
        Layout {
            nodes,
            index: HashMap::new(),
            state,
            crossing,
        }
    }

    #[test]
    fn trims_interior_points_on_both_ends_other_endpoint_leaf_and_group() {
        // render.spec.ts: "trims interior points on both ends (other endpoint is group: %s)"
        let state = ElkState::default();
        let crossing: Crossing<'_> = &no_outline;
        for other_is_group in [false, true] {
            let layout = clip_layout(
                &state,
                crossing,
                &[
                    (
                        "other",
                        other_is_group,
                        Point::new(-100.0, 130.0),
                        40.0,
                        40.0,
                    ),
                    ("group", true, Point::new(100.0, 100.0), 100.0, 120.0),
                ],
            );
            let points = vec![
                p(-100.0, 130.0),
                p(-80.0, 130.0),
                p(20.0, 130.0),
                p(53.0, 130.0),
                p(60.0, 130.0),
                p(100.0, 100.0),
            ];
            let expected = vec![p(-80.0, 130.0), p(20.0, 130.0), p(50.0, 130.0)];
            assert_eq!(layout.sanitize(points.clone(), "other", "group"), expected);
            let mut reversed = points;
            reversed.reverse();
            let mut expected_reversed = expected;
            expected_reversed.reverse();
            assert_eq!(
                layout.sanitize(reversed, "group", "other"),
                expected_reversed
            );
        }
    }

    #[test]
    fn keeps_valid_group_anchors_unchanged() {
        // render.spec.ts: "keeps valid group anchors unchanged"
        let state = ElkState::default();
        let crossing: Crossing<'_> = &no_outline;
        let layout = clip_layout(
            &state,
            crossing,
            &[
                ("source", true, Point::new(-100.0, 130.0), 40.0, 40.0),
                ("target", true, Point::new(100.0, 100.0), 100.0, 120.0),
            ],
        );
        let route = vec![p(-80.0, 130.0), p(20.0, 130.0), p(50.0, 130.0)];
        let mut points = vec![p(-100.0, 130.0)];
        points.extend(route.iter().copied());
        points.push(p(100.0, 100.0));
        assert_eq!(layout.sanitize(points, "source", "target"), route);
    }

    #[test]
    fn clips_the_actual_crossing_from_outside_through_inside_to_the_centre() {
        // render.spec.ts: "clips the actual crossing from %j through %j to %j"
        let state = ElkState::default();
        let crossing: Crossing<'_> = &no_outline;
        let cases = [
            (p(20.0, 130.0), p(60.0, 130.0), p(50.0, 130.0)),
            (p(180.0, 130.0), p(140.0, 130.0), p(150.0, 130.0)),
            (p(80.0, 10.0), p(80.0, 60.0), p(80.0, 40.0)),
            (p(80.0, 190.0), p(80.0, 140.0), p(80.0, 160.0)),
            (p(20.0, 50.0), p(80.0, 150.0), p(50.0, 100.0)),
            (p(20.0, 10.0), p(80.0, 70.0), p(50.0, 40.0)),
        ];
        for (outside, inside, crossing_point) in cases {
            // The source is a group whose centre is 20 left of `outside`, and the target is the
            // group of the spec, centred at (100, 100).
            let layout = clip_layout(
                &state,
                crossing,
                &[
                    ("source", true, p(outside.x - 20.0, outside.y), 40.0, 40.0),
                    ("group", true, p(100.0, 100.0), 100.0, 120.0),
                ],
            );
            let points = vec![
                p(outside.x - 20.0, outside.y),
                outside,
                inside,
                p(100.0, 100.0),
            ];
            let expected = vec![outside, crossing_point];
            assert_eq!(layout.sanitize(points.clone(), "source", "group"), expected);
            let mut reversed = points;
            reversed.reverse();
            let mut expected_reversed = expected;
            expected_reversed.reverse();
            assert_eq!(
                layout.sanitize(reversed, "group", "source"),
                expected_reversed
            );
        }
    }

    // render.spec.ts and terminalLabels.spec.ts: cases that run the full pipeline (the build and
    // the ELK layout), or that test the terminal-label passes, which are not ported (see the module
    // documentation). They keep their reference names so the count of ignored tests shows the work
    // left.

    #[test]
    #[ignore = "port: runs runElkLayoutCore (build and ELK layout)"]
    fn reserves_title_plus_padding_before_routing_around_child_width() {}

    #[test]
    #[ignore = "port: runs runElkLayoutCore (build and ELK layout)"]
    fn keeps_the_title_minimum_when_cross_boundary_edges_replace_a_container_algorithm() {}

    #[test]
    #[ignore = "port: runs runElkLayoutCore; the small-node cases need ELK's ports"]
    fn centres_a_single_edge_between_two_start_end_state_circles() {}

    #[test]
    #[ignore = "port: runs runElkLayoutCore; the small-node cases need ELK's ports"]
    fn moves_a_small_node_onto_the_routed_line_instead_of_bending_the_edge() {}

    #[test]
    #[ignore = "port: runs runElkLayoutCore; the small-node cases need ELK's ports"]
    fn keeps_spread_anchors_on_a_wide_thin_fork_join_bar() {}

    #[test]
    #[ignore = "port: runs runElkLayoutCore with elk.box and elk.rectpacking, which the Rust ELK refuses"]
    fn clips_the_fallback_endpoints_back_to_the_node_borders_under_elk_box() {}

    #[test]
    #[ignore = "port: runs runElkLayoutCore with elk.box and elk.rectpacking, which the Rust ELK refuses"]
    fn positions_the_label_of_an_unrouted_edge_under_elk_box() {}

    #[test]
    #[ignore = "port: flowchart has no terminal labels; the terminal-label passes are not ported"]
    fn keeps_every_cardinality_label_beside_its_own_end_clear_of_its_marker_and_the_class_boxes() {}

    #[test]
    #[ignore = "port: flowchart has no terminal labels; the terminal-label passes are not ported"]
    fn keeps_a_label_off_a_neighbouring_edge_that_leaves_an_adjacent_port() {}

    #[test]
    #[ignore = "port: flowchart has no terminal labels; the terminal-label passes are not ported"]
    fn puts_start_and_end_labels_on_the_same_sides_of_a_top_down_edge_as_dagre() {}

    #[test]
    #[ignore = "port: flowchart has no terminal labels; the terminal-label passes are not ported"]
    fn drops_centres_from_an_earlier_layered_run_when_the_same_data_is_laid_out_again() {}

    #[test]
    #[ignore = "port: flowchart has no terminal labels; the terminal-label passes are not ported"]
    fn keeps_a_label_that_enters_another_namespace_clear_of_its_frame() {}

    #[test]
    #[ignore = "port: slideTerminalLabelsOffFrames is not ported (no flowchart terminal labels)"]
    fn does_not_slide_a_label_past_the_far_end_of_its_end_segment() {}

    #[test]
    #[ignore = "port: followMovedEndpoints is not ported (no flowchart terminal labels)"]
    fn moves_a_placed_label_with_an_endpoint_that_a_later_pass_slid_along_the_side() {}
}
