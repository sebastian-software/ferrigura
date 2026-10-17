//! The part of d3 the renderers use (`docs-port/xychart/NOTES.md` 6.3): the number text and
//! rounding of JavaScript ([`number`]), the band and linear scales of `d3-scale` 4.0.2 ([`scale`]),
//! the ordinal scale and its colours ([`ordinal`]), the ticks of `d3-array` 3.2.4 ([`ticks`]), the
//! reductions of `d3-array` 2.12.1 ([`array`]), the sort of `Array.prototype.sort` ([`sort`]), the
//! linear line of `d3-shape` ([`line`]), and the `Math.max` and `Math.min` of two arguments
//! ([`math`]).
//!
//! The scales and the line keep the reference's order of operations, and the path numbers are
//! rounded to three digits as `d3-path` rounds them.

pub mod array;
pub mod line;
pub mod math;
pub mod number;
pub mod ordinal;
pub mod scale;
pub mod sort;
pub mod ticks;

pub use array::{max_by, min_by, sum_by};
pub use line::{linear_path, link_horizontal};
pub use math::{math_max, math_min};
pub use number::{d3_number, interpolate_number, js_number, js_round, js_to_number};
pub use ordinal::{OrdinalScale, SCHEME_TABLEAU10};
pub use scale::{BandScale, LinearScale, normalize};
pub use sort::sort_js;
pub use ticks::ticks;
