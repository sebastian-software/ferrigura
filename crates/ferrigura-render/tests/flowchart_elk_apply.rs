//! Differential test of the flowchart's ELK post-processing against the reference.
//!
//! Each file of `tests/data/flowchart-elk` is one flowchart fixture of the reference, written by
//! `tools/port/geometry/flowchart-elk-fixtures.ts`: the elkjs output, the sizes of the input graph
//! with each leaf's drawing inputs, and the reference's node centres, subgraph frames, edge points
//! and edge label centres. For every leaf the test draws the shape with the port's own shape
//! handler (`shapes::node_label` and `shapes::draw`), which gives the leaf's `Intersect` as the
//! pipeline does, and runs `apply` on the elkjs output with those intersects. The result is compared
//! with the reference within 0.05 px: the node centres, the subgraph frames, every edge's points
//! whatever shapes its ends are, and the edge label centres.
//!
//! The leaf's drawn box, as the measure step stores it (a float32 value, `BBox::browser`), must be
//! the size the reference's ELK input gives it, since the intersect reads the box at call time. The
//! ports come from the reference's elkjs output, and whether a port lies on a box edge is decided
//! on these sizes; the float32 rounding is what makes those decisions coincide with the reference's.
//!
//! Two polylines are compared after their near-coincident points are merged (see [`merged`]).
//!
//! A subgraph without children is compared by its centre only: the reference's frame painter widens
//! it to its title plus padding (`clusters.js:63`), which is paint.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ferrigura_elk::json::JsonNode;
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::theme::{Theme, ThemeTokens};
use ferrigura_model::value::Value as ModelValue;
use ferrigura_render::context::Globals;
use ferrigura_render::unified::data::{Edge, LayoutData, Node};
use ferrigura_render::unified::elk_apply::apply;
use ferrigura_render::unified::elk_build::{ElkState, LabelData};
use ferrigura_render::unified::geom::Point;
use ferrigura_render::unified::label::Anchor;
use ferrigura_render::unified::shapes::{self, Intersect, ShapeInput};
use ferrigura_render::unified::{Config, Section};
use serde_json::Value;

/// The largest difference that counts as equal, in SVG user units.
const TOLERANCE: f64 = 0.05;

/// The fewest edges the comparison must cover: every edge of the 104 fixtures, so that a fixture
/// whose edges stop being compared fails the test.
const MIN_COMPARED_EDGES: usize = 805;

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(0.0)
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

