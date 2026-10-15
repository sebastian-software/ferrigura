//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/data/LayoutMetaDataService.java
//! Source: plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/data/LayoutOptionData.java
//!
//! Layout option metadata, `org.eclipse.elk.core.data` in the Java sources.
//!
//! [`OPTIONS`] is generated from the melk files (see `tools/gen_option_registry.py`).
//! Lookup by suffix follows `LayoutMetaDataService.getOptionDataBySuffix`: a
//! suffix names an option only if exactly one registered id ends with it, on a
//! `.` boundary or as the whole id. Parsing follows `LayoutOptionData.parseValue`:
//! a value that does not parse gives no value, and the option is left unset.

use crate::core::math::{KVector, Spacing};

pub mod enums;
pub mod property;
pub mod registry;

pub use enums::canonical_constant;

pub use registry::OPTIONS;

/// A Java enum as the port uses it: read by constant name or ordinal, and named back.
pub trait JavaEnum: Copy + Sized {
    /// `Enum.valueOf`, falling back to an ordinal index as `enumForString` does.
    fn from_java_text(text: &str) -> Option<Self>;
    /// The Java constant name.
    fn java_name(self) -> &'static str;
}

/// The Java type behind an option, as far as parsing needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    Boolean,
    Int,
    Double,
    String,
    /// An enum; the payload is the Java class name.
    Enum(&'static str),
    /// An `EnumSet` of the named enum.
    EnumSet(&'static str),
    /// A `List` of the named element type.
    List(&'static str),
    /// Any other class; the payload is its simple name.
    Object(&'static str),
    /// `org.eclipse.elk.algorithm`: a layout algorithm id.
    Algorithm,
}

/// One registered option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionSpec {
    /// The full option id, e.g. `org.eclipse.elk.layered.spacing.baseValue`.
    pub id: &'static str,
    pub kind: OptionKind,
    /// The type as written in the melk file.
    pub melk_type: &'static str,
    /// The default as written in the melk file, if it has one.
    pub melk_default: Option<&'static str>,
    /// The melk file that declares it, or `generated`.
    pub source: &'static str,
    /// Ids this option had in older ELK releases (`legacyIds` in melk).
    pub legacy_ids: &'static [&'static str],
}

/// A parsed option value.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Boolean(bool),
    Int(i32),
    Double(f64),
    String(String),
    /// An enum constant by name; the typed enum resolves it.
    Enum {
        class: &'static str,
        name: String,
    },
    /// An enum set, by constant names.
    EnumSet {
        class: &'static str,
        names: Vec<String>,
    },
    /// `ElkMargin`.
    Margin(Spacing),
    /// `ElkPadding`.
    Padding(Spacing),
    /// `KVector`.
    Vector(KVector),
    /// A layout algorithm id, as given (resolved by the caller).
    Algorithm(String),
}

/// Does `suffix` name `id`: equal to it, or the tail after a `.`?
fn suffix_matches(id: &str, suffix: &str) -> bool {
    id.ends_with(suffix)
        && (suffix.len() == id.len() || id.as_bytes()[id.len() - suffix.len() - 1] == b'.')
}

/// The option with exactly this full id.
pub fn option_by_id(id: &str) -> Option<&'static OptionSpec> {
    OPTIONS.iter().find(|spec| spec.id == id)
}

/// `getOptionDataBySuffix`: the one option whose id ends with `suffix`.
///
/// Returns `None` for an empty suffix, for no match, and for more than one
/// match. Legacy ids are consulted only when no current id matches, as in ELK.
pub fn option_by_suffix(suffix: &str) -> Option<&'static OptionSpec> {
    if suffix.is_empty() {
        return None;
    }
    unique_match(suffix, |spec| std::slice::from_ref(&spec.id))
        .or_else(|| unique_match(suffix, |spec| spec.legacy_ids))
}

