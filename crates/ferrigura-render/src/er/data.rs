//! The ER diagram as the layout and the renderer read it: a port of `getData` in
//! `packages/mermaid/src/diagrams/er/erDb.ts` (lines 377-459), which fills the layout data of
//! [`crate::unified::data`]. The differential run (`pnpm port:diff er-data`) compares it with the
//! reference's `getData()`; see `docs-port/er/NOTES.md`, section 5.2.
//!
//! The reference's `getData` gives the subgraphs first, in reverse closing order, then the
//! entities in insertion order, then the relationships. An entity's `padding` is undefined in the
//! reference, and so is its `labelStyle`; the renderer never reads either for an entity (`erBox`
//! takes its paddings from the configuration, and `labelHelper` returns the half padding without
//! using it), so the layout data holds `0` and `""`.

use std::collections::{HashMap, HashSet};

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::er::{Cardinality, ErClass, ErDiagram, Identification};
use ferrigura_model::flowchart::LabelType;

use crate::unified::data::EntityFields;
pub use crate::unified::data::{Edge, LayoutData, Node};

/// The padding of a subgraph node, which the reference hard-codes (`erDb.ts` 400).
const SUBGRAPH_PADDING: f64 = 8.0;

/// Maps the parsed ER diagram to the layout data, as `getData` does (`erDb.ts` 377-459).
///
/// The configuration is read as the reference reads it: `look` from the top level, which the
/// ER scope resolves to `neo` by default (`config.ts` 79-103).
#[must_use]
pub fn get_data(diagram: &ErDiagram, config: &MermaidConfig) -> LayoutData {
    let classes: HashMap<&str, &ErClass> = diagram
        .classes
        .iter()
        .map(|class| (class.id.as_str(), class))
        .collect();

    // `parentDB` (erDb.ts 380-395): the subgraph that lists each member. The walk runs backwards
    // and each write overwrites, so a member gets the subgraph that closes first.
    let mut parent: HashMap<&str, &str> = HashMap::new();
    for subgraph in diagram.subgraphs.iter().rev() {
        for member in &subgraph.nodes {
            parent.insert(member.as_str(), subgraph.id.as_str());
        }
    }
    let subgraph_ids: HashSet<&str> = diagram
        .subgraphs
        .iter()
        .map(|subgraph| subgraph.id.as_str())
        .collect();

    let mut nodes = Vec::with_capacity(diagram.subgraphs.len() + diagram.entities.len());
    for subgraph in diagram.subgraphs.iter().rev() {
        nodes.push(Node {
            id: subgraph.id.clone(),
            dom_id: subgraph.id.clone(),
            label: subgraph.title.clone(),
            label_type: LabelType::Markdown,
            label_style: String::new(),
            parent_id: parent.get(subgraph.id.as_str()).map(|id| (*id).to_owned()),
            padding: SUBGRAPH_PADDING,
            css_styles: subgraph.styles.clone(),
            css_compiled_styles: compiled_styles(
                &classes,
                subgraph.classes.iter().map(String::as_str),
            ),
            css_classes: subgraph.classes.join(" "),
            dir: subgraph.dir.clone(),
            look: config.look.clone(),
            is_group: true,
            shape: "rect".to_owned(),
            ..Node::default()
        });
    }

    // The entities, in insertion order. An entity whose name is a subgraph id is skipped
    // (erDb.ts 417-422), and the colour index counts the entities that are kept.
    let mut color_index = 0;
    for entity in &diagram.entities {
        if subgraph_ids.contains(entity.name.as_str()) {
            continue;
        }
        // `cssClasses` starts as `default` and gains a space and the name of each class
        // (`addEntity` 61-72 and `setClass` 303-316), so `getCompiledStyles` sees `default` first.
        let mut css_classes = String::from("default");
        for class in &entity.classes {
            css_classes.push(' ');
            css_classes.push_str(class);
        }
        nodes.push(Node {
            id: entity.id.clone(),
            dom_id: entity.id.clone(),
            label: entity.name.clone(),
            label_type: LabelType::Markdown,
            label_style: String::new(),
            parent_id: parent.get(entity.name.as_str()).map(|id| (*id).to_owned()),
            padding: 0.0,
            css_styles: entity.styles.clone(),
            css_compiled_styles: compiled_styles(&classes, css_classes.split(' ')),
            css_classes,
            dir: None,
            look: config.look.clone(),
            is_group: false,
            shape: "erBox".to_owned(),
            color_index: Some(color_index),
            entity: Some(EntityFields {
                alias: entity.alias.clone(),
                attributes: entity.attributes.clone(),
            }),
            ..Node::default()
        });
        color_index += 1;
    }

    // The relationships (erDb.ts 436-457). The id counts the relationships, not the pairs of ends.
    let edges = diagram
        .relationships
        .iter()
        .enumerate()
        .map(|(counter, relationship)| Edge {
            id: format!(
                "id_{}_{}_{counter}",
                relationship.entity_a, relationship.entity_b
            ),
            start: relationship.entity_a.clone(),
            end: relationship.entity_b.clone(),
            edge_type: "normal".to_owned(),
            label: relationship.role_a.clone(),
            label_type: LabelType::Markdown,
            labelpos: "c".to_owned(),
            thickness: "normal".to_owned(),
            pattern: match relationship.rel_spec.rel_type {
                Identification::Identifying => "solid",
                Identification::NonIdentifying => "dashed",
            }
            .to_owned(),
            classes: "relationshipLine".to_owned(),
            arrow_type_start: cardinality_name(relationship.rel_spec.card_b),
            arrow_type_end: cardinality_name(relationship.rel_spec.card_a),
            look: config.look.clone(),
            curve: Some("basis".to_owned()),
            ..Edge::default()
        })
        .collect();

    LayoutData {
        nodes,
        edges,
        direction: diagram.direction.clone(),
        default_style: None,
        default_interpolate: None,
        diagram_type: String::new(),
    }
}

/// `relSpec.cardA.toLowerCase()` of the reference: the cardinality's name in the DB's spelling,
/// lower-cased (`zero_or_one`, `only_one`, `md_parent`, ...).
fn cardinality_name(cardinality: Cardinality) -> String {
    match cardinality {
        Cardinality::ZeroOrOne => "zero_or_one",
        Cardinality::ZeroOrMore => "zero_or_more",
        Cardinality::OneOrMore => "one_or_more",
        Cardinality::OnlyOne => "only_one",
        Cardinality::MdParent => "md_parent",
    }
    .to_owned()
}

/// `getCompiledStyles` (`erDb.ts` 168-180): for each class name that is defined, its styles and
/// then its text styles, each trimmed.
fn compiled_styles<'n>(
    classes: &HashMap<&str, &ErClass>,
    names: impl IntoIterator<Item = &'n str>,
) -> Vec<String> {
    let mut compiled = Vec::new();
    for name in names {
        let Some(class) = classes.get(name) else {
            continue;
        };
        compiled.extend(class.styles.iter().map(|style| js_trim(style).to_owned()));
        compiled.extend(
            class
                .text_styles
                .iter()
                .map(|style| js_trim(style).to_owned()),
        );
    }
    compiled
}

/// `String.prototype.trim`: Rust's whitespace less U+0085, and with the byte order mark, which
/// JavaScript trims and Rust does not.
fn js_trim(text: &str) -> &str {
    text.trim_matches(|c: char| (c.is_whitespace() && c != '\u{85}') || c == '\u{FEFF}')
}
