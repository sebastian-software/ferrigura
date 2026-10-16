//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph/transform/ElkGraphImporter.java
//!
//! The layered graph from the ELK graph: `ElkGraphImporter` and `ElkGraphTransformer.importGraph`.
//!
//! Graphs of plain nodes, with their inside labels, and edges between them, self loops included.
//! A level whose hierarchy handling is INCLUDE_CHILDREN becomes a top graph with nested graphs
//! (`importHierarchicalGraph`). Ports on nodes, inside self loops, and edges between nodes of
//! different graphs are reported as unsupported.

use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::core::data::OptionValue;
use crate::core::data::enums::{
    ComponentOrderingStrategy, EdgeLabelPlacement, GraphProperties, HierarchyHandling,
    OrderingStrategy, PortConstraints, PortSide,
};
use crate::core::data::property::PropertyHolder;
use crate::core::math::KVector;
use crate::error::LayoutError;
use crate::graph::{ElkEdge, ElkNode, ElkPort};
use crate::layered::graph_util::{create_port, note_port_side};
use crate::layered::lgraph::{
    CrossHierarchyEdge, LEdge, LGraph, LLabel, LNode, LPort, LabelId, PortType,
};
use crate::layered::nodespacing;
use crate::layered::options;
use crate::layered::spacings::{SPACING_BASE_VALUE, apply_base_value};

/// `importGraph` for a root whose children are plain nodes, joined by edges.
pub fn import_root(root: &ElkNode) -> Result<LGraph, LayoutError> {
    import_level(root, None, &root.id)
}

/// `ElkGraphImporter.importGraph` for the level of `root`: its children, as plain nodes, joined by
/// edges. `parent` holds the options of `root`'s parent, which the Java reads for the spacings of
/// `root`'s inside labels and for its minimum size; it is `None` for the root of the graph. `chain`
/// is `root`'s designation prefix: the ids from the root of the graph down to `root`.
pub fn import_level(
    root: &ElkNode,
    parent: Option<&PropertyHolder>,
    chain: &str,
) -> Result<LGraph, LayoutError> {
    let mut graph = create_graph(root, parent, chain)?;
    let needs_model_order = needs_model_order_based_on_parent(root);
    let mut model_order = 0;
    // The layered ports of this level's nodes, by their ELK ids.
    let mut ports: HashMap<String, usize> = HashMap::new();
    for child in &root.children {
        // A child with children is a leaf of this level once it was laid out on its own, which its
        // hierarchy handling says. Any other child with children is a nested graph of this level.
        if !child.children.is_empty()
            && child
                .properties
                .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
                != Some(HierarchyHandling::SEPARATE_CHILDREN)
        {
            return Err(LayoutError::Unsupported(format!(
                "nested graph in {}",
                child.id
            )));
        }
        // `needsModelOrder(child)`: the parent asks for the input order, and the child does not opt out.
        let ordered = needs_model_order
            && !child
                .properties
                .bool_of(options::CONSIDER_MODEL_ORDER_NO_MODEL_ORDER);
        let node_model_order = ordered.then(|| {
            model_order += 1;
            model_order - 1
        });
        let node = transform_node(
            child,
            child.properties.clone(),
            &mut graph,
            node_model_order,
        )?;
        import_ports(child, node, &mut graph, &mut ports)?;
    }
    graph.layerless_nodes = (0..graph.nodes.len()).collect();
    import_edges(root, &mut graph, needs_model_order, &ports)?;
    Ok(graph)
}

