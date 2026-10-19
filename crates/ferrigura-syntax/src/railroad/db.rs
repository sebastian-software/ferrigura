//! The railroad DB, `railroadDb.ts`, with the setters of the common DB that the parsers call. The
//! four syntaxes share it. Every name and text is sanitised as the reference's `sanitizeText` does,
//! on the strict path (`crate::sanitize`); the accessibility texts take the whitespace rules of
//! their setters. See `docs-port/railroad/PLAN.md`, section 4.

use ferrigura_model::railroad::{RailroadDiagram, RailroadExpr, RailroadRule};

use super::grammar::Document;
use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// The DB's state: the title and the accessibility texts, and the rules in order.
#[derive(Debug, Default)]
struct RailroadDb {
    title: String,
    acc_title: String,
    acc_descr: String,
    rules: Vec<RailroadRule>,
}

/// Builds the model from the parsed document, as `populateCommonDb` and `populateDb` do: the
/// accessibility texts and the title are set only when they are non-empty, then the rules are added.
pub(crate) fn build(document: Document) -> RailroadDiagram {
    let mut db = RailroadDb::default();
    if !document.acc_descr.is_empty() {
        db.set_acc_descr(&document.acc_descr);
    }
    if !document.acc_title.is_empty() {
        db.set_acc_title(&document.acc_title);
    }
    if !document.title.is_empty() {
        db.set_title(&document.title);
    }
    for rule in document.rules {
        db.add_rule(rule);
    }
    db.into_model()
}

impl RailroadDb {
    /// `setTitle`: the sanitised text.
    fn set_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle`: the sanitised text without its leading whitespace.
    fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription`: the sanitised text with the whitespace after each line break removed.
    fn set_acc_descr(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `addRule`: the name and the definition are sanitised; a duplicate name is kept as a rule of
    /// its own (the reference only warns).
    fn add_rule(&mut self, rule: RailroadRule) {
        self.rules.push(RailroadRule {
            name: sanitize_text(&rule.name),
            definition: sanitize_expr(rule.definition),
        });
    }

    fn into_model(self) -> RailroadDiagram {
        let mut model = RailroadDiagram::new();
        model.title = self.title;
        model.acc_title = self.acc_title;
        model.acc_descr = self.acc_descr;
        model.rules = self.rules;
        model
    }
}

/// `sanitizeAstNode`: the terminal's value, the nonterminal's name and the special's text are
/// sanitised, and so is every child.
fn sanitize_expr(expr: RailroadExpr) -> RailroadExpr {
    match expr {
        RailroadExpr::Terminal { value } => RailroadExpr::Terminal {
            value: sanitize_text(&value),
        },
        RailroadExpr::Nonterminal { name } => RailroadExpr::Nonterminal {
            name: sanitize_text(&name),
        },
        RailroadExpr::Special { text } => RailroadExpr::Special {
            text: sanitize_text(&text),
        },
        RailroadExpr::Sequence { elements } => RailroadExpr::Sequence {
            elements: elements.into_iter().map(sanitize_expr).collect(),
        },
        RailroadExpr::Choice { alternatives } => RailroadExpr::Choice {
            alternatives: alternatives.into_iter().map(sanitize_expr).collect(),
        },
        RailroadExpr::Optional { element } => RailroadExpr::Optional {
            element: Box::new(sanitize_expr(*element)),
        },
        RailroadExpr::Repetition { element, min, max } => RailroadExpr::Repetition {
            element: Box::new(sanitize_expr(*element)),
            min,
            max,
        },
    }
}
