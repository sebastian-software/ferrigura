//! Port of `packages/mermaid/src/diagrams/sequence/sequenceDiagram.spec.js`, the statement tests of
//! the "when parsing a sequenceDiagram" describe (`:438-1618`, from "should handle comments in a
//! sequenceDiagram" to "should handle no-label par", and the two `it.each` declared and implicit
//! endpoint tables of `:1592-1617`). The first half of the block is in `sequence_parse_spec.rs`.
//!
//! The names and the rules are those of `sequence_parse_spec.rs`: a title that repeats after
//! normalising gets `_2`, `_3`, … in source order, and an `it.each` table is one test per row.

use ferrigura_model::sequence::{
    LineType, MessageText, Placement, SequenceDiagram, SequenceMessage,
};
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

fn description<'a>(diagram: &'a SequenceDiagram, id: &str) -> Option<&'a str> {
    diagram
        .actors
        .iter()
        .find(|actor| actor.id == id)
        .unwrap_or_else(|| panic!("no actor {id}"))
        .description
        .as_str()
}

fn text(message: &SequenceMessage) -> &str {
    match &message.message {
        MessageText::Text(text) => text,
        MessageText::Autonumber(_) => panic!("an autonumber item has no text"),
    }
}

mod when_parsing_a_sequence_diagram_statements {
    use super::*;

