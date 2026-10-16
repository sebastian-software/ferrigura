//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/NodeLabelLocation.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/NodeLabelCellCreator.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/CellSystemConfigurator.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/LabelPlacer.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/NodeLabelAndSizeCalculator.java
//!
//! Node labels: their locations, the label cells that hold them in the inside label grid, the size
//! contributions that NODE_LABELS adds, their placement, and the padding they take inside a node.
//!
//! The outside locations are in the table, so a label placed outside is found and reported, not
//! placed wrongly. Their containers are not ported, so [`create_node_label_cells`] refuses them when
//! the outside labels are needed.

use crate::core::data::enums::{NodeLabelPlacement, SizeConstraint};
use crate::core::math::{KVector, Spacing};
use crate::layered::LayoutError;
use crate::layered::lgraph::{LGraph, LabelId};

use super::cells::{Area, CellId, HorizontalAlignment, VerticalAlignment};
use super::context::{NODE_LABELS_PLACEMENT, NodeContext, enum_set};
use super::java_max;

use NodeLabelPlacement as P;

/// One row of `NodeLabelLocation`: the cell a label of this location goes into, and how it aligns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Location {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
    /// `getContainerRow()` and `getContainerColumn()`.
    pub row: Area,
    pub column: Area,
    pub inside: bool,
    /// The placements of the location, without `H_PRIORITY`. The table also has the variant with
    /// `H_PRIORITY`, which [`Location::matches`] accepts.
    placements: [NodeLabelPlacement; 3],
}

const fn location(
    horizontal: HorizontalAlignment,
    vertical: VerticalAlignment,
    row: Area,
    column: Area,
    inside: bool,
    placements: [NodeLabelPlacement; 3],
) -> Location {
    Location {
        horizontal,
        vertical,
        row,
        column,
        inside,
        placements,
    }
}

use HorizontalAlignment::{Center as HC, Left as HL, Right as HR};
use VerticalAlignment::{Bottom as VB, Center as VC, Top as VT};

/// `NodeLabelLocation`, in the order of the Java constants. The placements are the ones the Java
/// `EnumSet`s hold, and the positions and alignments are the Java arguments.
static LOCATIONS: [Location; 21] = [
    location(
        HL,
        VB,
        Area::Begin,
        Area::Begin,
        false,
        [P::OUTSIDE, P::V_TOP, P::H_LEFT],
    ),
    location(
        HC,
        VB,
        Area::Begin,
        Area::Center,
        false,
        [P::OUTSIDE, P::V_TOP, P::H_CENTER],
    ),
    location(
        HR,
        VB,
        Area::Begin,
        Area::End,
        false,
        [P::OUTSIDE, P::V_TOP, P::H_RIGHT],
    ),
    location(
        HL,
        VT,
        Area::End,
        Area::Begin,
        false,
        [P::OUTSIDE, P::V_BOTTOM, P::H_LEFT],
    ),
    location(
        HC,
        VT,
        Area::End,
        Area::Center,
        false,
        [P::OUTSIDE, P::V_BOTTOM, P::H_CENTER],
    ),
    location(
        HR,
        VT,
        Area::End,
        Area::End,
        false,
        [P::OUTSIDE, P::V_BOTTOM, P::H_RIGHT],
    ),
    location(
        HR,
        VT,
        Area::Begin,
        Area::Begin,
        false,
        [P::OUTSIDE, P::H_LEFT, P::V_TOP],
    ),
    location(
        HR,
        VC,
        Area::Center,
        Area::Begin,
        false,
        [P::OUTSIDE, P::H_LEFT, P::V_CENTER],
    ),
    location(
        HR,
        VB,
        Area::End,
        Area::Begin,
        false,
        [P::OUTSIDE, P::H_LEFT, P::V_BOTTOM],
    ),
    location(
        HL,
        VT,
        Area::Begin,
        Area::End,
        false,
        [P::OUTSIDE, P::H_RIGHT, P::V_TOP],
    ),
    location(
        HL,
        VC,
        Area::Center,
        Area::End,
        false,
        [P::OUTSIDE, P::H_RIGHT, P::V_CENTER],
    ),
    location(
        HL,
        VB,
        Area::End,
        Area::End,
        false,
        [P::OUTSIDE, P::H_RIGHT, P::V_BOTTOM],
    ),
    location(
        HL,
        VT,
        Area::Begin,
        Area::Begin,
        true,
        [P::INSIDE, P::V_TOP, P::H_LEFT],
    ),
    location(
        HC,
        VT,
        Area::Begin,
        Area::Center,
        true,
        [P::INSIDE, P::V_TOP, P::H_CENTER],
    ),
    location(
        HR,
        VT,
        Area::Begin,
        Area::End,
        true,
        [P::INSIDE, P::V_TOP, P::H_RIGHT],
    ),
    location(
        HL,
        VC,
        Area::Center,
        Area::Begin,
        true,
        [P::INSIDE, P::V_CENTER, P::H_LEFT],
    ),
    location(
        HC,
        VC,
        Area::Center,
        Area::Center,
        true,
        [P::INSIDE, P::V_CENTER, P::H_CENTER],
    ),
    location(
        HR,
        VC,
        Area::Center,
        Area::End,
        true,
        [P::INSIDE, P::V_CENTER, P::H_RIGHT],
    ),
    location(
        HL,
        VB,
        Area::End,
        Area::Begin,
        true,
        [P::INSIDE, P::V_BOTTOM, P::H_LEFT],
    ),
    location(
        HC,
        VB,
        Area::End,
        Area::Center,
        true,
        [P::INSIDE, P::V_BOTTOM, P::H_CENTER],
    ),
    location(
        HR,
        VB,
        Area::End,
        Area::End,
        true,
        [P::INSIDE, P::V_BOTTOM, P::H_RIGHT],
    ),
];

