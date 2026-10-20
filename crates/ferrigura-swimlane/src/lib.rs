//! The swimlane layout of Mermaid 12.2.0 (`packages/mermaid/src/rendering-util/layout-algorithms/swimlanes/`),
//! ported from the TypeScript sources, on the default path.
//!
//! **Ported.** The data model (`model`), the graph view and the write-back (`helpers.ts`), the Sugiyama phases
//! (`phase0` to `phase4`, `pipeline.ts`), the orthogonal router (`orthogonalRouter/router.ts`), the geometry passes
//! of the post-processing (`direction/*.ts`, `postProcessing.ts`), and the seam, [`run_swimlane_layout_core`]
//! (`layoutCore.ts`).
//!
//! **Not ported.** `validateSwimlanesLayout`, which only logs and which the core does not call; the preparation
//! before the seam, `prepareLayoutForSwimlanes` and `createEdgeLabelNodes`, which the browser and the DDLT harness
//! run first; and the two non-default options, `ignoreCrossLaneEdges: false` and `automaticLaneOrdering: true`,
//! which are refused with [`Error::NonDefaultOption`].
//!
//! **Numbers.** Each expression keeps the operand order of its source, so the output agrees with the reference
//! bit for bit. `toFixed`, `Math.round`, `localeCompare` and `Array.prototype.sort` are in [`js`]; `Math.hypot` is
//! `ferrigura_jsmath::hypot`. The sort is exact from 17 to 129 elements as Chromium 151 runs it, with two gaps
//! (2 to 16 elements, and 130 and more); see [`js::js_sort_by`].
//!
//! **Corpus.** `tests/corpus` holds 131 captured calls of the seam (44 fixtures, 27 DDLT inputs and 60 generated
//! diagrams), each with its input, its stages and its output, captured from the reference in Chromium. The
//! end-to-end test, `tests/corpus.rs`, compares every output exactly. `docs-port/swimlanes/PLAN.md` gives the
//! exclusions and the tools.
//!
//! Licence: MIT, as the sources are. See `NOTICE.md`.

#![forbid(unsafe_code)]

mod direction;
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
    pub use crate::post_processing::passes;
    pub use crate::router::route_edges_orthogonal;
}
