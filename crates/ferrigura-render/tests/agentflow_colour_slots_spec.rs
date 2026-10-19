//! Port of packages/mermaid/src/diagrams/agentflow/colorSlots.spec.ts and containerOrder.spec.ts: the
//! colour slots of agentflow. A node takes a colour by its kind, from a fixed slot; a container takes one
//! from a counter in declaration order, above the kinds' slots. The stylesheet's rules name the same slots,
//! at every palette length.

#[path = "support/agentflow.rs"]
#[allow(dead_code)]
mod support;

use std::collections::HashMap;

use ferrigura_model::theme::ThemeTokens;
use ferrigura_model::value::Value;
use ferrigura_render::agentflow::colours::{
    KIND_COUNT, KINDS, assign, container_slot, container_slot_count, kind_class,
};
use ferrigura_render::agentflow::style::style_sheet_with_fonts;
use ferrigura_render::context::{Globals, ThemeView};
use ferrigura_render::unified::data::Node;
use support::{Setup, config_with, style_of};

/// A vertex with the classes it already carries.
fn node(id: &str, classes: &str) -> Node {
    Node {
        id: id.to_owned(),
        dom_id: id.to_owned(),
        css_classes: classes.to_owned(),
        shape: "roundedRect".to_owned(),
        ..Node::default()
    }
}

/// An expanded container.
fn group(id: &str) -> Node {
    Node {
        id: id.to_owned(),
        dom_id: id.to_owned(),
        is_group: true,
        shape: "flowGroup".to_owned(),
        ..Node::default()
    }
}

