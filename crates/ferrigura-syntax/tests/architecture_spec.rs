//! Port of `packages/mermaid/src/diagrams/architecture/architecture.spec.ts`: the parser and DB
//! tests, through the public parse. Test names follow the reference's `it` titles in snake_case.
//! The reference's `describe` groups are comments.
//!
//! The reference's DB methods are not part of the port's API. A test that calls `addGroup`,
//! `addService`, `addJunction` or `addLayoutHint` directly is written as the statement that makes
//! the same call, which is the only way the DB is reached. The one check that no statement can
//! reach, an alignment with fewer than two members, is a unit test in `architecture/db.rs`.
//!
//! The configuration tests and the cytoscape and renderer tests stay ignored, with the reason.

use ferrigura_model::architecture::{AlignDirection, ArchitectureDiagram};
use ferrigura_syntax::architecture::parse;

fn parsed(text: &str) -> ArchitectureDiagram {
    parse(text).expect("the diagram parses")
}

fn rejected(text: &str) -> String {
    parse(text).expect_err("the diagram is rejected").message
}

fn ids_of_services(diagram: &ArchitectureDiagram) -> Vec<&str> {
    diagram.services.iter().map(|s| s.id.as_str()).collect()
}

mod architecture_diagram_definitions {
    use super::*;

    #[test]
    fn should_handle_the_architecture_keyword() {
        parsed("architecture-beta");
    }

    #[test]
    fn should_handle_a_simple_radar_definition() {
        parsed(
            "architecture-beta
            service db
            ",
        );
    }
}

mod should_handle_titleandaccessibilities {
    use super::*;

    #[test]
    fn should_handle_title_on_the_first_line() {
        let diagram = parsed("architecture-beta title Simple Architecture Diagram");
        assert_eq!(diagram.title, "Simple Architecture Diagram");
    }

    #[test]
    fn should_handle_title_on_another_line() {
        let diagram = parsed(
            "architecture-beta
            title Simple Architecture Diagram
            ",
        );
        assert_eq!(diagram.title, "Simple Architecture Diagram");
    }

    #[test]
    fn should_handle_accessibility_title_and_description() {
        let diagram = parsed(
            "architecture-beta
            accTitle: Accessibility Title
            accDescr: Accessibility Description
            ",
        );
        assert_eq!(diagram.acc_title, "Accessibility Title");
        assert_eq!(diagram.acc_descr, "Accessibility Description");
    }

    #[test]
    fn should_handle_multiline_accessibility_description() {
        let diagram = parsed(
            "architecture-beta
            accDescr {
                Accessibility Description
            }
            ",
        );
        assert_eq!(diagram.acc_descr, "Accessibility Description");
    }
}

mod cytoscape_stylesheet_warnings {
    #[test]
    #[ignore = "port: the stylesheet and the cytoscape label warnings are the renderer stage (docs-port/architecture/PLAN.md, section 8)"]
    fn should_not_produce_console_warnings_for_edges_without_labels() {}
}

mod randomize_config {
    use super::*;

    #[test]
    #[ignore = "port: the defaults are the configuration's, compared by pnpm port:diff config (docs-port/architecture/PLAN.md, section 7)"]
    fn should_default_randomize_to_false() {}

    #[test]
    fn should_parse_a_complex_deeply_nested_diagram_with_randomize_defaulting_to_false() {
        let diagram = parsed(
            "architecture-beta
    group sub1(cloud)[Subscription A]
    group vnet1(cloud)[VNet A] in sub1
    service vm1(server)[VM] in vnet1

    group sub2(cloud)[Subscription B]
    group shared(cloud)[Shared] in sub2
    service reg(database)[Registry] in shared

    group env(cloud)[Environment] in sub2
    group vnet(cloud)[VNet] in env
    group snet1(cloud)[App Subnet] in vnet
    service nsg(server)[NSG] in snet1
    service asp(server)[App Plan] in snet1
    service web(server)[Web App] in snet1

    group snet2(cloud)[PE Subnet] in vnet
    service pe1(server)[PE Blob] in snet2
    service pe2(server)[PE Bus] in snet2

    service storage(disk)[Storage] in env
    service container(disk)[Container] in env
    service bus(server)[Service Bus] in env
    service dns(server)[DNS Zone] in env

    service client(internet)[Client]

    reg:B --> T:web
    nsg:R --> L:asp
    asp:R --> L:web
    web:R --> L:pe1
    pe1:R --> L:storage
    storage:B --> T:container
    web:B --> T:pe2
    pe2:R --> L:bus
    vm1:R --> L:pe2",
        );
        assert_eq!(diagram.services.len(), 12);
        assert_eq!(diagram.groups.len(), 8);
        assert_eq!(diagram.edges.len(), 9);
    }
}

