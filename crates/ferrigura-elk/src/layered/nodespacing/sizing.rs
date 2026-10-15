//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/NodeLabelAndSizeCalculator.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/HorizontalPortPlacementSizeCalculator.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/VerticalPortPlacementSizeCalculator.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/PortPlacementCalculator.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/NodeSizeCalculator.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/CellSystemConfigurator.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/algorithm/NodeLabelAndSizeUtilities.java
//!
//! `NodeLabelAndSizeCalculator.process(graph, node, applyStuff = true, ignoreInsidePortLabels =
//! false)` for one node. Ports with the constraints FREE, FIXED_SIDE, FIXED_ORDER and UNDEFINED
//! take the free branch of every Java switch on the constraints. FIXED_POS has its own branches,
//! which are ported here. FIXED_RATIO is rejected in `NodeContext::new`.
//!
//! The phases follow the Java. Some steps do nothing on this path and are left out, with a note
//! where they are: the minimum client area (no MINIMUM_SIZE), the padding for ports with a border
//! offset (no port carries one), the port label steps (no labels), the cell system's contribution
//! flags (no size constraints), `LabelPlacer` (its only effect is on label cells and on the
//! outside containers, which are not made) and the node padding (COMPUTE_PADDING is rejected).

use crate::core::data::enums::{PortAlignment, PortConstraints, PortSide, SizeOptions};
use crate::core::math::KVector;
use crate::layered::LayoutError;
use crate::layered::lgraph::LGraph;

use super::context::{NodeContext, PortContext};
use super::java_max;

/// What the size calculation gives one node: its size, and the position of each of its ports.
pub(super) struct NodeResult {
    /// Index into `LGraph::nodes`.
    pub node: usize,
    pub size: KVector,
    /// Index into `LGraph::ports`, with the position the port gets.
    pub ports: Vec<(usize, KVector)>,
}

/// `NodeLabelAndSizeCalculator.process(graph, node, true, false)`, without changing the graph.
pub(super) fn calculate(graph: &LGraph, node: usize) -> Result<NodeResult, LayoutError> {
    let mut ctx = NodeContext::new(graph, node)?;

    // Phase 3: the inside port label cells need the room their ports take.
    horizontal_port_placement_size(&mut ctx);
    vertical_port_placement_size(&mut ctx);

    // Phase 4 (CellSystemConfigurator) returns at once without size constraints.

    // Phase 5: the width, then the horizontal ports.
    set_node_width(&mut ctx);
    place_horizontal_ports(&mut ctx);

    // Phase 6: the height, then the vertical ports. The southern ports are offset by the height.
    update_vertical_inside_port_label_cell_padding(&mut ctx);
    set_node_height(&mut ctx);
    offset_southern_ports(&mut ctx);
    place_vertical_ports(&mut ctx);

    // Phase 7: applyStuff. The node size is applied, and every port gets its position.
    Ok(NodeResult {
        node,
        size: ctx.node_size,
        ports: ctx
            .port_contexts
            .iter()
            .map(|port| (port.port, port.position))
            .collect(),
    })
}

/// The space that the ports on one side take along that side, with the spacing between them:
/// `portWidthPlusPortPortSpacing` (horizontal) and `portHeightPlusPortPortSpacing` (vertical).
fn span_with_spacing(ctx: &NodeContext, contexts: &[usize], horizontal: bool) -> f64 {
    let mut result = 0.0;
    for (position, &index) in contexts.iter().enumerate() {
        let port = &ctx.port_contexts[index];
        result += if horizontal {
            port.margin.left + port.size.x + port.margin.right
        } else {
            port.margin.top + port.size.y + port.margin.bottom
        };
        if position + 1 < contexts.len() {
            result += ctx.port_port_spacing;
        }
    }
    result
}

/// `HorizontalPortPlacementSizeCalculator.calculateHorizontalPortPlacementSize`: the northern and
/// southern inside cells get the width their ports need.
fn horizontal_port_placement_size(ctx: &mut NodeContext) {
    for side in [PortSide::NORTH, PortSide::SOUTH] {
        if ctx.port_constraints == PortConstraints::FIXED_POS {
            fixed_pos_horizontal_size(ctx, side);
        } else {
            free_horizontal_size(ctx, side);
        }
    }
}

