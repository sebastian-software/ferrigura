//! The four railroad grammars, read by recursive descent over the scanned tokens, as the Langium
//! grammars of `packages/parser/src/language/railroad*` parse them. Each builds the expression of
//! the reference's `transform*` functions directly: one-element sequences and choices collapse, and
//! the postfixes apply in their order. The strings keep the converter's text; the DB sanitises them.
//! See `docs-port/railroad/PLAN.md`, section 3.

use ferrigura_model::railroad::{RailroadExpr, RailroadRule};

use super::scanner::{Spanned, Sym, Syntax, Token, Word};
use crate::error::{ParseError, Source};

/// The header values (the last line of each kind wins, even when it is empty) and the rules.
#[derive(Debug, Default)]
pub(crate) struct Document {
    pub(crate) title: String,
    pub(crate) acc_title: String,
    pub(crate) acc_descr: String,
    pub(crate) rules: Vec<RailroadRule>,
}

/// Parses the tokens of one syntax: the header, the header lines, then the rules.
pub(crate) fn document(
    syntax: Syntax,
    tokens: &[Spanned],
    source: Source<'_>,
) -> Result<Document, ParseError> {
    let mut reader = Reader {
        tokens,
        at: 0,
        source,
    };
    reader.expect(&Token::Header, &format!("the '{}' header", syntax.header()))?;

    let mut document = Document::default();
    while let Some(line) = reader.peek() {
        match line {
            Token::Title(text) => document.title = text.clone(),
            Token::AccTitle(text) => document.acc_title = text.clone(),
            Token::AccDescr(text) => document.acc_descr = text.clone(),
            _ => break,
        }
        reader.advance();
    }
    while reader.peek().is_some() {
        let rule = match syntax {
            Syntax::Ir => ir_rule(&mut reader)?,
            Syntax::Ebnf => ebnf_rule(&mut reader)?,
            Syntax::Abnf => abnf_rule(&mut reader)?,
            Syntax::Peg => peg_rule(&mut reader)?,
        };
        document.rules.push(rule);
    }
    Ok(document)
}

/// The token cursor of the grammars.
struct Reader<'a> {
    tokens: &'a [Spanned],
    at: usize,
    source: Source<'a>,
}

impl Reader<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at).map(|spanned| &spanned.token)
    }

    fn advance(&mut self) {
        self.at += 1;
    }

    /// Consumes the next token if it is `sym`.
    fn eat(&mut self, sym: Sym) -> bool {
        let found = self.peek() == Some(&Token::Sym(sym));
        if found {
            self.advance();
        }
        found
    }

    fn expect_sym(&mut self, sym: Sym, what: &str) -> Result<(), ParseError> {
        if self.eat(sym) {
            Ok(())
        } else {
            Err(self.error(format!("expected {what}")))
        }
    }

    fn expect(&mut self, token: &Token, what: &str) -> Result<(), ParseError> {
        if self.peek() == Some(token) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected {what}")))
        }
    }

    /// The text of the next rule name.
    fn name(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            Some(Token::Id(name)) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(self.error("expected a rule name")),
        }
    }

    /// The value of the next string.
    fn string(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            Some(Token::Str(value)) => {
                let value = value.clone();
                self.advance();
                Ok(value)
            }
            _ => Err(self.error("expected a string")),
        }
    }

    /// An error at the next token, or at the end of the text.
    fn error(&self, message: impl Into<String>) -> ParseError {
        let offset = self
            .tokens
            .get(self.at)
            .map_or(self.source.text().len(), |spanned| spanned.start);
        self.source.error(offset, message)
    }
}

fn rule(name: String, definition: RailroadExpr) -> RailroadRule {
    RailroadRule { name, definition }
}

/// A choice of its alternatives, or the alternative itself when there is one.
fn choice(mut alternatives: Vec<RailroadExpr>) -> RailroadExpr {
    if alternatives.len() == 1 {
        return alternatives.remove(0);
    }
    RailroadExpr::Choice { alternatives }
}

/// A sequence of its elements, or the element itself when there is one.
fn sequence(mut elements: Vec<RailroadExpr>) -> RailroadExpr {
    if elements.len() == 1 {
        return elements.remove(0);
    }
    RailroadExpr::Sequence { elements }
}

fn optional(element: RailroadExpr) -> RailroadExpr {
    RailroadExpr::Optional {
        element: Box::new(element),
    }
}

