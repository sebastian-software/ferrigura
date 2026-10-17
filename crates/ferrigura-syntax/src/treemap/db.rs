//! The treemap DB of `packages/mermaid/src/diagrams/treemap/db.ts` and the hierarchy of
//! `utils.ts`'s `buildHierarchy`, as the parser's `populate` calls them (`parser.ts` 13-75).
//!
//! The DB's header setters sanitise the texts, and the grammar's `title` replaces the front matter's
//! only when it is non-empty (`populateCommonDb`). The classes are added before any item, so an
//! item takes the styles of a class defined after it.

use ferrigura_model::treemap::{TreemapClass, TreemapDiagram, TreemapNode};

use super::grammar::{Ast, Item, Row};
use crate::sanitize::sanitize_text;
use crate::text::{newline_then_space_to_newline, strip_leading_space};

/// The label styles of `isLabelStyle`: a style piece that is exactly one of these names is also a
/// text style of its class.
const LABEL_STYLES: &[&str] = &[
    "color",
    "font-size",
    "font-family",
    "font-weight",
    "font-style",
    "text-decoration",
    "text-align",
    "text-transform",
    "line-height",
    "letter-spacing",
    "word-spacing",
    "text-shadow",
    "text-overflow",
    "white-space",
    "word-wrap",
    "word-break",
    "overflow-wrap",
    "hyphens",
];

/// Builds the model of a treemap from its syntax tree, with the front matter's title when it has
/// one.
pub(crate) fn build(ast: &Ast, front_matter_title: Option<&str>) -> TreemapDiagram {
    let mut diagram = TreemapDiagram::new();
    // `Diagram.fromText` sets the front matter title first; the grammar's title replaces it when
    // the grammar gives a non-empty one.
    diagram.title = if ast.header.title.is_empty() {
        front_matter_title.map(sanitize_text).unwrap_or_default()
    } else {
        sanitize_text(&ast.header.title)
    };
    if !ast.header.acc_title.is_empty() {
        diagram.acc_title = strip_leading_space(&sanitize_text(&ast.header.acc_title)).to_owned();
    }
    if !ast.header.acc_descr.is_empty() {
        diagram.acc_descr = newline_then_space_to_newline(&sanitize_text(&ast.header.acc_descr));
    }

    // `populate`: every classDef row first, in order, then the items.
    for row in &ast.rows {
        if let Row::ClassDef { name, style } = row {
            add_class(&mut diagram.classes, name, style);
        }
    }
    let mut items = Vec::new();
    for row in &ast.rows {
        if let Row::Item { level, item } = row {
            items.push(flat_item(*level, item, &diagram.classes));
        }
    }
    diagram.tree = TreemapNode {
        name: String::new(),
        children: Some(build_hierarchy(&items)),
        value: None,
        class_selector: None,
        css_compiled_styles: None,
    };
    diagram
}

/// `TreeMapDB.addClass`: the style text has its escaped commas kept, every other comma becomes a
/// separator, and the text is split at each separator. Each piece is a style, and a label style
/// is also a text style. Adding again to a class appends to it.
fn add_class(classes: &mut Vec<TreemapClass>, id: &str, style: &str) {
    let index = match classes.iter().position(|class| class.id == id) {
        Some(index) => index,
        None => {
            classes.push(TreemapClass {
                id: id.to_owned(),
                styles: Vec::new(),
                text_styles: Vec::new(),
            });
            classes.len() - 1
        }
    };
    let Some(class) = classes.get_mut(index) else {
        return;
    };
    let pieces = style
        .replace("\\,", "§§§")
        .replace(',', ";")
        .replace("§§§", ",");
    for piece in pieces.split(';') {
        if LABEL_STYLES.contains(&piece) {
            class.text_styles.push(piece.to_owned());
        }
        class.styles.push(piece.to_owned());
    }
}

/// One item of the flat list that `buildHierarchy` takes, with the styles its class gives.
struct FlatItem {
    level: usize,
    node: TreemapNode,
    leaf: bool,
}

fn flat_item(level: usize, item: &Item, classes: &[TreemapClass]) -> FlatItem {
    let (name, value, class, leaf) = match item {
        Item::Leaf { name, value, class } => (name, Some(*value), class, true),
        Item::Section { name, class } => (name, None, class, false),
    };
    // `getStylesForClass`, and no list when there are no styles.
    let css_compiled_styles = class
        .as_ref()
        .and_then(|id| classes.iter().find(|class| class.id == *id))
        .map(|class| class.styles.clone())
        .filter(|styles| !styles.is_empty());
    FlatItem {
        level,
        node: TreemapNode {
            name: name.clone(),
            children: if leaf { None } else { Some(Vec::new()) },
            value,
            class_selector: class.clone(),
            css_compiled_styles,
        },
        leaf,
    }
}

/// `buildHierarchy`: a row pops every open section at its level or deeper, and becomes a child of
/// the section on top, or a root when none is open. Only sections are kept open.
fn build_hierarchy(items: &[FlatItem]) -> Vec<TreemapNode> {
    // The nodes are kept in an arena, so each one can get children after it is placed.
    let mut arena: Vec<(TreemapNode, bool, Vec<usize>)> = Vec::new();
    let mut roots = Vec::new();
    // The open sections, innermost last, with their level and their arena index.
    let mut open: Vec<(usize, usize)> = Vec::new();
    for item in items {
        while open.last().is_some_and(|&(level, _)| level >= item.level) {
            open.pop();
        }
        let index = arena.len();
        arena.push((item.node.clone(), item.leaf, Vec::new()));
        match open.last() {
            None => roots.push(index),
            Some(&(_, parent)) => {
                if let Some(entry) = arena.get_mut(parent) {
                    entry.2.push(index);
                }
            }
        }
        if !item.leaf {
            open.push((item.level, index));
        }
    }
    roots
        .into_iter()
        .map(|index| finish(&arena, index))
        .collect()
}

/// The node at `index` with its children, which are the arena entries it lists.
fn finish(arena: &[(TreemapNode, bool, Vec<usize>)], index: usize) -> TreemapNode {
    let Some((node, leaf, children)) = arena.get(index) else {
        return TreemapNode::root();
    };
    let mut node = node.clone();
    if !leaf {
        node.children = Some(children.iter().map(|&child| finish(arena, child)).collect());
    }
    node
}