/// The option whose ids (as given by `ids_of`) are matched by `suffix`, if exactly one is.
fn unique_match<'a>(
    suffix: &str,
    ids_of: impl Fn(&'static OptionSpec) -> &'a [&'static str],
) -> Option<&'static OptionSpec> {
    let mut found = None;
    for spec in OPTIONS {
        if ids_of(spec).iter().any(|id| suffix_matches(id, suffix)) {
            if found.is_some() {
                return None;
            }
            found = Some(spec);
        }
    }
    found
}

/// `LayoutOptionData.parseValue` for an option kind.
///
/// `Ok(None)` means Java would return `null`: the text does not parse for the
/// type, so the option is not set. `Err` is returned for kinds this port does
/// not parse yet, so that such an option is never dropped silently.
pub fn parse_value(spec: &OptionSpec, text: &str) -> Result<Option<OptionValue>, UnsupportedKind> {
    if text == "null" {
        return Ok(None);
    }
    if text.is_empty() && !matches!(spec.kind, OptionKind::EnumSet(_)) {
        return Ok(None);
    }
    let value = match spec.kind {
        OptionKind::Boolean => {
            if text.eq_ignore_ascii_case("true") {
                Some(OptionValue::Boolean(true))
            } else if text.eq_ignore_ascii_case("false") {
                Some(OptionValue::Boolean(false))
            } else {
                None
            }
        }
        // `Integer.valueOf`: an optional sign and digits, nothing else.
        OptionKind::Int => text.parse::<i32>().ok().map(OptionValue::Int),
        OptionKind::Double => crate::core::math::parse_java_double(text).map(OptionValue::Double),
        OptionKind::String => Some(OptionValue::String(text.to_string())),
        OptionKind::Enum(class) => match canonical_constant(class, text) {
            None => return Err(UnsupportedKind::new(spec)),
            Some(None) => None,
            Some(Some(name)) => Some(OptionValue::Enum {
                class,
                name: name.to_string(),
            }),
        },
        OptionKind::EnumSet(class) => {
            let mut names = Vec::new();
            for name in enum_set_names(text) {
                match canonical_constant(class, &name) {
                    None => return Err(UnsupportedKind::new(spec)),
                    Some(None) => return Ok(None),
                    Some(Some(canonical)) => names.push(canonical.to_string()),
                }
            }
            Some(OptionValue::EnumSet { class, names })
        }
        OptionKind::Algorithm => Some(OptionValue::Algorithm(text.to_string())),
        OptionKind::Object("ElkMargin") => match Spacing::parse(text) {
            Ok(spacing) => Some(OptionValue::Margin(spacing)),
            Err(_) => None,
        },
        OptionKind::Object("ElkPadding") => match Spacing::parse(text) {
            Ok(spacing) => Some(OptionValue::Padding(spacing)),
            Err(_) => None,
        },
        OptionKind::Object("KVector") => KVector::parse(text).ok().map(OptionValue::Vector),
        OptionKind::List(_) | OptionKind::Object(_) => {
            return Err(UnsupportedKind {
                id: spec.id,
                melk_type: spec.melk_type,
            });
        }
    };
    Ok(value)
}

/// An option type this port does not parse yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedKind {
    pub id: &'static str,
    pub melk_type: &'static str,
}

impl UnsupportedKind {
    fn new(spec: &OptionSpec) -> Self {
        UnsupportedKind {
            id: spec.id,
            melk_type: spec.melk_type,
        }
    }
}

/// `new ElkPadding(12)`, `new ElkMargin(0)` or `new ElkMargin()`: every side gets the
/// one value, and no argument gives zero on every side.
fn constructor_default(kind: &OptionKind, text: &str) -> Option<OptionValue> {
    let (class, args) = text.strip_prefix("new ")?.split_once('(')?;
    let args = args.strip_suffix(')')?.trim();
    let value = if args.is_empty() {
        0.0
    } else {
        args.parse::<f64>().ok()?
    };
    let sides = Spacing {
        top: value,
        left: value,
        bottom: value,
        right: value,
    };
    match (kind, class) {
        (OptionKind::Object("ElkPadding"), "ElkPadding") => Some(OptionValue::Padding(sides)),
        (OptionKind::Object("ElkMargin"), "ElkMargin") => Some(OptionValue::Margin(sides)),
        _ => None,
    }
}

