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
    "tools/port/geometry/probes/mindmap/p09-wide-plain.mmd",
    "e2e/diagrams/flowchart/regressions/routing-review-cycle-bt.mmd",
    "e2e/diagrams/er-diagram/visual-check/er-subgraphs-default.mmd",
    "e2e/diagrams/xychart/nightly-check/should-render-the-simplest-possible-xy-chart.mmd",
    "tools/port/geometry/probes/quadrantChart/p14-empty-quadrant-texts-with-points.mmd",
    "e2e/diagrams/class-diagram/nightly-check/handdrawn/hd-should-render-a-simple-class-diagram-with-the-handdrawn-look.mmd",
    "e2e/diagrams/state-diagram-v2/nightly-check/v2-should-render-a-compact-self-loop-edge.mmd",
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
        let mut mindmap = String::from("mindmap\nroot(Root)\n");
        for n in 0..depth {
            flow.push_str(&format!("subgraph g{n}\n"));
            state.push_str(&format!("state s{n} {{\n"));
            mindmap.push_str(&format!("{}n{n}\n", "  ".repeat(n + 1)));
        }
        flow.push_str("A --> B\n");
        state.push_str("[*] --> Ready\n");
        for _ in 0..depth {
            flow.push_str("end\n");
            state.push_str("}\n");
        }
        for (kind, source) in [("flowchart", flow), ("state", state), ("mindmap", mindmap)] {
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

    for (name, input, expected_error, max_yaml_line) in [
        (
            "billion laughs front matter",
            billion_laughs.as_str(),
            "YAML alias expansion exceeded size limit",
            16usize,
        ),
        (
            "string amplification front matter",
            string_bomb.as_str(),
            "YAML alias expansion exceeded size limit",
            5usize,
        ),
        (
            "deep alias chain front matter",
            deep_chain.as_str(),
            "YAML alias expansion exceeded nesting limit",
            252usize,
        ),
        (
            "cyclic sequence front matter",
            cyclic_seq.as_str(),
            "a collection that contains itself has no tree",
            2usize,
        ),
        (
            "cyclic mapping front matter",
            cyclic_map.as_str(),
            "a collection that contains itself has no tree",
            2usize,
        ),
        (
            "billion laughs shape data",
            shape_bomb.as_str(),
            "YAML alias expansion exceeded size limit",
            2usize,
        ),
    ] {
        let error = renderer
            .render(input)
            .expect_err(&format!("{name} must be rejected"));
        let message = error.to_string();
        assert!(
            message.contains(expected_error),
            "{name}: expected {expected_error:?}, got {message}"
        );
        let reported_line: usize = message
            .strip_prefix("line ")
            .and_then(|rest| rest.split(',').next())
            .and_then(|digits| digits.parse().ok())
            .unwrap_or_else(|| panic!("{name}: expected leading line number in {message:?}"));
        assert!(
            (2..=max_yaml_line).contains(&reported_line),
            "{name}: expected error line in 2..={max_yaml_line}, got {message}"
        );
        assert_eq!(renderer.render(CONTROL).unwrap(), control, "after {name}");
    }

    // Blank lines after the opening fence and indented fences keep accurate YAML line and column.
    // On line 3 (`  a: &x [*x]`), the cyclic alias `*x` starts at 1-based column 10.
    let indented_with_blank = "  ---\n\n  a: &x [*x]\n  ---\nflowchart LR\nA --> B\n";
    let indented_err = renderer
        .render(indented_with_blank)
        .expect_err("indented cyclic front matter must fail")
        .to_string();
    assert!(
        indented_err.starts_with("line 3, column 10:"),
        "expected line 3, column 10 for indented cyclic front matter, got {indented_err}"
    );

    // A later mapping entry that crosses the byte budget blames that entry's line (line 6), not
    // the leading `title:` line (line 2), across both `Renderer::render` and `ferrigura_syntax::parse`.
    let titled_bomb = format!(
        "---\ntitle: Demo\ns: &s \"{payload}\"\na: &a [*s, *s, *s, *s]\nb: &b [*a, *a, *a, *a]\nc: [*b, *b, *b, *b]\n---\nflowchart LR\nA --> B\n"
    );
    let render_err = renderer
        .render(&titled_bomb)
        .expect_err("titled string bomb must fail in render")
        .to_string();
    assert!(
        render_err.starts_with("line 6,")
            && render_err.contains("YAML alias expansion exceeded size limit"),
        "expected line 6 size limit error from render, got {render_err}"
    );
    let parse_err = ferrigura_syntax::parse(&titled_bomb)
        .expect_err("titled string bomb must fail in syntax::parse")
        .to_string();
    assert!(
        parse_err.starts_with("line 6,")
            && parse_err.contains("YAML alias expansion exceeded size limit"),
        "expected line 6 size limit error from parse, got {parse_err}"
    );

    // Value spans, flow-sequence pair spans, and non-BMP character columns stay exact.
    let multiline_value_cycle = "---\nx: &x\n  self:\n    *x\n---\nflowchart LR\nA --> B\n";
    let value_err = renderer
        .render(multiline_value_cycle)
        .expect_err("multiline mapping value cycle must fail")
        .to_string();
    assert!(
        value_err.starts_with("line 4, column 5:"),
        "expected line 4, column 5 for multiline value cycle, got {value_err}"
    );

    let flow_pair_cycle = "---\nx: &x [key: *x\n]\n---\nflowchart LR\nA --> B\n";
    let flow_err = renderer
        .render(flow_pair_cycle)
        .expect_err("flow pair cycle must fail")
        .to_string();
    assert!(
        flow_err.starts_with("line 2, column 13:"),
        "expected line 2, column 13 for flow pair cycle, got {flow_err}"
    );

    let emoji_cycle = "---\nemoji: &x [\"😀\", *x]\n---\nflowchart LR\nA --> B\n";
    let emoji_err = renderer
        .render(emoji_cycle)
        .expect_err("emoji cyclic front matter must fail")
        .to_string();
    assert!(
        emoji_err.starts_with("line 2, column 17:"),
        "expected line 2, column 17 for emoji cycle, got {emoji_err}"
    );

    let bom_cycle = "---\n\u{FEFF}a: &x [*x]\n---\nflowchart LR\nA --> B\n";
    let bom_err = renderer
        .render(bom_cycle)
        .expect_err("BOM cyclic front matter must fail")
        .to_string();
    assert!(
        bom_err.starts_with("line 2, column 9:"),
        "expected line 2, column 9 for BOM cycle, got {bom_err}"
    );
}

