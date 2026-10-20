//! Canonical model of a swimlane diagram (`swimlane-beta`).
//!
//! The reference's swimlane diagram is the flowchart's factory with its own styles
//! (`packages/mermaid/src/diagrams/swimlanes/swimlanesDiagram.ts`, `createFlowDiagram`), so the
//! text is parsed by the flowchart's parser and stored in the flowchart's DB (`flowDb.ts`). A lane
//! is a subgraph of that DB. The model is therefore the flowchart's model, field for field, and its
//! rules are the flowchart's (`docs-port/canonical-model.md`, Flowchart and Swimlanes). What the
//! swimlane layout reads from the DB, and the layout's own data, are the renderer's: see
//! `docs-port/swimlanes/PLAN.md`.

/// A parsed swimlane diagram: the flowchart DB's state after the parse.
pub type SwimlanesDiagram = crate::flowchart::FlowchartDiagram;
