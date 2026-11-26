//! The requirement box, `rendering-util/rendering-elements/shapes/requirementBox.ts`.
//!
//! Mermaid draws the requirements and the elements of a requirement diagram with it, from the fields
//! that only such a node carries. A flowchart node can name the shape but carries no such fields, so
//! the shape refuses it.

use super::{ShapeInput, ShapeOutput};
use crate::RenderError;

/// `requirementBox.ts` for a node without requirement fields: an error, as the shape has nothing to
/// draw.
///
/// # Errors
///
/// Always: a flowchart node carries no requirement fields.
pub(super) fn requirement_box(input: &ShapeInput<'_>) -> Result<ShapeOutput, RenderError> {
    Err(RenderError::Options(format!(
        "the requirement box {} has no requirement fields",
        input.node.id
    )))
}
