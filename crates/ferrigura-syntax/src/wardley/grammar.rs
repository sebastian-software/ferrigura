//! The parser of the wardley grammar, `packages/parser/src/language/wardley/wardley.langium`:
//! recursive descent over the tokens of `scanner.rs`, one function per rule. The rules and the
//! order of their alternatives are in `docs-port/wardley/NOTES.md`, section 2.3.
//!
//! The entry is `NEWLINE* KW_WARDLEY (NEWLINE | TitleAndAccessibilities | Statement)*`, and every
//! statement ends with `EOL`: one or more line breaks, or the end of the text. The header values
//! are the last ones given, as the grammar assigns them. Lists keep the text order.

use ferrigura_model::wardley::Strategy;

use super::scanner::{Kw, Spanned, Token, tokenize};
use crate::error::{ParseError, Source};

/// The statements of a wardley map, as the grammar gives them.
#[derive(Debug, Default)]
pub(crate) struct Ast {
    /// The last `title` line, `""` when there is none or its value is empty.
    pub title: String,
    /// The last `accTitle` line.
    pub acc_title: String,
    /// The last `accDescr` line.
    pub acc_descr: String,
    /// The last `size` statement.
    pub size: Option<Size>,
    /// The last `evolution` statement.
    pub evolution: Option<Evolution>,
    pub anchors: Vec<Anchor>,
    pub components: Vec<Component>,
    pub links: Vec<Link>,
    pub evolves: Vec<Evolve>,
    pub pipelines: Vec<Pipeline>,
    pub notes: Vec<Note>,
    /// Every `annotations` statement; the DB reads the first.
    pub annotations: Vec<Coordinates>,
    pub annotation: Vec<Annotation>,
    pub accelerators: Vec<Mark>,
    pub deaccelerators: Vec<Mark>,
}

/// `size [INT, INT]`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Size {
    pub width: f64,
    pub height: f64,
}

/// `evolution Stage (-> Stage)+`.
#[derive(Debug)]
pub(crate) struct Evolution {
    pub stages: Vec<Stage>,
}

/// `name ('@' boundary)? ('/' secondName)?`.
#[derive(Debug)]
pub(crate) struct Stage {
    pub name: String,
    pub boundary: Option<f64>,
    pub second_name: Option<String>,
}

/// `anchor name [visibility, evolution]`.
#[derive(Debug)]
pub(crate) struct Anchor {
    pub name: String,
    pub visibility: f64,
    pub evolution: f64,
    pub offset: usize,
}

/// `component name [visibility, evolution] label? decorator? inertia?`.
#[derive(Debug)]
pub(crate) struct Component {
    pub name: String,
    pub visibility: f64,
    pub evolution: f64,
    pub label: Option<Label>,
    /// The strategy of the decorator.
    pub decorator: Option<Strategy>,
    /// Whether the word `inertia` (bare or in parentheses) is there.
    pub inertia: bool,
    pub offset: usize,
}

/// `label [-? offsetX, -? offsetY]`: the signs are flags, the offsets are the integers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Label {
    pub neg_x: bool,
    pub offset_x: f64,
    pub neg_y: bool,
    pub offset_y: f64,
}

/// `from fromPort? arrow? to toPort? linkLabel?`.
#[derive(Debug)]
pub(crate) struct Link {
    pub from: String,
    pub from_port: Option<&'static str>,
    /// The arrow's text: `->`, `-->`, `-.->`, `>` or a `+'label'` form.
    pub arrow: Option<String>,
    pub to: String,
    pub to_port: Option<&'static str>,
    /// The `;` label, after the converter.
    pub link_label: Option<String>,
}

/// `evolve component target`.
#[derive(Debug)]
pub(crate) struct Evolve {
    pub component: String,
    pub target: f64,
    pub offset: usize,
}

/// `pipeline parent { components }`.
#[derive(Debug)]
pub(crate) struct Pipeline {
    pub parent: String,
    pub components: Vec<PipelineComponent>,
    pub offset: usize,
}

/// `component name [evolution] label?`, inside a pipeline.
#[derive(Debug)]
pub(crate) struct PipelineComponent {
    pub name: String,
    pub evolution: f64,
    pub label: Option<Label>,
    pub offset: usize,
}

/// `note "text" [visibility, evolution]`.
#[derive(Debug)]
pub(crate) struct Note {
    pub text: String,
    pub visibility: f64,
    pub evolution: f64,
    pub offset: usize,
}

/// A coordinate pair of `annotations` or `annotation`: the two numbers as the text gives them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Coordinates {
    pub x: f64,
    pub y: f64,
    pub offset: usize,
}

