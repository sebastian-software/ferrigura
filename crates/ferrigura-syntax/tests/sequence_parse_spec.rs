//! Port of `packages/mermaid/src/diagrams/sequence/sequenceDiagram.spec.js`, the parse-level blocks
//! (`:97-938`: "more than one sequence diagram", the central connections, and the definitions,
//! aliases, arrows and activations of "when parsing a sequenceDiagram"), the two `__proto__` and
//! `constructor` actor-name tests of the mirror block (`:2370-2395`), and the quirks of
//! `docs-port/sequence/PLAN.md` section 8 that the spec does not cover. The statement tests from
//! "should handle comments in a sequenceDiagram" to the endpoint tables are in
//! `sequence_statements_spec.rs`; the DB-level blocks (`links`, `box`, creation and destruction, and
//! the "sequence db class" block) are in `sequence_db_spec.rs`.
//!
//! Test names are the spec's titles in snake_case; a title that repeats after normalising gets `_2`,
//! `_3`, … in source order. The `it.each` tables are expanded into one test per row.
//!
//! The reference's `await Diagram.fromText(...)` is `parse(...)`, and `db.getX()` is the model field
//! of the same meaning. The `actors.get(id).description = 'Bob'` assignments of the spec have no
//! effect on any expectation, so they are not ported. The `draw(...)` calls are not ported; where a
//! test checks only what the parse produced, its intent is kept.
//!
//! The `describe` blocks of the reference are `mod`s here.

use ferrigura_model::sequence::{
    CentralConnection, LineType, MessageText, SequenceActor, SequenceDiagram, SequenceMessage,
};
use ferrigura_syntax::sequence::parse;

fn parsed(text: &str) -> SequenceDiagram {
    parse(text).unwrap_or_else(|error| panic!("does not parse: {error}"))
}

fn actor<'a>(diagram: &'a SequenceDiagram, id: &str) -> &'a SequenceActor {
    diagram
        .actors
        .iter()
        .find(|actor| actor.id == id)
        .unwrap_or_else(|| panic!("no actor {id}"))
}

fn actor_ids(diagram: &SequenceDiagram) -> Vec<&str> {
    diagram
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .collect()
}

fn description<'a>(diagram: &'a SequenceDiagram, id: &str) -> Option<&'a str> {
    actor(diagram, id).description.as_str()
}

fn actor_type<'a>(diagram: &'a SequenceDiagram, id: &str) -> Option<&'a str> {
    actor(diagram, id).actor_type.as_str()
}

fn text(message: &SequenceMessage) -> &str {
    match &message.message {
        MessageText::Text(text) => text,
        MessageText::Autonumber(_) => panic!("an autonumber item has no text"),
    }
}

/// The first message between two actors, which the spec finds with
/// `msg.type !== undefined && msg.from && msg.to`.
fn actual_message(diagram: &SequenceDiagram) -> &SequenceMessage {
    diagram
        .messages
        .iter()
        .find(|message| message.from.is_some() && message.to.is_some())
        .expect("a message between two actors")
}

fn actual_messages(diagram: &SequenceDiagram) -> Vec<&SequenceMessage> {
    diagram
        .messages
        .iter()
        .filter(|message| message.from.is_some() && message.to.is_some())
        .collect()
}

mod more_than_one_sequence_diagram {
    use super::*;

    #[test]
    fn should_not_have_duplicated_messages() {
        let diagram = parsed(
            "
        sequenceDiagram
        Alice->Bob:Hello Bob, how are you?
        Bob-->Alice: I am good thanks!",
        );
        let messages = &diagram.messages;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, "0");
        assert_eq!(messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(messages[0].to.as_deref(), Some("Bob"));
        assert_eq!(text(&messages[0]), "Hello Bob, how are you?");
        assert_eq!(messages[0].line_type, LineType::SolidOpen);
        assert_eq!(messages[0].activate, Some(false));
        assert_eq!(
            messages[0].central_connection,
            Some(CentralConnection::NoConnection)
        );
        assert!(!messages[0].wrap);
        assert_eq!(messages[1].id, "1");
        assert_eq!(messages[1].from.as_deref(), Some("Bob"));
        assert_eq!(messages[1].to.as_deref(), Some("Alice"));
        assert_eq!(text(&messages[1]), "I am good thanks!");
        assert_eq!(messages[1].line_type, LineType::DottedOpen);

        // The same text parsed again gives the same list.
        let again = parsed(
            "
        sequenceDiagram
        Alice->Bob:Hello Bob, how are you?
        Bob-->Alice: I am good thanks!",
        );
        assert_eq!(again.messages, *messages);

        // A different third actor gives its own list.
        let john = parsed(
            "
        sequenceDiagram
        Alice->John:Hello John, how are you?
        John-->Alice: I am good thanks!",
        );
        assert_eq!(john.messages.len(), 2);
        assert_eq!(john.messages[0].to.as_deref(), Some("John"));
        assert_eq!(text(&john.messages[0]), "Hello John, how are you?");
        assert_eq!(john.messages[1].from.as_deref(), Some("John"));
    }
}

