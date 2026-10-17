//! The layout of a mindmap: the name the text gives, and the three engines it can reach.
//!
//! The reference's `draw` resolves the name with `getRegisteredLayoutAlgorithm(config.layout,
//! {fallback: 'cose-bilkent'})` (`mindmapRenderer.ts` 47-49), and `getData` sets the layout to
//! `cose-bilkent` unless the top-level `layout` is user-defined (`mindmapDb.ts` 361-367).
//!
//! The cose-bilkent hand-off (`cose-bilkent/render.ts` 86-135) gives the node centres from
//! `ferrigura-cose`, and the edge points from cytoscape's rscratch for its default 30 by 30 node style
//! (`docs-port/cose/NOTES.md` 5.2, `mindmap/NOTES.md` 6.5). The engine never reads Mermaid's box size
//! for the endpoints, so every edge end lies on a circle of radius 15 around its node's centre.

use ferrigura_cose::{Edge as CoseEdge, Node as CoseNode};

use crate::RenderError;
use crate::unified::data::LayoutData;
use crate::unified::geom::Point;

/// The layouts a mindmap can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// `cose-bilkent`: the default, and the fallback of an unregistered name.
    CoseBilkent,
    /// `tidy-tree`: the removed package's tidy tree (`unified::tidy_tree`).
    TidyTree,
    /// The ELK names (`elk`, `elk.layered`, ...): the unified ELK path. The reference crashes for
    /// every mindmap with these (`mindmap/NOTES.md` 6.12), so the port lays the mindmap out with ELK,
    /// a named exception; the harness does not compare these fixtures.
    Elk,
}

/// The layout of a layout name. `swimlane` and the names that are not registered fall back to
/// cose-bilkent (`getRegisteredLayoutAlgorithm`, `render.ts` 113-129); `swimlane` is registered in the
/// reference, but the port does not lay out with it, which is the one named difference of the name
/// rule (`mindmap/NOTES.md` 8, question 8).
#[must_use]
pub fn of(name: &str) -> Layout {
    match name {
        "tidy-tree" => Layout::TidyTree,
        name if name == "elk" || name.starts_with("elk.") => Layout::Elk,
        _ => Layout::CoseBilkent,
    }
}

/// The half of the default node's style size: cytoscape's `outerWidth()` of a node is 30 with no
/// border and no padding (`cytoscape.cjs.js` 18547-18548, 12087-12101), so the endpoints lie on a
/// circle of this radius (`mindmap/NOTES.md` 6.5).
const DEFAULT_NODE_HALF_SIZE: f64 = 15.0;

/// Lays out the measured nodes with cose-bilkent: each node takes its centre, and each edge the three
/// points of cytoscape's straight edge (start, mid, end), as `render.ts` 125-135 stores them.
///
/// # Errors
///
/// Returns an error when the engine rejects the graph (a cycle, or two or more nodes with no edge).
pub fn cose(data: &mut LayoutData) -> Result<(), RenderError> {
    let nodes: Vec<CoseNode> = data
        .nodes
        .iter()
        .map(|node| CoseNode {
            id: node.id.clone(),
            width: node.width.unwrap_or(0.0),
            height: node.height.unwrap_or(0.0),
        })
        .collect();
    let edges: Vec<CoseEdge> = data
        .edges
        .iter()
        .map(|edge| CoseEdge {
            id: edge.id.clone(),
            source: edge.start.clone(),
            target: edge.end.clone(),
        })
        .collect();
    let positions = ferrigura_cose::layout(&nodes, &edges)
        .map_err(|error| RenderError::Layout(error.to_string()))?;

    let centres: std::collections::HashMap<&str, Point> = positions
        .iter()
        .map(|position| (position.id.as_str(), Point::new(position.x, position.y)))
        .collect();
    for node in &mut data.nodes {
        if let Some(centre) = centres.get(node.id.as_str()) {
            node.x = Some(centre.x);
            node.y = Some(centre.y);
        }
    }
    for edge in &mut data.edges {
        let (Some(&source), Some(&target)) = (
            centres.get(edge.start.as_str()),
            centres.get(edge.end.as_str()),
        ) else {
            continue;
        };
        let (start, mid, end) = straight_edge(source, target);
        edge.points = vec![start, mid, end];
    }
    Ok(())
}

/// The three rscratch points of a straight edge between two node centres, as cytoscape computes them
/// for a plain edge with the default node style: `findEndpoints` (`cytoscape.cjs.js` 24566-24741) and
/// `storeAllpts` (24175-24179).
///
/// The end is where the line from the source centre leaves the target's ellipse, shortened by 0; the
/// start is the same on the source side, from the target centre. The midpoint is the average of the
/// four points in cytoscape's order, `((startX + endX) + arrowStartX + arrowEndX) / 4`: the arrow
/// points are the ends themselves, since the default arrow is `none` with no spacing and no gap.
#[must_use]
pub fn straight_edge(source: Point, target: Point) -> (Point, Point, Point) {
    // findEndpoints: p1 is the source centre (for the target's end), p2 the target centre.
    let end_hit = ellipse_hit(source.x, source.y, target.x, target.y);
    let end = shorten(end_hit, source, 0.0);
    let start_hit = ellipse_hit(target.x, target.y, source.x, source.y);
    let start = shorten(start_hit, target, 0.0);
    let mid_x = (start.x + end.x + start.x + end.x) / 4.0;
    let mid_y = (start.y + end.y + start.y + end.y) / 4.0;
    (start, Point::new(mid_x, mid_y), end)
}

