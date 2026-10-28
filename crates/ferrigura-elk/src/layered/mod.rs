//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/
//!
//! ELK Layered, `org.eclipse.elk.alg.layered` in the Java sources.
//!
//! The module holds the driver, the graph configuration, the five phases (`p1cycles` to
//! `p5edges`) and the intermediate processors, in the order `AlgorithmAssembler` gives them for
//! the options Mermaid sets. Every processor logs its name, so the trace can be compared with
//! elkjs's. A feature that is not ported reports `Unsupported` with its name instead of being
//! skipped; the README of the crate lists them.

pub use crate::error::LayoutError;

pub mod components;
pub mod compound;
pub mod configurator;
pub mod driver;
pub mod dummies;
pub mod endlabels;
pub mod graph_transformer;
pub mod graph_util;
pub mod hierarchical_ports;
pub mod importer;
pub mod intermediate;
pub mod inverted_ports;
pub mod labelcell;
pub mod labels;
pub mod layerconstraints;
pub mod lgraph;
pub mod loops;
pub mod nodespacing;
pub mod northsouth;
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
