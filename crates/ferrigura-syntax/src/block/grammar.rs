//! The grammar of `block.jison` (lines 164-292) as recursive descent over the scanner's tokens. The
//! grammar's actions build the statements (`Item`) that the DB populates after the parse; the DB
//! calls that the actions make while parsing (`generateId`, `typeStr2Type`, the edge helpers) are
//! made here. See `docs-port/block/PLAN.md`, section 4.
//!
//! A statement is one of the items; a chained link (`A --> B --> C`) gives the three items of its
//! last link, whose first block has no id, as the reference's `$1.id` on an array gives.

use super::db::{
    edge_str_to_edge_data, edge_str_to_edge_start_data, edge_str_to_pattern, edge_str_to_thickness,
    type_str_to_type,
};
use super::scanner::{Scanner, Spanned, Token};
use crate::error::{ParseError, Source};
use crate::text::js_trim;

/// One statement of the diagram, as the grammar's actions give it to the DB.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    /// A node, a block arrow, or a block of a link. Its fields are those the action copies.
    Node {
        id: Option<String>,
        label: Option<String>,
        block_type: Option<String>,
        directions: Option<Vec<String>>,
        width_in_columns: Option<f64>,
    },
    /// A link between two blocks.
    Edge(EdgeItem),
    /// A space statement: a cell with a generated id and its number of cells.
    Space { id: String, width: f64 },
    /// A composite: `block:id … end` or `block … end`, with its statements.
    Composite {
        id: Option<String>,
        label: Option<String>,
        directions: Option<Vec<String>>,
        width_in_columns: Option<f64>,
        children: Vec<Item>,
    },
    /// `columns N`, with `-1` for auto.
    ColumnSetting(f64),
    /// `classDef id css`.
    ClassDef { id: String, css: String },
    /// `class ids classes`.
    ApplyClass { id: String, style_class: String },
    /// `style id definition`, with the byte offset of the statement for errors.
    ApplyStyles {
        id: String,
        styles: String,
        start: usize,
    },
}

/// A link as the grammar's `nodeStatement` action builds it, before the DB sees it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EdgeItem {
    /// `start-end`, before the DB counts the edges of its key.
    pub(crate) id: String,
    /// The id of the start block; absent when the start is the `undefined` block of a chain.
    pub(crate) start: Option<String>,
    pub(crate) end: String,
    pub(crate) label: String,
    pub(crate) thickness: &'static str,
    pub(crate) pattern: &'static str,
    pub(crate) directions: Option<Vec<String>>,
    pub(crate) arrow_type_end: &'static str,
    pub(crate) arrow_type_start: &'static str,
}

/// A node as the `node` rule gives it: its id, its label and shape text, and its directions.
struct NodeFields {
    id: String,
    label: Option<String>,
    type_str: Option<String>,
    directions: Option<Vec<String>>,
}

/// The fields a link's left side carries into the edge: those of a node, and none of an array.
#[derive(Default)]
struct Left {
    id: Option<String>,
    label: Option<String>,
    block_type: Option<String>,
    directions: Option<Vec<String>>,
}

/// The parser: the scanner, one token of lookahead, and the counter of generated ids.
struct Parser<'a> {
    scanner: Scanner<'a>,
    peeked: Option<Spanned>,
    source: Source<'a>,
    ids: u64,
}

/// Parses the prepared text (`text`, the text the scanner reads) into its statements.
pub(crate) fn parse(text: &str, source: Source<'_>) -> Result<Vec<Item>, ParseError> {
    let mut parser = Parser {
        scanner: Scanner::new(text),
        peeked: None,
        source,
        ids: 0,
    };
    parser.start()
}

/// Whether a token is one of the links that extend a node statement.
fn is_link(token: &Token) -> bool {
    matches!(token, Token::Link(_) | Token::StartLink)
}

