//! Small, deterministic malformed-input regressions at the public rendering boundary.

use ferrigura::Renderer;
use ferrigura_model::value::Value;

const CONTROL: &str = "pie\n\"Cats\": 2\n\"Dogs\": 3\n";

#[test]
fn truncated_and_unicode_inputs_leave_the_renderer_usable() {
    let renderer = Renderer::new(&Value::empty_object());
    let expected = renderer.render(CONTROL).expect("control renders");
    let seeds = [
        "flowchart LR\nA[Grüße 🦀] --> B{Done}\n",
        "sequenceDiagram\nAlice->>Bob: こんにちは 🦀\n",
        "pie title Café\n\"猫\": 2\n\"犬\": 3\n",
        "stateDiagram-v2\n[*] --> Ready\nReady --> [*]\n",
        "classDiagram\nclass Animal\nAnimal : +String name\n",
    ];
    let invalid = [
        "---\nconfig: [unterminated\n---\nflowchart LR\nA-->B",
        "---\nconfig:\n  flowchart:\n    padding: [wrong, type]\n---\nflowchart LR\nA-->B",
        "flowchart LR\nsubgraph outer\nsubgraph inner\nA -->",
        "sequenceDiagram\nAlice->>Bob: <&\u{0}\n",
    ];

    for seed in seeds {
        let baseline = renderer.render(seed).expect("seed renders");
        // Slice only at character boundaries so multibyte text reaches the public API intact.
        for (end, _) in seed.char_indices().step_by(7).chain([(seed.len(), '\0')]) {
            let text = &seed[..end];
            if let Ok(svg) = renderer.render(text) {
                finite_geometry(&svg, text, &format!("{text:?}"));
            }
            assert_eq!(renderer.render(seed).unwrap(), baseline, "after {text:?}");
            assert_eq!(
                renderer.render(CONTROL).unwrap(),
                expected,
                "after {text:?}"
            );
        }
    }
    for text in invalid {
        if let Ok(svg) = renderer.render(text) {
            finite_geometry(&svg, text, &format!("{text:?}"));
        }
        assert_eq!(
            renderer.render(CONTROL).unwrap(),
            expected,
            "after {text:?}"
        );
    }
}

/// Checks numeric geometry attributes, including path and transform lists, for non-finite values.
fn finite_geometry(svg: &str, source: &str, context: &str) {
    let doc = roxmltree::Document::parse(svg).unwrap_or_else(|e| panic!("{context}: {e}"));
    let mut non_finite = Vec::new();
    for node in doc.descendants().filter(roxmltree::Node::is_element) {
        for attr in node.attributes() {
            if matches!(
                attr.name(),
                "x" | "y"
                    | "x1"
                    | "y1"
                    | "x2"
                    | "y2"
                    | "cx"
                    | "cy"
                    | "r"
                    | "rx"
                    | "ry"
                    | "width"
                    | "height"
                    | "viewBox"
                    | "d"
                    | "points"
                    | "transform"
                    | "stroke-width"
            ) && (attr.value().contains("NaN")
                || attr.value().contains("Infinity")
                || attr
                    .value()
                    .split(|c: char| {
                        !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
                    })
                    .filter_map(|token| token.parse::<f64>().ok())
                    .any(|value| !value.is_finite()))
            {
                non_finite.push((node.tag_name().name(), attr.name(), attr.value()));
            }
        }
    }
    // The pinned reference also renders this title-only Gantt with these two NaN endpoints.
    // Keep the exception tied to the complete input and the exact element and attributes.
    let expected: &[(&str, &str, &str)] = if source == "gantt\n  title Te" {
        &[("line", "x1", "NaN"), ("line", "x2", "NaN")]
    } else {
        &[]
    };
    assert_eq!(non_finite, expected, "{context}: non-finite geometry");
}

/// A compact corpus fixture per registered kind. Paths are explicit so a new fixture cannot change
/// which cases CI runs; the registry assertion below requires a seed when a plugin is added.
const ALL_KIND_FIXTURES: &[&str] = &[
    "tools/port/geometry/probes/kanban/p01-items-under-sections.mmd",
    "tools/port/geometry/probes/packet/p20-multiline-label.mmd",
    "tools/port/geometry/probes/mindmap/p09-wide-plain.mmd",
    "tools/port/geometry/probes/treemap/p-single-leaf.mmd",
    "tools/port/geometry/probes/eventmodeling/p16-quoted-data-single.mmd",
    "tools/port/geometry/probes/ishikawa/p11-short-head-and-one-leaf.mmd",
    "tools/port/geometry/probes/railroad/c18-peg.mmd",
    "tools/port/geometry/probes/sankey/p01-plain-two-nodes.mmd",
    "tools/port/geometry/probes/treeView/p02-empty-and-slash-names.mmd",
    "tools/port/geometry/probes/radar/p14-one-axis.mmd",
    "tools/port/geometry/probes/venn/p05-disjoint-labelled-union.mmd",
    "tools/port/geometry/probes/cynefin/p08-confusion-four-items.mmd",
    "tools/port/geometry/probes/wardley/p04-stage-names-and-boundaries.mmd",
    "tools/port/geometry/probes/agentflow/p11-edge-kinds.mmd",
    "e2e/diagrams/block/nightly-check/block-types-3-stadium.mmd",
    "e2e/diagrams/flowchart/regressions/routing-review-cycle-bt.mmd",
    "tools/port/geometry/probes/swimlanes/p01-direction-br.mmd",
    "e2e/diagrams/info/pr-check/should-handle-an-info-definition.mmd",
    "e2e/diagrams/usecase/nightly-check/17-direction-bottom-up.mmd",
    "e2e/diagrams/er-diagram/visual-check/er-subgraphs-default.mmd",
    "e2e/diagrams/xychart/nightly-check/should-render-the-simplest-possible-xy-chart.mmd",
    "e2e/diagrams/git/nightly-check/15-should-render-a-simple-gitgraph-with-commit-on-main-branch-vertical-branch.mmd",
    "tools/port/geometry/probes/quadrantChart/p14-empty-quadrant-texts-with-points.mmd",
    "tools/port/geometry/probes/timeline/p-named-entities.mmd",
    "e2e/diagrams/class-diagram/nightly-check/handdrawn/hd-should-render-a-simple-class-diagram-with-the-handdrawn-look.mmd",
    "e2e/diagrams/requirement/nightly-check/a-requirement-diagram-with-empty-information-default.mmd",
    "tools/port/geometry/probes/architecture/p08-seed-override.mmd",
    "e2e/diagrams/state-diagram-v2/nightly-check/v2-should-render-a-compact-self-loop-edge.mmd",
    "e2e/diagrams/journey/pr-check/simple-test.mmd",
    "e2e/diagrams/c4/nightly-check/relationships/bidirectional-relationship.mmd",
    "e2e/diagrams/gantt/nightly-check/should-render-only-the-day-when-using-dateformat-d.mmd",
    "e2e/diagrams/sequence/nightly-check/should-render-stick-half-arrow-top.mmd",
    "e2e/diagrams/pie/pr-check/should-render-a-pie-diagram-with-showdata.mmd",
];

