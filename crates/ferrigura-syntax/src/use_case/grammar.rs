//! The grammar of the use case diagram, as `usecase.parser.ts` states it: recursive descent with
//! the reference's choices. Where Chevrotain picks an alternative by its lookahead or a GATE, the
//! same test is made here, in the same order: the `statement` alternatives, the OPTION guards of
//! `relationTail` and `entityName`, the GATEs of the arrows and of the metadata assignment, and
//! `LA(0)` for the exact image of `<--` and `--`. The syntax tree keeps the tokens, so the visitor
//! takes the spans the reference takes.

use std::ops::Range;

use super::scanner::{Tok, Token};

/// A failed parse: the byte offset of the token where it failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fail {
    pub(crate) offset: usize,
}

/// The whole diagram: the header and the lines after it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Syntax {
    pub(crate) header: Token,
    pub(crate) lines: Vec<Line>,
}

/// One line. `range` is the token range of the line, the line break included.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    pub(crate) range: Range<usize>,
    pub(crate) kind: LineKind,
}

/// The kinds of line. The boundary's content has its own kinds, since the reference treats an
/// entity or an actor there as a declaration.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LineKind {
    Blank(Token),
    Comment(Token),
    Statement(Box<Statement>),
    /// An entity line inside a system boundary (`boundaryElement`).
    BoundaryElement(EntityName),
    /// An `actor` line inside a system boundary (`actorDeclarationOnly`).
    BoundaryActors(Vec<ActorItem>),
}

/// A statement of the diagram.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Statement {
    AccTitle(Token),
    AccDescr(Token),
    Direction(Token),
    Actor {
        items: Vec<ActorItem>,
        relation: Option<Relation>,
    },
    Boundary(Boundary),
    Note {
        target: Token,
        label: NodeLabel,
    },
    Json {
        start: Token,
        literal: Token,
        classes: Option<ClassSuffix>,
    },
    ClassDef {
        ids: Vec<Token>,
        styles: Vec<StyleValue>,
    },
    Class {
        ids: Vec<Token>,
    },
    Style {
        target: Token,
        styles: Vec<StyleValue>,
    },
    MetadataAssign {
        target: Token,
        metadata: Metadata,
    },
    Entity {
        name: EntityName,
        relation: Option<Relation>,
    },
}

/// A label: a quoted string (one token), or the label tokens of an unquoted label, in order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodeLabel {
    pub(crate) tokens: Vec<Token>,
}

/// An actor, in an `actor` statement or a boundary's `actor` line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActorItem {
    pub(crate) name: ActorName,
    pub(crate) metadata: Option<Metadata>,
    pub(crate) stereotype: Option<Stereotype>,
    pub(crate) classes: Option<ClassSuffix>,
    pub(crate) range: Range<usize>,
}

/// The name of an actor: an identifier with an optional `(label)`, or a quoted string.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ActorName {
    Identifier {
        token: Token,
        label: Option<NodeLabel>,
    },
    Quoted(Token),
}

/// An entity: an identifier with an optional `(label)` or `[label]`, or a quoted string, and its
/// suffixes. The `bool` of the label is true for the bracket form.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EntityName {
    Identifier {
        token: Token,
        label: Option<(NodeLabel, bool)>,
        suffixes: Suffixes,
        range: Range<usize>,
    },
    Quoted {
        token: Token,
        suffixes: Suffixes,
        range: Range<usize>,
    },
}

/// The suffixes of an entity: metadata, a stereotype and classes, in this order.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Suffixes {
    pub(crate) metadata: Option<Metadata>,
    pub(crate) stereotype: Option<Stereotype>,
    pub(crate) classes: Option<ClassSuffix>,
}

/// A system boundary: its name, its suffixes, its content and its `end`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Boundary {
    pub(crate) name: BoundaryName,
    pub(crate) metadata: Option<Metadata>,
    pub(crate) classes: Option<ClassSuffix>,
    pub(crate) content: Vec<Line>,
    pub(crate) end: Token,
    pub(crate) range: Range<usize>,
}

