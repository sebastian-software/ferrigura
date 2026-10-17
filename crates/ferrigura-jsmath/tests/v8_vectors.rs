//! The crate against V8's `Math` functions, bit for bit, on the samples that
//! `tools/port/jsmath-probe.ts sample` writes from a run of the probe.
//!
//! The samples are Node 24.21.0 (V8 13.6.233.17) on arm64 macOS, a sample of a million inputs per
//! function in segments that cover the argument ranges of the layouts, the special values and the
//! thresholds of fdlibm. A NaN result matches any NaN; every other result must match in all 64 bits.
//!
//! V8 builds its fdlibm code with the compiler's default contraction, which fuses multiply-adds on
//! targets with fused multiply-add instructions. The crate does the same there (see
//! `src/fused.rs`), so the fdlibm functions are checked on those targets only; on the others the
//! same tests would need samples from a V8 built without fused multiply-adds. `pow` is the
//! platform's `pow` in V8 by default, so its samples are checked on macOS only, where they were
//! taken. `hypot` is V8's plain arithmetic, the same on every target.

use ferrigura_jsmath as jsmath;

/// The lines of a sample, as bit patterns: the comment line is skipped.
fn cases(text: &str) -> Vec<Vec<u64>> {
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

fn check_unary(name: &str, text: &str, function: fn(f64) -> f64) {
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
                "x={:#018x}: V8 {:#018x}, crate {got:#018x}",
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

fn check_binary(name: &str, text: &str, function: fn(f64, f64) -> f64) {
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
                "x={:#018x} y={:#018x}: V8 {:#018x}, crate {got:#018x}",
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

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn sin_matches_v8() {
    check_unary("sin", include_str!("data/sin.txt"), jsmath::sin);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn cos_matches_v8() {
    check_unary("cos", include_str!("data/cos.txt"), jsmath::cos);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn tan_matches_v8() {
    check_unary("tan", include_str!("data/tan.txt"), jsmath::tan);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn asin_matches_v8() {
    check_unary("asin", include_str!("data/asin.txt"), jsmath::asin);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn acos_matches_v8() {
    check_unary("acos", include_str!("data/acos.txt"), jsmath::acos);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn atan_matches_v8() {
    check_unary("atan", include_str!("data/atan.txt"), jsmath::atan);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn log_matches_v8() {
    check_unary("log", include_str!("data/log.txt"), jsmath::log);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn atan2_matches_v8() {
    check_binary("atan2", include_str!("data/atan2.txt"), jsmath::atan2);
}

#[cfg(target_os = "macos")]
#[test]
fn pow_matches_v8() {
    check_binary("pow", include_str!("data/pow.txt"), jsmath::pow);
}

#[test]
fn hypot_matches_v8() {
    check_binary("hypot", include_str!("data/hypot.txt"), jsmath::hypot);
}
