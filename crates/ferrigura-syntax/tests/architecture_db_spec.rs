//! The architecture DB: the order of the lists, the header texts and their sanitising, the front
//! matter title, and the derived block (`getDataStructures`: the group alignments and the spatial
//! maps). The expected values are the reference's, from its DB after the same text
//! (`tools/port/export-architecture.ts`), and the differential run compares every input of these
//! tests as well.

use ferrigura_model::architecture::{AlignDirection, ArchitectureDiagram, Axis, Direction};
use ferrigura_syntax::architecture::parse;

const HEADER: &str = "architecture-beta\n";

fn parsed(text: &str) -> ArchitectureDiagram {
    parse(text).expect("the diagram parses")
}

/// The spatial map of the first component, as `(id, x, y)` triples.
fn first_map(diagram: &ArchitectureDiagram) -> Vec<(&str, i64, i64)> {
    diagram.derived.spatial_maps[0]
        .iter()
        .map(|p| (p.id.as_str(), p.x, p.y))
        .collect()
}

#[test]
fn groups_are_added_before_services_whatever_the_text_order() {
    let diagram = parsed(&format!(
        "{HEADER}service a(server)[A] in g\ngroup g(cloud)[G]\njunction j in g\n"
    ));
    assert_eq!(diagram.groups.len(), 1);
    assert_eq!(diagram.services[0].parent, "g");
    assert_eq!(diagram.junctions[0].parent, "g");
}

#[test]
fn services_and_junctions_are_separate_lists_in_text_order() {
    let diagram = parsed(&format!(
        "{HEADER}junction j1\nservice b(server)[B]\njunction j2\nservice a(server)[A]\n"
    ));
    let services: Vec<&str> = diagram.services.iter().map(|s| s.id.as_str()).collect();
    let junctions: Vec<&str> = diagram.junctions.iter().map(|j| j.id.as_str()).collect();
    assert_eq!(services, ["b", "a"]);
    assert_eq!(junctions, ["j1", "j2"]);
}

#[test]
fn an_edge_keeps_its_sides_arrows_modifiers_and_label() {
    let diagram = parsed(&format!(
        "{HEADER}group g1(cloud)[G1]\ngroup g2(cloud)[G2]\nservice a(server)[A] in g1\nservice b(server)[B] in g2\na{{group}}:R <-[link]-> L:b{{group}}\n"
    ));
    let edge = &diagram.edges[0];
    assert_eq!(edge.lhs_id, "a");
    assert_eq!(edge.lhs_dir, Direction::R);
    assert!(edge.lhs_group);
    // `<` before the line is the left arrow head, `>` after the label the right one.
    assert!(edge.lhs_into);
    assert_eq!(edge.rhs_id, "b");
    assert_eq!(edge.rhs_dir, Direction::L);
    assert!(edge.rhs_group);
    assert!(edge.rhs_into);
    assert_eq!(edge.title, "link");
}

#[test]
fn an_edge_without_arrows_or_label_has_false_flags_and_an_empty_label() {
    let diagram = parsed(&format!(
        "{HEADER}service a(server)[A]\nservice b(server)[B]\na:R -- L:b\n"
    ));
    let edge = &diagram.edges[0];
    assert!(!edge.lhs_into && !edge.rhs_into && !edge.lhs_group && !edge.rhs_group);
    assert_eq!(edge.title, "");
}

#[test]
fn alignments_keep_their_direction_and_members_in_text_order() {
    let diagram = parsed(&format!(
        "{HEADER}service a(server)[A]\nservice b(server)[B]\nservice c(server)[C]\nalign row c a\nalign column a b\n"
    ));
    assert_eq!(diagram.alignments.len(), 2);
    assert_eq!(diagram.alignments[0].direction, AlignDirection::Row);
    assert_eq!(diagram.alignments[0].members, ["c", "a"]);
    assert_eq!(diagram.alignments[1].direction, AlignDirection::Column);
}

#[test]
fn the_header_texts_are_sanitised_as_the_common_setters() {
    let diagram = parsed(&format!(
        "{HEADER}title Hello <b>bold</b> & more\naccTitle:    <i>x</i>   \naccDescr: x<img src=x onerror=alert(1)>y\n"
    ));
    assert_eq!(diagram.title, "Hello <b>bold</b> &amp; more");
    assert_eq!(diagram.acc_title, "<i>x</i>");
    assert_eq!(diagram.acc_descr, "x<img src=\"x\">y");
}

#[test]
fn an_accessibility_description_keeps_its_line_breaks_and_drops_the_indentation() {
    let diagram = parsed(&format!(
        "{HEADER}accDescr {{\n   <b>bold</b>\n   text\n}}\n"
    ));
    assert_eq!(diagram.acc_descr, "<b>bold</b>\ntext");
}

#[test]
fn the_grammar_title_wins_over_the_front_matter_title() {
    let diagram = parse(&format!(
        "---\ntitle: FM Title\n---\n{HEADER}title Grammar\nservice a(server)[A]\n"
    ))
    .expect("parses");
    assert_eq!(diagram.title, "Grammar");
}

