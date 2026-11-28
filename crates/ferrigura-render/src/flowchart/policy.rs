//! Flowchart rendering mode and attachment policy (decision 0019).

use std::collections::HashMap;

use ferrigura_elk::json::{JsonLabel, JsonNode, LayoutOptions};

/// The flowchart's attachment policy. Both modes draw full-opacity label backgrounds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FlowchartRouting {
    /// Center ports with label-aware spacing and draw edge-label backgrounds at full opacity.
    /// Theme colors, including explicit transparency, still apply. The default.
    #[default]
    Adaptive,
    /// Preserve the pinned Mermaid reference's ports and spacing. Label backgrounds remain at full opacity.
    Mermaid,
}

/// The largest measured edge-label dimensions in a layout container.
#[derive(Default)]
struct LabelExtent {
    height: f64,
    width: f64,
}

/// Center implicit ports without changing node constraints, edge routing or post-processing.
/// ELK fits each port group into the available node side; the requested spacing is not a minimum
/// node size. Parent options control implicit ports, including those of nested subgraphs.
pub(crate) fn center_ports(graph: &mut JsonNode) {
    let mut paths = HashMap::new();
    index_paths(graph, &mut Vec::new(), &mut paths);
    let mut extents = HashMap::new();
    collect_label_extents(graph, &paths, &mut extents);
    apply_centering(graph, &mut Vec::new(), &extents, "DOWN");
}

/// Locate nodes before ELK moves the root-stored edges into their actual containers.
fn index_paths(node: &JsonNode, path: &mut Vec<usize>, paths: &mut HashMap<String, Vec<usize>>) {
    paths.insert(node.id.clone(), path.clone());
    for (index, child) in node.children.iter().enumerate() {
        path.push(index);
        index_paths(child, path, paths);
        path.pop();
    }
}

/// Match ELK's best edge containment: the lowest common ancestor, or a self-loop's parent.
/// The edge's storage location is independent of its layout container.
fn collect_label_extents(
    node: &JsonNode,
    paths: &HashMap<String, Vec<usize>>,
    extents: &mut HashMap<Vec<usize>, LabelExtent>,
) {
    for edge in &node.edges {
        let (Some(source), Some(target)) = (
            edge.sources
                .first()
                .or(edge.source.as_ref())
                .and_then(|id| paths.get(id)),
            edge.targets
                .first()
                .or(edge.target.as_ref())
                .and_then(|id| paths.get(id)),
        ) else {
            continue;
        };
        let container_depth = if source == target {
            let Some(parent_depth) = source.len().checked_sub(1) else {
                continue;
            };
            parent_depth
        } else {
            source
                .iter()
                .zip(target)
                .take_while(|(a, b)| a == b)
                .count()
        };
        let height = edge
            .labels
            .iter()
            .filter_map(|label| label.height)
            .filter(|height| height.is_finite())
            .fold(0.0_f64, f64::max);
        let width = edge
            .labels
            .iter()
            .filter_map(|label| label.width)
            .filter(|width| width.is_finite())
            .fold(0.0_f64, f64::max);
        if height > 0.0 {
            // An edge that crosses a frame needs label-aware spacing on both endpoint branches.
            // Otherwise the inner endpoint stays distributed while its outer neighbor is centered.
            for endpoint in [source, target] {
                for depth in container_depth..endpoint.len() {
                    let extent = extents.entry(endpoint[..depth].to_vec()).or_default();
                    extent.height = extent.height.max(height);
                    extent.width = extent.width.max(width);
                }
            }
        }
    }
    for child in &node.children {
        collect_label_extents(child, paths, extents);
    }
}