mod fcose_layout_config {
    #[test]
    #[ignore = "port: the fcose knobs are the configuration's and the layout's (docs-port/architecture/PLAN.md, sections 7 and 8)"]
    fn should_default_the_fcose_knobs_to_documented_values() {}

    #[test]
    #[ignore = "port: the fcose knobs are the configuration's and the layout's (docs-port/architecture/PLAN.md, sections 7 and 8)"]
    fn should_round_trip_user_supplied_fcose_knobs() {}

    #[test]
    #[ignore = "port: the fcose knobs are the configuration's and the layout's (docs-port/architecture/PLAN.md, sections 7 and 8)"]
    fn should_leave_defaults_intact_when_only_one_knob_is_set() {}
}

#[test]
fn should_throw_when_missing_a_group() {
    // `__proto__` is not a group here, so the service's parent does not exist.
    let message = rejected(
        "architecture-beta
      group mermaidPrototypePollutionMarker(cloud)[Marker]
      service a(server)[A] in __proto__
      service b(server)[B] in mermaidPrototypePollutionMarker
      a:R -- L:b",
    );
    assert!(
        message.contains("The service [a]'s parent does not exist."),
        "{message}"
    );
}

#[test]
fn should_allow_proto_as_a_group_name() {
    let diagram = parsed(
        "architecture-beta
      group __proto__(cloud)[Marker]
      service a(server)[A] in __proto__
      service b(server)[B] in __proto__
      a:R -- L:b",
    );
    let ids: Vec<&str> = diagram.groups.iter().map(|g| g.id.as_str()).collect();
    assert!(ids.contains(&"__proto__"));
}

#[test]
fn should_block_proto_pollution_via_service_id() {
    // GHSA-3rrr-jr9j-h3q3 happened in the alignment map of the DB.
    let diagram = parsed(
        "architecture-beta
      group __proto__(cloud)[P]
      group myPrototypePollutionKey(cloud)[Real]
      service a(server)[A] in __proto__
      service b(server)[B] in myPrototypePollutionKey
      a:R -- L:b",
    );
    let keys: Vec<&str> = diagram
        .derived
        .group_alignments
        .iter()
        .map(|(key, _)| key.as_str())
        .collect();
    assert!(keys.contains(&"\"__proto__\"-\"myPrototypePollutionKey\""));
}

#[test]
fn should_store_services_in_the_order_they_were_added() {
    let diagram = parsed(
        "architecture-beta
      service a(server)[A]
      service 20(server)[B]
      service 10(server)[C]",
    );
    // The reference's object iteration would sort the numeric keys first; the DB keeps the order.
    assert_eq!(ids_of_services(&diagram), ["a", "20", "10"]);
}

mod align_directive {
    use super::*;

    #[test]
    fn should_parse_a_row_alignment_and_expose_it_via_get_layout_hints() {
        let diagram = parsed(
            "architecture-beta
  group api(cloud)[API]
  service db1(database)[DB1] in api
  service db2(database)[DB2] in api
  service db3(database)[DB3] in api
  align row db1 db2 db3",
        );
        assert_eq!(diagram.alignments.len(), 1);
        assert_eq!(diagram.alignments[0].direction, AlignDirection::Row);
        assert_eq!(diagram.alignments[0].members, ["db1", "db2", "db3"]);
    }

    #[test]
    fn should_parse_a_column_alignment() {
        let diagram = parsed(
            "architecture-beta
  service a(server)[A]
  service b(server)[B]
  align column a b",
        );
        assert_eq!(diagram.alignments.len(), 1);
        assert_eq!(diagram.alignments[0].direction, AlignDirection::Column);
        assert_eq!(diagram.alignments[0].members, ["a", "b"]);
    }

