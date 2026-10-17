//! Port of `packages/mermaid/src/diagrams/c4/parser/c4NamedAttributes.spec.ts`: 42 tests, the
//! eight literal cases and the 34 rows of its `it.each` blocks, one `#[test]` per row. The
//! plan is `docs-port/c4/PLAN.md`.
//!
//! A text field (`descr`, `techn`, `type`) and a plain field (`tags`, `sprite`, `link`) are both
//! checked as the string they hold. The model types the text fields as `Option<String>`, so the
//! reference's `{ text }` wrapper has no counterpart that a check could miss.

use ferrigura_syntax::c4::parse;

// c4NamedAttributes.spec.ts:19
#[test]
fn keeps_tags_a_string_when_the_description_is_omitted() {
    let diagram = parse(
        r#"C4Context
System(s, "S", $tags="cylinder")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].tags.as_deref(), Some("cylinder"));
}

// c4NamedAttributes.spec.ts:25
#[test]
fn keeps_tags_a_string_when_the_description_is_given() {
    let diagram = parse(
        r#"C4Context
System(s, "S", "desc", $tags="cylinder")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.tags.as_deref(), Some("cylinder"));
    assert_eq!(shape.descr.as_deref(), Some("desc"));
}

// c4NamedAttributes.spec.ts:33
#[test]
fn still_stores_the_description_as_text_when_it_is_the_one_named() {
    let diagram = parse(
        r#"C4Context
System(s, "S", $descr="a description")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some("a description"));
}

// c4NamedAttributes.spec.ts:39
#[test]
fn keeps_sprite_and_link_strings_on_a_container() {
    let diagram = parse(
        r#"C4Container
Container(c, "C", $sprite="browser", $link="https://x.test")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.sprite.as_deref(), Some("browser"));
    assert_eq!(shape.link.as_deref(), Some("https://x.test"));
}

// c4NamedAttributes.spec.ts:47
#[test]
fn keeps_a_relationship_tags_a_string_when_techn_and_descr_are_omitted() {
    let diagram = parse(
        r#"C4Context
Person(a, "A")
Person(b, "B")
Rel(a, b, "uses", $tags="async")"#,
    )
    .expect("parses");
    assert_eq!(diagram.rels[0].tags.as_deref(), Some("async"));
}

// c4NamedAttributes.spec.ts:60
#[test]
fn keeps_a_named_descr_that_lands_in_an_earlier_slot() {
    let diagram = parse(
        r#"C4Container
Container(c, "C", $descr="a description")"#,
    )
    .expect("parses");
    assert_eq!(diagram.shapes[0].descr.as_deref(), Some("a description"));
}

// c4NamedAttributes.spec.ts:66
#[test]
fn still_defaults_descr_to_empty_when_it_is_genuinely_absent() {
    let diagram = parse(
        r#"C4Container
Container(c, "C", $techn="Java")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.techn.as_deref(), Some("Java"));
    assert_eq!(shape.descr.as_deref(), Some(""));
}

// c4NamedAttributes.spec.ts:74
#[test]
fn keeps_a_named_descr_on_a_relationship_where_techn_also_comes_first() {
    let diagram = parse(
        r#"C4Context
Person(a, "A")
Person(b, "B")
Rel(a, b, "uses", $descr="how it is used")"#,
    )
    .expect("parses");
    assert_eq!(diagram.rels[0].descr.as_deref(), Some("how it is used"));
}

