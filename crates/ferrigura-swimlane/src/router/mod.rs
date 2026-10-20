//! Router: `routeEdgesOrthogonal` and its helpers (`orthogonalRouter/router.ts`, lines 122-2283).
//!
//! Each edge is routed as an axis-aligned polyline between the sides of its two nodes. The reference keeps the
//! state of the function in closures; here it is the struct [`Router`], and each part of the function is a
//! method in a submodule, named after the step it ports:
//!
//! - `setup`: the lanes (165-176), the obstacles (177-213), the pipes (215-238), and the port of a side and
//!   the side choice (82-106, 243-269);
//! - `ports`: the port assignment, step 6 (397-735);
//! - `initial`: the routing order (332-368) and the routing of each edge (735-1612): the anchors, the straight
//!   fast path (937-1061), the pipes of the anchors and the search;
//! - `search`: the crossing penalty (278-327), the L-shaped candidates and the A* search (1152-1324), and the
//!   simplification of a long path (1329-1493);
//! - `crossing`: the crossing reduction, phase 2 (1618-2004);
//! - `rebuild`: the geometry rebuild, phase 3 (2006-2205), and the write-back to the layout (2210-2282);
//!
//! The port keeps the operand order and the order of accumulation of the reference, uses `js_max` and `js_min`
//! for `Math.max` and `Math.min`, `ferrigura_jsmath::hypot` for `Math.hypot`, and the key `to_fixed(x, 1)` for
//! `toFixed(1)`. Maps keep their insertion order (`indexmap`).

mod crossing;
mod initial;
mod ports;
mod rebuild;
mod search;
mod setup;

use std::cell::RefCell;
use std::collections::HashMap;

use indexmap::{IndexMap, IndexSet};

use crate::error::Error;
use crate::model::{Direction, LayoutData, Node, Point};

/// `PRECISION.EPSILON` (`config.ts:11`).
const EPS: f64 = 1e-6;
/// `NODE_PADDING` (`router.ts:19`): the margin of an obstacle around its node.
const NODE_PADDING: f64 = 8.0;
/// `HORIZONTAL_PIPE_MARGIN` (`router.ts:20`).
const HORIZONTAL_PIPE_MARGIN: f64 = 15.0;
/// `VERTICAL_PIPE_MARGIN` (`router.ts:21`).
const VERTICAL_PIPE_MARGIN: f64 = 15.0;
/// `ROUTING_MARGIN` (`router.ts:22`): the room the search gets around an obstacle that blocks the direct path.
const ROUTING_MARGIN: f64 = 25.0;
/// `ANCHOR_OFFSET` (`router.ts:23`): the distance of an anchor from its port.
const ANCHOR_OFFSET: f64 = 20.0;
/// `TRACK_SPACING` (`router.ts:24`): the distance between two tracks of a pipe.
const TRACK_SPACING: f64 = 10.0;
/// `CROSSING_PENALTY` (`router.ts:276`): the cost of a step that crosses a routed segment.
const CROSSING_PENALTY: f64 = 1000.0;
/// `MIN_PORT_SPACING` (`router.ts:635`).
const MIN_PORT_SPACING: f64 = 8.0;
/// `MAX_PORT_SPACING` (`router.ts:662`).
const MAX_PORT_SPACING: f64 = 20.0;

/// `Math.sign(x)`: `NaN` for `NaN`, and `x` itself (`-0` or `+0`) for a zero.
fn js_sign(x: f64) -> f64 {
    if x.is_nan() {
        f64::NAN
    } else if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// A side of a node that a port sits on. `OrthogonalSide` and `SideT` of the reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    /// `top` or `bottom`: the ports of the side are offset along x.
    pub(super) fn is_vertical(self) -> bool {
        matches!(self, Side::Top | Side::Bottom)
    }
}

/// Whether a segment or a pipe runs along x (`vertical`, a constant x) or along y.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Orientation {
    Horizontal,
    Vertical,
}

/// The role of an edge at a node: `src` (the edge leaves the node) or `dst` (it arrives).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Role {
    Src,
    Dst,
}

/// A node as the router reads it through `nodeById`: `x` and `y` with the default `?? 0` applied, the sizes as
/// the reference keeps them (each use gives its own default), and whether the shape is a diamond.
#[derive(Clone, Copy, Debug)]
pub(super) struct Geo {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) width: Option<f64>,
    pub(super) height: Option<f64>,
    pub(super) diamond: bool,
}

