//! The bounds of the diagram while it is laid out: the `bounds` object of `sequenceRenderer.ts`
//! (`:23-238`), with its vertical position, its stack of open blocks and its open activations.
//!
//! The update rule grows every open block and activation by its nesting depth times the box
//! margin, so that the outermost one has the largest margin. The reference counts the depth with
//! one counter shared by the blocks and the activations (`cnt` in `updateBounds`), so an
//! activation can get a depth of zero or less. That is reproduced: it is part of the geometry.

use super::model::{Activation, Block, Extent, Section};

/// JavaScript's `Math.min` of two numbers: a NaN on either side gives NaN.
#[must_use]
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// JavaScript's `Math.max` of two numbers: a NaN on either side gives NaN.
#[must_use]
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// `Math.round`: a tie goes up, towards positive infinity, so `-2.5` becomes `-2`.
#[must_use]
pub fn js_round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor >= 0.5 { floor + 1.0 } else { floor }
}

/// `common.getMax` (`common.ts:168`): the largest of the values that are not NaN, or negative
/// infinity when none is left. An `undefined` edge is NaN there, so it is left out.
#[must_use]
pub fn get_max(values: &[Option<f64>]) -> f64 {
    values
        .iter()
        .flatten()
        .filter(|value| !value.is_nan())
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(*value))
}

/// `common.getMin` (`common.ts:181`), with the same treatment of NaN.
#[must_use]
pub fn get_min(values: &[Option<f64>]) -> f64 {
    values
        .iter()
        .flatten()
        .filter(|value| !value.is_nan())
        .fold(f64::INFINITY, |acc, value| acc.min(*value))
}

/// `updateVal(obj, key, val, fun)`: an edge that is not set takes the value, and one that is set
/// takes `fun(val, edge)`, with `Math.min` or `Math.max`.
fn update(edge: &mut Option<f64>, value: f64, fun: fn(f64, f64) -> f64) {
    *edge = Some(match *edge {
        None => value,
        Some(old) => fun(value, old),
    });
}

/// The bounds state (`bounds` in `sequenceRenderer.ts`).
#[derive(Debug, Clone, Default)]
pub struct Bounds {
    /// `bounds.data`: the edges of the whole diagram so far.
    pub data: Extent,
    /// `bounds.verticalPos`: the y of the next row.
    pub vertical_pos: f64,
    /// `bounds.sequenceItems`: the open blocks, innermost last.
    pub items: Vec<Block>,
    /// `bounds.activations`: the open activations.
    pub activations: Vec<Activation>,
    /// `bounds.models.boxes`: the boxes, as indices into the layout's boxes, in the order the
    /// reference adds them.
    pub boxes: Vec<usize>,
}

impl Bounds {
    /// `bounds.insert(startx, starty, stopx, stopy)`: grows the diagram and every open block and
    /// activation around the given rectangle. The corners are put in order first.
    pub fn insert(&mut self, box_margin: f64, startx: f64, starty: f64, stopx: f64, stopy: f64) {
        let (startx, stopx) = (get_min(&[Some(startx), Some(stopx)]), get_max(&[Some(startx), Some(stopx)]));
        let (starty, stopy) = (get_min(&[Some(starty), Some(stopy)]), get_max(&[Some(starty), Some(stopy)]));
        update(&mut self.data.startx, startx, js_min);
        update(&mut self.data.starty, starty, js_min);
        update(&mut self.data.stopx, stopx, js_max);
        update(&mut self.data.stopy, stopy, js_max);
        self.update_bounds(box_margin, startx, starty, stopx, stopy);
    }

    /// `updateBounds(startx, starty, stopx, stopy)` (`:57-80`). The blocks are visited from the
    /// outermost, then the activations, with one counter for both.
    fn update_bounds(&mut self, box_margin: f64, startx: f64, starty: f64, stopx: f64, stopy: f64) {
        let mut cnt = 0_i64;
        let items_len = self.items.len() as i64;
        for index in 0..self.items.len() {
            cnt += 1;
            let n = (items_len - cnt + 1) as f64;
            let item = &mut self.items[index];
            update(&mut item.extent.starty, starty - n * box_margin, js_min);
            update(&mut item.extent.stopy, stopy + n * box_margin, js_max);
            update(&mut self.data.startx, startx - n * box_margin, js_min);
            update(&mut self.data.stopx, stopx + n * box_margin, js_max);
            update(&mut item.extent.startx, startx - n * box_margin, js_min);
            update(&mut item.extent.stopx, stopx + n * box_margin, js_max);
            update(&mut self.data.starty, starty - n * box_margin, js_min);
            update(&mut self.data.stopy, stopy + n * box_margin, js_max);
        }
        for index in 0..self.activations.len() {
            cnt += 1;
            let n = (items_len - cnt + 1) as f64;
            let activation = &mut self.activations[index];
            update(&mut activation.extent.starty, starty - n * box_margin, js_min);
            update(&mut activation.extent.stopy, stopy + n * box_margin, js_max);
            update(&mut self.data.startx, startx - n * box_margin, js_min);
            update(&mut self.data.stopx, stopx + n * box_margin, js_max);
        }
    }

