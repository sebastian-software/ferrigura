//! Port of `packages/mermaid/src/diagrams/state/stateNoteNode.spec.js` and `stateNoteEdge.spec.ts`:
//! the note nodes and the note edges of `getData()`. The texts and titles are the specs'.

use ferrigura_model::config::MermaidConfig;
use ferrigura_render::context::Globals;
use ferrigura_render::state::data::get_data;
use ferrigura_render::unified::Section;
use ferrigura_syntax::state::parse_grammar;

/// The layout data of a text, read after the grammar and the DB have parsed it.
fn layout(text: &str) -> ferrigura_render::unified::data::LayoutData {
    let diagram = parse_grammar(text).expect("parses");
    get_data(
        &diagram,
        &Section::of(&MermaidConfig::default()),
        &Globals::of(&MermaidConfig::default()),
    )
}

mod note_nodes {
    use super::*;

    /// A note is a leaf placed inside a note group, never a container: it does not take the
    /// composite's `type` and `isGroup`.
    const SRC: &str = "
  stateDiagram-v2
  state Composite {
    A --> B
  }
  note right of Composite : hello
  ";

    #[test]
    fn does_not_mark_a_note_on_a_composite_state_as_a_group() {
        let data = layout(SRC);
        let note = data
            .nodes
            .iter()
            .find(|node| node.shape == "note")
            .expect("a note node should be produced");
        assert!(!note.is_group);
    }

    #[test]
    fn still_produces_the_note_group_that_contains_the_note() {
        let data = layout(SRC);
        let note_group = data
            .nodes
            .iter()
            .find(|node| node.shape == "noteGroup")
            .expect("a noteGroup should still wrap the note");
        assert!(note_group.is_group);
    }
}

mod note_edges {
    use super::*;

    /// The edge joining a state to its note is dashed on the edge itself, not only in CSS: under
    /// the neo look `insertEdge` picks its dash generator from `pattern`.
    const DIAGRAM: &str = "stateDiagram-v2
    [*] --> Active
    Active --> Idle
    note right of Active
      A note
    end note
  ";

    #[test]
    fn declares_the_note_edge_as_dashed_on_the_edge_not_only_in_css() {
        let data = layout(DIAGRAM);
        let note_edge = data
            .edges
            .iter()
            .find(|edge| edge.classes.contains("note-edge"))
            .expect("a note edge");
        assert_eq!(note_edge.pattern, "dashed");
    }

    #[test]
    fn leaves_ordinary_transitions_solid() {
        // `pattern` is absent on a transition, which sends it down the solid branch.
        let data = layout(DIAGRAM);
        let transitions: Vec<_> = data
            .edges
            .iter()
            .filter(|edge| !edge.classes.contains("note-edge"))
            .collect();
        assert!(!transitions.is_empty());
        assert!(transitions.iter().all(|edge| edge.pattern.is_empty()));
    }

    #[test]
    fn keeps_the_note_edge_class_which_is_what_the_classic_look_still_styles() {
        let data = layout(DIAGRAM);
        let note_edge = data
            .edges
            .iter()
            .find(|edge| edge.classes.contains("note-edge"))
            .expect("a note edge");
        assert!(note_edge.classes.contains("note-edge"));
    }
}
