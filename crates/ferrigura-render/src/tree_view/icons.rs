//! The icons of a tree view node: `icons.ts` (`treeViewIcons`, `detectIcon`, `qualifyIcon` and
//! `getNodeIcon`, lines 16-100) and the icon the renderer writes for the result, `getIconSVG`
//! (`rendering-util/icons.ts` 88-106), with the unknown icon of `rendering-util/icons.ts` 20-24.
//!
//! The reference's icon packs are the built-in `mermaid-treeview` pack (its `folder` and `file`) and
//! the packs the user registers with `registerIconPacks`. ferrigura has no registration, so no name
//! other than the built-in two resolves: every other icon is drawn as the reference draws an icon it
//! cannot find, the 80 by 80 unknown icon (the reference logs the failure and draws the same).

use ferrigura_model::config::TreeViewDiagramConfig;
use ferrigura_model::tree_view::{NodeType, TreeViewNode};
use ferrigura_model::value::Value;

use crate::RenderError;

/// The prefix of the built-in pack (`icons.ts` 16).
const BUILT_IN_PREFIX: &str = "mermaid-treeview";

/// The names of the built-in icons (`treeViewIcons.icons`, `icons.ts` 21 and 24).
const BUILT_IN_NAMES: [&str; 2] = ["folder", "file"];

/// The size the icon is drawn at (`ICON_SIZE` of `renderer.ts` 17).
const ICON_PIXELS: &str = "14";

/// The names that `Object.prototype` gives every object. A map of icons reads one of them as the
/// inherited member when the map does not hold it, which is a function or an object: the reference
/// then fails in `qualifyIcon` (`icon.includes`), and so does the render.
const OBJECT_PROTOTYPE_NAMES: [&str; 12] = [
    "constructor",
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toString",
    "valueOf",
    "toLocaleString",
];

/// The icon a node is drawn with, after `getNodeIcon` has chosen its name: a built-in icon, or the
/// unknown icon of a name that does not resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Icon {
    /// `mermaid-treeview:folder`.
    Folder,
    /// `mermaid-treeview:file`.
    File,
    /// Any name that is not a built-in icon (`rendering-util/icons.ts` 94-99).
    Unknown,
}

impl Icon {
    /// The `<svg>` element the reference's `getIconSVG` writes for the icon, sized to
    /// [`ICON_PIXELS`]: `iconToHTML` of the icon's body, with the attributes in the order the
    /// reference writes them.
    #[must_use]
    pub(super) fn markup(self) -> String {
        let (view_box, body) = match self {
            Self::Folder => (
                "0 0 24 24",
                r#"<path fill="currentColor" d="M10.59 4.59A2 2 0 0 0 9.17 4H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.17z"/>"#,
            ),
            Self::File => (
                "0 0 24 24",
                r#"<path fill="currentColor" fill-rule="evenodd" d="M6 2a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8.83a2 2 0 0 0-.59-1.42l-4.82-4.82A2 2 0 0 0 13.17 2H6Zm7.5 1.9l4.6 4.6h-3.6a1 1 0 0 1-1-1V3.9Z" clip-rule="evenodd"/>"#,
            ),
            Self::Unknown => (
                "0 0 80 80",
                r#"<g><rect width="80" height="80" style="fill: #087ebf; stroke-width: 0px;"/><text transform="translate(21.16 64.67)" style="fill: #fff; font-family: ArialMT, Arial; font-size: 67.75px;"><tspan x="0" y="0">?</tspan></text></g>"#,
            ),
        };
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{ICON_PIXELS}" height="{ICON_PIXELS}" viewBox="{view_box}">{body}</svg>"#
        )
    }
}

/// What `detectIcon` found for a file: the reference's `'none'`, which hides the icon, or the
/// qualified name of an icon. Nothing found is `None` at the call site.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Detected {
    /// The map gives `none` for the file.
    Hidden,
    /// The map gives a name, qualified as `qualifyIcon` qualifies it.
    Icon(String),
}

/// The icon of a node, as `getNodeIcon` decides it (`icons.ts` 77-100), resolved to the icon the
/// renderer draws. `Ok(None)` is an undefined `getNodeIcon`: no icon.
///
/// # Errors
///
/// Returns [`RenderError::Config`] where the reference fails: a map entry that is not text (its
/// `qualifyIcon` calls `includes`), or a file name that reads an `Object.prototype` member.
pub(super) fn node_icon(
    node: &TreeViewNode,
    section: &TreeViewDiagramConfig,
) -> Result<Option<Icon>, RenderError> {
    // An explicit `icon()` annotation: `none` hides the icon, any other name is qualified.
    if node.icon == "none" {
        return Ok(None);
    }
    if !node.icon.is_empty() {
        return Ok(Some(resolve(&qualify(
            &node.icon,
            &section.default_icon_pack,
        ))));
    }
    if !section.show_icons {
        return Ok(None);
    }
    if node.node_type == NodeType::File {
        match detect(&node.name, section)? {
            Some(Detected::Hidden) => return Ok(None),
            Some(Detected::Icon(qualified)) => return Ok(Some(resolve(&qualified))),
            None => {}
        }
    }
    Ok(Some(match node.node_type {
        NodeType::Directory => Icon::Folder,
        NodeType::File => Icon::File,
    }))
}

