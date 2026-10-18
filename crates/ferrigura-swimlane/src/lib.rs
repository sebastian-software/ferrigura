//! The swimlane layout of Mermaid 12.2.0 (`packages/mermaid/src/rendering-util/layout-algorithms/swimlanes/`),
//! ported from the TypeScript sources. This is the default path: the Sugiyama-style phases, the
//! orthogonal router and the geometry passes. The two non-default options (`ignoreCrossLaneEdges: false`,
//! `automaticLaneOrdering: true`) are refused with [`Error::NonDefaultOption`].
//!
//! The seam is `runSwimlaneLayoutCore` (`layoutCore.ts`). The corpus in `tests/corpus` holds its input
//! and output, and the stages between them, captured from the reference in Chromium and from the DDLT
//! harness. See `NOTICE.md` for the sources.
//!
//! Licence: MIT, as the sources are.

#![forbid(unsafe_code)]

mod constants;
mod direction;
mod edge_label_nodes;
mod error;
mod helpers;
pub mod js;
mod layout_core;
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

/// Internal stages, exposed to the corpus tests in `tests/` and hidden from the documentation.
#[doc(hidden)]
pub mod testing {}
