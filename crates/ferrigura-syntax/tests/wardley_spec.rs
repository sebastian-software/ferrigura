//! The wardley parser spec, `packages/mermaid/src/diagrams/wardley/wardleyParser.spec.ts`, ported
//! test by test: the same texts, and the same reads of the DB state (`getWardleyData`). The
//! expected values are the reference's, and the differential run compares these inputs as well.

use ferrigura_model::wardley::{Flow, NodeClass, Strategy, WardleyDiagram};
use ferrigura_syntax::wardley::parse;

fn parsed(text: &str) -> WardleyDiagram {
    parse(text).expect("the map parses")
}

fn node<'a>(diagram: &'a WardleyDiagram, label: &str) -> &'a ferrigura_model::wardley::Node {
    diagram
        .nodes
        .iter()
        .find(|node| node.label == label)
        .unwrap_or_else(|| panic!("a node labelled {label:?}"))
}

/// `toBeCloseTo` with the precision of the arithmetic here: the values are exact, or off by an ulp.
fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1e-9
}

#[test]
fn parses_components_and_links() {
    let diagram = parsed(
        "wardley-beta\ntitle Example\ncomponent Alpha [0.2, 0.1]\ncomponent Beta [0.4, 0.3]\nAlpha -> Beta",
    );
    assert_eq!(diagram.nodes.len(), 2);
    assert_eq!(diagram.links.len(), 1);
    assert_eq!(diagram.title, "Example");
}

#[test]
fn parses_custom_evolution_stages() {
    let diagram = parsed(
        "wardley-beta\ntitle Test\nevolution Genesis -> Custom -> Product -> Commodity\ncomponent A [0.5, 0.5]",
    );
    assert_eq!(
        diagram.axes.stages,
        Some(vec![
            "Genesis".to_owned(),
            "Custom".to_owned(),
            "Product".to_owned(),
            "Commodity".to_owned()
        ])
    );
}

#[test]
fn parses_dual_label_evolution_stages_with_slashes() {
    let diagram = parsed(
        "wardley-beta\ntitle Test\nevolution Genesis / Concept -> Custom / Emerging -> Product / Converging -> Commodity / Accepted\ncomponent A [0.5, 0.5]",
    );
    assert_eq!(
        diagram.axes.stages,
        Some(
            [
                "Genesis / Concept",
                "Custom / Emerging",
                "Product / Converging",
                "Commodity / Accepted",
            ]
            .map(str::to_owned)
            .to_vec()
        )
    );
}

#[test]
fn parses_pipeline_blocks_with_single_coordinate_components() {
    let diagram = parsed(
        "wardley-beta\ntitle Test Pipeline\ncomponent Kettle [0.45, 0.57]\npipeline Kettle {\n  component Campfire Kettle [0.35] label [-60, 35]\n  component Electric Kettle [0.53]\n}",
    );
    let kettle = node(&diagram, "Kettle");
    let campfire = node(&diagram, "Campfire Kettle");
    let electric = node(&diagram, "Electric Kettle");

    // Pipeline components inherit the Y coordinate of their parent.
    assert_eq!(campfire.y, kettle.y);
    assert_eq!(electric.y, kettle.y);
    // They have their own X coordinates, the evolution values.
    assert!(close(campfire.x, 35.0));
    assert!(close(electric.x, 53.0));
    assert_eq!(campfire.label_offset_x.map(|n| n.0), Some(-60.0));
    assert_eq!(campfire.label_offset_y.map(|n| n.0), Some(35.0));
}

#[test]
fn parses_custom_stage_widths_with_boundary_notation() {
    let diagram = parsed(
        "wardley-beta\ntitle Test Custom Widths\nevolution Genesis@0.3 -> Custom@0.6 -> Product@0.85 -> Commodity@1.0\ncomponent A [0.5, 0.5]",
    );
    assert_eq!(
        diagram.axes.stages,
        Some(
            ["Genesis", "Custom", "Product", "Commodity"]
                .map(str::to_owned)
                .to_vec()
        )
    );
    let boundaries: Vec<f64> = diagram
        .axes
        .stage_boundaries
        .expect("the boundaries")
        .iter()
        .map(|n| n.0)
        .collect();
    assert_eq!(boundaries, vec![0.3, 0.6, 0.85, 1.0]);
}

