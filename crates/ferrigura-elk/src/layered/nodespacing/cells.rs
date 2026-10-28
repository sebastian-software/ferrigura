//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/cellsystem/AtomicCell.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/cellsystem/Cell.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/cellsystem/ContainerCell.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/cellsystem/StripContainerCell.java
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/cellsystem/GridContainerCell.java
//!
//! The cell system of the node spacing code, for the reduced path. Cells live in one arena and
//! refer to each other by [`CellId`]: the Java shares a cell between its container and the node
//! context, which plain Rust references cannot express.
//!
//! `LabelCell` holds the node labels of one inside location. Its labels are not in the arena: the
//! caller keeps them, and passes their sizes to [`CellSystem::label_positions`]. Left out, because
//! only outside labels, port labels or the options they need reach them: the outside containers,
//! the minimum size of a grid's center cell (set only for `MINIMUM_SIZE_ACCOUNTS_FOR_PADDING`) and
//! the grid's center rectangle (read only by `COMPUTE_PADDING`).
//!
//! The arithmetic keeps the Java operand order. All values are `double` in Java, so `f64`.

use crate::core::math::{KVector, Spacing};

use super::{Rect, java_max};

/// The cell of a container's row or column, `ContainerArea`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Area {
    Begin,
    Center,
    End,
}

const AREAS: [Area; 3] = [Area::Begin, Area::Center, Area::End];

impl Area {
    fn index(self) -> usize {
        match self {
            Area::Begin => 0,
            Area::Center => 1,
            Area::End => 2,
        }
    }
}

/// `StripContainerCell.Strip`: whether the children of a strip are stacked vertically or placed
/// side by side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Strip {
    Vertical,
    Horizontal,
}

impl Strip {
    /// The axis the children are arranged along.
    fn along(self) -> Axis {
        match self {
            Strip::Vertical => Axis::Y,
            Strip::Horizontal => Axis::X,
        }
    }
}

/// The two axes: X runs left to right, Y top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

impl Axis {
    /// The padding at the start of the axis: left or top.
    fn start(self, padding: &Spacing) -> f64 {
        match self {
            Axis::X => padding.left,
            Axis::Y => padding.top,
        }
    }

    /// The padding at the end of the axis: right or bottom.
    fn end(self, padding: &Spacing) -> f64 {
        match self {
            Axis::X => padding.right,
            Axis::Y => padding.bottom,
        }
    }

    fn pos(self, rect: &Rect) -> f64 {
        match self {
            Axis::X => rect.x,
            Axis::Y => rect.y,
        }
    }

    fn len(self, rect: &Rect) -> f64 {
        match self {
            Axis::X => rect.width,
            Axis::Y => rect.height,
        }
    }

    /// `applyHorizontalLayout` and `applyVerticalLayout`: sets the position and the extent.
    fn set(self, rect: &mut Rect, pos: f64, len: f64) {
        match self {
            Axis::X => {
                rect.x = pos;
                rect.width = len;
            }
            Axis::Y => {
                rect.y = pos;
                rect.height = len;
            }
        }
    }

    fn content(self, size: &KVector) -> f64 {
        match self {
            Axis::X => size.x,
            Axis::Y => size.y,
        }
    }
}

/// A cell's handle in a [`CellSystem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CellId(usize);

/// `HorizontalLabelAlignment`: where a label cell puts its labels across its width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

/// `VerticalLabelAlignment`: where a label cell puts its labels along its height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

