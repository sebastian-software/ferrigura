//! Port of `packages/mermaid/src/diagrams/usecase/usecase.spec.ts` to the public API of the parser,
//! `ferrigura_syntax::use_case::parse`, whose model is `ferrigura_model::use_case::UseCaseDiagram`.
//!
//! The reference's DB getters are the model's arrays and fields: `getActors()`, `getUseCases()`,
//! `getSystemBoundaries()`, `getRelationships()` and `getClassDefs()` are the arrays `actors`,
//! `use_cases`, `system_boundaries`, `relationships` and `class_defs`; `getAST()` is `ast`;
//! `getDirection()` is `direction`; the spec's `diagram.type` is the AST's `diagram_type`. A lookup
//! by id is a search of the array. `db.clear()` has no port of its own: each parse returns a new
//! model, so a parse of the bare header starts from nothing.
//!
//! The expectations about `getData` (the render input) are not in this port. A test that asserts
//! only them is `#[ignore]`d, and its body checks what the model holds, or is empty with a comment
//! naming the expectations. A test that asserts both is split: the part about what the parse
//! publishes (the AST, the model's fields and the JSON rows) is active, and the render part is
//! ignored. The spec commits host-built models (`createModel`, `commit`), which the public API does
//! not offer, so those tests take the same JSON as text.

use std::collections::BTreeMap;

use ferrigura_model::use_case::{
    Actor, ActorType, Animation, BoundaryType, Direction, GraphNode, JsonNode, RelationshipType,
    SystemBoundary, UseCase, UseCaseDiagram, UseCaseShape,
};
use ferrigura_model::value::Value;
use ferrigura_syntax::use_case::parse;
use serde_json::json;

/// `ARROW_TYPE.SOLID_ARROW` of `usecaseTypes.ts`. The AST keeps an arrow type as a number.
const SOLID_ARROW: f64 = 0.0;

fn parsed(text: &str) -> UseCaseDiagram {
    parse(text).unwrap_or_else(|error| panic!("the diagram does not parse: {error}"))
}

fn actor<'a>(diagram: &'a UseCaseDiagram, id: &str) -> Option<&'a Actor> {
    diagram.actors.iter().find(|candidate| candidate.id == id)
}

fn use_case<'a>(diagram: &'a UseCaseDiagram, id: &str) -> Option<&'a UseCase> {
    diagram
        .use_cases
        .iter()
        .find(|candidate| candidate.id == id)
}

fn boundary<'a>(diagram: &'a UseCaseDiagram, id: &str) -> Option<&'a SystemBoundary> {
    diagram
        .system_boundaries
        .iter()
        .find(|candidate| candidate.id == id)
}

fn json_node<'a>(diagram: &'a UseCaseDiagram, id: &str) -> &'a JsonNode {
    diagram
        .json_nodes
        .iter()
        .find(|candidate| candidate.id == id)
        .unwrap_or_else(|| panic!("no JSON table {id}"))
}

fn node<'a>(diagram: &'a UseCaseDiagram, id: &str) -> &'a GraphNode {
    diagram
        .ast
        .nodes
        .get(id)
        .unwrap_or_else(|| panic!("no node {id} in the AST"))
}

/// The attribute `key` of an AST element, when it has it.
fn attr<'a>(attrs: Option<&'a BTreeMap<String, Value>>, key: &str) -> Option<&'a Value> {
    attrs?.get(key)
}

/// The AST node `id` has the shape given, and each attribute given with its value.
fn assert_node(diagram: &UseCaseDiagram, id: &str, shape: &str, attrs: &[(&str, Value)]) {
    let found = node(diagram, id);
    assert_eq!(found.shape.as_deref(), Some(shape), "the shape of {id}");
    for (key, expected) in attrs {
        assert_eq!(
            attr(found.attrs.as_ref(), key),
            Some(expected),
            "{id}: attribute {key}"
        );
    }
}

/// The rows of a JSON table as `(key, accessibleKey, value)`.
fn rows_of(table: &JsonNode) -> Vec<(&str, &str, &str)> {
    table
        .rows
        .iter()
        .map(|row| {
            (
                row.key.as_str(),
                row.accessible_key.as_str(),
                row.value.as_str(),
            )
        })
        .collect()
}

mod basic_actors {
    use super::*;