impl Geo {
    fn of(node: &Node) -> Geo {
        Geo {
            x: node.x.unwrap_or(0.0),
            y: node.y.unwrap_or(0.0),
            width: node.width,
            height: node.height,
            diamond: matches!(node.shape.as_deref(), Some("question" | "diamond")),
        }
    }
}

/// An edge of the routing view (`InternalRoutingEdge`): a copy of an edge of the layout that is not layout-only,
/// with the fields the router reads. An empty id is `None`, as the reference's truthiness tests treat it.
#[derive(Clone, Debug)]
pub(super) struct RouteEdge {
    pub(super) start: Option<String>,
    pub(super) end: Option<String>,
    pub(super) label_node_id: Option<String>,
    /// `points`: set by the fast path or by the rebuild, and copied back to the layout at the end.
    pub(super) points: Option<Vec<Point>>,
    /// The index of the edge in `LayoutData::edges`, which the routed points are written to.
    pub(super) origin: usize,
}

/// An obstacle (`ObstacleRect`): the inflated box of a node that edges route around, with the index of the node.
#[derive(Clone, Copy, Debug)]
pub(super) struct Obstacle {
    pub(super) node: usize,
    pub(super) min_x: f64,
    pub(super) min_y: f64,
    pub(super) max_x: f64,
    pub(super) max_y: f64,
    /// `visualXHalfExtent`: the half extent along x of the visual shape, which is the height in LR.
    pub(super) visual_x_half_extent: f64,
}

/// A pipe (`Pipe`): a line that segments run along, at a constant `coord`, with the tracks that share it.
#[derive(Clone, Debug)]
pub(super) struct Pipe {
    pub(super) orientation: Orientation,
    pub(super) coord: f64,
    /// `tracks`: the segments on each track. A track's index is its position in this list.
    pub(super) tracks: Vec<Vec<SegmentRef>>,
}

/// A segment as a track holds it (`SegmentRef`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SegmentRef {
    pub(super) edge: usize,
    pub(super) segment: usize,
    pub(super) from: f64,
    pub(super) to: f64,
}

/// A routed segment (`RoutedSegment`), in the global list of the router.
#[derive(Clone, Copy, Debug)]
pub(super) struct RoutedSegment {
    pub(super) edge: usize,
    /// `segmentIndex`: the position of the segment in the route of its edge.
    pub(super) segment: usize,
    pub(super) orientation: Orientation,
    /// The pipe of the segment, or `None` for the straight fast path, whose pipe is not in the list of pipes.
    pub(super) pipe: Option<usize>,
    /// `pipe.coord` of the pipe the segment runs on. The coordinate of a pipe never changes, so it is copied.
    pub(super) pipe_coord: f64,
    /// `trackIndex`: the track of the pipe that holds the segment.
    pub(super) track: usize,
    pub(super) from: f64,
    pub(super) to: f64,
}

/// The destination of the first change of direction of an edge (`DestInfo`), cached by edge.
#[derive(Clone, Copy, Debug)]
pub(super) struct DestInfo {
    pub(super) dest: f64,
    pub(super) deviation: f64,
    pub(super) delta: f64,
}

/// The side that an edge leaves or enters a node by (`EdgeSideInfo`), keyed by the edge's index.
#[derive(Clone, Debug)]
pub(super) struct SideInfo {
    pub(super) src_id: String,
    pub(super) dst_id: String,
    pub(super) src_side: Side,
    pub(super) dst_side: Side,
    pub(super) abs_dx: f64,
    pub(super) abs_dy: f64,
    pub(super) dx_sign: f64,
    pub(super) dy_sign: f64,
}

/// An edge in a port group, with the coordinate of the other node that orders the group.
#[derive(Clone, Copy, Debug)]
pub(super) struct PortEntry {
    pub(super) edge: usize,
    pub(super) opposite_coord: f64,
}

/// The key of a port group: the node, its side and the role of the edge there (`nodeId:side:role`).
pub(super) type PortKey = (String, Side, Role);

