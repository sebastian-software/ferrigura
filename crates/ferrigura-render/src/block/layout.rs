//! The layout of the blocks: a port of `diagrams/block/layout.ts`. The layout is the type's own
//! arithmetic, with no graph engine: the sizes come bottom up (`setBlockSizes`), the positions top down
//! (`layoutBlocks`), and the bounds are the extremes of the blocks (`findBounds`).
//!
//! The sizes the measure step gives the leaves are the input (`Block::size`); the arithmetic reads and
//! writes the sizes in the reference's order, since a later step reads what an earlier one wrote.
//! `docs-port/block/NOTES.md` section 6.4 has the steps.

use std::collections::BTreeMap;

use super::{Block, Size};
use crate::RenderError;

/// The bounds of the blocks, as `findBounds` and `layout` return them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// The left edge.
    pub x: f64,
    /// The top edge.
    pub y: f64,
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

/// JavaScript's truthiness for a number: not zero and not NaN.
fn truthy(value: f64) -> bool {
    value != 0.0 && !value.is_nan()
}

/// `block.columns ?? -1`.
fn columns_of(block: &Block) -> f64 {
    block.columns.unwrap_or(-1.0)
}

/// `child.widthInColumns ?? 1`.
fn span_of(block: &Block) -> f64 {
    block.width_in_columns.unwrap_or(1.0)
}

/// `calculateBlockPosition` (`layout.ts` 11-36): the row and the column of a block at `position`
/// in a grid of `columns`. Columns of `-1` (auto) lay the blocks in one row, and one column lays them
/// in one column.
///
/// # Errors
///
/// Returns an error when `columns` is zero or not an integer, as the reference throws
/// (`Columns must be an integer !== 0.`).
pub fn calculate_block_position(columns: f64, position: f64) -> Result<(f64, f64), RenderError> {
    if columns == 0.0 || columns.fract() != 0.0 || !columns.is_finite() {
        return Err(RenderError::Layout(
            "Columns must be an integer !== 0.".to_owned(),
        ));
    }
    if position < 0.0 || position.fract() != 0.0 {
        return Err(RenderError::Layout(format!(
            "Position must be a non-negative integer.{position}"
        )));
    }
    if columns < 0.0 {
        return Ok((position, 0.0));
    }
    if columns == 1.0 {
        return Ok((0.0, position));
    }
    Ok((position % columns, (position / columns).floor()))
}

/// `getMaxChildSize` (`layout.ts` 58-87): the largest width per column and the largest height of the
/// children, spaces left out. A child's width is spread over its span.
fn max_child_size(block: &Block) -> (f64, f64) {
    let mut max_width = 0.0;
    let mut max_height = 0.0;
    for child in &block.children {
        if child.is_space() {
            continue;
        }
        let size = child.size.unwrap_or_default();
        let normalised = size.width / span_of(child);
        if normalised > max_width {
            max_width = normalised;
        }
        if size.height > max_height {
            max_height = size.height;
        }
    }
    (max_width, max_height)
}

/// `setBlockSizes` (`layout.ts` 89-207): the sizes of a block and of its children, from the bottom up.
/// `sibling_width` and `sibling_height` are the largest sizes of the block's siblings, which a block
/// with no size of its own takes, and to which a smaller composite grows.
pub fn set_block_sizes(block: &mut Block, sibling_width: f64, sibling_height: f64, padding: f64) {
    if !block.size.is_some_and(|size| truthy(size.width)) {
        block.size = Some(Size {
            width: sibling_width,
            height: sibling_height,
            x: 0.0,
            y: 0.0,
        });
    }
    if block.children.is_empty() {
        return;
    }
    for child in &mut block.children {
        set_block_sizes(child, 0.0, 0.0, padding);
    }
    let (max_width, max_height) = max_child_size(block);
    for child in &mut block.children {
        let span = span_of(child);
        if let Some(size) = child.size.as_mut() {
            size.width = max_width * span + padding * (span - 1.0);
            size.height = max_height;
            size.x = 0.0;
            size.y = 0.0;
        }
    }
    for child in &mut block.children {
        set_block_sizes(child, max_width, max_height, padding);
    }

    let columns = columns_of(block);
    let num_items: f64 = block.children.iter().map(span_of).sum();
    let child_count = block.children.len() as f64;
    let x_size = if columns > 0.0 && columns < num_items {
        columns
    } else {
        child_count
    };
    let y_size = (num_items / x_size).ceil();

    let mut width = x_size * (max_width + padding) + padding;
    let mut height = y_size * (max_height + padding) + padding;
    if width < sibling_width {
        width = sibling_width;
        height = sibling_height;
        let child_width = (sibling_width - x_size * padding - padding) / x_size;
        let child_height = (sibling_height - y_size * padding - padding) / y_size;
        for child in &mut block.children {
            if let Some(size) = child.size.as_mut() {
                size.width = child_width;
                size.height = child_height;
                size.x = 0.0;
                size.y = 0.0;
            }
        }
    }

    // The block's own width, as the sizes above left it: a wider composite grows its children.
    let assigned = block.size.map_or(0.0, |size| size.width);
    let assigned = if truthy(assigned) { assigned } else { 0.0 };
    if width < assigned {
        width = assigned;
        let num = if columns > 0.0 {
            child_count.min(columns)
        } else {
            child_count
        };
        if num > 0.0 {
            let child_width = (width - num * padding - padding) / num;
            for child in &mut block.children {
                if let Some(size) = child.size.as_mut() {
                    size.width = child_width;
                }
            }
        }
    }
    block.size = Some(Size {
        width,
        height,
        x: 0.0,
        y: 0.0,
    });
}

