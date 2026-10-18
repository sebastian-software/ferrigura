//! The JavaScript semantics of `src/js.rs` against V8 (Node v24.21.0, V8 13.6, ICU 78.3, locale en-US).
//!
//! The expected values are in `tests/data`, written by `tools/js-semantics.mjs` (see its first comment for
//! the command). Lines starting with `#` are comments. Each test reports every mismatch, not just the first.

use std::cmp::Ordering;
use std::path::PathBuf;

use ferrigura_swimlane::js::{js_round, locale_compare, to_fixed};

/// The number of strings of printable ASCII of length 0 to 2: the empty string, 95 of length 1 and 95^2 of length 2.
const STRINGS_UP_TO_TWO: usize = 1 + 95 + 95 * 95;

/// The number of pairs in `ascii-pairs.txt`.
const PAIRS: usize = 200_000;

fn data(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "data", name]
        .iter()
        .collect();
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The data lines, without the comment lines.
fn rows(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
}

fn bits(hex: &str) -> u64 {
    u64::from_str_radix(hex, 16).unwrap_or_else(|error| panic!("bad bits {hex}: {error}"))
}

/// Reports the failures, if there are any, with a count and the first twenty.
fn report(what: &str, failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{what}: {} mismatches with V8; the first ones:\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn to_fixed_matches_v8() {
    let text = data("to-fixed.txt");
    let mut failures = Vec::new();
    let mut count = 0;
    for line in rows(&text) {
        let fields: Vec<&str> = line.split(' ').collect();
        assert_eq!(fields.len(), 3, "to-fixed row {line:?}");
        let x = f64::from_bits(bits(fields[0]));
        let digits: u32 = fields[1].parse().expect("digits");
        let expected = fields[2];
        let got = to_fixed(x, digits);
        if got != expected {
            failures.push(format!(
                "x={:#018x} ({x:e}) digits={digits}: expected {expected:?}, got {got:?}",
                bits(fields[0])
            ));
        }
        count += 1;
    }
    assert!(count >= 2000, "to-fixed has {count} cases");
    report("to_fixed", &failures);
}

#[test]
fn js_round_matches_v8() {
    let text = data("js-round.txt");
    let mut failures = Vec::new();
    let mut count = 0;
    for line in rows(&text) {
        let fields: Vec<&str> = line.split(' ').collect();
        assert_eq!(fields.len(), 2, "js-round row {line:?}");
        let x = f64::from_bits(bits(fields[0]));
        let expected = f64::from_bits(bits(fields[1]));
        let got = js_round(x);
        // A NaN is compared as a NaN; every other value, including the sign of zero, by its bits.
        let same = if expected.is_nan() {
            got.is_nan()
        } else {
            got.to_bits() == expected.to_bits()
        };
        if !same {
            failures.push(format!(
                "x={:#018x} ({x:e}): expected {expected:e}, got {got:e}",
                bits(fields[0])
            ));
        }
        count += 1;
    }
    assert!(count >= 2000, "js-round has {count} cases");
    report("js_round", &failures);
}

/// Every string of printable ASCII of length 0 to 2, in generation order (not sorted).
fn printable_strings_up_to_two() -> Vec<String> {
    let chars: Vec<char> = (0x20u8..=0x7e).map(char::from).collect();
    let mut all = vec![String::new()];
    for &a in &chars {
        all.push(a.to_string());
    }
    for &a in &chars {
        for &b in &chars {
            all.push(format!("{a}{b}"));
        }
    }
    all
}

#[test]
fn locale_compare_matches_v8_for_every_string_of_up_to_two_characters() {
    let v8: Vec<String> = rows(&data("ascii-sorted-len2.txt"))
        .map(|line| serde_json::from_str(line).unwrap_or_else(|error| panic!("{line}: {error}")))
        .collect();
    assert_eq!(v8.len(), STRINGS_UP_TO_TWO, "the list of V8's order");

    // The same strings, sorted by the port, give V8's sequence.
    let mut ours = printable_strings_up_to_two();
    assert_eq!(ours.len(), STRINGS_UP_TO_TWO);
    ours.sort_by(|a, b| locale_compare(a, b));
    let first_difference = ours.iter().zip(&v8).position(|(a, b)| a != b);
    assert_eq!(
        first_difference, None,
        "the port's sorted order differs from V8's at index {first_difference:?}"
    );

    // And each adjacent pair of V8's list is strictly ascending in the port.
    let mut failures = Vec::new();
    for pair in v8.windows(2) {
        if locale_compare(&pair[0], &pair[1]) != Ordering::Less {
            failures.push(format!("{:?} is not before {:?}", pair[0], pair[1]));
        }
    }
    report("locale_compare adjacency", &failures);
}

#[test]
fn locale_compare_matches_v8_on_random_pairs() {
    let text = data("ascii-pairs.txt");
    let mut failures = Vec::new();
    let mut count = 0;
    for line in rows(&text) {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 3, "pair row {line:?}");
        let a: String = serde_json::from_str(fields[0]).expect("a JSON string");
        let b: String = serde_json::from_str(fields[1]).expect("a JSON string");
        let expected: i8 = fields[2].parse().expect("a sign");
        let got = match locale_compare(&a, &b) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        };
        if got != expected {
            failures.push(format!("{a:?} vs {b:?}: expected {expected}, got {got}"));
        }
        count += 1;
    }
    assert_eq!(count, PAIRS, "pairs in ascii-pairs.txt");
    report("locale_compare pairs", &failures);
}
