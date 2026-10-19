//! The layout data of the use case diagram: `getData` (`usecaseDb.ts` 295-543) and the steps of
//! `prepareUsecaseLayoutData` (`usecaseRenderer.ts` 100-142) that change it before the layout, on the
//! unified layout data (`unified::data`). The accessible names (`usecaseRenderer.ts` 37-84) are made
//! from the sanitised labels, before the escaping of `prepare`, as the reference's `draw` makes them.

use std::collections::HashMap;

use ferrigura_model::flowchart::LabelType;
use ferrigura_model::use_case::{
    ActorType, Animation, BoundaryType, ClassDef, Direction, JsonRow, LabelType as UseLabel,
    Relationship, RelationshipType, UseCaseDiagram, UseCaseShape,
};
use ferrigura_syntax::sanitize::sanitize_text;

use super::UseCaseSection;
use crate::context::Globals;
use crate::unified::data::{Edge, LayoutData, Node};
use crate::unified::markdown::markdown_to_lines;

/// The `data-usecase-kind` of a node (`annotateUsecaseElements`, `usecaseRenderer.ts` 152-161).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An actor.
    Actor,
    /// A use case.
    UseCase,
    /// A note.
    Note,
    /// A JSON table.
    Json,
    /// A system boundary.
    Boundary,
}

impl Kind {
    /// The attribute value of the kind.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::UseCase => "usecase",
            Self::Note => "note",
            Self::Json => "json",
            Self::Boundary => "boundary",
        }
    }
}

/// The fields that only a use case node carries, which its shapes and the paint read.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeFields {
    /// The `data-usecase-kind` of the node.
    pub kind: Kind,
    /// The glyph of an actor (`None` for the other nodes).
    pub actor: Option<ActorType>,
    /// Whether the node has the `business` metadata.
    pub business: bool,
    /// The stereotype as the shape draws it: escaped, and set on an actor or on a business use case. The
    /// stereotype of an ellipse or a rectangle is folded into its label instead.
    pub stereotype: Option<String>,
    /// Whether the stereotype was folded into the label (`prepareUsecaseLayoutData` 122-134).
    pub folded_stereotype: bool,
    /// The rows of a JSON table, escaped as the shape draws them.
    pub json_rows: Vec<JsonRow>,
    /// The geometry of a system boundary.
    pub boundary_type: Option<BoundaryType>,
    /// The accessible name of the node (`getUsecaseNodeAccessibleName` and its boundary form).
    pub accessible_name: String,
    /// The accessible label of the node, which the names of its edges use (`accessibleLabels`).
    pub accessible_label: String,
    /// The font size the label is drawn in when the stylesheet sets one after the label is measured: the
    /// actor's and the business use case's labels get their class after `labelHelper` (`usecaseActor.ts`
    /// 190, `usecaseBusiness.ts` 49), so they are sized at the theme's size and drawn at this one.
    pub drawn_font_size: Option<f64>,
}

impl Default for NodeFields {
    fn default() -> Self {
        Self {
            kind: Kind::UseCase,
            actor: None,
            business: false,
            stereotype: None,
            folded_stereotype: false,
            json_rows: Vec::new(),
            boundary_type: None,
            accessible_name: String::new(),
            accessible_label: String::new(),
            drawn_font_size: None,
        }
    }
}

/// The shape of an actor's glyph (`usecaseDb.ts` 213-224).
fn actor_shape(actor_type: ActorType) -> &'static str {
    match actor_type {
        ActorType::Normal => "usecaseActor",
        ActorType::Hollow => "usecaseActorHollow",
        ActorType::Awesome => "usecaseActorAwesome",
        ActorType::Icon => "usecaseActorIcon",
    }
}

/// The name of an actor's variant in its classes and its accessible name.
fn actor_name(actor_type: ActorType) -> &'static str {
    match actor_type {
        ActorType::Normal => "normal",
        ActorType::Hollow => "hollow",
        ActorType::Awesome => "awesome",
        ActorType::Icon => "icon",
    }
}