/// What a cell is, with the references to its children.
#[derive(Debug, Clone, Copy)]
enum Kind {
    /// `AtomicCell`: content of a minimum size, which the padding surrounds.
    Atomic { min_content: KVector },
    /// `LabelCell`: the labels added so far, as one minimum content area. `count` is the number of
    /// labels, which decides where the gap goes.
    Label {
        horizontal_layout: bool,
        gap: f64,
        horizontal: HorizontalAlignment,
        vertical: VerticalAlignment,
        min_content: KVector,
        count: usize,
    },
    /// `StripContainerCell`, with its children at begin, center and end.
    Strip {
        strip: Strip,
        symmetrical: bool,
        gap: f64,
        cells: [Option<CellId>; 3],
    },
    /// `GridContainerCell`, with children at the nine row and column positions.
    Grid {
        tabular: bool,
        symmetrical: bool,
        gap: f64,
        cells: [[Option<CellId>; 3]; 3],
    },
}

/// `Cell`: a padded rectangle that a container lays out.
#[derive(Debug, Clone, Copy)]
struct Cell {
    padding: Spacing,
    rect: Rect,
    contributes_to_min_width: bool,
    contributes_to_min_height: bool,
    kind: Kind,
}

impl Cell {
    fn contributes(&self, axis: Axis) -> bool {
        match axis {
            Axis::X => self.contributes_to_min_width,
            Axis::Y => self.contributes_to_min_height,
        }
    }
}

/// The arena of cells of one node.
#[derive(Debug, Clone, Default)]
pub(super) struct CellSystem {
    cells: Vec<Cell>,
}

/// Sums the positive values and adds `gap` between each two of them: `sumWithGaps`.
fn sum_with_gaps(values: &[f64], gap: f64) -> f64 {
    let mut sum = 0.0;
    let mut active = 0usize;
    for &value in values {
        if value > 0.0 {
            sum += value;
            active += 1;
        }
    }
    if active > 1 {
        sum += gap * (active - 1) as f64;
    }
    sum
}

impl CellSystem {
    fn add(&mut self, kind: Kind) -> CellId {
        self.cells.push(Cell {
            padding: Spacing::default(),
            rect: Rect::default(),
            contributes_to_min_width: false,
            contributes_to_min_height: false,
            kind,
        });
        CellId(self.cells.len() - 1)
    }

    /// An `AtomicCell` without content.
    pub fn atomic(&mut self) -> CellId {
        self.add(Kind::Atomic {
            min_content: KVector::default(),
        })
    }

    /// A `StripContainerCell` without children.
    pub fn strip(&mut self, strip: Strip, symmetrical: bool, gap: f64) -> CellId {
        self.add(Kind::Strip {
            strip,
            symmetrical,
            gap,
            cells: [None; 3],
        })
    }

    /// A `GridContainerCell` without children.
    pub fn grid(&mut self, tabular: bool, symmetrical: bool, gap: f64) -> CellId {
        self.add(Kind::Grid {
            tabular,
            symmetrical,
            gap,
            cells: [[None; 3]; 3],
        })
    }

    /// `StripContainerCell.setCell(area, cell)`.
    pub fn set_strip_cell(&mut self, strip: CellId, area: Area, child: CellId) {
        match &mut self.cells[strip.0].kind {
            Kind::Strip { cells, .. } => cells[area.index()] = Some(child),
            _ => unreachable!("set_strip_cell on a cell that is not a strip"),
        }
    }

    /// `GridContainerCell.setCell(row, column, cell)`.
    pub fn set_grid_cell(&mut self, grid: CellId, row: Area, column: Area, child: CellId) {
        match &mut self.cells[grid.0].kind {
            Kind::Grid { cells, .. } => cells[row.index()][column.index()] = Some(child),
            _ => unreachable!("set_grid_cell on a cell that is not a grid"),
        }
    }

    /// `new LabelCell(gap, location, horizontalLayoutMode)`: a label cell without labels, with the
    /// alignments of its location.
    pub fn label(
        &mut self,
        gap: f64,
        horizontal_layout: bool,
        horizontal: HorizontalAlignment,
        vertical: VerticalAlignment,
    ) -> CellId {
        self.add(Kind::Label {
            horizontal_layout,
            gap,
            horizontal,
            vertical,
            min_content: KVector::default(),
            count: 0,
        })
    }