fn repetition(element: RailroadExpr, min: f64, max: f64) -> RailroadExpr {
    RailroadExpr::Repetition {
        element: Box::new(element),
        min,
        max,
    }
}

// The IR grammar: `RailroadRule: RR_ID '=' expr ';'`, with the forms `sequence(…)`, `choice(…)`,
// `optional(e)`, `oneOrMore(e)`, `zeroOrMore(e)`, `terminal(s)`, `nonterminal(s)` and `special(s)`.

fn ir_rule(reader: &mut Reader<'_>) -> Result<RailroadRule, ParseError> {
    let name = reader.name()?;
    reader.expect_sym(Sym::Eq, "'='")?;
    let definition = ir_expr(reader)?;
    reader.expect_sym(Sym::Semi, "';'")?;
    Ok(rule(name, definition))
}

fn ir_expr(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let Some(Token::Word(word)) = reader.peek() else {
        return Err(reader.error("expected an expression"));
    };
    let word = *word;
    reader.advance();
    reader.expect_sym(Sym::LParen, "'('")?;
    let expr = match word {
        Word::Sequence => sequence(ir_list(reader)?),
        Word::Choice => choice(ir_list(reader)?),
        Word::Optional => optional(ir_expr(reader)?),
        Word::OneOrMore => repetition(ir_expr(reader)?, 1.0, f64::INFINITY),
        Word::ZeroOrMore => repetition(ir_expr(reader)?, 0.0, f64::INFINITY),
        Word::Terminal => RailroadExpr::Terminal {
            value: reader.string()?,
        },
        Word::Nonterminal => RailroadExpr::Nonterminal {
            name: reader.string()?,
        },
        Word::Special => RailroadExpr::Special {
            text: reader.string()?,
        },
    };
    reader.expect_sym(Sym::RParen, "')'")?;
    Ok(expr)
}

/// `e (',' e)*`: one or more expressions, separated by commas.
fn ir_list(reader: &mut Reader<'_>) -> Result<Vec<RailroadExpr>, ParseError> {
    let mut items = vec![ir_expr(reader)?];
    while reader.eat(Sym::Comma) {
        items.push(ir_expr(reader)?);
    }
    Ok(items)
}

// The EBNF grammar: `EbnfRule: EBNF_ID ('=' | '::=') choice ';'`, where a sequence may have commas
// between its terms, and a term has postfixes `?`, `*`, `+` and `- primary`.

fn ebnf_rule(reader: &mut Reader<'_>) -> Result<RailroadRule, ParseError> {
    let name = reader.name()?;
    if !(reader.eat(Sym::Eq) || reader.eat(Sym::ColonEq)) {
        return Err(reader.error("expected '=' or '::='"));
    }
    let definition = ebnf_choice(reader)?;
    reader.expect_sym(Sym::Semi, "';'")?;
    Ok(rule(name, definition))
}

fn ebnf_choice(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let mut alternatives = vec![ebnf_sequence(reader)?];
    while reader.eat(Sym::Alt) {
        alternatives.push(ebnf_sequence(reader)?);
    }
    Ok(choice(alternatives))
}

/// `EbnfTerm (','? EbnfTerm)*`: a comma with no term after it is an error.
fn ebnf_sequence(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let mut elements = vec![ebnf_term(reader)?];
    loop {
        let comma = reader.eat(Sym::Comma);
        if starts_ebnf_term(reader) {
            elements.push(ebnf_term(reader)?);
        } else if comma {
            return Err(reader.error("expected a term after ','"));
        } else {
            break;
        }
    }
    Ok(sequence(elements))
}

fn starts_ebnf_term(reader: &Reader<'_>) -> bool {
    match reader.peek() {
        Some(Token::Str(_) | Token::SpecialSeq(_) | Token::Id(_)) => true,
        Some(Token::Sym(sym)) => matches!(sym, Sym::LParen | Sym::LBracket | Sym::LBrace),
        _ => false,
    }
}