impl Location {
    /// `assignedPlacements.contains(set)`: the set is the location's three placements, with or
    /// without `H_PRIORITY`.
    fn matches(&self, placement: &[NodeLabelPlacement]) -> bool {
        let mut distinct = placement.to_vec();
        distinct.sort();
        distinct.dedup();
        let has_all = self
            .placements
            .iter()
            .all(|wanted| distinct.contains(wanted));
        has_all
            && (distinct.len() == 3 || (distinct.len() == 4 && distinct.contains(&P::H_PRIORITY)))
    }
}

/// `NodeLabelLocation.fromNodeLabelPlacement`: the location a placement names, or `None` for
/// UNDEFINED, which is the case of a placement that names no location.
pub(super) fn location_of(placement: &[NodeLabelPlacement]) -> Option<&'static Location> {
    LOCATIONS
        .iter()
        .find(|location| location.matches(placement))
}

/// A label cell and the labels in it, in the order they were added. `NodeContext.nodeLabelCells`.
#[derive(Debug, Clone)]
pub(super) struct NodeLabelCell {
    pub location: &'static Location,
    pub cell: CellId,
    /// The labels of the cell, as ids in `LGraph::labels`, with their sizes.
    pub labels: Vec<(LabelId, KVector)>,
}

/// `NodeLabelCellCreator.createNodeLabelCells` for the labels of `node` in `graph`. A label goes to
/// the cell of its location. `only_inside` leaves the outside labels out, as `computeInsideNodeLabelPadding`
/// does; without it, an outside label is refused, since its containers are not ported.
pub(super) fn create_node_label_cells(
    ctx: &mut NodeContext,
    graph: &LGraph,
    node: usize,
    only_inside: bool,
    horizontal_layout: bool,
) -> Result<(), LayoutError> {
    for &label in &graph.nodes[node].labels {
        let label_props = &graph.labels[label].properties;
        // `label.hasProperty(NODE_LABELS_PLACEMENT) ? label's : nodeContext.nodeLabelPlacement`.
        let placement = if label_props.has(NODE_LABELS_PLACEMENT) {
            enum_set::<NodeLabelPlacement>(label_props, NODE_LABELS_PLACEMENT, &[])?
        } else {
            ctx.node_label_placement.clone()
        };
        let Some(location) = location_of(&placement) else {
            continue;
        };
        if only_inside && !location.inside {
            continue;
        }
        if !location.inside {
            return Err(LayoutError::Unsupported(
                "outside node labels (their containers are not ported)".to_string(),
            ));
        }
        let size = graph.labels[label].size;
        let index = match ctx
            .label_cells
            .iter()
            .position(|entry| entry.location == location)
        {
            Some(index) => index,
            None => {
                // `retrieveNodeLabelCell`: a new cell, placed in its grid position.
                let cell = ctx.cells.label(
                    ctx.label_label_spacing,
                    horizontal_layout,
                    location.horizontal,
                    location.vertical,
                );
                ctx.cells
                    .set_grid_cell(ctx.inside_label_grid, location.row, location.column, cell);
                ctx.label_cells.push(NodeLabelCell {
                    location,
                    cell,
                    labels: Vec::new(),
                });
                ctx.label_cells.len() - 1
            }
        };
        let entry = &mut ctx.label_cells[index];
        ctx.cells.add_label(entry.cell, size);
        entry.labels.push((label, size));
    }
    Ok(())
}

/// `CellSystemConfigurator.configureCellSystemSizeContributions`, for MINIMUM_SIZE (which adds no
/// contribution here, see `NodeContext::new`) and NODE_LABELS.
pub(super) fn configure_size_contributions(ctx: &mut NodeContext) {
    if !ctx.size_constraints.contains(&SizeConstraint::NODE_LABELS) {
        return;
    }
    ctx.cells.set_contributes(ctx.inside_label_grid, true, true);
    ctx.cells.set_contributes(ctx.middle_row, true, true);
    // The inside labels contribute in both directions. Outside labels cannot be here (see above).
    let cells: Vec<CellId> = ctx.label_cells.iter().map(|entry| entry.cell).collect();
    for cell in cells {
        ctx.cells.set_contributes(cell, true, true);
    }
}

