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
        format!("quadrantChart\ntitle {TYPED}\nx-axis {TYPED}\nPoint {TYPED}: [0.3, 0.6]\n"),
        format!("xychart\ntitle \"{TYPED}\"\nx-axis [\"{TYPED}\", \"c\"]\nbar [1, 2]\n"),
        format!(
            "gantt\ndateFormat YYYY-MM-DD\ntitle {TYPED}\nsection s\ntask :t1, 2024-01-01, 1d\n"
        ),
        format!("flowchart LR\nA[{TYPED}] --> B\n"),
        "flowchart LR\nA@{ icon: \"fa:user\", label: \"User\" }\nstyle A stroke:red\"x&y\n"
            .to_owned(),
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

#[test]
fn style_breakout_payloads_stay_well_formed_without_script_injection() {
    let payload = "16px}</style><script>alert(1)</script><style>";
    let cases: [(&str, &str, serde_json::Value, bool); 11] = [
        (
            "pie-directive",
            "%%{init: {'themeVariables': {'pieTitleTextSize': '25px}</style><script>alert(1)</script><style>'}}}%%\npie\naccTitle: T\naccDescr: D\n\"A\": 60\n\"B\": 40\n",
            serde_json::json!({}),
            true,
        ),
        (
            "pie",
            "pie\naccTitle: T\naccDescr: D\n\"A\": 60\n\"B\": 40\n",
            serde_json::json!({
                "themeVariables": {
                    "pieTitleTextSize": payload,
                    "pieSectionTextSize": payload,
                    "pieLegendTextSize": payload,
                    "pieStrokeWidth": payload,
                    "pieOuterStrokeWidth": payload,
                    "pieOpacity": payload,
                    "fontFamily": payload,
                }
            }),
            true,
        ),
        (
            "gantt",
            "gantt\naccTitle: T\naccDescr: D\ndateFormat YYYY-MM-DD\nsection S\nTask :t1, 2024-01-01, 2d\n",
            serde_json::json!({
                "themeVariables": {
                    "fontSize": payload,
                    "fontFamily": payload,
                    "titleColor": payload,
                }
            }),
            true,
        ),
        (
            "flowchart",
            "flowchart LR\naccTitle: T\naccDescr: D\nA --> B\nclassDef ok fill:#f00\nclass A ok\n",
            serde_json::json!({
                "themeVariables": {
                    "nodeBorder": payload,
                    "fontFamily": payload,
                }
            }),
            true,
        ),
        (
            "state",
            "stateDiagram-v2\naccTitle: T\naccDescr: D\n[*] --> Idle\nclassDef evil fill:red} body{display:none} .x{color:red\nclass Idle evil\n",
            serde_json::json!({
                "themeVariables": {
                    "stateBkg": payload,
                    "fontFamily": payload,
                }
            }),
            false,
        ),
        (
            "class",
            "classDiagram\naccTitle: T\naccDescr: D\nclass Animal\nAnimal <|-- Dog\n",
            serde_json::json!({
                "themeVariables": {
                    "nodeBorder": payload,
                    "classText": payload,
                }
            }),
            false,
        ),
        (
            "er",
            "erDiagram\naccTitle: T\naccDescr: D\nCUSTOMER ||--o{ ORDER : places\n",
            serde_json::json!({
                "look": "neo]{body{display:none}",
                "themeVariables": {
                    "nodeBorder": payload,
                    "mainBkg": payload,
                }
            }),
            false,
        ),
        (
            "mindmap",
            "mindmap\n  Root\n    Leaf\n",
            serde_json::json!({
                "themeVariables": {
                    "nodeBorder": payload,
                    "fontFamily": payload,
                }
            }),
            true,
        ),
        (
            "sequence",
            "sequenceDiagram\naccTitle: T\naccDescr: D\nAlice->>Bob: hi\n",
            serde_json::json!({
                "themeVariables": {
                    "actorBorder": payload,
                    "fontFamily": payload,
                }
            }),
            false,
        ),
        (
            "xychart",
            "xychart-beta\naccTitle: T\naccDescr: D\nx-axis [a, b]\nbar [1, 2]\n",
            serde_json::json!({
                "themeVariables": {
                    "textColor": payload,
                    "fontFamily": payload,
                }
            }),
            true,
        ),
        (
            "quadrant",
            "quadrantChart\naccTitle: T\naccDescr: D\nx-axis Low --> High\ny-axis Low --> High\nPoint A: [0.3, 0.6]\n",
            serde_json::json!({
                "themeVariables": {
                    "textColor": payload,
                    "fontFamily": payload,
                }
            }),
            false,
        ),
    ];

    for (name, text, config, animated) in cases {
        let config = Value::from(config);
        let options = RenderOptions {
            now: Some(NOW),
            animation: animated.then(ferrigura::Animation::default),
            ..RenderOptions::default()
        };
        let svg = render_with(text, &config, &options)
            .unwrap_or_else(|error| panic!("{name} failed to render: {error}"));
        let doc = roxmltree::Document::parse(&svg)
            .unwrap_or_else(|error| panic!("{name} is not well-formed XML ({error}): {svg}"));
        assert!(
            !doc.descendants().any(|node| node.has_tag_name("script")),
            "{name} injected a <script> element"
        );
        assert!(
            !svg.contains("</style><script>"),
            "{name} broke out of <style>"
        );
        if name == "state" || name == "er" {
            assert!(
                !svg.contains("body{display:none}"),
                "{name} broke out of diagram id scope"
            );
        }
    }
}

