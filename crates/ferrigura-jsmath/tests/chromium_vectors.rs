//! The crate's default functions against Chromium 151 (Playwright's), bit for bit, on the samples in
//! `tests/data/chromium`, taken from a probe run of a million inputs per function.
//!
//! `sin`, `cos`, `tan`, `asin`, `acos` and `log` are correctly rounded in Chromium on every probed
//! input, and so are they here (see `src/correct.rs`): their results do not depend on the target.
//! `atan` and `atan2` are llvm-libc's, which fuse multiply-adds where the target has them, so their
//! samples are checked on those targets only (see `CONTRACTS`). `pow` is the platform's `pow` in
//! Chromium on macOS, where the samples were taken, so it is checked there only. `hypot` is V8's plain
//! arithmetic, the same everywhere.

mod support;

use ferrigura_jsmath as jsmath;
use support::{check_binary, check_unary};

#[test]
fn sin_matches_chromium() {
    check_unary("sin", include_str!("data/chromium/sin.txt"), jsmath::sin);
}

#[test]
fn cos_matches_chromium() {
    check_unary("cos", include_str!("data/chromium/cos.txt"), jsmath::cos);
}

#[test]
fn tan_matches_chromium() {
    check_unary("tan", include_str!("data/chromium/tan.txt"), jsmath::tan);
}

#[test]
fn asin_matches_chromium() {
    check_unary("asin", include_str!("data/chromium/asin.txt"), jsmath::asin);
}

#[test]
fn acos_matches_chromium() {
    check_unary("acos", include_str!("data/chromium/acos.txt"), jsmath::acos);
}

#[test]
fn log_matches_chromium() {
    check_unary("log", include_str!("data/chromium/log.txt"), jsmath::log);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn atan_matches_chromium() {
    check_unary("atan", include_str!("data/chromium/atan.txt"), jsmath::atan);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn atan2_matches_chromium() {
    check_binary(
        "atan2",
        include_str!("data/chromium/atan2.txt"),
        jsmath::atan2,
    );
}

#[cfg(target_os = "macos")]
#[test]
fn pow_matches_chromium() {
    check_binary("pow", include_str!("data/chromium/pow.txt"), jsmath::pow);
}

#[test]
fn hypot_matches_chromium() {
    check_binary(
        "hypot",
        include_str!("data/chromium/hypot.txt"),
        jsmath::hypot,
    );
}