    /// `LabelCell.addLabel(label)`, given the size of the label: the minimum content area grows
    /// along the stacking direction, and by the gap between two labels.
    pub fn add_label(&mut self, cell: CellId, size: KVector) {
        let Kind::Label {
            horizontal_layout,
            gap,
            min_content,
            count,
            ..
        } = &mut self.cells[cell.0].kind
        else {
            unreachable!("add_label on a cell that is not a label cell");
        };
        *count += 1;
        if *horizontal_layout {
            min_content.x = java_max(min_content.x, size.x);
            min_content.y += size.y;
            if *count > 1 {
                min_content.y += *gap;
            }
        } else {
            min_content.x += size.x;
            min_content.y = java_max(min_content.y, size.y);
            if *count > 1 {
                min_content.x += *gap;
            }
        }
    }

    /// `Cell.setContributesToMinimumWidth` and `setContributesToMinimumHeight`.
    pub fn set_contributes(&mut self, cell: CellId, width: bool, height: bool) {
        let entry = &mut self.cells[cell.0];
        if width {
            entry.contributes_to_min_width = true;
        }
        if height {
            entry.contributes_to_min_height = true;
        }
    }

    /// `LabelCell.applyLabelLayout()` as positions: the top left corner of each label, relative to
    /// the cell's rectangle, in the order the labels were added. `sizes` are the labels' sizes in
    /// that order.
    pub fn label_positions(&self, cell: CellId, sizes: &[KVector]) -> Vec<KVector> {
        let entry = &self.cells[cell.0];
        let Kind::Label {
            horizontal_layout,
            gap,
            horizontal,
            vertical,
            min_content,
            ..
        } = entry.kind
        else {
            return Vec::new();
        };
        let rect = entry.rect;
        let padding = entry.padding;
        let mut positions = Vec::with_capacity(sizes.len());
        if horizontal_layout {
            let mut y = rect.y;
            match vertical {
                VerticalAlignment::Top => {}
                VerticalAlignment::Center => y += (rect.height - min_content.y) / 2.0,
                VerticalAlignment::Bottom => y += rect.height - min_content.y,
            }
            for size in sizes {
                let x = match horizontal {
                    HorizontalAlignment::Left => rect.x + padding.left,
                    HorizontalAlignment::Center => {
                        rect.x + padding.left + (rect.width - size.x) / 2.0
                    }
                    HorizontalAlignment::Right => rect.x + rect.width - padding.right - size.x,
                };
                positions.push(KVector { x, y });
                y += size.y + gap;
            }
        } else {
            let mut x = rect.x;
            match horizontal {
                HorizontalAlignment::Left => {}
                HorizontalAlignment::Center => x += (rect.width - min_content.x) / 2.0,
                HorizontalAlignment::Right => x += rect.width - min_content.x,
            }
            for size in sizes {
                let y = match vertical {
                    VerticalAlignment::Top => rect.y + padding.top,
                    VerticalAlignment::Center => {
                        rect.y + padding.top + (rect.height - size.y) / 2.0
                    }
                    VerticalAlignment::Bottom => rect.y + rect.height - padding.bottom - size.y,
                };
                positions.push(KVector { x, y });
                x += size.x + gap;
            }
        }
        positions
    }

    pub fn padding(&self, cell: CellId) -> Spacing {
        self.cells[cell.0].padding
    }

    pub fn padding_mut(&mut self, cell: CellId) -> &mut Spacing {
        &mut self.cells[cell.0].padding
    }

    pub fn rect(&self, cell: CellId) -> Rect {
        self.cells[cell.0].rect
    }

    pub fn rect_mut(&mut self, cell: CellId) -> &mut Rect {
        &mut self.cells[cell.0].rect
    }

    /// `AtomicCell.getMinimumContentAreaSize()`.
    pub fn min_content(&self, cell: CellId) -> KVector {
        match self.cells[cell.0].kind {
            Kind::Atomic { min_content } => min_content,
            _ => unreachable!("only atomic cells have a minimum content area"),
        }
    }

