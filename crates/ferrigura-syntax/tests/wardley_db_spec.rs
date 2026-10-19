//! The wardley DB: the order of the lists, the merge of a repeated node, the pipelines, the links
//! and their resolution, the coordinates and their errors, and the header texts. The quirks D1 to
//! D10 of `docs-port/wardley/NOTES.md`, section 2.5, are the first tests; the probes are the
//! reference's own inputs, and the expected values are the reference's DB state.

use ferrigura_model::wardley::{Flow, NodeClass, Strategy, WardleyDiagram};
use ferrigura_syntax::wardley::parse;

const HEADER: &str = "wardley-beta\n";

fn parsed(text: &str) -> WardleyDiagram {
    parse(text).expect("the map parses")
}

/// Percentages are computed from decimals, so they are compared with a tolerance.
fn near(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1e-9
}

fn assert_pair((ax, ay): (f64, f64), (ex, ey): (f64, f64)) {
    assert!(
        near(ax, ex) && near(ay, ey),
        "({ax}, {ay}) is not ({ex}, {ey})"
    );
}

fn labels(diagram: &WardleyDiagram) -> Vec<&str> {
    diagram
        .nodes
        .iter()
        .map(|node| node.label.as_str())
        .collect()
}

fn find<'a>(diagram: &'a WardleyDiagram, id: &str) -> &'a ferrigura_model::wardley::Node {
    diagram
        .nodes
        .iter()
        .find(|node| node.id == id)
        .unwrap_or_else(|| panic!("a node with id {id:?}"))
}

#[test]
fn d1_a_repeated_component_is_one_node_at_the_first_position_with_the_last_values() {
    let diagram = parsed(&format!(
        "{HEADER}component A [0.1, 0.2]\ncomponent B [0.3, 0.4]\ncomponent A [0.5, 0.6]\n"
    ));
    assert_eq!(labels(&diagram), vec!["A", "B"]);
    let a = find(&diagram, "A");
    assert_pair((a.x, a.y), (60.0, 50.0));
}

#[test]
fn d1_a_repeated_node_keeps_a_label_offset_it_is_not_given_again() {
    let diagram = parsed(&format!(
        "{HEADER}component A [0.1, 0.2] label [1, 2] (build) (inertia)\ncomponent A [0.1, 0.2]\n"
    ));
    let a = find(&diagram, "A");
    assert_eq!(a.label_offset_x.map(|n| n.0), Some(1.0));
    assert_eq!(a.label_offset_y.map(|n| n.0), Some(2.0));
    // The inertia and the strategy are overwritten by the second call, even with nothing.
    assert_eq!(a.inertia, Some(false));
    assert_eq!(a.source_strategy, None);
}

#[test]
fn an_anchor_and_a_component_of_one_name_share_a_node_of_the_later_class() {
    let diagram = parsed(&format!(
        "{HEADER}anchor A [0.9, 0.9]\ncomponent A [0.1, 0.2]\n"
    ));
    assert_eq!(diagram.nodes.len(), 1);
    assert_eq!(diagram.nodes[0].class_name, Some(NodeClass::Component));
    assert_eq!(diagram.nodes[0].inertia, Some(false));
}

#[test]
fn d2_a_second_pipeline_for_one_parent_replaces_the_first_components_of_its_box() {
    let diagram = parsed(&format!(
        "{HEADER}component K [0.5, 0.5]\npipeline K {{\n  component X [0.3]\n}}\npipeline K {{\n  component Y [0.6]\n}}\n"
    ));
    assert_eq!(diagram.pipelines.len(), 1);
    assert_eq!(diagram.pipelines[0].node_id, "K");
    assert_eq!(diagram.pipelines[0].component_ids, vec!["K_Y".to_owned()]);
    // Both sets of component nodes exist.
    assert!(diagram.nodes.iter().any(|node| node.id == "K_X"));
    assert!(diagram.nodes.iter().any(|node| node.id == "K_Y"));
    assert_eq!(find(&diagram, "K").is_pipeline_parent, Some(true));
    assert_eq!(find(&diagram, "K_X").in_pipeline, Some(true));
}

#[test]
fn pipeline_components_take_the_parent_y_and_their_evolution_as_x() {
    let diagram = parsed(&format!(
        "{HEADER}component K [0.5, 0.25]\npipeline K {{\n  component X [0.3] label [-1, -2]\n}}\n"
    ));
    let x = find(&diagram, "K_X");
    assert_pair((x.x, x.y), (30.0, 50.0));
    assert_eq!(x.class_name, Some(NodeClass::PipelineComponent));
    assert_eq!(x.label_offset_x.map(|n| n.0), Some(-1.0));
    assert_eq!(x.label_offset_y.map(|n| n.0), Some(-2.0));
    assert_eq!(x.inertia, None);
    assert_eq!(x.source_strategy, None);
}

