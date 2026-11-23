//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.graph.json/src/org/eclipse/elk/graph/json/JsonImporter.xtend
//!
//! Reads the elkjs JSON graph into the ELK graph model.
//!
//! Follows `JsonImporter.xtend` (ELK, `org.eclipse.elk.graph.json`) in its
//! order: a node takes its identifier, its options, its individual spacings, its
//! shape, its ports, its labels, and then its children. The edges are read
//! afterwards, level by level in document order: each edge is read where the
//! input has it, and then moved to the lowest common ancestor of its ends, as
//! `ElkGraphUtil.updateContainment` does.
//!
//! Unknown option keys are ignored, and so are keys whose text does not parse
//! for their type, as `setOption` does. A key whose type is not ported yet is an
//! error, so an option is never dropped silently.

use std::collections::HashMap;

use crate::core::data::property::PropertyHolder;
use crate::core::data::{option_by_suffix, parse_value};
use crate::error::LayoutError;
use crate::graph::{ElkEdge, ElkLabel, ElkNode, ElkPort, Shape};
use crate::json::{JsonEdge, JsonLabel, JsonNode, JsonPort, LayoutOptions, string_val};

/// `transformNode` for the root, with no parent, and then the edges of every level.
pub fn import_graph(root: &JsonNode) -> Result<ElkNode, LayoutError> {
    let mut elk = transform_node(root)?;
    let mut paths = HashMap::new();
    index_nodes(&elk, &mut Vec::new(), &mut paths);
    transform_edges(root, &mut elk, &paths)?;
    Ok(elk)
}

/// The path of child indices from the root to each node, by id. A repeated id keeps the first node.
/// A port is indexed by the path of its node: `ElkGraphUtil.connectableShapeToNode` gives an edge end
/// at a port the port's node.
fn index_nodes(node: &ElkNode, path: &mut Vec<usize>, paths: &mut HashMap<String, Vec<usize>>) {
    paths.entry(node.id.clone()).or_insert_with(|| path.clone());
    for port in &node.ports {
        paths.entry(port.id.clone()).or_insert_with(|| path.clone());
    }
    for (index, child) in node.children.iter().enumerate() {
        path.push(index);
        index_nodes(child, path, paths);
        path.pop();
    }
}

/// `transformEdges`: the edges of a JSON node, each moved to its container, then the children's
/// edges, in document order.
fn transform_edges(
    json: &JsonNode,
    root: &mut ElkNode,
    paths: &HashMap<String, Vec<usize>>,
) -> Result<(), LayoutError> {
    for edge in &json.edges {
        let elk_edge = transform_edge(edge)?;
        let container = edge_container(&elk_edge, paths)?;
        node_at_mut(root, &container)?.edges.push(elk_edge);
    }
    for child in &json.children {
        transform_edges(child, root, paths)?;
    }
    Ok(())
}

/// `ElkGraphUtil.findBestEdgeContainment` for an edge with one source and one target: the parent of
/// the two when they are siblings, the source or the target when one contains the other, and the
/// lowest common ancestor otherwise. A self loop is contained by the node's parent.
fn edge_container(
    edge: &ElkEdge,
    paths: &HashMap<String, Vec<usize>>,
) -> Result<Vec<usize>, LayoutError> {
    let (Some(source), Some(target)) = (edge.sources.first(), edge.targets.first()) else {
        return Err(LayoutError::Unsupported(format!(
            "edge {} without a source or a target",
            edge.id
        )));
    };
    let (Some(source_path), Some(target_path)) = (paths.get(source), paths.get(target)) else {
        return Err(LayoutError::Unsupported(format!(
            "edge {} to a node outside the graph",
            edge.id
        )));
    };
    if source_path == target_path {
        let mut parent = source_path.clone();
        if parent.pop().is_none() {
            return Err(LayoutError::Unsupported(format!(
                "a self loop on the root, edge {}",
                edge.id
            )));
        }
        return Ok(parent);
    }
    let common = source_path
        .iter()
        .zip(target_path)
        .take_while(|(a, b)| a == b)
        .count();
    Ok(source_path[..common].to_vec())
}

