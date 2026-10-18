//! The agentflow grammar of `agentflow.jison` as recursive descent over the scanner's tokens. Each
//! production has a function named after it; the DB calls happen where the grammar's actions run,
//! in the reduction order of the LALR parser (`docs-port/agentflow/PLAN.md`, section 3). The
//! lookahead of each choice is the one the LALR table makes: a token that the grammar can only
//! shift after a production decides it, and the grammar's greedy lists take every token they can.

use ferrigura_model::agentflow::AgentflowDiagram;

use super::DIALECT;
use super::db::{BodyItem, Db, DbError, FlowText, LinkData, Position};
use crate::error::{ParseError, Source};
use crate::flowchart::scanner::{Kind, Token, tokenize};

/// What a statement gives the list it is in: the value of the grammar's statement.
enum Statement {
    /// The node ids of a vertex statement, in the order the grammar gives them.
    Nodes(Vec<String>),
    /// The id of a container or of a connector: a string in the list.
    Str(String),
    /// A `direction` statement.
    Dir(String),
    /// A statement whose value is nothing (or an empty list).
    Empty,
}

/// The kinds of `idStringToken`.
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

/// The kinds of `alphaNumToken`.
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

/// The kinds of `textToken`.
fn is_text_token(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Text | Kind::TagStart | Kind::TagEnd | Kind::UnicodeText
    )
}

/// The kinds of `textNoTagsToken`: the keywords count as text here.
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
            | Kind::Flow
            | Kind::Connector
            | Kind::Global
            | Kind::End
            | Kind::Down
            | Kind::Up
            | Kind::StartLink
    )
}

/// The kinds of `styleComponent`.
fn is_style_component(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Num | Kind::NodeString | Kind::Colon | Kind::Space | Kind::Brkt | Kind::Style
    )
}

/// The kinds of `separator`: a line break, a semicolon, the end, or a comment.
fn is_separator(kind: Kind) -> bool {
    matches!(kind, Kind::Newline | Kind::Semi | Kind::Eof | Kind::Comment)
}

/// Parses the prepared agentflow text: the header, the document, and the DB's model.
pub(crate) fn parse_text(
    text: &str,
    line_offset: usize,
    title: String,
    max_edges: usize,
) -> Result<AgentflowDiagram, ParseError> {
    let source = Source::new(text, line_offset);
    let tokens = tokenize(text, &DIALECT)
        .map_err(|error| source.error(error.offset, "Lexical error. Unrecognized text."))?;
    let mut parser = Parser {
        source,
        tokens,
        pos: 0,
        db: Db::new(max_edges),
    };
    parser.graph_config()?;
    parser.document(false)?;
    Ok(parser.db.into_model(title))
}

pub(crate) struct Parser<'a> {
    source: Source<'a>,
    tokens: Vec<Token>,
    pos: usize,
    db: Db,
}

