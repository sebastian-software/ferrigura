//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.graph/src/org/eclipse/elk/graph/properties/MapPropertyHolder.java
//!
//! Property holders, `org.eclipse.elk.graph.properties.MapPropertyHolder`.
//!
//! A holder stores only values that were set. Reading an unset property gives
//! the option's default, which is the melk default where the port can read it.
//! Setting `None` removes the value, as `setProperty(property, null)` does.

use std::collections::BTreeMap;

use crate::core::data::{
    JavaEnum, OptionKind, OptionSpec, OptionValue, melk_default_value, option_by_id,
};

/// The values set on one element, keyed by full option id.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PropertyHolder {
    values: BTreeMap<&'static str, OptionValue>,
}

/// The option with this id. Panics for an id that is not registered, which is a
/// bug in the port: callers use ids from the generated registry.
fn spec_of(id: &str) -> &'static OptionSpec {
    option_by_id(id).unwrap_or_else(|| panic!("option {id} is not registered"))
}

impl PropertyHolder {
    /// An empty holder.
    pub fn new() -> Self {
        Self::default()
    }

    /// `setProperty(id, value)`; `None` removes the value.
    pub fn set(&mut self, id: &'static str, value: Option<OptionValue>) {
        match value {
            Some(value) => {
                self.values.insert(id, value);
            }
            None => {
                self.values.remove(id);
            }
        }
    }

    /// `hasProperty(id)`: whether a value was set (defaults do not count).
    pub fn has(&self, id: &str) -> bool {
        self.values.contains_key(id)
    }

    /// The value set for `id`, without defaults.
    pub fn get(&self, id: &str) -> Option<&OptionValue> {
        self.values.get(id)
    }

    /// `getProperty(id)`: the set value, else the melk default, else nothing.
    pub fn value_or_default(&self, id: &str) -> Option<OptionValue> {
        match self.values.get(id) {
            Some(value) => Some(value.clone()),
            None => melk_default_value(spec_of(id)),
        }
    }

    /// `getProperty` for a boolean option; `false` when there is no value.
    pub fn bool_of(&self, id: &str) -> bool {
        matches!(self.value_or_default(id), Some(OptionValue::Boolean(true)))
    }

    /// `getProperty` for an int option.
    pub fn int_of(&self, id: &str) -> Option<i32> {
        match self.value_or_default(id) {
            Some(OptionValue::Int(value)) => Some(value),
            _ => None,
        }
    }

    /// `getProperty` for a double option.
    pub fn double_of(&self, id: &str) -> Option<f64> {
        match self.value_or_default(id) {
            Some(OptionValue::Double(value)) => Some(value),
            Some(OptionValue::Int(value)) => Some(f64::from(value)),
            _ => None,
        }
    }

    /// `getProperty` for an enum option, read through its Java constant name.
    pub fn enum_of<E: JavaEnum>(&self, id: &str) -> Option<E> {
        match self.value_or_default(id) {
            Some(OptionValue::Enum { name, .. }) => E::from_java_text(&name),
            _ => None,
        }
    }

    /// `EnumSet.contains` for an `EnumSet` option; false when there is no value.
    pub fn enum_set_contains(&self, id: &str, name: &str) -> bool {
        matches!(
            self.value_or_default(id),
            Some(OptionValue::EnumSet { names, .. }) if names.iter().any(|found| found == name)
        )
    }

    /// `EnumSet.isEmpty` for an `EnumSet` option; true when there is no value.
    pub fn enum_set_is_empty(&self, id: &str) -> bool {
        match self.value_or_default(id) {
            Some(OptionValue::EnumSet { names, .. }) => names.is_empty(),
            _ => true,
        }
    }

    /// Whether the option's kind is one this holder can answer for, for tests.
    pub fn kind_of(id: &str) -> OptionKind {
        spec_of(id).kind
    }

    /// All values set on this holder, for copying (`copyProperties`).
    pub fn copy_from(&mut self, other: &PropertyHolder) {
        for (id, value) in &other.values {
            self.values.insert(id, value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::data::enums::Direction;

    #[test]
    fn unset_values_fall_back_to_the_melk_default() {
        let holder = PropertyHolder::new();
        assert!(!holder.has("org.eclipse.elk.layered.feedbackEdges"));
        assert!(!holder.bool_of("org.eclipse.elk.layered.feedbackEdges"));
        assert_eq!(
            holder.enum_of::<Direction>("org.eclipse.elk.direction"),
            Some(Direction::UNDEFINED)
        );
    }

    #[test]
    fn set_values_win_and_none_removes_them() {
        let mut holder = PropertyHolder::new();
        holder.set(
            "org.eclipse.elk.layered.feedbackEdges",
            Some(OptionValue::Boolean(true)),
        );
        assert!(holder.bool_of("org.eclipse.elk.layered.feedbackEdges"));
        holder.set("org.eclipse.elk.layered.feedbackEdges", None);
        assert!(!holder.has("org.eclipse.elk.layered.feedbackEdges"));
    }
}