    /// `AtomicCell.getMinimumContentAreaSize()`, to be set.
    pub fn min_content_mut(&mut self, cell: CellId) -> &mut KVector {
        match &mut self.cells[cell.0].kind {
            Kind::Atomic { min_content } => min_content,
            _ => unreachable!("only atomic cells have a minimum content area"),
        }
    }

    /// `Cell.getMinimumHeight()`.
    pub fn min_height(&self, cell: CellId) -> f64 {
        self.min_extent(cell, Axis::Y)
    }

    /// `Cell.getMinimumWidth()`.
    pub fn min_width(&self, cell: CellId) -> f64 {
        self.min_extent(cell, Axis::X)
    }

    /// `GridContainerCell.getCell(row, column)`: the child at that position, if there is one.
    pub fn grid_cell(&self, grid: CellId, row: Area, column: Area) -> Option<CellId> {
        match self.cells[grid.0].kind {
            Kind::Grid { cells, .. } => cells[row.index()][column.index()],
            _ => None,
        }
    }

    /// `Cell.getMinimumWidth()` or `getMinimumHeight()`: the content plus the padding, or zero for
    /// a container that holds nothing.
    fn min_extent(&self, cell: CellId, axis: Axis) -> f64 {
        let entry = &self.cells[cell.0];
        let start = axis.start(&entry.padding);
        let end = axis.end(&entry.padding);
        let value = match entry.kind {
            Kind::Atomic { min_content } => {
                return axis.content(&min_content) + start + end;
            }
            // `LabelCell.getMinimumWidth` and `getMinimumHeight`: the content and the padding, with
            // no zero case.
            Kind::Label { min_content, .. } => {
                return axis.content(&min_content) + start + end;
            }
            Kind::Strip {
                strip,
                symmetrical,
                gap,
                cells,
            } => {
                if strip.along() == axis {
                    sum_with_gaps(&self.strip_min_sizes(cells, axis, symmetrical, true), gap)
                } else {
                    // Across the strip, the largest contributing child decides. `Stream.max()`
                    // reduces with `Math.max`.
                    cells
                        .into_iter()
                        .flatten()
                        .filter(|&child| self.cells[child.0].contributes(axis))
                        .map(|child| self.min_extent(child, axis))
                        .reduce(java_max)
                        .unwrap_or(0.0)
                }
            }
            Kind::Grid {
                tabular,
                symmetrical,
                gap,
                cells,
            } => match axis {
                Axis::X if tabular => sum_with_gaps(
                    &self.grid_column_min_widths(cells, None, symmetrical, true),
                    gap,
                ),
                Axis::X => AREAS.iter().fold(0.0, |max, &row| {
                    java_max(
                        max,
                        sum_with_gaps(
                            &self.grid_column_min_widths(cells, Some(row), symmetrical, true),
                            gap,
                        ),
                    )
                }),
                Axis::Y => sum_with_gaps(&self.grid_row_min_heights(cells, symmetrical, true), gap),
            },
        };
        if value > 0.0 {
            value + start + end
        } else {
            0.0
        }
    }

    /// `minWidthOfCell` and `minHeightOfCell`: zero for a missing cell, for one that does not
    /// contribute when the flag is respected, and for an atomic cell without content.
    fn min_of_child(&self, child: Option<CellId>, axis: Axis, respect: bool) -> f64 {
        let Some(child) = child else {
            return 0.0;
        };
        let entry = &self.cells[child.0];
        if respect && !entry.contributes(axis) {
            return 0.0;
        }
        match entry.kind {
            Kind::Atomic { min_content } if axis.content(&min_content) == 0.0 => 0.0,
            _ => self.min_extent(child, axis),
        }
    }

