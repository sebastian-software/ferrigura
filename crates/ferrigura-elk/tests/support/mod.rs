//! Helpers shared by the differential tests: the corpus test and the random graph test compare
//! the port's node boxes, edge routes and label boxes with elkjs's, and measure the difference.

use ferrigura_elk::json::JsonNode;
use serde_json::Value;

/// Collects `(id, x, y, width, height)` of every node below `value`, in document order.
pub fn node_boxes(value: &Value, out: &mut Vec<(String, f64, f64, f64, f64)>) {
    for child in value["children"].as_array().into_iter().flatten() {
        let field = |key: &str| child[key].as_f64().unwrap_or(0.0);
        out.push((
            child["id"].as_str().unwrap_or_default().to_string(),
            field("x"),
            field("y"),
            field("width"),
            field("height"),
        ));
        node_boxes(child, out);
    }
}

/// Every edge below `value`, level by level: the edges a level holds, then the levels of its
/// children, in document order. Both the input and the output keep each edge at its level.
fn all_edges(value: &Value) -> Vec<&Value> {
    let mut edges: Vec<&Value> = value["edges"].as_array().into_iter().flatten().collect();
    for child in value["children"].as_array().into_iter().flatten() {
        edges.extend(all_edges(child));
    }
    edges
}

/// Each edge's route as a flat list of numbers: the number of sections, then per section its
/// start, the number of its bend points and the bend points, and its end.
fn edge_routes(value: &Value) -> Vec<Vec<f64>> {
    let point = |value: &Value| {
        [
            value["x"].as_f64().unwrap_or(0.0),
            value["y"].as_f64().unwrap_or(0.0),
        ]
    };
    all_edges(value)
        .into_iter()
        .map(|edge| {
            let sections = edge["sections"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default();
            let mut route = vec![sections.len() as f64];
            for section in sections {
                route.extend(point(&section["startPoint"]));
                let bends = section["bendPoints"]
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                route.push(bends.len() as f64);
                for bend in bends {
                    route.extend(point(bend));
                }
                route.extend(point(&section["endPoint"]));
            }
            route
        })
        .collect()
}

/// Each edge label's box as `[x, y, width, height]`, in the order of the edges and their labels.
fn label_boxes(value: &Value) -> Vec<[f64; 4]> {
    all_edges(value)
        .into_iter()
        .flat_map(|edge| edge["labels"].as_array().into_iter().flatten())
        .map(|label| {
            [
                label["x"].as_f64().unwrap_or(0.0),
                label["y"].as_f64().unwrap_or(0.0),
                label["width"].as_f64().unwrap_or(0.0),
                label["height"].as_f64().unwrap_or(0.0),
            ]
        })
        .collect()
}

/// The largest difference between the port's node boxes and elkjs's, and between the root sizes.
/// Errors when the nodes differ in number or order: that is not a difference in coordinates.
pub fn largest_difference(ours: &JsonNode, theirs: &Value) -> Result<f64, String> {
    let ours_value = serde_json::to_value(ours).unwrap();
    let (mut ours_boxes, mut theirs_boxes) = (Vec::new(), Vec::new());
    node_boxes(&ours_value, &mut ours_boxes);
    node_boxes(theirs, &mut theirs_boxes);
    if ours_boxes.len() != theirs_boxes.len() {
        return Err(format!(
            "{} nodes, elkjs has {}",
            ours_boxes.len(),
            theirs_boxes.len()
        ));
    }
    let mut worst = 0.0_f64;
    for (a, b) in ours_boxes.iter().zip(&theirs_boxes) {
        if a.0 != b.0 {
            return Err(format!("node {} where elkjs has {}", a.0, b.0));
        }
        worst = [a.1 - b.1, a.2 - b.2, a.3 - b.3, a.4 - b.4]
            .into_iter()
            .map(f64::abs)
            .fold(worst, f64::max);
    }
    let (ours_routes, theirs_routes) = (edge_routes(&ours_value), edge_routes(theirs));
    if ours_routes.len() != theirs_routes.len() {
        return Err(format!(
            "{} edges, elkjs has {}",
            ours_routes.len(),
            theirs_routes.len()
        ));
    }
    for (ours_route, theirs_route) in ours_routes.iter().zip(&theirs_routes) {
        if ours_route.len() != theirs_route.len() {
            return Err("an edge route with another shape".to_string());
        }
        for (a, b) in ours_route.iter().zip(theirs_route) {
            worst = worst.max((a - b).abs());
        }
    }
    let (ours_labels, theirs_labels) = (label_boxes(&ours_value), label_boxes(theirs));
    if ours_labels.len() != theirs_labels.len() {
        return Err(format!(
            "{} labels, elkjs has {}",
            ours_labels.len(),
            theirs_labels.len()
        ));
    }
    for (a, b) in ours_labels.iter().zip(&theirs_labels) {
        for (x, y) in a.iter().zip(b) {
            worst = worst.max((x - y).abs());
        }
    }
    let size = |value: &Value| {
        (
            value["width"].as_f64().unwrap_or(0.0),
            value["height"].as_f64().unwrap_or(0.0),
        )
    };
    let ((ours_width, ours_height), (theirs_width, theirs_height)) =
        (size(&ours_value), size(theirs));
    Ok(worst
        .max((ours_width - theirs_width).abs())
        .max((ours_height - theirs_height).abs()))
}
