//! Spacings between nodes, `Spacings` and `LayeredSpacings` in `org.eclipse.elk.alg.layered.options`,
//! and `ElkSpacings.AbstractSpacingsBuilder` from ELK core.
//!
//! A base value (`spacing.baseValue`) sets `spacing.nodeNode` and derives the other
//! spacings from it, each scaled by its melk default relative to the base default.
//! The derived values are written to the layered graph, and only where the graph
//! does not set them already (`NO_OVERWRITE_HOLDER`).

use crate::core::data::enums::NodeType;
use crate::core::data::{OptionId, OptionValue, option_default};
use crate::layered::lgraph::{LGraph, LNode};

/// Spacing option ids, as `CoreOptions` and `LayeredOptions` name them.
pub const SPACING_NODE_NODE: OptionId = OptionId::of("org.eclipse.elk.spacing.nodeNode");
pub const SPACING_NODE_NODE_BETWEEN_LAYERS: OptionId =
    OptionId::of("org.eclipse.elk.layered.spacing.nodeNodeBetweenLayers");
pub const SPACING_EDGE_NODE: OptionId = OptionId::of("org.eclipse.elk.spacing.edgeNode");
pub const SPACING_EDGE_NODE_BETWEEN_LAYERS: OptionId =
    OptionId::of("org.eclipse.elk.layered.spacing.edgeNodeBetweenLayers");
pub const SPACING_EDGE_EDGE: OptionId = OptionId::of("org.eclipse.elk.spacing.edgeEdge");
pub const SPACING_EDGE_EDGE_BETWEEN_LAYERS: OptionId =
    OptionId::of("org.eclipse.elk.layered.spacing.edgeEdgeBetweenLayers");
pub const SPACING_LABEL_NODE: OptionId = OptionId::of("org.eclipse.elk.spacing.labelNode");
pub const SPACING_LABEL_PORT_HORIZONTAL: OptionId =
    OptionId::of("org.eclipse.elk.spacing.labelPortHorizontal");
pub const SPACING_LABEL_PORT_VERTICAL: OptionId =
    OptionId::of("org.eclipse.elk.spacing.labelPortVertical");
pub const SPACING_PORT_PORT: OptionId = OptionId::of("org.eclipse.elk.spacing.portPort");
pub const SPACING_BASE_VALUE: OptionId = OptionId::of("org.eclipse.elk.layered.spacing.baseValue");

/// The options a base value derives, `LayeredSpacingsBuilder.DEPENDENT_SPACING_OPTIONS`.
const DEPENDENT: &[OptionId] = &[
    OptionId::of("org.eclipse.elk.spacing.componentComponent"),
    SPACING_EDGE_EDGE,
    OptionId::of("org.eclipse.elk.spacing.edgeLabel"),
    SPACING_EDGE_NODE,
    OptionId::of("org.eclipse.elk.spacing.labelLabel"),
    SPACING_LABEL_NODE,
    SPACING_LABEL_PORT_HORIZONTAL,
    SPACING_LABEL_PORT_VERTICAL,
    OptionId::of("org.eclipse.elk.spacing.nodeSelfLoop"),
    SPACING_PORT_PORT,
    SPACING_EDGE_EDGE_BETWEEN_LAYERS,
    SPACING_EDGE_NODE_BETWEEN_LAYERS,
    SPACING_NODE_NODE_BETWEEN_LAYERS,
];

/// The melk default of an option, as a number.
fn default_of(id: OptionId) -> f64 {
    match option_default(id) {
        Some(&OptionValue::Double(value)) => value,
        Some(&OptionValue::Int(value)) => f64::from(value),
        _ => panic!("option {id} has no numeric default"),
    }
}

/// `LayeredSpacings.withBaseValue(base).apply(graph)`: sets the derived spacings
/// that the graph does not set yet.
///
/// Does nothing when `base` is the default base value, as Java does
/// (`!overwrite && fuzzyEquals(baseSpacing, default)`).
pub fn apply_base_value(graph: &mut LGraph, base: f64) {
    let base_default = default_of(SPACING_NODE_NODE);
    if (base - base_default).abs() <= 10e-5 {
        return;
    }
    let mut assign = |id: OptionId, factor: f64| {
        if !graph.properties.has(id) {
            graph
                .properties
                .set(id, Some(OptionValue::Double(factor * base)));
        }
    };
    assign(SPACING_NODE_NODE, 1.0);
    for &option in DEPENDENT {
        assign(option, default_of(option) / base_default);
    }
}