/// The name of a boundary: an identifier with an optional label, or a quoted string.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BoundaryName {
    Identifier {
        token: Token,
        label: Option<NodeLabel>,
    },
    Quoted(Token),
}

/// A relation tail: an optional `id@`, the arrow and the target.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Relation {
    pub(crate) explicit_id: Option<Token>,
    pub(crate) arrow: Arrow,
    pub(crate) target: EntityName,
    pub(crate) range: Range<usize>,
}

/// An arrow of a relation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Arrow {
    pub(crate) kind: ArrowKind,
    pub(crate) range: Range<usize>,
}

/// The arrow forms of the grammar; the labelled ones carry their label and closing operator.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ArrowKind {
    /// `..>:include` or `..>:extend`: the dependency token and the `include` or `extend` token.
    Dependency {
        arrow: Token,
        keyword: Token,
    },
    Generalization(Token),
    ForwardSolid(Token),
    BackwardSolid(Token, Option<Labelled>),
    Markerless(Token, Option<Labelled>),
    ForwardCircle(Token),
    BackwardCircle(Token, Option<Labelled>),
    ForwardCross(Token),
    BackwardCross(Token, Option<Labelled>),
}

/// A labelled arrow: its label and the operator that closes it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Labelled {
    pub(crate) label: NodeLabel,
    pub(crate) closer: Token,
}

/// A metadata block `@{ … }`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Metadata {
    pub(crate) properties: Vec<Property>,
    pub(crate) range: Range<usize>,
}

/// One `key: value` property: the key token and the value token.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Property {
    pub(crate) key: Token,
    pub(crate) value: Token,
}

/// A stereotype: the text between `<<` and `>>`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Stereotype {
    pub(crate) text: Token,
    pub(crate) range: Range<usize>,
}

/// Classes after `:::`: the identifiers.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClassSuffix {
    pub(crate) ids: Vec<Token>,
    pub(crate) range: Range<usize>,
}

/// One value of a style list: its first and last tokens (the reference slices the source).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StyleValue {
    pub(crate) first: Token,
    pub(crate) last: Token,
}

/// Parses the tokens of a diagram.
///
/// # Errors
///
/// Returns the position of the first token the grammar does not accept.
pub(crate) fn parse(text: &str, tokens: &[Token]) -> Result<Syntax, Fail> {
    let mut parser = Parser {
        text,
        tokens,
        pos: 0,
    };
    parser.start()
}

/// The parser: the tokens and the position of the next one.
struct Parser<'a> {
    text: &'a str,
    tokens: &'a [Token],
    pos: usize,
}

/// The tokens that begin an arrow (`arrow` in the grammar).
fn is_arrow_start(kind: Tok) -> bool {
    matches!(
        kind,
        Tok::DependencyArrow
            | Tok::Generalization
            | Tok::ForwardSolid
            | Tok::BackwardSolid
            | Tok::MarkerlessSolid
            | Tok::ForwardCircle
            | Tok::BackwardCircle
            | Tok::ForwardCross
            | Tok::BackwardCross
    )
}

/// `isLabelToken`: a label-text token, or a quoted string.
fn is_label_token(kind: Tok) -> bool {
    kind.is_label_text() || matches!(kind, Tok::PlainString | Tok::MarkdownString)
}

/// The tokens of a style component (`styleComponent`).
fn is_style_component(kind: Tok) -> bool {
    kind.is_word()
        || matches!(
            kind,
            Tok::PlainString
                | Tok::NumberLiteral
                | Tok::HashColor
                | Tok::CssIdentifier
                | Tok::CssEscapedComma
                | Tok::Dash
                | Tok::Dot
                | Tok::Percent
                | Tok::CssPunctuation
                | Tok::Colon
                | Tok::LeftParen
                | Tok::RightParen
                | Tok::LeftBracket
                | Tok::RightBracket
                | Tok::LeftBrace
                | Tok::RightBrace
                | Tok::At
                | Tok::MarkerlessSolid
        )
}

/// `isForbiddenPlantUmlStatement`: the words that cannot start an entity statement.
fn is_forbidden_plantuml(image: &str) -> bool {
    matches!(
        image.to_ascii_lowercase().as_str(),
        "allowmixing" | "newpage" | "package" | "rectangle" | "skinparam"
    )
}

