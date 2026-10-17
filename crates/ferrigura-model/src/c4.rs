//! Canonical model of a C4 diagram: the DB state after the parse (`c4Db.ts`), with the fields of
//! `docs-port/c4/NOTES.md`, section 4.
//!
//! A text field is a string when the DB holds `{ text }`. It is absent (left out of the JSON) when
//! the DB holds nothing, or a bare string that a style statement wrote over it; the reference reads
//! `.text` of such a value as `undefined`. Texts keep the entity placeholders the DB keeps, see
//! `docs-port/canonical-model.md`. The offsets of a relation are the strings that `String()` gives
//! the DB value (`docs-port/c4/PLAN.md`, section 4).

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::CANONICAL_VERSION;

/// A parsed C4 diagram: its header and title, the layout counts, and the boundaries, shapes and
/// relations in the order the DB holds them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct C4Diagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The header keyword: `C4Context`, `C4Container`, `C4Component`, `C4Dynamic` or
    /// `C4Deployment`.
    pub c4_type: String,
    /// The diagram title: the last `title` line, or the last `accTitle` (the reference stores an
    /// accessibility title as the title, `docs-port/c4/NOTES.md` P26). Empty if none.
    pub title: String,
    /// The accessible title. The C4 parser never sets it, so it is always empty.
    pub acc_title: String,
    /// The accessible description, from `accDescr`. Empty if none.
    pub acc_descr: String,
    /// The number of shapes in a row, `4` unless `UpdateLayoutConfig` sets it.
    pub c4_shape_in_row: f64,
    /// The number of boundaries in a row, `2` unless `UpdateLayoutConfig` sets it.
    pub c4_boundary_in_row: f64,
    /// Every boundary in the order the DB holds them. The first is the `global` boundary.
    pub boundaries: Vec<C4Boundary>,
    /// Every shape (person, system, container, component and their kinds) in declaration order.
    pub shapes: Vec<C4Shape>,
    /// Every relation in the order it was first given.
    pub rels: Vec<C4Relation>,
}

