//! Port of `packages/parser/tests/packet.test.ts`, and the grammar's rules of `packet.langium` that
//! the reference's probes fixed (`docs-port/packet/NOTES.md`, section 2.3): the keyword boundary,
//! the unsigned numbers, the same-line block, the blocks that need the end of their line, and the
//! labels that span lines.

use ferrigura_model::packet::PacketDiagram;
use ferrigura_syntax::packet::parse;

fn accepts(input: &str) -> PacketDiagram {
    parse(input).unwrap_or_else(|error| panic!("{input:?} should parse: {error}"))
}

fn rejects(input: &str) {
    assert!(parse(input).is_err(), "{input:?} should be rejected");
}

#[test]
fn should_handle_regular_packet() {
    for input in ["packet", "  packet  ", "\tpacket\t", "\n    \tpacket\n    "] {
        assert!(accepts(input).rows.is_empty(), "{input:?}");
    }
}

#[test]
fn keyword_needs_a_boundary() {
    for input in [
        "packet0-7: \"a\"",
        "packet-0-7: \"a\"",
        "packet+8: \"a\"",
        "packet\"x\"",
        "packet;",
        "PACKET\n0-7: \"a\"",
    ] {
        rejects(input);
    }
    assert!(accepts("packet%%c\n0-7: \"a\"").rows.len() == 1);
    assert!(accepts("packet 0-7: \"a\"").rows.len() == 1);
}

#[test]
fn numbers_are_unsigned_and_have_no_leading_zero() {
    rejects("packet\n0.5: \"x\"\n");
    rejects("packet\n01: \"x\"\n");
    rejects("packet\n-1: \"x\"\n");
    accepts("packet\n0-10: \"a\"\n");
    accepts("packet\n0: \"a\"\n");
    accepts("packet\n+10: \"a\"\n");
}

#[test]
fn block_needs_colon_quoted_label_and_end_of_line() {
    rejects("packet\n0-7 \"x\"\n");
    rejects("packet\n0-7: x\n");
    rejects("packet\n0-7: \"a\" 8-15: \"b\"\n");
    rejects("packet\n0-7: \"a\"\n5-3 \"x\"\n");
    accepts("packet\n0-7: 'single'\n");
}

#[test]
fn a_block_may_follow_the_keyword_on_its_line() {
    assert_eq!(accepts("packet 0-7: \"a\"").rows[0].len(), 1);
}

#[test]
fn a_line_break_in_a_label_is_kept() {
    let diagram = accepts("packet\n0-7: \"a\nb\"\n");
    assert_eq!(diagram.rows[0][0].label, "a\nb");
}

#[test]
fn escapes_in_a_label_are_resolved() {
    let diagram = accepts("packet\n0-7: \"a\\nb\\tc\\\"d\"\n");
    assert_eq!(diagram.rows[0][0].label, "a\nb\tc\"d");
    assert_eq!(accepts("packet\n0-7: \"\"\n").rows[0][0].label, "");
}

#[test]
fn crlf_and_tabs_are_accepted() {
    assert_eq!(accepts("packet\r\n0-7: \"a\"\r\n").rows[0].len(), 1);
    assert_eq!(accepts("packet\n0-7:\t\"tab\"\n").rows[0].len(), 1);
}

#[test]
fn a_comment_is_hidden_after_a_block() {
    let diagram = accepts("packet\n0-7: \"a\" %% trailing\n8-15: \"b\"\n");
    assert_eq!(diagram.rows[0].len(), 2);
}

#[test]
fn a_comment_line_before_the_blocks_is_hidden() {
    assert_eq!(accepts("packet\n%% note\n0-7: \"a\"\n").rows[0].len(), 1);
}

#[test]
fn a_header_line_needs_its_end_of_line() {
    rejects("packet\naccDescr x\n0-7: \"a\"\n");
    rejects("packet\naccTitle x\n0-7: \"a\"\n");
    rejects("packet\nnumber\n0-7: \"a\"\n");
}