#[test]
fn deeply_nested_html_sanitizer_does_not_overflow() {
    let renderer = Renderer::new(&Value::empty_object());
    let deep_html = "<b>".repeat(16_000);
    let sanitized = ferrigura_syntax::sanitize::sanitize_text(&deep_html);
    assert!(sanitized.starts_with("<b>"));
    assert!(sanitized.ends_with("</b>"));

    // Forbidden-content elements at or beyond the depth cap must keep discarding their subtree,
    // while allowed elements in FORBID_CONTENTS (like <audio>) keep their fallback text.
    for depth in [255, 256, 260] {
        let payload = format!(
            "{}<noscript><p>secret</p></noscript>",
            "<div>".repeat(depth)
        );
        let cleaned = ferrigura_syntax::sanitize::sanitize_text(&payload);
        assert!(
            !cleaned.contains("secret"),
            "noscript content leaked at depth {depth}: {cleaned}"
        );
        let audio = format!("{}<audio>fallback</audio>", "<div>".repeat(depth));
        let cleaned_audio = ferrigura_syntax::sanitize::sanitize_text(&audio);
        assert!(
            cleaned_audio.contains("fallback"),
            "allowed audio fallback lost at depth {depth}: {cleaned_audio}"
        );
    }

    let flow_html = format!("flowchart LR\nA[\"{}\"]\n", "<b>".repeat(4_000));
    let svg = renderer
        .render(&flow_html)
        .expect("deeply nested HTML label renders");
    finite_geometry(&svg, &flow_html, "deep HTML label");
}

