//! The layout data of the swimlane core: the contract between the phases ported here and the router and
//! the geometry passes that follow them.
//!
//! The types mirror Mermaid's `LayoutData` (`rendering-util/types.ts`) for the properties that the core and
//! its passes read or write. The other own properties of each object are kept, unread, in `extra`, in their
//! order, so that a comparison with the reference sees every key.
//!
//! # Undefined and absent
//!
//! A JavaScript property that is `undefined`, because it was never set or was removed with `delete`, is
//! `None` in a typed field, and its key is absent from the JSON. JSON `null` has no `undefined` counterpart:
//! a key whose value does not have the typed shape (`null`, or a string where a number belongs) is kept in
//! `extra` as it was read, and its typed field stays `None`. A typed field that is `Some` replaces an `extra`
//! entry with the same key when the object is written back.
//!
//! # Numbers
//!
//! Every JavaScript number is an `f64`. `layer` and `order` are integers in the reference, and they are `f64`
//! here too, so that a value written by the port compares with a value read from JSON in the same way. As
//! `JSON.stringify` does, a `NaN` or an infinity is written as `null`.

use indexmap::IndexMap;
use serde_json::{Map, Value};

/// The own properties of an object that the port does not model, in their order.
pub type Extra = IndexMap<String, Value>;

/// The direction of the diagram, as `flowDb.ts` normalises it: `TB`, `LR`, `BT` or `RL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    TB,
    LR,
    BT,
    RL,
}

impl Direction {
    /// The direction named by `text`, or `None` for any other string.
    pub fn parse(text: &str) -> Option<Direction> {
        match text {
            "TB" => Some(Direction::TB),
            "LR" => Some(Direction::LR),
            "BT" => Some(Direction::BT),
            "RL" => Some(Direction::RL),
            _ => None,
        }
    }

    /// The name of the direction, as the layout data spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::TB => "TB",
            Direction::LR => "LR",
            Direction::BT => "BT",
            Direction::RL => "RL",
        }
    }
}

/// A rectangle, as `groupTitleRect` holds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

/// A point of an edge path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// A node of the layout: a lane (`is_group`), a nested subgraph, a node, a label node or a dummy.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
    /// `id`.
    pub id: String,
    /// `parentId`: the id of the enclosing group.
    pub parent_id: Option<String>,
    /// `isGroup`. A lane or a subgraph is a group.
    pub is_group: Option<bool>,
    /// `shape`: `swimlane` for a top-level group.
    pub shape: Option<String>,
    /// `isDummy`.
    pub is_dummy: Option<bool>,
    /// `isEdgeLabel`: the node holds the label of an edge.
    pub is_edge_label: Option<bool>,
    /// `width`.
    pub width: Option<f64>,
    /// `height`.
    pub height: Option<f64>,
    /// `x`: the centre of the node.
    pub x: Option<f64>,
    /// `y`: the centre of the node.
    pub y: Option<f64>,
    /// `layer`: the rank of the node, written back by the core.
    pub layer: Option<f64>,
    /// `order`: the position of the node in its layer, written back by the core.
    pub order: Option<f64>,
    /// `padding`.
    pub padding: Option<f64>,
    /// `swimlaneContentTop`: the top of the content of the lanes, on the top-level groups.
    pub swimlane_content_top: Option<f64>,
    /// `groupTitleRect`: the title band of a top-level group.
    pub group_title_rect: Option<Rect>,
    /// `look`.
    pub look: Option<String>,
    /// `colorIndex`: the palette slot of a lane.
    pub color_index: Option<f64>,
    /// `dir`: the direction of the node, which an edge label node copies from its source.
    pub dir: Option<String>,
    /// The other own properties, unread by the port.
    pub extra: Extra,
}

/// An edge of the layout.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Edge {
    /// `id`.
    pub id: String,
    /// `start`: the id of the source node.
    pub start: Option<String>,
    /// `end`: the id of the target node.
    pub end: Option<String>,
    /// `label`: the text of the edge. Cleared when the label moves to a label node.
    pub label: Option<String>,
    /// `labelNodeId`: the label node of a labelled edge.
    pub label_node_id: Option<String>,
    /// `isLayoutOnly`: a virtual edge that is laid out and never routed or drawn.
    pub is_layout_only: Option<bool>,
    /// `curve`.
    pub curve: Option<String>,
    /// `points`: the routed path. Deleted by the core before routing.
    pub points: Option<Vec<Point>>,
    /// `arrowTypeStart`.
    pub arrow_type_start: Option<String>,
    /// `arrowTypeEnd`.
    pub arrow_type_end: Option<String>,
    /// The other own properties, unread by the port.
    pub extra: Extra,
}