#[test]
fn every_registered_kind_handles_bounded_truncations() {
    use ferrigura::{RenderOptions, diagrams};
    use std::{collections::HashSet, path::Path};
    let renderer = Renderer::new(&Value::empty_object());
    let options = RenderOptions {
        now: Some(1_760_000_000_000),
        ..RenderOptions::default()
    };
    let control = renderer
        .render_with(CONTROL, &options)
        .expect("control renders");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registered: HashSet<_> = diagrams::all().iter().map(diagrams::Plugin::kind).collect();
    let mut exercised = HashSet::new();
    for path in ALL_KIND_FIXTURES {
        let original = std::fs::read_to_string(root.join(path)).expect("fixture exists");
        // Strip fixture front matter so truncations exercise the diagram body. The first test
        // separately covers malformed front matter and configuration.
        let source = original
            .strip_prefix("---\n")
            .and_then(|s| s.split_once("\n---\n").map(|(_, body)| body))
            .unwrap_or(&original);
        assert!(source.len() <= 1200, "{path}: seed too large");
        let kind = ferrigura_syntax::detect(source).unwrap_or_else(|| panic!("{path}: no kind"));
        assert!(exercised.insert(kind), "{path}: duplicate {kind:?}");
        let baseline = renderer
            .render_with(source, &options)
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        finite_geometry(&baseline, source, path);
        let offsets: Vec<usize> = source.char_indices().map(|(i, _)| i).collect();
        for index in [
            offsets.len() / 3,
            offsets.len() / 2,
            offsets.len().saturating_sub(1),
        ] {
            let text = &source[..offsets[index]];
            if let Ok(svg) = renderer.render_with(text, &options) {
                finite_geometry(&svg, text, &format!("{path} at {}", offsets[index]));
            }
            assert_eq!(
                renderer.render_with(source, &options).unwrap(),
                baseline,
                "{path} at {}",
                offsets[index]
            );
            assert_eq!(
                renderer.render_with(CONTROL, &options).unwrap(),
                control,
                "{path} at {}",
                offsets[index]
            );
        }
    }
    assert_eq!(exercised, registered, "seed each registered diagram kind");
}

/// Opt in to repeating the fixed corpus cases in isolated processes. A timeout kills and reaps the
/// child; the synchronous renderer itself does not offer cancellation.
#[test]
fn extended_robustness_in_bounded_children() {
    use std::{
        process::Command,
        thread,
        time::{Duration, Instant},
    };
    if std::env::var_os("FERRIGURA_ROBUSTNESS_EXTENDED").is_none() {
        return;
    }
    const RUNS: usize = 4;
    let deadline = Instant::now() + Duration::from_secs(30);
    for run in 0..RUNS {
        let mut child = Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "every_registered_kind_handles_bounded_truncations",
                "--test-threads=1",
            ])
            .env_remove("FERRIGURA_ROBUSTNESS_EXTENDED")
            .spawn()
            .expect("start robustness child");
        loop {
            if let Some(status) = child.try_wait().expect("poll robustness child") {
                assert!(status.success(), "robustness child {run} failed: {status}");
                break;
            }
            if Instant::now() >= deadline {
                child.kill().expect("stop timed-out robustness child");
                child.wait().expect("reap timed-out robustness child");
                panic!("robustness children exceeded 30 seconds at run {run}");
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

#[test]
fn geometry_check_detects_non_finite_numbers_in_numeric_attributes() {
    for svg in [
        r#"<svg><rect x="NaN"/></svg>"#,
        r#"<svg><rect width="1e309"/></svg>"#,
        r#"<svg><path d="M0,NaN"/></svg>"#,
    ] {
        assert!(std::panic::catch_unwind(|| finite_geometry(svg, "example", "example")).is_err());
    }
    finite_geometry(
        r#"<svg><text>NaN is a label</text></svg>"#,
        "example",
        "example",
    );
}