/// `ElkGraphImporter.createLGraph` for the graph of `root`, with what `importGraph` (or, for a
/// nested graph, `importHierarchicalGraph`) does before the children are imported: the minimum
/// size and the spacings of the base value. `parent` holds the options of `root`'s parent ELK node,
/// `None` for the root of the whole graph; `chain` is `root`'s designation prefix.
fn create_graph(
    root: &ElkNode,
    parent: Option<&PropertyHolder>,
    chain: &str,
) -> Result<LGraph, LayoutError> {
    let mut graph = LGraph {
        properties: root.properties.clone(),
        root_id: chain.to_string(),
        ..LGraph::default()
    };
    // `ElkGraphImporter`: the root's padding is the margin the drawing keeps inside the root node.
    graph.padding = match root.properties.value_or_default(options::PADDING) {
        Some(OptionValue::Padding(spacing)) => spacing,
        _ => {
            return Err(LayoutError::Unsupported(
                "a root without a padding value".to_string(),
            ));
        }
    };
    // `createLGraph`: the padding also leaves room for the root's own inside labels. Without labels
    // that room is zero, so the node spacing code is not asked.
    if !root.labels.is_empty() {
        let label_padding = nodespacing::inside_label_padding(&label_probe(root, parent), 0, true)?;
        graph.padding.top += label_padding.top;
        graph.padding.left += label_padding.left;
        graph.padding.bottom += label_padding.bottom;
        graph.padding.right += label_padding.right;
    }
    // `calculateMinimumGraphSize`: a nested graph's node with size constraints takes a minimum size
    // from its labels, and the configured minimum, and the graph gets the MINIMUM_SIZE constraint.
    // The Java also writes the result into the node's own options, through shared objects; see
    // `shared_minimum_size` for the node of a nested graph.
    if let Some(parent) = parent
        && !root
            .properties
            .enum_set_is_empty(options::NODE_SIZE_CONSTRAINTS)
    {
        let probe = label_probe(root, Some(parent));
        let minimum = nodespacing::minimum_size_of(&probe, 0)?;
        let configured = match graph
            .properties
            .value_or_default(options::NODE_SIZE_MINIMUM)
        {
            Some(OptionValue::Vector(vector)) => vector,
            _ => KVector::default(),
        };
        let mut names = match root.properties.get(options::NODE_SIZE_CONSTRAINTS) {
            Some(OptionValue::EnumSet { names, .. }) => names.clone(),
            _ => Vec::new(),
        };
        if !names.iter().any(|name| name == "MINIMUM_SIZE") {
            names.push("MINIMUM_SIZE".to_string());
        }
        graph.properties.set(
            options::NODE_SIZE_CONSTRAINTS,
            Some(OptionValue::EnumSet {
                class: "SizeConstraint",
                names,
            }),
        );
        graph.properties.set(
            options::NODE_SIZE_MINIMUM,
            Some(OptionValue::Vector(KVector {
                x: minimum.x.max(configured.x),
                y: minimum.y.max(configured.y),
            })),
        );
    }
    // `importGraph`: the base value derives the spacings before the graph is laid out.
    if root.properties.has(SPACING_BASE_VALUE)
        && let Some(base) = root.properties.double_of(SPACING_BASE_VALUE)
    {
        apply_base_value(&mut graph, base);
    }
    Ok(graph)
}

/// `transformNode`: the ELK node `child` becomes a node of `graph`, with `properties` as its
/// options. Returns its index in `graph.nodes`; the caller puts it in the layerless list.
fn transform_node(
    child: &ElkNode,
    mut properties: PropertyHolder,
    graph: &mut LGraph,
    model_order: Option<usize>,
) -> Result<usize, LayoutError> {
    // A node without port constraints is free; any other constraint is not supported by the port
    // sides yet.
    match properties.enum_of::<PortConstraints>(options::PORT_CONSTRAINTS) {
        None | Some(PortConstraints::UNDEFINED) => properties.set(
            options::PORT_CONSTRAINTS,
            Some(OptionValue::Enum {
                class: "PortConstraints",
                name: "FREE".to_string(),
            }),
        ),
        Some(PortConstraints::FREE) => {}
        Some(_) => {
            graph
                .graph_properties
                .insert(GraphProperties::NON_FREE_PORTS);
        }
    }
    let labels = import_node_labels(child, graph);
    // `LNode.getDesignation()`: the first label's text when there is one, else the ids of the
    // ELK node and its ancestors up to the root of the graph.
    let designation = match labels.first() {
        Some(&first) => graph.labels[first].text.clone(),
        None => format!("{}.{}", graph.root_id, child.id),
    };
    graph.nodes.push(LNode {
        id: child.id.clone(),
        designation,
        properties,
        size: KVector {
            x: child.shape.width,
            y: child.shape.height,
        },
        model_order,
        labels,
        ..LNode::default()
    });
    Ok(graph.nodes.len() - 1)
}

