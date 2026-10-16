//! Port of `packages/mermaid/src/diagrams/sequence/keywordActors.spec.ts`: regression tests for the
//! actor-menu keywords (`link`, `links`, `properties`, `details`) used as participant ids. The
//! keywords are lexed as statement starters only when an actor follows them, so `A->>Link: hello`
//! is an actor reference.
//!
//! The reference's `db.getActors().keys()`, `getMessages()` and `getActor(id)` are the model's
//! `actors`, `messages` and the actor with that id. The `details` test reads a page element (PLAN
//! section 8, Q11) and is ignored.

use std::collections::BTreeMap;

use ferrigura_model::sequence::{MessageText, SequenceDiagram, SequenceMessage};
use ferrigura_model::value::Value;
use ferrigura_syntax::sequence::parse;

fn parsed(text: &str) -> SequenceDiagram {
    parse(text).unwrap_or_else(|error| panic!("does not parse: {error}"))
}

fn actor_ids(diagram: &SequenceDiagram) -> Vec<&str> {
    diagram
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .collect()
}

fn text(message: &SequenceMessage) -> &str {
    match &message.message {
        MessageText::Text(text) => text,
        MessageText::Autonumber(_) => panic!("an autonumber item has no text"),
    }
}

fn strings(map: &BTreeMap<String, Value>) -> BTreeMap<String, String> {
    map.iter()
        .map(|(key, value)| {
            (
                key.clone(),
                value.as_str().expect("a string value").to_owned(),
            )
        })
        .collect()
}

fn single(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

/// `it.each(['Links', 'Properties', 'Details'])('accepts %s as a recipient')`.
fn accepts_as_a_recipient(name: &str) {
    let diagram = parsed(&format!(
        "sequenceDiagram\n    participant A\n    participant {name} as Alias\n    A->>{name}: message"
    ));

    assert_eq!(diagram.messages[0].from.as_deref(), Some("A"));
    assert_eq!(diagram.messages[0].to.as_deref(), Some(name));
}

mod actor_menu_keywords_as_participant_ids {
    use super::*;

    #[test]
    fn accepts_link_as_a_message_recipient_the_reported_case() {
        let diagram = parsed(
            "sequenceDiagram
    participant A
    participant Link as Channel
    A->>Link: message",
        );

        assert_eq!(actor_ids(&diagram), ["A", "Link"]);
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("A"));
        assert_eq!(diagram.messages[0].to.as_deref(), Some("Link"));
    }

    #[test]
    fn accepts_link_as_a_message_sender() {
        let diagram = parsed(
            "sequenceDiagram
    participant A
    participant Link as Channel
    Link->>A: message",
        );

        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Link"));
        assert_eq!(diagram.messages[0].to.as_deref(), Some("A"));
    }

    #[test]
    fn accepts_link_as_a_sender_with_spaces_around_the_arrow() {
        let diagram = parsed(
            "sequenceDiagram
    participant A
    participant Link as Channel
    Link  -->>A: message",
        );

        assert_eq!(diagram.messages[0].from.as_deref(), Some("Link"));
        assert_eq!(diagram.messages[0].to.as_deref(), Some("A"));
    }

    #[test]
    fn accepts_link_as_a_recipient_with_a_space_before_the_colon() {
        let diagram = parsed(
            "sequenceDiagram
    participant A
    participant Link as Channel
    A->>Link : message",
        );

        assert_eq!(diagram.messages[0].from.as_deref(), Some("A"));
        assert_eq!(diagram.messages[0].to.as_deref(), Some("Link"));
    }

    #[test]
    fn accepts_links_as_a_recipient() {
        accepts_as_a_recipient("Links");
    }

    #[test]
    fn accepts_properties_as_a_recipient() {
        accepts_as_a_recipient("Properties");
    }

    #[test]
    fn accepts_details_as_a_recipient() {
        accepts_as_a_recipient("Details");
    }

    #[test]
    fn keeps_the_original_spelling_of_the_participant_id() {
        let diagram = parsed(
            "sequenceDiagram
    participant A
    participant LINK as Channel
    A->>LINK: message",
        );

        assert!(diagram.actors.iter().any(|actor| actor.id == "LINK"));
        assert_eq!(diagram.messages[0].from.as_deref(), Some("A"));
        assert_eq!(diagram.messages[0].to.as_deref(), Some("LINK"));
    }

    #[test]
    fn does_not_treat_the_word_link_inside_a_message_as_a_directive() {
        let diagram = parsed(
            "sequenceDiagram
    participant A
    participant B
    A->>B: see the link page",
        );

        assert_eq!(text(&diagram.messages[0]), "see the link page");
    }
}

mod actor_menu_statements_still_parse {
    use super::*;

    fn links_of(diagram: &SequenceDiagram, id: &str) -> BTreeMap<String, String> {
        strings(
            &diagram
                .actors
                .iter()
                .find(|actor| actor.id == id)
                .expect("actor")
                .links,
        )
    }

    #[test]
    fn parses_a_link_statement() {
        let diagram = parsed(
            "sequenceDiagram
    participant A
    link A: Help @ https://example.com",
        );

        assert_eq!(
            links_of(&diagram, "A"),
            single(&[("Help", "https://example.com")])
        );
    }

    #[test]
    fn parses_a_link_statement_with_a_space_before_the_colon() {
        let diagram = parsed(
            "sequenceDiagram
    participant A
    link A : Help @ https://example.com",
        );

        assert_eq!(
            links_of(&diagram, "A"),
            single(&[("Help", "https://example.com")])
        );
    }

    #[test]
    fn parses_a_links_statement() {
        let diagram = parsed(
            "sequenceDiagram
    participant A
    links A: {\"help\": \"https://example.com\"}",
        );

        assert_eq!(
            links_of(&diagram, "A"),
            single(&[("help", "https://example.com")])
        );
    }

    #[test]
    fn parses_a_properties_statement() {
        let diagram = parsed(
            "sequenceDiagram
    participant A
    properties A: {\"color\": \"red\"}",
        );

        let properties = &diagram.actors[0].properties;
        assert_eq!(properties.len(), 1);
        assert_eq!(properties.get("color").and_then(Value::as_str), Some("red"));
    }

    #[test]
    #[ignore = "port: needs the DOM (details reads a page element, PLAN section 8, Q11)"]
    fn parses_a_details_statement() {}
}