    /// `minCellWidths` and `minCellHeights` of a strip; symmetrical strips take the larger of the
    /// two outer cells for both.
    fn strip_min_sizes(
        &self,
        cells: [Option<CellId>; 3],
        axis: Axis,
        symmetrical: bool,
        respect: bool,
    ) -> [f64; 3] {
        let mut sizes = cells.map(|child| self.min_of_child(child, axis, respect));
        if symmetrical {
            sizes[0] = java_max(sizes[0], sizes[2]);
            sizes[2] = sizes[0];
        }
        sizes
    }

    /// `minColumnWidths`: the widths of the three columns, for one row or for all rows.
    fn grid_column_min_widths(
        &self,
        cells: [[Option<CellId>; 3]; 3],
        row: Option<Area>,
        symmetrical: bool,
        respect: bool,
    ) -> [f64; 3] {
        let mut widths = AREAS.map(|column| {
            let mut max = 0.0;
            match row {
                Some(row) => {
                    max = self.min_of_child(cells[row.index()][column.index()], Axis::X, respect);
                }
                None => {
                    for row in AREAS {
                        max = java_max(
                            max,
                            self.min_of_child(cells[row.index()][column.index()], Axis::X, respect),
                        );
                    }
                }
            }
            max
        });
        if symmetrical {
            widths[0] = java_max(widths[0], widths[2]);
            widths[2] = widths[0];
        }
        widths
    }

    /// `minRowHeights`: the heights of the three rows.
    fn grid_row_min_heights(
        &self,
        cells: [[Option<CellId>; 3]; 3],
        symmetrical: bool,
        respect: bool,
    ) -> [f64; 3] {
        let mut heights = AREAS.map(|row| {
            let mut max = 0.0;
            for column in AREAS {
                max = java_max(
                    max,
                    self.min_of_child(cells[row.index()][column.index()], Axis::Y, respect),
                );
            }
            max
        });
        if symmetrical {
            heights[0] = java_max(heights[0], heights[2]);
            heights[2] = heights[0];
        }
        heights
    }

    /// `layoutChildrenHorizontally` of a cell whose rectangle is already set.
    pub fn layout_horizontally(&mut self, cell: CellId) {
        self.layout(cell, Axis::X);
    }

    /// `layoutChildrenVertically` of a cell whose rectangle is already set.
    pub fn layout_vertically(&mut self, cell: CellId) {
        self.layout(cell, Axis::Y);
    }

    fn layout(&mut self, cell: CellId, axis: Axis) {
        let entry = self.cells[cell.0];
        let rect = entry.rect;
        let padding = entry.padding;
        match entry.kind {
            Kind::Atomic { .. } | Kind::Label { .. } => {}
            Kind::Strip {
                strip,
                symmetrical,
                gap,
                cells,
            } => {
                let start = axis.start(&padding);
                let end = axis.end(&padding);
                let pos = axis.pos(&rect);
                let len = axis.len(&rect);
                if strip.along() == axis {
                    let mut sizes = self.strip_min_sizes(cells, axis, symmetrical, false);
                    self.apply(cells[0], axis, pos + start, sizes[0]);
                    self.apply(cells[2], axis, pos + len - end - sizes[2], sizes[2]);
                    let mut free = len - start - end;
                    if sizes[0] > 0.0 {
                        free -= sizes[0] + gap;
                        sizes[0] += gap;
                    }
                    if sizes[2] > 0.0 {
                        free -= sizes[2] + gap;
                    }
                    sizes[1] = java_max(sizes[1], free);
                    self.apply(
                        cells[1],
                        axis,
                        pos + start + sizes[0] - (sizes[1] - free) / 2.0,
                        sizes[1],
                    );
                } else {
                    // Across the strip every child gets the full extent, minus the padding.
                    for child in cells {
                        self.apply(child, axis, pos + start, len - start - end);
                    }
                }
                for child in cells.into_iter().flatten() {
                    self.layout(child, axis);
                }
            }
            Kind::Grid {
                tabular,
                symmetrical,
                gap,
                cells,
            } => {
                if axis == Axis::X {
                    // Tabular grids use one set of column widths for every row, which the
                    // Java passes on by reference and mutates each time.
                    if tabular {
                        let mut widths =
                            self.grid_column_min_widths(cells, None, symmetrical, false);
                        for _ in AREAS {
                            self.apply_widths(cells, rect, padding, gap, &mut widths);
                        }
                    } else {
                        for row in AREAS {
                            let mut widths =
                                self.grid_column_min_widths(cells, Some(row), symmetrical, false);
                            self.apply_widths(cells, rect, padding, gap, &mut widths);
                        }
                    }
                } else {
                    let mut heights = self.grid_row_min_heights(cells, symmetrical, false);
                    self.apply_heights(cells, rect, padding, gap, &mut heights);
                }
            }
        }
    }

