//! Flowchart rendering mode and attachment policy (decision 0019).

use std::collections::HashMap;

use ferrigura_elk::json::JsonNode;

/// The flowchart's attachment policy. Both modes draw full-opacity label backgrounds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FlowchartMode {
    /// Center ports with label-aware spacing and draw edge-label backgrounds at full opacity.
    /// Theme colors, including explicit transparency, still apply. The default.
    #[default]
    Adaptive,
    /// Preserve the pinned Mermaid reference's ports and spacing. Label backgrounds remain at full opacity.
    Mermaid,
}

impl std::str::FromStr for FlowchartMode {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "adaptive" => Ok(Self::Adaptive),
            "mermaid" => Ok(Self::Mermaid),
            other => Err(format!(
                "unknown flowchart mode {other}, expected adaptive or mermaid"
            )),
        }
    }
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