/// `calculateHorizontalNodeSizeRequiredByFreePorts`.
fn free_horizontal_size(ctx: &mut NodeContext, side: PortSide) {
    let cell = ctx.inside_cell(side);
    let contexts = ctx.port_contexts_on(side);
    if contexts.is_empty() {
        let padding = ctx.cells.padding_mut(cell);
        padding.left = 0.0;
        padding.right = 0.0;
        return;
    }
    let surrounding = ctx.surrounding_port_margins;
    {
        let padding = ctx.cells.padding_mut(cell);
        padding.left = surrounding.left;
        padding.right = surrounding.right;
    }
    let mut width = span_with_spacing(ctx, &contexts, true);
    if ctx.alignment(side) == PortAlignment::DISTRIBUTED {
        width += 2.0 * ctx.port_port_spacing;
    }
    ctx.cells.min_content_mut(cell).x = width;
}

/// `calculateHorizontalNodeSizeRequiredByFixedPosPorts`: the cell reaches the rightmost port
/// border, and its left padding is zero. The ports keep the x they were given.
fn fixed_pos_horizontal_size(ctx: &mut NodeContext, side: PortSide) {
    let cell = ctx.inside_cell(side);
    let mut rightmost_port_border = 0.0;
    for index in ctx.port_contexts_on(side) {
        let port = &ctx.port_contexts[index];
        rightmost_port_border = java_max(rightmost_port_border, port.position.x + port.size.x);
    }
    ctx.cells.padding_mut(cell).left = 0.0;
    ctx.cells.min_content_mut(cell).x = rightmost_port_border;
}

/// `VerticalPortPlacementSizeCalculator.calculateVerticalPortPlacementSize`: the eastern and
/// western inside cells get the height their ports need.
fn vertical_port_placement_size(ctx: &mut NodeContext) {
    for side in [PortSide::EAST, PortSide::WEST] {
        if ctx.port_constraints == PortConstraints::FIXED_POS {
            fixed_pos_vertical_size(ctx, side);
        } else {
            free_vertical_size(ctx, side);
        }
    }
}

/// `calculateVerticalNodeSizeRequiredByFreePorts`.
fn free_vertical_size(ctx: &mut NodeContext, side: PortSide) {
    let cell = ctx.inside_cell(side);
    let contexts = ctx.port_contexts_on(side);
    if contexts.is_empty() {
        let padding = ctx.cells.padding_mut(cell);
        padding.top = 0.0;
        padding.bottom = 0.0;
        return;
    }
    let surrounding = ctx.surrounding_port_margins;
    {
        let padding = ctx.cells.padding_mut(cell);
        padding.top = surrounding.top;
        padding.bottom = surrounding.bottom;
    }
    let mut height = span_with_spacing(ctx, &contexts, false);
    if ctx.alignment(side) == PortAlignment::DISTRIBUTED {
        height += 2.0 * ctx.port_port_spacing;
    }
    ctx.cells.min_content_mut(cell).y = height;
}

/// `calculateVerticalNodeSizeRequiredByFixedPosPorts`: the cell reaches the bottommost port
/// border, and its top padding is zero. The ports keep the y they were given.
fn fixed_pos_vertical_size(ctx: &mut NodeContext, side: PortSide) {
    let cell = ctx.inside_cell(side);
    let mut bottommost_port_border = 0.0;
    for index in ctx.port_contexts_on(side) {
        let port = &ctx.port_contexts[index];
        bottommost_port_border = java_max(bottommost_port_border, port.position.y + port.size.y);
    }
    ctx.cells.padding_mut(cell).top = 0.0;
    ctx.cells.min_content_mut(cell).y = bottommost_port_border;
}

/// `NodeSizeCalculator.setNodeWidth` without size constraints: the node keeps its width (and the
/// graph's fixed size only lets it grow), and the node container lays its cells out across it.
fn set_node_width(ctx: &mut NodeContext) {
    let width = ctx.node_size.x;
    ctx.node_size.x = if ctx.fixed_graph_size {
        java_max(ctx.node_size.x, width)
    } else {
        width
    };
    let rect = ctx.cells.rect_mut(ctx.node_container);
    rect.x = 0.0;
    rect.width = width;
    ctx.cells.layout_horizontally(ctx.node_container);
}

/// `NodeSizeCalculator.setNodeHeight` without size constraints, like the width.
fn set_node_height(ctx: &mut NodeContext) {
    let height = ctx.node_size.y;
    ctx.node_size.y = if ctx.fixed_graph_size {
        java_max(ctx.node_size.y, height)
    } else {
        height
    };
    let rect = ctx.cells.rect_mut(ctx.node_container);
    rect.y = 0.0;
    rect.height = height;
    ctx.cells.layout_vertically(ctx.node_container);
}

