//! The 24 `mermaid-example` blocks of the syntax page of use case diagrams, which the reference's
//! `usecase.docs.spec.ts` renders. Each block must parse; the rendering is out of this port's scope.

use ferrigura_syntax::use_case::parse;

const DOCS: &str = include_str!("../../../packages/mermaid/src/docs/syntax/usecase.md");

/// The sources of the `mermaid-example` fences, in order.
fn examples(markdown: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut start = None;
    let mut offset = 0;
    for line in markdown.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        match start {
            None if content.starts_with("```mermaid-example") => start = Some(offset + line.len()),
            Some(begin) if content == "```" => {
                found.push(markdown[begin..offset].trim_end_matches(['\r', '\n']));
                start = None;
            }
            _ => {}
        }
        offset += line.len();
    }
    found
}

#[test]
fn the_page_has_twenty_four_examples() {
    assert_eq!(examples(DOCS).len(), 24);
}

#[test]
fn every_documentation_example_parses() {
    for (index, source) in examples(DOCS).iter().enumerate() {
        if let Err(error) = parse(source) {
            panic!(
                "documentation example {} does not parse: {error}\n{source}",
                index + 1
            );
        }
    }
}
