//! The cytoscape state of an architecture diagram: its nodes and edges in the order the renderer adds
//! them (groups, then services and junctions, then edges), their positions and their styles, and the
//! functions cytoscape computes from them.
//!
//! The reference gives every node the style of `architectureRenderer.ts` 124-145 and 330-510, with
//! cytoscape's defaults under it. The values that reach the output, and the rules they follow, are
//! written where they are used:
//!
//! - a service and a junction are `iconSize` wide and high, with no padding and no border; a group
//!   that has children is as large as its children (`updateCompoundBounds`), with a border of one
//!   pixel (the `:parent` default) and the padding of `.node-group`; a group without children is a
//!   leaf of 30 by 30 with the same padding (the style default);
//! - a label is measured as `Math.ceil` of the canvas width of the text (`calculateLabelDimensions`),
//!   in the bundled sans face, which the harness maps `Helvetica Neue` to; its box has a margin of 2
//!   on each side (`updateBoundsFromLabel`, `marginOfError`);
//! - an edge is 3 px wide, so its box has 1.5 px on each side (`edge` default in `properties.mjs`).
//!
//! The functions return values as cytoscape computes them at the moment of the call. The state is
//! kept consistent by [`Scene::update_compounds`], which the layout calls after every change of
//! position, as cytoscape's dirty flags would.

use ferrigura_model::architecture::{ArchitectureDiagram, Direction};
use ferrigura_text::{FontFamily, FontStyle, FontWeight, face, measure};

use super::geometry::{Box2, js_max3, js_min3};
use crate::RenderError;

/// The font size of an edge label: the cytoscape default, since the stylesheet sets the size of the
/// node labels only (`architectureRenderer.ts` 124-145).
pub const EDGE_FONT_SIZE: f64 = 16.0;

/// The margin of a label's box on each side: `marginOfError` of `updateBoundsFromLabel`.
const LABEL_MARGIN: f64 = 2.0;

/// The border of a compound node: `border-width` 1 from cytoscape's `:parent` default, drawn at the
/// centre, so `outerWidth` adds it once.
const COMPOUND_BORDER: f64 = 1.0;

/// The width and height of a group without children, the style default (`properties.mjs`, `width`).
const CHILDLESS_GROUP_SIZE: f64 = 30.0;

/// The width of an edge, the `edge` default of cytoscape's stylesheet, which gives a half-width of
/// 1.5 px around the path.
const EDGE_HALF_WIDTH: f64 = 1.5;

/// The kinds of node, in the order the renderer adds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A group (`.node-group`): a compound when it has children.
    Group,
    /// A service (`.node-service`).
    Service,
    /// A junction (`.node-junction`).
    Junction,
}

/// A node of the scene.
#[derive(Debug, Clone)]
pub struct Node {
    /// The id.
    pub id: String,
    /// The kind of the node.
    pub kind: Kind,
    /// The index of the enclosing group, or `None` at the top.
    pub parent: Option<usize>,
    /// The indices of the children, in the order they were added.
    pub children: Vec<usize>,
    /// The title, or `None` when the node has none (`data(label)` is then undefined).
    pub label: Option<String>,
    /// The icon name, or `None` when the node has none.
    pub icon: Option<String>,
    /// The icon text, or `None` when the service has none.
    pub icon_text: Option<String>,
    /// The position: the centre of a node. For a group with children it is recomputed from them.
    pub position: (f64, f64),
    /// `_p.autoWidth` and `_p.autoHeight` of a group with children, set by
    /// [`Scene::update_compounds`]. Unused by the other nodes.
    pub auto_size: (f64, f64),
    /// The canvas width of the label, `Math.ceil` of the measured text, or `None` without a label.
    pub label_width: Option<f64>,
}

