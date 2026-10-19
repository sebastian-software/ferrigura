//! Port of the layout-data expectations of the use case diagram. The reference's `getData` is
//! `ferrigura_render::use_case::data::layout_data`, which builds the unified layout data from the parsed
//! model, and these tests read that data.
//!
//! Ported from `usecase-colorIndex.spec.ts` (the colour slots, six tests), from the getData tests of
//! `ferrigura-syntax/tests/use_case_spec.rs` and `use_case_db_spec.rs` that were ignored because the
//! render input was not ported. Where a field has another name than the reference's, a comment says so:
//! the reference's `classes` of an edge hold `edge-animation-*`, which the port's paint adds from
//! `Edge::animation`, and the diagram type is set by the renderer, not by `layout_data`.

use ferrigura_model::config::{MermaidConfig, defaults, typed};
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::use_case::Direction;
use ferrigura_render::context::Globals;
use ferrigura_render::unified::Section;
use ferrigura_render::unified::data::{Edge, LayoutData, Node};
use ferrigura_render::use_case::UseCaseSection;
use ferrigura_render::use_case::data::{Kind, NodeFields, layout_data};
use ferrigura_syntax::use_case::parse;

/// The layout data of a text, with the configuration that `configure` gives (the defaults otherwise).
fn data_with(text: &str, configure: impl FnOnce(&mut MermaidConfig)) -> LayoutData {
    let diagram = parse(text).expect("the text parses");
    let mut config = typed(&defaults()).expect("the defaults are typed");
    configure(&mut config);
    let globals = Globals::of(&config);
    let section = UseCaseSection {
        usecase: &config.usecase,
        unified: Section::of(&config),
    };
    layout_data(&diagram, &section, &globals, "diagram")
}

fn data(text: &str) -> LayoutData {
    data_with(text, |_| {})
}

fn node<'a>(data: &'a LayoutData, id: &str) -> &'a Node {
    data.nodes
        .iter()
        .find(|node| node.id == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

fn edge<'a>(data: &'a LayoutData, id: &str) -> &'a Edge {
    data.edges
        .iter()
        .find(|edge| edge.id == id)
        .unwrap_or_else(|| panic!("no edge {id}"))
}

/// The use case fields of a node, which only the use case nodes carry.
fn fields(node: &Node) -> &NodeFields {
    node.use_case
        .as_ref()
        .unwrap_or_else(|| panic!("{} has no use case fields", node.id))
}

mod colour_slots {
    //! `usecase-colorIndex.spec.ts`: the slots that `getData` assigns. Actors and use cases share one
    //! cycle, system boundaries have their own, and notes and JSON tables take none. The slot is
    //! assigned whatever the colour scheme, so these hold for both schemes.
    use super::*;

    fn slot(data: &LayoutData, id: &str) -> Option<u32> {
        node(data, id).color_index
    }

    #[test]
    fn gives_actors_and_use_cases_slots_from_one_shared_cycle() {
        // One counter across kinds: no two elements share a colour until the palette wraps.
        let data =
            data("usecase-beta\nactor User\nactor Admin\nLogin(\"Sign in\")\nLogout(\"Sign out\")");
        assert_eq!(slot(&data, "User"), Some(0));
        assert_eq!(slot(&data, "Admin"), Some(1));
        assert_eq!(slot(&data, "Login"), Some(2));
        assert_eq!(slot(&data, "Logout"), Some(3));
    }

    #[test]
    fn numbers_the_actors_before_the_use_cases_whatever_their_source_order() {
        // The two kinds are listed apart, so the cycle runs over the actors first (`getData` 340-384).
        let data =
            data("usecase-beta\nactor A\nMiddle(\"A use case between the two actors\")\nactor B");
        assert_eq!(slot(&data, "A"), Some(0));
        assert_eq!(slot(&data, "B"), Some(1));
        assert_eq!(slot(&data, "Middle"), Some(2));
    }

    #[test]
    fn numbers_system_boundaries_from_zero_on_their_own_counter() {
        let data = data(
            "usecase-beta\nsystemBoundary sb1[\"Payment service\"]\n  actor Clerk\n  Authorize(\"Authorize payment\")\nend\nsystemBoundary sb2[\"Shipping\"]\n  Dispatch(\"Dispatch order\")\nend\nReceipt(\"Create receipt\")",
        );
        // Boundaries run on their own counter, as flowchart subgraphs do (`getData` 430-435).
        assert_eq!(slot(&data, "sb1"), Some(0));
        assert_eq!(slot(&data, "sb2"), Some(1));
        // The actors and use cases keep their shared cycle, undisturbed by the boundaries.
        assert_eq!(slot(&data, "Clerk"), Some(0));
        assert_eq!(slot(&data, "Authorize"), Some(1));
        assert_eq!(slot(&data, "Dispatch"), Some(2));
        assert_eq!(slot(&data, "Receipt"), Some(3));
    }

