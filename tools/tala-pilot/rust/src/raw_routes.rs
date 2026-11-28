use std::error::Error;

use ferrigura_tala::ovg_seed::flat_search::{
    Appearance, FlatSearch, Flavor, OrdinaryEdge, Rectangle, RouteStrategy, Weights,
};
use ferrigura_tala::ovg_seed::rectangular_linked_visibility;
use ferrigura_tala::{NodeGeometry, Point};
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

pub(super) fn trace_raw_routes(capture: RawRouteInput) -> Result<RawRoutingOutput, Box<dyn Error>> {
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
            .into_iter()
            .map(|route| RawPath {
                edge: route.edge,
                strategy: match route.strategy {
                    RouteStrategy::Slingshot => "slingshot",
                    RouteStrategy::Weighted => "search",
                },
                cost: route.route.cost,
                points: route
                    .route
                    .vertices
                    .into_iter()
                    .map(|index| point(graph.vertices[index].point))
                    .collect(),
            })
            .collect();
        flavors.push(RawFlavor {
            flavor: flavor_name(flavor),
            cost: routed.cost,
            routes,
        });
    }
    let selected = search
        .select_flavor(&node_ids, &edges, &mut work)?
        .ok_or("ordinary native routing did not resolve a winning flavor")?;
    let routes = selected
        .routes
        .into_iter()
        .map(|route| {
            Ok(SelectedPath {
                edge: route.edge,
                points: search
                    .route_points(&route.route)?
                    .into_iter()
                    .map(point)
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, ferrigura_tala::Error>>()?;
    Ok(RawRoutingOutput {
        flavors,
        selected: SelectedFlavor {
            flavor: flavor_name(selected.flavor),
            routes,
        },
    })
}

fn flavor_name(flavor: Flavor) -> &'static str {
    match flavor {
        Flavor::ShortestToLongest => "ShortestToLongest",
        Flavor::LongestToShortest => "LongestToShortest",
        Flavor::Default => "Default",
        Flavor::TopDownLeftRight => "TopDownLeftRight",
    }
}
