//! The tidy tree layout against the reference: the removed package `@mermaid-js/layout-tidy-tree`
//! 1.0.1, run in Chromium by `pnpm port:tidy-tree` (`tools/port/geometry/tidy-tree-capture.ts`).
//!
//! `tests/data/tidy-tree/fixtures.json` holds the six mindmap fixtures that set `layout: tidy-tree`:
//! the input is what the reference's renderer hands the layout (the measured boxes of the nodes), and
//! the output is the layout's result. `generated.json` holds ten hand-picked trees (the probes of
//! NOTES 3.6 and the reference's edge cases) and 240 seeded random trees, with the reference's
//! rejections as errors. A larger run (`TIDY_TREE_RANDOM`, thousands of trees) is checked by the
//! ignored test `larger_random_corpus_matches_the_reference`, which reads it from `target/`. Every case is laid out by the port and compared with the reference's result,
//! field by field, with `==` and no tolerance: the nodes in their order, and the edges with their
//! points, their midpoints and the sections and sizes of their ends.
//!
//! `lay_out_tidy_tree` is checked on the fixtures too: the centres and the points it writes into the
//! layout data are the reference's.

use std::fmt::Write as _;

use ferrigura_render::unified::data::{Edge, LayoutData, Node};
use ferrigura_render::unified::geom::Point;
use ferrigura_render::unified::tidy_tree::{self, LayoutEdge, LayoutNode, Section, TidyTreeLayout};
use serde::Deserialize;

/// The six fixtures and the generated cases, as the reference's capture wrote them.
const FIXTURES: &str = include_str!("data/tidy-tree/fixtures.json");
const GENERATED: &str = include_str!("data/tidy-tree/generated.json");

