//! The flowchart grammar of `flow.jison` as recursive descent over the scanner's tokens. Every
//! production of the grammar has a function here, named after it; the DB calls happen where the
//! grammar's actions run, in the same order (see `docs-port/flowchart/PLAN.md`, section 3.4).
//!
//! The constructs of later steps (styles, classes, `linkStyle`, interactions, shape data) are
//! reported as not yet ported; they are not parsed into the model.

use ferrigura_model::flowchart::{FlowchartDiagram, LabelType};

use super::db::{BodyItem, Db, DbError, FlowText, LinkData, LinkPosition};
use super::scanner::{Kind, Token};
use super::{FlowchartError, Options};
use crate::error::Source;

/// What a statement gives its enclosing document: the grammar's value of the statement.
enum Statement {
    /// The node ids of a vertex statement, in the order the grammar gives them.
    Nodes(Vec<String>),
    /// The id of a subgraph.
    Subgraph(String),
    /// A direction statement.
    Dir(String),
    /// An accessibility value, which the grammar returns as a string.
    Str(String),
}

/// The DB's view of an arrow: its type, stroke and length.
struct LinkInfo {
    edge_type: String,
    stroke: String,
    length: Option<u32>,
}

pub(crate) struct Parser<'a> {
    source: Source<'a>,
    tokens: Vec<Token>,
    pos: usize,
    db: Db,
}

/// Parses the prepared flowchart text: the header, the document, and the DB's model.
pub(crate) fn parse_text(
    text: &str,
    line_offset: usize,
    title: String,
    options: Options,
) -> Result<(FlowchartDiagram, Vec<String>), FlowchartError> {
    let source = Source::new(text, line_offset);
    let tokens = super::scanner::tokenize(text)
        .map_err(|error| FlowchartError::Syntax(source.error(error.offset, "unrecognized text")))?;
    let mut parser = Parser {
        source,
        tokens,
        pos: 0,
        db: Db::new(options.max_edges),
    };
    parser.graph_config()?;
    parser.document(false)?;
    Ok(parser.db.into_model(title))
}

/// The kinds that make up an id: `idStringToken`.
fn is_id_token(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Num
            | Kind::NodeString
            | Kind::Down
            | Kind::Minus
            | Kind::Default
            | Kind::Comma
            | Kind::Colon
            | Kind::Amp
            | Kind::Brkt
            | Kind::Mult
            | Kind::UnicodeText
    )
}

/// The kinds that make up an alphanumeric word: `alphaNumToken`.
fn is_alpha_num_token(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Num
            | Kind::UnicodeText
            | Kind::NodeString
            | Kind::Dir
            | Kind::Down
            | Kind::Minus
            | Kind::Comma
            | Kind::Colon
            | Kind::Amp
            | Kind::Brkt
            | Kind::Mult
    )
}

/// The kinds that make up a label without tags: `textNoTagsToken`.
fn is_text_no_tags_token(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Num
            | Kind::NodeString
            | Kind::Space
            | Kind::Minus
            | Kind::Amp
            | Kind::UnicodeText
            | Kind::Colon
            | Kind::Mult
            | Kind::Brkt
            | Kind::Style
            | Kind::LinkStyle
            | Kind::ClassDef
            | Kind::Class
            | Kind::Click
            | Kind::Graph
            | Kind::Dir
            | Kind::Subgraph
            | Kind::End
            | Kind::Down
            | Kind::Up
            | Kind::StartLink
    )
}

