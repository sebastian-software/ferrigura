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
use crate::yaml::Yaml;

use super::shapes::is_valid_shape;

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

/// The position a `linkStyle` statement gives: the default, or the index of an edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkPosition {
    Default,
    Index(String),
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
    class_index: HashMap<String, usize>,
    tooltips: Vec<FlowTooltip>,
    tooltip_index: HashMap<String, usize>,
    default_style: Option<Vec<String>>,
    default_interpolate: Option<String>,
    warnings: Vec<String>,
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
            class_index: HashMap::new(),
            tooltips: Vec::new(),
            tooltip_index: HashMap::new(),
            default_style: None,
            default_interpolate: None,
            warnings: Vec::new(),
        }
    }

    /// The model the DB holds, and the warnings it logged while parsing.
    pub(crate) fn into_model(self, title: String) -> (FlowchartDiagram, Vec<String>) {
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
        (model, self.warnings)
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

    /// `addVertex`. `metadata` is the text of the shape data (`@{ … }`) after the id, if any: a
    /// subgraph id takes it as its metadata, an edge id as its edge data, and a vertex takes its
    /// fields. Each check comes in the order of the reference, and an id that is already an edge's
    /// id returns before a vertex is made, with or without shape data.
    pub(crate) fn add_vertex(
        &mut self,
        id: &str,
        text: Option<FlowText>,
        shape: Option<&str>,
        styles: Option<&[String]>,
        props: Option<(String, String)>,
        metadata: Option<&str>,
    ) -> Result<(), DbError> {
        if js_trim(id).is_empty() {
            return Ok(());
        }
        let doc = match metadata {
            Some(text) => parse_shape_data(text)?,
            None => None,
        };
        if let Some(pairs) = doc.as_ref().and_then(mapping)
            && let Some(&index) = self.subgraph_index.get(id)
        {
            let metadata = &mut self.subgraphs[index].metadata;
            for (key, value) in pairs {
                metadata.insert(key.clone(), value.to_json());
            }
            return Ok(());
        }
        if let Some(&index) = self.edge_index.get(id) {
            if let Some(pairs) = doc.as_ref().and_then(mapping) {
                apply_edge_data(&mut self.edges[index], pairs);
            }
            return Ok(());
        }

        let index = match self.vertex_index.get(id) {
            Some(&index) => index,
            None => {
                if text.is_none() && shape.is_none() && styles.is_some() {
                    self.warnings.push(format!(
                        "Style applied to unknown node \"{id}\". This may indicate a typo. The node will be created automatically."
                    ));
                }
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
        if let Some(styles) = styles {
            vertex.styles.extend(styles.iter().cloned());
        }
        if let Some((field, value)) = props {
            vertex.props.insert(field, value);
        }
        if let Some(doc) = doc {
            apply_vertex_data(vertex, &doc)?;
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

    /// `addClass`: each class of `ids` gets the styles, split at `;` after `,` is read as a
    /// separator, as the reference splits them. The class is made the first time it is named.
    pub(crate) fn add_class(&mut self, ids: &str, styles: &[String]) {
        let joined = styles.join(",").replace("\\,", "\u{0}").replace(',', ";");
        let parts: Vec<String> = joined
            .replace('\u{0}', ",")
            .split(';')
            .map(str::to_owned)
            .collect();
        for id in ids.split(',') {
            let index = match self.class_index.get(id) {
                Some(&index) => index,
                None => {
                    self.classes.push(FlowClass {
                        id: id.to_owned(),
                        styles: Vec::new(),
                    });
                    let index = self.classes.len() - 1;
                    self.class_index.insert(id.to_owned(), index);
                    index
                }
            };
            self.classes[index].styles.extend(parts.iter().cloned());
        }
    }

    /// `setClass`: adds the class to the vertex, the first edge and the subgraph that have each
    /// id of the comma-separated `ids`; an id that names none of them is ignored.
    pub(crate) fn set_class(&mut self, ids: &str, class: &str) {
        for id in ids.split(',') {
            if let Some(&index) = self.vertex_index.get(id) {
                self.vertices[index].classes.push(class.to_owned());
            }
            if let Some(&index) = self.edge_index.get(id) {
                self.edges[index].classes.push(class.to_owned());
            }
            if let Some(&index) = self.subgraph_index.get(id) {
                self.subgraphs[index].classes.push(class.to_owned());
            }
        }
    }

    /// `updateLink` with a style list: the default, or the edges at the given positions. A style
    /// without a fill gets `fill:none` added, as the reference does.
    pub(crate) fn update_link(
        &mut self,
        positions: &[LinkPosition],
        styles: &[String],
    ) -> Result<(), DbError> {
        for position in positions {
            match position {
                LinkPosition::Default => self.default_style = Some(styles.to_vec()),
                LinkPosition::Index(index) => {
                    let index = self.edge_at(index)?;
                    let edge = &mut self.edges[index];
                    let mut style = styles.to_vec();
                    if !style.is_empty() && !style.iter().any(|s| s.starts_with("fill")) {
                        style.push(String::from("fill:none"));
                    }
                    edge.style = Some(style);
                }
            }
        }
        Ok(())
    }

    /// `updateLinkInterpolate`: the default, or the edges at the given positions.
    pub(crate) fn update_link_interpolate(
        &mut self,
        positions: &[LinkPosition],
        interpolate: &str,
    ) -> Result<(), DbError> {
        for position in positions {
            match position {
                LinkPosition::Default => self.default_interpolate = Some(interpolate.to_owned()),
                LinkPosition::Index(index) => {
                    let index = self.edge_at(index)?;
                    self.edges[index].interpolate = Some(interpolate.to_owned());
                }
            }
        }
        Ok(())
    }

    /// The position of the edge that a `linkStyle` index names. The index must be in range; the
    /// reference reports an index past the last edge.
    fn edge_at(&self, index: &str) -> Result<usize, DbError> {
        let position = index
            .parse::<usize>()
            .ok()
            .filter(|_| !index.starts_with('0') || index == "0");
        match position {
            Some(position) if position < self.edges.len() => Ok(position),
            _ => Err(DbError(format!(
                "The index {index} for linkStyle is out of bounds. Valid indices for linkStyle are between 0 and {}. (Help: Ensure that the index is within the range of existing edges.)",
                self.edges.len().saturating_sub(1)
            ))),
        }
    }

    /// `setClickEvent`: the callback itself is not in the model (it runs in a browser, and only
    /// under `securityLevel: loose`); the classes the reference adds are.
    pub(crate) fn set_click_event(&mut self, ids: &str) {
        self.set_class(ids, "clickable");
    }

    /// `setLink`: the link and its target, for the vertices that exist now, then the `clickable`
    /// class. A link that is empty after trimming is no link.
    pub(crate) fn set_link(&mut self, ids: &str, link: &str, target: Option<&str>) {
        let trimmed = js_trim(link);
        let url = (!trimmed.is_empty()).then(|| trimmed.to_owned());
        for id in ids.split(',') {
            if let Some(&index) = self.vertex_index.get(id) {
                self.vertices[index].link = url.clone();
                self.vertices[index].link_target = target.map(str::to_owned);
            }
        }
        self.set_class(ids, "clickable");
    }

    /// `setTooltip`: the tooltip of each id, kept by id whether or not a vertex has it yet.
    pub(crate) fn set_tooltip(&mut self, ids: &str, tooltip: &str) {
        for id in ids.split(',') {
            match self.tooltip_index.get(id) {
                Some(&index) => self.tooltips[index].tooltip = tooltip.to_owned(),
                None => {
                    self.tooltips.push(FlowTooltip {
                        id: id.to_owned(),
                        tooltip: tooltip.to_owned(),
                    });
                    self.tooltip_index
                        .insert(id.to_owned(), self.tooltips.len() - 1);
                }
            }
        }
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

/// `yaml.load(…, JSON_SCHEMA)` of the shape data. A single line is wrapped in braces, as the
/// reference does; a line break inside it makes it a block mapping.
fn parse_shape_data(metadata: &str) -> Result<Option<Yaml>, DbError> {
    let text = if metadata.contains('\n') {
        format!("{metadata}\n")
    } else {
        format!("{{\n{metadata}\n}}")
    };
    crate::yaml::load(&text).map_err(|error| DbError(error.message))
}

fn mapping(doc: &Yaml) -> Option<&[(String, Yaml)]> {
    match doc {
        Yaml::Mapping(pairs) => Some(pairs),
        _ => None,
    }
}

fn field<'a>(pairs: &'a [(String, Yaml)], key: &str) -> Option<&'a Yaml> {
    pairs
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}

/// JavaScript's truthiness of a YAML value.
fn truthy(value: &Yaml) -> bool {
    match value {
        Yaml::Null => false,
        Yaml::Bool(flag) => *flag,
        Yaml::Number(number) => *number != 0.0 && !number.is_nan(),
        Yaml::String(text) => !text.is_empty(),
        Yaml::Sequence(_) | Yaml::Mapping(_) => true,
    }
}

/// JavaScript's `Number(…)` of a YAML value.
fn js_number(value: &Yaml) -> f64 {
    match value {
        Yaml::Null => 0.0,
        Yaml::Bool(flag) => f64::from(u8::from(*flag)),
        Yaml::Number(number) => *number,
        Yaml::String(text) => {
            let trimmed = js_trim(text);
            if trimmed.is_empty() {
                0.0
            } else {
                trimmed.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Yaml::Sequence(_) | Yaml::Mapping(_) => f64::NAN,
    }
}

/// `sanitizeNodeLabelType`: the three kinds are kept, anything else is Markdown.
fn label_kind(value: Option<&Yaml>) -> LabelType {
    match value {
        Some(Yaml::String(kind)) if kind == "text" => LabelType::Text,
        Some(Yaml::String(kind)) if kind == "string" => LabelType::String,
        _ => LabelType::Markdown,
    }
}

/// The edge fields of shape data: `animate`, `animation` and `curve` (the interpolation).
fn apply_edge_data(edge: &mut FlowEdge, pairs: &[(String, Yaml)]) {
    if let Some(value) = field(pairs, "animate") {
        match value {
            Yaml::Bool(flag) => edge.animate = Some(*flag),
            Yaml::Null => edge.animate = None,
            _ => {}
        }
    }
    if let Some(value) = field(pairs, "animation") {
        edge.animation = (!matches!(value, Yaml::Null)).then(|| value.js_string());
    }
    if let Some(value) = field(pairs, "curve") {
        edge.interpolate = (!matches!(value, Yaml::Null)).then(|| value.js_string());
    }
}

/// The vertex fields of shape data, applied in the reference's order: shape, label, icon, form,
/// position, image, constraint, and the width and height of the asset.
fn apply_vertex_data(vertex: &mut FlowVertex, doc: &Yaml) -> Result<(), DbError> {
    let Some(pairs) = mapping(doc) else {
        return Ok(());
    };
    if let Some(shape) = field(pairs, "shape").filter(|value| truthy(value)) {
        let Yaml::String(name) = shape else {
            return Err(DbError(format!("No such shape: {}.", shape.js_string())));
        };
        if name != &name.to_lowercase() || name.contains('_') {
            return Err(DbError(format!(
                "No such shape: {name}. Shape names should be lowercase."
            )));
        }
        if !is_valid_shape(name) {
            return Err(DbError(format!("No such shape: {name}.")));
        }
        vertex.shape = Some(name.clone());
    }
    let label = field(pairs, "label").filter(|value| truthy(value));
    if let Some(label) = label {
        vertex.text = label.js_string();
        vertex.label_type = label_kind(field(pairs, "labelType"));
    }
    // A blank label with an icon or an image leaves the text empty, when the text is still the id.
    let label_blank = match field(pairs, "label") {
        None | Some(Yaml::Null) => true,
        Some(Yaml::String(text)) => js_trim(text).is_empty(),
        Some(_) => false,
    };
    let blank_text_if_no_label = |vertex: &mut FlowVertex| {
        if label_blank && vertex.text == vertex.id {
            vertex.text = String::new();
        }
    };
    if let Some(icon) = field(pairs, "icon").filter(|value| truthy(value)) {
        vertex.icon = Some(icon.js_string());
        blank_text_if_no_label(vertex);
    }
    if let Some(form) = field(pairs, "form").filter(|value| truthy(value)) {
        vertex.form = Some(form.js_string());
    }
    if let Some(pos) = field(pairs, "pos").filter(|value| truthy(value)) {
        vertex.pos = Some(pos.js_string());
    }
    if let Some(img) = field(pairs, "img").filter(|value| truthy(value)) {
        vertex.img = Some(img.js_string());
        blank_text_if_no_label(vertex);
    }
    if let Some(constraint) = field(pairs, "constraint").filter(|value| truthy(value)) {
        vertex.constraint = Some(constraint.js_string());
    }
    if let Some(width) = field(pairs, "w").filter(|value| truthy(value)) {
        vertex.asset_width = Some(js_number(width));
    }
    if let Some(height) = field(pairs, "h").filter(|value| truthy(value)) {
        vertex.asset_height = Some(js_number(height));
    }
    Ok(())
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
