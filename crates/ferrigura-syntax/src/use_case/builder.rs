//! The model builder of the use case diagram (`usecaseModelBuilder.ts`).
//!
//! The visitor hands the drafts of the diagram to the builder in source order, and `finalize`
//! checks the whole diagram and builds the model and the graph AST. `finalize` runs the reference's
//! steps in their order (`docs-port/usecase/NOTES.md` section 3.1), and the first failing check
//! decides the error, so that order is part of the behaviour. The comments name the reference lines
//! each check ports.

use std::collections::BTreeMap;

use ferrigura_model::CANONICAL_VERSION;
use ferrigura_model::use_case::{
    Actor, ActorType, Animation, BoundaryType, ClassDef, Direction, EdgeOccurrence, GraphAst,
    GraphStatement, JsonNode, Note, Relationship, RelationshipType, Span, StatementKind,
    SystemBoundary, UseCase, UseCaseDiagram, UseCaseShape,
};
use ferrigura_model::value::Value;

use super::ast::{AstInput, build_graph_ast};
use super::draft::{
    Boundary, ClassAssignment, ClassDefDraft, Element, ElementKind, Endpoint, JsonDraft, Label,
    Location, Metadata, MetadataAssignment, MetadataProperty, MetadataValue, NoteDraft,
    RelationshipDraft, StyleAssignment, SymbolKind,
};
use super::json::json_rows;
use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// A failed check of the diagram: the error the parser reports at `offset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuildError {
    /// The UTF-16 code unit offset of the element or statement that caused the error.
    pub(crate) offset: usize,
    /// A short description of the failed check.
    pub(crate) message: String,
}

/// Collects the drafts of a use case diagram, and builds its model and graph AST in `finalize`.
#[derive(Debug, Default)]
pub(crate) struct UsecaseModelBuilder {
    source: String,
    acc_title: String,
    acc_description: String,
    direction: Option<Direction>,
    elements: Vec<Element>,
    boundaries: Vec<Boundary>,
    json: Vec<JsonDraft>,
    relationships: Vec<RelationshipDraft>,
    notes: Vec<NoteDraft>,
    metadata: Vec<MetadataAssignment>,
    class_defs: Vec<ClassDefDraft>,
    class_assignments: Vec<ClassAssignment>,
    style_assignments: Vec<StyleAssignment>,
}

impl UsecaseModelBuilder {
    /// Creates an empty builder for `source`, the prepared text the AST is read from.
    pub(crate) fn new(source: &str) -> Self {
        Self {
            source: source.to_owned(),
            ..Self::default()
        }
    }

    /// Stores the accessibility title as the visitor trimmed it. `finalize` sanitises it.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = text.to_owned();
    }

    /// Stores the accessibility description as the visitor trimmed it. `finalize` sanitises it.
    pub(crate) fn set_acc_description(&mut self, text: &str) {
        self.acc_description = text.to_owned();
    }

    /// Stores the direction of a direction statement; the last one wins. `TD` is stored as `TB`
    /// (`setDirection`, reference 237-239). The span is accepted for the call sites; the model
    /// keeps no location of a direction.
    pub(crate) fn set_direction(&mut self, text: &str, _span: Span) {
        let direction = match text {
            "TD" | "TB" => Direction::Tb,
            "BT" => Direction::Bt,
            "RL" => Direction::Rl,
            "LR" => Direction::Lr,
            _ => return,
        };
        self.direction = Some(direction);
    }

    /// Adds an actor or a use case declaration.
    pub(crate) fn add_element(&mut self, element: Element) {
        self.elements.push(element);
    }

    /// Adds a relationship, as written.
    pub(crate) fn add_relationship(&mut self, relationship: RelationshipDraft) {
        self.relationships.push(relationship);
    }

    /// Adds a system boundary declaration.
    pub(crate) fn add_boundary(&mut self, boundary: Boundary) {
        self.boundaries.push(boundary);
    }

    /// Adds a JSON table declaration.
    pub(crate) fn add_json(&mut self, json: JsonDraft) {
        self.json.push(json);
    }

    /// Adds a note.
    pub(crate) fn add_note(&mut self, note: NoteDraft) {
        self.notes.push(note);
    }

    /// Adds a metadata assignment to an existing id, or to an edge.
    pub(crate) fn add_metadata_assignment(&mut self, assignment: MetadataAssignment) {
        self.metadata.push(assignment);
    }

    /// Adds a class definition.
    pub(crate) fn add_class_def(&mut self, def: ClassDefDraft) {
        self.class_defs.push(def);
    }

    /// Adds a class assignment to one or more targets.
    pub(crate) fn add_class_assignment(&mut self, assignment: ClassAssignment) {
        self.class_assignments.push(assignment);
    }

    /// Adds a style assignment to one target.
    pub(crate) fn add_style_assignment(&mut self, assignment: StyleAssignment) {
        self.style_assignments.push(assignment);
    }

    /// Checks the whole diagram and builds its model and graph AST, running the reference's steps
    /// (1) to (14) of `finalize` (`usecaseModelBuilder.ts` 280-356) in order. The statements are
    /// the top-level statements of the parse; the metadata statement of an edge is changed to the
    /// `edgeMetadata` kind, as the reference does. Returns the first failed check.
    pub(crate) fn finalize(
        self,
        mut statements: Vec<GraphStatement>,
        header_span: Span,
    ) -> Result<UseCaseDiagram, BuildError> {
        let first = first_offsets(
            &self.relationships,
            &self.elements,
            &self.boundaries,
            &self.json,
        );
        let mut resolution = Resolution {
            first,
            ..Resolution::default()
        };
        // (1) and (2): the declarations, in source order. (3), the materialising of the elements,
        // boundaries and JSON tables, has no check; the entries are built when the model is.
        resolution.collect_declarations(
            &self.elements,
            &self.boundaries,
            &self.json,
            &self.relationships,
        )?;
        // (4): the relationships and their endpoints.
        resolution.resolve_relationships(&self.relationships)?;
        // (6): the metadata assignments and the animation of the edges.
        resolution.apply_metadata(&self.metadata, &mut statements)?;
        // (7): the elements, then (8): the boundaries and their members.
        resolution.validate_elements()?;
        resolution.refresh_boundaries(&self.elements)?;
        // (9): the notes.
        resolution.materialize_notes(&self.notes)?;
        // (10) and (11): the class definitions, then the classes and styles.
        resolution.apply_class_defs(&self.class_defs);
        resolution.apply_classes_and_styles(&self.class_assignments, &self.style_assignments)?;
        // (12): the direction, the last one wins; (13): the symbols.
        let direction = self.direction.unwrap_or(Direction::Lr);
        let symbols = resolution.symbol_kinds();
        // The accessibility texts go through the common setters at commit (`commit`, 81-95).
        let acc_title = strip_leading_space(&sanitize_text(&self.acc_title)).to_owned();
        let acc_description = newline_then_space_to_newline(&sanitize_text(&self.acc_description));
        let mut diagram = resolution.into_diagram(direction, acc_title, acc_description);
        // (14): the graph AST, from the finished model.
        // The reference builds the AST before the commit, so it reads the accessibility texts as the
        // visitor gave them, not as the common setters store them.
        let mut ast_view = diagram.clone();
        ast_view.acc_title = self.acc_title.clone();
        ast_view.acc_description = self.acc_description.clone();
        diagram.ast = build_graph_ast(AstInput {
            source: &self.source,
            header_span,
            statements: &statements,
            diagram: &ast_view,
            symbols: &symbols,
        });
        Ok(diagram)
    }
}

