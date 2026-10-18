//! Post-processing of the routed layout (`postProcessing.ts`, `postProcessSwimlaneLayout`).
//!
//! Not ported yet. [`post_process_swimlane_layout`] is the stub that the seam calls: it returns
//! [`Error::NotPorted`]`("post-processing")` and changes nothing. The port of the chain of geometry passes
//! (`direction/*.ts`) replaces the stub, with the same signature.

use crate::error::Error;
use crate::model::{Direction, LayoutData};

/// `postProcessSwimlaneLayout(data, direction)`: stub, see the module documentation.
pub(crate) fn post_process_swimlane_layout(
    _data: &mut LayoutData,
    _direction: Direction,
) -> Result<(), Error> {
    Err(Error::NotPorted("post-processing"))
}
