//! Port of `packages/mermaid/src/diagrams/er/parser/erDiagram.spec.js`, without the generated
//! character cases (`er_characters_spec.rs`). Test names follow the spec's titles in snake_case; a
//! title that repeats after normalising gets `_2`, `_3`, … in source order.
//!
//! The spec's `beforeEach` (`clear`) is the fresh DB of each parse here. The `expect(…).toThrow()`
//! checks are ported as the reference runs them: the parse must fail. Some of the reference's
//! throw-checks refer to a `name` that is not in scope there, so they pass whatever the parse does;
//! the port asserts the failure the spec means. The `TODO` tests have empty bodies in the reference.

use ferrigura_model::er::{Cardinality, ErDiagram, ErEntity, ErRelationship, Identification};
use ferrigura_syntax::er::parse_grammar;

fn parse(text: &str) -> ErDiagram {
    parse_grammar(text).unwrap_or_else(|error| panic!("{text:?} does not parse: {error}"))
}

fn fails(text: &str) -> bool {
    parse_grammar(text).is_err()
}

fn entity<'a>(diagram: &'a ErDiagram, name: &str) -> &'a ErEntity {
    diagram
        .entities
        .iter()
        .find(|entity| entity.name == name)
        .unwrap_or_else(|| panic!("no entity {name}"))
}

fn has_entity(diagram: &ErDiagram, name: &str) -> bool {
    diagram.entities.iter().any(|entity| entity.name == name)
}

fn relationship(diagram: &ErDiagram) -> &ErRelationship {
    diagram
        .relationships
        .first()
        .unwrap_or_else(|| panic!("no relationship"))
}

mod when_parsing_er_diagram_it {
    use super::*;

    #[test]
    fn should_allow_stand_alone_entities_with_no_relationships() {
        let diagram = parse("erDiagram\nISLAND\nMAINLAND");

        assert_eq!(diagram.entities.len(), 2);
        assert_eq!(diagram.relationships.len(), 0);
    }

    mod entity_name {
        use super::*;

        #[test]
        fn cannot_be_empty_quotes() {
            assert!(fails("erDiagram\n \"\"\n"));
        }

        #[test]
        fn cannot_contain_percent_because_it_interferes_with_parsing_comments() {
            assert!(fails("erDiagram\n \"Blo%rf\"\n"));
        }

        #[test]
        fn cannot_contain_backslash_because_it_could_start_and_escape_code() {
            assert!(fails("erDiagram\n \"Blo\\rf\"\n"));
        }

        #[test]
        fn cannot_newline_backspace_or_vertical_characters() {
            for bad in ['\n', '\r', '\u{8}', '\u{b}'] {
                let name = format!("Blo{bad}rf");
                let text = format!("erDiagram\n \"{name}\"\n");
                // The reference's `toThrow` passes when the parse fails, or when its inner check
                // finds the entity, which only a successful parse can do.
                assert!(fails(&text) || has_entity(&parse(&text), &name), "{text:?}");
            }
        }

        #[test]
        #[ignore = "port: skipped upstream (it.skip): jison cannot handle non-english letters"]
        fn skipped_test_can_contain_accented_letters() {
            let name = "DUCK-àáâäæãåā";
            let diagram = parse(&format!("erDiagram\n{name}\n"));
            assert!(has_entity(&diagram, name));
        }

        #[test]
        fn can_contain_hyphens_and_underscore_without_needing_quotes() {
            let name = "DUCK-BILLED_PLATYPUS";
            assert!(has_entity(&parse(&format!("erDiagram\n{name}\n")), name));
        }

        #[test]
        fn can_have_an_alias() {
            let diagram = parse("erDiagram\nfoo[\"bar\"]\n");

            assert!(has_entity(&diagram, "foo"));
            assert_eq!(entity(&diagram, "foo").alias, "bar");
        }

        #[test]
        fn can_have_an_alias_even_if_the_relationship_is_defined_before_buzz() {
            let diagram = parse("erDiagram\nfoo ||--o| bar : rel\nbuzz foo[\"batman\"]\n");

            assert!(has_entity(&diagram, "foo"));
            assert!(has_entity(&diagram, "bar"));
            assert_eq!(entity(&diagram, "foo").alias, "batman");
            assert_eq!(entity(&diagram, "bar").alias, "");
        }

        #[test]
        fn can_have_an_alias_even_if_the_relationship_is_defined_after_buzz() {
            let diagram = parse("erDiagram\nbuzz foo[\"batman\"]\nfoo ||--o| bar : rel\n");

            assert!(has_entity(&diagram, "foo"));
            assert!(has_entity(&diagram, "bar"));
            assert_eq!(entity(&diagram, "foo").alias, "batman");
            assert_eq!(entity(&diagram, "bar").alias, "");
        }

        #[test]
        fn can_start_with_an_underscore() {
            assert!(has_entity(&parse("erDiagram\n_foo\n"), "_foo"));
        }
    }

    mod attribute_name {
        use super::*;

