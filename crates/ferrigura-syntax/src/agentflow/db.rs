//! The agentflow DB (`agentflowDb.ts`, the state the parser's actions write), as a Rust struct.
//!
//! Each method is one of the DB's, with its order of checks and its effects; the reference's
//! line numbers are in `docs-port/agentflow/PLAN.md`, section 3. The model is what the DB holds
//! after the parse ([`AgentflowDiagram`]). The DB's diagnostics, the element mappings and the
//! layout data (`getData`) are not here: the reference fills the first at render time and the
//! other two are not part of the parse.

use std::collections::{BTreeMap, HashSet};

use ferrigura_model::agentflow::{AgentflowDiagram, ClassDef, Edge, SubGraph, Vertex};
use ferrigura_model::value::Value;

use super::metadata::{js_string, json_text, merge_metadata, parse_metadata, truthy};
use super::shapes::{is_valid_shape_name, resolve_shape_alias};
use crate::sanitize::sanitize_text;
use crate::text::{is_js_space, js_trim};
use crate::url::format_url;

/// The longest an edge may be: the length is capped at this (`agentflowDb.ts:758`).
const MAX_EDGE_LENGTH: u32 = 10;

/// A text of the grammar: the string and its kind, `text`, `string` or `markdown`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowText {
    pub text: String,
    pub kind: &'static str,
}

impl FlowText {
    /// A text of the given kind.
    pub(crate) fn new(text: impl Into<String>, kind: &'static str) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}

/// What a link gives the edges it makes: the arrow, its stroke and length, its meaning, its label
/// and the author's id (`linkData`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LinkData {
    pub arrow: &'static str,
    pub stroke: &'static str,
    pub length: u32,
    pub edge_semantic: Option<&'static str>,
    pub text: Option<FlowText>,
    pub id: Option<String>,
}

/// A linkStyle position: the word `default`, or a NUM token's text (a string in the reference, so
/// the index is looked up as a property, Q5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Position {
    Default,
    Index(String),
}

/// A list item of a block, as the grammar's actions see it: a string (an id, a subgraph id, a
/// blank or a semicolon), or a `direction` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BodyItem {
    Str(String),
    Dir(String),
}

/// A DB call that throws: the message of the reference's error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DbError(pub String);

/// The DB state the parser fills in.
pub(crate) struct Db {
    model: AgentflowDiagram,
    /// The index of each vertex in `model.vertices`, by id.
    index: BTreeMap<String, usize>,
    /// The ids of the connectors, in creation order. A connector record is the vertex of its id.
    connector_ids: Vec<String>,
    /// The `defaultInterpolate` of the edges: a `linkStyle default interpolate` statement sets it
    /// for the edges that come after it.
    default_interpolate: Option<String>,
    /// The ids that a `global` block declared (`globalNodes`).
    global_nodes: HashSet<String>,
    /// The number of containers without an id (`subCount`).
    sub_count: usize,
    /// The most edges a diagram may have.
    max_edges: usize,
}

impl Db {
    /// A DB that has been cleared, as `Diagram.fromText` leaves it before the parse.
    pub(crate) fn new(max_edges: usize) -> Self {
        Self {
            model: AgentflowDiagram::new(),
            index: BTreeMap::new(),
            connector_ids: Vec::new(),
            default_interpolate: None,
            global_nodes: HashSet::new(),
            sub_count: 0,
            max_edges,
        }
    }

    /// The model the parse leaves. `title` is the front matter's title (`setDiagramTitle`).
    pub(crate) fn into_model(mut self, title: String) -> AgentflowDiagram {
        self.model.title = title;
        self.model.connectors = self
            .connector_ids
            .iter()
            .filter_map(|id| self.index.get(id).map(|&i| self.model.vertices[i].clone()))
            .collect();
        self.model
    }

    /// `setDirection(dir)`: the header's direction, with `<`, `^`, `>` and `v` mapped to RL, BT,
    /// LR and TB in that order, and `TD` to `TB` (`agentflowDb.ts:902-920`, Q4).
    pub(crate) fn set_direction(&mut self, dir: &str) {
        let mut direction = js_trim(dir).to_owned();
        if direction.contains('<') {
            direction = "RL".to_owned();
        }
        if direction.contains('^') {
            direction = "BT".to_owned();
        }
        if direction.contains('>') {
            direction = "LR".to_owned();
        }
        if direction.contains('v') {
            direction = "TB".to_owned();
        }
        if direction == "TD" {
            direction = "TB".to_owned();
        }
        self.model.direction = direction;
    }

