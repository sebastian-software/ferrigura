//! Differential test of the flowchart's ELK post-processing against the reference.
//!
//! Each file of `tests/data/flowchart-elk` is one flowchart fixture of the reference, written by
//! `tools/port/geometry/flowchart-elk-fixtures.ts`: the elkjs output, the sizes of the input graph,
//! and the reference's node centres, subgraph frames, edge points and edge label centres. The test
//! runs `apply_with_crossing` on the elkjs output and compares the result with the reference, within
//! 0.05 px.
//!
//! The leaves' outlines are the reference's `intersect-rect`, transcribed in [`rect_crossing`], for
//! the rectangular shapes (`squareRect`, `roundedRect`). Only the edges whose two ends are
//! rectangles or subgraphs are compared; the other leaves have no outline here. Polygon and circle
//! ends (diamonds, circles, hexagons, ...) join the comparison when the shapes' intersect functions
//! land (task T3): `apply` then passes each leaf's `Intersect`, and this test can pass the same
//! function for every leaf and compare every edge.
//!
//! Node centres, subgraph frames and label centres do not depend on the outlines, so they are
//! compared for every fixture. A subgraph without children is compared by its centre only: the
//! reference's frame painter widens it to its title plus padding (`clusters.js:63`), which is paint.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ferrigura_elk::json::JsonNode;
use ferrigura_render::flowchart::data::{Edge, LayoutData, Node};
use ferrigura_render::flowchart::elk_apply::apply_with_crossing;
use ferrigura_render::flowchart::elk_build::{ElkState, LabelData};
use ferrigura_render::flowchart::geom::{CenterBox, Point};
use serde_json::Value;

/// The largest difference that counts as equal, in SVG user units.
const TOLERANCE: f64 = 0.05;

/// The fewest edges the comparison must cover. Guards against a pass that compares nothing.
const MIN_COMPARED_EDGES: usize = 150;

/// The leaf shapes whose outline is the rectangle.
const RECTANGLE_SHAPES: [&str; 2] = ["squareRect", "roundedRect"];

/// `intersect-rect.js` of the reference, transcribed: where the ray from the centre of a rectangle
/// through `point` leaves it.
fn rect_crossing(node: CenterBox, point: Point) -> Point {
    let dx = point.x - node.x;
    let dy = point.y - node.y;
    let mut w = node.width / 2.0;
    let mut h = node.height / 2.0;
    let (sx, sy);
    if dy.abs() * w > dx.abs() * h {
        // The intersection is on the top or bottom of the box.
        if dy < 0.0 {
            h = -h;
        }
        sx = if dy == 0.0 { 0.0 } else { (h * dx) / dy };
        sy = h;
    } else {
        // The intersection is on the left or right of the box.
        if dx < 0.0 {
            w = -w;
        }
        sx = w;
        sy = if dx == 0.0 { 0.0 } else { (w * dy) / dx };
    }
    Point::new(node.x + sx, node.y + sy)
}

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(0.0)
}

fn point_of(value: &Value) -> Point {
    Point::new(number(&value["x"]), number(&value["y"]))
}

/// The layout data, the side tables and the rectangular leaves of one fixture.
struct Scene {
    data: LayoutData,
    state: ElkState,
    rectangles: HashSet<String>,
}

fn scene(fixture: &Value) -> Scene {
    let mut nodes = Vec::new();
    let mut parent_by_id = HashMap::new();
    let mut label_data = HashMap::new();
    let mut rectangles = HashSet::new();
    let mut has_children = HashSet::new();
    for node in fixture["sizes"]["nodes"].as_array().into_iter().flatten() {
        let id = node["id"].as_str().unwrap_or_default().to_string();
        let parent = node["parent"].as_str().map(str::to_string);
        let is_group = node["isGroup"].as_bool().unwrap_or(false);
        let shape = node["shape"].as_str().unwrap_or_default().to_string();
        if let Some(parent) = &parent {
            parent_by_id.insert(id.clone(), parent.clone());
            has_children.insert(parent.clone());
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
        } else if RECTANGLE_SHAPES.contains(&shape.as_str()) {
            rectangles.insert(id.clone());
        }
        nodes.push(Node {
            id,
            parent_id: parent,
            is_group,
            shape,
            padding: number(&node["padding"]),
            width: node["width"].as_f64(),
            height: node["height"].as_f64(),
            ..Node::default()
        });
    }

    let mut edges = Vec::new();
    let mut reversed_edges = HashSet::new();
    for edge in fixture["sizes"]["edges"].as_array().into_iter().flatten() {
        let id = edge["id"].as_str().unwrap_or_default().to_string();
        if edge["reversed"].as_bool().unwrap_or(false) {
            reversed_edges.insert(id.clone());
        }
        edges.push(Edge {
            id,
            start: edge["start"].as_str().unwrap_or_default().to_string(),
            end: edge["end"].as_str().unwrap_or_default().to_string(),
            arrow_type_start: edge["arrowTypeStart"]
                .as_str()
                .unwrap_or("none")
                .to_string(),
            arrow_type_end: edge["arrowTypeEnd"].as_str().unwrap_or("none").to_string(),
            label: edge["label"].as_str().unwrap_or_default().to_string(),
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
        rectangles,
    }
}

/// Counts of what the test compared, over all fixtures.
#[derive(Default)]
struct Totals {
    nodes: usize,
    clusters: usize,
    edges: usize,
    labels: usize,
    skipped_edges: usize,
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
fn check(name: &str, fixture: &Value, totals: &mut Totals, failures: &mut Vec<String>) {
    let mut scene = scene(fixture);
    let elk: JsonNode = serde_json::from_value(fixture["elk"].clone()).expect("the elkjs output");
    let rectangles = scene.rectangles.clone();
    let crossing = move |id: &str, node: CenterBox, probe: Point| {
        rectangles.contains(id).then(|| rect_crossing(node, probe))
    };
    apply_with_crossing(&mut scene.data, &elk, &scene.state, &crossing, true);

    let index: HashMap<&str, usize> = scene
        .data
        .nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.id.as_str(), position))
        .collect();
    let groups: HashSet<&str> = scene
        .data
        .nodes
        .iter()
        .filter(|node| node.is_group)
        .map(|node| node.id.as_str())
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
        let rect_ended = [&edge.start, &edge.end]
            .iter()
            .all(|id| groups.contains(id.as_str()) || scene.rectangles.contains(id.as_str()));
        if !rect_ended {
            totals.skipped_edges += 1;
            continue;
        }
        if let Some(reference) = fixture["reference"]["edges"][&edge.id].as_array() {
            totals.edges += 1;
            if reference.len() != edge.points.len() {
                failures.push(format!(
                    "{name}: edge {} has {} points, the reference {}",
                    edge.id,
                    edge.points.len(),
                    reference.len()
                ));
            } else {
                for (position, (ours, theirs)) in edge.points.iter().zip(reference).enumerate() {
                    let theirs = point_of(theirs);
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
        check(&name, &fixture, &mut totals, &mut failures);
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
        "only {} rectangular edges compared",
        totals.edges
    );
    eprintln!(
        "{} fixtures: {} nodes, {} subgraphs, {} edges ({} edge labels) compared; {} edges with other shapes skipped",
        paths.len(),
        totals.nodes,
        totals.clusters,
        totals.edges,
        totals.labels,
        totals.skipped_edges
    );
}
