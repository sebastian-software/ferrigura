// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, internal/quality/labels.go and
// internal/labeling/placement.go.

use super::routes::{box_crosses_line, shared_segment_boxes};
use super::{
    Box, EvaluationError, LABEL_PADDING, PositionedEdge, PositionedGraph, PositionedNode, Work,
};

pub(super) fn score(graph: &PositionedGraph, work: &mut Work<'_>) -> Result<f64, EvaluationError> {
    work.step()?;
    let mut placed = Vec::new();
    let mut score = 0.0;
    for edge in &graph.edges {
        work.step()?;
        for arrowhead in [edge.source_arrowhead_label, edge.target_arrowhead_label]
            .into_iter()
            .flatten()
        {
            work.add(edge.points.len())?;
            placed.push(arrowhead);
        }
    }
    for edge in &graph.edges {
        work.step()?;
        if edge.source == edge.target
            && let Some(label) = edge.label
        {
            work.add(edge.points.len())?;
            placed.push(label);
        }
    }
    let outside = graph.nodes.len();
    let mut direct_children = vec![Vec::new(); outside + 1];
    for (index, node) in graph.nodes.iter().enumerate() {
        direct_children[node.container.unwrap_or(outside)].push(index);
    }
    for (index, node) in graph.nodes.iter().enumerate() {
        work.step()?;
        if let Some(icon) = node.icon {
            let node_overlaps = box_count(icon, graph.nodes.iter().map(|n| n.body), 0.0, work)?;
            let edge_overlaps = edge_count(icon, &graph.edges, None, 0.0, work)?;
            let label_overlaps = box_count(icon, placed.iter().copied(), 0.0, work)?;
            score += f64::from(node_overlaps + edge_overlaps + 2 * label_overlaps);
            placed.push(icon);
        }
        if let Some(label) = node.label {
            let mut peers = Vec::new();
            for &sibling in &direct_children[node.container.unwrap_or(outside)] {
                work.step()?;
                if sibling != index {
                    peers.push(graph.nodes[sibling].body);
                }
            }
            if node.is_container {
                for &child in &direct_children[index] {
                    work.step()?;
                    peers.push(graph.nodes[child].body);
                }
            }
            let mut ancestors = Vec::new();
            let mut current = Some(index);
            while let Some(index) = current {
                work.step()?;
                let ancestor = &graph.nodes[index];
                ancestors.push(ancestor.body);
                current = ancestor.effective_container;
            }
            let sibling_overlaps = box_count(label, peers, LABEL_PADDING, work)?;
            let mut ancestor_overlaps = 0;
            for ancestor in ancestors {
                work.step()?;
                if overlaps(label, ancestor, LABEL_PADDING) && !covers(ancestor, label) {
                    ancestor_overlaps += 1;
                }
            }
            let edge_overlaps = edge_count(label, &graph.edges, None, LABEL_PADDING - 1.0, work)?;
            let label_overlaps = box_count(label, placed.iter().copied(), LABEL_PADDING, work)?;
            score += f64::from(
                sibling_overlaps + ancestor_overlaps + edge_overlaps + 2 * label_overlaps,
            );
            placed.push(label);
        }
    }

    let shared = shared_segment_boxes(&graph.edges, work)?;
    for (index, edge) in graph.edges.iter().enumerate() {
        work.step()?;
        if let Some(label) = edge.label
            && edge.source != edge.target
        {
            work.add(edge.points.len())?;
            for _ in &graph.edges {
                work.step()?;
            }
            let mut ancestors = Vec::new();
            let mut non_ancestors = Vec::new();
            for (node_index, node) in graph.nodes.iter().enumerate() {
                work.step()?;
                let source_descendant = is_descendant(graph, edge.source, node_index, work)?;
                let target_descendant = is_descendant(graph, edge.target, node_index, work)?;
                if node_index != edge.source
                    && node_index != edge.target
                    && (source_descendant || target_descendant)
                {
                    ancestors.push(node);
                } else {
                    non_ancestors.push(node);
                }
            }
            let ancestor_area = node_overlap_area(label, &ancestors, true, work)?;
            let non_ancestor_area = node_overlap_area(label, &non_ancestors, false, work)?;
            let edge_overlaps = edge_count(label, &graph.edges, Some(index), LABEL_PADDING, work)?;
            let label_overlaps = box_count(label, placed.iter().copied(), 0.0, work)?;
            let almost_label_overlaps =
                box_count(label, placed.iter().copied(), LABEL_PADDING, work)?;
            let shared_segment_overlaps =
                box_count(label, shared.iter().copied(), LABEL_PADDING, work)?;
            let label_area = label.width * label.height;
            let mut label_score = 0.0;
            if label_area > 0.0 {
                label_score = (ancestor_area + non_ancestor_area) / label_area * 2.0;
            }
            label_score += f64::from(label_overlaps) * 10.0;
            label_score += f64::from(almost_label_overlaps);
            label_score += f64::from(edge_overlaps) * 2.0;
            label_score += f64::from(shared_segment_overlaps);
            score += label_score;
            placed.push(label);
        }
    }
    Ok(1.0 / (1.0 + score))
}

fn is_descendant(
    graph: &PositionedGraph,
    mut descendant: usize,
    ancestor: usize,
    work: &mut Work<'_>,
) -> Result<bool, EvaluationError> {
    loop {
        work.step()?;
        if descendant == ancestor {
            return Ok(true);
        }
        let Some(parent) = graph.ancestry_parent(descendant) else {
            return Ok(false);
        };
        descendant = parent;
    }
}

fn edge_count(
    bounds: Box,
    edges: &[PositionedEdge],
    except: Option<usize>,
    padding: f64,
    work: &mut Work<'_>,
) -> Result<u32, EvaluationError> {
    let mut count = 0;
    for (index, edge) in edges.iter().enumerate() {
        if Some(index) != except {
            for points in edge.points.windows(2) {
                work.step()?;
                count += u32::from(box_crosses_line(bounds, points[0], points[1], padding));
            }
        }
    }
    Ok(count)
}

fn box_count(
    bounds: Box,
    others: impl IntoIterator<Item = Box>,
    padding: f64,
    work: &mut Work<'_>,
) -> Result<u32, EvaluationError> {
    let mut count = 0;
    for other in others {
        work.step()?;
        count += u32::from(overlaps(bounds, other, padding));
    }
    Ok(count)
}

fn node_overlap_area(
    bounds: Box,
    nodes: &[&PositionedNode],
    partial: bool,
    work: &mut Work<'_>,
) -> Result<f64, EvaluationError> {
    let mut area = 0.0;
    for node in nodes {
        work.step()?;
        if !overlaps(bounds, node.body, LABEL_PADDING) || partial && covers(node.body, bounds) {
            continue;
        }
        if overlaps(bounds, node.body, 0.0) {
            let width = bounds.right().min(node.body.right()) - bounds.x.max(node.body.x);
            let height = bounds.bottom().min(node.body.bottom()) - bounds.y.max(node.body.y);
            area += width * height;
        }
        if !node.is_container {
            area += bounds.width * bounds.height;
        }
    }
    Ok(area)
}

fn overlaps(first: Box, second: Box, padding: f64) -> bool {
    first.x < second.right() + padding
        && second.x < first.right() + padding
        && first.y < second.bottom() + padding
        && second.y < first.bottom() + padding
}

fn covers(outer: Box, inner: Box) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.right() <= outer.right()
        && inner.bottom() <= outer.bottom()
}
