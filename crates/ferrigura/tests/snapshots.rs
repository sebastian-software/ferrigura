//! ferrigura's own output, pinned (`docs/decisions/0015-differential-testing.md`). Every `.mmd` of
//! the corpus is rendered under three configurations and four color schemes, and one hash over its
//! twelve SVGs (or errors) is compared with `snapshots/svg.tsv`. Each line has the file and two
//! hashes: the twelve renders of the default rendering
//! (`docs/decisions/0023-the-default-rendering.md`), then the twelve of `compat`, Mermaid's
//! geometry (`docs/decisions/0019-mermaid-geometry-with-compat.md`). Any change of the output fails
//! the test, the ones the geometry check cannot see included (markup, styles, embedded fonts).
//!
//! An intended change rewrites the manifest in the same commit:
//!
//! ```text
//! UPDATE_SNAPSHOTS=1 cargo test --release --test snapshots
//! ```
//!
//! and `tools/snapshots/diff.sh` shows which renders of the changed files moved. When the test
//! fails, the new renders of the changed files are in `target/snapshots/head/` (the `compat`
//! renders with a `compat-` prefix).
//!
//! The output depends on the machine: `ferrigura_jsmath` rounds as V8 does there, with fused
//! multiply-adds on aarch64 and without them on x86-64. The manifest holds the aarch64 renders, so
//! the test runs on aarch64 (CI: a Linux arm64 runner) and is ignored elsewhere. Someone on another
//! architecture takes the new manifest from the log or the artifact of the failed CI job, which
//! regenerates it there. It takes about a minute in a release build and several in a debug build, so
//! it runs in release builds only.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use ferrigura::{Animation, AnimationOrder, ColorScheme, RenderOptions, Renderer};
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

#[test]
#[cfg_attr(
    any(debug_assertions, not(target_arch = "aarch64")),
    ignore = "aarch64 release builds only: cargo test --release --test snapshots"
)]
fn the_draw_animation_is_pinned() {
    let renderer = Renderer::new(&Value::empty_object());
    let cases = [
        (
            "branch",
            "flowchart LR\n A[Start] --> B{Review}\n B -->|yes| C[Done]\n B -->|no| D[Retry]\n",
        ),
        ("cycle", "flowchart TD\n A --> B\n B --> A\n B --> C\n"),
        (
            "styles",
            "flowchart LR\n subgraph Work\n A edge@-->|go| B\n B -.-> A\n end\n B <--> C\n linkStyle 0 stroke:#f00,stroke-width:4px\n",
        ),
        (
            "gantt",
            "gantt\n dateFormat YYYY-MM-DD\n todayMarker off\n section Build\n Design :done, a, 2026-01-01, 3d\n Implement :active, b, after a, 5d\n Launch :milestone, after b, 0d\n",
        ),
        (
            "mindmap",
            "---\nconfig:\n  layout: tidy-tree\n---\nmindmap\n root((Release))\n  Research\n   Interviews\n  Build\n   Test\n  Ship\n",
        ),
        (
            "pie",
            "pie showData\n title Work\n \"Layout\" : 45\n \"Text\" : 30\n \"SVG\" : 25\n",
        ),
        (
            "donut",
            "---\nconfig:\n  pie:\n    donutHole: 0.55\n---\npie\n \"Compile\" : 60\n \"Link\" : 25\n \"Package\" : 15\n",
        ),
        (
            "xychart",
            "xychart\n x-axis [Mon, Tue, Wed]\n y-axis 0 --> 100\n bar \"Builds\" [32, 54, 92]\n line \"Trend\" [28, 42, 88]\n",
        ),
        (
            "xy-horizontal",
            "xychart horizontal\n x-axis [Build, Test, Ship]\n y-axis 0 --> 50\n bar [40, 26, 18]\n",
        ),
    ];
    let mut actual = String::new();
    for (name, text) in cases {
        // One hash for the default rendering, one for `compat`.
        let mut hashes = Vec::new();
        for compat in [false, true] {
            let mut outputs = Vec::new();
            for (label, order) in [
                ("source", AnimationOrder::Source),
                ("flow", AnimationOrder::Flow),
            ] {
                for speed in [1.0, 2.0] {
                    let mut animation = Animation::new(speed).unwrap();
                    animation.order = order;
                    let svg = renderer
                        .render_with(
                            text,
                            &RenderOptions {
                                animation: Some(animation),
                                now: Some(NOW),
                                compat,
                                ..RenderOptions::default()
                            },
                        )
                        .unwrap();
                    outputs.push((format!("{label}-{speed}"), svg));
                }
            }
            hashes.push(hash(&outputs));
        }
        writeln!(actual, "{name}\t{}", hashes.join("\t")).unwrap();
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/ferrigura/tests/snapshots/animation.tsv");
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, &actual).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        actual,
        "the optional draw animation changed"
    );
}

/// The color schemes: none (the configured theme's colors), light, dark and both.
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

/// The rendering of a hash: the default, or `compat`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Default,
    Compat,
}

/// The twelve renders of one text in one mode, labeled `<configuration>-<scheme>`: the SVG, or the
/// error.
fn renders(
    text: &str,
    renderers: &[(&'static str, Renderer, String)],
    mode: Mode,
) -> Vec<(String, String)> {
    let mut out = Vec::with_capacity(renderers.len() * SCHEMES.len());
    for (config, renderer, original_config) in renderers {
        // Keep the baseline IDs derived from the original appearance configuration. The extra
        // text allowance below is a test harness setting, not an SVG appearance change.
        let hash = fnv1a(
            fnv1a(fnv1a(0xcbf2_9ce4_8422_2325, text.as_bytes()), &[0xff]),
            original_config.as_bytes(),
        );
        let id = format!("ferrigura-{hash:016x}");
        for (scheme, color_scheme) in SCHEMES {
            let options = RenderOptions {
                now: Some(NOW),
                id: Some(id.clone()),
                color_scheme,
                compat: mode == Mode::Compat,
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
    // These checked-in corpus files are trusted and include large performance fixtures. A finite
    // allowance keeps their SVG baselines covered without changing the public default of 50,000.
    let renderers: Vec<(&'static str, Renderer, String)> = configurations(root)
        .into_iter()
        .map(|(name, mut config)| {
            let original = serde_json::to_string(&config).expect("configuration serializes");
            if let Value::Object(settings) = &mut config {
                settings.insert("maxTextSize".to_owned(), Value::Number(300_000.0));
            }
            (name, Renderer::new(&config), original)
        })
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
                    // The second column pins `compat` separately, so updating the default cannot
                    // silently bless a change in Mermaid's geometry.
                    let hash = format!(
                        "{}\t{}",
                        hash(&renders(&text, &renderers, Mode::Default)),
                        hash(&renders(&text, &renderers, Mode::Compat))
                    );
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
        for (label, result) in renders(&text, &renderers, Mode::Default) {
            std::fs::write(folder.join(format!("{label}.svg")), result).expect("a review render");
        }
        for (label, result) in renders(&text, &renderers, Mode::Compat) {
            std::fs::write(folder.join(format!("compat-{label}.svg")), result)
                .expect("a review render");
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
