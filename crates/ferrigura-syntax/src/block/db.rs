//! The block DB (`blockDB.ts`): the blocks, edges and classes of the parse, populated from the
//! statements in the reference's order. The reference keeps one object per block, and the map of
//! ids and the tree hold the same objects, so a later mention can change an object that the tree
//! does not show. The objects live in an arena here, and the map and the tree hold their indices,
//! which keeps those identities. See `docs-port/block/PLAN.md`, sections 5 and 8.

use std::collections::HashMap;

use ferrigura_model::block::{Block, BlockDiagram, ClassDef, Edge};

use super::grammar::{EdgeItem, Item};
use crate::error::{ParseError, Source};
use crate::sanitize::sanitize_text;
use crate::text::js_trim;

/// One object of the DB: a block, as the reference's objects hold it. Every field the reference
/// may leave `undefined` is an `Option`.
#[derive(Debug, Clone, Default)]
struct Obj {
    id: Option<String>,
    label: Option<String>,
    block_type: Option<String>,
    width: Option<f64>,
    width_in_columns: Option<f64>,
    columns: Option<f64>,
    directions: Option<Vec<String>>,
    classes: Option<Vec<String>>,
    styles: Option<Vec<String>>,
    color_index: Option<u32>,
    children: Option<Vec<usize>>,
}

/// A `classDef` entry of the DB.
struct Class {
    id: String,
    styles: Vec<String>,
    text_styles: Vec<String>,
}

/// The DB of one parse.
pub(crate) struct Db {
    /// Every object the DB has made. Index 0 is the root.
    objects: Vec<Obj>,
    /// `blockDatabase`: the object of each id. `None` is the `undefined` id of a chained link.
    by_id: HashMap<Option<String>, usize>,
    /// `edgeList`, with the ids counted per key.
    edges: Vec<Edge>,
    /// `edgeCount`: the number of edges of each key so far.
    edge_count: HashMap<String, u32>,
    /// `classes`, in the order they were first declared.
    classes: Vec<Class>,
    /// The index of each class in `classes`.
    class_index: HashMap<String, usize>,
    /// `nextColorIndex`: the next palette slot of a composite.
    next_color: u32,
    /// The warnings the reference logs while it populates.
    warnings: Vec<String>,
}

/// The `type` of a shape text, as `typeStr2Type` gives it; `na` for no shape.
pub(crate) fn type_str_to_type(type_str: Option<&str>) -> &'static str {
    match type_str {
        Some("[]") => "square",
        Some("()") => "round",
        Some("(())") => "circle",
        Some(">]") => "rect_left_inv_arrow",
        Some("{}") => "diamond",
        Some("{{}}") => "hexagon",
        Some("([])") => "stadium",
        Some("[[]]") => "subroutine",
        Some("[()]") => "cylinder",
        Some("((()))") => "doublecircle",
        Some("[//]") => "lean_right",
        Some("[\\\\]") => "lean_left",
        Some("[/\\]") => "trapezoid",
        Some("[\\/]") => "inv_trapezoid",
        Some("<[]>") => "block_arrow",
        _ => "na",
    }
}

/// The end arrow of a link (`edgeStrToEdgeData`): from its last character, after the blanks.
pub fn edge_str_to_edge_data(type_str: &str) -> &'static str {
    match type_str
        .trim_matches(crate::text::is_js_space)
        .chars()
        .last()
    {
        Some('x') => "arrow_cross",
        Some('o') => "arrow_circle",
        Some('>') => "arrow_point",
        _ => "",
    }
}

/// The start arrow of a link (`edgeStrToEdgeStartData`): from its first character, after the blanks.
pub fn edge_str_to_edge_start_data(type_str: &str) -> &'static str {
    match type_str
        .trim_matches(crate::text::is_js_space)
        .chars()
        .next()
    {
        Some('x') => "arrow_cross",
        Some('o') => "arrow_circle",
        Some('<') => "arrow_point",
        _ => "arrow_open",
    }
}