#[test]
fn direct_renderers_escape_special_characters_in_svg_id() {
    use ferrigura_model::config::{MermaidConfig, defaults, typed};
    use ferrigura_model::theme::{Theme, ThemeTokens};
    use ferrigura_render::context::{Context, Globals, ThemeView};

    let config: MermaidConfig = typed(&defaults()).unwrap();
    let globals = Globals::of(&config);
    let tokens = ThemeTokens::from_value(&Theme::ReduxColor.variables(None));
    let theme = ThemeView {
        light: &tokens,
        dark: None,
    };
    let bad_id = "id\"&</style><script>alert(1)</script>";
    let anim = Some(ferrigura_render::animation::Animation::default());
    let unified = ferrigura_render::unified::Section::of(&config);

    let pie_model =
        ferrigura_syntax::pie::parse("pie\naccTitle: T\naccDescr: D\n\"A\": 60\n\"B\": 40\n")
            .unwrap();
    let gantt_model = ferrigura_syntax::gantt::parse(
        "gantt\naccTitle: T\naccDescr: D\ndateFormat YYYY-MM-DD\nsection S\nTask :t1, 2024-01-01, 2d\n",
    )
    .unwrap();
    let flow_model =
        ferrigura_syntax::flowchart::parse("flowchart LR\naccTitle: T\naccDescr: D\nA --> B\n")
            .unwrap();
    let state_model =
        ferrigura_syntax::state::parse("stateDiagram-v2\naccTitle: T\naccDescr: D\n[*] --> Idle\n")
            .unwrap();
    let class_model = ferrigura_syntax::class::parse(
        "classDiagram\naccTitle: T\naccDescr: D\nclass Animal\nAnimal <|-- Dog\n",
    )
    .unwrap();
    let er_model = ferrigura_syntax::er::parse(
        "erDiagram\naccTitle: T\naccDescr: D\nCUSTOMER ||--o{ ORDER : places\n",
    )
    .unwrap();
    let mindmap_model = ferrigura_syntax::mindmap::parse("mindmap\n  Root\n    Leaf\n").unwrap();
    let sequence_model = ferrigura_syntax::sequence::parse(
        "sequenceDiagram\naccTitle: T\naccDescr: D\nAlice->>Bob: hi\n",
    )
    .unwrap();
    let xychart_model = ferrigura_syntax::xychart::parse(
        "xychart-beta\naccTitle: T\naccDescr: D\nx-axis [a, b]\nbar [1, 2]\n",
    )
    .unwrap();
    let quadrant_model = ferrigura_syntax::quadrant_chart::parse(
        "quadrantChart\naccTitle: T\naccDescr: D\nx-axis Low --> High\ny-axis Low --> High\nPoint A: [0.3, 0.6]\n",
    )
    .unwrap();

    let gantt_section = ferrigura_render::gantt::GanttSection {
        config: &config.gantt,
        now: NOW,
    };
    let mindmap_section = ferrigura_render::mindmap::MindmapSection {
        mindmap: &config.mindmap,
        unified,
    };

    let svgs: [(&str, String); 10] = [
        (
            "pie",
            ferrigura_render::pie::render_with_animation(
                &Context {
                    fonts: None,
                    diagram: &pie_model,
                    section: &config.pie,
                    globals: &globals,
                    theme,
                    id: bad_id,
                },
                anim,
            )
            .unwrap(),
        ),
        (
            "gantt",
            ferrigura_render::gantt::render_with_animation(
                &Context {
                    fonts: None,
                    diagram: &gantt_model,
                    section: &gantt_section,
                    globals: &globals,
                    theme,
                    id: bad_id,
                },
                anim,
            )
            .unwrap(),
        ),
        (
            "flowchart",
            ferrigura_render::flowchart::render_with_animation(
                &Context {
                    fonts: None,
                    diagram: &flow_model,
                    section: &unified,
                    globals: &globals,
                    theme,
                    id: bad_id,
                },
                anim,
            )
            .unwrap(),
        ),
        (
            "state",
            ferrigura_render::state::render(&Context {
                fonts: None,
                diagram: &state_model,
                section: &unified,
                globals: &globals,
                theme,
                id: bad_id,
            })
            .unwrap(),
        ),
        (
            "class",
            ferrigura_render::class::render(&Context {
                fonts: None,
                diagram: &class_model,
                section: &unified,
                globals: &globals,
                theme,
                id: bad_id,
            })
            .unwrap(),
        ),
        (
            "er",
            ferrigura_render::er::render(&Context {
                fonts: None,
                diagram: &er_model,
                section: &unified,
                globals: &globals,
                theme,
                id: bad_id,
            })
            .unwrap(),
        ),
        (
            "mindmap",
            ferrigura_render::mindmap::render_with_animation(
                &Context {
                    fonts: None,
                    diagram: &mindmap_model,
                    section: &mindmap_section,
                    globals: &globals,
                    theme,
                    id: bad_id,
                },
                anim,
            )
            .unwrap(),
        ),
        (
            "sequence",
            ferrigura_render::sequence::render(&Context {
                fonts: None,
                diagram: &sequence_model,
                section: &config.sequence,
                globals: &globals,
                theme,
                id: bad_id,
            })
            .unwrap(),
        ),
        (
            "xychart",
            ferrigura_render::xychart::render_with_animation(
                &Context {
                    fonts: None,
                    diagram: &xychart_model,
                    section: &config.xy_chart,
                    globals: &globals,
                    theme,
                    id: bad_id,
                },
                anim,
            )
            .unwrap(),
        ),
        (
            "quadrant",
            ferrigura_render::quadrant_chart::render(&Context {
                fonts: None,
                diagram: &quadrant_model,
                section: &config.quadrant_chart,
                globals: &globals,
                theme,
                id: bad_id,
            })
            .unwrap(),
        ),
    ];

    for (name, svg) in svgs {
        let doc = roxmltree::Document::parse(&svg)
            .unwrap_or_else(|error| panic!("{name} is not well-formed XML ({error}): {svg}"));
        assert!(
            !doc.descendants().any(|node| node.has_tag_name("script")),
            "{name} injected a <script> element"
        );
        assert!(
            !svg.contains("</style><script>"),
            "{name} broke out of <style>"
        );
    }
}