/// The kinds of the nodes by id, as the DB reports them.
fn kinds(pairs: &[(&str, &'static str)]) -> HashMap<String, &'static str> {
    pairs
        .iter()
        .map(|(id, kind)| ((*id).to_owned(), *kind))
        .collect()
}

/// `assignColorSlots` over the nodes, in the order the list gives the containers (the declaration order
/// of these fixtures), with a palette of `palette_len` colours.
fn assign_slots(nodes: &mut [Node], palette_len: usize, kinds: &HashMap<String, &'static str>) {
    let order: HashMap<String, usize> = nodes
        .iter()
        .filter(|node| node.is_group || node.shape == "collapsedGroup")
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();
    let order: HashMap<&str, usize> = order
        .iter()
        .map(|(id, index)| (id.as_str(), *index))
        .collect();
    assign(nodes, palette_len, &order, |id| kinds.get(id).copied());
}

/// The stylesheet of a palette of `border` colours and `fills` fills, in a theme of the given name.
fn sheet(border: usize, fills: usize, theme: &str, look: &str) -> String {
    let colours: Vec<String> = (0..border).map(|i| format!("#b0000{}", i % 10)).collect();
    let fill_colours: Vec<String> = (0..fills).map(|i| format!("#f0000{}", i % 10)).collect();
    let variables = serde_json::json!({
        "arrowheadColor": "#333",
        "border2": "#333",
        "clusterBkg": "#f4f4f4",
        "clusterBorder": "#ccc",
        "edgeLabelBackground": "#fff",
        "fontFamily": "trebuchet ms",
        "lineColor": "#333",
        "mainBkg": "#eee",
        "nodeBorder": "#999",
        "nodeTextColor": "#333",
        "tertiaryColor": "#ffffde",
        "textColor": "#333",
        "titleColor": "#333",
        "borderColorArray": colours,
        "bkgColorArray": fill_colours,
    });
    let tokens = ThemeTokens::from_value(&Value::from(variables));
    let globals = Globals {
        look: look.to_owned(),
        theme: theme.to_owned(),
        font_family: "trebuchet ms".to_owned(),
        font_size: 16.0,
        security_level: "strict".to_owned(),
        layout: "elk".to_owned(),
        html_labels: Some(false),
    };
    style_sheet_with_fonts(
        "svg",
        ThemeView {
            light: &tokens,
            dark: None,
        },
        &globals,
        &[],
        &[],
    )
}

#[test]
fn tags_a_node_with_the_kind_the_db_reports() {
    let mut nodes = vec![node("a", ""), node("b", ""), node("c", "")];
    let kinds = kinds(&[("a", "tool"), ("b", "decision"), ("c", "task")]);
    assign_slots(&mut nodes, 12, &kinds);
    assert_eq!(nodes[0].css_classes, "af-kind-tool");
    assert_eq!(nodes[1].css_classes, "af-kind-decision");
    assert_eq!(nodes[2].css_classes, "af-kind-task");
}

#[test]
fn gives_two_nodes_of_one_kind_the_same_class_wherever_they_sit() {
    // The whole point of colouring by kind: editing around a node does not recolour it.
    let mut nodes = vec![node("a", ""), node("b", ""), node("c", "")];
    let kinds = kinds(&[("a", "tool"), ("b", "decision"), ("c", "tool")]);
    assign_slots(&mut nodes, 12, &kinds);
    assert_eq!(nodes[0].css_classes, nodes[2].css_classes);
}

#[test]
fn keeps_any_classes_the_node_already_carried() {
    let mut nodes = vec![node("a", "mine")];
    assign_slots(&mut nodes, 12, &kinds(&[("a", "decision")]));
    assert_eq!(nodes[0].css_classes, "mine af-kind-decision");
}

#[test]
fn numbers_containers_from_the_order_it_is_given_above_the_kind_slots() {
    let mut nodes = vec![group("one"), node("a", ""), group("two"), group("three")];
    assign_slots(&mut nodes, 12, &kinds(&[("a", "decision")]));
    assert_eq!(nodes[0].color_index, Some(KIND_COUNT as u32));
    assert_eq!(nodes[2].color_index, Some(KIND_COUNT as u32 + 1));
    assert_eq!(nodes[3].color_index, Some(KIND_COUNT as u32 + 2));
    // A node takes no slot; its colour comes from its class.
    assert_eq!(nodes[1].color_index, None);
}

#[test]
fn never_folds_a_container_back_onto_a_kind_slot() {
    // The wrap happens in the assignment, so the container that runs off the end of the palette does not
    // land back on the `tool` colour.
    let palette_len = 12;
    let mut nodes: Vec<Node> = (0..palette_len * 2)
        .map(|index| group(&format!("g{index}")))
        .collect();
    assign_slots(&mut nodes, palette_len, &HashMap::new());
    for node in &nodes {
        let index = node.color_index.expect("a container has a slot") as usize;
        assert!(index >= KIND_COUNT, "{index}");
        assert!(index % palette_len >= KIND_COUNT, "{index}");
    }
}

#[test]
fn stamps_a_slot_the_stylesheet_actually_names_on_a_palette_shorter_than_the_kinds() {
    // A 3-colour palette turned an assigned slot 7 into `color-1` while the stylesheet named `color-7`.
    // Both sides go through `container_slot`, so the assigned value is the stamped one.
    let mut nodes = vec![group("one"), group("two")];
    assign_slots(&mut nodes, 3, &HashMap::new());
    for node in &nodes {
        let slot = node.color_index.expect("a container has a slot") as usize;
        assert_eq!(slot % 3, slot);
        assert_eq!(slot, container_slot(0, 3));
    }
    assert_eq!(container_slot_count(3), 1);
}

#[test]
fn agrees_with_the_stylesheet_at_every_palette_length() {
    // Whatever the palette length, the slot the assignment produces is one the stylesheet names.
    for len in [1, 3, 6, 7, 8, 12, 24] {
        for index in 0..container_slot_count(len) {
            let slot = container_slot(index, len);
            assert!(slot < len.max(1), "palette {len}, container {index}");
            assert_eq!(slot, slot % len.max(1));
        }
    }
}

#[test]
fn emits_a_rule_for_every_kind_at_that_kind_slot_colour() {
    let css = sheet(12, 12, "redux-color", "neo");
    for (slot, kind) in KINDS.iter().enumerate() {
        let selector = format!("[data-look=\"neo\"].node.{}", kind_class(kind));
        assert!(css.contains(&selector), "a rule for {kind}");
        assert!(
            css.contains(&format!("--borderColor{slot}:#b0000{slot};")),
            "the colour of {kind}"
        );
    }
}

#[test]
fn paints_containers_from_the_slots_above_the_kind_range() {
    let css = sheet(12, 12, "redux-color", "neo");
    let first = format!("[data-color-id=\"color-{KIND_COUNT}\"]");
    assert!(css.contains(&format!("{first}.cluster")));
    assert!(css.contains(&format!("stroke:var(--borderColor{KIND_COUNT});")));
    // No container rule uses a kind's colour.
    let containers = &css[css.find(&first).expect("the first container rule")..];
    for slot in 0..KIND_COUNT {
        let colour = format!("stroke:var(--borderColor{slot});");
        assert!(
            !containers.contains(&colour),
            "a container with the colour of kind slot {slot}"
        );
    }
}

#[test]
fn names_both_forms_of_a_container_so_a_collapsed_one_is_not_left_grey() {
    // A collapsed container is drawn as a `.node` and still holds its slot.
    let css = sheet(12, 12, "redux-color", "neo");
    let first = format!("[data-color-id=\"color-{KIND_COUNT}\"]");
    assert!(css.contains(&format!("{first}.cluster")));
    assert!(css.contains(&format!("{first}.node")));
}

#[test]
fn uses_no_container_slot_below_the_kind_range() {
    let css = sheet(12, 12, "redux-color", "neo");
    for slot in 0..KIND_COUNT {
        assert!(
            !css.contains(&format!("[data-color-id=\"color-{slot}\"]")),
            "slot {slot} belongs to a kind"
        );
    }
}

#[test]
fn strokes_without_filling_when_the_theme_carries_no_background_palette() {
    // `redux-dark-color` has 12 borders and no fills.
    let css = sheet(12, 0, "redux-color", "neo");
    assert!(css.contains(".af-kind-tool"));
    assert!(css.contains("stroke:var(--borderColor0);"));
    assert!(!css.contains("fill:var(--bkgColor"), "{css}");
}

#[test]
fn emits_nothing_for_a_theme_that_carries_no_palette() {
    let css = sheet(12, 12, "default", "neo");
    assert!(!css.contains("af-kind-"));
    assert!(!css.contains("data-color-id"));
}

#[test]
fn rejects_a_look_that_would_break_out_of_the_selector() {
    let css = sheet(12, 12, "redux-color", "neo\"] { fill: red } [x=\"");
    assert!(!css.contains("fill: red"));
    assert!(css.contains("[data-look=\"classic\"]"));
}

#[test]
fn the_slot_of_a_container_is_on_its_group_and_the_sheet_names_it() {
    // The render writes the slot as `data-color-id` on the group, and the sheet names that slot.
    let setup = Setup::new(config_with(serde_json::json!({})), None);
    let svg = setup.svg(
        "agentflow-beta TB\n  flow first[\"First\"]\n    a[\"a\"]\n  end\n",
        "svg",
    );
    assert!(svg.contains("data-color-id=\"color-7\""), "{svg}");
    assert!(style_of(&svg).contains("[data-color-id=\"color-7\"].cluster"));
}

// containerOrder.spec.ts: the slots follow the declaration order, through a parse.

fn slots_by_id(text: &str) -> HashMap<String, u32> {
    let setup = Setup::new(config_with(serde_json::json!({})), None);
    setup
        .data(text)
        .nodes
        .iter()
        .filter_map(|node| node.color_index.map(|slot| (node.id.clone(), slot)))
        .collect()
}

#[test]
fn gives_the_first_declared_container_the_first_container_slot() {
    let slots = slots_by_id(
        "agentflow-beta TB\n  flow first[\"First\"]\n    a[\"a\"]\n  end\n  flow second[\"Second\"]\n    b[\"b\"]\n  end\n  flow third[\"Third\"]\n    c[\"c\"]\n  end\n",
    );
    assert_eq!(slots["first"], KIND_COUNT as u32);
    assert_eq!(slots["second"], KIND_COUNT as u32 + 1);
    assert_eq!(slots["third"], KIND_COUNT as u32 + 2);
}

#[test]
fn numbers_a_parent_before_the_container_nested_inside_it() {
    // Completion order would put `inner` first; the colours follow the source.
    let slots = slots_by_id(
        "agentflow-beta TB\n  flow outer[\"Outer\"]\n    flow inner[\"Inner\"]\n      a[\"a\"]\n    end\n  end\n  flow sibling[\"Sibling\"]\n    b[\"b\"]\n  end\n",
    );
    assert_eq!(slots["outer"], KIND_COUNT as u32);
    assert_eq!(slots["inner"], KIND_COUNT as u32 + 1);
    assert_eq!(slots["sibling"], KIND_COUNT as u32 + 2);
}

#[test]
fn keeps_a_collapsed_container_in_the_sequence() {
    // A collapsed container is drawn as a node and still holds its slot.
    let slots = slots_by_id(
        "agentflow-beta TB\n  flow one[\"One\"]\n    a[\"a\"]\n  end\n  flow two[\"Two\"]\n    b[\"b\"]\n  end\n  two@{ view: \"collapsed\" }\n  flow three[\"Three\"]\n    c[\"c\"]\n  end\n",
    );
    assert_eq!(slots["one"], KIND_COUNT as u32);
    assert_eq!(slots["two"], KIND_COUNT as u32 + 1);
    assert_eq!(slots["three"], KIND_COUNT as u32 + 2);
}
