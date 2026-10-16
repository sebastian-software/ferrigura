//! The class diagram as the layout and the renderer read it: a port of `getData` in
//! `packages/mermaid/src/diagrams/class/classDb.ts` (lines 738-865), which fills the layout data of
//! [`crate::unified::data`]. The differential run (`pnpm port:diff class-data`) compares it with the
//! reference's `getData()`.
//!
//! Notes on the shape of the output, where the reference leaves a field `undefined` and the Rust
//! type has no `Option` for it (the exporter in `tools/port/export-class-data.ts` writes the same
//! defaults on its side):
//! - a class, an interface and a relation have no `padding` (zero here, and left out of the JSON);
//! - a class, a namespace, a note and an interface have no `labelType` but the notes' Markdown; the
//!   Rust default (`text`) stands for the absent one, which no renderer reads for them;
//! - a relation without a title has an empty `label`, and a note edge an empty `labelpos`;
//! - the `style` of a relation is `classRelation.style || ''`, an empty list here.
//!
//! The class id of a DOM element is `classId-<name>` without the counter `classDb.ts` appends
//! (`classCounter`, a module-level counter that the reference never resets); the counter only makes
//! the ids of a page unique, and the names are unique in a diagram already.

use ferrigura_model::class::{
    ClassDiagram, ClassNamespace, ClassNode, ClassNote, ClassRelation, LineType, RelationEnd,
};
use ferrigura_model::config::MermaidConfig;
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::value::Value;

use crate::unified::data::{ClassNodeData, Edge, LayoutData, Node};

/// The padding of a namespace when the configuration gives none (`classDb.ts` 752).
const NAMESPACE_PADDING: f64 = 16.0;

/// The padding of a note when the configuration gives none (`classDb.ts` 787).
const NOTE_PADDING: f64 = 6.0;

/// The class attribute of the namespaces, notes and interfaces. Their `getData` node has no
/// `cssClasses`, so the reference writes `undefined` into the attribute (`'cluster ' + undefined`
/// in `clusters.js`, `getNodeClasses` in `shapes/util.ts`), and the Rust field holds that text.
const UNDEFINED_CLASSES: &str = "undefined";

/// The `classId-` prefix of the DOM ids of classes (`classDb.ts` 32).
const CLASS_DOM_ID_PREFIX: &str = "classId-";

/// Maps the parsed class diagram to the layout data, as `getData` does: the namespaces, the classes,
/// the notes (with their note edges), the interfaces, then the relations.
///
/// `theme` gives the note colours (`noteBkgColor` and `noteBorderColor`), which the note nodes carry
/// as their CSS styles.
#[must_use]
pub fn get_data(diagram: &ClassDiagram, config: &MermaidConfig, theme: &Value) -> LayoutData {
    let hierarchical = config.class.hierarchical_namespaces;
    let look = config.look.clone();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for namespace in &diagram.namespaces {
        if !hierarchical && !namespace.explicit {
            continue;
        }
        nodes.push(namespace_node(namespace, hierarchical, config, &look));
    }

    // Only classes take a colour slot; namespaces and notes do not shift the cycle.
    for (index, class) in diagram.classes.iter().enumerate() {
        let parent = if hierarchical {
            class.parent.clone()
        } else {
            resolve_explicit_ancestor(diagram, class.parent.as_deref())
        };
        nodes.push(class_node(class, parent, &look, index));
    }

    for note in &diagram.notes {
        let parent = if hierarchical {
            note.parent.clone()
        } else {
            resolve_explicit_ancestor(diagram, note.parent.as_deref())
        };
        nodes.push(note_node(note, parent, config, theme, &look));
        // `classes.get(note.class)?.id`: a note on a class that exists gets a dotted edge to it.
        if let Some(class) = note.class.as_deref().and_then(|name| diagram.class(name)) {
            edges.push(note_edge(note, &class.id, &look));
        }
    }

    for interface in &diagram.interfaces {
        nodes.push(Node {
            id: interface.id.clone(),
            dom_id: interface.id.clone(),
            label: interface.label.clone(),
            css_classes: UNDEFINED_CLASSES.to_owned(),
            shape: "rect".to_owned(),
            css_styles: vec!["opacity: 0;".to_owned()],
            look: look.clone(),
            ..Node::default()
        });
    }

    for (index, relation) in diagram.relations.iter().enumerate() {
        edges.push(relation_edge(relation, index + 1, &look));
    }

    LayoutData {
        nodes,
        edges,
        direction: diagram.direction.clone(),
        default_style: None,
        default_interpolate: None,
        diagram_type: String::new(),
    }
}

/// A namespace as a group node: its label (the id in compact mode), its padding and its parent
/// (`classDb.ts` 744-760).
fn namespace_node(
    namespace: &ClassNamespace,
    hierarchical: bool,
    config: &MermaidConfig,
    look: &str,
) -> Node {
    Node {
        id: namespace.id.clone(),
        dom_id: namespace.id.clone(),
        label: if hierarchical {
            namespace.label.clone()
        } else {
            namespace.id.clone()
        },
        padding: config.class.padding.unwrap_or(NAMESPACE_PADDING),
        css_classes: UNDEFINED_CLASSES.to_owned(),
        is_group: true,
        shape: "rect".to_owned(),
        look: look.to_owned(),
        parent_id: if hierarchical {
            namespace.parent.clone()
        } else {
            None
        },
        ..Node::default()
    }
}