    /// `bumpVerticalPos(bump)`: moves the row down and grows the diagram's bottom.
    pub fn bump(&mut self, bump: f64) {
        self.vertical_pos += bump;
        self.data.stopy = Some(get_max(&[self.data.stopy, Some(self.vertical_pos)]));
    }

    /// `newActivation`: opens an activation on the actor, at the current row, in the group
    /// `anchored`. `x` is the left edge the caller worked out from the actor and the stack.
    pub fn new_activation(&mut self, actor: &str, x: f64, activation_width: f64, anchored: usize) {
        self.activations.push(Activation {
            extent: Extent {
                startx: Some(x),
                starty: Some(self.vertical_pos + 2.0),
                stopx: Some(x + activation_width),
                stopy: None,
            },
            actor: actor.to_owned(),
            anchored,
        });
    }

    /// The number of open activations on the actor (`actorActivations(actor).length`).
    #[must_use]
    pub fn activation_count(&self, actor: &str) -> usize {
        self.activations.iter().filter(|a| a.actor == actor).count()
    }

    /// `endActivation(message)`: removes the most recent activation of the actor and returns it.
    /// The reference's `splice(-1, 1)` takes the last one when the actor has none, so that is
    /// what happens here too. `None` only when there is no activation at all.
    pub fn end_activation(&mut self, actor: &str) -> Option<Activation> {
        let index = self.activations.iter().rposition(|a| a.actor == actor);
        match index {
            Some(index) => Some(self.activations.remove(index)),
            None => {
                if self.activations.is_empty() {
                    None
                } else {
                    self.activations.pop()
                }
            }
        }
    }

    /// `createLoop` and `newLoop`: opens a block at the current row (`sequenceItems.push`).
    pub fn new_block(&mut self, title: Option<String>, wrap: bool, width: Option<f64>, fill: Option<String>) {
        self.items.push(Block {
            extent: Extent {
                startx: None,
                stopx: None,
                starty: Some(self.vertical_pos),
                stopy: None,
            },
            title,
            wrap,
            width,
            height: 0.0,
            fill,
            sections: Vec::new(),
            section_titles: Vec::new(),
        });
    }

    /// `endLoop`: closes the innermost block and returns it.
    pub fn end_block(&mut self) -> Option<Block> {
        self.items.pop()
    }

    /// `addSectionToLoop(message)`: a divider at the current row, with the section's title, on the
    /// innermost block.
    pub fn add_section(&mut self, title: String) {
        let y = self.vertical_pos;
        if let Some(block) = self.items.last_mut() {
            block.sections.push(Section { y });
            block.section_titles.push(title);
        }
    }
}

#[cfg(test)]
mod tests {
    //! `sequenceDiagram.spec.js:1783-1916`, the bounds block. The reference's `bounds.init()` is
    //! `Bounds::default()`, and its `insert` reads `boxMargin` from the configuration, which the spec
    //! sets to 10. The margin is passed here.

    use super::Bounds;

    /// `boxMargin` in the bounds block of the spec.
    const BOX_MARGIN: f64 = 10.0;

    #[test]
    fn should_handle_a_simple_bound_call() {
        // Reference: sequenceDiagram.spec.js:1812.
        let mut bounds = Bounds::default();
        bounds.insert(BOX_MARGIN, 100.0, 100.0, 200.0, 200.0);
        assert_eq!(bounds.data.startx, Some(100.0));
        assert_eq!(bounds.data.starty, Some(100.0));
        assert_eq!(bounds.data.stopx, Some(200.0));
        assert_eq!(bounds.data.stopy, Some(200.0));
    }

    #[test]
    fn should_handle_an_expanding_bound() {
        // Reference: sequenceDiagram.spec.js:1821.
        let mut bounds = Bounds::default();
        bounds.insert(BOX_MARGIN, 100.0, 100.0, 200.0, 200.0);
        bounds.insert(BOX_MARGIN, 25.0, 50.0, 300.0, 400.0);
        assert_eq!(bounds.data.startx, Some(25.0));
        assert_eq!(bounds.data.starty, Some(50.0));
        assert_eq!(bounds.data.stopx, Some(300.0));
        assert_eq!(bounds.data.stopy, Some(400.0));
    }