impl Parser<'_> {
    /// `LA(k)` for `k >= 1`: the kind of the k-th token from the next one, `Eof` past the end.
    fn la(&self, k: usize) -> Tok {
        self.tokens
            .get(self.pos + k - 1)
            .map_or(Tok::Eof, |token| token.kind)
    }

    /// The token `LA(k)`, if there is one.
    fn token(&self, k: usize) -> Option<Token> {
        self.tokens.get(self.pos + k - 1).copied()
    }

    /// The fail position: the start of the next token, or the end of the text.
    fn fail(&self) -> Fail {
        let offset = self
            .tokens
            .get(self.pos)
            .map_or(self.text.len(), |token| token.start);
        Fail { offset }
    }

    /// Consumes the next token, which must not be the end of the input.
    fn bump(&mut self) -> Result<Token, Fail> {
        match self.token(1) {
            Some(token) if token.kind != Tok::Eof => {
                self.pos += 1;
                Ok(token)
            }
            _ => Err(self.fail()),
        }
    }

    /// Consumes the next token, which must be of `kind`.
    fn eat(&mut self, kind: Tok) -> Result<Token, Fail> {
        if self.la(1) == kind {
            self.bump()
        } else {
            Err(self.fail())
        }
    }

    /// Consumes the next token if it is of `kind`.
    fn take(&mut self, kind: Tok) -> Option<Token> {
        (self.la(1) == kind).then(|| self.bump().ok()).flatten()
    }

    /// The image of a token of the text.
    fn image(&self, token: Token) -> &str {
        token.image(self.text)
    }

    /// `hasLabeledRight(allowed)`: the next token is a label, and the first token after the label
    /// tokens that is not a label is one of `allowed`.
    fn has_labeled_right(&self, allowed: &[Tok]) -> bool {
        if !is_label_token(self.la(1)) {
            return false;
        }
        let mut k = 2;
        loop {
            let kind = self.la(k);
            if allowed.contains(&kind) {
                return true;
            }
            if !is_label_token(kind) {
                return false;
            }
            k += 1;
        }
    }

    /// The token range from `from` to the next token.
    fn range_from(&self, from: usize) -> Range<usize> {
        from..self.pos
    }

    fn start(&mut self) -> Result<Syntax, Fail> {
        let header = self.eat(Tok::Usecase)?;
        self.line_end()?;
        let mut lines = Vec::new();
        while self.la(1) != Tok::Eof {
            lines.push(self.line()?);
        }
        Ok(Syntax { header, lines })
    }

    /// `lineEnd`: a line break, or the end of the input (which is not consumed).
    fn line_end(&mut self) -> Result<(), Fail> {
        match self.la(1) {
            Tok::NewLine => {
                self.bump()?;
                Ok(())
            }
            Tok::Eof => Ok(()),
            _ => Err(self.fail()),
        }
    }

    /// `line`: a blank line, a comment line, or a statement.
    fn line(&mut self) -> Result<Line, Fail> {
        let from = self.pos;
        let kind = match self.la(1) {
            Tok::NewLine => LineKind::Blank(self.bump()?),
            Tok::Comment => {
                let comment = self.bump()?;
                self.line_end()?;
                LineKind::Comment(comment)
            }
            _ => LineKind::Statement(Box::new(self.statement()?)),
        };
        Ok(Line {
            range: self.range_from(from),
            kind,
        })
    }

    /// `statement`: the alternatives in the reference's order, with its GATEs.
    fn statement(&mut self) -> Result<Statement, Fail> {
        match self.la(1) {
            Tok::AccTitleLine => {
                let token = self.bump()?;
                self.line_end()?;
                Ok(Statement::AccTitle(token))
            }
            Tok::AccDescrLine | Tok::AccDescrBlock => {
                let token = self.bump()?;
                self.line_end()?;
                Ok(Statement::AccDescr(token))
            }
            Tok::Direction => self.direction_statement(),
            Tok::Actor => self.actor_statement(),
            Tok::SystemBoundary => self.boundary_statement().map(Statement::Boundary),
            Tok::Note => self.note_statement(),
            Tok::JsonDeclarationStart => self.json_statement(),
            Tok::ClassDef => self.class_def_statement(),
            Tok::Class => self.class_statement(),
            Tok::Style => self.style_statement(),
            Tok::Identifier | Tok::PlainString | Tok::MarkdownString
                if self.la(2) == Tok::MetadataStart =>
            {
                self.metadata_assignment()
            }
            Tok::Identifier | Tok::PlainString | Tok::MarkdownString => {
                // `isForbiddenPlantUmlStatement` disables the entity alternative for these words.
                let forbidden = self.la(1) == Tok::Identifier
                    && self
                        .token(1)
                        .is_some_and(|token| is_forbidden_plantuml(self.image(token)));
                if forbidden {
                    return Err(self.fail());
                }
                self.entity_statement()
            }
            _ => Err(self.fail()),
        }
    }

    fn direction_statement(&mut self) -> Result<Statement, Fail> {
        self.eat(Tok::Direction)?;
        let token = match self.la(1) {
            Tok::Td | Tok::Tb | Tok::Bt | Tok::Lr | Tok::Rl => self.bump()?,
            _ => return Err(self.fail()),
        };
        self.line_end()?;
        Ok(Statement::Direction(token))
    }

    /// `actorStatement`: `actor`, an actor, then either more actors or a relation, then the line end.
    fn actor_statement(&mut self) -> Result<Statement, Fail> {
        self.eat(Tok::Actor)?;
        let mut items = vec![self.actor_item()?];
        let mut relation = None;
        if self.la(1) == Tok::Comma {
            while self.take(Tok::Comma).is_some() {
                items.push(self.actor_item()?);
            }
        } else if self.relation_starts() {
            relation = Some(self.relation_tail()?);
        }
        self.line_end()?;
        Ok(Statement::Actor { items, relation })
    }

    /// The first tokens of an OPTION that holds a relation tail: `id@` or an arrow.
    fn relation_starts(&self) -> bool {
        (self.la(1) == Tok::Identifier && self.la(2) == Tok::At) || is_arrow_start(self.la(1))
    }

    /// `actorItem`: the name, then the optional metadata, stereotype and classes.
    fn actor_item(&mut self) -> Result<ActorItem, Fail> {
        let from = self.pos;
        let name = self.actor_name()?;
        let metadata = (self.la(1) == Tok::MetadataStart)
            .then(|| self.metadata())
            .transpose()?;
        let stereotype = (self.la(1) == Tok::StereotypeStart)
            .then(|| self.stereotype())
            .transpose()?;
        let classes = (self.la(1) == Tok::ClassSeparator)
            .then(|| self.class_suffix())
            .transpose()?;
        Ok(ActorItem {
            name,
            metadata,
            stereotype,
            classes,
            range: self.range_from(from),
        })
    }

    /// `actorName`: an identifier with an optional `(label)`, or a quoted string.
    fn actor_name(&mut self) -> Result<ActorName, Fail> {
        match self.la(1) {
            Tok::Identifier => {
                let token = self.bump()?;
                let label = if self.la(1) == Tok::LeftParen {
                    self.bump()?;
                    let label = self.node_label()?;
                    self.eat(Tok::RightParen)?;
                    Some(label)
                } else {
                    None
                };
                Ok(ActorName::Identifier { token, label })
            }
            Tok::PlainString | Tok::MarkdownString => Ok(ActorName::Quoted(self.bump()?)),
            _ => Err(self.fail()),
        }
    }

    /// `actorDeclarationOnly`: `actor`, actors separated by commas, the line end (in a boundary).
    fn actor_declaration_only(&mut self) -> Result<Line, Fail> {
        let from = self.pos;
        self.eat(Tok::Actor)?;
        let mut items = vec![self.actor_item()?];
        while self.take(Tok::Comma).is_some() {
            items.push(self.actor_item()?);
        }
        self.line_end()?;
        Ok(Line {
            range: self.range_from(from),
            kind: LineKind::BoundaryActors(items),
        })
    }

    /// `entityStatement`: an entity, an optional relation tail, the line end.
    fn entity_statement(&mut self) -> Result<Statement, Fail> {
        let name = self.entity_name()?;
        let relation = self
            .relation_starts()
            .then(|| self.relation_tail())
            .transpose()?;
        self.line_end()?;
        Ok(Statement::Entity { name, relation })
    }

    /// `entityName`: an identifier with an optional `(label)` or `[label]`, or a quoted string,
    /// then the optional metadata, stereotype and classes.
    fn entity_name(&mut self) -> Result<EntityName, Fail> {
        let from = self.pos;
        match self.la(1) {
            Tok::Identifier => {
                let token = self.bump()?;
                let label = match self.la(1) {
                    Tok::LeftParen => {
                        self.bump()?;
                        let label = self.node_label()?;
                        self.eat(Tok::RightParen)?;
                        Some((label, false))
                    }
                    Tok::LeftBracket => {
                        self.bump()?;
                        let label = self.node_label()?;
                        self.eat(Tok::RightBracket)?;
                        Some((label, true))
                    }
                    _ => None,
                };
                let suffixes = self.suffixes()?;
                Ok(EntityName::Identifier {
                    token,
                    label,
                    suffixes,
                    range: self.range_from(from),
                })
            }
            Tok::PlainString | Tok::MarkdownString => {
                let token = self.bump()?;
                let suffixes = self.suffixes()?;
                Ok(EntityName::Quoted {
                    token,
                    suffixes,
                    range: self.range_from(from),
                })
            }
            _ => Err(self.fail()),
        }
    }

    /// The OPTION2 to OPTION4 of `entityName`: metadata, stereotype, classes.
    fn suffixes(&mut self) -> Result<Suffixes, Fail> {
        let metadata = (self.la(1) == Tok::MetadataStart)
            .then(|| self.metadata())
            .transpose()?;
        let stereotype = (self.la(1) == Tok::StereotypeStart)
            .then(|| self.stereotype())
            .transpose()?;
        let classes = (self.la(1) == Tok::ClassSeparator)
            .then(|| self.class_suffix())
            .transpose()?;
        Ok(Suffixes {
            metadata,
            stereotype,
            classes,
        })
    }

    /// `nodeLabel`: one quoted string, or the label-text tokens (at least one).
    fn node_label(&mut self) -> Result<NodeLabel, Fail> {
        match self.la(1) {
            Tok::PlainString | Tok::MarkdownString => Ok(NodeLabel {
                tokens: vec![self.bump()?],
            }),
            kind if kind.is_label_text() => {
                let mut tokens = Vec::new();
                while self.la(1).is_label_text() {
                    tokens.push(self.bump()?);
                }
                Ok(NodeLabel { tokens })
            }
            _ => Err(self.fail()),
        }
    }

    /// `relationTail`: the optional explicit id (`id@`), the arrow, the target.
    fn relation_tail(&mut self) -> Result<Relation, Fail> {
        let from = self.pos;
        let explicit_id = if self.la(1) == Tok::Identifier && self.la(2) == Tok::At {
            let id = self.bump()?;
            self.eat(Tok::At)?;
            Some(id)
        } else {
            None
        };
        let arrow = self.arrow()?;
        let target = self.entity_name()?;
        Ok(Relation {
            explicit_id,
            arrow,
            target,
            range: self.range_from(from),
        })
    }

    /// `arrow` and its operators. The labelled forms have their GATEs (`LA(0)` for the exact image of
    /// `<--` and `--`, `hasLabeledRight` for the closing operator).
    fn arrow(&mut self) -> Result<Arrow, Fail> {
        let from = self.pos;
        let kind = match self.la(1) {
            Tok::DependencyArrow => {
                let arrow = self.bump()?;
                self.eat(Tok::Colon)?;
                let keyword = match self.la(1) {
                    Tok::Include | Tok::Extend => self.bump()?,
                    _ => return Err(self.fail()),
                };
                ArrowKind::Dependency { arrow, keyword }
            }
            Tok::Generalization => ArrowKind::Generalization(self.bump()?),
            Tok::ForwardSolid => ArrowKind::ForwardSolid(self.bump()?),
            Tok::BackwardSolid => {
                let token = self.bump()?;
                let labelled = (self.image(token) == "<--"
                    && self.has_labeled_right(&[Tok::MarkerlessSolid]))
                .then(|| self.labelled(&[Tok::MarkerlessSolid]))
                .transpose()?;
                ArrowKind::BackwardSolid(token, labelled)
            }
            Tok::MarkerlessSolid => {
                let token = self.bump()?;
                let closers = [
                    Tok::ForwardSolid,
                    Tok::MarkerlessSolid,
                    Tok::ForwardCircle,
                    Tok::ForwardCross,
                ];
                let labelled = (self.image(token) == "--" && self.has_labeled_right(&closers))
                    .then(|| self.labelled(&closers))
                    .transpose()?;
                ArrowKind::Markerless(token, labelled)
            }
            Tok::ForwardCircle => ArrowKind::ForwardCircle(self.bump()?),
            Tok::BackwardCircle => {
                let token = self.bump()?;
                let labelled = self
                    .has_labeled_right(&[Tok::MarkerlessSolid])
                    .then(|| self.labelled(&[Tok::MarkerlessSolid]))
                    .transpose()?;
                ArrowKind::BackwardCircle(token, labelled)
            }
            Tok::ForwardCross => ArrowKind::ForwardCross(self.bump()?),
            Tok::BackwardCross => {
                let token = self.bump()?;
                let labelled = self
                    .has_labeled_right(&[Tok::MarkerlessSolid])
                    .then(|| self.labelled(&[Tok::MarkerlessSolid]))
                    .transpose()?;
                ArrowKind::BackwardCross(token, labelled)
            }
            _ => return Err(self.fail()),
        };
        Ok(Arrow {
            kind,
            range: self.range_from(from),
        })
    }

    /// The label of a labelled arrow and the operator that closes it, one of `closers`.
    fn labelled(&mut self, closers: &[Tok]) -> Result<Labelled, Fail> {
        let label = self.node_label()?;
        let closer = match self.la(1) {
            kind if closers.contains(&kind) => self.bump()?,
            _ => return Err(self.fail()),
        };
        Ok(Labelled { label, closer })
    }

    /// `metadata`: `@{`, the newlines, the properties with their separators, an optional comma and
    /// newlines, `}`.
    fn metadata(&mut self) -> Result<Metadata, Fail> {
        let from = self.pos;
        self.eat(Tok::MetadataStart)?;
        while self.take(Tok::NewLine).is_some() {}
        let mut properties = Vec::new();
        if matches!(self.la(1), Tok::Identifier | Tok::PlainString) {
            properties.push(self.property()?);
            while self.separator_then_property_ahead() {
                self.metadata_separator();
                properties.push(self.property()?);
            }
            self.take(Tok::Comma);
            while self.take(Tok::NewLine).is_some() {}
        }
        self.eat(Tok::RightBrace)?;
        Ok(Metadata {
            properties,
            range: self.range_from(from),
        })
    }

    /// Whether a separator (a comma, or line breaks with an optional comma) is followed by a
    /// property. The reference's lookahead decides the same way.
    fn separator_then_property_ahead(&self) -> bool {
        let mut k = 1;
        match self.la(k) {
            Tok::Comma => {
                k += 1;
                while self.la(k) == Tok::NewLine {
                    k += 1;
                }
            }
            Tok::NewLine => {
                while self.la(k) == Tok::NewLine {
                    k += 1;
                }
                if self.la(k) == Tok::Comma {
                    k += 1;
                }
                while self.la(k) == Tok::NewLine {
                    k += 1;
                }
            }
            _ => return false,
        }
        matches!(self.la(k), Tok::Identifier | Tok::PlainString)
    }

    /// `metadataSeparator`: a comma with line breaks, or line breaks with an optional comma.
    fn metadata_separator(&mut self) {
        if self.take(Tok::Comma).is_some() {
            while self.take(Tok::NewLine).is_some() {}
        } else {
            while self.take(Tok::NewLine).is_some() {}
            self.take(Tok::Comma);
            while self.take(Tok::NewLine).is_some() {}
        }
    }

    /// `metadataProperty`: a key, a colon, and an identifier, a quoted string, `true` or `false`.
    fn property(&mut self) -> Result<Property, Fail> {
        let key = match self.la(1) {
            Tok::Identifier | Tok::PlainString => self.bump()?,
            _ => return Err(self.fail()),
        };
        self.eat(Tok::Colon)?;
        let value = match self.la(1) {
            Tok::Identifier | Tok::PlainString | Tok::True | Tok::False => self.bump()?,
            _ => return Err(self.fail()),
        };
        Ok(Property { key, value })
    }

    /// `stereotype`: `<<`, the text, `>>`.
    fn stereotype(&mut self) -> Result<Stereotype, Fail> {
        let from = self.pos;
        self.eat(Tok::StereotypeStart)?;
        let text = self.eat(Tok::StereotypeText)?;
        self.eat(Tok::StereotypeEnd)?;
        Ok(Stereotype {
            text,
            range: self.range_from(from),
        })
    }

    /// `classSuffix`: `:::` and the identifiers, separated by commas.
    fn class_suffix(&mut self) -> Result<ClassSuffix, Fail> {
        let from = self.pos;
        self.eat(Tok::ClassSeparator)?;
        let mut ids = vec![self.eat(Tok::Identifier)?];
        while self.la(1) == Tok::Comma && self.la(2) == Tok::Identifier {
            self.bump()?;
            ids.push(self.bump()?);
        }
        Ok(ClassSuffix {
            ids,
            range: self.range_from(from),
        })
    }

    /// `jsonStatement`: the declaration, the object, an optional class suffix, the line end.
    fn json_statement(&mut self) -> Result<Statement, Fail> {
        let start = self.eat(Tok::JsonDeclarationStart)?;
        let literal = self.eat(Tok::JsonObjectLiteral)?;
        let classes = (self.la(1) == Tok::ClassSeparator)
            .then(|| self.class_suffix())
            .transpose()?;
        self.line_end()?;
        Ok(Statement::Json {
            start,
            literal,
            classes,
        })
    }

    /// `classDefStatement`: `classDef`, identifiers, the styles, the line end.
    fn class_def_statement(&mut self) -> Result<Statement, Fail> {
        self.eat(Tok::ClassDef)?;
        let mut ids = vec![self.eat(Tok::Identifier)?];
        while self.take(Tok::Comma).is_some() {
            ids.push(self.eat(Tok::Identifier)?);
        }
        let styles = self.styles()?;
        self.line_end()?;
        Ok(Statement::ClassDef { ids, styles })
    }

    /// `classStatement`: `class`, identifiers (the targets), identifiers (the classes), the line
    /// end. The two lists are the reference's `MANY` groups; the visitor splits them by commas.
    fn class_statement(&mut self) -> Result<Statement, Fail> {
        self.eat(Tok::Class)?;
        let mut ids = vec![self.eat(Tok::Identifier)?];
        while self.take(Tok::Comma).is_some() {
            ids.push(self.eat(Tok::Identifier)?);
        }
        ids.push(self.eat(Tok::Identifier)?);
        while self.take(Tok::Comma).is_some() {
            ids.push(self.eat(Tok::Identifier)?);
        }
        self.line_end()?;
        Ok(Statement::Class { ids })
    }

    /// `styleStatement`: `style`, an identifier, the styles, the line end.
    fn style_statement(&mut self) -> Result<Statement, Fail> {
        self.eat(Tok::Style)?;
        let target = self.eat(Tok::Identifier)?;
        let styles = self.styles()?;
        self.line_end()?;
        Ok(Statement::Style { target, styles })
    }

    /// `styles`: style values separated by commas.
    fn styles(&mut self) -> Result<Vec<StyleValue>, Fail> {
        let mut values = vec![self.style_value()?];
        while self.take(Tok::Comma).is_some() {
            values.push(self.style_value()?);
        }
        Ok(values)
    }

    /// `styleValue`: a word, a CSS identifier or `--` followed by a word; a colon; components.
    fn style_value(&mut self) -> Result<StyleValue, Fail> {
        let first = match self.la(1) {
            Tok::MarkerlessSolid => {
                let first = self.bump()?;
                if !self.la(1).is_word() {
                    return Err(self.fail());
                }
                self.bump()?;
                first
            }
            kind if kind.is_word() || kind == Tok::CssIdentifier => self.bump()?,
            _ => return Err(self.fail()),
        };
        self.eat(Tok::Colon)?;
        if !is_style_component(self.la(1)) {
            return Err(self.fail());
        }
        let mut last = self.bump()?;
        while is_style_component(self.la(1)) {
            last = self.bump()?;
        }
        Ok(StyleValue { first, last })
    }

    /// `noteStatement`: `note`, `for`, an identifier, the label, the line end.
    fn note_statement(&mut self) -> Result<Statement, Fail> {
        self.eat(Tok::Note)?;
        self.eat(Tok::For)?;
        let target = self.eat(Tok::Identifier)?;
        let label = self.node_label()?;
        self.line_end()?;
        Ok(Statement::Note { target, label })
    }

    /// `metadataAssignmentStatement`: the target, its metadata, the line end.
    fn metadata_assignment(&mut self) -> Result<Statement, Fail> {
        let target = self.bump()?;
        let metadata = self.metadata()?;
        self.line_end()?;
        Ok(Statement::MetadataAssign { target, metadata })
    }

    /// `systemBoundaryStatement`: `systemBoundary`, its name, optional metadata and classes, the
    /// line end, the content, `end`, the line end.
    fn boundary_statement(&mut self) -> Result<Boundary, Fail> {
        let from = self.pos;
        self.eat(Tok::SystemBoundary)?;
        let name = self.boundary_name()?;
        let metadata = (self.la(1) == Tok::MetadataStart)
            .then(|| self.metadata())
            .transpose()?;
        let classes = (self.la(1) == Tok::ClassSeparator)
            .then(|| self.class_suffix())
            .transpose()?;
        self.line_end()?;
        let content = self.boundary_content()?;
        let end = self.eat(Tok::End)?;
        self.line_end()?;
        Ok(Boundary {
            name,
            metadata,
            classes,
            content,
            end,
            range: self.range_from(from),
        })
    }

    /// `systemBoundaryName`: an identifier with an optional `(label)` or `[label]`, or a quoted string.
    fn boundary_name(&mut self) -> Result<BoundaryName, Fail> {
        match self.la(1) {
            Tok::Identifier => {
                let token = self.bump()?;
                let label = match self.la(1) {
                    Tok::LeftParen => {
                        self.bump()?;
                        let label = self.node_label()?;
                        self.eat(Tok::RightParen)?;
                        Some(label)
                    }
                    Tok::LeftBracket => {
                        self.bump()?;
                        let label = self.node_label()?;
                        self.eat(Tok::RightBracket)?;
                        Some(label)
                    }
                    _ => None,
                };
                Ok(BoundaryName::Identifier { token, label })
            }
            Tok::PlainString | Tok::MarkdownString => Ok(BoundaryName::Quoted(self.bump()?)),
            _ => Err(self.fail()),
        }
    }

    /// `systemBoundaryContent`: blank lines, comment lines, actor lines and entity lines, until
    /// anything else (in practice `end`).
    fn boundary_content(&mut self) -> Result<Vec<Line>, Fail> {
        let mut lines = Vec::new();
        loop {
            let line = match self.la(1) {
                Tok::NewLine | Tok::Comment => self.line()?,
                Tok::Actor => self.actor_declaration_only()?,
                Tok::Identifier | Tok::PlainString | Tok::MarkdownString => {
                    let from = self.pos;
                    let name = self.entity_name()?;
                    self.line_end()?;
                    Line {
                        range: self.range_from(from),
                        kind: LineKind::BoundaryElement(name),
                    }
                }
                _ => break,
            };
            lines.push(line);
        }
        Ok(lines)
    }
}