fn ebnf_term(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let mut node = ebnf_primary(reader)?;
    loop {
        if reader.eat(Sym::Question) {
            node = optional(node);
        } else if reader.eat(Sym::Star) {
            node = repetition(node, 0.0, f64::INFINITY);
        } else if reader.eat(Sym::Plus) {
            node = repetition(node, 1.0, f64::INFINITY);
        } else if reader.eat(Sym::Minus) {
            let except = ebnf_primary(reader)?;
            node = RailroadExpr::Sequence {
                elements: vec![
                    node,
                    RailroadExpr::Terminal {
                        value: "-".to_owned(),
                    },
                    except,
                ],
            };
        } else {
            return Ok(node);
        }
    }
}

fn ebnf_primary(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    match reader.peek().cloned() {
        Some(Token::Str(value)) => {
            reader.advance();
            Ok(RailroadExpr::Terminal { value })
        }
        Some(Token::SpecialSeq(text)) => {
            reader.advance();
            Ok(RailroadExpr::Special { text })
        }
        Some(Token::Id(name)) => {
            reader.advance();
            Ok(RailroadExpr::Nonterminal { name })
        }
        Some(Token::Sym(Sym::LParen)) => {
            reader.advance();
            let group = ebnf_choice(reader)?;
            reader.expect_sym(Sym::RParen, "')'")?;
            Ok(group)
        }
        Some(Token::Sym(Sym::LBracket)) => {
            reader.advance();
            let group = ebnf_choice(reader)?;
            reader.expect_sym(Sym::RBracket, "']'")?;
            Ok(optional(group))
        }
        Some(Token::Sym(Sym::LBrace)) => {
            reader.advance();
            let group = ebnf_choice(reader)?;
            reader.expect_sym(Sym::RBrace, "'}'")?;
            Ok(repetition(group, 0.0, f64::INFINITY))
        }
        _ => Err(reader.error("expected a term")),
    }
}

// The ABNF grammar: `AbnfRule: ABNF_RULENAME '=' alternation ';'`. An element may start with a
// repeat, which is `*`, `n*m` or a count, and the bounds decide the form (`0*1` is an optional).

fn abnf_rule(reader: &mut Reader<'_>) -> Result<RailroadRule, ParseError> {
    let name = reader.name()?;
    reader.expect_sym(Sym::Eq, "'='")?;
    let definition = abnf_alternation(reader)?;
    reader.expect_sym(Sym::Semi, "';'")?;
    Ok(rule(name, definition))
}

fn abnf_alternation(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let mut alternatives = vec![abnf_concatenation(reader)?];
    while reader.eat(Sym::Alt) {
        alternatives.push(abnf_concatenation(reader)?);
    }
    Ok(choice(alternatives))
}

/// `AbnfElement+`: at least one element.
fn abnf_concatenation(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let mut elements = vec![abnf_element(reader)?];
    while starts_abnf_element(reader) {
        elements.push(abnf_element(reader)?);
    }
    Ok(sequence(elements))
}

fn starts_abnf_element(reader: &Reader<'_>) -> bool {
    match reader.peek() {
        Some(Token::Repeat(_) | Token::Str(_) | Token::NumVal(_) | Token::Id(_)) => true,
        Some(Token::Sym(sym)) => matches!(sym, Sym::LParen | Sym::LBracket),
        _ => false,
    }
}

fn abnf_element(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let repeat = match reader.peek() {
        Some(Token::Repeat(text)) => {
            let text = text.clone();
            reader.advance();
            Some(text)
        }
        _ => None,
    };
    let inner = abnf_primary(reader)?;
    let Some(text) = repeat else {
        return Ok(inner);
    };
    let (min, max) = repeat_bounds(&text);
    if min == 0.0 && max == 1.0 {
        return Ok(optional(inner));
    }
    Ok(repetition(inner, min, max))
}

fn abnf_primary(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    match reader.peek().cloned() {
        Some(Token::Str(value) | Token::NumVal(value)) => {
            reader.advance();
            Ok(RailroadExpr::Terminal { value })
        }
        Some(Token::Id(name)) => {
            reader.advance();
            Ok(RailroadExpr::Nonterminal { name })
        }
        Some(Token::Sym(Sym::LParen)) => {
            reader.advance();
            let group = abnf_alternation(reader)?;
            reader.expect_sym(Sym::RParen, "')'")?;
            Ok(group)
        }
        Some(Token::Sym(Sym::LBracket)) => {
            reader.advance();
            let group = abnf_alternation(reader)?;
            reader.expect_sym(Sym::RBracket, "']'")?;
            Ok(optional(group))
        }
        _ => Err(reader.error("expected an element")),
    }
}