/// `thick` for a link with `==`, `normal` otherwise (`edgeStrToThickness`).
pub fn edge_str_to_thickness(type_str: &str) -> &'static str {
    if type_str.contains("==") {
        "thick"
    } else {
        "normal"
    }
}

/// `dotted` for a link with `.-`, `solid` otherwise (`edgeStrToPattern`).
pub fn edge_str_to_pattern(type_str: &str) -> &'static str {
    if type_str.contains(".-") {
        "dotted"
    } else {
        "solid"
    }
}

impl Db {
    /// The DB of a new parse: the root is registered under its id, as `clear` does.
    pub(crate) fn new() -> Self {
        let root = Obj {
            id: Some("root".to_owned()),
            block_type: Some("composite".to_owned()),
            children: Some(Vec::new()),
            columns: Some(-1.0),
            ..Obj::default()
        };
        let mut by_id = HashMap::new();
        by_id.insert(Some("root".to_owned()), 0);
        Self {
            objects: vec![root],
            by_id,
            edges: Vec::new(),
            edge_count: HashMap::new(),
            classes: Vec::new(),
            class_index: HashMap::new(),
            next_color: 0,
            warnings: Vec::new(),
        }
    }

    /// `setHierarchy`: populates the root from the statements of the diagram.
    pub(crate) fn populate_root(
        &mut self,
        items: Vec<Item>,
        source: Source<'_>,
    ) -> Result<(), ParseError> {
        self.populate(items, 0, source)
    }

    /// `populateBlockDatabase`: the statements of one list, in order, under the object `parent`.
    fn populate(
        &mut self,
        items: Vec<Item>,
        parent: usize,
        source: Source<'_>,
    ) -> Result<(), ParseError> {
        self.warn_widths(&items);
        let mut children = Vec::new();
        for item in items {
            match item {
                Item::ClassDef { id, css } => self.add_style_class(&id, &css),
                Item::ApplyClass { id, style_class } => self.set_css_class(&id, &style_class),
                Item::ApplyStyles { id, styles, start } => {
                    if !styles.is_empty() && !self.add_style_to_node(&id, &styles) {
                        return Err(source.error(
                            start,
                            format!("the style names {id}, which is not declared before it"),
                        ));
                    }
                }
                Item::ColumnSetting(columns) => self.objects[parent].columns = Some(columns),
                Item::Edge(edge) => self.add_edge(edge),
                block => self.add_block(block, &mut children, source)?,
            }
        }
        self.objects[parent].children = Some(children);
        Ok(())
    }

    /// The warning of a block wider than the first `columns` of its list (`populateBlockDatabase`,
    /// lines 108-121).
    fn warn_widths(&mut self, items: &[Item]) {
        let column = items
            .iter()
            .find_map(|item| match item {
                Item::ColumnSetting(columns) => Some(*columns),
                _ => None,
            })
            .unwrap_or(-1.0);
        if column <= 0.0 {
            return;
        }
        for item in items {
            let Some((id, width)) = span_of(item) else {
                continue;
            };
            if width > column {
                self.warnings.push(format!(
                    "Block {} width {width} exceeds configured column width {column}",
                    id.unwrap_or("undefined")
                ));
            }
        }
    }

    /// `edge`: the edge takes the count of its key as a prefix, and joins the edge list. Its label
    /// is sanitised, as every label is.
    fn add_edge(&mut self, edge: EdgeItem) {
        let label = if edge.label.is_empty() {
            edge.label
        } else {
            sanitize_text(&edge.label)
        };
        let count = self.edge_count.entry(edge.id.clone()).or_insert(0);
        *count += 1;
        let id = format!("{count}-{}", edge.id);
        self.edges.push(Edge {
            id,
            start: edge.start,
            end: edge.end,
            label,
            thickness: edge.thickness.to_owned(),
            pattern: edge.pattern.to_owned(),
            directions: edge.directions,
            arrow_type_end: edge.arrow_type_end.to_owned(),
            arrow_type_start: edge.arrow_type_start.to_owned(),
        });
    }

