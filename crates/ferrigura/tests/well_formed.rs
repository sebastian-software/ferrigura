//! Every SVG is well-formed XML: each `.mmd` under `e2e/diagrams` and `tools/port/geometry/probes`
//! that ferrigura renders is read with a strict XML parser, which rejects what the browser's HTML
//! parser lets through (an unbound `xlink` prefix, a quote that ends an attribute value early, an
//! entity XML does not define).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use ferrigura::{RenderOptions, render_with};
use ferrigura_model::value::Value;

/// The clock of the gantt dates, as the corpus comparison fixes it.
const NOW: i64 = 1_760_000_000_000;

/// The folders of the corpus, from the repository root.
const CORPUS: [&str; 2] = ["e2e/diagrams", "tools/port/geometry/probes"];

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

/// Links that no corpus input draws: a sequence actor's menu links and icon, and a gantt task's link.
const LINKS: [&str; 3] = [
    "sequenceDiagram\nparticipant A\nlinks A: {\"Dashboard\": \"https://a.example/?x=1&y=2\", \"Wiki\": \"https://b.example\"}\nA->>A: hi\n",
    "sequenceDiagram\nparticipant A\nproperties A: {\"icon\": \"https://example.com/a.png\"}\nA->>A: hi\n",
    "gantt\ndateFormat YYYY-MM-DD\nsection S\nTask one :t1, 2024-01-01, 3d\nclick t1 href \"https://example.com/?a=1&b=2\"\n",
];

/// Entity codes typed into a text: a name HTML knows and one it does not, a control, a C1 code and
/// a character.
const TYPED: &str = "a #nbsp; b #foo; c #1; d #128; e #9829;";

/// Diagrams whose renderers write the typed entities of their texts themselves.
fn typed_entities() -> Vec<String> {
    vec![
        format!("sequenceDiagram\ntitle {TYPED}\nA->>B: {TYPED}\nNote over A: {TYPED}\n"),
        format!("timeline\ntitle {TYPED}\n2020 : {TYPED}\n"),
        format!("quadrantChart\ntitle {TYPED}\nx-axis {TYPED}\nPoint {TYPED}: [0.3, 0.6]\n"),
        format!("xychart\ntitle \"{TYPED}\"\nx-axis [\"{TYPED}\", \"c\"]\nbar [1, 2]\n"),
        format!("C4Context\ntitle {TYPED}\nPerson(a, \"{TYPED}\", \"{TYPED}\")\n"),
        format!(
            "gantt\ndateFormat YYYY-MM-DD\ntitle {TYPED}\nsection s\ntask :t1, 2024-01-01, 1d\n"
        ),
        format!("journey\ntitle {TYPED}\nsection {TYPED}\n {TYPED}: 5: Me\n"),
        format!("packet\n0-10: \"{TYPED}\"\n"),
        format!("treemap-beta\n\"{TYPED}\"\n    \"{TYPED}\": 10\n"),
        format!("flowchart LR\nA[{TYPED}] --> B\n"),
    ]
}

/// Renders a text that ferrigura accepts and parses the SVG with the strict parser.
fn well_formed(text: &str) -> String {
    let options = RenderOptions {
        now: Some(NOW),
        ..RenderOptions::default()
    };
    let svg = render_with(text, &Value::empty_object(), &options).expect("the diagram renders");
    if let Err(error) = roxmltree::Document::parse(&svg) {
        panic!("not well-formed XML ({error}) for {text}");
    }
    svg
}

#[test]
fn links_outside_the_corpus_declare_the_xlink_prefix() {
    for text in LINKS {
        assert!(
            well_formed(text).contains("xlink:href="),
            "a link is drawn for {text}"
        );
    }
}

#[test]
fn typed_entities_are_written_as_xml_reads_them() {
    for text in typed_entities() {
        well_formed(&text);
    }
}

/// What one input gave: not rendered, or rendered with the parser's error if there is one.
enum Outcome {
    Refused,
    Rendered(Option<String>),
}

#[test]
fn every_rendered_svg_is_well_formed_xml() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for folder in CORPUS {
        collect(&root.join(folder), &mut files);
    }
    files.sort();

    let options = RenderOptions {
        now: Some(NOW),
        ..RenderOptions::default()
    };
    let config = Value::empty_object();
    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(files.len().max(1));
    let next = AtomicUsize::new(0);
    let outcomes: Mutex<Vec<(usize, Outcome)>> = Mutex::new(Vec::new());
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
                    let outcome = match render_with(&text, &config, &options) {
                        Err(_) => Outcome::Refused,
                        Ok(svg) => Outcome::Rendered(
                            roxmltree::Document::parse(&svg)
                                .err()
                                .map(|error| error.to_string()),
                        ),
                    };
                    outcomes.lock().expect("outcomes").push((index, outcome));
                }
            });
        }
    });
    let mut outcomes = outcomes.into_inner().expect("outcomes");
    outcomes.sort_by_key(|(index, _)| *index);

    let mut rendered = 0;
    let mut failures = Vec::new();
    for (index, outcome) in &outcomes {
        if let Outcome::Rendered(error) = outcome {
            rendered += 1;
            if let Some(error) = error {
                let name = files[*index].strip_prefix(root).unwrap_or(&files[*index]);
                failures.push(format!("{}: {error}", name.display()));
            }
        }
    }
    println!("{rendered} of {} inputs rendered", files.len());
    // The corpus had 2459 inputs, of which ferrigura rendered 2390, when this test was written: a
    // walk that finds far fewer has lost a folder.
    assert!(rendered > 2000, "only {rendered} inputs rendered");
    assert!(
        failures.is_empty(),
        "{} of {rendered} SVGs are not well-formed XML:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
