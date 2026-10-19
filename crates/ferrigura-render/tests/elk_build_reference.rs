//! The ELK graph that `flowchart::elk_build::build` makes is the graph the reference hands to
//! elkjs, for every flowchart fixture of `e2e/diagrams/flowchart`.
//!
//! `tests/data/flowchart-elk-input.json` holds, per fixture, the reference's ELK input as it was
//! captured in Chromium: the graph as `runElkLayout` receives it, after the reversal of feedback
//! edges and the entry constraint, before elkjs runs (`render.ts`, `runElkLayout`). The graph
//! carries the layout data the reference built it from (`parentId`, `labelBBox`, `dir`, `metadata`,
//! `padding`, `shape`, the measured `width` and `height` of every node and edge label), so the
//! layout data is rebuilt from it. The sizes are therefore the reference's measured sizes, and this
//! test checks the graph construction alone, not the measurement.
//!
//! The comparison is exact: ids and their order, sizes, labels and their sizes, the layout options
//! with their order and their JSON types, and the edges with their order, their ends and the ones
//! handed to ELK reversed. Fixtures that ferrigura does not lay out as the reference does are listed
//! with their reason and not compared.
//!
//! Fields of the reference's graph that ferrigura does not carry are left out of the comparison:
//! the application fields of the graph (`isGroup` on a leaf, `labelData`, `orgWidth`, `orgHeight`,
//! `sourceId`, `targetId`, ...). None of them is read by elkjs.

use std::collections::{BTreeMap, HashMap};

use ferrigura_elk::json::{JsonEdge, JsonLabel, JsonNode, LayoutOptions};
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::theme::ThemeTokens;
use ferrigura_model::value::Value;
use ferrigura_render::context::Globals;
use ferrigura_render::unified::data::{Edge, LayoutData, Node};
use ferrigura_render::unified::elk_build::{ElkInput, ElkState, build};
use ferrigura_render::unified::{Config, Section};
use serde::Deserialize;

/// The captured reference inputs, one per fixture.
const CORPUS: &str = include_str!("data/flowchart-elk-input.json");

/// The fixtures that feature the graphs the port must reproduce, counted in the summary.
const MINIMUM_FIXTURES: usize = 40;

#[derive(Deserialize)]
struct Corpus {
    fixtures: Vec<Fixture>,
}

#[derive(Deserialize)]
struct Fixture {
    name: String,
    /// The `elk` settings the fixture sets itself; every other fixture runs with the defaults.
    #[serde(default)]
    config: Option<serde_json::Value>,
    /// Why the fixture is not compared, when it is not.
    #[serde(default)]
    excluded: Option<String>,
    input: Graph,
}

/// The root of a captured graph.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Graph {
    layout_options: Option<LayoutOptions>,
    #[serde(default)]
    children: Vec<CapturedNode>,
    #[serde(default)]
    edges: Vec<CapturedEdge>,
}

/// A node of a captured graph: an ELK node with the layout data it was built from.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapturedNode {
    id: String,
    #[serde(default)]
    is_group: bool,
    #[serde(default)]
    label: String,
    parent_id: Option<String>,
    #[serde(default)]
    padding: f64,
    #[serde(default)]
    shape: String,
    dir: Option<String>,
    metadata: Option<serde_json::Value>,
    #[serde(rename = "labelBBox")]
    label_bbox: Option<Size>,
    width: Option<f64>,
    height: Option<f64>,
    layout_options: Option<LayoutOptions>,
    #[serde(default)]
    labels: Vec<JsonLabel>,
    #[serde(default)]
    children: Vec<CapturedNode>,
}

/// A size as the reference's layout data writes it.
#[derive(Deserialize)]
struct Size {
    width: f64,
    height: f64,
}

/// An edge of a captured graph: an ELK edge with the layout data it was built from.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapturedEdge {
    id: String,
    #[serde(default)]
    start: String,
    #[serde(default)]
    end: String,
    #[serde(default)]
    label: String,
    width: Option<f64>,
    height: Option<f64>,
    sources: Vec<String>,
    targets: Vec<String>,
    #[serde(default)]
    labels: Vec<JsonLabel>,
    #[serde(default)]
    layout_reversed: bool,
}

