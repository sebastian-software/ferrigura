//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/NodeContext.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/PortContext.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/PortContextCreator.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/InsidePortLabelCellCreator.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/NodeLabelCellCreator.java
//!
//! The context of one node's size calculation: the node's options, its port contexts and the
//! cell system around it. [`NodeContext::new`] reads the options and runs the checks for what
//! the reduced path does not port, before anything is built.
//!
//! The option ids that are not in `layered::options` are declared here, because that module has
//! one writer. The test module checks them against the registry. `portAlignment.default` is not
//! in the registry at all, so its default is written out (see [`DEFAULT_PORT_ALIGNMENT`]).

use crate::core::data::enums::{
    NodeLabelPlacement, PortAlignment, PortConstraints, PortLabelPlacement, PortSide,
    SizeConstraint, SizeOptions,
};
use crate::core::data::property::PropertyHolder;
use crate::core::data::{JavaEnum, OptionValue};
use crate::core::math::{KVector, Spacing};
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;
use crate::layered::options;
use crate::layered::spacings::{SPACING_LABEL_PORT_VERTICAL, SPACING_PORT_PORT};

use super::cells::{Area, CellId, CellSystem, Strip};
use super::labels::NodeLabelCell;

/// `ElkUtil.DEFAULT_MIN_WIDTH` and `DEFAULT_MIN_HEIGHT`, the minimum size DEFAULT_MINIMUM_SIZE gives.
const DEFAULT_MIN_WIDTH: f64 = 20.0;
const DEFAULT_MIN_HEIGHT: f64 = 20.0;

pub(super) const PORT_LABELS_PLACEMENT: &str = "org.eclipse.elk.portLabels.placement";
pub(super) const NODE_LABELS_PLACEMENT: &str = "org.eclipse.elk.nodeLabels.placement";
pub(super) const NODE_LABELS_PADDING: &str = "org.eclipse.elk.nodeLabels.padding";
pub(super) const SPACING_PORTS_SURROUNDING: &str = "org.eclipse.elk.spacing.portsSurrounding";
pub(super) const SPACING_INDIVIDUAL: &str = "org.eclipse.elk.spacing.individual";
pub(super) const PORT_ALIGNMENT_NORTH: &str = "org.eclipse.elk.portAlignment.north";
pub(super) const PORT_ALIGNMENT_EAST: &str = "org.eclipse.elk.portAlignment.east";
pub(super) const PORT_ALIGNMENT_SOUTH: &str = "org.eclipse.elk.portAlignment.south";
pub(super) const PORT_ALIGNMENT_WEST: &str = "org.eclipse.elk.portAlignment.west";

/// `org.eclipse.elk.portAlignment.default`. The registry lacks it, although `Core.melk` declares it
/// (group `portAlignment`, `option ^default`, line 518). Its value is read with
/// `PropertyHolder::get`, which does not look the registry up.
const PORT_ALIGNMENT_DEFAULT: &str = "org.eclipse.elk.portAlignment.default";

/// The melk default of `portAlignment.default` (`Core.melk` line 522): `PortAlignment.DISTRIBUTED`.
const DEFAULT_PORT_ALIGNMENT: PortAlignment = PortAlignment::DISTRIBUTED;

/// `PortContext`: a port of the node, with the position that the placement computes for it.
#[derive(Debug, Clone)]
pub(super) struct PortContext {
    /// Index into `LGraph::ports`.
    pub port: usize,
    pub side: PortSide,
    /// The port's place in `LNode::ports`, which `PortContextCreator` gives as the volatile id.
    pub volatile_id: usize,
    /// `LPort.getSize()`.
    pub size: KVector,
    /// `portPosition`: the position the placement computes, applied to the port at the end.
    pub position: KVector,
    /// `portMargin`: space around the port. Only label placement sets it, so it stays zero here.
    pub margin: Spacing,
}