/// `ElkGraphImporter.importGraph` for a level whose hierarchy handling is INCLUDE_CHILDREN, with
/// `importHierarchicalGraph`: the children of `root` and of every descendant that is laid out in
/// the same run become nodes of nested graphs. Returns the graphs of the layout: the graph of
/// `root` first, then the nested graphs in the order they were created. A node holds the index of
/// the graph nested in it, and the graph the index of its node.
///
/// An edge whose ends are in different graphs goes into the top graph's list of cross-hierarchy
/// edges, for the compound graph preprocessor.
pub fn import_compound(
    root: &ElkNode,
    parent: Option<&PropertyHolder>,
    chain: &str,
) -> Result<Vec<LGraph>, LayoutError> {
    let top = create_graph(root, parent, chain)?;
    // `parentGraphDirection`: every nested graph takes the direction of the top graph, whatever its
    // own node asks for.
    let direction = top.properties.value_or_default(options::DIRECTION);
    let mut graphs = vec![top];
    // Where each ELK node went, by its path of child indices from `root`: its graph and its index.
    let mut node_map: HashMap<Vec<usize>, (usize, usize)> = HashMap::new();
    // `elkGraphQueue`: the ELK nodes, breadth first, with the graph each one becomes a node of.
    let mut queue: VecDeque<(Vec<usize>, usize)> = (0..root.children.len())
        .map(|index| (vec![index], 0))
        .collect();
    // The model order counts the nodes of all graphs, in the order they are read.
    let mut model_order = 0;
    // The layered ports by their ELK ids: the path of their node, their graph and their index.
    let mut port_map: HashMap<String, (Vec<usize>, usize, usize)> = HashMap::new();
    while let Some((path, graph_index)) = queue.pop_front() {
        let (elk_parent, elk_node) = parent_and_node(root, &path)?;
        // `needsModelOrder(elknode)`: the node's parent asks for the input order, and the node
        // does not opt out.
        let ordered = needs_model_order_based_on_parent(elk_parent)
            && !elk_node
                .properties
                .bool_of(options::CONSIDER_MODEL_ORDER_NO_MODEL_ORDER);
        let node_model_order = ordered.then(|| {
            model_order += 1;
            model_order - 1
        });
        if elk_node.properties.bool_of(options::NO_LAYOUT) {
            continue;
        }
        if elk_node
            .properties
            .bool_of(options::INSIDE_SELF_LOOPS_ACTIVATE)
        {
            return Err(LayoutError::Unsupported(format!(
                "insideSelfLoops.activate on {} (inside self loops)",
                elk_node.id
            )));
        }
        // The node needs a graph of its own if it has children laid out in this run.
        let nested = if uses_elk_layered(&elk_node.properties)
            && elk_node
                .properties
                .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
                == Some(HierarchyHandling::INCLUDE_CHILDREN)
            && !elk_node.children.is_empty()
        {
            let nested_chain = format!("{}.{}", graphs[graph_index].root_id, elk_node.id);
            let mut nested = create_graph(elk_node, Some(&elk_parent.properties), &nested_chain)?;
            nested.properties.set(options::DIRECTION, direction.clone());
            Some(nested)
        } else {
            if !elk_node.children.is_empty()
                && elk_node
                    .properties
                    .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
                    != Some(HierarchyHandling::SEPARATE_CHILDREN)
            {
                return Err(LayoutError::Unsupported(format!(
                    "nested graph in {}",
                    elk_node.id
                )));
            }
            None
        };
        let properties = match &nested {
            Some(nested) => shared_minimum_size(elk_node, nested),
            None => elk_node.properties.clone(),
        };
        let graph = &mut graphs[graph_index];
        let index = transform_node(elk_node, properties, graph, node_model_order)?;
        graph.layerless_nodes.push(index);
        node_map.insert(path.clone(), (graph_index, index));
        let mut ports = HashMap::new();
        import_ports(elk_node, index, &mut graphs[graph_index], &mut ports)?;
        for (id, port) in ports {
            port_map.insert(id, (path.clone(), graph_index, port));
        }
        if let Some(mut nested) = nested {
            let nested_index = graphs.len();
            nested.parent_node = Some((graph_index, index));
            graphs[graph_index].nodes[index].nested_graph = Some(nested_index);
            graphs.push(nested);
            for child in 0..elk_node.children.len() {
                let mut child_path = path.clone();
                child_path.push(child);
                queue.push_back((child_path, nested_index));
            }
        }
    }
    import_compound_edges(root, &mut graphs, &node_map, &port_map)?;
    Ok(graphs)
}

