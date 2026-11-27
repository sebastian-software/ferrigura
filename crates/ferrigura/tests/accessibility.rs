//! Accessible metadata and local ID references at the public renderer boundary.

use std::collections::HashSet;

use ferrigura::{RenderOptions, Renderer};
use ferrigura_model::value::Value;
use roxmltree::{Document, Node};

const CASES: [(&str, &str); 5] = [
    ("pie", "pie\n\"Cats\": 2\n"),
    ("flowchart", "flowchart LR\nA-->B\n"),
    ("sequence", "sequenceDiagram\nAlice->>Bob: Hello\n"),
    ("state", "stateDiagram-v2\n[*] --> Ready\n"),
    ("class", "classDiagram\nclass Animal\n"),
];

fn with_metadata(text: &str, title: &str, description: &str) -> String {
    let (header, body) = text.split_once('\n').unwrap();
    format!("{header}\naccTitle: {title}\naccDescr: {description}\n{body}")
}

fn render(renderer: &Renderer, text: &str, id: &str) -> String {
    renderer
        .render_with(
            text,
            &RenderOptions {
                id: Some(id.to_owned()),
                ..RenderOptions::default()
            },
        )
        .expect("diagram renders")
}

fn metadata(svg: Node<'_, '_>, kind: &str, id: &str, title: &str, description: &str) {
    assert_eq!(svg.attribute("id"), Some(id), "{kind}: root ID");
    assert_eq!(
        svg.attribute("role"),
        Some("graphics-document document"),
        "{kind}: {id} role"
    );
    for (attr, tag, expected) in [
        ("aria-labelledby", "title", title),
        ("aria-describedby", "desc", description),
    ] {
        let reference = svg.attribute(attr).expect("reference exists");
        let targets: Vec<_> = svg
            .descendants()
            .filter(|node| node.attribute("id") == Some(reference))
            .collect();
        assert_eq!(targets.len(), 1, "{kind}: {attr} resolves within {id}");
        assert_eq!(targets[0].tag_name().name(), tag, "{kind}: {id} {attr}");
        assert_eq!(targets[0].text(), Some(expected), "{kind}: {id} {attr}");
    }
}

#[test]
fn metadata_references_remain_local_and_do_not_leak_between_renders() {
    let renderer = Renderer::new(&Value::empty_object());
    for (kind, text) in CASES {
        let first = render(
            &renderer,
            &with_metadata(text, "First & title", "First description"),
            "chart-one",
        );
        let second = render(
            &renderer,
            &with_metadata(text, "Second & title", "Second description"),
            "chart-two",
        );
        let document = format!("<root>{first}{second}</root>");
        let parsed = Document::parse(&document).expect("both SVGs are XML");
        let roots: Vec<_> = parsed
            .root_element()
            .children()
            .filter(Node::is_element)
            .collect();
        assert_eq!(roots.len(), 2, "{kind}");
        metadata(
            roots[0],
            kind,
            "chart-one",
            "First & title",
            "First description",
        );
        metadata(
            roots[1],
            kind,
            "chart-two",
            "Second & title",
            "Second description",
        );
        let references: Vec<_> = roots
            .iter()
            .flat_map(|svg| {
                [
                    svg.attribute("aria-labelledby").unwrap(),
                    svg.attribute("aria-describedby").unwrap(),
                ]
            })
            .collect();
        for reference in &references {
            assert_eq!(
                parsed
                    .descendants()
                    .filter(|node| node.attribute("id") == Some(*reference))
                    .count(),
                1,
                "{kind}: {reference} resolves once in the document"
            );
        }
        assert_eq!(
            references.iter().copied().collect::<HashSet<_>>().len(),
            4,
            "{kind}"
        );
        let empty = render(&renderer, text, "chart-empty");
        let empty = Document::parse(&empty).unwrap();
        let svg = empty.root_element();
        assert_eq!(svg.attribute("aria-labelledby"), None, "{kind}");
        assert_eq!(svg.attribute("aria-describedby"), None, "{kind}");
        assert!(
            !svg.children()
                .any(|node| node.has_tag_name("title") || node.has_tag_name("desc")),
            "{kind}"
        );
    }
}