mod central_connection_parsing {
    use super::*;

    mod when_parsing_central_connection_syntax {
        use super::*;

        #[test]
        fn should_parse_actor_actor_syntax_as_central_connection_dual() {
            // `()->>()` is the dual central connection, written as `()->>()` between the actors.
            let diagram = parsed(
                "
        sequenceDiagram
        participant Alice
        participant Bob
        Alice ()->>() Bob: Hello Bob, how are you?
      ",
            );

            assert_eq!(diagram.messages.len(), 3);
            let message = actual_message(&diagram);
            assert_eq!(message.from.as_deref(), Some("Alice"));
            assert_eq!(message.to.as_deref(), Some("Bob"));
            assert_eq!(text(message), "Hello Bob, how are you?");
            assert_eq!(message.central_connection, Some(CentralConnection::Dual));
            assert_eq!(message.activate, Some(true));
            assert_eq!(message.line_type, LineType::Solid);
        }

        #[test]
        fn should_parse_actor_actor_syntax_as_central_connection_dual_2() {
            let diagram = parsed(
                "
        sequenceDiagram
        participant Alice
        participant Bob
        Alice ()-->>() Bob: Hello Bob, how are you?
      ",
            );

            assert_eq!(diagram.messages.len(), 3);
            let message = actual_message(&diagram);
            assert_eq!(message.from.as_deref(), Some("Alice"));
            assert_eq!(message.to.as_deref(), Some("Bob"));
            assert_eq!(text(message), "Hello Bob, how are you?");
            assert_eq!(message.central_connection, Some(CentralConnection::Dual));
            assert_eq!(message.activate, Some(true));
            assert_eq!(message.line_type, LineType::Dotted);
        }

        #[test]
        fn should_parse_actor_actor_syntax_as_central_connection() {
            let diagram = parsed(
                "
        sequenceDiagram
        participant Alice
        participant Bob
        Alice ->>() Bob: Hello Bob, how are you?
      ",
            );

            assert_eq!(diagram.messages.len(), 2);
            let message = actual_message(&diagram);
            assert_eq!(message.from.as_deref(), Some("Alice"));
            assert_eq!(message.to.as_deref(), Some("Bob"));
            assert_eq!(text(message), "Hello Bob, how are you?");
            assert_eq!(message.central_connection, Some(CentralConnection::Central));
            assert_eq!(message.activate, Some(true));
            assert_eq!(message.line_type, LineType::Solid);
        }

        #[test]
        fn should_parse_actor_actor_syntax_as_central_connection_reverse() {
            let diagram = parsed(
                "
        sequenceDiagram
        participant Alice
        participant Bob
        Alice ()-->> Bob: Hello Bob, how are you?
      ",
            );

            assert_eq!(diagram.messages.len(), 2);
            let message = actual_message(&diagram);
            assert_eq!(message.from.as_deref(), Some("Alice"));
            assert_eq!(message.to.as_deref(), Some("Bob"));
            assert_eq!(text(message), "Hello Bob, how are you?");
            assert_eq!(message.central_connection, Some(CentralConnection::Reverse));
            assert_eq!(message.activate, Some(false));
            assert_eq!(message.line_type, LineType::Dotted);
        }

        #[test]
        fn should_parse_actor_actor_syntax_as_central_connection_reverse_2() {
            let diagram = parsed(
                "
        sequenceDiagram
        participant Alice
        participant Bob
        Alice ()->> Bob: Hello Bob, how are you?
      ",
            );

            assert_eq!(diagram.messages.len(), 2);
            let message = actual_message(&diagram);
            assert_eq!(message.from.as_deref(), Some("Alice"));
            assert_eq!(message.to.as_deref(), Some("Bob"));
            assert_eq!(text(message), "Hello Bob, how are you?");
            assert_eq!(message.central_connection, Some(CentralConnection::Reverse));
            assert_eq!(message.activate, Some(false));
            assert_eq!(message.line_type, LineType::Solid);
        }

