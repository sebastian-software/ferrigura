//! Canonical model of a requirement diagram: the state the DB holds after the parse, see
//! `docs-port/requirement/PLAN.md`, section 4. Texts keep the DB's entity placeholders (the
//! `ﬂ°name¶ß` marks of `encodeEntities`); the renderer decodes them when it draws.

use serde::{Deserialize, Serialize};

use crate::CANONICAL_VERSION;

/// A parsed requirement diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementDiagram {
    /// Canonical model version, see [`CANONICAL_VERSION`].
    pub version: u32,
    /// The direction of the diagram: the last `direction` statement, `TB` when there is none.
    pub direction: Direction,
    /// Requirements in the order they were first given; a name that repeats keeps its first block.
    pub requirements: Vec<Requirement>,
    /// Elements in the order they were first given.
    pub elements: Vec<Element>,
    /// Relationships in the order of their statements, duplicates kept.
    pub relationships: Vec<Relationship>,
    /// Classes in the order they were first defined.
    pub classes: Vec<RequirementClass>,
    /// Diagram title from the front matter, sanitised. Empty if absent.
    pub title: String,
    /// Accessible title, sanitised. Empty if absent.
    pub acc_title: String,
    /// Accessible description, sanitised. Empty if absent.
    pub acc_descr: String,
}

/// The direction of a requirement diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Direction {
    /// Top to bottom, the default.
    #[default]
    Tb,
    /// Bottom to top.
    Bt,
    /// Right to left.
    Rl,
    /// Left to right.
    Lr,
}

/// The kind of a requirement, as the DB's display string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequirementKind {
    /// `requirement`.
    #[serde(rename = "Requirement")]
    Requirement,
    /// `functionalRequirement`.
    #[serde(rename = "Functional Requirement")]
    FunctionalRequirement,
    /// `interfaceRequirement`.
    #[serde(rename = "Interface Requirement")]
    InterfaceRequirement,
    /// `performanceRequirement`.
    #[serde(rename = "Performance Requirement")]
    PerformanceRequirement,
    /// `physicalRequirement`.
    #[serde(rename = "Physical Requirement")]
    PhysicalRequirement,
    /// `designConstraint`.
    #[serde(rename = "Design Constraint")]
    DesignConstraint,
}

/// The kind of a relationship, as the keyword that gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationshipKind {
    /// `contains`.
    Contains,
    /// `copies`.
    Copies,
    /// `derives`.
    Derives,
    /// `satisfies`.
    Satisfies,
    /// `verifies`.
    Verifies,
    /// `refines`.
    Refines,
    /// `traces`.
    Traces,
}

/// A requirement. `risk` is `Low`, `Medium` or `High`, and `verify_method` is `Analysis`,
/// `Demonstration`, `Inspection` or `Test`; an unset one is the empty string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    /// The name, as written (keywords are case-insensitive, names keep their case).
    pub name: String,
    /// The kind of the requirement.
    #[serde(rename = "type")]
    pub kind: RequirementKind,
    /// The `id` property, empty if absent.
    pub requirement_id: String,
    /// The `text` property, empty if absent.
    pub text: String,
    /// The `risk` property, empty if absent.
    pub risk: String,
    /// The `verifyMethod` property, empty if absent.
    pub verify_method: String,
    /// The classes of the requirement: `default` first, then the classes given to it in order.
    pub classes: Vec<String>,
    /// The styles given to the requirement, in order.
    pub css_styles: Vec<String>,
}

/// An element. `kind` is the `type` property, `doc_ref` the `docref` property; both are empty
/// when absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    /// The name, as written.
    pub name: String,
    /// The `type` property, empty if absent.
    #[serde(rename = "type")]
    pub kind: String,
    /// The `docref` property, empty if absent.
    pub doc_ref: String,
    /// The classes of the element: `default` first, then the classes given to it in order.
    pub classes: Vec<String>,
    /// The styles given to the element, in order.
    pub css_styles: Vec<String>,
}

/// A relationship from `src` to `dst`. The names are not checked against the requirements and
/// elements, so an endpoint may name nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// The kind of the relationship.
    #[serde(rename = "type")]
    pub kind: RelationshipKind,
    /// The name of the source.
    pub src: String,
    /// The name of the destination.
    pub dst: String,
}

/// A class defined with `classDef`: its styles, and the styles that colour its text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementClass {
    /// The class name.
    pub id: String,
    /// The styles given to the class, in order.
    pub styles: Vec<String>,
    /// The styles that match `color`, with `fill` replaced by `bgFill` (the first `fill` only).
    pub text_styles: Vec<String>,
}

impl RequirementDiagram {
    /// Creates an empty model with the current [`CANONICAL_VERSION`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CANONICAL_VERSION,
            direction: Direction::default(),
            requirements: Vec::new(),
            elements: Vec::new(),
            relationships: Vec::new(),
            classes: Vec::new(),
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
        }
    }
}

impl Default for RequirementDiagram {
    fn default() -> Self {
        Self::new()
    }
}