/// The same selected corpus cases are compared with the pinned reference by the port harness.
#[test]
fn accessibility_across_registered_kinds() {
    use ferrigura::diagrams;
    use std::path::Path;
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("accessibility-cases.json")).unwrap();
    let renderer = Renderer::new(&Value::empty_object());
    let mut seen = HashSet::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for case in cases.as_array().unwrap() {
        let path = case["path"].as_str().unwrap();
        let mode = case["mode"].as_str().unwrap();
        let original = std::fs::read_to_string(root.join(path)).unwrap();
        let body = original
            .strip_prefix("---\n")
            .and_then(|s| s.split_once("\n---\n").map(|(_, body)| body))
            .unwrap_or(&original);
        let clean = body
            .lines()
            .filter(|line| !line.trim_start().starts_with("%%"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let kind = ferrigura_syntax::detect(&clean).unwrap_or_else(|| panic!("{path}: detection"));
        assert!(seen.insert(kind), "{path}: duplicate kind {kind:?}");
        let make = |title: &str, description: &str| {
            if mode == "unsupported" {
                return clean.clone();
            }
            let (head, tail) = clean.split_once('\n').unwrap();
            format!("{head}\naccTitle: {title}\naccDescr: {description}\n{tail}")
        };
        let first = render(
            &renderer,
            &make("First & title", "First description"),
            "all-one",
        );
        let second = render(
            &renderer,
            &make("Second & title", "Second description"),
            "all-two",
        );
        let both = format!("<root>{first}{second}</root>");
        let parsed = Document::parse(&both).unwrap_or_else(|error| panic!("{path}: {error}"));
        let svgs: Vec<_> = parsed
            .root_element()
            .children()
            .filter(Node::is_element)
            .collect();
        assert_eq!(svgs.len(), 2, "{path}");
        for (svg, id, title, description) in [
            (svgs[0], "all-one", "First & title", "First description"),
            (svgs[1], "all-two", "Second & title", "Second description"),
        ] {
            assert_eq!(svg.attribute("id"), Some(id), "{path}");
            assert_eq!(
                svg.attribute("role"),
                Some("graphics-document document"),
                "{path}"
            );
            let expected_title = match mode {
                "unsupported" | "omitted" => None,
                _ => Some(title.to_owned()),
            };
            let expected_desc = match mode {
                "unsupported" | "omitted" => None,
                _ => Some(description.to_owned()),
            };
            for (attr, tag, expected) in [
                ("aria-labelledby", "title", expected_title),
                ("aria-describedby", "desc", expected_desc),
            ] {
                if let Some(expected) = expected {
                    let reference = svg
                        .attribute(attr)
                        .unwrap_or_else(|| panic!("{path}: {id} {attr}"));
                    let matches: Vec<_> = parsed
                        .descendants()
                        .filter(|node| node.attribute("id") == Some(reference))
                        .collect();
                    assert_eq!(matches.len(), 1, "{path}: {id} {attr} globally");
                    assert!(
                        matches[0].ancestors().any(|ancestor| ancestor == svg),
                        "{path}: {id} {attr} local"
                    );
                    assert_eq!(matches[0].tag_name().name(), tag, "{path}: {id} {attr}");
                    assert_eq!(
                        matches[0].text(),
                        Some(expected.as_str()),
                        "{path}: {id} {attr}"
                    );
                } else {
                    assert_eq!(svg.attribute(attr), None, "{path}: {id} {attr}");
                }
            }
        }
        let empty = render(&renderer, &clean, "all-empty");
        let doc = Document::parse(&empty).unwrap();
        assert_eq!(
            doc.root_element().attribute("aria-labelledby"),
            None,
            "{path}: leakage"
        );
        assert_eq!(
            doc.root_element().attribute("aria-describedby"),
            None,
            "{path}: leakage"
        );
    }
    let registered: HashSet<_> = diagrams::all().iter().map(diagrams::Plugin::kind).collect();
    assert_eq!(
        seen, registered,
        "every registered kind has an accessibility case"
    );
}
