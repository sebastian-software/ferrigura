//! Port of `packages/mermaid/src/diagrams/sequence/sequenceDiagram.spec.js`, the DB-level blocks:
//! the actor-menu link, properties, box and creation and destruction tests of "when parsing a
//! sequenceDiagram" (`:1592-1782`, the `it.each` link table at `:1611-1617` included), and the
//! "sequence db class" block (`:2397-2908`), which parses participants with the extended syntax.
//!
//! The reference's `db.getActors()`, `getBoxes()`, `getCreatedActors()` and `getDestroyedActors()`
//! are the model's `actors`, `boxes`, `created_actors` and `destroyed_actors`. The
//! `Object.hasOwn(sequenceDb, …)` test checks the JavaScript object and is ignored.
//!
//! Test names follow the rules of `sequence_parse_spec.rs`. A title that starts with a number gets
//! a `case_` prefix (`1 should parse` is `case_1_should_parse`).

use std::collections::BTreeMap;

use ferrigura_model::sequence::{MessageText, SequenceActor, SequenceDiagram, SequenceMessage};
use ferrigura_model::value::Value;
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

fn map_text<'a>(map: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key).and_then(Value::as_str)
}

fn link<'a>(diagram: &'a SequenceDiagram, id: &str, key: &str) -> Option<&'a str> {
    map_text(&actor(diagram, id).links, key)
}

fn property<'a>(diagram: &'a SequenceDiagram, id: &str, key: &str) -> Option<&'a str> {
    map_text(&actor(diagram, id).properties, key)
}

/// `it.each(['Link', 'link', 'LINK'])` over the actor-menu statements of a declared participant.
fn attach_actor_menu_links(id: &str) {
    let diagram = parsed(&format!(
        "\nsequenceDiagram\nparticipant A\nparticipant {id} as Channel\nlink A: Help @ https://example.com/help\nlink {id}: Help @ https://example.com/channel\nlinks {id}: {{ \"Repo\": \"https://example.com/repo\" }}\n"
    ));

    assert_eq!(
        link(&diagram, "A", "Help"),
        Some("https://example.com/help")
    );
    assert_eq!(
        link(&diagram, id, "Help"),
        Some("https://example.com/channel")
    );
    assert_eq!(link(&diagram, id, "Repo"), Some("https://example.com/repo"));
    assert_eq!(description(&diagram, id), Some("Channel"));
}

mod sequence_diagram_actor_db_state {
    use super::*;

    #[test]
    fn should_attach_actor_menu_links_to_link() {
        attach_actor_menu_links("Link");
    }

    #[test]
    fn should_attach_actor_menu_links_to_link_2() {
        attach_actor_menu_links("link");
    }

    #[test]
    fn should_attach_actor_menu_links_to_link_3() {
        attach_actor_menu_links("LINK");
    }

