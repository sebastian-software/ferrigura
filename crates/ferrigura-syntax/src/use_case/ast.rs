//! The graph AST of the use case diagram: the port of `usecaseAst.ts` (`buildUsecaseGraphAST` and
//! the node, edge and group builders). The AST is built on every parse and is read by the specs and
//! the editors, not by the renderer.
//!
//! Every span is already in UTF-16 code units, so nothing is converted here. Two truthiness rules of
//! the reference are kept. A key that the reference adds only when a text is truthy (`label` of an
//! edge, `icon`, `stereotype`, `parentId`, `labelType`, `animation`, `accTitle`, `accDescr`) is left
//! out when the text is empty. The `label` of an actor, a use case or a boundary is left out when it
//! equals the id, whatever its length.
//!
//! The `symbols` of the input are not read: the reference's `buildUsecaseGraphAST` takes no symbols.

use std::collections::BTreeMap;

use ferrigura_model::use_case::{
    Actor, ActorType, Animation, AstClassDef, AstHeader, BoundaryType, GraphAst, GraphEdge,
    GraphGroup, GraphNode, GraphStatement, JsonNode, LabelType, Note, Relationship,
    RelationshipType, Span, SystemBoundary, UseCase, UseCaseDiagram, UseCaseShape,
};
use ferrigura_model::value::Value;

use super::draft::SymbolKind;

/// `ARROW_TYPE.LINE_SOLID` of `usecaseTypes.ts`: the arrow of a note's edge.
const LINE_SOLID_ARROW: f64 = 2.0;

/// The attributes of a node, an edge or a group, keyed as the reference keys them.
type Attrs = BTreeMap<String, Value>;

/// The input of [`build_graph_ast`]: the prepared text, the span of the header, the statements the
/// parser built, the model built so far and the builder's symbol map.
pub(crate) struct AstInput<'a> {
    /// The prepared text the AST is read from.
    pub(crate) source: &'a str,
    /// The span of the header keyword.
    pub(crate) header_span: Span,
    /// The statements the parser built, in order, with UTF-16 spans.
    pub(crate) statements: &'a [GraphStatement],
    /// The model built so far. Its `ast` field is not read.
    pub(crate) diagram: &'a UseCaseDiagram,
    /// The builder's symbol map. The reference's AST builder does not read it.
    #[allow(dead_code)]
    pub(crate) symbols: &'a BTreeMap<String, SymbolKind>,
}

/// Builds the graph AST of a use case diagram (`buildUsecaseGraphAST`, `usecaseAst.ts` 117-177).
///
/// The nodes are the actors, the use cases, the notes and the JSON tables, in that order; a later
/// id replaces an earlier one, as the reference's object assignment does. The edges are the
/// relationships in their order, then one dotted edge per note. The groups are the system
/// boundaries, and the class definitions are keyed by their id.
/// The key the reference's plain objects drop: assigning `obj["__proto__"]` sets the prototype, not
/// an own property, so the AST's `nodes`, `groups` and `classDefs` never hold it.
const PROTO_KEY: &str = "__proto__";

/// `obj[id] = value` on a plain object: every key but `__proto__`.
fn insert_plain<V>(map: &mut BTreeMap<String, V>, id: &str, value: V) {
    if id != PROTO_KEY {
        map.insert(id.to_owned(), value);
    }
}

