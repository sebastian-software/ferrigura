//! The flowchart DB of `flowDb.ts`: the setters the grammar calls, with the state they keep. The
//! state is the canonical model of `ferrigura-model` plus the indexes the DB reads through (ids to
//! positions, the member set of the subgraphs, the count of edges per pair). Every lookup the
//! reference does by scanning an array is a map here, so the parse stays linear.
//!
//! Each method follows the reference method of the same name; the comments name the rule that
//! the reference applies in place of the sanitising step, which the model leaves out.

use std::collections::{BTreeMap, HashMap, HashSet};

use ferrigura_model::flowchart::{
    FlowClass, FlowEdge, FlowSubgraph, FlowTooltip, FlowVertex, FlowchartDiagram, LabelType,
};

use crate::text::{is_js_space, js_trim};

/// A text with its kind, as the grammar's `text`, `edgeText` and `textNoTags` rules give it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowText {
    pub text: String,
    pub label_type: LabelType,
}

/// The part of a link the DB needs: its arrow type, stroke and length, its label, and the id a
/// `LINK_ID` gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkData {
    pub edge_type: String,
    pub stroke: String,
    pub length: Option<u32>,
    pub text: Option<FlowText>,
    pub id: Option<String>,
}

/// An item of a subgraph's body, as the document collects it: a node or subgraph id, a line that
/// the grammar gives a string value (a lone `;`, an accessibility value), or a direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BodyItem {
    Str(String),
    Dir(String),
}

/// A DB error. The parser turns it into a syntax error at the statement that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DbError(pub String);

/// The largest number of edges the reference draws unless `maxEdges` says otherwise.
pub(crate) const DEFAULT_MAX_EDGES: usize = 500;

#[derive(Debug)]
pub(crate) struct Db {
    max_edges: usize,
    direction: String,
    acc_title: String,
    acc_descr: String,
    vertices: Vec<FlowVertex>,
    vertex_index: HashMap<String, usize>,
    edges: Vec<FlowEdge>,
    edge_index: HashMap<String, usize>,
    pair_counts: HashMap<(String, String), usize>,
    subgraphs: Vec<FlowSubgraph>,
    subgraph_index: HashMap<String, usize>,
    subgraph_members: HashSet<String>,
    sub_count: usize,
    classes: Vec<FlowClass>,
    tooltips: Vec<FlowTooltip>,
    default_style: Option<Vec<String>>,
    default_interpolate: Option<String>,
}

impl Db {
    pub(crate) fn new(max_edges: usize) -> Self {
        Self {
            max_edges,
            direction: String::from("TB"),
            acc_title: String::new(),
            acc_descr: String::new(),
            vertices: Vec::new(),
            vertex_index: HashMap::new(),
            edges: Vec::new(),
            edge_index: HashMap::new(),
            pair_counts: HashMap::new(),
            subgraphs: Vec::new(),
            subgraph_index: HashMap::new(),
            subgraph_members: HashSet::new(),
            sub_count: 0,
            classes: Vec::new(),
            tooltips: Vec::new(),
            default_style: None,
            default_interpolate: None,
        }
    }

    /// The model the DB holds: what the canonical JSON writes.
    pub(crate) fn into_model(self, title: String) -> FlowchartDiagram {
        let mut model = FlowchartDiagram::new();
        model.title = title;
        model.acc_title = self.acc_title;
        model.acc_descr = self.acc_descr;
        model.direction = self.direction;
        model.vertices = self.vertices;
        model.edges = self.edges;
        model.subgraphs = self.subgraphs;
        model.classes = self.classes;
        model.tooltips = self.tooltips;
        model.default_style = self.default_style;
        model.default_interpolate = self.default_interpolate;
        model
    }

