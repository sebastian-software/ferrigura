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

/// The passes of the post-processing, each on its own, for the corpus tests (`tests/corpus_post.rs`), through the
/// doc-hidden `testing` module. The driver runs the same functions in its order.
#[doc(hidden)]
pub mod passes {
    use crate::direction::{shared_track_nudging, sibling_shared_face_routing};
    use crate::model::LayoutData;

    /// `nudgeSharedInteriorSubpaths(edges, nodeByIdMap)`.
    pub fn nudge_shared_interior_subpaths(data: &mut LayoutData) {
        shared_track_nudging::nudge_shared_interior_subpaths(data);
    }

    /// `straightenCollinearSiblingDetours(edges, nodes)`.
    pub fn straighten_collinear_sibling_detours(data: &mut LayoutData) {
        sibling_shared_face_routing::straighten_collinear_sibling_detours(data);
    }
}