    /// A node, a space or a composite: the label is sanitised and defaulted, the object is
    /// registered or merged with the one that holds its id, and its children are populated.
    fn add_block(
        &mut self,
        item: Item,
        children: &mut Vec<usize>,
        source: Source<'_>,
    ) -> Result<(), ParseError> {
        let (mut block, raw_children) = match item {
            Item::Node {
                id,
                label,
                block_type,
                directions,
                width_in_columns,
            } => (
                Obj {
                    id,
                    label,
                    block_type,
                    directions,
                    width_in_columns,
                    ..Obj::default()
                },
                None,
            ),
            Item::Space { id, width } => (
                Obj {
                    id: Some(id),
                    label: Some(String::new()),
                    block_type: Some("space".to_owned()),
                    width: Some(width),
                    ..Obj::default()
                },
                Some(Vec::new()),
            ),
            Item::Composite {
                id,
                label,
                directions,
                width_in_columns,
                children,
            } => (
                Obj {
                    id,
                    label,
                    block_type: Some("composite".to_owned()),
                    directions,
                    width_in_columns,
                    ..Obj::default()
                },
                Some(children),
            ),
            _ => unreachable!("only blocks reach add_block"),
        };
        if let Some(text) = block.label.as_deref().filter(|text| !text.is_empty()) {
            block.label = Some(sanitize_text(text));
        }
        if block.label.as_deref().is_none_or(str::is_empty) {
            block.label = if block.block_type.as_deref() == Some("composite") {
                Some(String::new())
            } else {
                block.id.clone()
            };
        }

        let existing = self.by_id.get(&block.id).copied();
        let label_differs = block.label != block.id;
        let is_composite = block.block_type.as_deref() == Some("composite");
        let is_space = block.block_type.as_deref() == Some("space");
        let key = block.id.clone();
        let block_type = block.block_type.clone();
        let index = self.objects.len();
        self.objects.push(block);

        match existing {
            None => {
                if is_composite {
                    self.objects[index].color_index = Some(self.next_color);
                    self.next_color += 1;
                }
                self.by_id.insert(key, index);
            }
            Some(found) => {
                if block_type.as_deref() != Some("na") {
                    self.objects[found].block_type = block_type;
                }
                if label_differs {
                    self.objects[found].label = self.objects[index].label.clone();
                }
            }
        }

        if let Some(raw) = raw_children {
            self.populate(raw, index, source)?;
        }
        if is_space {
            self.add_space_cells(index, children);
        } else if existing.is_none() {
            children.push(index);
        }
        Ok(())
    }

    /// A space statement with span `w` becomes `w` cells, with ids `<id>-0` to `<id>-(w-1)`. Each
    /// cell is a copy of the space, and the space itself stays out of the tree.
    fn add_space_cells(&mut self, index: usize, children: &mut Vec<usize>) {
        let width = self.objects[index].width.unwrap_or(1.0);
        let base = self.objects[index].id.clone().unwrap_or_default();
        let mut cell = 0_u32;
        while f64::from(cell) < width {
            let mut copy = self.objects[index].clone();
            let id = format!("{base}-{cell}");
            copy.id = Some(id.clone());
            let copy_index = self.objects.len();
            self.objects.push(copy);
            self.by_id.insert(Some(id), copy_index);
            children.push(copy_index);
            cell += 1;
        }
    }