#[test]
fn parses_notes_with_quoted_text() {
    let diagram = parsed(
        "wardley-beta\ntitle Test Notes\ncomponent API [0.6, 0.7]\nnote \"Critical decision point\" [0.65, 0.55]",
    );
    assert_eq!(diagram.notes.len(), 1);
    assert_eq!(diagram.notes[0].text, "Critical decision point");
}

#[test]
fn parses_annotations_with_quoted_text() {
    let diagram = parsed(
        "wardley-beta\ntitle Test Annotations\ncomponent API [0.6, 0.7]\nannotations [0.1, 0.9]\nannotation 1,[0.6, 0.65] \"Critical component\"\nannotation 2,[0.5, 0.5] \"Performance layer\"",
    );
    assert_eq!(diagram.annotations.len(), 2);
    assert_eq!(diagram.annotations[0].text, "Critical component");
    assert_eq!(diagram.annotations[1].text, "Performance layer");
}

#[test]
fn parses_anchors() {
    let diagram = parsed(
        "wardley-beta\ntitle Test Anchors\nanchor Business [0.95, 0.63]\nanchor Public [0.95, 0.78]\ncomponent Tea [0.63, 0.81]",
    );
    let anchors: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|node| node.class_name == Some(NodeClass::Anchor))
        .collect();
    assert_eq!(anchors.len(), 2);
    assert_eq!(anchors[0].label, "Business");
}

#[test]
fn parses_evolve_statements() {
    let diagram = parsed(
        "wardley-beta\ntitle Test Evolve\ncomponent Kettle [0.35, 0.43]\nevolve Kettle 0.62",
    );
    assert_eq!(diagram.trends.len(), 1);
    assert!(close(diagram.trends[0].target_x, 62.0));
}

#[test]
fn parses_component_decorators() {
    let diagram = parsed(
        "wardley-beta\ntitle Test Decorators\ncomponent API [0.6, 0.7] (build)\ncomponent Database [0.4, 0.5] (buy)\ncomponent Cache [0.5, 0.6] (outsource)",
    );
    assert_eq!(node(&diagram, "API").source_strategy, Some(Strategy::Build));
    assert_eq!(
        node(&diagram, "Database").source_strategy,
        Some(Strategy::Buy)
    );
    assert_eq!(
        node(&diagram, "Cache").source_strategy,
        Some(Strategy::Outsource)
    );
}

#[test]
fn parses_size_directive() {
    let diagram = parsed("wardley-beta\ntitle Test Size\nsize [1200, 900]\ncomponent A [0.5, 0.5]");
    let size = diagram.size.expect("a size");
    assert_eq!(size.width.0, 1200.0);
    assert_eq!(size.height.0, 900.0);
}

#[test]
fn handles_quoted_identifiers_inline_labels_and_converts_coordinates_to_percentages() {
    let diagram = parsed(
        "wardley-beta\ntitle Coordinate Handling\ncomponent \"Mobile App\" [0.2, 0.4] (build) (inertia)\ncomponent API [0.3, 0.5]\n\"Mobile App\" +<> API; constraint\nnote \"Check dependencies\" [0.25, 0.45]\nannotations [0.10, 0.90]\nannotation 1,[0.60, 0.65] \"Critical component\"\naccelerator \"Cloud Native\" [0.20, 0.85]\ndeaccelerator \"Legacy Data\" [0.40, 0.35]",
    );

    let mobile = node(&diagram, "Mobile App");
    assert!(close(mobile.x, 40.0));
    assert!(close(mobile.y, 20.0));
    assert_eq!(mobile.source_strategy, Some(Strategy::Build));
    assert_eq!(mobile.inertia, Some(true));

    assert_eq!(diagram.links.len(), 1);
    assert_eq!(diagram.links[0].flow, Some(Flow::Bidirectional));
    assert_eq!(diagram.links[0].label.as_deref(), Some("constraint"));

    assert!(close(diagram.notes[0].x, 45.0));
    assert!(close(diagram.notes[0].y, 25.0));

    let boxed = diagram.annotations_box.expect("an annotations box");
    assert_eq!((boxed.x, boxed.y), (90.0, 10.0));
    assert_eq!(diagram.annotations[0].text, "Critical component");
    let point = diagram.annotations[0].coordinates[0];
    assert_eq!((point.x, point.y), (65.0, 60.0));

    assert_eq!(diagram.accelerators[0].name, "Cloud Native");
    assert!(close(diagram.accelerators[0].x, 85.0));
    assert!(close(diagram.accelerators[0].y, 20.0));
    assert_eq!(diagram.deaccelerators[0].name, "Legacy Data");
    assert!(close(diagram.deaccelerators[0].x, 35.0));
    assert!(close(diagram.deaccelerators[0].y, 40.0));
}

