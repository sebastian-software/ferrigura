//! `sort_js` (`common/d3/sort.rs`) against V8's `Array.prototype.sort`.
//!
//! `fixtures/d3_sort_v8.json` holds 150 arrays of keys, with NaN and the infinities among them, and
//! the order in which Node 24 (V8 13.6) sorts them with `a.y0 - b.y0`, and with the same comparator
//! and the index as the tie-break (`link.index` in `d3-sankey`'s `ascendingSourceBreadth`). With a
//! comparator that is NaN for some pairs, the order depends on the algorithm, so this checks the
//! TimSort port's comparisons, not only the result for consistent keys.

use ferrigura_render::common::d3::sort_js;
use serde_json::Value;

const FIXTURE: &str = include_str!("fixtures/d3_sort_v8.json");

fn key(text: &str) -> f64 {
    match text {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        other => other.parse().expect("a number"),
    }
}

fn ids(value: &Value) -> Vec<usize> {
    value
        .as_array()
        .expect("an array of ids")
        .iter()
        .map(|id| id.as_u64().expect("an id") as usize)
        .collect()
}

#[test]
fn the_sort_gives_the_order_v8_gives() {
    let cases: Vec<Value> = serde_json::from_str(FIXTURE).expect("the fixture is JSON");
    for (index, case) in cases.iter().enumerate() {
        let keys: Vec<f64> = case["keys"]
            .as_array()
            .expect("keys")
            .iter()
            .map(|text| key(text.as_str().expect("a key")))
            .collect();
        let plain: Vec<(usize, f64)> = keys.iter().copied().enumerate().collect();

        let mut sorted = plain.clone();
        sort_js(&mut sorted, |a, b| a.1 - b.1);
        let order: Vec<usize> = sorted.iter().map(|item| item.0).collect();
        assert_eq!(order, ids(&case["plain"]), "case {index}: a.y0 - b.y0");

        let mut sorted = plain;
        sort_js(&mut sorted, |a, b| {
            let difference = a.1 - b.1;
            if difference != 0.0 && !difference.is_nan() {
                difference
            } else {
                a.0 as f64 - b.0 as f64
            }
        });
        let order: Vec<usize> = sorted.iter().map(|item| item.0).collect();
        assert_eq!(
            order,
            ids(&case["fallback"]),
            "case {index}: with the index"
        );
    }
}
