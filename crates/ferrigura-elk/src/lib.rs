//! ELK Layered, ported from the Eclipse Layout Kernel Java sources.
//!
//! The port follows the elkjs 0.9.3 build, which GWT compiled from ELK commit
//! `9bc93474e` (see `docs-port/elk/SOURCES.md` in the repository). Java
//! semantics that GWT emulates differently from the JVM are reproduced where
//! they affect results: `java.util.Random`, the merge sort used by
//! `Collections.sort`, and 32-bit integer arithmetic.
//!
//! Modules are named after the Java packages they port. Files keep the Eclipse
//! Public License 2.0 notice of the original sources; see `LICENSE` and `NOTICE.md`.

pub mod core;
pub mod java;
pub mod json;
