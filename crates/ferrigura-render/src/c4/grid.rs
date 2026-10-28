//! The grid of the C4 layout: the `Bounds` class of `c4Renderer.ts:69-172`. A level of the diagram (the
//! screen, or the contents of a boundary) has one `Bounds`: `data` is the extent of what it has drawn,
//! `next` is the cursor where the next shape goes, and `cnt` counts the shapes of the current row.
//!
//! Two properties matter for the output. The width limit is an absolute x: a row wraps when its right
//! edge reaches it, even near the left of a boundary (`c4Renderer.ts:125`). And `cnt` is not reset
//! between the sibling boundaries of one level (`setData` does not touch it), so the shapes of the
//! siblings share one row count.

/// The extent of a box: each side is absent until something has been placed (`undefined` in the
/// reference, where `updateVal` takes the first value as it is).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Extent {
    /// The left edge.
    pub startx: Option<f64>,
    /// The right edge.
    pub stopx: Option<f64>,
    /// The top edge.
    pub starty: Option<f64>,
    /// The bottom edge.
    pub stopy: Option<f64>,
}

/// The grid of one level: the extents of what is drawn and of the cursor, the shapes in the row, and
/// the width limit at which a row wraps.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Bounds {
    /// The extent of what the level has drawn.
    pub data: Extent,
    /// The cursor of the next shape.
    pub next: Extent,
    /// The number of shapes placed in the current row (`nextData.cnt`).
    pub cnt: f64,
    /// The width limit, an absolute x (`data.widthLimit`).
    pub width_limit: f64,
}

/// `updateVal` with `Math.min`: the first value is taken as it is, then the smaller one is kept.
fn min_into(slot: &mut Option<f64>, value: f64) {
    *slot = Some(slot.map_or(value, |old| value.min(old)));
}

/// `updateVal` with `Math.max`: the first value is taken as it is, then the larger one is kept.
fn max_into(slot: &mut Option<f64>, value: f64) {
    *slot = Some(slot.map_or(value, |old| value.max(old)));
}

impl Bounds {
    /// A level with no shapes, and the width limit `width_limit`.
    #[must_use]
    pub fn new(width_limit: f64) -> Self {
        Self {
            width_limit,
            ..Self::default()
        }
    }

    /// `setData`: seeds both boxes with the same extent. The row count is not reset.
    pub fn set_data(&mut self, startx: f64, stopx: f64, starty: f64, stopy: f64) {
        self.next = Extent {
            startx: Some(startx),
            stopx: Some(stopx),
            starty: Some(starty),
            stopy: Some(stopy),
        };
        self.data = self.next;
    }

    /// `insert` (`c4Renderer.ts:113-147`): places a shape of `width` by `height` after the cursor,
    /// with a margin of `margin` around it, and returns its top-left corner. A shape that would reach the
    /// width limit, or the `shape_in_row`-th of a row, starts a new row at the left of the level plus
    /// `padding_x`.
    pub fn insert(
        &mut self,
        width: f64,
        height: f64,
        margin: f64,
        shape_in_row: f64,
        padding_x: f64,
    ) -> (f64, f64) {
        self.cnt += 1.0;
        let next_startx = self.next.startx.unwrap_or(0.0);
        let next_stopx = self.next.stopx.unwrap_or(0.0);
        let mut startx = if next_startx == next_stopx {
            next_stopx + margin
        } else {
            next_stopx + margin * 2.0
        };
        let mut stopx = startx + width;
        let mut starty = self.next.starty.unwrap_or(0.0) + margin * 2.0;
        let mut stopy = starty + height;
        if startx >= self.width_limit || stopx >= self.width_limit || self.cnt > shape_in_row {
            startx = next_startx + margin + padding_x;
            starty = self.next.stopy.unwrap_or(0.0) + margin * 2.0;
            stopx = startx + width;
            self.next.stopx = Some(stopx);
            self.next.starty = self.next.stopy;
            stopy = starty + height;
            self.next.stopy = Some(stopy);
            self.cnt = 1.0;
        }
        min_into(&mut self.data.startx, startx);
        min_into(&mut self.data.starty, starty);
        max_into(&mut self.data.stopx, stopx);
        max_into(&mut self.data.stopy, stopy);
        min_into(&mut self.next.startx, startx);
        min_into(&mut self.next.starty, starty);
        max_into(&mut self.next.stopx, stopx);
        max_into(&mut self.next.stopy, stopy);
        (startx, starty)
    }

    /// `bumpLastMargin`: the extent grows by `margin` on the right and at the bottom.
    pub fn bump_last_margin(&mut self, margin: f64) {
        self.data.stopx = Some(self.data.stopx.unwrap_or(0.0) + margin);
        self.data.stopy = Some(self.data.stopy.unwrap_or(0.0) + margin);
    }
}
