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

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::core::math::{KVector, Spacing};

pub mod enums;
pub mod property;
pub mod registry;

pub use enums::canonical_constant;

use registry::LAYERED_PARENT_DEFAULTS;
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
        name: &'static str,
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

/// A registered option, by its position in [`OPTIONS`].
///
/// [`OPTIONS`] is sorted by id, so ordering options by `OptionId` orders them by
/// id, as Java's `MapPropertyHolder` and its string keys do. Property holders are
/// keyed by it, so that reading an option compares integers, not long ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OptionId(u16);

impl OptionId {
    /// The option with this full id, for constants: an id that is not registered
    /// stops the build, so a misspelled id can never read a default at run time.
    /// At run time, [`option_by_id`] is the lookup.
    pub const fn of(id: &str) -> OptionId {
        let mut index = 0;
        while index < OPTIONS.len() {
            if const_str_eq(OPTIONS[index].id, id) {
                return OptionId(index as u16);
            }
            index += 1;
        }
        panic!("the option id is not registered");
    }

    /// The registry entry of this option.
    pub fn spec(self) -> &'static OptionSpec {
        &OPTIONS[self.index()]
    }

    /// The full option id.
    pub fn name(self) -> &'static str {
        self.spec().id
    }

    /// The position in [`OPTIONS`], for tables indexed by option.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

impl std::fmt::Display for OptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// `str` equality for constant evaluation, where `==` on strings is not available.
const fn const_str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut index = 0;
    while index < a.len() {
        if a[index] != b[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// The option with exactly this full id.
pub fn option_by_id(id: &str) -> Option<OptionId> {
    OPTIONS
        .binary_search_by(|spec| spec.id.cmp(id))
        .ok()
        .map(|index| OptionId(index as u16))
}

/// `getOptionDataBySuffix`: the one option whose id ends with `suffix`.
///
/// A suffix names an id when it equals the id or is its tail after a `.`.
/// Returns `None` for an empty suffix, for no match, and for more than one
/// match. Legacy ids are consulted only when no current id matches, as in ELK.
pub fn option_by_suffix(suffix: &str) -> Option<OptionId> {
    // Every graph element names its options by suffix, so the suffixes of all ids
    // are indexed once instead of matched against the whole registry per key.
    static SUFFIXES: LazyLock<[HashMap<&'static str, SuffixMatch>; 2]> = LazyLock::new(|| {
        [
            suffix_index(|spec| std::slice::from_ref(&spec.id)),
            suffix_index(|spec| spec.legacy_ids),
        ]
    });
    if suffix.is_empty() {
        return None;
    }
    let [current, legacy] = &*SUFFIXES;
    let unique = |index: &HashMap<&'static str, SuffixMatch>| match index.get(suffix) {
        Some(SuffixMatch::Unique(option)) => Some(*option),
        Some(SuffixMatch::Ambiguous) | None => None,
    };
    unique(current).or_else(|| unique(legacy))
}

/// What a suffix names in a [`suffix_index`].
#[derive(Debug, Clone, Copy)]
enum SuffixMatch {
    Unique(OptionId),
    Ambiguous,
}

/// Every suffix of the ids `ids_of` gives for each option: the id itself and each
/// tail after a `.`, with the one option it names or with [`SuffixMatch::Ambiguous`].
fn suffix_index(
    ids_of: impl Fn(&'static OptionSpec) -> &'static [&'static str],
) -> HashMap<&'static str, SuffixMatch> {
    let mut index = HashMap::new();
    for (position, spec) in OPTIONS.iter().enumerate() {
        let option = OptionId(position as u16);
        for &id in ids_of(spec) {
            let tails = id
                .match_indices('.')
                .map(|(dot, _)| &id[dot + 1..])
                .chain([id]);
            for tail in tails {
                index
                    .entry(tail)
                    .and_modify(|found| {
                        if !matches!(found, SuffixMatch::Unique(other) if *other == option) {
                            *found = SuffixMatch::Ambiguous;
                        }
                    })
                    .or_insert(SuffixMatch::Unique(option));
            }
        }
    }
    index
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
            Some(Some(name)) => Some(OptionValue::Enum { class, name }),
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

/// The value an unset option reads as, [`default_value`] parsed once per process.
pub fn option_default(id: OptionId) -> Option<&'static OptionValue> {
    static DEFAULTS: LazyLock<Vec<Option<OptionValue>>> =
        LazyLock::new(|| OPTIONS.iter().map(default_value).collect());
    DEFAULTS[id.index()].as_ref()
}

/// The value an unset option reads as. An option that targets parents takes the
/// layered algorithm's `supports` value from Layered.melk, which elkjs reads in
/// place of the melk default; every other option takes its melk default. This
/// crate runs only the layered algorithm, so no other algorithm's `supports` is
/// consulted.
pub fn default_value(spec: &OptionSpec) -> Option<OptionValue> {
    match LAYERED_PARENT_DEFAULTS
        .iter()
        .find(|(id, _)| *id == spec.id)
    {
        Some((_, text)) => parse_default(spec, text),
        None => melk_default_value(spec),
    }
}

/// The default of an option as the melk file gives it, when it is a literal this
/// port reads: booleans, numbers, `Integer.MAX_VALUE`, enum constants, and the
/// all-sides `ElkPadding` and `ElkMargin` constructors.
///
/// Other expressions such as `EnumSet.of(...)` return `None`: their defaults are
/// set by the typed reader, not read from the text.
pub fn melk_default_value(spec: &OptionSpec) -> Option<OptionValue> {
    parse_default(spec, spec.melk_default?)
}

/// One default expression of the option's kind, as [`melk_default_value`] reads it.
fn parse_default(spec: &OptionSpec, text: &str) -> Option<OptionValue> {
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
            return Some(OptionValue::Enum { class, name });
        }
    }
    None
}