/// The state of `routeEdgesOrthogonal`: the closures' variables of the reference.
pub(super) struct Router {
    /// `nodes`: a copy of the nodes of the layout, which the routing reads and does not change.
    pub(super) nodes: Vec<Node>,
    /// `edges`: the routing view (lines 136-153).
    pub(super) edges: Vec<RouteEdge>,
    /// `nodeById`: the index of the node with each id. A repeated id keeps its first position, with the last node.
    pub(super) node_by_id: IndexMap<String, usize>,
    /// `laneByNodeId`: the top-level group that holds each node.
    pub(super) lane_by_node_id: IndexMap<String, String>,
    /// `isLR`: the direction of the diagram is `LR`.
    pub(super) is_lr: bool,
    pub(super) obstacles: Vec<Obstacle>,
    pub(super) pipes: Vec<Pipe>,
    /// `allRoutedSegments`: every segment routed so far, including the fast-path segments.
    pub(super) segments: Vec<RoutedSegment>,
    /// `edgeSegmentIndices`: for each edge, the indices in `segments` of its segments, in order.
    pub(super) edge_segments: Vec<Vec<usize>>,
    /// `straightIntraLaneEdges`: the edges on the straight fast path.
    pub(super) straight_intra_lane_edges: IndexSet<usize>,
    /// `sideInfoByIdx`: the side of each edge that the routing can place (lines 436-462).
    pub(super) side_info: IndexMap<usize, SideInfo>,
    /// `incidentEdgeTotals`: the number of edges at each node, self-loops and invalid edges excluded.
    pub(super) incident_totals: HashMap<String, usize>,
    /// `portGroups`: the edges at each side of each node, in the order they were added.
    pub(super) port_groups: IndexMap<PortKey, Vec<PortEntry>>,
    /// `portOffsets`: the offset of the port of an edge along its side, by edge and role.
    pub(super) port_offsets: HashMap<(usize, Role), f64>,
    /// `destInfoCache`: the destinations of the edges, computed on first use and kept.
    pub(super) dest_info_cache: RefCell<HashMap<usize, DestInfo>>,
}

impl Router {
    /// The router of a layout: the nodes, the routing view of the edges, and the node lookup (lines 145-163).
    fn new(data: &LayoutData, is_lr: bool) -> Router {
        let nodes = data.nodes.clone();
        let mut node_by_id = IndexMap::new();
        for (index, node) in nodes.iter().enumerate() {
            node_by_id.insert(node.id.clone(), index);
        }
        let edges: Vec<RouteEdge> = data
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.is_layout_only != Some(true))
            .map(|(origin, edge)| RouteEdge {
                start: non_empty(&edge.start),
                end: non_empty(&edge.end),
                label_node_id: non_empty(&edge.label_node_id),
                points: edge.points.clone(),
                origin,
            })
            .collect();
        let edge_count = edges.len();
        Router {
            nodes,
            edges,
            node_by_id,
            lane_by_node_id: IndexMap::new(),
            is_lr,
            obstacles: Vec::new(),
            pipes: Vec::new(),
            segments: Vec::new(),
            edge_segments: vec![Vec::new(); edge_count],
            straight_intra_lane_edges: IndexSet::new(),
            side_info: IndexMap::new(),
            incident_totals: HashMap::new(),
            port_groups: IndexMap::new(),
            port_offsets: HashMap::new(),
            dest_info_cache: RefCell::new(HashMap::new()),
        }
    }

    /// The node with this id, as the routing reads it, or `None` when there is no such node.
    fn geo(&self, id: &str) -> Option<Geo> {
        let index = *self.node_by_id.get(id)?;
        Some(Geo::of(&self.nodes[index]))
    }
}

/// The string of a truthy id: `None` for an absent or an empty string.
fn non_empty(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// `routeEdgesOrthogonal(data, direction)`: routes the edges of the layout and writes their points.
///
/// The steps run in the order of the reference: the lanes, the obstacles, the routing order, the ports, the
/// routing of each edge, the crossing reduction, the rebuild of the geometry, and the write-back.
pub fn route_edges_orthogonal(data: &mut LayoutData, direction: Direction) -> Result<(), Error> {
    let mut router = Router::new(data, direction == Direction::LR);
    router.assign_lanes();
    router.build_obstacles();
    let order = router.routing_order();
    router.assign_ports();
    for edge in order {
        router.route_edge(edge);
    }
    router.reduce_crossings();
    router.rebuild_geometry();
    router.write_back(data);
    Ok(())
}