/// The x of the first block of a row: the left edge of a composite that has a centre, else the
/// root's `-padding` (`layoutBlocks`, `startingPosX`). A centre of exactly zero counts as no centre,
/// as the reference's truthiness test reads it (`layout.ts` 257).
fn row_start(block: &Block, padding: f64) -> f64 {
    match block.size {
        Some(size) if truthy(size.x) => {
            let half = -size.width / 2.0;
            size.x + if truthy(half) { half } else { 0.0 }
        }
        _ => -padding,
    }
}

/// `layoutBlocks` (`layout.ts` 209-322): the centres of the children of a block, row by row. Each row is
/// as high as its tallest block; a composite's children are placed in turn, from the top down.
fn layout_blocks(block: &mut Block, padding: f64) -> Result<(), RenderError> {
    if block.children.is_empty() {
        return Ok(());
    }
    let columns = columns_of(block);

    // The height of each row: the tallest child in it.
    let mut row_heights: BTreeMap<u64, f64> = BTreeMap::new();
    let mut column_position = 0.0;
    for child in &block.children {
        let Some(size) = child.size else {
            continue;
        };
        let (_, row) = calculate_block_position(columns, column_position)?;
        let row = row as u64;
        let current = row_heights.get(&row).copied().unwrap_or(0.0);
        if size.height > current {
            row_heights.insert(row, size.height);
        }
        let mut filled = span_of(child);
        if columns > 0.0 {
            filled = filled.min(columns - (column_position % columns));
        }
        column_position += filled;
    }
    // The y of each row: the heights and the paddings of the rows above it.
    let mut row_offsets: BTreeMap<u64, f64> = BTreeMap::new();
    let mut offset = 0.0;
    for (row, height) in &row_heights {
        row_offsets.insert(*row, offset);
        offset += height + padding;
    }

    let parent = block.size.unwrap_or_default();
    // The parent's own position does not change while its children are placed.
    let first_x = row_start(block, padding);
    let mut column_position = 0.0;
    let mut starting_x = first_x;
    let mut row_position = 0_u64;
    for child in &mut block.children {
        let Some(size) = child.size else {
            continue;
        };
        let (_, row) = calculate_block_position(columns, column_position)?;
        let row = row as u64;
        if row != row_position {
            row_position = row;
            starting_x = first_x;
        }
        let half_width = size.width / 2.0;
        let x = starting_x + padding + half_width;
        starting_x = x + half_width;
        let row_offset = row_offsets.get(&row).copied().unwrap_or(0.0);
        let row_height = row_heights.get(&row).copied().unwrap_or(size.height);
        let y = parent.y - parent.height / 2.0 + row_offset + row_height / 2.0 + padding;
        if let Some(child_size) = child.size.as_mut() {
            child_size.x = x;
            child_size.y = y;
        }
        layout_blocks(child, padding)?;
        let mut filled = span_of(child);
        if columns > 0.0 {
            filled = filled.min(columns - (column_position % columns));
        }
        column_position += filled;
    }
    Ok(())
}

/// `findBounds` (`layout.ts` 324-349): the extremes of the blocks below `block`, starting from the
/// origin, the root itself left out.
fn find_bounds(block: &Block, bounds: &mut (f64, f64, f64, f64)) {
    if let Some(size) = block.size.filter(|_| block.id != "root") {
        let (min_x, min_y, max_x, max_y) = bounds;
        let left = size.x - size.width / 2.0;
        let top = size.y - size.height / 2.0;
        let right = size.x + size.width / 2.0;
        let bottom = size.y + size.height / 2.0;
        if left < *min_x {
            *min_x = left;
        }
        if top < *min_y {
            *min_y = top;
        }
        if right > *max_x {
            *max_x = right;
        }
        if bottom > *max_y {
            *max_y = bottom;
        }
    }
    for child in &block.children {
        find_bounds(child, bounds);
    }
}

/// `layout` (`layout.ts` 351-369): sizes the root and places its blocks with the padding the caller
/// reads from the configuration. Returns the bounds of the blocks, which the renderer uses only when
/// the drawn bounds are not finite.
///
/// # Errors
///
/// Returns an error for a grid of zero columns.
pub fn layout(root: &mut Block, padding: f64) -> Result<Bounds, RenderError> {
    set_block_sizes(root, 0.0, 0.0, padding);
    layout_blocks(root, padding)?;
    let mut bounds = (0.0, 0.0, 0.0, 0.0);
    find_bounds(root, &mut bounds);
    let (min_x, min_y, max_x, max_y) = bounds;
    Ok(Bounds {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    })
}