        #[test]
        fn should_allow_alphanumeric_characters_dashes_underscores_and_brackets_not_leading_chars()
        {
            let diagram = parse(
                "erDiagram\nBOOK {\nstring myBookTitle\nstring MYBOOKSUBTITLE_1\nstring author-ref[name](1)\n}",
            );

            assert_eq!(diagram.entities.len(), 1);
            let attributes = &entity(&diagram, "BOOK").attributes;
            assert_eq!(attributes.len(), 3);
            assert_eq!(attributes[0].name, "myBookTitle");
            assert_eq!(attributes[1].name, "MYBOOKSUBTITLE_1");
            assert_eq!(attributes[2].name, "author-ref[name](1)");
        }

        #[test]
        fn should_allow_asterisk_at_the_start_of_attribute_name() {
            let diagram = parse("erDiagram\nBOOK{\nstring *title}");

            assert_eq!(diagram.entities.len(), 1);
            assert_eq!(entity(&diagram, "BOOK").attributes.len(), 1);
        }

        #[test]
        fn should_allow_asterisks_at_the_start_of_attribute_declared_with_type_and_name() {
            let diagram = parse("erDiagram\nBOOK {\nid *the_Primary_Key}");

            assert_eq!(diagram.entities.len(), 1);
            assert_eq!(entity(&diagram, "BOOK").attributes.len(), 1);
        }

        #[test]
        fn should_allow_commas_within_an_attribute_type() {
            let diagram = parse("erDiagram\nBUILDING {\npublic.geometry(point,4326) location}");

            assert_eq!(diagram.entities.len(), 1);
            let attributes = &entity(&diagram, "BUILDING").attributes;
            assert_eq!(attributes.len(), 1);
            assert_eq!(attributes[0].attr_type, "public.geometry(point,4326)");
            assert_eq!(attributes[0].name, "location");
        }

        #[test]
        fn should_not_allow_leading_numbers_dashes_or_brackets() {
            for leading in "0-[]()".chars() {
                let text = format!("erDiagram\nBOOK {{\nstring {leading}author\n}}");
                assert!(fails(&text), "{text:?}");
            }
        }

        #[test]
        fn should_allow_special_characters_in_a_backtick_escaped_attribute_name() {
            let diagram = parse("erDiagram\nHOTEL {\nstring `geo.accuracy`\n}");

            assert_eq!(diagram.entities.len(), 1);
            let attributes = &entity(&diagram, "HOTEL").attributes;
            assert_eq!(attributes.len(), 1);
            assert_eq!(attributes[0].attr_type, "string");
            assert_eq!(attributes[0].name, "geo.accuracy");
        }

        #[test]
        fn should_allow_a_backtick_escaped_attribute_name_together_with_a_key_and_comment() {
            let diagram = parse("erDiagram\nHOTEL {\nstring `geo.accuracy` PK \"the comment\"\n}");

            assert_eq!(diagram.entities.len(), 1);
            let attributes = &entity(&diagram, "HOTEL").attributes;
            assert_eq!(attributes.len(), 1);
            assert_eq!(attributes[0].name, "geo.accuracy");
            assert!(attributes[0].keys.contains(&"PK".to_owned()));
            assert_eq!(attributes[0].comment, "the comment");
        }

        #[test]
        fn should_allow_a_backtick_escaped_attribute_type() {
            let diagram = parse("erDiagram\nHOTEL {\n`my.type` address\n}");

            assert_eq!(diagram.entities.len(), 1);
            let attributes = &entity(&diagram, "HOTEL").attributes;
            assert_eq!(attributes.len(), 1);
            assert_eq!(attributes[0].attr_type, "my.type");
            assert_eq!(attributes[0].name, "address");
        }

        #[test]
        fn should_allow_a_question_mark_suffix_on_an_attribute_type() {
            let diagram = parse("erDiagram\nPERSON {\nstring? alias\n}");

            assert_eq!(diagram.entities.len(), 1);
            let attributes = &entity(&diagram, "PERSON").attributes;
            assert_eq!(attributes.len(), 1);
            assert_eq!(attributes[0].attr_type, "string?");
            assert_eq!(attributes[0].name, "alias");
        }

        #[test]
        fn should_allow_a_question_mark_suffix_on_an_attribute_type_with_a_key_and_comment() {
            let diagram =
                parse("erDiagram\nPERSON {\nstring? alias UK \"optional display name\"\n}");

            assert_eq!(diagram.entities.len(), 1);
            let attributes = &entity(&diagram, "PERSON").attributes;
            assert_eq!(attributes.len(), 1);
            assert_eq!(attributes[0].attr_type, "string?");
            assert_eq!(attributes[0].keys, ["UK"]);
            assert_eq!(attributes[0].comment, "optional display name");
        }
    }

    #[test]
    fn should_allow_an_entity_with_a_single_attribute_to_be_defined() {
        let diagram = parse("erDiagram\nBOOK {\nstring title\n}");

        assert_eq!(diagram.entities.len(), 1);
        assert_eq!(entity(&diagram, "BOOK").attributes.len(), 1);
    }

    #[test]
    fn should_allow_an_entity_with_a_single_attribute_to_be_defined_with_a_key() {
        let diagram = parse("erDiagram\nBOOK {\nstring title PK\n}");

        assert_eq!(diagram.entities.len(), 1);
        assert_eq!(entity(&diagram, "BOOK").attributes.len(), 1);
    }