    #[test]
    fn parses_a_single_actor() {
        let diagram = parsed(
            r#"usecase-beta
        actor User"#,
        );

        assert_eq!(diagram.ast.diagram_type, "usecase");
        assert_eq!(diagram.actors.len(), 1);
        let user = actor(&diagram, "User").expect("the actor User");
        assert_eq!(user.label, "User");
    }

    #[test]
    fn parses_multiple_actors() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        actor Admin
        actor Guest"#,
        );

        assert_eq!(diagram.actors.len(), 3);
        assert!(actor(&diagram, "User").is_some());
        assert!(actor(&diagram, "Admin").is_some());
        assert!(actor(&diagram, "Guest").is_some());
    }

    #[test]
    fn parses_actor_with_simple_name() {
        let diagram = parsed(
            r#"usecase-beta
        actor SystemUser"#,
        );

        assert_eq!(diagram.actors.len(), 1);
        assert!(actor(&diagram, "SystemUser").is_some());
    }
}

mod use_cases {
    use super::*;

    #[test]
    fn parses_use_cases_from_relationships() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        User --> Login"#,
        );

        assert_eq!(diagram.use_cases.len(), 1);
        assert!(use_case(&diagram, "Login").is_some());
    }

    #[test]
    fn parses_multiple_use_cases_from_relationships() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        User --> Login
        User --> Logout
        User --> Register"#,
        );

        assert_eq!(diagram.use_cases.len(), 3);
        assert!(use_case(&diagram, "Login").is_some());
        assert!(use_case(&diagram, "Logout").is_some());
        assert!(use_case(&diagram, "Register").is_some());
    }

    #[test]
    fn parses_use_case_from_relationship() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        User --> UserLoginProcess"#,
        );

        assert_eq!(diagram.use_cases.len(), 1);
        assert!(use_case(&diagram, "UserLoginProcess").is_some());
    }

    #[test]
    fn parses_use_cases_with_quoted_names() {
        let diagram = parsed(
            r#"usecase-beta
        actor "Customer Service"
        actor "System Administrator"
        "Customer Service" --> "Handle Tickets"
        "System Administrator" --> "Manage System""#,
        );

        // The ids replace the spaces with underscores; the labels keep the original text.
        assert_eq!(diagram.actors.len(), 2);
        assert_eq!(
            actor(&diagram, "Customer_Service").map(|found| found.label.as_str()),
            Some("Customer Service")
        );
        assert_eq!(
            actor(&diagram, "System_Administrator").map(|found| found.label.as_str()),
            Some("System Administrator")
        );

        assert_eq!(diagram.use_cases.len(), 2);
        assert_eq!(
            use_case(&diagram, "Handle_Tickets").map(|found| found.label.as_str()),
            Some("Handle Tickets")
        );
        assert_eq!(
            use_case(&diagram, "Manage_System").map(|found| found.label.as_str()),
            Some("Manage System")
        );
    }
}

mod relationships {
    use super::*;

    #[test]
    fn parses_actor_to_use_case_relationship() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        User --> Login"#,
        );

        assert_eq!(diagram.relationships.len(), 1);
        let relationship = &diagram.relationships[0];
        assert_eq!(relationship.source, "User");
        assert_eq!(relationship.target, "Login");
        assert_eq!(
            relationship.relationship_type,
            RelationshipType::Association
        );
    }

    #[test]
    fn parses_multiple_relationships() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        User --> Login
        User --> Logout"#,
        );

        assert_eq!(diagram.relationships.len(), 2);
        assert_eq!(diagram.relationships[0].source, "User");
        assert_eq!(diagram.relationships[0].target, "Login");
        assert_eq!(diagram.relationships[1].source, "User");
        assert_eq!(diagram.relationships[1].target, "Logout");
    }

    #[test]
    fn parses_relationship_with_label() {
        let diagram = parsed(
            r#"usecase-beta
        actor Developer
        Developer --important--> WriteCode"#,
        );

        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.relationships[0].label.as_deref(), Some("important"));
    }

    #[test]
    fn parses_different_arrow_types() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        actor Admin
        User --> Login
        Admin <-- Logout
        User -- ViewData"#,
        );

        assert_eq!(diagram.relationships.len(), 3);
    }
}

mod system_boundaries {
    use super::*;