/// An edge of the scene.
#[derive(Debug, Clone)]
pub struct Edge {
    /// The id, `${lhsId}-${rhsId}` as `architectureRenderer.ts` 112-120 gives it.
    pub id: String,
    /// The index of the source node.
    pub source: usize,
    /// The index of the target node.
    pub target: usize,
    /// The side of the source end.
    pub source_dir: Direction,
    /// The side of the target end.
    pub target_dir: Direction,
    /// Whether the source end has an arrow head.
    pub source_arrow: bool,
    /// Whether the target end has an arrow head.
    pub target_arrow: bool,
    /// Whether the source end carries the `{group}` modifier.
    pub source_group: bool,
    /// Whether the target end carries the `{group}` modifier.
    pub target_group: bool,
    /// The title of the edge, or `None`.
    pub label: Option<String>,
    /// Whether the edge is an XY edge, drawn with the `segments` curve style (`addEdges`).
    pub segments: bool,
    /// The `segment-weights` style: 0 until the layout's segment handler sets it.
    pub weight: f64,
    /// The `segment-distances` style: 0.5 until the layout's segment handler sets it.
    pub distance: f64,
    /// The canvas width of the label, or `None`.
    pub label_width: Option<f64>,
}

/// A point.
pub type Point = (f64, f64);

/// The geometry cytoscape computes for an edge: the ends, the middle and the points of a segmented
/// edge (`rscratch`, `rstyle`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeGeometry {
    /// The start of the line (`startX`, `startY`).
    pub start: Point,
    /// The end of the line.
    pub end: Point,
    /// The start of the arrow, which is `sourceEndpoint()`.
    pub arrow_start: Point,
    /// The end of the arrow, which is `targetEndpoint()`.
    pub arrow_end: Point,
    /// The middle, which is `midpoint()`.
    pub middle: Point,
    /// The points of a `segments` edge (`linePts`), or `None` for a straight edge.
    pub line_points: Option<[Point; 3]>,
}

/// The sizes of the diagram's configuration that the scene uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// `iconSize`: the width and the height of a service, a junction and a group's icon box.
    pub icon_size: f64,
    /// `padding`: the padding of a group.
    pub padding: f64,
    /// `fontSize`: the size of the node labels.
    pub font_size: f64,
}

/// The state of the graph: the nodes, the edges, and the configuration's sizes.
#[derive(Debug, Clone)]
pub struct Scene {
    /// The nodes: groups, then services, then junctions, each in the order of the diagram.
    pub nodes: Vec<Node>,
    /// The edges, in the order of the diagram.
    pub edges: Vec<Edge>,
    /// The sizes of the configuration.
    pub metrics: Metrics,
}

/// The label box of a node or an edge: its centre (the label position) and its size, with the margin.
fn label_box(x: f64, y_top: f64, lw: f64, lh: f64, centred: bool) -> Box2 {
    // `labelHalign` centre: `lx ± lw / 2`; `labelValign` bottom: `ly` to `ly + lh`; the edge's
    // centre: `ly ± lh / 2`. Each side then moves by the margin (`updateBoundsFromLabel`).
    let (y1, y2) = if centred {
        (y_top - lh / 2.0, y_top + lh / 2.0)
    } else {
        (y_top, y_top + lh)
    };
    let x1 = x - lw / 2.0;
    let x2 = x + lw / 2.0;
    Box2::new(
        x1 - LABEL_MARGIN,
        y1 - LABEL_MARGIN,
        x2 + LABEL_MARGIN,
        y2 + LABEL_MARGIN,
    )
}

/// `manualEndptToPx` for a side: the node's position plus the side's fraction of its outer size.
/// `'0 50%'`, `'100% 50%'`, `'50% 0'` and `'50% 100%'` are the four sides (`architectureRenderer.ts`
/// 126-145), and a value without `%` is a number of pixels.
fn side_offset(dir: Direction) -> ((f64, bool), (f64, bool)) {
    match dir {
        Direction::L => ((0.0, false), (0.5, true)),
        Direction::R => ((1.0, true), (0.5, true)),
        Direction::T => ((0.5, true), (0.0, false)),
        Direction::B => ((0.5, true), (1.0, true)),
    }
}

/// `Shape.shortenIntersection`: the point at `amount` before `intersection`, on the line from
/// `offset`. With no arrow, the amount is zero and the point is `intersection` up to rounding.
fn shorten(intersection: Point, offset: Point, amount: f64) -> Point {
    let disp = (intersection.0 - offset.0, intersection.1 - offset.1);
    let length = (disp.0 * disp.0 + disp.1 * disp.1).sqrt();
    let mut ratio = (length - amount) / length;
    if ratio < 0.0 {
        ratio = 0.00001;
    }
    (offset.0 + ratio * disp.0, offset.1 + ratio * disp.1)
}