    /// `setAccTitle(txt)`: the sanitised text without its leading blanks.
    pub(crate) fn set_acc_title(&mut self, text: &str) {
        self.model.acc_title = sanitize_text(text)
            .trim_start_matches(is_js_space)
            .to_owned();
    }

    /// `setAccDescription(txt)`: the sanitised text with the blanks after each line break removed.
    pub(crate) fn set_acc_descr(&mut self, text: &str) {
        let sanitized = sanitize_text(text);
        let mut out = String::with_capacity(sanitized.len());
        let mut chars = sanitized.chars().peekable();
        while let Some(c) = chars.next() {
            out.push(c);
            if c == '\n' && chars.peek().is_some_and(|&next| is_js_space(next)) {
                while chars.peek().is_some_and(|&next| is_js_space(next)) {
                    chars.next();
                }
            }
        }
        self.model.acc_descr = out;
    }

    /// `addVertex(id, text, type, style, classes, dir, props, metadata)`: the grammar's calls pass
    /// the text, the shape, the styles, the props and the metadata block; the classes and the
    /// direction are never passed.
    pub(crate) fn add_vertex(
        &mut self,
        id: &str,
        text: Option<&FlowText>,
        shape: Option<&str>,
        styles: Option<&[String]>,
        props: Option<BTreeMap<String, String>>,
        metadata: Option<&str>,
    ) -> Result<(), DbError> {
        if js_trim(id).is_empty() {
            return Ok(());
        }
        let mut doc = match metadata {
            Some(block) => Some(parse_metadata(block).map_err(DbError)?),
            None => None,
        };
        // The alias of the shape is resolved in the metadata before anything else reads it; the
        // authored name is kept for the case check.
        let mut authored: Option<String> = None;
        if let Some(Value::Object(map)) = doc.as_mut()
            && let Some(Value::String(name)) = map.get("shape")
        {
            let name = name.clone();
            let resolved = resolve_shape_alias(&name);
            if resolved != name {
                map.insert("shape".to_owned(), Value::String(resolved));
            }
            authored = Some(name);
        }
        let doc_truthy = doc.as_ref().is_some_and(truthy);

        // A container id takes the metadata, and nothing else.
        if doc_truthy && let Some(pos) = self.sub_graph_position(id) {
            let sub_graph = &mut self.model.sub_graphs[pos];
            let merged = merge_metadata(sub_graph.metadata.take(), doc.as_ref());
            sub_graph.metadata = Some(merged);
            return Ok(());
        }
        // A connector takes the metadata too.
        if doc_truthy && self.connector_ids.iter().any(|c| c == id) {
            let pos = self.index[id];
            let vertex = &mut self.model.vertices[pos];
            let merged = merge_metadata(vertex.metadata.take(), doc.as_ref());
            vertex.metadata = Some(merged);
            return Ok(());
        }
        // `connectors` is a reserved id: it is never a vertex.
        if id == "connectors" {
            return Ok(());
        }
        // An edge id takes its metadata as the presentation keys of the edge, and never makes a
        // vertex.
        if let Some(pos) = self.edge_position(id) {
            let edge = &mut self.model.edges[pos];
            if let Some(Value::Object(map)) = doc.as_ref() {
                if let Some(value) = map.get("animate") {
                    edge.animate = Some(value.clone());
                }
                if let Some(value) = map.get("animation") {
                    edge.animation = Some(value.clone());
                }
                if let Some(value) = map.get("curve") {
                    edge.interpolate = Some(value.clone());
                }
            }
            if doc_truthy {
                let merged = merge_metadata(edge.metadata.take(), doc.as_ref());
                edge.metadata = Some(merged);
            }
            return Ok(());
        }

        let pos = match self.index.get(id) {
            Some(&pos) => pos,
            None => {
                self.model.vertices.push(Vertex {
                    id: id.to_owned(),
                    text: id.to_owned(),
                    label_type: "text".to_owned(),
                    shape: None,
                    styles: Vec::new(),
                    classes: Vec::new(),
                    props: None,
                    metadata: None,
                    link: None,
                    link_target: None,
                    is_connector: None,
                });
                let pos = self.model.vertices.len() - 1;
                self.index.insert(id.to_owned(), pos);
                pos
            }
        };
        let vertex = &mut self.model.vertices[pos];
        if let Some(text) = text {
            let mut label = sanitize_text(js_trim(&text.text));
            // One pair of surrounding quotes goes; a lone quote is kept.
            if label.len() >= 2 && label.starts_with('"') && label.ends_with('"') {
                label = label[1..label.len() - 1].to_owned();
            }
            vertex.text = label;
            vertex.label_type = text.kind.to_owned();
        }
        if let Some(shape) = shape {
            vertex.shape = Some(shape.to_owned());
        }
        if let Some(styles) = styles {
            vertex.styles.extend(styles.iter().cloned());
        }
        let props = props.unwrap_or_default();
        match vertex.props.as_mut() {
            Some(existing) => existing.extend(props),
            None => vertex.props = Some(props),
        }

        if doc_truthy {
            let merged = merge_metadata(vertex.metadata.take(), doc.as_ref());
            vertex.metadata = Some(merged);
            if let Some(Value::Object(map)) = doc.as_ref() {
                if let Some(shape_value) = map.get("shape")
                    && truthy(shape_value)
                {
                    // The authored name is judged for its case; the resolved name must be a shape
                    // of the registry. A shape that is not a string is an unknown shape.
                    let Some(authored_name) = authored.as_deref() else {
                        return Err(DbError(format!(
                            "No such shape: {}.",
                            json_text(shape_value)
                        )));
                    };
                    if authored_name != authored_name.to_lowercase() || authored_name.contains('_')
                    {
                        return Err(DbError(format!(
                            "No such shape: {authored_name}. Shape names should be lowercase."
                        )));
                    }
                    let resolved = value_str(shape_value).unwrap_or_default();
                    if !is_valid_shape_name(resolved) {
                        return Err(DbError(format!("No such shape: {resolved}.")));
                    }
                    vertex.shape = Some(resolved.to_owned());
                }
                if let Some(label) = map.get("label")
                    && truthy(label)
                {
                    vertex.text = sanitize_text(&js_string(label));
                    vertex.label_type =
                        sanitize_label_type(map.get("labelType").and_then(value_str)).to_owned();
                }
            }
        }
        Ok(())
    }