/// The layout data a captured graph was built from, rebuilt field by field.
fn layout_data(graph: &Graph) -> LayoutData {
    let mut nodes = Vec::new();
    for child in &graph.children {
        flatten(child, &mut nodes);
    }
    let edges = graph
        .edges
        .iter()
        .map(|edge| Edge {
            id: edge.id.clone(),
            start: edge.start.clone(),
            end: edge.end.clone(),
            label: edge.label.clone(),
            width: edge.width,
            height: edge.height,
            ..Edge::default()
        })
        .collect();
    LayoutData {
        nodes,
        edges,
        direction: direction_of(graph),
        ..LayoutData::default()
    }
}

/// The nodes of a captured subtree, parents before their members, as `flatten` lists them.
fn flatten(node: &CapturedNode, out: &mut Vec<Node>) {
    out.push(Node {
        id: node.id.clone(),
        label: node.label.clone(),
        is_group: node.is_group,
        parent_id: node.parent_id.clone(),
        padding: node.padding,
        shape: node.shape.clone(),
        dir: node.dir.clone(),
        metadata: metadata_of(node.metadata.as_ref()),
        label_bbox: node
            .label_bbox
            .as_ref()
            .map(|size| (size.width, size.height)),
        width: node.width,
        height: node.height,
        ..Node::default()
    });
    for child in &node.children {
        flatten(child, out);
    }
}

/// The metadata of a node, as the layout data holds it.
fn metadata_of(metadata: Option<&serde_json::Value>) -> BTreeMap<String, Value> {
    match metadata {
        Some(serde_json::Value::Object(map)) => map
            .iter()
            .map(|(key, value)| (key.clone(), Value::from(value.clone())))
            .collect(),
        _ => BTreeMap::new(),
    }
}

/// The diagram direction, read back from the ELK direction of the root. `TB` and `TD` are both
/// `DOWN`, which is all the graph construction reads.
fn direction_of(graph: &Graph) -> String {
    let elk = graph
        .layout_options
        .as_ref()
        .and_then(|options| options.get("elk.direction"))
        .and_then(serde_json::Value::as_str);
    match elk {
        Some("RIGHT") => "LR",
        Some("LEFT") => "RL",
        Some("UP") => "BT",
        _ => "TB",
    }
    .to_owned()
}

/// The configuration of a fixture: the defaults, with the `elk` settings the fixture sets.
fn config_of(fixture: &Fixture) -> MermaidConfig {
    let mut config = MermaidConfig::default();
    let settings = fixture
        .config
        .as_ref()
        .and_then(|config| config.get("elk"))
        .and_then(serde_json::Value::as_object);
    if let Some(settings) = settings {
        for key in settings.keys() {
            assert_eq!(
                key, "keepEntryNodeOnTop",
                "{}: the test reads only keepEntryNodeOnTop from the fixture's elk settings",
                fixture.name
            );
        }
        if let Some(value) = settings
            .get("keepEntryNodeOnTop")
            .and_then(serde_json::Value::as_bool)
        {
            config.elk.keep_entry_node_on_top = value;
        }
    }
    config
}

/// Whether two option values are the same: numbers by value, everything else exactly.
fn same_value(actual: &serde_json::Value, expected: &serde_json::Value) -> bool {
    match (actual, expected) {
        (serde_json::Value::Number(a), serde_json::Value::Number(b)) => a.as_f64() == b.as_f64(),
        _ => actual == expected,
    }
}

/// Compares two layout-option objects: the keys in order, and each value with its JSON type.
fn same_options(
    actual: Option<&LayoutOptions>,
    expected: Option<&LayoutOptions>,
    path: &str,
) -> Result<(), String> {
    match (actual, expected) {
        (None, None) => Ok(()),
        (Some(actual), Some(expected)) => {
            if actual.len() != expected.len() {
                return Err(format!(
                    "{path}: {} options, expected {}",
                    actual.len(),
                    expected.len()
                ));
            }
            for ((key, value), (expected_key, expected_value)) in actual.iter().zip(expected) {
                if key != expected_key {
                    return Err(format!(
                        "{path}: option {key} where {expected_key} was expected"
                    ));
                }
                if !same_value(value, expected_value) {
                    return Err(format!(
                        "{path}: {key} = {value}, expected {expected_value}"
                    ));
                }
            }
            Ok(())
        }
        (actual, expected) => Err(format!(
            "{path}: options {}, expected {}",
            if actual.is_some() {
                "present"
            } else {
                "absent"
            },
            if expected.is_some() {
                "present"
            } else {
                "absent"
            },
        )),
    }
}

