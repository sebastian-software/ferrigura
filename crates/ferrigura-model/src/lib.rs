//! Diagram models of ferrigura and their canonical JSON form.
//!
//! A model is what the TypeScript diagram DB holds after parsing one diagram, written as serde
//! structs, less what the input does not contain or only rendering needs. The canonical JSON of a
//! model is the form the Rust parsers and the JS exporters of `tools/port/` both produce, so the
//! differential run compares them directly. The conventions (text, sanitisation, title precedence,
//! what is left out) are in `docs-port/canonical-model.md`; the testing approach is in
//! `docs-port/decisions/0003-differential-testing.md`.

#![warn(missing_docs)]

pub mod agentflow;
pub mod architecture;
pub mod block;
pub mod c4;
pub mod class;
pub mod config;
pub mod cynefin;
pub mod er;
pub mod event_modeling;
pub mod flowchart;
pub mod gantt;
pub mod git_graph;
pub mod info;
pub mod ishikawa;
pub mod journey;
pub mod js_object;
pub mod kanban;
pub mod mindmap;
pub mod packet;
pub mod pie;
pub mod quadrant_chart;
pub mod radar;
pub mod railroad;
pub mod requirement;
pub mod sankey;
pub mod sequence;
pub mod state;
pub mod swimlanes;
pub mod theme;
pub mod timeline;
pub mod tree_view;
pub mod treemap;
pub mod use_case;
pub mod value;
pub mod venn;
pub mod wardley;
pub mod xychart;

/// Version of the canonical model, written as `"version"` into every model. It is bumped when a
/// field is added, removed or changes meaning; see `docs-port/canonical-model.md`.
pub const CANONICAL_VERSION: u32 = 1;
