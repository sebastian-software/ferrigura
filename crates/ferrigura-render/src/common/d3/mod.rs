//! The part of d3 the custom renderers use (`docs-port/xychart/NOTES.md` 6.3): the number text and
//! rounding of JavaScript ([`number`]), the band and linear scales of `d3-scale` 4.0.2 ([`scale`]),
//! the ticks of `d3-array` 3.2.4 ([`ticks`]) and the linear line of `d3-shape` ([`line`]).
//!
//! The scales and the line keep the reference's order of operations, and the path numbers are
//! rounded to three digits as `d3-path` rounds them.

pub mod line;
pub mod number;
pub mod scale;
pub mod ticks;

pub use line::linear_path;
pub use number::{d3_number, interpolate_number, js_number, js_round, js_to_number};
pub use scale::{BandScale, LinearScale, normalize};
pub use ticks::ticks;