    #[test]
    fn should_handle_comments_in_a_sequence_diagram() {
        let diagram = parsed(
            "
      sequenceDiagram
      Alice->Bob: Hello Bob, how are you?
      %% Comment
      Note right of Bob: Bob thinks
      Bob-->Alice: I am good thanks!",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.messages.len(), 3);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[2].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_new_lines_in_a_sequence_diagram() {
        let diagram = parsed(
            "
      sequenceDiagram
      Alice->Bob: Hello Bob, how are you?

      %% Comment
      Note right of Bob: Bob thinks
      Bob-->Alice: I am good thanks!
      ",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.messages.len(), 3);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[2].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_semicolons() {
        let diagram = parsed(
            "
sequenceDiagram;Alice->Bob: Hello Bob, how are you?;Note right of Bob: Bob thinks;Bob-->Alice: I am good thanks!;",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.messages.len(), 3);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[2].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_one_leading_space_in_lines_in_a_sequence_diagram() {
        let diagram = parsed(
            "
sequenceDiagram
 Alice->Bob: Hello Bob, how are you?

%% Comment
Note right of Bob: Bob thinks
Bob-->Alice: I am good thanks!",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.messages.len(), 3);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[2].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_several_leading_spaces_in_lines_in_a_sequence_diagram() {
        let diagram = parsed(
            "
sequenceDiagram
   Alice->Bob: Hello Bob, how are you?

%% Comment
Note right of Bob: Bob thinks
Bob-->Alice: I am good thanks!",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.messages.len(), 3);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[2].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_several_leading_spaces_in_lines_in_a_sequence_diagram_2() {
        let diagram = parsed(
            "
sequenceDiagram
participant Alice
participant Bob
Alice->John: Hello John, how are you?
    loop Healthcheck
John->John: Fight against hypochondria
 end
Note right of John: Rational thoughts<br/>prevail...
    John-->Alice: Great!
    John->Bob: How about you?
Bob-->John: Jolly good!",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        let messages = &diagram.messages;
        assert_eq!(messages.len(), 8);
        assert_eq!(messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(messages[2].from.as_deref(), Some("John"));
    }

    #[test]
    fn should_handle_different_line_breaks() {
        // The spec's `\t` escapes are tabs in the diagram text, so this text is a normal string.
        let diagram = parsed(
            "\nsequenceDiagram\nparticipant 1 as multiline<br>text\nparticipant 2 as multiline<br/>text\nparticipant 3 as multiline<br />text\nparticipant 4 as multiline<br \t/>text\n1->>2: multiline<br>text\nnote right of 2: multiline<br>text\n2->>3: multiline<br/>text\nnote right of 3: multiline<br/>text\n3->>4: multiline<br />text\nnote right of 4: multiline<br />text\n4->>1: multiline<br \t/>text\nnote right of 1: multiline<br \t/>text\n",
        );

        assert_eq!(description(&diagram, "1"), Some("multiline<br>text"));
        assert_eq!(description(&diagram, "2"), Some("multiline<br/>text"));
        assert_eq!(description(&diagram, "3"), Some("multiline<br />text"));
        assert_eq!(description(&diagram, "4"), Some("multiline<br \t/>text"));

        let messages = &diagram.messages;
        let expected = [
            "multiline<br>text",
            "multiline<br>text",
            "multiline<br/>text",
            "multiline<br/>text",
            "multiline<br />text",
            "multiline<br />text",
            "multiline<br \t/>text",
            "multiline<br \t/>text",
        ];
        for (message, expected) in messages.iter().zip(expected) {
            assert_eq!(text(message), expected);
        }
    }

    #[test]
    fn should_handle_notes_and_messages_without_wrap_specified() {
        let diagram = parsed(
            "
sequenceDiagram
participant 1
participant 2
participant 3
participant 4
1->>2: single-line text
note right of 2: single-line text
2->>3:nowrap: single-line text
note right of 3:nowrap: single-line text
3->>4: multiline<br/>text
note right of 4: multiline<br/>text
4->>1:nowrap: multiline<br/>text
note right of 1:nowrap: multiline<br/>text
",
        );

        let messages = &diagram.messages;
        let expected = [
            "single-line text",
            "single-line text",
            "single-line text",
            "single-line text",
            "multiline<br/>text",
            "multiline<br/>text",
            "multiline<br/>text",
            "multiline<br/>text",
        ];
        for (index, expected) in expected.into_iter().enumerate() {
            assert_eq!(text(&messages[index]), expected);
            // `wrap` says whether the wrap prefix was given.
            assert!(!messages[index].wrap);
        }
    }

    #[test]
    fn should_handle_notes_and_messages_with_wrap_specified() {
        let diagram = parsed(
            "
sequenceDiagram
participant 1
participant 2
participant 3
participant 4
1->>2:wrap: single-line text
note right of 2:wrap: single-line text
2->>3:wrap: multiline<br/>text
note right of 3:wrap: multiline<br/>text
",
        );

        let messages = &diagram.messages;
        assert_eq!(text(&messages[0]), "single-line text");
        assert_eq!(text(&messages[1]), "single-line text");
        assert_eq!(text(&messages[2]), "multiline<br/>text");
        assert_eq!(text(&messages[3]), "multiline<br/>text");
        for message in &messages[..4] {
            assert!(message.wrap);
        }
    }

    #[test]
    fn should_handle_notes_and_messages_with_nowrap_or_line_breaks() {
        let diagram = parsed(
            "
sequenceDiagram
participant 1
participant 2
1->>2: single-line text
note right of 2: single-line text
",
        );

        let messages = &diagram.messages;
        assert_eq!(text(&messages[0]), "single-line text");
        assert_eq!(text(&messages[1]), "single-line text");
        assert!(!messages[0].wrap);
        assert!(!messages[1].wrap);
    }

    #[test]
    fn should_handle_notes_over_a_single_actor() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
Note over Bob: Bob thinks
",
        );

        assert_eq!(diagram.messages[1].from.as_deref(), Some("Bob"));
        assert_eq!(diagram.messages[1].to.as_deref(), Some("Bob"));
        assert_eq!(diagram.messages[1].placement, Some(Placement::Over));
    }

    #[test]
    fn should_handle_notes_over_multiple_actors() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
Note over Alice,Bob: confusion
Note over Bob,Alice: resolution
",
        );

        let messages = &diagram.messages;
        assert_eq!(messages[1].from.as_deref(), Some("Alice"));
        assert_eq!(messages[1].to.as_deref(), Some("Bob"));
        assert_eq!(messages[2].from.as_deref(), Some("Bob"));
        assert_eq!(messages[2].to.as_deref(), Some("Alice"));
    }

    #[test]
    fn should_handle_loop_statements() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
%% Comment
Note right of Bob: Bob thinks
loop Multiple happy responses

Bob-->Alice: I am good thanks!
end",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.messages.len(), 5);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[1].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_add_a_rect_around_sequence() {
        let diagram = parsed(
            "
      sequenceDiagram
        Alice->Bob: Hello Bob, how are you?
        %% Comment
        rect rgb(200, 255, 200)
        Note right of Bob: Bob thinks
        Bob-->Alice: I am good thanks
        end
    ",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        let messages = &diagram.messages;
        assert_eq!(messages[1].line_type, LineType::RectStart);
        assert_eq!(text(&messages[1]), "rgb(200, 255, 200)");
        assert_eq!(messages[2].line_type, LineType::Note);
        assert_eq!(messages[3].line_type, LineType::DottedOpen);
        assert_eq!(messages[4].line_type, LineType::RectEnd);
    }

    #[test]
    fn should_allow_for_nested_rects() {
        let diagram = parsed(
            "
      sequenceDiagram
        Alice->Bob: Hello Bob, how are you?
        %% Comment
        rect rgb(200, 255, 200)
        rect rgb(0, 0, 0)
        Note right of Bob: Bob thinks
        end
        Bob-->Alice: I am good thanks
        end
    ",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        let messages = &diagram.messages;
        assert_eq!(messages[1].line_type, LineType::RectStart);
        assert_eq!(text(&messages[1]), "rgb(200, 255, 200)");
        assert_eq!(messages[2].line_type, LineType::RectStart);
        assert_eq!(text(&messages[2]), "rgb(0, 0, 0)");
        assert_eq!(messages[3].line_type, LineType::Note);
        assert_eq!(messages[4].line_type, LineType::RectEnd);
        assert_eq!(messages[5].line_type, LineType::DottedOpen);
        assert_eq!(messages[6].line_type, LineType::RectEnd);
    }

    #[test]
    fn should_handle_opt_statements() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?

%% Comment
Note right of Bob: Bob thinks
opt Perhaps a happy response

Bob-->Alice: I am good thanks!
end",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.messages.len(), 5);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[1].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_alt_statements() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?

%% Comment
Note right of Bob: Bob thinks
alt isWell

Bob-->Alice: I am good thanks!
else isSick
Bob-->Alice: Feel sick...
end",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(diagram.messages.len(), 7);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(diagram.messages[1].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_alt_statements_with_multiple_elses() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?

%% Comment
Note right of Bob: Bob thinks
alt isWell

Bob-->Alice: I am good thanks!
else isSick
Bob-->Alice: Feel sick...
else default
Bob-->Alice: :-)
end",
        );

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 9);
        assert_eq!(messages[1].from.as_deref(), Some("Bob"));
        assert_eq!(messages[2].line_type, LineType::AltStart);
        assert_eq!(messages[3].from.as_deref(), Some("Bob"));
        assert_eq!(messages[4].line_type, LineType::AltElse);
        assert_eq!(messages[5].from.as_deref(), Some("Bob"));
        assert_eq!(messages[6].line_type, LineType::AltElse);
        assert_eq!(messages[7].from.as_deref(), Some("Bob"));
        assert_eq!(messages[8].line_type, LineType::AltEnd);
    }