    /// `addSingleLink(start, end, type, id)`: one edge, with its label, its length (at most 10),
    /// its id and the default interpolation (`agentflowDb.ts:728-789`).
    fn add_single_link(
        &mut self,
        start: &str,
        end: &str,
        link: &LinkData,
        id: Option<&str>,
    ) -> Result<(), DbError> {
        let mut edge = Edge {
            id: String::new(),
            is_user_defined_id: false,
            start: start.to_owned(),
            end: end.to_owned(),
            arrow: link.arrow.to_owned(),
            stroke: link.stroke.to_owned(),
            length: link.length.min(MAX_EDGE_LENGTH),
            text: String::new(),
            label_type: "text".to_owned(),
            classes: Vec::new(),
            style: None,
            interpolate: self.default_interpolate.clone().map(Value::String),
            animate: None,
            animation: None,
            edge_semantic: link.edge_semantic.map(str::to_owned),
            metadata: None,
        };
        if let Some(text) = &link.text {
            let mut label = sanitize_text(js_trim(&text.text));
            if label.len() >= 2 && label.starts_with('"') && label.ends_with('"') {
                label = label[1..label.len() - 1].to_owned();
            }
            edge.text = label;
            edge.label_type = sanitize_label_type(Some(text.kind)).to_owned();
        }
        let user_id =
            id.filter(|id| !id.is_empty() && !self.model.edges.iter().any(|e| e.id == *id));
        match user_id {
            Some(user_id) => {
                edge.id = user_id.to_owned();
                edge.is_user_defined_id = true;
            }
            None => {
                let existing = self
                    .model
                    .edges
                    .iter()
                    .filter(|e| e.start == start && e.end == end)
                    .count();
                let counter = if existing == 0 { 0 } else { existing + 1 };
                edge.id = format!("L_{start}_{end}_{counter}");
            }
        }
        if self.model.edges.len() < self.max_edges {
            self.model.edges.push(edge);
            Ok(())
        } else {
            Err(DbError(format!(
                "Edge limit exceeded. {} edges found, but the limit is {}.\n\nInitialize mermaid with maxEdges set to a higher number to allow more edges.\nYou cannot set this config via configuration inside the diagram as it is a secure config.\nYou have to call mermaid.initialize.",
                self.model.edges.len(),
                self.max_edges
            )))
        }
    }