/// `NodeContext`: what the size calculation of one node reads and changes.
pub(super) struct NodeContext {
    /// `nodeSize`: the size the node gets. Without size constraints it is the node's own size.
    pub node_size: KVector,
    /// `NODE_SIZE_FIXED_GRAPH_SIZE` of the graph: the node may only grow.
    pub fixed_graph_size: bool,
    /// `portConstraints` of the node; the placement switches on it (FIXED_POS has its own branches).
    pub port_constraints: PortConstraints,
    /// `sizeOptions` of the node, with the defaults.
    pub size_options: Vec<SizeOptions>,
    /// `sizeConstraints` of the node: only MINIMUM_SIZE and NODE_LABELS are ported.
    pub size_constraints: Vec<SizeConstraint>,
    /// `NodeSizeCalculator`'s minimum size, `getMinimumNodeSize`: set when MINIMUM_SIZE applies.
    pub minimum_size: Option<KVector>,
    /// `nodeLabelPlacement` of the node: where a label goes unless the label has its own.
    pub node_label_placement: Vec<NodeLabelPlacement>,
    /// `labelLabelSpacing`: the gap between the labels of one cell.
    pub label_label_spacing: f64,
    /// `insideNodeLabelContainer`: the grid that holds the inside label cells, in the middle row.
    pub inside_label_grid: CellId,
    /// `nodeContainerMiddleRow`: the west cell, the inside label grid and the east cell.
    pub middle_row: CellId,
    /// The node label cells, one per location that has labels, in the order they were made.
    pub label_cells: Vec<NodeLabelCell>,
    /// `portPortSpacing`.
    pub port_port_spacing: f64,
    /// `labelCellSpacing`: twice the label-label spacing, the gap of the inside label grid.
    pub label_cell_spacing: f64,
    /// `surroundingPortMargins`.
    pub surrounding_port_margins: Spacing,
    /// The alignment of each side, indexed by `PortSide` ordinal (`getPortAlignment`).
    alignments: [PortAlignment; 5],
    /// `portContexts`, in the order of the node's ports.
    pub port_contexts: Vec<PortContext>,
    /// The cell system of the node: the node container, its middle row and the inside label grid.
    pub cells: CellSystem,
    pub node_container: CellId,
    north: CellId,
    east: CellId,
    south: CellId,
    west: CellId,
}

/// An `EnumSet` option of a node. A missing value reads as `default`: `melk_default_value` does
/// not read the `EnumSet.of(...)` defaults, so the defaults are given here.
pub(super) fn enum_set<E: JavaEnum>(
    props: &PropertyHolder,
    id: &str,
    default: &[E],
) -> Result<Vec<E>, LayoutError> {
    match props.get(id) {
        Some(OptionValue::EnumSet { names, .. }) => names
            .iter()
            .map(|name| {
                E::from_java_text(name)
                    .ok_or_else(|| LayoutError::Format(format!("{name} is not a constant of {id}")))
            })
            .collect(),
        Some(_) => Err(LayoutError::Format(format!("{id} is not an enum set"))),
        None => Ok(default.to_vec()),
    }
}

/// A spacing option of the graph, with its default. The node's own individual spacings are
/// rejected before this is called, so the value is the graph's (`getIndividualOrInherited`).
fn spacing_option(props: &PropertyHolder, id: &str) -> Result<Spacing, LayoutError> {
    match props.value_or_default(id) {
        Some(OptionValue::Margin(spacing) | OptionValue::Padding(spacing)) => Ok(spacing),
        _ => Err(LayoutError::Format(format!(
            "{id} is not a margin or padding"
        ))),
    }
}

/// A number option of the graph, with its default.
fn number_option(props: &PropertyHolder, id: &str) -> Result<f64, LayoutError> {
    props
        .double_of(id)
        .ok_or_else(|| LayoutError::Format(format!("{id} is not a number")))
}

/// A port alignment option of a node; `None` when the node does not set it.
fn alignment_option(
    props: &PropertyHolder,
    id: &str,
) -> Result<Option<PortAlignment>, LayoutError> {
    match props.get(id) {
        None => Ok(None),
        Some(OptionValue::Enum { name, .. }) => PortAlignment::from_java_text(name)
            .map(Some)
            .ok_or_else(|| LayoutError::Format(format!("{name} is not a port alignment"))),
        Some(_) => Err(LayoutError::Format(format!("{id} is not a port alignment"))),
    }
}

