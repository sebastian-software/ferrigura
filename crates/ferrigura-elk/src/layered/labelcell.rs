//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/cellsystem/LabelCell.java
//!
//! A `LabelCell` stacks labels in one rectangle, in a column or a row, and lays them out inside it.
//! The end label processor keeps one per port, beside the port. The labels are named by their edge
//! and their index in that edge's labels, since the graph keeps them there.

use crate::core::math::{ElkRectangle, KVector};
use crate::layered::lgraph::LGraph;

/// `HorizontalLabelAlignment`: where the labels sit across the cell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HorizontalLabelAlignment {
    Left,
    #[default]
    Center,
    Right,
}

/// `VerticalLabelAlignment`: where the labels sit along the cell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VerticalLabelAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

/// A label of the graph: the index of its edge and its index in the edge's labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelRef {
    pub edge: usize,
    pub label: usize,
}

/// `LabelCell`. Its padding is always zero here, so the padding terms of the Java are left out.
#[derive(Debug, Clone, Default)]
pub struct LabelCell {
    /// `Cell.getCellRectangle()`.
    pub rect: ElkRectangle,
    horizontal_layout_mode: bool,
    horizontal_alignment: HorizontalLabelAlignment,
    vertical_alignment: VerticalLabelAlignment,
    gap: f64,
    /// The labels, in the order they are laid out.
    pub labels: Vec<LabelRef>,
    minimum_content_area_size: KVector,
}

impl LabelCell {
    /// `LabelCell(gap, horizontalLayoutMode)`.
    pub fn new(gap: f64, horizontal_layout_mode: bool) -> Self {
        LabelCell {
            rect: ElkRectangle::default(),
            horizontal_layout_mode,
            horizontal_alignment: HorizontalLabelAlignment::Center,
            vertical_alignment: VerticalLabelAlignment::Center,
            gap,
            labels: Vec::new(),
            minimum_content_area_size: KVector::default(),
        }
    }

    /// `setHorizontalAlignment`.
    pub fn set_horizontal_alignment(&mut self, alignment: HorizontalLabelAlignment) {
        self.horizontal_alignment = alignment;
    }

    /// `setVerticalAlignment`.
    pub fn set_vertical_alignment(&mut self, alignment: VerticalLabelAlignment) {
        self.vertical_alignment = alignment;
    }

    /// `getMinimumWidth`.
    pub fn minimum_width(&self) -> f64 {
        self.minimum_content_area_size.x
    }

    /// `getMinimumHeight`.
    pub fn minimum_height(&self) -> f64 {
        self.minimum_content_area_size.y
    }

    /// `addLabel`: appends a label, and grows the minimum content area by its size and the gap.
    pub fn add_label(&mut self, graph: &LGraph, label: LabelRef) {
        self.labels.push(label);
        let size = label_size(graph, label);
        if self.horizontal_layout_mode {
            self.minimum_content_area_size.x = self.minimum_content_area_size.x.max(size.x);
            self.minimum_content_area_size.y += size.y;
            if self.labels.len() > 1 {
                self.minimum_content_area_size.y += self.gap;
            }
        } else {
            self.minimum_content_area_size.x += size.x;
            self.minimum_content_area_size.y = self.minimum_content_area_size.y.max(size.y);
            if self.labels.len() > 1 {
                self.minimum_content_area_size.x += self.gap;
            }
        }
    }

    /// `applyLabelLayout`, as the positions each label takes inside the cell rectangle.
    pub fn label_layout(&self, graph: &LGraph) -> Vec<(LabelRef, KVector)> {
        if self.horizontal_layout_mode {
            self.horizontal_mode_layout(graph)
        } else {
            self.vertical_mode_layout(graph)
        }
    }

    fn horizontal_mode_layout(&self, graph: &LGraph) -> Vec<(LabelRef, KVector)> {
        let rect = self.rect;
        let mut y = rect.y;
        match self.vertical_alignment {
            VerticalLabelAlignment::Center => {
                y += (rect.height - self.minimum_content_area_size.y) / 2.0;
            }
            VerticalLabelAlignment::Bottom => {
                y += rect.height - self.minimum_content_area_size.y;
            }
            VerticalLabelAlignment::Top => {}
        }
        let mut layout = Vec::with_capacity(self.labels.len());
        for &label in &self.labels {
            let size = label_size(graph, label);
            let x = match self.horizontal_alignment {
                HorizontalLabelAlignment::Left => rect.x,
                HorizontalLabelAlignment::Center => rect.x + (rect.width - size.x) / 2.0,
                HorizontalLabelAlignment::Right => rect.x + rect.width - size.x,
            };
            layout.push((label, KVector { x, y }));
            y += size.y + self.gap;
        }
        layout
    }

    fn vertical_mode_layout(&self, graph: &LGraph) -> Vec<(LabelRef, KVector)> {
        let rect = self.rect;
        let mut x = rect.x;
        match self.horizontal_alignment {
            HorizontalLabelAlignment::Center => {
                x += (rect.width - self.minimum_content_area_size.x) / 2.0;
            }
            HorizontalLabelAlignment::Right => {
                x += rect.width - self.minimum_content_area_size.x;
            }
            HorizontalLabelAlignment::Left => {}
        }
        let mut layout = Vec::with_capacity(self.labels.len());
        for &label in &self.labels {
            let size = label_size(graph, label);
            let y = match self.vertical_alignment {
                VerticalLabelAlignment::Top => rect.y,
                VerticalLabelAlignment::Center => rect.y + (rect.height - size.y) / 2.0,
                VerticalLabelAlignment::Bottom => rect.y + rect.height - size.y,
            };
            layout.push((label, KVector { x, y }));
            x += size.x + self.gap;
        }
        layout
    }
}

/// `LabelAdapter.getSize` of an edge label.
pub fn label_size(graph: &LGraph, label: LabelRef) -> KVector {
    graph.edges[label.edge].labels[label.label].size
}