impl Parser<'_> {
    fn kind(&self) -> Kind {
        self.tokens
            .get(self.pos)
            .map_or(Kind::Eof, |token| token.kind)
    }

    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |token| token.start)
    }

    /// Consumes the next token and returns it; the end of the input yields an empty end token.
    fn take(&mut self) -> Token {
        let token = self.tokens.get(self.pos).cloned().unwrap_or(Token {
            kind: Kind::Eof,
            value: String::new(),
            start: self.source.text().len(),
        });
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        token
    }

    fn syntax<T>(&self, message: impl Into<String>) -> Result<T, FlowchartError> {
        Err(FlowchartError::Syntax(
            self.source.error(self.offset(), message),
        ))
    }

    fn expect(&mut self, kind: Kind, what: &str) -> Result<Token, FlowchartError> {
        if self.kind() == kind {
            Ok(self.take())
        } else {
            self.syntax(format!("expected {what}"))
        }
    }

    fn db<T>(&self, result: Result<T, DbError>) -> Result<T, FlowchartError> {
        result.or_else(|DbError(message)| self.syntax(message))
    }

    fn not_yet<T>(&self, construct: &'static str) -> Result<T, FlowchartError> {
        let at = self.source.error(self.offset(), "");
        Err(FlowchartError::NotYetPorted {
            line: at.line,
            column: at.column,
            construct,
        })
    }

    /// `graphConfig`: leading blanks, then `graph` with a direction, or with none. The DB gets the
    /// direction when the header is complete.
    fn graph_config(&mut self) -> Result<(), FlowchartError> {
        while matches!(self.kind(), Kind::Space | Kind::Newline) {
            self.take();
        }
        self.expect(Kind::Graph, "'graph', 'flowchart' or 'swimlane-beta'")?;
        match self.kind() {
            Kind::NoDir => {
                self.take();
                self.db.set_direction("TB");
            }
            Kind::Dir => {
                let dir = self.take();
                self.db.set_direction(&dir.value);
                self.first_stmt_separator()?;
            }
            _ => return self.syntax("expected a direction after 'graph'"),
        }
        Ok(())
    }

    /// `FirstStmtSeparator: SEMI | NEWLINE | spaceList NEWLINE`.
    fn first_stmt_separator(&mut self) -> Result<(), FlowchartError> {
        match self.kind() {
            Kind::Semi | Kind::Newline => {
                self.take();
                Ok(())
            }
            Kind::Space => {
                while self.kind() == Kind::Space {
                    self.take();
                }
                self.expect(Kind::Newline, "the end of the header line")?;
                Ok(())
            }
            _ => self.syntax("expected the end of the header line"),
        }
    }

    /// `document`: lines until the end of the input, or until `end` in a subgraph. The items it
    /// returns are what the subgraph's list holds; at the top level they are not used.
    fn document(&mut self, in_subgraph: bool) -> Result<Vec<BodyItem>, FlowchartError> {
        let mut items = Vec::new();
        loop {
            match self.kind() {
                Kind::Eof => {
                    if in_subgraph {
                        return self.syntax("expected 'end' to close the subgraph");
                    }
                    return Ok(items);
                }
                Kind::End => {
                    if in_subgraph {
                        return Ok(items);
                    }
                    return self.syntax("'end' without a subgraph");
                }
                Kind::Newline | Kind::Space => {
                    self.take();
                }
                Kind::Semi => {
                    self.take();
                    // A lone `;` is a line whose value is the `;` itself.
                    items.push(BodyItem::Str(String::from(";")));
                }
                _ => match self.statement()? {
                    Statement::Nodes(nodes) => {
                        items.extend(nodes.into_iter().map(BodyItem::Str));
                    }
                    Statement::Subgraph(id) => items.push(BodyItem::Str(id)),
                    Statement::Dir(dir) => items.push(BodyItem::Dir(dir)),
                    Statement::Str(text) => items.push(BodyItem::Str(text)),
                },
            }
        }
    }

    /// `statement`: a vertex statement with its separator, a subgraph, a direction, or an
    /// accessibility line.
    fn statement(&mut self) -> Result<Statement, FlowchartError> {
        match self.kind() {
            Kind::Subgraph => self.subgraph(),
            Kind::DirectionTb => self.direction("TB"),
            Kind::DirectionBt => self.direction("BT"),
            Kind::DirectionRl => self.direction("RL"),
            Kind::DirectionLr => self.direction("LR"),
            Kind::DirectionTd => self.direction("TD"),
            Kind::AccTitle => {
                self.take();
                let value = self.expect(Kind::AccTitleValue, "the accessible title")?;
                let text = crate::text::js_trim(&value.value).to_owned();
                self.db.set_acc_title(&text);
                Ok(Statement::Str(text))
            }
            Kind::AccDescr => {
                self.take();
                let value = self.expect(Kind::AccDescrValue, "the accessible description")?;
                let text = crate::text::js_trim(&value.value).to_owned();
                self.db.set_acc_description(&text);
                Ok(Statement::Str(text))
            }
            Kind::AccDescrMultiValue => {
                let value = self.take();
                let text = crate::text::js_trim(&value.value).to_owned();
                self.db.set_acc_description(&text);
                Ok(Statement::Str(text))
            }
            Kind::Style => {
                self.style_statement()?;
                self.separator()?;
                Ok(Statement::Nodes(Vec::new()))
            }
            Kind::LinkStyle => {
                self.link_style_statement()?;
                self.separator()?;
                Ok(Statement::Nodes(Vec::new()))
            }
            Kind::ClassDef => {
                self.class_def_statement()?;
                self.separator()?;
                Ok(Statement::Nodes(Vec::new()))
            }
            Kind::Class => {
                self.class_statement()?;
                self.separator()?;
                Ok(Statement::Nodes(Vec::new()))
            }
            Kind::Click => {
                self.click_statement()?;
                self.separator()?;
                Ok(Statement::Nodes(Vec::new()))
            }
            _ => {
                let nodes = self.vertex_statement()?;
                self.separator()?;
                Ok(Statement::Nodes(nodes))
            }
        }
    }

    /// `direction`: the statement only; it has no separator of its own.
    fn direction(&mut self, dir: &str) -> Result<Statement, FlowchartError> {
        self.take();
        Ok(Statement::Dir(dir.to_owned()))
    }

    /// `separator: NEWLINE | SEMI | EOF`; the end of the input is left for the document.
    fn separator(&mut self) -> Result<(), FlowchartError> {
        match self.kind() {
            Kind::Newline | Kind::Semi => {
                self.take();
                Ok(())
            }
            Kind::Eof => Ok(()),
            _ => self.syntax("expected the end of the statement"),
        }
    }

    /// `subgraph`, in its three forms: with an id and a bracketed title, with a title that is
    /// also the id, and without either, which the reference rejects.
    fn subgraph(&mut self) -> Result<Statement, FlowchartError> {
        self.take();
        let id: String;
        let id_is_title: bool;
        let title: FlowText;
        match self.kind() {
            Kind::Space => {
                self.take();
                let (text, label_type) = self.text_no_tags()?;
                if self.kind() == Kind::Sqs {
                    self.take();
                    let bracketed = self.text()?;
                    self.expect(Kind::Sqe, "']' after the subgraph title")?;
                    self.separator()?;
                    id = text;
                    id_is_title = false;
                    title = bracketed;
                } else {
                    self.separator()?;
                    id = text.clone();
                    id_is_title = true;
                    title = FlowText { text, label_type };
                }
            }
            Kind::Newline | Kind::Semi | Kind::Eof => {
                self.separator()?;
                let body = self.document(true)?;
                self.expect(Kind::End, "'end'")?;
                let subgraph_id = self.db.add_subgraph(
                    None,
                    false,
                    &FlowText {
                        text: String::new(),
                        label_type: LabelType::Text,
                    },
                    &body,
                );
                return self.db(subgraph_id).map(Statement::Subgraph);
            }
            _ => return self.syntax("expected a subgraph id or title"),
        }
        let body = self.document(true)?;
        self.expect(Kind::End, "'end'")?;
        let subgraph_id = self.db.add_subgraph(Some(&id), id_is_title, &title, &body);
        self.db(subgraph_id).map(Statement::Subgraph)
    }

    /// `vertexStatement`: a chain of node groups joined by links. The DB gets each vertex when
    /// its node reduces, and each link after the node that follows it; the returned ids are in the
    /// grammar's order, in which each chain step puts its nodes in front.
    fn vertex_statement(&mut self) -> Result<Vec<String>, FlowchartError> {
        let group = self.node_group()?;
        let mut chunks = vec![group.clone()];
        let mut previous = group;
        loop {
            // `spaceList` may end a vertex statement, so blanks may come before the next link.
            let save = self.pos;
            while self.kind() == Kind::Space {
                self.take();
            }
            if !matches!(self.kind(), Kind::Link | Kind::StartLink | Kind::LinkId) {
                self.pos = save;
                break;
            }
            let link = self.link()?;
            let group = self.node_group()?;
            let added = self.db.add_link(&previous, &group, &link);
            self.db(added)?;
            chunks.push(group.clone());
            previous = group;
        }
        // `spaceList` at the end of the statement.
        while self.kind() == Kind::Space {
            self.take();
        }
        Ok(chunks.into_iter().rev().flatten().collect())
    }

    /// `node`: styled vertices joined by `&`. Shape data after a vertex belongs to the group only
    /// when an `&` follows it; otherwise it is returned as pending for the statement.
    fn node_group(&mut self) -> Result<Vec<String>, FlowchartError> {
        let first = self.styled_vertex()?;
        let mut group = vec![first];
        loop {
            if self.kind() == Kind::ShapeData {
                return self.not_yet("shape data");
            }
            let save = self.pos;
            let mut spaces = 0;
            while self.kind() == Kind::Space {
                self.take();
                spaces += 1;
            }
            if self.kind() == Kind::Amp && spaces > 0 {
                self.take();
                self.expect(Kind::Space, "a space after '&'")?;
                while self.kind() == Kind::Space {
                    self.take();
                }
                let next = self.styled_vertex()?;
                group.push(next);
                continue;
            }
            // Not a continuation: the spaces belong to the statement.
            self.pos = save;
            break;
        }
        Ok(group)
    }

    /// `styledVertex: vertex | vertex STYLE_SEPARATOR idString`.
    fn styled_vertex(&mut self) -> Result<String, FlowchartError> {
        let id = self.vertex()?;
        if self.kind() == Kind::StyleSeparator {
            self.take();
            let class = self.id_string()?;
            self.db.set_class(&id, &class);
        }
        Ok(id)
    }

    /// `clickStatement`: the id the `click` state gave, then a callback, a link, or an alphanumeric
    /// id as a callback, with an optional tooltip and an optional link target. The DB calls keep the
    /// order of the grammar's actions: the link (or the callback's classes) first, then the tooltip.
    fn click_statement(&mut self) -> Result<(), FlowchartError> {
        let ids = self.take().value;
        match self.kind() {
            Kind::CallbackName => {
                self.take();
                if self.kind() == Kind::CallbackArgs {
                    self.take();
                }
                self.db.set_click_event(&ids);
                self.optional_tooltip(&ids);
            }
            Kind::Href | Kind::Str => {
                if self.kind() == Kind::Href {
                    self.take();
                }
                let link = self.expect(Kind::Str, "a link in quotes")?.value;
                let mut tooltip = None;
                let mut target = None;
                if self.kind() == Kind::Space && self.kind_at(1) == Kind::Str {
                    self.take();
                    tooltip = Some(self.take().value);
                }
                if self.kind() == Kind::Space && self.kind_at(1) == Kind::LinkTarget {
                    self.take();
                    target = Some(self.take().value);
                }
                self.db.set_link(&ids, &link, target.as_deref());
                if let Some(tooltip) = tooltip {
                    self.db.set_tooltip(&ids, &tooltip);
                }
            }
            kind if is_alpha_num_token(kind) => {
                self.alpha_num()?;
                self.db.set_click_event(&ids);
                self.optional_tooltip(&ids);
            }
            _ => return self.syntax("expected a callback or a link after 'click'"),
        }
        Ok(())
    }

    /// `SPACE STR` after a callback: the tooltip.
    fn optional_tooltip(&mut self, ids: &str) {
        if self.kind() == Kind::Space && self.kind_at(1) == Kind::Str {
            self.take();
            let tooltip = self.take().value;
            self.db.set_tooltip(ids, &tooltip);
        }
    }

    /// The token at `offset` past the next one; the end of the input yields `Eof`.
    fn kind_at(&self, offset: usize) -> Kind {
        self.tokens
            .get(self.pos + offset)
            .map_or(Kind::Eof, |token| token.kind)
    }

    /// `styleStatement: STYLE SPACE idString SPACE stylesOpt`: the vertex gets the styles, and is
    /// made if it does not exist.
    fn style_statement(&mut self) -> Result<(), FlowchartError> {
        self.take();
        self.expect(Kind::Space, "a space after 'style'")?;
        let id = self.id_string()?;
        self.expect(Kind::Space, "a space after the node id")?;
        let styles = self.styles_opt()?;
        let added = self.db.add_vertex(&id, None, None, Some(&styles), None);
        self.db(added)
    }

    /// `classDefStatement: CLASSDEF SPACE idString SPACE stylesOpt`.
    fn class_def_statement(&mut self) -> Result<(), FlowchartError> {
        self.take();
        self.expect(Kind::Space, "a space after 'classDef'")?;
        let id = self.id_string()?;
        self.expect(Kind::Space, "a space after the class name")?;
        let styles = self.styles_opt()?;
        self.db.add_class(&id, &styles);
        Ok(())
    }

    /// `classStatement: CLASS SPACE idString SPACE idString`.
    fn class_statement(&mut self) -> Result<(), FlowchartError> {
        self.take();
        self.expect(Kind::Space, "a space after 'class'")?;
        let vertices = self.id_string()?;
        self.expect(Kind::Space, "a space after the node ids")?;
        let class = self.id_string()?;
        self.db.set_class(&vertices, &class);
        Ok(())
    }

    /// `linkStyleStatement`: the styles, or the interpolation, or both, of the default or of the
    /// edges at the given indices.
    fn link_style_statement(&mut self) -> Result<(), FlowchartError> {
        self.take();
        self.expect(Kind::Space, "a space after 'linkStyle'")?;
        let positions = if self.kind() == Kind::Default {
            self.take();
            vec![LinkPosition::Default]
        } else {
            self.num_list()?
        };
        self.expect(Kind::Space, "a space after the positions")?;
        if self.kind() == Kind::Interpolate {
            self.take();
            self.expect(Kind::Space, "a space after 'interpolate'")?;
            let interpolate = self.alpha_num()?;
            let added = self.db.update_link_interpolate(&positions, &interpolate);
            self.db(added)?;
            if self.kind() == Kind::Space {
                self.take();
                let styles = self.styles_opt()?;
                let added = self.db.update_link(&positions, &styles);
                self.db(added)?;
            }
            return Ok(());
        }
        let styles = self.styles_opt()?;
        let added = self.db.update_link(&positions, &styles);
        self.db(added)
    }

    /// `numList: NUM (COMMA NUM)*`, as the indices of `linkStyle`.
    fn num_list(&mut self) -> Result<Vec<LinkPosition>, FlowchartError> {
        let mut positions = vec![LinkPosition::Index(
            self.expect(Kind::Num, "a number")?.value,
        )];
        while self.kind() == Kind::Comma {
            self.take();
            positions.push(LinkPosition::Index(
                self.expect(Kind::Num, "a number")?.value,
            ));
        }
        Ok(positions)
    }

    /// `stylesOpt: style (COMMA style)*`.
    fn styles_opt(&mut self) -> Result<Vec<String>, FlowchartError> {
        let mut styles = vec![self.style()?];
        while self.kind() == Kind::Comma {
            self.take();
            styles.push(self.style()?);
        }
        Ok(styles)
    }

    /// `style: styleComponent+`, the components joined as written.
    fn style(&mut self) -> Result<String, FlowchartError> {
        let mut style = String::new();
        while matches!(
            self.kind(),
            Kind::Num | Kind::NodeString | Kind::Colon | Kind::Space | Kind::Brkt | Kind::Style
        ) {
            style.push_str(&self.take().value);
        }
        if style.is_empty() {
            return self.syntax("expected a style");
        }
        Ok(style)
    }

    /// `alphaNum`: one or more of the tokens of an alphanumeric word, joined as written.
    fn alpha_num(&mut self) -> Result<String, FlowchartError> {
        let mut text = String::new();
        while is_alpha_num_token(self.kind()) {
            text.push_str(&self.take().value);
        }
        if text.is_empty() {
            return self.syntax("expected a word");
        }
        Ok(text)
    }

    /// `vertex`: an id with its shape, or a bare id. The DB's `addVertex` is called once the
    /// shape's closing token is read.
    fn vertex(&mut self) -> Result<String, FlowchartError> {
        let id = self.id_string()?;
        let (text, shape) = match self.kind() {
            Kind::Sqs => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::Sqe, "']'")?;
                (Some(text), Some("square"))
            }
            Kind::DoubleCircleStart => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::DoubleCircleEnd, "')))'")?;
                (Some(text), Some("doublecircle"))
            }
            Kind::Ps => {
                self.take();
                if self.kind() == Kind::Ps {
                    self.take();
                    let text = self.text()?;
                    self.expect(Kind::Pe, "')'")?;
                    self.expect(Kind::Pe, "')'")?;
                    (Some(text), Some("circle"))
                } else {
                    let text = self.text()?;
                    self.expect(Kind::Pe, "')'")?;
                    (Some(text), Some("round"))
                }
            }
            Kind::EllipseStart => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::EllipseEnd, "'-)'")?;
                (Some(text), Some("ellipse"))
            }
            Kind::StadiumStart => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::StadiumEnd, "'])'")?;
                (Some(text), Some("stadium"))
            }
            Kind::SubroutineStart => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::SubroutineEnd, "']]'")?;
                (Some(text), Some("subroutine"))
            }
            Kind::VertexWithPropsStart => {
                self.take();
                let field = self.expect(Kind::NodeString, "a property name")?;
                self.expect(Kind::Colon, "':'")?;
                let value = self.expect(Kind::NodeString, "a property value")?;
                self.expect(Kind::Pipe, "'|'")?;
                let text = self.text()?;
                self.expect(Kind::Sqe, "']'")?;
                let props = Some((field.value, value.value));
                let added = self
                    .db
                    .add_vertex(&id, Some(text), Some("rect"), None, props);
                self.db(added)?;
                return Ok(id);
            }
            Kind::CylinderStart => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::CylinderEnd, "')]'")?;
                (Some(text), Some("cylinder"))
            }
            Kind::DiamondStart => {
                self.take();
                if self.kind() == Kind::DiamondStart {
                    self.take();
                    let text = self.text()?;
                    self.expect(Kind::DiamondStop, "'}'")?;
                    self.expect(Kind::DiamondStop, "'}}'")?;
                    (Some(text), Some("hexagon"))
                } else {
                    let text = self.text()?;
                    self.expect(Kind::DiamondStop, "'}'")?;
                    (Some(text), Some("diamond"))
                }
            }
            Kind::TagEnd => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::Sqe, "']'")?;
                (Some(text), Some("odd"))
            }
            Kind::TrapStart => {
                self.take();
                let text = self.text()?;
                match self.kind() {
                    Kind::TrapEnd => {
                        self.take();
                        (Some(text), Some("trapezoid"))
                    }
                    Kind::InvTrapEnd => {
                        self.take();
                        (Some(text), Some("lean_right"))
                    }
                    _ => return self.syntax("expected the end of a trapezoid"),
                }
            }
            Kind::InvTrapStart => {
                self.take();
                let text = self.text()?;
                match self.kind() {
                    Kind::InvTrapEnd => {
                        self.take();
                        (Some(text), Some("inv_trapezoid"))
                    }
                    Kind::TrapEnd => {
                        self.take();
                        (Some(text), Some("lean_left"))
                    }
                    _ => return self.syntax("expected the end of a trapezoid"),
                }
            }
            _ => (None, None),
        };
        let added = self.db.add_vertex(&id, text, shape, None, None);
        self.db(added)?;
        Ok(id)
    }

    /// `idString`: one or more id tokens, joined as written.
    fn id_string(&mut self) -> Result<String, FlowchartError> {
        if !is_id_token(self.kind()) {
            return self.syntax("expected a node id");
        }
        let mut id = String::new();
        while is_id_token(self.kind()) {
            id.push_str(&self.take().value);
        }
        Ok(id)
    }

    /// `text`: a run of text, or a string or Markdown string followed by text.
    fn text(&mut self) -> Result<FlowText, FlowchartError> {
        let (mut text, label_type) = match self.kind() {
            Kind::Text => (self.take().value, LabelType::Text),
            Kind::Str => (self.take().value, LabelType::String),
            Kind::MdStr => (self.take().value, LabelType::Markdown),
            _ => return self.syntax("expected text"),
        };
        while self.kind() == Kind::Text {
            text.push_str(&self.take().value);
        }
        Ok(FlowText { text, label_type })
    }

    /// `textNoTags`: text without tags, as subgraph titles take it. A plain string here is text.
    fn text_no_tags(&mut self) -> Result<(String, LabelType), FlowchartError> {
        let (mut text, label_type) = match self.kind() {
            Kind::Str => (self.take().value, LabelType::Text),
            Kind::MdStr => (self.take().value, LabelType::Markdown),
            kind if is_text_no_tags_token(kind) => (self.take().value, LabelType::Text),
            _ => return self.syntax("expected a subgraph id or title"),
        };
        while is_text_no_tags_token(self.kind()) {
            text.push_str(&self.take().value);
        }
        Ok((text, label_type))
    }

    /// `edgeText`: the text of a link written between its start and end.
    fn edge_text(&mut self) -> Result<FlowText, FlowchartError> {
        let (mut text, label_type) = match self.kind() {
            Kind::EdgeText => (self.take().value, LabelType::Text),
            Kind::Str => (self.take().value, LabelType::String),
            Kind::MdStr => (self.take().value, LabelType::Markdown),
            _ => return self.syntax("expected the text of a link"),
        };
        while self.kind() == Kind::EdgeText {
            text.push_str(&self.take().value);
        }
        Ok(FlowText { text, label_type })
    }

    /// `link`: a link with its arrow and optional text, or a link whose text is between its start
    /// and end. The text in pipes is `arrowText`; a space after it belongs to the link.
    fn link(&mut self) -> Result<LinkData, FlowchartError> {
        let id = if self.kind() == Kind::LinkId {
            Some(self.take().value)
        } else {
            None
        };
        match self.kind() {
            Kind::Link => {
                let link = self.take();
                let info = destruct_link(&link.value, None);
                let text = if self.kind() == Kind::Pipe {
                    let text = self.arrow_text()?;
                    if self.kind() == Kind::Space {
                        self.take();
                    }
                    Some(text)
                } else {
                    None
                };
                Ok(link_data(info, text, id))
            }
            Kind::StartLink => {
                let start = self.take();
                let text = self.edge_text()?;
                let end = self.expect(Kind::Link, "the end of the link")?;
                let info = destruct_link(&end.value, Some(&start.value));
                Ok(link_data(info, Some(text), id))
            }
            _ => self.syntax("expected a link"),
        }
    }

    /// `arrowText: PIPE text PIPE`.
    fn arrow_text(&mut self) -> Result<FlowText, FlowchartError> {
        self.expect(Kind::Pipe, "'|'")?;
        let text = self.text()?;
        self.expect(Kind::Pipe, "'|'")?;
        Ok(text)
    }
}