fn apply_centering(
    graph: &mut JsonNode,
    path: &mut Vec<usize>,
    extents: &HashMap<Vec<usize>, LabelExtent>,
    inherited_direction: &str,
) {
    let direction = graph
        .layout_options
        .as_ref()
        // INCLUDE_CHILDREN merges a nested layout into its ancestor; its own direction is inert.
        .filter(|options| {
            path.is_empty()
                || options
                    .get("elk.hierarchyHandling")
                    .and_then(|value| value.as_str())
                    != Some("INCLUDE_CHILDREN")
        })
        .and_then(|options| options.get("elk.direction"))
        .and_then(|value| value.as_str())
        .unwrap_or(inherited_direction)
        .to_owned();
    if let Some(extent) = extents.get(path) {
        // Keep unlabeled containers unchanged. ELK fits centered ports into the available side.
        // Reserve two measured label heights plus 2 units for neighboring reciprocal routes.
        let mut spacing = 20.0_f64.max(2.0 * extent.height + 2.0);
        // On vertical routes, an inline label also extends horizontally toward its neighbor.
        if matches!(direction.as_str(), "DOWN" | "UP") {
            spacing = spacing.max(extent.width / 2.0 + 2.0);
        }
        graph
            .layout_options
            .get_or_insert_default()
            .insert("elk.spacing.portPort".to_owned(), spacing.into());
        for child in &mut graph.children {
            child
                .layout_options
                .get_or_insert_default()
                .insert("elk.portAlignment.default".to_owned(), "CENTER".into());
        }
    }
    for (index, child) in graph.children.iter_mut().enumerate() {
        path.push(index);
        apply_centering(child, path, extents, &direction);
        path.pop();
    }
}

/// The node placement of the design mode: ELK's network simplex placement, which lines nodes up so
/// that edges run straight where the reference's Brandes & Koepf placement leaves a step.
const DESIGN_PLACEMENT: &str = "NETWORK_SIMPLEX";

/// Gives a flowchart in the design mode more room than the reference's spacings do, straighter
/// edges, and edges that meet a node around the middle of its side
/// (`docs/decisions/0023-design-mode.md`): a wider base value at the root, a group padding with a
/// row for the title, the mode's distance between the ports of a side in every graph and the
/// center alignment of the ports of every node, and, when `place` is set, the network simplex node
/// placement in every graph. The options are ELK's own; the engine lays the graph out as it does
/// any other.
pub(crate) fn relax(graph: &mut JsonNode, place: bool) {
    let options = graph.layout_options.get_or_insert_default();
    options.insert(
        "spacing.baseValue".to_owned(),
        crate::design::ROOT_SPACING.into(),
    );
    relax_graph(graph, place);
    for child in &mut graph.children {
        relax_below(child, place);
    }
}

/// Sets what every graph of the design mode has, the root and the groups: the distance between
/// the ports of a side, which replaces the label-aware one of [`center_ports`], the distance
/// between a layer and the turns of the edges beside it, and the node placement when `place` is
/// set.
fn relax_graph(graph: &mut JsonNode, place: bool) {
    let options = graph.layout_options.get_or_insert_default();
    options.insert(
        "elk.spacing.portPort".to_owned(),
        crate::design::PORT_PITCH.into(),
    );
    options.insert(
        "elk.layered.spacing.edgeNodeBetweenLayers".to_owned(),
        crate::design::LAYER_TURN_SPACING.into(),
    );
    if place {
        options.insert(
            "elk.layered.nodePlacement.strategy".to_owned(),
            DESIGN_PLACEMENT.into(),
        );
    }
}

/// Sets the design mode's padding on every group at or below a node, with what every graph has,
/// and the center alignment of the ports on every node without children.
fn relax_below(node: &mut JsonNode, place: bool) {
    if node.children.is_empty() {
        node.layout_options
            .get_or_insert_default()
            .insert("elk.portAlignment.default".to_owned(), "CENTER".into());
        return;
    }
    node.layout_options.get_or_insert_default().insert(
        "elk.padding".to_owned(),
        crate::design::GROUP_PADDING.into(),
    );
    relax_graph(node, place);
    for child in &mut node.children {
        relax_below(child, place);
    }
}

