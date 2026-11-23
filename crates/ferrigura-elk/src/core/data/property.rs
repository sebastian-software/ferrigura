//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.graph/src/org/eclipse/elk/graph/properties/MapPropertyHolder.java
//!
//! Property holders, `org.eclipse.elk.graph.properties.MapPropertyHolder`.
//!
//! A holder stores only values that were set. Reading an unset property gives
//! the option's default: the layered `supports` value for options that target
//! parents, else the melk default, where the port can read it.
//! Setting `None` removes the value, as `setProperty(property, null)` does.
//! Options are named by [`OptionId`] constants, so an id that is not
//! registered does not compile.

use crate::core::data::{JavaEnum, OptionId, OptionKind, OptionValue, option_default};

/// The values set on one element, sorted by option.
///
/// An element sets few options and the layout reads them often, so the values
/// are a vector searched by binary search. [`OptionId`] orders like the option
/// ids, so the values are in the order of Java's string keys.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PropertyHolder {
    values: Vec<(OptionId, OptionValue)>,
}

impl PropertyHolder {
    /// An empty holder.
    pub fn new() -> Self {
        Self::default()
    }

    /// The position of `id` in `values`, or where it would be inserted.
    fn position(&self, id: OptionId) -> Result<usize, usize> {
        self.values.binary_search_by_key(&id, |(key, _)| *key)
    }

    /// `setProperty(id, value)`; `None` removes the value.
    pub fn set(&mut self, id: OptionId, value: Option<OptionValue>) {
        match (self.position(id), value) {
            (Ok(index), Some(value)) => self.values[index].1 = value,
            (Err(index), Some(value)) => self.values.insert(index, (id, value)),
            (Ok(index), None) => {
                self.values.remove(index);
            }
            (Err(_), None) => {}
        }
    }

    /// `hasProperty(id)`: whether a value was set (defaults do not count).
    pub fn has(&self, id: OptionId) -> bool {
        self.position(id).is_ok()
    }

    /// The value set for `id`, without defaults.
    pub fn get(&self, id: OptionId) -> Option<&OptionValue> {
        self.position(id).ok().map(|index| &self.values[index].1)
    }

    /// `getProperty(id)`: the set value, else the melk default, else nothing.
    pub fn value_or_default(&self, id: OptionId) -> Option<&OptionValue> {
        self.get(id).or_else(|| option_default(id))
    }

    /// `getProperty` for a boolean option; `false` when there is no value.
    pub fn bool_of(&self, id: OptionId) -> bool {
        matches!(self.value_or_default(id), Some(OptionValue::Boolean(true)))
    }

    /// `getProperty` for an int option.
    pub fn int_of(&self, id: OptionId) -> Option<i32> {
        match self.value_or_default(id) {
            Some(&OptionValue::Int(value)) => Some(value),
            _ => None,
        }
    }

    /// `getProperty` for a double option.
    pub fn double_of(&self, id: OptionId) -> Option<f64> {
        match self.value_or_default(id) {
            Some(&OptionValue::Double(value)) => Some(value),
            Some(&OptionValue::Int(value)) => Some(f64::from(value)),
            _ => None,
        }
    }

    /// `getProperty` for an enum option, read through its Java constant name.
    pub fn enum_of<E: JavaEnum>(&self, id: OptionId) -> Option<E> {
        match self.value_or_default(id) {
            Some(OptionValue::Enum { name, .. }) => E::from_java_text(name),
            _ => None,
        }
    }

    /// `getProperty` for an int option that always has a default. Panics for an
    /// option without one: that is a gap in the registry, not in the input.
    pub fn int_with_default(&self, id: OptionId) -> i32 {
        self.int_of(id)
            .unwrap_or_else(|| panic!("option {id} has no default"))
    }

    /// `getProperty` for a double option that always has a default; see
    /// [`Self::int_with_default`].
    pub fn double_with_default(&self, id: OptionId) -> f64 {
        self.double_of(id)
            .unwrap_or_else(|| panic!("option {id} has no default"))
    }

    /// `getProperty` for an enum option that always has a default; see
    /// [`Self::int_with_default`].
    pub fn enum_with_default<E: JavaEnum>(&self, id: OptionId) -> E {
        self.enum_of::<E>(id)
            .unwrap_or_else(|| panic!("option {id} has no default"))
    }