#[test]
fn the_front_matter_title_is_kept_when_the_grammar_gives_none() {
    let diagram = parse(&format!(
        "---\ntitle: FM Title\n---\n{HEADER}service a(server)[A]\n"
    ))
    .expect("parses");
    assert_eq!(diagram.title, "FM Title");
}

#[test]
fn an_empty_last_grammar_title_lets_the_front_matter_title_through() {
    let diagram = parse(&format!("---\ntitle: FM\n---\n{HEADER}title A\ntitle\n")).expect("parses");
    assert_eq!(diagram.title, "FM");
}

#[test]
fn a_front_matter_title_is_sanitised_too() {
    let diagram = parse(&format!("---\ntitle: FM <b>x</b>\n---\n{HEADER}title\n")).expect("parses");
    assert_eq!(diagram.title, "FM <b>x</b>");
}

#[test]
fn a_group_alignment_is_horizontal_for_sides_on_the_horizontal_axis() {
    let diagram = parsed(&format!(
        "{HEADER}group g1(cloud)[G1]\ngroup g2(cloud)[G2]\nservice a(server)[A] in g1\nservice b(server)[B] in g2\na:R -- L:b\n"
    ));
    assert_eq!(
        diagram.derived.group_alignments,
        [(String::from("\"g1\"-\"g2\""), Axis::Horizontal)]
    );
}

#[test]
fn a_later_edge_between_the_same_groups_overwrites_the_alignment_in_place() {
    // Reference: `"g1"-"g2"` is vertical, the later edge's alignment.
    let diagram = parsed(&format!(
        "{HEADER}group g1(cloud)[G1]\ngroup g2(cloud)[G2]\nservice a(server)[A] in g1\nservice b(server)[B] in g2\na:R -- L:b\nb:B -- T:a\n"
    ));
    assert_eq!(
        diagram.derived.group_alignments,
        [(String::from("\"g1\"-\"g2\""), Axis::Vertical)]
    );
}

#[test]
fn a_bend_between_groups_is_no_group_alignment() {
    let diagram = parsed(&format!(
        "{HEADER}group g1(cloud)[G1]\ngroup g2(cloud)[G2]\nservice a(server)[A] in g1\nservice b(server)[B] in g2\na:R -- T:b\n"
    ));
    assert!(diagram.derived.group_alignments.is_empty());
}

#[test]
fn a_spatial_map_places_the_neighbour_one_step_along_the_edge() {
    let diagram = parsed(&format!(
        "{HEADER}service a(server)[A]\nservice b(server)[B]\na:R -- L:b\n"
    ));
    assert_eq!(first_map(&diagram), [("a", 0, 0), ("b", 1, 0)]);
}

#[test]
fn each_disconnected_component_has_its_own_spatial_map() {
    let diagram = parsed(&format!(
        "{HEADER}service a(server)[A]\nservice b(server)[B]\n"
    ));
    assert_eq!(diagram.derived.spatial_maps.len(), 2);
    assert_eq!(diagram.derived.spatial_maps[0][0].id, "a");
    assert_eq!(diagram.derived.spatial_maps[1][0].id, "b");
}

#[test]
fn a_node_queued_twice_takes_the_position_of_its_last_path() {
    // The reference sets `visited` when a node is taken from the queue, not when it is queued. `c`
    // is queued from `a` at (0,-1), then again from `b` at (1,-1); the last position wins.
    let diagram = parsed(&format!(
        "{HEADER}service a(server)[A]\nservice b(server)[B]\nservice c(server)[C]\na:R -- L:b\na:B -- T:c\nb:B -- T:c\n"
    ));
    assert_eq!(
        first_map(&diagram),
        [("a", 0, 0), ("b", 1, 0), ("c", 1, -1)]
    );
}

#[test]
fn a_pair_that_a_node_has_twice_is_visited_in_the_order_of_its_first_insertion() {
    // The adjacency of `a` holds `RL` then `TB`, both to `b`; the breadth-first search places `b`
    // at each in turn and keeps the last one.
    let diagram = parsed(&format!(
        "{HEADER}service a(server)[A]\nservice b(server)[B]\na:R -- L:b\na:T -- B:b\n"
    ));
    assert_eq!(first_map(&diagram), [("a", 0, 0), ("b", 0, 1)]);
}

#[test]
fn junctions_come_after_the_services_in_the_search_order() {
    let diagram = parsed(&format!(
        "{HEADER}service a(server)[A]\njunction j\nservice b(server)[B]\na:R -- L:j\nj:R -- L:b\n"
    ));
    assert_eq!(first_map(&diagram), [("a", 0, 0), ("j", 1, 0), ("b", 2, 0)]);
}

#[test]
fn a_self_loop_is_no_adjacency_and_no_group_alignment() {
    let diagram = parsed(&format!("{HEADER}service a(server)[A]\na:R -- L:a\n"));
    assert_eq!(first_map(&diagram), [("a", 0, 0)]);
    assert!(diagram.derived.group_alignments.is_empty());
}