fn point_of(value: &Value) -> Point {
    Point::new(number(&value["x"]), number(&value["y"]))
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn label_type(name: &str) -> LabelType {
    match name {
        "string" => LabelType::String,
        "markdown" => LabelType::Markdown,
        _ => LabelType::Text,
    }
}

/// A leaf as the flowchart's measure step builds it, from the fixture's drawing inputs. Its width
/// is left unset, as it is when the measure step draws the label (`node.width` is set afterwards).
fn leaf_node(node: &Value) -> Node {
    let id = text(&node["id"]);
    let drawing = &node["drawing"];
    Node {
        dom_id: format!("flowchart-{id}-0"),
        id,
        label: text(&drawing["label"]),
        label_type: label_type(drawing["labelType"].as_str().unwrap_or("text")),
        label_style: text(&drawing["labelStyle"]),
        parent_id: node["parent"].as_str().map(str::to_string),
        padding: number(&node["padding"]),
        css_styles: strings(&drawing["cssStyles"]),
        css_compiled_styles: strings(&drawing["cssCompiledStyles"]),
        css_classes: text(&drawing["cssClasses"]),
        look: drawing["look"].as_str().unwrap_or("neo").to_string(),
        icon: drawing["icon"].as_str().map(str::to_string),
        is_group: false,
        shape: text(&node["shape"]),
        min_width: drawing["minWidth"].as_f64(),
        ..Node::default()
    }
}

/// The layout data, the side tables and the outline of every leaf of one fixture.
struct Scene {
    data: LayoutData,
    state: ElkState,
    intersects: HashMap<String, Intersect>,
}

/// Builds the scene of a fixture. Each leaf is drawn with the port's shape handler, and the drawn
/// box is compared with the reference's size for it.
fn build_scene(
    name: &str,
    fixture: &Value,
    config: &MermaidConfig,
    theme: &ModelValue,
    failures: &mut Vec<String>,
) -> Scene {
    let globals = Globals::of(config);
    let tokens = ThemeTokens::from_value(theme);
    let unified = Config {
        section: Section::of(config),
        globals: &globals,
        theme: &tokens,
    };
    let mut nodes = Vec::new();
    let mut parent_by_id = HashMap::new();
    let mut label_data = HashMap::new();
    let mut intersects = HashMap::new();
    for node in fixture["sizes"]["nodes"].as_array().into_iter().flatten() {
        let id = text(&node["id"]);
        let parent = node["parent"].as_str().map(str::to_string);
        let is_group = node["isGroup"].as_bool().unwrap_or(false);
        if let Some(parent) = &parent {
            parent_by_id.insert(id.clone(), parent.clone());
        }
        if is_group {
            label_data.insert(
                id.clone(),
                LabelData {
                    width: number(&node["labelWidth"]),
                    height: number(&node["labelHeight"]),
                    wrapping_width: 0.0,
                },
            );
        } else {
            let drawn_node = leaf_node(node);
            let label = shapes::node_label(&drawn_node, &unified, None, Anchor::Middle)
                .unwrap_or_else(|error| panic!("{name}: the label of {id}: {error}"));
            let dir = node["drawing"]["dir"].as_str();
            let drawn = shapes::draw(&ShapeInput {
                node: &drawn_node,
                label: label.as_ref(),
                config: &unified,
                dir,
            })
            .unwrap_or_else(|error| panic!("{name}: the shape of {id}: {error}"));
            // The layout takes the drawn box as the measure step stores it, a float32 value.
            let size = drawn.bbox.browser();
            compare_value(
                failures,
                name,
                &format!("leaf {id} drawn width"),
                size.width,
                number(&node["width"]),
            );
            compare_value(
                failures,
                name,
                &format!("leaf {id} drawn height"),
                size.height,
                number(&node["height"]),
            );
            intersects.insert(id.clone(), drawn.intersect);
        }
        nodes.push(Node {
            id,
            parent_id: parent,
            is_group,
            shape: text(&node["shape"]),
            padding: number(&node["padding"]),
            width: node["width"].as_f64(),
            height: node["height"].as_f64(),
            ..Node::default()
        });
    }

    let mut edges = Vec::new();
    let mut reversed_edges = HashSet::new();
    for edge in fixture["sizes"]["edges"].as_array().into_iter().flatten() {
        let id = text(&edge["id"]);
        if edge["reversed"].as_bool().unwrap_or(false) {
            reversed_edges.insert(id.clone());
        }
        edges.push(Edge {
            id,
            start: text(&edge["start"]),
            end: text(&edge["end"]),
            arrow_type_start: edge["arrowTypeStart"]
                .as_str()
                .unwrap_or("none")
                .to_string(),
            arrow_type_end: edge["arrowTypeEnd"].as_str().unwrap_or("none").to_string(),
            label: text(&edge["label"]),
            width: edge["width"].as_f64(),
            height: edge["height"].as_f64(),
            ..Edge::default()
        });
    }

    Scene {
        data: LayoutData {
            nodes,
            edges,
            ..LayoutData::default()
        },
        state: ElkState {
            parent_by_id,
            label_data,
            reversed_edges,
            ..ElkState::default()
        },
        intersects,
    }
}

/// Counts of what the test compared, over all fixtures.
#[derive(Default)]
struct Totals {
    nodes: usize,
    leaves: usize,
    clusters: usize,
    edges: usize,
    labels: usize,
}

/// The points of a polyline with each point that lies within the tolerance of the point kept before
/// it removed. At the tolerance such a point is the one before it. Whether a port lies on the edge
/// of its box is decided on differences of a few thousandths of a pixel (`outsideNode`,
/// `outlineAttachPoint`), so a decision can add or drop such a point on one side only.
fn merged(points: &[Point]) -> Vec<Point> {
    let mut kept: Vec<Point> = Vec::with_capacity(points.len());
    for &point in points {
        if kept
            .last()
            .is_none_or(|last| (point.x - last.x).hypot(point.y - last.y) >= TOLERANCE)
        {
            kept.push(point);
        }
    }
    kept
}

/// Records a value that differs from the reference's by more than the tolerance.
fn compare_value(failures: &mut Vec<String>, name: &str, what: &str, ours: f64, theirs: f64) {
    // A missing value (NaN) is a difference too.
    let difference = (ours - theirs).abs();
    if difference.is_nan() || difference > TOLERANCE {
        failures.push(format!(
            "{name}: {what}: ours {ours:.4}, reference {theirs:.4}"
        ));
    }
}

/// Runs `apply` on one fixture and compares the result with the reference, recording every value
/// that differs by more than the tolerance.
fn check(
    name: &str,
    fixture: &Value,
    config: &MermaidConfig,
    theme: &ModelValue,
    totals: &mut Totals,
    failures: &mut Vec<String>,
) {
    let mut scene = build_scene(name, fixture, config, theme, failures);
    let elk: JsonNode = serde_json::from_value(fixture["elk"].clone()).expect("the elkjs output");
    apply(
        &mut scene.data,
        &elk,
        &scene.state,
        &scene.intersects,
        config.elk.straighten_edges,
    );

    let index: HashMap<&str, usize> = scene
        .data
        .nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.id.as_str(), position))
        .collect();
    let with_children: HashSet<&str> = scene
        .state
        .parent_by_id
        .values()
        .map(String::as_str)
        .collect();

    for (id, centre) in fixture["reference"]["nodes"]
        .as_object()
        .into_iter()
        .flatten()
    {
        let Some(&position) = index.get(id.as_str()) else {
            failures.push(format!("{name}: node {id} is missing"));
            continue;
        };
        let node = &scene.data.nodes[position];
        let theirs = point_of(centre);
        compare_value(
            failures,
            name,
            &format!("node {id} x"),
            node.x.unwrap_or(f64::NAN),
            theirs.x,
        );
        compare_value(
            failures,
            name,
            &format!("node {id} y"),
            node.y.unwrap_or(f64::NAN),
            theirs.y,
        );
        totals.nodes += 1;
        if !scene.intersects.contains_key(id.as_str()) {
            continue;
        }
        totals.leaves += 1;
    }

    for (id, rect) in fixture["reference"]["clusters"]
        .as_object()
        .into_iter()
        .flatten()
    {
        let Some(&position) = index.get(id.as_str()) else {
            failures.push(format!("{name}: subgraph {id} is missing"));
            continue;
        };
        let node = &scene.data.nodes[position];
        compare_value(
            failures,
            name,
            &format!("subgraph {id} x"),
            node.x.unwrap_or(f64::NAN),
            number(&rect["x"]) + number(&rect["width"]) / 2.0,
        );
        compare_value(
            failures,
            name,
            &format!("subgraph {id} y"),
            node.y.unwrap_or(f64::NAN),
            number(&rect["y"]) + number(&rect["height"]) / 2.0,
        );
        if with_children.contains(id.as_str()) {
            compare_value(
                failures,
                name,
                &format!("subgraph {id} width"),
                node.width.unwrap_or(f64::NAN),
                number(&rect["width"]),
            );
            compare_value(
                failures,
                name,
                &format!("subgraph {id} height"),
                node.height.unwrap_or(f64::NAN),
                number(&rect["height"]),
            );
        }
        totals.clusters += 1;
    }

    for edge in &scene.data.edges {
        let Some(reference) = fixture["reference"]["edges"][&edge.id].as_array() else {
            failures.push(format!("{name}: edge {} has no reference", edge.id));
            continue;
        };
        totals.edges += 1;
        let ours = merged(&edge.points);
        let theirs = merged(&reference.iter().map(point_of).collect::<Vec<_>>());
        if ours.len() != theirs.len() {
            failures.push(format!(
                "{name}: edge {} has {} points, the reference {}: ours {:?}, reference {:?}",
                edge.id,
                ours.len(),
                theirs.len(),
                ours,
                theirs
            ));
        } else {
            for (position, (ours, theirs)) in ours.iter().zip(&theirs).enumerate() {
                compare_value(
                    failures,
                    name,
                    &format!("edge {} point {position} x", edge.id),
                    ours.x,
                    theirs.x,
                );
                compare_value(
                    failures,
                    name,
                    &format!("edge {} point {position} y", edge.id),
                    ours.y,
                    theirs.y,
                );
            }
        }
        if let Some(label) = fixture["reference"]["labels"].get(&edge.id) {
            let theirs = point_of(label);
            compare_value(
                failures,
                name,
                &format!("edge {} label x", edge.id),
                edge.x.unwrap_or(f64::NAN),
                theirs.x,
            );
            compare_value(
                failures,
                name,
                &format!("edge {} label y", edge.id),
                edge.y.unwrap_or(f64::NAN),
                theirs.y,
            );
            totals.labels += 1;
        }
    }
}