    #[test]
    fn should_handle_links() {
        let diagram = parsed(
            "
sequenceDiagram
participant a as Alice
participant b as Bob
participant c as Charlie
links a: { \"Repo\": \"https://repo.contoso.com/\", \"Dashboard\": \"https://dashboard.contoso.com/\" }
links b: { \"Dashboard\": \"https://dashboard.contoso.com/\" }
links a: { \"On-Call\": \"https://oncall.contoso.com/?svc=alice\" }
link a: Endpoint @ https://alice.contoso.com
link a: Swagger @ https://swagger.contoso.com
link a: Tests @ https://tests.contoso.com/?svc=alice@contoso.com
",
        );

        assert_eq!(
            link(&diagram, "a", "Repo"),
            Some("https://repo.contoso.com/")
        );
        assert_eq!(link(&diagram, "b", "Repo"), None);
        assert_eq!(
            link(&diagram, "a", "Dashboard"),
            Some("https://dashboard.contoso.com/")
        );
        assert_eq!(
            link(&diagram, "b", "Dashboard"),
            Some("https://dashboard.contoso.com/")
        );
        assert_eq!(
            link(&diagram, "a", "On-Call"),
            Some("https://oncall.contoso.com/?svc=alice")
        );
        assert_eq!(link(&diagram, "c", "Dashboard"), None);
        assert_eq!(
            link(&diagram, "a", "Endpoint"),
            Some("https://alice.contoso.com")
        );
        assert_eq!(
            link(&diagram, "a", "Swagger"),
            Some("https://swagger.contoso.com")
        );
        assert_eq!(
            link(&diagram, "a", "Tests"),
            Some("https://tests.contoso.com/?svc=alice@contoso.com")
        );
    }

    #[test]
    fn should_handle_properties_experimental_use_with_caution() {
        // The syntax of `properties` is likely to change, as the spec's comment says.
        let diagram = parsed(
            "
sequenceDiagram
participant a as Alice
participant b as Bob
participant c as Charlie
properties a: {\"class\": \"internal-service-actor\", \"icon\": \"@clock\"}
properties b: {\"class\": \"external-service-actor\", \"icon\": \"@computer\"}
",
        );

        assert_eq!(
            property(&diagram, "a", "class"),
            Some("internal-service-actor")
        );
        assert_eq!(
            property(&diagram, "b", "class"),
            Some("external-service-actor")
        );
        assert_eq!(property(&diagram, "a", "icon"), Some("@clock"));
        assert_eq!(property(&diagram, "b", "icon"), Some("@computer"));
        assert_eq!(property(&diagram, "c", "class"), None);
    }

    #[test]
    fn should_handle_box() {
        let diagram = parsed(
            "
sequenceDiagram
box green Group 1
participant a as Alice
participant b as Bob
end
participant c as Charlie
links a: { \"Repo\": \"https://repo.contoso.com/\", \"Dashboard\": \"https://dashboard.contoso.com/\" }
links b: { \"Dashboard\": \"https://dashboard.contoso.com/\" }
links a: { \"On-Call\": \"https://oncall.contoso.com/?svc=alice\" }
link a: Endpoint @ https://alice.contoso.com
link a: Swagger @ https://swagger.contoso.com
link a: Tests @ https://tests.contoso.com/?svc=alice@contoso.com
",
        );

        let boxes = &diagram.boxes;
        assert_eq!(boxes[0].name.as_deref(), Some("Group 1"));
        assert_eq!(boxes[0].actor_keys, ["a", "b"]);
        assert_eq!(boxes[0].fill, "green");
    }

    #[test]
    fn should_handle_box_without_color() {
        let diagram = parsed(
            "
  sequenceDiagram
  box Group 1
  participant a as Alice
  participant b as Bob
  end
  participant c as Charlie
  links a: { \"Repo\": \"https://repo.contoso.com/\", \"Dashboard\": \"https://dashboard.contoso.com/\" }
  links b: { \"Dashboard\": \"https://dashboard.contoso.com/\" }
  links a: { \"On-Call\": \"https://oncall.contoso.com/?svc=alice\" }
  link a: Endpoint @ https://alice.contoso.com
  link a: Swagger @ https://swagger.contoso.com
  link a: Tests @ https://tests.contoso.com/?svc=alice@contoso.com
  ",
        );

        let boxes = &diagram.boxes;
        assert_eq!(boxes[0].name.as_deref(), Some("Group 1"));
        assert_eq!(boxes[0].actor_keys, ["a", "b"]);
        assert_eq!(boxes[0].fill, "transparent");
    }

    #[test]
    fn should_handle_box_without_description() {
        let diagram = parsed(
            "
  sequenceDiagram
  box aqua
  participant a as Alice
  participant b as Bob
  end
  participant c as Charlie
  links a: { \"Repo\": \"https://repo.contoso.com/\", \"Dashboard\": \"https://dashboard.contoso.com/\" }
  links b: { \"Dashboard\": \"https://dashboard.contoso.com/\" }
  links a: { \"On-Call\": \"https://oncall.contoso.com/?svc=alice\" }
  link a: Endpoint @ https://alice.contoso.com
  link a: Swagger @ https://swagger.contoso.com
  link a: Tests @ https://tests.contoso.com/?svc=alice@contoso.com
  ",
        );

        let boxes = &diagram.boxes;
        assert_eq!(boxes[0].name, None);
        assert_eq!(boxes[0].actor_keys, ["a", "b"]);
        assert_eq!(boxes[0].fill, "aqua");
    }

    #[test]
    fn should_handle_simple_actor_creation() {
        let diagram = parsed(
            "
  sequenceDiagram
  participant a as Alice
  a ->>b: Hello Bob?
  create participant c
  b-->>c: Hello c!
  c ->> b: Hello b?
  create actor d as Donald
  a ->> d: Hello Donald?
  ",
        );

        let c = actor(&diagram, "c");
        assert_eq!(c.name, "c");
        assert_eq!(description(&diagram, "c"), Some("c"));
        assert_eq!(actor_type(&diagram, "c"), Some("participant"));
        assert_eq!(diagram.created_actors.get("c"), Some(&1));

        let d = actor(&diagram, "d");
        assert_eq!(d.name, "d");
        assert_eq!(description(&diagram, "d"), Some("Donald"));
        assert_eq!(actor_type(&diagram, "d"), Some("actor"));
        assert_eq!(diagram.created_actors.get("d"), Some(&3));
    }

    #[test]
    fn should_handle_simple_actor_destruction() {
        let diagram = parsed(
            "
  sequenceDiagram
  participant a as Alice
  a ->>b: Hello Bob?
  destroy a
  b-->>a: Hello Alice!
  b ->> c: Where is Alice?
  destroy c
  b ->> c: Where are you?
  ",
        );

        assert_eq!(diagram.destroyed_actors.get("a"), Some(&1));
        assert_eq!(diagram.destroyed_actors.get("c"), Some(&3));
    }

    #[test]
    fn should_handle_the_creation_and_destruction_of_the_same_actor() {
        let diagram = parsed(
            "
  sequenceDiagram
  a ->>b: Hello Bob?
  create participant c
  b ->>c: Hello c!
  c ->> b: Hello b?
  destroy c
  b ->> c : Bye c !
  ",
        );

        assert_eq!(diagram.created_actors.get("c"), Some(&1));
        assert_eq!(diagram.destroyed_actors.get("c"), Some(&3));
    }
}

mod sequence_db_class {
    use super::*;

    // This is to ensure that functions used in sequence JISON are exposed as functions from
    // SequenceDB. The reference checks the own properties of a JavaScript object.
    #[test]
    #[ignore = "port: JavaScript API, not ported"]
    fn should_have_functions_used_in_sequence_jison_as_own_property() {}

    // A message with a trailing colon but no content (`Alice->>Bob:`) is a message with an empty
    // text.
    #[test]
    fn should_parse_a_message_with_a_trailing_colon_but_no_content() {
        let diagram = parsed(
            "
sequenceDiagram
Alice->>Bob:
Bob->>Alice:Got it!
",
        );

        let messages = &diagram.messages;
        assert_eq!(messages.len(), 2);
        assert_eq!(text(&messages[0]), "");
        assert_eq!(messages[0].from.as_deref(), Some("Alice"));
        assert_eq!(messages[0].to.as_deref(), Some("Bob"));
    }

    #[test]
    fn case_1_should_parse() {
        // The reference has no expectations here: the diagram must parse, with every arrow form
        // of the lexer's fixed table.
        parsed(
            "
    sequenceDiagram
    actor Bob
    actor Alice
        Bob -|\\ Alice: Hello Alice, how are you?
        Bob -|/ Alice: Hello Alice, how are you?
        Bob -// Alice: Hello Alice, how are you?
        Bob -\\\\ Alice: Hello Alice, how are you?
        
        Bob \\|- Alice: Hello Alice, how are you?
        Bob /|- Alice: Hello Alice, how are you?
        Bob //- Alice: Hello Alice, how are you?
        Bob \\\\- Alice: Hello Alice, how are you?        
    ",
        );
    }

    #[test]
    fn case_2_should_parse() {
        parsed(
            "
    sequenceDiagram
    actor Bob
    actor Alice
      Alice ()<<->>() Bob: hey?
    ",
        );
    }

    mod when_parsing_extended_participant_syntax {
        use super::*;

        #[test]
        fn should_parse_participants_with_different_quote_styles_and_whitespace() {
            let diagram = parsed(
                "
  sequenceDiagram
      participant Alice@{ \"type\" : \"database\" }
      participant Bob@{ \"type\" : \"database\" }
      participant Carl@{ type: \"database\" }
      participant David@{ \"type\" : 'database' }
      participant Eve@{ type: 'database' }
      participant Favela@{ \"type\" : \"database\"    }
      Bob->>+Alice: Hi Alice
      Alice->>+Bob: Hi Bob
    ",
            );

            for id in ["Alice", "Bob", "Carl", "David", "Eve", "Favela"] {
                assert_eq!(actor_type(&diagram, id), Some("database"), "{id}");
                assert_eq!(description(&diagram, id), Some(id), "{id}");
            }

            // Two messages and two activation messages.
            let messages = &diagram.messages;
            assert_eq!(messages.len(), 4);
            assert_eq!(messages[0].from.as_deref(), Some("Bob"));
            assert_eq!(messages[0].to.as_deref(), Some("Alice"));
            assert_eq!(text(&messages[0]), "Hi Alice");
            assert_eq!(messages[2].from.as_deref(), Some("Alice"));
            assert_eq!(messages[2].to.as_deref(), Some("Bob"));
            assert_eq!(text(&messages[2]), "Hi Bob");
        }

        #[test]
        fn should_parse_a_participant_config_object_separated_from_the_name_by_whitespace() {
            let diagram = parsed(
                "
  sequenceDiagram
      participant Gina@{ \"type\" : \"database\" }
      participant Harry @{ \"type\" : \"database\" }
    ",
            );

            assert_eq!(actor_type(&diagram, "Gina"), Some("database"));
            assert_eq!(description(&diagram, "Gina"), Some("Gina"));
            assert_eq!(actor_type(&diagram, "Harry"), Some("database"));
            assert_eq!(description(&diagram, "Harry"), Some("Harry"));
        }

        #[test]
        fn should_parse_mixed_participant_types_with_extended_syntax() {
            let diagram = parsed(
                "
    sequenceDiagram
        participant lead
        participant dsa@{ \"type\" : \"queue\" }
        API->>+Database: getUserb
        Database-->>-API: userb
        dsa --> Database: hello
",
            );

            assert_eq!(actor_type(&diagram, "lead"), Some("participant"));
            assert_eq!(description(&diagram, "lead"), Some("lead"));
            assert_eq!(actor_type(&diagram, "dsa"), Some("queue"));
            assert_eq!(description(&diagram, "dsa"), Some("dsa"));
            // The actors that messages declare implicitly.
            assert_eq!(actor_type(&diagram, "API"), Some("participant"));
            assert_eq!(description(&diagram, "API"), Some("API"));
            assert_eq!(actor_type(&diagram, "Database"), Some("participant"));
            assert_eq!(description(&diagram, "Database"), Some("Database"));

            // Three messages and two activation messages.
            let messages = &diagram.messages;
            assert_eq!(messages.len(), 5);
            assert_eq!(messages[0].from.as_deref(), Some("API"));
            assert_eq!(messages[0].to.as_deref(), Some("Database"));
            assert_eq!(text(&messages[0]), "getUserb");
            assert_eq!(messages[0].activate, Some(true));
            assert_eq!(messages[2].from.as_deref(), Some("Database"));
            assert_eq!(messages[2].to.as_deref(), Some("API"));
            assert_eq!(text(&messages[2]), "userb");
            assert_eq!(messages[4].from.as_deref(), Some("dsa"));
            assert_eq!(messages[4].to.as_deref(), Some("Database"));
            assert_eq!(text(&messages[4]), "hello");
        }

        #[test]
        fn should_parse_hyphenated_names_with_an_attached_config_object() {
            let diagram = parsed(
                "
    sequenceDiagram
        actor lead-actor@{ \"type\" : \"database\" }
        participant order-svc@{ \"type\" : \"queue\" }
        actor kebab-case@{ \"type\" : \"control\" } as Kebab-Cased
        participant web-ui@{ \"type\" : \"entity\" } as Web UI
    ",
            );

            assert_eq!(actor_type(&diagram, "lead-actor"), Some("database"));
            assert_eq!(description(&diagram, "lead-actor"), Some("lead-actor"));
            assert_eq!(actor_type(&diagram, "order-svc"), Some("queue"));
            assert_eq!(description(&diagram, "order-svc"), Some("order-svc"));
            assert_eq!(actor_type(&diagram, "kebab-case"), Some("control"));
            assert_eq!(description(&diagram, "kebab-case"), Some("Kebab-Cased"));
            assert_eq!(actor_type(&diagram, "web-ui"), Some("entity"));
            assert_eq!(description(&diagram, "web-ui"), Some("Web UI"));
        }

        #[test]
        fn should_fail_for_malformed_json_in_participant_definition() {
            let invalid = "
    sequenceDiagram
      participant D@{ \"type: \"entity\" }
      participant E@{ \"type\": \"dat
      abase }
  ";

            assert!(parse(invalid).is_err());
        }

        #[test]
        fn should_fail_for_missing_colon_separator() {
            let invalid = "
    sequenceDiagram
      participant C@{ \"type\" \"control\" }
      C ->> C: action
  ";

            assert!(parse(invalid).is_err());
        }

        #[test]
        fn should_fail_for_missing_closing_brace() {
            let invalid = "
    sequenceDiagram
      participant E@{ \"type\": \"entity\"
      E ->> E: process
  ";

            assert!(parse(invalid).is_err());
        }
    }

    mod participant_type_parsing {
        use super::*;

        #[test]
        fn should_parse_boundary_participant() {
            let diagram = parsed(
                "
          sequenceDiagram
          participant boundary@{ \"type\" : \"boundary\" }
          boundary->boundary: test
      ",
            );
            assert_eq!(actor_type(&diagram, "boundary"), Some("boundary"));
            assert_eq!(description(&diagram, "boundary"), Some("boundary"));
        }

        #[test]
        fn should_parse_control_participant() {
            let diagram = parsed(
                "
        sequenceDiagram
         participant C@{ \"type\" : \"control\" }
        C->C: test
        ",
            );
            assert_eq!(actor_type(&diagram, "C"), Some("control"));
            assert_eq!(description(&diagram, "C"), Some("C"));
        }

        #[test]
        fn should_parse_entity_participant() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant E@{ \"type\" : \"entity\" }
      E->E: test
      ",
            );
            assert_eq!(actor_type(&diagram, "E"), Some("entity"));
            assert_eq!(description(&diagram, "E"), Some("E"));
        }

        #[test]
        fn should_parse_database_participant() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant D@{ \"type\" : \"database\" }
      D->D: test
      ",
            );
            assert_eq!(actor_type(&diagram, "D"), Some("database"));
            assert_eq!(description(&diagram, "D"), Some("D"));
        }

        #[test]
        fn should_parse_collections_participant() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant L@{ \"type\" : \"collections\" }
      L->L: test
      ",
            );
            assert_eq!(actor_type(&diagram, "L"), Some("collections"));
            assert_eq!(description(&diagram, "L"), Some("L"));
        }

        #[test]
        fn should_parse_queue_participant() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant Q@{ \"type\" : \"queue\" }
      Q->Q: test 
      ",
            );
            assert_eq!(actor_type(&diagram, "Q"), Some("queue"));
            assert_eq!(description(&diagram, "Q"), Some("Q"));
        }
    }

    mod participant_type_parsing_2 {
        use super::*;

        #[test]
        fn should_parse_actor_participant() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant A@{ \"type\" : \"queue\" }
      A->A: test
      ",
            );
            assert_eq!(actor_type(&diagram, "A"), Some("queue"));
            assert_eq!(description(&diagram, "A"), Some("A"));
        }

        #[test]
        fn should_parse_participant_participant() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant P@{ \"type\" : \"database\" }
      P->P: test
      ",
            );
            assert_eq!(actor_type(&diagram, "P"), Some("database"));
            assert_eq!(description(&diagram, "P"), Some("P"));
        }

        #[test]
        fn should_parse_boundary_using_actor_keyword() {
            let diagram = parsed(
                "
      sequenceDiagram
        participant Alice@{ \"type\" : \"collections\" }
        participant Bob@{ \"type\" : \"control\" }
        Alice->>Bob: Hello Bob, how are you?
      ",
            );
            assert_eq!(actor_type(&diagram, "Alice"), Some("collections"));
            assert_eq!(actor_type(&diagram, "Bob"), Some("control"));
            assert_eq!(description(&diagram, "Bob"), Some("Bob"));
        }

        #[test]
        fn should_parse_control_using_participant_keyword() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant C@{ \"type\" : \"control\" }
      C->C: test
      ",
            );
            assert_eq!(actor_type(&diagram, "C"), Some("control"));
            assert_eq!(description(&diagram, "C"), Some("C"));
        }

        #[test]
        fn should_parse_entity_using_actor_keyword() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant E@{ \"type\" : \"entity\" }
      E->E: test
      ",
            );
            assert_eq!(actor_type(&diagram, "E"), Some("entity"));
            assert_eq!(description(&diagram, "E"), Some("E"));
        }

        #[test]
        fn should_handle_fail_parsing_when_alias_token_causes_conflicts_in_participant_definition()
        {
            assert!(
                parse(
                    "
        sequenceDiagram
        participant SAS MyServiceWithMoreThan20Chars <br> service decription
       "
                )
                .is_err()
            );
        }

        #[test]
        fn should_not_hang_when_as_is_used_without_a_space_before_the_alias_text() {
            // The reference checks that the error is not a lexer failure. Only the error is kept.
            assert!(
                parse(
                    "
          sequenceDiagram
          participant X_AutoPublishable asAAAAAAAAAAAAA:AAAAAAAAAAAAA
        "
                )
                .is_err()
            );
        }

        #[test]
        fn should_parse_participant_with_stereotype_and_alias() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant Alice@{ \"type\" : \"boundary\" } as Public API
      participant Bob@{ \"type\" : \"control\" } as Controller
      Alice->>Bob: Request
      Bob-->>Alice: Response
      ",
            );
            assert_eq!(actor_type(&diagram, "Alice"), Some("boundary"));
            assert_eq!(description(&diagram, "Alice"), Some("Public API"));
            assert_eq!(actor_type(&diagram, "Bob"), Some("control"));
            assert_eq!(description(&diagram, "Bob"), Some("Controller"));
        }

        #[test]
        fn should_parse_actor_with_stereotype_and_alias() {
            let diagram = parsed(
                "
      sequenceDiagram
      actor A@{ \"type\" : \"database\" } AS Database Server
      actor B@{ \"type\" : \"queue\" } as Message Queue
      A->>B: Send message
      ",
            );
            assert_eq!(actor_type(&diagram, "A"), Some("database"));
            assert_eq!(description(&diagram, "A"), Some("Database Server"));
            assert_eq!(actor_type(&diagram, "B"), Some("queue"));
            assert_eq!(description(&diagram, "B"), Some("Message Queue"));
        }

        #[test]
        fn should_parse_participant_with_stereotype_and_simple_alias() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant API@{ \"type\" : \"boundary\" } AS Public API
      API->>API: test
      ",
            );
            assert_eq!(actor_type(&diagram, "API"), Some("boundary"));
            assert_eq!(description(&diagram, "API"), Some("Public API"));
        }

        #[test]
        fn should_parse_participant_with_inline_alias_in_config_object() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant API@{ \"type\" : \"boundary\", \"alias\": \"Public API\" }
      participant Auth@{ \"type\" : \"control\", \"alias\": \"Auth Controller\" }
      API->>Auth: Request
      Auth-->>API: Response
      ",
            );
            assert_eq!(actor_type(&diagram, "API"), Some("boundary"));
            assert_eq!(description(&diagram, "API"), Some("Public API"));
            assert_eq!(actor_type(&diagram, "Auth"), Some("control"));
            assert_eq!(description(&diagram, "Auth"), Some("Auth Controller"));
        }

        #[test]
        fn should_parse_actor_with_inline_alias_in_config_object() {
            let diagram = parsed(
                "
      sequenceDiagram
      actor U@{ \"type\" : \"actor\", \"alias\": \"End User\" }
      actor DB@{ \"type\" : \"database\", \"alias\": \"User Database\" }
      U->>DB: Query
      DB-->>U: Result
      ",
            );
            assert_eq!(actor_type(&diagram, "U"), Some("actor"));
            assert_eq!(description(&diagram, "U"), Some("End User"));
            assert_eq!(actor_type(&diagram, "DB"), Some("database"));
            assert_eq!(description(&diagram, "DB"), Some("User Database"));
        }

        #[test]
        fn should_prioritize_external_alias_over_inline_alias() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant API@{ \"type\" : \"boundary\", \"alias\": \"Internal Name\" } as External Name
      API->>API: test
      ",
            );
            assert_eq!(actor_type(&diagram, "API"), Some("boundary"));
            assert_eq!(description(&diagram, "API"), Some("External Name"));
        }

        #[test]
        fn should_handle_participant_with_only_inline_alias_no_type() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant API@{ \"alias\": \"Public API\" }
      API->>API: test
      ",
            );
            assert_eq!(description(&diagram, "API"), Some("Public API"));
        }

        #[test]
        fn should_handle_mixed_inline_and_external_alias_syntax() {
            let diagram = parsed(
                "
      sequenceDiagram
      participant A@{ \"type\" : \"boundary\", \"alias\": \"Service A\" }
      participant B@{ \"type\" : \"control\" } as Service B
      participant C@{ \"type\" : \"database\" }
      A->>B: Request
      B->>C: Query
      ",
            );
            assert_eq!(actor_type(&diagram, "A"), Some("boundary"));
            assert_eq!(description(&diagram, "A"), Some("Service A"));
            assert_eq!(actor_type(&diagram, "B"), Some("control"));
            assert_eq!(description(&diagram, "B"), Some("Service B"));
            assert_eq!(actor_type(&diagram, "C"), Some("database"));
            assert_eq!(description(&diagram, "C"), Some("C"));
        }
    }
}
