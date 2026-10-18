//! The swimlane layout of Mermaid 12.2.0 (`packages/mermaid/src/rendering-util/layout-algorithms/swimlanes/`),
//! ported from the TypeScript sources, on the default path. Ported: the data model ([`model`]), the graph view
//! and write-back (`helpers.ts`), the Sugiyama phases (`phase0` to `phase4`, `pipeline.ts`) and the seam
//! ([`run_swimlane_layout_core`], `layoutCore.ts`). Not yet ported: the orthogonal router and the geometry
//! passes, which are stubs that return [`Error::NotPorted`]. The two non-default options (`ignoreCrossLaneEdges:
//! false`, `automaticLaneOrdering: true`) are refused with [`Error::NonDefaultOption`].
//!
//! The corpus in `tests/corpus` holds the input and output of the seam, and the stages between them, captured
//! from the reference in Chromium and from the DDLT harness. See `NOTICE.md` for the sources.
//!
//! Licence: MIT, as the sources are.

#![forbid(unsafe_code)]

mod direction;
mod edge_label_nodes;
mod error;
mod helpers;
pub mod js;
mod layout_core;
mod math;
mod model;
mod phase0;
mod phase1;
mod phase2_dummies;
mod phase2_layering;
mod phase2_options;
mod phase3;
mod phase4;
mod pipeline;
mod post_processing;
mod router;

pub use error::Error;
pub use layout_core::run_swimlane_layout_core;
pub use model::{
    Config, Direction, Edge, FlowchartConfig, LayoutData, Node, Point, Rect, SwimlaneConfig,
};

/// Internal stages, exposed to the corpus tests in `tests/` and hidden from the documentation. Nothing here
/// is a stable interface.
#[doc(hidden)]
pub mod testing {
    pub use crate::direction::detour_simplification::simplify_detoured_edges;
    pub use crate::direction::endpoint_clip::{
        clip_edge_endpoints_to_node_boundaries, prepare_edge_endpoints_for_renderer,
    };
    pub use crate::direction::geometry::orthogonalize_and_simplify_edges;
    pub use crate::direction::label_anchoring::anchor_labels_to_polyline;
    pub use crate::direction::lr_transform::{
        apply_bt_direction_transform, apply_lr_direction_transform,
    };
    pub use crate::direction::materialized::{
        collapse_redundant_rectangular_doglegs, lift_obstacle_hugging_same_side_rails,
        lift_top_lane_title_bands_above_rails, reassign_crossing_external_rail_channels,
        resolve_rendered_orthogonal_crossings, separate_shared_rendered_terminal_lanes,
        shift_left_lane_title_bands_left_of_rails, shortcut_redundant_orthogonal_jogs,
        swap_destination_terminal_tails_to_reduce_crossings,
    };
    pub use crate::helpers::{
        Coordinates, EdgeOrigin, EdgeRef, Graph, GraphNode, Layering, NodeId, OrderedLayers,
        to_graph_view, write_back_to_layout_data,
    };
    pub use crate::model::{layout_data_from_json, layout_data_to_json};
    pub use crate::pipeline::{LayoutOptions, LayoutResult, sugiyama_layout};
    pub use crate::router::route_edges_orthogonal;
}
