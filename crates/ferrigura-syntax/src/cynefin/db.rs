//! The cynefin DB, `packages/mermaid/src/diagrams/cynefin/cynefinDb.ts`: the state the parser's
//! calls change, and the model it gives. The order of the calls is in [`super::populate`], and the
//! quirks are in `docs-port/cynefin/NOTES.md`, sections 3.2 and 3.4.
//!
//! The reference's `Map` keeps the first position of a key that is set again, and `IndexMap`
//! does the same. The header texts go through the sanitiser and the whitespace rules of the common
//! DB (`commonDb.ts`); the domain items and the labels are kept as the grammar gives them.

use ferrigura_model::CANONICAL_VERSION;
use ferrigura_model::cynefin::{CynefinDiagram, CynefinDomain, CynefinTransition};
use indexmap::IndexMap;

use super::grammar::{DomainBlock, Transition};
use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// The state of the DB after a parse.
pub(crate) struct Db {
    title: String,
    acc_title: String,
    acc_descr: String,
    domains: IndexMap<&'static str, Vec<String>>,
    transitions: Vec<CynefinTransition>,
}

impl Db {
    /// An empty DB, as `clear` leaves it.
    pub(crate) fn new() -> Self {
        Self {
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
            domains: IndexMap::new(),
            transitions: Vec::new(),
        }
    }

    /// `setDiagramTitle`: the sanitised text.
    pub(crate) fn set_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle`: the sanitised text without its leading whitespace.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = strip_leading_space(&sanitize_text(text)).to_owned();
    }

    /// `setAccDescription`: the sanitised text, with the blanks after each line break removed.
    pub(crate) fn set_acc_descr(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(&sanitize_text(text));
    }

    /// `setDomains`: one entry per block. A block whose name is already there replaces the items
    /// and keeps the entry's place.
    pub(crate) fn set_domains(&mut self, blocks: &[DomainBlock]) {
        for block in blocks {
            self.domains.insert(block.name, block.items.clone());
        }
    }

    /// `setTransitions`: the self-loops are dropped, and an empty label is no label (the
    /// reference's `t.label || undefined`). The declaration order is kept.
    pub(crate) fn set_transitions(&mut self, transitions: &[Transition]) {
        self.transitions = transitions
            .iter()
            .filter(|t| t.from != t.to)
            .map(|t| CynefinTransition {
                from: t.from.to_owned(),
                to: t.to.to_owned(),
                label: t.label.clone().filter(|label| !label.is_empty()),
            })
            .collect();
    }

    /// The model of the DB's state.
    pub(crate) fn into_model(self) -> CynefinDiagram {
        CynefinDiagram {
            version: CANONICAL_VERSION,
            title: self.title,
            acc_title: self.acc_title,
            acc_descr: self.acc_descr,
            domains: self
                .domains
                .into_iter()
                .map(|(name, items)| CynefinDomain {
                    name: name.to_owned(),
                    items,
                })
                .collect(),
            transitions: self.transitions,
        }
    }
}