/// The vertical and horizontal option of a pair of node types, as `Spacings` records them.
#[derive(Debug, Clone, Copy, Default)]
struct Pair {
    vertical: Option<OptionId>,
    horizontal: Option<OptionId>,
}

const PAIR_EMPTY: Pair = Pair {
    vertical: None,
    horizontal: None,
};

/// `Spacings.precalculateNodeTypeSpacings`, as a compile-time table indexed by node type ordinals.
const TABLE: [[Pair; 6]; 6] = build_table();

const fn build_table() -> [[Pair; 6]; 6] {
    use NodeType::*;
    let mut t = [[PAIR_EMPTY; 6]; 6];
    const fn same(t: &mut [[Pair; 6]; 6], nt: NodeType, v: Option<OptionId>, h: Option<OptionId>) {
        let i = nt as usize;
        t[i][i] = Pair {
            vertical: v,
            horizontal: h,
        };
    }
    const fn pair_v(t: &mut [[Pair; 6]; 6], a: NodeType, b: NodeType, v: OptionId) {
        let (ia, ib) = (a as usize, b as usize);
        t[ia][ib].vertical = Some(v);
        t[ib][ia].vertical = Some(v);
    }
    const fn pair_vh(t: &mut [[Pair; 6]; 6], a: NodeType, b: NodeType, v: OptionId, h: OptionId) {
        let (ia, ib) = (a as usize, b as usize);
        let entry = Pair {
            vertical: Some(v),
            horizontal: Some(h),
        };
        t[ia][ib] = entry;
        t[ib][ia] = entry;
    }
    same(
        &mut t,
        NORMAL,
        Some(SPACING_NODE_NODE),
        Some(SPACING_NODE_NODE_BETWEEN_LAYERS),
    );
    pair_vh(
        &mut t,
        NORMAL,
        LONG_EDGE,
        SPACING_EDGE_NODE,
        SPACING_EDGE_NODE_BETWEEN_LAYERS,
    );
    pair_v(&mut t, NORMAL, NORTH_SOUTH_PORT, SPACING_EDGE_NODE);
    pair_v(&mut t, NORMAL, EXTERNAL_PORT, SPACING_EDGE_NODE);
    pair_vh(
        &mut t,
        NORMAL,
        LABEL,
        SPACING_NODE_NODE,
        SPACING_NODE_NODE_BETWEEN_LAYERS,
    );
    same(
        &mut t,
        LONG_EDGE,
        Some(SPACING_EDGE_EDGE),
        Some(SPACING_EDGE_EDGE_BETWEEN_LAYERS),
    );
    pair_v(&mut t, LONG_EDGE, NORTH_SOUTH_PORT, SPACING_EDGE_EDGE);
    pair_v(&mut t, LONG_EDGE, EXTERNAL_PORT, SPACING_EDGE_EDGE);
    pair_vh(
        &mut t,
        LONG_EDGE,
        LABEL,
        SPACING_EDGE_NODE,
        SPACING_EDGE_NODE_BETWEEN_LAYERS,
    );
    same(&mut t, NORTH_SOUTH_PORT, Some(SPACING_EDGE_EDGE), None);
    pair_v(&mut t, NORTH_SOUTH_PORT, EXTERNAL_PORT, SPACING_EDGE_EDGE);
    pair_v(&mut t, NORTH_SOUTH_PORT, LABEL, SPACING_LABEL_NODE);
    same(&mut t, EXTERNAL_PORT, Some(SPACING_PORT_PORT), None);
    pair_vh(
        &mut t,
        EXTERNAL_PORT,
        LABEL,
        SPACING_LABEL_PORT_VERTICAL,
        SPACING_LABEL_PORT_HORIZONTAL,
    );
    same(
        &mut t,
        LABEL,
        Some(SPACING_EDGE_EDGE),
        Some(SPACING_EDGE_EDGE),
    );
    same(
        &mut t,
        BREAKING_POINT,
        Some(SPACING_EDGE_EDGE),
        Some(SPACING_EDGE_EDGE_BETWEEN_LAYERS),
    );
    pair_vh(
        &mut t,
        BREAKING_POINT,
        NORMAL,
        SPACING_EDGE_NODE,
        SPACING_EDGE_NODE_BETWEEN_LAYERS,
    );
    pair_vh(
        &mut t,
        BREAKING_POINT,
        LABEL,
        SPACING_EDGE_NODE,
        SPACING_EDGE_NODE_BETWEEN_LAYERS,
    );
    pair_vh(
        &mut t,
        BREAKING_POINT,
        LONG_EDGE,
        SPACING_EDGE_NODE,
        SPACING_EDGE_NODE_BETWEEN_LAYERS,
    );
    t
}