/// The text of the label that holds a layer for an edge without one ([`hold_label_layers`]): ELK
/// lays out a label only when it has text. It has no size, and nothing draws it.
pub(crate) const LAYER_HOLDER: &str = " ";

/// Gives an edge without a label the layer a label would take, where its sibling carries one, so
/// that siblings stand in one layer (`docs/decisions/0023-design-mode.md`).
///
/// ELK gives the centered label of an edge a node in a layer of its own, so a labeled edge spans
/// two layers where an edge without a label spans one. Of two children of one node, the child
/// whose edge carries a label would stand a layer lower; of two parents of one node, the parent
/// whose edge carries none would stand in the layer of the other's label. An edge without a label
/// whose source has a labeled edge to another node, or whose target has a labeled edge from
/// another node, therefore takes a label of no size, unless one of the two other ends reaches the
/// other along the edges: then the two are no siblings, and one stands after the other anyway.
/// The label takes its edge's centered label's place, or is added where the edge has none.
pub(crate) fn hold_label_layers(graph: &mut JsonNode) {
    let mut ends = Vec::new();
    collect_ends(graph, &mut ends);
    let mut next: HashMap<&str, Vec<&str>> = HashMap::new();
    for (source, target, _) in &ends {
        next.entry(source.as_str())
            .or_default()
            .push(target.as_str());
    }
    let reaches = |from: &str, to: &str| {
        let mut seen = std::collections::HashSet::from([from]);
        let mut stack = vec![from];
        while let Some(node) = stack.pop() {
            for &after in next.get(node).into_iter().flatten() {
                if after == to {
                    return true;
                }
                if seen.insert(after) {
                    stack.push(after);
                }
            }
        }
        false
    };
    let apart = |a: &str, b: &str| a != b && !reaches(a, b) && !reaches(b, a);
    let mut held = std::collections::HashSet::new();
    for (index, (source, target, labeled)) in ends.iter().enumerate() {
        if *labeled || source == target {
            continue;
        }
        let sibling = ends
            .iter()
            .any(|(other_source, other_target, other_labeled)| {
                *other_labeled
                    && other_source != other_target
                    && ((other_source == source && apart(target, other_target))
                        || (other_target == target && apart(source, other_source)))
            });
        if sibling {
            held.insert(index);
        }
    }
    if !held.is_empty() {
        let mut index = 0;
        hold_edges(graph, &held, &mut index);
    }
}

/// The source, the target and whether the edge carries a centered label with text, for every
/// edge at or below `node`, in the order [`hold_edges`] visits them.
fn collect_ends(node: &JsonNode, ends: &mut Vec<(String, String, bool)>) {
    for edge in &node.edges {
        let end = |ids: &[String], id: &Option<String>| {
            ids.first().or(id.as_ref()).cloned().unwrap_or_default()
        };
        let labeled = edge.labels.first().is_some_and(|label| {
            centered(label) && label.text.as_deref().is_some_and(|text| !text.is_empty())
        });
        ends.push((
            end(&edge.sources, &edge.source),
            end(&edge.targets, &edge.target),
            labeled,
        ));
    }
    for child in &node.children {
        collect_ends(child, ends);
    }
}

/// Whether a label is the centered one of its edge.
fn centered(label: &JsonLabel) -> bool {
    label
        .layout_options
        .as_ref()
        .and_then(|options| options.get("edgeLabels.placement"))
        .and_then(|value| value.as_str())
        == Some("CENTER")
}

