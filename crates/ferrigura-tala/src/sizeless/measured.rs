// SPDX-License-Identifier: MPL-2.0
// Ported from D2 TALA at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde:
// internal/layoutgraph/{graph,node}.go and internal/placement/node_placement.go.

use crate::layout_graph::{Direction, LayoutGraph, Shape};
use crate::{Edge, Error, Graph, Node, NodeGeometry};

/// Runs translation, prescale and seeded placement on the measured input of a
/// flat rectangular component whose topology is unchanged by preprocessing.
///
/// # Errors
/// Rejects invalid measurements, nested or nonrectangular input, or topology
/// outside the flat placement stages.
pub fn sized_transition_from_measurements(
    input: &Graph,
    seed: i64,
) -> Result<Vec<NodeGeometry>, Error> {
    super::transition_to_sized(&measured_component(input)?.graph, seed)
}

/// Runs the first complete sized-search pass from Ferrigura's measured input.
///
/// # Errors
/// Returns an error for invalid measurements, unsupported topology, or placement work limits.
pub fn first_sized_pass_from_measurements(
    input: &Graph,
    seed: i64,
) -> Result<Vec<NodeGeometry>, Error> {
    let component = measured_component(input)?;
    super::first_sized_pass(&component.graph, seed, &component.obstacle_order)
}

/// Runs the complete size-aware annealing phase for a flat measured component.
///
/// # Errors
/// Rejects invalid preprocessed topology, measurements, or exhausted placement work.
pub fn sized_phase_from_measurements(input: &Graph, seed: i64) -> Result<Vec<NodeGeometry>, Error> {
    let component = measured_component(input)?;
    super::optimize_sized_phase(&component.graph, seed, &component.obstacle_order)
}

/// Runs placement and local packing for a flat measured component.
///
/// # Errors
/// Rejects invalid topology, measurements, or exhausted placement work.
pub fn flat_placement_from_measurements(
    input: &Graph,
    seed: i64,
) -> Result<Vec<NodeGeometry>, Error> {
    let component = measured_component(input)?;
    super::place_flat_component(&component.graph, seed, &component.obstacle_order)
}

/// Runs measured flat placement through the box refinements before routing.
///
/// # Errors
/// Rejects invalid topology, measurements, or exhausted placement work.
pub fn refined_flat_from_measurements(
    input: &Graph,
    seed: i64,
) -> Result<Vec<NodeGeometry>, Error> {
    measured_component(input)?.place(seed)
}

pub(crate) struct MeasuredComponent {
    pub graph: Graph,
    pub obstacle_order: Vec<usize>,
    pub node_ids: Vec<i64>,
    pub edge_owners: Vec<[usize; 2]>,
}

fn measured_component(input: &Graph) -> Result<MeasuredComponent, Error> {
    MeasuredComponent::new(input, LayoutGraph::translate(input)?)
}

impl MeasuredComponent {
    pub(crate) fn place(&self, seed: i64) -> Result<Vec<NodeGeometry>, Error> {
        let bodies = super::place_flat_component(&self.graph, seed, &self.obstacle_order)?;
        super::refine_flat_component(&self.graph, &bodies, &self.obstacle_order)
    }

    pub(crate) fn new(input: &Graph, mut graph: LayoutGraph) -> Result<Self, Error> {
        if graph.roots.len() != graph.nodes.len()
            || graph.nodes.iter().any(|node| node.shape != Shape::Square)
        {
            return Err(Error::UnsupportedGraph);
        }
        graph.prescale();
        let components = graph.flat_components();
        let [component] = components.as_slice() else {
            return Err(Error::UnsupportedGraph);
        };
        let direction = graph.direction.map(|direction| {
            match direction {
                Direction::Top => "BT",
                Direction::Right => "LR",
                Direction::Bottom => "TB",
                Direction::Left => "RL",
            }
            .to_owned()
        });
        let ids = |index: usize| &input.nodes[graph.nodes[index].input_index].id;
        let nodes = component
            .nodes
            .iter()
            .map(|&index| {
                let node = &graph.nodes[index];
                Node {
                    id: ids(index).clone(),
                    parent_id: None,
                    direction: None,
                    is_group: false,
                    shape: "rect".into(),
                    width: node.width,
                    height: node.height,
                    label: String::new(),
                    label_width: 0.0,
                    label_height: 0.0,
                }
            })
            .collect();
        let edges = component
            .edges
            .iter()
            .map(|&index| {
                let edge = &graph.edges[index];
                let (label, label_width, label_height) =
                    edge.label.as_ref().map_or_else(Default::default, |label| {
                        (label.text.clone(), label.width, label.height)
                    });
                Edge {
                    id: input.edges[index].id.clone(),
                    source: ids(edge.from).clone(),
                    target: ids(edge.to).clone(),
                    source_arrow: edge.source_arrow,
                    target_arrow: edge.target_arrow,
                    label,
                    label_width,
                    label_height,
                }
            })
            .collect();
        // The split orders optimizer nodes by breadth-first reachability. Go's
        // obstruction scan still runs in original container sibling order.
        let mut bfs_index = vec![0; graph.nodes.len()];
        for (index, &slot) in component.nodes.iter().enumerate() {
            bfs_index[slot] = index;
        }
        let obstacle_order = graph.roots.iter().map(|&slot| bfs_index[slot]).collect();
        let node_ids = component
            .nodes
            .iter()
            .map(|&slot| graph.nodes[slot].id)
            .collect();
        let edge_owners = component
            .edges
            .iter()
            .map(|&slot| {
                let edge = &graph.edges[slot];
                [bfs_index[edge.from], bfs_index[edge.to]]
            })
            .collect();
        Ok(Self {
            graph: Graph {
                direction,
                nodes,
                edges,
            },
            obstacle_order,
            node_ids,
            edge_owners,
        })
    }
}