/// The options the node of a nested graph is created with. `calculateMinimumGraphSize` changes
/// the graph's size constraints and minimum size in place, and the Java graph shares those objects
/// with the ELK node, whose options `transformNode` copies afterwards: the node sees the changes
/// where the ELK node set the option itself. (An unset minimum is a default the graph alone holds.)
fn shared_minimum_size(elk_node: &ElkNode, nested: &LGraph) -> PropertyHolder {
    let mut properties = elk_node.properties.clone();
    for id in [options::NODE_SIZE_CONSTRAINTS, options::NODE_SIZE_MINIMUM] {
        if properties.has(id) {
            properties.set(id, nested.properties.get(id).cloned());
        }
    }
    properties
}

/// `LayeredOptions.ALGORITHM_ID.endsWith(algorithm)`: the node has no algorithm, or names the
/// layered one.
fn uses_elk_layered(properties: &PropertyHolder) -> bool {
    match properties.get(options::ALGORITHM) {
        None => true,
        Some(OptionValue::Algorithm(name)) => "org.eclipse.elk.layered".ends_with(name.as_str()),
        Some(_) => false,
    }
}

/// The ELK node at `path` below `root`, with its parent.
fn parent_and_node<'a>(
    root: &'a ElkNode,
    path: &[usize],
) -> Result<(&'a ElkNode, &'a ElkNode), LayoutError> {
    let Some((&last, ancestors)) = path.split_last() else {
        return Err(LayoutError::Format("an empty node path".to_string()));
    };
    let parent = node_at(root, ancestors)?;
    let node = parent
        .children
        .get(last)
        .ok_or_else(|| LayoutError::Format("a node path outside the graph".to_string()))?;
    Ok((parent, node))
}

/// The ELK node at `path` below `root`; `root` itself for the empty path.
fn node_at<'a>(root: &'a ElkNode, path: &[usize]) -> Result<&'a ElkNode, LayoutError> {
    let mut node = root;
    for &index in path {
        node = node
            .children
            .get(index)
            .ok_or_else(|| LayoutError::Format("a node path outside the graph".to_string()))?;
    }
    Ok(node)
}

/// The paths of `node` and its descendants by id, below `prefix`.
fn collect_paths<'a>(
    node: &'a ElkNode,
    prefix: &mut Vec<usize>,
    paths: &mut HashMap<&'a str, Vec<usize>>,
) {
    paths.insert(node.id.as_str(), prefix.clone());
    for (index, child) in node.children.iter().enumerate() {
        prefix.push(index);
        collect_paths(child, prefix, paths);
        prefix.pop();
    }
}

