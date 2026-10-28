//! Port of the generated cases of `packages/mermaid/src/diagrams/er/parser/erDiagram.spec.js`
//! (`describe('has non A-Za-z0-9_- chars')`): five cases for each character of the spec's list, and
//! the case with all of them together. The file is generated from that list; a title names each
//! character (`~` is `tilde`, a character outside ASCII is `u` and its code point). A repeated title
//! gets `_2`, `_3`, … in source order.
//!
//! The reference's checks have two shapes. A `toThrow` passes when the parse fails, and also when
//! its inner `expect(entities.has(name)).toBe(false)` fails, which it does when the entity exists:
//! those cases are `throws_or_defines`. The others must parse and create the entity: `defines`.

use ferrigura_syntax::er::parse_grammar;

fn defines(text: &str, name: &str) -> bool {
    parse_grammar(text)
        .is_ok_and(|diagram| diagram.entities.iter().any(|entity| entity.name == name))
}

fn throws_or_defines(text: &str, name: &str) -> bool {
    match parse_grammar(text) {
        Err(_) => true,
        Ok(diagram) => diagram.entities.iter().any(|entity| entity.name == name),
    }
}

mod when_parsing_er_diagram_it {
    use super::*;

    mod entity_name {
        use super::*;

        mod has_non_a_z0_9_chars {
            use super::*;

            #[test]
            fn blo_tilde_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo~rf\n", "Blo~rf"));
            }