        #[test]
        fn should_parse_actor_actor_syntax_as_central_connection_dual_3() {
            let diagram = parsed(
                "
        sequenceDiagram
        participant Alice
        participant Bob
        Alice ()<<-->>() Bob: Hello Bob, how are you?
      ",
            );

            assert_eq!(diagram.messages.len(), 3);
            let message = actual_message(&diagram);
            assert_eq!(message.from.as_deref(), Some("Alice"));
            assert_eq!(message.to.as_deref(), Some("Bob"));
            assert_eq!(text(message), "Hello Bob, how are you?");
            assert_eq!(message.central_connection, Some(CentralConnection::Dual));
            assert_eq!(message.activate, Some(true));
            assert_eq!(message.line_type, LineType::BidirectionalDotted);
        }

        #[test]
        fn should_parse_actor_actor_syntax_as_central_connection_dual_4() {
            let diagram = parsed(
                "
        sequenceDiagram
        participant Alice
        participant Bob
        Alice ()<<->>() Bob: Hello Bob, how are you?
      ",
            );

            assert_eq!(diagram.messages.len(), 3);
            let message = actual_message(&diagram);
            assert_eq!(message.from.as_deref(), Some("Alice"));
            assert_eq!(message.to.as_deref(), Some("Bob"));
            assert_eq!(text(message), "Hello Bob, how are you?");
            assert_eq!(message.central_connection, Some(CentralConnection::Dual));
            assert_eq!(message.activate, Some(true));
            assert_eq!(message.line_type, LineType::BidirectionalSolid);
        }

        #[test]
        fn should_handle_multiple_central_connection_types_in_one_diagram() {
            let diagram = parsed(
                "
        sequenceDiagram
        participant Alice
        participant Bob
        participant Charlie
        Alice ()->>() Bob: Message 1
        Bob ()-->> Charlie: Message 2
        Charlie ()<<-->>() Alice: Message 3
      ",
            );

            // Three messages and five central connection markers.
            assert_eq!(diagram.messages.len(), 8);
            let actual = actual_messages(&diagram);
            assert_eq!(actual.len(), 3);
            assert_eq!(actual[0].from.as_deref(), Some("Alice"));
            assert_eq!(actual[0].to.as_deref(), Some("Bob"));
            assert_eq!(actual[0].central_connection, Some(CentralConnection::Dual));
            assert_eq!(actual[1].from.as_deref(), Some("Bob"));
            assert_eq!(actual[1].to.as_deref(), Some("Charlie"));
            assert_eq!(
                actual[1].central_connection,
                Some(CentralConnection::Reverse)
            );
            assert_eq!(actual[2].from.as_deref(), Some("Charlie"));
            assert_eq!(actual[2].to.as_deref(), Some("Alice"));
            assert_eq!(actual[2].central_connection, Some(CentralConnection::Dual));
        }

        #[test]
        fn should_handle_central_connections_with_different_arrow_types() {
            let diagram = parsed(
                "
        sequenceDiagram
        participant Alice
        participant Bob
        Alice ()-x() Bob: Cross message
        Alice ()--x() Bob: Dotted cross message
      ",
            );

            // Two messages and four central connection markers.
            assert_eq!(diagram.messages.len(), 6);
            let actual = actual_messages(&diagram);
            assert_eq!(actual.len(), 2);
            assert_eq!(actual[0].from.as_deref(), Some("Alice"));
            assert_eq!(actual[0].to.as_deref(), Some("Bob"));
            assert_eq!(actual[0].central_connection, Some(CentralConnection::Dual));
            assert_eq!(actual[0].line_type, LineType::SolidCross);
            assert_eq!(actual[1].from.as_deref(), Some("Alice"));
            assert_eq!(actual[1].to.as_deref(), Some("Bob"));
            assert_eq!(actual[1].central_connection, Some(CentralConnection::Dual));
            assert_eq!(actual[1].line_type, LineType::DottedCross);
        }