/// A class as a `classBox` node, with the spread of the DB's class node (`classDb.ts` 765-778).
/// `index` counts the classes, and is the class's colour slot.
fn class_node(class: &ClassNode, parent: Option<String>, look: &str, index: usize) -> Node {
    Node {
        id: class.id.clone(),
        dom_id: format!("{CLASS_DOM_ID_PREFIX}{}", class.id),
        label: class.label.clone(),
        parent_id: parent,
        css_classes: class.css_classes.clone(),
        look: look.to_owned(),
        link: class.link.clone(),
        link_target: class.link_target.clone(),
        tooltip: class.tooltip.clone(),
        shape: "classBox".to_owned(),
        color_index: Some(u32::try_from(index).unwrap_or(u32::MAX)),
        class: Some(ClassNodeData {
            text: class.text.clone(),
            annotations: class.annotations.clone(),
            members: class.members.clone(),
            methods: class.methods.clone(),
            styles: class.styles.clone(),
            parent: class.parent.clone(),
            have_callback: class.have_callback,
        }),
        ..Node::default()
    }
}

/// A note as a `note` node: its text is the label, unsanitised, and its colours are the theme's
/// note colours (`classDb.ts` 780-798).
fn note_node(
    note: &ClassNote,
    parent: Option<String>,
    config: &MermaidConfig,
    theme: &Value,
    look: &str,
) -> Node {
    let colour = |key: &str| {
        theme
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    Node {
        id: note.id.clone(),
        dom_id: note.id.clone(),
        label: note.text.clone(),
        label_type: LabelType::Markdown,
        padding: config.class.padding.unwrap_or(NOTE_PADDING),
        css_classes: UNDEFINED_CLASSES.to_owned(),
        css_styles: vec![
            "text-align: left".to_owned(),
            "white-space: nowrap".to_owned(),
            format!("fill: {}", colour("noteBkgColor")),
            format!("stroke: {}", colour("noteBorderColor")),
        ],
        look: look.to_owned(),
        parent_id: parent,
        shape: "note".to_owned(),
        ..Node::default()
    }
}

/// The dotted edge from a note to the class it is written on (`classDb.ts` 800-819).
fn note_edge(note: &ClassNote, class_id: &str, look: &str) -> Edge {
    Edge {
        id: format!("edgeNote{}", note.index),
        start: note.id.clone(),
        end: class_id.to_owned(),
        edge_type: "normal".to_owned(),
        thickness: "normal".to_owned(),
        classes: "relation".to_owned(),
        arrow_type_start: "none".to_owned(),
        arrow_type_end: "none".to_owned(),
        arrowhead_style: String::new(),
        label_style: vec![String::new()],
        style: vec!["fill: none".to_owned()],
        pattern: "dotted".to_owned(),
        look: look.to_owned(),
        ..Edge::default()
    }
}

/// A relation as an edge with its two cardinalities and its markers (`classDb.ts` 834-862). `count`
/// is the relation's position from 1, which the edge id takes.
fn relation_edge(relation: &ClassRelation, count: usize, look: &str) -> Edge {
    let cardinality = |title: &str| {
        if title == "none" {
            String::new()
        } else {
            title.to_owned()
        }
    };
    Edge {
        id: format!("id_{}_{}_{count}", relation.id1, relation.id2),
        start: relation.id1.clone(),
        end: relation.id2.clone(),
        edge_type: "normal".to_owned(),
        label: relation.title.clone().unwrap_or_default(),
        label_type: LabelType::Markdown,
        labelpos: "c".to_owned(),
        thickness: "normal".to_owned(),
        classes: "relation".to_owned(),
        arrow_type_start: marker_name(relation.type1).to_owned(),
        arrow_type_end: marker_name(relation.type2).to_owned(),
        start_label_right: Some(cardinality(&relation.relation_title1)),
        end_label_left: Some(cardinality(&relation.relation_title2)),
        arrowhead_style: String::new(),
        label_style: vec!["display: inline-block".to_owned()],
        pattern: match relation.line_type {
            LineType::Dotted => "dashed",
            LineType::Solid => "solid",
        }
        .to_owned(),
        look: look.to_owned(),
        ..Edge::default()
    }
}

/// The marker of a relation end, by the names of `getArrowMarker` (`classDb.ts` 694-716).
fn marker_name(end: RelationEnd) -> &'static str {
    match end {
        RelationEnd::None => "none",
        RelationEnd::Aggregation => "aggregation",
        RelationEnd::Extension => "extension",
        RelationEnd::Composition => "composition",
        RelationEnd::Dependency => "dependency",
        RelationEnd::Lollipop => "lollipop",
    }
}

/// The nearest ancestor of `id` (or `id` itself) that the user declared, as
/// `resolveExplicitAncestor` finds it (`classDb.ts` 723-736). Compact mode uses it.
fn resolve_explicit_ancestor(diagram: &ClassDiagram, id: Option<&str>) -> Option<String> {
    let mut current = id.filter(|id| !id.is_empty());
    while let Some(id) = current {
        let namespace = diagram.namespace(id)?;
        if namespace.explicit {
            return Some(id.to_owned());
        }
        current = namespace
            .parent
            .as_deref()
            .filter(|parent| !parent.is_empty());
    }
    None
}
