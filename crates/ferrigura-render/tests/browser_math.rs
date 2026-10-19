//! The renderer takes JavaScript's `Math` functions from `ferrigura_jsmath`, which gives Chromium's
//! results on every host, and never from the standard library: the `f64` methods call the platform's
//! libm, whose last bits differ between hosts, and the optimiser may fold or inline them into other
//! values again. A source file that calls one fails the test.

use std::fs;
use std::path::{Path, PathBuf};

/// The `f64` and `f32` methods whose results the platform decides. `sqrt`, `mul_add`, `abs`, `floor`
/// and the like are exact and stay. `powi` is here because the standard library leaves its precision
/// open; `ferrigura_jsmath::pow(x, 2.0)` is V8's `x * x`.
const PLATFORM_FUNCTIONS: [&str; 26] = [
    "sin", "cos", "tan", "sin_cos", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh",
    "asinh", "acosh", "atanh", "exp", "exp2", "exp_m1", "ln", "ln_1p", "log", "log2", "log10",
    "powf", "powi", "hypot", "cbrt",
];

#[test]
fn no_source_calls_the_platform_math_functions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["src", "tests", "examples", "benches"] {
        collect(&root.join(dir), &mut files);
    }
    assert!(files.len() > 100, "the sources were not found");
    for file in files {
        let name = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&file).unwrap_or_default();
        for (index, line) in source.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if let Some(function) = platform_call(line) {
                panic!(
                    "{name}:{} calls the standard library's `{function}`; take it from \
                     `ferrigura_jsmath`",
                    index + 1
                );
            }
        }
    }
}

#[test]
fn the_scan_finds_method_calls_and_paths() {
    let method = format!("let y = (x + 1.0).{}() * 2.0;", "sin");
    let path = format!("values.map(f64::{});", "ln");
    let pair = format!("let (s, c) = angle.{}();", "sin_cos");
    assert_eq!(platform_call(&method), Some("sin"));
    assert_eq!(platform_call(&path), Some("ln"));
    assert_eq!(platform_call(&pair), Some("sin_cos"));
    assert_eq!(platform_call("let y = sin(x + 1.0) * hypot(a, b);"), None);
    assert_eq!(
        platform_call("let y = ferrigura_jsmath::atan2(dy, dx);"),
        None
    );
    assert_eq!(platform_call("let r = (a * a + b * b).sqrt();"), None);
    // A reference's `Math.hypot(...)`, quoted in a string or a trailing comment, is not a call.
    let quoted = format!("let text = \"Math.{}(dx, dy)\";", "hypot");
    assert_eq!(platform_call(&quoted), None);
}

/// The first platform function that a line calls as a method (`x.sin()`) or names by its path
/// (`f64::sin`), if any.
fn platform_call(line: &str) -> Option<&'static str> {
    PLATFORM_FUNCTIONS.into_iter().find(|function| {
        let method = format!(".{function}(");
        let called = line
            .match_indices(&method)
            .any(|(at, _)| !line[..at].ends_with("Math"));
        let named = ["f64::", "f32::"].iter().any(|ty| {
            let path = format!("{ty}{function}");
            line.match_indices(&path).any(|(at, _)| {
                line[at + path.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'))
            })
        });
        called || named
    })
}

/// The `.rs` files below a directory.
fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}