#[test]
fn deeply_nested_state_ast_drop_does_not_overflow() {
    use ferrigura_model::state::{StateStmt, Statement};

    // Programmatic deep AST trees drop iteratively without overflowing the stack.
    let mut state_stmt = StateStmt::new("leaf", Some("default"));
    for i in 0..10_000 {
        let mut parent = StateStmt::new(format!("s{i}"), Some("default"));
        parent.doc = Some(vec![Statement::State(state_stmt)]);
        state_stmt = parent;
    }
    drop(state_stmt);
}

#[test]
fn deeply_nested_diagram_parsers_return_clean_errors() {
    let renderer = Renderer::new(&Value::empty_object());
    let depth = 500;
    let mut flowchart = String::from("flowchart TD\n");
    let mut sequence = String::from("sequenceDiagram\n");
    let mut class = String::from("classDiagram\n");
    let mut state = String::from("stateDiagram-v2\n");
    let mut state_described = String::from("stateDiagram-v2\n");
    let mut er = String::from("erDiagram\n");
    for n in 0..depth {
        flowchart.push_str(&format!("subgraph g{n}\n"));
        sequence.push_str(&format!("loop l{n}\n"));
        class.push_str(&format!("namespace n{n} {{\n"));
        state.push_str(&format!("state s{n} {{\n"));
        state_described.push_str(&format!("state \"d{n}\" as s{n} {{\n"));
        er.push_str(&format!("subgraph g{n}\n"));
    }
    flowchart.push_str("A --> B\n");
    sequence.push_str("Alice->>Bob: hi\n");
    class.push_str("class Leaf\n");
    state.push_str("[*] --> Ready\n");
    state_described.push_str("[*] --> Ready\n");
    er.push_str("USER ||--o{ ORDER : places\n");
    for _ in 0..depth {
        flowchart.push_str("end\n");
        sequence.push_str("end\n");
        class.push_str("}\n");
        state.push_str("}\n");
        state_described.push_str("}\n");
        er.push_str("end\n");
    }
    for (name, source) in [
        ("flowchart", flowchart),
        ("sequence", sequence),
        ("class", class),
        ("state", state),
        ("state described", state_described),
        ("er", er),
    ] {
        assert!(
            renderer.render(&source).is_err(),
            "{name} should fail cleanly"
        );
    }
}

#[test]
fn deeply_nested_markdown_does_not_overflow() {
    let renderer = Renderer::new(&Value::empty_object());

    let markdown_cases = [
        format!("flowchart LR\nA[\"`{}hi`\"]\n", "> ".repeat(2_000)),
        format!("flowchart LR\nA[\"`{}hi`\"]\n", "- ".repeat(1_000)),
        format!(
            "flowchart LR\nA[\"`{}x{}`\"]\n",
            "*".repeat(1_000),
            "*".repeat(1_000)
        ),
        format!(
            "flowchart LR\nA[\"`{}x{}`\"]\n",
            "~~".repeat(500),
            "~~".repeat(500)
        ),
        format!(
            "flowchart LR\nA[\"`{}Grüße 🦀 日本語{}`\"]\n",
            "*".repeat(100),
            "*".repeat(100)
        ),
    ];
    for (index, source) in markdown_cases.iter().enumerate() {
        let svg = renderer
            .render(source)
            .unwrap_or_else(|error| panic!("markdown case {index} failed: {error}"));
        finite_geometry(&svg, source, &format!("markdown case {index}"));
    }
}