/// `detectIcon` (`icons.ts` 40-56) with the maps of the section: the exact file name first, when it
/// gives a truthy value, then the extension (the last dot, a dot that is not the first character,
/// lower-cased, with the dot and then without it). `Ok(None)` is the reference's `undefined` or a
/// falsy value, which `getNodeIcon` takes as no detection.
fn detect(name: &str, section: &TreeViewDiagramConfig) -> Result<Option<Detected>, RenderError> {
    let filenames = section.filename_icons.as_ref();
    if let Some(value) = entry(filenames, name)?
        && truthy(value)
    {
        return classify(value, &section.default_icon_pack).map(Some);
    }
    let Some(dot) = name.rfind('.').filter(|dot| *dot > 0) else {
        return Ok(None);
    };
    let extension = name[dot..].to_lowercase();
    let extensions = section.extension_icons.as_ref();
    // `extensionIcons[ext] ?? extensionIcons[ext.slice(1)]`: only an absent value falls through.
    let found = match entry(extensions, &extension)? {
        Some(value) => Some(value),
        None => entry(extensions, &extension[1..])?,
    };
    match found {
        Some(value) if truthy(value) => classify(value, &section.default_icon_pack).map(Some),
        _ => Ok(None),
    }
}

/// What `getNodeIcon` makes of a truthy value of a map: `none` hides the icon, any other text is
/// qualified.
fn classify(value: &Value, default_pack: &str) -> Result<Detected, RenderError> {
    let Value::String(text) = value else {
        return Err(RenderError::Config(
            "an icon map holds a value that is not text".to_owned(),
        ));
    };
    if text == "none" {
        Ok(Detected::Hidden)
    } else {
        Ok(Detected::Icon(qualify(text, default_pack)))
    }
}

/// `map[key]` as the reference reads it: the entry the map holds, with `null` as absent. A key that
/// the map does not hold but `Object.prototype` does reads the inherited member, which the render
/// fails on (see [`OBJECT_PROTOTYPE_NAMES`]). A map that is not an object is read as empty here; the
/// reference would index the value's own properties (an open question of the port).
fn entry<'a>(map: Option<&'a Value>, key: &str) -> Result<Option<&'a Value>, RenderError> {
    let Some(Value::Object(entries)) = map else {
        return Ok(None);
    };
    match entries.get(key) {
        Some(Value::Null) => Ok(None),
        Some(value) => Ok(Some(value)),
        None if OBJECT_PROTOTYPE_NAMES.contains(&key) => Err(RenderError::Config(format!(
            "an icon map reads the inherited member {key:?}"
        ))),
        None => Ok(None),
    }
}

/// JavaScript's truthiness: `true`, a non-empty string, a number that is neither zero nor `NaN`, and
/// any list or object.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Bool(flag) => *flag,
        Value::String(text) => !text.is_empty(),
        Value::Number(number) => *number != 0.0 && !number.is_nan(),
        Value::Array(_) | Value::Object(_) => true,
        Value::Null => false,
    }
}

/// `qualifyIcon` (`icons.ts` 58-66): a name with a `:` is kept; a built-in name, or any name when no
/// default pack is set, takes the built-in prefix; any other name takes the default pack.
fn qualify(icon: &str, default_pack: &str) -> String {
    if icon.contains(':') {
        icon.to_owned()
    } else if BUILT_IN_NAMES.contains(&icon) || default_pack.is_empty() {
        format!("{BUILT_IN_PREFIX}:{icon}")
    } else {
        format!("{default_pack}:{icon}")
    }
}

/// The icon a qualified name resolves to: the two built-in icons, and the unknown icon for any
/// other name (no pack is registered, so no other name is found).
fn resolve(qualified: &str) -> Icon {
    match qualified
        .strip_prefix(BUILT_IN_PREFIX)
        .and_then(|rest| rest.strip_prefix(':'))
    {
        Some("folder") => Icon::Folder,
        Some("file") => Icon::File,
        _ => Icon::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_qualified_built_in_name_resolves_to_its_icon() {
        assert_eq!(resolve("mermaid-treeview:folder"), Icon::Folder);
        assert_eq!(resolve("mermaid-treeview:file"), Icon::File);
        assert_eq!(resolve("mermaid-treeview:bell"), Icon::Unknown);
        assert_eq!(resolve("fa:folder"), Icon::Unknown);
    }

    #[test]
    fn the_unknown_icon_is_the_reference_fallback_in_eighty_units() {
        let markup = Icon::Unknown.markup();
        assert!(markup.starts_with(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 80 80"><g>"#
        ));
        assert!(markup.contains(">?</tspan>"));
    }
}
