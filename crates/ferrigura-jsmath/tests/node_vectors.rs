//! The `node` module (V8 13.6, Node 24's fdlibm) against Node, bit for bit, on the samples in
//! `tests/data/node`, taken from a probe run of a million inputs per function.
//!
//! The fdlibm functions fuse multiply-adds where the target has them, as V8's compiler does, so
//! they are checked on those targets only (see `CONTRACTS`). `pow` is the platform's `pow` in V8 by
//! default, so it is checked on macOS, where the samples were taken. `hypot` is V8's plain
//! arithmetic, the same everywhere.

mod support;

use ferrigura_jsmath::node;
use support::{check_binary, check_unary};

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn sin_matches_node() {
    check_unary("sin", include_str!("data/node/sin.txt"), node::sin);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn cos_matches_node() {
    check_unary("cos", include_str!("data/node/cos.txt"), node::cos);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn tan_matches_node() {
    check_unary("tan", include_str!("data/node/tan.txt"), node::tan);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn asin_matches_node() {
    check_unary("asin", include_str!("data/node/asin.txt"), node::asin);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn acos_matches_node() {
    check_unary("acos", include_str!("data/node/acos.txt"), node::acos);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn atan_matches_node() {
    check_unary("atan", include_str!("data/node/atan.txt"), node::atan);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn log_matches_node() {
    check_unary("log", include_str!("data/node/log.txt"), node::log);
}

#[cfg(any(target_arch = "aarch64", target_feature = "fma"))]
#[test]
fn atan2_matches_node() {
    check_binary("atan2", include_str!("data/node/atan2.txt"), node::atan2);
}

#[cfg(target_os = "macos")]
#[test]
fn pow_matches_node() {
    check_binary("pow", include_str!("data/node/pow.txt"), node::pow);
}

#[test]
fn hypot_matches_node() {
    check_binary("hypot", include_str!("data/node/hypot.txt"), node::hypot);
}
