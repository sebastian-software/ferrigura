//! The treeView DB, `packages/mermaid/src/diagrams/treeView/db.ts`: a stack of nodes built from the
//! indentation level of each line, with the root `/` at the bottom and the common setters of the
//! title and the accessibility texts.
//!
//! The reference keeps the children in the nodes as they are added, and the stack as the chain of
//! open nodes. Here a node is appended to its parent when it is popped, which gives the same order:
//! at most one child of a node is open at a time, and the open chain has strictly growing levels.

use ferrigura_model::tree_view::{NodeType, TreeViewDiagram, TreeViewNode};

use crate::sanitize::sanitize_text;
use crate::text::is_js_space;

/// The state of the DB for one diagram.
pub(crate) struct Db {
    /// `cnt`: the id of the next node. It starts at 1, so `count` is the number of nodes plus one.
    count: u32,
    /// The open nodes, from the root at the bottom to the most recent node at the top.
    stack: Vec<TreeViewNode>,
    title: String,
    acc_title: String,
    acc_descr: String,
}

impl Db {
    /// A cleared DB: the root alone, and empty texts.
    pub(crate) fn new() -> Self {
        Self {
            count: 1,
            stack: vec![TreeViewNode::root()],
            title: String::new(),
            acc_title: String::new(),
            acc_descr: String::new(),
        }
    }

    /// `setDiagramTitle`: the title, sanitised.
    pub(crate) fn set_title(&mut self, text: &str) {
        self.title = sanitize_text(text);
    }

    /// `setAccTitle`: sanitised, without the blanks at its start (`^\s+`).
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = sanitize_text(text)
            .trim_start_matches(is_js_space)
            .to_owned();
    }

    /// `setAccDescription`: sanitised, with the blanks after each line break removed (`\n\s+`
    /// becomes `\n`).
    pub(crate) fn set_acc_descr(&mut self, text: &str) {
        let sanitised = sanitize_text(text);
        let mut out = String::with_capacity(sanitised.len());
        let mut chars = sanitised.chars().peekable();
        while let Some(c) = chars.next() {
            out.push(c);
            if c == '\n' {
                while chars.peek().is_some_and(|&next| is_js_space(next)) {
                    chars.next();
                }
            }
        }
        self.acc_descr = out;
    }

    /// `addNode`: the node becomes a child of the nearest open node with a smaller level, after the
    /// nodes at its level or deeper are closed.
    pub(crate) fn add_node(
        &mut self,
        level: i64,
        name: String,
        node_type: NodeType,
        css_class: String,
        icon: String,
        description: String,
    ) {
        while self.stack.len() > 1 && self.stack.last().is_some_and(|top| level <= top.level) {
            self.close_top();
        }
        let node = TreeViewNode {
            id: self.count,
            level,
            name,
            node_type,
            icon,
            css_class,
            description,
            children: Vec::new(),
        };
        self.count += 1;
        self.stack.push(node);
    }

    /// Closes the most recent open node, which joins the children of the node below it.
    /// The root is never closed.
    fn close_top(&mut self) {
        if self.stack.len() < 2 {
            return;
        }
        let node = self.stack.pop();
        if let (Some(node), Some(parent)) = (node, self.stack.last_mut()) {
            parent.children.push(node);
        }
    }

    /// The model: the root with its tree, and the texts and the count.
    pub(crate) fn into_model(mut self) -> TreeViewDiagram {
        while self.stack.len() > 1 {
            self.close_top();
        }
        let root = self.stack.pop().unwrap_or_else(TreeViewNode::root);
        TreeViewDiagram {
            version: ferrigura_model::CANONICAL_VERSION,
            title: self.title,
            acc_title: self.acc_title,
            acc_descr: self.acc_descr,
            count: self.count,
            root,
        }
    }
}
