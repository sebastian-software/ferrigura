//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/alg/LayoutProcessorConfiguration.java
//! Source: plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/alg/AlgorithmAssembler.java
//!
//! Processor configuration of ELK core, `org.eclipse.elk.core.alg`.
//!
//! [`LayoutProcessorConfiguration`] keeps one set of processors per slot: slot
//! `i` is "before phase `i`", and slot `phases` is "after the last phase".
//! Each set is ordered by the enum ordinal of its processors, which is what
//! `EnumBasedFactoryComparator` sorts by, so the set's own order never matters.

use std::collections::BTreeSet;

/// `LayoutProcessorConfiguration<P, G>`, with `F` the processor factory enum.
#[derive(Debug, Clone)]
pub struct LayoutProcessorConfiguration<F: Ord + Copy> {
    slots: Vec<BTreeSet<F>>,
    current: Option<usize>,
}

impl<F: Ord + Copy> LayoutProcessorConfiguration<F> {
    /// `create()` for a configuration over `phases` phases.
    pub fn new(phases: usize) -> Self {
        LayoutProcessorConfiguration {
            slots: vec![BTreeSet::new(); phases + 1],
            current: None,
        }
    }

    /// `before(phase)`: the next `add` goes before the phase with this ordinal.
    pub fn before(&mut self, phase: usize) -> &mut Self {
        self.current = Some(phase);
        self
    }

    /// `after(phase)`: the next `add` goes after the phase with this ordinal.
    pub fn after(&mut self, phase: usize) -> &mut Self {
        self.current = Some(phase + 1);
        self
    }

    /// `add(processor)` at the position set by `before` or `after`.
    ///
    /// # Panics
    ///
    /// Panics when no position was set, as the Java code throws.
    pub fn add(&mut self, processor: F) -> &mut Self {
        let index = self
            .current
            .expect("Did not call before(...) or after(...) before calling add(...).");
        self.slots[index].insert(processor);
        self
    }

    /// `addBefore(phase, processor)`.
    pub fn add_before(&mut self, phase: usize, processor: F) -> &mut Self {
        self.current = None;
        self.slots[phase].insert(processor);
        self
    }

    /// `addAfter(phase, processor)`.
    pub fn add_after(&mut self, phase: usize, processor: F) -> &mut Self {
        self.current = None;
        self.slots[phase + 1].insert(processor);
        self
    }

    /// `addAll(configuration)`: slot-wise union.
    pub fn add_all(&mut self, other: &LayoutProcessorConfiguration<F>) -> &mut Self {
        for (slot, source) in self.slots.iter_mut().zip(&other.slots) {
            slot.extend(source.iter().copied());
        }
        self
    }

    /// `processorsBefore(phase)`: the processors of slot `phase`, in ordinal order.
    pub fn processors_before(&self, phase: usize) -> Vec<F> {
        self.slots[phase].iter().copied().collect()
    }

    /// `processorsAfter(phase)`: the processors of slot `phase + 1`, in ordinal order.
    pub fn processors_after(&self, phase: usize) -> Vec<F> {
        self.slots[phase + 1].iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Factory {
        First,
        Second,
        Third,
    }

    #[test]
    fn slots_order_by_ordinal_not_insertion() {
        let mut config = LayoutProcessorConfiguration::new(2);
        config.add_before(0, Factory::Third);
        config.add_before(0, Factory::First);
        config.add_after(1, Factory::Second);
        assert_eq!(
            config.processors_before(0),
            vec![Factory::First, Factory::Third]
        );
        assert_eq!(config.processors_after(1), vec![Factory::Second]);
    }

    #[test]
    fn before_and_after_pick_the_slot_for_add() {
        let mut config = LayoutProcessorConfiguration::new(1);
        config.before(0).add(Factory::Second);
        config.after(0).add(Factory::Third);
        assert_eq!(config.processors_before(0), vec![Factory::Second]);
        assert_eq!(config.processors_after(0), vec![Factory::Third]);
    }
}