            #[test]
            fn blo_tilde_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo~rf\"\n", "Blo~rf"));
            }

            #[test]
            fn blo_tilde_tilde_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo~~rf\"\n", "Blo~~rf"));
            }

            #[test]
            fn blo_tilde_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf~\"\n", "Blorf~"));
            }

            #[test]
            fn tilde_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo~~rf\"\n", "Blo~~rf"));
            }

            #[test]
            fn blo_backtick_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo`rf\n", "Blo`rf"));
            }

            #[test]
            fn blo_backtick_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo`rf\"\n", "Blo`rf"));
            }

            #[test]
            fn blo_backtick_backtick_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo``rf\"\n", "Blo``rf"));
            }

            #[test]
            fn blo_backtick_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf`\"\n", "Blorf`"));
            }

            #[test]
            fn backtick_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo``rf\"\n", "Blo``rf"));
            }

            #[test]
            fn blo_bang_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo!rf\n", "Blo!rf"));
            }

            #[test]
            fn blo_bang_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo!rf\"\n", "Blo!rf"));
            }

            #[test]
            fn blo_bang_bang_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo!!rf\"\n", "Blo!!rf"));
            }

            #[test]
            fn blo_bang_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf!\"\n", "Blorf!"));
            }

            #[test]
            fn bang_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo!!rf\"\n", "Blo!!rf"));
            }

            #[test]
            fn blo_at_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo@rf\n", "Blo@rf"));
            }

            #[test]
            fn blo_at_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo@rf\"\n", "Blo@rf"));
            }

            #[test]
            fn blo_at_at_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo@@rf\"\n", "Blo@@rf"));
            }

            #[test]
            fn blo_at_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf@\"\n", "Blorf@"));
            }

            #[test]
            fn at_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo@@rf\"\n", "Blo@@rf"));
            }

            #[test]
            fn blo_hash_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo#rf\n", "Blo#rf"));
            }

            #[test]
            fn blo_hash_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo#rf\"\n", "Blo#rf"));
            }

            #[test]
            fn blo_hash_hash_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo##rf\"\n", "Blo##rf"));
            }

            #[test]
            fn blo_hash_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf#\"\n", "Blorf#"));
            }

            #[test]
            fn hash_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo##rf\"\n", "Blo##rf"));
            }

            #[test]
            fn blo_dollar_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo$rf\n", "Blo$rf"));
            }

            #[test]
            fn blo_dollar_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo$rf\"\n", "Blo$rf"));
            }

            #[test]
            fn blo_dollar_dollar_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo$$rf\"\n", "Blo$$rf"));
            }

            #[test]
            fn blo_dollar_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf$\"\n", "Blorf$"));
            }

            #[test]
            fn dollar_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo$$rf\"\n", "Blo$$rf"));
            }

            #[test]
            fn blo_caret_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo^rf\n", "Blo^rf"));
            }

            #[test]
            fn blo_caret_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo^rf\"\n", "Blo^rf"));
            }

            #[test]
            fn blo_caret_caret_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo^^rf\"\n", "Blo^^rf"));
            }

            #[test]
            fn blo_caret_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf^\"\n", "Blorf^"));
            }

            #[test]
            fn caret_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo^^rf\"\n", "Blo^^rf"));
            }

            #[test]
            fn blo_ampersand_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo&rf\n", "Blo&rf"));
            }

            #[test]
            fn blo_ampersand_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo&rf\"\n", "Blo&rf"));
            }

            #[test]
            fn blo_ampersand_ampersand_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo&&rf\"\n", "Blo&&rf"));
            }

            #[test]
            fn blo_ampersand_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf&\"\n", "Blorf&"));
            }

            #[test]
            fn ampersand_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo&&rf\"\n", "Blo&&rf"));
            }

            #[test]
            fn blo_asterisk_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo*rf\n", "Blo*rf"));
            }

            #[test]
            fn blo_asterisk_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo*rf\"\n", "Blo*rf"));
            }

            #[test]
            fn blo_asterisk_asterisk_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo**rf\"\n", "Blo**rf"));
            }

            #[test]
            fn blo_asterisk_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf*\"\n", "Blorf*"));
            }

            #[test]
            fn asterisk_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo**rf\"\n", "Blo**rf"));
            }

            #[test]
            fn blo_lparen_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo(rf\n", "Blo(rf"));
            }

            #[test]
            fn blo_lparen_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo(rf\"\n", "Blo(rf"));
            }

            #[test]
            fn blo_lparen_lparen_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo((rf\"\n", "Blo((rf"));
            }

            #[test]
            fn blo_lparen_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf(\"\n", "Blorf("));
            }

            #[test]
            fn lparen_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo((rf\"\n", "Blo((rf"));
            }

            #[test]
            fn blo_rparen_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo)rf\n", "Blo)rf"));
            }

            #[test]
            fn blo_rparen_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo)rf\"\n", "Blo)rf"));
            }

            #[test]
            fn blo_rparen_rparen_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo))rf\"\n", "Blo))rf"));
            }

            #[test]
            fn blo_rparen_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf)\"\n", "Blorf)"));
            }

            #[test]
            fn rparen_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo))rf\"\n", "Blo))rf"));
            }

            #[test]
            fn blo_dash_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo-rf\n", "Blo-rf"));
            }

            #[test]
            fn blo_dash_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo-rf\"\n", "Blo-rf"));
            }

            #[test]
            fn blo_dash_dash_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo--rf\"\n", "Blo--rf"));
            }

            #[test]
            fn blo_dash_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf-\"\n", "Blorf-"));
            }

            #[test]
            fn dash_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo--rf\"\n", "Blo--rf"));
            }

            #[test]
            fn blo_equals_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo=rf\n", "Blo=rf"));
            }

            #[test]
            fn blo_equals_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo=rf\"\n", "Blo=rf"));
            }

            #[test]
            fn blo_equals_equals_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo==rf\"\n", "Blo==rf"));
            }

            #[test]
            fn blo_equals_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf=\"\n", "Blorf="));
            }

            #[test]
            fn equals_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo==rf\"\n", "Blo==rf"));
            }

            #[test]
            fn blo_plus_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo+rf\n", "Blo+rf"));
            }

            #[test]
            fn blo_plus_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo+rf\"\n", "Blo+rf"));
            }

            #[test]
            fn blo_plus_plus_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo++rf\"\n", "Blo++rf"));
            }

            #[test]
            fn blo_plus_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf+\"\n", "Blorf+"));
            }

            #[test]
            fn plus_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo++rf\"\n", "Blo++rf"));
            }

            #[test]
            fn blo_lbracket_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo[rf\n", "Blo[rf"));
            }

            #[test]
            fn blo_lbracket_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo[rf\"\n", "Blo[rf"));
            }

            #[test]
            fn blo_lbracket_lbracket_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo[[rf\"\n", "Blo[[rf"));
            }

            #[test]
            fn blo_lbracket_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf[\"\n", "Blorf["));
            }

            #[test]
            fn lbracket_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo[[rf\"\n", "Blo[[rf"));
            }

            #[test]
            fn blo_rbracket_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo]rf\n", "Blo]rf"));
            }

            #[test]
            fn blo_rbracket_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo]rf\"\n", "Blo]rf"));
            }

            #[test]
            fn blo_rbracket_rbracket_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo]]rf\"\n", "Blo]]rf"));
            }

            #[test]
            fn blo_rbracket_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf]\"\n", "Blorf]"));
            }

            #[test]
            fn rbracket_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo]]rf\"\n", "Blo]]rf"));
            }

            #[test]
            fn blo_lbrace_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo{rf\n", "Blo{rf"));
            }

            #[test]
            fn blo_lbrace_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo{rf\"\n", "Blo{rf"));
            }

            #[test]
            fn blo_lbrace_lbrace_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo{{rf\"\n", "Blo{{rf"));
            }

            #[test]
            fn blo_lbrace_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf{\"\n", "Blorf{"));
            }

            #[test]
            fn lbrace_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo{{rf\"\n", "Blo{{rf"));
            }

            #[test]
            fn blo_rbrace_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo}rf\n", "Blo}rf"));
            }

            #[test]
            fn blo_rbrace_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo}rf\"\n", "Blo}rf"));
            }

            #[test]
            fn blo_rbrace_rbrace_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo}}rf\"\n", "Blo}}rf"));
            }

            #[test]
            fn blo_rbrace_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf}\"\n", "Blorf}"));
            }

            #[test]
            fn rbrace_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo}}rf\"\n", "Blo}}rf"));
            }

            #[test]
            fn blo_pipe_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo|rf\n", "Blo|rf"));
            }

            #[test]
            fn blo_pipe_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo|rf\"\n", "Blo|rf"));
            }

            #[test]
            fn blo_pipe_pipe_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo||rf\"\n", "Blo||rf"));
            }

            #[test]
            fn blo_pipe_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf|\"\n", "Blorf|"));
            }

            #[test]
            fn pipe_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo||rf\"\n", "Blo||rf"));
            }

            #[test]
            fn blo_slash_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo/rf\n", "Blo/rf"));
            }

            #[test]
            fn blo_slash_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo/rf\"\n", "Blo/rf"));
            }

            #[test]
            fn blo_slash_slash_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo//rf\"\n", "Blo//rf"));
            }

            #[test]
            fn blo_slash_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf/\"\n", "Blorf/"));
            }

            #[test]
            fn slash_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo//rf\"\n", "Blo//rf"));
            }

            #[test]
            fn blo_semicolon_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo;rf\n", "Blo;rf"));
            }

            #[test]
            fn blo_semicolon_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo;rf\"\n", "Blo;rf"));
            }

            #[test]
            fn blo_semicolon_semicolon_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo;;rf\"\n", "Blo;;rf"));
            }

            #[test]
            fn blo_semicolon_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf;\"\n", "Blorf;"));
            }

            #[test]
            fn semicolon_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo;;rf\"\n", "Blo;;rf"));
            }

            #[test]
            fn blo_colon_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo:rf\n", "Blo:rf"));
            }

            #[test]
            fn blo_colon_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo:rf\"\n", "Blo:rf"));
            }

            #[test]
            fn blo_colon_colon_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo::rf\"\n", "Blo::rf"));
            }

            #[test]
            fn blo_colon_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf:\"\n", "Blorf:"));
            }

            #[test]
            fn colon_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo::rf\"\n", "Blo::rf"));
            }

            #[test]
            fn blo_apostrophe_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo'rf\n", "Blo'rf"));
            }

            #[test]
            fn blo_apostrophe_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo'rf\"\n", "Blo'rf"));
            }

            #[test]
            fn blo_apostrophe_apostrophe_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo''rf\"\n", "Blo''rf"));
            }

            #[test]
            fn blo_apostrophe_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf'\"\n", "Blorf'"));
            }

            #[test]
            fn apostrophe_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo''rf\"\n", "Blo''rf"));
            }

            #[test]
            fn blo_dot_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo.rf\n", "Blo.rf"));
            }

            #[test]
            fn blo_dot_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo.rf\"\n", "Blo.rf"));
            }

            #[test]
            fn blo_dot_dot_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo..rf\"\n", "Blo..rf"));
            }

            #[test]
            fn blo_dot_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf.\"\n", "Blorf."));
            }

            #[test]
            fn dot_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo..rf\"\n", "Blo..rf"));
            }

            #[test]
            fn blo_question_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo?rf\n", "Blo?rf"));
            }

            #[test]
            fn blo_question_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo?rf\"\n", "Blo?rf"));
            }

            #[test]
            fn blo_question_question_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo??rf\"\n", "Blo??rf"));
            }

            #[test]
            fn blo_question_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf?\"\n", "Blorf?"));
            }

            #[test]
            fn question_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo??rf\"\n", "Blo??rf"));
            }

            #[test]
            fn blo_u00a1_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo¡rf\n", "Blo¡rf"));
            }

            #[test]
            fn blo_u00a1_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo¡rf\"\n", "Blo¡rf"));
            }

            #[test]
            fn blo_u00a1_u00a1_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo¡¡rf\"\n", "Blo¡¡rf"));
            }

            #[test]
            fn blo_u00a1_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf¡\"\n", "Blorf¡"));
            }

            #[test]
            fn u00a1_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo¡¡rf\"\n", "Blo¡¡rf"));
            }

            #[test]
            fn blo_u2044_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo⁄rf\n", "Blo⁄rf"));
            }

            #[test]
            fn blo_u2044_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo⁄rf\"\n", "Blo⁄rf"));
            }

            #[test]
            fn blo_u2044_u2044_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo⁄⁄rf\"\n", "Blo⁄⁄rf"));
            }

            #[test]
            fn blo_u2044_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf⁄\"\n", "Blorf⁄"));
            }

            #[test]
            fn u2044_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo⁄⁄rf\"\n", "Blo⁄⁄rf"));
            }

            #[test]
            fn blo_u2122_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo™rf\n", "Blo™rf"));
            }

            #[test]
            fn blo_u2122_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo™rf\"\n", "Blo™rf"));
            }

            #[test]
            fn blo_u2122_u2122_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo™™rf\"\n", "Blo™™rf"));
            }

            #[test]
            fn blo_u2122_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf™\"\n", "Blorf™"));
            }

            #[test]
            fn u2122_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo™™rf\"\n", "Blo™™rf"));
            }

            #[test]
            fn blo_u20ac_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo€rf\n", "Blo€rf"));
            }

            #[test]
            fn blo_u20ac_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo€rf\"\n", "Blo€rf"));
            }

            #[test]
            fn blo_u20ac_u20ac_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo€€rf\"\n", "Blo€€rf"));
            }

            #[test]
            fn blo_u20ac_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf€\"\n", "Blorf€"));
            }

            #[test]
            fn u20ac_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo€€rf\"\n", "Blo€€rf"));
            }

            #[test]
            fn blo_u00a3_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo£rf\n", "Blo£rf"));
            }

            #[test]
            fn blo_u00a3_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo£rf\"\n", "Blo£rf"));
            }

            #[test]
            fn blo_u00a3_u00a3_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo££rf\"\n", "Blo££rf"));
            }

            #[test]
            fn blo_u00a3_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf£\"\n", "Blorf£"));
            }

            #[test]
            fn u00a3_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo££rf\"\n", "Blo££rf"));
            }

            #[test]
            fn blo_u2039_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo‹rf\n", "Blo‹rf"));
            }

            #[test]
            fn blo_u2039_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo‹rf\"\n", "Blo‹rf"));
            }

            #[test]
            fn blo_u2039_u2039_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo‹‹rf\"\n", "Blo‹‹rf"));
            }

            #[test]
            fn blo_u2039_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf‹\"\n", "Blorf‹"));
            }

            #[test]
            fn u2039_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo‹‹rf\"\n", "Blo‹‹rf"));
            }

            #[test]
            fn blo_u00a2_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo¢rf\n", "Blo¢rf"));
            }

            #[test]
            fn blo_u00a2_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo¢rf\"\n", "Blo¢rf"));
            }

            #[test]
            fn blo_u00a2_u00a2_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo¢¢rf\"\n", "Blo¢¢rf"));
            }

            #[test]
            fn blo_u00a2_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf¢\"\n", "Blorf¢"));
            }

            #[test]
            fn u00a2_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo¢¢rf\"\n", "Blo¢¢rf"));
            }

            #[test]
            fn blo_u203a_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo›rf\n", "Blo›rf"));
            }

            #[test]
            fn blo_u203a_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo›rf\"\n", "Blo›rf"));
            }

            #[test]
            fn blo_u203a_u203a_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo››rf\"\n", "Blo››rf"));
            }

            #[test]
            fn blo_u203a_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf›\"\n", "Blorf›"));
            }

            #[test]
            fn u203a_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo››rf\"\n", "Blo››rf"));
            }

            #[test]
            fn blo_u221e_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo∞rf\n", "Blo∞rf"));
            }

            #[test]
            fn blo_u221e_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo∞rf\"\n", "Blo∞rf"));
            }

            #[test]
            fn blo_u221e_u221e_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo∞∞rf\"\n", "Blo∞∞rf"));
            }

            #[test]
            fn blo_u221e_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf∞\"\n", "Blorf∞"));
            }

            #[test]
            fn u221e_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo∞∞rf\"\n", "Blo∞∞rf"));
            }

            #[test]
            fn blo_ufb01_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloﬁrf\n", "Bloﬁrf"));
            }

            #[test]
            fn blo_ufb01_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloﬁrf\"\n", "Bloﬁrf"));
            }

            #[test]
            fn blo_ufb01_ufb01_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloﬁﬁrf\"\n", "Bloﬁﬁrf"));
            }

            #[test]
            fn blo_ufb01_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfﬁ\"\n", "Blorfﬁ"));
            }

            #[test]
            fn ufb01_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloﬁﬁrf\"\n", "Bloﬁﬁrf"));
            }

            #[test]
            fn blo_u00a7_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo§rf\n", "Blo§rf"));
            }

            #[test]
            fn blo_u00a7_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo§rf\"\n", "Blo§rf"));
            }

            #[test]
            fn blo_u00a7_u00a7_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo§§rf\"\n", "Blo§§rf"));
            }

            #[test]
            fn blo_u00a7_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf§\"\n", "Blorf§"));
            }

            #[test]
            fn u00a7_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo§§rf\"\n", "Blo§§rf"));
            }

            #[test]
            fn blo_u2021_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo‡rf\n", "Blo‡rf"));
            }

            #[test]
            fn blo_u2021_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo‡rf\"\n", "Blo‡rf"));
            }

            #[test]
            fn blo_u2021_u2021_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo‡‡rf\"\n", "Blo‡‡rf"));
            }

            #[test]
            fn blo_u2021_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf‡\"\n", "Blorf‡"));
            }

            #[test]
            fn u2021_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo‡‡rf\"\n", "Blo‡‡rf"));
            }

            #[test]
            fn blo_u2022_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo•rf\n", "Blo•rf"));
            }

            #[test]
            fn blo_u2022_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo•rf\"\n", "Blo•rf"));
            }

            #[test]
            fn blo_u2022_u2022_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo••rf\"\n", "Blo••rf"));
            }

            #[test]
            fn blo_u2022_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf•\"\n", "Blorf•"));
            }

            #[test]
            fn u2022_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo••rf\"\n", "Blo••rf"));
            }

            #[test]
            fn blo_u00b0_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo°rf\n", "Blo°rf"));
            }

            #[test]
            fn blo_u00b0_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo°rf\"\n", "Blo°rf"));
            }

            #[test]
            fn blo_u00b0_u00b0_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo°°rf\"\n", "Blo°°rf"));
            }

            #[test]
            fn blo_u00b0_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf°\"\n", "Blorf°"));
            }

            #[test]
            fn u00b0_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo°°rf\"\n", "Blo°°rf"));
            }

            #[test]
            fn blo_u00aa_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloªrf\n", "Bloªrf"));
            }

            #[test]
            fn blo_u00aa_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloªrf\"\n", "Bloªrf"));
            }

            #[test]
            fn blo_u00aa_u00aa_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloªªrf\"\n", "Bloªªrf"));
            }

            #[test]
            fn blo_u00aa_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfª\"\n", "Blorfª"));
            }

            #[test]
            fn u00aa_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloªªrf\"\n", "Bloªªrf"));
            }

            #[test]
            fn blo_u00b7_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo·rf\n", "Blo·rf"));
            }

            #[test]
            fn blo_u00b7_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo·rf\"\n", "Blo·rf"));
            }

            #[test]
            fn blo_u00b7_u00b7_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo··rf\"\n", "Blo··rf"));
            }

            #[test]
            fn blo_u00b7_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf·\"\n", "Blorf·"));
            }

            #[test]
            fn u00b7_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo··rf\"\n", "Blo··rf"));
            }

            #[test]
            fn blo_u00ba_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloºrf\n", "Bloºrf"));
            }

            #[test]
            fn blo_u00ba_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloºrf\"\n", "Bloºrf"));
            }

            #[test]
            fn blo_u00ba_u00ba_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloººrf\"\n", "Bloººrf"));
            }

            #[test]
            fn blo_u00ba_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfº\"\n", "Blorfº"));
            }

            #[test]
            fn u00ba_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloººrf\"\n", "Bloººrf"));
            }

            #[test]
            fn blo_u201a_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo‚rf\n", "Blo‚rf"));
            }

            #[test]
            fn blo_u201a_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo‚rf\"\n", "Blo‚rf"));
            }

            #[test]
            fn blo_u201a_u201a_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo‚‚rf\"\n", "Blo‚‚rf"));
            }

            #[test]
            fn blo_u201a_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf‚\"\n", "Blorf‚"));
            }

            #[test]
            fn u201a_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo‚‚rf\"\n", "Blo‚‚rf"));
            }

            #[test]
            fn blo_u2260_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo≠rf\n", "Blo≠rf"));
            }

            #[test]
            fn blo_u2260_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo≠rf\"\n", "Blo≠rf"));
            }

            #[test]
            fn blo_u2260_u2260_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo≠≠rf\"\n", "Blo≠≠rf"));
            }

            #[test]
            fn blo_u2260_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf≠\"\n", "Blorf≠"));
            }

            #[test]
            fn u2260_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo≠≠rf\"\n", "Blo≠≠rf"));
            }

            #[test]
            fn blo_u00b1_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo±rf\n", "Blo±rf"));
            }

            #[test]
            fn blo_u00b1_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo±rf\"\n", "Blo±rf"));
            }

            #[test]
            fn blo_u00b1_u00b1_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo±±rf\"\n", "Blo±±rf"));
            }

            #[test]
            fn blo_u00b1_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf±\"\n", "Blorf±"));
            }

            #[test]
            fn u00b1_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo±±rf\"\n", "Blo±±rf"));
            }

            #[test]
            fn blo_u0153_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloœrf\n", "Bloœrf"));
            }

            #[test]
            fn blo_u0153_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloœrf\"\n", "Bloœrf"));
            }

            #[test]
            fn blo_u0153_u0153_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloœœrf\"\n", "Bloœœrf"));
            }

            #[test]
            fn blo_u0153_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfœ\"\n", "Blorfœ"));
            }

            #[test]
            fn u0153_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloœœrf\"\n", "Bloœœrf"));
            }

            #[test]
            fn blo_u0152_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloŒrf\n", "BloŒrf"));
            }

            #[test]
            fn blo_u0152_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloŒrf\"\n", "BloŒrf"));
            }

            #[test]
            fn blo_u0152_u0152_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloŒŒrf\"\n", "BloŒŒrf"));
            }

            #[test]
            fn blo_u0152_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfŒ\"\n", "BlorfŒ"));
            }

            #[test]
            fn u0152_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloŒŒrf\"\n", "BloŒŒrf"));
            }

            #[test]
            fn blo_u2211_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo∑rf\n", "Blo∑rf"));
            }

            #[test]
            fn blo_u2211_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo∑rf\"\n", "Blo∑rf"));
            }

            #[test]
            fn blo_u2211_u2211_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo∑∑rf\"\n", "Blo∑∑rf"));
            }

            #[test]
            fn blo_u2211_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf∑\"\n", "Blorf∑"));
            }

            #[test]
            fn u2211_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo∑∑rf\"\n", "Blo∑∑rf"));
            }

            #[test]
            fn blo_u201e_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo„rf\n", "Blo„rf"));
            }

            #[test]
            fn blo_u201e_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo„rf\"\n", "Blo„rf"));
            }

            #[test]
            fn blo_u201e_u201e_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo„„rf\"\n", "Blo„„rf"));
            }

            #[test]
            fn blo_u201e_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf„\"\n", "Blorf„"));
            }

            #[test]
            fn u201e_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo„„rf\"\n", "Blo„„rf"));
            }

            #[test]
            fn blo_u00ae_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo®rf\n", "Blo®rf"));
            }

            #[test]
            fn blo_u00ae_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo®rf\"\n", "Blo®rf"));
            }

            #[test]
            fn blo_u00ae_u00ae_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo®®rf\"\n", "Blo®®rf"));
            }

            #[test]
            fn blo_u00ae_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf®\"\n", "Blorf®"));
            }

            #[test]
            fn u00ae_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo®®rf\"\n", "Blo®®rf"));
            }

            #[test]
            fn blo_u2020_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo†rf\n", "Blo†rf"));
            }

            #[test]
            fn blo_u2020_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo†rf\"\n", "Blo†rf"));
            }

            #[test]
            fn blo_u2020_u2020_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo††rf\"\n", "Blo††rf"));
            }

            #[test]
            fn blo_u2020_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf†\"\n", "Blorf†"));
            }

            #[test]
            fn u2020_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo††rf\"\n", "Blo††rf"));
            }

            #[test]
            fn blo_u02c7_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloˇrf\n", "Bloˇrf"));
            }

            #[test]
            fn blo_u02c7_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloˇrf\"\n", "Bloˇrf"));
            }

            #[test]
            fn blo_u02c7_u02c7_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloˇˇrf\"\n", "Bloˇˇrf"));
            }

            #[test]
            fn blo_u02c7_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfˇ\"\n", "Blorfˇ"));
            }

            #[test]
            fn u02c7_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloˇˇrf\"\n", "Bloˇˇrf"));
            }

            #[test]
            fn blo_u00a5_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo¥rf\n", "Blo¥rf"));
            }

            #[test]
            fn blo_u00a5_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo¥rf\"\n", "Blo¥rf"));
            }

            #[test]
            fn blo_u00a5_u00a5_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo¥¥rf\"\n", "Blo¥¥rf"));
            }

            #[test]
            fn blo_u00a5_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf¥\"\n", "Blorf¥"));
            }

            #[test]
            fn u00a5_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo¥¥rf\"\n", "Blo¥¥rf"));
            }

            #[test]
            fn blo_u00c1_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÁrf\n", "BloÁrf"));
            }

            #[test]
            fn blo_u00c1_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÁrf\"\n", "BloÁrf"));
            }

            #[test]
            fn blo_u00c1_u00c1_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÁÁrf\"\n", "BloÁÁrf"));
            }

            #[test]
            fn blo_u00c1_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÁ\"\n", "BlorfÁ"));
            }

            #[test]
            fn u00c1_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÁÁrf\"\n", "BloÁÁrf"));
            }

            #[test]
            fn blo_u00a8_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo¨rf\n", "Blo¨rf"));
            }

            #[test]
            fn blo_u00a8_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo¨rf\"\n", "Blo¨rf"));
            }

            #[test]
            fn blo_u00a8_u00a8_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo¨¨rf\"\n", "Blo¨¨rf"));
            }

            #[test]
            fn blo_u00a8_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf¨\"\n", "Blorf¨"));
            }

            #[test]
            fn u00a8_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo¨¨rf\"\n", "Blo¨¨rf"));
            }

            #[test]
            fn blo_u02c6_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloˆrf\n", "Bloˆrf"));
            }

            #[test]
            fn blo_u02c6_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloˆrf\"\n", "Bloˆrf"));
            }

            #[test]
            fn blo_u02c6_u02c6_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloˆˆrf\"\n", "Bloˆˆrf"));
            }

            #[test]
            fn blo_u02c6_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfˆ\"\n", "Blorfˆ"));
            }

            #[test]
            fn u02c6_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloˆˆrf\"\n", "Bloˆˆrf"));
            }

            #[test]
            fn blo_u02c6_rf_fails_if_not_surrounded_by_quotes_2() {
                assert!(throws_or_defines("erDiagram\n Bloˆrf\n", "Bloˆrf"));
            }

            #[test]
            fn blo_u02c6_rf_single_occurrence_2() {
                assert!(defines("erDiagram\n \"Bloˆrf\"\n", "Bloˆrf"));
            }

            #[test]
            fn blo_u02c6_u02c6_rf_repeated_occurrence_2() {
                assert!(defines("erDiagram\n \"Bloˆˆrf\"\n", "Bloˆˆrf"));
            }

            #[test]
            fn blo_u02c6_rf_ends_with_2() {
                assert!(defines("erDiagram\n \"Blorfˆ\"\n", "Blorfˆ"));
            }

            #[test]
            fn u02c6_blorf_cannot_start_with_the_character_2() {
                assert!(throws_or_defines("erDiagram\n \"Bloˆˆrf\"\n", "Bloˆˆrf"));
            }

            #[test]
            fn blo_u00d8_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloØrf\n", "BloØrf"));
            }

            #[test]
            fn blo_u00d8_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloØrf\"\n", "BloØrf"));
            }

            #[test]
            fn blo_u00d8_u00d8_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloØØrf\"\n", "BloØØrf"));
            }

            #[test]
            fn blo_u00d8_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfØ\"\n", "BlorfØ"));
            }

            #[test]
            fn u00d8_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloØØrf\"\n", "BloØØrf"));
            }

            #[test]
            fn blo_u03c0_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloπrf\n", "Bloπrf"));
            }

            #[test]
            fn blo_u03c0_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloπrf\"\n", "Bloπrf"));
            }

            #[test]
            fn blo_u03c0_u03c0_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloππrf\"\n", "Bloππrf"));
            }

            #[test]
            fn blo_u03c0_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfπ\"\n", "Blorfπ"));
            }

            #[test]
            fn u03c0_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloππrf\"\n", "Bloππrf"));
            }

            #[test]
            fn blo_u220f_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo∏rf\n", "Blo∏rf"));
            }

            #[test]
            fn blo_u220f_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo∏rf\"\n", "Blo∏rf"));
            }

            #[test]
            fn blo_u220f_u220f_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo∏∏rf\"\n", "Blo∏∏rf"));
            }

            #[test]
            fn blo_u220f_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf∏\"\n", "Blorf∏"));
            }

            #[test]
            fn u220f_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo∏∏rf\"\n", "Blo∏∏rf"));
            }

            #[test]
            fn blo_u201c_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo“rf\n", "Blo“rf"));
            }

            #[test]
            fn blo_u201c_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo“rf\"\n", "Blo“rf"));
            }

            #[test]
            fn blo_u201c_u201c_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo““rf\"\n", "Blo““rf"));
            }

            #[test]
            fn blo_u201c_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf“\"\n", "Blorf“"));
            }

            #[test]
            fn u201c_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo““rf\"\n", "Blo““rf"));
            }

            #[test]
            fn blo_u00ab_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo«rf\n", "Blo«rf"));
            }

            #[test]
            fn blo_u00ab_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo«rf\"\n", "Blo«rf"));
            }

            #[test]
            fn blo_u00ab_u00ab_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo««rf\"\n", "Blo««rf"));
            }

            #[test]
            fn blo_u00ab_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf«\"\n", "Blorf«"));
            }

            #[test]
            fn u00ab_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo««rf\"\n", "Blo««rf"));
            }

            #[test]
            fn blo_u00bb_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo»rf\n", "Blo»rf"));
            }

            #[test]
            fn blo_u00bb_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo»rf\"\n", "Blo»rf"));
            }

            #[test]
            fn blo_u00bb_u00bb_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo»»rf\"\n", "Blo»»rf"));
            }

            #[test]
            fn blo_u00bb_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf»\"\n", "Blorf»"));
            }

            #[test]
            fn u00bb_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo»»rf\"\n", "Blo»»rf"));
            }

            #[test]
            fn blo_u00e5_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloårf\n", "Bloårf"));
            }

            #[test]
            fn blo_u00e5_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloårf\"\n", "Bloårf"));
            }

            #[test]
            fn blo_u00e5_u00e5_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloåårf\"\n", "Bloåårf"));
            }

            #[test]
            fn blo_u00e5_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfå\"\n", "Blorfå"));
            }

            #[test]
            fn u00e5_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloåårf\"\n", "Bloåårf"));
            }

            #[test]
            fn blo_u00c5_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÅrf\n", "BloÅrf"));
            }

            #[test]
            fn blo_u00c5_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÅrf\"\n", "BloÅrf"));
            }

            #[test]
            fn blo_u00c5_u00c5_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÅÅrf\"\n", "BloÅÅrf"));
            }

            #[test]
            fn blo_u00c5_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÅ\"\n", "BlorfÅ"));
            }

            #[test]
            fn u00c5_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÅÅrf\"\n", "BloÅÅrf"));
            }

            #[test]
            fn blo_u00df_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloßrf\n", "Bloßrf"));
            }

            #[test]
            fn blo_u00df_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloßrf\"\n", "Bloßrf"));
            }

            #[test]
            fn blo_u00df_u00df_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloßßrf\"\n", "Bloßßrf"));
            }

            #[test]
            fn blo_u00df_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfß\"\n", "Blorfß"));
            }

            #[test]
            fn u00df_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloßßrf\"\n", "Bloßßrf"));
            }

            #[test]
            fn blo_u00cd_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÍrf\n", "BloÍrf"));
            }

            #[test]
            fn blo_u00cd_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÍrf\"\n", "BloÍrf"));
            }

            #[test]
            fn blo_u00cd_u00cd_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÍÍrf\"\n", "BloÍÍrf"));
            }

            #[test]
            fn blo_u00cd_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÍ\"\n", "BlorfÍ"));
            }

            #[test]
            fn u00cd_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÍÍrf\"\n", "BloÍÍrf"));
            }

            #[test]
            fn blo_u2202_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo∂rf\n", "Blo∂rf"));
            }

            #[test]
            fn blo_u2202_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo∂rf\"\n", "Blo∂rf"));
            }

            #[test]
            fn blo_u2202_u2202_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo∂∂rf\"\n", "Blo∂∂rf"));
            }

            #[test]
            fn blo_u2202_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf∂\"\n", "Blorf∂"));
            }

            #[test]
            fn u2202_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo∂∂rf\"\n", "Blo∂∂rf"));
            }

            #[test]
            fn blo_u00ce_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÎrf\n", "BloÎrf"));
            }

            #[test]
            fn blo_u00ce_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÎrf\"\n", "BloÎrf"));
            }

            #[test]
            fn blo_u00ce_u00ce_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÎÎrf\"\n", "BloÎÎrf"));
            }

            #[test]
            fn blo_u00ce_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÎ\"\n", "BlorfÎ"));
            }

            #[test]
            fn u00ce_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÎÎrf\"\n", "BloÎÎrf"));
            }

            #[test]
            fn blo_u0192_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloƒrf\n", "Bloƒrf"));
            }

            #[test]
            fn blo_u0192_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloƒrf\"\n", "Bloƒrf"));
            }

            #[test]
            fn blo_u0192_u0192_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloƒƒrf\"\n", "Bloƒƒrf"));
            }

            #[test]
            fn blo_u0192_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfƒ\"\n", "Blorfƒ"));
            }

            #[test]
            fn u0192_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloƒƒrf\"\n", "Bloƒƒrf"));
            }

            #[test]
            fn blo_u00cf_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÏrf\n", "BloÏrf"));
            }

            #[test]
            fn blo_u00cf_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÏrf\"\n", "BloÏrf"));
            }

            #[test]
            fn blo_u00cf_u00cf_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÏÏrf\"\n", "BloÏÏrf"));
            }

            #[test]
            fn blo_u00cf_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÏ\"\n", "BlorfÏ"));
            }

            #[test]
            fn u00cf_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÏÏrf\"\n", "BloÏÏrf"));
            }

            #[test]
            fn blo_u00a9_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo©rf\n", "Blo©rf"));
            }

            #[test]
            fn blo_u00a9_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo©rf\"\n", "Blo©rf"));
            }

            #[test]
            fn blo_u00a9_u00a9_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo©©rf\"\n", "Blo©©rf"));
            }

            #[test]
            fn blo_u00a9_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf©\"\n", "Blorf©"));
            }

            #[test]
            fn u00a9_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo©©rf\"\n", "Blo©©rf"));
            }

            #[test]
            fn blo_u02d9_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo˙rf\n", "Blo˙rf"));
            }

            #[test]
            fn blo_u02d9_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo˙rf\"\n", "Blo˙rf"));
            }

            #[test]
            fn blo_u02d9_u02d9_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo˙˙rf\"\n", "Blo˙˙rf"));
            }

            #[test]
            fn blo_u02d9_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf˙\"\n", "Blorf˙"));
            }

            #[test]
            fn u02d9_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo˙˙rf\"\n", "Blo˙˙rf"));
            }

            #[test]
            fn blo_u00d3_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÓrf\n", "BloÓrf"));
            }

            #[test]
            fn blo_u00d3_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÓrf\"\n", "BloÓrf"));
            }

            #[test]
            fn blo_u00d3_u00d3_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÓÓrf\"\n", "BloÓÓrf"));
            }

            #[test]
            fn blo_u00d3_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÓ\"\n", "BlorfÓ"));
            }

            #[test]
            fn u00d3_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÓÓrf\"\n", "BloÓÓrf"));
            }

            #[test]
            fn blo_u2206_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo∆rf\n", "Blo∆rf"));
            }

            #[test]
            fn blo_u2206_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo∆rf\"\n", "Blo∆rf"));
            }

            #[test]
            fn blo_u2206_u2206_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo∆∆rf\"\n", "Blo∆∆rf"));
            }

            #[test]
            fn blo_u2206_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf∆\"\n", "Blorf∆"));
            }

            #[test]
            fn u2206_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo∆∆rf\"\n", "Blo∆∆rf"));
            }

            #[test]
            fn blo_u00d4_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÔrf\n", "BloÔrf"));
            }

            #[test]
            fn blo_u00d4_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÔrf\"\n", "BloÔrf"));
            }

            #[test]
            fn blo_u00d4_u00d4_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÔÔrf\"\n", "BloÔÔrf"));
            }

            #[test]
            fn blo_u00d4_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÔ\"\n", "BlorfÔ"));
            }

            #[test]
            fn u00d4_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÔÔrf\"\n", "BloÔÔrf"));
            }

            #[test]
            fn blo_u02da_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo˚rf\n", "Blo˚rf"));
            }

            #[test]
            fn blo_u02da_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo˚rf\"\n", "Blo˚rf"));
            }

            #[test]
            fn blo_u02da_u02da_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo˚˚rf\"\n", "Blo˚˚rf"));
            }

            #[test]
            fn blo_u02da_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf˚\"\n", "Blorf˚"));
            }

            #[test]
            fn u02da_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo˚˚rf\"\n", "Blo˚˚rf"));
            }

            #[test]
            fn blo_uf8ff_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blorf\n", "Blorf"));
            }

            #[test]
            fn blo_uf8ff_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blorf\"\n", "Blorf"));
            }

            #[test]
            fn blo_uf8ff_uf8ff_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blorf\"\n", "Blorf"));
            }

            #[test]
            fn blo_uf8ff_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf\"\n", "Blorf"));
            }

            #[test]
            fn uf8ff_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blorf\"\n", "Blorf"));
            }

            #[test]
            fn blo_u00ac_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo¬rf\n", "Blo¬rf"));
            }

            #[test]
            fn blo_u00ac_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo¬rf\"\n", "Blo¬rf"));
            }

            #[test]
            fn blo_u00ac_u00ac_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo¬¬rf\"\n", "Blo¬¬rf"));
            }

            #[test]
            fn blo_u00ac_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf¬\"\n", "Blorf¬"));
            }

            #[test]
            fn u00ac_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo¬¬rf\"\n", "Blo¬¬rf"));
            }

            #[test]
            fn blo_u00d2_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÒrf\n", "BloÒrf"));
            }

            #[test]
            fn blo_u00d2_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÒrf\"\n", "BloÒrf"));
            }

            #[test]
            fn blo_u00d2_u00d2_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÒÒrf\"\n", "BloÒÒrf"));
            }

            #[test]
            fn blo_u00d2_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÒ\"\n", "BlorfÒ"));
            }

            #[test]
            fn u00d2_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÒÒrf\"\n", "BloÒÒrf"));
            }

            #[test]
            fn blo_u2026_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo…rf\n", "Blo…rf"));
            }

            #[test]
            fn blo_u2026_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo…rf\"\n", "Blo…rf"));
            }

            #[test]
            fn blo_u2026_u2026_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo……rf\"\n", "Blo……rf"));
            }

            #[test]
            fn blo_u2026_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf…\"\n", "Blorf…"));
            }

            #[test]
            fn u2026_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo……rf\"\n", "Blo……rf"));
            }

            #[test]
            fn blo_u00da_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÚrf\n", "BloÚrf"));
            }

            #[test]
            fn blo_u00da_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÚrf\"\n", "BloÚrf"));
            }

            #[test]
            fn blo_u00da_u00da_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÚÚrf\"\n", "BloÚÚrf"));
            }

            #[test]
            fn blo_u00da_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÚ\"\n", "BlorfÚ"));
            }

            #[test]
            fn u00da_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÚÚrf\"\n", "BloÚÚrf"));
            }

            #[test]
            fn blo_u00e6_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloærf\n", "Bloærf"));
            }

            #[test]
            fn blo_u00e6_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloærf\"\n", "Bloærf"));
            }

            #[test]
            fn blo_u00e6_u00e6_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloæærf\"\n", "Bloæærf"));
            }

            #[test]
            fn blo_u00e6_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfæ\"\n", "Blorfæ"));
            }

            #[test]
            fn u00e6_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloæærf\"\n", "Bloæærf"));
            }

            #[test]
            fn blo_u00c6_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÆrf\n", "BloÆrf"));
            }

            #[test]
            fn blo_u00c6_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÆrf\"\n", "BloÆrf"));
            }

            #[test]
            fn blo_u00c6_u00c6_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÆÆrf\"\n", "BloÆÆrf"));
            }

            #[test]
            fn blo_u00c6_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÆ\"\n", "BlorfÆ"));
            }

            #[test]
            fn u00c6_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÆÆrf\"\n", "BloÆÆrf"));
            }

            #[test]
            fn blo_u03a9_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloΩrf\n", "BloΩrf"));
            }

            #[test]
            fn blo_u03a9_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloΩrf\"\n", "BloΩrf"));
            }

            #[test]
            fn blo_u03a9_u03a9_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloΩΩrf\"\n", "BloΩΩrf"));
            }

            #[test]
            fn blo_u03a9_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfΩ\"\n", "BlorfΩ"));
            }

            #[test]
            fn u03a9_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloΩΩrf\"\n", "BloΩΩrf"));
            }

            #[test]
            fn blo_u00b8_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo¸rf\n", "Blo¸rf"));
            }

            #[test]
            fn blo_u00b8_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo¸rf\"\n", "Blo¸rf"));
            }

            #[test]
            fn blo_u00b8_u00b8_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo¸¸rf\"\n", "Blo¸¸rf"));
            }

            #[test]
            fn blo_u00b8_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf¸\"\n", "Blorf¸"));
            }

            #[test]
            fn u00b8_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo¸¸rf\"\n", "Blo¸¸rf"));
            }

            #[test]
            fn blo_u2248_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo≈rf\n", "Blo≈rf"));
            }

            #[test]
            fn blo_u2248_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo≈rf\"\n", "Blo≈rf"));
            }

            #[test]
            fn blo_u2248_u2248_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo≈≈rf\"\n", "Blo≈≈rf"));
            }

            #[test]
            fn blo_u2248_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf≈\"\n", "Blorf≈"));
            }

            #[test]
            fn u2248_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo≈≈rf\"\n", "Blo≈≈rf"));
            }

            #[test]
            fn blo_u03c0_rf_fails_if_not_surrounded_by_quotes_2() {
                assert!(throws_or_defines("erDiagram\n Bloπrf\n", "Bloπrf"));
            }

            #[test]
            fn blo_u03c0_rf_single_occurrence_2() {
                assert!(defines("erDiagram\n \"Bloπrf\"\n", "Bloπrf"));
            }

            #[test]
            fn blo_u03c0_u03c0_rf_repeated_occurrence_2() {
                assert!(defines("erDiagram\n \"Bloππrf\"\n", "Bloππrf"));
            }

            #[test]
            fn blo_u03c0_rf_ends_with_2() {
                assert!(defines("erDiagram\n \"Blorfπ\"\n", "Blorfπ"));
            }

            #[test]
            fn u03c0_blorf_cannot_start_with_the_character_2() {
                assert!(throws_or_defines("erDiagram\n \"Bloππrf\"\n", "Bloππrf"));
            }

            #[test]
            fn blo_u02db_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo˛rf\n", "Blo˛rf"));
            }

            #[test]
            fn blo_u02db_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo˛rf\"\n", "Blo˛rf"));
            }

            #[test]
            fn blo_u02db_u02db_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo˛˛rf\"\n", "Blo˛˛rf"));
            }

            #[test]
            fn blo_u02db_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf˛\"\n", "Blorf˛"));
            }

            #[test]
            fn u02db_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo˛˛rf\"\n", "Blo˛˛rf"));
            }

            #[test]
            fn blo_u00e7_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloçrf\n", "Bloçrf"));
            }

            #[test]
            fn blo_u00e7_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloçrf\"\n", "Bloçrf"));
            }

            #[test]
            fn blo_u00e7_u00e7_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloççrf\"\n", "Bloççrf"));
            }

            #[test]
            fn blo_u00e7_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfç\"\n", "Blorfç"));
            }

            #[test]
            fn u00e7_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloççrf\"\n", "Bloççrf"));
            }

            #[test]
            fn blo_u00c7_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÇrf\n", "BloÇrf"));
            }

            #[test]
            fn blo_u00c7_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÇrf\"\n", "BloÇrf"));
            }

            #[test]
            fn blo_u00c7_u00c7_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÇÇrf\"\n", "BloÇÇrf"));
            }

            #[test]
            fn blo_u00c7_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÇ\"\n", "BlorfÇ"));
            }

            #[test]
            fn u00c7_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÇÇrf\"\n", "BloÇÇrf"));
            }

            #[test]
            fn blo_u221a_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo√rf\n", "Blo√rf"));
            }

            #[test]
            fn blo_u221a_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo√rf\"\n", "Blo√rf"));
            }

            #[test]
            fn blo_u221a_u221a_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo√√rf\"\n", "Blo√√rf"));
            }

            #[test]
            fn blo_u221a_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf√\"\n", "Blorf√"));
            }

            #[test]
            fn u221a_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo√√rf\"\n", "Blo√√rf"));
            }

            #[test]
            fn blo_u25ca_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo◊rf\n", "Blo◊rf"));
            }

            #[test]
            fn blo_u25ca_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo◊rf\"\n", "Blo◊rf"));
            }

            #[test]
            fn blo_u25ca_u25ca_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo◊◊rf\"\n", "Blo◊◊rf"));
            }

            #[test]
            fn blo_u25ca_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf◊\"\n", "Blorf◊"));
            }

            #[test]
            fn u25ca_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo◊◊rf\"\n", "Blo◊◊rf"));
            }

            #[test]
            fn blo_u222b_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo∫rf\n", "Blo∫rf"));
            }

            #[test]
            fn blo_u222b_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo∫rf\"\n", "Blo∫rf"));
            }

            #[test]
            fn blo_u222b_u222b_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo∫∫rf\"\n", "Blo∫∫rf"));
            }

            #[test]
            fn blo_u222b_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf∫\"\n", "Blorf∫"));
            }

            #[test]
            fn u222b_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo∫∫rf\"\n", "Blo∫∫rf"));
            }

            #[test]
            fn blo_u0131_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloırf\n", "Bloırf"));
            }

            #[test]
            fn blo_u0131_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloırf\"\n", "Bloırf"));
            }

            #[test]
            fn blo_u0131_u0131_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloıırf\"\n", "Bloıırf"));
            }

            #[test]
            fn blo_u0131_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfı\"\n", "Blorfı"));
            }

            #[test]
            fn u0131_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloıırf\"\n", "Bloıırf"));
            }

            #[test]
            fn blo_u02dc_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo˜rf\n", "Blo˜rf"));
            }

            #[test]
            fn blo_u02dc_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo˜rf\"\n", "Blo˜rf"));
            }

            #[test]
            fn blo_u02dc_u02dc_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo˜˜rf\"\n", "Blo˜˜rf"));
            }

            #[test]
            fn blo_u02dc_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf˜\"\n", "Blorf˜"));
            }

            #[test]
            fn u02dc_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo˜˜rf\"\n", "Blo˜˜rf"));
            }

            #[test]
            fn blo_u00b5_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Bloµrf\n", "Bloµrf"));
            }

            #[test]
            fn blo_u00b5_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Bloµrf\"\n", "Bloµrf"));
            }

            #[test]
            fn blo_u00b5_u00b5_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Bloµµrf\"\n", "Bloµµrf"));
            }

            #[test]
            fn blo_u00b5_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorfµ\"\n", "Blorfµ"));
            }

            #[test]
            fn u00b5_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Bloµµrf\"\n", "Bloµµrf"));
            }

            #[test]
            fn blo_u00c2_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n BloÂrf\n", "BloÂrf"));
            }

            #[test]
            fn blo_u00c2_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"BloÂrf\"\n", "BloÂrf"));
            }

            #[test]
            fn blo_u00c2_u00c2_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"BloÂÂrf\"\n", "BloÂÂrf"));
            }

            #[test]
            fn blo_u00c2_rf_ends_with() {
                assert!(defines("erDiagram\n \"BlorfÂ\"\n", "BlorfÂ"));
            }

            #[test]
            fn u00c2_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"BloÂÂrf\"\n", "BloÂÂrf"));
            }

            #[test]
            fn blo_u2264_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo≤rf\n", "Blo≤rf"));
            }

            #[test]
            fn blo_u2264_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo≤rf\"\n", "Blo≤rf"));
            }

            #[test]
            fn blo_u2264_u2264_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo≤≤rf\"\n", "Blo≤≤rf"));
            }

            #[test]
            fn blo_u2264_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf≤\"\n", "Blorf≤"));
            }

            #[test]
            fn u2264_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo≤≤rf\"\n", "Blo≤≤rf"));
            }

            #[test]
            fn blo_u00af_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo¯rf\n", "Blo¯rf"));
            }

            #[test]
            fn blo_u00af_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo¯rf\"\n", "Blo¯rf"));
            }

            #[test]
            fn blo_u00af_u00af_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo¯¯rf\"\n", "Blo¯¯rf"));
            }

            #[test]
            fn blo_u00af_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf¯\"\n", "Blorf¯"));
            }

            #[test]
            fn u00af_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo¯¯rf\"\n", "Blo¯¯rf"));
            }

            #[test]
            fn blo_u2265_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo≥rf\n", "Blo≥rf"));
            }

            #[test]
            fn blo_u2265_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo≥rf\"\n", "Blo≥rf"));
            }

            #[test]
            fn blo_u2265_u2265_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo≥≥rf\"\n", "Blo≥≥rf"));
            }

            #[test]
            fn blo_u2265_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf≥\"\n", "Blorf≥"));
            }

            #[test]
            fn u2265_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo≥≥rf\"\n", "Blo≥≥rf"));
            }

            #[test]
            fn blo_u02d8_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo˘rf\n", "Blo˘rf"));
            }

            #[test]
            fn blo_u02d8_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo˘rf\"\n", "Blo˘rf"));
            }

            #[test]
            fn blo_u02d8_u02d8_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo˘˘rf\"\n", "Blo˘˘rf"));
            }

            #[test]
            fn blo_u02d8_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf˘\"\n", "Blorf˘"));
            }

            #[test]
            fn u02d8_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo˘˘rf\"\n", "Blo˘˘rf"));
            }

            #[test]
            fn blo_u00f7_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo÷rf\n", "Blo÷rf"));
            }

            #[test]
            fn blo_u00f7_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo÷rf\"\n", "Blo÷rf"));
            }

            #[test]
            fn blo_u00f7_u00f7_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo÷÷rf\"\n", "Blo÷÷rf"));
            }

            #[test]
            fn blo_u00f7_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf÷\"\n", "Blorf÷"));
            }

            #[test]
            fn u00f7_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo÷÷rf\"\n", "Blo÷÷rf"));
            }

            #[test]
            fn blo_u00bf_rf_fails_if_not_surrounded_by_quotes() {
                assert!(throws_or_defines("erDiagram\n Blo¿rf\n", "Blo¿rf"));
            }

            #[test]
            fn blo_u00bf_rf_single_occurrence() {
                assert!(defines("erDiagram\n \"Blo¿rf\"\n", "Blo¿rf"));
            }

            #[test]
            fn blo_u00bf_u00bf_rf_repeated_occurrence() {
                assert!(defines("erDiagram\n \"Blo¿¿rf\"\n", "Blo¿¿rf"));
            }

            #[test]
            fn blo_u00bf_rf_ends_with() {
                assert!(defines("erDiagram\n \"Blorf¿\"\n", "Blorf¿"));
            }

            #[test]
            fn u00bf_blorf_cannot_start_with_the_character() {
                assert!(throws_or_defines("erDiagram\n \"Blo¿¿rf\"\n", "Blo¿¿rf"));
            }

            #[test]
            fn a_tilde_backtick_bang_at_hash_dollar_caret_ampersand_asterisk_lparen_rparen_dash_equals_plus_lbracket_rbracket_lbrace_rbrace_pipe_slash_semicolon_colon_apostrophe_dot_question_u00a1_u2044_u2122_u20ac_u00a3_u2039_u00a2_u203a_u221e_ufb01_u00a7_u2021_u2022_u00b0_u00aa_u00b7_u00ba_u201a_u2260_u00b1_u0153_u0152_u2211_u201e_u00ae_u2020_u02c7_u00a5_u00c1_u00a8_u02c6_u02c6_u00d8_u03c0_u220f_u201c_u00ab_u00bb_u00e5_u00c5_u00df_u00cd_u2202_u00ce_u0192_u00cf_u00a9_u02d9_u00d3_u2206_u00d4_u02da_uf8ff_u00ac_u00d2_u2026_u00da_u00e6_u00c6_u03a9_u00b8_u2248_u03c0_u02db_u00e7_u00c7_u221a_u25ca_u222b_u0131_u02dc_u00b5_u00c2_u2264_u00af_u2265_u02d8_u00f7_u00bf_all_non_alphanumerics_in_one_starting_with_a()
             {
                assert!(defines(
                    "erDiagram\n \"a~`!@#$^&*()-=+[]{}|/;:'.?¡⁄™€£‹¢›∞ﬁ§‡•°ª·º‚≠±œŒ∑„®†ˇ¥Á¨ˆˆØπ∏“«»åÅßÍ∂ÎƒÏ©˙Ó∆Ô˚¬Ò…ÚæÆΩ¸≈π˛çÇ√◊∫ı˜µÂ≤¯≥˘÷¿\"\n",
                    "a~`!@#$^&*()-=+[]{}|/;:'.?¡⁄™€£‹¢›∞ﬁ§‡•°ª·º‚≠±œŒ∑„®†ˇ¥Á¨ˆˆØπ∏“«»åÅßÍ∂ÎƒÏ©˙Ó∆Ô˚¬Ò…ÚæÆΩ¸≈π˛çÇ√◊∫ı˜µÂ≤¯≥˘÷¿"
                ));
            }
        }
    }
}