// c4NamedAttributes.spec.ts:113 (row system)
#[test]
fn one_named_attribute_system_descr() {
    let diagram = parse(
        r#"C4Context
System(x, "L", $descr="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.descr.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row system)
#[test]
fn one_named_attribute_system_tags() {
    let diagram = parse(
        r#"C4Context
System(x, "L", $tags="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.tags.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row system)
#[test]
fn one_named_attribute_system_sprite() {
    let diagram = parse(
        r#"C4Context
System(x, "L", $sprite="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.sprite.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row system)
#[test]
fn one_named_attribute_system_link() {
    let diagram = parse(
        r#"C4Context
System(x, "L", $link="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.link.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row person)
#[test]
fn one_named_attribute_person_descr() {
    let diagram = parse(
        r#"C4Context
Person(x, "L", $descr="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.descr.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row person)
#[test]
fn one_named_attribute_person_tags() {
    let diagram = parse(
        r#"C4Context
Person(x, "L", $tags="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.tags.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row person)
#[test]
fn one_named_attribute_person_sprite() {
    let diagram = parse(
        r#"C4Context
Person(x, "L", $sprite="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.sprite.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row person)
#[test]
fn one_named_attribute_person_link() {
    let diagram = parse(
        r#"C4Context
Person(x, "L", $link="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.link.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row container)
#[test]
fn one_named_attribute_container_descr() {
    let diagram = parse(
        r#"C4Container
Container(x, "L", $descr="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.descr.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row container)
#[test]
fn one_named_attribute_container_techn() {
    let diagram = parse(
        r#"C4Container
Container(x, "L", $techn="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.techn.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row container)
#[test]
fn one_named_attribute_container_tags() {
    let diagram = parse(
        r#"C4Container
Container(x, "L", $tags="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.tags.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row container)
#[test]
fn one_named_attribute_container_sprite() {
    let diagram = parse(
        r#"C4Container
Container(x, "L", $sprite="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.sprite.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row container)
#[test]
fn one_named_attribute_container_link() {
    let diagram = parse(
        r#"C4Container
Container(x, "L", $link="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.link.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row component)
#[test]
fn one_named_attribute_component_descr() {
    let diagram = parse(
        r#"C4Component
Component(x, "L", $descr="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.descr.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row component)
#[test]
fn one_named_attribute_component_techn() {
    let diagram = parse(
        r#"C4Component
Component(x, "L", $techn="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.techn.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row component)
#[test]
fn one_named_attribute_component_tags() {
    let diagram = parse(
        r#"C4Component
Component(x, "L", $tags="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.tags.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row component)
#[test]
fn one_named_attribute_component_sprite() {
    let diagram = parse(
        r#"C4Component
Component(x, "L", $sprite="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.sprite.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:113 (row component)
#[test]
fn one_named_attribute_component_link() {
    let diagram = parse(
        r#"C4Component
Component(x, "L", $link="V")"#,
    )
    .expect("parses");
    let shape = &diagram.shapes[0];
    assert_eq!(shape.link.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row system_boundary)
#[test]
fn one_named_attribute_system_boundary_type() {
    let diagram = parse(
        r#"C4Context
System_Boundary(x, "L", $type="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.boundary_type.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row system_boundary)
#[test]
fn one_named_attribute_system_boundary_tags() {
    let diagram = parse(
        r#"C4Context
System_Boundary(x, "L", $tags="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.tags.as_ref().and_then(|t| t.as_deref()), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row system_boundary)
#[test]
fn one_named_attribute_system_boundary_link() {
    let diagram = parse(
        r#"C4Context
System_Boundary(x, "L", $link="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.link.as_ref().and_then(|l| l.as_deref()), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row container_boundary)
#[test]
fn one_named_attribute_container_boundary_type() {
    let diagram = parse(
        r#"C4Container
Container_Boundary(x, "L", $type="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.boundary_type.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row container_boundary)
#[test]
fn one_named_attribute_container_boundary_tags() {
    let diagram = parse(
        r#"C4Container
Container_Boundary(x, "L", $tags="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.tags.as_ref().and_then(|t| t.as_deref()), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row container_boundary)
#[test]
fn one_named_attribute_container_boundary_link() {
    let diagram = parse(
        r#"C4Container
Container_Boundary(x, "L", $link="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.link.as_ref().and_then(|l| l.as_deref()), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row node)
#[test]
fn one_named_attribute_node_type() {
    let diagram = parse(
        r#"C4Deployment
Node(x, "L", $type="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.boundary_type.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row node)
#[test]
fn one_named_attribute_node_descr() {
    let diagram = parse(
        r#"C4Deployment
Node(x, "L", $descr="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.descr.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row node)
#[test]
fn one_named_attribute_node_tags() {
    let diagram = parse(
        r#"C4Deployment
Node(x, "L", $tags="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.tags.as_ref().and_then(|t| t.as_deref()), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row node)
#[test]
fn one_named_attribute_node_sprite() {
    let diagram = parse(
        r#"C4Deployment
Node(x, "L", $sprite="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.sprite.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:127 (row node)
#[test]
fn one_named_attribute_node_link() {
    let diagram = parse(
        r#"C4Deployment
Node(x, "L", $link="V") {
Container(i, "I")
}"#,
    )
    .expect("parses");
    let boundary = diagram
        .boundaries
        .iter()
        .find(|b| b.alias != "global")
        .expect("a declared boundary");
    assert_eq!(boundary.link.as_ref().and_then(|l| l.as_deref()), Some("V"));
}

// c4NamedAttributes.spec.ts:135 (row Rel)
#[test]
fn one_named_attribute_rel_techn() {
    let diagram = parse(
        r#"C4Context
Person(a, "A")
Person(b, "B")
Rel(a, b, "uses", $techn="V")"#,
    )
    .expect("parses");
    assert_eq!(diagram.rels[0].techn.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:135 (row Rel)
#[test]
fn one_named_attribute_rel_descr() {
    let diagram = parse(
        r#"C4Context
Person(a, "A")
Person(b, "B")
Rel(a, b, "uses", $descr="V")"#,
    )
    .expect("parses");
    assert_eq!(diagram.rels[0].descr.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:135 (row Rel)
#[test]
fn one_named_attribute_rel_tags() {
    let diagram = parse(
        r#"C4Context
Person(a, "A")
Person(b, "B")
Rel(a, b, "uses", $tags="V")"#,
    )
    .expect("parses");
    assert_eq!(diagram.rels[0].tags.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:135 (row Rel)
#[test]
fn one_named_attribute_rel_sprite() {
    let diagram = parse(
        r#"C4Context
Person(a, "A")
Person(b, "B")
Rel(a, b, "uses", $sprite="V")"#,
    )
    .expect("parses");
    assert_eq!(diagram.rels[0].sprite.as_deref(), Some("V"));
}

// c4NamedAttributes.spec.ts:135 (row Rel)
#[test]
fn one_named_attribute_rel_link() {
    let diagram = parse(
        r#"C4Context
Person(a, "A")
Person(b, "B")
Rel(a, b, "uses", $link="V")"#,
    )
    .expect("parses");
    assert_eq!(diagram.rels[0].link.as_deref(), Some("V"));
}