/// `importHierarchicalGraph`'s edges: the ELK nodes laid out in this run are read breadth first,
/// starting with `root`, and the edges each one contains go into the graph of the node the edge
/// belongs to. The model order counts the edges of all graphs, by the top graph's options.
fn import_compound_edges(
    root: &ElkNode,
    graphs: &mut [LGraph],
    node_map: &HashMap<Vec<usize>, (usize, usize)>,
    port_map: &HashMap<String, (Vec<usize>, usize, usize)>,
) -> Result<(), LayoutError> {
    let mut paths = HashMap::new();
    collect_paths(root, &mut Vec::new(), &mut paths);
    let needs_model_order = needs_model_order_based_on_parent(root);
    let mut model_order = 0;
    let mut queue: VecDeque<Vec<usize>> = VecDeque::from([Vec::new()]);
    while let Some(path) = queue.pop_front() {
        let container = node_at(root, &path)?;
        for edge in &container.edges {
            // `checkEdgeValidity`: one source and one target.
            let (Some(source), Some(target)) = (edge.sources.first(), edge.targets.first()) else {
                return Err(LayoutError::Unsupported(format!(
                    "edge {} without a source or a target",
                    edge.id
                )));
            };
            if edge.sources.len() > 1 || edge.targets.len() > 1 {
                return Err(LayoutError::Unsupported(format!(
                    "hyperedge {} (several sources or targets)",
                    edge.id
                )));
            }
            let edge_model_order = needs_model_order.then(|| {
                model_order += 1;
                model_order - 1
            });
            let (Some((source_path, source_explicit)), Some((target_path, target_explicit))) = (
                end_of_edge(source, &paths, port_map),
                end_of_edge(target, &paths, port_map),
            ) else {
                return Err(LayoutError::Unsupported(format!(
                    "edge {} to a node outside the layout",
                    edge.id
                )));
            };
            let source_node = node_at(root, source_path)?;
            let target_node = node_at(root, target_path)?;
            if edge.properties.bool_of(options::NO_LAYOUT)
                || source_node.properties.bool_of(options::NO_LAYOUT)
                || target_node.properties.bool_of(options::NO_LAYOUT)
            {
                continue;
            }
            // The graph the edge goes into: the inside of its source or its target when the other
            // end is a descendant, else the graph of the ELK node that contains it.
            let parent_path = if is_descendant(target_path, source_path) {
                source_path
            } else if is_descendant(source_path, target_path) {
                target_path
            } else {
                &path
            };
            let parent_graph = if parent_path.is_empty() {
                0
            } else {
                node_map
                    .get(parent_path)
                    .and_then(|&(graph, node)| graphs[graph].nodes[node].nested_graph)
                    .ok_or_else(|| {
                        LayoutError::Unsupported(format!(
                            "edge {} inside a node without a nested graph",
                            edge.id
                        ))
                    })?
            };
            let (Some(&(source_graph, source_index)), Some(&(target_graph, target_index))) =
                (node_map.get(source_path), node_map.get(target_path))
            else {
                return Err(LayoutError::Unsupported(format!(
                    "the source or the target of edge {} could not be found",
                    edge.id
                )));
            };
            if source_graph != parent_graph || target_graph != parent_graph {
                if source_explicit.is_some() || target_explicit.is_some() {
                    return Err(LayoutError::Unsupported(format!(
                        "edge {} across levels from an explicit port",
                        edge.id
                    )));
                }
                // `transformEdge` for ends in different graphs: each end gets a port in its own
                // graph, and the edge joins them across the hierarchy. The labels set their graph
                // properties on the graph the edge is placed in.
                let source_port =
                    create_port(&mut graphs[source_graph], source_index, PortType::OUTPUT);
                let target_port =
                    create_port(&mut graphs[target_graph], target_index, PortType::INPUT);
                let labels = edge_labels(edge, &mut graphs[parent_graph].graph_properties);
                // `findCoordinateSystemOrigin`: an edge whose ends are not one inside the other has
                // its coordinates relative to the graph of its ELK container.
                let related = is_descendant(target_path, source_path)
                    || is_descendant(source_path, target_path);
                graphs[0].cross_hierarchy_edges.push(CrossHierarchyEdge {
                    id: edge.id.clone(),
                    properties: edge.properties.clone(),
                    model_order: edge_model_order,
                    thickness: 1.0,
                    source: (source_graph, source_port),
                    target: (target_graph, target_port),
                    labels,
                    coordinate_system_origin: (!related).then_some(parent_graph),
                    attached: true,
                    ..CrossHierarchyEdge::default()
                });
                continue;
            }
            add_edge(
                &mut graphs[parent_graph],
                edge,
                (source_index, source_explicit.map(|(_, port)| port)),
                (target_index, target_explicit.map(|(_, port)| port)),
                edge_model_order,
            );
        }
        // The children laid out in this run contain edges of their own.
        if container
            .properties
            .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
            == Some(HierarchyHandling::INCLUDE_CHILDREN)
        {
            for (index, child) in container.children.iter().enumerate() {
                if uses_elk_layered(&child.properties)
                    && child
                        .properties
                        .enum_of::<HierarchyHandling>(options::HIERARCHY_HANDLING)
                        == Some(HierarchyHandling::INCLUDE_CHILDREN)
                {
                    let mut child_path = path.clone();
                    child_path.push(index);
                    queue.push_back(child_path);
                }
            }
        }
    }
    Ok(())
}

/// `ElkGraphUtil.isDescendant(child, ancestor)` by paths: `child` is below `ancestor`.
fn is_descendant(child: &[usize], ancestor: &[usize]) -> bool {
    child.len() > ancestor.len() && child.starts_with(ancestor)
}

/// `transformNode`'s labels: those with text and laid out, each with its origin, the index among the
/// ELK node's labels, so the position can be written back to the right label.
fn import_node_labels(node: &ElkNode, graph: &mut LGraph) -> Vec<LabelId> {
    let mut labels = Vec::new();
    for (index, label) in node.labels.iter().enumerate() {
        let text = label.text.clone().unwrap_or_default();
        if text.is_empty() || label.properties.bool_of(options::NO_LAYOUT) {
            continue;
        }
        let id = graph.add_label(LLabel {
            text,
            size: KVector {
                x: label.shape.width,
                y: label.shape.height,
            },
            position: KVector {
                x: label.shape.x,
                y: label.shape.y,
            },
            properties: label.properties.clone(),
            end_label_edge: None,
            origin: Some(index),
        });
        labels.push(id);
    }
    labels
}