/// `PortLabelPlacement.isValid`: at most one of INSIDE and OUTSIDE, and at most one of the three
/// position options.
fn port_label_placement_is_valid(placement: &[PortLabelPlacement]) -> bool {
    let inside_or_outside = placement
        .iter()
        .filter(|p| matches!(p, PortLabelPlacement::INSIDE | PortLabelPlacement::OUTSIDE))
        .count();
    let position = placement
        .iter()
        .filter(|p| {
            matches!(
                p,
                PortLabelPlacement::ALWAYS_SAME_SIDE
                    | PortLabelPlacement::ALWAYS_OTHER_SAME_SIDE
                    | PortLabelPlacement::SPACE_EFFICIENT
            )
        })
        .count();
    inside_or_outside <= 1 && position <= 1
}

/// `NodeLabelPlacement.isValid`: at most one of INSIDE and OUTSIDE, one horizontal and one
/// vertical position.
fn node_label_placement_is_valid(placement: &[NodeLabelPlacement]) -> bool {
    let count =
        |group: &[NodeLabelPlacement]| placement.iter().filter(|p| group.contains(*p)).count();
    count(&[NodeLabelPlacement::INSIDE, NodeLabelPlacement::OUTSIDE]) <= 1
        && count(&[
            NodeLabelPlacement::H_LEFT,
            NodeLabelPlacement::H_CENTER,
            NodeLabelPlacement::H_RIGHT,
        ]) <= 1
        && count(&[
            NodeLabelPlacement::V_TOP,
            NodeLabelPlacement::V_CENTER,
            NodeLabelPlacement::V_BOTTOM,
        ]) <= 1
}

fn unsupported(text: impl Into<String>) -> LayoutError {
    LayoutError::Unsupported(text.into())
}