#[must_use]
pub(crate) fn build_graph_ast(input: AstInput<'_>) -> GraphAst {
    let diagram = input.diagram;
    let mut nodes = BTreeMap::new();
    for actor in &diagram.actors {
        insert_plain(&mut nodes, &actor.id, actor_node(actor));
    }
    for use_case in &diagram.use_cases {
        insert_plain(&mut nodes, &use_case.id, use_case_node(use_case));
    }
    for note in &diagram.notes {
        insert_plain(&mut nodes, &note.id, note_node(note));
    }
    for json in &diagram.json_nodes {
        insert_plain(&mut nodes, &json.id, json_node(json));
    }

    let groups = diagram
        .system_boundaries
        .iter()
        .filter(|boundary| boundary.id != PROTO_KEY)
        .map(|boundary| (boundary.id.clone(), boundary_group(boundary)))
        .collect();
    let class_defs = diagram
        .class_defs
        .iter()
        .filter(|definition| definition.id != PROTO_KEY)
        .map(|definition| {
            (
                definition.id.clone(),
                AstClassDef {
                    styles: definition.styles.clone(),
                },
            )
        })
        .collect();
    let edges = diagram
        .relationships
        .iter()
        .map(relationship_edge)
        .chain(diagram.notes.iter().map(note_edge))
        .collect();

    GraphAst {
        version: 1,
        diagram_type: "usecase".to_owned(),
        source: input.source.to_owned(),
        header: AstHeader {
            keyword: "usecase".to_owned(),
            direction: diagram.direction,
            span: input.header_span,
        },
        acc_title: truthy(Some(diagram.acc_title.as_str())),
        acc_descr: truthy(Some(diagram.acc_description.as_str())),
        nodes,
        edges,
        groups,
        class_defs,
        statements: input.statements.to_vec(),
    }
}

/// The node of an actor (`actorNode`).
fn actor_node(actor: &Actor) -> GraphNode {
    GraphNode {
        label: unless_same(&actor.label, &actor.id),
        shape: Some(actor_shape(actor.actor_type).to_owned()),
        classes: list(&actor.classes),
        styles: list(&actor.styles),
        attrs: Some(attrs(vec![
            ("kind", Some(text("actor"))),
            ("actorType", Some(text(actor_type_name(actor.actor_type)))),
            ("business", Some(Value::Bool(actor.business))),
            ("labelType", Some(text(label_type_name(actor.label_type)))),
            ("icon", truthy_value(actor.icon.as_deref())),
            ("stereotype", truthy_value(actor.stereotype.as_deref())),
            ("parentId", truthy_value(actor.parent_id.as_deref())),
        ])),
    }
}

/// The node of a use case (`useCaseNode`).
fn use_case_node(use_case: &UseCase) -> GraphNode {
    let shape = use_case_shape_name(use_case.shape);
    GraphNode {
        label: unless_same(&use_case.label, &use_case.id),
        shape: Some(shape.to_owned()),
        classes: list(&use_case.classes),
        styles: list(&use_case.styles),
        attrs: Some(attrs(vec![
            ("kind", Some(text("usecase"))),
            ("useCaseShape", Some(text(shape))),
            ("business", Some(Value::Bool(use_case.business))),
            (
                "labelType",
                Some(text(label_type_name(use_case.label_type))),
            ),
            ("stereotype", truthy_value(use_case.stereotype.as_deref())),
            ("parentId", truthy_value(use_case.parent_id.as_deref())),
        ])),
    }
}

/// The node of a note (`noteNode`). A note has no classes and no styles.
fn note_node(note: &Note) -> GraphNode {
    GraphNode {
        label: Some(note.label.clone()),
        shape: Some("note".to_owned()),
        classes: None,
        styles: None,
        attrs: Some(attrs(vec![
            ("kind", Some(text("note"))),
            ("target", Some(text(&note.target))),
            ("labelType", Some(text(label_type_name(note.label_type)))),
        ])),
    }
}

/// The node of a JSON table (`jsonNode`). Its label is its id, and its attributes carry the value
/// and the key order of its objects.
fn json_node(json: &JsonNode) -> GraphNode {
    GraphNode {
        label: Some(json.id.clone()),
        shape: Some("json-table".to_owned()),
        classes: list(&json.classes),
        styles: list(&json.styles),
        attrs: Some(attrs(vec![
            ("kind", Some(text("json"))),
            ("value", Some(json.value.clone())),
            (
                "propertyOrder",
                Some(property_order_value(&json.property_order)),
            ),
            ("labelType", Some(text("text"))),
        ])),
    }
}

