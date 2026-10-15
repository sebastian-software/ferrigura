//! The ELK graph model, `org.eclipse.elk.graph` in the Java sources.
//!
//! The Java model is generated EMF; the port keeps the fields and the property
//! holders and drops the EMF machinery. [`ElkNode`] is the root or a child,
//! [`ElkPort`], [`ElkLabel`] and [`ElkEdge`] are its parts.

use crate::core::data::property::PropertyHolder;

pub mod json_import;

/// A shape: position and size, as `ElkShape` has them.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Shape {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A node, or the root of a graph.
#[derive(Debug, Clone, Default)]
pub struct ElkNode {
    pub id: String,
    pub shape: Shape,
    pub properties: PropertyHolder,
    pub ports: Vec<ElkPort>,
    pub labels: Vec<ElkLabel>,
    pub children: Vec<ElkNode>,
    pub edges: Vec<ElkEdge>,
}

/// A port of a node.
#[derive(Debug, Clone, Default)]
pub struct ElkPort {
    pub id: String,
    pub shape: Shape,
    pub properties: PropertyHolder,
    pub labels: Vec<ElkLabel>,
}

/// A label.
#[derive(Debug, Clone, Default)]
pub struct ElkLabel {
    pub id: Option<String>,
    pub text: Option<String>,
    pub shape: Shape,
    pub properties: PropertyHolder,
}

/// An edge. Edges are not imported yet; the field is here for the model's shape.
#[derive(Debug, Clone, Default)]
pub struct ElkEdge {
    pub id: String,
    pub properties: PropertyHolder,
    pub labels: Vec<ElkLabel>,
}
