//! The cynefin DB, as `packages/mermaid/src/diagrams/cynefin/cynefin.spec.ts` tests it. The Rust DB
//! is not public, so each DB call of the reference's spec is reached through `parse`, which runs the
//! same calls in the same order (`populate`). The expected values are the reference's, and the
//! differential run compares every input of these tests as well. The configuration is read through
//! the defaults of the schema, which `pnpm port:diff config` compares.

use ferrigura_model::config::defaults;
use ferrigura_model::cynefin::{CynefinDiagram, CynefinDomain};
use ferrigura_syntax::cynefin::parse;

fn parsed(text: &str) -> CynefinDiagram {
    parse(text).expect("the diagram parses")
}

fn domain<'a>(diagram: &'a CynefinDiagram, name: &str) -> &'a CynefinDomain {
    diagram
        .domains
        .iter()
        .find(|d| d.name == name)
        .expect("the domain is set")
}

#[test]
fn should_start_empty() {
    let diagram = parsed("cynefin-beta\n");
    assert!(diagram.domains.is_empty());
    assert!(diagram.transitions.is_empty());
}

#[test]
fn should_set_domains_from_blocks() {
    let diagram = parsed("cynefin-beta\ncomplex \"Test\"\nclear\n");
    assert_eq!(diagram.domains.len(), 2);
    assert_eq!(domain(&diagram, "complex").items, ["Test"]);
    assert!(domain(&diagram, "clear").items.is_empty());
}

#[test]
fn should_set_transitions() {
    let diagram = parsed("cynefin-beta\ncomplex --> complicated : \"Pattern found\"\n");
    assert_eq!(diagram.transitions.len(), 1);
    let transition = &diagram.transitions[0];
    assert_eq!(transition.from, "complex");
    assert_eq!(transition.to, "complicated");
    assert_eq!(transition.label.as_deref(), Some("Pattern found"));
}

#[test]
fn should_set_transitions_without_labels() {
    let diagram = parsed("cynefin-beta\nchaotic --> complex\n");
    assert_eq!(diagram.transitions[0].label, None);
}

#[test]
fn should_filter_out_self_loop_transitions() {
    let diagram = parsed(
        "cynefin-beta\ncomplex --> complex\ncomplex --> complicated : \"Pattern found\"\nclear --> clear\n",
    );
    assert_eq!(diagram.transitions.len(), 1);
    assert_eq!(diagram.transitions[0].from, "complex");
    assert_eq!(diagram.transitions[0].to, "complicated");
}

#[test]
fn should_handle_a_list_of_only_self_loops_gracefully() {
    let diagram = parsed("cynefin-beta\ncomplex --> complex\nchaotic --> chaotic\n");
    assert!(diagram.transitions.is_empty());
}

#[test]
fn should_handle_no_blocks_and_no_transitions() {
    let diagram = parsed("cynefin-beta\n");
    assert!(diagram.domains.is_empty());
    assert!(diagram.transitions.is_empty());
}

#[test]
fn should_start_fresh_after_a_clear() {
    // The reference clears the DB before each parse: a second diagram sees none of the first.
    let first = parsed("cynefin-beta\ncomplex \"A\"\nclear --> chaotic\n");
    assert_eq!(first.domains.len(), 1);
    let second = parsed("cynefin-beta\n");
    assert!(second.domains.is_empty());
    assert!(second.transitions.is_empty());
}

#[test]
fn should_handle_multiple_domains() {
    let diagram = parsed("cynefin-beta\ncomplex\ncomplicated\nclear\nchaotic\nconfusion\n");
    assert_eq!(diagram.domains.len(), 5);
}

#[test]
fn should_handle_domain_with_multiple_items() {
    let diagram = parsed("cynefin-beta\ncomplex \"A\" \"B\" \"C\" \"D\"\n");
    assert_eq!(domain(&diagram, "complex").items.len(), 4);
}

#[test]
fn should_return_config() {
    // The cynefin section of the defaults: the schema's values of the keys the renderer reads.
    let section = &defaults()["cynefin"];
    assert_eq!(section["width"].as_f64(), Some(800.0));
    assert_eq!(section["height"].as_f64(), Some(600.0));
    assert_eq!(section["padding"].as_f64(), Some(40.0));
    assert_eq!(section["boundaryAmplitude"].as_f64(), Some(8.0));
    assert_eq!(section["seed"].as_f64(), Some(0.0));
    assert!(section.is_object());
}