/// `EnumSet` text: `[A, B]` or `A, B`, names trimmed, empty names dropped.
fn enum_set_names(text: &str) -> Vec<String> {
    // `enumSetForStringArray`: `split("[\\[\\]\\s,]+")`, so brackets, whitespace and commas all
    // separate names, and the empty components are skipped.
    text.split(|c: char| c == '[' || c == ']' || c == ',' || c.is_whitespace())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_sorted_by_id_so_option_ids_sort_like_ids() {
        assert!(OPTIONS.windows(2).all(|pair| pair[0].id < pair[1].id));
        for (index, spec) in OPTIONS.iter().enumerate() {
            let id = option_by_id(spec.id).expect("registered option");
            assert_eq!((id.index(), id.name()), (index, spec.id));
        }
        assert!(option_by_id("org.eclipse.elk.nonexistent").is_none());
    }

    /// `getOptionDataBySuffix` as ELK writes it: a scan of the registry per key.
    fn suffix_by_scan(suffix: &str) -> Option<OptionId> {
        let matches = |id: &str| {
            id.ends_with(suffix)
                && (suffix.len() == id.len() || id.as_bytes()[id.len() - suffix.len() - 1] == b'.')
        };
        let unique = |ids_of: &dyn Fn(&OptionSpec) -> &[&str]| {
            let mut found = None;
            for (index, spec) in OPTIONS.iter().enumerate() {
                if ids_of(spec).iter().any(|id| matches(id)) {
                    if found.is_some() {
                        return None;
                    }
                    found = Some(OptionId(index as u16));
                }
            }
            found
        };
        if suffix.is_empty() {
            return None;
        }
        unique(&|spec| std::slice::from_ref(&spec.id)).or_else(|| unique(&|spec| spec.legacy_ids))
    }

    #[test]
    fn the_suffix_index_agrees_with_a_registry_scan() {
        let mut keys = vec![
            "", "layered", "spacing", "elk", "org", "nodeNode", "Node", "x.y",
        ];
        for spec in OPTIONS {
            for id in std::iter::once(&spec.id).chain(spec.legacy_ids) {
                for start in 0..id.len() {
                    keys.push(&id[start..]);
                }
            }
        }
        for key in keys {
            assert_eq!(option_by_suffix(key), suffix_by_scan(key), "suffix {key:?}");
        }
    }

    #[test]
    fn suffixes_resolve_only_when_unique() {
        // `spacing.baseValue` is registered by Layered only; `nodeNode` by Core only.
        assert_eq!(
            option_by_suffix("spacing.baseValue").map(OptionId::name),
            Some("org.eclipse.elk.layered.spacing.baseValue")
        );
        assert_eq!(
            option_by_suffix("spacing.nodeNode").map(OptionId::name),
            Some("org.eclipse.elk.spacing.nodeNode")
        );
        assert!(option_by_suffix("").is_none());
        assert!(option_by_suffix("nonexistent.option").is_none());
    }

    #[test]
    fn integer_options_reject_fractions_like_java() {
        let spec = OptionId::of("org.eclipse.elk.layered.layering.coffmanGraham.layerBound").spec();
        assert_eq!(parse_value(spec, "4").unwrap(), Some(OptionValue::Int(4)));
        assert_eq!(parse_value(spec, "4.0").unwrap(), None);
    }

    #[test]
    fn padding_default_is_the_all_sides_constructor() {
        let padding = OptionId::of("org.eclipse.elk.padding").spec();
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
        let padding = OptionId::of("org.eclipse.elk.padding").spec();
        assert_eq!(
            parse_value(padding, "[top=24,left=24,bottom=24,right=24]").unwrap(),
            Some(OptionValue::Padding(Spacing {
                top: 24.0,
                left: 24.0,
                bottom: 24.0,
                right: 24.0
            }))
        );
        let flag = OptionId::of("org.eclipse.elk.expandNodes").spec();
        assert_eq!(
            parse_value(flag, "TRUE").unwrap(),
            Some(OptionValue::Boolean(true))
        );
    }
}