/// `intersectLineEllipse` (`cytoscape.cjs.js` 4073-4086) for the default node: the point where the
/// line from `(x, y)` towards the centre `(cx, cy)` leaves the ellipse of that centre, whose half sizes
/// are both [`DEFAULT_NODE_HALF_SIZE`]. Where the external point lies within the ellipse, cytoscape
/// returns no point and the reference reads `undefined`, so the result is NaN (its `badLine` case).
fn ellipse_hit(x: f64, y: f64, cx: f64, cy: f64) -> Point {
    let radius = DEFAULT_NODE_HALF_SIZE;
    let mut disp_x = cx - x;
    let mut disp_y = cy - y;
    disp_x /= radius;
    disp_y /= radius;
    let len = (disp_x * disp_x + disp_y * disp_y).sqrt();
    let new_length = len - 1.0;
    if new_length < 0.0 {
        return Point::new(f64::NAN, f64::NAN);
    }
    let proportion = new_length / len;
    Point::new((cx - x) * proportion + x, (cy - y) * proportion + y)
}

/// `shortenIntersection` (`cytoscape.cjs.js` 4300-4308): moves `intersection` towards `offset` by
/// `amount`. With an amount of 0 the result is `offset + 1 * (intersection - offset)`, which is not
/// always the intersection itself in the last bit, so the operations are kept as they are.
fn shorten(intersection: Point, offset: Point, amount: f64) -> Point {
    let disp_x = intersection.x - offset.x;
    let disp_y = intersection.y - offset.y;
    let length = (disp_x * disp_x + disp_y * disp_y).sqrt();
    let mut proportion = (length - amount) / length;
    if proportion < 0.0 {
        proportion = 0.00001;
    }
    Point::new(
        offset.x + proportion * disp_x,
        offset.y + proportion * disp_y,
    )
}

/// The tidy tree of `layout: tidy-tree`: the centres and the four points of each edge
/// (`LayoutData::lay_out_tidy_tree`).
///
/// # Errors
///
/// Returns an error where the tidy tree does (no nodes, or a cycle).
pub fn tidy(data: &mut LayoutData) -> Result<(), RenderError> {
    data.lay_out_tidy_tree().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// The corpus cases of the cose-bilkent port (`crates/ferrigura-cose/tests/corpus`): the centres
    /// cytoscape-cose-bilkent returned and the rscratch points of its edges, all in Chromium.
    fn corpus_cases() -> Vec<Value> {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ferrigura-cose/tests/corpus");
        let manifest: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join("manifest.json")).expect("manifest.json"),
        )
        .expect("manifest.json parses");
        manifest["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .map(|case| {
                let name = case["name"].as_str().expect("name");
                serde_json::from_str(
                    &std::fs::read_to_string(dir.join(format!("{name}.json"))).expect("case"),
                )
                .expect("case parses")
            })
            .collect()
    }

    fn number(value: &Value) -> f64 {
        value.as_f64().expect("a number")
    }

    /// `straight_edge` gives the rscratch points of every corpus edge exactly, from the centres the
    /// engine returned. The centres and the points are cytoscape's own values, so the comparison is
    /// bit for bit: a difference is an operation order, not rounding.
    #[test]
    fn straight_edge_matches_cytoscape_rscratch_on_the_corpus() {
        let mut compared = 0;
        for case in corpus_cases() {
            let centres: std::collections::HashMap<String, Point> = case["output"]["positions"]
                .as_array()
                .expect("positions")
                .iter()
                .map(|p| {
                    (
                        p["id"].as_str().expect("id").to_owned(),
                        Point::new(number(&p["x"]), number(&p["y"])),
                    )
                })
                .collect();
            let edges = case["input"]["edges"].as_array().expect("edges");
            for point in case["output"]["edgePoints"].as_array().expect("edgePoints") {
                let id = point["id"].as_str().expect("id");
                let edge = edges
                    .iter()
                    .find(|edge| edge["id"].as_str() == Some(id))
                    .expect("the edge of the input");
                let source = centres[edge["source"].as_str().expect("source")];
                let target = centres[edge["target"].as_str().expect("target")];
                let (start, mid, end) = straight_edge(source, target);
                assert_eq!(
                    (start.x, start.y, mid.x, mid.y, end.x, end.y),
                    (
                        number(&point["startX"]),
                        number(&point["startY"]),
                        number(&point["midX"]),
                        number(&point["midY"]),
                        number(&point["endX"]),
                        number(&point["endY"]),
                    ),
                    "{} in {}",
                    id,
                    case["name"]
                );
                compared += 1;
            }
        }
        assert!(compared > 0, "the corpus has edges to compare");
    }

    #[test]
    fn the_layout_names_resolve_as_the_reference_does() {
        assert_eq!(of("tidy-tree"), Layout::TidyTree);
        assert_eq!(of("elk"), Layout::Elk);
        assert_eq!(of("elk.stress"), Layout::Elk);
        assert_eq!(of("cose-bilkent"), Layout::CoseBilkent);
        // An unregistered name falls back to cose-bilkent (`getRegisteredLayoutAlgorithm`).
        assert_eq!(of("dagre"), Layout::CoseBilkent);
        assert_eq!(of(""), Layout::CoseBilkent);
    }
}