    /// `addLink(starts, ends, linkData)`: an edge for each pair. The author's id goes to the last
    /// start and the first end only, and the `@` of the id is dropped (`agentflowDb.ts:801-820`).
    pub(crate) fn add_link(
        &mut self,
        starts: &[String],
        ends: &[String],
        link: &LinkData,
    ) -> Result<(), DbError> {
        let id = link.id.as_ref().map(|id| id.replacen('@', "", 1));
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

    /// `updateLinkInterpolate(positions, interpolate)` (`agentflowDb.ts:825-840`).
    pub(crate) fn update_link_interpolate(
        &mut self,
        positions: &[Position],
        interpolate: &str,
    ) -> Result<(), DbError> {
        for position in positions {
            match position {
                Position::Default => self.default_interpolate = Some(interpolate.to_owned()),
                Position::Index(text) => {
                    let pos = self.edge_index(text)?;
                    self.model.edges[pos].interpolate = Some(Value::String(interpolate.to_owned()));
                }
            }
        }
        Ok(())
    }

    /// `updateLink(positions, style)` (`agentflowDb.ts:848-870`). The style array is shared by the
    /// edges of one statement, and a style with no `fill` gets `fill:none` once.
    pub(crate) fn update_link(
        &mut self,
        positions: &[Position],
        style: &[String],
    ) -> Result<(), DbError> {
        let mut style = style.to_vec();
        for position in positions {
            if let Position::Index(text) = position {
                self.edge_index(text)?;
                if !style.is_empty() && !style.iter().any(|s| s.starts_with("fill")) {
                    style.push("fill:none".to_owned());
                }
            }
        }
        for position in positions {
            if let Position::Index(text) = position {
                let pos = self.edge_index(text)?;
                self.model.edges[pos].style = Some(style.clone());
            }
        }
        Ok(())
    }

    /// `addClass(ids, styles)`: the class of each id, with the colour styles also kept apart with
    /// `fill` renamed `bgFill` (`agentflowDb.ts:872-896`).
    pub(crate) fn add_class(&mut self, ids: &str, styles: &[String]) {
        let joined = styles.join(",");
        let fixed = joined
            .replace("\\,", "\u{1}")
            .replace(',', ";")
            .replace('\u{1}', ",");
        for id in ids.split(',') {
            let pos = match self.model.classes.iter().position(|c| c.id == id) {
                Some(pos) => pos,
                None => {
                    self.model.classes.push(ClassDef {
                        id: id.to_owned(),
                        styles: Vec::new(),
                        text_styles: Vec::new(),
                    });
                    self.model.classes.len() - 1
                }
            };
            for style in fixed.split(';') {
                let class = &mut self.model.classes[pos];
                if style.contains("color") {
                    class.text_styles.push(style.replacen("fill", "bgFill", 1));
                }
                class.styles.push(style.to_owned());
            }
        }
    }

    /// `setClass(ids, className)`: the class goes to each vertex, edge and container of the ids.
    pub(crate) fn set_class(&mut self, ids: &str, class_name: &str) {
        for id in ids.split(',') {
            if let Some(&pos) = self.index.get(id) {
                self.model.vertices[pos].classes.push(class_name.to_owned());
            }
            if let Some(pos) = self.edge_position(id) {
                self.model.edges[pos].classes.push(class_name.to_owned());
            }
            if let Some(pos) = self.sub_graph_position(id) {
                self.model.sub_graphs[pos]
                    .classes
                    .push(class_name.to_owned());
            }
        }
    }

    /// `setTooltip(ids, tooltip)`: the sanitised tooltip of each id.
    pub(crate) fn set_tooltip(&mut self, ids: &str, tooltip: &str) {
        let sanitized = sanitize_text(tooltip);
        for id in ids.split(',') {
            self.model.tooltips.insert(id.to_owned(), sanitized.clone());
        }
    }

    /// `setLink(ids, linkStr, target)`: the URL of each vertex of the ids, then the class
    /// `clickable` (`agentflowDb.ts:1010-1019`). The security level is the default one, so the URL
    /// is sanitised.
    pub(crate) fn set_link(&mut self, ids: &str, link: &str, target: Option<&str>) {
        for id in ids.split(',') {
            if let Some(&pos) = self.index.get(id) {
                let vertex = &mut self.model.vertices[pos];
                vertex.link = format_url(link, false);
                vertex.link_target = target.map(str::to_owned);
            }
        }
        self.set_class(ids, "clickable");
    }

    /// `setClickEvent(ids, …)`: the callbacks run only at `securityLevel: loose`, which the default
    /// configuration is not, so a click sets the class `clickable` and nothing else.
    pub(crate) fn set_click_event(&mut self, ids: &str) {
        self.set_class(ids, "clickable");
    }

    /// `addSubGraph(id, list, title, type)` (`agentflowDb.ts:1175-1282`): the members of a `flow`
    /// block, its id, its title and its direction. The id is `subGraph<n>` when there is none.
    pub(crate) fn add_sub_graph(
        &mut self,
        id: Option<&FlowText>,
        list: &[BodyItem],
        title: Option<&FlowText>,
    ) -> Result<String, DbError> {
        let (members, dir) = unique_members(list)?;
        let id = match id {
            Some(id) => js_trim(&id.text).to_owned(),
            None => format!("subGraph{}", self.sub_count),
        };
        let title_text = title.map_or("", |t| t.text.as_str());
        let title_text = sanitize_text(title_text);
        self.sub_count += 1;
        let nodes: Vec<String> = members
            .into_iter()
            .filter(|node| *node != id && !self.global_nodes.contains(node))
            .collect();
        let mut sub_graph = SubGraph {
            id: id.clone(),
            nodes,
            title: js_trim(&title_text).to_owned(),
            label_type: sanitize_label_type(title.map(|t| t.kind)).to_owned(),
            dir,
            kind: "flow".to_owned(),
            classes: Vec::new(),
            metadata: None,
        };
        // A member that already belongs to another container is taken out of this one.
        let existing_members: Vec<String> = self
            .model
            .sub_graphs
            .iter()
            .flat_map(|sg| sg.nodes.iter().cloned())
            .collect();
        sub_graph
            .nodes
            .retain(|node| !existing_members.contains(node));

        match self.sub_graph_position(&id) {
            Some(pos) => {
                // A repeated id merges its members into the first record; the title and the
                // direction of the later block win when they are set.
                let existing = &mut self.model.sub_graphs[pos];
                for node in sub_graph.nodes {
                    if !existing.nodes.contains(&node) {
                        existing.nodes.push(node);
                    }
                }
                if !sub_graph.title.is_empty() {
                    existing.title = sub_graph.title;
                }
                if sub_graph.dir.is_some() {
                    existing.dir = sub_graph.dir;
                }
            }
            None => self.model.sub_graphs.push(sub_graph),
        }
        Ok(id)
    }

    /// `addGlobal(list)`: the ids of a `global` block are outside every container, and the
    /// containers that already took them let them go.
    pub(crate) fn add_global(&mut self, list: &[BodyItem]) {
        for item in list {
            if let BodyItem::Str(text) = item {
                let id = js_trim(text);
                if !id.is_empty() {
                    self.global_nodes.insert(id.to_owned());
                }
            }
        }
        let global = &self.global_nodes;
        for sub_graph in &mut self.model.sub_graphs {
            sub_graph.nodes.retain(|node| !global.contains(node));
        }
    }

    /// `addConnector(textObj, titleObj)`: a connector is a vertex with its title, or its id when
    /// it has none. Returns the id, or nothing when the id is blank (`agentflowDb.ts:1380-1407`).
    pub(crate) fn add_connector(&mut self, text: &FlowText, title: Option<&FlowText>) -> String {
        let id = js_trim(&text.text).to_owned();
        if id.is_empty() {
            return String::new();
        }
        let title = title.map_or("", |t| js_trim(&t.text)).to_owned();
        let resolved = if title.is_empty() {
            id.clone()
        } else {
            sanitize_text(&title)
        };
        let existing = self.connector_ids.contains(&id);
        if existing {
            if !title.is_empty() {
                let pos = self.index[&id];
                self.model.vertices[pos].text = resolved;
            }
        } else {
            let vertex = Vertex {
                id: id.clone(),
                text: resolved,
                label_type: "text".to_owned(),
                shape: None,
                styles: Vec::new(),
                classes: Vec::new(),
                props: None,
                metadata: None,
                link: None,
                link_target: None,
                is_connector: Some(true),
            };
            match self.index.get(&id) {
                Some(&pos) => self.model.vertices[pos] = vertex,
                None => {
                    self.model.vertices.push(vertex);
                    self.index.insert(id.clone(), self.model.vertices.len() - 1);
                }
            }
            self.connector_ids.push(id.clone());
        }
        id
    }

    /// `destructLink(linkStr, startStr)` (`agentflowDb.ts:1442-1544`): the arrow, the stroke and
    /// the length of a link, and its edge semantic, from the end of the link. A link with a start
    /// (`--` and its text) has a solid start, and its end is a solid end too: the grammar gives
    /// no other pair, so the reference's mismatch case (`INVALID`) cannot be reached.
    pub(crate) fn destruct_link(&self, end: &str) -> LinkData {
        let (arrow, stroke, length) = destruct_end_link(end);
        LinkData {
            arrow,
            stroke,
            length,
            edge_semantic: edge_semantic(arrow, stroke),
            text: None,
            id: None,
        }
    }

    /// The position of an edge by its id, or `None`.
    fn edge_position(&self, id: &str) -> Option<usize> {
        self.model.edges.iter().position(|e| e.id == id)
    }

    /// The position of a container by its id, or `None`.
    fn sub_graph_position(&self, id: &str) -> Option<usize> {
        self.model.sub_graphs.iter().position(|sg| sg.id == id)
    }

    /// The index of an edge named by a linkStyle position. The reference reads the position as a
    /// property of the edge array, so only the canonical decimal index of an edge that exists
    /// is one; any other is a TypeError (Q5).
    fn edge_index(&self, text: &str) -> Result<usize, DbError> {
        let canonical =
            text == "0" || (!text.starts_with('0') && text.bytes().all(|b| b.is_ascii_digit()));
        let index = canonical
            .then(|| text.parse::<usize>().ok())
            .flatten()
            .filter(|&index| index < self.model.edges.len());
        index.ok_or_else(|| {
            DbError(format!(
                "Cannot set properties of undefined (setting 'style') at linkStyle {text}"
            ))
        })
    }
}

/// `sanitizeNodeLabelType(labelType)`: the three kinds are kept, anything else is `markdown`.
fn sanitize_label_type(kind: Option<&str>) -> &str {
    match kind {
        Some(kind @ ("markdown" | "string" | "text")) => kind,
        _ => "markdown",
    }
}

/// A metadata value as a string, when it is one.
fn value_str(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) => Some(text),
        _ => None,
    }
}

