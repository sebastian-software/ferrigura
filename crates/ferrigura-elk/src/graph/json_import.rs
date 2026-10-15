//! Reads the elkjs JSON graph into the ELK graph model.
//!
//! Follows `JsonImporter.xtend` (ELK, `org.eclipse.elk.graph.json`) in its
//! order: a node takes its identifier, its options, its individual spacings, its
//! shape, its ports, its labels, and then its children. Edges are read after
//! all nodes; that part is not ported yet and is reported as unsupported.
//!
//! Unknown option keys are ignored, and so are keys whose text does not parse
//! for their type, as `setOption` does. A key whose type is not ported yet is an
//! error, so an option is never dropped silently.

use std::fmt;

use crate::core::data::property::PropertyHolder;
use crate::core::data::{option_by_suffix, parse_value};
use crate::graph::{ElkLabel, ElkNode, ElkPort, Shape};
use crate::json::{JsonLabel, JsonNode, JsonPort, option_pairs_of};

/// Why a graph cannot be imported.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    /// The JSON is malformed for elkjs too (for example an unsupported value).
    Format(String),
    /// The graph uses something this port does not implement yet.
    Unsupported(String),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Format(message) => write!(f, "{message}"),
            ImportError::Unsupported(message) => write!(f, "not supported yet: {message}"),
        }
    }
}

impl std::error::Error for ImportError {}

/// `transformNode` for the root, with no parent.
pub fn import_graph(root: &JsonNode) -> Result<ElkNode, ImportError> {
    transform_node(root)
}

fn transform_node(json: &JsonNode) -> Result<ElkNode, ImportError> {
    let mut node = ElkNode {
        id: json.id.clone(),
        ..ElkNode::default()
    };
    transform_properties(
        json.layout_options.as_ref(),
        json.properties.as_ref(),
        &mut node.properties,
    )?;
    if json.individual_spacings.is_some() {
        return Err(ImportError::Unsupported(
            "individualSpacings (no Mermaid graph uses them)".to_string(),
        ));
    }
    node.shape = shape_layout(json.x, json.y, json.width, json.height);
    for port in &json.ports {
        node.ports.push(transform_port(port)?);
    }
    node.labels = transform_labels(&json.labels)?;
    for child in &json.children {
        node.children.push(transform_node(child)?);
    }
    if !json.edges.is_empty() {
        return Err(ImportError::Unsupported(format!(
            "edges in graph {} (edge import is the next step)",
            json.id
        )));
    }
    Ok(node)
}

/// `transformProperties`: `layoutOptions` if present, else `properties`.
fn transform_properties(
    layout_options: Option<&crate::json::LayoutOptions>,
    properties: Option<&crate::json::LayoutOptions>,
    holder: &mut PropertyHolder,
) -> Result<(), ImportError> {
    let pairs = option_pairs_of(layout_options, properties)
        .map_err(|error| ImportError::Format(error.message().to_string()))?;
    for (key, text) in pairs {
        set_option(holder, &key, &text)?;
    }
    Ok(())
}

/// `setOption`: suffix lookup, then parse; unknown keys and unparsable text are ignored.
fn set_option(holder: &mut PropertyHolder, key: &str, text: &str) -> Result<(), ImportError> {
    let Some(spec) = option_by_suffix(key) else {
        return Ok(());
    };
    match parse_value(spec, text) {
        Ok(Some(value)) => {
            holder.set(spec.id, Some(value));
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(unsupported) => Err(ImportError::Unsupported(format!(
            "option {} of type {}",
            unsupported.id, unsupported.melk_type
        ))),
    }
}

/// `transformShapeLayout`: missing or non-finite numbers become zero.
fn shape_layout(x: Option<f64>, y: Option<f64>, width: Option<f64>, height: Option<f64>) -> Shape {
    let valid = |value: Option<f64>| match value {
        Some(number) if number.is_finite() => number,
        _ => 0.0,
    };
    Shape {
        x: valid(x),
        y: valid(y),
        width: valid(width),
        height: valid(height),
    }
}

fn transform_port(json: &JsonPort) -> Result<ElkPort, ImportError> {
    let mut port = ElkPort {
        id: json.id.clone(),
        shape: shape_layout(json.x, json.y, json.width, json.height),
        ..ElkPort::default()
    };
    transform_properties(
        json.layout_options.as_ref(),
        json.properties.as_ref(),
        &mut port.properties,
    )?;
    port.labels = transform_labels(&json.labels)?;
    Ok(port)
}

/// `transformLabels`: each label takes its text, options and shape.
fn transform_labels(labels: &[JsonLabel]) -> Result<Vec<ElkLabel>, ImportError> {
    labels.iter().map(transform_label).collect()
}

fn transform_label(json: &JsonLabel) -> Result<ElkLabel, ImportError> {
    let mut label = ElkLabel {
        id: json.id.clone(),
        text: json.text.clone(),
        shape: shape_layout(json.x, json.y, json.width, json.height),
        ..ElkLabel::default()
    };
    transform_properties(
        json.layout_options.as_ref(),
        json.properties.as_ref(),
        &mut label.properties,
    )?;
    Ok(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::enums::Direction;
    use serde_json::json;

    fn parse(value: serde_json::Value) -> JsonNode {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn options_shapes_and_children_are_read_in_order() {
        let root = parse(json!({
            "id": "root",
            "layoutOptions": {
                "elk.direction": "RIGHT",
                "spacing.baseValue": 40,
                "not.an.option": 1
            },
            "children": [{ "id": "a", "width": 50, "height": 40 }]
        }));
        let node = import_graph(&root).unwrap();
        assert_eq!(node.id, "root");
        assert_eq!(
            node.properties
                .enum_of::<Direction>("org.eclipse.elk.direction"),
            Some(Direction::RIGHT)
        );
        assert_eq!(
            node.properties
                .double_of("org.eclipse.elk.layered.spacing.baseValue"),
            Some(40.0)
        );
        assert_eq!(node.children[0].shape.width, 50.0);
        assert_eq!(node.children[0].shape.x, 0.0);
    }

    #[test]
    fn edges_are_reported_as_unsupported() {
        let root = parse(json!({
            "id": "root",
            "children": [{ "id": "a" }, { "id": "b" }],
            "edges": [{ "id": "e", "sources": ["a"], "targets": ["b"] }]
        }));
        assert!(matches!(
            import_graph(&root),
            Err(ImportError::Unsupported(_))
        ));
    }
}
