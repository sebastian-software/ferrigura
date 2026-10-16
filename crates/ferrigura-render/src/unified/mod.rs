//! The unified renderer: a port of `packages/mermaid/src/rendering-util/`, which the flowchart,
//! class, state and ER diagrams share. Each of them fills the layout data with its own `getData`;
//! the labels, the node shapes, the elkjs graph and the post-processing of its result, the edges,
//! the markers and the cluster frames are common. See `docs-port/flowchart/RENDER-PLAN.md`.

pub mod clusters;
pub mod data;
pub mod edges;
pub mod elk_apply;
pub mod elk_build;
pub mod elk_geometry;
pub mod elk_straighten;
pub mod elk_tree;
pub mod geom;
pub mod label;
pub mod line_jumps;
pub mod markdown;
pub mod markers;
pub mod palette;
pub mod path;
pub mod pipeline;
pub mod shapes;
pub mod terminal;