impl NodeContext {
    /// `new NodeContext(graph, node)` followed by `PortContextCreator.createPortContexts` and the
    /// creation of the cells in `NodeLabelCellCreator` and `InsidePortLabelCellCreator`.
    ///
    /// Fails before building anything for the input the reduced path does not port, and for the
    /// configurations that the Java rejects.
    pub fn new(graph: &LGraph, node: usize) -> Result<NodeContext, LayoutError> {
        let lnode = &graph.nodes[node];
        let props = &lnode.properties;
        let who = &lnode.designation;

        // `getIndividualOrInherited` would read the node's own spacings first; they are not ported.
        if props.has(SPACING_INDIVIDUAL) {
            return Err(unsupported(format!("individual spacings on {who}")));
        }

        // Size constraints: MINIMUM_SIZE and NODE_LABELS are ported. PORTS and PORT_LABELS need the
        // port label cells, which are not ported.
        let size_constraints =
            enum_set::<SizeConstraint>(props, options::NODE_SIZE_CONSTRAINTS, &[])?;
        if let Some(constraint) = size_constraints.iter().find(|constraint| {
            !matches!(
                constraint,
                SizeConstraint::MINIMUM_SIZE | SizeConstraint::NODE_LABELS
            )
        }) {
            return Err(unsupported(format!(
                "size constraint {} on {who}",
                constraint.java_name()
            )));
        }

        // FIXED_ORDER takes the free branch of every Java switch on the constraints, so it is
        // handled like FREE. FIXED_POS has its own branches, which are ported. FIXED_RATIO is not.
        let port_constraints = props
            .enum_of::<PortConstraints>(options::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::UNDEFINED);
        if port_constraints == PortConstraints::FIXED_RATIO {
            return Err(unsupported(format!(
                "port constraints {} on {who}",
                port_constraints.java_name()
            )));
        }

        // `PortLabelPlacement.isValid` throws for the invalid combinations; a placement without
        // INSIDE or OUTSIDE is the fixed placement, whose positions are not ported.
        let port_label_placement = enum_set::<PortLabelPlacement>(
            props,
            PORT_LABELS_PLACEMENT,
            &[PortLabelPlacement::OUTSIDE],
        )?;
        if !port_label_placement_is_valid(&port_label_placement) {
            return Err(LayoutError::Format(format!(
                "invalid port label placement on {who}"
            )));
        }
        if !port_label_placement.contains(&PortLabelPlacement::INSIDE)
            && !port_label_placement.contains(&PortLabelPlacement::OUTSIDE)
        {
            return Err(unsupported(format!("fixed port label placement on {who}")));
        }

        // The default, `NodeLabelPlacement.fixed()`, is the empty set, which is valid: only a set
        // value needs the check.
        let node_label_placement =
            enum_set::<NodeLabelPlacement>(props, NODE_LABELS_PLACEMENT, &[])?;
        if !node_label_placement_is_valid(&node_label_placement) {
            return Err(LayoutError::Format(format!(
                "invalid node label placement on {who}"
            )));
        }

        let size_options = enum_set::<SizeOptions>(
            props,
            options::NODE_SIZE_OPTIONS,
            &[SizeOptions::DEFAULT_MINIMUM_SIZE],
        )?;
        if size_options.contains(&SizeOptions::COMPUTE_PADDING) {
            return Err(unsupported(format!("COMPUTE_PADDING on {who}")));
        }
        // `getMinimumClientAreaSize` and the centre cell's minimum size are not ported.
        if size_options.contains(&SizeOptions::MINIMUM_SIZE_ACCOUNTS_FOR_PADDING)
            && size_constraints.contains(&SizeConstraint::MINIMUM_SIZE)
        {
            return Err(unsupported(format!(
                "MINIMUM_SIZE_ACCOUNTS_FOR_PADDING on {who}"
            )));
        }
        // `getMinimumNodeOrClientAreaSize`: the node's minimum size, with the defaults.
        let minimum_size = size_constraints
            .contains(&SizeConstraint::MINIMUM_SIZE)
            .then(|| {
                let mut minimum = match props.value_or_default(options::NODE_SIZE_MINIMUM) {
                    Some(OptionValue::Vector(vector)) => vector,
                    _ => KVector::default(),
                };
                if size_options.contains(&SizeOptions::DEFAULT_MINIMUM_SIZE) {
                    if minimum.x <= 0.0 {
                        minimum.x = DEFAULT_MIN_WIDTH;
                    }
                    if minimum.y <= 0.0 {
                        minimum.y = DEFAULT_MIN_HEIGHT;
                    }
                }
                minimum
            });

        // The spacings come from the graph, with their defaults (`IndividualSpacings`).
        let graph_props = &graph.properties;
        let surrounding_port_margins = spacing_option(graph_props, SPACING_PORTS_SURROUNDING)?;
        let port_port_spacing = number_option(graph_props, SPACING_PORT_PORT)?;
        let port_label_spacing_vertical = number_option(graph_props, SPACING_LABEL_PORT_VERTICAL)?;
        let label_label_spacing = number_option(graph_props, options::SPACING_LABEL_LABEL)?;
        let label_cell_spacing = 2.0 * label_label_spacing;
        let node_labels_padding = spacing_option(graph_props, NODE_LABELS_PADDING)?;
        let fixed_graph_size = graph_props.bool_of(options::NODE_SIZE_FIXED_GRAPH_SIZE);

        // `getPortAlignment(side)`: the side's own alignment, else the node's default.
        let default_alignment =
            alignment_option(props, PORT_ALIGNMENT_DEFAULT)?.unwrap_or(DEFAULT_PORT_ALIGNMENT);
        let mut alignments = [default_alignment; 5];
        for (side, id) in [
            (PortSide::NORTH, PORT_ALIGNMENT_NORTH),
            (PortSide::EAST, PORT_ALIGNMENT_EAST),
            (PortSide::SOUTH, PORT_ALIGNMENT_SOUTH),
            (PortSide::WEST, PORT_ALIGNMENT_WEST),
        ] {
            if let Some(alignment) = alignment_option(props, id)? {
                alignments[side as usize] = alignment;
            }
        }

        // `NodeContext` constructor: the main cells. The node container stacks the top
        // row, the middle row and the bottom row; the middle row holds the west and east cells
        // around the inside label grid.
        let symmetry = !size_options.contains(&SizeOptions::ASYMMETRICAL);
        let tabular = size_options.contains(&SizeOptions::FORCE_TABULAR_NODE_LABELS);
        let mut cells = CellSystem::default();
        let node_container = cells.strip(Strip::Vertical, symmetry, 0.0);
        let middle_row = cells.strip(Strip::Horizontal, symmetry, 0.0);
        cells.set_strip_cell(node_container, Area::Center, middle_row);

        // `NodeLabelCellCreator.createNodeLabelCellContainers`: the inside label grid. The label cells
        // go into it in `labels::create_node_label_cells`. The outside label containers are not made:
        // they are only needed for outside labels, which are not ported.
        let inside_container = cells.grid(tabular, symmetry, label_cell_spacing);
        *cells.padding_mut(inside_container) = node_labels_padding;
        cells.set_strip_cell(middle_row, Area::Center, inside_container);

        // `InsidePortLabelCellCreator.createInsidePortLabelCells`: one cell per side.
        let north = cells.atomic();
        cells.set_strip_cell(node_container, Area::Begin, north);
        let south = cells.atomic();
        cells.set_strip_cell(node_container, Area::End, south);
        let west = cells.atomic();
        cells.set_strip_cell(middle_row, Area::Begin, west);
        let east = cells.atomic();
        cells.set_strip_cell(middle_row, Area::End, east);

        // `setupNorthOrSouthPortLabelCell`: the vertical label spacing, if it is not negative, and
        // the surrounding margins on the sides.
        if port_label_spacing_vertical >= 0.0 {
            cells.padding_mut(north).top = port_label_spacing_vertical;
        }
        {
            let padding = cells.padding_mut(north);
            padding.left = surrounding_port_margins.left;
            padding.right = surrounding_port_margins.right;
        }
        if port_label_spacing_vertical >= 0.0 {
            cells.padding_mut(south).bottom = port_label_spacing_vertical;
        }
        {
            let padding = cells.padding_mut(south);
            padding.left = surrounding_port_margins.left;
            padding.right = surrounding_port_margins.right;
        }
        // `setupEastOrWestPortLabelCell`: the width due to labels is zero without labels, so the
        // east and west padding only takes the surrounding margins above and below.
        for cell in [west, east] {
            let padding = cells.padding_mut(cell);
            padding.top = surrounding_port_margins.top;
            padding.bottom = surrounding_port_margins.bottom;
        }

        // `PortContextCreator.createPortContexts`: the volatile id is the port's place in the list.
        let mut port_contexts = Vec::with_capacity(lnode.ports.len());
        for (volatile_id, &port) in lnode.ports.iter().enumerate() {
            let lport = &graph.ports[port];
            if lport.side == PortSide::UNDEFINED {
                return Err(LayoutError::Format(format!(
                    "the node size calculation needs port sides, and a port of {who} has none"
                )));
            }
            port_contexts.push(PortContext {
                port,
                side: lport.side,
                volatile_id,
                size: lport.size,
                position: lport.position,
                margin: Spacing::default(),
            });
        }

        Ok(NodeContext {
            node_size: lnode.size,
            fixed_graph_size,
            port_constraints,
            size_options,
            size_constraints,
            minimum_size,
            node_label_placement,
            label_label_spacing,
            inside_label_grid: inside_container,
            middle_row,
            label_cells: Vec::new(),
            port_port_spacing,
            label_cell_spacing,
            surrounding_port_margins,
            alignments,
            port_contexts,
            cells,
            node_container,
            north,
            east,
            south,
            west,
        })
    }

