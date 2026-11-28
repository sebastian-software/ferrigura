// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported from D2 TALA v0.10.0 at b463ea5d47272f4da6e1e08012dbb15a6a4dbcde.
// See internal/routing/priority_queue.go.

use crate::Error;

/// Identifies a queue entry until the queue is reset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryId(usize);

/// The routing state removed by a successful queue pop.
#[derive(Clone, Copy, Debug)]
pub struct Popped {
    pub id: EntryId,
    pub node: usize,
    pub horizontal: bool,
    pub priority: f64,
}

#[derive(Clone, Copy)]
struct Entry {
    node: usize,
    horizontal: bool,
    priority: f64,
    index: Option<usize>,
}

/// An indexed min-heap for visibility search. Equal priorities leave in
/// insertion order, including after decrease. Work is charged before mutation.
#[derive(Default)]
pub struct PriorityQueue {
    items: Vec<EntryId>,
    entries: Vec<Entry>,
}

impl PriorityQueue {
    pub fn reset(&mut self) {
        self.items.clear();
        self.entries.clear();
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Adds one routing state and returns its stable handle.
    ///
    /// # Errors
    /// Returns a work limit if the heap insertion cannot be reserved.
    pub fn push(
        &mut self,
        node: usize,
        horizontal: bool,
        priority: f64,
        work: &mut usize,
    ) -> Result<EntryId, Error> {
        let index = self.items.len();
        reserve(work, index + 1, 1)?;
        let id = EntryId(self.entries.len());
        self.entries.push(Entry {
            node,
            horizontal,
            priority,
            index: Some(index),
        });
        self.items.push(id);
        self.sift_up(index);
        Ok(id)
    }

    /// Removes the minimum routing state.
    ///
    /// # Errors
    /// Returns an invalid graph error for an empty queue or a work limit
    /// if the removal cannot be reserved.
    pub fn pop(&mut self, work: &mut usize) -> Result<Popped, Error> {
        let &first = self.items.first().ok_or(Error::InvalidGraph)?;
        reserve(work, self.items.len(), 2)?;
        let last = self.items.pop().unwrap();
        let entry = &mut self.entries[first.0];
        entry.index = None;
        let state = Popped {
            id: first,
            node: entry.node,
            horizontal: entry.horizontal,
            priority: entry.priority,
        };
        if !self.items.is_empty() {
            self.place(0, last);
            self.sift_down(0);
        }
        Ok(state)
    }

    /// Lowers the priority of an active entry while preserving insertion order.
    ///
    /// # Errors
    /// Returns an invalid graph error if the entry has left the queue or
    /// the priority is not lower, or a work limit if the move cannot be reserved.
    pub fn decrease(&mut self, id: EntryId, priority: f64, work: &mut usize) -> Result<(), Error> {
        let entry = self.entries.get(id.0).ok_or(Error::InvalidGraph)?;
        let index = entry.index.ok_or(Error::InvalidGraph)?;
        if self.items.get(index) != Some(&id) || priority >= entry.priority {
            return Err(Error::InvalidGraph);
        }
        reserve(work, index + 1, 1)?;
        self.entries[id.0].priority = priority;
        self.sift_up(index);
        Ok(())
    }

    fn place(&mut self, index: usize, id: EntryId) {
        self.items[index] = id;
        self.entries[id.0].index = Some(index);
    }

    fn less(&self, a: EntryId, b: EntryId) -> bool {
        let left = self.entries[a.0].priority;
        let right = self.entries[b.0].priority;
        // Preserve Go's exact comparison with NaN. It does not fall through
        // to the insertion-order tie-break unless priorities compare equal.
        if left != right {
            left < right
        } else {
            a.0 < b.0
        }
    }

    fn sift_up(&mut self, mut index: usize) {
        let item = self.items[index];
        while index > 0 {
            let parent = (index - 1) / 2;
            let next = self.items[parent];
            if !self.less(item, next) {
                break;
            }
            self.place(index, next);
            index = parent;
        }
        self.place(index, item);
    }

    fn sift_down(&mut self, mut index: usize) {
        let item = self.items[index];
        loop {
            let left = index * 2 + 1;
            let Some(&first) = self.items.get(left) else {
                break;
            };
            let right = left + 1;
            let child = if self
                .items
                .get(right)
                .is_some_and(|&next| self.less(next, first))
            {
                right
            } else {
                left
            };
            let next = self.items[child];
            if !self.less(next, item) {
                break;
            }
            self.place(index, next);
            index = child;
        }
        self.place(index, item);
    }
}

fn reserve(work: &mut usize, upper: usize, units: usize) -> Result<(), Error> {
    let levels = (usize::BITS - upper.leading_zeros()) as usize;
    *work = work.checked_sub(levels * units).ok_or(Error::WorkLimit)?;
    Ok(())
}

#[cfg(test)]
mod tests;