/// Compares two label lists: text, size and options, in order.
fn same_labels(actual: &[JsonLabel], expected: &[JsonLabel], path: &str) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err(format!(
            "{path}: {} labels, expected {}",
            actual.len(),
            expected.len()
        ));
    }
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        let label = format!("{path}.labels[{index}]");
        if actual.text != expected.text {
            return Err(format!(
                "{label}: text {:?}, expected {:?}",
                actual.text, expected.text
            ));
        }
        if actual.width != expected.width || actual.height != expected.height {
            return Err(format!(
                "{label}: size {:?} x {:?}, expected {:?} x {:?}",
                actual.width, actual.height, expected.width, expected.height
            ));
        }
        same_options(
            actual.layout_options.as_ref(),
            expected.layout_options.as_ref(),
            &label,
        )?;
    }
    Ok(())
}

/// Compares a node of the graph with the captured node: its size, options and labels, and then its
/// members in order.
fn same_node(actual: &JsonNode, expected: &CapturedNode, path: &str) -> Result<(), String> {
    if actual.id != expected.id {
        return Err(format!(
            "{path}: node {} where {} was expected",
            actual.id, expected.id
        ));
    }
    let path = format!("{path}/{}", expected.id);
    if actual.width != expected.width || actual.height != expected.height {
        return Err(format!(
            "{path}: size {:?} x {:?}, expected {:?} x {:?}",
            actual.width, actual.height, expected.width, expected.height
        ));
    }
    same_options(
        actual.layout_options.as_ref(),
        expected.layout_options.as_ref(),
        &path,
    )?;
    same_labels(&actual.labels, &expected.labels, &path)?;
    if expected.is_group {
        if actual.children.len() != expected.children.len() {
            return Err(format!(
                "{path}: {} members, expected {}",
                actual.children.len(),
                expected.children.len()
            ));
        }
        for (actual, expected) in actual.children.iter().zip(&expected.children) {
            same_node(actual, expected, &path)?;
        }
    } else if !actual.children.is_empty() {
        return Err(format!(
            "{path}: a leaf with {} members",
            actual.children.len()
        ));
    }
    Ok(())
}

/// Compares the root edges with the captured ones: order, ends, labels, and the reversal handed to
/// ELK.
fn same_edges(
    actual: &[JsonEdge],
    expected: &[CapturedEdge],
    state: &ElkState,
) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err(format!(
            "{} edges, expected {}",
            actual.len(),
            expected.len()
        ));
    }
    for (actual, expected) in actual.iter().zip(expected) {
        let path = format!("edge {}", expected.id);
        if actual.id != expected.id {
            return Err(format!(
                "edge {} where {} was expected",
                actual.id, expected.id
            ));
        }
        if actual.sources != expected.sources || actual.targets != expected.targets {
            return Err(format!(
                "{path}: {:?} -> {:?}, expected {:?} -> {:?}",
                actual.sources, actual.targets, expected.sources, expected.targets
            ));
        }
        if state.reversed_edges.contains(&expected.id) != expected.layout_reversed {
            return Err(format!("{path}: reversed {}", !expected.layout_reversed));
        }
        same_labels(&actual.labels, &expected.labels, &path)?;
    }
    Ok(())
}

/// The features of a fixture's graph that the port must reproduce, in the order of
/// [`FEATURES`]: subgraphs, directions, cross-hierarchy edges, edge labels and self loops.
fn features_of(data: &LayoutData) -> [bool; 5] {
    let parent: HashMap<&str, Option<&str>> = data
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.parent_id.as_deref()))
        .collect();
    [
        data.nodes.iter().any(|node| node.is_group),
        data.direction != "TB" || data.nodes.iter().any(|node| node.dir.is_some()),
        data.edges
            .iter()
            .any(|edge| parent.get(edge.start.as_str()) != parent.get(edge.end.as_str())),
        data.edges.iter().any(|edge| !edge.label.is_empty()),
        data.edges.iter().any(|edge| edge.start == edge.end),
    ]
}

