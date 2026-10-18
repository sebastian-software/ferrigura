//! The materialized-geometry passes of `postProcessSwimlaneLayout` (`direction/materializedGeometry.ts`).
//!
//! Each pass is a `pub(crate)` function with the TypeScript name in snake_case. It edits the layout in place, in
//! the order of `postProcessing.ts`, and its stage in the corpus is `post:<name>`. The module allows dead code
//! until the post-processing driver calls the passes. The title-band passes take the layout direction, and the
//! others take the layout data alone.

#![allow(dead_code)]

mod common;
mod destination_tails;
mod doglegs;
mod external_rails;
mod jogs;
mod rails;
mod terminal_lanes;
mod title_bands;

pub use destination_tails::swap_destination_terminal_tails_to_reduce_crossings;
pub use doglegs::collapse_redundant_rectangular_doglegs;
pub use external_rails::reassign_crossing_external_rail_channels;
pub use jogs::shortcut_redundant_orthogonal_jogs;
pub use rails::lift_obstacle_hugging_same_side_rails;
pub use terminal_lanes::separate_shared_rendered_terminal_lanes;
pub use title_bands::{
    lift_top_lane_title_bands_above_rails, shift_left_lane_title_bands_left_of_rails,
};
