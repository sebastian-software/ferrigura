//! The ishikawa DB, `ishikawaDb.ts`, with the title of the common DB (`commonDb.ts`).
//!
//! The DB keeps a tree of causes. Each statement after the first is placed by its indentation: the
//! first cause fixes the base level, and a statement goes under the nearest open node of a lower
//! level. See `docs-port/ishikawa/PLAN.md`, section 3.

use ferrigura_model::ishikawa::{IshikawaDiagram, IshikawaNode};

use crate::sanitize::sanitize_text;

/// A node as the DB holds it: its text, and the indexes of its children in the arena.
#[derive(Debug)]
struct RawNode {
    text: String,
    children: Vec<usize>,
}

/// The state the ishikawa grammar builds. The arena's first node is the head, when there is one.
#[derive(Debug, Default)]
pub(crate) struct IshikawaDb {
    /// The common DB's diagram title, sanitised.
    title: String,
    /// The nodes in the order they were added.
    nodes: Vec<RawNode>,
    /// The open nodes, each with its level, from the head (level 0) down to the last cause.
    stack: Vec<(i64, usize)>,
    /// The level of the first cause, set when the second node is added.
    base_level: Option<i64>,
}

impl IshikawaDb {
    /// `setDiagramTitle`: sanitised, and it replaces the title given before.
    pub(crate) fn set_diagram_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `addNode(rawLevel, text)`: the first node is the head and sets the title to its own text,
    /// which the common DB sanitises once more. Each later node is placed by its level.
    pub(crate) fn add_node(&mut self, raw_level: i64, text: &str) {
        let label = sanitize_text(text);
        if self.nodes.is_empty() {
            self.nodes.push(RawNode {
                text: label.clone(),
                children: Vec::new(),
            });
            self.stack = vec![(0, 0)];
            self.set_diagram_title(&label);
            return;
        }

        // `baseLevel ??= rawLevel`: the effect's own indentation does not count.
        let base = *self.base_level.get_or_insert(raw_level);
        let mut level = raw_level - base + 1;
        if level <= 0 {
            level = 1;
        }

        // Pop while the top is not lower than the new node. The head is never popped.
        while self.stack.len() > 1 && self.stack.last().is_some_and(|&(top, _)| top >= level) {
            self.stack.pop();
        }
        let parent = self.stack.last().map_or(0, |&(_, index)| index);
        let index = self.nodes.len();
        self.nodes.push(RawNode {
            text: label,
            children: Vec::new(),
        });
        if let Some(node) = self.nodes.get_mut(parent) {
            node.children.push(index);
        }
        self.stack.push((level, index));
    }

    /// The model: the title, the accessibility texts (always empty, since no rule sets them), and
    /// the tree of the head.
    pub(crate) fn into_model(self) -> IshikawaDiagram {
        let mut model = IshikawaDiagram::new();
        model.title = self.title;
        model.root = build(&self.nodes, 0);
        model
    }
}

/// The tree below the node at `index`, or `None` when the arena has no such node.
fn build(nodes: &[RawNode], index: usize) -> Option<IshikawaNode> {
    let node = nodes.get(index)?;
    let children = node
        .children
        .iter()
        .filter_map(|&child| build(nodes, child))
        .collect();
    Some(IshikawaNode {
        text: node.text.clone(),
        children,
    })
}