impl Parser<'_> {
    /// `start: BLOCK_DIAGRAM_KEY document EOF`.
    fn start(&mut self) -> Result<Vec<Item>, ParseError> {
        let first = self.bump()?;
        if first.token != Token::BlockKey {
            return Err(self.unexpected(&first, "'block' at the start of the diagram"));
        }
        let items = self.document(false)?;
        let last = self.bump()?;
        if last.token != Token::Eof {
            return Err(self.unexpected(&last, "the end of the diagram"));
        }
        Ok(items)
    }

    /// The next token, read once and kept until it is taken.
    fn peek(&mut self) -> Result<&Spanned, ParseError> {
        let next = match self.peeked.take() {
            Some(spanned) => spanned,
            None => self.lex()?,
        };
        Ok(self.peeked.insert(next))
    }

    /// The kind of the next token.
    fn peek_token(&mut self) -> Result<Token, ParseError> {
        Ok(self.peek()?.token.clone())
    }

    /// Takes the next token.
    fn bump(&mut self) -> Result<Spanned, ParseError> {
        match self.peeked.take() {
            Some(spanned) => Ok(spanned),
            None => self.lex(),
        }
    }

    /// Reads the next token of the scanner; a text no rule takes is an error at its position.
    fn lex(&mut self) -> Result<Spanned, ParseError> {
        let source = self.source;
        self.scanner
            .next_token()
            .map_err(|offset| source.error(offset, "unrecognised text"))
    }

    /// Takes the next token, which must be one that `pick` accepts, and returns its value.
    fn expect<T>(
        &mut self,
        expected: &str,
        pick: fn(&Token) -> Option<T>,
    ) -> Result<T, ParseError> {
        let spanned = self.bump()?;
        match pick(&spanned.token) {
            Some(value) => Ok(value),
            None => Err(self.unexpected(&spanned, expected)),
        }
    }

    /// Takes the next token, which must satisfy `accept`.
    fn expect_where(
        &mut self,
        expected: &str,
        accept: fn(&Token) -> bool,
    ) -> Result<Spanned, ParseError> {
        let spanned = self.bump()?;
        if accept(&spanned.token) {
            Ok(spanned)
        } else {
            Err(self.unexpected(&spanned, expected))
        }
    }

    fn expect_str(&mut self) -> Result<String, ParseError> {
        self.expect("a quoted string", |token| match token {
            Token::Str(text) => Some(text.clone()),
            _ => None,
        })
    }

    /// An error at a token that the grammar does not accept there.
    fn unexpected(&self, spanned: &Spanned, expected: &str) -> ParseError {
        let source = self.source;
        source.error(
            spanned.start,
            format!("unexpected token, expected {expected}"),
        )
    }

    /// `document`: one or more statements. A composite's document ends at its `end`, and the
    /// diagram's at the end of the input.
    fn document(&mut self, composite: bool) -> Result<Vec<Item>, ParseError> {
        let mut items = Vec::new();
        loop {
            let stop = match self.peek()?.token {
                Token::End => composite,
                Token::Eof => !composite,
                _ => false,
            };
            if stop {
                break;
            }
            items.extend(self.statement()?);
        }
        if items.is_empty() {
            let spanned = self.peek()?.clone();
            return Err(self.unexpected(&spanned, "a statement"));
        }
        Ok(items)
    }

    /// `statement`: one of the alternatives of the grammar, each with the action of its rule.
    fn statement(&mut self) -> Result<Vec<Item>, ParseError> {
        let spanned = self.peek()?.clone();
        match spanned.token {
            Token::NodeId(_) => self.node_statement(),
            Token::Columns(columns) => {
                self.bump()?;
                Ok(vec![Item::ColumnSetting(columns)])
            }
            Token::SpaceBlock(width) => {
                self.bump()?;
                let id = self.generate_id();
                Ok(vec![Item::Space { id, width }])
            }
            Token::IdBlock => {
                let composite = self.id_block()?;
                Ok(vec![composite])
            }
            Token::BlockKey => {
                self.bump()?;
                let children = self.document(true)?;
                self.end()?;
                let id = self.generate_id();
                Ok(vec![Item::Composite {
                    id: Some(id),
                    label: Some(String::new()),
                    directions: None,
                    width_in_columns: None,
                    children,
                }])
            }
            Token::ClassDef => {
                self.bump()?;
                let id = self.expect("a class name", |token| match token {
                    Token::ClassDefId(text) => Some(text.clone()),
                    _ => None,
                })?;
                let css = self.expect("the style options", |token| match token {
                    Token::ClassDefStyleOpts(text) => Some(text.clone()),
                    _ => None,
                })?;
                Ok(vec![Item::ClassDef {
                    id: js_trim(&id).to_owned(),
                    css: js_trim(&css).to_owned(),
                }])
            }
            Token::Class => {
                self.bump()?;
                let id = self.expect("the ids of the class", |token| match token {
                    Token::ClassEntityIds(text) => Some(text.clone()),
                    _ => None,
                })?;
                let style_class = self.expect("the class names", |token| match token {
                    Token::StyleClass(text) => Some(text.clone()),
                    _ => None,
                })?;
                Ok(vec![Item::ApplyClass {
                    id: js_trim(&id).to_owned(),
                    style_class: js_trim(&style_class).to_owned(),
                }])
            }
            Token::Style => {
                self.bump()?;
                let id = self.expect("the ids of the style", |token| match token {
                    Token::StyleEntityIds(text) => Some(text.clone()),
                    _ => None,
                })?;
                let styles = self.expect("the style definition", |token| match token {
                    Token::StyleDefinitionData(text) => Some(text.clone()),
                    _ => None,
                })?;
                Ok(vec![Item::ApplyStyles {
                    id: js_trim(&id).to_owned(),
                    styles: js_trim(&styles).to_owned(),
                    start: spanned.start,
                }])
            }
            _ => Err(self.unexpected(&spanned, "a statement")),
        }
    }

    /// `end`, which closes a composite.
    fn end(&mut self) -> Result<(), ParseError> {
        self.expect_where("'end'", |token| *token == Token::End)?;
        Ok(())
    }

    /// `blockStatement: id-block nodeStatement document end`: the composite's id comes from its
    /// node statement, and the reference's second generated id is taken and dropped.
    fn id_block(&mut self) -> Result<Item, ParseError> {
        self.bump()?;
        let head = self.node_statement()?;
        let children = self.document(true)?;
        self.end()?;
        self.generate_id();
        Ok(composite_from(head, children))
    }

    /// `nodeStatement`: a node with its span, followed by any number of links and nodes. A plain
    /// statement is one item; a link gives the three items of its last link.
    fn node_statement(&mut self) -> Result<Vec<Item>, ParseError> {
        let node = self.node()?;
        let size = match self.peek_token()? {
            Token::Size(width) => {
                self.bump()?;
                Some(width)
            }
            _ => None,
        };
        let block_type = Some(type_str_to_type(node.type_str.as_deref()).to_owned());
        if !is_link(&self.peek_token()?) {
            return Ok(vec![Item::Node {
                id: Some(node.id),
                label: node.label,
                block_type,
                directions: node.directions,
                width_in_columns: Some(size.unwrap_or(1.0)),
            }]);
        }
        let mut left = Left {
            id: Some(node.id),
            label: node.label,
            block_type,
            directions: node.directions,
        };
        let mut items = Vec::new();
        while is_link(&self.peek_token()?) {
            let (link, label) = self.link()?;
            let right = self.node()?;
            let right_type = type_str_to_type(right.type_str.as_deref());
            items = chain(&left, &link, label, right, right_type);
            left = Left::default();
        }
        Ok(items)
    }

    /// `link: LINK | START_LINK LINK_LABEL STR LINK`: the text of the closing link and the label.
    fn link(&mut self) -> Result<(String, String), ParseError> {
        let spanned = self.bump()?;
        match &spanned.token {
            Token::Link(text) => Ok((text.clone(), String::new())),
            Token::StartLink => {
                self.expect_where("the start of the label", |token| *token == Token::LinkLabel)?;
                let label = self.expect_str()?;
                let text = self.expect("the closing link", |token| match token {
                    Token::Link(text) => Some(text.clone()),
                    _ => None,
                })?;
                Ok((text, label))
            }
            _ => Err(self.unexpected(&spanned, "a link")),
        }
    }

    /// `node: NODE_ID | NODE_ID nodeShapeNLabel`.
    fn node(&mut self) -> Result<NodeFields, ParseError> {
        let id = self.expect("an id", |token| match token {
            Token::NodeId(text) => Some(text.clone()),
            _ => None,
        })?;
        match self.peek_token()? {
            Token::NodeDStart(open) => {
                self.bump()?;
                let label = self.expect_str()?;
                let close =
                    self.expect("the closing bracket of the label", |token| match token {
                        Token::NodeDEnd(text) => Some(text.clone()),
                        _ => None,
                    })?;
                Ok(NodeFields {
                    id,
                    label: Some(label),
                    type_str: Some(format!("{open}{close}")),
                    directions: None,
                })
            }
            Token::BlockArrowStart => {
                self.bump()?;
                let label = self.expect_str()?;
                let directions = self.dir_list()?;
                self.expect_where("the end of the block arrow", |token| {
                    *token == Token::BlockArrowEnd
                })?;
                Ok(NodeFields {
                    id,
                    label: Some(label),
                    type_str: Some("<[]>".to_owned()),
                    directions: Some(directions),
                })
            }
            _ => Ok(NodeFields {
                id,
                label: None,
                type_str: None,
                directions: None,
            }),
        }
    }

    /// `dirList: DIR | DIR dirList`.
    fn dir_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut directions = vec![self.expect("a direction", |token| match token {
            Token::Dir(text) => Some(text.clone()),
            _ => None,
        })?];
        while let Token::Dir(text) = self.peek_token()? {
            self.bump()?;
            directions.push(text);
        }
        Ok(directions)
    }

    /// `generateId()`: a new id in the reference's format, `id-<random>-<n>`. The random part is
    /// the counter in twelve digits, so the ids are the same on every run.
    fn generate_id(&mut self) -> String {
        self.ids += 1;
        format!("id-{:012}-{}", self.ids, self.ids)
    }
}

