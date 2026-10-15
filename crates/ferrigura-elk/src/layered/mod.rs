//! ELK Layered, `org.eclipse.elk.alg.layered` in the Java sources.
//!
//! Stage C of M4 covers the driver, the graph configuration and the processor
//! pipeline. For now, graphs with nodes only are laid out: every processor
//! runs its task and logs its name, and graphs that need edges, ports or
//! labels report `Unsupported` instead of skipping them.

pub use crate::error::LayoutError;

pub mod configurator;
pub mod driver;
pub mod importer;
pub mod intermediate;
pub mod lgraph;
pub mod options;
pub mod phases;
