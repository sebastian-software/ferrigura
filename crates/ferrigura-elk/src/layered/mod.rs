//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/
//!
//! ELK Layered, `org.eclipse.elk.alg.layered` in the Java sources.
//!
//! Stage C of M4 covers the driver, the graph configuration and the processor
//! pipeline. For now, graphs with nodes only are laid out: every processor
//! runs its task and logs its name, and graphs that need edges, ports or
//! labels report `Unsupported` instead of skipping them.

pub use crate::error::LayoutError;

pub mod configurator;
pub mod driver;
pub mod dummies;
pub mod endlabels;
pub mod graph_transformer;
pub mod graph_util;
pub mod importer;
pub mod intermediate;
pub mod labelcell;
pub mod labels;
pub mod lgraph;
pub mod nodespacing;
pub mod options;
pub mod p1cycles;
pub mod p2layers;
pub mod p3order;
pub mod p4nodes;
pub mod p5edges;
pub mod phases;
pub mod ports;
pub mod preserveorder;
pub mod spacings;
pub mod transfer;
