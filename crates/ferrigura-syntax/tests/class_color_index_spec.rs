//! Port of packages/mermaid/src/diagrams/class/classDiagram-colorIndex.spec.ts (62 lines). Test
//! names follow the spec's titles in snake_case.
//!
//! The spec pins the colour slot of each class through `getData()`, which is rendering: `classBox`
//! stamps the slot on the node and the theme maps it to a colour. The model has no colour slot, so
//! all three tests are ignored. Their bodies keep what the model can state: the slot follows the
//! order of `diagram.classes`, and a namespace or a note is not a class.

use ferrigura_model::class::ClassDiagram;
use ferrigura_syntax::class::parse_grammar;

fn parse(text: &str) -> ClassDiagram {
    parse_grammar(text).unwrap_or_else(|error| panic!("{text:?} does not parse: {error:?}"))
}

/// The ids of the classes, in the order the diagram keeps them.
fn class_ids(diagram: &ClassDiagram) -> Vec<&str> {
    diagram
        .classes
        .iter()
        .map(|class| class.id.as_str())
        .collect()
}

#[test]
#[ignore = "port: getData() colour slots are rendering, not in the model"]
fn should_give_each_class_its_own_slot_in_declaration_order() {
    let diagram = parse("classDiagram\nclass Order\nclass Customer\nclass Payment");
    assert_eq!(class_ids(&diagram), ["Order", "Customer", "Payment"]);
}

#[test]
#[ignore = "port: getData() colour slots are rendering, not in the model"]
fn should_not_spend_a_slot_on_a_namespace_container() {
    // The spec calls `addNamespace` first, so the namespace exists and the check below does not
    // pass on absence.
    let diagram = parse("classDiagram\nnamespace shop {\n class Order\n}\nclass Customer");
    let shop = diagram.namespace("shop").expect("the namespace exists");
    assert_eq!(shop.classes, ["Order"]);
    // The namespace is a container, not a participant: it is no class and takes no slot.
    assert!(diagram.class("shop").is_none());
    assert_eq!(class_ids(&diagram), ["Order", "Customer"]);
}

#[test]
#[ignore = "port: getData() colour slots are rendering, not in the model"]
fn should_not_spend_a_slot_on_a_note() {
    let diagram = parse("classDiagram\nclass Order\nnote for Order \"a note\"\nclass Customer");
    assert_eq!(diagram.notes.len(), 1, "the note is kept as a note");
    // Notes carry the theme's fixed note colour, so they stay outside the cycle: no class is a note.
    assert_eq!(class_ids(&diagram), ["Order", "Customer"]);
}
