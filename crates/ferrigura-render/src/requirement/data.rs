//! The requirement diagram as the layout and the renderer read it: a port of `getData` in
//! `packages/mermaid/src/diagrams/requirement/requirementDb.ts` (lines 292-342), which fills the
//! layout data of [`crate::unified::data`]. The differential run (`pnpm port:diff requirement-data`)
//! compares it with the reference's `getData()`; see `docs-port/requirement/NOTES.md`, section 3.5.
//!
//! The nodes are the requirements in their order, then the elements, each a `requirementBox`; the
//! edges are the relationships in their order, with the id `<src>-<dst>-<n>`, where `n` counts
//! every relationship. The reference's `getData` mutates the DB's own objects; the port builds new
//! ones from the model, which holds the same values.
//!
//! An endpoint that names no requirement and no element is `undefined` in the reference, and the
//! layout fails on it inside ELK. The port keeps the empty name, and the render refuses it
//! (`requirement/mod.rs`); the differential compares the empty name with the reference's `null`.

use ferrigura_model::flowchart::LabelType;
use ferrigura_model::requirement::{
    Direction, Element, RelationshipKind, Requirement, RequirementDiagram, RequirementKind,
};

use crate::context::Globals;
use crate::unified::data::RequirementFields;
pub use crate::unified::data::{Edge, LayoutData, Node};

/// The shape every requirement and element is drawn with (`getData`, lines 298 and 310).
pub const SHAPE: &str = "requirementBox";

/// The relationship kind of `contains`, whose edge is drawn with the contains marker and no dash
/// (`getData`, lines 325-329).
const CONTAINS: &str = "contains";

/// Maps the parsed requirement diagram to the layout data, as `getData` does
/// (`requirementDb.ts` 292-342).
///
/// The configuration is read as the reference reads it: `look` from the top level, which the
/// requirement scope resolves to `neo` by default (`config.ts` 79-103).
#[must_use]
pub fn get_data(diagram: &RequirementDiagram, globals: &Globals) -> LayoutData {
    let mut nodes = Vec::with_capacity(diagram.requirements.len() + diagram.elements.len());
    for requirement in &diagram.requirements {
        nodes.push(requirement_node(requirement, nodes.len(), globals));
    }
    for element in &diagram.elements {
        nodes.push(element_node(element, nodes.len(), globals));
    }

    let edges = diagram
        .relationships
        .iter()
        .enumerate()
        .map(|(counter, relation)| {
            let kind = relation_name(relation.kind);
            let contains = kind == CONTAINS;
            Edge {
                id: format!("{}-{}-{counter}", relation.src, relation.dst),
                // A name that is no requirement and no element is `undefined` in the reference.
                start: endpoint(diagram, &relation.src),
                end: endpoint(diagram, &relation.dst),
                edge_type: "normal".to_owned(),
                label: format!("&lt;&lt;{kind}&gt;&gt;"),
                label_type: LabelType::Markdown,
                labelpos: "c".to_owned(),
                thickness: "normal".to_owned(),
                pattern: if contains { "normal" } else { "dashed" }.to_owned(),
                classes: "relationshipLine".to_owned(),
                arrow_type_start: if contains { "requirement_contains" } else { "" }.to_owned(),
                arrow_type_end: if contains { "" } else { "requirement_arrow" }.to_owned(),
                look: globals.look.clone(),
                style: vec![
                    "fill:none".to_owned(),
                    if contains {
                        String::new()
                    } else {
                        "stroke-dasharray: 10,7".to_owned()
                    },
                ],
                ..Edge::default()
            }
        })
        .collect();

    LayoutData {
        nodes,
        edges,
        direction: direction_name(diagram.direction).to_owned(),
        default_style: None,
        default_interpolate: None,
        diagram_type: String::new(),
    }
}

/// The node of a requirement (`getData`, lines 298-308): its id is its name, its classes and
/// styles are its own, and its colour index is its place among the nodes.
fn requirement_node(requirement: &Requirement, index: usize, globals: &Globals) -> Node {
    Node {
        id: requirement.name.clone(),
        dom_id: requirement.name.clone(),
        label_type: LabelType::default(),
        padding: 0.0,
        css_styles: requirement.css_styles.clone(),
        css_classes: requirement.classes.join(" "),
        look: globals.look.clone(),
        shape: SHAPE.to_owned(),
        color_index: Some(index as u32),
        requirement: Some(RequirementFields::Requirement {
            name: requirement.name.clone(),
            kind: kind_name(requirement.kind).to_owned(),
            requirement_id: requirement.requirement_id.clone(),
            text: requirement.text.clone(),
            risk: requirement.risk.clone(),
            verify_method: requirement.verify_method.clone(),
        }),
        ..Node::default()
    }
}

/// The node of an element (`getData`, lines 310-316): the same as a requirement's, with the
/// element's own fields.
fn element_node(element: &Element, index: usize, globals: &Globals) -> Node {
    Node {
        id: element.name.clone(),
        dom_id: element.name.clone(),
        label_type: LabelType::default(),
        padding: 0.0,
        css_styles: element.css_styles.clone(),
        css_classes: element.classes.join(" "),
        look: globals.look.clone(),
        shape: SHAPE.to_owned(),
        color_index: Some(index as u32),
        requirement: Some(RequirementFields::Element {
            name: element.name.clone(),
            kind: element.kind.clone(),
            doc_ref: element.doc_ref.clone(),
        }),
        ..Node::default()
    }
}

/// The name of an endpoint: the requirement's or the element's name, which is the name itself, or
/// empty when the diagram has neither (the reference gives `undefined`).
fn endpoint(diagram: &RequirementDiagram, name: &str) -> String {
    let known = diagram
        .requirements
        .iter()
        .any(|requirement| requirement.name == name)
        || diagram.elements.iter().any(|element| element.name == name);
    if known {
        name.to_owned()
    } else {
        String::new()
    }
}

/// The relationship's keyword, as the reference writes it in its label (`relation.type`).
#[must_use]
pub fn relation_name(kind: RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Contains => "contains",
        RelationshipKind::Copies => "copies",
        RelationshipKind::Derives => "derives",
        RelationshipKind::Satisfies => "satisfies",
        RelationshipKind::Verifies => "verifies",
        RelationshipKind::Refines => "refines",
        RelationshipKind::Traces => "traces",
    }
}

/// The kind of a requirement as the DB's display string (`requirementDb.ts` 30-37).
#[must_use]
pub fn kind_name(kind: RequirementKind) -> &'static str {
    match kind {
        RequirementKind::Requirement => "Requirement",
        RequirementKind::FunctionalRequirement => "Functional Requirement",
        RequirementKind::InterfaceRequirement => "Interface Requirement",
        RequirementKind::PerformanceRequirement => "Performance Requirement",
        RequirementKind::PhysicalRequirement => "Physical Requirement",
        RequirementKind::DesignConstraint => "Design Constraint",
    }
}

/// The direction as the layout data names it (`TB`, `BT`, `RL` or `LR`).
#[must_use]
pub fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Tb => "TB",
        Direction::Bt => "BT",
        Direction::Rl => "RL",
        Direction::Lr => "LR",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_direction_names_are_the_reference_keywords() {
        assert_eq!(direction_name(Direction::Tb), "TB");
        assert_eq!(direction_name(Direction::Lr), "LR");
    }
}