    #[test]
    fn should_reject_an_align_directive_whose_member_is_not_a_service_or_junction() {
        let message = rejected(
            "architecture-beta
  service a(server)[A]
  service b(server)[B]
  align row a b ghost",
        );
        assert!(message.contains("ghost"), "{message}");
    }

    #[test]
    fn should_reject_an_align_directive_that_lists_the_same_member_twice() {
        let message = rejected(
            "architecture-beta
  service a(server)[A]
  align row a a",
        );
        assert!(message.contains("more than once"), "{message}");
    }

    #[test]
    fn should_not_collide_with_services_whose_id_starts_with_row_or_column() {
        let diagram = parsed(
            "architecture-beta
  service rowspan(server)[Rowspan]
  service columnar(server)[Columnar]
  align row rowspan columnar",
        );
        assert_eq!(ids_of_services(&diagram), ["rowspan", "columnar"]);
        assert_eq!(diagram.alignments[0].members, ["rowspan", "columnar"]);
    }

    #[test]
    fn should_reject_a_service_whose_id_is_the_exact_reserved_keyword_align() {
        rejected("architecture-beta\n  service align(database)[Service Using Reserved Word]");
    }

    #[test]
    fn should_reject_a_service_whose_id_is_the_exact_reserved_keyword_row() {
        rejected("architecture-beta\n  service row(database)[Service Using Reserved Word]");
    }

    #[test]
    fn should_reject_a_service_whose_id_is_the_exact_reserved_keyword_column() {
        rejected("architecture-beta\n  service column(database)[Service Using Reserved Word]");
    }
}

mod add_junction_validation {
    use super::*;

    #[test]
    fn should_throw_if_junction_id_is_already_in_use_by_a_service() {
        let message = rejected(
            "architecture-beta
  group g1(cloud)[Group]
  service svc(server)[Svc] in g1
  junction svc",
        );
        assert!(message.contains("already in use"), "{message}");
    }

    #[test]
    fn should_throw_if_junction_id_is_already_in_use_by_a_group() {
        let message = rejected(
            "architecture-beta
  group g1(cloud)[Group]
  junction g1",
        );
        assert!(message.contains("already in use"), "{message}");
    }

    #[test]
    fn should_throw_if_junction_parent_does_not_exist() {
        let message = rejected(
            "architecture-beta
  junction j1 in nonexistent",
        );
        assert!(message.contains("parent does not exist"), "{message}");
    }

    #[test]
    fn should_throw_if_junction_parent_is_a_node_service_not_a_group() {
        let message = rejected(
            "architecture-beta
  service app(server)[App]
  junction j1 in app",
        );
        assert!(message.contains("parent is not a group"), "{message}");
    }

    #[test]
    fn should_throw_if_junction_is_placed_within_itself() {
        let message = rejected(
            "architecture-beta
  junction j1 in j1",
        );
        assert!(
            message.contains("cannot be placed within itself"),
            "{message}"
        );
    }

    #[test]
    fn should_allow_junction_in_a_valid_group() {
        let diagram = parsed(
            "architecture-beta
  group app_env(cloud)[App Env]
  junction mid in app_env",
        );
        assert_eq!(diagram.junctions.len(), 1);
    }

    #[test]
    fn should_not_crash_with_group_name_prefix_matching_a_service_name_issue_7408() {
        let diagram = parsed(
            "architecture-beta
  group app_env(cloud)[App Env]
  service app(server)[Web App] in app_env
  service db(database)[DB] in app_env
  service api(server)[API] in app_env
  junction mid in app_env
  app:B -- T:mid
  mid:R -- L:db
  mid:B -- T:api",
        );
        assert_eq!(diagram.junctions.len(), 1);
        assert_eq!(diagram.junctions[0].parent, "app_env");
    }
}

// The renderer's specs (the seeded random generator of the layout and the SVG drawing of
// `architectureSeed.spec.ts` and `svgDraw.spec.ts`) are ported to `crates/ferrigura-render/tests/
// architecture_render.rs`, where the renderer is, so they are not repeated here.