    #[test]
    fn should_handle_inserts_within_the_bound_without_changing_the_outer_bounds() {
        // Reference: sequenceDiagram.spec.js:1831.
        let mut bounds = Bounds::default();
        bounds.insert(BOX_MARGIN, 100.0, 100.0, 200.0, 200.0);
        bounds.insert(BOX_MARGIN, 25.0, 50.0, 300.0, 400.0);
        bounds.insert(BOX_MARGIN, 125.0, 150.0, 150.0, 200.0);
        assert_eq!(bounds.data.startx, Some(25.0));
        assert_eq!(bounds.data.starty, Some(50.0));
        assert_eq!(bounds.data.stopx, Some(300.0));
        assert_eq!(bounds.data.stopy, Some(400.0));
    }

    #[test]
    fn should_handle_a_loop_without_expanding_the_area() {
        // Reference: sequenceDiagram.spec.js:1842.
        let mut bounds = Bounds::default();
        bounds.insert(BOX_MARGIN, 25.0, 50.0, 300.0, 400.0);
        bounds.vertical_pos = 150.0;
        bounds.new_block(None, false, None, None);
        bounds.insert(BOX_MARGIN, 125.0, 150.0, 150.0, 200.0);

        let lp = bounds.end_block().expect("the loop is open");
        assert_eq!(lp.extent.startx, Some(125.0 - BOX_MARGIN));
        assert_eq!(lp.extent.starty, Some(150.0 - BOX_MARGIN));
        assert_eq!(lp.extent.stopx, Some(150.0 + BOX_MARGIN));
        assert_eq!(lp.extent.stopy, Some(200.0 + BOX_MARGIN));

        // The bounds of the first loop are unchanged.
        assert_eq!(bounds.data.startx, Some(25.0));
        assert_eq!(bounds.data.starty, Some(50.0));
        assert_eq!(bounds.data.stopx, Some(300.0));
        assert_eq!(bounds.data.stopy, Some(400.0));
    }

    #[test]
    fn should_handle_multiple_loops_without_expanding_the_bounds() {
        // Reference: sequenceDiagram.spec.js:1863.
        let mut bounds = Bounds::default();
        bounds.insert(BOX_MARGIN, 100.0, 100.0, 1000.0, 1000.0);
        bounds.vertical_pos = 200.0;
        bounds.new_block(None, false, None, None);
        bounds.new_block(None, false, None, None);
        bounds.insert(BOX_MARGIN, 200.0, 200.0, 300.0, 300.0);

        // The innermost loop is closed first.
        let inner = bounds.end_block().expect("the inner loop is open");
        assert_eq!(inner.extent.startx, Some(200.0 - BOX_MARGIN));
        assert_eq!(inner.extent.starty, Some(200.0 - BOX_MARGIN));
        assert_eq!(inner.extent.stopx, Some(300.0 + BOX_MARGIN));
        assert_eq!(inner.extent.stopy, Some(300.0 + BOX_MARGIN));

        let outer = bounds.end_block().expect("the outer loop is open");
        assert_eq!(outer.extent.startx, Some(200.0 - 2.0 * BOX_MARGIN));
        assert_eq!(outer.extent.starty, Some(200.0 - 2.0 * BOX_MARGIN));
        assert_eq!(outer.extent.stopx, Some(300.0 + 2.0 * BOX_MARGIN));
        assert_eq!(outer.extent.stopy, Some(300.0 + 2.0 * BOX_MARGIN));

        assert_eq!(bounds.data.startx, Some(100.0));
        assert_eq!(bounds.data.starty, Some(100.0));
        assert_eq!(bounds.data.stopx, Some(1000.0));
        assert_eq!(bounds.data.stopy, Some(1000.0));
    }

    #[test]
    fn should_handle_a_loop_that_expands_the_area() {
        // Reference: sequenceDiagram.spec.js:1894.
        let mut bounds = Bounds::default();
        bounds.insert(BOX_MARGIN, 100.0, 100.0, 200.0, 200.0);
        bounds.vertical_pos = 200.0;
        bounds.new_block(None, false, None, None);
        bounds.insert(BOX_MARGIN, 50.0, 50.0, 300.0, 300.0);

        let lp = bounds.end_block().expect("the loop is open");
        assert_eq!(lp.extent.startx, Some(50.0 - BOX_MARGIN));
        assert_eq!(lp.extent.starty, Some(50.0 - BOX_MARGIN));
        assert_eq!(lp.extent.stopx, Some(300.0 + BOX_MARGIN));
        assert_eq!(lp.extent.stopy, Some(300.0 + BOX_MARGIN));

        // After the loop, the bounds are the loop's edges.
        assert_eq!(bounds.data.startx, lp.extent.startx);
        assert_eq!(bounds.data.starty, lp.extent.starty);
        assert_eq!(bounds.data.stopx, lp.extent.stopx);
        assert_eq!(bounds.data.stopy, lp.extent.stopy);
    }
}