#[test]
fn deep_container_hierarchies_do_not_overflow_on_small_stack() {
    let handle = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let renderer = Renderer::new(&Value::empty_object());

            // 100 nested composite states pass the parser limit (128) and must not overflow
            // `state::data::Fetcher` or the unified layout hierarchy check.
            let mut state_src = String::from("stateDiagram-v2\n");
            for n in 0..100 {
                state_src.push_str(&format!("state s{n} {{\n"));
            }
            state_src.push_str("[*] --> Ready\n");
            for _ in 0..100 {
                state_src.push_str("}\n");
            }
            assert!(
                renderer.render(&state_src).is_err(),
                "100-level composite state should fail cleanly at the container depth limit"
            );

            // Syntactically flat subgraphs linked via member IDs form a deep parent chain.
            let mut flow_src = String::from("flowchart TD\n");
            for n in 0..200 {
                flow_src.push_str(&format!("subgraph g{n}\ng{}\nend\n", n + 1));
            }
            flow_src.push_str("subgraph g200\nleaf\nend\n");
            assert!(
                renderer.render(&flow_src).is_err(),
                "200-level member-linked subgraph chain should fail cleanly"
            );
        })
        .expect("spawn thread");
    handle
        .join()
        .expect("deep container hierarchies did not overflow");
}

#[test]
fn deep_elk_chain_does_not_overflow() {
    let handle = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let config = Value::from(serde_json::json!({
                "maxEdges": 2000,
                "maxTextSize": 200000
            }));
            let renderer = Renderer::new(&config);
            let mut elk_chain = String::from("---\nconfig:\n  layout: elk\n---\nflowchart TD\n");
            for n in 0..600 {
                elk_chain.push_str(&format!("n{n} --> n{}\n", n + 1));
            }
            let elk_svg = renderer
                .render(&elk_chain)
                .expect("deep ELK chain renders cleanly");
            finite_geometry(&elk_svg, &elk_chain, "deep ELK chain");
        })
        .expect("spawn thread");
    handle.join().expect("deep ELK chain did not overflow");
}