/// `PortPlacementCalculator.placeHorizontalPorts`: north, then south.
fn place_horizontal_ports(ctx: &mut NodeContext) {
    for side in [PortSide::NORTH, PortSide::SOUTH] {
        if ctx.port_constraints == PortConstraints::FIXED_POS {
            place_fixed_pos_horizontal(ctx, side);
        } else {
            place_free_horizontal(ctx, side);
        }
    }
}

/// `PortPlacementCalculator.placeHorizontalFixedPosPorts`: the x of each port is already fixed, so
/// only the y follows the border.
fn place_fixed_pos_horizontal(ctx: &mut NodeContext, side: PortSide) {
    for index in ctx.port_contexts_on(side) {
        let port = &mut ctx.port_contexts[index];
        port.position.y = horizontal_port_y(port);
    }
}

/// `PortPlacementCalculator.placeHorizontalFreePorts`.
fn place_free_horizontal(ctx: &mut NodeContext, side: PortSide) {
    let contexts = ctx.port_contexts_on(side);
    if contexts.is_empty() {
        return;
    }
    let cell = ctx.inside_cell(side);
    let rect = ctx.cells.rect(cell);
    let padding = ctx.cells.padding(cell);
    let placement = ctx.cells.min_content(cell).x;
    let available = rect.width - padding.left - padding.right;
    let (offset, spacing) = spread_ports(
        available,
        placement,
        contexts.len(),
        ctx.alignment(side),
        ctx.port_port_spacing,
        ctx.has_size_option(SizeOptions::PORTS_OVERHANG),
    );
    let mut current = rect.x + padding.left + offset;
    for &index in &contexts {
        let port = &mut ctx.port_contexts[index];
        port.position.x = current + port.margin.left;
        port.position.y = horizontal_port_y(port);
        current += port.margin.left + port.size.x + port.margin.right + spacing;
    }
}

/// The part of `placeHorizontalFreePorts` that decides where the first port starts and how far
/// apart the ports are. Returns the offset of the first port from the start of the free space,
/// and the spacing between two ports. The single-port case and the overhang case both change the
/// alignment or the placement size, as in the Java.
pub(super) fn spread_ports(
    available: f64,
    mut placement: f64,
    count: usize,
    mut alignment: PortAlignment,
    port_port_spacing: f64,
    ports_overhang: bool,
) -> (f64, f64) {
    let count = count as f64;
    let mut offset = 0.0;
    let mut spacing = port_port_spacing;

    // A distributed or justified alignment with a single port becomes centered.
    if matches!(
        alignment,
        PortAlignment::DISTRIBUTED | PortAlignment::JUSTIFIED
    ) && count == 1.0
    {
        placement = modified_placement_size(alignment, port_port_spacing, placement);
        alignment = PortAlignment::CENTER;
    }

    if available < placement && !ports_overhang {
        // Not enough room, and the ports may not overhang: squeeze the spacing.
        if alignment == PortAlignment::DISTRIBUTED {
            spacing += (available - placement) / (count + 1.0);
            offset = spacing;
        } else {
            spacing += (available - placement) / (count - 1.0);
        }
    } else {
        // The ports may overhang. Distributed and justified fall back to centered if they cannot fit.
        if available < placement {
            placement = modified_placement_size(alignment, port_port_spacing, placement);
            alignment = PortAlignment::CENTER;
        }
        match alignment {
            PortAlignment::BEGIN => {}
            PortAlignment::CENTER => offset = (available - placement) / 2.0,
            PortAlignment::END => offset = available - placement,
            PortAlignment::DISTRIBUTED => {
                let additional = (available - placement) / (count + 1.0);
                spacing += java_max(0.0, additional);
                offset = spacing;
            }
            PortAlignment::JUSTIFIED => {
                let additional = (available - placement) / (count - 1.0);
                spacing += java_max(0.0, additional);
            }
        }
    }
    (offset, spacing)
}

/// `modifiedPortPlacementSize`: going from distributed back to centered drops the two spacings
/// that a distributed placement adds.
fn modified_placement_size(alignment: PortAlignment, port_port_spacing: f64, size: f64) -> f64 {
    if alignment == PortAlignment::DISTRIBUTED {
        size - 2.0 * port_port_spacing
    } else {
        size
    }
}