    pub fn has_size_option(&self, option: SizeOptions) -> bool {
        self.size_options.contains(&option)
    }

    /// `getPortAlignment(side)`.
    pub fn alignment(&self, side: PortSide) -> PortAlignment {
        self.alignments[side as usize]
    }

    /// The inside port label cell of a side: `insidePortLabelCells.get(side)`.
    pub fn inside_cell(&self, side: PortSide) -> CellId {
        match side {
            PortSide::NORTH => self.north,
            PortSide::EAST => self.east,
            PortSide::SOUTH => self.south,
            PortSide::WEST => self.west,
            PortSide::UNDEFINED => {
                unreachable!("ports without a side are rejected in NodeContext::new")
            }
        }
    }

    /// The indices into `port_contexts` of the ports on `side`, in the order of the Java
    /// `portContexts.get(side)`: north and east by ascending volatile id, south and west by
    /// descending volatile id. The volatile ids are unique, so the order is total.
    pub fn port_contexts_on(&self, side: PortSide) -> Vec<usize> {
        let mut found: Vec<usize> = (0..self.port_contexts.len())
            .filter(|&index| self.port_contexts[index].side == side)
            .collect();
        let ascending = matches!(side, PortSide::NORTH | PortSide::EAST);
        found.sort_by(|&a, &b| {
            let (x, y) = (
                self.port_contexts[a].volatile_id,
                self.port_contexts[b].volatile_id,
            );
            if ascending { x.cmp(&y) } else { y.cmp(&x) }
        });
        found
    }
}
