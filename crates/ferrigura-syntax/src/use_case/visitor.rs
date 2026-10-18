//! The visitor of the use case diagram: the syntax tree of `grammar.rs` becomes the drafts of the
//! model builder and the statements of the graph AST, in the order of `usecase.visitor.ts`. The
//! spans are the reference's: UTF-16 code units, the quotes trimmed where the reference trims
//! them, and a statement's span is that of its tokens without the line break. Anonymous edges and
//! notes are numbered in the order of the statements.

use std::ops::Range;

use ferrigura_model::use_case::{
    EdgeOccurrence, GraphStatement, LabelType, MetadataOccurrence, NodeOccurrence,
    RelationshipType, Span, StatementKind, UseCaseDiagram, UseCaseShape,
};

use super::builder::{BuildError, UsecaseModelBuilder};
use super::draft::{
    Boundary as BoundaryDraft, ClassAssignment, ClassDefDraft, Element, ElementKind, Endpoint,
    JsonDraft, Label, Location, Metadata as MetadataDraft, MetadataAssignment, MetadataProperty,
    MetadataValue, NoteDraft, RelationshipDraft, StyleAssignment,
};
use super::grammar::{
    ActorItem, ActorName, Arrow, ArrowKind, Boundary, BoundaryName, ClassSuffix, EntityName,
    Labelled, Line, LineKind, Metadata, NodeLabel, Property, Relation, Statement, StyleValue,
    Suffixes, Syntax,
};
use super::json::parse_ordered_json_object;
use super::scanner::{Tok, Token};
use crate::text::js_trim;

/// A failure of the visitor or the builder: a byte offset of the prepared text, and the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Failure {
    pub(crate) offset: usize,
    pub(crate) message: String,
}

/// The drafts of the suffixes of an entity: metadata, stereotype, classes and their spans.
type SuffixDrafts = (
    Option<MetadataDraft>,
    Option<(String, Span)>,
    Vec<String>,
    Vec<Span>,
);

/// The arrow type numbers of the reference's `ARROW_TYPE`.
const SOLID_ARROW: u8 = 0;
const BACK_ARROW: u8 = 1;
const LINE_SOLID: u8 = 2;
const CIRCLE_ARROW: u8 = 3;
const CROSS_ARROW: u8 = 4;
const CIRCLE_ARROW_REVERSED: u8 = 5;
const CROSS_ARROW_REVERSED: u8 = 6;

/// An actor or an entity after its visit (`ParsedActorItem` and `ParsedEntity`).
#[derive(Clone)]
struct Item {
    id: String,
    label: Label,
    location: Location,
    generated: bool,
    metadata: Option<MetadataDraft>,
    stereotype: Option<(String, Span)>,
    classes: Vec<String>,
    class_spans: Vec<Span>,
    shape: Option<UseCaseShape>,
    explicit: bool,
    /// The span of the item's syntax (the actor item or the entity name), for the occurrences.
    range: Range<usize>,
}

/// The arrow of a relation after its visit (`ParsedArrow`).
struct ParsedArrow {
    relationship_type: RelationshipType,
    arrow_type: u8,
    label: Option<Label>,
    minlen: u32,
}

/// Visits the syntax tree, calls the builder and builds the statements, then finalizes the model.
///
/// # Errors
///
/// Returns the first failure of a JSON object or of the builder's checks, at its byte offset.
pub(crate) fn build(
    text: &str,
    tokens: &[Token],
    syntax: &Syntax,
) -> Result<UseCaseDiagram, Failure> {
    let mut visitor = Visitor {
        text,
        tokens,
        builder: UsecaseModelBuilder::new(text),
        parent: None,
        anonymous_edge: 0,
        anonymous_note: 0,
    };
    let statements = visitor.lines(&syntax.lines)?;
    let header = visitor.tok_span(syntax.header, 0);
    let Visitor { builder, .. } = visitor;
    builder
        .finalize(statements, header)
        .map_err(|error: BuildError| Failure {
            offset: utf16_to_byte(text, error.offset),
            message: error.message,
        })
}

/// The byte offset of a UTF-16 offset of the text.
fn utf16_to_byte(text: &str, units: usize) -> usize {
    let mut seen = 0;
    for (byte, character) in text.char_indices() {
        if seen >= units {
            return byte;
        }
        seen += character.len_utf16();
    }
    text.len()
}

/// A statement with no nodes, edges or other fields, of the given kind and span.
fn bare(kind: StatementKind, span: Span) -> GraphStatement {
    GraphStatement {
        kind,
        span,
        nodes: None,
        edges: None,
        group: None,
        id_span: None,
        title_span: None,
        end_span: None,
        r#ref: None,
        ref_span: None,
        children: None,
        metadata: None,
        stereotype_span: None,
        class_spans: None,
    }
}

