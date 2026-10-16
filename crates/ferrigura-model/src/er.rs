//! Canonical model of an ER diagram, as the ER DB of `erDb.ts` holds it after parsing, less what the
//! input does not contain or only rendering needs (see `docs-port/er/PLAN.md`, section 5).
//!
//! Field names are the DB's names in camelCase. The entity and class lists keep the DB's order (the
//! order of creation), and so do the attribute and member lists. The cardinalities and the
//! identification are the DB's strings, `MD_PARENT` included.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// The cardinality of one end of a relationship, as `erDb.ts` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Cardinality {
    /// `|o`, `o|`, `one or zero`, `zero or one`.
    ZeroOrOne,
    /// `}o`, `o{`, `0+`, `zero or more`, `zero or many`, `many`, `many(0)`.
    ZeroOrMore,
    /// `}|`, `|{`, `1+`, `one or more`, `one or many`, `many(1)`.
    OneOrMore,
    /// `||`, `one`, `only one`, and `1` in the positions where the lexer reads it as a cardinality.
    OnlyOne,
    /// `u` before `.`, `-` or `|`: the parent side of a parent-child relationship.
    MdParent,
}

/// Whether a relationship identifies its child (`--`, `to`) or not (`..`, `optionally to`, `.-`,
/// `-.`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Identification {
    /// The relationship is non-identifying.
    NonIdentifying,
    /// The relationship is identifying.
    Identifying,
}

/// The cardinalities and the identification of one relationship. `cardA` is the end written on the
/// right of the line, `cardB` the end on the left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelSpec {
    /// Cardinality of the entity on the right.
    pub card_a: Cardinality,
    /// Identification of the relationship.
    pub rel_type: Identification,
    /// Cardinality of the entity on the left.
    pub card_b: Cardinality,
}

/// A parsed ER diagram: its header texts, its direction, and the DB's entities, relationships,
/// classes and subgraphs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// Diagram title from the front matter, sanitised; the grammar has no title statement.
    pub title: String,
    /// Accessible title from `accTitle`, sanitised, empty if absent.
    pub acc_title: String,
    /// Accessible description from `accDescr`, sanitised, empty if absent.
    pub acc_descr: String,
    /// Direction: `TB` unless a top-level `direction` statement gave another.
    pub direction: String,
    /// Entities in the order they were first named. The id is `entity-<name>-<n>`, where `n` is the
    /// number of entities before it.
    pub entities: Vec<ErEntity>,
    /// Relationships in source order. Each end is an entity id, or a subgraph id.
    pub relationships: Vec<ErRelationship>,
    /// Class definitions in the order they are first named.
    pub classes: Vec<ErClass>,
    /// Subgraphs in the order they close, a repeated id included as its own entry.
    pub subgraphs: Vec<ErSubgraph>,
}

/// An entity: a name, an optional alias, its attributes, and its classes and styles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErEntity {
    /// The DB's id, `entity-<name>-<n>`.
    pub id: String,
    /// The name, without quotes.
    pub name: String,
    /// The alias from `name["alias"]`, empty if none.
    pub alias: String,
    /// Attributes in source order.
    pub attributes: Vec<ErAttribute>,
    /// Classes applied with `:::` or `class`, in order; the DB's implicit `default` is left out.
    pub classes: Vec<String>,
    /// Styles from `style` statements, in order.
    pub styles: Vec<String>,
}

/// An attribute of an entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErAttribute {
    /// The type as written, with a trailing `?` when the source had one.
    #[serde(rename = "type")]
    pub attr_type: String,
    /// The name as written, backticks removed.
    pub name: String,
    /// The keys as written (`PK`, `pk`, `FK`, `UK`), in order; empty if none.
    pub keys: Vec<String>,
    /// The comment without its quotes, empty if none.
    pub comment: String,
}

/// A relationship between two ends. The ends are entity ids, or the ids of subgraphs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErRelationship {
    /// The left end: an entity id, or a subgraph id.
    pub entity_a: String,
    /// The role written after the colon, empty if none.
    pub role_a: String,
    /// The right end: an entity id, or a subgraph id.
    pub entity_b: String,
    /// The cardinalities and the identification.
    pub rel_spec: RelSpec,
}

/// A class from `classDef`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErClass {
    /// The class name.
    pub id: String,
    /// The styles, as the concatenated tokens of each comma-separated part.
    pub styles: Vec<String>,
    /// The styles that name a colour, with `fill` replaced by `bgFill` in the first occurrence.
    pub text_styles: Vec<String>,
}

/// A subgraph: its id and title, its members, its classes and styles, and its direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErSubgraph {
    /// The id, trimmed.
    pub id: String,
    /// The title, sanitised and trimmed; the id when no title is given.
    pub title: String,
    /// Members in source order: entity names, nested subgraph ids, and the strings the grammar keeps
    /// from `classDef`, `class`, `style`, `accTitle` and `accDescr` statements (see the plan).
    pub nodes: Vec<String>,
    /// Classes applied with `class` or `:::` on the subgraph id, in order.
    pub classes: Vec<String>,
    /// Styles from `style` statements on the subgraph id, in order.
    pub styles: Vec<String>,
    /// Direction from a `direction` statement inside the subgraph, `None` if none.
    pub dir: Option<String>,
}

impl ErDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`] and direction `TB`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            direction: String::from("TB"),
            entities: Vec::new(),
            relationships: Vec::new(),
            classes: Vec::new(),
            subgraphs: Vec::new(),
        }
    }
}

impl Default for ErDiagram {
    fn default() -> Self {
        Self::new()
    }
}