fn link_data(info: LinkInfo, text: Option<FlowText>, id: Option<String>) -> LinkData {
    LinkData {
        edge_type: info.edge_type,
        stroke: info.stroke,
        length: info.length,
        text,
        id,
    }
}

/// `destructEndLink`: the arrow at the end of a link, its stroke and its length.
fn destruct_end_link(text: &str) -> LinkInfo {
    let trimmed = crate::text::js_trim(text);
    let mut line = trimmed.get(..trimmed.len().saturating_sub(1)).unwrap_or("");
    let last = trimmed.chars().last();
    let mut edge_type = String::from("arrow_open");
    match last {
        Some('x') => {
            edge_type = String::from("arrow_cross");
            if trimmed.starts_with('x') {
                edge_type = format!("double_{edge_type}");
                line = line.get(1..).unwrap_or("");
            }
        }
        Some('>') => {
            edge_type = String::from("arrow_point");
            if trimmed.starts_with('<') {
                edge_type = format!("double_{edge_type}");
                line = line.get(1..).unwrap_or("");
            }
        }
        Some('o') => {
            edge_type = String::from("arrow_circle");
            if trimmed.starts_with('o') {
                edge_type = format!("double_{edge_type}");
                line = line.get(1..).unwrap_or("");
            }
        }
        _ => {}
    }
    let mut stroke = String::from("normal");
    let mut length = line.chars().count() as i64 - 1;
    if line.starts_with('=') {
        stroke = String::from("thick");
    }
    if line.starts_with('~') {
        stroke = String::from("invisible");
    }
    let dots = line.matches('.').count();
    if dots > 0 {
        stroke = String::from("dotted");
        length = dots as i64;
    }
    LinkInfo {
        edge_type,
        stroke,
        length: u32::try_from(length).ok(),
    }
}