/// The names of the features in [`features_of`].
const FEATURES: [&str; 5] = [
    "subgraphs",
    "directions",
    "cross-hierarchy edges",
    "edge labels",
    "self loops",
];

/// Compares the whole graph and the state with the captured input.
fn same_graph(input: &ElkInput, expected: &Graph) -> Result<(), String> {
    let graph = &input.graph;
    same_options(
        graph.layout_options.as_ref(),
        expected.layout_options.as_ref(),
        "root",
    )?;
    if graph.children.len() != expected.children.len() {
        return Err(format!(
            "{} root members, expected {}",
            graph.children.len(),
            expected.children.len()
        ));
    }
    for (actual, expected) in graph.children.iter().zip(&expected.children) {
        same_node(actual, expected, "root")?;
    }
    same_edges(&graph.edges, &expected.edges, &input.state)?;
    let reversed = expected
        .edges
        .iter()
        .filter(|edge| edge.layout_reversed)
        .count();
    if input.state.reversed_edges.len() != reversed {
        return Err(format!(
            "{} reversed edges, expected {reversed}",
            input.state.reversed_edges.len()
        ));
    }
    if !input.state.warnings.is_empty() {
        return Err(format!("unexpected warnings: {:?}", input.state.warnings));
    }
    Ok(())
}

#[test]
fn the_reference_elk_inputs_are_rebuilt_exactly() {
    let corpus: Corpus = serde_json::from_str(CORPUS).expect("the captured corpus parses");

    let mut compared = 0;
    let mut excluded = Vec::new();
    let mut failures = Vec::new();
    let mut counts = [0usize; 5];

    for fixture in &corpus.fixtures {
        if let Some(reason) = &fixture.excluded {
            excluded.push(format!("{}: {reason}", fixture.name));
            continue;
        }
        let data = layout_data(&fixture.input);
        let config = config_of(fixture);
        let globals = Globals::of(&config);
        let tokens = ThemeTokens::from_value(&Value::empty_object());
        let input = build(
            &data,
            &Config {
                section: Section::of(&config),
                globals: &globals,
                theme: &tokens,
            },
        );
        match same_graph(&input, &fixture.input) {
            Ok(()) => {
                compared += 1;
                for (count, present) in counts.iter_mut().zip(features_of(&data)) {
                    *count += usize::from(present);
                }
            }
            Err(message) => failures.push(format!("{}: {message}", fixture.name)),
        }
    }

    eprintln!(
        "flowchart ELK inputs: {} fixtures, {compared} rebuilt exactly, {} not compared",
        corpus.fixtures.len(),
        excluded.len()
    );
    let coverage: Vec<String> = FEATURES
        .iter()
        .zip(counts)
        .map(|(name, count)| format!("{name} {count}"))
        .collect();
    eprintln!("features of the compared fixtures: {}", coverage.join(", "));
    for line in &excluded {
        eprintln!("  not compared: {line}");
    }
    let shown: Vec<&String> = failures.iter().take(20).collect();
    assert!(
        failures.is_empty(),
        "{} of {} fixtures differ from the reference:\n{}",
        failures.len(),
        compared + failures.len(),
        shown
            .iter()
            .map(|line| line.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        compared >= MINIMUM_FIXTURES,
        "only {compared} fixtures compared, at least {MINIMUM_FIXTURES} expected"
    );
    // Self loops are rare in the corpus (three fixtures); the others each cover MINIMUM_FIXTURES.
    for (name, count) in FEATURES.iter().zip(counts).take(4) {
        assert!(count >= MINIMUM_FIXTURES, "{name} in {count} fixtures");
    }
    assert!(counts[4] >= 1, "no compared fixture has a self loop");
}

#[test]
fn the_captured_corpus_names_its_exclusions() {
    let corpus: Corpus = serde_json::from_str(CORPUS).expect("the captured corpus parses");
    let excluded: Vec<&str> = corpus
        .fixtures
        .iter()
        .filter(|fixture| fixture.excluded.is_some())
        .map(|fixture| fixture.name.as_str())
        .collect();
    assert_eq!(
        excluded,
        [
            "nightly-check-elk-container-algorithms-elk-box-layout-with-edges",
            "nightly-check-elk-container-algorithms-elk-rectpacking-layout-with-edges",
            "nightly-check-elk-container-algorithms-elk-subgraph-with-box-algorithm",
        ]
    );
}