/// The `flowchart` section of the configuration, for the keys the core reads.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlowchartConfig {
    /// `nodeSpacing`: the horizontal gap between nodes of a lane.
    pub node_spacing: Option<f64>,
    /// `rankSpacing`: the vertical gap between layers.
    pub rank_spacing: Option<f64>,
    /// The other keys of the section.
    pub extra: Extra,
}

/// The `swimlane` section of the configuration, for the keys the core reads.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SwimlaneConfig {
    /// `ignoreCrossLaneEdges`. The port runs the default, `true`.
    pub ignore_cross_lane_edges: Option<bool>,
    /// `optimizeRanksByCrossings`. Read only by the non-default gravity layering.
    pub optimize_ranks_by_crossings: Option<bool>,
    /// `automaticLaneOrdering`. The port runs the default, `false`.
    pub automatic_lane_ordering: Option<bool>,
    /// The other keys of the section.
    pub extra: Extra,
}

/// The configuration of the diagram, for the sections and keys the core reads.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    /// `flowchart`. Absent when the key is absent or not an object.
    pub flowchart: Option<FlowchartConfig>,
    /// `swimlane`. Absent when the key is absent or not an object.
    pub swimlane: Option<SwimlaneConfig>,
    /// The other keys of the configuration.
    pub extra: Extra,
}

/// The layout data that the swimlane core reads and writes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutData {
    /// `nodes`, in the order of the diagram.
    pub nodes: Vec<Node>,
    /// `edges`, in the order of the diagram.
    pub edges: Vec<Edge>,
    /// `direction`, as the diagram gives it. The core reads an absent direction as `TB`.
    pub direction: Option<String>,
    /// `config`.
    pub config: Config,
    /// The other top-level properties (`diagramId`, `markers`, ...), unread by the port.
    pub extra: Extra,
}

// JSON conversion. The corpus tests read a snapshot into the model and write the result back, so the
// conversions are exposed only to the tests, through `crate::testing`.

fn object<'a>(value: &'a Value, what: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{what} is not a JSON object"))
}

/// Reads the keys of `map` in order. `typed` gets each key and its value; it stores the value in a typed
/// field and returns `true` when the value has the typed shape. Every other key goes to the returned extra.
fn read_extra(map: &Map<String, Value>, mut typed: impl FnMut(&str, &Value) -> bool) -> Extra {
    let mut extra = Extra::new();
    for (key, value) in map {
        if !typed(key, value) {
            extra.insert(key.clone(), value.clone());
        }
    }
    extra
}

