//! The diagram types as plugins of a renderer: what `Renderer::new` registers, what a builder
//! registers, and what a renderer says about a type it does not draw.

use std::collections::HashSet;

use ferrigura::{DiagramType, Error, Renderer, Value, diagrams};

const PIE: &str = "pie\n  \"Dogs\" : 386\n  \"Cats\" : 85\n";
const FLOWCHART: &str = "flowchart LR\n  a --> b\n";

#[test]
fn all_registers_every_diagram_type_once() {
    let kinds: Vec<DiagramType> = diagrams::all().iter().map(diagrams::Plugin::kind).collect();
    let unique: HashSet<DiagramType> = kinds.iter().copied().collect();
    assert_eq!(kinds.len(), 33);
    assert_eq!(unique.len(), kinds.len());
}

#[test]
fn a_builder_draws_the_types_it_registers_and_refuses_the_others() {
    let renderer = Renderer::builder(&Value::empty_object())
        .with(diagrams::pie())
        .build();
    assert!(
        renderer
            .render(PIE)
            .is_ok_and(|svg| svg.starts_with("<svg "))
    );
    match renderer.render(FLOWCHART) {
        Err(Error::Unsupported(message)) => {
            assert!(message.contains("flowchart"), "{message}");
            assert!(message.contains("not registered"), "{message}");
        }
        other => panic!("expected an unsupported-type error, got {other:?}"),
    }
}

#[test]
fn a_plugin_registered_twice_counts_once() {
    let renderer = Renderer::builder(&Value::empty_object())
        .with(diagrams::pie())
        .with(diagrams::pie())
        .build();
    assert!(renderer.render(PIE).is_ok());
}

#[test]
fn the_builder_with_every_plugin_draws_what_new_draws() {
    let config = Value::empty_object();
    let all = Renderer::builder(&config).with_all().build();
    let default = Renderer::new(&config);
    for text in [PIE, FLOWCHART] {
        assert_eq!(all.render(text).unwrap(), default.render(text).unwrap());
    }
}

#[test]
fn a_text_of_no_diagram_type_is_a_parse_error_whatever_is_registered() {
    let renderer = Renderer::builder(&Value::empty_object())
        .with(diagrams::pie())
        .build();
    assert!(matches!(
        renderer.render("not a diagram"),
        Err(Error::Parse(_))
    ));
}