/// `uniq(list.flat())` of `addSubGraph`: the members in order without repeats or blanks, and the
/// last direction. The reference's dedupe keeps its seen strings in an object that the
/// `hasOwnProperty` member of an id overwrites, so the next string after that id throws; a
/// `__proto__` member is never a repeat (`agentflowDb.ts:1200-1218`).
fn unique_members(list: &[BodyItem]) -> Result<(Vec<String>, Option<String>), DbError> {
    let mut members = Vec::new();
    let mut dir = None;
    let mut seen: HashSet<&str> = HashSet::new();
    let mut poisoned = false;
    for item in list {
        match item {
            BodyItem::Dir(value) => dir = Some(value.clone()),
            BodyItem::Str(text) => {
                if js_trim(text).is_empty() {
                    continue;
                }
                if poisoned {
                    return Err(DbError(
                        "prims[type].hasOwnProperty is not a function".to_owned(),
                    ));
                }
                if text == "__proto__" {
                    members.push(text.clone());
                    continue;
                }
                if !seen.insert(text) {
                    continue;
                }
                if text == "hasOwnProperty" {
                    poisoned = true;
                }
                members.push(text.clone());
            }
        }
    }
    Ok((members, dir))
}

/// `destructEndLink(str)`: the arrow from the last character, the stroke and the length. The
/// length is the number of dashes less one, or the number of dots when there are any.
fn destruct_end_link(text: &str) -> (&'static str, &'static str, u32) {
    let str = js_trim(text);
    let last = str.chars().last();
    let (arrow, line) = match last {
        Some('x') => ("arrow_cross", &str[..str.len() - 1]),
        Some('>') => ("arrow_point", &str[..str.len() - 1]),
        Some('-' | '.') => ("arrow_open", str),
        _ => ("arrow_open", &str[..str.len().saturating_sub(1)]),
    };
    let dots = line.matches('.').count();
    if dots > 0 {
        return (arrow, "dotted", dots as u32);
    }
    let length = (line.chars().count() as u32).saturating_sub(1);
    (arrow, "normal", length)
}

/// `computeEdgeSemantic(type, stroke)`: the three arrows the specification names.
fn edge_semantic(arrow: &str, stroke: &str) -> Option<&'static str> {
    match (arrow, stroke) {
        ("arrow_point", "normal") => Some("sequence"),
        ("arrow_cross", "normal") => Some("failure"),
        ("arrow_open", "dotted") => Some("reference"),
        _ => None,
    }
}
