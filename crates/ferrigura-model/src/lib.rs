//! Diagram models of ferrigura and their canonical JSON form.
//!
//! A model is what the TypeScript diagram DB holds after parsing one diagram, written as serde
//! structs, less what the input does not contain or only rendering needs. The canonical JSON of a
//! model is the form the Rust parsers and the JS exporters of `tools/port/` both produce, so the
//! differential run compares them directly. The conventions (text, sanitisation, title precedence,
//! what is left out) are in `docs-port/canonical-model.md`; the testing approach is in
//! `docs-port/decisions/0003-differential-testing.md`.

#![warn(missing_docs)]

pub mod config;
pub mod flowchart;
pub mod git_graph;
pub mod info;
pub mod journey;
pub mod pie;
pub mod theme;
pub mod timeline;
pub mod value;

/// Version of the canonical model, written as `"version"` into every model. It is bumped when a
/// field is added, removed or changes meaning; see `docs-port/canonical-model.md`.
pub const CANONICAL_VERSION: u32 = 1;