/// The edge of a relationship (`relationshipEdge`).
fn relationship_edge(relationship: &Relationship) -> GraphEdge {
    GraphEdge {
        id: relationship.id.clone(),
        source: relationship.source.clone(),
        target: relationship.target.clone(),
        label: truthy(relationship.label.as_deref()),
        classes: list(&relationship.classes),
        styles: list(&relationship.styles),
        attrs: Some(attrs(vec![
            (
                "relationshipType",
                Some(text(relationship_type_name(relationship.relationship_type))),
            ),
            (
                "arrowType",
                Some(Value::Number(f64::from(relationship.arrow_type))),
            ),
            (
                "minlen",
                Some(Value::Number(f64::from(relationship.minlen))),
            ),
            ("explicitId", Some(Value::Bool(relationship.explicit_id))),
            ("animate", Some(Value::Bool(relationship.animate))),
            (
                "animation",
                relationship
                    .animation
                    .map(|kind| text(animation_name(kind))),
            ),
            (
                "labelType",
                relationship
                    .label_type
                    .map(|label_type| text(label_type_name(label_type))),
            ),
        ])),
    }
}

/// The dotted edge of a note (`noteEdge`), from the note to its target. It has no label.
fn note_edge(note: &Note) -> GraphEdge {
    GraphEdge {
        id: format!("{}-edge", note.id),
        source: note.id.clone(),
        target: note.target.clone(),
        label: None,
        classes: None,
        styles: None,
        attrs: Some(attrs(vec![
            ("relationshipType", Some(text("note"))),
            ("arrowType", Some(Value::Number(LINE_SOLID_ARROW))),
            ("pattern", Some(text("dotted"))),
            ("minlen", Some(Value::Number(1.0))),
            ("explicitId", Some(Value::Bool(false))),
            ("animate", Some(Value::Bool(false))),
            ("internal", Some(Value::Bool(true))),
        ])),
    }
}

/// The group of a system boundary, with its title only when the label differs from the id.
fn boundary_group(boundary: &SystemBoundary) -> GraphGroup {
    GraphGroup {
        title: unless_same(&boundary.label, &boundary.id),
        parent: None,
        nodes: boundary.members.clone(),
        direction: None,
        classes: list(&boundary.classes),
        styles: list(&boundary.styles),
        attrs: Some(attrs(vec![
            ("kind", Some(text("systemBoundary"))),
            (
                "boundaryType",
                Some(text(boundary_type_name(boundary.boundary_type))),
            ),
            (
                "labelType",
                Some(text(label_type_name(boundary.label_type))),
            ),
        ])),
    }
}

/// The shape of an actor, by its glyph.
fn actor_shape(actor_type: ActorType) -> &'static str {
    match actor_type {
        ActorType::Normal => "actor",
        ActorType::Hollow => "actor-hollow",
        ActorType::Awesome => "actor-awesome",
        ActorType::Icon => "actor-icon",
    }
}

/// The `actorType` attribute of an actor.
fn actor_type_name(actor_type: ActorType) -> &'static str {
    match actor_type {
        ActorType::Normal => "normal",
        ActorType::Hollow => "hollow",
        ActorType::Awesome => "awesome",
        ActorType::Icon => "icon",
    }
}

/// The `shape` and the `useCaseShape` attribute of a use case.
fn use_case_shape_name(shape: UseCaseShape) -> &'static str {
    match shape {
        UseCaseShape::Ellipse => "ellipse",
        UseCaseShape::Rect => "rect",
    }
}

/// The `labelType` attribute.
fn label_type_name(label_type: LabelType) -> &'static str {
    match label_type {
        LabelType::Text => "text",
        LabelType::Markdown => "markdown",
    }
}

