//! The sankey layout against `d3-sankey` 0.12.3 (`sankey/layout.rs`).
//!
//! `fixtures/sankey_d3_layout.json` holds 80 random graphs, with the settings Mermaid passes
//! (`sankeyRenderer.ts` 121-129), and the layout the library gives them when run with Node 24. The
//! graphs have zero, negative, NaN and infinite values, zero heights, every alignment, and cycles
//! (which the library rejects with `circular link`). The fixture was made by running the library's
//! `dist` build from `node_modules/.pnpm/d3-sankey@0.12.3` on graphs of a fixed seed; each field is
//! compared to the bit, with NaN equal to NaN.

use ferrigura_render::sankey::layout::{Align, CircularLink, LinkSpec, Settings, sankey};
use serde_json::Value;

const FIXTURE: &str = include_str!("fixtures/sankey_d3_layout.json");

/// A number as the fixture writes it: a JSON number, or the sentinel strings for the values that
/// JSON has no number for.
fn number(value: &Value) -> f64 {
    match value {
        Value::Number(number) => number.as_f64().expect("a finite number"),
        Value::String(text) => match text.as_str() {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            other => other.parse().expect("a number"),
        },
        other => panic!("not a number: {other}"),
    }
}

fn same(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a == b
}

fn align(name: &str) -> Align {
    match name {
        "left" => Align::Left,
        "right" => Align::Right,
        "center" => Align::Center,
        "justify" => Align::Justify,
        other => panic!("an alignment of the fixture: {other}"),
    }
}

#[test]
fn every_graph_of_the_fixture_lays_out_as_d3_sankey_does() {
    let cases: Vec<Value> = serde_json::from_str(FIXTURE).expect("the fixture is JSON");
    let mut cycles = 0;
    for (index, case) in cases.iter().enumerate() {
        let settings_json = &case["settings"];
        let settings = Settings {
            width: settings_json["width"].as_f64().expect("a width"),
            height: settings_json["height"].as_f64().expect("a height"),
            node_width: settings_json["nodeWidth"].as_f64().expect("a node width"),
            node_padding: settings_json["nodePadding"].as_f64().expect("a padding"),
            align: align(settings_json["align"].as_str().expect("an alignment")),
        };
        let links: Vec<LinkSpec> = case["links"]
            .as_array()
            .expect("links")
            .iter()
            .map(|link| LinkSpec {
                source: link["s"].as_u64().expect("a source") as usize,
                target: link["t"].as_u64().expect("a target") as usize,
                value: number(&link["v"]),
            })
            .collect();
        let node_count = case["nodes"].as_array().map_or_else(
            || {
                links
                    .iter()
                    .map(|link| link.source.max(link.target) + 1)
                    .max()
                    .unwrap_or(0)
            },
            Vec::len,
        );
        let result = sankey(node_count, &links, settings);

        if case["error"] != Value::Bool(false) {
            cycles += 1;
            assert_eq!(
                result,
                Err(CircularLink),
                "case {index}: a cycle is rejected"
            );
            continue;
        }
        let layout = result.unwrap_or_else(|_| panic!("case {index}: d3 lays it out"));
        for (position, expected) in case["nodes"].as_array().expect("nodes").iter().enumerate() {
            let node = &layout.nodes[position];
            assert!(
                same(node.value, number(&expected["value"])),
                "case {index} node {position} value"
            );
            assert!(
                same(node.x0, number(&expected["x0"])),
                "case {index} node {position} x0"
            );
            assert!(
                same(node.x1, number(&expected["x1"])),
                "case {index} node {position} x1"
            );
            assert!(
                same(node.y0, number(&expected["y0"])),
                "case {index} node {position} y0"
            );
            assert!(
                same(node.y1, number(&expected["y1"])),
                "case {index} node {position} y1"
            );
            assert_eq!(
                node.depth as u64,
                expected["depth"].as_u64().expect("a depth")
            );
            assert_eq!(
                node.height as u64,
                expected["height"].as_u64().expect("a height")
            );
            assert_eq!(
                node.layer as u64,
                expected["layer"].as_u64().expect("a layer")
            );
        }
        for (position, expected) in case["linkOut"]
            .as_array()
            .expect("links")
            .iter()
            .enumerate()
        {
            let link = &layout.links[position];
            assert!(
                same(link.width, number(&expected["width"])),
                "case {index} link {position} width"
            );
            assert!(
                same(link.y0, number(&expected["y0"])),
                "case {index} link {position} y0"
            );
            assert!(
                same(link.y1, number(&expected["y1"])),
                "case {index} link {position} y1"
            );
        }
    }
    assert!(cycles > 0, "the fixture holds cycles");
}

#[test]
fn the_alignment_fixture_of_the_reference_places_c_at_200() {
    // `align-nodes-left` (`e2e/diagrams/sankey/nightly-check`): the library gives `c` the box
    // `x0 = 200`, `y0 = 13.288167426099502` and height `373.00000000000006`.
    let links: Vec<LinkSpec> = [
        (0, 1, 8.0),
        (1, 2, 8.0),
        (2, 3, 8.0),
        (3, 4, 8.0),
        (5, 2, 4.0),
        (2, 6, 4.0),
    ]
    .into_iter()
    .map(|(source, target, value)| LinkSpec {
        source,
        target,
        value,
    })
    .collect();
    let settings = Settings {
        width: 410.0,
        height: 400.0,
        node_width: 10.0,
        node_padding: 27.0,
        align: Align::Left,
    };
    let layout = sankey(7, &links, settings).expect("lays out");
    let c = &layout.nodes[2];
    assert_eq!(c.x0, 200.0);
    assert_eq!(c.y0, 13.288_167_426_099_502);
    assert_eq!(c.y1 - c.y0, 373.000_000_000_000_06);
}
