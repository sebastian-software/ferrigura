//! Post-processing of the routed layout (`postProcessing.ts`, `postProcessSwimlaneLayout`): the chain of the
//! direction passes in the order of the reference, with its early returns.
//!
//! The chain calls each pass on the layout data in place. The stage reports of the reference (`createPassReporter`)
//! are not ported. Validation is not ported either: `validateSwimlanesLayout` only logs and changes nothing.

use crate::direction::detour_simplification::simplify_detoured_edges;
use crate::direction::endpoint_clip::{
    clip_edge_endpoints_to_node_boundaries, prepare_edge_endpoints_for_renderer,
};
use crate::direction::geometry::orthogonalize_and_simplify_edges;
use crate::direction::label_anchoring::anchor_labels_to_polyline;
use crate::direction::lr_transform::{apply_bt_direction_transform, apply_lr_direction_transform};
use crate::direction::materialized::{
    collapse_redundant_rectangular_doglegs, lift_obstacle_hugging_same_side_rails,
    lift_top_lane_title_bands_above_rails, reassign_crossing_external_rail_channels,
    resolve_rendered_orthogonal_crossings, separate_shared_rendered_terminal_lanes,
    shift_left_lane_title_bands_left_of_rails, shortcut_redundant_orthogonal_jogs,
    swap_destination_terminal_tails_to_reduce_crossings,
};
use crate::direction::port_swap::port_swap_to_l_shape;
use crate::direction::shared_track_nudging::nudge_shared_interior_subpaths;
use crate::direction::sibling_shared_face_routing::straighten_collinear_sibling_detours;
use crate::direction::terminal_stub::collapse_short_terminal_stub;
use crate::error::Error;
use crate::model::{Direction, LayoutData};

/// `postProcessSwimlaneLayout(layout, direction)` (`postProcessing.ts`, `runPostProcessingPasses`).
pub(crate) fn post_process_swimlane_layout(
    data: &mut LayoutData,
    direction: Direction,
) -> Result<(), Error> {
    // `contentNodes` is the nodes that are not groups. The LR, RL and BT transforms run only when there is one.
    let has_content_nodes = data.nodes.iter().any(|node| node.is_group != Some(true));

    // TB is the canonical orientation. LR and RL rotate the rank progression onto X, and BT mirrors it in Y. When
    // a transform reports that it did not apply, the chain stops here.
    if matches!(direction, Direction::LR | Direction::RL)
        && has_content_nodes
        && !apply_lr_direction_transform(data, direction)
    {
        return Ok(());
    }
    if direction == Direction::BT && has_content_nodes && !apply_bt_direction_transform(data) {
        return Ok(());
    }

    orthogonalize_and_simplify_edges(data);
    simplify_detoured_edges(data);
    straighten_collinear_sibling_detours(data);
    port_swap_to_l_shape(data);
    anchor_labels_to_polyline(data);
    clip_edge_endpoints_to_node_boundaries(data);
    collapse_short_terminal_stub(data);
    nudge_shared_interior_subpaths(data);
    separate_shared_rendered_terminal_lanes(data);
    collapse_redundant_rectangular_doglegs(data);
    lift_obstacle_hugging_same_side_rails(data);
    swap_destination_terminal_tails_to_reduce_crossings(data);
    finalize_rendered_edges(data);
    nudge_shared_interior_subpaths(data);
    finalize_rendered_edges(data);
    lift_top_lane_title_bands_above_rails(data, direction);
    shift_left_lane_title_bands_left_of_rails(data, direction);
    lift_top_lane_title_bands_above_rails(data, direction);
    shift_left_lane_title_bands_left_of_rails(data, direction);
    Ok(())
}

/// `finalizeRenderedEdges` (`postProcessing.ts`): the crossing cleanup of the materialized render geometry, and the
/// endpoint handoff to the renderer.
fn finalize_rendered_edges(data: &mut LayoutData) {
    resolve_rendered_orthogonal_crossings(data);
    separate_shared_rendered_terminal_lanes(data);
    reassign_crossing_external_rail_channels(data);
    shortcut_redundant_orthogonal_jogs(data);
    anchor_labels_to_polyline(data);
    prepare_edge_endpoints_for_renderer(data);
    lift_obstacle_hugging_same_side_rails(data);
    anchor_labels_to_polyline(data);
    prepare_edge_endpoints_for_renderer(data);
}

/// The passes of the post-processing, each on its own, for the corpus tests (`tests/corpus_post.rs`), through the
/// doc-hidden `testing` module. The driver runs the same functions in its order.
#[doc(hidden)]
pub mod passes {
    use crate::direction::{
        port_swap, shared_track_nudging, sibling_shared_face_routing, terminal_stub,
    };
    use crate::error::Error;
    use crate::model::{Direction, LayoutData};

    /// `postProcessSwimlaneLayout(layout, direction)`: the whole chain.
    pub fn post_process(data: &mut LayoutData, direction: Direction) -> Result<(), Error> {
        super::post_process_swimlane_layout(data, direction)
    }

    /// `nudgeSharedInteriorSubpaths(edges, nodeByIdMap)`.
    pub fn nudge_shared_interior_subpaths(data: &mut LayoutData) {
        shared_track_nudging::nudge_shared_interior_subpaths(data);
    }

    /// `straightenCollinearSiblingDetours(edges, nodes)`.
    pub fn straighten_collinear_sibling_detours(data: &mut LayoutData) {
        sibling_shared_face_routing::straighten_collinear_sibling_detours(data);
    }

    /// `portSwapToLShape(edges, nodes)`.
    pub fn port_swap_to_l_shape(data: &mut LayoutData) {
        port_swap::port_swap_to_l_shape(data);
    }

    /// `collapseShortTerminalStub(edges, nodeByIdMap)`.
    pub fn collapse_short_terminal_stub(data: &mut LayoutData) {
        terminal_stub::collapse_short_terminal_stub(data);
    }
}
