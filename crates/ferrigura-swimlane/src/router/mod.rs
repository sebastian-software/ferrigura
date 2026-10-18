//! Router: `routeEdgesOrthogonal` and its helpers (`orthogonalRouter/router.ts`).
//!
//! Not ported yet. [`route_edges_orthogonal`] is the stub that the seam calls: it returns
//! [`Error::NotPorted`]`("router")` and changes nothing. The router replaces the stub, with the same
//! signature, and it sets the `points` of the edges.

use crate::error::Error;
use crate::model::{Direction, LayoutData};

/// `routeEdgesOrthogonal(data, direction)`: stub, see the module documentation.
pub(crate) fn route_edges_orthogonal(
    _data: &mut LayoutData,
    _direction: Direction,
) -> Result<(), Error> {
    Err(Error::NotPorted("router"))
}