/// The default of an option as the melk file gives it, when it is a literal this
/// port reads: booleans, numbers, `Integer.MAX_VALUE`, enum constants, and the
/// all-sides `ElkPadding` and `ElkMargin` constructors.
///
/// Other expressions such as `EnumSet.of(...)` return `None`: their defaults are
/// set by the typed reader, not read from the text.
pub fn melk_default_value(spec: &OptionSpec) -> Option<OptionValue> {
    let text = spec.melk_default?;
    if let Some(value) = constructor_default(&spec.kind, text) {
        return Some(value);
    }
    match text {
        "true" => return Some(OptionValue::Boolean(true)),
        "false" => return Some(OptionValue::Boolean(false)),
        "Integer.MAX_VALUE" => return Some(OptionValue::Int(i32::MAX)),
        _ => {}
    }
    if let Ok(number) = text.parse::<i32>() {
        return Some(OptionValue::Int(number));
    }
    // Java float and double literals may carry a suffix: `20f`, `0.5d`.
    let literal = text.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(text);
    if (literal.contains('.') || literal != text)
        && let Some(number) = crate::core::math::parse_java_double(literal)
    {
        return Some(OptionValue::Double(number));
    }
    if let Ok(number) = literal.parse::<i32>() {
        return Some(OptionValue::Int(number));
    }
    if let OptionKind::Enum(class) = spec.kind {
        let constant = text.rsplit('.').next()?;
        if let Some(Some(name)) = canonical_constant(class, constant) {
            return Some(OptionValue::Enum {
                class,
                name: name.to_string(),
            });
        }
    }
    None
}

/// `EnumSet` text: `[A, B]` or `A, B`, names trimmed, empty names dropped.
fn enum_set_names(text: &str) -> Vec<String> {
    let trimmed = text.trim().trim_start_matches('[').trim_end_matches(']');
    trimmed
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffixes_resolve_only_when_unique() {
        // `spacing.baseValue` is registered by Layered only; `nodeNode` by Core only.
        assert_eq!(
            option_by_suffix("spacing.baseValue").map(|spec| spec.id),
            Some("org.eclipse.elk.layered.spacing.baseValue")
        );
        assert_eq!(
            option_by_suffix("spacing.nodeNode").map(|spec| spec.id),
            Some("org.eclipse.elk.spacing.nodeNode")
        );
        assert!(option_by_suffix("").is_none());
        assert!(option_by_suffix("nonexistent.option").is_none());
    }

    #[test]
    fn integer_options_reject_fractions_like_java() {
        let spec =
            option_by_id("org.eclipse.elk.layered.layering.coffmanGraham.layerBound").unwrap();
        assert_eq!(parse_value(spec, "4").unwrap(), Some(OptionValue::Int(4)));
        assert_eq!(parse_value(spec, "4.0").unwrap(), None);
    }

    #[test]
    fn padding_default_is_the_all_sides_constructor() {
        let padding = option_by_id("org.eclipse.elk.padding").unwrap();
        assert_eq!(
            melk_default_value(padding),
            Some(OptionValue::Padding(Spacing {
                top: 12.0,
                left: 12.0,
                bottom: 12.0,
                right: 12.0
            }))
        );
    }

    #[test]
    fn mermaid_values_parse() {
        let padding = option_by_id("org.eclipse.elk.padding").unwrap();
        assert_eq!(
            parse_value(padding, "[top=24,left=24,bottom=24,right=24]").unwrap(),
            Some(OptionValue::Padding(Spacing {
                top: 24.0,
                left: 24.0,
                bottom: 24.0,
                right: 24.0
            }))
        );
        let flag = option_by_id("org.eclipse.elk.expandNodes").unwrap();
        assert_eq!(
            parse_value(flag, "TRUE").unwrap(),
            Some(OptionValue::Boolean(true))
        );
    }
}