/// The `boundaryType` attribute of a system boundary.
fn boundary_type_name(boundary_type: BoundaryType) -> &'static str {
    match boundary_type {
        BoundaryType::Rect => "rect",
        BoundaryType::Package => "package",
    }
}

/// The `relationshipType` attribute of a relationship.
fn relationship_type_name(relationship_type: RelationshipType) -> &'static str {
    match relationship_type {
        RelationshipType::Association => "association",
        RelationshipType::Include => "include",
        RelationshipType::Extend => "extend",
        RelationshipType::Generalization => "generalization",
    }
}

/// The `animation` attribute of a relationship.
fn animation_name(animation: Animation) -> &'static str {
    match animation {
        Animation::Fast => "fast",
        Animation::Slow => "slow",
    }
}

/// A string value.
fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

/// The value of a text that the reference writes only when it is truthy (non-empty).
fn truthy_value(value: Option<&str>) -> Option<Value> {
    truthy(value).map(Value::String)
}

/// The text when it is non-empty, which is JavaScript's truthiness for a string.
fn truthy(value: Option<&str>) -> Option<String> {
    value.filter(|text| !text.is_empty()).map(str::to_owned)
}

/// The label when it differs from the id: the reference writes `label` only then.
fn unless_same(label: &str, id: &str) -> Option<String> {
    (label != id).then(|| label.to_owned())
}

/// The list when it has entries: the reference writes `classes` and `styles` only then.
fn list(values: &[String]) -> Option<Vec<String>> {
    (!values.is_empty()).then(|| values.to_vec())
}

/// A list of strings as a JSON array value.
fn string_list(values: &[String]) -> Value {
    Value::Array(values.iter().map(|value| text(value.as_str())).collect())
}

/// The `propertyOrder` attribute: each object's key list by its JSON pointer.
fn property_order_value(order: &BTreeMap<String, Vec<String>>) -> Value {
    Value::Object(
        order
            .iter()
            .map(|(pointer, keys)| (pointer.clone(), string_list(keys)))
            .collect(),
    )
}