    #[test]
    fn parses_a_system_boundary() {
        let diagram = parsed(
            r#"usecase-beta
        actor Admin, User
        systemBoundary "Authentication"
          Login
          Logout
        end
        Admin --> Login
        User --> Login"#,
        );

        assert!(!diagram.system_boundaries.is_empty());
    }

    #[test]
    fn parses_use_cases_within_system_boundary() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        systemBoundary "Authentication System"
          Login
          Logout
        end
        User --> Login"#,
        );

        assert_eq!(diagram.use_cases.len(), 2);
        assert!(use_case(&diagram, "Login").is_some());
        assert!(use_case(&diagram, "Logout").is_some());
    }

    #[test]
    fn parses_a_boundary_with_an_explicit_id_a_label_and_inline_metadata() {
        let diagram = parsed(
            r#"usecase-beta
        systemBoundary sb1["Payment service"]@{ type: package }
          actor Clerk("Payment clerk")
          Authorize("Authorize payment")
        end
        Clerk --> Authorize"#,
        );

        let payment = boundary(&diagram, "sb1").expect("the boundary sb1");
        assert_eq!(payment.label, "Payment service");
        assert_eq!(payment.boundary_type, BoundaryType::Package);
        assert_eq!(payment.members, ["Clerk", "Authorize"]);
    }
}

mod direction {
    use super::*;

    #[test]
    fn handles_tb_direction() {
        let diagram = parsed(
            r#"usecase-beta
        direction TB
        actor User"#,
        );

        assert_eq!(diagram.direction, Direction::Tb);
    }

    #[test]
    fn handles_lr_direction() {
        let diagram = parsed(
            r#"usecase-beta
        direction LR
        actor User"#,
        );

        assert_eq!(diagram.direction, Direction::Lr);
    }

    #[test]
    fn normalizes_td_to_tb() {
        let diagram = parsed(
            r#"usecase-beta
        direction TD
        actor User"#,
        );

        assert_eq!(diagram.direction, Direction::Tb);
    }
}

mod actor_metadata {
    use super::*;

    #[test]
    fn parses_actor_with_metadata() {
        let diagram = parsed(
            r#"usecase-beta
        actor User@{ "type" : "normal", "icon" : "user" }
        User --> Login"#,
        );

        assert_eq!(diagram.actors.len(), 1);
        let user = actor(&diagram, "User").expect("the actor User");
        // An actor with an icon is drawn with the icon, whatever its `type` says.
        assert_eq!(user.actor_type, ActorType::Icon);
        assert_eq!(user.icon.as_deref(), Some("user"));
        assert!(!user.business);
    }

    #[test]
    fn parses_multiple_actors_with_different_metadata() {
        let diagram = parsed(
            r#"usecase-beta
        actor User@{ "icon" : "user" }
        actor Admin@{ "type" : "hollow" }
        User --> Login
        Admin --> ManageUsers"#,
        );

        assert_eq!(diagram.actors.len(), 2);
        let user = actor(&diagram, "User").expect("the actor User");
        assert_eq!(user.actor_type, ActorType::Icon);
        assert_eq!(user.icon.as_deref(), Some("user"));
        let admin = actor(&diagram, "Admin").expect("the actor Admin");
        assert_eq!(admin.actor_type, ActorType::Hollow);
        assert!(!admin.business);
    }
}

mod complex_diagrams {
    use super::*;

    #[test]
    fn parses_a_complete_authentication_system() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        actor Admin

        systemBoundary "Authentication System"
          Login
          Logout
          Register
          ResetPassword
        end

        User --> Login
        User --> Register
        User --> Logout
        Admin --> Login"#,
        );

        assert_eq!(diagram.actors.len(), 2);
        assert_eq!(diagram.use_cases.len(), 4);
        assert_eq!(diagram.relationships.len(), 4);
    }

    #[test]
    fn parses_diagram_with_multiple_arrow_types() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        actor Admin
        User --> Login
        Admin <-- Logout
        User -- ViewData"#,
        );

        assert_eq!(diagram.relationships.len(), 3);
    }

    #[test]
    fn handles_use_case_creation_from_relationships() {
        let diagram = parsed(
            r#"usecase-beta
        actor Developer
        Developer --> LoginSystem
        Developer --> Authentication"#,
        );

        assert_eq!(diagram.use_cases.len(), 2);
        assert!(use_case(&diagram, "LoginSystem").is_some());
        assert!(use_case(&diagram, "Authentication").is_some());
    }
}