    #[test]
    fn should_handle_critical_statements_without_options() {
        let diagram = parsed(
            "
sequenceDiagram
    critical Establish a connection to the DB
        Service-->DB: connect
    end",
        );

        assert_eq!(description(&diagram, "Service"), Some("Service"));
        assert_eq!(description(&diagram, "DB"), Some("DB"));

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].line_type, LineType::CriticalStart);
        assert_eq!(messages[1].from.as_deref(), Some("Service"));
        assert_eq!(messages[2].line_type, LineType::CriticalEnd);
    }

    #[test]
    fn should_handle_critical_statements_with_options() {
        let diagram = parsed(
            "
sequenceDiagram
    critical Establish a connection to the DB
        Service-->DB: connect
    option Network timeout
        Service-->Service: Log error
    option Credentials rejected
        Service-->Service: Log different error
    end",
        );

        assert_eq!(description(&diagram, "Service"), Some("Service"));
        assert_eq!(description(&diagram, "DB"), Some("DB"));

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 7);
        assert_eq!(messages[0].line_type, LineType::CriticalStart);
        assert_eq!(messages[1].from.as_deref(), Some("Service"));
        assert_eq!(messages[2].line_type, LineType::CriticalOption);
        assert_eq!(messages[3].from.as_deref(), Some("Service"));
        assert_eq!(messages[4].line_type, LineType::CriticalOption);
        assert_eq!(messages[5].from.as_deref(), Some("Service"));
        assert_eq!(messages[6].line_type, LineType::CriticalEnd);
    }

    #[test]
    fn should_handle_break_statements() {
        let diagram = parsed(
            "
sequenceDiagram
    Consumer-->API: Book something
    API-->BookingService: Start booking process
    break when the booking process fails
        API-->Consumer: show failure
    end
    API-->BillingService: Start billing process",
        );

        assert_eq!(description(&diagram, "Consumer"), Some("Consumer"));
        assert_eq!(description(&diagram, "API"), Some("API"));

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 6);
        assert_eq!(messages[0].from.as_deref(), Some("Consumer"));
        assert_eq!(messages[1].from.as_deref(), Some("API"));
        assert_eq!(messages[2].line_type, LineType::BreakStart);
        assert_eq!(messages[3].from.as_deref(), Some("API"));
        assert_eq!(messages[4].line_type, LineType::BreakEnd);
        assert_eq!(messages[5].from.as_deref(), Some("API"));
    }

    #[test]
    fn should_handle_par_statements_a_sequence_diagram() {
        let diagram = parsed(
            "
sequenceDiagram
par Parallel one
Alice->>Bob: Hello Bob, how are you?
Bob-->>Alice: I am good thanks!
and Parallel two
Alice->>Bob: Are you OK?
Bob-->>Alice: Fine!
and Parallel three
Alice->>Bob: What do you think about it?
Bob-->>Alice: It's good!
end",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 10);
        assert_eq!(text(&messages[0]), "Parallel one");
        assert_eq!(messages[1].from.as_deref(), Some("Alice"));
        assert_eq!(messages[2].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn it_should_handle_par_over_statements() {
        let diagram = parsed(
            "
sequenceDiagram
par_over Parallel overlap
Alice ->> Bob: Message
Note left of Alice: Alice note
Note right of Bob: Bob note
end",
        );

        assert_eq!(description(&diagram, "Alice"), Some("Alice"));
        assert_eq!(description(&diagram, "Bob"), Some("Bob"));

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 5);
        assert_eq!(text(&messages[0]), "Parallel overlap");
        assert_eq!(messages[1].from.as_deref(), Some("Alice"));
        assert_eq!(messages[2].from.as_deref(), Some("Alice"));
        assert_eq!(messages[3].from.as_deref(), Some("Bob"));
    }

    #[test]
    fn should_handle_special_characters_in_signals() {
        let diagram = parsed("sequenceDiagram\nAlice->Bob: -:<>,;# comment");

        assert_eq!(text(&diagram.messages[0]), "-:<>,");
    }

    #[test]
    fn should_handle_special_characters_in_notes() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
Note right of Bob: -:<>,;# comment",
        );

        assert_eq!(text(&diagram.messages[1]), "-:<>,");
    }

    #[test]
    fn should_handle_special_characters_in_loop() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