        #[test]
        fn should_not_break_existing_parsing_without_central_connections() {
            let diagram = parsed(
                "
        sequenceDiagram
        participant Alice
        participant Bob
        Alice ->> Bob: Normal message
        Bob -->> Alice: Normal dotted message
        Alice -x Bob: Normal cross message
      ",
            );

            let messages = &diagram.messages;
            assert_eq!(messages.len(), 3);
            for message in messages {
                assert_eq!(
                    message.central_connection,
                    Some(CentralConnection::NoConnection)
                );
            }
            assert_eq!(messages[0].line_type, LineType::Solid);
            assert_eq!(messages[1].line_type, LineType::Dotted);
            assert_eq!(messages[2].line_type, LineType::SolidCross);
        }
    }
}

mod when_parsing_a_sequence_diagram {
    use super::*;

    /// The fixture of the spec's `beforeEach`.
    fn basic() -> SequenceDiagram {
        parsed(
            "
sequenceDiagram
Alice->Bob:Hello Bob, how are you?
Note right of Bob: Bob thinks
Bob-->Alice: I am good thanks!",
        )
    }

    #[test]
    fn should_handle_a_sequence_diagram_definition() {
        let diagram = basic();

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(messages[2].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_not_show_sequence_numbers_per_default() {
        // The reference shows the numbers when `draw` sees a visible autonumber item. The parse
        // result is the absence of any autonumber item.
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob:Hello Bob, how are you?
Note right of Bob: Bob thinks
Bob-->Alice: I am good thanks!",
        );

        assert!(
            !diagram
                .messages
                .iter()
                .any(|message| matches!(message.message, MessageText::Autonumber(_)))
        );
    }

    #[test]
    fn should_show_sequence_numbers_when_autonumber_is_enabled() {
        // The parse result is an autonumber item that is visible.
        let diagram = parsed(
            "
sequenceDiagram
autonumber
Alice->Bob:Hello Bob, how are you?
Note right of Bob: Bob thinks
Bob-->Alice: I am good thanks!",
        );

        let numbering = diagram
            .messages
            .iter()
            .find_map(|message| match &message.message {
                MessageText::Autonumber(numbering) => Some(numbering),
                MessageText::Text(_) => None,
            })
            .expect("an autonumber item");
        assert!(numbering.visible);
    }

    #[test]
    fn should_allow_sequence_numbers_to_have_decimals_up_to_the_hundredths_place() {
        let diagram = parsed(
            "
        sequenceDiagram
        autonumber 10.1 .01
        Alice->Bob:Hello Bob, how are you?
        Note right of Bob: Bob thinks
        Bob-->Alice: I am good thanks!
      ",
        );

        let numbering = match &diagram.messages[0].message {
            MessageText::Autonumber(numbering) => numbering,
            MessageText::Text(_) => panic!("the first item is the autonumber"),
        };
        assert_eq!(numbering.start, Some(10.1));
        assert_eq!(numbering.step, Some(0.01));
    }

    #[test]
    fn should_not_allow_sequence_numbers_to_have_decimals_to_the_thousandths_place_or_greater() {
        let error = parse(
            "
      sequenceDiagram
      autonumber 10.001
      Alice->Bob:Hello Bob, how are you?
      Note right of Bob: Bob thinks
      Bob-->Alice: I am good thanks!
    ",
        )
        .expect_err("the number has three decimals");
        assert_eq!(error.line, 3);
    }

    #[test]
    fn should_handle_a_sequence_diagram_definition_with_a_title() {
        let diagram = parsed(
            "
sequenceDiagram
title: Diagram Title
Alice->Bob:Hello Bob, how are you?
Note right of Bob: Bob thinks
Bob-->Alice: I am good thanks!",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.acc_descr, "");
        assert_eq!(diagram.messages.len(), 3);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[2].from.as_deref(), Some("Bob"));
        assert_eq!(diagram.title, "Diagram Title");
    }

    #[test]
    fn should_handle_a_sequence_diagram_definition_with_a_title_without_a_colon() {
        let diagram = parsed(
            "
sequenceDiagram
title Diagram Title
Alice->Bob:Hello Bob, how are you?
Note right of Bob: Bob thinks
Bob-->Alice: I am good thanks!",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.acc_descr, "");
        assert_eq!(diagram.messages.len(), 3);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[2].from.as_deref(), Some("Bob"));
        assert_eq!(diagram.title, "Diagram Title");
    }

    #[test]
    fn should_handle_a_sequence_diagram_definition_with_a_accessibility_title_and_description_acc_descr()
     {
        let diagram = parsed(
            "
sequenceDiagram
title: Diagram Title
accTitle: This is the title
accDescr: Accessibility Description
Alice->Bob:Hello Bob, how are you?
",
        );

        assert_eq!(diagram.title, "Diagram Title");
        assert_eq!(diagram.acc_title, "This is the title");
        assert_eq!(diagram.acc_descr, "Accessibility Description");
    }

    #[test]
    fn should_handle_a_sequence_diagram_definition_with_a_accessibility_title_and_multiline_description_acc_descr()
     {
        let diagram = parsed(
            "
sequenceDiagram
accTitle: This is the title
accDescr {
Accessibility
Description
}
Alice->Bob:Hello Bob, how are you?
",
        );

        assert_eq!(diagram.acc_title, "This is the title");
        assert_eq!(diagram.acc_descr, "Accessibility\nDescription");
    }

    #[test]
    fn should_space_in_actor_names() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob:Hello Bob, how are - you?
Bob-->Alice: I am good thanks!",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.messages.len(), 2);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[1].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_dashes_in_actor_names() {
        let diagram = parsed(
            "
sequenceDiagram
Alice-in-Wonderland->Bob:Hello Bob, how are - you?
Bob-->Alice-in-Wonderland:I am good thanks!",
        );

        assert_eq!(
            description(&diagram, "Alice-in-Wonderland"),
            Some("Alice-in-Wonderland")
        );
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 2);
        assert_eq!(
            diagram.messages[0].from.as_deref(),
            Some("Alice-in-Wonderland")
        );
        assert_eq!(diagram.messages[1].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_dashes_in_participant_names() {
        let diagram = parsed(
            "
sequenceDiagram
participant Alice-in-Wonderland
participant Bob
Alice-in-Wonderland->Bob:Hello Bob, how are - you?
Bob-->Alice-in-Wonderland:I am good thanks!",
        );

        assert_eq!(actor_ids(&diagram), ["Alice-in-Wonderland", "Bob"]);
        assert_eq!(
            description(&diagram, "Alice-in-Wonderland"),
            Some("Alice-in-Wonderland")
        );
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 2);
        assert_eq!(
            diagram.messages[0].from.as_deref(),
            Some("Alice-in-Wonderland")
        );
        assert_eq!(diagram.messages[1].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_equals_in_participant_names() {
        let diagram = parsed(
            "
sequenceDiagram
participant Alice=Wonderland
participant Bob
Alice=Wonderland->Bob:Hello Bob, how are - you?
Bob-->Alice=Wonderland:I am good thanks!",
        );

        assert_eq!(actor_ids(&diagram), ["Alice=Wonderland", "Bob"]);
        assert_eq!(
            description(&diagram, "Alice=Wonderland"),
            Some("Alice=Wonderland")
        );
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 2);
        assert_eq!(
            diagram.messages[0].from.as_deref(),
            Some("Alice=Wonderland")
        );
        assert_eq!(diagram.messages[1].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_alias_participants() {
        let diagram = parsed(
            "
sequenceDiagram
participant A as Alice
participant B as Bob
A->B:Hello Bob, how are you?
B-->A: I am good thanks!",
        );

        assert_eq!(actor_ids(&diagram), ["A", "B"]);
        assert_eq!(description(&diagram, "A"), Some("Alice"));
        assert_eq!(description(&diagram, "B"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 2);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("A"));
        assert_eq!(diagram.messages[1].from.as_deref(), Some("B"));
    }

    #[test]
    fn should_alias_a_mix_of_actors_and_participants_apa12() {
        let diagram = parsed(
            "
sequenceDiagram
  actor Alice as Alice2
  actor Bob
  participant John as John2
  participant Mandy
  Alice->>Bob: Hi Bob
  Bob->>Alice: Hi Alice
  Alice->>John: Hi John
  John->>Mandy: Hi Mandy
  Mandy ->>Joan: Hi Joan",
        );

        assert_eq!(
            actor_ids(&diagram),
            ["Alice", "Bob", "John", "Mandy", "Joan"]
        );
        assert_eq!(description(&diagram, "Alice"), Some("Alice2"));
        assert_eq!(actor_type(&diagram, "Alice"), Some("actor"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(actor_type(&diagram, "John"), Some("participant"));
        assert_eq!(actor_type(&diagram, "Joan"), Some("participant"));

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(messages[4].to.as_deref(), Some("Joan"));
    }

    #[test]
    fn should_alias_actors_apa13() {
        let diagram = parsed(
            "
sequenceDiagram
actor A as Alice
actor B as Bob
A->B:Hello Bob, how are you?
B-->A: I am good thanks!",
        );

        assert_eq!(actor_ids(&diagram), ["A", "B"]);
        assert_eq!(description(&diagram, "A"), Some("Alice"));
        assert_eq!(description(&diagram, "B"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 2);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("A"));
        assert_eq!(diagram.messages[1].from.as_deref(), Some("B"));
    }

    #[test]
    fn should_handle_in_async_messages() {
        let diagram = parsed(
            "
sequenceDiagram
Alice-xBob:Hello Bob, how are you?",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].line_type, LineType::SolidCross);
    }

    #[test]
    fn should_handle_in_async_dotted_messages() {
        let diagram = parsed(
            "
sequenceDiagram
Alice--xBob:Hello Bob, how are you?",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].line_type, LineType::DottedCross);
    }

    #[test]
    fn should_handle_in_sync_messages() {
        let diagram = parsed(
            "
sequenceDiagram
Alice-)Bob:Hello Bob, how are you?",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].line_type, LineType::SolidPoint);
    }

    #[test]
    fn should_handle_in_sync_dotted_messages() {
        let diagram = parsed(
            "
sequenceDiagram
Alice--)Bob:Hello Bob, how are you?",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].line_type, LineType::DottedPoint);
    }

    #[test]
    fn should_handle_in_arrow_messages() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->>Bob:Hello Bob, how are you?",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].line_type, LineType::Solid);
    }

    #[test]
    fn should_handle_in_arrow_messages_2() {
        let diagram = parsed("sequenceDiagram\nAlice-->>Bob:Hello Bob, how are you?");

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].line_type, LineType::Dotted);
    }

    #[test]
    fn should_handle_bidirectional_arrow_messages() {
        let diagram = parsed(
            "
sequenceDiagram
Alice<<->>Bob:Hello Bob, how are you?",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].line_type, LineType::BidirectionalSolid);
    }

    #[test]
    fn should_handle_bidirectional_dotted_arrow_messages() {
        let diagram = parsed(
            "
    sequenceDiagram
    Alice<<-->>Bob:Hello Bob, how are you?",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].line_type, LineType::BidirectionalDotted);
    }

    #[test]
    fn should_handle_actor_activation() {
        let diagram = parsed(
            "
sequenceDiagram
Alice-->>Bob:Hello Bob, how are you?
activate Bob
Bob-->>Alice:Hello Alice, I'm fine and  you?
deactivate Bob",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].line_type, LineType::Dotted);
        assert_eq!(messages[1].line_type, LineType::ActiveStart);
        assert_eq!(messages[1].from.as_deref(), Some("Bob"));
        assert_eq!(messages[2].line_type, LineType::Dotted);
        assert_eq!(messages[3].line_type, LineType::ActiveEnd);
        assert_eq!(messages[3].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_actor_one_line_notation_activation() {
        let diagram = parsed(
            "
    sequenceDiagram
    Alice-->>+Bob:Hello Bob, how are you?
    Bob-->>- Alice:Hello Alice, I'm fine and  you?",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].line_type, LineType::Dotted);
        assert_eq!(messages[0].activate, Some(true));
        assert_eq!(messages[1].line_type, LineType::ActiveStart);
        assert_eq!(messages[1].from.as_deref(), Some("Bob"));
        assert_eq!(messages[2].line_type, LineType::Dotted);
        assert_eq!(messages[3].line_type, LineType::ActiveEnd);
        assert_eq!(messages[3].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_stacked_activations() {
        let diagram = parsed(
            "
      sequenceDiagram
      Alice-->>+Bob:Hello Bob, how are you?
      Bob-->>+Carol:Carol, let me introduce Alice?
      Bob-->>- Alice:Hello Alice, please meet Carol?
      Carol->>- Bob:Oh Bob, I'm so happy to be here!",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 8);
        assert_eq!(messages[0].line_type, LineType::Dotted);
        assert_eq!(messages[1].line_type, LineType::ActiveStart);
        assert_eq!(messages[1].from.as_deref(), Some("Bob"));
        assert_eq!(messages[2].line_type, LineType::Dotted);
        assert_eq!(messages[3].line_type, LineType::ActiveStart);
        assert_eq!(messages[3].from.as_deref(), Some("Carol"));
        assert_eq!(messages[5].line_type, LineType::ActiveEnd);
        assert_eq!(messages[5].from.as_deref(), Some("Bob"));
        assert_eq!(messages[7].line_type, LineType::ActiveEnd);
        assert_eq!(messages[7].from.as_deref(), Some("Carol"));
    }

    #[test]
    fn should_handle_fail_parsing_when_activating_an_inactive_participant() {
        let error = parse(
            "
      sequenceDiagram
      participant user as End User
      participant Server as Server
      participant System as System
      participant System2 as System2

      user->>+Server: Test
      user->>+Server: Test2
      user->>System: Test
      Server->>-user: Test
      Server->>-user: Test2

      %% The following deactivation of Server will fail
      Server->>-user: Test3",
        )
        .expect_err("the third deactivation has no activation to end");

        // The DB error, not a syntax error: its text is the reference's.
        assert_eq!(
            error.message,
            "Trying to inactivate an inactive participant (Server)"
        );
    }
}

/// The two parse-level tests of the mirror block (`when rendering a sequenceDiagram with actor mirror
/// activated`, `:2391-2395`): the actor names that are keys of JavaScript objects.
mod actor_names_that_are_object_keys {
    use super::*;

    fn allows_as_actor_name(prop: &str) {
        assert!(
            parse(&format!(
                "\nsequenceDiagram\n{prop}-->>A: Hello, how are you?"
            ))
            .is_ok()
        );
    }

    #[test]
    fn should_allow_proto_as_an_actor_name() {
        allows_as_actor_name("__proto__");
    }

    #[test]
    fn should_allow_constructor_as_an_actor_name() {
        allows_as_actor_name("constructor");
    }
}

/// Quirks of the reference that the spec does not cover, one test each (`docs-port/sequence/PLAN.md`,
/// section 8). The reference lines are in each test's comment.
mod quirks {
    use super::*;

    /// Q1: rule 25 (`\s+`) comes after rule 24 (`[\n]+`) in INITIAL, so blanks before a line break
    /// swallow the break (`sequenceDiagram.jison:24-25`). `autonumber` then has no NEWLINE to end
    /// with.
    #[test]
    fn quirk_q1_blanks_before_a_line_break_swallow_the_break() {
        assert!(parse("sequenceDiagram\nautonumber   \nAlice->>Bob: hi").is_err());
        assert!(parse("sequenceDiagram\nautonumber\nAlice->>Bob: hi").is_ok());
    }

    /// Q3: a lone `/` is INVALID (rule 126), and `line: INVALID` is accepted and dropped
    /// (`sequenceDiagram.jison:126`, the `line` productions).
    #[test]
    fn quirk_q3_a_lone_slash_line_is_dropped() {
        let diagram = parsed("sequenceDiagram\nAlice->>Bob: hi\n/\nBob->>Alice: yo\n");

        assert_eq!(diagram.messages.len(), 2);
        assert_eq!(text(&diagram.messages[0]), "hi");
        assert_eq!(text(&diagram.messages[1]), "yo");
    }

    /// Q4: a re-declaration with a description replaces the record (`sequenceDb.ts:155-231`). Its
    /// links are reset, its `prevActor` is the earlier declaration (here itself), and the earlier
    /// record's `nextActor` is set to the same id (the new record, `:208-225`).
    #[test]
    fn quirk_q4_a_redeclaration_with_a_description_overwrites_the_actor() {
        let diagram = parsed(
            "sequenceDiagram\nparticipant A\nlink A: Help @ https://example.com\nparticipant A as Alpha\n",
        );

        let a = actor(&diagram, "A");
        assert_eq!(a.description.as_str(), Some("Alpha"));
        assert!(a.links.is_empty());
        assert_eq!(a.prev_actor.as_deref(), Some("A"));
        assert_eq!(a.next_actor.as_deref(), Some("A"));
        assert_eq!(actor_ids(&diagram), ["A"]);
    }

    /// Q5: `box #color Team` is a comment after `box`: the box has no name, and its colour is
    /// `transparent` (`sequenceDb.ts:396-412`, `LINE` state rule 26 for `#`).
    #[test]
    fn quirk_q5_a_box_colour_after_the_hash_is_a_comment() {
        let diagram = parsed("sequenceDiagram\nbox #color Team\nparticipant a\nend\n");

        assert_eq!(diagram.boxes[0].name, None);
        assert_eq!(diagram.boxes[0].fill, "transparent");
        assert_eq!(diagram.boxes[0].actor_keys, ["a"]);
    }

    /// Q6: the created-participant error prints `undefined` for the name, because `lastCreated` is
    /// an id and the message reads its `.name` (`sequenceDb.ts:615-621`). The text is asserted
    /// exactly.
    #[test]
    fn quirk_q6_the_created_participant_error_names_undefined() {
        let error = parse("sequenceDiagram\nparticipant A\ncreate participant C\nA->>B: hi\n")
            .expect_err("the created participant has no creating message");

        assert_eq!(
            error.message,
            "The created participant undefined does not have an associated creating message after its declaration. Please check the sequence diagram."
        );
    }

    /// Q7: a `break` block's end carries no text, so its message is `""` (`sequenceDb.ts:705-706`).
    #[test]
    fn quirk_q7_the_break_end_message_is_empty() {
        let diagram =
            parsed("sequenceDiagram\nAlice->>Bob: hi\nbreak when it fails\nBob->>Alice: no\nend\n");

        let end = diagram
            .messages
            .iter()
            .find(|message| message.line_type == LineType::BreakEnd)
            .expect("a break end");
        assert_eq!(text(end), "");
    }

    /// Q8: the bare `autonumber` stores `start` and `step` as undefined, and its item has no
    /// `activate` or central connection (`sequenceDb.ts:549-560`, `sequenceDiagram.jison:175-178`).
    /// `autonumber 5` gives step 1.
    #[test]
    fn quirk_q8_bare_autonumber_has_no_start_or_step() {
        let bare = parsed("sequenceDiagram\nautonumber\nAlice->>Bob: hi\n");
        let item = &bare.messages[0];
        match &item.message {
            MessageText::Autonumber(numbering) => {
                assert_eq!(numbering.start, None);
                assert_eq!(numbering.step, None);
                assert!(numbering.visible);
            }
            MessageText::Text(_) => panic!("the first item is the autonumber"),
        }
        assert_eq!(item.activate, None);
        assert_eq!(item.central_connection, None);

        let numbered = parsed("sequenceDiagram\nautonumber 5\nAlice->>Bob: hi\n");
        match &numbered.messages[0].message {
            MessageText::Autonumber(numbering) => {
                assert_eq!(numbering.start, Some(5.0));
                assert_eq!(numbering.step, Some(1.0));
            }
            MessageText::Text(_) => panic!("the first item is the autonumber"),
        }
    }

    /// Q9: `#quot;` is encoded before the grammar as the placeholder `ﬂ°quot¶ß`, and the model keeps
    /// the placeholder (`utils.ts:874-905`).
    #[test]
    fn quirk_q9_entities_are_placeholders_in_messages() {
        let diagram = parsed("sequenceDiagram\nAlice->>Bob: #quot;x#quot;\n");

        assert_eq!(text(&diagram.messages[0]), "ﬂ°quot¶ßxﬂ°quot¶ß");
    }

    /// Q12: a `title` takes one character after its keyword, so the second blank stays
    /// (`sequenceDiagram.jison:70`, `sequenceRenderer.ts:1450`).
    #[test]
    fn quirk_q12_a_title_keeps_the_blank_after_its_keyword() {
        let diagram = parsed("sequenceDiagram\ntitle  x\nAlice->>Bob: hi\n");

        assert_eq!(diagram.title, " x");
    }

    /// Q13: `accTitle:` followed by a line break takes the next line as its value, because the
    /// blanks after the colon span the break (`sequenceDiagram.jison:72`).
    #[test]
    fn quirk_q13_acc_title_with_a_line_break_takes_the_next_line() {
        let diagram = parsed("sequenceDiagram\naccTitle:\nHello\nAlice->>Bob: hi\n");

        assert_eq!(diagram.acc_title, "Hello");
    }

    /// Q14: the value of `accTitle` is the rest of the line, so `;` and `#` stay in it
    /// (`sequenceDiagram.jison:73`).
    #[test]
    fn quirk_q14_acc_title_keeps_semicolons_and_hashes() {
        let diagram = parsed("sequenceDiagram\naccTitle: a;b #c\nAlice->>Bob: hi\n");

        assert_eq!(diagram.acc_title, "a;b #c");
    }
}