/// The kind and origin of an id the diagram declares (`symbols`, `SymbolOrigin`).
#[derive(Debug, Clone, Copy)]
struct Origin {
    kind: SymbolKind,
    generated: bool,
}

/// An actor or a use case while the checks run. Its model entry is built from it at the end.
#[derive(Debug, Clone)]
struct ElementState {
    kind: ElementKind,
    id: String,
    label: Label,
    location: Location,
    parent_id: Option<String>,
    parent_location: Option<Location>,
    shape: Option<UseCaseShape>,
    stereotype: Option<String>,
    classes: Vec<String>,
    styles: Vec<String>,
    actor_type: Option<ActorType>,
    icon: Option<String>,
    business: Option<bool>,
}

impl ElementState {
    /// An element with no metadata, classes or styles yet.
    fn new(kind: ElementKind, id: &str, label: &Label, location: &Location) -> Self {
        Self {
            kind,
            id: id.to_owned(),
            label: label.clone(),
            location: location.clone(),
            parent_id: None,
            parent_location: None,
            shape: None,
            stereotype: None,
            classes: Vec::new(),
            styles: Vec::new(),
            actor_type: None,
            icon: None,
            business: None,
        }
    }

    /// The state a first declaration starts from (`collectElement`, reference 382-397).
    fn from_draft(draft: &Element) -> Self {
        Self {
            parent_id: draft.parent_id.clone(),
            parent_location: draft.parent_location.clone(),
            shape: draft.shape,
            stereotype: draft.stereotype.clone(),
            classes: draft.classes.clone(),
            ..Self::new(draft.kind, &draft.id, &draft.label, &draft.location)
        }
    }

    /// The state of an endpoint that is not declared: an ellipse use case (`resolveEndpoint`,
    /// reference 581-597).
    fn from_endpoint(endpoint: &Endpoint) -> Self {
        Self {
            shape: Some(UseCaseShape::Ellipse),
            ..Self::new(
                ElementKind::UseCase,
                &endpoint.id,
                &endpoint.label,
                &endpoint.location,
            )
        }
    }
}

/// A system boundary while the checks run.
#[derive(Debug, Clone)]
struct BoundaryState {
    id: String,
    label: Label,
    boundary_type: Option<BoundaryType>,
    classes: Vec<String>,
    styles: Vec<String>,
    members: Vec<String>,
}

/// A JSON table while the checks run.
#[derive(Debug, Clone)]
struct JsonState {
    id: String,
    value: Value,
    property_order: BTreeMap<String, Vec<String>>,
    classes: Vec<String>,
    styles: Vec<String>,
}

/// The classes and styles of a target that can be styled (`getStylable`, reference 871-896).
struct StyleSlot<'a> {
    classes: &'a mut Vec<String>,
    styles: &'a mut Vec<String>,
}

