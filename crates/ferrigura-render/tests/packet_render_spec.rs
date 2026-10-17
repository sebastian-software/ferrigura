//! Port of `packages/mermaid/src/diagrams/packet/renderer.spec.ts`. Test names follow the spec's
//! titles in snake_case. The spec reads the rendered SVG back (the x position and width of each
//! block, its bit numbers and their anchors), which is the packet renderer's output. The renderer
//! is not ported yet, so every test is ignored with that reason; the rows the renderer draws are
//! checked by the parser's model in `ferrigura-syntax`'s `packet_spec`.

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn lays_fields_out_from_bit_0_each_numbered_from_its_start_bit() {}

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn renders_an_explicit_ascending_bit_order_exactly_like_the_default() {}

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn mirrors_the_row_so_it_reads_from_the_most_significant_bit_down() {}

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn numbers_a_row_of_mixed_width_fields_straight_down_from_its_highest_bit() {}

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn keeps_every_field_the_width_it_has_in_ascending_order() {}

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn centres_the_number_on_a_single_bit_field_as_ascending_order_does() {}

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn mirrors_each_row_of_a_multi_row_packet_on_its_own() {}

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn mirrors_within_bits_per_row_so_a_part_filled_row_is_flush_right() {}

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn draws_no_bit_numbers_with_show_bits_false_and_bit_order_ascending() {}

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn draws_no_bit_numbers_with_show_bits_false_and_bit_order_descending() {}

#[test]
#[ignore = "port: the packet renderer is not ported (docs-port/packet/NOTES.md, section 5.2)"]
fn still_mirrors_the_row_with_show_bits_false() {}
