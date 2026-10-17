//! The part of d3 the custom renderers use (`docs-port/xychart/NOTES.md` 6.3): the number text and
//! rounding of JavaScript ([`number`]), the band and linear scales of `d3-scale` 4.0.2 ([`scale`]),
//! the ticks of `d3-array` 3.2.4 ([`ticks`]) and the linear line of `d3-shape` ([`line`]).
//!
//! The treemap's pieces of `d3-hierarchy` ([`hierarchy`], [`treemap`]), `d3-format` ([`format`]),
//! and the ordinal scale of `d3-scale` ([`ordinal`]), see `docs-port/treemap/NOTES.md` (section 6).
//!
//! The scales and the line keep the reference's order of operations, and the path numbers are
//! rounded to three digits as `d3-path` rounds them.

pub mod format;
pub mod hierarchy;
pub mod line;
pub mod number;
pub mod ordinal;
pub mod scale;
pub mod ticks;
pub mod treemap;

pub use line::linear_path;
pub use number::{d3_number, interpolate_number, js_max, js_number, js_round, js_to_number};
pub use scale::{BandScale, LinearScale, normalize};
pub use ticks::ticks;