mod class_definitions {
    use super::*;

    #[test]
    fn handles_class_def() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        User --> Login
        classDef important fill:#f96,stroke:#333,stroke-width:4px
        class Login important"#,
        );

        assert!(!diagram.class_defs.is_empty());
    }
}

mod database_methods {
    use super::*;

    #[test]
    fn clears_all_data_before_the_next_parse() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        User --> Login"#,
        );

        assert_eq!(diagram.actors.len(), 1);
        assert_eq!(diagram.use_cases.len(), 1);
        assert_eq!(diagram.relationships.len(), 1);

        // The spec clears the DB. A parse of the bare header starts from an empty model.
        let cleared = parsed("usecase-beta");

        assert!(cleared.actors.is_empty());
        assert!(cleared.use_cases.is_empty());
        assert!(cleared.relationships.is_empty());
    }

    #[test]
    fn gets_a_specific_actor_by_id() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        actor Admin"#,
        );

        let user = actor(&diagram, "User").expect("the actor User");
        assert_eq!(user.id, "User");
        assert_eq!(user.label, "User");
    }

    #[test]
    fn gets_a_specific_use_case_by_id() {
        let diagram = parsed(
            r#"usecase-beta
        actor User
        User --> Login
        User --> Logout"#,
        );

        let login = use_case(&diagram, "Login").expect("the use case Login");
        assert_eq!(login.id, "Login");
        assert_eq!(login.label, "Login");
    }

    #[test]
    fn returns_no_actor_for_an_unknown_id() {
        // The spec reads an empty DB (cleared before each test).
        let empty = parsed("usecase-beta");

        assert!(actor(&empty, "NonExistent").is_none());
    }

    #[test]
    fn returns_no_use_case_for_an_unknown_id() {
        let empty = parsed("usecase-beta");

        assert!(use_case(&empty, "NonExistent").is_none());
    }
}

mod get_data {
    use super::*;

    #[test]
    #[ignore = "the render input (getData) is not ported; it belongs to the renderer stage"]
    fn converts_diagram_data_to_layout_data() {
        // getData: two nodes (the actor and the use case), one edge, the type `usecase`.
        // The model: one actor and one use case, one relationship, and the AST's diagram type.
        let diagram = parsed(
            r#"usecase-beta
        actor User
        User --> Login"#,
        );

        assert_eq!(diagram.actors.len() + diagram.use_cases.len(), 2);
        assert_eq!(diagram.relationships.len(), 1);
        assert_eq!(diagram.ast.diagram_type, "usecase");
    }

    #[test]
    #[ignore = "the render input (getData) is not ported; it belongs to the renderer stage"]
    fn includes_direction_in_layout_data() {
        let diagram = parsed(
            r#"usecase-beta
        direction LR
        actor User"#,
        );

        assert_eq!(diagram.direction, Direction::Lr);
    }
}

#[test]
fn projects_parser_produced_actor_and_use_case_geometry_into_the_ast() {
    let diagram = parsed(
        r#"usecase-beta
actor Normal
actor Hollow@{ type: hollow, business: true }
Business(Business)@{ business: true }
Ellipse
RectCase[Rectangle]"#,
    );

    assert_node(
        &diagram,
        "Normal",
        "actor",
        &[
            ("kind", Value::from("actor")),
            ("actorType", Value::from("normal")),
            ("business", Value::Bool(false)),
        ],
    );
    assert_node(
        &diagram,
        "Hollow",
        "actor-hollow",
        &[
            ("kind", Value::from("actor")),
            ("actorType", Value::from("hollow")),
            ("business", Value::Bool(true)),
        ],
    );
    assert_node(
        &diagram,
        "Business",
        "ellipse",
        &[
            ("kind", Value::from("usecase")),
            ("useCaseShape", Value::from("ellipse")),
            ("business", Value::Bool(true)),
        ],
    );
    assert_node(
        &diagram,
        "Ellipse",
        "ellipse",
        &[
            ("kind", Value::from("usecase")),
            ("useCaseShape", Value::from("ellipse")),
            ("business", Value::Bool(false)),
        ],
    );
    assert_node(
        &diagram,
        "RectCase",
        "rect",
        &[
            ("kind", Value::from("usecase")),
            ("useCaseShape", Value::from("rect")),
            ("business", Value::Bool(false)),
        ],
    );
}

