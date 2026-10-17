//! The sample checks shared by the vector tests: the samples are the probe's (`tools/port/jsmath-probe.ts`),
//! one case per line, `x result` or `x y result`, in hexadecimal, with `#` comment lines.

/// The cases of a sample, as bit patterns.
pub fn cases(text: &str) -> Vec<Vec<u64>> {
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            line.split_whitespace()
                .map(|word| {
                    u64::from_str_radix(word.trim_start_matches("0x"), 16)
                        .unwrap_or_else(|error| panic!("bad word {word}: {error}"))
                })
                .collect()
        })
        .collect()
}

/// Whether a result matches the expected one: all bits, or both NaN.
fn matches(expected: u64, actual: u64) -> bool {
    expected == actual || (f64::from_bits(expected).is_nan() && f64::from_bits(actual).is_nan())
}

pub fn check_unary(name: &str, text: &str, function: fn(f64) -> f64) {
    let rows = cases(text);
    let mut failures = Vec::new();
    for row in &rows {
        assert_eq!(
            row.len(),
            2,
            "{name}: a unary case has an input and a result"
        );
        let got = function(f64::from_bits(row[0])).to_bits();
        if !matches(row[1], got) {
            failures.push(format!(
                "x={:#018x}: expected {:#018x}, got {got:#018x}",
                row[0], row[1]
            ));
        }
    }
    assert!(
        rows.len() > 2000,
        "{name}: the sample has {} cases",
        rows.len()
    );
    assert!(
        failures.is_empty(),
        "{name}: {} of {} differ, first {:?}",
        failures.len(),
        rows.len(),
        failures.first()
    );
}

pub fn check_binary(name: &str, text: &str, function: fn(f64, f64) -> f64) {
    let rows = cases(text);
    let mut failures = Vec::new();
    for row in &rows {
        assert_eq!(
            row.len(),
            3,
            "{name}: a binary case has two inputs and a result"
        );
        let got = function(f64::from_bits(row[0]), f64::from_bits(row[1])).to_bits();
        if !matches(row[2], got) {
            failures.push(format!(
                "x={:#018x} y={:#018x}: expected {:#018x}, got {got:#018x}",
                row[0], row[1], row[2]
            ));
        }
    }
    assert!(
        rows.len() > 2000,
        "{name}: the sample has {} cases",
        rows.len()
    );
    assert!(
        failures.is_empty(),
        "{name}: {} of {} differ, first {:?}",
        failures.len(),
        rows.len(),
        failures.first()
    );
}