    #[test]
    fn should_allow_an_entity_with_a_single_attribute_to_be_defined_with_a_comment() {
        let diagram = parse("erDiagram\nBOOK {\nstring title \"comment\"\n}");

        assert_eq!(diagram.entities.len(), 1);
        let attributes = &entity(&diagram, "BOOK").attributes;
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].comment, "comment");
    }

    #[test]
    fn should_allow_an_entity_with_a_single_attribute_to_be_defined_with_a_key_and_a_comment() {
        let diagram = parse("erDiagram\nBOOK {\nstring title PK \"comment\"\n}");

        assert_eq!(diagram.entities.len(), 1);
        assert_eq!(entity(&diagram, "BOOK").attributes.len(), 1);
    }

    #[test]
    fn should_allow_an_entity_with_attribute_starting_with_fk_pk_or_uk_and_a_comment() {
        let diagram = parse(
            "erDiagram\nBOOK {\nint fk_title FK \nstring pk_author PK\nstring uk_address UK\nfloat pk_price PK \"comment\"\n}",
        );

        assert_eq!(entity(&diagram, "BOOK").attributes.len(), 4);
    }

    #[test]
    fn should_allow_an_entity_with_attributes_that_have_many_constraints_and_comments() {
        let diagram = parse(
            "erDiagram\nCUSTOMER {\nint customer_number PK, FK \"comment1\"\ndatetime customer_status_start_datetime PK,UK, FK\ndatetime customer_status_end_datetime PK , UK \"comment3\"\nstring customer_firstname\nstring customer_lastname \"comment5\"\n}",
        );

        let attributes = &entity(&diagram, "CUSTOMER").attributes;
        assert_eq!(attributes[0].keys, ["PK", "FK"]);
        assert_eq!(attributes[0].comment, "comment1");
        assert_eq!(attributes[1].keys, ["PK", "UK", "FK"]);
        assert_eq!(attributes[2].keys, ["PK", "UK"]);
        assert_eq!(attributes[2].comment, "comment3");
        assert!(attributes[3].keys.is_empty());
        assert!(attributes[4].keys.is_empty());
        assert_eq!(attributes[4].comment, "comment5");
    }

    #[test]
    fn should_allow_an_entity_with_attribute_that_has_a_generic_type() {
        let diagram = parse(
            "erDiagram\nBOOK {\ntype~T~ type\noption~T~ readable \"comment\"\nstring id PK\n}",
        );

        assert_eq!(diagram.entities.len(), 1);
        assert_eq!(entity(&diagram, "BOOK").attributes.len(), 3);
    }

    #[test]
    fn should_allow_an_entity_with_attribute_that_is_an_array() {
        let diagram =
            parse("erDiagram\nBOOK {\nstring[] readers FK \"comment\"\nstring[] authors FK\n}");

        assert_eq!(diagram.entities.len(), 1);
        assert_eq!(entity(&diagram, "BOOK").attributes.len(), 2);
    }

    #[test]
    fn should_allow_an_entity_with_attribute_that_is_a_limited_length_string() {
        let diagram = parse(
            "erDiagram\nBOOK {\ncharacter(10) isbn FK\nvarchar(5) postal_code \"Five digits\"\n}",
        );

        assert_eq!(diagram.entities.len(), 1);
        let attributes = &entity(&diagram, "BOOK").attributes;
        assert_eq!(attributes.len(), 2);
        assert_eq!(attributes[0].attr_type, "character(10)");
        assert_eq!(attributes[1].attr_type, "varchar(5)");
    }

    #[test]
    fn should_allow_an_entity_with_multiple_attributes_to_be_defined() {
        let diagram = parse("erDiagram\nBOOK {\nstring title\nstring author\nfloat price\n}");

        assert_eq!(entity(&diagram, "BOOK").attributes.len(), 3);
    }

    #[test]
    fn should_allow_attribute_definitions_to_be_split_into_multiple_blocks() {
        let diagram =
            parse("erDiagram\nBOOK {\nstring title\n}\nBOOK {\nstring author\nfloat price\n}");

        assert_eq!(entity(&diagram, "BOOK").attributes.len(), 3);
    }

    #[test]
    fn should_allow_an_empty_attribute_block() {
        let diagram = parse("erDiagram\nBOOK {}");

        assert!(has_entity(&diagram, "BOOK"));
        assert!(entity(&diagram, "BOOK").attributes.is_empty());
    }

    #[test]
    fn should_allow_an_attribute_block_to_start_immediately_after_the_entity_name() {
        let diagram = parse("erDiagram\nBOOK{}");

        assert!(has_entity(&diagram, "BOOK"));
        assert!(entity(&diagram, "BOOK").attributes.is_empty());
    }

    #[test]
    fn should_allow_an_attribute_block_to_be_separated_from_the_entity_name_by_spaces() {
        let diagram = parse("erDiagram\nBOOK         {}");

        assert!(has_entity(&diagram, "BOOK"));
        assert!(entity(&diagram, "BOOK").attributes.is_empty());
    }

    #[test]
    fn should_allow_whitespace_before_and_after_attribute_definitions() {
        let diagram = parse("erDiagram\nBOOK {\n  \n\n  string title\n\n  \n}");

        assert_eq!(diagram.entities.len(), 1);
        assert_eq!(entity(&diagram, "BOOK").attributes.len(), 1);
    }

    #[test]
    fn should_allow_no_whitespace_before_and_after_attribute_definitions() {
        let diagram = parse("erDiagram\nBOOK{string title}");

        assert_eq!(diagram.entities.len(), 1);
        assert_eq!(entity(&diagram, "BOOK").attributes.len(), 1);
    }

    #[test]
    fn should_associate_two_entities_correctly() {
        let diagram = parse("erDiagram\nCAR ||--o{ DRIVER : \"insured for\"");

        assert!(has_entity(&diagram, "CAR"));
        assert!(has_entity(&diagram, "DRIVER"));
        assert_eq!(diagram.relationships.len(), 1);
        let spec = relationship(&diagram).rel_spec;
        assert_eq!(spec.card_a, Cardinality::ZeroOrMore);
        assert_eq!(spec.card_b, Cardinality::OnlyOne);
        assert_eq!(spec.rel_type, Identification::Identifying);
    }

    #[test]
    fn should_not_create_duplicate_entities() {
        let diagram =
            parse("erDiagram\nCAR ||--o{ DRIVER : \"insured for\"\nDRIVER ||--|| LICENSE : has");

        assert_eq!(diagram.entities.len(), 3);
    }

    #[test]
    fn should_create_the_role_specified() {
        let diagram = parse("erDiagram\nTEACHER }o--o{ STUDENT : \"is teacher of\"");

        assert_eq!(relationship(&diagram).role_a, "is teacher of");
    }

    #[test]
    fn should_allow_recursive_relationships() {
        let diagram = parse("erDiagram\nNODE ||--o{ NODE : \"leads to\"");

        assert_eq!(diagram.entities.len(), 1);
    }

    mod accessible_title_and_description {
        use super::*;

        #[test]
        fn should_allow_for_a_accessibility_title_and_description_accdescr() {
            let diagram = parse(
                "erDiagram\n      accTitle: graph title\n      accDescr: this graph is about stuff\n      TEACHER }o--o{ STUDENT : \"is teacher of\"",
            );

            assert_eq!(diagram.acc_title, "graph title");
            assert_eq!(diagram.acc_descr, "this graph is about stuff");
        }

        #[test]
        fn parses_a_multi_line_description_accdescr() {
            let diagram = parse(
                "erDiagram\n      accTitle: graph title\n      accDescr { this graph is\n        about\n        stuff\n        }\n\n      TEACHER }o--o{ STUDENT : \"is teacher of\"",
            );

            assert_eq!(diagram.acc_title, "graph title");
            assert_eq!(diagram.acc_descr, "this graph is\nabout\nstuff");
        }
    }

    #[test]
    fn should_allow_more_than_one_relationship_between_the_same_two_entities() {
        let diagram = parse(
            "erDiagram\nCAR ||--o{ PERSON : \"insured for\"\nCAR }o--|| PERSON : \"owned by\"",
        );

        assert_eq!(diagram.entities.len(), 2);
        assert_eq!(diagram.relationships.len(), 2);
    }

    #[test]
    #[ignore = "port: empty body in the reference (TODO)"]
    fn should_limit_the_number_of_relationships_between_the_same_two_entities() {}

    #[test]
    #[ignore = "port: empty body in the reference (TODO)"]
    fn should_not_allow_multiple_relationships_between_the_same_two_entities_unless_the_roles_are_different()
     {
    }

    /// Checks the two cardinalities of `text`, a relationship between two entities.
    fn assert_cardinalities(text: &str, card_a: Cardinality, card_b: Cardinality) {
        let diagram = parse(text);

        assert_eq!(diagram.entities.len(), 2);
        assert_eq!(diagram.relationships.len(), 1);
        let spec = relationship(&diagram).rel_spec;
        assert_eq!(spec.card_a, card_a, "{text:?}");
        assert_eq!(spec.card_b, card_b, "{text:?}");
    }

    #[test]
    fn should_handle_only_one_to_one_or_more_relationships() {
        assert_cardinalities(
            "erDiagram\nA ||--|{ B : has",
            Cardinality::OneOrMore,
            Cardinality::OnlyOne,
        );
    }

    #[test]
    fn should_handle_only_one_to_zero_or_more_relationships() {
        assert_cardinalities(
            "erDiagram\nA ||..o{ B : has",
            Cardinality::ZeroOrMore,
            Cardinality::OnlyOne,
        );
    }

    #[test]
    fn should_handle_zero_or_one_to_zero_or_more_relationships() {
        assert_cardinalities(
            "erDiagram\nA |o..o{ B : has",
            Cardinality::ZeroOrMore,
            Cardinality::ZeroOrOne,
        );
    }

    #[test]
    fn should_handle_zero_or_one_to_one_or_more_relationships() {
        assert_cardinalities(
            "erDiagram\nA |o--|{ B : has",
            Cardinality::OneOrMore,
            Cardinality::ZeroOrOne,
        );
    }

    #[test]
    fn should_handle_one_or_more_to_only_one_relationships() {
        assert_cardinalities(
            "erDiagram\nA }|--|| B : has",
            Cardinality::OnlyOne,
            Cardinality::OneOrMore,
        );
    }

    #[test]
    fn should_handle_zero_or_more_to_only_one_relationships() {
        assert_cardinalities(
            "erDiagram\nA }o--|| B : has",
            Cardinality::OnlyOne,
            Cardinality::ZeroOrMore,
        );
    }

    #[test]
    fn should_handle_zero_or_more_to_zero_or_one_relationships() {
        assert_cardinalities(
            "erDiagram\nA }o..o| B : has",
            Cardinality::ZeroOrOne,
            Cardinality::ZeroOrMore,
        );
    }

    #[test]
    fn should_handle_one_or_more_to_zero_or_one_relationships() {
        assert_cardinalities(
            "erDiagram\nA }|..o| B : has",
            Cardinality::ZeroOrOne,
            Cardinality::OneOrMore,
        );
    }

    #[test]
    fn should_handle_zero_or_one_to_only_one_relationships() {
        assert_cardinalities(
            "erDiagram\nA |o..|| B : has",
            Cardinality::OnlyOne,
            Cardinality::ZeroOrOne,
        );
    }

    #[test]
    fn should_handle_only_one_to_only_one_relationships() {
        assert_cardinalities(
            "erDiagram\nA ||..|| B : has",
            Cardinality::OnlyOne,
            Cardinality::OnlyOne,
        );
    }

    #[test]
    fn should_handle_only_one_to_zero_or_one_relationships() {
        assert_cardinalities(
            "erDiagram\nA ||--o| B : has",
            Cardinality::ZeroOrOne,
            Cardinality::OnlyOne,
        );
    }

    #[test]
    fn should_handle_zero_or_one_to_zero_or_one_relationships() {
        assert_cardinalities(
            "erDiagram\nA |o..o| B : has",
            Cardinality::ZeroOrOne,
            Cardinality::ZeroOrOne,
        );
    }

    #[test]
    fn should_handle_zero_or_more_to_zero_or_more_relationships() {
        assert_cardinalities(
            "erDiagram\nA }o--o{ B : has",
            Cardinality::ZeroOrMore,
            Cardinality::ZeroOrMore,
        );
    }

    #[test]
    fn should_handle_one_or_more_to_one_or_more_relationships() {
        assert_cardinalities(
            "erDiagram\nA }|..|{ B : has",
            Cardinality::OneOrMore,
            Cardinality::OneOrMore,
        );
    }

    #[test]
    fn should_handle_zero_or_more_to_one_or_more_relationships() {
        assert_cardinalities(
            "erDiagram\nA }o--|{ B : has",
            Cardinality::OneOrMore,
            Cardinality::ZeroOrMore,
        );
    }

    #[test]
    fn should_handle_one_or_more_to_zero_or_more_relationships() {
        assert_cardinalities(
            "erDiagram\nA }|..o{ B : has",
            Cardinality::ZeroOrMore,
            Cardinality::OneOrMore,
        );
    }

    #[test]
    fn should_handle_zero_or_one_to_zero_or_more_relationships_aliases_one_or_zero_and_zero_or_many()
     {
        assert_cardinalities(
            "erDiagram\nA one or zero to many B : has",
            Cardinality::ZeroOrMore,
            Cardinality::ZeroOrOne,
        );
    }

    #[test]
    fn should_handle_one_or_more_to_zero_or_one_relationships_aliases_one_or_many_and_zero_or_one()
    {
        assert_cardinalities(
            "erDiagram\nA one or many optionally to zero or one B : has",
            Cardinality::ZeroOrOne,
            Cardinality::OneOrMore,
        );
    }

    #[test]
    fn should_handle_zero_or_more_to_zero_or_more_relationships_aliases_zero_or_more_and_zero_or_many()
     {
        assert_cardinalities(
            "erDiagram\nA zero or more to zero or many B : has",
            Cardinality::ZeroOrMore,
            Cardinality::ZeroOrMore,
        );
    }

    #[test]
    fn should_handle_zero_or_more_to_one_or_more_relationships_aliases_many_0_and_many_1() {
        assert_cardinalities(
            "erDiagram\nA many(0) to many(1) B : has",
            Cardinality::OneOrMore,
            Cardinality::ZeroOrMore,
        );
    }

    #[test]
    fn should_handle_zero_or_more_to_only_one_relationships_aliases_many_0_and_many_1() {
        assert_cardinalities(
            "erDiagram\nA many optionally to one B : has",
            Cardinality::OnlyOne,
            Cardinality::ZeroOrMore,
        );
    }

    #[test]
    fn should_handle_only_one_to_only_one_relationships_aliases_only_one_and_1_plus() {
        assert_cardinalities(
            "erDiagram\nA only one optionally to 1+ B : has",
            Cardinality::OneOrMore,
            Cardinality::OnlyOne,
        );
    }

    #[test]
    fn should_handle_zero_or_more_to_only_one_relationships_aliases_0_plus_and_1() {
        assert_cardinalities(
            "erDiagram\nA 0+ optionally to 1 B : has",
            Cardinality::OnlyOne,
            Cardinality::ZeroOrMore,
        );
    }

    #[test]
    fn should_handle_1_alias_directly_followed_by_identifying_relationship_operator() {
        let diagram = parse("erDiagram\nCUSTOMER 1--one or more DELIVERY-ADDRESS : has");

        assert_eq!(diagram.entities.len(), 2);
        assert_eq!(diagram.relationships.len(), 1);
        let spec = relationship(&diagram).rel_spec;
        assert_eq!(spec.card_a, Cardinality::OneOrMore);
        assert_eq!(spec.card_b, Cardinality::OnlyOne);
        assert_eq!(spec.rel_type, Identification::Identifying);
    }

    #[test]
    fn should_handle_1_alias_directly_followed_by_non_identifying_relationship_operator() {
        let diagram = parse("erDiagram\nCUSTOMER 1..one or more DELIVERY-ADDRESS : has");

        assert_eq!(diagram.entities.len(), 2);
        assert_eq!(diagram.relationships.len(), 1);
        let spec = relationship(&diagram).rel_spec;
        assert_eq!(spec.card_a, Cardinality::OneOrMore);
        assert_eq!(spec.card_b, Cardinality::OnlyOne);
        assert_eq!(spec.rel_type, Identification::NonIdentifying);
    }

    #[test]
    fn should_handle_1_alias_directly_followed_by_mixed_relationship_operator() {
        // ".-" is dotted-to-solid, which is NON_IDENTIFYING on the left side.
        let diagram = parse("erDiagram\nCUSTOMER 1.-one or more DELIVERY-ADDRESS : has");

        assert_eq!(diagram.entities.len(), 2);
        assert_eq!(diagram.relationships.len(), 1);
        let spec = relationship(&diagram).rel_spec;
        assert_eq!(spec.card_a, Cardinality::OneOrMore);
        assert_eq!(spec.card_b, Cardinality::OnlyOne);
        assert_eq!(spec.rel_type, Identification::NonIdentifying);
    }

    #[test]
    fn should_handle_1_alias_directly_followed_by_mixed_relationship_operator_dash_dot() {
        // "-." is solid-to-dotted, which is NON_IDENTIFYING per the grammar.
        let diagram = parse("erDiagram\nCUSTOMER 1-.one or more DELIVERY-ADDRESS : has");

        assert_eq!(diagram.entities.len(), 2);
        assert_eq!(diagram.relationships.len(), 1);
        let spec = relationship(&diagram).rel_spec;
        assert_eq!(spec.card_a, Cardinality::OneOrMore);
        assert_eq!(spec.card_b, Cardinality::OnlyOne);
        assert_eq!(spec.rel_type, Identification::NonIdentifying);
    }

    #[test]
    fn should_allow_numeric_entity_identifier_1_on_right_side_with_cardinality_1_issue_7472() {
        let diagram = parse("erDiagram\na many to 1 1: label");

        assert_eq!(diagram.entities.len(), 2);
        assert!(has_entity(&diagram, "a"));
        assert!(has_entity(&diagram, "1"));
        assert_eq!(diagram.relationships.len(), 1);
        let spec = relationship(&diagram).rel_spec;
        assert_eq!(spec.card_a, Cardinality::OnlyOne);
        assert_eq!(spec.card_b, Cardinality::ZeroOrMore);
        assert_eq!(spec.rel_type, Identification::Identifying);
    }

    #[test]
    fn should_allow_numeric_entity_identifier_12_on_right_side_with_cardinality_1_issue_7472_boundary_test()
     {
        let diagram = parse("erDiagram\na many to 1 12: label");

        assert_eq!(diagram.entities.len(), 2);
        assert!(has_entity(&diagram, "a"));
        assert!(has_entity(&diagram, "12"));
        assert_eq!(diagram.relationships.len(), 1);
        let spec = relationship(&diagram).rel_spec;
        assert_eq!(spec.card_a, Cardinality::OnlyOne);
        assert_eq!(spec.card_b, Cardinality::ZeroOrMore);
        assert_eq!(spec.rel_type, Identification::Identifying);
    }

    #[test]
    fn should_represent_identifying_relationships_properly() {
        let diagram = parse("erDiagram\nHOUSE ||--|{ ROOM : contains");

        assert_eq!(
            relationship(&diagram).rel_spec.rel_type,
            Identification::Identifying
        );
    }

    #[test]
    fn should_represent_identifying_relationships_properly_alias_to() {
        let diagram = parse("erDiagram\nHOUSE one to one ROOM : contains");

        assert_eq!(
            relationship(&diagram).rel_spec.rel_type,
            Identification::Identifying
        );
    }

    #[test]
    fn should_represent_non_identifying_relationships_properly() {
        let diagram = parse("erDiagram\n PERSON ||..o{ POSSESSION : owns");

        assert_eq!(
            relationship(&diagram).rel_spec.rel_type,
            Identification::NonIdentifying
        );
    }

    #[test]
    fn should_represent_non_identifying_relationships_properly_alias_optionally_to() {
        let diagram = parse("erDiagram\n PERSON many optionally to many POSSESSION : owns");

        assert_eq!(
            relationship(&diagram).rel_spec.rel_type,
            Identification::NonIdentifying
        );
    }

    #[test]
    fn should_not_accept_a_syntax_error() {
        assert!(fails("erDiagram\nA xxx B : has"));
    }

    #[test]
    fn should_be_possible_to_apply_a_style_to_an_entity() {
        let diagram = parse("erDiagram\nCUSTOMER\n        style CUSTOMER color:red\n      ");

        assert_eq!(entity(&diagram, "CUSTOMER").styles, ["color:red"]);
    }

    #[test]
    fn should_be_possible_to_apply_multiple_styles_to_an_entity_at_the_same_time() {
        let diagram = parse(
            "erDiagram\nCUSTOMER\n        style CUSTOMER color:red,stroke:blue,fill:#f9f\n      ",
        );

        assert_eq!(
            entity(&diagram, "CUSTOMER").styles,
            ["color:red", "stroke:blue", "fill:#f9f"]
        );
    }

    #[test]
    fn should_be_possible_to_apply_multiple_separately_defined_styles() {
        let diagram = parse(
            "erDiagram\nCUSTOMER\n        style CUSTOMER color:red\n        style CUSTOMER fill:#f9f\n      ",
        );

        assert_eq!(
            entity(&diagram, "CUSTOMER").styles,
            ["color:red", "fill:#f9f"]
        );
    }

    #[test]
    fn should_be_possible_to_assign_a_class_to_an_entity() {
        let diagram = parse("erDiagram\nCUSTOMER\nclass CUSTOMER myClass");

        assert_eq!(entity(&diagram, "CUSTOMER").classes, ["myClass"]);
    }

    #[test]
    fn should_be_possible_to_assign_multiple_classes_to_an_entity_at_the_same_time() {
        let diagram =
            parse("erDiagram\nCUSTOMER\nclass CUSTOMER firstClass, secondClass, thirdClass");

        assert_eq!(
            entity(&diagram, "CUSTOMER").classes,
            ["firstClass", "secondClass", "thirdClass"]
        );
    }

    #[test]
    fn should_be_possible_to_assign_multiple_separately_defined_classes_to_an_entity() {
        let diagram =
            parse("erDiagram\nCUSTOMER\nclass CUSTOMER firstClass\nclass CUSTOMER secondClass");

        assert_eq!(
            entity(&diagram, "CUSTOMER").classes,
            ["firstClass", "secondClass"]
        );
    }

    #[test]
    fn should_be_possible_to_configure_the_default_class_and_have_it_apply_to_each_entity() {
        let diagram = parse(
            "erDiagram\n        ENTITY1\n        ENTITY2\n        classDef default fill:#f9f\n      ",
        );

        assert!(entity(&diagram, "ENTITY1").classes.is_empty());
        assert!(entity(&diagram, "ENTITY2").classes.is_empty());
        assert_eq!(diagram.classes.len(), 1);
        let class = &diagram.classes[0];
        assert_eq!(class.id, "default");
        assert_eq!(class.styles, ["fill:#f9f"]);
        assert!(class.text_styles.is_empty());
    }

    #[test]
    fn should_be_possible_to_define_a_class_with_styles() {
        let diagram = parse(
            "erDiagram\n        classDef myClass fill:#f9f, stroke: red, color: pink\n      ",
        );

        assert_eq!(diagram.classes.len(), 1);
        let class = &diagram.classes[0];
        assert_eq!(class.id, "myClass");
        assert_eq!(class.styles, ["fill:#f9f", "stroke:red", "color:pink"]);
        assert_eq!(class.text_styles, ["color:pink"]);
    }

    #[test]
    fn should_be_possible_to_define_multiple_class_with_styles_at_the_same_time() {
        let diagram = parse(
            "erDiagram\n        classDef firstClass,secondClass fill:#f9f, stroke: red, color: pink\n       ",
        );

        assert_eq!(diagram.classes.len(), 2);
        for (class, id) in diagram.classes.iter().zip(["firstClass", "secondClass"]) {
            assert_eq!(class.id, id);
            assert_eq!(class.styles, ["fill:#f9f", "stroke:red", "color:pink"]);
            assert_eq!(class.text_styles, ["color:pink"]);
        }
    }

    #[test]
    fn should_be_possible_to_assign_a_class_using_the_shorthand_syntax_just_by_itself() {
        let diagram = parse("erDiagram\nCUSTOMER:::myClass");

        assert_eq!(entity(&diagram, "CUSTOMER").classes, ["myClass"]);
    }

    #[test]
    fn should_be_possible_to_assign_a_class_using_the_shorthand_syntax_with_empty_block() {
        let diagram = parse("erDiagram\nCUSTOMER:::myClass {}");

        assert_eq!(entity(&diagram, "CUSTOMER").classes, ["myClass"]);
    }

    #[test]
    fn should_be_possible_to_assign_a_class_using_the_shorthand_syntax_with_block_of_attributes() {
        let diagram = parse("erDiagram\nCUSTOMER:::myClass {\nstring name\n}");

        assert_eq!(entity(&diagram, "CUSTOMER").classes, ["myClass"]);
    }

    #[test]
    fn should_be_possible_to_assign_multiple_classes_using_the_shorthand_syntax() {
        let diagram = parse("erDiagram\nCUSTOMER:::firstClass,secondClass");

        assert_eq!(
            entity(&diagram, "CUSTOMER").classes,
            ["firstClass", "secondClass"]
        );
    }

    #[test]
    fn should_be_possible_to_assign_classes_using_the_shorthand_syntax_after_defining_an_alias() {
        let diagram = parse("erDiagram\nc[CUSTOMER]:::myClass");

        assert_eq!(entity(&diagram, "c").alias, "CUSTOMER");
        assert_eq!(entity(&diagram, "c").classes, ["myClass"]);
    }

    #[test]
    fn should_be_possible_to_assign_classes_using_the_shorthand_syntax_while_defining_a_relationship()
     {
        let diagram = parse("erDiagram\nCUSTOMER:::myClass ||--o{ PERSON:::myClass : allows");

        assert_eq!(entity(&diagram, "CUSTOMER").classes, ["myClass"]);
        assert_eq!(entity(&diagram, "PERSON").classes, ["myClass"]);
    }

    mod relationship_labels {
        use super::*;

        #[test]
        fn should_allow_an_empty_quoted_label() {
            let diagram = parse("erDiagram\nCUSTOMER ||--|{ ORDER : \"\"");

            assert_eq!(relationship(&diagram).role_a, "");
        }

        #[test]
        fn should_allow_a_non_empty_quoted_label() {
            let diagram = parse("erDiagram\nCUSTOMER ||--|{ ORDER : \"places\"");

            assert_eq!(relationship(&diagram).role_a, "places");
        }

        #[test]
        fn should_allow_a_non_empty_unquoted_label() {
            let diagram = parse("erDiagram\nCUSTOMER ||--|{ ORDER : places");

            assert_eq!(relationship(&diagram).role_a, "places");
        }

        #[test]
        fn should_represent_parent_child_relationship_correctly() {
            let diagram = parse("erDiagram\nPROJECT u--o{ TEAM_MEMBER : \"parent\"");

            assert_eq!(diagram.entities.len(), 2);
            assert_eq!(diagram.relationships.len(), 1);
            let spec = relationship(&diagram).rel_spec;
            assert_eq!(spec.card_b, Cardinality::MdParent);
            assert_eq!(spec.card_a, Cardinality::ZeroOrMore);
        }
    }

    mod prototype_properties {
        use super::*;

        #[test]
        fn should_work_with_a_proto_property() {
            assert!(!fails("erDiagram\n__proto__ ||--|{ ORDER : place"));
        }

        #[test]
        fn should_work_with_a_constructor_property() {
            assert!(!fails("erDiagram\nconstructor ||--|{ ORDER : place"));
        }

        #[test]
        fn should_work_with_a_prototype_property() {
            assert!(!fails("erDiagram\nprototype ||--|{ ORDER : place"));
        }
    }

    mod syntax_fixes_for_special_characters_and_numbers {
        use super::*;

        #[test]
        fn should_allow_number_1_as_standalone_entity() {
            parse("erDiagram\nCUSTOMER }|..|{ DELIVERY-ADDRESS : has\n1");
        }

        #[test]
        fn should_allow_character_u_as_standalone_entity() {
            parse("erDiagram\nCUSTOMER }|..|{ DELIVERY-ADDRESS : has\nu");
        }

        #[test]
        fn should_allow_decimal_numbers_as_standalone_entities() {
            for number in ["2.5", "1.5", "0.1", "99.99"] {
                parse(&format!(
                    "erDiagram\nCUSTOMER }}|..|{{ DELIVERY-ADDRESS : has\n{number}"
                ));
            }
        }

        #[test]
        fn should_allow_u_as_entity_name_with_attributes() {
            parse("erDiagram\nu {\nstring name\nint id\n}");
        }

        #[test]
        fn should_allow_number_1_as_entity_name_with_attributes() {
            parse("erDiagram\n1 {\nstring name\nint id\n}");
        }

        #[test]
        fn should_allow_decimal_numbers_as_entity_names_with_attributes() {
            parse("erDiagram\n2.5 {\nstring name\nint id\n}");
            parse("erDiagram\n1.5 {\nstring value\n}");
        }

        #[test]
        fn should_allow_u_in_relationships() {
            parse("erDiagram\nCUSTOMER ||--|| u : has");
            parse("erDiagram\nu ||--|| ORDER : places");
            parse("erDiagram\nu ||--|| v : connects");
        }

        #[test]
        fn should_allow_numbers_in_relationships() {
            parse("erDiagram\nCUSTOMER ||--|| 1 : has");
            parse("erDiagram\n1 ||--|| ORDER : places");
            parse("erDiagram\n1 ||--|| 2 : connects");
        }

        #[test]
        fn should_allow_decimal_numbers_in_relationships() {
            parse("erDiagram\nCUSTOMER ||--|| 2.5 : has");
            parse("erDiagram\n1.5 ||--|| ORDER : places");
            parse("erDiagram\n2.5 ||--|| 5.5 : connects");
        }

        #[test]
        fn should_handle_complex_diagram_with_special_entity_names() {
            parse(
                "erDiagram\n              CUSTOMER ||--o{ 1 : places\n              1 ||--|{ u : contains\n              u {\n                string name\n                int quantity\n              }\n              \"2.5\" ||--|| ORDER : processes\n              ORDER {\n                int id\n                date created\n              }\n        ",
            );
        }

        #[test]
        fn should_handle_attributes_with_numbers_in_names_but_not_starting() {
            parse(
                "erDiagram\n              ENTITY {\n                string name1\n                int value2\n                float point3_5\n              }\n        ",
            );
        }
    }
}
