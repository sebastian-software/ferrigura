//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/overlaps/RectangleStripOverlapRemover.java
//!   and GreedyRectangleStripOverlapRemover.java
//!
//! Moves rectangles that lie in a strip apart, so that none overlaps another. The rectangles are
//! ordered along the strip, the ones that overlap along it are found with a scan line, and the
//! greedy strategy stacks each one below the overlapping rectangles already placed.

use crate::core::math::ElkRectangle;
use crate::java::sort;

/// The default gap of `RectangleStripOverlapRemover`.
const DEFAULT_GAP: f64 = 5.0;

/// `RectangleStripOverlapRemover.OverlapRemovalDirection`: the direction the rectangles are moved in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlapRemovalDirection {
    Up,
    Down,
    Left,
    Right,
}

/// `RectangleStripOverlapRemover`, with the greedy strategy, which is its default. The greedy
/// strategy only uses the vertical gap, so the horizontal one is not kept.
#[derive(Debug, Clone)]
pub struct RectangleStripOverlapRemover {
    direction: OverlapRemovalDirection,
    gap_vertical: f64,
    start_coordinate: f64,
    /// The rectangles as they were added.
    rectangles: Vec<ElkRectangle>,
}

impl RectangleStripOverlapRemover {
    /// `RectangleStripOverlapRemover.createForDirection`.
    pub fn create_for_direction(direction: OverlapRemovalDirection) -> Self {
        RectangleStripOverlapRemover {
            direction,
            gap_vertical: DEFAULT_GAP,
            start_coordinate: 0.0,
            rectangles: Vec::new(),
        }
    }

    /// `withGap(horizontal, vertical)`; only the vertical gap takes part in the greedy placement.
    pub fn with_gap(mut self, _horizontal: f64, vertical: f64) -> Self {
        self.gap_vertical = vertical;
        self
    }

    /// `withStartCoordinate`: where the strip starts, along the direction.
    pub fn with_start_coordinate(mut self, coordinate: f64) -> Self {
        self.start_coordinate = coordinate;
        self
    }

    /// `addRectangle`.
    pub fn add_rectangle(&mut self, rectangle: ElkRectangle) {
        self.rectangles.push(rectangle);
    }

    /// `removeOverlaps`: returns the rectangles in the order they were added, moved apart.
    pub fn remove_overlaps(self) -> Vec<ElkRectangle> {
        let count = self.rectangles.len();
        // `importRectangle`: the strip runs along x, so left and right are turned to put it there.
        let mut placed: Vec<ElkRectangle> = self
            .rectangles
            .iter()
            .map(|rectangle| self.import(rectangle))
            .collect();

        // The nodes sorted by their left border, the order the greedy strategy works in.
        let mut order: Vec<usize> = (0..count).collect();
        sort::sort_by(&mut order, |&a, &b| placed[a].x.total_cmp(&placed[b].x));

        let overlapping = compute_overlaps(&order, &placed);
        let mut overlapping = overlapping;
        greedy_placement(&order, &mut placed, &mut overlapping, self.gap_vertical);

        self.rectangles
            .iter()
            .zip(&placed)
            .map(|(original, rectangle)| self.export(*original, *rectangle))
            .collect()
    }

    /// `importRectangle`.
    fn import(&self, rectangle: &ElkRectangle) -> ElkRectangle {
        match self.direction {
            OverlapRemovalDirection::Up | OverlapRemovalDirection::Down => *rectangle,
            OverlapRemovalDirection::Left | OverlapRemovalDirection::Right => {
                ElkRectangle::new(rectangle.y, 0.0, rectangle.height, rectangle.width)
            }
        }
    }

