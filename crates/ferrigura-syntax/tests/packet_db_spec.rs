//! The packet DB's quirks, as the reference keeps them (`docs-port/packet/PLAN.md`): the `bits` of a
//! split block, the row cap, the configuration that sets the row width, the title precedence, the
//! header texts through the sanitiser, and the labels as the DB holds them.

use ferrigura_model::packet::{PacketBlock, PacketDiagram};
use ferrigura_syntax::packet::parse;

fn block(start: f64, end: f64, bits: f64, label: &str) -> PacketBlock {
    PacketBlock {
        start,
        end,
        bits,
        label: label.to_owned(),
    }
}

fn accepts(text: &str) -> PacketDiagram {
    parse(text).unwrap_or_else(|error| panic!("{text:?} should parse: {error}"))
}

#[test]
fn a_split_block_carries_one_less_than_its_width() {
    // Quirk 1: `parser.ts:82` and `88` give `bits` = width - 1 for both pieces.
    let diagram = accepts("packet\n0-40: \"wide\"\n");
    assert_eq!(
        diagram.rows,
        vec![
            vec![block(0.0, 31.0, 31.0, "wide")],
            vec![block(32.0, 40.0, 8.0, "wide")],
        ]
    );
}

#[test]
fn a_block_that_ends_on_a_row_boundary_is_not_split() {
    let diagram = accepts("packet\n0-31: \"a\"\n32-63: \"b\"\n64-95: \"c\"\n");
    assert_eq!(diagram.rows.len(), 3);
    assert!(diagram.rows.iter().all(|row| row.len() == 1));
}

#[test]
fn rows_past_the_cap_are_dropped_without_an_error() {
    // Quirk 2: 10,000 rows at one bit each; the bits after them are gone.
    let diagram = accepts(
        "---
config:
  packet:
    bitsPerRow: 1
---
packet
0-10010: \"x\"
10011: \"y\"
",
    );
    assert_eq!(diagram.rows.len(), 10_000);
    let last = diagram.rows.last().expect("a row");
    assert_eq!(last, &vec![block(9999.0, 9999.0, 0.0, "x")]);
}

#[test]
fn blocks_past_the_cap_are_still_validated() {
    let error = parse(
        "---
config:
  packet:
    bitsPerRow: 1
---
packet
0-10010: \"x\"
5-3: \"bad\"
",
    )
    .expect_err("the block after the cap is still checked");
    assert_eq!(
        error.message,
        "Packet block 5 - 3 is invalid. End must be greater than start."
    );
}

#[test]
fn bits_per_row_comes_from_the_front_matter() {
    let diagram = accepts(
        "---
config:
  packet:
    bitsPerRow: 16
---
packet
0-40: \"wide\"
",
    );
    assert_eq!(
        diagram.rows,
        vec![
            vec![block(0.0, 15.0, 15.0, "wide")],
            vec![block(16.0, 31.0, 15.0, "wide")],
            vec![block(32.0, 40.0, 8.0, "wide")],
        ]
    );
}

#[test]
fn bits_per_row_comes_from_a_packet_directive_only() {
    let directive =
        accepts("%%{init: {\"packet\": {\"bitsPerRow\": 16}}}%%\npacket\n0-40: \"wide\"\n");
    assert_eq!(directive.rows.len(), 3);
    // A top-level key is not the packet section, so the default of 32 holds.
    let top = accepts("%%{init: {\"bitsPerRow\": 16}}%%\npacket\n0-40: \"wide\"\n");
    assert_eq!(top.rows.len(), 2);
}

#[test]
fn a_zero_bits_per_row_gives_rows_that_end_before_they_start() {
    // Not a case of the spec: the reference's arithmetic at `bitsPerRow` 0, reproduced as it is.
    let diagram = accepts(
        "---
config:
  packet:
    bitsPerRow: 0
---
packet
0-3: \"x\"
",
    );
    assert_eq!(diagram.rows.len(), 10_000);
    assert_eq!(diagram.rows[0], vec![block(0.0, -1.0, -1.0, "x")]);
}

#[test]
fn an_infinite_bits_per_row_puts_the_block_in_one_row_as_a_sentinel() {
    let diagram = accepts(
        "---
config:
  packet:
    bitsPerRow: .inf
---
packet
+1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000: \"x\"
",
    );
    assert_eq!(diagram.rows.len(), 1);
    assert_eq!(diagram.rows[0][0].start, 0.0);
    assert_eq!(diagram.rows[0][0].end, f64::INFINITY);
    let json = serde_json::to_value(&diagram).expect("the model serialises");
    assert_eq!(json["rows"][0][0]["end"], serde_json::json!("Infinity"));
}

#[test]
fn title_precedence_is_the_grammars_title_then_the_front_matter() {
    let grammar = accepts("---\ntitle: FM\n---\npacket\ntitle Text\n0-7: \"a\"\n");
    assert_eq!(grammar.title, "Text");
    let front = accepts("---\ntitle: FM\n---\npacket\n0-7: \"a\"\n");
    assert_eq!(front.title, "FM");
    let empty = accepts("---\ntitle: FM\n---\npacket\ntitle\n0-7: \"a\"\n");
    assert_eq!(empty.title, "FM");
    let last_empty = accepts("packet\ntitle Text\ntitle\n0-7: \"a\"\n");
    assert_eq!(last_empty.title, "");
}

#[test]
fn the_header_texts_go_through_the_sanitiser() {
    let diagram = accepts(
        "packet
title <b>T</b> &amp;
accTitle:    spaced   title  
accDescr {
  line one

  line two
}
0-7: \"a\"
",
    );
    assert_eq!(diagram.title, "<b>T</b> &amp;");
    assert_eq!(diagram.acc_title, "spaced title");
    assert_eq!(diagram.acc_descr, "line one\nline two");
    let script = accepts("packet\ntitle <script>alert(1)</script>Hi\n0-7: \"a\"\n");
    assert_eq!(script.title, "Hi");
}

#[test]
fn labels_keep_their_entity_placeholders_and_markup() {
    let diagram = accepts("packet\n0-7: \"#amp; #35; <b>bold</b> & x\"\n");
    assert_eq!(diagram.rows[0][0].label, "ﬂ°amp¶ß ﬂ°°35¶ß <b>bold</b> & x");
}

#[test]
fn a_front_matter_title_is_sanitised_as_the_grammars_title_is() {
    let diagram = accepts("---\ntitle: \"<b>FM</b> & x\"\n---\npacket\n0-7: \"a\"\n");
    assert_eq!(diagram.title, "<b>FM</b> &amp; x");
}