loop -:<>,;# comment
Bob-->Alice: I am good thanks!
end",
        );

        assert_eq!(text(&diagram.messages[1]), "-:<>,");
    }

    #[test]
    fn should_handle_special_characters_in_opt() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
opt -:<>,;# comment
Bob-->Alice: I am good thanks!
end",
        );

        assert_eq!(text(&diagram.messages[1]), "-:<>,");
    }

    #[test]
    fn should_handle_special_characters_in_alt() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
alt -:<>,;# comment
Bob-->Alice: I am good thanks!
else ,<>:-#; comment
Bob-->Alice: I am good thanks!
end",
        );

        let messages = &diagram.messages;
        assert_eq!(text(&messages[1]), "-:<>,");
        assert_eq!(text(&messages[3]), ",<>:-");
    }

    #[test]
    fn should_handle_special_characters_in_par() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
par -:<>,;# comment
Bob-->Alice: I am good thanks!
and ,<>:-#; comment
Bob-->Alice: I am good thanks!
end",
        );

        let messages = &diagram.messages;
        assert_eq!(text(&messages[1]), "-:<>,");
        assert_eq!(text(&messages[3]), ",<>:-");
    }

    #[test]
    fn should_handle_no_label_loop() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
loop
Bob-->Alice: I am good thanks!
end",
        );

        let messages = &diagram.messages;
        assert_eq!(text(&messages[1]), "");
        assert_eq!(text(&messages[2]), "I am good thanks!");
    }

    #[test]
    fn should_handle_no_label_opt() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
