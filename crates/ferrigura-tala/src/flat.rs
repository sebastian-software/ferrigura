// SPDX-License-Identifier: MPL-2.0
// The native flat component uses D2 TALA's ordinary placement and routing stages.

use serde::Serialize;

use crate::layout_graph::LayoutGraph;
use crate::ovg_seed::flat_search::{Appearance, FlatSearch, OrdinaryEdge, Weights};
use crate::ovg_seed::rectangular_linked_visibility;
use crate::quality::{Box, EvaluationError, PositionedEdge, PositionedGraph, PositionedNode};
use crate::route_output::{LAYOUT_STROKE_WIDTH, RoutedEdge};
use crate::seed_selection::completed::{
    AuthoredEdge, AuthoredTopology, CompletedEdge, CompletedGraph, CompletedNode, Dimensions,
};
use crate::seed_selection::{AttemptError, LayoutScore, Scored, SeedPlan, SelectionError};
use crate::sizeless::measured::MeasuredComponent;
use crate::{EdgeGeometry, Error, Graph, Layout, NodeGeometry, Point};

const MAX_ROUTE_WORK: usize = 3 * 120_000_000;

/// Captures positioned nodes and balanced edges for a prepared flat seed.
///
/// # Errors
/// Rejects invalid graphs, unsupported topology, and exhausted layout work.
pub fn route_for_seed(graph: &Graph, seed: i64) -> Result<Layout, Error> {
    let component = MeasuredComponent::new(graph, LayoutGraph::translate(graph)?)?;
    route_component(&component, seed)
}

/// An evaluated seed and its authored layout.
#[derive(Serialize)]
pub struct FlatSeed {
    pub seed: i64,
    pub score: LayoutScore,
    pub work: u64,
    pub layout: Layout,
}

/// Runs one native seed through placement, routing, caption placement and scoring.
/// # Errors
/// Rejects invalid graphs, unsupported flat topology and exhausted layout work.
pub fn layout_for_seed(graph: &Graph, seed: i64) -> Result<FlatSeed, Error> {
    FlatWorkspace::new(graph)?.layout(seed)
}

/// Evaluates the default seeds and returns the selected candidate.
/// # Errors
/// Returns an input error or the failure reported by the unsuccessful seed attempts.
pub fn default_layout(graph: &Graph) -> Result<FlatSeed, Error> {
    let workspace = FlatWorkspace::new(graph)?;
    let plan = SeedPlan::new(None, None, 0).map_err(|_| Error::InvalidGraph)?;
    let selected = plan
        .coordinate(
            || false,
            |_, seed| {
                let candidate = workspace.layout(seed)?;
                Ok(Scored {
                    score: candidate.score,
                    candidate,
                })
            },
        )
        .map_err(|error| match error {
            SelectionError::Cancelled => Error::WorkLimit,
            SelectionError::AllFailed(failures) => failures
                .into_iter()
                .find_map(|failure| match failure.error {
                    AttemptError::Failed(error) => Some(error),
                    AttemptError::Panicked => None,
                })
                .unwrap_or(Error::InvalidGraph),
        })?;
    Ok(selected.result.candidate)
}

struct FlatWorkspace {
    component: MeasuredComponent,
    translated: LayoutGraph,
}

impl FlatWorkspace {
    fn new(graph: &Graph) -> Result<Self, Error> {
        let translated = LayoutGraph::translate(graph)?;
        let component = MeasuredComponent::new(graph, translated.clone())?;
        Ok(Self {
            component,
            translated,
        })
    }