#[test]
fn accepts_hyphens_in_unquoted_component_names() {
    let diagram = parsed(
        "wardley-beta\ncomponent real-time processing [0.5, 0.5]\ncomponent end-user [0.8, 0.9]",
    );
    let realtime = node(&diagram, "real-time processing");
    let enduser = node(&diagram, "end-user");
    assert!(close(realtime.x, 50.0));
    assert!(close(realtime.y, 50.0));
    assert!(close(enduser.x, 90.0));
    assert!(close(enduser.y, 80.0));
}

#[test]
fn accepts_hyphens_in_link_endpoints() {
    let diagram = parsed(
        "wardley-beta\ncomponent real-time processing [0.5, 0.5]\ncomponent end-user [0.8, 0.9]\nreal-time processing -> end-user",
    );
    assert_eq!(diagram.links.len(), 1);
    assert_eq!(diagram.links[0].source, "real-time processing");
    assert_eq!(diagram.links[0].target, "end-user");
}

#[test]
fn accepts_hyphens_in_anchor_names() {
    let diagram = parsed("wardley-beta\nanchor on-call engineer [0.9, 0.95]");
    let anchor = node(&diagram, "on-call engineer");
    assert_eq!(anchor.class_name, Some(NodeClass::Anchor));
    assert!(close(anchor.x, 95.0));
    assert!(close(anchor.y, 90.0));
}

#[test]
fn still_tokenises_a_to_b_as_an_arrow() {
    let diagram = parsed("wardley-beta\ncomponent A [0.1, 0.1]\ncomponent B [0.2, 0.2]\nA->B");
    assert_eq!(diagram.links.len(), 1);
    assert_eq!(diagram.links[0].source, "A");
    assert_eq!(diagram.links[0].target, "B");
}

#[test]
fn parses_foo_bar_arrow_baz_as_a_link_from_foo_bar_to_baz() {
    let diagram = parsed(
        "wardley-beta\ncomponent foo-bar [0.3, 0.3]\ncomponent baz [0.6, 0.6]\nfoo-bar->baz",
    );
    assert_eq!(diagram.links.len(), 1);
    assert_eq!(diagram.links[0].source, "foo-bar");
    assert_eq!(diagram.links[0].target, "baz");
}

#[test]
fn accepts_hyphens_in_pipeline_component_names_with_label_overrides() {
    let diagram = parsed(
        "wardley-beta\ncomponent Data Store [0.5, 0.5]\npipeline Data Store {\n  component real-time queue [0.3] label [-40, 20]\n  component batch-loader [0.7]\n}",
    );
    let realtime = node(&diagram, "real-time queue");
    let batch = node(&diagram, "batch-loader");
    assert_eq!(realtime.class_name, Some(NodeClass::PipelineComponent));
    assert_eq!(realtime.label_offset_x.map(|n| n.0), Some(-40.0));
    assert_eq!(realtime.label_offset_y.map(|n| n.0), Some(20.0));
    assert!(close(realtime.x, 30.0));
    assert_eq!(batch.class_name, Some(NodeClass::PipelineComponent));
    assert!(close(batch.x, 70.0));
}

#[test]
fn accepts_consecutive_hyphens_inside_a_name() {
    let diagram = parsed("wardley-beta\ncomponent foo--bar [0.3, 0.4]");
    let node = node(&diagram, "foo--bar");
    assert!(close(node.x, 40.0));
    assert!(close(node.y, 30.0));
}

#[test]
fn accepts_a_trailing_hyphen_in_a_name() {
    let diagram = parsed("wardley-beta\ncomponent foo- [0.2, 0.3]");
    let node = node(&diagram, "foo-");
    assert!(close(node.x, 30.0));
    assert!(close(node.y, 20.0));
}
