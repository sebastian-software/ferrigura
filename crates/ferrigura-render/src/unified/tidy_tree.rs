//! The tidy tree layout of `layout: tidy-tree`, which mindmaps choose with that layout name: a port
//! of `@mermaid-js/layout-tidy-tree` 1.0.1 (`packages/mermaid-layout-tidy-tree/src/`, at the commit
//! before `2d85261d6`, which removed the package) on the library `non-layered-tidy-tree-layout`
//! 2.0.2 (`library`, the algorithm of van der Ploeg). The research, with the reference's file and
//! line for each rule, is `docs-port/tidy-tree/NOTES.md`.
//!
//! The root's children alternate between a left tree and a right tree. Each tree is laid out by the
//! library with its boxes turned, so that its breadth is the node's height and its depth the node's
//! width, and the trees stand on either side of the root at `(0, 20)`. Every edge gets four points
//! (`edges`). The layout reads only the sizes, the shapes and the edges: it draws nothing, and draws
//! no random numbers.
//!
//! The quirks of the reference that the port keeps, each probed (NOTES 3.6):
//! - Q1: the first level of each side is centred on the tops of its nodes, which is off by the half
//!   height of the nodes (`layout.ts` 216-233 and 255-256). A node of height 30 at depth 41 has its
//!   centre at `y = 15`, not at the root's `y = 20`.
//! - Q2: the reported `mid` is the midpoint of the first pass, which is not an endpoint of the edge
//!   (`layout.ts` 526-527). The renderer does not read it.
//! - Q3: the round shapes are `circle`, `cloud` and `bang`. `mindmapCircle` is not one of them, so a
//!   `((...))` node is clipped as a rectangle (`layout.ts` 493-498).
//! - Q4: an edge between two nodes of one side goes out past the source and back in to the target,
//!   so it reverses its direction (`layout.ts` 529-577).
//!
//! [`LayoutData::lay_out_tidy_tree`] is the call the mindmap renderer makes after its measure step:
//! see its documentation for what the renderer does with the result.

mod edges;
mod library;
mod split;

use std::collections::HashMap;
use std::fmt;

use crate::RenderError;
use crate::unified::data::LayoutData;
use crate::unified::geom::Point;

/// The width of a node whose box has no measured width (`render.ts` 99).
const DEFAULT_WIDTH: f64 = 100.0;

/// The height of a node whose box has no measured height (`render.ts` 100).
const DEFAULT_HEIGHT: f64 = 50.0;

/// A node of the layout, as `executeTidyTreeLayout` reads it: its id, the size of its box, and its
/// shape, which decides how the edges are clipped.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutNode {
    /// The node's id.
    pub id: String,
    /// The width of its box.
    pub width: f64,
    /// The height of its box.
    pub height: f64,
    /// The shape's name, as the mindmap DB gives it (`circle`, `cloud`, `bang`, `mindmapCircle`, ...).
    pub shape: String,
}

/// An edge of the layout: its id, and the ids of its parent (`start`) and child (`end`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutEdge {
    /// The edge's id.
    pub id: String,
    /// The id of the parent node.
    pub start: String,
    /// The id of the child node.
    pub end: String,
}

/// The side of the root a node is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// The root.
    Root,
    /// The left tree, which grows to the left.
    Left,
    /// The right tree, which grows to the right.
    Right,
}

/// A node after the layout: its centre (`x`, `y`), its section and its size.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedNode {
    /// The node's id.
    pub id: String,
    /// The horizontal centre.
    pub x: f64,
    /// The vertical centre.
    pub y: f64,
    /// The side of the root.
    pub section: Section,
    /// The width of its box.
    pub width: f64,
    /// The height of its box.
    pub height: f64,
    /// The shape of the node.
    pub shape: String,
}

/// An edge after the layout. `points` are the four points the renderer draws, from the start to the
/// end; `start`, `mid` and `end` are the reference's `startX`/`startY`, `midX`/`midY` and
/// `endX`/`endY`, which the renderer does not read. An edge with a node that is not in the layout
/// has the reference's zero result, and no section or size.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedEdge {
    /// The edge's id.
    pub id: String,
    /// The id of the parent node.
    pub source: String,
    /// The id of the child node.
    pub target: String,
    /// The start of the path (the second pass).
    pub start: Point,
    /// The midpoint of the first pass (Q2).
    pub mid: Point,
    /// The end of the path (the second pass).
    pub end: Point,
    /// The points of the path, from the start to the end.
    pub points: Vec<Point>,
    /// The section of the source node.
    pub source_section: Option<Section>,
    /// The section of the target node.
    pub target_section: Option<Section>,
    /// The width of the source node.
    pub source_width: Option<f64>,
    /// The height of the source node.
    pub source_height: Option<f64>,
    /// The width of the target node.
    pub target_width: Option<f64>,
    /// The height of the target node.
    pub target_height: Option<f64>,
}