/// Gives the edges of `held`, by their place in [`collect_ends`]'s order, the label that holds a
/// layer.
fn hold_edges(node: &mut JsonNode, held: &std::collections::HashSet<usize>, index: &mut usize) {
    for edge in &mut node.edges {
        if held.contains(index) {
            match edge.labels.first_mut().filter(|label| centered(label)) {
                Some(label) => {
                    label.text = Some(LAYER_HOLDER.to_owned());
                    label.width = Some(0.0);
                    label.height = Some(0.0);
                }
                None => {
                    let mut options = LayoutOptions::new();
                    options.insert("edgeLabels.inline".to_owned(), "true".into());
                    options.insert("edgeLabels.placement".to_owned(), "CENTER".into());
                    edge.labels.insert(
                        0,
                        JsonLabel {
                            text: Some(LAYER_HOLDER.to_owned()),
                            width: Some(0.0),
                            height: Some(0.0),
                            layout_options: Some(options),
                            ..JsonLabel::default()
                        },
                    );
                }
            }
        }
        *index += 1;
    }
    for child in &mut node.children {
        hold_edges(child, held, index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relaxing_sets_the_spacing_the_padding_the_placement_and_the_ports() {
        let leaf = |id: &str| JsonNode {
            id: id.to_owned(),
            ..JsonNode::default()
        };
        let group = |id: &str, children: Vec<JsonNode>| JsonNode {
            id: id.to_owned(),
            children,
            ..JsonNode::default()
        };
        let mut graph = group(
            "root",
            vec![
                leaf("a"),
                group("outer", vec![leaf("b"), group("inner", vec![leaf("c")])]),
            ],
        );
        relax(&mut graph, true);
        let option = |node: &JsonNode, name: &str| {
            node.layout_options
                .as_ref()
                .and_then(|options| options.get(name))
                .cloned()
        };
        assert_eq!(
            option(&graph, "spacing.baseValue"),
            Some(crate::design::ROOT_SPACING.into())
        );
        assert_eq!(option(&graph, "elk.padding"), None);
        let placement = "elk.layered.nodePlacement.strategy";
        assert_eq!(option(&graph, placement), Some("NETWORK_SIMPLEX".into()));
        assert_eq!(option(&graph.children[0], placement), None);
        assert_eq!(
            option(&graph.children[1], placement),
            Some("NETWORK_SIMPLEX".into())
        );
        // Every graph has the mode's distance between ports, and every leaf centers its ports.
        let (pitch, alignment) = ("elk.spacing.portPort", "elk.portAlignment.default");
        assert_eq!(option(&graph, pitch), Some(20.0.into()));
        assert_eq!(option(&graph.children[1], pitch), Some(20.0.into()));
        assert_eq!(
            option(&graph.children[1].children[1], pitch),
            Some(20.0.into())
        );
        assert_eq!(option(&graph.children[0], pitch), None);
        // Every graph keeps its layers 30 from the turns of the edges between them.
        let turns = "elk.layered.spacing.edgeNodeBetweenLayers";
        assert_eq!(option(&graph, turns), Some(30.0.into()));
        assert_eq!(
            option(&graph.children[1].children[1], turns),
            Some(30.0.into())
        );
        assert_eq!(option(&graph.children[0], alignment), Some("CENTER".into()));
        assert_eq!(
            option(&graph.children[1].children[1].children[0], alignment),
            Some("CENTER".into())
        );
        assert_eq!(option(&graph.children[1], alignment), None);
        // A configured placement strategy is the user's: the mode leaves it alone.
        let mut kept = group("root", vec![group("outer", vec![leaf("b")])]);
        relax(&mut kept, false);
        assert_eq!(option(&kept, placement), None);
        assert_eq!(option(&kept.children[0], placement), None);
        assert_eq!(
            option(&kept.children[0], "elk.padding"),
            Some(crate::design::GROUP_PADDING.into())
        );
        assert_eq!(option(&graph.children[0], "elk.padding"), None);
        let outer = &graph.children[1];
        assert_eq!(
            option(outer, "elk.padding"),
            Some(crate::design::GROUP_PADDING.into())
        );
        assert_eq!(
            option(&outer.children[1], "elk.padding"),
            Some(crate::design::GROUP_PADDING.into())
        );
        assert_eq!(option(&outer.children[0], "elk.padding"), None);
    }
}