#[test]
#[ignore = "the render input (getData) is not ported; it belongs to the renderer stage"]
fn projects_actor_and_use_case_geometry_into_the_layout() {
    // The render input: the shapes `usecaseActor`, `usecaseActorHollow`, `usecaseBusiness`,
    // `usecaseEllipse` and `rect`. The model holds the flags they read.
    let diagram = parsed(
        r#"usecase-beta
actor Normal
actor Hollow@{ type: hollow, business: true }
Business(Business)@{ business: true }
Ellipse
RectCase[Rectangle]"#,
    );

    let normal = actor(&diagram, "Normal").expect("the actor Normal");
    assert_eq!(
        (normal.actor_type, normal.business),
        (ActorType::Normal, false)
    );
    let hollow = actor(&diagram, "Hollow").expect("the actor Hollow");
    assert_eq!(
        (hollow.actor_type, hollow.business),
        (ActorType::Hollow, true)
    );
    let business = use_case(&diagram, "Business").expect("the use case Business");
    assert_eq!(
        (business.shape, business.business),
        (UseCaseShape::Ellipse, true)
    );
    let ellipse = use_case(&diagram, "Ellipse").expect("the use case Ellipse");
    assert_eq!(
        (ellipse.shape, ellipse.business),
        (UseCaseShape::Ellipse, false)
    );
    let rect = use_case(&diagram, "RectCase").expect("the use case RectCase");
    assert_eq!((rect.shape, rect.business), (UseCaseShape::Rect, false));
}

#[test]
fn projects_base_one_extra_n_extra_and_labelled_minimum_lengths_into_the_ast() {
    let diagram = parsed(
        r#"usecase-beta
A base@--> B
B two@---> C
C many@-----> D
D labelled@-- "long" ----> E"#,
    );

    let edges: Vec<(&str, Option<&str>, Option<f64>)> = diagram
        .ast
        .edges
        .iter()
        .map(|edge| {
            (
                edge.id.as_str(),
                edge.label.as_deref(),
                attr(edge.attrs.as_ref(), "minlen").and_then(Value::as_f64),
            )
        })
        .collect();
    assert_eq!(
        edges,
        [
            ("base", None, Some(1.0)),
            ("two", None, Some(2.0)),
            ("many", None, Some(4.0)),
            ("labelled", Some("long"), Some(3.0)),
        ]
    );
}

#[test]
#[ignore = "the render input (getData) is not ported; it belongs to the renderer stage"]
fn projects_minimum_lengths_and_labels_into_the_layout() {
    // The render input: the edges' `id`, `label` and `minlen`, which the model's relationships hold.
    let diagram = parsed(
        r#"usecase-beta
A base@--> B
B two@---> C
C many@-----> D
D labelled@-- "long" ----> E"#,
    );

    let edges: Vec<(&str, Option<&str>, u32)> = diagram
        .relationships
        .iter()
        .map(|relationship| {
            (
                relationship.id.as_str(),
                relationship.label.as_deref(),
                relationship.minlen,
            )
        })
        .collect();
    assert_eq!(
        edges,
        [
            ("base", None, 1),
            ("two", None, 2),
            ("many", None, 4),
            ("labelled", Some("long"), 3),
        ]
    );
}

#[test]
fn projects_parser_produced_animation_metadata_into_the_relationships() {
    let diagram = parsed(
        r#"usecase-beta
A trueEdge@--> B
trueEdge@{ animate: true }
A fastEdge@--> B
fastEdge@{ animation: fast }
A slowEdge@--> B
slowEdge@{ animation: slow }
A offEdge@--> B
offEdge@{ animate: false }"#,
    );

    // `animate: true` sets the flag and leaves `animation` unset, as the reference's DB does.
    let edges: Vec<(&str, bool, Option<Animation>)> = diagram
        .relationships
        .iter()
        .map(|relationship| {
            (
                relationship.id.as_str(),
                relationship.animate,
                relationship.animation,
            )
        })
        .collect();
    assert_eq!(
        edges,
        [
            ("trueEdge", true, None),
            ("fastEdge", true, Some(Animation::Fast)),
            ("slowEdge", true, Some(Animation::Slow)),
            ("offEdge", false, None),
        ]
    );
}

