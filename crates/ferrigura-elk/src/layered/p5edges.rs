//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p5edges/OrthogonalEdgeRouter.java
//!
//! The orthogonal edge router, `OrthogonalEdgeRouter.process`, for graphs without edges.
//! Routing edges needs the orthogonal routing generator, which is not ported; so far the
//! layers are placed and the graph's width is set.

use crate::layered::LayoutError;
use crate::layered::graph_util::place_nodes_horizontally;
use crate::layered::lgraph::LGraph;
use crate::layered::spacings::SPACING_NODE_NODE_BETWEEN_LAYERS;

/// `OrthogonalEdgeRouter.process` for a graph with no edges. `routeEdges` finds no slots, so
/// the gap between two layers is `nodeNodeBetweenLayers`, unless one of them is external. An
/// empty layer, and the layers at either end of the graph, count as external.
///
/// The Java declares `xpos` as a float; the port keeps `f64`, as the corpus output requires.
pub fn route_nodes_only(graph: &mut LGraph) -> Result<(), LayoutError> {
    let node_node_spacing = graph
        .properties
        .double_of(SPACING_NODE_NODE_BETWEEN_LAYERS)
        .ok_or_else(|| {
            LayoutError::Unsupported("a graph without nodeNodeBetweenLayers".to_string())
        })?;
    let layer_count = graph.layers.len();
    let mut xpos = 0.0;
    let mut left: Option<usize> = None;
    // Like the Java loop, one step past the last layer, so the last layer is placed too.
    for index in 0..=layer_count {
        let right = (index < layer_count).then_some(index);
        if let Some(layer) = left {
            place_nodes_horizontally(graph, layer, xpos);
            xpos += graph.layer_sizes[layer].x;
        }
        let left_external = left.is_none_or(|layer| graph.layers[layer].is_empty());
        let right_external = right.is_none_or(|layer| graph.layers[layer].is_empty());
        if !left_external && !right_external {
            xpos += node_node_spacing;
        }
        left = right;
    }
    graph.size.x = xpos;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::OptionValue;
    use crate::core::math::KVector;
    use crate::layered::lgraph::LNode;

    #[test]
    fn layers_are_spaced_by_node_node_between_layers() {
        let mut graph = LGraph::default();
        graph.properties.set(
            SPACING_NODE_NODE_BETWEEN_LAYERS,
            Some(OptionValue::Double(40.0)),
        );
        for (index, width) in [50.0, 30.0].into_iter().enumerate() {
            graph.nodes.push(LNode {
                size: KVector { x: width, y: 10.0 },
                ..LNode::default()
            });
            graph.layers.push(vec![index]);
            graph.layer_sizes.push(KVector { x: width, y: 10.0 });
        }
        route_nodes_only(&mut graph).unwrap();
        assert_eq!(graph.size.x, 50.0 + 40.0 + 30.0);
        assert_eq!(graph.nodes[0].position.x, 0.0);
        assert_eq!(graph.nodes[1].position.x, 90.0);
    }
}