impl Scene {
    /// The scene of a diagram, with every node at the origin and the compounds at the centre of
    /// their children, as the renderer's `cy.add` leaves them.
    ///
    /// # Errors
    ///
    /// Returns an error if a label cannot be measured, or if an edge names a node that is not in the
    /// diagram (the parser rules that out).
    pub fn new(
        diagram: &ArchitectureDiagram,
        metrics: Metrics,
        measure_label: &mut impl FnMut(&str, f64) -> Result<f64, RenderError>,
    ) -> Result<Self, RenderError> {
        // The nodes in the order the renderer adds them, each with the id of its group (`in`).
        let mut nodes: Vec<Node> = Vec::new();
        let mut parent_ids: Vec<String> = Vec::new();
        let text = |value: &str| (!value.is_empty()).then(|| value.to_owned());
        let mut label_width =
            |title: &Option<String>, size: f64| -> Result<Option<f64>, RenderError> {
                match title {
                    Some(title) => Ok(Some(ceil(measure_label(title, size)?))),
                    None => Ok(None),
                }
            };
        for group in &diagram.groups {
            let label = text(&group.title);
            let width = label_width(&label, metrics.font_size)?;
            nodes.push(Node {
                id: group.id.clone(),
                kind: Kind::Group,
                parent: None,
                children: Vec::new(),
                label,
                icon: text(&group.icon),
                icon_text: None,
                position: (0.0, 0.0),
                auto_size: (0.0, 0.0),
                label_width: width,
            });
            parent_ids.push(group.parent.clone());
        }
        for service in &diagram.services {
            let label = text(&service.title);
            let width = label_width(&label, metrics.font_size)?;
            nodes.push(Node {
                id: service.id.clone(),
                kind: Kind::Service,
                parent: None,
                children: Vec::new(),
                label,
                icon: text(&service.icon),
                icon_text: text(&service.icon_text),
                position: (0.0, 0.0),
                auto_size: (0.0, 0.0),
                label_width: width,
            });
            parent_ids.push(service.parent.clone());
        }
        for junction in &diagram.junctions {
            nodes.push(Node {
                id: junction.id.clone(),
                kind: Kind::Junction,
                parent: None,
                children: Vec::new(),
                label: None,
                icon: None,
                icon_text: None,
                position: (0.0, 0.0),
                auto_size: (0.0, 0.0),
                label_width: None,
            });
            parent_ids.push(junction.parent.clone());
        }
        let index_of: std::collections::HashMap<String, usize> = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.clone(), index))
            .collect();
        // A group is added before the nodes that name it: the parser checks it (`architectureDb.ts`).
        for (index, parent_id) in parent_ids.iter().enumerate() {
            if parent_id.is_empty() {
                continue;
            }
            let parent_index = *index_of.get(parent_id).ok_or_else(|| {
                RenderError::Layout(format!("{parent_id:?} is not a group of the diagram"))
            })?;
            nodes[index].parent = Some(parent_index);
            nodes[parent_index].children.push(index);
        }

        let mut edges = Vec::with_capacity(diagram.edges.len());
        for edge in &diagram.edges {
            let source = *index_of.get(&edge.lhs_id).ok_or_else(|| {
                RenderError::Layout(format!("{:?} is not a node of the diagram", edge.lhs_id))
            })?;
            let target = *index_of.get(&edge.rhs_id).ok_or_else(|| {
                RenderError::Layout(format!("{:?} is not a node of the diagram", edge.rhs_id))
            })?;
            let label = text(&edge.title);
            let label_width = label_width(&label, EDGE_FONT_SIZE)?;
            edges.push(Edge {
                id: format!("{}-{}", edge.lhs_id, edge.rhs_id),
                source,
                target,
                source_dir: edge.lhs_dir,
                target_dir: edge.rhs_dir,
                source_arrow: edge.lhs_into,
                target_arrow: edge.rhs_into,
                source_group: edge.lhs_group,
                target_group: edge.rhs_group,
                label,
                segments: is_xy(edge.lhs_dir, edge.rhs_dir),
                weight: 0.0,
                distance: 0.5,
                label_width,
            });
        }
        let mut scene = Self {
            nodes,
            edges,
            metrics,
        };
        scene.update_compounds();
        Ok(scene)
    }

    /// `updateCompoundBounds` of every group with children, children first: each group's size and
    /// position come from the boxes of its children, with their labels (`compound-sizing-wrt-labels`).
    /// Groups are added before the nodes they hold, so the reverse order of the nodes is children
    /// first.
    pub fn update_compounds(&mut self) {
        for index in (0..self.nodes.len()).rev() {
            if self.nodes[index].children.is_empty() {
                continue;
            }
            let children = self.children_box(index).finite();
            let (bounds, fallback) = if children.w() == 0.0 || children.h() == 0.0 {
                // Children with no area keep the group where it is, at the style size.
                let (x, y) = self.nodes[index].position;
                let size = CHILDLESS_GROUP_SIZE;
                (
                    Box2::new(
                        x - size / 2.0,
                        y - size / 2.0,
                        x + size / 2.0,
                        y + size / 2.0,
                    ),
                    true,
                )
            } else {
                (children, false)
            };
            let width = bounds.w().max(0.0);
            let height = bounds.h().max(0.0);
            self.nodes[index].auto_size = (width, height);
            if !fallback {
                // `pos.x = (x1 + x2) / 2` with no bias, as `updateCompoundBounds` writes it.
                self.nodes[index].position =
                    ((bounds.x1 + bounds.x2) / 2.0, (bounds.y1 + bounds.y2) / 2.0);
            }
        }
    }

    /// Whether the node is a compound: a group with children.
    #[must_use]
    pub fn is_compound(&self, index: usize) -> bool {
        !self.nodes[index].children.is_empty()
    }

    /// `width()` and `height()`: the style size of a leaf, or the size of a compound from its children.
    #[must_use]
    pub fn style_size(&self, index: usize) -> (f64, f64) {
        let node = &self.nodes[index];
        if self.is_compound(index) {
            return node.auto_size;
        }
        match node.kind {
            Kind::Group => (CHILDLESS_GROUP_SIZE, CHILDLESS_GROUP_SIZE),
            Kind::Service | Kind::Junction => (self.metrics.icon_size, self.metrics.icon_size),
        }
    }

    /// `padding()`: the padding of a group (`padding` of `.node-group`, or the compound's own, which is
    /// the same value in pixels), and none for a service or a junction.
    #[must_use]
    pub fn padding(&self, index: usize) -> f64 {
        match self.nodes[index].kind {
            Kind::Group => self.metrics.padding,
            Kind::Service | Kind::Junction => 0.0,
        }
    }

    /// `outerWidth()` and `outerHeight()`: the size with the border and the padding.
    #[must_use]
    pub fn outer_size(&self, index: usize) -> (f64, f64) {
        let (width, height) = self.style_size(index);
        let border = if self.is_compound(index) {
            COMPOUND_BORDER
        } else {
            0.0
        };
        let padding = 2.0 * self.padding(index);
        (width + border + padding, height + border + padding)
    }

    /// The box of the node's label, with its margin, where the label is set: `recalculateNodeLabelProjection`
    /// centres it on the node (`text-halign: center`) and sets it below the body (`text-valign: bottom`).
    /// `None` without a label.
    ///
    /// The label of a group sits on the group's centre line: the reference's `textY` for a group has
    /// no padding (measured with `pnpm port:geometry architecture`, probes `p09` and `p11`).
    #[must_use]
    pub fn node_label_box(&self, index: usize) -> Option<Box2> {
        let node = &self.nodes[index];
        let label_width = node.label_width?;
        let (x, y) = node.position;
        let (_, height) = self.style_size(index);
        let padding = match node.kind {
            Kind::Group => 0.0,
            Kind::Service | Kind::Junction => self.padding(index),
        };
        let label_y = y + height / 2.0 + padding;
        Some(label_box(
            x,
            label_y,
            label_width,
            self.metrics.font_size,
            false,
        ))
    }

    /// The label box that the bounds take from a node: a group with children contributes none of its
    /// own label to the bounds of itself or of its parent (the reference's bounds of the group are the
    /// body, and its title is drawn but not measured; probes `p11` and the corpus of `fcose`). A leaf's
    /// label always counts.
    #[must_use]
    fn bounds_label_box(&self, index: usize) -> Option<Box2> {
        if self.is_compound(index) {
            return None;
        }
        self.node_label_box(index)
    }

    /// The default box of a node, as `boundingBox()` of one node gives it: the body and the label,
    /// settled.
    #[must_use]
    pub fn node_box(&self, index: usize) -> Box2 {
        let (x, y) = self.nodes[index].position;
        let (w, h) = self.outer_size(index);
        let mut bounds = Box2::around(x, y, w, h);
        if let Some(label) = self.bounds_label_box(index) {
            bounds.include_box(label);
        }
        bounds.settled()
    }

    /// The box of a node as its parent's bounds see it: the body with its one-pixel expansion, and the
    /// label with its margin (`cachedBoundingBoxImpl` with the parent's options, which have no overlay).
    #[must_use]
    pub fn child_entry_box(&self, index: usize) -> Box2 {
        let (x, y) = self.nodes[index].position;
        let (w, h) = self.outer_size(index);
        let mut bounds = Box2::around(x, y, w, h).expand1();
        if let Some(label) = self.bounds_label_box(index) {
            bounds.include_box(label);
        }
        bounds
    }

    /// The box of the children of a group, with their labels: `children.boundingBox({ includeLabels })`.
    #[must_use]
    pub fn children_box(&self, index: usize) -> Box2 {
        let mut bounds = Box2::EMPTY;
        for &child in &self.nodes[index].children {
            bounds.include_box(self.child_entry_box(child));
        }
        bounds
    }

    /// The geometry of an edge from the current positions and styles: the endpoints from the
    /// endpoint percentages, the middle, and for an XY edge the bend of its one segment
    /// (`edge-control-points.mjs`, `findSegmentsPoints`, `storeAllpts`).
    #[must_use]
    pub fn edge_geometry(&self, index: usize) -> EdgeGeometry {
        let edge = &self.edges[index];
        let source_pos = self.nodes[edge.source].position;
        let target_pos = self.nodes[edge.target].position;
        let source_manual = self.manual_endpoint(edge.source, edge.source_dir);
        let target_manual = self.manual_endpoint(edge.target, edge.target_dir);
        if edge.segments {
            let (x1, y1) = source_manual;
            let (x2, y2) = target_manual;
            // `findMidptPtsEtc` with `edge-distances: endpoints`: the normal of the endpoints' line.
            let dy = y2 - y1;
            let dx = x2 - x1;
            let length = (dx * dx + dy * dy).sqrt();
            let normal = (-dy / length, dx / length);
            let w1 = 1.0 - edge.weight;
            let w2 = edge.weight;
            let adjusted = (x1 * w1 + x2 * w2, y1 * w1 + y2 * w2);
            let bend = (
                adjusted.0 + normal.0 * edge.distance,
                adjusted.1 + normal.1 * edge.distance,
            );
            // With one bend the end points are shortened towards it by no arrow (amount zero).
            let arrow_start = shorten(source_manual, bend, 0.0);
            let arrow_end = shorten(target_manual, bend, 0.0);
            return EdgeGeometry {
                start: arrow_start,
                end: arrow_end,
                arrow_start,
                arrow_end,
                middle: bend,
                line_points: Some([arrow_start, bend, arrow_end]),
            };
        }
        // A straight edge: the reference's `p1` and `p2` are the other end's node position plus its
        // endpoint fraction (`findEndpoints`, the non-segment branch).
        let source_fraction = endpoint_fraction(edge.source_dir);
        let target_fraction = endpoint_fraction(edge.target_dir);
        let p1 = (
            source_pos.0 + source_fraction.0,
            source_pos.1 + source_fraction.1,
        );
        let p2 = (
            target_pos.0 + target_fraction.0,
            target_pos.1 + target_fraction.1,
        );
        let arrow_end = shorten(target_manual, p1, 0.0);
        let arrow_start = shorten(source_manual, p2, 0.0);
        let middle = (
            (arrow_start.0 + arrow_end.0 + arrow_start.0 + arrow_end.0) / 4.0,
            (arrow_start.1 + arrow_end.1 + arrow_start.1 + arrow_end.1) / 4.0,
        );
        EdgeGeometry {
            start: arrow_start,
            end: arrow_end,
            arrow_start,
            arrow_end,
            middle,
            line_points: None,
        }
    }

    /// `manualEndptToPx` of a side of a node: the node's position plus the side's offset.
    #[must_use]
    pub fn manual_endpoint(&self, index: usize, dir: Direction) -> Point {
        let (x, y) = self.nodes[index].position;
        let (w, h) = self.outer_size(index);
        let ((fx, x_pct), (fy, y_pct)) = side_offset(dir);
        let dx = if x_pct { fx * w } else { fx };
        let dy = if y_pct { fy * h } else { fy };
        (dx + x, dy + y)
    }

    /// The default box of an edge: its rough box, its segments and its label, settled
    /// (`boundingBoxImpl` for an edge).
    #[must_use]
    pub fn edge_box(&self, index: usize) -> Box2 {
        let edge = &self.edges[index];
        let geometry = self.edge_geometry(index);
        let mut bounds = Box2::EMPTY;
        let xs = [
            geometry.arrow_start.0,
            geometry.middle.0,
            geometry.arrow_end.0,
        ];
        let ys = [
            geometry.arrow_start.1,
            geometry.middle.1,
            geometry.arrow_end.1,
        ];
        bounds.include(
            js_min3(xs[0], xs[1], xs[2]) - EDGE_HALF_WIDTH,
            js_min3(ys[0], ys[1], ys[2]) - EDGE_HALF_WIDTH,
            js_max3(xs[0], xs[1], xs[2]) + EDGE_HALF_WIDTH,
            js_max3(ys[0], ys[1], ys[2]) + EDGE_HALF_WIDTH,
        );
        if let Some(points) = geometry.line_points {
            for (x, y) in points {
                bounds.include(
                    x - EDGE_HALF_WIDTH,
                    y - EDGE_HALF_WIDTH,
                    x + EDGE_HALF_WIDTH,
                    y + EDGE_HALF_WIDTH,
                );
            }
        }
        if let Some(label_width) = edge.label_width {
            let (x, y) = geometry.middle;
            bounds.include_box(label_box(x, y, label_width, EDGE_FONT_SIZE, true));
        }
        bounds.settled()
    }

    /// `eles.boundingBox()` of every element: the nodes and the edges, with their default boxes, made
    /// finite (`boundingBox` ends with `noninf`). Fcose takes its component centre from it.
    #[must_use]
    pub fn elements_box(&self) -> Box2 {
        let mut bounds = Box2::EMPTY;
        for index in 0..self.nodes.len() {
            bounds.include_box(self.node_box(index));
        }
        for index in 0..self.edges.len() {
            bounds.include_box(self.edge_box(index));
        }
        bounds.finite()
    }

    /// The `[x, y]` the layout writes to a leaf, after a run: the positions of the leaves are the
    /// output of the layout, and a group with children keeps the position its children give it.
    pub fn set_leaf_positions(&mut self, positions: &[Point]) {
        for (node, &position) in self.nodes.iter_mut().zip(positions) {
            if node.children.is_empty() {
                node.position = position;
            }
        }
        self.update_compounds();
    }
}

/// The side offset of an endpoint as the fraction `pfValue` holds (a percentage divided by 100, or
/// the number of pixels).
fn endpoint_fraction(dir: Direction) -> Point {
    let ((fx, _), (fy, _)) = side_offset(dir);
    (fx, fy)
}

/// Whether two sides make an XY edge (`isArchitectureDirectionXY`).
#[must_use]
pub fn is_xy(a: Direction, b: Direction) -> bool {
    a.is_horizontal() != b.is_horizontal()
}

/// `Math.ceil`, as cytoscape rounds a label's canvas width.
fn ceil(width: f64) -> f64 {
    width.ceil()
}

/// The width of a text in the label face at `size` pixels, as the canvas measures it.
///
/// # Errors
///
/// Returns an error if the bundled font cannot be read or shaped.
pub fn canvas_width(text: &str, size: f64) -> Result<f64, RenderError> {
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);
    Ok(measure(sans, text, size)?)
}
