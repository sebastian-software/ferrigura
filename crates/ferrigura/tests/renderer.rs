//! The `Renderer`: one that resolves its configuration once gives the output of the shorthand
//! `render_with`, byte for byte, for a text without directives, one with directives and front matter,
//! and one that changes the themes; and it serves renders on several threads at once.

use std::thread;

use ferrigura::{ColorScheme, RenderOptions, Renderer, render, render_with};
use ferrigura_model::value::Value;

const PIE: &str = "pie title Pets\n  \"Dogs\" : 386\n  \"Cats\" : 85\n";
const FLOWCHART: &str = "flowchart LR\n  A[Start] --> B{Choice}\n  B -->|yes| C[Done]\n";
const SEQUENCE: &str = "sequenceDiagram\n  Alice->>Bob: Hello\n  Bob-->>Alice: Hi\n";
const DIRECTED_PIE: &str =
    "%%{init: {\"theme\": \"forest\"}}%%\npie\n  \"Dogs\" : 386\n  \"Cats\" : 85\n";
const FRONT_MATTER_PIE: &str = "---\nconfig:\n  theme: dark\n  themeVariables:\n    pie1: \"#00ff00\"\n---\npie\n  \"Dogs\" : 386\n";

const TEXTS: [&str; 5] = [PIE, FLOWCHART, SEQUENCE, DIRECTED_PIE, FRONT_MATTER_PIE];

const SCHEMES: [Option<ColorScheme>; 4] = [
    None,
    Some(ColorScheme::Light),
    Some(ColorScheme::Dark),
    Some(ColorScheme::Auto),
];

/// A configuration, as JSON.
fn value(json: &str) -> Value {
    Value::from(serde_json::from_str::<serde_json::Value>(json).expect("the JSON parses"))
}

fn configurations() -> Vec<Value> {
    vec![
        value("{}"),
        value(
            r##"{"themeVariables":{"primaryColor":"#336699","lineColor":"#ff00aa"},"fontFamily":"Arial"}"##,
        ),
        value(r#"{"theme":"redux-dark-color","look":"classic"}"#),
        value(r#"{"theme":"forest","look":"handDrawn","layout":"elk"}"#),
    ]
}

/// The output of a render, or its message, so that the results compare.
fn outcome(result: Result<String, ferrigura::Error>) -> Result<String, String> {
    result.map_err(|error| error.to_string())
}

#[test]
fn a_renderer_gives_the_output_of_the_shorthand_for_every_text_and_scheme() {
    for config in configurations() {
        // One renderer for all the texts, so that a render cannot leave anything behind for the next.
        let renderer = Renderer::new(&config);
        for text in TEXTS {
            for scheme in SCHEMES {
                let options = RenderOptions {
                    color_scheme: scheme,
                    ..RenderOptions::default()
                };
                assert_eq!(
                    outcome(renderer.render_with(text, &options)),
                    outcome(render_with(text, &config, &options)),
                    "{config:?} with {scheme:?} for {text:?}"
                );
            }
        }
    }
}

#[test]
fn the_default_render_is_the_default_shorthand() {
    let renderer = Renderer::new(&Value::empty_object());
    for text in TEXTS {
        assert_eq!(
            outcome(renderer.render(text)),
            outcome(render(text, &Value::empty_object()))
        );
    }
}

#[test]
fn a_renderer_of_a_configuration_that_is_not_an_object_gives_the_shorthand_output() {
    // Such a configuration reads as the defaults, and its id is derived from `null` as it always was.
    let renderer = Renderer::new(&Value::Null);
    assert_eq!(
        outcome(renderer.render(PIE)),
        outcome(render(PIE, &Value::Null))
    );
}

#[test]
fn a_renderer_gives_the_errors_of_the_shorthand() {
    let renderer = Renderer::new(&Value::empty_object());
    assert_eq!(
        outcome(renderer.render("not a diagram\n")),
        outcome(render("not a diagram\n", &Value::empty_object()))
    );
    let options = RenderOptions {
        id: Some("1st".to_owned()),
        ..RenderOptions::default()
    };
    assert_eq!(
        outcome(renderer.render_with(PIE, &options)),
        outcome(render_with(PIE, &Value::empty_object(), &options))
    );
    assert!(renderer.render_with(PIE, &options).is_err());
}

#[test]
fn one_renderer_serves_renders_on_several_threads() {
    let renderer = Renderer::new(&value(r#"{"theme":"forest"}"#));
    let expected: Vec<String> = TEXTS
        .iter()
        .map(|text| renderer.render(text).expect("the text renders"))
        .collect();
    thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                for (text, svg) in TEXTS.iter().zip(&expected) {
                    assert_eq!(&renderer.render(text).expect("the text renders"), svg);
                }
            });
        }
    });
}

#[test]
fn the_shorthand_gives_the_output_of_a_fresh_renderer_after_its_cache_is_full() {
    // More configurations than the cache keeps: each one is resolved again when it comes back.
    let configs: Vec<Value> = (0..12)
        .map(|index| value(&format!(r#"{{"pie":{{"textPosition":0.{}}}}}"#, 30 + index)))
        .collect();
    for _ in 0..2 {
        for config in &configs {
            assert_eq!(
                outcome(render(PIE, config)),
                outcome(Renderer::new(config).render(PIE))
            );
        }
    }
}

#[test]
fn configurations_that_differ_only_in_the_sign_of_zero_have_their_own_ids() {
    // The derived id hashes the configuration as it serialises, so `-0` and `0` give different ids,
    // and the shorthand must not hand the renderer of one to the other.
    let negative = value(r#"{"unused":-0.0}"#);
    let positive = value(r#"{"unused":0.0}"#);
    let negative_svg = render(PIE, &negative).expect("the pie renders");
    let positive_svg = render(PIE, &positive).expect("the pie renders");
    assert_ne!(negative_svg, positive_svg);
    assert_eq!(
        negative_svg,
        Renderer::new(&negative)
            .render(PIE)
            .expect("the pie renders")
    );
    assert_eq!(
        positive_svg,
        Renderer::new(&positive)
            .render(PIE)
            .expect("the pie renders")
    );
}