#[test]
fn apply_matches_the_reference_on_the_captured_flowcharts() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/flowchart-elk");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("the fixtures of the differential test")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no fixtures in {}", directory.display());

    // The configuration and the theme of the fixtures: the flowchart defaults, with the redux-color
    // theme, as the harness forces for both sides.
    let config = MermaidConfig::default();
    let theme = Theme::ReduxColor.variables(None);

    let mut totals = Totals::default();
    let mut failures = Vec::new();
    for path in &paths {
        let name = path
            .file_stem()
            .expect("a file name")
            .to_string_lossy()
            .to_string();
        let fixture: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("a readable fixture"))
                .expect("a fixture as JSON");
        check(&name, &fixture, &config, &theme, &mut totals, &mut failures);
    }

    assert!(
        failures.is_empty(),
        "{} values differ from the reference, the first are:\n{}",
        failures.len(),
        failures
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        totals.edges >= MIN_COMPARED_EDGES,
        "only {} edges compared",
        totals.edges
    );
    eprintln!(
        "{} fixtures: {} nodes ({} leaves drawn), {} subgraphs, {} edges ({} edge labels) compared, every end",
        paths.len(),
        totals.nodes,
        totals.leaves,
        totals.clusters,
        totals.edges,
        totals.labels
    );
}
