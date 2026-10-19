//! The plot of the map and the positions of its nodes: the projection of `wardleyRenderer.ts`
//! (147-166, 297-304), which is the whole layout. There is no layout engine: a position is the
//! projection of the statement's coordinates (NOTES 6.2).

use std::collections::HashMap;

use ferrigura_model::wardley::WardleyDiagram;

/// The canvas and the plot's margins, as the reference's arithmetic reads them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plot {
    /// The width of the canvas, the `size` statement's or the configuration's.
    pub width: f64,
    /// The height of the canvas.
    pub height: f64,
    /// The margin of the plot (`padding`).
    pub padding: f64,
    /// `width - padding * 2`.
    pub chart_width: f64,
    /// `height - padding * 2`.
    pub chart_height: f64,
}

impl Plot {
    /// The plot of a canvas of the given size with the given margin.
    #[must_use]
    pub fn new(width: f64, height: f64, padding: f64) -> Self {
        Self {
            width,
            height,
            padding,
            chart_width: width - padding * 2.0,
            chart_height: height - padding * 2.0,
        }
    }

    /// `projectX`: a percentage of the evolution, as the x coordinate on the canvas.
    #[must_use]
    pub fn x(&self, value: f64) -> f64 {
        self.padding + (value / 100.0) * self.chart_width
    }

    /// `projectY`: a percentage of the visibility, as the y coordinate on the canvas. Visibility
    /// grows upwards.
    #[must_use]
    pub fn y(&self, value: f64) -> f64 {
        self.height - self.padding - (value / 100.0) * self.chart_height
    }
}

/// A point of the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// The x coordinate.
    pub x: f64,
    /// The y coordinate.
    pub y: f64,
}

/// The placements of the nodes, by the index of the node in the diagram's node list. The pipeline
/// step moves a pipeline's parent in place, before the links, the trends and the nodes are drawn,
/// so every later step reads the moved position (NOTES 6.2, "Pipeline parents move").
#[derive(Debug, Clone)]
pub struct Placements<'a> {
    points: Vec<Placement>,
    index: HashMap<&'a str, usize>,
}

impl<'a> Placements<'a> {
    /// The projected positions of every node of the diagram.
    #[must_use]
    pub fn of(diagram: &'a WardleyDiagram, plot: &Plot) -> Self {
        let mut index = HashMap::with_capacity(diagram.nodes.len());
        let points = diagram
            .nodes
            .iter()
            .enumerate()
            .map(|(position, node)| {
                // The DB keeps one node per id, so the first entry of an id is its node.
                index.entry(node.id.as_str()).or_insert(position);
                Placement {
                    x: plot.x(node.x),
                    y: plot.y(node.y),
                }
            })
            .collect();
        Self { points, index }
    }

    /// The index of the node with the id, when there is one.
    #[must_use]
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }

    /// The placement of the node at the index.
    #[must_use]
    pub fn at(&self, index: usize) -> Placement {
        self.points[index]
    }

    /// Moves the node at the index.
    pub fn set(&mut self, index: usize, placement: Placement) {
        self.points[index] = placement;
    }
}