/// `safeId` of the DOM ids (`usecaseRenderer.ts` 27-35): every run of characters that are not word
/// characters or `-` becomes one `_`, the `_` at the ends go, and an empty id becomes `element`.
#[must_use]
pub fn safe_id(value: &str) -> String {
    let mut out = String::new();
    let mut in_run = false;
    for c in value.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
            in_run = false;
        } else if !in_run {
            out.push('_');
            in_run = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "element".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// The DOM id of a node, before the diagram's id prefixes it (`usecaseNodeDomId`).
#[must_use]
pub fn node_dom_id(model_id: &str) -> String {
    format!("usecase-{}", safe_id(model_id))
}

/// The DOM id of an edge's path, named by the diagram's id and the edge's id (`usecaseDomId`).
#[must_use]
pub fn edge_dom_id(diagram_id: &str, model_id: &str) -> String {
    format!("usecase-{}-{}", safe_id(diagram_id), safe_id(model_id))
}

/// `escapePlainLabel` (`usecaseRenderer.ts` 86-88).
#[must_use]
pub fn escape_plain(label: &str) -> String {
    label
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `escapeMarkdownMarkers` (`usecaseRenderer.ts` 89): a backslash before each Markdown marker.
fn escape_markdown_markers(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for c in label.chars() {
        if matches!(c, '*' | '[' | '\\' | ']' | '_' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `sanitizeText` of a label as `getData` gives it.
fn sanitize(value: &str) -> String {
    sanitize_text(value)
}

/// The label type of a use case label, as the layout data writes it.
fn label_type_of(label_type: UseLabel) -> LabelType {
    match label_type {
        UseLabel::Text => LabelType::Text,
        UseLabel::Markdown => LabelType::Markdown,
    }
}

/// `getAccessibleLabel` (`usecaseRenderer.ts` 37-44): a Markdown label as lines of words, each line's
/// words joined with spaces and the lines with newlines; any other label as it is.
#[must_use]
pub fn accessible_label(label: &str, label_type: UseLabel) -> String {
    if label_type == UseLabel::Text {
        return label.to_owned();
    }
    markdown_to_lines(label)
        .iter()
        .map(|line| {
            line.iter()
                .map(|word| word.content.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `getCompiledStyles` (`usecaseDb.ts` 118-135): the declarations of the `default` class and of the
/// node's classes, one per property. A later write of a property replaces its value and keeps its place.
fn compiled_styles(classes: &[String], class_defs: &[ClassDef]) -> Vec<String> {
    let mut compiled: Vec<(String, String)> = Vec::new();
    let names = std::iter::once("default").chain(classes.iter().map(String::as_str));
    for name in names {
        let Some(definition) = class_defs.iter().find(|definition| definition.id == name) else {
            continue;
        };
        for raw in &definition.styles {
            let style = raw.trim();
            let property = match style.find(':') {
                Some(separator) => style[..separator].trim(),
                None => style,
            };
            if property.is_empty() {
                continue;
            }
            match compiled.iter_mut().find(|(key, _)| key == property) {
                Some(entry) => entry.1 = style.to_owned(),
                None => compiled.push((property.to_owned(), style.to_owned())),
            }
        }
    }
    compiled.into_iter().map(|(_, style)| style).collect()
}

/// `classNames` (`usecaseDb.ts` 291-292): the non-empty names, joined with one space.
fn class_names(names: &[&str]) -> String {
    names
        .iter()
        .filter(|name| !name.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The node every use case node starts from, with its look.
fn base_node(id: &str, look: &str) -> Node {
    Node {
        id: id.to_owned(),
        look: look.to_owned(),
        ..Node::default()
    }
}

/// The accessible name's `, stereotype …` suffix (`usecaseRenderer.ts` 52, 64).
fn stereotype_suffix(stereotype: Option<&str>) -> String {
    stereotype
        .map(|stereotype| format!(", stereotype {stereotype}"))
        .unwrap_or_default()
}

/// The layout data of a use case diagram: the nodes and edges in `getData`'s order, with the
/// labels as `prepareUsecaseLayoutData` leaves them, and the fields the paint reads. `diagram_id` is the
/// id that the DOM ids of the edges are named by.
#[must_use]
pub fn layout_data(
    diagram: &UseCaseDiagram,
    section: &UseCaseSection<'_>,
    globals: &Globals,
    diagram_id: &str,
) -> LayoutData {
    let config = section.usecase;
    let look = globals.look.as_str();
    let class_defs = &diagram.class_defs;

    // `accessibleLabels`: the accessible label of every node, by id, from its sanitised label.
    let mut accessible: HashMap<String, String> = HashMap::new();
    for actor in &diagram.actors {
        accessible.insert(
            actor.id.clone(),
            accessible_label(&sanitize(&actor.label), actor.label_type),
        );
    }
    for use_case in &diagram.use_cases {
        accessible.insert(
            use_case.id.clone(),
            accessible_label(&sanitize(&use_case.label), use_case.label_type),
        );
    }
    for note in &diagram.notes {
        accessible.insert(
            note.id.clone(),
            accessible_label(&sanitize(&note.label), note.label_type),
        );
    }
    for json in &diagram.json_nodes {
        accessible.insert(json.id.clone(), sanitize(&json.id));
    }
    // `endpointLabel`: the sanitised label of an endpoint, the fallback of an edge's accessible name.
    let endpoint_label = |id: &str| -> String {
        diagram
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .map(|actor| sanitize(&actor.label))
            .or_else(|| {
                diagram
                    .use_cases
                    .iter()
                    .find(|use_case| use_case.id == id)
                    .map(|use_case| sanitize(&use_case.label))
            })
            .or_else(|| {
                diagram
                    .json_nodes
                    .iter()
                    .find(|json| json.id == id)
                    .map(|json| sanitize(&json.id))
            })
            .or_else(|| {
                diagram
                    .notes
                    .iter()
                    .find(|note| note.id == id)
                    .map(|note| sanitize(&note.label))
            })
            .unwrap_or_else(|| sanitize(id))
    };

    let mut nodes = Vec::new();
    let mut color_index: u32 = 0;
    let mut boundary_color_index: u32 = 0;

    for actor in &diagram.actors {
        let raw = sanitize(&actor.label);
        let variant = actor.actor_type;
        let plain = actor.label_type == UseLabel::Text;
        let mut classes = vec![
            "default".to_owned(),
            "usecase-actor".to_owned(),
            format!("usecase-actor-{}", actor_name(variant)),
        ];
        if actor.business {
            classes.push("usecase-business".to_owned());
        }
        classes.extend(actor.classes.iter().cloned());
        let stereotype = actor
            .stereotype
            .as_deref()
            .map(|value| escape_plain(&sanitize(value)));
        let accessible_label = accessible_label(&raw, actor.label_type);
        let variant_word = if variant == ActorType::Normal {
            String::new()
        } else {
            format!("{} ", actor_name(variant))
        };
        let accessible_name = format!(
            "{}{variant_word}actor {accessible_label}{}",
            if actor.business { "business " } else { "" },
            stereotype_suffix(actor.stereotype.as_deref().map(sanitize).as_deref()),
        );
        let mut node = base_node(&actor.id, look);
        node.dom_id = node_dom_id(&actor.id);
        node.label = if plain {
            escape_plain(&raw)
        } else {
            raw.clone()
        };
        node.label_type = label_type_of(actor.label_type);
        node.shape = actor_shape(variant).to_owned();
        node.padding = 10.0;
        let class_refs: Vec<&str> = classes.iter().map(String::as_str).collect();
        node.css_classes = class_names(&class_refs);
        node.css_styles = actor.styles.clone();
        node.css_compiled_styles = compiled_styles(&actor.classes, class_defs);
        node.parent_id = actor.parent_id.clone();
        // The icon of an icon actor is not the node's: `labelHelper` gets the node without it
        // (`usecaseActor.ts` 165-172), and the glyph is the fallback icon, so the node takes none.
        node.color_index = Some(color_index);
        node.wrapping_width = Some(config.wrapping_width);
        node.use_case = Some(NodeFields {
            kind: Kind::Actor,
            actor: Some(variant),
            business: actor.business,
            stereotype,
            folded_stereotype: false,
            json_rows: Vec::new(),
            boundary_type: None,
            accessible_name,
            accessible_label,
            drawn_font_size: Some(config.actor_font_size),
        });
        color_index += 1;
        nodes.push(node);
    }

    for use_case in &diagram.use_cases {
        let raw = sanitize(&use_case.label);
        let plain = use_case.label_type == UseLabel::Text;
        let ellipse = use_case.shape == UseCaseShape::Ellipse;
        let shape = match (ellipse, use_case.business) {
            (true, true) => "usecaseBusiness",
            (true, false) => "usecaseEllipse",
            (false, _) => "rect",
        };
        let shape_name = if ellipse { "ellipse" } else { "rect" };
        let mut classes = vec![
            "default".to_owned(),
            "usecase-element".to_owned(),
            format!("usecase-{shape_name}"),
        ];
        if use_case.business {
            classes.push("usecase-business".to_owned());
        }
        classes.extend(use_case.classes.iter().cloned());
        let stereotype = use_case
            .stereotype
            .as_deref()
            .map(|value| escape_plain(&sanitize(value)));
        let accessible_label = accessible_label(&raw, use_case.label_type);
        let accessible_name = format!(
            "{}use case {accessible_label}{}",
            if use_case.business { "business " } else { "" },
            stereotype_suffix(use_case.stereotype.as_deref().map(sanitize).as_deref()),
        );
        let mut node = base_node(&use_case.id, look);
        node.dom_id = node_dom_id(&use_case.id);
        node.label = if plain {
            escape_plain(&raw)
        } else {
            raw.clone()
        };
        node.label_type = label_type_of(use_case.label_type);
        node.shape = shape.to_owned();
        node.padding = if ellipse { 20.0 } else { 10.0 };
        let class_refs: Vec<&str> = classes.iter().map(String::as_str).collect();
        node.css_classes = class_names(&class_refs);
        node.css_styles = use_case.styles.clone();
        node.css_compiled_styles = compiled_styles(&use_case.classes, class_defs);
        node.parent_id = use_case.parent_id.clone();
        node.color_index = Some(color_index);
        node.wrapping_width = Some(config.wrapping_width);
        node.min_width = Some(config.min_node_width);
        node.spread_ports = shape == "usecaseEllipse" || shape == "usecaseBusiness";
        let mut fields = NodeFields {
            business: use_case.business,
            accessible_name,
            accessible_label,
            drawn_font_size: (shape == "usecaseBusiness").then_some(config.usecase_font_size),
            ..NodeFields::default()
        };
        // `prepareUsecaseLayoutData` 122-134: the stereotype of an ellipse or a rectangle is folded into the
        // label, as a Markdown line before it. A business ellipse keeps its stereotype as its own label.
        match stereotype {
            Some(stereotype) if shape == "usecaseEllipse" || shape == "rect" => {
                let body = if plain {
                    escape_markdown_markers(&node.label)
                } else {
                    node.label.clone()
                };
                node.label = format!("«{}»<br/>{body}", escape_markdown_markers(&stereotype));
                node.label_type = LabelType::Markdown;
                fields.folded_stereotype = true;
            }
            other => fields.stereotype = other,
        }
        node.use_case = Some(fields);
        color_index += 1;
        nodes.push(node);
    }

    for note in &diagram.notes {
        let raw = sanitize(&note.label);
        let plain = note.label_type == UseLabel::Text;
        let target_label = accessible
            .get(&note.target)
            .cloned()
            .unwrap_or_else(|| note.target.clone());
        let mut node = base_node(&note.id, look);
        node.dom_id = node_dom_id(&note.id);
        node.label = if plain {
            escape_plain(&raw)
        } else {
            raw.clone()
        };
        node.label_type = label_type_of(note.label_type);
        node.shape = "note".to_owned();
        node.padding = 10.0;
        node.css_classes = "default usecase-note".to_owned();
        node.wrapping_width = Some(config.wrapping_width);
        node.min_width = Some(config.min_node_width);
        let accessible_label = accessible_label(&raw, note.label_type);
        node.use_case = Some(NodeFields {
            kind: Kind::Note,
            actor: None,
            business: false,
            stereotype: None,
            folded_stereotype: false,
            json_rows: Vec::new(),
            boundary_type: None,
            accessible_name: format!("Note for {target_label}: {accessible_label}"),
            accessible_label,
            drawn_font_size: None,
        });
        nodes.push(node);
    }

    for json in &diagram.json_nodes {
        let raw = sanitize(&json.id);
        let rows: Vec<JsonRow> = json
            .rows
            .iter()
            .map(|row| JsonRow {
                key: escape_plain(&row.key),
                accessible_key: row.accessible_key.clone(),
                value: escape_plain(&row.value),
            })
            .collect();
        // `getUsecaseNodeAccessibleName`: the rows as `key: value` pairs, sanitised and unescaped.
        let row_text = json
            .rows
            .iter()
            .map(|row| format!("{}: {}", row.accessible_key, row.value))
            .collect::<Vec<_>>()
            .join("; ");
        let accessible_name = if row_text.is_empty() {
            raw.clone()
        } else {
            format!("{raw}: {row_text}")
        };
        let mut node = base_node(&json.id, look);
        node.dom_id = node_dom_id(&json.id);
        node.label = escape_plain(&raw);
        node.label_type = LabelType::Text;
        node.shape = "usecaseJsonTable".to_owned();
        node.padding = 10.0;
        let mut classes = vec!["default".to_owned(), "usecase-json-table".to_owned()];
        classes.extend(json.classes.iter().cloned());
        let class_refs: Vec<&str> = classes.iter().map(String::as_str).collect();
        node.css_classes = class_names(&class_refs);
        node.css_styles = json.styles.clone();
        node.css_compiled_styles = compiled_styles(&json.classes, class_defs);
        node.wrapping_width = Some(config.wrapping_width);
        node.min_width = Some(config.min_node_width);
        node.use_case = Some(NodeFields {
            kind: Kind::Json,
            actor: None,
            business: false,
            stereotype: None,
            folded_stereotype: false,
            json_rows: rows,
            boundary_type: None,
            accessible_name,
            accessible_label: raw,
            drawn_font_size: None,
        });
        nodes.push(node);
    }

    for boundary in &diagram.system_boundaries {
        let raw = sanitize(&boundary.label);
        let plain = boundary.label_type == UseLabel::Text;
        let (boundary_name, boundary_kind) = match boundary.boundary_type {
            BoundaryType::Rect => ("rect", BoundaryType::Rect),
            BoundaryType::Package => ("package", BoundaryType::Package),
        };
        let accessible_label = accessible_label(&raw, boundary.label_type);
        let mut node = base_node(&boundary.id, look);
        node.dom_id = node_dom_id(&boundary.id);
        node.label = if plain {
            escape_plain(&raw)
        } else {
            raw.clone()
        };
        node.label_type = label_type_of(boundary.label_type);
        node.shape = "usecaseSystemBoundary".to_owned();
        node.is_group = true;
        node.padding = 20.0;
        let mut classes = vec![
            "default".to_owned(),
            "system-boundary".to_owned(),
            format!("system-boundary-{boundary_name}"),
        ];
        classes.extend(boundary.classes.iter().cloned());
        let class_refs: Vec<&str> = classes.iter().map(String::as_str).collect();
        node.css_classes = class_names(&class_refs);
        node.css_styles = boundary.styles.clone();
        node.css_compiled_styles = compiled_styles(&boundary.classes, class_defs);
        node.wrapping_width = Some(config.wrapping_width);
        node.color_index = Some(boundary_color_index);
        boundary_color_index += 1;
        node.use_case = Some(NodeFields {
            kind: Kind::Boundary,
            actor: None,
            business: false,
            stereotype: None,
            folded_stereotype: false,
            json_rows: Vec::new(),
            boundary_type: Some(boundary_kind),
            accessible_name: format!("{boundary_name} system boundary {accessible_label}"),
            accessible_label,
            drawn_font_size: None,
        });
        nodes.push(node);
    }

    let mut edges: Vec<Edge> = Vec::new();
    for relationship in &diagram.relationships {
        let edge = relationship_edge(
            relationship,
            look,
            class_defs,
            diagram_id,
            &accessible,
            &endpoint_label,
        );
        edges.push(edge);
    }
    for note in &diagram.notes {
        let edge = note_edge(note, look, diagram_id);
        edges.push(edge);
    }

    let direction = match diagram.direction {
        Direction::Tb => "TB",
        Direction::Bt => "BT",
        Direction::Rl => "RL",
        Direction::Lr => "LR",
    };
    LayoutData {
        nodes,
        edges,
        direction: direction.to_owned(),
        ..LayoutData::default()
    }
}

/// The marker arrow types of an association's arrow type (`usecaseDb.ts` 233-252), as `(start, end)`.
fn association_markers(arrow_type: u8) -> (&'static str, &'static str) {
    match arrow_type {
        0 => ("none", "arrow_point"),
        1 => ("arrow_point", "none"),
        3 => ("none", "arrow_circle"),
        4 => ("none", "arrow_cross"),
        5 => ("arrow_circle", "none"),
        6 => ("arrow_cross", "none"),
        // 2 is the plain line; the model holds no other value.
        _ => ("none", "none"),
    }
}

/// The edge of a relationship (`usecaseDb.ts` 449-481, `relationshipVisuals` 254-284), with the
/// attributes that `usecaseRenderer.ts` 194-209 gives its path.
fn relationship_edge(
    relationship: &Relationship,
    look: &str,
    class_defs: &[ClassDef],
    diagram_id: &str,
    accessible: &HashMap<String, String>,
    endpoint_label: &impl Fn(&str) -> String,
) -> Edge {
    let (arrow_start, arrow_end, pattern, label, label_type) = match relationship.relationship_type
    {
        RelationshipType::Include | RelationshipType::Extend => {
            let text = match relationship.relationship_type {
                RelationshipType::Include => "include",
                _ => "extend",
            };
            (
                "none",
                "arrow_point",
                "dotted",
                Some(text.to_owned()),
                Some(LabelType::Text),
            )
        }
        RelationshipType::Generalization => ("none", "extension", "solid", None, None),
        RelationshipType::Association => {
            let (start, end) = association_markers(relationship.arrow_type);
            let label = relationship.label.as_ref().map(|label| sanitize(label));
            let label_type = relationship.label_type.map(label_type_of);
            (start, end, "solid", label, label_type)
        }
    };
    // The kind of the relationship, as the reference names it in its classes and accessible names.
    let kind_name = match relationship.relationship_type {
        RelationshipType::Association => "association",
        RelationshipType::Include => "include",
        RelationshipType::Extend => "extend",
        RelationshipType::Generalization => "generalization",
    };
    let mut classes = vec![
        "default".to_owned(),
        "relationship".to_owned(),
        format!("relationship-{kind_name}"),
    ];
    classes.extend(relationship.classes.iter().cloned());
    let class_refs: Vec<&str> = classes.iter().map(String::as_str).collect();
    let mut edge = Edge {
        id: relationship.id.clone(),
        is_user_defined_id: relationship.explicit_id,
        start: relationship.source.clone(),
        end: relationship.target.clone(),
        edge_type: "edge".to_owned(),
        labelpos: "c".to_owned(),
        thickness: "normal".to_owned(),
        pattern: pattern.to_owned(),
        minlen: Some(relationship.minlen),
        classes: class_names(&class_refs),
        arrow_type_start: arrow_start.to_owned(),
        arrow_type_end: arrow_end.to_owned(),
        css_compiled_styles: compiled_styles(&relationship.classes, class_defs),
        label_style: Vec::new(),
        style: relationship.styles.clone(),
        look: look.to_owned(),
        // The class of an animated edge is `edge-animation-<animation>`, `fast` when only `animate` is set
        // (`usecaseDb.ts` 286-289); the paint takes that name from `animation` alone.
        animate: None,
        animation: relationship
            .animation
            .map(animation_name)
            .or_else(|| relationship.animate.then(|| "fast".to_owned())),
        ..Edge::default()
    };
    if let Some(label) = label {
        edge.label_type = label_type.unwrap_or(LabelType::Text);
        edge.label = match edge.label_type {
            LabelType::Text => escape_plain(&label),
            _ => label.clone(),
        };
        edge.font_size = Some(EDGE_LABEL_FONT_SIZE);
    }
    let source_label = accessible
        .get(&relationship.source)
        .cloned()
        .unwrap_or_else(|| endpoint_label(&relationship.source));
    let target_label = accessible
        .get(&relationship.target)
        .cloned()
        .unwrap_or_else(|| endpoint_label(&relationship.target));
    let relation = match (relationship.relationship_type, relationship.label.as_ref()) {
        (RelationshipType::Association, Some(label)) if !label.is_empty() => {
            format!(
                "association {}",
                accessible_label(
                    &sanitize(label),
                    relationship.label_type.unwrap_or(UseLabel::Text)
                )
            )
        }
        _ => kind_name.to_owned(),
    };
    let dom = edge_dom_id(diagram_id, &relationship.id);
    edge.path_id = Some(dom);
    edge.path_attributes = format!(
        " data-usecase-id=\"{}\" data-usecase-kind=\"relationship\" role=\"img\" aria-label=\"{}\"",
        escape_attribute(&relationship.id),
        escape_attribute(&format!("{relation} from {source_label} to {target_label}")),
    );
    edge
}

/// The edge of a note to its target (`usecaseDb.ts` 483-508): dotted and without markers, internal, so
/// that its path is hidden from assistive technology.
fn note_edge(note: &ferrigura_model::use_case::Note, look: &str, diagram_id: &str) -> Edge {
    let id = format!("{}-edge", note.id);
    Edge {
        path_id: Some(edge_dom_id(diagram_id, &id)),
        path_attributes: format!(
            " data-usecase-id=\"{}\" data-usecase-kind=\"note-connector\" aria-hidden=\"true\"",
            escape_attribute(&id)
        ),
        id,
        is_user_defined_id: false,
        start: note.id.clone(),
        end: note.target.clone(),
        edge_type: "edge".to_owned(),
        labelpos: "c".to_owned(),
        thickness: "normal".to_owned(),
        pattern: "dotted".to_owned(),
        minlen: Some(1),
        classes: "default relationship relationship-note".to_owned(),
        arrow_type_start: "none".to_owned(),
        arrow_type_end: "none".to_owned(),
        look: look.to_owned(),
        animate: Some(false),
        ..Edge::default()
    }
}

/// The font size of an edge label in the use case stylesheet (`.edgeLabel`, `styles.ts` 361-383).
const EDGE_LABEL_FONT_SIZE: f64 = 10.0;

/// The name of an animation as the class takes it.
fn animation_name(animation: Animation) -> String {
    match animation {
        Animation::Fast => "fast".to_owned(),
        Animation::Slow => "slow".to_owned(),
    }
}

/// An attribute value, escaped for XML.
fn escape_attribute(value: &str) -> String {
    crate::svg::escape(value)
}