    #[test]
    fn keeps_boundary_slots_stable_when_an_actor_is_inserted_before_them() {
        let data = data(
            "usecase-beta\nactor Extra\nsystemBoundary sb1[\"Payment service\"]\n  actor Clerk\nend\nsystemBoundary sb2[\"Shipping\"]\n  Dispatch(\"Dispatch order\")\nend",
        );
        assert_eq!(slot(&data, "sb1"), Some(0));
        assert_eq!(slot(&data, "sb2"), Some(1));
    }

    #[test]
    fn does_not_spend_a_slot_on_a_note() {
        // A note carries the theme's fixed note colour, so it stays outside the cycle.
        let data = data(
            "usecase-beta\nactor User\nnote for User \"Starts the workflow\"\nLogin(\"Sign in\")",
        );
        assert_eq!(slot(&data, "note-0"), None);
        assert_eq!(slot(&data, "User"), Some(0));
        assert_eq!(slot(&data, "Login"), Some(1));
    }

    #[test]
    fn does_not_spend_a_slot_on_a_json_table() {
        // A JSON table is reference data, not a participant.
        let data =
            data("usecase-beta\nactor User\njson Payload@{ \"active\": true }\nLogin(\"Sign in\")");
        assert_eq!(slot(&data, "Payload"), None);
        assert_eq!(slot(&data, "User"), Some(0));
        assert_eq!(slot(&data, "Login"), Some(1));
    }
}

#[test]
fn converts_diagram_data_to_layout_data() {
    // Two nodes (the actor and the use case) and one edge. The diagram type is set by the renderer,
    // so it is checked with the render, not here.
    let data = data("usecase-beta\nactor User\nUser --> Login");
    assert_eq!(data.nodes.len(), 2);
    assert_eq!(data.edges.len(), 1);
    assert_eq!(node(&data, "User").shape, "usecaseActor");
    assert_eq!(node(&data, "Login").shape, "usecaseEllipse");
}

#[test]
fn includes_direction_in_layout_data() {
    // `getData` writes the direction of the diagram, which is `LR` by default.
    assert_eq!(data("usecase-beta\nactor User").direction, "LR");
    assert_eq!(
        data("usecase-beta\ndirection TB\nactor User").direction,
        "TB"
    );
    let direction = parse("usecase-beta\ndirection RL\nactor User")
        .expect("the text parses")
        .direction;
    assert_eq!(direction, Direction::Rl);
}

#[test]
fn projects_actor_and_use_case_geometry_into_the_layout() {
    // The shapes `usecaseActor`, `usecaseActorHollow`, `usecaseBusiness`, `usecaseEllipse` and `rect`,
    // with the business flags they read (`usecaseDb.ts` 213-224, 346-420).
    let data = data(
        "usecase-beta\nactor Normal\nactor Hollow@{ type: hollow, business: true }\nBusiness(Business)@{ business: true }\nEllipse\nRectCase[Rectangle]",
    );
    let normal = node(&data, "Normal");
    assert_eq!(normal.shape, "usecaseActor");
    assert!(!fields(normal).business);

    let hollow = node(&data, "Hollow");
    assert_eq!(hollow.shape, "usecaseActorHollow");
    assert!(fields(hollow).business);

    let business = node(&data, "Business");
    assert_eq!(business.shape, "usecaseBusiness");
    assert!(fields(business).business);
    assert!(
        business.spread_ports,
        "an ellipse spreads its ports (usecaseDb.ts 519-521)"
    );

    let ellipse = node(&data, "Ellipse");
    assert_eq!(ellipse.shape, "usecaseEllipse");
    assert!(!fields(ellipse).business);
    assert!(ellipse.spread_ports);

    let rect = node(&data, "RectCase");
    assert_eq!(rect.shape, "rect");
    assert_eq!(rect.label, "Rectangle");
    assert!(!rect.spread_ports, "a rectangle does not spread its ports");
}

#[test]
fn projects_minimum_lengths_and_labels_into_the_layout() {
    // `minlen` is the number of arrow segments, and a label is the text between the dashes.
    let data = data(
        "usecase-beta\nA base@--> B\nB two@---> C\nC many@-----> D\nD labelled@-- \"long\" ----> E",
    );
    let summary: Vec<(&str, &str, Option<u32>)> = data
        .edges
        .iter()
        .map(|edge| (edge.id.as_str(), edge.label.as_str(), edge.minlen))
        .collect();
    assert_eq!(
        summary,
        [
            ("base", "", Some(1)),
            ("two", "", Some(2)),
            ("many", "", Some(4)),
            ("labelled", "long", Some(3)),
        ]
    );
}

