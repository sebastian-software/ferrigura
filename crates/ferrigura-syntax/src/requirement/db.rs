//! The requirement DB, `requirementDb.ts`: the state the grammar's calls build, and the model it
//! becomes. The DB keeps the texts as the grammar gives them, with the entity placeholders; the
//! header texts go through the sanitiser, as `commonDb.ts` does (`docs-port/requirement/PLAN.md`,
//! section 5).

use ferrigura_model::requirement::{
    Direction, Element, Relationship, RelationshipKind, Requirement, RequirementClass,
    RequirementDiagram, RequirementKind,
};

use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// The properties of a requirement block. Each is the value of its first line: the grammar's
/// actions run from the last line to the first, and each setter overwrites, so the first wins.
#[derive(Debug, Default)]
pub(crate) struct RequirementFields {
    pub(crate) requirement_id: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) risk: Option<String>,
    pub(crate) verify_method: Option<String>,
}

/// The properties of an element block, first line wins as for [`RequirementFields`].
#[derive(Debug, Default)]
pub(crate) struct ElementFields {
    pub(crate) kind: Option<String>,
    pub(crate) doc_ref: Option<String>,
}

/// The DB: the requirements and elements by name, the relationships, the classes, the direction and
/// the header texts. Lookups by name go to the requirements first, then to the elements.
#[derive(Debug, Default)]
pub(crate) struct RequirementDb {
    requirements: Vec<Requirement>,
    elements: Vec<Element>,
    relations: Vec<Relationship>,
    classes: Vec<RequirementClass>,
    direction: Direction,
    title: String,
    acc_title: String,
    acc_descr: String,
}

/// The first value of an optional property, as a setter that runs last keeps it.
pub(crate) fn set_first(slot: &mut Option<String>, value: String) {
    if slot.is_none() {
        *slot = Some(value);
    }
}

impl RequirementDb {
    /// `setDirection`: the last direction wins.
    pub(crate) fn set_direction(&mut self, direction: Direction) {
        self.direction = direction;
    }

    /// `setDiagramTitle`.
    pub(crate) fn set_diagram_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle`: sanitised, without leading white space.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription`: sanitised, with the white space after each line break removed.
    pub(crate) fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `addRequirement`: a new name takes the block's properties. A name that exists keeps its
    /// first block, and the properties of the second are dropped (`requirementDb.ts:126-142`).
    pub(crate) fn add_requirement(
        &mut self,
        name: String,
        kind: RequirementKind,
        fields: RequirementFields,
    ) {
        if self.requirements.iter().any(|r| r.name == name) {
            return;
        }
        self.requirements.push(Requirement {
            name,
            kind,
            requirement_id: fields.requirement_id.unwrap_or_default(),
            text: fields.text.unwrap_or_default(),
            risk: fields.risk.unwrap_or_default(),
            verify_method: fields.verify_method.unwrap_or_default(),
            classes: vec!["default".to_owned()],
            css_styles: Vec::new(),
        });
    }

    /// `addElement`: the same rule as [`Self::add_requirement`].
    pub(crate) fn add_element(&mut self, name: String, fields: ElementFields) {
        if self.elements.iter().any(|e| e.name == name) {
            return;
        }
        self.elements.push(Element {
            name,
            kind: fields.kind.unwrap_or_default(),
            doc_ref: fields.doc_ref.unwrap_or_default(),
            classes: vec!["default".to_owned()],
            css_styles: Vec::new(),
        });
    }

    /// `addRelationship`: appended, without checking that the endpoints exist.
    pub(crate) fn add_relationship(&mut self, kind: RelationshipKind, src: String, dst: String) {
        self.relations.push(Relationship { kind, src, dst });
    }

    /// `setCssStyle`: the styles go to each node in turn, and the call stops at the first name
    /// that is not a requirement or element, so the nodes after it keep their styles
    /// (`requirementDb.ts:226-240`).
    pub(crate) fn set_css_style(&mut self, ids: &[String], styles: &[String]) {
        for id in ids {
            let Some(css_styles) = self.node_styles_mut(id) else {
                return;
            };
            css_styles.extend(styles.iter().cloned());
        }
    }

    /// `setClass`: each node that exists gets each class name, and the styles of a class that is
    /// already defined (`requirementDb.ts:242-255`).
    pub(crate) fn set_class(&mut self, ids: &[String], class_names: &[String]) {
        for id in ids {
            let styles: Vec<String> = class_names
                .iter()
                .filter_map(|class| self.classes.iter().find(|c| c.id == *class))
                .flat_map(|class| class.styles.iter().cloned())
                .collect();
            let class_list = class_names.to_vec();
            if let Some((classes, css_styles)) = self.node_classes_mut(id) {
                classes.extend(class_list);
                css_styles.extend(styles);
            }
        }
    }

    /// `defineClass`: the class is created if needed, its styles are appended, and the styles
    /// go to every node that already carries the class (`requirementDb.ts:257-286`).
    pub(crate) fn define_class(&mut self, ids: &[String], styles: &[String]) {
        for id in ids {
            let index = match self.classes.iter().position(|c| c.id == *id) {
                Some(index) => index,
                None => {
                    self.classes.push(RequirementClass {
                        id: id.clone(),
                        styles: Vec::new(),
                        text_styles: Vec::new(),
                    });
                    self.classes.len() - 1
                }
            };
            for style in styles {
                let class = &mut self.classes[index];
                if style.contains("color") {
                    class.text_styles.push(style.replacen("fill", "bgFill", 1));
                }
                class.styles.push(style.clone());
            }
            for requirement in &mut self.requirements {
                if requirement.classes.contains(id) {
                    requirement.css_styles.extend(styles.iter().cloned());
                }
            }
            for element in &mut self.elements {
                if element.classes.contains(id) {
                    element.css_styles.extend(styles.iter().cloned());
                }
            }
        }
    }

    /// The style list of the requirement or element called `id`, requirements first.
    fn node_styles_mut(&mut self, id: &str) -> Option<&mut Vec<String>> {
        if let Some(requirement) = self.requirements.iter_mut().find(|r| r.name == id) {
            return Some(&mut requirement.css_styles);
        }
        self.elements
            .iter_mut()
            .find(|e| e.name == id)
            .map(|element| &mut element.css_styles)
    }

    /// The class list and the style list of the requirement or element called `id`.
    fn node_classes_mut(&mut self, id: &str) -> Option<(&mut Vec<String>, &mut Vec<String>)> {
        if let Some(requirement) = self.requirements.iter_mut().find(|r| r.name == id) {
            return Some((&mut requirement.classes, &mut requirement.css_styles));
        }
        self.elements
            .iter_mut()
            .find(|e| e.name == id)
            .map(|element| (&mut element.classes, &mut element.css_styles))
    }

    /// The model: the DB's state after the parse.
    pub(crate) fn into_model(self) -> RequirementDiagram {
        let mut model = RequirementDiagram::new();
        model.direction = self.direction;
        model.requirements = self.requirements;
        model.elements = self.elements;
        model.relationships = self.relations;
        model.classes = self.classes;
        model.title = self.title;
        model.acc_title = self.acc_title;
        model.acc_descr = self.acc_descr;
        model
    }
}
