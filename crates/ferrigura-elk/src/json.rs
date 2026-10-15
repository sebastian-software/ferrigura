//! The elkjs JSON graph format, read and written with serde.
//!
//! Field names and shapes follow elkjs 0.9.3 (`ElkGraphJson` in ELK, plus the
//! elkjs glue in `JsonAdapter.xtend`). Unknown fields are ignored, so graphs
//! carrying extra application fields still load. Option values stay as
//! `serde_json::Value`, because elkjs accepts strings, numbers and booleans.
//!
//! The types are added in Stage C step 1. This module is only a placeholder
//! until then.
