//! The shape vocabulary of agentflow (`shapes.ts`): the aliases of §4.3.2, the removed shapes and
//! the allowed ones. The DB resolves an alias and checks the registry; the normalisation of a
//! node's shape, which reports the removed and unsupported ones, runs when the layout data is
//! read (see `semantic.rs`).

use crate::flowchart::shapes::is_valid_shape;

/// The shape a node takes when its own is removed, unsupported, or absent (`DEFAULT_SHAPE`).
pub(crate) const DEFAULT_SHAPE: &str = "roundedRect";

/// The §4.3.2 aliases: the author's name and the canonical shape it stands for.
const ALIASES: [(&str, &str); 8] = [
    ("task", "roundedRect"),
    ("tool", "subroutine"),
    ("input", "lean-right"),
    ("lean_right", "lean-right"),
    ("decision", "diamond"),
    ("refdoc", "lin-doc"),
    ("action", "hexagon"),
    ("round", "rect"),
];

/// The §4.3.3 shapes that v0.8.1 removed: authoring one is an error.
pub(crate) const REMOVED_SHAPES: [&str; 27] = [
    "doc",
    "stadium",
    "terminal",
    "circle",
    "trapezoid",
    "inv_trapezoid",
    "inv-trapezoid",
    "doublecircle",
    "double-circle",
    "typeDeclaration",
    "procs",
    "lean_left",
    "lean-left",
    "in-out",
    "cylinder",
    "ellipse",
    "odd",
    "tag-rect",
    "tagged-rectangle",
    "delay",
    "half-rounded-rectangle",
    "lin-rect",
    "lined-rectangle",
    "win-pane",
    "window-pane",
    "curv-trap",
    "curved-trapezoid",
];

/// The shapes that an author may use, after alias resolution.
pub(crate) const ALLOWED_SHAPES: [&str; 13] = [
    "roundedRect",
    "subroutine",
    "subprocess",
    "subproc",
    "framed-rectangle",
    "lean-right",
    "diamond",
    "lin-doc",
    "lined-document",
    "hexagon",
    "hex",
    "connector",
    "collapsedGroup",
];

/// The shapes of a tool definition (`SUBROUTINE_ALIASES`).
pub(crate) const TOOL_SHAPES: [&str; 5] = [
    "subroutine",
    "subprocess",
    "subproc",
    "framed-rectangle",
    "tool",
];

/// `resolveShapeAlias(shape)`: the canonical name of an alias, or the name itself.
pub(crate) fn resolve_shape_alias(shape: &str) -> String {
    ALIASES
        .iter()
        .find(|(alias, _)| *alias == shape)
        .map_or_else(
            || shape.to_owned(),
            |(_, canonical)| (*canonical).to_owned(),
        )
}

/// `isValidShape(name)`: the name is a shape of the registry.
pub(crate) fn is_valid_shape_name(name: &str) -> bool {
    is_valid_shape(name)
}

/// What the normalisation reports about a node's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapeIssue {
    /// `SHAPE_REMOVED`, an error.
    Removed,
    /// `SHAPE_UNSUPPORTED`, a warning.
    Unsupported,
}

/// `normaliseNodeShapes` for one node: the shape it is drawn with, and the issue it raises. A
/// missing shape, `squareRect` and `rect` are the default shape without an issue.
pub(crate) fn normalise_shape(shape: Option<&str>) -> (String, Option<ShapeIssue>) {
    let shape = match shape {
        None | Some("squareRect" | "rect") | Some("") => DEFAULT_SHAPE,
        Some(shape) => shape,
    };
    if REMOVED_SHAPES.contains(&shape) {
        return (DEFAULT_SHAPE.to_owned(), Some(ShapeIssue::Removed));
    }
    if !ALLOWED_SHAPES.contains(&shape) {
        return (DEFAULT_SHAPE.to_owned(), Some(ShapeIssue::Unsupported));
    }
    (shape.to_owned(), None)
}

/// `getTypeFromVertex`: the shape the layout reads for a vertex's type. An alias resolves first;
/// no type and `square` are `squareRect`, and `ellipse` stays.
pub(crate) fn layout_shape(vertex_type: Option<&str>) -> String {
    match vertex_type.map(resolve_shape_alias) {
        None => "squareRect".to_owned(),
        Some(resolved) if resolved == "square" => "squareRect".to_owned(),
        Some(resolved) => resolved,
    }
}