/// `LabelPlacer.placeLabels` for the inside label cells: each label's top left corner, relative to
/// the node, with its id. The outside containers are not made, so they are not placed.
pub(super) fn place_labels(ctx: &NodeContext) -> Vec<(LabelId, KVector)> {
    let mut placed = Vec::new();
    for entry in &ctx.label_cells {
        let sizes: Vec<KVector> = entry.labels.iter().map(|(_, size)| *size).collect();
        let positions = ctx.cells.label_positions(entry.cell, &sizes);
        for ((label, _), position) in entry.labels.iter().zip(positions) {
            placed.push((*label, position));
        }
    }
    placed
}

/// `NodeLabelAndSizeCalculator.computeInsideNodeLabelPadding` for the node in `probe`: the space
/// its inside labels need on each side, from the label cells of the inside grid. `probe` holds the
/// node alone, with the spacings of the node's parent (or of none, for the root).
pub(super) fn inside_label_padding(
    probe: &LGraph,
    node: usize,
    horizontal_layout: bool,
) -> Result<Spacing, LayoutError> {
    let mut ctx = NodeContext::new(probe, node)?;
    create_node_label_cells(&mut ctx, probe, node, true, horizontal_layout)?;
    let grid = ctx.inside_label_grid;
    let cells = &ctx.cells;
    let areas = [Area::Begin, Area::Center, Area::End];
    let mut padding = Spacing::default();
    // Top and bottom: the cells of the first and of the last row, by column.
    for column in areas {
        if let Some(cell) = cells.grid_cell(grid, Area::Begin, column) {
            padding.top = java_max(padding.top, cells.min_height(cell));
        }
        if let Some(cell) = cells.grid_cell(grid, Area::End, column) {
            padding.bottom = java_max(padding.bottom, cells.min_height(cell));
        }
    }
    // Left and right: the cells of the first and of the last column, by row.
    for row in areas {
        if let Some(cell) = cells.grid_cell(grid, row, Area::Begin) {
            padding.left = java_max(padding.left, cells.min_width(cell));
        }
        if let Some(cell) = cells.grid_cell(grid, row, Area::End) {
            padding.right = java_max(padding.right, cells.min_width(cell));
        }
    }
    // The insets and the gap are added where a side has a label.
    let grid_padding = cells.padding(grid);
    let gap = ctx.label_cell_spacing;
    if padding.top > 0.0 {
        padding.top += grid_padding.top;
        padding.top += gap;
    }
    if padding.bottom > 0.0 {
        padding.bottom += grid_padding.bottom;
        padding.bottom += gap;
    }
    if padding.left > 0.0 {
        padding.left += grid_padding.left;
        padding.left += gap;
    }
    if padding.right > 0.0 {
        padding.right += grid_padding.right;
        padding.right += gap;
    }
    Ok(padding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::OptionValue;
    use crate::core::data::property::PropertyHolder;
    use crate::layered::options;

    /// `H_CENTER V_TOP INSIDE` is the inside top-centred location, with or without `H_PRIORITY`.
    #[test]
    fn the_inside_top_centre_location_is_found_with_or_without_h_priority() {
        let plain = [P::H_CENTER, P::V_TOP, P::INSIDE];
        let with_priority = [P::H_CENTER, P::V_TOP, P::INSIDE, P::H_PRIORITY];
        for placement in [&plain[..], &with_priority[..]] {
            let location = location_of(placement).expect("a location");
            assert!(location.inside);
            assert_eq!(location.horizontal, HorizontalAlignment::Center);
            assert_eq!(location.vertical, VerticalAlignment::Top);
            assert_eq!(location.row, Area::Begin);
            assert_eq!(location.column, Area::Center);
        }
        assert!(location_of(&[]).is_none());
        assert!(location_of(&[P::H_CENTER, P::INSIDE]).is_none());
    }

    /// The transposed placement of `H_CENTER V_TOP INSIDE` is `H_LEFT V_CENTER INSIDE`, with
    /// `H_PRIORITY` added, as `GraphTransformer.transposeNodeLabelPlacement` builds it.
    #[test]
    fn the_transposed_placement_swaps_the_axes_and_sets_h_priority() {
        let mut properties = PropertyHolder::new();
        properties.set(
            options::NODE_LABELS_PLACEMENT,
            Some(OptionValue::EnumSet {
                class: "NodeLabelPlacement",
                names: vec!["H_CENTER".into(), "V_TOP".into(), "INSIDE".into()],
            }),
        );
        crate::layered::graph_transformer::transpose_node_label_placement(&mut properties);
        assert!(properties.enum_set_contains(options::NODE_LABELS_PLACEMENT, "H_LEFT"));
        assert!(properties.enum_set_contains(options::NODE_LABELS_PLACEMENT, "V_CENTER"));
        assert!(properties.enum_set_contains(options::NODE_LABELS_PLACEMENT, "H_PRIORITY"));
        assert!(properties.enum_set_contains(options::NODE_LABELS_PLACEMENT, "INSIDE"));
        assert!(!properties.enum_set_contains(options::NODE_LABELS_PLACEMENT, "V_TOP"));
    }
}