    fn layout(&self, seed: i64) -> Result<FlatSeed, Error> {
        let routed = route_component(&self.component, seed)?;
        let mut nodes: Vec<_> = self
            .component
            .obstacle_order
            .iter()
            .map(|&index| routed.nodes[index].clone())
            .collect();
        let bodies: Vec<_> = nodes.iter().map(body_box).collect();
        let paths: Vec<_> = routed.edges.into_iter().map(|edge| edge.points).collect();
        let node_labels: Vec<_> = self
            .translated
            .nodes
            .iter()
            .map(|node| node.label.as_ref())
            .collect();
        let edge_labels: Vec<_> = self
            .translated
            .edges
            .iter()
            .map(|edge| edge.label.as_ref())
            .collect();
        let captions =
            crate::labeling::place_ordinary(&bodies, &node_labels, &paths, &edge_labels)?;
        let mut edges: Vec<_> = paths
            .into_iter()
            .zip(captions.edge_labels)
            .enumerate()
            .map(|(index, (points, label))| RoutedEdge {
                id: self.component.graph.edges[index].id.clone(),
                points,
                label,
                reversed_for_layout: self.translated.edges[index].reversed_for_layout,
            })
            .collect();
        crate::normalize::normalize_routed(&mut nodes, &mut edges)?;
        self.validate(&nodes, &edges)?;
        let evaluation =
            self.evaluate(&nodes, &edges, &captions.node_boxes, &captions.edge_boxes)?;
        let layout = Layout {
            nodes,
            edges: crate::route_output::project(edges)?,
        };
        Ok(FlatSeed {
            seed,
            score: evaluation.score,
            work: evaluation.work,
            layout,
        })
    }

    fn validate(&self, nodes: &[NodeGeometry], edges: &[RoutedEdge]) -> Result<(), Error> {
        if nodes.len() != self.translated.nodes.len()
            || edges.len() != self.translated.edges.len()
            || nodes
                .iter()
                .zip(&self.component.obstacle_order)
                .any(|(body, &index)| body.id != self.component.graph.nodes[index].id)
            || edges
                .iter()
                .zip(&self.component.graph.edges)
                .any(|(route, source)| route.id != source.id)
        {
            return Err(Error::InvalidGraph);
        }

        let node_ids: Vec<_> = self.translated.nodes.iter().map(|node| node.id).collect();
        let completed_nodes: Vec<_> = nodes
            .iter()
            .zip(&self.translated.nodes)
            .map(|(body, source)| CompletedNode {
                id: source.id,
                top_left: Some(Point {
                    x: body.x,
                    y: body.y,
                }),
                size: Dimensions {
                    width: body.width,
                    height: body.height,
                },
                label_size: source.label.as_ref().map(|caption| Dimensions {
                    width: caption.width,
                    height: caption.height,
                }),
            })
            .collect();
        let completed_edges: Vec<_> = edges
            .iter()
            .zip(&self.translated.edges)
            .map(|(route, source)| CompletedEdge {
                id: source.id,
                source: Some(source.from),
                target: Some(source.to),
                points: &route.points,
                label_percentage: route.label.map_or(0.0, |label| label.percentage),
                label_size: route.label.map(|label| Dimensions {
                    width: label.width,
                    height: label.height,
                }),
                source_arrowhead_label_size: None,
                target_arrowhead_label_size: None,
            })
            .collect();
        let expected_edges: Vec<_> = self
            .translated
            .edges
            .iter()
            .map(|edge| AuthoredEdge {
                id: edge.id,
                source: node_ids[edge.from],
                target: node_ids[edge.to],
            })
            .collect();
        CompletedGraph {
            nodes: &completed_nodes,
            edges: &completed_edges,
        }
        .validate_topology(
            &AuthoredTopology {
                node_ids: &node_ids,
                edges: &expected_edges,
            },
            Default::default(),
            &|| false,
        )
        .map(|_| ())
        .map_err(|_| Error::InvalidGraph)
    }

