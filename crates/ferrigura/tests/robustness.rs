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

/// Scans SVG number lists, where a sign or a second decimal point can start the next number.
fn contains_non_finite(value: &str) -> bool {
    if value.contains("NaN") || value.contains("Infinity") || value.contains("inf") {
        return true;
    }
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        if matches!(bytes[i], b'+' | b'-') {
            i += 1;
        }
        let mut digits = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            i = start + 1;
            continue;
        }
        if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
            let exponent = i;
            i += 1;
            if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
                i += 1;
            }
            let first_digit = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == first_digit {
                i = exponent;
            }
        }
        if value[start..i].parse::<f64>().is_ok_and(|n| !n.is_finite()) {
            return true;
        }
    }
    false
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
            ) && contains_non_finite(attr.value())
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
    non_finite.sort_unstable();
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
#[ignore = "extended: repeated corpus renders in child processes"]
fn extended_robustness_in_bounded_children() {
    use std::{
        process::Command,
        thread,
        time::{Duration, Instant},
    };
    const RUNS: usize = 4;
    let deadline = Instant::now() + Duration::from_secs(30);
    for run in 0..RUNS {
        let mut child = Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--skip",
                "extended_robustness_in_bounded_children",
                "--test-threads=1",
            ])
            .spawn()
            .expect("start robustness child");
        loop {
            if let Some(status) = child.try_wait().expect("poll robustness child") {
                assert!(status.success(), "robustness child {run} failed: {status}");
                break;
            }
            if Instant::now() >= deadline {
                if let Err(error) = child.kill() {
                    assert!(
                        child
                            .try_wait()
                            .expect("check child after kill race")
                            .is_some(),
                        "failed to stop child: {error}"
                    );
                }
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
        r#"<svg><rect x="inf"/></svg>"#,
        r#"<svg><rect x="-inf"/></svg>"#,
        r#"<svg><path d="M0-1e309"/></svg>"#,
        r#"<svg><path d="M.2.3e309"/></svg>"#,
    ] {
        assert!(std::panic::catch_unwind(|| finite_geometry(svg, "example", "example")).is_err());
    }
    finite_geometry(
        r#"<svg><text>NaN is a label</text></svg>"#,
        "example",
        "example",
    );
}

#[test]
fn small_and_moderate_nesting_stays_isolated() {
    let renderer = Renderer::new(&Value::empty_object());
    let control = renderer.render(CONTROL).unwrap();
    for depth in [2, 8] {
        let mut flow = String::from("flowchart TD\n");
        let mut state = String::from("stateDiagram-v2\n");
        let mut block = String::from("block\n");
        let mut mindmap = String::from("mindmap\nroot(Root)\n");
        let mut tree = String::from("treeView-beta\n");
        for n in 0..depth {
            flow.push_str(&format!("subgraph g{n}\n"));
            state.push_str(&format!("state s{n} {{\n"));
            block.push_str(&format!("block:g{n}\n"));
            mindmap.push_str(&format!("{}n{n}\n", "  ".repeat(n + 1)));
            tree.push_str(&format!("{}n{n}/\n", "  ".repeat(n + 1)));
        }
        flow.push_str("A --> B\n");
        state.push_str("[*] --> Ready\n");
        block.push_str("leaf\n");
        tree.push_str(&format!("{}leaf\n", "  ".repeat(depth + 1)));
        for _ in 0..depth {
            flow.push_str("end\n");
            state.push_str("}\n");
            block.push_str("end\n");
        }
        for (kind, source) in [
            ("flowchart", flow),
            ("state", state),
            ("block", block),
            ("mindmap", mindmap),
            ("treeView", tree),
        ] {
            let baseline = renderer
                .render(&source)
                .unwrap_or_else(|e| panic!("{kind} depth {depth}: {e}"));
            finite_geometry(&baseline, &source, &format!("{kind} depth {depth}"));
            let truncated = source
                .trim_end_matches('\n')
                .rsplit_once('\n')
                .map_or("", |(prefix, _)| prefix);
            if let Ok(svg) = renderer.render(truncated) {
                finite_geometry(&svg, truncated, &format!("{kind} depth {depth} truncated"));
            }
            assert_eq!(
                renderer.render(&source).unwrap(),
                baseline,
                "{kind} depth {depth}"
            );
            assert_eq!(
                renderer.render(CONTROL).unwrap(),
                control,
                "{kind} depth {depth}"
            );
        }
    }
}

#[test]
fn yaml_alias_bombs_and_cyclic_anchors_fail_cleanly() {
    let renderer = Renderer::new(&Value::empty_object());
    let control = renderer.render(CONTROL).expect("control renders");

    // 15 levels (a0..a14) expand to ~131k nodes: exceeds the 50,000-node expansion cap while
    // staying safe to materialize if the guard ever regresses.
    let mut billion_laughs = String::from("---\na0: &a0 [1, 1]\n");
    for i in 1..15 {
        billion_laughs.push_str(&format!("a{i}: &a{i} [*a{}, *a{}]\n", i - 1, i - 1));
    }
    billion_laughs.push_str("---\nflowchart LR\nA --> B\n");

    let payload = "x".repeat(40_000);
    let string_bomb = format!(
        "---\ns: &s \"{payload}\"\na: &a [*s, *s, *s, *s]\nb: &b [*a, *a, *a, *a]\nc: [*b, *b, *b, *b]\n---\nflowchart LR\nA --> B\n"
    );

    let mut deep_chain = String::from("---\na0: &a0 [1]\n");
    for i in 1..=250 {
        deep_chain.push_str(&format!("a{i}: &a{i} [*a{}]\n", i - 1));
    }
    deep_chain.push_str("---\nflowchart LR\nA --> B\n");

    let cyclic_seq = format!("---\na: &x [*x, \"{payload}\"]\n---\nflowchart LR\nA --> B\n");
    let cyclic_map =
        format!("---\na: &x {{self: *x, data: \"{payload}\"}}\n---\nflowchart LR\nA --> B\n");

    let mut shape_bomb = String::from("flowchart LR\nA@{ a0: &a0 [1, 1]");
    for i in 1..15 {
        shape_bomb.push_str(&format!(", a{i}: &a{i} [*a{}, *a{}]", i - 1, i - 1));
    }
    shape_bomb.push_str(" }\n");

    for (name, input) in [
        ("billion laughs front matter", billion_laughs.as_str()),
        ("string amplification front matter", string_bomb.as_str()),
        ("deep alias chain front matter", deep_chain.as_str()),
        ("cyclic sequence front matter", cyclic_seq.as_str()),
        ("cyclic mapping front matter", cyclic_map.as_str()),
        ("billion laughs shape data", shape_bomb.as_str()),
    ] {
        let error = renderer
            .render(input)
            .expect_err(&format!("{name} must be rejected"));
        let message = error.to_string();
        assert!(
            !message.contains("maxTextSize"),
            "{name} should fail in YAML expansion, not input size limit: {message}"
        );
        assert_eq!(renderer.render(CONTROL).unwrap(), control, "after {name}");
    }
}