impl C4Diagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`] and the DB's defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            c4_type: String::new(),
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            c4_shape_in_row: 4.0,
            c4_boundary_in_row: 2.0,
            boundaries: vec![C4Boundary::global()],
            shapes: Vec::new(),
            rels: Vec::new(),
        }
    }
}

impl Default for C4Diagram {
    fn default() -> Self {
        Self::new()
    }
}

/// The text of a label. A label is a string, except when the statement gave it as a named argument
/// (`Person(a, $sprite="x")`): the reference then keeps the one-entry object of the argument as the
/// text, and the model writes that object (`docs-port/c4/PLAN.md`, section 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum C4Text {
    /// A string.
    Plain(String),
    /// The `{ key: value }` object of a named argument.
    Named(BTreeMap<String, String>),
}

/// A person, system, container or component, or one of their kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct C4Shape {
    /// The alias the statement gives, which identifies the shape.
    pub alias: String,
    /// The kind: `person`, `external_person`, `system`, `system_db`, `container`, `component` and
    /// the other kinds of the reference's `typeC4Shape`.
    pub type_c4_shape: String,
    /// The label, `None` when it was overwritten by a bare string.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub label: Option<C4Text>,
    /// The description, absent when only a named attribute set the slot.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub descr: Option<String>,
    /// The technology of a container or component (absent for persons and systems).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub techn: Option<String>,
    /// The sprite name, `$sprite`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sprite: Option<String>,
    /// The tags, `$tags`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<String>,
    /// The link, `$link`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub link: Option<String>,
    /// The alias of the boundary the shape is in, `global` at the top level.
    pub parent_boundary: String,
    /// Whether the text wraps: the top-level `wrap` configuration when the shape was read.
    pub wrap: bool,
    /// `UpdateElementStyle` background colour.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bg_color: Option<String>,
    /// `UpdateElementStyle` font colour.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub font_color: Option<String>,
    /// `UpdateElementStyle` border colour.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub border_color: Option<String>,
    /// `UpdateElementStyle` shadowing.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub shadowing: Option<String>,
    /// `UpdateElementStyle` shape.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub shape: Option<String>,
    /// `UpdateElementStyle` legend text.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub legend_text: Option<String>,
    /// `UpdateElementStyle` legend sprite.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub legend_sprite: Option<String>,
    /// Named attributes that have no field of their own, as the DB stores them: a string, or
    /// `{ "text": … }` for a text field.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A boundary: a `Boundary`, `System_Boundary`, `Enterprise_Boundary`, `Container_Boundary` or a
/// deployment node, or the `global` boundary that the DB holds from the start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct C4Boundary {
    /// The alias the statement gives.
    pub alias: String,
    /// The label.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub label: Option<C4Text>,
    /// The boundary type: `system`, `container`, `global`, `ENTERPRISE`, `SYSTEM`, `CONTAINER`
    /// or the third argument of a bare `Boundary`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    pub boundary_type: Option<String>,
    /// The deployment node type (`node`, `nodeL` or `nodeR`), only for deployment nodes.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub node_type: Option<String>,
    /// The description. Only deployment nodes set it by default.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub descr: Option<String>,
    /// The technology, only from a named attribute.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub techn: Option<String>,
    /// The tags. `null` on the global boundary.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "nullable"
    )]
    pub tags: Option<Option<String>>,
    /// The link. `null` on the global boundary.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "nullable"
    )]
    pub link: Option<Option<String>>,
    /// The sprite, only from a named attribute.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sprite: Option<String>,
    /// The alias of the boundary this one is in; empty for the global boundary.
    pub parent_boundary: String,
    /// Whether the text wraps, as for shapes.
    pub wrap: bool,
    /// `UpdateElementStyle` background colour.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bg_color: Option<String>,
    /// `UpdateElementStyle` font colour.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub font_color: Option<String>,
    /// `UpdateElementStyle` border colour.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub border_color: Option<String>,
    /// `UpdateElementStyle` shadowing.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub shadowing: Option<String>,
    /// `UpdateElementStyle` shape.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub shape: Option<String>,
    /// `UpdateElementStyle` legend text.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub legend_text: Option<String>,
    /// `UpdateElementStyle` legend sprite.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub legend_sprite: Option<String>,
    /// Named attributes that have no field of their own.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl C4Boundary {
    /// The `global` boundary the DB holds before any declaration (`c4Db.ts:53-62`).
    #[must_use]
    pub fn global() -> Self {
        Self {
            alias: "global".to_owned(),
            label: Some(C4Text::Plain("global".to_owned())),
            boundary_type: Some("global".to_owned()),
            node_type: None,
            descr: None,
            techn: None,
            tags: Some(None),
            link: Some(None),
            sprite: None,
            parent_boundary: String::new(),
            wrap: false,
            bg_color: None,
            font_color: None,
            border_color: None,
            shadowing: None,
            shape: None,
            legend_text: None,
            legend_sprite: None,
            extra: BTreeMap::new(),
        }
    }
}

/// A relationship between two aliases. The DB keeps one per `(from, to)` pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct C4Relation {
    /// The kind: `rel`, `birel`, `rel_u`, `rel_d`, `rel_l`, `rel_r` or `rel_b`.
    #[serde(rename = "type")]
    pub relation_type: String,
    /// The alias the relation starts at (a shape or a boundary).
    pub from: String,
    /// The alias the relation ends at (a shape or a boundary).
    pub to: String,
    /// The label.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub label: Option<C4Text>,
    /// The technology, `""` when none was given.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub techn: Option<String>,
    /// The description, `""` when none was given.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub descr: Option<String>,
    /// The sprite, `$sprite`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sprite: Option<String>,
    /// The tags, `$tags`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<String>,
    /// The link, `$link`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub link: Option<String>,
    /// Whether the text wraps, as for shapes.
    pub wrap: bool,
    /// `UpdateRelStyle` text colour.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub text_color: Option<String>,
    /// `UpdateRelStyle` line colour.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub line_color: Option<String>,
    /// `UpdateRelStyle` horizontal offset, as the text of the DB value (`String()`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub offset_x: Option<String>,
    /// `UpdateRelStyle` vertical offset, as the text of the DB value (`String()`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub offset_y: Option<String>,
    /// Named attributes that have no field of their own.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Reads a field that may be `null`: an absent field is `None`, `null` is `Some(None)`, and a
/// string is `Some(Some(_))`. The outer option is set by `default` and the inner one by the value.
fn nullable<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_diagram_has_the_global_boundary_and_the_defaults() {
        let model = C4Diagram::new();
        assert_eq!(model.boundaries.len(), 1);
        assert_eq!(model.boundaries[0].alias, "global");
        assert_eq!(model.c4_shape_in_row, 4.0);
        assert_eq!(model.c4_boundary_in_row, 2.0);
    }

    #[test]
    fn the_global_boundary_writes_null_tags_and_link() {
        let json = serde_json::to_value(C4Boundary::global()).expect("serialises");
        assert_eq!(json["tags"], Value::Null);
        assert_eq!(json["link"], Value::Null);
        assert_eq!(json["type"], "global");
        assert!(json.get("descr").is_none());
    }

    #[test]
    fn an_absent_text_field_is_left_out() {
        let shape = C4Shape {
            alias: "a".to_owned(),
            type_c4_shape: "person".to_owned(),
            label: Some(C4Text::Plain("A".to_owned())),
            descr: None,
            techn: None,
            sprite: None,
            tags: None,
            link: None,
            parent_boundary: "global".to_owned(),
            wrap: false,
            bg_color: None,
            font_color: None,
            border_color: None,
            shadowing: None,
            shape: None,
            legend_text: None,
            legend_sprite: None,
            extra: BTreeMap::new(),
        };
        let json = serde_json::to_value(shape).expect("serialises");
        assert!(json.get("descr").is_none());
        assert!(json.get("techn").is_none());
        assert_eq!(json["typeC4Shape"], "person");
    }

    #[test]
    fn a_relation_type_is_written_as_type() {
        let relation = C4Relation {
            relation_type: "rel".to_owned(),
            from: "a".to_owned(),
            to: "b".to_owned(),
            label: Some(C4Text::Plain("x".to_owned())),
            techn: Some(String::new()),
            descr: Some(String::new()),
            sprite: None,
            tags: None,
            link: None,
            wrap: false,
            text_color: None,
            line_color: None,
            offset_x: None,
            offset_y: None,
            extra: BTreeMap::new(),
        };
        let json = serde_json::to_value(relation).expect("serialises");
        assert_eq!(json["type"], "rel");
        assert_eq!(json["techn"], "");
    }
}