    fn evaluate(
        &self,
        nodes: &[NodeGeometry],
        edges: &[RoutedEdge],
        node_boxes: &[Option<Box>],
        edge_boxes: &[Option<Box>],
    ) -> Result<crate::quality::Evaluation, Error> {
        let nodes = nodes
            .iter()
            .zip(node_boxes)
            .map(|(body, caption)| PositionedNode {
                body: body_box(body),
                container: None,
                effective_container: None,
                ancestry_parent: None,
                is_container: false,
                clustered: false,
                label: caption.map(|caption| Box {
                    x: body.x + body.width / 2.0 - caption.width / 2.0,
                    y: body.y + body.height / 2.0 - caption.height / 2.0,
                    width: caption.width,
                    height: caption.height,
                }),
                outside_label: false,
                icon: None,
                outside_icon_reserve: None,
                fixed_top_left: None,
                modifier_extension: None,
                loop_offsets: [0.0; 4],
            })
            .collect();
        let edges = edges
            .iter()
            .zip(&self.translated.edges)
            .zip(edge_boxes)
            .map(|((route, edge), caption)| PositionedEdge {
                source: edge.from,
                target: edge.to,
                points: route.points.clone(),
                label: route.label.and_then(|anchor| {
                    anchor
                        .top_left(&route.points, LAYOUT_STROKE_WIDTH)
                        .map(|point| Box {
                            x: point.x,
                            y: point.y,
                            width: anchor.width,
                            height: anchor.height,
                        })
                }),
                label_is_positioned: caption.is_some(),
                source_arrowhead_label: None,
                target_arrowhead_label: None,
            })
            .collect();
        crate::quality::evaluate(
            &PositionedGraph {
                nodes,
                edges,
                detached_ancestry: Vec::new(),
            },
            None,
        )
        .map_err(|error| match error {
            EvaluationError::InvalidGeometry => Error::InvalidGraph,
            EvaluationError::ResourceLimit
            | EvaluationError::WorkLimit
            | EvaluationError::Cancelled => Error::WorkLimit,
        })
    }
}

fn body_box(body: &NodeGeometry) -> Box {
    Box {
        x: body.x,
        y: body.y,
        width: body.width,
        height: body.height,
    }
}

fn route_component(component: &MeasuredComponent, seed: i64) -> Result<Layout, Error> {
    let nodes = component.place(seed)?;
    let edges: Vec<_> = component
        .graph
        .edges
        .iter()
        .zip(&component.edge_owners)
        .map(|(edge, &owners)| OrdinaryEdge {
            owners,
            arrowheads: [edge.source_arrow, edge.target_arrow]
                .map(|present| if present { "triangle" } else { "none" }.into()),
            appearance: Appearance::default(),
            invisible: false,
        })
        .collect();
    let paths = {
        let graph = rectangular_linked_visibility(&nodes, &component.edge_owners)?;
        let weights = Weights::from_graph(&nodes, &component.edge_owners)?;
        let labels = vec![None; nodes.len()];
        let search = FlatSearch::new(&graph, &nodes, &labels, weights)?;
        let mut work = MAX_ROUTE_WORK;
        let selection = search
            .select_flavor(&component.node_ids, &edges, &mut work)?
            .ok_or(Error::UnsupportedGraph)?;
        let mut paths = search.route_points_in_input_order(&selection)?;
        let mut stage = Layout {
            nodes: nodes.clone(),
            edges: component
                .graph
                .edges
                .iter()
                .zip(paths.drain(..))
                .map(|(edge, points)| EdgeGeometry {
                    id: edge.id.clone(),
                    points,
                    label: None,
                })
                .collect(),
        };
        crate::simplify::simplify_routes(&component.graph, &mut stage)?;
        paths.extend(
            stage
                .edges
                .iter_mut()
                .map(|edge| std::mem::take(&mut edge.points)),
        );
        search.straight_edges_fallback(&edges, &mut paths, &mut work)?;
        for (edge, path) in stage.edges.iter_mut().zip(paths) {
            edge.points = path;
        }
        stage.edges
    };
    let mut layout = Layout {
        nodes,
        edges: paths,
    };
    crate::balance::balance_routes(&component.graph, &mut layout)?;
    Ok(layout)
}