/// The visitor's state: the text and its tokens, the builder, the enclosing boundary, the counters.
struct Visitor<'a> {
    text: &'a str,
    tokens: &'a [Token],
    builder: UsecaseModelBuilder,
    /// The boundary the statements are in: its id and its location (`parentBoundary`).
    parent: Option<(String, Location)>,
    anonymous_edge: usize,
    anonymous_note: usize,
}

impl Visitor<'_> {
    /// The span of a token, trimmed by `trim` code units at both ends (`tokenSpan`).
    fn tok_span(&self, token: Token, trim: usize) -> Span {
        [token.u16_start + trim, token.u16_end.saturating_sub(trim)]
    }

    /// The trim of a token's text: the quotes of a string (`tokenLocation`, `contentSpan`).
    fn trim_of(kind: Tok) -> usize {
        match kind {
            Tok::MarkdownString => 2,
            Tok::PlainString => 1,
            _ => 0,
        }
    }

    /// The span of the tokens of a range: the first and the last token that are not line breaks
    /// (`nodeSpan`).
    fn span_of(&self, range: &Range<usize>) -> Span {
        let mut significant = self.tokens[range.clone()]
            .iter()
            .filter(|token| !matches!(token.kind, Tok::NewLine | Tok::Eof));
        match (significant.clone().next(), significant.next_back()) {
            (Some(first), Some(last)) => [first.u16_start, last.u16_end],
            _ => [0, 0],
        }
    }

    /// The first token of a range that is not a line break, for the position of a location.
    fn range_location(&self, range: &Range<usize>) -> Location {
        let span = self.span_of(range);
        let first = self.tokens[range.clone()]
            .iter()
            .find(|token| !matches!(token.kind, Tok::NewLine | Tok::Eof))
            .map_or(0, |token| token.start);
        let (line, column) = self.position(first);
        Location { span, line, column }
    }

    /// The location of a token, trimmed as the reference trims it (`tokenLocation`).
    fn token_location(&self, token: Token, trim: usize) -> Location {
        let (line, column) = self.position(token.start + trim);
        Location {
            span: self.tok_span(token, trim),
            line,
            column,
        }
    }

    /// The 1-based line and column of a byte offset, for the positions of the errors.
    fn position(&self, byte: usize) -> (usize, usize) {
        let before = &self.text[..byte.min(self.text.len())];
        let line = before.matches('\n').count() + 1;
        let column = match before.rfind('\n') {
            Some(newline) => before[newline + 1..].chars().count(),
            None => before.chars().count(),
        } + 1;
        (line, column)
    }

    /// `generateId`: `label.replace(/\W/g, '_')`, one `_` for each UTF-16 code unit that is not a
    /// word character.
    fn generate_id(label: &str) -> String {
        label
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '_' {
                    character.to_string()
                } else {
                    "_".repeat(character.len_utf16())
                }
            })
            .collect()
    }

    /// The text of a token without its quotes (`decodePlain`).
    fn decode_plain(&self, token: Token) -> String {
        let image = token.image(self.text);
        if token.kind == Tok::PlainString {
            image[1..image.len() - 1].to_owned()
        } else {
            image.to_owned()
        }
    }

    /// `tokenLabel`: the text of a token without its quotes, and its kind.
    fn token_label(&self, token: Token) -> Label {
        let trim = Self::trim_of(token.kind);
        let kind = if token.kind == Tok::MarkdownString {
            LabelType::Markdown
        } else {
            LabelType::Text
        };
        Label {
            text: self.text[token.start + trim..token.end - trim].to_owned(),
            kind,
            span: self.tok_span(token, trim),
        }
    }

    /// `nodeLabel`, which `edgeLabel` shares: a quoted string, or the source text from the first
    /// label token to the last one.
    fn node_label(&self, label: &NodeLabel) -> Label {
        if let [only] = label.tokens.as_slice()
            && matches!(only.kind, Tok::PlainString | Tok::MarkdownString)
        {
            return self.token_label(*only);
        }
        match (label.tokens.first(), label.tokens.last()) {
            (Some(first), Some(last)) => Label {
                text: self.text[first.start..last.end].to_owned(),
                kind: LabelType::Text,
                span: [first.u16_start, last.u16_end],
            },
            _ => Label {
                text: String::new(),
                kind: LabelType::Text,
                span: [0, 0],
            },
        }
    }

    /// `solidMinlen`: the dashes of the operator minus one, and at least 1.
    fn solid_minlen(token: Token, text: &str) -> u32 {
        let dashes = token.image(text).matches('-').count();
        u32::try_from(dashes.saturating_sub(1).max(1)).unwrap_or(1)
    }

    /// The metadata block as the draft (`metadata`, `metadataProperty`).
    fn metadata(&self, metadata: &Metadata) -> MetadataDraft {
        MetadataDraft {
            properties: metadata
                .properties
                .iter()
                .map(|property| self.property(property))
                .collect(),
            location: self.range_location(&metadata.range),
        }
    }

    fn property(&self, property: &Property) -> MetadataProperty {
        let value = match property.value.kind {
            Tok::True => MetadataValue::Bool(true),
            Tok::False => MetadataValue::Bool(false),
            _ => MetadataValue::Text(self.decode_plain(property.value)),
        };
        MetadataProperty {
            key: self.decode_plain(property.key),
            value,
            span: [property.key.u16_start, property.value.u16_end],
            key_span: self.tok_span(property.key, Self::trim_of(property.key.kind)),
            value_span: self.tok_span(property.value, Self::trim_of(property.value.kind)),
            location: self.token_location(property.key, 0),
        }
    }

    /// The metadata occurrences of a statement or a node (`MetadataOccurrence`).
    fn occurrences(metadata: &MetadataDraft) -> Vec<MetadataOccurrence> {
        metadata
            .properties
            .iter()
            .map(|property| MetadataOccurrence {
                key: property.key.clone(),
                span: property.span,
                key_span: property.key_span,
                value_span: property.value_span,
            })
            .collect()
    }

    /// The stereotype's text, trimmed, and its span (`stereotype`).
    fn stereotype(&self, stereotype: &super::grammar::Stereotype) -> (String, Span) {
        (
            js_trim(stereotype.text.image(self.text)).to_owned(),
            self.tok_span(stereotype.text, 0),
        )
    }

    /// The classes and their spans of a class suffix (`classSuffix`).
    fn classes(&self, suffix: Option<&ClassSuffix>) -> (Vec<String>, Vec<Span>) {
        suffix.map_or_else(
            || (Vec::new(), Vec::new()),
            |suffix| {
                (
                    suffix
                        .ids
                        .iter()
                        .map(|token| token.image(self.text).to_owned())
                        .collect(),
                    suffix
                        .ids
                        .iter()
                        .map(|token| self.tok_span(*token, 0))
                        .collect(),
                )
            },
        )
    }

    /// The suffixes of an entity or an actor: metadata, stereotype, classes, as drafts.
    fn suffixes(&self, suffixes: &Suffixes) -> SuffixDrafts {
        let metadata = suffixes
            .metadata
            .as_ref()
            .map(|metadata| self.metadata(metadata));
        let stereotype = suffixes
            .stereotype
            .as_ref()
            .map(|stereotype| self.stereotype(stereotype));
        let (classes, class_spans) = self.classes(suffixes.classes.as_ref());
        (metadata, stereotype, classes, class_spans)
    }

    /// `actorItem`: the name, then the suffixes.
    fn actor_item(&self, item: &ActorItem) -> Item {
        let (id, label, location, generated) = match &item.name {
            ActorName::Identifier { token, label } => (
                token.image(self.text).to_owned(),
                label
                    .as_ref()
                    .map_or_else(|| self.token_label(*token), |label| self.node_label(label)),
                self.token_location(*token, 0),
                false,
            ),
            ActorName::Quoted(token) => {
                let label = self.token_label(*token);
                (
                    Self::generate_id(&label.text),
                    label,
                    self.token_location(*token, Self::trim_of(token.kind)),
                    true,
                )
            }
        };
        let metadata = item
            .metadata
            .as_ref()
            .map(|metadata| self.metadata(metadata));
        let stereotype = item
            .stereotype
            .as_ref()
            .map(|stereotype| self.stereotype(stereotype));
        let (classes, class_spans) = self.classes(item.classes.as_ref());
        Item {
            id,
            label,
            location,
            generated,
            metadata,
            stereotype,
            classes,
            class_spans,
            shape: None,
            explicit: false,
            range: item.range.clone(),
        }
    }

    /// `entityName`: an identifier with an optional label (the bracket form is a rectangle), or a
    /// quoted string; explicit when it has a shape, metadata or a stereotype.
    fn entity_item(&self, name: &EntityName) -> Item {
        match name {
            EntityName::Identifier {
                token,
                label,
                suffixes,
                range,
            } => {
                let label_draft = label.as_ref().map_or_else(
                    || self.token_label(*token),
                    |(label, _)| self.node_label(label),
                );
                let shape = label.as_ref().map(|(_, bracket)| {
                    if *bracket {
                        UseCaseShape::Rect
                    } else {
                        UseCaseShape::Ellipse
                    }
                });
                let (metadata, stereotype, classes, class_spans) = self.suffixes(suffixes);
                let explicit = shape.is_some() || metadata.is_some() || stereotype.is_some();
                Item {
                    id: token.image(self.text).to_owned(),
                    label: label_draft,
                    location: self.token_location(*token, 0),
                    generated: false,
                    metadata,
                    stereotype,
                    classes,
                    class_spans,
                    shape,
                    explicit,
                    range: range.clone(),
                }
            }
            EntityName::Quoted {
                token,
                suffixes,
                range,
            } => {
                let label = self.token_label(*token);
                let (metadata, stereotype, classes, class_spans) = self.suffixes(suffixes);
                let explicit = metadata.is_some() || stereotype.is_some();
                Item {
                    id: Self::generate_id(&label.text),
                    label,
                    location: self.token_location(*token, Self::trim_of(token.kind)),
                    generated: true,
                    metadata,
                    stereotype,
                    classes,
                    class_spans,
                    shape: None,
                    explicit,
                    range: range.clone(),
                }
            }
        }
    }

    /// `actorOccurrence` and `entityOccurrence`: the node of the AST for an item.
    fn occurrence(&self, item: &Item, defines: bool) -> NodeOccurrence {
        NodeOccurrence {
            id: item.id.clone(),
            span: self.span_of(&item.range),
            id_span: item.location.span,
            label_span: Some(item.label.span),
            defines: defines.then_some(true),
            stereotype_span: item.stereotype.as_ref().map(|(_, span)| *span),
            metadata: item.metadata.as_ref().map(Self::occurrences),
            class_spans: (!item.class_spans.is_empty()).then(|| item.class_spans.clone()),
        }
    }

    /// `actorDraft` and `entityDraft`: the element draft of an item, in the current boundary.
    fn element(&self, item: &Item, kind: ElementKind) -> Element {
        Element {
            id: item.id.clone(),
            kind,
            label: item.label.clone(),
            location: item.location.clone(),
            generated: item.generated,
            parent_id: self.parent.as_ref().map(|(id, _)| id.clone()),
            parent_location: self.parent.as_ref().map(|(_, location)| location.clone()),
            shape: item.shape,
            metadata: item.metadata.clone(),
            stereotype: item.stereotype.as_ref().map(|(text, _)| text.clone()),
            stereotype_span: item.stereotype.as_ref().map(|(_, span)| *span),
            classes: item.classes.clone(),
        }
    }

    /// `endpoint`: an end of a relation as the draft keeps it.
    fn endpoint(item: &Item, declaration: bool) -> Endpoint {
        Endpoint {
            id: item.id.clone(),
            label: item.label.clone(),
            location: item.location.clone(),
            generated: item.generated,
            declaration,
            classes_on_reference: !item.classes.is_empty(),
        }
    }

    /// The arrow of a relation (`arrow` and its operators), as `ParsedArrow`.
    fn arrow(&self, arrow: &Arrow) -> ParsedArrow {
        let association = RelationshipType::Association;
        match &arrow.kind {
            ArrowKind::Dependency { keyword, .. } => {
                let (relationship_type, name) = if keyword.kind == Tok::Include {
                    (RelationshipType::Include, "include")
                } else {
                    (RelationshipType::Extend, "extend")
                };
                ParsedArrow {
                    relationship_type,
                    arrow_type: SOLID_ARROW,
                    label: Some(Label {
                        text: name.to_owned(),
                        kind: LabelType::Text,
                        span: self.tok_span(*keyword, 0),
                    }),
                    minlen: 1,
                }
            }
            ArrowKind::Generalization(_) => ParsedArrow {
                relationship_type: RelationshipType::Generalization,
                arrow_type: SOLID_ARROW,
                label: None,
                minlen: 1,
            },
            ArrowKind::ForwardSolid(token) => ParsedArrow {
                relationship_type: association,
                arrow_type: SOLID_ARROW,
                label: None,
                minlen: Self::solid_minlen(*token, self.text),
            },
            ArrowKind::BackwardSolid(token, labelled) => {
                let length_token = labelled.as_ref().map_or(*token, |labelled| labelled.closer);
                ParsedArrow {
                    relationship_type: association,
                    arrow_type: BACK_ARROW,
                    label: labelled
                        .as_ref()
                        .map(|labelled| self.node_label(&labelled.label)),
                    minlen: Self::solid_minlen(length_token, self.text),
                }
            }
            ArrowKind::Markerless(token, labelled) => self.markerless(*token, labelled.as_ref()),
            ArrowKind::ForwardCircle(_) => ParsedArrow {
                relationship_type: association,
                arrow_type: CIRCLE_ARROW,
                label: None,
                minlen: 1,
            },
            ArrowKind::BackwardCircle(_, labelled) => {
                self.reversed(CIRCLE_ARROW_REVERSED, labelled.as_ref())
            }
            ArrowKind::ForwardCross(_) => ParsedArrow {
                relationship_type: association,
                arrow_type: CROSS_ARROW,
                label: None,
                minlen: 1,
            },
            ArrowKind::BackwardCross(_, labelled) => {
                self.reversed(CROSS_ARROW_REVERSED, labelled.as_ref())
            }
        }
    }

    /// A marker-less arrow, with or without a label; a label's closer gives the arrow type.
    fn markerless(&self, token: Token, labelled: Option<&Labelled>) -> ParsedArrow {
        let Some(labelled) = labelled else {
            return ParsedArrow {
                relationship_type: RelationshipType::Association,
                arrow_type: LINE_SOLID,
                label: None,
                minlen: Self::solid_minlen(token, self.text),
            };
        };
        let closer = labelled.closer;
        let arrow_type = match closer.kind {
            Tok::ForwardSolid => SOLID_ARROW,
            Tok::ForwardCircle => CIRCLE_ARROW,
            Tok::ForwardCross => CROSS_ARROW,
            _ => LINE_SOLID,
        };
        let minlen = if arrow_type == SOLID_ARROW || arrow_type == LINE_SOLID {
            Self::solid_minlen(closer, self.text)
        } else {
            1
        };
        ParsedArrow {
            relationship_type: RelationshipType::Association,
            arrow_type,
            label: Some(self.node_label(&labelled.label)),
            minlen,
        }
    }

    /// A reversed circle or cross arrow, with an optional label.
    fn reversed(&self, arrow_type: u8, labelled: Option<&Labelled>) -> ParsedArrow {
        ParsedArrow {
            relationship_type: RelationshipType::Association,
            arrow_type,
            label: labelled.map(|labelled| self.node_label(&labelled.label)),
            minlen: 1,
        }
    }
}

