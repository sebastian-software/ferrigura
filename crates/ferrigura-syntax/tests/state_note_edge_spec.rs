//! Placeholders for `packages/mermaid/src/diagrams/state/stateNoteEdge.spec.ts`. Every test reads
//! the edges from `getData().edges`, which the port does not model here (the data differential
//! covers it), so the bodies are not ported. Each test keeps the spec's title, in snake_case.

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn declares_the_note_edge_as_dashed_on_the_edge_not_only_in_css() {
    // stateNoteEdge.spec.ts: "declares the note edge as dashed on the edge, not only in CSS"
}

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn leaves_ordinary_transitions_solid() {
    // stateNoteEdge.spec.ts: "leaves ordinary transitions solid"
}

#[test]
#[ignore = "port: getData (dataFetcher node and edge data), M6 data differential"]
fn keeps_the_note_edge_class_which_is_what_the_classic_look_still_styles() {
    // stateNoteEdge.spec.ts: "keeps the note-edge class, which is what the classic look still styles"
}
