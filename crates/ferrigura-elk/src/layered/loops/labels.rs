//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/loops/SelfHyperLoopLabels.java
//!
//! The labels of a self loop: their stacked size, and their placement once the loop is routed. A
//! hyper loop of one edge carries one edge's labels, so the block holds exactly those; the hyper
//! loops of several edges are not ported (see [`super::model`]). Label management needs a label
//! manager, which no graph of this port has, so it is not ported either.

use crate::core::data::enums::{Direction, PortSide};
use crate::core::math::KVector;
use crate::layered::lgraph::{LGraph, LabelId};
use crate::layered::loops::model::graph_direction;
use crate::layered::options;

/// `SelfHyperLoopLabels.Alignment`: where the labels sit along their side.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    CENTER,
    LEFT,
    RIGHT,
    TOP,
}

/// `SelfHyperLoopLabels`: the labels of a self loop, stacked into one block.
#[derive(Debug, Clone, Default)]
pub struct SelfHyperLoopLabels {
    /// `id`: the index the routing slot assigner gives the labels, for the overlap matrix.
    pub id: usize,
    /// The labels, as ids in `LGraph::labels`, in the order of their edges.
    pub labels: Vec<LabelId>,
    /// `getSize()`: the size of the stacked block.
    pub size: KVector,
    /// `getPosition()`: the position of the block, relative to the node.
    pub position: KVector,
    /// `getSide()`: the side of the node the labels are placed on.
    pub side: Option<PortSide>,
    /// `getAlignment()`.
    pub alignment: Option<Alignment>,
    /// `getAlignmentReferenceSLPort()`: the index of the port in `SelfLoopHolder::sl_ports`.
    pub alignment_reference: Option<usize>,
    /// `layoutDirection.isHorizontal()`: the labels are stacked vertically.
    horizontal: bool,
    /// `labelLabelSpacing`: the space between two labels.
    label_spacing: f64,
}

impl SelfHyperLoopLabels {
    /// `new SelfHyperLoopLabels(slLoop)`: the layout direction and the label spacing of the graph.
    pub fn new(graph: &LGraph) -> Self {
        let horizontal = matches!(graph_direction(graph), Direction::LEFT | Direction::RIGHT);
        let label_spacing = graph
            .properties
            .double_with_default(options::SPACING_LABEL_LABEL);
        SelfHyperLoopLabels {
            horizontal,
            label_spacing,
            ..SelfHyperLoopLabels::default()
        }
    }

    /// `addLLabels`: adds the labels, and grows the block by each one.
    pub fn add_labels(&mut self, graph: &LGraph, labels: &[LabelId]) {
        for &label in labels {
            self.labels.push(label);
            self.update_size(graph.labels[label].size);
        }
    }

    /// `updateSize`: stacks a label onto the block, with a spacing from the second label on.
    fn update_size(&mut self, new_size: KVector) {
        if self.horizontal {
            // The labels will be stacked vertically.
            self.size.x = self.size.x.max(new_size.x);
            self.size.y += new_size.y;
            if self.labels.len() > 1 {
                self.size.y += self.label_spacing;
            }
        } else {
            // The labels will be stacked horizontally.
            self.size.x += new_size.x;
            self.size.y = self.size.y.max(new_size.y);
            if self.labels.len() > 1 {
                self.size.x += self.label_spacing;
            }
        }
    }

    /// `applyPlacement(offset)`: moves each label to its place in the block, relative to the node.
    pub fn apply_placement(&self, graph: &mut LGraph, offset: KVector) {
        if self.horizontal {
            self.apply_horizontal_placement(graph, offset);
        } else {
            self.apply_vertical_placement(graph, offset);
        }
    }

    /// `applyPlacementForHorizontalLayout`.
    fn apply_horizontal_placement(&self, graph: &mut LGraph, offset: KVector) {
        let mut y = self.position.y;
        for &label in &self.labels {
            let size = graph.labels[label].size;
            let x = if self.alignment == Some(Alignment::LEFT) || self.side == Some(PortSide::EAST)
            {
                self.position.x
            } else if self.alignment == Some(Alignment::RIGHT) || self.side == Some(PortSide::WEST)
            {
                self.position.x + self.size.x - size.x
            } else {
                // Alignment is centre.
                self.position.x + (self.size.x - size.x) / 2.0
            };
            let position = &mut graph.labels[label].position;
            position.x = x + offset.x;
            position.y = y + offset.y;
            y += size.y + self.label_spacing;
        }
    }

    /// `applyPlacementVerticalForVerticalLayout`: labels are top-aligned, except on the north side.
    fn apply_vertical_placement(&self, graph: &mut LGraph, offset: KVector) {
        let mut x = self.position.x;
        for &label in &self.labels {
            let size = graph.labels[label].size;
            let y = if self.side == Some(PortSide::NORTH) {
                self.position.y + self.size.y - size.y
            } else {
                self.position.y
            };
            let position = &mut graph.labels[label].position;
            position.x = x + offset.x;
            position.y = y + offset.y;
            x += size.x + self.label_spacing;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::OptionValue;
    use crate::layered::lgraph::LLabel;

    /// A graph laid out downwards, with a label spacing of 2, and two labels of the given sizes.
    fn graph_with_labels(sizes: &[(f64, f64)]) -> (LGraph, Vec<LabelId>) {
        let mut graph = LGraph::default();
        graph.properties.set(
            options::DIRECTION,
            Some(OptionValue::Enum {
                class: "Direction",
                name: "DOWN".to_string(),
            }),
        );
        graph
            .properties
            .set(options::SPACING_LABEL_LABEL, Some(OptionValue::Double(2.0)));
        let ids = sizes
            .iter()
            .map(|&(x, y)| {
                graph.add_label(LLabel {
                    size: KVector { x, y },
                    ..LLabel::default()
                })
            })
            .collect();
        (graph, ids)
    }

    #[test]
    fn labels_of_a_downward_graph_stack_sideways_with_their_spacing() {
        let (graph, ids) = graph_with_labels(&[(10.0, 5.0), (20.0, 7.0)]);
        let mut labels = SelfHyperLoopLabels::new(&graph);

        labels.add_labels(&graph, &ids);

        // The widths add up with one spacing between the two; the height is the taller label.
        assert_eq!(labels.size, KVector { x: 32.0, y: 7.0 });
        assert_eq!(labels.labels, ids);
    }

    #[test]
    fn a_single_label_takes_its_own_size() {
        let (graph, ids) = graph_with_labels(&[(10.0, 5.0)]);
        let mut labels = SelfHyperLoopLabels::new(&graph);

        labels.add_labels(&graph, &ids);

        assert_eq!(labels.size, KVector { x: 10.0, y: 5.0 });
    }
}
