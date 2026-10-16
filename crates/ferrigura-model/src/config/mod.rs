//! Configuration of ferrigura.
//!
//! The types and their defaults are generated from the schema that the reference uses, by
//! `build.rs`; the schema stays the single source. [`effective`] computes the configuration of one
//! diagram from the `initialize` settings and the text's directives, in the order the reference
//! applies them, and [`deep_merge`] merges front matter and directives. The differential run
//! (`pnpm port:diff config`) compares the results with the reference; see
//! `docs-port/decisions/0003-differential-testing.md`.

mod generated {
    include!(concat!(env!("OUT_DIR"), "/config_schema.rs"));
}

mod layers;

pub use generated::*;
pub use layers::{assign_with_depth, deep_merge, effective, sanitize_directive};

/// The reference's defaults that its schema does not hold, as a JSON object. See `ts_defaults.json`.
const TS_DEFAULTS: &str = include_str!("ts_defaults.json");

/// The defaults of every key as JSON: the schema's defaults, with the reference's TypeScript-only
/// defaults on top.
#[must_use]
pub fn defaults() -> serde_json::Value {
    let mut value = serde_json::to_value(MermaidConfig::default()).unwrap_or_default();
    let overrides: serde_json::Value = serde_json::from_str(TS_DEFAULTS).unwrap_or_default();
    layers::assign_with_depth(&mut value, &overrides, 2);
    value
}

/// The typed view of a configuration value. Keys that the types do not name are ignored; a value of
/// the wrong type is an error.
pub fn typed(value: &serde_json::Value) -> Result<MermaidConfig, serde_json::Error> {
    serde_json::from_value(value.clone())
}
