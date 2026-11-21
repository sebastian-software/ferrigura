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