/// `annotation number, [x, y] text`.
#[derive(Debug)]
pub(crate) struct Annotation {
    pub number: f64,
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub offset: usize,
}

/// `accelerator name [x, y]` and `deaccelerator name [x, y]`.
#[derive(Debug)]
pub(crate) struct Mark {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub offset: usize,
}

/// Parses the prepared text of a wardley map into its statements.
pub(crate) fn parse_text(source: Source<'_>) -> Result<Ast, ParseError> {
    let tokens = tokenize(source.text())
        .map_err(|failure| source.error(failure.offset, "this character does not start a token"))?;
    Parser {
        source,
        tokens,
        pos: 0,
    }
    .document()
}

struct Parser<'a> {
    source: Source<'a>,
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|spanned| &spanned.token)
    }

    fn peek_at(&self, ahead: usize) -> Option<&Token> {
        self.tokens
            .get(self.pos + ahead)
            .map(|spanned| &spanned.token)
    }

    /// The byte offset of the current token, or the end of the text.
    fn offset(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |spanned| spanned.start)
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        self.source.error(self.offset(), message)
    }

    fn is(&self, token: &Token) -> bool {
        self.peek() == Some(token)
    }

    /// Takes the current token when it is `token`.
    fn eat(&mut self, token: &Token) -> bool {
        if self.is(token) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &Token, what: &str) -> Result<(), ParseError> {
        if self.eat(token) {
            Ok(())
        } else {
            Err(self.error(format!("expected {what}")))
        }
    }

    fn expect_punct(&mut self, punct: char) -> Result<(), ParseError> {
        self.expect(&Token::Punct(punct), &format!("'{punct}'"))
    }

    fn expect_kw(&mut self, kw: Kw, what: &str) -> Result<(), ParseError> {
        self.expect(&Token::Kw(kw), what)
    }

    fn skip_newlines(&mut self) {
        while self.eat(&Token::Newline) {}
    }

    /// `EOL: NEWLINE+ | EOF`.
    fn eol(&mut self) -> Result<(), ParseError> {
        if self.peek().is_none() {
            return Ok(());
        }
        if !self.is(&Token::Newline) {
            return Err(self.error("expected the end of the line"));
        }
        self.skip_newlines();
        Ok(())
    }

    /// `STRING | ID | NAME_WITH_SPACES`: the name of a node, as its text.
    fn name(&mut self) -> Result<String, ParseError> {
        let text = match self.peek() {
            Some(Token::Name(text) | Token::Id(text) | Token::Str(text)) => text.clone(),
            _ => return Err(self.error("expected a name")),
        };
        self.pos += 1;
        Ok(text)
    }

    fn string(&mut self) -> Result<String, ParseError> {
        let text = match self.peek() {
            Some(Token::Str(text)) => text.clone(),
            _ => return Err(self.error("expected a quoted text")),
        };
        self.pos += 1;
        Ok(text)
    }

    /// `WARDLEY_NUMBER`: a decimal.
    fn decimal(&mut self) -> Result<f64, ParseError> {
        let value = match self.peek() {
            Some(Token::Decimal(value)) => *value,
            _ => return Err(self.error("expected a decimal number")),
        };
        self.pos += 1;
        Ok(value)
    }

    /// `INT`: an integer.
    fn integer(&mut self) -> Result<f64, ParseError> {
        let value = match self.peek() {
            Some(Token::Integer(value)) => *value,
            _ => return Err(self.error("expected an integer")),
        };
        self.pos += 1;
        Ok(value)
    }

    /// `CoordinateValue returns number: WARDLEY_NUMBER | INT`.
    fn coordinate(&mut self) -> Result<f64, ParseError> {
        match self.peek() {
            Some(Token::Decimal(value) | Token::Integer(value)) => {
                let value = *value;
                self.pos += 1;
                Ok(value)
            }
            _ => Err(self.error("expected a number")),
        }
    }

    /// `[` x `,` y `]`, both decimals.
    fn decimal_pair(&mut self) -> Result<(f64, f64), ParseError> {
        self.expect_punct('[')?;
        let first = self.decimal()?;
        self.expect_punct(',')?;
        let second = self.decimal()?;
        self.expect_punct(']')?;
        Ok((first, second))
    }

    /// `Entry: NEWLINE* KW_WARDLEY (NEWLINE | TitleAndAccessibilities | Statement)*`.
    fn document(mut self) -> Result<Ast, ParseError> {
        let mut ast = Ast::default();
        self.skip_newlines();
        self.expect_kw(Kw::Wardley, "the header 'wardley-beta'")?;
        while let Some(token) = self.peek().cloned() {
            match token {
                Token::Newline => self.pos += 1,
                Token::Title(text) => {
                    self.pos += 1;
                    self.eol()?;
                    ast.title = text;
                }
                Token::AccTitle(text) => {
                    self.pos += 1;
                    self.eol()?;
                    ast.acc_title = text;
                }
                Token::AccDescr(text) => {
                    self.pos += 1;
                    self.eol()?;
                    ast.acc_descr = text;
                }
                Token::Kw(kw) => self.statement(kw, &mut ast)?,
                Token::Name(_) | Token::Id(_) | Token::Str(_) => {
                    let link = self.link()?;
                    ast.links.push(link);
                }
                _ => return Err(self.error("expected a statement")),
            }
        }
        Ok(ast)
    }

    /// `Statement`, dispatched on its keyword. The keywords that start no statement are errors.
    fn statement(&mut self, kw: Kw, ast: &mut Ast) -> Result<(), ParseError> {
        let offset = self.offset();
        match kw {
            Kw::Size => {
                self.pos += 1;
                self.expect_punct('[')?;
                let width = self.integer()?;
                self.expect_punct(',')?;
                let height = self.integer()?;
                self.expect_punct(']')?;
                self.eol()?;
                ast.size = Some(Size { width, height });
            }
            Kw::Evolution => {
                self.pos += 1;
                let mut stages = vec![self.stage()?];
                self.expect(&Token::Arrow, "'->'")?;
                stages.push(self.stage()?);
                while self.eat(&Token::Arrow) {
                    stages.push(self.stage()?);
                }
                self.eol()?;
                ast.evolution = Some(Evolution { stages });
            }
            Kw::Anchor => {
                self.pos += 1;
                let name = self.name()?;
                let (visibility, evolution) = self.decimal_pair()?;
                self.eol()?;
                ast.anchors.push(Anchor {
                    name,
                    visibility,
                    evolution,
                    offset,
                });
            }
            Kw::Component => {
                let component = self.component()?;
                self.eol()?;
                ast.components.push(component);
            }
            Kw::Evolve => {
                self.pos += 1;
                let component = self.name()?;
                let target = self.decimal()?;
                self.eol()?;
                ast.evolves.push(Evolve {
                    component,
                    target,
                    offset,
                });
            }
            Kw::Pipeline => {
                let pipeline = self.pipeline()?;
                ast.pipelines.push(pipeline);
            }
            Kw::Note => {
                self.pos += 1;
                let text = self.string()?;
                let (visibility, evolution) = self.decimal_pair()?;
                self.eol()?;
                ast.notes.push(Note {
                    text,
                    visibility,
                    evolution,
                    offset,
                });
            }
            Kw::Annotations => {
                self.pos += 1;
                self.expect_punct('[')?;
                let x = self.coordinate()?;
                self.expect_punct(',')?;
                let y = self.coordinate()?;
                self.expect_punct(']')?;
                self.eol()?;
                ast.annotations.push(Coordinates { x, y, offset });
            }
            Kw::Annotation => {
                self.pos += 1;
                let number = self.integer()?;
                self.expect_punct(',')?;
                self.expect_punct('[')?;
                let x = self.coordinate()?;
                self.expect_punct(',')?;
                let y = self.coordinate()?;
                self.expect_punct(']')?;
                let text = self.string()?;
                self.eol()?;
                ast.annotation.push(Annotation {
                    number,
                    x,
                    y,
                    text,
                    offset,
                });
            }
            Kw::Accelerator | Kw::Deaccelerator => {
                self.pos += 1;
                let name = self.name()?;
                let (x, y) = self.decimal_pair()?;
                self.eol()?;
                let mark = Mark { name, x, y, offset };
                if kw == Kw::Accelerator {
                    ast.accelerators.push(mark);
                } else {
                    ast.deaccelerators.push(mark);
                }
            }
            Kw::Wardley | Kw::Label | Kw::Inertia => {
                return Err(self.error("this keyword does not start a statement"));
            }
        }
        Ok(())
    }

    /// `EvolutionStage`: the name, the boundary after `@`, and the second name after `/`.
    fn stage(&mut self) -> Result<Stage, ParseError> {
        let name = self.name()?;
        let boundary = if self.eat(&Token::Punct('@')) {
            Some(self.decimal()?)
        } else {
            None
        };
        let second_name = if self.eat(&Token::Punct('/')) {
            Some(self.name()?)
        } else {
            None
        };
        Ok(Stage {
            name,
            boundary,
            second_name,
        })
    }

    /// `Component` without its `EOL`: the label, the decorator and the inertia mark, in that order.
    fn component(&mut self) -> Result<Component, ParseError> {
        let offset = self.offset();
        self.expect_kw(Kw::Component, "'component'")?;
        let name = self.name()?;
        let (visibility, evolution) = self.decimal_pair()?;
        let label = if self.is(&Token::Kw(Kw::Label)) {
            Some(self.label()?)
        } else {
            None
        };
        let decorator =
            if self.is(&Token::Punct('(')) && matches!(self.peek_at(1), Some(Token::Strategy(_))) {
                self.pos += 1;
                let Some(Token::Strategy(strategy)) = self.peek().cloned() else {
                    return Err(self.error("expected a strategy"));
                };
                self.pos += 1;
                self.expect_punct(')')?;
                Some(strategy)
            } else {
                None
            };
        let inertia = if self.eat(&Token::Kw(Kw::Inertia)) {
            true
        } else if self.is(&Token::Punct('(')) && self.peek_at(1) == Some(&Token::Kw(Kw::Inertia)) {
            self.pos += 2;
            self.expect_punct(')')?;
            true
        } else {
            false
        };
        Ok(Component {
            name,
            visibility,
            evolution,
            label,
            decorator,
            inertia,
            offset,
        })
    }

    /// `Label: KW_LABEL '[' negX? offsetX ',' negY? offsetY ']'`.
    fn label(&mut self) -> Result<Label, ParseError> {
        self.expect_kw(Kw::Label, "'label'")?;
        self.expect_punct('[')?;
        let neg_x = self.eat(&Token::Punct('-'));
        let offset_x = self.integer()?;
        self.expect_punct(',')?;
        let neg_y = self.eat(&Token::Punct('-'));
        let offset_y = self.integer()?;
        self.expect_punct(']')?;
        Ok(Label {
            neg_x,
            offset_x,
            neg_y,
            offset_y,
        })
    }

    /// `Link: from fromPort? arrow? to toPort? linkLabel? EOL`.
    fn link(&mut self) -> Result<Link, ParseError> {
        let from = self.name()?;
        let from_port = self.link_port();
        let arrow = match self.peek().cloned() {
            Some(Token::LinkArrow(text)) => {
                self.pos += 1;
                Some(text)
            }
            Some(Token::Arrow) => {
                self.pos += 1;
                Some("->".to_owned())
            }
            _ => None,
        };
        let to = self.name()?;
        let to_port = self.link_port();
        let link_label = match self.peek().cloned() {
            Some(Token::LinkLabel(text)) => {
                self.pos += 1;
                Some(text)
            }
            _ => None,
        };
        self.eol()?;
        Ok(Link {
            from,
            from_port,
            arrow,
            to,
            to_port,
            link_label,
        })
    }

    fn link_port(&mut self) -> Option<&'static str> {
        match self.peek() {
            Some(Token::LinkPort(port)) => {
                let port = *port;
                self.pos += 1;
                Some(port)
            }
            _ => None,
        }
    }

    /// `Pipeline: KW_PIPELINE parent '{' NEWLINE+ PipelineComponent+ '}' EOL`.
    fn pipeline(&mut self) -> Result<Pipeline, ParseError> {
        let offset = self.offset();
        self.expect_kw(Kw::Pipeline, "'pipeline'")?;
        let parent = self.name()?;
        self.expect_punct('{')?;
        if !self.is(&Token::Newline) {
            return Err(self.error("expected a line break after '{'"));
        }
        self.skip_newlines();
        let mut components = Vec::new();
        while self.is(&Token::Kw(Kw::Component)) {
            components.push(self.pipeline_component()?);
        }
        if components.is_empty() {
            return Err(self.error("expected a component of the pipeline"));
        }
        self.expect_punct('}')?;
        self.eol()?;
        Ok(Pipeline {
            parent,
            components,
            offset,
        })
    }

    /// `PipelineComponent: KW_COMPONENT name '[' evolution ']' label? EOL`.
    fn pipeline_component(&mut self) -> Result<PipelineComponent, ParseError> {
        let offset = self.offset();
        self.expect_kw(Kw::Component, "'component'")?;
        let name = self.name()?;
        self.expect_punct('[')?;
        let evolution = self.decimal()?;
        self.expect_punct(']')?;
        let label = if self.is(&Token::Kw(Kw::Label)) {
            Some(self.label()?)
        } else {
            None
        };
        self.eol()?;
        Ok(PipelineComponent {
            name,
            evolution,
            label,
            offset,
        })
    }
}