#[derive(Deserialize)]
struct Corpus {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    input: Input,
    #[serde(default)]
    output: Option<Output>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct Input {
    nodes: Vec<InputNode>,
    edges: Vec<InputEdge>,
}

#[derive(Deserialize)]
struct InputNode {
    id: String,
    width: f64,
    height: f64,
    shape: String,
}

#[derive(Deserialize)]
struct InputEdge {
    id: String,
    start: String,
    end: String,
}

#[derive(Deserialize)]
struct Output {
    nodes: Vec<OutputNode>,
    edges: Vec<OutputEdge>,
}

#[derive(Deserialize)]
struct OutputNode {
    id: String,
    x: f64,
    y: f64,
    section: String,
    width: f64,
    height: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutputEdge {
    id: String,
    source: String,
    target: String,
    start_x: f64,
    start_y: f64,
    mid_x: f64,
    mid_y: f64,
    end_x: f64,
    end_y: f64,
    points: Vec<JsonPoint>,
    source_section: Option<String>,
    target_section: Option<String>,
    source_width: Option<f64>,
    source_height: Option<f64>,
    target_width: Option<f64>,
    target_height: Option<f64>,
}

#[derive(Deserialize)]
struct JsonPoint {
    x: f64,
    y: f64,
}

fn corpus(text: &str) -> Corpus {
    serde_json::from_str(text).expect("the captured corpus is valid JSON of the expected shape")
}

fn section(name: &str) -> Section {
    match name {
        "root" => Section::Root,
        "left" => Section::Left,
        "right" => Section::Right,
        other => panic!("unknown section {other:?} in the capture"),
    }
}

fn layout_input(input: &Input) -> (Vec<LayoutNode>, Vec<LayoutEdge>) {
    let nodes = input
        .nodes
        .iter()
        .map(|node| LayoutNode {
            id: node.id.clone(),
            width: node.width,
            height: node.height,
            shape: node.shape.clone(),
        })
        .collect();
    let edges = input
        .edges
        .iter()
        .map(|edge| LayoutEdge {
            id: edge.id.clone(),
            start: edge.start.clone(),
            end: edge.end.clone(),
        })
        .collect();
    (nodes, edges)
}

/// The first field of the layout that differs from the reference's, or `None` when all are equal.
fn first_difference(expected: &Output, actual: &TidyTreeLayout) -> Option<String> {
    if expected.nodes.len() != actual.nodes.len() {
        return Some(format!(
            "{} nodes, the reference has {}",
            actual.nodes.len(),
            expected.nodes.len()
        ));
    }
    for (index, (want, got)) in expected.nodes.iter().zip(&actual.nodes).enumerate() {
        let same = want.id == got.id
            && want.x == got.x
            && want.y == got.y
            && section(&want.section) == got.section
            && want.width == got.width
            && want.height == got.height;
        if !same {
            return Some(format!(
                "node {index} ({}): the reference has ({}, {}) {} {}x{}, the port ({}, {}) {:?} {}x{} as {}",
                want.id,
                want.x,
                want.y,
                want.section,
                want.width,
                want.height,
                got.x,
                got.y,
                got.section,
                got.width,
                got.height,
                got.id,
            ));
        }
    }
    if expected.edges.len() != actual.edges.len() {
        return Some(format!(
            "{} edges, the reference has {}",
            actual.edges.len(),
            expected.edges.len()
        ));
    }
    for (index, (want, got)) in expected.edges.iter().zip(&actual.edges).enumerate() {
        let points: Vec<Point> = want.points.iter().map(|p| Point::new(p.x, p.y)).collect();
        let same = want.id == got.id
            && want.source == got.source
            && want.target == got.target
            && Point::new(want.start_x, want.start_y) == got.start
            && Point::new(want.mid_x, want.mid_y) == got.mid
            && Point::new(want.end_x, want.end_y) == got.end
            && points == got.points
            && want.source_section.as_deref().map(section) == got.source_section
            && want.target_section.as_deref().map(section) == got.target_section
            && want.source_width == got.source_width
            && want.source_height == got.source_height
            && want.target_width == got.target_width
            && want.target_height == got.target_height;
        if !same {
            let mut text = String::new();
            let _ = write!(
                text,
                "edge {index} ({}): the reference's points {:?}, the port's {:?}; mid ({}, {}) against ({}, {})",
                want.id, points, got.points, want.mid_x, want.mid_y, got.mid.x, got.mid.y,
            );
            return Some(text);
        }
    }
    None
}

/// Lays one case out and compares it with the reference: the result, or the rejection of both.
fn check(case: &Case) -> Option<String> {
    let (nodes, edges) = layout_input(&case.input);
    let result = tidy_tree::layout(&nodes, &edges);
    match (&case.output, &case.error, result) {
        (Some(expected), None, Ok(actual)) => first_difference(expected, &actual),
        (None, Some(_), Err(_)) => None,
        (None, Some(error), Ok(_)) => Some(format!(
            "the reference rejects with {error:?}, the port lays it out"
        )),
        (Some(_), None, Err(error)) => Some(format!(
            "the port rejects with {error}, the reference lays it out"
        )),
        _ => Some("the capture has neither a result nor an error".to_string()),
    }
}

/// Every case of a corpus agrees with the reference; the failures are listed with their first
/// difference.
fn assert_corpus(name: &str, corpus: &Corpus) {
    let mut failures = Vec::new();
    for case in &corpus.cases {
        if let Some(difference) = check(case) {
            failures.push(format!("{}: {difference}", case.name));
        }
    }
    assert!(
        failures.is_empty(),
        "{name}: {} of {} cases differ from the reference:\n{}",
        failures.len(),
        corpus.cases.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn fixtures_match_the_reference_exactly() {
    let fixtures = corpus(FIXTURES);
    assert_eq!(
        fixtures.cases.len(),
        6,
        "the six tidy-tree fixtures of the reference"
    );
    assert_corpus("fixtures", &fixtures);
}

#[test]
fn generated_trees_match_the_reference_exactly() {
    let generated = corpus(GENERATED);
    let rejected = generated
        .cases
        .iter()
        .filter(|case| case.error.is_some())
        .count();
    assert!(
        generated.cases.len() >= 250 && rejected == 2,
        "the hand-picked and random trees: {} cases, {rejected} rejected",
        generated.cases.len()
    );
    assert_corpus("generated", &generated);
}

/// The larger run of `pnpm port:tidy-tree` with `TIDY_TREE_RANDOM=<n> TIDY_TREE_OUT=<dir>`, read from
/// `FERRIGURA_TIDY_TREE_CORPUS` (default `target/tidy-tree`). It is ignored by default, because the
/// run is not committed: run `--ignored`.
#[test]
#[ignore = "needs the larger run of pnpm port:tidy-tree; run with --ignored --nocapture"]
fn larger_random_corpus_matches_the_reference() {
    let directory = std::env::var("FERRIGURA_TIDY_TREE_CORPUS").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/tidy-tree").to_string()
    });
    let text = std::fs::read_to_string(format!("{directory}/generated.json"))
        .unwrap_or_else(|error| panic!("{directory}/generated.json: {error}"));
    let larger = corpus(&text);
    println!("{} cases in {directory}", larger.cases.len());
    assert_corpus("larger corpus", &larger);
}

/// `lay_out_tidy_tree` writes the reference's centres and points into the layout data.
#[test]
fn lay_out_tidy_tree_writes_the_reference_positions() {
    for case in &corpus(FIXTURES).cases {
        let expected = case.output.as_ref().expect("a fixture has a result");
        let mut data = LayoutData {
            nodes: case
                .input
                .nodes
                .iter()
                .map(|node| Node {
                    id: node.id.clone(),
                    width: Some(node.width),
                    height: Some(node.height),
                    shape: node.shape.clone(),
                    ..Default::default()
                })
                .collect(),
            edges: case
                .input
                .edges
                .iter()
                .map(|edge| Edge {
                    id: edge.id.clone(),
                    start: edge.start.clone(),
                    end: edge.end.clone(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        data.lay_out_tidy_tree()
            .unwrap_or_else(|error| panic!("{}: {error}", case.name));
        for want in &expected.nodes {
            let got = data
                .nodes
                .iter()
                .find(|node| node.id == want.id)
                .expect("every node of the result is in the data");
            assert_eq!(
                (got.x, got.y),
                (Some(want.x), Some(want.y)),
                "{}: the centre of node {}",
                case.name,
                want.id
            );
        }
        for want in &expected.edges {
            let got = data
                .edges
                .iter()
                .find(|edge| edge.id == want.id)
                .expect("every edge of the result is in the data");
            let points: Vec<Point> = want.points.iter().map(|p| Point::new(p.x, p.y)).collect();
            assert_eq!(
                got.points, points,
                "{}: the points of edge {}",
                case.name, want.id
            );
        }
    }
}