    /// `addStyleClass`: the class is created when it is new, and each comma-separated style is added
    /// without its first `;`, trimmed. A style that mentions `color` also gives a text style.
    fn add_style_class(&mut self, id: &str, attributes: &str) {
        let index = match self.class_index.get(id) {
            Some(&index) => index,
            None => {
                self.classes.push(Class {
                    id: id.to_owned(),
                    styles: Vec::new(),
                    text_styles: Vec::new(),
                });
                let index = self.classes.len() - 1;
                self.class_index.insert(id.to_owned(), index);
                index
            }
        };
        let class = &mut self.classes[index];
        for attribute in attributes.split(',') {
            let fixed = js_trim(&attribute.replacen(';', "", 1)).to_owned();
            if attribute.contains("color") {
                let with_bg = fixed.replacen("fill", "bgFill", 1);
                class.text_styles.push(with_bg.replacen("color", "fill", 1));
            }
            class.styles.push(fixed);
        }
    }

    /// `addStyle2Node`: the styles of the block with `id`, split at each comma and not trimmed.
    /// Returns false when no block has the id, which the reference reports as an error.
    fn add_style_to_node(&mut self, id: &str, styles: &str) -> bool {
        let Some(&index) = self.by_id.get(&Some(id.to_owned())) else {
            return false;
        };
        self.objects[index].styles = Some(styles.split(',').map(str::to_owned).collect());
        true
    }

    /// `setCssClass`: applies the class to each id of the comma list. An id the DB does not know yet
    /// gets an entry of its own, which is not a child of any block.
    fn set_css_class(&mut self, item_ids: &str, css_class: &str) {
        for id in item_ids.split(',') {
            let index = match self.by_id.get(&Some(id.to_owned())) {
                Some(&index) => index,
                None => {
                    let trimmed = js_trim(id).to_owned();
                    let index = self.objects.len();
                    self.objects.push(Obj {
                        id: Some(trimmed.clone()),
                        block_type: Some("na".to_owned()),
                        children: Some(Vec::new()),
                        ..Obj::default()
                    });
                    self.by_id.insert(Some(trimmed), index);
                    index
                }
            };
            self.objects[index]
                .classes
                .get_or_insert_with(Vec::new)
                .push(css_class.to_owned());
        }
    }

    /// The model of the parse: the root's columns, the blocks of the root in order, the edges, and
    /// the classes, with the warnings the reference logs.
    pub(crate) fn into_model(self) -> (BlockDiagram, Vec<String>) {
        let root = self
            .by_id
            .get(&Some("root".to_owned()))
            .copied()
            .unwrap_or(0);
        let blocks = self.objects[0]
            .children
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|index| self.block_model(index))
            .collect();
        let class_defs = self
            .classes
            .iter()
            .map(|class| ClassDef {
                id: class.id.clone(),
                styles: class.styles.clone(),
                text_styles: class.text_styles.clone(),
            })
            .collect();
        let mut diagram = BlockDiagram::new();
        diagram.columns = self.objects[root].columns;
        diagram.blocks = blocks;
        diagram.edges = self.edges;
        diagram.class_defs = class_defs;
        (diagram, self.warnings)
    }

    /// The model of one object and, recursively, of its children.
    fn block_model(&self, index: usize) -> Block {
        let obj = &self.objects[index];
        Block {
            id: obj.id.clone(),
            block_type: obj.block_type.clone(),
            label: obj.label.clone(),
            width: obj.width,
            width_in_columns: obj.width_in_columns,
            columns: obj.columns,
            directions: obj.directions.clone(),
            classes: obj.classes.clone(),
            styles: obj.styles.clone(),
            color_index: obj.color_index,
            children: obj
                .children
                .as_ref()
                .map(|list| list.iter().map(|&child| self.block_model(child)).collect()),
        }
    }
}

/// The id and the span of a block that has a span (`widthInColumns`): a node or a composite.
fn span_of(item: &Item) -> Option<(Option<&str>, f64)> {
    match item {
        Item::Node {
            id,
            width_in_columns: Some(width),
            ..
        }
        | Item::Composite {
            id,
            width_in_columns: Some(width),
            ..
        } => Some((id.as_deref(), *width)),
        _ => None,
    }
}
