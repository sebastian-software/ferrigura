//! Evaluates the crate's functions on the inputs the V8 probe wrote, and writes the results as bit
//! patterns, for `tools/port/jsmath-probe.ts --compare`.
//!
//! Usage: `cargo run --release -p ferrigura-jsmath --example evaluate -- <dir> [--std]`
//!
//! For each function `f`, it reads `<dir>/f.in` (little-endian `u64` bit patterns: one per input for
//! the unary functions, two (`x`, `y`) for the binary ones) and writes `<dir>/f.rs` (one `u64` per
//! input), with NaN written as the canonical quiet NaN, as the probe does. With `--std` it writes
//! `<dir>/f.std` instead, from the platform's own functions (the `f64` methods), for comparison.

use std::fs;
use std::path::{Path, PathBuf};

use ferrigura_jsmath as jsmath;

type UnaryFn = fn(f64) -> f64;
type BinaryFn = fn(f64, f64) -> f64;

const UNARY: [(&str, UnaryFn); 7] = [
    ("sin", jsmath::sin),
    ("cos", jsmath::cos),
    ("tan", jsmath::tan),
    ("asin", jsmath::asin),
    ("acos", jsmath::acos),
    ("atan", jsmath::atan),
    ("log", jsmath::log),
];

const BINARY: [(&str, BinaryFn); 3] = [
    ("atan2", jsmath::atan2),
    ("pow", jsmath::pow),
    ("hypot", jsmath::hypot),
];

/// The platform's functions, for the comparison with `--std`.
const STD_UNARY: [(&str, UnaryFn); 7] = [
    ("sin", f64::sin),
    ("cos", f64::cos),
    ("tan", f64::tan),
    ("asin", f64::asin),
    ("acos", f64::acos),
    ("atan", f64::atan),
    ("log", f64::ln),
];

const STD_BINARY: [(&str, BinaryFn); 3] = [
    ("atan2", f64::atan2),
    ("pow", f64::powf),
    ("hypot", f64::hypot),
];

fn canonical(value: f64) -> u64 {
    if value.is_nan() {
        f64::NAN.to_bits()
    } else {
        value.to_bits()
    }
}

fn read_words(path: &Path) -> Vec<u64> {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|chunk| u64::from_le_bytes(*chunk))
        .collect()
}

fn write_results(path: &Path, results: impl Iterator<Item = u64>) {
    let bytes: Vec<u8> = results.flat_map(u64::to_le_bytes).collect();
    fs::write(path, bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
}

fn evaluate_unary(dir: &Path, name: &str, function: UnaryFn, extension: &str) {
    let inputs = read_words(&dir.join(format!("{name}.in")));
    let results = inputs
        .into_iter()
        .map(|bits| canonical(function(f64::from_bits(bits))));
    write_results(&dir.join(format!("{name}.{extension}")), results);
}

fn evaluate_binary(dir: &Path, name: &str, function: BinaryFn, extension: &str) {
    let inputs = read_words(&dir.join(format!("{name}.in")));
    let results = inputs
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| canonical(function(f64::from_bits(pair[0]), f64::from_bits(pair[1]))));
    write_results(&dir.join(format!("{name}.{extension}")), results);
}

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    let dir = PathBuf::from(arguments.get(1).expect("usage: evaluate <dir> [--std]"));
    let extension = if arguments.iter().any(|arg| arg == "--std") {
        "std"
    } else {
        "rs"
    };
    let (unary, binary) = if extension == "std" {
        (&STD_UNARY[..], &STD_BINARY[..])
    } else {
        (&UNARY[..], &BINARY[..])
    };
    for (name, function) in unary {
        evaluate_unary(&dir, name, *function, extension);
    }
    for (name, function) in binary {
        evaluate_binary(&dir, name, *function, extension);
    }
}
