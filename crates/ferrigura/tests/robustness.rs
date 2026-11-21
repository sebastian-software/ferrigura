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
                roxmltree::Document::parse(&svg)
                    .unwrap_or_else(|error| panic!("invalid XML for {text:?}: {error}"));
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
            roxmltree::Document::parse(&svg)
                .unwrap_or_else(|error| panic!("invalid XML for {text:?}: {error}"));
        }
        assert_eq!(
            renderer.render(CONTROL).unwrap(),
            expected,
            "after {text:?}"
        );
    }
}