/// The graph a node's inside labels are measured in: the node alone, with all of its labels (the
/// Java's ELK node adapter gives every label, even one without text), and the properties of its
/// parent, which is where the Java reads the spacings (none for a root).
fn label_probe(node: &ElkNode, parent: Option<&PropertyHolder>) -> LGraph {
    let mut probe = LGraph {
        properties: parent.cloned().unwrap_or_default(),
        ..LGraph::default()
    };
    let mut labels = Vec::new();
    for label in &node.labels {
        labels.push(probe.add_label(LLabel {
            text: label.text.clone().unwrap_or_default(),
            size: KVector {
                x: label.shape.width,
                y: label.shape.height,
            },
            position: KVector {
                x: label.shape.x,
                y: label.shape.y,
            },
            properties: label.properties.clone(),
            end_label_edge: None,
            origin: None,
        }));
    }
    probe.nodes.push(LNode {
        id: node.id.clone(),
        properties: node.properties.clone(),
        size: KVector {
            x: node.shape.width,
            y: node.shape.height,
        },
        labels,
        ..LNode::default()
    });
    probe
}

/// `importFlatGraph` for the root's edges: each edge gets a port of its own on its source and
/// on its target node, as `transformEdge` creates them for edges that meet nodes.
fn import_edges(
    root: &ElkNode,
    graph: &mut LGraph,
    needs_model_order: bool,
    ports: &HashMap<String, usize>,
) -> Result<(), LayoutError> {
    let index_of: HashMap<String, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();
    let mut model_order = 0;
    for edge in &root.edges {
        // The model order counts every edge of the graph, laid out or not.
        let edge_model_order = needs_model_order.then(|| {
            model_order += 1;
            model_order - 1
        });
        if edge.properties.bool_of(options::NO_LAYOUT) {
            continue;
        }
        let (Some(source), Some(target)) = (edge.sources.first(), edge.targets.first()) else {
            return Err(LayoutError::Unsupported(format!(
                "edge {} without a source or a target",
                edge.id
            )));
        };
        let (Some(source), Some(target)) = (
            end_in_level(source, &index_of, ports, graph),
            end_in_level(target, &index_of, ports, graph),
        ) else {
            return Err(LayoutError::Unsupported(format!(
                "edge {} to a node outside the root",
                edge.id
            )));
        };
        add_edge(graph, edge, source, target, edge_model_order);
    }
    Ok(())
}

/// `ElkGraphImporter.transformNode`'s ports: each laid out ELK port of `elk_node` becomes a port of the
/// layered node `node`, recorded by its ELK id.
fn import_ports(
    elk_node: &ElkNode,
    node: usize,
    graph: &mut LGraph,
    ports: &mut HashMap<String, usize>,
) -> Result<(), LayoutError> {
    for elk_port in &elk_node.ports {
        if elk_port.properties.bool_of(options::NO_LAYOUT) {
            continue;
        }
        let index = import_port(graph, node, elk_port)?;
        ports.insert(elk_port.id.clone(), index);
    }
    Ok(())
}

/// `ElkGraphImporter.transformPort` with `LGraphUtil.initializePort`, for a port whose side is set:
/// the side, size and position of the ELK port, the anchor its side gives, and its border offset.
/// Port labels, an explicit anchor, and a port without a side are not ported yet.
fn import_port(graph: &mut LGraph, node: usize, elk_port: &ElkPort) -> Result<usize, LayoutError> {
    if !elk_port.labels.is_empty() {
        return Err(LayoutError::Unsupported(format!(
            "labels on port {}",
            elk_port.id
        )));
    }
    if elk_port.properties.has(options::PORT_ANCHOR) {
        return Err(LayoutError::Unsupported(format!(
            "an explicit anchor on port {}",
            elk_port.id
        )));
    }
    let side = match elk_port.properties.enum_of::<PortSide>(options::PORT_SIDE) {
        Some(side) if side != PortSide::UNDEFINED => side,
        _ => {
            return Err(LayoutError::Unsupported(format!(
                "port {} without a side",
                elk_port.id
            )));
        }
    };
    let size = KVector {
        x: elk_port.shape.width,
        y: elk_port.shape.height,
    };
    let mut port = LPort::new(node, side);
    port.size = size;
    port.position = KVector {
        x: elk_port.shape.x,
        y: elk_port.shape.y,
    };
    port.anchor = side_anchor(side, size);
    port.border_offset = elk_port
        .properties
        .double_of(options::PORT_BORDER_OFFSET)
        .unwrap_or(0.0);
    note_port_side(graph, side);
    graph.ports.push(port);
    let index = graph.ports.len() - 1;
    graph.nodes[node].ports.push(index);
    Ok(index)
}