/// The bounds of an ABNF repeat, as the reference's `parseRepeat` reads them: `a*b` with an empty
/// `a` at 0 and an empty `b` at infinity, and a count with no `*` as both bounds.
fn repeat_bounds(text: &str) -> (f64, f64) {
    match text.split_once('*') {
        Some((least, most)) => {
            let min = if least.is_empty() { 0.0 } else { digits(least) };
            let max = if most.is_empty() {
                f64::INFINITY
            } else {
                digits(most)
            };
            (min, max)
        }
        None => {
            let count = digits(text);
            (count, count)
        }
    }
}

/// A run of decimal digits as a number, as `parseInt` reads it.
fn digits(text: &str) -> f64 {
    text.parse::<f64>().unwrap_or(f64::NAN)
}

// The PEG grammar: `PegRule: PEG_ID '<-' ordered ';'`. A prefix `&` or `!` on an expression is a
// special node whose text is the operator and a label of the expression.

fn peg_rule(reader: &mut Reader<'_>) -> Result<RailroadRule, ParseError> {
    let name = reader.name()?;
    reader.expect_sym(Sym::Arrow, "'<-'")?;
    let definition = peg_ordered(reader)?;
    reader.expect_sym(Sym::Semi, "';'")?;
    Ok(rule(name, definition))
}

fn peg_ordered(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let mut alternatives = vec![peg_sequence(reader)?];
    while reader.eat(Sym::Alt) {
        alternatives.push(peg_sequence(reader)?);
    }
    Ok(choice(alternatives))
}

/// `PegPrefix+`: at least one prefix.
fn peg_sequence(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let mut elements = vec![peg_prefix(reader)?];
    while starts_peg_prefix(reader) {
        elements.push(peg_prefix(reader)?);
    }
    Ok(sequence(elements))
}

fn starts_peg_prefix(reader: &Reader<'_>) -> bool {
    match reader.peek() {
        Some(Token::Str(_) | Token::Id(_)) => true,
        Some(Token::Sym(sym)) => matches!(sym, Sym::Amp | Sym::Bang | Sym::LParen | Sym::Dot),
        _ => false,
    }
}

fn peg_prefix(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let operator = if reader.eat(Sym::Amp) {
        Some('&')
    } else if reader.eat(Sym::Bang) {
        Some('!')
    } else {
        None
    };
    let inner = peg_suffix(reader)?;
    Ok(match operator {
        Some(operator) => RailroadExpr::Special {
            text: format!("{operator}{}", peg_label(&inner)),
        },
        None => inner,
    })
}

/// The label of a lookahead operand: a quoted terminal, a rule name, a special text, or `(...)`
/// for any other expression.
fn peg_label(node: &RailroadExpr) -> String {
    match node {
        RailroadExpr::Terminal { value } => format!("\"{value}\""),
        RailroadExpr::Nonterminal { name } => name.clone(),
        RailroadExpr::Special { text } => text.clone(),
        _ => "(...)".to_owned(),
    }
}

/// `PegPrimary ('?' | '*' | '+')?`: one suffix at most.
fn peg_suffix(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    let inner = peg_primary(reader)?;
    if reader.eat(Sym::Question) {
        return Ok(optional(inner));
    }
    if reader.eat(Sym::Star) {
        return Ok(repetition(inner, 0.0, f64::INFINITY));
    }
    if reader.eat(Sym::Plus) {
        return Ok(repetition(inner, 1.0, f64::INFINITY));
    }
    Ok(inner)
}

fn peg_primary(reader: &mut Reader<'_>) -> Result<RailroadExpr, ParseError> {
    match reader.peek().cloned() {
        Some(Token::Str(value)) => {
            reader.advance();
            Ok(RailroadExpr::Terminal { value })
        }
        Some(Token::Id(name)) => {
            reader.advance();
            Ok(RailroadExpr::Nonterminal { name })
        }
        Some(Token::Sym(Sym::LParen)) => {
            reader.advance();
            let group = peg_ordered(reader)?;
            reader.expect_sym(Sym::RParen, "')'")?;
            Ok(group)
        }
        Some(Token::Sym(Sym::Dot)) => {
            reader.advance();
            Ok(RailroadExpr::Special {
                text: ".".to_owned(),
            })
        }
        _ => Err(reader.error("expected an expression")),
    }
}
