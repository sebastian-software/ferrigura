//! Flowchart attachment policy, applied to the measured graph before ELK (decision 0019).

use ferrigura_elk::json::JsonNode;

/// How a flowchart places its edge attachment points before ELK routes the edges.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FlowchartRouting {
    /// Center the ports with spacing derived from measured edge labels. The default.
    #[default]
    Adaptive,
    /// Preserve the pinned Mermaid reference's ports and spacing.
    Mermaid,
}

impl std::str::FromStr for FlowchartRouting {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "adaptive" => Ok(Self::Adaptive),
            "mermaid" => Ok(Self::Mermaid),
            other => Err(format!(
                "unknown flowchart routing {other}, expected adaptive or mermaid"
            )),
        }
    }
}

/// Center implicit ports without changing node constraints, edge routing or post-processing.
/// ELK fits each port group into the available node side; the requested spacing is not a minimum
/// node size. Parent options control implicit ports, including those of nested subgraphs.
pub(crate) fn center_ports(graph: &mut JsonNode) {
    if graph.children.is_empty() {
        return;
    }
    let label_height = graph
        .edges
        .iter()
        .flat_map(|edge| &edge.labels)
        .filter_map(|label| label.height)
        .filter(|height| height.is_finite())
        .fold(0.0_f64, f64::max);
    // Preserve unlabeled parent graphs: compact centered ports can add steps to already straight
    // parallel routes and dense fans. A positive measured label height is the reason to reserve
    // channels and center their attachments, not the presence of a subgraph by itself.
    if label_height <= 0.0 {
        for child in &mut graph.children {
            center_ports(child);
        }
        return;
    }
    // Keep ELK's 20-unit port gap as the floor. Reserve two measured label heights plus
    // 2 units of clearance for neighboring reciprocal routes (decision 0019).
    let spacing = 20.0_f64.max(2.0 * label_height + 2.0);
    graph
        .layout_options
        .get_or_insert_default()
        .insert("elk.spacing.portPort".to_owned(), spacing.into());
    for child in &mut graph.children {
        child
            .layout_options
            .get_or_insert_default()
            .insert("elk.portAlignment.default".to_owned(), "CENTER".into());
        center_ports(child);
    }
}