    /// `setAccTitle`: the grammar's trimmed value without leading whitespace. The sanitising step
    /// is left out (see the canonical model).
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.acc_title = text.trim_start_matches(is_js_space).to_owned();
    }

    /// `setAccDescription`: each line break and the whitespace after it become one line break.
    pub(crate) fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = newline_then_space_to_newline(text);
    }

    /// `setDirection`: trims, then maps the arrow characters and `TD` as the reference does.
    pub(crate) fn set_direction(&mut self, dir: &str) {
        let mut direction = js_trim(dir).to_owned();
        if direction.contains('<') {
            direction = String::from("RL");
        }
        if direction.contains('^') {
            direction = String::from("BT");
        }
        if direction.contains('>') {
            direction = String::from("LR");
        }
        if direction.contains('v') {
            direction = String::from("TB");
        }
        if direction == "TD" {
            direction = String::from("TB");
        }
        self.direction = direction;
    }

    /// `addVertex`, for the forms the grammar uses here: an id with an optional text and shape,
    /// and the `[|key:value|…]` properties.
    pub(crate) fn add_vertex(
        &mut self,
        id: &str,
        text: Option<FlowText>,
        shape: Option<&str>,
        props: Option<(String, String)>,
    ) -> Result<(), DbError> {
        if js_trim(id).is_empty() {
            return Ok(());
        }
        // An id that is already the id of an edge returns before any vertex is made. The
        // reference does this for every call, not only for shape data.
        if self.edge_index.contains_key(id) {
            return Ok(());
        }
        let index = match self.vertex_index.get(id) {
            Some(&index) => index,
            None => {
                self.vertices.push(FlowVertex {
                    id: id.to_owned(),
                    text: id.to_owned(),
                    label_type: LabelType::Text,
                    shape: None,
                    styles: Vec::new(),
                    classes: Vec::new(),
                    link: None,
                    link_target: None,
                    props: BTreeMap::new(),
                    icon: None,
                    form: None,
                    pos: None,
                    img: None,
                    constraint: None,
                    asset_width: None,
                    asset_height: None,
                });
                let index = self.vertices.len() - 1;
                self.vertex_index.insert(id.to_owned(), index);
                index
            }
        };
        let vertex = &mut self.vertices[index];
        if let Some(text) = text {
            // The text is trimmed; one pair of surrounding quotes is removed.
            let trimmed = js_trim(&text.text);
            let stripped = strip_quotes(trimmed);
            vertex.text = stripped.to_owned();
            vertex.label_type = text.label_type;
        }
        if let Some(shape) = shape {
            vertex.shape = Some(shape.to_owned());
        }
        if let Some((field, value)) = props {
            vertex.props.insert(field, value);
        }
        Ok(())
    }

    /// `addLink`: one edge for every pair of the two sides. The user's id goes only to the pair
    /// that is the last start and the first end.
    pub(crate) fn add_link(
        &mut self,
        starts: &[String],
        ends: &[String],
        link: &LinkData,
    ) -> Result<(), DbError> {
        let id = link.id.as_deref().map(|id| id.replacen('@', "", 1));
        let last_start = starts.last();
        let first_end = ends.first();
        for start in starts {
            for end in ends {
                let is_last_start = Some(start) == last_start;
                let is_first_end = Some(end) == first_end;
                let edge_id = if is_last_start && is_first_end {
                    id.as_deref()
                } else {
                    None
                };
                self.add_single_link(start, end, link, edge_id)?;
            }
        }
        Ok(())
    }

    /// `addSingleLink`: the edge, its id, and the edge limit.
    fn add_single_link(
        &mut self,
        start: &str,
        end: &str,
        link: &LinkData,
        id: Option<&str>,
    ) -> Result<(), DbError> {
        let mut edge = FlowEdge {
            id: String::new(),
            is_user_defined_id: false,
            start: start.to_owned(),
            end: end.to_owned(),
            edge_type: link.edge_type.clone(),
            stroke: link.stroke.clone(),
            length: link.length.map(|length| length.min(10)),
            text: String::new(),
            label_type: LabelType::Text,
            classes: Vec::new(),
            style: None,
            interpolate: self.default_interpolate.clone(),
            animate: None,
            animation: None,
        };
        if let Some(text) = &link.text {
            edge.text = strip_quotes(js_trim(&text.text)).to_owned();
            edge.label_type = text.label_type;
        }
        match id {
            Some(id) if !self.edge_index.contains_key(id) => {
                edge.id = id.to_owned();
                edge.is_user_defined_id = true;
            }
            _ => {
                let key = (start.to_owned(), end.to_owned());
                let existing = self.pair_counts.get(&key).copied().unwrap_or(0);
                let counter = if existing == 0 { 0 } else { existing + 1 };
                edge.id = format!("L_{start}_{end}_{counter}");
            }
        }
        if self.edges.len() >= self.max_edges {
            return Err(DbError(format!(
                "Edge limit exceeded. {} edges found, but the limit is {}.\n\nInitialize mermaid with maxEdges set to a higher number to allow more edges.\nYou cannot set this config via configuration inside the diagram as it is a secure config.\nYou have to call mermaid.initialize.",
                self.edges.len(),
                self.max_edges
            )));
        }
        *self
            .pair_counts
            .entry((start.to_owned(), end.to_owned()))
            .or_insert(0) += 1;
        self.edge_index
            .entry(edge.id.clone())
            .or_insert(self.edges.len());
        self.edges.push(edge);
        Ok(())
    }

    /// `addSubGraph`. `id` is the text of the id, `None` when the subgraph has none; `title` is the
    /// title with its kind, and `id_is_title` says that the two are the same text (the form without
    /// brackets). The result is the subgraph's id.
    pub(crate) fn add_subgraph(
        &mut self,
        id: Option<&str>,
        id_is_title: bool,
        title: &FlowText,
        body: &[BodyItem],
    ) -> Result<String, DbError> {
        let Some(id) = id else {
            return Err(DbError(String::from(
                "subgraph needs an id or a title: the reference reads the text of an undefined id",
            )));
        };
        let mut id = Some(js_trim(id).to_owned());
        if id_is_title && title.text.chars().any(is_js_space) {
            id = None;
        }

        let (nodes, dir) = unique_items(body);
        let id = match id {
            Some(id) => id,
            None => format!("subGraph{}", self.sub_count),
        };
        // The counter moves on for every subgraph, named or not.
        self.sub_count += 1;

        // Members that already belong to a subgraph are left out, and so is the subgraph itself.
        let nodes: Vec<String> = nodes
            .into_iter()
            .filter(|node| !self.subgraph_members.contains(node) && *node != id)
            .collect();

        match self.subgraph_index.get(&id).copied() {
            Some(index) => {
                // A repeated id merges into the first declaration, which keeps its title and direction.
                for node in &nodes {
                    self.subgraph_members.insert(node.clone());
                }
                self.subgraphs[index].nodes.extend(nodes);
            }
            None => {
                for node in &nodes {
                    self.subgraph_members.insert(node.clone());
                }
                let subgraph = FlowSubgraph {
                    id: id.clone(),
                    title: js_trim(&title.text).to_owned(),
                    label_type: title.label_type,
                    nodes,
                    dir,
                    classes: Vec::new(),
                    metadata: BTreeMap::new(),
                };
                self.subgraphs.push(subgraph);
                self.subgraph_index
                    .insert(id.clone(), self.subgraphs.len() - 1);
            }
        }
        Ok(id)
    }
}

/// `uniq` of `addSubGraph`: the members in order, each once, without blank entries; the last
/// direction statement is the subgraph's direction.
fn unique_items(body: &[BodyItem]) -> (Vec<String>, Option<String>) {
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();
    let mut dir = None;
    for item in body {
        match item {
            BodyItem::Dir(direction) => dir = Some(direction.clone()),
            BodyItem::Str(text) => {
                if js_trim(text).is_empty() {
                    continue;
                }
                if seen.insert(text.clone()) {
                    nodes.push(text.clone());
                }
            }
        }
    }
    (nodes, dir)
}

/// The quotes that `addVertex` and `addSingleLink` remove: one pair around the whole text.
fn strip_quotes(text: &str) -> &str {
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

/// `\n\s+` replaced by `\n`.
fn newline_then_space_to_newline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '\n' {
            while chars.peek().is_some_and(|&next| is_js_space(next)) {
                chars.next();
            }
        }
    }
    out
}
