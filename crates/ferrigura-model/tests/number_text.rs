//! `number_text` against JavaScript's `String(number)` on the table in `tests/data/number_text.txt`,
//! written by Node (see `tools/port/number-text-table.ts`): the special values, the limits of the
//! plain notation, every decimal exponent, random bit patterns and the short decimals of diagrams.

use ferrigura_model::value::number_text;

#[test]
fn numbers_are_written_as_javascript_writes_them() {
    let table = include_str!("data/number_text.txt");
    let mut checked = 0;
    let mut wrong = Vec::new();
    for line in table.lines().filter(|line| !line.starts_with('#')) {
        let (bits, expected) = line.split_once(' ').expect("bits and text");
        let bits = u64::from_str_radix(bits.trim_start_matches("0x"), 16).expect("hex bits");
        let actual = number_text(f64::from_bits(bits));
        if actual != expected {
            wrong.push(format!("{bits:#018x}: {actual} != {expected}"));
        }
        checked += 1;
    }
    assert!(checked > 5000, "the table has {checked} lines");
    assert!(
        wrong.is_empty(),
        "{} of {checked} differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

#[test]
fn tiny_and_huge_numbers_take_an_exponent() {
    assert_eq!(
        number_text(1.836_970_198_721_029_7e-14),
        "1.8369701987210297e-14"
    );
    assert_eq!(number_text(-0.000_000_1), "-1e-7");
    assert_eq!(number_text(0.000_001), "0.000001");
    assert_eq!(number_text(1e21), "1e+21");
    assert_eq!(
        number_text(999_999_999_999_999_900_000.0),
        "999999999999999900000"
    );
    assert_eq!(number_text(-0.0), "0");
}