#[test]
fn projects_animation_metadata_into_the_layout_classes() {
    // The reference puts `edge-animation-fast` (`trueEdge`, `fastEdge`) and `edge-animation-slow`
    // (`slowEdge`) in the edge's classes (`usecaseDb.ts` 286-289). The port keeps the animation in
    // `Edge::animation`, and the paint writes the class from it (the quirks test checks the class).
    let data = data(
        "usecase-beta\nA trueEdge@--> B\ntrueEdge@{ animate: true }\nA fastEdge@--> B\nfastEdge@{ animation: fast }\nA slowEdge@--> B\nslowEdge@{ animation: slow }\nA offEdge@--> B\noffEdge@{ animate: false }",
    );
    assert_eq!(edge(&data, "trueEdge").animation.as_deref(), Some("fast"));
    assert_eq!(edge(&data, "fastEdge").animation.as_deref(), Some("fast"));
    assert_eq!(edge(&data, "slowEdge").animation.as_deref(), Some("slow"));
    assert_eq!(edge(&data, "offEdge").animation, None);
    assert_eq!(
        edge(&data, "trueEdge").classes,
        "default relationship relationship-association"
    );
}

#[test]
fn projects_the_layout_of_json_classes_styles_and_edges() {
    // The table's classes and styles, and the edge's classes, styles, `minlen` and `isUserDefinedId`.
    let data = data(
        "usecase-beta\nclassDef data fill:#eef,stroke:#369\njson Payload@{\n  \"2\": \"second\",\n  \"1\": \"first\",\n  \"colors\": [\"Red\", \"Green\"],\n  \"items\": [{ \"name\": \"A\" }, {}],\n  \"empty\": {},\n  \"duplicate\": \"old\",\n  \"duplicate\": \"new\"\n}:::data\nstyle Payload stroke-width:4px\nInspect payloadEdge@--> Payload\nclass payloadEdge data\nstyle payloadEdge stroke:#f00",
    );
    let table = node(&data, "Payload");
    assert_eq!(table.shape, "usecaseJsonTable");
    assert_eq!(table.css_classes, "default usecase-json-table data");
    assert_eq!(table.css_styles, ["stroke-width:4px"]);
    assert_eq!(table.css_compiled_styles, ["fill:#eef", "stroke:#369"]);

    let payload = edge(&data, "payloadEdge");
    assert_eq!(payload.start, "Inspect");
    assert_eq!(payload.end, "Payload");
    assert_eq!(
        payload.classes,
        "default relationship relationship-association data"
    );
    assert_eq!(payload.style, ["stroke:#f00"]);
    assert_eq!(payload.minlen, Some(1));
    assert!(payload.is_user_defined_id);
}

#[test]
fn emits_every_node_and_boundary_variant_in_committed_model_order() {
    // The nodes come actors first, then use cases, notes, JSON tables and boundaries, each kind in the
    // order it was declared (`getData` 340-543). The variants of each kind have their own shape.
    let data = data(
        "usecase-beta\nclassDef data fill:#eef,stroke:#369\nactor Normal\nclass Normal data\nstyle Normal stroke:#f00\nactor Hollow@{ type: hollow }\nactor Awesome@{ type: awesome }\nactor Robot@{ icon: \"fa:robot\" }\nEllipse(\"Ellipse label\")\nRectCase[Rectangle]\nBiz(Biz)@{ business: true }\nnote for Normal \"note\"\njson Payload@{ \"a\": 1 }\nsystemBoundary Shop[\"Store\"]\nend",
    );
    let order: Vec<(&str, &str)> = data
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.shape.as_str()))
        .collect();
    assert_eq!(
        order,
        [
            ("Normal", "usecaseActor"),
            ("Hollow", "usecaseActorHollow"),
            ("Awesome", "usecaseActorAwesome"),
            ("Robot", "usecaseActorIcon"),
            ("Ellipse", "usecaseEllipse"),
            ("RectCase", "rect"),
            ("Biz", "usecaseBusiness"),
            ("note-0", "note"),
            ("Payload", "usecaseJsonTable"),
            ("Shop", "usecaseSystemBoundary"),
        ]
    );
    let normal = node(&data, "Normal");
    assert_eq!(normal.label, "Normal");
    assert_eq!(
        normal.css_classes,
        "default usecase-actor usecase-actor-normal data"
    );
    assert_eq!(normal.css_compiled_styles, ["fill:#eef", "stroke:#369"]);
    assert_eq!(normal.css_styles, ["stroke:#f00"]);
    assert_eq!(node(&data, "Ellipse").label, "Ellipse label");
    assert_eq!(node(&data, "RectCase").label, "Rectangle");
    assert_eq!(node(&data, "Shop").label, "Store");
}