    /// `exportRectangle`: the original rectangle, moved to where the strip put its counterpart.
    /// For up and down the counterpart is the original itself, so its y is the placed one.
    fn export(&self, original: ElkRectangle, rectangle: ElkRectangle) -> ElkRectangle {
        let mut result = original;
        match self.direction {
            OverlapRemovalDirection::Up => {
                result.y = self.start_coordinate - rectangle.height - rectangle.y;
            }
            OverlapRemovalDirection::Down => {
                result.y = rectangle.y + self.start_coordinate;
            }
            OverlapRemovalDirection::Left => {
                result.x = self.start_coordinate - rectangle.height - rectangle.y;
            }
            OverlapRemovalDirection::Right => {
                result.x = self.start_coordinate + rectangle.y;
            }
        }
        result
    }
}

/// `computeOverlaps`: for each rectangle, the rectangles it overlaps along the strip. A scan line
/// moves along the left borders; a tree keeps the rectangles still reaching it, ordered by their
/// right borders. As in Java's `TreeSet`, a rectangle whose right border equals one already kept
/// is not added.
fn compute_overlaps(order: &[usize], placed: &[ElkRectangle]) -> Vec<Vec<usize>> {
    let right = |index: usize| placed[index].x + placed[index].width;
    let mut overlapping = vec![Vec::new(); placed.len()];
    let mut intersecting: Vec<usize> = Vec::new();
    for &current in order {
        let scanline = placed[current].x;
        while let Some(&first) = intersecting.first() {
            if placed[first].x + placed[first].width < scanline {
                intersecting.remove(0);
            } else {
                break;
            }
        }
        for &other in &intersecting {
            overlapping[other].push(current);
            overlapping[current].push(other);
        }
        let key = right(current);
        if let Err(position) = intersecting.binary_search_by(|&other| right(other).total_cmp(&key))
        {
            intersecting.insert(position, current);
        }
    }
    overlapping
}

/// `GreedyRectangleStripOverlapRemover.removeOverlaps`: each rectangle, in order, is moved below
/// the overlapping rectangles that are already placed.
fn greedy_placement(
    order: &[usize],
    placed: &mut [ElkRectangle],
    overlapping: &mut [Vec<usize>],
    vertical_gap: f64,
) {
    let mut already_placed = vec![false; placed.len()];
    for &current in order {
        let mut y_pos = 0.0;
        sort::sort_by(&mut overlapping[current], |&a, &b| {
            placed[a].y.total_cmp(&placed[b].y)
        });
        for &other in &overlapping[current] {
            if already_placed[other] {
                let (current_rect, other_rect) = (placed[current], placed[other]);
                if y_pos < other_rect.y + other_rect.height + vertical_gap
                    && y_pos + current_rect.height + vertical_gap > other_rect.y
                {
                    y_pos = other_rect.y + other_rect.height + vertical_gap;
                }
            }
        }
        placed[current].y = y_pos;
        already_placed[current] = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_in_a_column_are_stacked_below_each_other() {
        let mut remover =
            RectangleStripOverlapRemover::create_for_direction(OverlapRemovalDirection::Up)
                .with_gap(5.0, 5.0)
                .with_start_coordinate(100.0);
        remover.add_rectangle(ElkRectangle::new(0.0, 0.0, 10.0, 10.0));
        remover.add_rectangle(ElkRectangle::new(2.0, 3.0, 10.0, 10.0));
        let moved = remover.remove_overlaps();
        // The first rectangle is placed at the strip's start, the second below it with the gap.
        assert_eq!(moved[0].y, 100.0 - 10.0 - 0.0);
        assert_eq!(moved[1].y, 100.0 - 10.0 - 15.0);
    }

    #[test]
    fn rectangles_that_overlap_nothing_go_to_the_start_of_the_strip() {
        let mut remover =
            RectangleStripOverlapRemover::create_for_direction(OverlapRemovalDirection::Down)
                .with_start_coordinate(7.0);
        remover.add_rectangle(ElkRectangle::new(0.0, 0.0, 10.0, 10.0));
        remover.add_rectangle(ElkRectangle::new(50.0, 4.0, 10.0, 10.0));
        let moved = remover.remove_overlaps();
        assert_eq!(moved[0].y, 7.0);
        assert_eq!(moved[1].y, 7.0);
    }
}