#[test]
fn deeply_nested_config_value_does_not_overflow() {
    use std::collections::BTreeMap;

    use ferrigura_model::config::{assign_with_depth, deep_merge, effective};
    use ferrigura_model::value::drop_iterative;

    let renderer = Renderer::new(&Value::empty_object());
    let control = renderer.render(CONTROL).expect("control renders");

    let mut deep_obj = Value::String("forest".to_owned());
    let mut deep_arr = Value::String("leaf".to_owned());
    for _ in 0..5_000 {
        let mut map = BTreeMap::new();
        map.insert("flowchart".to_owned(), deep_obj);
        deep_obj = Value::Object(map);
        deep_arr = Value::Array(vec![deep_arr]);
    }
    let eff_obj = effective(Some(&deep_obj), Some("flowchart"), &deep_obj);
    let eff_arr = effective(Some(&deep_arr), None, &deep_arr);
    let mut merged = Value::empty_object();
    deep_merge(&mut merged, &deep_obj);
    assign_with_depth(&mut merged, &deep_arr, 2);
    let deep_renderer = Renderer::new(&deep_obj);
    let deep_control = deep_renderer
        .render(CONTROL)
        .expect("control renders with deep config");
    finite_geometry(&deep_control, CONTROL, "deep config control");
    let deep_flow = deep_renderer
        .render("flowchart LR\nA --> B\n")
        .expect("flowchart renders with deep config");
    finite_geometry(
        &deep_flow,
        "flowchart LR\nA --> B\n",
        "deep config flowchart",
    );
    let shorthand_control =
        ferrigura::render(CONTROL, &deep_obj).expect("shorthand render handles deep config");
    finite_geometry(&shorthand_control, CONTROL, "deep config shorthand");

    // Host theme overrides with deeply nested arrays (e.g. THEME_COLOR_LIMIT) must not overflow
    // `theme::helpers::string_of`, and deep configs with different themes must get distinct IDs.
    let mut deep_limit = Value::String("8".to_owned());
    for _ in 0..2_000 {
        deep_limit = Value::Array(vec![deep_limit]);
    }
    let mut dark_cfg = BTreeMap::new();
    dark_cfg.insert("theme".to_owned(), Value::String("dark".to_owned()));
    dark_cfg.insert(
        "themeVariables".to_owned(),
        Value::Object(BTreeMap::from([(
            "THEME_COLOR_LIMIT".to_owned(),
            deep_limit.clone(),
        )])),
    );
    let dark_cfg = Value::Object(dark_cfg);
    let mut forest_cfg = BTreeMap::new();
    forest_cfg.insert("theme".to_owned(), Value::String("forest".to_owned()));
    forest_cfg.insert(
        "themeVariables".to_owned(),
        Value::Object(BTreeMap::from([(
            "THEME_COLOR_LIMIT".to_owned(),
            deep_limit,
        )])),
    );
    let forest_cfg = Value::Object(forest_cfg);
    let dark_svg = Renderer::new(&dark_cfg)
        .render(CONTROL)
        .expect("deep dark config renders");
    let forest_svg = Renderer::new(&forest_cfg)
        .render(CONTROL)
        .expect("deep forest config renders");
    let extract_id = |svg: &str| {
        let start = svg.find("id=\"").expect("svg has id") + 4;
        let end = svg[start..].find('"').expect("closing quote");
        svg[start..start + end].to_owned()
    };
    assert_ne!(
        extract_id(&dark_svg),
        extract_id(&forest_svg),
        "deep configs with different themes must not collide on derived SVG ID"
    );

    // Deep configs differing only in +0.0 vs -0.0 must also keep distinct derived IDs.
    let mut deep_pad = Value::Null;
    for _ in 0..200 {
        deep_pad = Value::Array(vec![deep_pad]);
    }
    let pos_zero_cfg = Value::Object(BTreeMap::from([
        ("x".to_owned(), Value::Number(0.0)),
        ("deep".to_owned(), deep_pad.clone()),
    ]));
    let neg_zero_cfg = Value::Object(BTreeMap::from([
        ("x".to_owned(), Value::Number(-0.0)),
        ("deep".to_owned(), deep_pad),
    ]));
    let pos_svg = Renderer::new(&pos_zero_cfg)
        .render(CONTROL)
        .expect("pos zero deep config renders");
    let neg_svg = Renderer::new(&neg_zero_cfg)
        .render(CONTROL)
        .expect("neg zero deep config renders");
    assert_ne!(
        extract_id(&pos_svg),
        extract_id(&neg_svg),
        "deep configs differing in +0.0 vs -0.0 must not collide on derived SVG ID"
    );

    // Replacing a deep target in `deep_merge` and removing an unknown deep key in
    // `sanitize_directive` must drop iteratively without overflowing a 2 MiB stack.
    let mut deep_target = Value::Null;
    let mut unknown_deep = Value::Null;
    for _ in 0..5_000 {
        deep_target = Value::Array(vec![deep_target]);
        unknown_deep = Value::Array(vec![unknown_deep]);
    }
    deep_merge(&mut deep_target, &Value::Null);
    let mut dir_with_unknown =
        Value::Object(BTreeMap::from([("unknown".to_owned(), unknown_deep)]));
    ferrigura_model::config::sanitize_directive(&mut dir_with_unknown);

    drop_iterative(deep_obj);
    drop_iterative(deep_arr);
    drop_iterative(eff_obj);
    drop_iterative(eff_arr);
    drop_iterative(merged);
    drop_iterative(dark_cfg);
    drop_iterative(forest_cfg);
    drop_iterative(pos_zero_cfg);
    drop_iterative(neg_zero_cfg);
    assert_eq!(renderer.render(CONTROL).unwrap(), control);
}