/// A declaration of an id, checked in source order (`finalize` step 2).
enum Declaration<'a> {
    Element(&'a Element),
    Boundary(&'a Boundary),
    Json(&'a JsonDraft),
    Edge { id: &'a str, location: &'a Location },
}

/// The state `finalize` builds while it runs the checks. Vectors keep the order the reference's
/// `Map`s keep (insertion order); the indexes find an entry by id.
#[derive(Debug, Default)]
struct Resolution {
    first: BTreeMap<String, usize>,
    symbols: BTreeMap<String, Origin>,
    elements: Vec<ElementState>,
    element_index: BTreeMap<String, usize>,
    boundaries: Vec<BoundaryState>,
    boundary_index: BTreeMap<String, usize>,
    json: Vec<JsonState>,
    json_index: BTreeMap<String, usize>,
    relationships: Vec<Relationship>,
    edge_index: BTreeMap<String, usize>,
    notes: Vec<Note>,
    class_defs: Vec<ClassDef>,
}

impl Resolution {
    /// Step (2): checks the declarations in the order of their source offset (`finalize` 290-322).
    /// Elements, boundaries and JSON tables go through `collectElement` (358-441),
    /// `collectBoundary` (443-495) and `registerUnique` (1014-1032).
    fn collect_declarations(
        &mut self,
        elements: &[Element],
        boundaries: &[Boundary],
        json: &[JsonDraft],
        relationships: &[RelationshipDraft],
    ) -> Result<(), BuildError> {
        let mut declarations: Vec<(usize, Declaration<'_>)> = Vec::new();
        declarations.extend(
            elements
                .iter()
                .map(|element| (element.location.span[0], Declaration::Element(element))),
        );
        declarations.extend(
            boundaries
                .iter()
                .map(|boundary| (boundary.location.span[0], Declaration::Boundary(boundary))),
        );
        declarations.extend(
            json.iter()
                .map(|draft| (draft.location.span[0], Declaration::Json(draft))),
        );
        declarations.extend(relationships.iter().filter_map(|relationship| {
            match (
                &relationship.explicit_id,
                &relationship.explicit_id_location,
            ) {
                (Some(id), Some(location)) => {
                    Some((location.span[0], Declaration::Edge { id, location }))
                }
                _ => None,
            }
        }));
        declarations.sort_by_key(|(offset, _)| *offset);
        for (_, declaration) in declarations {
            match declaration {
                Declaration::Element(element) => self.collect_element(element)?,
                Declaration::Boundary(boundary) => self.collect_boundary(boundary)?,
                Declaration::Json(draft) => self.collect_json(draft)?,
                Declaration::Edge { id, location } => {
                    self.register_unique(id, SymbolKind::Edge, location)?;
                }
            }
        }
        Ok(())
    }

    /// Checks one actor or use case declaration (`collectElement`, reference 358-441).
    fn collect_element(&mut self, draft: &Element) -> Result<(), BuildError> {
        let kind = symbol_of(draft.kind);
        if let Some(origin) = self.symbols.get(&draft.id).copied() {
            if origin.kind != kind {
                return Err(error_at(
                    &draft.location,
                    format!(
                        "ID '{}' is declared as both {} and {}",
                        draft.id,
                        kind_name(origin.kind),
                        kind_name(kind)
                    ),
                ));
            }
            if origin.generated || draft.generated {
                return Err(collision(&draft.location, &draft.id));
            }
        }
        match self.element_index.get(&draft.id).copied() {
            Some(index) => self.merge_element(index, draft),
            None => self.insert_element(draft),
        }
    }

    /// A later declaration of an element: the labels, shapes, boundaries and stereotypes must agree
    /// (reference 410-433), then the classes and the metadata are added (reference 434-436).
    fn merge_element(&mut self, index: usize, draft: &Element) -> Result<(), BuildError> {
        let state = &mut self.elements[index];
        if state.label.text != draft.label.text || state.label.kind != draft.label.kind {
            return Err(error_at(
                &draft.location,
                format!("ID '{}' has conflicting labels", draft.id),
            ));
        }
        if let (Some(shape), Some(current)) = (draft.shape, state.shape)
            && shape != current
        {
            return Err(error_at(
                &draft.location,
                format!("Use case '{}' has conflicting shapes", draft.id),
            ));
        }
        if let (Some(parent), Some(current)) = (&draft.parent_id, &state.parent_id)
            && parent != current
        {
            let location = draft.parent_location.as_ref().unwrap_or(&draft.location);
            return Err(error_at(
                location,
                format!(
                    "Element '{}' belongs to more than one system boundary",
                    draft.id
                ),
            ));
        }
        if let (Some(stereotype), Some(current)) = (&draft.stereotype, &state.stereotype)
            && stereotype != current
        {
            return Err(error_at(
                &draft.location,
                format!("Element '{}' has conflicting stereotypes", draft.id),
            ));
        }
        state.shape = state.shape.or(draft.shape);
        if state.parent_id.is_none() {
            state.parent_id = draft.parent_id.clone();
        }
        if state.parent_location.is_none() {
            state.parent_location = draft.parent_location.clone();
        }
        if state.stereotype.is_none() {
            state.stereotype = draft.stereotype.clone();
        }
        push_unique(&mut state.classes, &draft.classes);
        apply_declared_metadata(state, draft.metadata.as_ref())
    }

    /// The first declaration of an element: its symbol and its state.
    fn insert_element(&mut self, draft: &Element) -> Result<(), BuildError> {
        let mut state = ElementState::from_draft(draft);
        apply_declared_metadata(&mut state, draft.metadata.as_ref())?;
        self.symbols.insert(
            draft.id.clone(),
            Origin {
                kind: symbol_of(draft.kind),
                generated: draft.generated,
            },
        );
        self.push_element(state);
        Ok(())
    }

    /// Checks one system boundary declaration (`collectBoundary`, reference 443-495).
    fn collect_boundary(&mut self, draft: &Boundary) -> Result<(), BuildError> {
        if let Some(origin) = self.symbols.get(&draft.id).copied() {
            if origin.kind != SymbolKind::Boundary {
                return Err(error_at(
                    &draft.location,
                    format!(
                        "ID '{}' is declared as both {} and boundary",
                        draft.id,
                        kind_name(origin.kind)
                    ),
                ));
            }
            if origin.generated || draft.generated {
                return Err(collision(&draft.location, &draft.id));
            }
        }
        if let Some(index) = self.boundary_index.get(&draft.id).copied() {
            let existing = &mut self.boundaries[index];
            if existing.label.text != draft.label.text || existing.label.kind != draft.label.kind {
                return Err(error_at(
                    &draft.location,
                    format!("Boundary '{}' has conflicting titles", draft.id),
                ));
            }
            push_unique(&mut existing.classes, &draft.classes);
            return Ok(());
        }
        self.boundary_index
            .insert(draft.id.clone(), self.boundaries.len());
        self.boundaries.push(BoundaryState {
            id: draft.id.clone(),
            label: draft.label.clone(),
            boundary_type: None,
            classes: draft.classes.clone(),
            styles: Vec::new(),
            members: Vec::new(),
        });
        self.symbols.insert(
            draft.id.clone(),
            Origin {
                kind: SymbolKind::Boundary,
                generated: draft.generated,
            },
        );
        Ok(())
    }

    /// Registers one JSON table (`registerUnique`, reference 1014-1032) and keeps its state.
    fn collect_json(&mut self, draft: &JsonDraft) -> Result<(), BuildError> {
        self.register_unique(&draft.id, SymbolKind::Json, &draft.location)?;
        self.json_index.insert(draft.id.clone(), self.json.len());
        self.json.push(JsonState {
            id: draft.id.clone(),
            value: draft.value.clone(),
            property_order: draft.property_order.clone(),
            classes: draft.classes.clone(),
            styles: Vec::new(),
        });
        Ok(())
    }

    /// Registers an id that may be declared once only: a JSON table or an explicit edge id
    /// (`registerUnique`, reference 1014-1032).
    fn register_unique(
        &mut self,
        id: &str,
        kind: SymbolKind,
        location: &Location,
    ) -> Result<(), BuildError> {
        if self.symbols.contains_key(id) {
            return Err(error_at(
                location,
                format!("ID '{id}' is declared more than once"),
            ));
        }
        self.symbols.insert(
            id.to_owned(),
            Origin {
                kind,
                generated: false,
            },
        );
        Ok(())
    }

    /// Step (4): resolves the endpoints, validates each relationship and stores it. The anonymous
    /// edges are numbered `edge-N` in source order (`materializeRelationships`, 535-565).
    fn resolve_relationships(&mut self, drafts: &[RelationshipDraft]) -> Result<(), BuildError> {
        let mut anonymous = 0_usize;
        for draft in drafts {
            let source = self.resolve_endpoint(&draft.source)?;
            let target = self.resolve_endpoint(&draft.target)?;
            validate_relationship(draft, source, target)?;
            let id = match &draft.explicit_id {
                Some(id) => id.clone(),
                None => {
                    let id = format!("edge-{anonymous}");
                    anonymous += 1;
                    id
                }
            };
            // A later relationship with the same id takes the entry, as the reference's `Map` does.
            self.edge_index.insert(id.clone(), self.relationships.len());
            self.relationships.push(relationship_of(draft, id));
        }
        Ok(())
    }

    /// Resolves one endpoint (`resolveEndpoint`, reference 566-598): an id that is not declared
    /// becomes an ellipse use case, and `:::` needs a declaration on the same reference.
    fn resolve_endpoint(&mut self, endpoint: &Endpoint) -> Result<SymbolKind, BuildError> {
        if endpoint.classes_on_reference && !endpoint.declaration {
            return Err(error_at(
                &endpoint.location,
                format!(
                    "Relationship endpoint '{}' uses ::: without declaring it",
                    endpoint.id
                ),
            ));
        }
        if let Some(origin) = self.symbols.get(&endpoint.id) {
            return Ok(origin.kind);
        }
        self.push_element(ElementState::from_endpoint(endpoint));
        self.symbols.insert(
            endpoint.id.clone(),
            Origin {
                kind: SymbolKind::UseCase,
                generated: endpoint.generated,
            },
        );
        Ok(SymbolKind::UseCase)
    }

    /// Adds an element state and indexes it by id.
    fn push_element(&mut self, state: ElementState) {
        self.element_index
            .insert(state.id.clone(), self.elements.len());
        self.elements.push(state);
    }

    /// The element state of an id, for a metadata assignment.
    fn element_mut(
        &mut self,
        id: &str,
        location: &Location,
    ) -> Result<&mut ElementState, BuildError> {
        let index = self
            .element_index
            .get(id)
            .copied()
            .ok_or_else(|| undeclared(location, id))?;
        Ok(&mut self.elements[index])
    }

    /// The boundary state of an id, for a metadata assignment.
    fn boundary_mut(
        &mut self,
        id: &str,
        location: &Location,
    ) -> Result<&mut BoundaryState, BuildError> {
        let index = self
            .boundary_index
            .get(id)
            .copied()
            .ok_or_else(|| undeclared(location, id))?;
        Ok(&mut self.boundaries[index])
    }

    /// Step (6): applies the metadata assignments (`applyMetadataAssignments`, reference 620-688),
    /// then sets `animate` on every edge with an animation (the last loop of that function).
    fn apply_metadata(
        &mut self,
        assignments: &[MetadataAssignment],
        statements: &mut [GraphStatement],
    ) -> Result<(), BuildError> {
        for assignment in assignments {
            let target = assignment.target.as_str();
            let kind = self
                .symbols
                .get(target)
                .map(|origin| origin.kind)
                .ok_or_else(|| unresolved_metadata(assignment))?;
            match kind {
                SymbolKind::Actor | SymbolKind::UseCase => {
                    let state = self.element_mut(target, &assignment.target_location)?;
                    apply_element_metadata(state, &assignment.metadata, true)?;
                }
                SymbolKind::Boundary => {
                    let state = self.boundary_mut(target, &assignment.target_location)?;
                    apply_boundary_metadata(state, &assignment.metadata)?;
                }
                SymbolKind::Edge => {
                    let index = self.edge_index.get(target).copied().ok_or_else(|| {
                        error_at(
                            &assignment.target_location,
                            format!("Metadata target '{target}' is not an explicit edge"),
                        )
                    })?;
                    mark_edge_statement(statements, target, &assignment.target_location)?;
                    let relationship = &mut self.relationships[index];
                    apply_edge_metadata(relationship, target, &assignment.metadata)?;
                }
                SymbolKind::Json => {
                    if let Some(property) = assignment.metadata.properties.first() {
                        return Err(invalid_metadata(target, "JSON table", property));
                    }
                }
            }
        }
        for index in self.edge_index.values() {
            let relationship = &mut self.relationships[*index];
            if relationship.animation.is_some() {
                relationship.animate = true;
            }
        }
        Ok(())
    }

    /// Step (7): the checks of each element after the metadata (`validateAndRefreshElements`,
    /// reference 760-781).
    fn validate_elements(&self) -> Result<(), BuildError> {
        self.elements.iter().try_for_each(validate_element)
    }

    /// Step (8): the members of the boundaries and their parents (`refreshBoundaries`, 782-819).
    /// The members follow the declaration order. An unresolved parent is reported at the first
    /// declaration that has it (reference 797-810).
    fn refresh_boundaries(&mut self, elements: &[Element]) -> Result<(), BuildError> {
        for boundary in &mut self.boundaries {
            boundary.members.clear();
        }
        let mut ordered: Vec<&Element> = elements.iter().collect();
        ordered.sort_by_key(|draft| draft.location.span[0]);
        for draft in ordered {
            let Some(parent) = &draft.parent_id else {
                continue;
            };
            let Some(index) = self.boundary_index.get(parent).copied() else {
                let location = draft.parent_location.as_ref().unwrap_or(&draft.location);
                return Err(unresolved_parent(location, parent, &draft.id));
            };
            let members = &mut self.boundaries[index].members;
            if !members.contains(&draft.id) {
                members.push(draft.id.clone());
            }
        }
        for state in &self.elements {
            if let Some(parent) = &state.parent_id
                && !self.boundary_index.contains_key(parent)
            {
                let location = state.parent_location.as_ref().unwrap_or(&state.location);
                return Err(unresolved_parent(location, parent, &state.id));
            }
        }
        Ok(())
    }

    /// Step (9): the notes, numbered `note-N` in declaration order (`materializeNotes`, 820-845).
    fn materialize_notes(&mut self, drafts: &[NoteDraft]) -> Result<(), BuildError> {
        for draft in drafts {
            let origin = self.symbols.get(&draft.target).copied().ok_or_else(|| {
                error_at(
                    &draft.target_location,
                    format!("Note target '{}' is unresolved", draft.target),
                )
            })?;
            if !matches!(origin.kind, SymbolKind::Actor | SymbolKind::UseCase) {
                return Err(error_at(
                    &draft.target_location,
                    format!(
                        "Note target '{}' must be an actor or use case, not {}",
                        draft.target,
                        kind_name(origin.kind)
                    ),
                ));
            }
            self.notes.push(Note {
                id: format!("note-{}", self.notes.len()),
                target: draft.target.clone(),
                label: draft.label.text.clone(),
                label_type: draft.label.kind,
            });
        }
        Ok(())
    }

    /// Step (10): the class definitions. The last write wins per id, at the first position
    /// (`applyClassDefinitions`, reference 846-852).
    fn apply_class_defs(&mut self, definitions: &[ClassDefDraft]) {
        for definition in definitions {
            for id in &definition.ids {
                match self.class_defs.iter_mut().find(|def| def.id == *id) {
                    Some(existing) => existing.styles.clone_from(&definition.styles),
                    None => self.class_defs.push(ClassDef {
                        id: id.clone(),
                        styles: definition.styles.clone(),
                    }),
                }
            }
        }
    }

    /// Step (11): the classes, then the styles (`applyClassesAndStyles`, reference 853-896).
    /// A class is added once; a style is appended.
    fn apply_classes_and_styles(
        &mut self,
        classes: &[ClassAssignment],
        styles: &[StyleAssignment],
    ) -> Result<(), BuildError> {
        for assignment in classes {
            for (id, location) in &assignment.targets {
                let slot = self.style_target(id, location)?;
                push_unique(slot.classes, &assignment.classes);
            }
        }
        for assignment in styles {
            let slot = self.style_target(&assignment.target, &assignment.location)?;
            slot.styles.extend(assignment.styles.iter().cloned());
        }
        Ok(())
    }

    /// The classes and styles of a target (`getStylable`, reference 871-896). An anonymous edge or
    /// an undeclared id has none, and is an error.
    fn style_target(&mut self, id: &str, location: &Location) -> Result<StyleSlot<'_>, BuildError> {
        let kind = self.symbols.get(id).map(|origin| origin.kind);
        let slot = match kind {
            Some(SymbolKind::Actor | SymbolKind::UseCase) => {
                match self.element_index.get(id).copied() {
                    Some(index) => {
                        let state = &mut self.elements[index];
                        Some(StyleSlot {
                            classes: &mut state.classes,
                            styles: &mut state.styles,
                        })
                    }
                    None => None,
                }
            }
            Some(SymbolKind::Boundary) => match self.boundary_index.get(id).copied() {
                Some(index) => {
                    let state = &mut self.boundaries[index];
                    Some(StyleSlot {
                        classes: &mut state.classes,
                        styles: &mut state.styles,
                    })
                }
                None => None,
            },
            Some(SymbolKind::Json) => match self.json_index.get(id).copied() {
                Some(index) => {
                    let state = &mut self.json[index];
                    Some(StyleSlot {
                        classes: &mut state.classes,
                        styles: &mut state.styles,
                    })
                }
                None => None,
            },
            Some(SymbolKind::Edge) => match self.edge_index.get(id).copied() {
                Some(index) => {
                    let relationship = &mut self.relationships[index];
                    Some(StyleSlot {
                        classes: &mut relationship.classes,
                        styles: &mut relationship.styles,
                    })
                }
                None => None,
            },
            None => None,
        };
        slot.ok_or_else(|| {
            error_at(
                location,
                format!("Class/style target '{id}' is unresolved or anonymous"),
            )
        })
    }

    /// The kind of each id the diagram declares, for the graph AST.
    fn symbol_kinds(&self) -> BTreeMap<String, SymbolKind> {
        self.symbols
            .iter()
            .map(|(id, origin)| (id.clone(), origin.kind))
            .collect()
    }

    /// Builds the model from the finished states (`reorderElements`, 599-618, and the entries of
    /// `setElementModel`, 945-975). Each array keeps the order of the first source offset of its ids.
    fn into_diagram(
        self,
        direction: Direction,
        acc_title: String,
        acc_description: String,
    ) -> UseCaseDiagram {
        let Self {
            first,
            mut elements,
            mut boundaries,
            mut json,
            relationships,
            notes,
            class_defs,
            ..
        } = self;
        elements.sort_by_key(|state| first_offset(&first, &state.id));
        boundaries.sort_by_key(|state| first_offset(&first, &state.id));
        json.sort_by_key(|state| first_offset(&first, &state.id));
        let actors: Vec<Actor> = elements
            .iter()
            .filter(|state| state.kind == ElementKind::Actor)
            .map(actor_of)
            .collect();
        let use_cases: Vec<UseCase> = elements
            .iter()
            .filter(|state| state.kind == ElementKind::UseCase)
            .map(use_case_of)
            .collect();
        let system_boundaries: Vec<SystemBoundary> = boundaries.iter().map(boundary_of).collect();
        let json_nodes: Vec<JsonNode> = json.iter().map(json_of).collect();
        UseCaseDiagram {
            version: CANONICAL_VERSION,
            title: String::new(),
            direction,
            acc_title,
            acc_description,
            actors,
            use_cases,
            system_boundaries,
            relationships,
            notes,
            json_nodes,
            class_defs,
            ast: GraphAst::empty(),
        }
    }
}

/// Records the first source offset of each id that a relationship endpoint, an element, a boundary
/// or a JSON table mentions (`finalize` step 1, reference 281-291).
fn first_offsets(
    relationships: &[RelationshipDraft],
    elements: &[Element],
    boundaries: &[Boundary],
    json: &[JsonDraft],
) -> BTreeMap<String, usize> {
    let mut first = BTreeMap::new();
    for relationship in relationships {
        record_first(
            &mut first,
            &relationship.source.id,
            relationship.source.location.span[0],
        );
        record_first(
            &mut first,
            &relationship.target.id,
            relationship.target.location.span[0],
        );
    }
    for element in elements {
        record_first(&mut first, &element.id, element.location.span[0]);
    }
    for boundary in boundaries {
        record_first(&mut first, &boundary.id, boundary.location.span[0]);
    }
    for draft in json {
        record_first(&mut first, &draft.id, draft.location.span[0]);
    }
    first
}

/// Keeps the smaller of the recorded offset and `offset` for `id` (`recordFirst`).
fn record_first(first: &mut BTreeMap<String, usize>, id: &str, offset: usize) {
    first
        .entry(id.to_owned())
        .and_modify(|current| *current = (*current).min(offset))
        .or_insert(offset);
}

/// The first offset of an id, for the sorts of the model (`first.get(id) ?? 0`).
fn first_offset(first: &BTreeMap<String, usize>, id: &str) -> usize {
    first.get(id).copied().unwrap_or(0)
}

/// Checks the endpoints and the form of a relationship (`validateRelationship`, reference 897-944).
fn validate_relationship(
    draft: &RelationshipDraft,
    source: SymbolKind,
    target: SymbolKind,
) -> Result<(), BuildError> {
    let allowed = |kind: SymbolKind| {
        matches!(
            kind,
            SymbolKind::Actor | SymbolKind::UseCase | SymbolKind::Json
        )
    };
    if !allowed(source) {
        return Err(error_at(
            &draft.source.location,
            format!(
                "Relationship source '{}' cannot be {}",
                draft.source.id,
                kind_name(source)
            ),
        ));
    }
    if !allowed(target) {
        return Err(error_at(
            &draft.target.location,
            format!(
                "Relationship target '{}' cannot be {}",
                draft.target.id,
                kind_name(target)
            ),
        ));
    }
    if matches!(
        draft.relationship_type,
        RelationshipType::Include | RelationshipType::Extend
    ) && (source != SymbolKind::UseCase || target != SymbolKind::UseCase)
    {
        let name = if draft.relationship_type == RelationshipType::Include {
            "include"
        } else {
            "extend"
        };
        return Err(error_at(
            &draft.location,
            format!("{name} relationship requires use-case endpoints"),
        ));
    }
    if draft.relationship_type == RelationshipType::Generalization
        && (!matches!(source, SymbolKind::Actor | SymbolKind::UseCase) || source != target)
    {
        return Err(error_at(
            &draft.location,
            "Generalization requires actor-to-actor or use-case-to-use-case endpoints".to_owned(),
        ));
    }
    if draft.relationship_type == RelationshipType::Association
        && (source == SymbolKind::Json || target == SymbolKind::Json)
        && !matches!(draft.arrow_type, 0..=2)
    {
        return Err(error_at(
            &draft.location,
            format!(
                "JSON relationship '{}' to '{}' permits only point, reversed-point, or markerless solid association",
                draft.source.id, draft.target.id
            ),
        ));
    }
    Ok(())
}

/// The model entry of a relationship, before its metadata is applied.
fn relationship_of(draft: &RelationshipDraft, id: String) -> Relationship {
    Relationship {
        id,
        explicit_id: draft.explicit_id.is_some(),
        source: draft.source.id.clone(),
        target: draft.target.id.clone(),
        relationship_type: draft.relationship_type,
        arrow_type: draft.arrow_type,
        label: draft.label.as_ref().map(|label| label.text.clone()),
        label_type: draft.label.as_ref().map(|label| label.kind),
        minlen: draft.minlen,
        classes: Vec::new(),
        styles: Vec::new(),
        animate: false,
        animation: None,
    }
}

/// Applies the metadata of an element: with `replace` (a standalone assignment) the values replace
/// the ones set, without it (a declaration) a different value is an error (reference 689-759).
fn apply_element_metadata(
    state: &mut ElementState,
    metadata: &Metadata,
    replace: bool,
) -> Result<(), BuildError> {
    for property in &metadata.properties {
        match state.kind {
            ElementKind::Actor => apply_actor_property(state, property, replace)?,
            ElementKind::UseCase => apply_use_case_property(state, property, replace)?,
        }
    }
    Ok(())
}

/// The metadata of a declaration (`applyDeclarationMetadata`, reference 708-759).
fn apply_declared_metadata(
    state: &mut ElementState,
    metadata: Option<&Metadata>,
) -> Result<(), BuildError> {
    match metadata {
        Some(metadata) => apply_element_metadata(state, metadata, false),
        None => Ok(()),
    }
}

/// One metadata property of an actor: `type`, `icon` or `business` (reference 721-759).
fn apply_actor_property(
    state: &mut ElementState,
    property: &MetadataProperty,
    replace: bool,
) -> Result<(), BuildError> {
    match (property.key.as_str(), &property.value) {
        ("type", MetadataValue::Text(text)) => {
            let Some(glyph) = glyph_of(text) else {
                return Err(invalid_metadata(&state.id, "actor", property));
            };
            if is_conflict(&state.actor_type, &glyph, replace) {
                return Err(conflict_error(property, "Actor", &state.id, "type"));
            }
            state.actor_type = Some(glyph);
            Ok(())
        }
        ("icon", MetadataValue::Text(icon)) => {
            if is_conflict(&state.icon, icon, replace) {
                return Err(conflict_error(property, "Actor", &state.id, "icon"));
            }
            state.icon = Some(icon.clone());
            Ok(())
        }
        ("business", MetadataValue::Bool(flag)) => {
            if is_conflict(&state.business, flag, replace) {
                return Err(conflict_error(property, "Actor", &state.id, "business"));
            }
            state.business = Some(*flag);
            Ok(())
        }
        _ => Err(invalid_metadata(&state.id, "actor", property)),
    }
}

/// One metadata property of a use case: `business` only (reference 693-697, 708-714).
fn apply_use_case_property(
    state: &mut ElementState,
    property: &MetadataProperty,
    replace: bool,
) -> Result<(), BuildError> {
    match (property.key.as_str(), &property.value) {
        ("business", MetadataValue::Bool(flag)) => {
            if is_conflict(&state.business, flag, replace) {
                return Err(conflict_error(property, "Use case", &state.id, "business"));
            }
            state.business = Some(*flag);
            Ok(())
        }
        _ => Err(invalid_metadata(&state.id, "use case", property)),
    }
}

/// The metadata of a boundary: `type` as `rect` or `package` (reference 634-646).
fn apply_boundary_metadata(
    state: &mut BoundaryState,
    metadata: &Metadata,
) -> Result<(), BuildError> {
    for property in &metadata.properties {
        let boundary_type = match (property.key.as_str(), text_value(property)) {
            ("type", Some("rect")) => BoundaryType::Rect,
            ("type", Some("package")) => BoundaryType::Package,
            _ => return Err(invalid_metadata(&state.id, "boundary", property)),
        };
        state.boundary_type = Some(boundary_type);
    }
    Ok(())
}

/// The metadata of an explicit edge: `animate` (a boolean), or `animation` (`fast` or `slow`, which
/// also sets `animate`) (reference 647-676).
fn apply_edge_metadata(
    relationship: &mut Relationship,
    target: &str,
    metadata: &Metadata,
) -> Result<(), BuildError> {
    for property in &metadata.properties {
        match (property.key.as_str(), &property.value) {
            ("animate", MetadataValue::Bool(flag)) => relationship.animate = *flag,
            ("animation", MetadataValue::Text(text)) => {
                let Some(animation) = animation_of(text) else {
                    return Err(invalid_metadata(target, "edge", property));
                };
                relationship.animation = Some(animation);
                relationship.animate = true;
            }
            _ => return Err(invalid_metadata(target, "edge", property)),
        }
    }
    Ok(())
}

/// Changes the metadata statement of an edge to the `edgeMetadata` kind (reference 647-656). The
/// statement is found in the parse by the span of its target, because the copy the draft holds may
/// not carry the final span.
fn mark_edge_statement(
    statements: &mut [GraphStatement],
    target: &str,
    location: &Location,
) -> Result<(), BuildError> {
    let span = location.span;
    let statement = find_metadata_statement(statements, span).ok_or_else(|| {
        error_at(
            location,
            format!("The metadata statement of '{target}' is missing"),
        )
    })?;
    statement.kind = StatementKind::EdgeMetadata;
    statement.edges = Some(vec![EdgeOccurrence {
        id: target.to_owned(),
        span: statement.span,
        label_span: None,
        id_span: Some(span),
        metadata: statement.metadata.clone(),
        class_spans: None,
    }]);
    statement.nodes = None;
    Ok(())
}

/// The metadata statement whose target node has `target` as its id span, searched through the
/// children of the groups too.
fn find_metadata_statement(
    statements: &mut [GraphStatement],
    target: Span,
) -> Option<&mut GraphStatement> {
    for statement in statements.iter_mut() {
        let is_target = statement.kind == StatementKind::Metadata
            && statement
                .nodes
                .as_ref()
                .and_then(|nodes| nodes.first())
                .is_some_and(|node| node.id_span == target);
        if is_target {
            return Some(statement);
        }
        if let Some(children) = statement.children.as_mut()
            && let Some(found) = find_metadata_statement(children, target)
        {
            return Some(found);
        }
    }
    None
}

/// The checks of one element after its metadata is applied (`validateAndRefreshElements`,
/// reference 760-781).
fn validate_element(state: &ElementState) -> Result<(), BuildError> {
    match state.kind {
        ElementKind::Actor => {
            let has_icon = is_set(&state.icon);
            if has_icon
                && let Some(glyph) = state.actor_type
                && glyph != ActorType::Normal
            {
                return Err(error_at(
                    &state.location,
                    format!(
                        "Actor '{}' cannot combine icon with type '{}'",
                        state.id,
                        glyph_name(glyph)
                    ),
                ));
            }
            let glyph = if has_icon {
                ActorType::Icon
            } else {
                state.actor_type.unwrap_or(ActorType::Normal)
            };
            if state.business == Some(true) && matches!(glyph, ActorType::Icon | ActorType::Awesome)
            {
                return Err(error_at(
                    &state.location,
                    format!(
                        "Business actor '{}' must use normal or hollow geometry",
                        state.id
                    ),
                ));
            }
        }
        ElementKind::UseCase => {
            let shape = state.shape.unwrap_or(UseCaseShape::Ellipse);
            if shape == UseCaseShape::Rect && state.business == Some(true) {
                return Err(error_at(
                    &state.location,
                    format!(
                        "Rectangular use case '{}' cannot be a business use case",
                        state.id
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// The kind that the metadata of an unresolved target implies (`inferMetadataKind`), for the message.
fn inferred_kind(metadata: &Metadata) -> Option<&'static str> {
    let mut possible: Vec<&'static str> = vec!["actor", "use case", "boundary", "edge"];
    for property in &metadata.properties {
        match property.key.as_str() {
            "icon" => possible = vec!["actor"],
            "animate" | "animation" => possible = vec!["edge"],
            "type" => {
                possible.clear();
                match text_value(property) {
                    Some("rect" | "package") => possible.push("boundary"),
                    Some("normal" | "hollow" | "awesome") => possible.push("actor"),
                    _ => {}
                }
            }
            "business" => possible.retain(|kind| *kind != "boundary" && *kind != "edge"),
            _ => return None,
        }
    }
    match possible.as_slice() {
        [only] => Some(*only),
        _ => None,
    }
}

/// The model entry of an actor (`setElementModel`, reference 945-975). An icon makes the glyph icon.
fn actor_of(state: &ElementState) -> Actor {
    let icon = truthy(&state.icon);
    Actor {
        id: state.id.clone(),
        label: state.label.text.clone(),
        label_type: state.label.kind,
        actor_type: if icon.is_some() {
            ActorType::Icon
        } else {
            state.actor_type.unwrap_or(ActorType::Normal)
        },
        icon,
        business: state.business.unwrap_or(false),
        stereotype: truthy(&state.stereotype),
        parent_id: truthy(&state.parent_id),
        classes: state.classes.clone(),
        styles: state.styles.clone(),
    }
}

/// The model entry of a use case (`setElementModel`, reference 945-975).
fn use_case_of(state: &ElementState) -> UseCase {
    UseCase {
        id: state.id.clone(),
        label: state.label.text.clone(),
        label_type: state.label.kind,
        shape: state.shape.unwrap_or(UseCaseShape::Ellipse),
        business: state.business.unwrap_or(false),
        stereotype: truthy(&state.stereotype),
        parent_id: truthy(&state.parent_id),
        classes: state.classes.clone(),
        styles: state.styles.clone(),
    }
}

/// The model entry of a system boundary (`materializeBoundaries`, 511-520, and `refreshBoundaries`,
/// 812-818).
fn boundary_of(state: &BoundaryState) -> SystemBoundary {
    SystemBoundary {
        id: state.id.clone(),
        label: state.label.text.clone(),
        label_type: state.label.kind,
        boundary_type: state.boundary_type.unwrap_or(BoundaryType::Rect),
        members: state.members.clone(),
        classes: state.classes.clone(),
        styles: state.styles.clone(),
    }
}

/// The model entry of a JSON table, with its display rows.
fn json_of(state: &JsonState) -> JsonNode {
    JsonNode {
        id: state.id.clone(),
        value: state.value.clone(),
        property_order: state.property_order.clone(),
        classes: state.classes.clone(),
        styles: state.styles.clone(),
        rows: json_rows(&state.value, &state.property_order),
    }
}

/// The symbol kind of an element declaration.
fn symbol_of(kind: ElementKind) -> SymbolKind {
    match kind {
        ElementKind::Actor => SymbolKind::Actor,
        ElementKind::UseCase => SymbolKind::UseCase,
    }
}

/// The name of a kind in the messages.
fn kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Actor => "actor",
        SymbolKind::UseCase => "use case",
        SymbolKind::Boundary => "boundary",
        SymbolKind::Json => "JSON table",
        SymbolKind::Edge => "edge",
    }
}

/// The name of an actor glyph in the messages.
fn glyph_name(glyph: ActorType) -> &'static str {
    match glyph {
        ActorType::Normal => "normal",
        ActorType::Hollow => "hollow",
        ActorType::Awesome => "awesome",
        ActorType::Icon => "icon",
    }
}

/// The glyph that an actor `type` metadata value names.
fn glyph_of(text: &str) -> Option<ActorType> {
    match text {
        "normal" => Some(ActorType::Normal),
        "hollow" => Some(ActorType::Hollow),
        "awesome" => Some(ActorType::Awesome),
        _ => None,
    }
}

/// The animation that an `animation` metadata value names.
fn animation_of(text: &str) -> Option<Animation> {
    match text {
        "fast" => Some(Animation::Fast),
        "slow" => Some(Animation::Slow),
        _ => None,
    }
}

/// The text of a metadata property, when its value is text.
fn text_value(property: &MetadataProperty) -> Option<&str> {
    match &property.value {
        MetadataValue::Text(text) => Some(text.as_str()),
        MetadataValue::Bool(_) => None,
    }
}

/// Whether an optional text is set and not empty (the reference's truthiness of a string).
fn is_set(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|text| !text.is_empty())
}

/// The optional text when it is set and not empty, as the model writes an optional text.
fn truthy(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// Whether a metadata value would change a value that is already set. A declaration checks it; a
/// standalone assignment replaces it.
fn is_conflict<T: PartialEq>(current: &Option<T>, value: &T, replace: bool) -> bool {
    !replace && current.as_ref().is_some_and(|current| current != value)
}

/// Adds each value that the list does not hold yet (`pushUnique`).
fn push_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}

/// The error at a location, reported at its start offset.
fn error_at(location: &Location, message: String) -> BuildError {
    BuildError {
        offset: location.span[0],
        message,
    }
}

/// The error of a generated id that collides with another declaration.
fn collision(location: &Location, id: &str) -> BuildError {
    error_at(
        location,
        format!("Generated ID '{id}' collides with another declaration"),
    )
}

/// The error of a metadata target or of a boundary that is not declared.
fn undeclared(location: &Location, id: &str) -> BuildError {
    error_at(location, format!("'{id}' is not declared"))
}

/// The error of a metadata target that is not declared, with the kind its metadata implies.
fn unresolved_metadata(assignment: &MetadataAssignment) -> BuildError {
    let hint = inferred_kind(&assignment.metadata)
        .map(|kind| format!(" (metadata implies {kind})"))
        .unwrap_or_default();
    error_at(
        &assignment.target_location,
        format!(
            "Metadata target '{}' is unresolved{hint}",
            assignment.target
        ),
    )
}

/// The error of a parent boundary that is not declared.
fn unresolved_parent(location: &Location, parent: &str, id: &str) -> BuildError {
    error_at(
        location,
        format!("Parent boundary '{parent}' for '{id}' is unresolved"),
    )
}

/// The error of a metadata property that the kind does not take.
fn invalid_metadata(id: &str, kind: &str, property: &MetadataProperty) -> BuildError {
    error_at(
        &property.location,
        format!(
            "Metadata property '{}' is invalid for {kind} '{id}'",
            property.key
        ),
    )
}

/// The error of two metadata values that disagree for one element.
fn conflict_error(property: &MetadataProperty, owner: &str, id: &str, what: &str) -> BuildError {
    error_at(
        &property.location,
        format!("{owner} '{id}' has conflicting {what} metadata"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrigura_model::use_case::{LabelType, NodeOccurrence};

    fn span_at(start: usize) -> Span {
        [start, start + 1]
    }

    fn loc(start: usize) -> Location {
        Location {
            span: span_at(start),
            line: 1,
            column: start + 1,
        }
    }

    fn label(text: &str, start: usize) -> Label {
        Label {
            text: text.to_owned(),
            kind: LabelType::Text,
            span: span_at(start),
        }
    }

    fn element(id: &str, kind: ElementKind, start: usize) -> Element {
        Element {
            id: id.to_owned(),
            kind,
            label: label(id, start),
            location: loc(start),
            generated: false,
            parent_id: None,
            parent_location: None,
            shape: None,
            metadata: None,
            stereotype: None,
            stereotype_span: None,
            classes: Vec::new(),
        }
    }

    fn actor(id: &str, start: usize) -> Element {
        element(id, ElementKind::Actor, start)
    }

    /// An actor with an icon and the hollow type, which the element check rejects at its offset.
    fn hollow_icon_actor() -> Element {
        let mut hollow = actor("A", 0);
        hollow.metadata = Some(metadata(
            vec![
                property("icon", MetadataValue::Text("x".to_owned()), 2),
                property("type", MetadataValue::Text("hollow".to_owned()), 4),
            ],
            2,
        ));
        hollow
    }

    fn endpoint(id: &str, start: usize, declaration: bool) -> Endpoint {
        Endpoint {
            id: id.to_owned(),
            label: label(id, start),
            location: loc(start),
            generated: false,
            declaration,
            classes_on_reference: false,
        }
    }

    fn relation(
        source: Endpoint,
        target: Endpoint,
        start: usize,
        kind: RelationshipType,
    ) -> RelationshipDraft {
        RelationshipDraft {
            source,
            target,
            location: loc(start),
            explicit_id: None,
            explicit_id_location: None,
            relationship_type: kind,
            arrow_type: 0,
            label: None,
            minlen: 1,
        }
    }

    fn property(key: &str, value: MetadataValue, start: usize) -> MetadataProperty {
        MetadataProperty {
            key: key.to_owned(),
            value,
            span: span_at(start),
            key_span: span_at(start),
            value_span: span_at(start),
            location: loc(start),
        }
    }

    fn metadata(properties: Vec<MetadataProperty>, start: usize) -> Metadata {
        Metadata {
            properties,
            location: loc(start),
        }
    }

    fn assignment(target: &str, start: usize, metadata: Metadata) -> MetadataAssignment {
        MetadataAssignment {
            target: target.to_owned(),
            target_location: loc(start),
            metadata,
            statement: statement(StatementKind::Metadata),
        }
    }

    fn statement(kind: StatementKind) -> GraphStatement {
        GraphStatement {
            kind,
            span: [0, 0],
            nodes: None,
            edges: None,
            group: None,
            id_span: None,
            title_span: None,
            end_span: None,
            r#ref: None,
            ref_span: None,
            children: None,
            metadata: None,
            stereotype_span: None,
            class_spans: None,
        }
    }

    fn error_of(builder: UsecaseModelBuilder) -> BuildError {
        builder
            .finalize(Vec::new(), span_at(0))
            .expect_err("the diagram is invalid")
    }

    #[test]
    fn td_is_stored_as_tb_and_the_last_direction_wins() {
        let mut builder = UsecaseModelBuilder::new("usecase\n");
        builder.set_direction("TD", span_at(8));
        let diagram = builder
            .finalize(Vec::new(), span_at(0))
            .expect("the diagram is valid");
        assert_eq!(diagram.direction, Direction::Tb);

        let mut builder = UsecaseModelBuilder::new("usecase\n");
        builder.set_direction("TD", span_at(8));
        builder.set_direction("BT", span_at(12));
        let diagram = builder
            .finalize(Vec::new(), span_at(0))
            .expect("the diagram is valid");
        assert_eq!(diagram.direction, Direction::Bt);

        let diagram = UsecaseModelBuilder::new("usecase\n")
            .finalize(Vec::new(), span_at(0))
            .expect("the diagram is valid");
        assert_eq!(diagram.direction, Direction::Lr);
    }

    #[test]
    fn undeclared_endpoint_becomes_an_ellipse_use_case() {
        let mut builder = UsecaseModelBuilder::new("usecase\n");
        builder.add_element(actor("A", 0));
        builder.add_relationship(relation(
            endpoint("A", 0, true),
            endpoint("UC1", 10, false),
            10,
            RelationshipType::Association,
        ));
        let diagram = builder
            .finalize(Vec::new(), span_at(0))
            .expect("the diagram is valid");
        assert_eq!(diagram.use_cases.len(), 1);
        let use_case = &diagram.use_cases[0];
        assert_eq!(use_case.id, "UC1");
        assert_eq!(use_case.label, "UC1");
        assert_eq!(use_case.shape, UseCaseShape::Ellipse);
        assert_eq!(diagram.relationships[0].id, "edge-0");
        assert!(!diagram.relationships[0].explicit_id);
    }

    #[test]
    fn metadata_target_check_precedes_the_element_check() {
        // The element check rejects the icon with the hollow type at offset 0.
        let mut builder = UsecaseModelBuilder::new("usecase\n");
        builder.add_element(hollow_icon_actor());
        assert_eq!(error_of(builder).offset, 0);

        // An unresolved metadata target at offset 40 is checked before the element, so it wins.
        let mut builder = UsecaseModelBuilder::new("usecase\n");
        builder.add_element(hollow_icon_actor());
        builder.add_metadata_assignment(assignment(
            "Z",
            40,
            metadata(
                vec![property("business", MetadataValue::Bool(true), 41)],
                40,
            ),
        ));
        assert_eq!(error_of(builder).offset, 40);
    }

    #[test]
    fn relationship_check_precedes_the_metadata_check() {
        let mut builder = UsecaseModelBuilder::new("usecase\n");
        builder.add_element(actor("A", 0));
        builder.add_element(actor("B", 4));
        builder.add_relationship(relation(
            endpoint("A", 0, true),
            endpoint("B", 4, true),
            20,
            RelationshipType::Include,
        ));
        builder.add_metadata_assignment(assignment(
            "Z",
            5,
            metadata(vec![property("business", MetadataValue::Bool(true), 6)], 5),
        ));
        assert_eq!(error_of(builder).offset, 20);
    }

    #[test]
    fn note_check_precedes_the_class_check() {
        let mut builder = UsecaseModelBuilder::new("usecase\n");
        builder.add_element(actor("A", 0));
        builder.add_note(NoteDraft {
            target: "Q".to_owned(),
            target_location: loc(10),
            label: label("note", 12),
            location: loc(9),
        });
        builder.add_class_assignment(ClassAssignment {
            targets: vec![("R".to_owned(), loc(30))],
            classes: vec!["c".to_owned()],
        });
        assert_eq!(error_of(builder).offset, 10);
    }

    #[test]
    fn edge_animation_marks_the_statement_and_sets_the_animation() {
        let mut builder = UsecaseModelBuilder::new("usecase\n");
        builder.add_element(actor("A", 0));
        builder.add_element(element("UC1", ElementKind::UseCase, 4));
        let mut edge = relation(
            endpoint("A", 0, true),
            endpoint("UC1", 4, true),
            8,
            RelationshipType::Association,
        );
        edge.explicit_id = Some("e1".to_owned());
        edge.explicit_id_location = Some(loc(20));
        builder.add_relationship(edge);
        builder.add_metadata_assignment(assignment(
            "e1",
            20,
            metadata(
                vec![property(
                    "animation",
                    MetadataValue::Text("slow".to_owned()),
                    25,
                )],
                24,
            ),
        ));

        let mut metadata_statement = statement(StatementKind::Metadata);
        metadata_statement.span = [18, 40];
        metadata_statement.nodes = Some(vec![NodeOccurrence {
            id: "e1".to_owned(),
            span: span_at(20),
            id_span: span_at(20),
            label_span: None,
            defines: None,
            stereotype_span: None,
            metadata: None,
            class_spans: None,
        }]);
        let diagram = builder
            .finalize(vec![metadata_statement], span_at(0))
            .expect("the diagram is valid");

        let relationship = &diagram.relationships[0];
        assert_eq!(relationship.animation, Some(Animation::Slow));
        assert!(relationship.animate);
        let marked = &diagram.ast.statements[0];
        assert_eq!(marked.kind, StatementKind::EdgeMetadata);
        assert!(marked.nodes.is_none());
        let edges = marked.edges.as_ref().expect("the edge occurrence is set");
        assert_eq!(edges[0].id, "e1");
        assert_eq!(edges[0].id_span, Some(span_at(20)));
    }
}