impl Visitor<'_> {
    /// `systemBoundaryContent` and the statements: the lines of a range, in order.
    fn lines(&mut self, lines: &[Line]) -> Result<Vec<GraphStatement>, Failure> {
        let mut statements = Vec::with_capacity(lines.len());
        for line in lines {
            statements.push(self.line(line)?);
        }
        Ok(statements)
    }

    /// One line: a blank and a comment keep their own token spans; a statement takes the span of
    /// its tokens (`wrap`), and so do the edges of the statement.
    fn line(&mut self, line: &Line) -> Result<GraphStatement, Failure> {
        match &line.kind {
            LineKind::Blank(token) => Ok(bare(StatementKind::Blank, self.tok_span(*token, 0))),
            LineKind::Comment(token) => Ok(bare(StatementKind::Comment, self.tok_span(*token, 0))),
            LineKind::Statement(statement) => {
                let span = self.span_of(&line.range);
                let mut built = self.statement(statement, &line.range, span)?;
                built.span = span;
                for edge in built.edges.iter_mut().flatten() {
                    edge.span = span;
                }
                Ok(built)
            }
            LineKind::BoundaryElement(name) => {
                let span = self.span_of(&line.range);
                let mut item = self.entity_item(name);
                item.shape.get_or_insert(UseCaseShape::Ellipse);
                item.explicit = true;
                self.builder
                    .add_element(self.element(&item, ElementKind::UseCase));
                let mut built = bare(StatementKind::Node, span);
                built.nodes = Some(vec![self.occurrence(&item, true)]);
                Ok(built)
            }
            LineKind::BoundaryActors(items) => {
                let span = self.span_of(&line.range);
                let items: Vec<Item> = items.iter().map(|item| self.actor_item(item)).collect();
                for item in &items {
                    self.builder
                        .add_element(self.element(item, ElementKind::Actor));
                }
                let mut built = bare(StatementKind::Node, span);
                built.nodes = Some(
                    items
                        .iter()
                        .map(|item| self.occurrence(item, true))
                        .collect(),
                );
                Ok(built)
            }
        }
    }

    /// The statement of one line (`statement` and the visits of the statement kinds).
    fn statement(
        &mut self,
        statement: &Statement,
        range: &Range<usize>,
        span: Span,
    ) -> Result<GraphStatement, Failure> {
        match statement {
            Statement::AccTitle(token) => {
                let image = token.image(self.text);
                let text = js_trim(&image[image.find(':').map_or(0, |colon| colon + 1)..]);
                self.builder.set_acc_title(text);
                Ok(bare(StatementKind::AccTitle, span))
            }
            Statement::AccDescr(token) => {
                let image = token.image(self.text);
                let text = if token.kind == Tok::AccDescrLine {
                    js_trim(&image[image.find(':').map_or(0, |colon| colon + 1)..])
                } else {
                    let open = image.find('{').map_or(0, |brace| brace + 1);
                    let close = image.rfind('}').unwrap_or(image.len());
                    js_trim(&image[open..close.max(open)])
                };
                self.builder.set_acc_description(text);
                Ok(bare(StatementKind::AccDescr, span))
            }
            Statement::Direction(token) => {
                self.builder
                    .set_direction(token.image(self.text), self.tok_span(*token, 0));
                Ok(bare(StatementKind::Direction, span))
            }
            Statement::Actor { items, relation } => self.actor(items, relation.as_ref()),
            Statement::Boundary(boundary) => self.boundary(boundary, span),
            Statement::Note { target, label } => Ok(self.note(*target, label, range)),
            Statement::Json {
                start,
                literal,
                classes,
            } => self.json(*start, *literal, classes.as_ref()),
            Statement::ClassDef { ids, styles } => Ok(self.class_def(ids, styles)),
            Statement::Class { ids } => self.class_assignment(ids),
            Statement::Style { target, styles } => {
                let values = self.style_values(styles);
                self.builder.add_style_assignment(StyleAssignment {
                    target: target.image(self.text).to_owned(),
                    location: self.token_location(*target, 0),
                    styles: values,
                });
                let mut built = bare(StatementKind::Style, span);
                built.nodes = Some(vec![self.plain_occurrence(*target)]);
                Ok(built)
            }
            Statement::MetadataAssign { target, metadata } => {
                Ok(self.metadata_assignment(*target, metadata, span))
            }
            Statement::Entity { name, relation } => self.entity(name, relation.as_ref(), span),
        }
    }

    /// An occurrence of a plain identifier token: its span is the token's.
    fn plain_occurrence(&self, token: Token) -> NodeOccurrence {
        NodeOccurrence {
            id: token.image(self.text).to_owned(),
            span: self.tok_span(token, 0),
            id_span: self.tok_span(token, 0),
            label_span: None,
            defines: None,
            stereotype_span: None,
            metadata: None,
            class_spans: None,
        }
    }

    /// `actorStatement`: the actors, then the relation when there is one.
    fn actor(
        &mut self,
        items: &[ActorItem],
        relation: Option<&Relation>,
    ) -> Result<GraphStatement, Failure> {
        let parsed: Vec<Item> = items.iter().map(|item| self.actor_item(item)).collect();
        let mut occurrences: Vec<NodeOccurrence> = parsed
            .iter()
            .map(|item| self.occurrence(item, true))
            .collect();
        for item in &parsed {
            self.builder
                .add_element(self.element(item, ElementKind::Actor));
        }
        let mut built = bare(StatementKind::Node, [0, 0]);
        let Some(relation) = relation else {
            built.nodes = Some(occurrences);
            return Ok(built);
        };
        let (explicit_id, arrow, target) = self.relation_parts(relation);
        if target.explicit {
            self.builder
                .add_element(self.element(&target, ElementKind::UseCase));
        }
        let source = &parsed[0];
        let draft = self.relationship_draft(
            Self::endpoint(source, true),
            &target,
            relation,
            explicit_id,
            arrow,
        );
        let id = self.edge_id(draft.explicit_id.as_deref());
        let edge = EdgeOccurrence {
            id: id.clone(),
            span: [0, 0],
            label_span: draft.label.as_ref().map(|label| label.span),
            id_span: draft
                .explicit_id_location
                .as_ref()
                .map(|location| location.span),
            metadata: None,
            class_spans: None,
        };
        self.builder.add_relationship(draft);
        occurrences.push(self.occurrence(&target, target.explicit));
        built.nodes = Some(occurrences);
        built.kind = StatementKind::Edge;
        built.edges = Some(vec![edge]);
        Ok(built)
    }

    /// The id of an edge: the explicit one, or the next `edge-N` of the anonymous edges.
    fn edge_id(&mut self, explicit: Option<&str>) -> String {
        match explicit {
            Some(id) => id.to_owned(),
            None => {
                let id = format!("edge-{}", self.anonymous_edge);
                self.anonymous_edge += 1;
                id
            }
        }
    }

    /// The parts of a relation tail: the explicit id, the arrow and the target item.
    fn relation_parts(
        &self,
        relation: &Relation,
    ) -> (Option<(String, Location)>, ParsedArrow, Item) {
        let explicit_id = relation.explicit_id.map(|token| {
            (
                token.image(self.text).to_owned(),
                self.token_location(token, 0),
            )
        });
        (
            explicit_id,
            self.arrow(&relation.arrow),
            self.entity_item(&relation.target),
        )
    }

    /// The relationship draft of a relation: its ends, its location, its id and arrow.
    fn relationship_draft(
        &self,
        source: Endpoint,
        target: &Item,
        relation: &Relation,
        explicit_id: Option<(String, Location)>,
        arrow: ParsedArrow,
    ) -> RelationshipDraft {
        let (explicit_id, explicit_id_location) = match explicit_id {
            Some((id, location)) => (Some(id), Some(location)),
            None => (None, None),
        };
        RelationshipDraft {
            source,
            target: Self::endpoint(target, target.explicit),
            location: self.range_location(&relation.range),
            explicit_id,
            explicit_id_location,
            relationship_type: arrow.relationship_type,
            arrow_type: arrow.arrow_type,
            label: arrow.label,
            minlen: arrow.minlen,
        }
    }

    /// `systemBoundaryStatement`: the boundary, its content, its metadata.
    fn boundary(&mut self, boundary: &Boundary, span: Span) -> Result<GraphStatement, Failure> {
        let (id, label, location, generated) = match &boundary.name {
            BoundaryName::Identifier { token, label } => (
                token.image(self.text).to_owned(),
                label
                    .as_ref()
                    .map_or_else(|| self.token_label(*token), |label| self.node_label(label)),
                self.token_location(*token, 0),
                false,
            ),
            BoundaryName::Quoted(token) => {
                let label = self.token_label(*token);
                (
                    Self::generate_id(&label.text),
                    label,
                    self.token_location(*token, Self::trim_of(token.kind)),
                    true,
                )
            }
        };
        let (classes, class_spans) = self.classes(boundary.classes.as_ref());
        self.builder.add_boundary(BoundaryDraft {
            id: id.clone(),
            label: label.clone(),
            location: location.clone(),
            generated,
            classes,
        });
        let previous = self.parent.replace((id.clone(), location.clone()));
        let children = self.lines(&boundary.content)?;
        self.parent = previous;
        let metadata = boundary
            .metadata
            .as_ref()
            .map(|metadata| self.metadata(metadata));
        let mut statement = bare(StatementKind::Group, span);
        statement.group = Some(id.clone());
        statement.id_span = Some(location.span);
        statement.title_span = Some(label.span);
        statement.end_span = Some(self.tok_span(boundary.end, 0));
        statement.class_spans = Some(class_spans);
        statement.metadata = metadata.as_ref().map(Self::occurrences);
        statement.children = (!children.is_empty()).then_some(children);
        if let Some(metadata) = metadata {
            self.builder.add_metadata_assignment(MetadataAssignment {
                target: id,
                target_location: location,
                metadata,
                statement: statement.clone(),
            });
        }
        Ok(statement)
    }

    /// `noteStatement`: the note for an identifier, numbered `note-N`.
    fn note(&mut self, target: Token, label: &NodeLabel, range: &Range<usize>) -> GraphStatement {
        let label = self.node_label(label);
        self.builder.add_note(NoteDraft {
            target: target.image(self.text).to_owned(),
            target_location: self.token_location(target, 0),
            label: label.clone(),
            location: self.range_location(range),
        });
        let mut built = bare(StatementKind::Note, [0, 0]);
        built.r#ref = Some(format!("note-{}", self.anonymous_note));
        self.anonymous_note += 1;
        built.ref_span = Some(label.span);
        built.nodes = Some(vec![self.plain_occurrence(target)]);
        built
    }

    /// `jsonStatement`: the table's id (after `json` and the blanks), its object and its classes.
    fn json(
        &mut self,
        start: Token,
        literal: Token,
        classes: Option<&ClassSuffix>,
    ) -> Result<GraphStatement, Failure> {
        let image = start.image(self.text);
        let blanks = image[4..]
            .bytes()
            .take_while(|byte| matches!(byte, b' ' | b'\t'))
            .count();
        let id: String = image[4 + blanks..]
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        let relative = 4 + blanks;
        let id_span = [
            start.u16_start + relative,
            start.u16_start + relative + id.len(),
        ];
        let (line, column) = self.position(start.start + relative);
        let location = Location {
            span: id_span,
            line,
            column,
        };
        let parsed =
            parse_ordered_json_object(literal.image(self.text)).map_err(|error| Failure {
                offset: literal.start + error.offset,
                message: error.message,
            })?;
        let (class_names, class_spans) = self.classes(classes);
        self.builder.add_json(JsonDraft {
            id: id.clone(),
            value: parsed.value,
            property_order: parsed.property_order,
            location,
            classes: class_names,
        });
        let mut built = bare(StatementKind::Json, [0, 0]);
        built.nodes = Some(vec![NodeOccurrence {
            id,
            span: id_span,
            id_span,
            label_span: None,
            defines: Some(true),
            stereotype_span: None,
            metadata: None,
            class_spans: Some(class_spans.clone()),
        }]);
        built.class_spans = Some(class_spans);
        Ok(built)
    }

    /// The values of a style list, as the source slices with `\,` read as `,`.
    fn style_values(&self, styles: &[StyleValue]) -> Vec<String> {
        styles
            .iter()
            .map(|style| self.text[style.first.start..style.last.end].replace("\\,", ","))
            .collect()
    }

    /// `classDefStatement`: the ids and the styles.
    fn class_def(&mut self, ids: &[Token], styles: &[StyleValue]) -> GraphStatement {
        let names: Vec<String> = ids
            .iter()
            .map(|token| token.image(self.text).to_owned())
            .collect();
        let values = self.style_values(styles);
        self.builder.add_class_def(ClassDefDraft {
            ids: names.clone(),
            styles: values,
        });
        let mut built = bare(StatementKind::ClassDef, [0, 0]);
        built.r#ref = names.first().cloned();
        built.ref_span = ids.first().map(|token| self.tok_span(*token, 0));
        built
    }

    /// `classStatement`: the targets are the identifiers before the first one that is not preceded
    /// by a comma; the rest are the classes.
    fn class_assignment(&mut self, ids: &[Token]) -> Result<GraphStatement, Failure> {
        let mut split = 1;
        while split < ids.len() {
            let between = &self.text[ids[split - 1].end..ids[split].start];
            if !between.contains(',') {
                break;
            }
            split += 1;
        }
        let targets: Vec<(String, Location)> = ids[..split]
            .iter()
            .map(|token| {
                (
                    token.image(self.text).to_owned(),
                    self.token_location(*token, 0),
                )
            })
            .collect();
        let classes: Vec<String> = ids[split..]
            .iter()
            .map(|token| token.image(self.text).to_owned())
            .collect();
        let mut built = bare(StatementKind::ClassAssign, [0, 0]);
        built.r#ref = classes.first().cloned();
        built.ref_span = ids.get(split).map(|token| self.tok_span(*token, 0));
        built.nodes = Some(
            targets
                .iter()
                .map(|(id, location)| NodeOccurrence {
                    id: id.clone(),
                    span: location.span,
                    id_span: location.span,
                    label_span: None,
                    defines: None,
                    stereotype_span: None,
                    metadata: None,
                    class_spans: None,
                })
                .collect(),
        );
        self.builder
            .add_class_assignment(ClassAssignment { targets, classes });
        Ok(built)
    }

    /// `metadataAssignmentStatement`: the target (an identifier, or a quoted string whose id is
    /// generated), its metadata.
    fn metadata_assignment(
        &mut self,
        target: Token,
        metadata: &Metadata,
        span: Span,
    ) -> GraphStatement {
        let (id, location) = if target.kind == Tok::Identifier {
            (
                target.image(self.text).to_owned(),
                self.token_location(target, 0),
            )
        } else {
            let label = self.token_label(target);
            (
                Self::generate_id(&label.text),
                self.token_location(target, Self::trim_of(target.kind)),
            )
        };
        let draft = self.metadata(metadata);
        let mut built = bare(StatementKind::Metadata, span);
        built.nodes = Some(vec![NodeOccurrence {
            id: id.clone(),
            span: location.span,
            id_span: location.span,
            label_span: None,
            defines: None,
            stereotype_span: None,
            metadata: None,
            class_spans: None,
        }]);
        built.metadata = Some(Self::occurrences(&draft));
        self.builder.add_metadata_assignment(MetadataAssignment {
            target: id,
            target_location: location,
            metadata: draft,
            statement: built.clone(),
        });
        built
    }

    /// `entityStatement`: an entity alone is a declaration; with a relation, the relation's ends.
    fn entity(
        &mut self,
        name: &EntityName,
        relation: Option<&Relation>,
        span: Span,
    ) -> Result<GraphStatement, Failure> {
        let mut source = self.entity_item(name);
        let Some(relation) = relation else {
            source.explicit = true;
            source.shape.get_or_insert(UseCaseShape::Ellipse);
            self.builder
                .add_element(self.element(&source, ElementKind::UseCase));
            let mut built = bare(StatementKind::Node, span);
            built.nodes = Some(vec![self.occurrence(&source, true)]);
            return Ok(built);
        };
        if source.explicit {
            self.builder
                .add_element(self.element(&source, ElementKind::UseCase));
        }
        let (explicit_id, arrow, target) = self.relation_parts(relation);
        if target.explicit {
            self.builder
                .add_element(self.element(&target, ElementKind::UseCase));
        }
        let draft = self.relationship_draft(
            Self::endpoint(&source, source.explicit),
            &target,
            relation,
            explicit_id,
            arrow,
        );
        let id = self.edge_id(draft.explicit_id.as_deref());
        let edge = EdgeOccurrence {
            id,
            span: [0, 0],
            label_span: draft.label.as_ref().map(|label| label.span),
            id_span: draft
                .explicit_id_location
                .as_ref()
                .map(|location| location.span),
            metadata: None,
            class_spans: None,
        };
        let nodes = vec![
            self.occurrence(&source, source.explicit),
            self.occurrence(&target, target.explicit),
        ];
        self.builder.add_relationship(draft);
        let mut built = bare(StatementKind::Edge, span);
        built.nodes = Some(nodes);
        built.edges = Some(vec![edge]);
        Ok(built)
    }
}