    /// Applies a position and an extent to a child, if there is one.
    fn apply(&mut self, child: Option<CellId>, axis: Axis, pos: f64, len: f64) {
        if let Some(child) = child {
            axis.set(&mut self.cells[child.0].rect, pos, len);
        }
    }

    /// `GridContainerCell.applyWidthsToRow`: the three columns, each of which reaches every row.
    fn apply_widths(
        &mut self,
        cells: [[Option<CellId>; 3]; 3],
        rect: Rect,
        padding: Spacing,
        gap: f64,
        widths: &mut [f64; 3],
    ) {
        self.apply_column(cells, Area::Begin, rect.x + padding.left, widths);
        self.apply_column(
            cells,
            Area::End,
            rect.x + rect.width - padding.right - widths[2],
            widths,
        );
        let mut free = rect.width - padding.left - padding.right;
        if widths[0] > 0.0 {
            widths[0] += gap;
            free -= widths[0];
        }
        if widths[2] > 0.0 {
            widths[2] += gap;
            free -= widths[2];
        }
        widths[1] = java_max(widths[1], free);
        self.apply_column(
            cells,
            Area::Center,
            rect.x + padding.left + widths[0] - (widths[1] - free) / 2.0,
            widths,
        );
    }

    /// `GridContainerCell.applyWidthToColumn`.
    fn apply_column(
        &mut self,
        cells: [[Option<CellId>; 3]; 3],
        column: Area,
        pos: f64,
        widths: &[f64; 3],
    ) {
        for row in AREAS {
            self.apply(
                cells[row.index()][column.index()],
                Axis::X,
                pos,
                widths[column.index()],
            );
        }
    }

    /// `GridContainerCell.layoutChildrenVertically`, for the rows.
    fn apply_heights(
        &mut self,
        cells: [[Option<CellId>; 3]; 3],
        rect: Rect,
        padding: Spacing,
        gap: f64,
        heights: &mut [f64; 3],
    ) {
        self.apply_row(cells, Area::Begin, rect.y + padding.top, heights);
        self.apply_row(
            cells,
            Area::End,
            rect.y + rect.height - padding.bottom - heights[2],
            heights,
        );
        let mut free = rect.height - padding.top - padding.bottom;
        if heights[0] > 0.0 {
            heights[0] += gap;
            free -= heights[0];
        }
        if heights[2] > 0.0 {
            heights[2] += gap;
            free -= heights[2];
        }
        heights[1] = java_max(heights[1], free);
        self.apply_row(
            cells,
            Area::Center,
            rect.y + padding.top + heights[0] - (heights[1] - free) / 2.0,
            heights,
        );
    }

    /// `GridContainerCell.applyHeightToRow`.
    fn apply_row(
        &mut self,
        cells: [[Option<CellId>; 3]; 3],
        row: Area,
        pos: f64,
        heights: &[f64; 3],
    ) {
        for column in AREAS {
            self.apply(
                cells[row.index()][column.index()],
                Axis::Y,
                pos,
                heights[row.index()],
            );
        }
    }
}