#[test]
fn css_ampersand_in_theme_variables_is_not_double_escaped() {
    let cases: [(&str, &str, &str); 10] = [
        ("flowchart", "flowchart LR\nA --> B\n", "nodeBorder"),
        ("class", "classDiagram\nclass Animal\n", "nodeBorder"),
        ("er", "erDiagram\nA ||--o{ B : has\n", "nodeBorder"),
        ("state", "stateDiagram-v2\n[*] --> A\n", "stateBkg"),
        ("pie", "pie\n\"A\": 1\n", "pieStrokeColor"),
        (
            "gantt",
            "gantt\ndateFormat YYYY-MM-DD\nsection S\nTask :t1, 2024-01-01, 2d\n",
            "titleColor",
        ),
        ("mindmap", "mindmap\n  Root\n    Leaf\n", "nodeBorder"),
        ("sequence", "sequenceDiagram\nA->>B: hi\n", "actorBorder"),
        (
            "xychart",
            "xychart-beta\nx-axis [a, b]\nbar [1, 2]\n",
            "textColor",
        ),
        (
            "quadrant",
            "quadrantChart\nx-axis Low --> High\ny-axis Low --> High\nPoint A: [0.3, 0.6]\n",
            "textColor",
        ),
    ];
    for (name, text, token) in cases {
        let config = Value::from(serde_json::json!({
            "themeVariables": {
                token: "x&y",
            }
        }));
        let options = RenderOptions {
            now: Some(NOW),
            ..RenderOptions::default()
        };
        let svg = render_with(text, &config, &options)
            .unwrap_or_else(|error| panic!("{name} failed to render: {error}"));
        roxmltree::Document::parse(&svg)
            .unwrap_or_else(|error| panic!("{name} is not well-formed XML: {error}"));
        assert!(
            svg.contains("x&amp;y"),
            "{name} did not preserve single-escaped ampersand"
        );
        assert!(
            !svg.contains("&amp;amp;"),
            "{name} double-escaped ampersand in CSS"
        );
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
    // The corpus holds about 1460 inputs, of which ferrigura renders about 1320: a walk that finds
    // far fewer has lost a folder.
    assert!(rendered > 1200, "only {rendered} inputs rendered");
    assert!(
        failures.is_empty(),
        "{} of {rendered} SVGs are not well-formed XML:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn theme_variables_array_and_font_family_cannot_inject_global_css() {
    // 1. Front-matter `themeVariables` as an array must not bypass sanitization and break out of
    // scoped rules in pie or other renderers.
    let pie_array = "\
---
config:
  themeVariables:
    - pieTitleTextSize: \"25px;} body { display: none; } .x { font-size: 1px\"
---
pie
  \"A\" : 1
";
    let empty = ferrigura::Value::empty_object();
    let svg = ferrigura::render(pie_array, &empty).expect("pie with array themeVariables renders");
    roxmltree::Document::parse(&svg).expect("well-formed XML");
    assert!(
        !svg.contains("body { display: none; }") && !svg.contains("display: none"),
        "themeVariables array broke out of scoped CSS: {svg}"
    );

    // 2. Top-level `fontFamily` directives (both semicolon declarations and balanced braces) must
    // not inject global CSS rules or extra declarations into `:root`.
    for (label, diagram) in [
        ("sequence", "sequenceDiagram\n  A->>B: hi\n"),
        ("mindmap", "mindmap\n  root\n    leaf\n"),
        ("xychart", "xychart-beta\n  x-axis [a, b]\n  bar [1, 2]\n"),
        (
            "quadrant",
            "quadrantChart\n  x-axis L --> R\n  y-axis B --> T\n  p: [0.5, 0.5]\n",
        ),
        ("pie", "pie\n  \"A\" : 1\n"),
        (
            "gantt",
            "gantt\n  dateFormat YYYY-MM-DD\n  section S\n  T : 2024-01-01, 1d\n",
        ),
    ] {
        for payload in [
            "sans-serif; background: red; display: none",
            "sans-serif} body{display:none} :root{--x:1",
            "a { b",
        ] {
            for source in [
                format!("---\nconfig:\n  fontFamily: \"{payload}\"\n---\n{diagram}"),
                format!(
                    "---\nconfig:\n  themeVariables:\n    fontFamily: \"{payload}\"\n---\n{diagram}"
                ),
            ] {
                let svg = ferrigura::render(&source, &empty)
                    .unwrap_or_else(|e| panic!("{label} with fontFamily {payload:?} failed: {e}"));
                roxmltree::Document::parse(&svg).unwrap_or_else(|e| {
                    panic!("{label} with fontFamily {payload:?} invalid XML: {e}")
                });
                assert!(
                    !svg.contains("display: none")
                        && !svg.contains("display:none")
                        && !svg.contains("background: red")
                        && !svg.contains("body{")
                        && !svg.contains("ERROR: Unbalanced CSS"),
                    "{label} injected CSS via fontFamily {payload:?}: {svg}"
                );
            }
        }
    }
}

mod accessibility;