    /// `EnumSet.contains` for an `EnumSet` option; false when there is no value.
    pub fn enum_set_contains(&self, id: OptionId, name: &str) -> bool {
        matches!(
            self.value_or_default(id),
            Some(OptionValue::EnumSet { names, .. }) if names.iter().any(|found| found == name)
        )
    }

    /// `EnumSet.isEmpty` for an `EnumSet` option; true when there is no value.
    pub fn enum_set_is_empty(&self, id: OptionId) -> bool {
        match self.value_or_default(id) {
            Some(OptionValue::EnumSet { names, .. }) => names.is_empty(),
            _ => true,
        }
    }

    /// Whether the option's kind is one this holder can answer for, for tests.
    pub fn kind_of(id: OptionId) -> OptionKind {
        id.spec().kind
    }

    /// All values set on this holder, for copying (`copyProperties`).
    pub fn copy_from(&mut self, other: &PropertyHolder) {
        for (id, value) in &other.values {
            self.set(*id, Some(value.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::enums::Direction;
    use crate::core::data::option_by_id;

    #[test]
    fn unset_values_fall_back_to_the_melk_default() {
        let holder = PropertyHolder::new();
        assert!(!holder.has(OptionId::of("org.eclipse.elk.layered.feedbackEdges")));
        assert!(!holder.bool_of(OptionId::of("org.eclipse.elk.layered.feedbackEdges")));
        assert_eq!(
            holder.enum_of::<Direction>(OptionId::of("org.eclipse.elk.direction")),
            Some(Direction::UNDEFINED)
        );
    }

    #[test]
    fn unset_parent_options_take_the_layered_value() {
        use crate::core::data::enums::{EdgeRouting, PortAlignment};
        let holder = PropertyHolder::new();
        // `edgeRouting` targets parents: Layered.melk's ORTHOGONAL replaces the melk UNDEFINED.
        assert_eq!(
            holder.enum_of::<EdgeRouting>(OptionId::of("org.eclipse.elk.edgeRouting")),
            Some(EdgeRouting::ORTHOGONAL)
        );
        assert_eq!(
            holder.int_with_default(OptionId::of("org.eclipse.elk.randomSeed")),
            1
        );
        assert_eq!(
            holder.double_with_default(OptionId::of("org.eclipse.elk.aspectRatio")),
            1.6
        );
        // `portAlignment.default` targets nodes: the layered JUSTIFIED does not apply to
        // unset values (elkjs 0.9.3 reads DISTRIBUTED), so the melk default stays.
        assert_eq!(
            holder.enum_with_default::<PortAlignment>(OptionId::of(
                "org.eclipse.elk.portAlignment.default"
            )),
            PortAlignment::DISTRIBUTED
        );
    }

    #[test]
    fn every_layered_parent_default_parses() {
        use crate::core::data::registry::LAYERED_PARENT_DEFAULTS;
        for (id, _) in LAYERED_PARENT_DEFAULTS {
            let option = option_by_id(id).expect("registered option");
            assert!(
                option_default(option).is_some(),
                "layered default of {id} does not parse"
            );
        }
    }

    #[test]
    fn set_values_win_and_none_removes_them() {
        let mut holder = PropertyHolder::new();
        holder.set(
            OptionId::of("org.eclipse.elk.layered.feedbackEdges"),
            Some(OptionValue::Boolean(true)),
        );
        assert!(holder.bool_of(OptionId::of("org.eclipse.elk.layered.feedbackEdges")));
        holder.set(OptionId::of("org.eclipse.elk.layered.feedbackEdges"), None);
        assert!(!holder.has(OptionId::of("org.eclipse.elk.layered.feedbackEdges")));
    }

    #[test]
    fn values_stay_in_id_order() {
        let late = OptionId::of("org.eclipse.elk.spacing.nodeNode");
        let early = OptionId::of("org.eclipse.elk.direction");
        let mut source = PropertyHolder::new();
        source.set(late, Some(OptionValue::Double(1.0)));
        source.set(early, Some(OptionValue::Int(2)));
        let mut target = PropertyHolder::new();
        target.set(late, Some(OptionValue::Double(3.0)));
        target.copy_from(&source);
        let ids: Vec<&str> = target.values.iter().map(|(id, _)| id.name()).collect();
        assert_eq!(
            ids,
            [
                "org.eclipse.elk.direction",
                "org.eclipse.elk.spacing.nodeNode"
            ]
        );
        assert_eq!(target.get(late), Some(&OptionValue::Double(1.0)));
    }
}