/// `destructStartLink`: the arrow at the start of a link and its stroke.
fn destruct_start_link(text: &str) -> (String, String) {
    let mut rest = crate::text::js_trim(text);
    let mut edge_type = String::from("arrow_open");
    match rest.chars().next() {
        Some('<') => {
            edge_type = String::from("arrow_point");
            rest = rest.get(1..).unwrap_or("");
        }
        Some('x') => {
            edge_type = String::from("arrow_cross");
            rest = rest.get(1..).unwrap_or("");
        }
        Some('o') => {
            edge_type = String::from("arrow_circle");
            rest = rest.get(1..).unwrap_or("");
        }
        _ => {}
    }
    let mut stroke = String::from("normal");
    if rest.contains('=') {
        stroke = String::from("thick");
    }
    if rest.contains('.') {
        stroke = String::from("dotted");
    }
    (edge_type, stroke)
}

/// `destructLink`: the arrow of a link with its start, when it has one. The two ends must agree in
/// stroke; with a head at the start, the type is doubled. Otherwise the link is `INVALID`.
fn destruct_link(end: &str, start: Option<&str>) -> LinkInfo {
    let info = destruct_end_link(end);
    let Some(start) = start else {
        return info;
    };
    let invalid = LinkInfo {
        edge_type: String::from("INVALID"),
        stroke: String::from("INVALID"),
        length: None,
    };
    let (start_type, start_stroke) = destruct_start_link(start);
    if start_stroke != info.stroke {
        return invalid;
    }
    let edge_type = if start_type == "arrow_open" {
        info.edge_type
    } else {
        if start_type != info.edge_type {
            return invalid;
        }
        format!("double_{start_type}")
    };
    LinkInfo {
        edge_type,
        stroke: start_stroke,
        length: info.length,
    }
}
