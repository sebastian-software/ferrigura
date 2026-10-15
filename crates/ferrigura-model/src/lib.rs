//! Diagram models of ferrigura and their canonical JSON form.
//!
//! A model is what the TypeScript diagram DB holds after parsing one diagram, written as serde
//! structs. The canonical JSON of a model is the form the Rust parsers and the JS exporters of
//! `tools/port/` both produce, so the differential run compares them directly. See
//! `docs-port/decisions/0003-differential-testing.md`.

#![warn(missing_docs)]

pub mod info;
pub mod pie;

/// Version of the canonical model. Written as `"version"` into every model and bumped when a field
/// is added, removed or changes meaning.
pub const CANONICAL_VERSION: u32 = 1;