#[test]
#[ignore = "the render input (getData) is not ported; it belongs to the renderer stage"]
fn projects_animation_metadata_into_the_layout_classes() {
    // The render input: `classes` of the edges, `default relationship relationship-association`
    // and `edge-animation-fast` (`trueEdge` and `fastEdge`) or `edge-animation-slow` (`slowEdge`).
    // The model holds `animate` and `animation`, checked in the test above.
}

#[test]
fn projects_complex_source_ordered_json_a_forward_edge_classes_and_styles_end_to_end() {
    let diagram = parsed(
        r#"usecase-beta
classDef data fill:#eef,stroke:#369
json Payload@{
  "2": "second",
  "1": "first",
  "colors": ["Red", "Green"],
  "items": [{ "name": "A" }, {}],
  "empty": {},
  "duplicate": "old",
  "duplicate": "new"
}:::data
style Payload stroke-width:4px
Inspect payloadEdge@--> Payload
class payloadEdge data
style payloadEdge stroke:#f00"#,
    );

    let expected_payload = GraphNode {
        label: Some("Payload".to_owned()),
        shape: Some("json-table".to_owned()),
        classes: Some(vec!["data".to_owned()]),
        styles: Some(vec!["stroke-width:4px".to_owned()]),
        attrs: Some(BTreeMap::from([
            ("kind".to_owned(), Value::from("json")),
            (
                "value".to_owned(),
                Value::from(json!({
                    "1": "first",
                    "2": "second",
                    "colors": ["Red", "Green"],
                    "items": [{ "name": "A" }, {}],
                    "empty": {},
                    "duplicate": "new",
                })),
            ),
            (
                "propertyOrder".to_owned(),
                Value::from(json!({
                    "": ["2", "1", "colors", "items", "empty", "duplicate"],
                    "/items/0": ["name"],
                    "/items/1": [],
                    "/empty": [],
                })),
            ),
            ("labelType".to_owned(), Value::from("text")),
        ])),
    };
    assert_eq!(node(&diagram, "Payload"), &expected_payload);

    let edge = &diagram.ast.edges[0];
    assert_eq!(edge.id, "payloadEdge");
    assert_eq!(edge.source, "Inspect");
    assert_eq!(edge.target, "Payload");
    assert_eq!(edge.classes, Some(vec!["data".to_owned()]));
    assert_eq!(edge.styles, Some(vec!["stroke:#f00".to_owned()]));
    assert_eq!(
        attr(edge.attrs.as_ref(), "relationshipType"),
        Some(&Value::from("association"))
    );
    assert_eq!(
        attr(edge.attrs.as_ref(), "arrowType").and_then(Value::as_f64),
        Some(SOLID_ARROW)
    );
    assert_eq!(
        attr(edge.attrs.as_ref(), "minlen").and_then(Value::as_f64),
        Some(1.0)
    );
    assert_eq!(
        attr(edge.attrs.as_ref(), "explicitId"),
        Some(&Value::Bool(true))
    );

    // The display rows of the table (`flattenJsonRows`), which the model keeps as `rows`.
    assert_eq!(
        rows_of(json_node(&diagram, "Payload")),
        [
            ("2", "2", "second"),
            ("1", "1", "first"),
            ("colors", "colors", "Red"),
            ("", "colors", "Green"),
            ("items[0].name", "items[0].name", "A"),
            ("items[1]", "items[1]", "{}"),
            ("empty", "empty", "{}"),
            ("duplicate", "duplicate", "new"),
        ]
    );
}

#[test]
#[ignore = "the render input (getData) is not ported; it belongs to the renderer stage"]
fn projects_the_layout_of_json_classes_styles_and_edges() {
    // The render input: the shape `usecaseJsonTable`, the CSS classes `default usecase-json-table
    // data`, and the edge's classes, style, `minlen` and `isUserDefinedId`. The model holds the
    // table's classes and styles, and the edge's classes, styles, `minlen` and `explicit_id`.
    let diagram = parsed(
        r#"usecase-beta
classDef data fill:#eef,stroke:#369
json Payload@{
  "2": "second",
  "1": "first",
  "colors": ["Red", "Green"],
  "items": [{ "name": "A" }, {}],
  "empty": {},
  "duplicate": "old",
  "duplicate": "new"
}:::data
style Payload stroke-width:4px
Inspect payloadEdge@--> Payload
class payloadEdge data
style payloadEdge stroke:#f00"#,
    );

    let table = json_node(&diagram, "Payload");
    assert_eq!(table.classes, ["data"]);
    assert_eq!(table.styles, ["stroke-width:4px"]);

    let edge = &diagram.relationships[0];
    assert_eq!(edge.id, "payloadEdge");
    assert_eq!(edge.classes, ["data"]);
    assert_eq!(edge.styles, ["stroke:#f00"]);
    assert_eq!(edge.minlen, 1);
    assert!(edge.explicit_id);
}

