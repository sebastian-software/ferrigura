//! Title precedence between the grammar and the front matter, as `Diagram.fromText` and the pie
//! DB decide it. See `docs-port/canonical-model.md`.

use ferrigura_syntax::{info, pie};

#[test]
fn the_grammar_title_wins_over_the_front_matter_title() {
    let diagram =
        pie::parse("---\ntitle: From front matter\n---\npie title From grammar\n\"a\": 1")
            .expect("parses");
    assert_eq!(diagram.title, "From grammar");
}

#[test]
fn the_front_matter_title_fills_in_when_the_grammar_gives_none() {
    let diagram = pie::parse("---\ntitle: Only front matter\n---\npie\n\"a\": 1").expect("parses");
    assert_eq!(diagram.title, "Only front matter");
}

#[test]
fn an_empty_grammar_title_lets_the_front_matter_title_through() {
    let diagram =
        pie::parse("---\ntitle: Empty grammar title\n---\npie title\n\"a\": 1").expect("parses");
    assert_eq!(diagram.title, "Empty grammar title");
}

#[test]
fn the_info_model_ignores_the_front_matter_title() {
    let diagram = info::parse("---\ntitle: Only front matter\n---\ninfo").expect("parses");
    assert_eq!(diagram.title, "");
}
