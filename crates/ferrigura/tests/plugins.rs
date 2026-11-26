//! The diagram types as plugins of a renderer: what `Renderer::new` registers, what a builder
//! registers, and what a renderer says about a type it does not register or ferrigura does not draw.

use std::collections::HashSet;
use std::sync::Arc;

use ferrigura::{DiagramType, Error, Renderer, Value, diagrams};

const PIE: &str = "pie\n  \"Dogs\" : 386\n  \"Cats\" : 85\n";
const FLOWCHART: &str = "flowchart LR\n  a --> b\n";

#[test]
fn all_registers_every_diagram_type_once() {
    let kinds: Vec<DiagramType> = diagrams::all().iter().map(diagrams::Plugin::kind).collect();
    let unique: HashSet<DiagramType> = kinds.iter().copied().collect();
    assert_eq!(kinds.len(), 10);
    assert_eq!(unique.len(), kinds.len());
    assert_eq!(unique, DiagramType::ALL.into_iter().collect());
}

/// The list of the types ferrigura draws, as the error of a type it does not draw gives it.
const DRAWN: &str = "it draws flowchart, state, class, ER, mindmap, sequence, gantt, XY chart, pie \
                     and quadrant chart diagrams";

/// One text per header of a diagram type that ferrigura does not draw, with the name the error
/// gives the type.
const NOT_DRAWN: [(&str, &str); 42] = [
    ("info", "info"),
    ("info showInfo", "info"),
    ("timeline\n  title T\n  2020 : a", "timeline"),
    (
        "journey\n  title T\n  section S\n    Task: 5: Me",
        "user journey",
    ),
    ("gitGraph\n  commit", "Git graph"),
    ("gitGraph TB:\n  commit", "Git graph"),
    ("block\n  A", "block"),
    ("block-beta\n  A", "block"),
    ("packet\n  0-15: \"Header\"", "packet"),
    ("packet-beta\n  0-15: \"Header\"", "packet"),
    ("sankey\n  a,b,8", "sankey"),
    ("sankey-beta\n  a,b,8", "sankey"),
    (
        "requirementDiagram\n  requirement r {\n  id: 1\n  }",
        "requirement",
    ),
    ("C4Context\n  title T", "C4"),
    ("C4Container\n  title T", "C4"),
    ("C4Component\n  title T", "C4"),
    ("C4Dynamic\n  title T", "C4"),
    ("C4Deployment\n  title T", "C4"),
    ("kanban\n  Todo", "kanban"),
    ("architecture-beta\n  service a(server)[A]", "architecture"),
    ("architecture\n  service a(server)[A]", "architecture"),
    ("treemap\n  \"Root\"", "treemap"),
    ("treemap-beta\n  \"Root\"", "treemap"),
    ("ishikawa\n  Cause", "ishikawa"),
    ("Ishikawa-Beta\n  Cause", "ishikawa"),
    ("eventmodeling\n  tf 01 cmd Do", "event modeling"),
    ("agentflow-beta\n  a --> b", "agentflow"),
    ("cynefin-beta\n  title T", "cynefin"),
    ("radar-beta\n  axis a, b, c", "radar"),
    ("railroad-beta\n  r = \"a\" ;", "railroad"),
    ("railroad-ebnf-beta\n  r = \"a\" ;", "railroad"),
    ("railroad-abnf-beta\n  r = \"a\"", "railroad"),
    ("RAILROAD-PEG-BETA\n  r <- \"a\"", "railroad"),
    ("swimlane-beta\n  lane a", "swimlane"),
    ("treeView-beta\n  root", "tree view"),
    ("usecase-beta\n  actor A", "use case"),
    ("venn-beta\n  set A", "venn"),
    ("wardley-beta\n  title T", "wardley"),
    ("Wardley-Beta\n  title T", "wardley"),
    ("---\ntitle: T\n---\nkanban\n  Todo", "kanban"),
    (
        "%%{init: {\"theme\": \"dark\"}}%%\nsankey-beta\n  a,b,8",
        "sankey",
    ),
    ("%% a comment\ntimeline\n  2020 : a", "timeline"),
];

#[test]
fn a_text_of_a_type_ferrigura_does_not_draw_names_the_type_and_the_drawn_ones() {
    let renderer = Renderer::new(&Value::empty_object());
    for (text, name) in NOT_DRAWN {
        match renderer.render(text) {
            Err(Error::Unsupported(message)) => assert_eq!(
                message,
                format!("ferrigura does not draw {name} diagrams; {DRAWN}"),
                "{text:?}"
            ),
            other => panic!("{text:?}: expected an unsupported-type error, got {other:?}"),
        }
    }
}

#[test]
fn a_type_ferrigura_does_not_draw_gives_the_same_error_in_every_mode_and_renderer() {
    let renderer = Renderer::builder(&Value::empty_object())
        .with(diagrams::pie())
        .build();
    let expected = format!("ferrigura does not draw sankey diagrams; {DRAWN}");
    for options in [
        ferrigura::RenderOptions::default(),
        ferrigura::RenderOptions {
            compat: true,
            ..ferrigura::RenderOptions::default()
        },
        ferrigura::RenderOptions {
            design: true,
            ..ferrigura::RenderOptions::default()
        },
    ] {
        match renderer.render_with("sankey-beta\n  a,b,8", &options) {
            Err(Error::Unsupported(message)) => assert_eq!(message, expected),
            other => panic!("expected an unsupported-type error, got {other:?}"),
        }
    }
    match ferrigura::render("gitGraph\n  commit", &Value::empty_object()) {
        Err(error) => assert_eq!(
            error.to_string(),
            format!("ferrigura does not draw Git graph diagrams; {DRAWN}")
        ),
        Ok(svg) => panic!("expected an error, got {svg}"),
    }
}

#[test]
fn a_header_that_only_starts_like_a_type_ferrigura_does_not_draw_is_a_parse_error() {
    let renderer = Renderer::new(&Value::empty_object());
    for text in [
        "sankeyfoo\n  a,b,8",
        "packets\n  0-15: \"Header\"",
        "blocks\n  A",
        "usecase-betas\n  actor A",
        "agentflow-betaX\n  a --> b",
    ] {
        assert!(
            matches!(renderer.render(text), Err(Error::Parse(_))),
            "{text:?}"
        );
    }
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

#[test]
fn shared_keeps_one_renderer_per_configuration_and_plugin_set() {
    let config = Value::empty_object();
    let pie = || Renderer::builder(&config).with(diagrams::pie());
    let first = pie().shared();
    assert!(Arc::ptr_eq(&first, &pie().shared()));

    let with_flowchart = pie().with(diagrams::flowchart()).shared();
    assert!(!Arc::ptr_eq(&first, &with_flowchart));
    assert!(first.render(FLOWCHART).is_err());
    assert!(with_flowchart.render(FLOWCHART).is_ok());

    let reversed = Renderer::builder(&config)
        .with(diagrams::flowchart())
        .with(diagrams::pie())
        .shared();
    assert!(Arc::ptr_eq(&with_flowchart, &reversed));

    // The shorthand `render` keeps a renderer with every plugin of its own, not the reduced one.
    let all = Renderer::builder(&config).with_all().shared();
    assert!(!Arc::ptr_eq(&all, &first));
    assert!(ferrigura::render(FLOWCHART, &config).is_ok());
}