/// `LGraphUtil.initializePort`'s anchor for a fixed side: the point of the port that the edges meet,
/// on the border of the side.
fn side_anchor(side: PortSide, size: KVector) -> KVector {
    match side {
        PortSide::NORTH => KVector {
            x: size.x / 2.0,
            y: 0.0,
        },
        PortSide::EAST => KVector {
            x: size.x,
            y: size.y / 2.0,
        },
        PortSide::SOUTH => KVector {
            x: size.x / 2.0,
            y: size.y,
        },
        PortSide::WEST => KVector {
            x: 0.0,
            y: size.y / 2.0,
        },
        PortSide::UNDEFINED => KVector {
            x: size.x / 2.0,
            y: size.y / 2.0,
        },
    }
}

/// The node an edge end of this level is at, and the port it names when it names one.
fn end_in_level(
    id: &str,
    index_of: &HashMap<String, usize>,
    ports: &HashMap<String, usize>,
    graph: &LGraph,
) -> Option<(usize, Option<usize>)> {
    if let Some(&port) = ports.get(id) {
        return Some((graph.ports[port].node, Some(port)));
    }
    index_of.get(id).map(|&node| (node, None))
}

/// An edge end of a compound level: the path of its node, and the graph and index of the port it
/// names, when it names one.
type EdgeEnd<'a> = (&'a Vec<usize>, Option<(usize, usize)>);

/// The path of the node an edge end is at, and the graph and index of the port it names when it
/// names one.
fn end_of_edge<'a>(
    id: &str,
    paths: &'a HashMap<&str, Vec<usize>>,
    ports: &'a HashMap<String, (Vec<usize>, usize, usize)>,
) -> Option<EdgeEnd<'a>> {
    if let Some(path) = paths.get(id) {
        return Some((path, None));
    }
    ports
        .get(id)
        .map(|(path, graph, port)| (path, Some((*graph, *port))))
}

/// `transformEdge` for an edge between two nodes of `graph`: the edge gets a port of its own on its
/// source and on its target node, and its labels.
fn add_edge(
    graph: &mut LGraph,
    edge: &ElkEdge,
    (source_node, explicit_source): (usize, Option<usize>),
    (target_node, explicit_target): (usize, Option<usize>),
    model_order: Option<usize>,
) {
    // `LGraphUtil.computeGraphProperties`: an edge from a node to itself is a self loop.
    if source_node == target_node {
        graph.graph_properties.insert(GraphProperties::SELF_LOOPS);
    }
    // An edge that names a port leaves or enters that port; otherwise it gets a port of its own.
    let source_port =
        explicit_source.unwrap_or_else(|| create_port(graph, source_node, PortType::OUTPUT));
    let target_port =
        explicit_target.unwrap_or_else(|| create_port(graph, target_node, PortType::INPUT));
    let index = graph.edges.len();
    graph.ports[source_port].outgoing.push(index);
    graph.ports[target_port].incoming.push(index);
    // `transformEdge`: a port with several incoming or several outgoing edges makes the graph one
    // with HYPEREDGES. Only explicit ports can be shared, since each other end gets its own port.
    if [source_port, target_port]
        .into_iter()
        .any(|port| graph.ports[port].incoming.len() > 1 || graph.ports[port].outgoing.len() > 1)
    {
        graph.graph_properties.insert(GraphProperties::HYPEREDGES);
    }
    let labels = import_edge_labels(edge, graph);
    graph.edges.push(LEdge {
        id: edge.id.clone(),
        source: source_port,
        target: target_port,
        labels,
        model_order,
        properties: edge.properties.clone(),
        reversed: false,
        bend_points: Vec::new(),
        thickness: 1.0,
    });
}