/// The value of a spacing option for a pair of nodes: the larger of the two nodes'
/// values, each from the graph (individual spacings are not imported).
fn local_spacing(graph: &LGraph, option: Option<OptionId>) -> f64 {
    match option {
        Some(id) => graph.properties.double_of(id).unwrap_or(0.0),
        None => 0.0,
    }
}

/// Resolved node-type spacing tables for one `LGraph`.
#[derive(Debug, Clone, Copy)]
pub struct Spacings {
    vertical: [[f64; 6]; 6],
    horizontal: [[f64; 6]; 6],
}

impl Spacings {
    /// Resolves every node-type pair's vertical and horizontal spacing once for `graph`.
    pub fn for_graph(graph: &LGraph) -> Self {
        let mut vertical = [[0.0; 6]; 6];
        let mut horizontal = [[0.0; 6]; 6];
        for row in 0..6 {
            for col in row..6 {
                let pair = TABLE[row][col];
                let v = local_spacing(graph, pair.vertical);
                let h = local_spacing(graph, pair.horizontal);
                vertical[row][col] = v;
                vertical[col][row] = v;
                horizontal[row][col] = h;
                horizontal[col][row] = h;
            }
        }
        Spacings {
            vertical,
            horizontal,
        }
    }

    /// `Spacings.getVerticalSpacing(n1, n2)` from the cached table.
    pub fn vertical(&self, a: &LNode, b: &LNode) -> f64 {
        self.vertical[a.node_type as usize][b.node_type as usize]
    }

    /// `Spacings.getHorizontalSpacing(n1, n2)` from the cached table.
    pub fn horizontal(&self, a: &LNode, b: &LNode) -> f64 {
        self.horizontal[a.node_type as usize][b.node_type as usize]
    }
}

/// `Spacings.getVerticalSpacing(n1, n2)`.
pub fn vertical(graph: &LGraph, a: &LNode, b: &LNode) -> f64 {
    let pair = TABLE[a.node_type as usize][b.node_type as usize];
    local_spacing(graph, pair.vertical)
}

/// `Spacings.getHorizontalSpacing(n1, n2)`.
pub fn horizontal(graph: &LGraph, a: &LNode, b: &LNode) -> f64 {
    let pair = TABLE[a.node_type as usize][b.node_type as usize];
    local_spacing(graph, pair.horizontal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::enums::NodeType;

    #[test]
    fn base_value_scales_the_derived_spacings_by_their_defaults() {
        let mut graph = LGraph::default();
        apply_base_value(&mut graph, 40.0);
        assert_eq!(graph.properties.double_of(SPACING_NODE_NODE), Some(40.0));
        // Between layers the melk default is 20 against a base default of 20: factor 1.
        assert_eq!(
            graph.properties.double_of(SPACING_NODE_NODE_BETWEEN_LAYERS),
            Some(40.0)
        );
        // Edge-to-edge between layers: default 10, so half the base.
        assert_eq!(
            graph.properties.double_of(SPACING_EDGE_EDGE_BETWEEN_LAYERS),
            Some(20.0)
        );
    }

    #[test]
    fn explicit_values_are_kept() {
        let mut graph = LGraph::default();
        graph
            .properties
            .set(SPACING_NODE_NODE, Some(OptionValue::Double(7.0)));
        apply_base_value(&mut graph, 40.0);
        assert_eq!(graph.properties.double_of(SPACING_NODE_NODE), Some(7.0));
    }

    #[test]
    fn normal_nodes_use_node_node_spacing_within_a_layer() {
        let mut graph = LGraph::default();
        apply_base_value(&mut graph, 40.0);
        let node = LNode {
            node_type: NodeType::NORMAL,
            ..LNode::default()
        };
        assert_eq!(vertical(&graph, &node, &node), 40.0);
        assert_eq!(horizontal(&graph, &node, &node), 40.0);
    }
}