#[test]
fn maps_relationship_semantics_markers_animation_styles_and_note_attachments_exactly() {
    // The seven arrow types of `A` to `B`, then the include, extend and generalization relationships,
    // and a note on `A` (`usecaseDb.ts` 233-284, 449-508). The arrow type numbers are the model's.
    let data = data(
        "usecase-beta\nactor A\nactor B\nLogin(Login)\nBrowse(Browse)\nA e0@--> B\nA e1@<-- B\nA e2@-- B\nA e3@--o B\nA e4@--x B\nA e5@o-- B\nA e6@x-- B\nLogin inc@..>:include Browse\nLogin ext@..>:extend Browse\nLogin gen@--|> Browse\nnote for A \"Remember\"",
    );
    // (start, end) markers of each arrow type, with the pattern of an association.
    let association = [
        ("none", "arrow_point"),
        ("arrow_point", "none"),
        ("none", "none"),
        ("none", "arrow_circle"),
        ("none", "arrow_cross"),
        ("arrow_circle", "none"),
        ("arrow_cross", "none"),
    ];
    for (index, (start, end)) in association.into_iter().enumerate() {
        let edge = edge(&data, &format!("e{index}"));
        assert_eq!(
            (edge.arrow_type_start.as_str(), edge.arrow_type_end.as_str()),
            (start, end),
            "the markers of arrow type {index}"
        );
        assert_eq!(edge.pattern, "solid");
        assert!(edge.label.is_empty());
    }

    let include = edge(&data, "inc");
    assert_eq!(include.pattern, "dotted");
    assert_eq!(
        (
            include.arrow_type_start.as_str(),
            include.arrow_type_end.as_str()
        ),
        ("none", "arrow_point")
    );
    assert_eq!(include.label, "include");
    assert_eq!(include.label_type, LabelType::Text);

    let extend = edge(&data, "ext");
    assert_eq!(extend.pattern, "dotted");
    assert_eq!(extend.label, "extend");

    let generalization = edge(&data, "gen");
    assert_eq!(generalization.pattern, "solid");
    assert_eq!(
        (
            generalization.arrow_type_start.as_str(),
            generalization.arrow_type_end.as_str()
        ),
        ("none", "extension")
    );
    assert!(generalization.label.is_empty());

    // The note's edge is dotted, without markers, and joins the note to its target.
    let note = edge(&data, "note-0-edge");
    assert_eq!(note.pattern, "dotted");
    assert_eq!(
        (note.arrow_type_start.as_str(), note.arrow_type_end.as_str()),
        ("none", "none")
    );
    assert_eq!((note.start.as_str(), note.end.as_str()), ("note-0", "A"));
    assert_eq!(note.minlen, Some(1));
}

#[test]
fn hands_schema_driven_layout_configuration_to_the_layout_data() {
    // The `usecase` section sets the wrapping width of the nodes, the minimum width of the use cases
    // (not of the actors, `usecaseDb.ts` 511-516) and the font size the actors' labels are drawn at.
    let data = data_with("usecase-beta\nactor User\nLogin(\"Sign in\")", |config| {
        config.usecase.wrapping_width = 77.0;
        config.usecase.min_node_width = 99.0;
        config.usecase.actor_font_size = 18.0;
    });
    let actor = node(&data, "User");
    assert_eq!(actor.wrapping_width, Some(77.0));
    assert_eq!(actor.min_width, None);
    assert_eq!(fields(actor).drawn_font_size, Some(18.0));

    let use_case = node(&data, "Login");
    assert_eq!(use_case.wrapping_width, Some(77.0));
    assert_eq!(use_case.min_width, Some(99.0));
}

#[test]
fn gives_oval_use_cases_extra_label_clearance_over_rectangular_use_cases() {
    // `usecaseDb.ts` 371: the padding is 20 for an ellipse and 10 for a rectangle.
    let data = data("usecase-beta\nBrowse(Browse catalog)\nManage[Manage catalog]");
    let browse = node(&data, "Browse");
    assert_eq!(browse.shape, "usecaseEllipse");
    assert_eq!(browse.padding, 20.0);
    let manage = node(&data, "Manage");
    assert_eq!(manage.shape, "rect");
    assert_eq!(manage.padding, 10.0);
}

#[test]
fn a_node_kind_has_its_own_use_case_fields() {
    // Only the use case nodes carry the fields that the paint reads: a note and a boundary keep their kind.
    let data = data(
        "usecase-beta\nactor User\nnote for User \"Remember\"\nsystemBoundary Shop[\"Store\"]\nend",
    );
    assert_eq!(fields(node(&data, "User")).kind, Kind::Actor);
    assert_eq!(fields(node(&data, "note-0")).kind, Kind::Note);
    assert_eq!(fields(node(&data, "Shop")).kind, Kind::Boundary);
}