#[test]
fn d3_a_link_between_undeclared_names_is_kept_with_its_names() {
    let diagram = parsed(&format!("{HEADER}X -> Y\n"));
    assert_eq!(diagram.links.len(), 1);
    assert_eq!(diagram.links[0].source, "X");
    assert_eq!(diagram.links[0].target, "Y");
    assert!(diagram.nodes.is_empty());
}

#[test]
fn d4_an_evolve_of_an_undeclared_name_gives_no_trend() {
    let diagram = parsed(&format!("{HEADER}evolve Nope 0.8\n"));
    assert!(diagram.trends.is_empty());
}

#[test]
fn d5_an_evolve_before_its_component_works() {
    let diagram = parsed(&format!("{HEADER}evolve A 0.8\ncomponent A [0.5, 0.5]\n"));
    assert_eq!(diagram.trends.len(), 1);
    assert_eq!(diagram.trends[0].node_id, "A");
    assert_pair(
        (diagram.trends[0].target_x, diagram.trends[0].target_y),
        (80.0, 50.0),
    );
}

#[test]
fn the_last_evolve_of_a_component_wins() {
    let diagram = parsed(&format!(
        "{HEADER}component A [0.5, 0.5]\nevolve A 0.2\nevolve A 0.7\n"
    ));
    assert_eq!(diagram.trends.len(), 1);
    assert!(near(diagram.trends[0].target_x, 70.0));
}

#[test]
fn d6_a_link_to_a_pipeline_component_resolves_by_its_label() {
    let diagram = parsed(&format!(
        "{HEADER}component K [0.5, 0.5]\npipeline K {{\n  component Queue [0.3]\n}}\nQueue -> K\n"
    ));
    assert_eq!(diagram.links[0].source, "K_Queue");
    assert_eq!(diagram.links[0].target, "K");
}

#[test]
fn d7_a_pipeline_on_an_unknown_parent_is_an_error() {
    let error = parse(&format!(
        "{HEADER}pipeline Nope {{\n  component X [0.3]\n}}\n"
    ))
    .expect_err("rejected");
    assert!(error.message.contains("Nope"), "{}", error.message);
}

#[test]
fn a_pipeline_on_an_anchor_is_accepted_because_an_anchor_has_coordinates() {
    assert!(
        parse(&format!(
            "{HEADER}anchor P [0.9, 0.9]\npipeline P {{\n  component X [0.3]\n}}\n"
        ))
        .is_ok()
    );
}

#[test]
fn d8_only_the_first_annotations_block_positions_the_box() {
    let diagram = parsed(&format!(
        "{HEADER}annotations [0.10, 0.90]\nannotations [0.20, 0.30]\n"
    ));
    let boxed = diagram.annotations_box.expect("a box");
    assert_pair((boxed.x, boxed.y), (90.0, 10.0));
}

#[test]
fn d9_a_value_above_one_is_not_scaled_and_a_value_of_one_is_a_hundred() {
    let diagram = parsed(&format!("{HEADER}component A [1.5, 1.0]\n"));
    let a = find(&diagram, "A");
    assert_pair((a.x, a.y), (100.0, 1.5));
}

#[test]
fn a_coordinate_above_one_hundred_is_an_error_and_the_evolution_is_checked_first() {
    let error = parse(&format!("{HEADER}component A [100.5, 100.5]\n")).expect_err("rejected");
    assert!(error.message.contains("evolution"), "{}", error.message);
    let error = parse(&format!("{HEADER}component A [100.5, 0.5]\n")).expect_err("rejected");
    assert!(error.message.contains("visibility"), "{}", error.message);
}

#[test]
fn d10_a_flow_label_with_a_less_than_sign_gives_a_backward_flow() {
    let diagram = parsed(&format!(
        "{HEADER}component A [0.1, 0.2]\ncomponent B [0.1, 0.2]\nA +'a<b'> B\n"
    ));
    assert_eq!(diagram.links[0].flow, Some(Flow::Backward));
    assert_eq!(diagram.links[0].label.as_deref(), Some("a<b"));
}

#[test]
fn a_port_gives_the_flow_before_the_arrow_and_the_label_of_the_arrow_wins() {
    let diagram = parsed(&format!(
        "{HEADER}component A [0.1, 0.2]\ncomponent B [0.1, 0.2]\nA +> B +< ;label\nA +'x'> B ;other\n"
    ));
    // The from port `+>` is forward; the `;` label is the link's label.
    assert_eq!(diagram.links[0].flow, Some(Flow::Forward));
    assert_eq!(diagram.links[0].label.as_deref(), Some("label"));
    // The arrow's label wins over the `;` label.
    assert_eq!(diagram.links[1].label.as_deref(), Some("x"));
}

#[test]
fn an_empty_flow_label_is_a_label_and_not_none() {
    let diagram = parsed(&format!(
        "{HEADER}component A [0.1, 0.2]\ncomponent B [0.1, 0.2]\nA +''> B\n"
    ));
    assert_eq!(diagram.links[0].label.as_deref(), Some(""));
}

