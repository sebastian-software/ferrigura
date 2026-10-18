//! Port of `packages/mermaid/src/diagrams/usecase/usecaseDb.spec.ts`.
//!
//! The reference's DB keeps the state of its last commit, and `commit` and `clear` have no counterpart
//! here: the parse is a pure function that returns a model. Each expectation is therefore ported through
//! `parse`. A commit is a parse that succeeds (`publish`), a failed parse publishes nothing and keeps the
//! previous model, and a parse of a text starts from an empty model, which is what `clear` gave.
//!
//! Not ported, as `docs-port/usecase/PLAN.md` leaves them out of the model: the counters and the symbols
//! of the DB (the counters are seen through the ids of a fresh parse), and the configuration. The
//! oval-padding expectation needs the render input (`getData`), so it is ignored.

use ferrigura_model::use_case::{Direction, UseCaseDiagram, UseCaseShape};
use ferrigura_syntax::use_case::parse;

/// The text of the first commit: an actor, two notes, a JSON table and a relation with its class,
/// its style and its animation.
const FIRST: &str = "usecase-beta\nactor User\nnote for User \"First note\"\nnote for User \"`Second note`\"\njson Payload@{\n  \"second\": 2,\n  \"first\": 1\n}:::payload\nUser edge@--> User\nclass edge important\nstyle edge stroke:#f00\nedge@{ animate: true, animation: fast }";

/// The text of the replacement: one use case and nothing else.
const REPLACEMENT: &str = "usecase-beta\nLogin(Sign in)";

/// Publishes the model of `text` as the committed state. The state is replaced only when the parse
/// succeeds, so a failed parse keeps the previous model. Returns whether the text was published.
fn publish(state: &mut Option<UseCaseDiagram>, text: &str) -> bool {
    match parse(text) {
        Ok(model) => {
            *state = Some(model);
            true
        }
        Err(_) => false,
    }
}

#[test]
fn atomically_replaces_committed_state_with_a_detached_model() {
    let mut state = None;
    assert!(publish(&mut state, FIRST));

    // A draft of the same text, changed after the commit, must not reach the committed state.
    let mut draft = parse(FIRST).expect("the first text parses");
    draft.actors[0].label = "mutated draft".to_owned();
    draft.notes.clear();
    draft.relationships.clear();

    let committed = state.as_ref().expect("the first commit is published");
    assert_eq!(committed.actors[0].id, "User");
    assert_eq!(committed.actors[0].label, "User");
    let note_ids: Vec<&str> = committed
        .notes
        .iter()
        .map(|note| note.id.as_str())
        .collect();
    assert_eq!(note_ids, ["note-0", "note-1"]);
    let json_ids: Vec<&str> = committed
        .json_nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect();
    assert_eq!(json_ids, ["Payload"]);
    assert_eq!(committed.relationships.len(), 1);
    // The AST is the AST of the text it was parsed from, with the line break `Diagram.fromText` adds.
    assert_eq!(committed.ast.source, format!("{FIRST}\n"));

    assert!(publish(&mut state, REPLACEMENT));
    let replaced = state.expect("the replacement is published");
    assert!(replaced.actors.is_empty());
    let use_case_ids: Vec<&str> = replaced
        .use_cases
        .iter()
        .map(|case| case.id.as_str())
        .collect();
    assert_eq!(use_case_ids, ["Login"]);
    assert_eq!(replaced.use_cases[0].label, "Sign in");
    assert!(replaced.notes.is_empty());
    assert!(replaced.json_nodes.is_empty());
    assert!(replaced.relationships.is_empty());
    assert_eq!(replaced.ast.source, format!("{REPLACEMENT}\n"));
}

#[test]
fn keeps_the_previous_commit_when_a_replacement_is_incomplete() {
    let mut state = None;
    assert!(publish(&mut state, "usecase-beta\nactor User"));

    // The replacement declares an actor, then fails on a line it cannot read, so it publishes nothing.
    assert!(!publish(
        &mut state,
        "usecase-beta\nactor Replacement\nunknown command"
    ));

    let committed = state.expect("the previous commit is kept");
    let actor_ids: Vec<&str> = committed
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .collect();
    assert_eq!(actor_ids, ["User"]);
}

#[test]
fn clears_every_collection_counter_ast_direction_config_title_and_accessibility_field() {
    // The state that the reset has to clear: every collection, the direction, the accessibility texts
    // and the front matter title. The title never reaches the model, as the parser clears the DB after
    // `Diagram.fromText` sets it.
    let rich = parse(
        "---\ntitle: Title\n---\nusecase-beta\ndirection RL\naccTitle: Accessible title\naccDescr: Accessible description\nclassDef important fill:red\nactor User\nnote for User \"note\"\njson Payload@{}\nUser edge@--> User",
    )
    .expect("the rich text parses");
    assert_eq!(rich.direction, Direction::Rl);
    assert_eq!(rich.actors.len(), 1);
    assert_eq!(rich.notes.len(), 1);
    assert_eq!(rich.json_nodes.len(), 1);
    assert_eq!(rich.relationships.len(), 1);
    assert_eq!(rich.class_defs.len(), 1);
    assert_eq!(rich.acc_title, "Accessible title");
    assert_eq!(rich.acc_description, "Accessible description");
    assert_eq!(rich.title, "");

    // A parse that starts from nothing has nothing of the rich state left.
    let reset = parse("usecase-beta").expect("the reset text parses");
    assert!(reset.actors.is_empty());
    assert!(reset.use_cases.is_empty());
    assert!(reset.system_boundaries.is_empty());
    assert!(reset.relationships.is_empty());
    assert!(reset.notes.is_empty());
    assert!(reset.json_nodes.is_empty());
    assert!(reset.class_defs.is_empty());
    assert_eq!(reset.direction, Direction::Lr);
    assert_eq!(reset.title, "");
    assert_eq!(reset.acc_title, "");
    assert_eq!(reset.acc_description, "");
    assert_eq!(reset.ast.source, "usecase-beta\n");

    // The counters start again at zero: the first note and the first anonymous relation of a new parse
    // are `note-0` and `edge-0`.
    let counted = parse("usecase-beta\nactor User\nnote for User \"note\"\nUser -- User")
        .expect("the counted text parses");
    assert_eq!(counted.notes[0].id, "note-0");
    assert_eq!(counted.relationships[0].id, "edge-0");

    // Without a model there is no AST: a parse that fails returns none.
    assert!(parse("usecase-beta\nunknown command").is_err());
}

#[test]
#[ignore = "the render input (getData) is not ported; it belongs to the renderer stage"]
fn gives_oval_use_cases_extra_label_clearance_over_rectangular_use_cases() {
    let model = parse("usecase-beta\nBrowse(Browse catalog)\nManage[Manage catalog]")
        .expect("the text parses");
    let shape = |id: &str| {
        model
            .use_cases
            .iter()
            .find(|case| case.id == id)
            .map(|case| case.shape)
    };
    assert_eq!(shape("Browse"), Some(UseCaseShape::Ellipse));
    assert_eq!(shape("Manage"), Some(UseCaseShape::Rect));
    // The render input gives the padding: 20 for an ellipse and 10 for a rectangle. It is not asserted
    // here, since `getData` is not ported.
}