opt # comment
Bob-->Alice: I am good thanks!
end",
        );

        let messages = &diagram.messages;
        assert_eq!(text(&messages[1]), "");
        assert_eq!(text(&messages[2]), "I am good thanks!");
    }

    #[test]
    fn should_handle_no_label_alt() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
alt;Bob-->Alice: I am good thanks!
else # comment
Bob-->Alice: I am good thanks!
end",
        );

        let messages = &diagram.messages;
        assert_eq!(text(&messages[1]), "");
        assert_eq!(text(&messages[2]), "I am good thanks!");
        assert_eq!(text(&messages[3]), "");
        assert_eq!(text(&messages[4]), "I am good thanks!");
    }

    #[test]
    fn should_handle_no_label_par() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->Bob: Hello Bob, how are you?
par;Bob-->Alice: I am good thanks!
and # comment
Bob-->Alice: I am good thanks!
end",
        );

        let messages = &diagram.messages;
        assert_eq!(text(&messages[1]), "");
        assert_eq!(text(&messages[2]), "I am good thanks!");
        assert_eq!(text(&messages[3]), "");
        assert_eq!(text(&messages[4]), "I am good thanks!");
    }

    /// `it.each(['Link', 'link', 'LINK'])` over a declared participant named by the keyword.
    fn declared_endpoint(id: &str) {
        let diagram = parsed(&format!(
            "\nsequenceDiagram\nparticipant A\nparticipant {id} as Channel\nA->>{id}: message\n{id}-->>A: reply\n"
        ));

        assert_eq!(actor_ids(&diagram), ["A", id]);
        assert_eq!(description(&diagram, id), Some("Channel"));
        assert_eq!(diagram.messages.len(), 2);
        assert_eq!(diagram.messages[0].from.as_deref(), Some("A"));
        assert_eq!(diagram.messages[0].to.as_deref(), Some(id));
        assert_eq!(text(&diagram.messages[0]), "message");
        assert_eq!(diagram.messages[1].from.as_deref(), Some(id));
        assert_eq!(diagram.messages[1].to.as_deref(), Some("A"));
        assert_eq!(text(&diagram.messages[1]), "reply");
    }

    /// `it.each(['Link', 'link', 'LINK'])` over an implicit participant named by the keyword.
    fn implicit_endpoint(id: &str) {
        let diagram = parsed(&format!(
            "\nsequenceDiagram\n{id}->>A: message\nA-->>{id}: reply\n"
        ));

        assert_eq!(actor_ids(&diagram), [id, "A"]);
        assert_eq!(diagram.messages.len(), 2);
        assert_eq!(diagram.messages[0].from.as_deref(), Some(id));
        assert_eq!(diagram.messages[0].to.as_deref(), Some("A"));
        assert_eq!(text(&diagram.messages[0]), "message");
        assert_eq!(diagram.messages[1].from.as_deref(), Some("A"));
        assert_eq!(diagram.messages[1].to.as_deref(), Some(id));
        assert_eq!(text(&diagram.messages[1]), "reply");
    }

    #[test]
    fn should_use_link_as_a_declared_message_endpoint() {
        declared_endpoint("Link");
    }

    #[test]
    fn should_use_link_as_a_declared_message_endpoint_2() {
        declared_endpoint("link");
    }

    #[test]
    fn should_use_link_as_a_declared_message_endpoint_3() {
        declared_endpoint("LINK");
    }

    #[test]
    fn should_use_link_as_an_implicit_message_endpoint() {
        implicit_endpoint("Link");
    }

    #[test]
    fn should_use_link_as_an_implicit_message_endpoint_2() {
        implicit_endpoint("link");
    }

    #[test]
    fn should_use_link_as_an_implicit_message_endpoint_3() {
        implicit_endpoint("LINK");
    }
}