/// The composite of an `id-block` statement. A node statement gives its id, label, directions and
/// span; a chain gives an object with none of them, as the reference's spread of an array does.
fn composite_from(head: Vec<Item>, children: Vec<Item>) -> Item {
    match head.as_slice() {
        [
            Item::Node {
                id,
                label,
                directions,
                width_in_columns,
                ..
            },
        ] => Item::Composite {
            id: id.clone(),
            label: label.clone(),
            directions: directions.clone(),
            width_in_columns: *width_in_columns,
            children,
        },
        _ => Item::Composite {
            id: None,
            label: None,
            directions: None,
            width_in_columns: None,
            children,
        },
    }
}

/// The three items of one link of a chain: the left block, the edge, and the right block. The
/// left block keeps the fields of a node, and the right block takes its type from its shape text.
fn chain(left: &Left, link: &str, label: String, right: NodeFields, right_type: &str) -> Vec<Item> {
    let start = left.id.clone();
    let end = right.id.clone();
    let id = format!("{}-{}", start.as_deref().unwrap_or("undefined"), end);
    vec![
        Item::Node {
            id: left.id.clone(),
            label: left.label.clone(),
            block_type: left.block_type.clone(),
            directions: left.directions.clone(),
            width_in_columns: None,
        },
        Item::Edge(EdgeItem {
            id,
            start,
            end,
            label,
            thickness: edge_str_to_thickness(link),
            pattern: edge_str_to_pattern(link),
            directions: right.directions.clone(),
            arrow_type_end: edge_str_to_edge_data(link),
            arrow_type_start: edge_str_to_edge_start_data(link),
        }),
        Item::Node {
            id: Some(right.id),
            label: right.label,
            block_type: Some(right_type.to_owned()),
            directions: right.directions,
            width_in_columns: None,
        },
    ]
}
