//! Port of `packages/mermaid/src/diagrams/packet/packet.spec.ts`: the parser and the DB (the
//! rows), and the errors of the DB. Test names follow the spec's titles in snake_case. The model
//! carries the DB state, so the expectations are the rows the spec snapshots.

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

/// The message of the error a text is rejected with.
fn rejected(text: &str) -> String {
    match parse(text) {
        Ok(diagram) => panic!("{text:?} should be rejected, got {diagram:?}"),
        Err(error) => error.message,
    }
}

mod parse_spec {
    use super::*;

    #[test]
    fn should_handle_a_packet_definition() {
        let diagram = accepts("packet");
        assert!(diagram.rows.is_empty());
    }

    #[test]
    fn should_handle_diagram_with_data_and_title() {
        let diagram = accepts(
            "packet
    title Packet diagram
    accTitle: Packet accTitle
    accDescr: Packet accDescription
    0-10: \"test\"
    ",
        );
        assert_eq!(diagram.title, "Packet diagram");
        assert_eq!(diagram.acc_title, "Packet accTitle");
        assert_eq!(diagram.acc_descr, "Packet accDescription");
        assert_eq!(diagram.rows, vec![vec![block(0.0, 10.0, 11.0, "test")]]);
    }

    #[test]
    fn should_handle_single_bits() {
        let diagram = accepts(
            "packet
    0-10: \"test\"
    11: \"single\"
    ",
        );
        assert_eq!(
            diagram.rows,
            vec![vec![
                block(0.0, 10.0, 11.0, "test"),
                block(11.0, 11.0, 1.0, "single"),
            ]]
        );
    }

    #[test]
    fn should_handle_bit_counts() {
        let diagram = accepts(
            "packet
    +8: \"byte\"
    +16: \"word\"
    ",
        );
        assert_eq!(
            diagram.rows,
            vec![vec![
                block(0.0, 7.0, 8.0, "byte"),
                block(8.0, 23.0, 16.0, "word"),
            ]]
        );
    }

    #[test]
    fn should_handle_bit_counts_with_bit_or_bits() {
        let diagram = accepts(
            "packet
    +8: \"byte\"
    +16: \"word\"
    ",
        );
        assert_eq!(
            diagram.rows,
            vec![vec![
                block(0.0, 7.0, 8.0, "byte"),
                block(8.0, 23.0, 16.0, "word"),
            ]]
        );
    }

    #[test]
    fn should_split_into_multiple_rows() {
        let diagram = accepts(
            "packet
    0-10: \"test\"
    11-90: \"multiple\"
    ",
        );
        assert_eq!(
            diagram.rows,
            vec![
                vec![
                    block(0.0, 10.0, 11.0, "test"),
                    block(11.0, 31.0, 20.0, "multiple"),
                ],
                vec![block(32.0, 63.0, 31.0, "multiple")],
                vec![block(64.0, 90.0, 26.0, "multiple")],
            ]
        );
    }

    #[test]
    fn should_split_into_multiple_rows_when_cut_at_exact_length() {
        let diagram = accepts(
            "packet
    0-16: \"test\"
    17-63: \"multiple\"
    ",
        );
        assert_eq!(
            diagram.rows,
            vec![
                vec![
                    block(0.0, 16.0, 17.0, "test"),
                    block(17.0, 31.0, 14.0, "multiple"),
                ],
                vec![block(32.0, 63.0, 31.0, "multiple")],
            ]
        );
    }
}

mod errors_spec {
    use super::*;

    #[test]
    fn should_throw_error_if_numbers_are_not_continuous() {
        assert_eq!(
            rejected(
                "packet
    0-16: \"test\"
    18-20: \"error\"
    "
            ),
            "Packet block 18 - 20 is not contiguous. It should start from 17."
        );
    }

    #[test]
    fn should_throw_error_if_numbers_are_not_continuous_with_bit_counts() {
        assert_eq!(
            rejected(
                "packet
    +16: \"test\"
    18-20: \"error\"
    "
            ),
            "Packet block 18 - 20 is not contiguous. It should start from 16."
        );
    }

    #[test]
    fn should_throw_error_if_numbers_are_not_continuous_for_single_packets() {
        assert_eq!(
            rejected(
                "packet
    0-16: \"test\"
    18: \"error\"
    "
            ),
            "Packet block 18 - 18 is not contiguous. It should start from 17."
        );
    }

    #[test]
    fn should_throw_error_if_numbers_are_not_continuous_for_single_packets_with_bit_counts() {
        assert_eq!(
            rejected(
                "packet
    +16: \"test\"
    18: \"error\"
    "
            ),
            "Packet block 18 - 18 is not contiguous. It should start from 16."
        );
    }

    #[test]
    fn should_throw_error_if_numbers_are_not_continuous_for_single_packets_2() {
        assert_eq!(
            rejected(
                "packet
    0-16: \"test\"
    17: \"good\"
    19: \"error\"
    "
            ),
            "Packet block 19 - 19 is not contiguous. It should start from 18."
        );
    }

    #[test]
    fn should_throw_error_if_end_is_less_than_start() {
        assert_eq!(
            rejected(
                "packet
    0-16: \"test\"
    25-20: \"error\"
    "
            ),
            "Packet block 25 - 20 is invalid. End must be greater than start."
        );
    }

    #[test]
    fn should_throw_error_if_bit_count_is_0() {
        assert_eq!(
            rejected(
                "packet
    +0: \"test\"
    "
            ),
            "Packet block 0 is invalid. Cannot have a zero bit field."
        );
    }
}