/// The attributes built from the entries that have a value. An entry without one is left out.
fn attrs(entries: Vec<(&str, Option<Value>)>) -> Attrs {
    entries
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key.to_owned(), value)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrigura_model::use_case::Direction;

    fn actor(id: &str, label: &str) -> Actor {
        Actor {
            id: id.to_owned(),
            label: label.to_owned(),
            label_type: LabelType::Text,
            actor_type: ActorType::Normal,
            icon: None,
            business: false,
            stereotype: None,
            parent_id: None,
            classes: Vec::new(),
            styles: Vec::new(),
        }
    }

    fn relationship(id: &str, source: &str, target: &str, label: Option<&str>) -> Relationship {
        Relationship {
            id: id.to_owned(),
            explicit_id: false,
            source: source.to_owned(),
            target: target.to_owned(),
            relationship_type: RelationshipType::Association,
            arrow_type: 2,
            label: label.map(str::to_owned),
            label_type: label.map(|_| LabelType::Text),
            minlen: 1,
            classes: Vec::new(),
            styles: Vec::new(),
            animate: false,
            animation: None,
        }
    }

    fn note(id: &str, target: &str) -> Note {
        Note {
            id: id.to_owned(),
            target: target.to_owned(),
            label: "memo".to_owned(),
            label_type: LabelType::Text,
        }
    }

    fn boundary(id: &str, label: &str) -> SystemBoundary {
        SystemBoundary {
            id: id.to_owned(),
            label: label.to_owned(),
            label_type: LabelType::Text,
            boundary_type: BoundaryType::Rect,
            members: vec!["A".to_owned()],
            classes: Vec::new(),
            styles: Vec::new(),
        }
    }

    /// The AST of a diagram, with no statements.
    fn ast_of(diagram: &UseCaseDiagram) -> GraphAst {
        let symbols = BTreeMap::new();
        build_graph_ast(AstInput {
            source: "usecase",
            header_span: [0, 7],
            statements: &[],
            diagram,
            symbols: &symbols,
        })
    }

    #[test]
    fn label_is_left_out_only_when_it_equals_the_id() {
        let mut diagram = UseCaseDiagram::new();
        diagram.actors = vec![actor("A", "A"), actor("B", ""), actor("C", "Carol")];
        let ast = ast_of(&diagram);
        assert_eq!(ast.nodes["A"].label, None);
        assert_eq!(ast.nodes["B"].label, Some(String::new()));
        assert_eq!(ast.nodes["C"].label.as_deref(), Some("Carol"));
    }

    #[test]
    fn empty_texts_are_left_out_of_the_truthy_attributes() {
        let mut diagram = UseCaseDiagram::new();
        let mut a = actor("A", "A");
        a.icon = Some(String::new());
        a.stereotype = Some("s".to_owned());
        diagram.actors = vec![a];
        diagram.relationships = vec![relationship("edge-1", "A", "B", Some(""))];
        let ast = ast_of(&diagram);
        let node_attrs = ast.nodes["A"].attrs.clone().unwrap_or_default();
        assert!(!node_attrs.contains_key("icon"));
        assert_eq!(node_attrs.get("stereotype"), Some(&text("s")));
        let edge = &ast.edges[0];
        assert_eq!(edge.label, None);
        let edge_attrs = edge.attrs.clone().unwrap_or_default();
        assert_eq!(edge_attrs.get("labelType"), Some(&text("text")));
    }

    #[test]
    fn edges_are_the_relationships_then_one_dotted_edge_per_note() {
        let mut diagram = UseCaseDiagram::new();
        diagram.relationships = vec![
            relationship("edge-1", "A", "B", Some("uses")),
            relationship("x", "B", "A", None),
        ];
        diagram.notes = vec![note("note-1", "A")];
        let ast = ast_of(&diagram);
        let ids: Vec<&str> = ast.edges.iter().map(|edge| edge.id.as_str()).collect();
        assert_eq!(ids, ["edge-1", "x", "note-1-edge"]);
        let note_edge = &ast.edges[2];
        assert_eq!(note_edge.source, "note-1");
        assert_eq!(note_edge.target, "A");
        let attrs = note_edge.attrs.clone().unwrap_or_default();
        assert_eq!(attrs.get("pattern"), Some(&text("dotted")));
        assert_eq!(attrs.get("arrowType"), Some(&Value::Number(2.0)));
        assert_eq!(attrs.get("internal"), Some(&Value::Bool(true)));
    }

    #[test]
    fn group_title_follows_the_id_rule_and_the_header_takes_the_direction() {
        let mut diagram = UseCaseDiagram::new();
        diagram.direction = Direction::Bt;
        diagram.acc_description = "d".to_owned();
        diagram.system_boundaries = vec![boundary("S", "S"), boundary("T", "")];
        let ast = ast_of(&diagram);
        assert_eq!(ast.groups["S"].title, None);
        assert_eq!(ast.groups["T"].title, Some(String::new()));
        assert_eq!(ast.header.direction, Direction::Bt);
        assert_eq!(ast.header.span, [0, 7]);
        assert_eq!(ast.acc_title, None);
        assert_eq!(ast.acc_descr.as_deref(), Some("d"));
    }

    #[test]
    fn shapes_use_the_names_of_the_reference() {
        assert_eq!(actor_shape(ActorType::Normal), "actor");
        assert_eq!(actor_shape(ActorType::Hollow), "actor-hollow");
        assert_eq!(actor_shape(ActorType::Awesome), "actor-awesome");
        assert_eq!(actor_shape(ActorType::Icon), "actor-icon");
        assert_eq!(use_case_shape_name(UseCaseShape::Ellipse), "ellipse");
        assert_eq!(use_case_shape_name(UseCaseShape::Rect), "rect");
    }
}
