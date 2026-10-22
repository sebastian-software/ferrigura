//! ferrigura's own output, pinned (`docs-port/ORACLE-PLAN.md`, step 4). Every `.mmd` of the corpus is
//! rendered under three configurations and four colour schemes, and one hash over its twelve SVGs (or
//! errors) is compared with `snapshots/svg.tsv`. Any change of the output fails the test, the ones the
//! geometry check cannot see included (markup, styles, embedded fonts).
//!
//! An intended change rewrites the manifest in the same commit:
//!
//! ```text
//! UPDATE_SNAPSHOTS=1 cargo test --release --test snapshots
//! ```
//!
//! and `tools/snapshots/diff.sh` shows which renders of the changed files moved. When the test fails,
//! the new renders of the changed files are in `target/snapshots/head/`.
//!
//! The output depends on the machine: `ferrigura_jsmath` rounds as V8 does there, with fused
//! multiply-adds on aarch64 and without them on x86-64. The manifest holds the aarch64 renders, so
//! the test runs on aarch64 (CI: macOS) and is ignored elsewhere. It takes about a minute in a
//! release build and several in a debug build, so it runs in release builds only.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use ferrigura::{ColorScheme, RenderOptions, Renderer};
use ferrigura_model::value::Value;

/// The clock of the gantt dates, fixed as in the corpus comparisons.
const NOW: i64 = 1_791_000_000_000;

/// The folders of the corpus, from the repository root.
const CORPUS: [&str; 3] = [
    "e2e/diagrams",
    "e2e/platform/dev-diagrams",
    "tools/port/geometry/probes",
];

/// The manifest, from the repository root: one line per corpus file, its path and its hash.
const MANIFEST: &str = "crates/ferrigura/tests/snapshots/svg.tsv";

/// The colour schemes: none (the configured theme's colours), light, dark and both.
const SCHEMES: [(&str, Option<ColorScheme>); 4] = [
    ("none", None),
    ("light", Some(ColorScheme::Light)),
    ("dark", Some(ColorScheme::Dark)),
    ("auto", Some(ColorScheme::Auto)),
];

/// The configurations: none, every theme variable overridden with values of each kind, and the dark
/// theme.
fn configurations(root: &Path) -> Vec<(&'static str, Value)> {
    let overrides =
        std::fs::read_to_string(root.join("crates/ferrigura/tests/snapshots/override.json"))
            .expect("the override configuration");
    let overrides: serde_json::Value =
        serde_json::from_str(&overrides).expect("the override configuration is JSON");
    vec![
        ("none", Value::empty_object()),
        ("override", Value::from(overrides)),
        (
            "dark",
            Value::from(serde_json::json!({ "theme": "redux-dark-color" })),
        ),
    ]
}

/// Appends every `.mmd` file below `folder` to `files`.
fn collect(folder: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(folder)
        .unwrap_or_else(|error| panic!("{} cannot be read: {error}", folder.display()));
    for entry in entries {
        let path = entry.expect("a folder entry").path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "mmd") {
            files.push(path);
        }
    }
}

/// FNV-1a over 64 bits: stable across platforms and Rust versions, and enough to see a change.
fn fnv1a(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The twelve renders of one text, labelled `<configuration>-<scheme>`: the SVG, or the error.
fn renders(text: &str, renderers: &[(&'static str, Renderer)]) -> Vec<(String, String)> {
    let mut out = Vec::with_capacity(renderers.len() * SCHEMES.len());
    for (config, renderer) in renderers {
        for (scheme, color_scheme) in SCHEMES {
            let options = RenderOptions {
                now: Some(NOW),
                color_scheme,
                ..RenderOptions::default()
            };
            let result = match renderer.render_with(text, &options) {
                Ok(svg) => svg,
                Err(error) => format!("error: {error}"),
            };
            out.push((format!("{config}-{scheme}"), result));
        }
    }
    out
}

/// The hash of a file's renders, over each label and result.
fn hash(renders: &[(String, String)]) -> String {
    let hash = renders
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, (label, result)| {
            let hash = fnv1a(hash, label.as_bytes());
            let hash = fnv1a(hash, &[0]);
            let hash = fnv1a(hash, result.as_bytes());
            fnv1a(hash, &[0])
        });
    format!("{hash:016x}")
}

#[test]
#[cfg_attr(
    any(debug_assertions, not(target_arch = "aarch64")),
    ignore = "aarch64 release builds only: cargo test --release --test snapshots"
)]
fn the_output_of_the_corpus_is_unchanged() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for folder in CORPUS {
        collect(&root.join(folder), &mut files);
    }
    files.sort();
    let renderers: Vec<(&'static str, Renderer)> = configurations(root)
        .into_iter()
        .map(|(name, config)| (name, Renderer::new(&config)))
        .collect();

    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(files.len().max(1));
    let next = AtomicUsize::new(0);
    let hashes: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = files.get(index) else {
                        break;
                    };
                    let text = std::fs::read_to_string(path).unwrap_or_else(|error| {
                        panic!("{} cannot be read: {error}", path.display())
                    });
                    let name = path
                        .strip_prefix(root)
                        .expect("a corpus file in the repository")
                        .to_string_lossy()
                        .replace('\\', "/");
                    let hash = hash(&renders(&text, &renderers));
                    hashes.lock().expect("hashes").push((name, hash));
                }
            });
        }
    });
    let mut hashes = hashes.into_inner().expect("hashes");
    hashes.sort();
    let mut manifest = String::new();
    for (name, hash) in &hashes {
        writeln!(manifest, "{name}\t{hash}").expect("a line");
    }

    let path = root.join(MANIFEST);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, &manifest).expect("the manifest is written");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    if expected == manifest {
        return;
    }

    let old: std::collections::BTreeMap<&str, &str> = expected
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    let new: std::collections::BTreeMap<&str, &str> = manifest
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    let mut changed = Vec::new();
    for (name, hash) in &new {
        match old.get(name) {
            Some(previous) if previous == hash => {}
            Some(_) => changed.push(format!("changed  {name}")),
            None => changed.push(format!("new      {name}")),
        }
    }
    for name in old.keys().filter(|name| !new.contains_key(*name)) {
        changed.push(format!("missing  {name}"));
    }

    // The new renders of the changed files, for review.
    let head = root.join("target/snapshots/head");
    for name in new
        .iter()
        .filter(|(name, hash)| old.get(*name) != Some(hash))
        .map(|(name, _)| *name)
    {
        let text = std::fs::read_to_string(root.join(name)).expect("a corpus file");
        let folder = head.join(name);
        std::fs::create_dir_all(&folder).expect("the review folder");
        for (label, result) in renders(&text, &renderers) {
            std::fs::write(folder.join(format!("{label}.svg")), result).expect("a review render");
        }
    }

    panic!(
        "the output of {} corpus files changed (renders in {}):\n{}\n\nIf the change is intended: \
         UPDATE_SNAPSHOTS=1 cargo test --release --test snapshots, and tools/snapshots/diff.sh \
         shows what moved.",
        changed.len(),
        head.display(),
        changed
            .iter()
            .take(50)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    );
}
