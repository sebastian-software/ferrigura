// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, d2talalayout/result_validate.go and d2fonts/d2fonts_common.go.

use std::collections::{HashMap, HashSet};

use crate::Point;

use super::{Dimensions, ValidationError, check_cancelled};

const UNSET_LABEL_POSITION: i8 = 0;
const ENGINE_FONT_SIZES: [i64; 7] = [13, 14, 16, 20, 24, 28, 32];

#[derive(Debug)]
pub struct GraphMetadata<'a> {
    pub root_hierarchy: bool,
    /// Canonical entity ID for a direction, or `None` for the root.
    pub directions: &'a [(Option<i64>, i64)],
    pub nodes: &'a [NodeMetadata<'a>],
    pub edges: &'a [EdgeMetadata<'a>],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NodeMetadata<'a> {
    pub id: i64,
    pub shape: &'a str,
    pub container_id: Option<i64>,
    pub table_columns: usize,
    pub is_3d: bool,
    pub is_multiple: bool,
    pub step_paint: Option<StepPaint>,
    pub is_invisible: bool,
    pub fixed_top_left: Option<Point>,
    pub desired_width: Option<f64>,
    pub desired_height: Option<f64>,
    pub force_hierarchy: bool,
    pub label: Option<LabelMetadata<'a>>,
    pub icon_position: Option<i8>,
    pub font_size: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepPaint {
    pub order: i64,
    pub z_index: i64,
    pub opaque: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelMetadata<'a> {
    pub text: &'a str,
    pub position: i8,
    pub size: Dimensions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EdgeMetadata<'a> {
    pub id: i64,
    pub reversed_for_layout: bool,
    pub source_arrowhead: &'a str,
    pub target_arrowhead: &'a str,
    pub source_arrowhead_label: Option<LabelMetadata<'a>>,
    pub target_arrowhead_label: Option<LabelMetadata<'a>>,
    pub minimum_size: Dimensions,
    pub from_table_column: Option<i64>,
    pub to_table_column: Option<i64>,
    pub keep_explicit: bool,
    pub is_invisible: bool,
    pub style: EdgeStyle<'a>,
    pub label: Option<LabelMetadata<'a>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EdgeStyle<'a> {
    pub opacity: Option<&'a str>,
    pub stroke: Option<&'a str>,
    pub fill: Option<&'a str>,
    pub fill_pattern: Option<&'a str>,
    pub stroke_width: Option<&'a str>,
    pub stroke_dash: Option<&'a str>,
    pub border_radius: Option<&'a str>,
    pub shadow: Option<&'a str>,
    pub three_dee: Option<&'a str>,
    pub multiple: Option<&'a str>,
    pub font: Option<&'a str>,
    pub font_size: Option<&'a str>,
    pub font_color: Option<&'a str>,
    pub animated: Option<&'a str>,
    pub bold: Option<&'a str>,
    pub italic: Option<&'a str>,
    pub underline: Option<&'a str>,
    pub filled: Option<&'a str>,
    pub double_border: Option<&'a str>,
    pub text_transform: Option<&'a str>,
}

/// Binds immutable metadata after the result topology has been checked.
///
/// # Errors
/// Rejects cancellation, changed metadata, and records that cannot be bound
/// to the same entity on both sides.
pub fn validate(
    expected: &GraphMetadata<'_>,
    actual: &GraphMetadata<'_>,
    missing_sequence_edges: &HashSet<i64>,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), ValidationError> {
    check_cancelled(cancelled)?;
    if expected.root_hierarchy != actual.root_hierarchy {
        return Err(ValidationError::ChangedGraphMetadata("root hierarchy"));
    }
    let expected_directions = canonical_directions(expected.directions, cancelled)?;
    let actual_directions = canonical_directions(actual.directions, cancelled)?;
    if expected_directions.len() != actual_directions.len() {
        return Err(ValidationError::ChangedGraphMetadata("graph direction"));
    }
    for (node, direction) in expected_directions {
        check_cancelled(cancelled)?;
        if actual_directions.get(&node) != Some(&direction) {
            return Err(ValidationError::ChangedGraphMetadata("graph direction"));
        }
    }

    let mut expected_nodes = HashMap::with_capacity(expected.nodes.len());
    for node in expected.nodes {
        check_cancelled(cancelled)?;
        expected_nodes.insert(node.id, node);
    }
    for node in actual.nodes {
        check_cancelled(cancelled)?;
        let input = expected_nodes
            .get(&node.id)
            .ok_or(ValidationError::UnexpectedNode(node.id))?;
        validate_node(input, node)?;
    }

    let mut expected_edges = HashMap::with_capacity(expected.edges.len());
    for edge in expected.edges {
        check_cancelled(cancelled)?;
        expected_edges.insert(edge.id, edge);
    }
    let mut actual_edges = HashSet::with_capacity(actual.edges.len());
    for edge in actual.edges {
        check_cancelled(cancelled)?;
        actual_edges.insert(edge.id);
        let input = expected_edges
            .get(&edge.id)
            .ok_or(ValidationError::UnexpectedEdge(edge.id))?;
        validate_edge(input, edge)?;
    }
    for edge in expected.edges {
        check_cancelled(cancelled)?;
        if !actual_edges.contains(&edge.id) && !missing_sequence_edges.contains(&edge.id) {
            return Err(ValidationError::MissingEdge(edge.id));
        }
    }
    Ok(())
}

fn canonical_directions(
    directions: &[(Option<i64>, i64)],
    cancelled: &dyn Fn() -> bool,
) -> Result<HashMap<Option<i64>, i64>, ValidationError> {
    let mut canonical = HashMap::with_capacity(directions.len());
    for &(node, direction) in directions {
        check_cancelled(cancelled)?;
        if canonical.insert(node, direction).is_some() {
            return Err(ValidationError::ChangedGraphMetadata(
                "duplicated graph direction",
            ));
        }
    }
    Ok(canonical)
}

fn validate_node(
    input: &NodeMetadata<'_>,
    output: &NodeMetadata<'_>,
) -> Result<(), ValidationError> {
    let input_label = input.label.map(|label| label.text);
    let output_label = output.label.map(|label| label.text);
    let input_position = input.label.map(|label| label.position);
    let output_position = output.label.map(|label| label.position);
    let font_size_valid = match (input.font_size, output.font_size) {
        (Some(old), Some(new)) => old == new || ENGINE_FONT_SIZES.contains(&new),
        (old, new) => old == new,
    };
    let fields = [
        (input.shape == output.shape, "shape"),
        (
            input.container_id == output.container_id,
            "container ownership",
        ),
        (
            input.table_columns == output.table_columns,
            "table column count",
        ),
        (input.is_3d == output.is_3d, "3D outline metadata"),
        (
            input.is_multiple == output.is_multiple,
            "multiple outline metadata",
        ),
        (input.step_paint == output.step_paint, "step paint metadata"),
        (
            input.is_invisible == output.is_invisible,
            "visibility metadata",
        ),
        (
            input.fixed_top_left == output.fixed_top_left,
            "fixed position metadata",
        ),
        (
            input.desired_width == output.desired_width
                && input.desired_height == output.desired_height,
            "desired-size metadata",
        ),
        (
            input.force_hierarchy == output.force_hierarchy,
            "forced-hierarchy metadata",
        ),
        (input_label == output_label, "label identity"),
        (
            valid_fixed_position(input_position, output_position),
            "fixed label position",
        ),
        (
            input.icon_position.is_some() == output.icon_position.is_some(),
            "icon presence",
        ),
        (
            valid_fixed_position(input.icon_position, output.icon_position),
            "fixed icon position",
        ),
        (font_size_valid, "font size outside the output domain"),
    ];
    let changed = fields.into_iter().find(|&(equal, _)| !equal);
    if let Some((_, field)) = changed {
        return Err(ValidationError::ChangedNodeMetadata {
            id: output.id,
            field,
        });
    }
    Ok(())
}

fn valid_fixed_position(input: Option<i8>, output: Option<i8>) -> bool {
    input.is_none_or(|position| position == UNSET_LABEL_POSITION || Some(position) == output)
}

fn validate_edge(
    input: &EdgeMetadata<'_>,
    output: &EdgeMetadata<'_>,
) -> Result<(), ValidationError> {
    let fields = [
        (
            layout_arrowheads(input) == layout_arrowheads(output),
            "arrowhead metadata",
        ),
        (
            input.source_arrowhead_label == output.source_arrowhead_label,
            "source arrowhead label metadata",
        ),
        (
            input.target_arrowhead_label == output.target_arrowhead_label,
            "target arrowhead label metadata",
        ),
        (
            input.minimum_size == output.minimum_size,
            "minimum geometry",
        ),
        (
            input.from_table_column == output.from_table_column
                && input.to_table_column == output.to_table_column,
            "table column attachment metadata",
        ),
        (
            input.keep_explicit == output.keep_explicit,
            "explicit-connection metadata",
        ),
        (
            input.is_invisible == output.is_invisible,
            "visibility metadata",
        ),
        (input.style == output.style, "style metadata"),
        (
            edge_label(input) == edge_label(output),
            "label identity or dimensions",
        ),
    ];
    let changed = fields.into_iter().find(|&(equal, _)| !equal);
    if let Some((_, field)) = changed {
        return Err(ValidationError::ChangedEdgeMetadata {
            id: output.id,
            field,
        });
    }
    Ok(())
}

fn layout_arrowheads<'a>(edge: &EdgeMetadata<'a>) -> (bool, &'a str, &'a str) {
    (
        edge.reversed_for_layout,
        edge.source_arrowhead,
        edge.target_arrowhead,
    )
}

fn edge_label<'a>(edge: &EdgeMetadata<'a>) -> Option<(&'a str, Dimensions)> {
    edge.label.map(|label| (label.text, label.size))
}

#[cfg(test)]
mod tests;
