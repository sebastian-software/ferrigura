//! `getData` of the kanban diagram (`kanbanDb.ts` 54-97): what the renderer's data keeps and drops,
//! and which children each section lists. The reference's `getData` is compared field by field by
//! `pnpm port:diff kanban-data`; these tests name the rules that the differential exercises.

use ferrigura_model::value::Value;
use ferrigura_render::kanban::data::{DataNode, Shape, get_data, js_string};

/// The layout data of a text, with the look the default configuration gives.
fn data(text: &str) -> Vec<DataNode> {
    let diagram = ferrigura_syntax::kanban::parse(text).expect("the kanban parses");
    get_data(&diagram, "classic")
}

#[test]
fn a_section_keeps_its_ticket_and_drops_its_priority_assignee_icon_and_classes() {
    let nodes = data(
        "kanban\n  docs[Docs]@{ priority: 'Very High', assigned: 'knsv', ticket: MC-1, icon: 'fa fa-book' }\n    Item\n",
    );
    let section = &nodes[0];
    assert_eq!(section.shape, Shape::Section);
    assert!(section.is_group);
    assert_eq!(section.ticket.as_deref(), Some("MC-1"));
    assert_eq!(section.priority, None);
    assert_eq!(section.assigned, None);
    assert_eq!(section.icon, None);
    assert_eq!(section.look.as_deref(), Some("classic"));
    assert_eq!(section.corner_radius, None);
}

#[test]
fn an_item_keeps_its_metadata_and_its_corner_radius_and_css_style() {
    let nodes = data(
        "kanban\n  docs[Docs]\n    a[Item]@{ priority: 'Low', assigned: 'knsv', ticket: 7, icon: 'fa fa-x' }\n",
    );
    let item = &nodes[1];
    assert_eq!(item.shape, Shape::Item);
    assert!(!item.is_group);
    assert_eq!(item.id, "a");
    assert_eq!(item.label, "Item");
    assert_eq!(item.parent_id.as_deref(), Some("docs"));
    assert_eq!(item.priority, Some(Value::String("Low".to_owned())));
    assert_eq!(item.assigned.as_deref(), Some("knsv"));
    assert_eq!(item.ticket.as_deref(), Some("7"));
    assert_eq!(item.icon.as_deref(), Some("fa fa-x"));
    assert_eq!(item.corner_radius, Some(5.0));
    assert_eq!(item.css_styles, vec!["text-align: left".to_owned()]);
    assert_eq!(item.look, None);
}

#[test]
fn a_repeated_section_lists_its_children_under_each_copy() {
    // `docs` twice, with `x` and `y` below the first copy: getData lists both children under each
    // copy, so the flat list holds four items (NOTES 3.4, probed).
    let nodes = data("kanban\n  docs[Docs]\n    x\n    y\n  docs[Docs]\n");
    let shape: Vec<(&str, Shape)> = nodes
        .iter()
        .map(|node| (node.id.as_str(), node.shape))
        .collect();
    assert_eq!(
        shape,
        vec![
            ("docs", Shape::Section),
            ("x", Shape::Item),
            ("y", Shape::Item),
            ("docs", Shape::Section),
            ("x", Shape::Item),
            ("y", Shape::Item),
        ]
    );
}

#[test]
fn a_section_without_children_is_listed_alone() {
    let nodes = data("kanban\n  a\n  b\n");
    assert_eq!(nodes.len(), 2);
    assert!(nodes.iter().all(|node| node.is_group));
}

#[test]
fn a_label_is_sanitised_a_second_time_and_a_number_label_is_its_string() {
    let nodes = data("kanban\n  a[x]\n  b@{ label: 5 }\n  c@{ label: [1, 2] }\n");
    assert_eq!(nodes[0].label, "x");
    assert_eq!(nodes[1].label, "5");
    assert_eq!(nodes[2].label, "1,2");
}

#[test]
fn a_label_is_written_as_javascript_writes_the_value() {
    assert_eq!(js_string(&Value::Bool(true)), "true");
    assert_eq!(js_string(&Value::Number(f64::INFINITY)), "Infinity");
    assert_eq!(js_string(&Value::Number(f64::NEG_INFINITY)), "-Infinity");
    assert_eq!(js_string(&Value::Number(f64::NAN)), "NaN");
    assert_eq!(
        js_string(&Value::Array(vec![
            Value::Number(1.0),
            Value::Null,
            Value::Number(2.5)
        ])),
        "1,,2.5"
    );
    assert_eq!(
        js_string(&Value::Object(Default::default())),
        "[object Object]"
    );
    assert_eq!(js_string(&Value::Null), "");
}