mod layout_contract {
    use super::*;

    #[test]
    #[ignore = "the render input (getData) is not ported; it belongs to the renderer stage"]
    fn emits_every_node_and_boundary_variant_in_committed_model_order() {
        // The spec commits a host-built model with three classDefs, a normal actor with classes
        // and styles, hollow and awesome actors, an icon actor, use cases of both shapes, a note, a
        // JSON table and two boundaries, and reads `getData`. The node ids and shapes in order
        // (`usecaseActor`, `usecaseActorHollow`, `usecaseActorAwesome`, `usecaseActorIcon`,
        // `usecaseEllipse`, `rect`, `usecaseBusiness`, `note`, `usecaseJsonTable`,
        // `usecaseSystemBoundary`), the sanitised labels, the compiled styles and the CSS classes
        // belong to the render input. None of them is a parse result.
    }

    #[test]
    #[ignore = "the render input (getData) is not ported; it belongs to the renderer stage"]
    fn maps_relationship_semantics_markers_animation_styles_and_note_attachments_exactly() {
        // The spec commits a host-built model: seven arrow types of `A` to `B`, the `include`,
        // `extend` and `generalization` relationships, and a note on `A`, then reads the edges'
        // `arrowTypeStart` and `arrowTypeEnd`, the markers, the patterns and the label rules of
        // `getData`. The arrow type numbers are the model's, but the markers are the render input.
    }

    #[test]
    fn flattens_json_rows_depth_first_using_source_property_order() {
        // The spec commits a host-built JSON table and reads its `jsonRows` from `getData`. The
        // model keeps the rows as `rows`, so the same table is parsed from its JSON text.
        let diagram = parsed(
            r#"usecase-beta
json Payload@{
  "2": "second",
  "1": "first",
  "colors": ["Red", "Green"],
  "address": { "city": "Oslo" },
  "items": [{ "name": "A" }, {}],
  "emptyArray": [],
  "emptyObject": {},
  "enabled": true,
  "count": 2,
  "nothing": null
}"#,
        );

        assert_eq!(
            rows_of(json_node(&diagram, "Payload")),
            [
                ("2", "2", "second"),
                ("1", "1", "first"),
                ("colors", "colors", "Red"),
                ("", "colors", "Green"),
                ("address.city", "address.city", "Oslo"),
                ("items[0].name", "items[0].name", "A"),
                ("items[1]", "items[1]", "{}"),
                ("emptyArray", "emptyArray", "[]"),
                ("emptyObject", "emptyObject", "{}"),
                ("enabled", "enabled", "true"),
                ("count", "count", "2"),
                ("nothing", "nothing", "null"),
            ]
        );
    }

    #[test]
    #[ignore = "the render input (getData) is not ported; it belongs to the renderer stage"]
    fn hands_schema_driven_layout_viewport_and_font_configuration_to_the_renderer_contract() {
        // The spec sets the `usecase` configuration (font sizes and families, spacings, the
        // padding, `useMaxWidth`) and reads it back through `getData`. The configuration is not in
        // the model. The direction is the one part the model holds: it is `LR` by default.
        let diagram = parsed("usecase-beta");

        assert_eq!(diagram.direction, Direction::Lr);
    }

    #[test]
    #[ignore = "the stylesheet (styles.ts) is not ported; it belongs to the renderer stage"]
    fn consolidates_canonical_selectors_and_relies_on_shared_animation_keyframes() {
        // The spec checks the CSS of `getStyles`: the selectors of the hollow and awesome actors,
        // the business marker, the boundary and title rules (the fills, the strokes and the
        // title text), the note, the JSON table, the include relationship, the edge label and
        // label background rules, the markers, the font-size variables, and that there are no
        // keyframes. The stylesheet is the renderer's, not the parser's.
    }
}