/// `calculateHorizontalPortYCoordinate` without a border offset: northern ports sit above the
/// node, the others on its border.
fn horizontal_port_y(port: &PortContext) -> f64 {
    if port.side == PortSide::NORTH {
        -port.size.y
    } else {
        0.0
    }
}

/// `CellSystemConfigurator.updateVerticalInsidePortLabelCellPadding`: the eastern and western
/// cells share the top and bottom padding, less the room the northern and southern cells take.
/// The Java returns at once for FIXED_POS and FIXED_RATIO, so this does too.
fn update_vertical_inside_port_label_cell_padding(ctx: &mut NodeContext) {
    if matches!(
        ctx.port_constraints,
        PortConstraints::FIXED_POS | PortConstraints::FIXED_RATIO
    ) {
        return;
    }
    let north = ctx.inside_cell(PortSide::NORTH);
    let south = ctx.inside_cell(PortSide::SOUTH);
    let east = ctx.inside_cell(PortSide::EAST);
    let west = ctx.inside_cell(PortSide::WEST);
    let container = ctx.cells.padding(ctx.node_container);
    let top_border_offset = container.top + ctx.cells.min_height(north) + ctx.label_cell_spacing;
    let bottom_border_offset =
        container.bottom + ctx.cells.min_height(south) + ctx.label_cell_spacing;
    let east_padding = ctx.cells.padding(east);
    let west_padding = ctx.cells.padding(west);
    let mut top_padding = java_max(0.0, east_padding.top - top_border_offset);
    top_padding = java_max(top_padding, west_padding.top - top_border_offset);
    let mut bottom_padding = java_max(0.0, east_padding.bottom - bottom_border_offset);
    bottom_padding = java_max(bottom_padding, west_padding.bottom - bottom_border_offset);
    for cell in [east, west] {
        let padding = ctx.cells.padding_mut(cell);
        padding.top = top_padding;
        padding.bottom = bottom_padding;
    }
}

/// `NodeLabelAndSizeUtilities.offsetSouthernPortsByNodeSize`.
fn offset_southern_ports(ctx: &mut NodeContext) {
    let height = ctx.node_size.y;
    for index in ctx.port_contexts_on(PortSide::SOUTH) {
        ctx.port_contexts[index].position.y += height;
    }
}

/// `PortPlacementCalculator.placeVerticalPorts`: east, then west.
fn place_vertical_ports(ctx: &mut NodeContext) {
    for side in [PortSide::EAST, PortSide::WEST] {
        if ctx.port_constraints == PortConstraints::FIXED_POS {
            place_fixed_pos_vertical(ctx, side);
        } else {
            place_free_vertical(ctx, side);
        }
    }
}

/// `PortPlacementCalculator.placeVerticalFixedPosPorts`: the y of each port is already fixed, so
/// only the x follows the border.
fn place_fixed_pos_vertical(ctx: &mut NodeContext, side: PortSide) {
    let node_width = ctx.node_size.x;
    for index in ctx.port_contexts_on(side) {
        let port = &mut ctx.port_contexts[index];
        port.position.x = vertical_port_x(port, node_width);
    }
}

/// `PortPlacementCalculator.placeVerticalFreePorts`.
fn place_free_vertical(ctx: &mut NodeContext, side: PortSide) {
    let contexts = ctx.port_contexts_on(side);
    if contexts.is_empty() {
        return;
    }
    let cell = ctx.inside_cell(side);
    let rect = ctx.cells.rect(cell);
    let padding = ctx.cells.padding(cell);
    let placement = ctx.cells.min_content(cell).y;
    let available = rect.height - padding.top - padding.bottom;
    let (offset, spacing) = spread_ports(
        available,
        placement,
        contexts.len(),
        ctx.alignment(side),
        ctx.port_port_spacing,
        ctx.has_size_option(SizeOptions::PORTS_OVERHANG),
    );
    let node_width = ctx.node_size.x;
    let mut current = rect.y + padding.top + offset;
    for &index in &contexts {
        let port = &mut ctx.port_contexts[index];
        port.position.x = vertical_port_x(port, node_width);
        port.position.y = current + port.margin.top;
        current += port.margin.top + port.size.y + port.margin.bottom + spacing;
    }
}

/// `calculateVerticalPortXCoordinate` without a border offset: western ports stick out to the
/// left of the node, eastern ports sit on its right border.
fn vertical_port_x(port: &PortContext, node_width: f64) -> f64 {
    if port.side == PortSide::WEST {
        -port.size.x
    } else {
        node_width
    }
}