/// `transformEdge`'s labels: those with text and laid out, with their placement recorded. A
/// centred label makes the graph need label dummies, a head or tail one end labels.
fn import_edge_labels(edge: &ElkEdge, graph: &mut LGraph) -> Vec<LabelId> {
    let labels = edge_labels(edge, &mut graph.graph_properties);
    labels
        .into_iter()
        .map(|label| graph.add_label(label))
        .collect()
}

/// `transformEdge`'s labels as values, with the graph properties of the graph the edge is placed in.
fn edge_labels(edge: &ElkEdge, graph_properties: &mut BTreeSet<GraphProperties>) -> Vec<LLabel> {
    let mut labels = Vec::new();
    for (index, label) in edge.labels.iter().enumerate() {
        let text = label.text.clone().unwrap_or_default();
        if text.is_empty() || label.properties.bool_of(options::NO_LAYOUT) {
            continue;
        }
        let placement = label
            .properties
            .enum_with_default::<EdgeLabelPlacement>(options::EDGE_LABELS_PLACEMENT);
        let mut properties = label.properties.clone();
        match placement {
            EdgeLabelPlacement::HEAD | EdgeLabelPlacement::TAIL => {
                graph_properties.insert(GraphProperties::END_LABELS);
            }
            EdgeLabelPlacement::CENTER => {
                graph_properties.insert(GraphProperties::CENTER_LABELS);
                properties.set(
                    options::EDGE_LABELS_PLACEMENT,
                    Some(OptionValue::Enum {
                        class: "EdgeLabelPlacement",
                        name: "CENTER".to_string(),
                    }),
                );
            }
        }
        labels.push(LLabel {
            text,
            size: KVector {
                x: label.shape.width,
                y: label.shape.height,
            },
            position: KVector::default(),
            properties,
            end_label_edge: None,
            origin: Some(index),
        });
    }
    labels
}

/// `ElkGraphImporter.needsModelOrderBasedOnParent`: whether the graph's options use the input order.
fn needs_model_order_based_on_parent(root: &ElkNode) -> bool {
    let props = &root.properties;
    let strategy_is = |id: &str, name: &str| {
        props.value_or_default(id).is_some_and(|value| match value {
            crate::core::data::OptionValue::Enum { name: found, .. } => found == name,
            _ => false,
        })
    };
    let considers_order = props
        .enum_of::<OrderingStrategy>(options::CONSIDER_MODEL_ORDER_STRATEGY)
        .is_some_and(|strategy| strategy != OrderingStrategy::NONE);
    considers_order
        || strategy_is(options::CYCLE_BREAKING_STRATEGY, "MODEL_ORDER")
        || strategy_is(options::CYCLE_BREAKING_STRATEGY, "GREEDY_MODEL_ORDER")
        || props.bool_of(options::FORCE_NODE_MODEL_ORDER)
        || props
            .enum_of::<ComponentOrderingStrategy>(options::CONSIDER_MODEL_ORDER_COMPONENTS)
            .is_some_and(|strategy| strategy != ComponentOrderingStrategy::NONE)
        || strategy_is(
            options::NODE_PROMOTION_STRATEGY,
            "MODEL_ORDER_LEFT_TO_RIGHT",
        )
        || strategy_is(
            options::NODE_PROMOTION_STRATEGY,
            "MODEL_ORDER_RIGHT_TO_LEFT",
        )
        || strategy_is(options::LAYERING_STRATEGY, "BF_MODEL_ORDER")
        || strategy_is(options::LAYERING_STRATEGY, "DF_MODEL_ORDER")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `elk.layered.edgeRouting.selfLoopDistribution` is a node option. Set on the root only, it is
    /// the root's value, and the child keeps its own, as Java's `copyProperties` gives it.
    #[test]
    fn a_root_only_self_loop_distribution_does_not_reach_the_children() {
        let mut root = ElkNode {
            id: "root".to_string(),
            ..ElkNode::default()
        };
        root.properties.set(
            options::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION,
            Some(OptionValue::Enum {
                class: "SelfLoopDistributionStrategy",
                name: "EQUALLY".to_string(),
            }),
        );
        root.children.push(ElkNode {
            id: "entity".to_string(),
            ..ElkNode::default()
        });

        let graph = import_root(&root).unwrap();

        assert!(
            graph
                .properties
                .has(options::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION)
        );
        assert!(
            !graph.nodes[0]
                .properties
                .has(options::EDGE_ROUTING_SELF_LOOP_DISTRIBUTION)
        );
    }
}
