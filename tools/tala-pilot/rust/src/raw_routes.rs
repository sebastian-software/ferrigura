use std::error::Error;

use ferrigura_tala::ovg_seed::flat_search::{
    Appearance, FlatSearch, Flavor, OrdinaryEdge, Rectangle, RouteStrategy, Weights,
};
use ferrigura_tala::ovg_seed::rectangular_linked_visibility;
use ferrigura_tala::{Edge, EdgeGeometry, Graph, Layout, Node, NodeGeometry, Point};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawRouteInput {
    nodes: Vec<NodeGeometry>,
    node_ids: Vec<String>,
    edges: Vec<PilotRouteEdge>,
    labels: Vec<Option<PilotLabelBox>>,
}

#[derive(Deserialize)]
struct PilotRouteEdge {
    owners: [usize; 2],
    arrowheads: [String; 2],
    appearance: PilotAppearance,
    invisible: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PilotAppearance {
    opacity: Option<String>,
    stroke: Option<String>,
    stroke_width: Option<String>,
    stroke_dash: Option<String>,
    animated: Option<String>,
}

#[derive(Deserialize)]
struct PilotLabelBox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Serialize)]
pub(super) struct RawRoutingOutput {
    flavors: Vec<RawFlavor>,
    selected: SelectedFlavor,
    before_balance: Vec<SelectedPath>,
    balanced: Option<Vec<SelectedPath>>,
}

#[derive(Serialize)]
struct RawFlavor {
    flavor: &'static str,
    cost: f64,
    routes: Vec<RawPath>,
}

#[derive(Serialize)]
struct RawPath {
    edge: usize,
    strategy: &'static str,
    cost: f64,
    points: Vec<[f64; 2]>,
}

#[derive(Serialize)]
struct SelectedFlavor {
    flavor: &'static str,
    routes: Vec<SelectedPath>,
}

#[derive(Serialize)]
struct SelectedPath {
    edge: usize,
    points: Vec<[f64; 2]>,
}

pub(super) fn trace_raw_routes(
    capture: RawRouteInput,
    include_balance: bool,
) -> Result<RawRoutingOutput, Box<dyn Error>> {
    let RawRouteInput {
        nodes,
        node_ids,
        edges,
        labels,
    } = capture;
    let node_ids = node_ids
        .iter()
        .map(|id| id.parse())
        .collect::<Result<Vec<i64>, _>>()?;
    let edges = edges
        .into_iter()
        .map(|edge| OrdinaryEdge {
            owners: edge.owners,
            arrowheads: edge.arrowheads,
            appearance: Appearance {
                opacity: edge.appearance.opacity,
                stroke: edge.appearance.stroke,
                stroke_width: edge.appearance.stroke_width,
                stroke_dash: edge.appearance.stroke_dash,
                animated: edge.appearance.animated,
            },
            invisible: edge.invisible,
        })
        .collect::<Vec<_>>();
    let labels = labels
        .into_iter()
        .map(|label| {
            label.map(|value| Rectangle {
                x: value.x,
                y: value.y,
                width: value.width,
                height: value.height,
            })
        })
        .collect::<Vec<_>>();
    let pairs = edges.iter().map(|edge| edge.owners).collect::<Vec<_>>();
    let graph = rectangular_linked_visibility(&nodes, &pairs)?;
    let weights = Weights::from_graph(&nodes, &pairs)?;
    let search = FlatSearch::new(&graph, &nodes, &labels, weights)?;
    let point = |point: Point| [point.x, point.y];
    let mut work = usize::MAX;
    let mut flavors = Vec::new();
    for flavor in [
        Flavor::ShortestToLongest,
        Flavor::LongestToShortest,
        Flavor::Default,
    ] {
        let routed = search
            .route_flavor(flavor, &node_ids, &edges, &mut work)?
            .ok_or("ordinary native routing did not resolve the requested flavor")?;
        let routes = routed
            .routes
            .iter()
            .map(|route| {
                Ok(RawPath {
                    edge: route.edge,
                    strategy: match route.strategy {
                        RouteStrategy::Slingshot => "slingshot",
                        RouteStrategy::Quick | RouteStrategy::Weighted => "search",
                        RouteStrategy::Fallback => "line",
                    },
                    cost: route.route.cost,
                    points: search
                        .raw_route_points(&routed, &route.route)?
                        .into_iter()
                        .map(point)
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, ferrigura_tala::Error>>()?;
        flavors.push(RawFlavor {
            flavor: flavor_name(flavor),
            cost: routed.cost,
            routes,
        });
    }
    let selected = search
        .select_flavor(&node_ids, &edges, &mut work)?
        .ok_or("ordinary native routing did not resolve a winning flavor")?;
    let raw_points = search.route_points_in_input_order(&selected)?;
    let before_balance = raw_points
        .iter()
        .enumerate()
        .map(|(edge, points)| SelectedPath {
            edge,
            points: points.iter().copied().map(point).collect(),
        })
        .collect();
    let routes = selected
        .routes
        .iter()
        .map(|route| SelectedPath {
            edge: route.edge,
            points: raw_points[route.edge].iter().copied().map(point).collect(),
        })
        .collect();
    let balanced = if include_balance {
        Some(balance_raw_routes(&nodes, &edges, raw_points)?)
    } else {
        None
    };
    Ok(RawRoutingOutput {
        flavors,
        selected: SelectedFlavor {
            flavor: flavor_name(selected.flavor),
            routes,
        },
        before_balance,
        balanced,
    })
}

fn balance_raw_routes(
    nodes: &[NodeGeometry],
    edges: &[OrdinaryEdge],
    raw_points: Vec<Vec<Point>>,
) -> Result<Vec<SelectedPath>, ferrigura_tala::Error> {
    let graph = Graph {
        direction: None,
        nodes: nodes
            .iter()
            .map(|node| Node {
                id: node.id.clone(),
                parent_id: None,
                direction: None,
                is_group: false,
                shape: String::from("rect"),
                width: node.width,
                height: node.height,
                label: String::new(),
                label_width: 0.0,
                label_height: 0.0,
            })
            .collect(),
        edges: edges
            .iter()
            .enumerate()
            .map(|(index, edge)| {
                let [source, target] = edge.owners.map(|owner| nodes[owner].id.clone());
                let [source_arrow, target_arrow] = edge
                    .arrowheads
                    .each_ref()
                    .map(|arrow| !arrow.is_empty() && arrow != "none");
                Edge {
                    id: index.to_string(),
                    source,
                    target,
                    source_arrow,
                    target_arrow,
                    label: String::new(),
                    label_width: 0.0,
                    label_height: 0.0,
                }
            })
            .collect(),
    };
    let mut layout = Layout {
        nodes: nodes.to_vec(),
        edges: raw_points
            .into_iter()
            .enumerate()
            .map(|(index, points)| EdgeGeometry {
                id: index.to_string(),
                points,
                label: None,
            })
            .collect(),
    };
    ferrigura_tala::balance::balance_routes(&graph, &mut layout)?;
    Ok(layout
        .edges
        .into_iter()
        .enumerate()
        .map(|(edge, route)| SelectedPath {
            edge,
            points: route
                .points
                .into_iter()
                .map(|point| [point.x, point.y])
                .collect(),
        })
        .collect())
}

fn flavor_name(flavor: Flavor) -> &'static str {
    match flavor {
        Flavor::ShortestToLongest => "ShortestToLongest",
        Flavor::LongestToShortest => "LongestToShortest",
        Flavor::Default => "Default",
        Flavor::TopDownLeftRight => "TopDownLeftRight",
    }
}