/// The result of the layout: the nodes in the order the reference pushes them (the root, then the
/// left tree and the right tree, each in pre-order), and the edges in the order of the input.
#[derive(Debug, Clone, PartialEq)]
pub struct TidyTreeLayout {
    /// The positioned nodes.
    pub nodes: Vec<PositionedNode>,
    /// The positioned edges.
    pub edges: Vec<PositionedEdge>,
}

/// An error of the layout. The reference rejects its promise with the same causes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TidyTreeError {
    /// There are no nodes (`layout.ts` 28-30).
    NoNodes,
    /// The contour walk ran past the list of left siblings: the library throws a `TypeError` there
    /// (`algorithm.js` 117-125, read; not seen on any probed tree).
    SiblingListExhausted,
    /// The edges form a cycle, and the reference's recursion does not end.
    Cycle,
}

impl fmt::Display for TidyTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoNodes => write!(f, "the tidy tree layout has no nodes"),
            Self::SiblingListExhausted => write!(
                f,
                "the tidy tree contour walk ran past the list of left siblings"
            ),
            Self::Cycle => write!(f, "the tidy tree edges form a cycle"),
        }
    }
}

impl std::error::Error for TidyTreeError {}

/// Lays out a tree with the tidy tree algorithm: `executeTidyTreeLayout` (`layout.ts` 23-72).
///
/// The nodes are in the order of the diagram, and the edges in the order of their parents'
/// children: the first child of a node is the first edge that starts at it.
///
/// # Errors
///
/// Returns an error when there are no nodes, when the edges form a cycle, or where the library
/// throws (see [`TidyTreeError`]).
pub fn layout(nodes: &[LayoutNode], edges: &[LayoutEdge]) -> Result<TidyTreeLayout, TidyTreeError> {
    split::layout(nodes, edges)
}

/// `Math.min` of two numbers: NaN when either is NaN, which `f64::min` does not give.
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if b < a {
        b
    } else {
        a
    }
}

/// `Math.max` of two numbers: NaN when either is NaN, which `f64::max` does not give.
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if b > a {
        b
    } else {
        a
    }
}