/// The node at a path of child indices from `root`.
fn node_at_mut<'a>(root: &'a mut ElkNode, path: &[usize]) -> Result<&'a mut ElkNode, LayoutError> {
    let mut node = root;
    for &index in path {
        node = node.children.get_mut(index).ok_or_else(|| {
            LayoutError::Format("an edge container outside the graph".to_string())
        })?;
    }
    Ok(node)
}

fn transform_node(json: &JsonNode) -> Result<ElkNode, LayoutError> {
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
        return Err(LayoutError::Unsupported(
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
    Ok(node)
}

/// `transformEdge`: the endpoints, the options and the labels. Endpoints at ports, and edges
/// without exactly one source and one target, are not supported yet.
fn transform_edge(json: &JsonEdge) -> Result<ElkEdge, LayoutError> {
    if json.source_port.is_some() || json.target_port.is_some() {
        return Err(LayoutError::Unsupported(format!(
            "edge {} at ports",
            json.id
        )));
    }
    let endpoints = |list: &[String], single: &Option<String>| -> Vec<String> {
        if list.is_empty() {
            single.iter().cloned().collect()
        } else {
            list.to_vec()
        }
    };
    let sources = endpoints(&json.sources, &json.source);
    let targets = endpoints(&json.targets, &json.target);
    if sources.len() != 1 || targets.len() != 1 {
        return Err(LayoutError::Unsupported(format!(
            "edge {} with several sources or targets",
            json.id
        )));
    }
    let mut edge = ElkEdge {
        id: json.id.clone(),
        sources,
        targets,
        labels: transform_labels(&json.labels)?,
        ..ElkEdge::default()
    };
    transform_properties(
        json.layout_options.as_ref(),
        json.properties.as_ref(),
        &mut edge.properties,
    )?;
    Ok(edge)
}

/// `transformProperties`: `layoutOptions` if present, else `properties`.
///
/// Every value is converted to its text before the first option is set, so a value that has no
/// text is reported even when an earlier key names an option type that is not ported.
fn transform_properties(
    layout_options: Option<&LayoutOptions>,
    properties: Option<&LayoutOptions>,
    holder: &mut PropertyHolder,
) -> Result<(), LayoutError> {
    let Some(options) = layout_options.or(properties) else {
        return Ok(());
    };
    let texts = options
        .values()
        .map(string_val)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| LayoutError::Format(error.message().to_string()))?;
    holder.reserve(options.len());
    for (key, text) in options.keys().zip(&texts) {
        set_option(holder, key, text)?;
    }
    Ok(())
}

/// `setOption`: suffix lookup, then parse; unknown keys and unparsable text are ignored.
fn set_option(holder: &mut PropertyHolder, key: &str, text: &str) -> Result<(), LayoutError> {
    let Some(option) = option_by_suffix(key) else {
        return Ok(());
    };
    match parse_value(option.spec(), text) {
        Ok(Some(value)) => {
            holder.set(option, Some(value));
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(unsupported) => Err(LayoutError::Unsupported(format!(
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

fn transform_port(json: &JsonPort) -> Result<ElkPort, LayoutError> {
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
fn transform_labels(labels: &[JsonLabel]) -> Result<Vec<ElkLabel>, LayoutError> {
    labels.iter().map(transform_label).collect()
}

fn transform_label(json: &JsonLabel) -> Result<ElkLabel, LayoutError> {
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
    use crate::core::data::{OptionId, OptionValue};
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
                .enum_of::<Direction>(OptionId::of("org.eclipse.elk.direction")),
            Some(Direction::RIGHT)
        );
        assert_eq!(
            node.properties
                .double_of(OptionId::of("org.eclipse.elk.layered.spacing.baseValue")),
            Some(40.0)
        );
        assert_eq!(node.children[0].shape.width, 50.0);
        assert_eq!(node.children[0].shape.x, 0.0);
    }

    #[test]
    fn edges_between_nodes_are_read_and_edges_at_ports_are_not() {
        let root = parse(json!({
            "id": "root",
            "children": [{ "id": "a" }, { "id": "b" }],
            "edges": [{
                "id": "e",
                "sources": ["a"],
                "targets": ["b"],
                "labels": [{ "text": "x", "width": 4, "height": 2 }]
            }]
        }));
        let node = import_graph(&root).unwrap();
        assert_eq!(node.edges.len(), 1);
        assert_eq!(node.edges[0].sources, vec!["a".to_string()]);
        assert_eq!(node.edges[0].labels.len(), 1);
        let at_ports = parse(json!({
            "id": "root",
            "children": [{ "id": "a" }, { "id": "b" }],
            "edges": [{ "id": "e", "sources": ["a"], "targets": ["b"], "sourcePort": "p" }]
        }));
        assert!(matches!(
            import_graph(&at_ports),
            Err(LayoutError::Unsupported(_))
        ));
    }

    #[test]
    fn layout_options_win_over_properties() {
        let root = parse(json!({
            "id": "root",
            "layoutOptions": { "spacing.baseValue": 40 },
            "properties": { "spacing.baseValue": 10, "elk.direction": "DOWN" }
        }));
        let node = import_graph(&root).unwrap();
        let base_value = OptionId::of("org.eclipse.elk.layered.spacing.baseValue");
        assert_eq!(node.properties.double_of(base_value), Some(40.0));
        assert!(
            !node
                .properties
                .has(OptionId::of("org.eclipse.elk.direction"))
        );
        let fallback = parse(json!({
            "id": "root",
            "properties": { "spacing.baseValue": 10 }
        }));
        let node = import_graph(&fallback).unwrap();
        assert_eq!(node.properties.double_of(base_value), Some(10.0));
    }

    #[test]
    fn numbers_and_booleans_are_read_through_their_javascript_text() {
        let root = parse(json!({
            "id": "root",
            "layoutOptions": {
                "layered.thoroughness": 7.0,
                "spacing.nodeNode": 0.5,
                "elk.separateConnectedComponents": false,
                "elk.spacing.edgeEdge": "not a number"
            }
        }));
        let node = import_graph(&root).unwrap();
        let properties = &node.properties;
        assert_eq!(
            properties.int_of(OptionId::of("org.eclipse.elk.layered.thoroughness")),
            Some(7)
        );
        assert_eq!(
            properties.double_of(OptionId::of("org.eclipse.elk.spacing.nodeNode")),
            Some(0.5)
        );
        assert_eq!(
            properties.get(OptionId::of("org.eclipse.elk.separateConnectedComponents")),
            Some(&OptionValue::Boolean(false))
        );
        assert!(!properties.has(OptionId::of("org.eclipse.elk.spacing.edgeEdge")));
    }

    #[test]
    fn a_later_key_for_the_same_option_overwrites_the_earlier_one() {
        let root = parse(json!({
            "id": "root",
            "layoutOptions": {
                "org.eclipse.elk.spacing.nodeNode": 20,
                "spacing.nodeNode": 30
            }
        }));
        let node = import_graph(&root).unwrap();
        assert_eq!(
            node.properties
                .double_of(OptionId::of("org.eclipse.elk.spacing.nodeNode")),
            Some(30.0)
        );
    }

    #[test]
    fn values_without_text_are_reported_before_unsupported_option_types() {
        let unsupported = parse(json!({
            "id": "root",
            "layoutOptions": { "elk.junctionPoints": "(1,2)" }
        }));
        assert!(matches!(
            import_graph(&unsupported),
            Err(LayoutError::Unsupported(message))
                if message == "option org.eclipse.elk.junctionPoints of type KVectorChain"
        ));
        for value in [json!(null), json!([1]), json!({ "a": 1 })] {
            let root = parse(json!({
                "id": "root",
                "layoutOptions": { "elk.junctionPoints": "(1,2)", "elk.direction": value }
            }));
            let expected = format!("Unsupported layout option value: {value}.");
            assert!(matches!(
                import_graph(&root),
                Err(LayoutError::Format(message)) if message == expected
            ));
        }
    }
}