impl Parser<'_> {
    /// The kind of the token at `pos + ahead`, or the end of the input.
    fn kind_at(&self, ahead: usize) -> Kind {
        self.tokens
            .get(self.pos + ahead)
            .map_or(Kind::Eof, |token| token.kind)
    }

    fn kind(&self) -> Kind {
        self.kind_at(0)
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

    fn syntax<T>(&self, message: impl Into<String>) -> Result<T, ParseError> {
        Err(self.source.error(self.offset(), message))
    }

    fn expect(&mut self, kind: Kind, what: &str) -> Result<Token, ParseError> {
        if self.kind() == kind {
            Ok(self.take())
        } else {
            self.syntax(format!("Parse error: expected {what}"))
        }
    }

    /// Consumes one or more blanks; a list of them is `spaceList`.
    fn space_list(&mut self) -> Result<(), ParseError> {
        self.expect(Kind::Space, "a blank")?;
        while self.kind() == Kind::Space {
            self.take();
        }
        Ok(())
    }

    fn separator(&mut self) -> Result<(), ParseError> {
        if is_separator(self.kind()) {
            self.take();
            Ok(())
        } else {
            self.syntax("Parse error: expected the end of the statement")
        }
    }

    /// Runs a DB call that may throw, and reports its message at the current position.
    fn db<T>(&self, result: Result<T, DbError>) -> Result<T, ParseError> {
        result.or_else(|DbError(message)| self.syntax(message))
    }

    /// `graphConfig`: the blanks, line breaks and comments before the header, then `agentflow-beta`
    /// with a direction or without one (`agentflow.jison:349-365`).
    fn graph_config(&mut self) -> Result<(), ParseError> {
        while matches!(self.kind(), Kind::Space | Kind::Newline | Kind::Comment) {
            self.take();
        }
        self.expect(Kind::Graph, "'agentflow-beta'")?;
        match self.kind() {
            Kind::NoDir => {
                self.take();
                self.db.set_direction("TB");
            }
            Kind::Dir => {
                let dir = self.take();
                self.first_stmt_separator()?;
                self.db.set_direction(&dir.value);
            }
            _ => return self.syntax("Parse error: expected a direction after 'agentflow-beta'"),
        }
        Ok(())
    }

    /// `FirstStmtSeparator: SEMI | NEWLINE | spaceList NEWLINE | COMMENT`.
    fn first_stmt_separator(&mut self) -> Result<(), ParseError> {
        match self.kind() {
            Kind::Semi | Kind::Newline | Kind::Comment => {
                self.take();
                Ok(())
            }
            Kind::Space => {
                self.space_list()?;
                self.expect(Kind::Newline, "the end of the header line")?;
                Ok(())
            }
            _ => self.syntax("Parse error: expected the end of the header line"),
        }
    }

    /// `document`: the lines up to the end of the input, or up to `end` in a flow. The items are
    /// what the flow's member list holds; at the top level they are dropped.
    fn document(&mut self, in_flow: bool) -> Result<Vec<BodyItem>, ParseError> {
        let mut items = Vec::new();
        loop {
            match self.kind() {
                Kind::Eof => {
                    if in_flow {
                        return self.syntax("Parse error: expected 'end' to close the flow");
                    }
                    self.take();
                    return Ok(items);
                }
                Kind::End => {
                    if in_flow {
                        return Ok(items);
                    }
                    return self.syntax("Parse error: 'end' without a flow");
                }
                Kind::Comment => {
                    self.take();
                }
                Kind::Semi | Kind::Newline | Kind::Space => {
                    let token = self.take();
                    items.push(BodyItem::Str(token.value));
                }
                _ => match self.statement()? {
                    Statement::Nodes(nodes) => {
                        items.extend(nodes.into_iter().map(BodyItem::Str));
                    }
                    Statement::Str(text) => items.push(BodyItem::Str(text)),
                    Statement::Dir(dir) => items.push(BodyItem::Dir(dir)),
                    Statement::Empty => {}
                },
            }
        }
    }

    /// `statement`: one of the statement forms (`agentflow.jison:392-429`).
    fn statement(&mut self) -> Result<Statement, ParseError> {
        match self.kind() {
            Kind::Flow => self.flow_statement(),
            Kind::Global => self.global_statement(),
            Kind::Connector => self.connector_statement(),
            Kind::DirectionTb => self.direction("TB"),
            Kind::DirectionBt => self.direction("BT"),
            Kind::DirectionRl => self.direction("RL"),
            Kind::DirectionLr => self.direction("LR"),
            Kind::DirectionTd => self.direction("TD"),
            Kind::AccTitle => {
                self.take();
                let value = self.expect(Kind::AccTitleValue, "the accessibility title")?;
                let text = value.value.trim().to_owned();
                self.db.set_acc_title(&text);
                Ok(Statement::Str(text))
            }
            Kind::AccDescr => {
                self.take();
                let value = self.expect(Kind::AccDescrValue, "the accessibility description")?;
                let text = value.value.trim().to_owned();
                self.db.set_acc_descr(&text);
                Ok(Statement::Str(text))
            }
            Kind::AccDescrMultiValue => {
                let value = self.take();
                let text = value.value.trim().to_owned();
                self.db.set_acc_descr(&text);
                Ok(Statement::Str(text))
            }
            Kind::Style => {
                self.style_statement()?;
                self.separator()?;
                Ok(Statement::Empty)
            }
            Kind::LinkStyle => {
                self.link_style_statement()?;
                self.separator()?;
                Ok(Statement::Empty)
            }
            Kind::ClassDef => {
                self.class_def_statement()?;
                self.separator()?;
                Ok(Statement::Empty)
            }
            Kind::Class => {
                self.class_statement()?;
                self.separator()?;
                Ok(Statement::Empty)
            }
            Kind::Click => {
                self.click_statement()?;
                self.separator()?;
                Ok(Statement::Empty)
            }
            _ => {
                let nodes = self.vertex_statement()?;
                self.separator()?;
                Ok(Statement::Nodes(nodes))
            }
        }
    }

    /// `direction_tb` and the others: the statement is the direction, and the line break after it
    /// is the next line.
    fn direction(&mut self, dir: &str) -> Result<Statement, ParseError> {
        self.take();
        Ok(Statement::Dir(dir.to_owned()))
    }

    /// `flow` (`agentflow.jison:405-416`): the five forms of a container, with its label, its
    /// shape data, its members and its `end`.
    fn flow_statement(&mut self) -> Result<Statement, ParseError> {
        self.take();
        let id = if self.kind() == Kind::Space {
            self.take();
            let name = self.text_no_tags()?;
            match self.kind() {
                Kind::Sqs => {
                    self.take();
                    let title = self.text()?;
                    self.expect(Kind::Sqe, "']'")?;
                    let shape = self.optional_shape_data();
                    self.separator()?;
                    let body = self.document(true)?;
                    self.expect(Kind::End, "'end'")?;
                    let result = self.db.add_sub_graph(Some(&name), &body, Some(&title));
                    let id = self.db(result)?;
                    self.add_shape_data(&id, shape)?;
                    id
                }
                Kind::ShapeData => {
                    let shape = self.shape_data()?;
                    self.separator()?;
                    let body = self.document(true)?;
                    self.expect(Kind::End, "'end'")?;
                    let blank = FlowText::new("", "text");
                    let result = self.db.add_sub_graph(Some(&name), &body, Some(&blank));
                    let id = self.db(result)?;
                    self.add_shape_data(&id, Some(shape))?;
                    id
                }
                kind if is_separator(kind) => {
                    self.separator()?;
                    let body = self.document(true)?;
                    self.expect(Kind::End, "'end'")?;
                    let blank = FlowText::new("", "text");
                    let result = self.db.add_sub_graph(Some(&name), &body, Some(&blank));
                    self.db(result)?
                }
                _ => return self.syntax("Parse error: expected the label of the flow"),
            }
        } else if is_separator(self.kind()) {
            self.separator()?;
            let body = self.document(true)?;
            self.expect(Kind::End, "'end'")?;
            let result = self.db.add_sub_graph(None, &body, None);
            self.db(result)?
        } else {
            return self
                .syntax("Parse error: expected a blank or the end of the line after 'flow'");
        };
        Ok(Statement::Str(id))
    }

    /// `addVertex(id, undefined, …, shapeData)` for the shape data of a container or a connector.
    fn add_shape_data(&mut self, id: &str, shape: Option<String>) -> Result<(), ParseError> {
        match shape {
            Some(data) => {
                let result = self.db.add_vertex(id, None, None, None, None, Some(&data));
                self.db(result)
            }
            None => Ok(()),
        }
    }

    /// `global separator document end`: its ids are outside every container, and it is no
    /// statement of its own.
    fn global_statement(&mut self) -> Result<Statement, ParseError> {
        self.take();
        self.separator()?;
        let body = self.document(true)?;
        self.expect(Kind::End, "'end'")?;
        self.db.add_global(&body);
        Ok(Statement::Empty)
    }

    /// `connector SPACE textNoTags [SQS text SQE] [shapeData] separator` (`agentflow.jison:417-424`).
    fn connector_statement(&mut self) -> Result<Statement, ParseError> {
        self.take();
        self.expect(Kind::Space, "a blank after 'connector'")?;
        let name = self.text_no_tags()?;
        let id = match self.kind() {
            Kind::Sqs => {
                self.take();
                let title = self.text()?;
                self.expect(Kind::Sqe, "']'")?;
                let shape = self.optional_shape_data();
                self.separator()?;
                let id = self.db.add_connector(&name, Some(&title));
                self.add_shape_data(&id, shape)?;
                id
            }
            Kind::ShapeData => {
                let shape = self.shape_data()?;
                self.separator()?;
                let blank = FlowText::new("", "text");
                let id = self.db.add_connector(&name, Some(&blank));
                self.add_shape_data(&id, Some(shape))?;
                id
            }
            kind if is_separator(kind) => {
                self.separator()?;
                let blank = FlowText::new("", "text");
                self.db.add_connector(&name, Some(&blank))
            }
            _ => return self.syntax("Parse error: expected the label of the connector"),
        };
        Ok(Statement::Str(id))
    }

    /// `vertexStatement`: a node, then the links and nodes that follow it, with the shape data of
    /// each node. The value is the nodes of the statement, the last link's group first.
    fn vertex_statement(&mut self) -> Result<Vec<String>, ParseError> {
        let mut nodes = self.node()?;
        // `node shapeData` is a statement of its own, which no blank can follow; `node spaceList`
        // is one too, and a link may come after it.
        let mut blanks_allowed = true;
        if self.kind() == Kind::ShapeData {
            let shape = self.shape_data()?;
            let last = nodes.last().cloned().unwrap_or_default();
            self.add_shape_data(&last, Some(shape))?;
            blanks_allowed = false;
        }
        let mut start_group = nodes.clone();
        loop {
            match self.kind() {
                Kind::Link | Kind::StartLink | Kind::LinkId => {
                    let link = self.link()?;
                    let group = self.node()?;
                    blanks_allowed = true;
                    if self.kind() == Kind::ShapeData {
                        let shape = self.shape_data()?;
                        let last = group.last().cloned().unwrap_or_default();
                        self.add_shape_data(&last, Some(shape))?;
                        blanks_allowed = false;
                    }
                    let result = self.db.add_link(&start_group, &group, &link);
                    self.db(result)?;
                    start_group = group.clone();
                    nodes = [group, nodes].concat();
                }
                Kind::Space if blanks_allowed => {
                    self.space_list()?;
                }
                _ => break,
            }
        }
        Ok(nodes)
    }

    /// `node`: a styled vertex, then the vertices joined to it with `&` (`agentflow.jison:456-462`).
    /// A `&` takes blanks on both sides; the shape data of the node before it is set when the
    /// `&` follows.
    fn node(&mut self) -> Result<Vec<String>, ParseError> {
        let mut ids = vec![self.styled_vertex()?];
        while self.amp_follows() {
            let shape = if self.kind() == Kind::ShapeData {
                Some(self.shape_data()?)
            } else {
                None
            };
            self.space_list()?;
            self.expect(Kind::Amp, "'&'")?;
            self.space_list()?;
            let vertex = self.styled_vertex()?;
            if let Some(data) = shape {
                let last = ids.last().cloned().unwrap_or_default();
                self.add_shape_data(&last, Some(data))?;
            }
            ids.push(vertex);
        }
        Ok(ids)
    }

    /// Whether the tokens ahead continue a node with `&`: optional shape data, blanks, and `&`.
    fn amp_follows(&self) -> bool {
        let mut ahead = 0;
        if self.kind_at(ahead) == Kind::ShapeData {
            while self.kind_at(ahead) == Kind::ShapeData {
                ahead += 1;
            }
        }
        let blanks_start = ahead;
        while self.kind_at(ahead) == Kind::Space {
            ahead += 1;
        }
        ahead > blanks_start && self.kind_at(ahead) == Kind::Amp
    }

    /// `styledVertex: vertex | vertex STYLE_SEPARATOR idString`.
    fn styled_vertex(&mut self) -> Result<String, ParseError> {
        let id = self.vertex()?;
        if self.kind() == Kind::StyleSeparator {
            self.take();
            let class = self.id_string()?;
            self.db.set_class(&id, &class);
        }
        Ok(id)
    }

    /// `vertex`: an id with its shape, or a bare id (`agentflow.jison:470-504`). The bracket form
    /// reads its text and then the DB call gets the text and the shape, after the closing bracket.
    fn vertex(&mut self) -> Result<String, ParseError> {
        let id = self.id_string()?;
        let (text, shape) = match self.kind() {
            Kind::Sqs => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::Sqe, "']'")?;
                (text, "square")
            }
            Kind::DoubleCircleStart => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::DoubleCircleEnd, "')))'")?;
                (text, "doublecircle")
            }
            Kind::Ps => {
                self.take();
                if self.kind() == Kind::Ps {
                    self.take();
                    let text = self.text()?;
                    self.expect(Kind::Pe, "')'")?;
                    self.expect(Kind::Pe, "')'")?;
                    (text, "circle")
                } else {
                    let text = self.text()?;
                    self.expect(Kind::Pe, "')'")?;
                    (text, "round")
                }
            }
            Kind::EllipseStart => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::EllipseEnd, "'-)'")?;
                (text, "ellipse")
            }
            Kind::StadiumStart => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::StadiumEnd, "'])'")?;
                (text, "stadium")
            }
            Kind::SubroutineStart => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::SubroutineEnd, "']]'")?;
                (text, "subroutine")
            }
            Kind::VertexWithPropsStart => {
                self.take();
                let field = self.expect(Kind::NodeString, "a property name")?.value;
                self.expect(Kind::Colon, "':'")?;
                let value = self.expect(Kind::NodeString, "a property value")?.value;
                self.expect(Kind::Pipe, "'|'")?;
                let text = self.text()?;
                self.expect(Kind::Sqe, "']'")?;
                let props = std::iter::once((field, value)).collect();
                let result =
                    self.db
                        .add_vertex(&id, Some(&text), Some("rect"), None, Some(props), None);
                self.db(result)?;
                return Ok(id);
            }
            Kind::CylinderStart => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::CylinderEnd, "')]'")?;
                (text, "cylinder")
            }
            Kind::DiamondStart => {
                self.take();
                if self.kind() == Kind::DiamondStart {
                    self.take();
                    let text = self.text()?;
                    self.expect(Kind::DiamondStop, "'}'")?;
                    self.expect(Kind::DiamondStop, "'}'")?;
                    (text, "hexagon")
                } else {
                    let text = self.text()?;
                    self.expect(Kind::DiamondStop, "'}'")?;
                    (text, "diamond")
                }
            }
            Kind::TagEnd => {
                self.take();
                let text = self.text()?;
                self.expect(Kind::Sqe, "']'")?;
                (text, "odd")
            }
            Kind::TrapStart => {
                self.take();
                let text = self.text()?;
                match self.kind() {
                    Kind::TrapEnd => {
                        self.take();
                        (text, "trapezoid")
                    }
                    Kind::InvTrapEnd => {
                        self.take();
                        (text, "lean_right")
                    }
                    _ => return self.syntax("Parse error: expected the end of the trapezoid"),
                }
            }
            Kind::InvTrapStart => {
                self.take();
                let text = self.text()?;
                match self.kind() {
                    Kind::InvTrapEnd => {
                        self.take();
                        (text, "inv_trapezoid")
                    }
                    Kind::TrapEnd => {
                        self.take();
                        (text, "lean_left")
                    }
                    _ => return self.syntax("Parse error: expected the end of the trapezoid"),
                }
            }
            _ => {
                let result = self.db.add_vertex(&id, None, None, None, None, None);
                self.db(result)?;
                return Ok(id);
            }
        };
        self.add_vertex_with_text(&id, &text, shape)?;
        Ok(id)
    }

    /// `addVertex(id, text, type)`: the vertex of a bracket form, with its text and its shape.
    fn add_vertex_with_text(
        &mut self,
        id: &str,
        text: &FlowText,
        shape: &str,
    ) -> Result<(), ParseError> {
        let result = self
            .db
            .add_vertex(id, Some(text), Some(shape), None, None, None);
        self.db(result)
    }

    /// `idString`: one or more id tokens, read as one id.
    fn id_string(&mut self) -> Result<String, ParseError> {
        if !is_id_token(self.kind()) {
            return self.syntax("Parse error: expected an id");
        }
        let mut id = String::new();
        while is_id_token(self.kind()) {
            id.push_str(&self.take().value);
        }
        Ok(id)
    }

    /// `alphaNum`: one or more word tokens, read as one word.
    fn alpha_num(&mut self) -> Result<String, ParseError> {
        if !is_alpha_num_token(self.kind()) {
            return self.syntax("Parse error: expected a word");
        }
        let mut word = String::new();
        while is_alpha_num_token(self.kind()) {
            word.push_str(&self.take().value);
        }
        Ok(word)
    }

    /// `text`: text tokens, or one quoted string (`agentflow.jison:544-552`).
    fn text(&mut self) -> Result<FlowText, ParseError> {
        match self.kind() {
            Kind::Str => Ok(FlowText::new(self.take().value, "string")),
            Kind::MdStr => Ok(FlowText::new(self.take().value, "markdown")),
            kind if is_text_token(kind) => {
                let mut text = String::new();
                while is_text_token(self.kind()) {
                    text.push_str(&self.take().value);
                }
                Ok(FlowText::new(text, "text"))
            }
            _ => self.syntax("Parse error: expected text"),
        }
    }

    /// `textNoTags`: text tokens that may include the keywords, then more of them after a quoted
    /// string (`agentflow.jison:560-568`).
    fn text_no_tags(&mut self) -> Result<FlowText, ParseError> {
        let (mut text, kind) = match self.kind() {
            Kind::Str => (self.take().value, "text"),
            Kind::MdStr => (self.take().value, "markdown"),
            kind if is_text_no_tags_token(kind) => (String::new(), "text"),
            _ => return self.syntax("Parse error: expected a label"),
        };
        while is_text_no_tags_token(self.kind()) {
            text.push_str(&self.take().value);
        }
        Ok(FlowText::new(text, kind))
    }

    /// `edgeText`: the text of a link that starts with `--`, one quoted string, or edge text tokens.
    fn edge_text(&mut self) -> Result<FlowText, ParseError> {
        match self.kind() {
            Kind::Str => Ok(FlowText::new(self.take().value, "string")),
            Kind::MdStr => Ok(FlowText::new(self.take().value, "markdown")),
            Kind::EdgeText | Kind::UnicodeText => {
                let mut text = String::new();
                while matches!(self.kind(), Kind::EdgeText | Kind::UnicodeText) {
                    text.push_str(&self.take().value);
                }
                Ok(FlowText::new(text, "text"))
            }
            _ => self.syntax("Parse error: expected the text of the link"),
        }
    }

    /// `link` (`agentflow.jison:508-520`): a solid link, optionally with a label between pipes; a
    /// link that starts with `--` and has its text before its end; either with an id before it.
    fn link(&mut self) -> Result<LinkData, ParseError> {
        match self.kind() {
            Kind::Link => {
                let token = self.take();
                let mut link = self.db.destruct_link(&token.value);
                if self.kind() == Kind::Pipe {
                    self.take();
                    let text = self.text()?;
                    self.expect(Kind::Pipe, "'|'")?;
                    link.text = Some(text);
                    if self.kind() == Kind::Space {
                        self.take();
                    }
                }
                Ok(link)
            }
            Kind::LinkId => {
                let id = self.take().value;
                let mut link = if self.kind() == Kind::StartLink {
                    self.take_start_link()?
                } else {
                    let token = self.expect(Kind::Link, "the end of the link")?;
                    self.db.destruct_link(&token.value)
                };
                link.id = Some(id);
                Ok(link)
            }
            Kind::StartLink => self.take_start_link(),
            _ => self.syntax("Parse error: expected a link"),
        }
    }

    /// `START_LINK edgeText LINK`, with no id.
    fn take_start_link(&mut self) -> Result<LinkData, ParseError> {
        self.take();
        let text = self.edge_text()?;
        let token = self.expect(Kind::Link, "the end of the link")?;
        let mut link = self.db.destruct_link(&token.value);
        link.text = Some(text);
        Ok(link)
    }

    /// `shapeData`: one or more shape data tokens, read as one body.
    fn shape_data(&mut self) -> Result<String, ParseError> {
        if self.kind() != Kind::ShapeData {
            return self.syntax("Parse error: expected shape data");
        }
        let mut body = String::new();
        while self.kind() == Kind::ShapeData {
            body.push_str(&self.take().value);
        }
        Ok(body)
    }

    /// A shape data block when one follows, as the statements that take it may have none.
    fn optional_shape_data(&mut self) -> Option<String> {
        if self.kind() == Kind::ShapeData {
            self.shape_data().ok()
        } else {
            None
        }
    }

    /// `style` (`styleStatement`): `STYLE SPACE idString SPACE stylesOpt`.
    fn style_statement(&mut self) -> Result<(), ParseError> {
        self.take();
        self.expect(Kind::Space, "a blank")?;
        let id = self.id_string()?;
        self.expect(Kind::Space, "a blank")?;
        let styles = self.styles_opt()?;
        let result = self
            .db
            .add_vertex(&id, None, None, Some(styles.as_slice()), None, None);
        self.db(result)
    }

    /// `classDef` (`classDefStatement`): `CLASSDEF SPACE idString SPACE stylesOpt`.
    fn class_def_statement(&mut self) -> Result<(), ParseError> {
        self.take();
        self.expect(Kind::Space, "a blank")?;
        let id = self.id_string()?;
        self.expect(Kind::Space, "a blank")?;
        let styles = self.styles_opt()?;
        self.db.add_class(&id, &styles);
        Ok(())
    }

    /// `class` (`classStatement`): `CLASS SPACE idString SPACE idString`.
    fn class_statement(&mut self) -> Result<(), ParseError> {
        self.take();
        self.expect(Kind::Space, "a blank")?;
        let vertex = self.id_string()?;
        self.expect(Kind::Space, "a blank")?;
        let class = self.id_string()?;
        self.db.set_class(&vertex, &class);
        Ok(())
    }

    /// `linkStyle` (`linkStyleStatement`): the positions (`default` or a list of numbers), then
    /// the optional `interpolate` and the optional styles.
    fn link_style_statement(&mut self) -> Result<(), ParseError> {
        self.take();
        self.expect(Kind::Space, "a blank")?;
        let positions = match self.kind() {
            Kind::Default => {
                self.take();
                vec![Position::Default]
            }
            Kind::Num => self.num_list()?.into_iter().map(Position::Index).collect(),
            _ => return self.syntax("Parse error: expected a position for linkStyle"),
        };
        self.expect(Kind::Space, "a blank")?;
        if self.kind() == Kind::Interpolate {
            self.take();
            self.expect(Kind::Space, "a blank")?;
            let interpolate = self.alpha_num()?;
            if self.kind() == Kind::Space {
                self.take();
                let styles = self.styles_opt()?;
                let result = self.db.update_link_interpolate(&positions, &interpolate);
                self.db(result)?;
                let result = self.db.update_link(&positions, &styles);
                self.db(result)?;
            } else {
                let result = self.db.update_link_interpolate(&positions, &interpolate);
                self.db(result)?;
            }
        } else {
            let styles = self.styles_opt()?;
            let result = self.db.update_link(&positions, &styles);
            self.db(result)?;
        }
        Ok(())
    }

    /// `numList: NUM | numList COMMA NUM`: the texts of the numbers.
    fn num_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut numbers = vec![self.expect(Kind::Num, "a number")?.value];
        while self.kind() == Kind::Comma {
            self.take();
            numbers.push(self.expect(Kind::Num, "a number")?.value);
        }
        Ok(numbers)
    }

    /// `stylesOpt: style | stylesOpt COMMA style`.
    fn styles_opt(&mut self) -> Result<Vec<String>, ParseError> {
        let mut styles = vec![self.style()?];
        while self.kind() == Kind::Comma {
            self.take();
            styles.push(self.style()?);
        }
        Ok(styles)
    }

    /// `style: styleComponent | style styleComponent`: the text of one style.
    fn style(&mut self) -> Result<String, ParseError> {
        if !is_style_component(self.kind()) {
            return self.syntax("Parse error: expected a style");
        }
        let mut style = String::new();
        while is_style_component(self.kind()) {
            style.push_str(&self.take().value);
        }
        Ok(style)
    }

    /// `click` (`clickStatement`, `agentflow.jison:579-594`): the sixteen forms, which name a
    /// callback, a link, a tooltip or a link target after the id.
    fn click_statement(&mut self) -> Result<(), ParseError> {
        let id = self.take().value;
        match self.kind() {
            Kind::CallbackName => {
                self.take();
                if self.kind() == Kind::CallbackArgs {
                    self.take();
                }
                self.db.set_click_event(&id);
                if self.kind() == Kind::Space {
                    self.take();
                    let tooltip = self.expect(Kind::Str, "a tooltip")?.value;
                    self.db.set_tooltip(&id, &tooltip);
                }
                Ok(())
            }
            Kind::Href => {
                self.take();
                let link = self.expect(Kind::Str, "a link")?.value;
                if self.kind() == Kind::Space {
                    self.take();
                    if self.kind() == Kind::LinkTarget {
                        let target = self.take().value;
                        self.db.set_link(&id, &link, Some(&target));
                        return Ok(());
                    }
                    let tooltip = self.expect(Kind::Str, "a tooltip or a link target")?.value;
                    if self.kind() == Kind::Space {
                        self.take();
                        let target = self.expect(Kind::LinkTarget, "a link target")?.value;
                        self.db.set_link(&id, &link, Some(&target));
                    } else {
                        self.db.set_link(&id, &link, None);
                    }
                    self.db.set_tooltip(&id, &tooltip);
                    return Ok(());
                }
                self.db.set_link(&id, &link, None);
                Ok(())
            }
            kind if is_alpha_num_token(kind) => {
                let _callback = self.alpha_num()?;
                self.db.set_click_event(&id);
                if self.kind() == Kind::Space {
                    self.take();
                    let tooltip = self.expect(Kind::Str, "a tooltip")?.value;
                    self.db.set_tooltip(&id, &tooltip);
                }
                Ok(())
            }
            Kind::Str => {
                let link = self.take().value;
                if self.kind() == Kind::Space {
                    self.take();
                    if self.kind() == Kind::LinkTarget {
                        let target = self.take().value;
                        self.db.set_link(&id, &link, Some(&target));
                        return Ok(());
                    }
                    let tooltip = self.expect(Kind::Str, "a tooltip or a link target")?.value;
                    if self.kind() == Kind::Space {
                        self.take();
                        let target = self.expect(Kind::LinkTarget, "a link target")?.value;
                        self.db.set_link(&id, &link, Some(&target));
                    } else {
                        self.db.set_link(&id, &link, None);
                    }
                    self.db.set_tooltip(&id, &tooltip);
                    return Ok(());
                }
                self.db.set_link(&id, &link, None);
                Ok(())
            }
            _ => self.syntax("Parse error: expected a callback, a link or a tooltip after the id"),
        }
    }
}