impl LayoutData {
    /// Lays the diagram out as a tidy tree, the layout of `layout: tidy-tree`.
    ///
    /// The renderer calls it after the measure step has set each node's `width` and `height` to the
    /// measured box (`render.ts` 81-84). A node without a size gets 100 by 50, as the reference's
    /// `render.ts` 99-100 gives it. The call sets every node's `x` and `y` to its centre, and every
    /// edge's `points` to the four points of its path; the returned layout carries the rest of the
    /// result (`mid`, the endpoints and the sizes of the edge ends).
    ///
    /// The renderer must skip an edge whose start or end node is not in the diagram (`render.ts`
    /// 132-135): such an edge has the reference's zero points here, and the reference does not draw
    /// it. Each node is translated to its centre (`render.ts` 117).
    ///
    /// # Errors
    ///
    /// Returns an error where [`layout`] does.
    pub fn lay_out_tidy_tree(&mut self) -> Result<TidyTreeLayout, RenderError> {
        let nodes: Vec<LayoutNode> = self
            .nodes
            .iter()
            .map(|node| LayoutNode {
                id: node.id.clone(),
                width: node.width.unwrap_or(DEFAULT_WIDTH),
                height: node.height.unwrap_or(DEFAULT_HEIGHT),
                shape: node.shape.clone(),
            })
            .collect();
        let edges: Vec<LayoutEdge> = self
            .edges
            .iter()
            .map(|edge| LayoutEdge {
                id: edge.id.clone(),
                start: edge.start.clone(),
                end: edge.end.clone(),
            })
            .collect();
        let layout =
            layout(&nodes, &edges).map_err(|error| RenderError::Layout(error.to_string()))?;

        // A node takes the position of the last positioned node with its id (`render.ts` 109-124),
        // and an edge the points of the first positioned edge with its id (`render.ts` 136).
        let mut centres: HashMap<&str, Point> = HashMap::new();
        for node in &layout.nodes {
            centres.insert(node.id.as_str(), Point::new(node.x, node.y));
        }
        for node in &mut self.nodes {
            if let Some(centre) = centres.get(node.id.as_str()) {
                node.x = Some(centre.x);
                node.y = Some(centre.y);
            }
        }
        let mut paths: HashMap<&str, &[Point]> = HashMap::new();
        for edge in &layout.edges {
            paths
                .entry(edge.id.as_str())
                .or_insert(edge.points.as_slice());
        }
        for edge in &mut self.edges {
            if let Some(points) = paths.get(edge.id.as_str()) {
                edge.points = points.to_vec();
            }
        }
        Ok(layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, width: f64, height: f64, shape: &str) -> LayoutNode {
        LayoutNode {
            id: id.to_string(),
            width,
            height,
            shape: shape.to_string(),
        }
    }

    fn edge(start: &str, end: &str) -> LayoutEdge {
        LayoutEdge {
            id: format!("edge_{start}_{end}"),
            start: start.to_string(),
            end: end.to_string(),
        }
    }

    #[test]
    fn a_single_node_is_the_root() {
        let layout = layout(&[node("0", 120.0, 40.0, "rect")], &[]).expect("layout");
        assert_eq!(layout.nodes.len(), 1);
        let root = &layout.nodes[0];
        assert_eq!((root.x, root.y), (0.0, 20.0));
        assert_eq!(root.section, Section::Root);
        assert!(layout.edges.is_empty());
    }

    #[test]
    fn no_nodes_is_an_error() {
        assert_eq!(layout(&[], &[]), Err(TidyTreeError::NoNodes));
    }

    #[test]
    fn a_cycle_is_an_error() {
        let nodes = [node("0", 10.0, 10.0, "rect"), node("1", 10.0, 10.0, "rect")];
        let edges = [edge("0", "1"), edge("1", "0")];
        assert_eq!(layout(&nodes, &edges), Err(TidyTreeError::Cycle));
    }

    // Worked by hand from the reference's rules, and matching NOTES 3.5 and 3.6: a root of 120 by 40
    // with one child of 60 by 30 on the left. The child's centre is at x = -161 (its right edge at
    // -131, which is 71 from the root's left edge at -60), and its y is 15 (Q1). The first-pass
    // midpoint is (-65.5, 17.03) (Q2).
    #[test]
    fn one_child_on_the_left() {
        let nodes = [
            node("0", 120.0, 40.0, "rect"),
            node("1", 60.0, 30.0, "rect"),
        ];
        let layout = layout(&nodes, &[edge("0", "1")]).expect("layout");
        let root = &layout.nodes[0];
        let child = &layout.nodes[1];
        assert_eq!((root.x, root.y), (0.0, 20.0));
        assert_eq!(child.section, Section::Left);
        assert_eq!((child.x, child.y), (-161.0, 15.0));

        let path = &layout.edges[0];
        assert_eq!(
            path.points,
            vec![
                Point::new(-60.0, 20.0),
                Point::new(-90.0, 20.0),
                Point::new(-101.0, 15.0),
                Point::new(-131.0, 15.0),
            ]
        );
        assert_eq!(path.start, Point::new(-60.0, 20.0));
        assert_eq!(path.end, Point::new(-131.0, 15.0));
        assert_eq!(path.mid.x, -65.5);
        assert!((path.mid.y - 17.034).abs() < 0.001, "midY {}", path.mid.y);
        assert_eq!(path.source_section, Some(Section::Root));
        assert_eq!(path.target_section, Some(Section::Left));
    }

    #[test]
    fn the_first_child_goes_left_and_the_second_right() {
        let nodes = [
            node("0", 100.0, 50.0, "rect"),
            node("1", 40.0, 20.0, "rect"),
            node("2", 40.0, 20.0, "rect"),
        ];
        let edges = [edge("0", "1"), edge("0", "2")];
        let layout = layout(&nodes, &edges).expect("layout");
        assert_eq!(layout.nodes[1].section, Section::Left);
        assert_eq!(layout.nodes[2].section, Section::Right);
        assert!(layout.nodes[1].x < 0.0);
        assert!(layout.nodes[2].x > 0.0);
    }

    #[test]
    fn lay_out_tidy_tree_sets_the_centres_and_the_points() {
        let mut data = LayoutData {
            nodes: vec![
                crate::unified::data::Node {
                    id: "0".into(),
                    width: Some(120.0),
                    height: Some(40.0),
                    shape: "rect".into(),
                    ..Default::default()
                },
                crate::unified::data::Node {
                    id: "1".into(),
                    width: Some(60.0),
                    height: Some(30.0),
                    shape: "rect".into(),
                    ..Default::default()
                },
            ],
            edges: vec![crate::unified::data::Edge {
                id: "edge_0_1".into(),
                start: "0".into(),
                end: "1".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let result = data.lay_out_tidy_tree().expect("layout");
        assert_eq!(result.nodes.len(), 2);
        assert_eq!((data.nodes[0].x, data.nodes[0].y), (Some(0.0), Some(20.0)));
        assert_eq!(
            (data.nodes[1].x, data.nodes[1].y),
            (Some(-161.0), Some(15.0))
        );
        assert_eq!(data.edges[0].points.len(), 4);
        assert_eq!(data.edges[0].points[0], Point::new(-60.0, 20.0));
    }
}