fn number(value: &Value) -> Option<f64> {
    value.as_f64()
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn flag(value: &Value) -> Option<bool> {
    value.as_bool()
}

/// Stores `value` in `slot` when it is present, and reports whether it was stored.
fn set<T>(slot: &mut Option<T>, value: Option<T>) -> bool {
    match value {
        Some(value) => {
            *slot = Some(value);
            true
        }
        None => false,
    }
}

fn point_from_json(value: &Value) -> Option<Point> {
    let map = value.as_object()?;
    if map.len() != 2 {
        return None;
    }
    Some(Point {
        x: number(map.get("x")?)?,
        y: number(map.get("y")?)?,
    })
}

fn rect_from_json(value: &Value) -> Option<Rect> {
    let map = value.as_object()?;
    if map.len() != 4 {
        return None;
    }
    Some(Rect {
        left: number(map.get("left")?)?,
        right: number(map.get("right")?)?,
        top: number(map.get("top")?)?,
        bottom: number(map.get("bottom")?)?,
    })
}

/// A JSON number. `NaN` and the infinities have no JSON form, and they are written as `null`, as
/// `JSON.stringify` writes them.
fn num(value: f64) -> Value {
    // `JSON.stringify` writes an integral number without a fraction, and `-0` as `0`.
    if value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0 {
        return Value::from(value as i64);
    }
    serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
}

fn text_value(value: &Option<String>) -> Option<Value> {
    value.clone().map(Value::String)
}

/// Builds an object from its typed entries, which are absent when `None`, and then the extra entries that
/// no typed entry replaces.
fn json_object(typed: Vec<(&'static str, Option<Value>)>, extra: &Extra) -> Map<String, Value> {
    let mut map = Map::new();
    for (key, value) in typed {
        if let Some(value) = value {
            map.insert(key.to_owned(), value);
        }
    }
    for (key, value) in extra {
        if !map.contains_key(key) {
            map.insert(key.clone(), value.clone());
        }
    }
    map
}

fn rect_value(rect: &Rect) -> Value {
    Value::Object(
        [
            ("left", num(rect.left)),
            ("right", num(rect.right)),
            ("top", num(rect.top)),
            ("bottom", num(rect.bottom)),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect(),
    )
}

fn point_value(point: &Point) -> Value {
    Value::Object(
        [("x", num(point.x)), ("y", num(point.y))]
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

impl Node {
    fn from_json(value: &Value) -> Result<Node, String> {
        let map = object(value, "a node")?;
        let id = map
            .get("id")
            .and_then(Value::as_str)
            .ok_or("a node has no string id")?;
        let mut node = Node {
            id: id.to_owned(),
            ..Node::default()
        };
        let extra = read_extra(map, |key, value| match key {
            "id" => true,
            "parentId" => set(&mut node.parent_id, text(value)),
            "isGroup" => set(&mut node.is_group, flag(value)),
            "shape" => set(&mut node.shape, text(value)),
            "isDummy" => set(&mut node.is_dummy, flag(value)),
            "isEdgeLabel" => set(&mut node.is_edge_label, flag(value)),
            "width" => set(&mut node.width, number(value)),
            "height" => set(&mut node.height, number(value)),
            "x" => set(&mut node.x, number(value)),
            "y" => set(&mut node.y, number(value)),
            "layer" => set(&mut node.layer, number(value)),
            "order" => set(&mut node.order, number(value)),
            "padding" => set(&mut node.padding, number(value)),
            "swimlaneContentTop" => set(&mut node.swimlane_content_top, number(value)),
            "groupTitleRect" => match rect_from_json(value) {
                Some(rect) => set(&mut node.group_title_rect, Some(rect)),
                None => false,
            },
            "look" => set(&mut node.look, text(value)),
            "colorIndex" => set(&mut node.color_index, number(value)),
            "dir" => set(&mut node.dir, text(value)),
            _ => false,
        });
        node.extra = extra;
        Ok(node)
    }

    fn to_json(&self) -> Value {
        Value::Object(json_object(
            vec![
                ("id", Some(Value::String(self.id.clone()))),
                ("parentId", text_value(&self.parent_id)),
                ("isGroup", self.is_group.map(Value::Bool)),
                ("shape", text_value(&self.shape)),
                ("isDummy", self.is_dummy.map(Value::Bool)),
                ("isEdgeLabel", self.is_edge_label.map(Value::Bool)),
                ("width", self.width.map(num)),
                ("height", self.height.map(num)),
                ("x", self.x.map(num)),
                ("y", self.y.map(num)),
                ("layer", self.layer.map(num)),
                ("order", self.order.map(num)),
                ("padding", self.padding.map(num)),
                ("swimlaneContentTop", self.swimlane_content_top.map(num)),
                (
                    "groupTitleRect",
                    self.group_title_rect.as_ref().map(rect_value),
                ),
                ("look", text_value(&self.look)),
                ("colorIndex", self.color_index.map(num)),
                ("dir", text_value(&self.dir)),
            ],
            &self.extra,
        ))
    }
}

impl Edge {
    fn from_json(value: &Value) -> Result<Edge, String> {
        let map = object(value, "an edge")?;
        let id = map
            .get("id")
            .and_then(Value::as_str)
            .ok_or("an edge has no string id")?;
        let mut edge = Edge {
            id: id.to_owned(),
            ..Edge::default()
        };
        let extra = read_extra(map, |key, value| match key {
            "id" => true,
            "start" => set(&mut edge.start, text(value)),
            "end" => set(&mut edge.end, text(value)),
            "label" => set(&mut edge.label, text(value)),
            "labelNodeId" => set(&mut edge.label_node_id, text(value)),
            "isLayoutOnly" => set(&mut edge.is_layout_only, flag(value)),
            "curve" => set(&mut edge.curve, text(value)),
            "points" => {
                let points = value.as_array().and_then(|items| {
                    items
                        .iter()
                        .map(point_from_json)
                        .collect::<Option<Vec<_>>>()
                });
                set(&mut edge.points, points)
            }
            "arrowTypeStart" => set(&mut edge.arrow_type_start, text(value)),
            "arrowTypeEnd" => set(&mut edge.arrow_type_end, text(value)),
            _ => false,
        });
        edge.extra = extra;
        Ok(edge)
    }

    fn to_json(&self) -> Value {
        Value::Object(json_object(
            vec![
                ("id", Some(Value::String(self.id.clone()))),
                ("start", text_value(&self.start)),
                ("end", text_value(&self.end)),
                ("label", text_value(&self.label)),
                ("labelNodeId", text_value(&self.label_node_id)),
                ("isLayoutOnly", self.is_layout_only.map(Value::Bool)),
                ("curve", text_value(&self.curve)),
                (
                    "points",
                    self.points
                        .as_ref()
                        .map(|points| Value::Array(points.iter().map(point_value).collect())),
                ),
                ("arrowTypeStart", text_value(&self.arrow_type_start)),
                ("arrowTypeEnd", text_value(&self.arrow_type_end)),
            ],
            &self.extra,
        ))
    }
}

impl FlowchartConfig {
    fn from_json(map: &Map<String, Value>) -> FlowchartConfig {
        let mut section = FlowchartConfig::default();
        section.extra = read_extra(map, |key, value| match key {
            "nodeSpacing" => set(&mut section.node_spacing, number(value)),
            "rankSpacing" => set(&mut section.rank_spacing, number(value)),
            _ => false,
        });
        section
    }

    fn to_json(&self) -> Value {
        Value::Object(json_object(
            vec![
                ("nodeSpacing", self.node_spacing.map(num)),
                ("rankSpacing", self.rank_spacing.map(num)),
            ],
            &self.extra,
        ))
    }
}

impl SwimlaneConfig {
    fn from_json(map: &Map<String, Value>) -> SwimlaneConfig {
        let mut section = SwimlaneConfig::default();
        section.extra = read_extra(map, |key, value| match key {
            "ignoreCrossLaneEdges" => set(&mut section.ignore_cross_lane_edges, flag(value)),
            "optimizeRanksByCrossings" => {
                set(&mut section.optimize_ranks_by_crossings, flag(value))
            }
            "automaticLaneOrdering" => set(&mut section.automatic_lane_ordering, flag(value)),
            _ => false,
        });
        section
    }

    fn to_json(&self) -> Value {
        Value::Object(json_object(
            vec![
                (
                    "ignoreCrossLaneEdges",
                    self.ignore_cross_lane_edges.map(Value::Bool),
                ),
                (
                    "optimizeRanksByCrossings",
                    self.optimize_ranks_by_crossings.map(Value::Bool),
                ),
                (
                    "automaticLaneOrdering",
                    self.automatic_lane_ordering.map(Value::Bool),
                ),
            ],
            &self.extra,
        ))
    }
}

impl Config {
    fn from_json(value: &Value) -> Result<Config, String> {
        let map = object(value, "the configuration")?;
        let mut config = Config::default();
        config.extra = read_extra(map, |key, value| match key {
            "flowchart" => match value.as_object() {
                Some(section) => {
                    config.flowchart = Some(FlowchartConfig::from_json(section));
                    true
                }
                None => false,
            },
            "swimlane" => match value.as_object() {
                Some(section) => {
                    config.swimlane = Some(SwimlaneConfig::from_json(section));
                    true
                }
                None => false,
            },
            _ => false,
        });
        Ok(config)
    }

    fn to_json(&self) -> Value {
        Value::Object(json_object(
            vec![
                (
                    "flowchart",
                    self.flowchart.as_ref().map(FlowchartConfig::to_json),
                ),
                (
                    "swimlane",
                    self.swimlane.as_ref().map(SwimlaneConfig::to_json),
                ),
            ],
            &self.extra,
        ))
    }
}

/// Reads a layout data object from its JSON, as the corpus snapshots hold it: `direction`, `config`,
/// `nodes` and `edges`, and the other top-level keys in `extra`.
pub fn layout_data_from_json(value: &Value) -> Result<LayoutData, String> {
    let map = object(value, "the layout data")?;
    let list = |key: &str| -> Result<Vec<&Value>, String> {
        match map.get(key) {
            None => Ok(Vec::new()),
            Some(Value::Array(items)) => Ok(items.iter().collect()),
            Some(_) => Err(format!("{key} is not a JSON array")),
        }
    };
    let nodes = list("nodes")?
        .into_iter()
        .map(Node::from_json)
        .collect::<Result<Vec<_>, _>>()?;
    let edges = list("edges")?
        .into_iter()
        .map(Edge::from_json)
        .collect::<Result<Vec<_>, _>>()?;
    let config = match map.get("config") {
        Some(value) => Config::from_json(value)?,
        None => Config::default(),
    };
    let direction = map.get("direction").and_then(text);
    let extra = read_extra(map, |key, _| match key {
        "nodes" | "edges" | "config" => true,
        "direction" => direction.is_some(),
        _ => false,
    });
    Ok(LayoutData {
        nodes,
        edges,
        direction,
        config,
        extra,
    })
}

/// Writes a layout data object as JSON, in the shape `layout_data_from_json` reads.
pub fn layout_data_to_json(data: &LayoutData) -> Value {
    let mut map = json_object(
        vec![
            ("direction", text_value(&data.direction)),
            ("config", Some(data.config.to_json())),
            (
                "nodes",
                Some(Value::Array(data.nodes.iter().map(Node::to_json).collect())),
            ),
            (
                "edges",
                Some(Value::Array(data.edges.iter().map(Edge::to_json).collect())),
            ),
        ],
        &data.extra,
    );
    map.sort_keys();
    Value::Object(map)
}