#[test]
fn the_second_size_wins_and_no_size_is_null() {
    let diagram = parsed(&format!("{HEADER}size [1, 2]\nsize [3, 4]\n"));
    let size = diagram.size.expect("a size");
    assert_pair((size.width.0, size.height.0), (3.0, 4.0));
    assert!(parsed(HEADER).size.is_none());
}

#[test]
fn without_an_evolution_the_axes_are_null() {
    let diagram = parsed(&format!("{HEADER}component A [0.1, 0.2]\n"));
    assert!(diagram.axes.stages.is_none());
    assert!(diagram.axes.stage_boundaries.is_none());
}

#[test]
fn evolution_stages_name_a_second_name_only_when_it_is_not_empty() {
    let diagram = parsed(&format!("{HEADER}evolution A / \"\" -> B / C\n"));
    assert_eq!(
        diagram.axes.stages,
        Some(vec!["A".to_owned(), "B / C".to_owned()])
    );
    // The boundaries are the present ones, in order, and no boundary gives an empty list.
    assert_eq!(
        diagram.axes.stage_boundaries.as_ref().map(Vec::len),
        Some(0)
    );
    let diagram = parsed(&format!("{HEADER}evolution A@0.3 -> B -> C@0.6\n"));
    let boundaries: Vec<f64> = diagram
        .axes
        .stage_boundaries
        .expect("boundaries")
        .iter()
        .map(|n| n.0)
        .collect();
    assert!(near(boundaries[0], 0.3) && boundaries.len() == 2 && near(boundaries[1], 0.6));
}

#[test]
fn an_annotation_with_empty_text_keeps_it_empty_and_its_number() {
    let diagram = parsed(&format!("{HEADER}annotation 3,[0.2, 0.3] \"\"\n"));
    assert_eq!(diagram.annotations[0].number.0, 3.0);
    assert_eq!(diagram.annotations[0].text, "");
}

#[test]
fn accelerators_and_notes_keep_the_text_order_and_their_positions() {
    let diagram = parsed(&format!(
        "{HEADER}accelerator \"B\" [0.2, 0.3]\naccelerator A [0.5, 0.6]\nnote \"n\" [0.1, 0.2]\n"
    ));
    let names: Vec<&str> = diagram
        .accelerators
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert_eq!(names, vec!["B", "A"]);
    assert_pair(
        (diagram.accelerators[0].x, diagram.accelerators[0].y),
        (30.0, 20.0),
    );
    assert_pair((diagram.notes[0].x, diagram.notes[0].y), (20.0, 10.0));
}

#[test]
fn the_nodes_are_in_the_order_the_calls_add_them_anchors_then_components_then_pipelines() {
    let diagram = parsed(&format!(
        "{HEADER}component C [0.5, 0.5]\nanchor A [0.9, 0.9]\npipeline C {{\n  component P [0.3]\n}}\ncomponent B [0.2, 0.2]\n"
    ));
    let ids: Vec<&str> = diagram.nodes.iter().map(|node| node.id.as_str()).collect();
    assert_eq!(ids, vec!["A", "C", "B", "C_P"]);
}

#[test]
fn the_node_of_a_strategy_keeps_it_and_the_component_class_is_its_own() {
    let diagram = parsed(&format!("{HEADER}component A [0.5, 0.5] (buy)\n"));
    assert_eq!(diagram.nodes[0].source_strategy, Some(Strategy::Buy));
    assert_eq!(diagram.nodes[0].class_name, Some(NodeClass::Component));
}

#[test]
fn the_front_matter_title_is_used_until_the_grammar_gives_a_non_empty_title() {
    let diagram =
        parsed("---\ntitle: From front matter\n---\nwardley-beta\ncomponent A [0.1, 0.2]\n");
    assert_eq!(diagram.title, "From front matter");
    let diagram = parsed(
        "---\ntitle: From front matter\n---\nwardley-beta\ntitle Grammar\ncomponent A [0.1, 0.2]\n",
    );
    assert_eq!(diagram.title, "Grammar");
    let diagram = parsed(
        "---\ntitle: From front matter\n---\nwardley-beta\ntitle Grammar\ntitle \ncomponent A [0.1, 0.2]\n",
    );
    assert_eq!(diagram.title, "From front matter");
}

#[test]
fn a_directive_and_a_comment_line_are_not_in_the_model() {
    let diagram = parsed(&format!(
        "%%{{init: {{\"theme\": \"forest\"}}}}%%\n%% a comment\n{HEADER}component A [0.1, 0.2]\n"
    ));
    assert_eq!(labels(&diagram), vec!["A"]);
}

#[test]
fn the_pipeline_component_of_a_name_with_spaces_keeps_its_label_as_written() {
    let diagram = parsed(&format!(
        "{HEADER}component Data Store [0.5, 0.5]\npipeline Data Store {{\n  component real-time queue [0.3]\n}}\n"
    ));
    assert!(
        diagram
            .nodes
            .iter()
            .any(|node| node.id == "Data Store_real-time queue")
    );
}
