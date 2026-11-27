//! The steps of the unified renderer after a diagram's `getData`: the edge render data, the measure
//! step of `createGraphWithElements`, the elkjs graph and ELK Layered, the post-processing of the
//! result, and the paint of `paintLayoutData` (`rendering-util/render.ts`,
//! `layout-algorithms/common/index.ts`, `createGraph.ts` and `elk/render.ts`).
//!
//! A diagram on the unified renderer (flowchart, class, state, ER) builds its layout data, runs
//! [`measure`], [`lay_out`] and [`paint`], and writes the root `<svg>` around the painted groups,
//! with its own markers, style sheet and title.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use ferrigura_elk::json::JsonNode;
use ferrigura_model::flowchart::LabelType;

use crate::RenderError;
use crate::unified::data::{Edge, LayoutData, Node};
use crate::unified::geom::BBox;
use crate::unified::label::{Anchor, LabelLayout};
use crate::unified::shapes::{self, ShapeInput, ShapeOutput};
use crate::unified::{Config, clusters, edges, elk_apply, elk_build, loops, terminal};

/// The graph handed to ELK and the graph it returned, for the differential comparison with the
/// reference's capture (`tools/port/geometry/flowchart-geometry.ts`).
#[derive(Debug, Clone)]
pub struct ElkStages {
    /// The input of the layout.
    pub input: JsonNode,
    /// The output of the layout.
    pub output: JsonNode,
}

/// The diagram after the measure step: the layout data with every size the layout needs, and
/// what was drawn to measure it.
#[derive(Debug, Clone)]
pub struct Measured {
    /// The layout data, with the sizes of the measure step and, after [`lay_out`], the positions.
    pub data: LayoutData,
    /// The shape of every leaf node, by id.
    pub shapes: HashMap<String, ShapeOutput>,
    /// The label of every edge that has one, by edge id.
    pub edge_labels: HashMap<String, LabelLayout>,
}

/// What [`paint`] drew: the groups below the root, the box they cover, and every label, so that
/// the caller can subset the fonts once for the whole diagram.
#[derive(Debug)]
pub struct Painted<'a> {
    /// `<g class="root">` with the clusters, the edge paths, the edge labels and the nodes.
    pub root: String,
    /// The markers in an edge's own color, which the reference appends after the root group.
    pub coloured_markers: String,
    /// Every box the painted elements cover: what `getBBox` of the svg gives, before the title.
    pub bounds: Option<BBox>,
    /// The cluster titles that were drawn.
    pub cluster_labels: Vec<LabelLayout>,
    /// The node and edge labels that were drawn.
    pub labels: Vec<&'a LabelLayout>,
    /// Whether a node is wrapped in a link, so that the root declares the `xlink` prefix.
    pub links: bool,
}

#[derive(Debug)]
pub(crate) struct RootParts {
    clusters: String,
    edge_paths: String,
    edge_labels: String,
    nodes: String,
    animation: String,
}

impl RootParts {
    pub(crate) fn slices(&self) -> [&str; 11] {
        [
            "<g class=\"root\"><g class=\"clusters\">",
            &self.clusters,
            "</g><g class=\"edges edgePaths\">",
            &self.edge_paths,
            "</g><g class=\"edgeLabels\">",
            &self.edge_labels,
            "</g><g class=\"nodes\">",
            &self.nodes,
            "</g>",
            &self.animation,
            "</g>",
        ]
    }
}

pub(crate) struct PaintedParts<'a> {
    pub root: RootParts,
    pub coloured_markers: String,
    pub bounds: Option<BBox>,
    pub cluster_labels: Vec<LabelLayout>,
    pub labels: Vec<&'a LabelLayout>,
    pub links: bool,
}

impl PaintedParts<'_> {
    pub(crate) fn all_labels(&self) -> impl Iterator<Item = &LabelLayout> {
        self.cluster_labels
            .iter()
            .chain(self.labels.iter().copied())
    }
}

impl Painted<'_> {
    /// Every label that was drawn: the cluster titles, then the node and edge labels.
    pub fn all_labels(&self) -> impl Iterator<Item = &LabelLayout> {
        self.cluster_labels
            .iter()
            .chain(self.labels.iter().copied())
    }
}

/// The nearest `dir` of the node or one of its subgraphs, else the diagram's direction, as
/// `resolveNodeDir` in `createGraph.ts` finds it.
pub(crate) fn node_dir<'a>(
    data: &'a LayoutData,
    index: usize,
    by_id: &HashMap<&str, &'a Node>,
) -> Option<&'a str> {
    let mut current = Some(&data.nodes[index]);
    let mut seen = std::collections::HashSet::new();
    while let Some(node) = current {
        if let Some(dir) = node.dir.as_deref() {
            return Some(dir);
        }
        let parent = node.parent_id.as_deref()?;
        if !seen.insert(parent) {
            break;
        }
        current = by_id.get(parent).copied();
    }
    Some(data.direction.as_str()).filter(|dir| !dir.is_empty())
}

/// For each node, the length its busiest side needs in the design mode
/// ([`crate::design::side_length`]) and whether that side runs along the node's height: it does in
/// a layout from left to right, where edges meet the left and the right side, and it runs along
/// the width in a layout from top to bottom. The edges into a node meet one side and the edges out
/// of it the other; a self-loop is left out.
fn busiest_sides(data: &LayoutData) -> Vec<(f64, bool)> {
    let by_id: HashMap<&str, &Node> = data
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let mut degrees: HashMap<&str, (usize, usize)> = HashMap::new();
    // The connector of a state's note is no transition: the note stands beside its state, and its
    // connector meets a side that no transition does.
    let is_note = |id: &str| {
        by_id
            .get(id)
            .is_some_and(|node| node.css_classes.contains("statediagram-note"))
    };
    for edge in &data.edges {
        if edge.start == edge.end || is_note(&edge.start) || is_note(&edge.end) {
            continue;
        }
        degrees.entry(edge.start.as_str()).or_default().1 += 1;
        degrees.entry(edge.end.as_str()).or_default().0 += 1;
    }
    data.nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let (into, out) = degrees.get(node.id.as_str()).copied().unwrap_or_default();
            // `node_dir` gives a node without a parent no direction; the diagram's is meant here.
            let direction = node_dir(data, index, &by_id).unwrap_or(data.direction.as_str());
            let along_height = matches!(direction, "LR" | "RL");
            (crate::design::side_length(into.max(out)), along_height)
        })
        .collect()
}

/// The edge render data and the measure step of `createGraphWithElements`, for the layout data
/// of a diagram's `getData`, laid out by ELK. `config` gives the sections and the globals, and the
/// theme's tokens give the sizes.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or a shape is unknown.
pub fn measure(mut data: LayoutData, config: &Config<'_>) -> Result<Measured, RenderError> {
    edges::prepare(&mut data);

    // Direction is only consumed by fork/join shapes. Resolve those before mutating sizes,
    // building the parent index once instead of once per node.
    let directions = if data
        .nodes
        .iter()
        .any(|node| !node.is_group && shapes::uses_direction(node))
    {
        let by_id: HashMap<&str, &Node> = data
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        data.nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                (!node.is_group && shapes::uses_direction(node))
                    .then(|| node_dir(&data, index, &by_id).map(str::to_owned))
                    .flatten()
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    // The design mode centers the label of a node in every diagram type it draws.
    let anchor = if config.section.design.is_some() {
        Anchor::Middle
    } else {
        node_anchor(&data.diagram_type)
    };
    let busiest = if config.section.design.is_some() {
        busiest_sides(&data)
    } else {
        Vec::new()
    };
    // The nodes with a self-loop that the design mode draws beside them.
    let mut shapes = HashMap::new();
    for index in 0..data.nodes.len() {
        let node = &data.nodes[index];
        if node.is_group {
            // ELK sizes a subgraph from its title, measured unwrapped unless it is Markdown. The
            // title's box is already a float32 value (`node_label`).
            let unwrapped = node.label_type != LabelType::Markdown;
            let width = unwrapped.then_some(f64::INFINITY);
            // `measureGroupLabel` gives a group without a label no size (`createGraph.ts` 56-58).
            let size = if node.label.is_empty() {
                (0.0, 0.0)
            } else {
                shapes::node_label(node, config, width, anchor)?
                    .map_or((0.0, 0.0), |label| (label.bbox.width, label.bbox.height))
            };
            data.nodes[index].label_bbox = Some(size);
            continue;
        }
        // A `labelRect` is a label that the stylesheet does not center: its lines start
        // at the origin, and `labelRect.ts` moves the label by half its width (`shapes/rect.rs`).
        // A note of a state or a class diagram is a block of text, which the design mode keeps at
        // the left.
        let label_anchor = if node.shape == "labelRect"
            || (node.shape == "note" && config.section.roles != crate::design::Roles::Flowchart)
        {
            Anchor::Start
        } else {
            anchor
        };
        let label = shapes::node_label(node, config, None, label_anchor)?;
        let mut shape = shapes::draw(&ShapeInput {
            node,
            label: label.as_ref(),
            config,
            dir: directions.get(index).and_then(Option::as_deref),
        })?;
        // The design mode gives a node room for the edges of its busiest side: it draws the shape
        // again with that length as its size along the side, and keeps the larger drawing.
        let grows =
            !(config.section.roles == crate::design::Roles::State && shapes::is_choice(node));
        let lengths: Vec<(f64, bool)> = busiest.get(index).copied().into_iter().collect();
        let mut grown = None;
        for (length, along_height) in lengths.into_iter().filter(|_| grows) {
            let current = if along_height {
                shape.bbox.height
            } else {
                shape.bbox.width
            };
            if length > current {
                let node = grown.get_or_insert_with(|| node.clone());
                if along_height {
                    node.height = Some(length);
                } else {
                    node.width = Some(length);
                }
            }
        }
        if let Some(grown) = grown {
            let label = shapes::node_label(&grown, config, None, label_anchor)?;
            let redrawn = shapes::draw(&ShapeInput {
                node: &grown,
                label: label.as_ref(),
                config,
                dir: directions.get(index).and_then(Option::as_deref),
            })?;
            if redrawn.bbox.width * redrawn.bbox.height > shape.bbox.width * shape.bbox.height {
                shape = redrawn;
            }
        }
        let id = node.id.clone();
        // `insertMeasuredNode` takes the node's size from the `getBBox` of its group (createGraph.ts
        // 61-71, line 67), so it is a float32 value.
        let bbox = shape.bbox.browser();
        data.nodes[index].width = Some(bbox.width);
        data.nodes[index].height = Some(bbox.height);
        shapes.insert(id, shape);
    }

    let mut edge_labels = HashMap::new();
    let font_size = config.theme.font_size.px().unwrap_or(DEFAULT_FONT_SIZE);
    for edge in &mut data.edges {
        if let Some(label) = edges::label(
            edge,
            font_size,
            &config.fonts,
            config.section.design.is_some(),
        )? {
            let (mut width, height) = edges::label_size(&label);
            // The design mode draws the label in a chip; the layout is given the chip's width and
            // the room around it.
            if config.section.design.is_some() {
                width += 2.0 * (crate::design::CHIP_PADDING + crate::design::CHIP_MARGIN);
            }
            edge.width = Some(width);
            edge.height = Some(height);
            edge_labels.insert(edge.id.clone(), label);
        }
        edge.terminals = terminal::measure(edge, &config.fonts)?;
    }

    Ok(Measured {
        data,
        shapes,
        edge_labels,
    })
}

/// Where the lines of a node's label start on `x = 0`. The flowchart's stylesheet centers node labels
/// (`.node .label text`); the state and class diagrams' do not, so their lines start at the label's
/// origin (`state/styles.js` and `class/styles.js` have no text-anchor for them).
#[must_use]
pub fn node_anchor(diagram_type: &str) -> Anchor {
    // The mindmap's stylesheet does not center its labels either: its `centerLabel` offsets the label
    // by half its width instead (`labelHelper`, `util.ts` 149-151).
    if diagram_type == crate::state::DIAGRAM_TYPE
        || diagram_type == crate::unified::markers::CLASS
        || diagram_type == crate::mindmap::data::DIAGRAM_TYPE
    {
        Anchor::Start
    } else {
        Anchor::Middle
    }
}

/// Lays the measured diagram out with ELK and applies the result.
///
/// # Errors
///
/// Returns an error if the layout fails.
pub fn lay_out(measured: &mut Measured, config: &Config<'_>) -> Result<ElkStages, RenderError> {
    let elk = elk_build::build(&measured.data, config);
    let layout = if own_placement(config) {
        ferrigura_elk::placement::layout_with(&elk.graph, false, crate::flowchart::placement::Axes)
    } else {
        ferrigura_elk::layout(&elk.graph, false)
    }
    .map_err(|error| RenderError::Layout(error.to_string()))?;
    apply_layout(measured, config, &layout.graph, &elk.state);
    Ok(ElkStages {
        input: elk.graph,
        output: layout.graph,
    })
}

/// Lays out a diagram without retaining diagnostic ELK stages.
pub(crate) fn lay_out_for_render(
    measured: &mut Measured,
    config: &Config<'_>,
) -> Result<(), RenderError> {
    if config.section.design.is_some() {
        return lay_out_adjusted(measured, config, &[], |_, _, _| true).map(|_| ());
    }
    let elk = elk_build::build(&measured.data, config);
    let layout = ferrigura_elk::layout_owned(elk.graph, false)
        .map_err(|error| RenderError::Layout(error.to_string()))?;
    apply_layout(measured, config, &layout.graph, &elk.state);
    Ok(())
}

/// Lays out a diagram as [`lay_out_for_render`] does, with a step of the caller's between ELK's
/// layout and its application: `adjust` gets ELK's result, the layout data and the side tables,
/// and may change all three. When it returns `false`, nothing is applied and the result is
/// `false`; the caller then lays the diagram out another way.
///
/// In the design mode, the self-loops stand beside their nodes ([`loops`]): a node whose loops
/// cannot is laid out again with its loops on the sides of the other direction, then with its
/// loops where ELK puts them. The nodes of `reserved` have their
/// boxes grown for something of the caller's beside them already.
///
/// # Errors
///
/// Returns an error if the layout fails.
pub(crate) fn lay_out_adjusted(
    measured: &mut Measured,
    config: &Config<'_>,
    reserved: &[loops::Reserved],
    mut adjust: impl FnMut(&mut JsonNode, &mut LayoutData, &mut elk_build::ElkState) -> bool,
) -> Result<bool, RenderError> {
    if config.section.design.is_none() {
        return lay_out_once(measured, config, adjust);
    }
    let original = measured.data.clone();
    let (mut kept, mut turned) = (
        std::collections::HashSet::new(),
        std::collections::HashSet::new(),
    );
    while let Some(beside) = loops::take_out(&mut measured.data, &kept, &turned, reserved) {
        let mut failed = None;
        let laid = lay_out_once(measured, config, |graph, data, state| {
            if let Err(host) = beside.put_back(graph, data, state) {
                failed = Some(host);
                return false;
            }
            adjust(graph, data, state)
        })?;
        if laid {
            failed = beside.blocked(&measured.data);
            if failed.is_none() {
                return Ok(true);
            }
        } else if failed.is_none() {
            // The caller's step declined the layout.
            return Ok(false);
        }
        measured.data.clone_from(&original);
        // A node whose loops do not fit tries the sides of the other direction, then keeps them
        // where ELK puts them.
        let Some(host) = failed else {
            break;
        };
        if !turned.insert(host.clone()) && !kept.insert(host) {
            break;
        }
    }
    measured.data = original;
    lay_out_once(measured, config, adjust)
}

/// One layout with the caller's step: see [`lay_out_adjusted`].
fn lay_out_once(
    measured: &mut Measured,
    config: &Config<'_>,
    adjust: impl FnOnce(&mut JsonNode, &mut LayoutData, &mut elk_build::ElkState) -> bool,
) -> Result<bool, RenderError> {
    let elk = elk_build::build(&measured.data, config);
    let mut layout = if own_placement(config) {
        ferrigura_elk::placement::layout_with(&elk.graph, false, crate::flowchart::placement::Axes)
    } else {
        ferrigura_elk::layout_owned(elk.graph, false)
    }
    .map_err(|error| RenderError::Layout(error.to_string()))?;
    let mut state = elk.state;
    if !adjust(&mut layout.graph, &mut measured.data, &mut state) {
        return Ok(false);
    }
    apply_layout(measured, config, &layout.graph, &state);
    Ok(true)
}

/// Whether the layout runs with the design mode's own node placement, on the layers and the order
/// ELK decided (`crate::flowchart::placement`): in the design mode, unless the configuration
/// names a placement strategy, which is the user's choice.
fn own_placement(config: &Config<'_>) -> bool {
    config.section.design.is_some() && config.section.elk.node_placement_strategy.is_none()
}

fn apply_layout(
    measured: &mut Measured,
    config: &Config<'_>,
    graph: &JsonNode,
    state: &elk_build::ElkState,
) {
    let crossing = |id: &str, node, probe| {
        measured
            .shapes
            .get(id)
            .map(|shape| shape.intersect.intersect(node, probe))
    };
    elk_apply::apply_with_crossing(
        &mut measured.data,
        graph,
        state,
        &crossing,
        config.section.elk.straighten_edges,
    );
    // The design mode takes the branch of a decision or a choice to a node beside it out of the
    // side tip.
    if config.section.design.is_some() {
        crate::flowchart::turns::side_exits(&mut measured.data);
        // A class relation's cardinalities stand beside its ends.
        if measured.data.diagram_type == super::markers::CLASS {
            crate::class::tree::merge_generalizations(
                &mut measured.data.edges,
                &measured.data.nodes,
            );
            super::terminal::place_beside_ends(&mut measured.data.edges, &measured.data.nodes);
        }
    }
}

/// Which accessibility elements (`<title>`, `<desc>`) the caller emits on the root `<svg>`.
#[derive(Clone, Copy, Default)]
pub(crate) struct A11yIds {
    pub(crate) title: bool,
    pub(crate) description: bool,
}

/// Paints a laid-out diagram in the order of `paintLayoutData`: the clusters in the order
/// `orderNodesForElkPaint` left, then the nodes as the measure step inserted them, then the edges
/// and their labels. `id` is the diagram's id, which prefixes every DOM id.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn paint<'a>(
    measured: &'a Measured,
    id: &str,
    config: &Config<'_>,
) -> Result<Painted<'a>, RenderError> {
    paint_for_diagram(measured, id, config, A11yIds::default())
}

pub(crate) fn paint_for_diagram<'a>(
    measured: &'a Measured,
    id: &str,
    config: &Config<'_>,
    a11y: A11yIds,
) -> Result<Painted<'a>, RenderError> {
    let parts = paint_with_animation(measured, id, config, None, a11y)?;
    Ok(Painted {
        root: parts.root.slices().concat(),
        coloured_markers: parts.coloured_markers,
        bounds: parts.bounds,
        cluster_labels: parts.cluster_labels,
        labels: parts.labels,
        links: parts.links,
    })
}

/// Applies XML 1.0 attribute-value whitespace normalization (`\r\n`, `\t`, `\n`, `\r` -> `' '`)
/// and [`crate::svg::escape`]'s invalid-XML-character replacement (`!is_xml_char` -> U+FFFD).
pub(crate) fn canonical_xml_attr(raw: &str) -> Cow<'_, str> {
    if !raw
        .chars()
        .any(|ch| matches!(ch, '\t' | '\n' | '\r') || !crate::svg::is_xml_char(ch))
    {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push(' ');
            }
            '\t' | '\n' => out.push(' '),
            other if !crate::svg::is_xml_char(other) => out.push('\u{fffd}'),
            other => out.push(other),
        }
    }
    Cow::Owned(out)
}

/// Resolves Mermaid entity placeholders once and applies [`canonical_xml_attr`] so collision
/// checks, emitted IDs, and fragment links agree with the serialized XML parser values.
pub(crate) fn canonical_attr_id(raw: &str) -> String {
    let resolved = crate::common::resolve_entity_placeholders(raw);
    canonical_xml_attr(resolved.as_ref()).into_owned()
}

/// Whether a raw node or edge ID contains a Mermaid entity placeholder, XML-normalizable
/// whitespace, or a character that XML 1.0 replaces with U+FFFD.
fn needs_id_normalization(raw: &str) -> bool {
    raw.contains("ﬂ°")
        || raw.contains("¶ß")
        || raw
            .chars()
            .any(|ch| matches!(ch, '\t' | '\n' | '\r') || !crate::svg::is_xml_char(ch))
}

/// Optional renderer-owned SVG IDs actually emitted for the current render.
#[derive(Clone, Copy)]
pub(crate) struct ReservedDefs {
    pub(crate) gradient: bool,
    pub(crate) label_gaps: bool,
    pub(crate) a11y_title: bool,
    pub(crate) a11y_desc: bool,
}

/// Checks whether `{a}-{b} == {prefix}{suffix}` without allocating.
fn matches_prefixed(a: &str, b: &str, prefix: &str, suffix: &str) -> bool {
    a.len() + 1 + b.len() == prefix.len() + suffix.len()
        && a.bytes()
            .chain(std::iter::once(b'-'))
            .chain(b.bytes())
            .eq(prefix.bytes().chain(suffix.bytes()))
}

/// Whether `candidate` matches an SVG ID generated by the unified renderer for the current render
/// (root `<svg>`, emitted accessibility `<title>`/`<desc>`, look/design `<filter>`s, and optional
/// `<linearGradient>` or label-gap `<mask>` definitions).
fn is_renderer_owned_id(candidate: &str, canonical_svg_id: &str, defs: ReservedDefs) -> bool {
    if candidate == canonical_svg_id
        || (defs.a11y_title && candidate.strip_prefix("chart-title-") == Some(canonical_svg_id))
        || (defs.a11y_desc && candidate.strip_prefix("chart-desc-") == Some(canonical_svg_id))
    {
        return true;
    }
    let Some(suffix) = candidate
        .strip_prefix(canonical_svg_id)
        .and_then(|rest| rest.strip_prefix('-'))
    else {
        return false;
    };
    is_renderer_owned_suffix(suffix, defs)
}

/// Whether the local part after `{svg_id}-` matches a renderer-owned definition ID emitted by the
/// current render.
fn is_renderer_owned_suffix(suffix: &str, defs: ReservedDefs) -> bool {
    match suffix {
        "drop-shadow" | "drop-shadow-small" => true,
        "gradient" => defs.gradient,
        "label-gaps" => defs.label_gaps,
        _ => false,
    }
}

/// Whether `{canonical_svg_id}-{local}` collides with a renderer-owned ID emitted by the current
/// render, checked without allocating.
fn is_renderer_owned_local(local: &str, canonical_svg_id: &str, defs: ReservedDefs) -> bool {
    is_renderer_owned_suffix(local, defs)
        || (defs.a11y_title
            && matches_prefixed(canonical_svg_id, local, "chart-title-", canonical_svg_id))
        || (defs.a11y_desc
            && matches_prefixed(canonical_svg_id, local, "chart-desc-", canonical_svg_id))
}

/// Serializes a canonical DOM ID as a same-document URL fragment (`#...`), percent-encoding `%`
/// and the WHATWG URL fragment percent-encode set (C0 controls, space, `"`, `<`, `>`, `` ` ``,
/// DEL, and non-ASCII UTF-8 bytes) so URL parsing preserves trailing spaces and percent-decoding
/// recovers the exact DOM ID.
fn format_fragment_url(id: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(1 + id.len());
    out.push('#');
    for byte in id.bytes() {
        if byte <= 0x20 || matches!(byte, b'"' | b'%' | b'<' | b'>' | b'`') || byte >= 0x7F {
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0F) as usize] as char);
        } else {
            out.push(byte as char);
        }
    }
    out
}

/// Percent-decodes valid `%XX` octets in a URL fragment into UTF-8 while keeping literal UTF-8
/// bytes intact. Returns `None` if the decoded bytes are not valid UTF-8.
fn percent_decode_fragment(fragment: &str) -> Option<String> {
    let bytes = fragment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (
                (bytes[index + 1] as char).to_digit(16),
                (bytes[index + 2] as char).to_digit(16),
            )
        {
            out.push(((hi << 4) | lo) as u8);
            index += 3;
            continue;
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(out).ok()
}

/// Strips a leading `#` fragment marker (after trimming leading/trailing C0 controls and ASCII
/// spaces, matching URL parsing), including when the marker itself was encoded as a Mermaid entity
/// placeholder (such as `#35;` -> `ﬂ°°35¶ß` or `#num;` -> `ﬂ°num¶ß`), returning the raw fragment
/// body without decoding the target ID.
fn strip_fragment_marker(raw_link: &str) -> Option<&str> {
    let trimmed = raw_link.trim_matches(|ch: char| ch <= ' ');
    if let Some(rest) = trimmed.strip_prefix('#') {
        return Some(rest);
    }
    if trimmed.starts_with("ﬂ°")
        && let Some(close) = trimmed.find("¶ß")
    {
        let end = close + "¶ß".len();
        if crate::common::resolve_entity_placeholders(&trimmed[..end]) == "#" {
            return Some(&trimmed[end..]);
        }
    }
    None
}

/// Pre-resolved, XML-normalized, collision-free DOM and edge IDs for one unified render pass.
struct ResolvedDomIds {
    nodes: Vec<String>,
    edge_ids: Vec<String>,
    element_owned: HashSet<String>,
    remapped_fragments: HashMap<String, String>,
}

impl ResolvedDomIds {
    /// Looks up the remapped fragment URL for `raw_link`, matching its percent-decoded fragment
    /// target first (so `#graph-classId-A%20B#45;C` targets `A B-C` — whether `A B-C` was
    /// remapped or kept unchanged — rather than a literal `A%20B-C`), then falling back to its
    /// raw spelling. Also recognizes entity-encoded leading `#` markers (such as `#35;`) and
    /// ignores leading/trailing URL whitespace (C0 controls and ASCII space).
    fn remap_fragment(&self, raw_link: &str) -> Option<Cow<'_, str>> {
        let trimmed = raw_link.trim_matches(|ch: char| ch <= ' ');
        let fragment = strip_fragment_marker(trimmed)?;
        if fragment.contains('%')
            && let Some(decoded) = percent_decode_fragment(fragment)
        {
            if self.element_owned.contains(&decoded) {
                return Some(Cow::Owned(format_fragment_url(&decoded)));
            }
            if let Some(target) = self.remapped_fragments.get(&format!("#{decoded}")) {
                return Some(Cow::Borrowed(target.as_str()));
            }
            let resolved_target = canonical_attr_id(&decoded);
            if self.element_owned.contains(&resolved_target) {
                return Some(Cow::Owned(format_fragment_url(&resolved_target)));
            }
        }
        if let Some(target) = if trimmed.starts_with('#') {
            self.remapped_fragments.get(trimmed)
        } else {
            self.remapped_fragments.get(&format!("#{fragment}"))
        } {
            return Some(Cow::Borrowed(target.as_str()));
        }
        let resolved_target = canonical_attr_id(fragment);
        if resolved_target != fragment && self.element_owned.contains(&resolved_target) {
            return Some(Cow::Owned(format_fragment_url(&resolved_target)));
        }
        None
    }

    /// Resolves and disambiguates node/cluster DOM IDs and edge IDs across the entire diagram in a
    /// single pass, reserving unchanged IDs before assigning decoded IDs so declaration order
    /// cannot steal an existing literal ID. Returns `None` without allocating when no ID contains
    /// a Mermaid entity placeholder, XML-normalizable whitespace, invalid XML control characters,
    /// or a collision with a renderer-owned SVG ID emitted by this render.
    fn resolve(data: &LayoutData, svg_id: &str, defs: ReservedDefs) -> Option<Self> {
        let canonical_svg_id = canonical_xml_attr(svg_id);
        let has_literal_collision = || {
            let mut local_seen: HashSet<&str> =
                HashSet::with_capacity(data.nodes.len() + data.edges.len());
            for node in &data.nodes {
                if !local_seen.insert(node.dom_id.as_str()) {
                    return true;
                }
            }
            for edge in &data.edges {
                if !local_seen.insert(edge.id.as_str()) {
                    return true;
                }
            }
            false
        };
        let needs_pass = canonical_svg_id != svg_id
            || data.nodes.iter().any(|node| {
                needs_id_normalization(&node.dom_id)
                    || is_renderer_owned_local(&node.dom_id, &canonical_svg_id, defs)
                    || node.link.as_deref().is_some_and(|link| {
                        strip_fragment_marker(link).is_some_and(|fragment| {
                            fragment.contains('%') || needs_id_normalization(fragment)
                        })
                    })
            })
            || data.edges.iter().any(|edge| {
                needs_id_normalization(&edge.id)
                    || is_renderer_owned_local(&edge.id, &canonical_svg_id, defs)
            })
            || has_literal_collision();
        if !needs_pass {
            return None;
        }

        let mut reserved: HashSet<String> =
            HashSet::with_capacity(data.nodes.len() + data.edges.len());
        let mut node_plans = Vec::with_capacity(data.nodes.len());
        for node in &data.nodes {
            let had_placeholder = node.dom_id.contains("ﬂ°") || node.dom_id.contains("¶ß");
            let canonical_local = canonical_attr_id(&node.dom_id);
            let canonical_full = format!("{canonical_svg_id}-{canonical_local}");
            let unchanged =
                !had_placeholder && canonical_local == node.dom_id && canonical_svg_id == svg_id;
            reserved.insert(canonical_full.clone());
            node_plans.push((had_placeholder, canonical_local, canonical_full, unchanged));
        }

        let mut edge_plans = Vec::with_capacity(data.edges.len());
        for edge in &data.edges {
            let had_placeholder = edge.id.contains("ﬂ°") || edge.id.contains("¶ß");
            let canonical_local = canonical_attr_id(&edge.id);
            let canonical_full = format!("{canonical_svg_id}-{canonical_local}");
            let unchanged =
                !had_placeholder && canonical_local == edge.id && canonical_svg_id == svg_id;
            reserved.insert(canonical_full.clone());
            edge_plans.push((had_placeholder, canonical_local, canonical_full, unchanged));
        }

        // Pass 1: claim unchanged literal IDs first so an earlier decoded/normalized ID cannot
        // steal an existing element's ID and break external selectors or links.
        let mut seen: HashSet<String> = HashSet::with_capacity(reserved.len());
        let mut element_owned: HashSet<String> = HashSet::with_capacity(reserved.len());
        let mut node_claimed = vec![false; node_plans.len()];
        for (index, (_, _, canonical_full, unchanged)) in node_plans.iter().enumerate() {
            if *unchanged
                && !is_renderer_owned_id(canonical_full, &canonical_svg_id, defs)
                && seen.insert(canonical_full.clone())
            {
                element_owned.insert(canonical_full.clone());
                node_claimed[index] = true;
            }
        }
        let mut edge_claimed = vec![false; edge_plans.len()];
        for (index, (_, _, canonical_full, unchanged)) in edge_plans.iter().enumerate() {
            if *unchanged
                && !is_renderer_owned_id(canonical_full, &canonical_svg_id, defs)
                && seen.insert(canonical_full.clone())
            {
                element_owned.insert(canonical_full.clone());
                edge_claimed[index] = true;
            }
        }

        let mut next_suffix: HashMap<String, usize> = HashMap::new();
        let mut remapped_fragments: HashMap<String, String> = HashMap::new();

        // Pass 2: assign decoded, normalized, and colliding IDs.
        let mut nodes = Vec::with_capacity(data.nodes.len());
        for (index, (had_placeholder, canonical_local, canonical_full, _)) in
            node_plans.into_iter().enumerate()
        {
            let node = &data.nodes[index];
            if node_claimed[index] {
                nodes.push(node.dom_id.clone());
                continue;
            }
            let canonical_differs = canonical_local != node.dom_id;
            let collided = is_renderer_owned_id(&canonical_full, &canonical_svg_id, defs)
                || !seen.insert(canonical_full.clone());
            let final_local = if !collided {
                element_owned.insert(canonical_full.clone());
                canonical_local
            } else {
                let suffix = next_suffix.entry(canonical_full.clone()).or_insert(1);
                loop {
                    let candidate_local = format!("{canonical_local}-{}", *suffix);
                    let candidate_full = format!("{canonical_svg_id}-{candidate_local}");
                    *suffix += 1;
                    if !is_renderer_owned_id(&candidate_full, &canonical_svg_id, defs)
                        && !reserved.contains(&candidate_full)
                        && seen.insert(candidate_full.clone())
                    {
                        element_owned.insert(candidate_full);
                        break candidate_local;
                    }
                }
            };
            let normalized_changed =
                had_placeholder || final_local != node.dom_id || canonical_svg_id != svg_id;
            let raw_distinct_from_owner = had_placeholder
                || canonical_differs
                || canonical_svg_id != svg_id
                || !element_owned.contains(&canonical_full);
            if normalized_changed && raw_distinct_from_owner {
                let target_id = format!("{canonical_svg_id}-{final_local}");
                remapped_fragments
                    .entry(format!("#{svg_id}-{}", node.dom_id))
                    .or_insert_with(|| format_fragment_url(&target_id));
            }
            if collided || had_placeholder {
                nodes.push(final_local);
            } else {
                nodes.push(node.dom_id.clone());
            }
        }

        let mut edge_ids = Vec::with_capacity(data.edges.len());
        for (index, (had_placeholder, canonical_local, canonical_full, _)) in
            edge_plans.into_iter().enumerate()
        {
            let edge = &data.edges[index];
            if edge_claimed[index] {
                edge_ids.push(edge.id.clone());
                continue;
            }
            let canonical_differs = canonical_local != edge.id;
            let collided = is_renderer_owned_id(&canonical_full, &canonical_svg_id, defs)
                || !seen.insert(canonical_full.clone());
            let final_local = if !collided {
                element_owned.insert(canonical_full.clone());
                canonical_local
            } else {
                let suffix = next_suffix.entry(canonical_full.clone()).or_insert(1);
                loop {
                    let candidate_local = format!("{canonical_local}-{}", *suffix);
                    let candidate_full = format!("{canonical_svg_id}-{candidate_local}");
                    *suffix += 1;
                    if !is_renderer_owned_id(&candidate_full, &canonical_svg_id, defs)
                        && !reserved.contains(&candidate_full)
                        && seen.insert(candidate_full.clone())
                    {
                        element_owned.insert(candidate_full);
                        break candidate_local;
                    }
                }
            };
            let normalized_changed =
                had_placeholder || final_local != edge.id || canonical_svg_id != svg_id;
            let raw_distinct_from_owner = had_placeholder
                || canonical_differs
                || canonical_svg_id != svg_id
                || !element_owned.contains(&canonical_full);
            if normalized_changed && raw_distinct_from_owner {
                let target_id = format!("{canonical_svg_id}-{final_local}");
                remapped_fragments
                    .entry(format!("#{svg_id}-{}", edge.id))
                    .or_insert_with(|| format_fragment_url(&target_id));
            }
            if collided || had_placeholder {
                edge_ids.push(final_local);
            } else {
                edge_ids.push(edge.id.clone());
            }
        }

        Some(Self {
            nodes,
            edge_ids,
            element_owned,
            remapped_fragments,
        })
    }
}

pub(crate) fn paint_with_animation<'a>(
    measured: &'a Measured,
    id: &str,
    config: &Config<'_>,
    animation: Option<crate::animation::Animation>,
    a11y: A11yIds,
) -> Result<PaintedParts<'a>, RenderError> {
    let data = &measured.data;
    let reserved_defs = ReservedDefs {
        gradient: config.section.design.is_none() && config.theme.use_gradient.truthy(),
        label_gaps: config.section.design.is_some()
            && data.edges.iter().any(|edge| {
                measured
                    .edge_labels
                    .get(&edge.id)
                    .is_some_and(|label| label.background.is_some())
            }),
        a11y_title: a11y.title,
        a11y_desc: a11y.description,
    };
    let resolved_ids = ResolvedDomIds::resolve(data, id, reserved_defs);
    let mut bounds: Option<BBox> = None;
    let mut labels: Vec<&LabelLayout> = Vec::new();

    let mut clusters = String::new();
    let mut painted_clusters = Vec::new();
    for (index, node) in data
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.is_group)
    {
        // `render` prefixes every node's DOM id with the diagram's id before the layout runs.
        let dom_id = resolved_ids
            .as_ref()
            .map_or(node.dom_id.as_str(), |ids| ids.nodes[index].as_str());
        let prefixed = Node {
            dom_id: format!("{id}-{dom_id}"),
            ..node.clone()
        };
        let cluster = clusters::paint_among(&prefixed, config, &data.edges)?;
        // The frame of an empty subgraph has no height: an unrendered rect, which `getBBox` leaves
        // out of the drawing's box. Its title is text and counts.
        if !cluster.bbox.is_empty() {
            cover(&mut bounds, cluster.bbox);
        }
        if !cluster.title_box.is_empty() {
            cover(&mut bounds, cluster.title_box);
        }
        clusters.push_str(&cluster.svg);
        painted_clusters.push(cluster);
    }

    let mut nodes = String::new();
    let mut node_parts = Vec::new();
    let mut links = false;
    let loose = config.globals.security_level == "loose";
    for (index, node) in data
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| !node.is_group)
    {
        let Some(shape) = measured.shapes.get(&node.id) else {
            continue;
        };
        links |= node.link.is_some();
        let (x, y) = (node.x.unwrap_or(0.0), node.y.unwrap_or(0.0));
        cover(
            &mut bounds,
            shape.extent.unwrap_or(shape.bbox).translate(x, y),
        );
        // A state diagram's links are written after its render (`LinkForm::PostRender`).
        let link_form = if data.diagram_type == crate::state::DIAGRAM_TYPE {
            shapes::LinkForm::PostRender
        } else {
            shapes::LinkForm::Flowchart
        };
        let dom_id = resolved_ids
            .as_ref()
            .map_or(node.dom_id.as_str(), |ids| ids.nodes[index].as_str());
        let remapped_link = resolved_ids.as_ref().and_then(|ids| {
            node.link
                .as_deref()
                .and_then(|link| ids.remap_fragment(link))
        });
        let svg = shapes::node_svg_with_resolved(
            node,
            shape,
            id,
            link_form,
            loose,
            dom_id,
            remapped_link.as_deref(),
        );
        if animation.is_some() {
            node_parts.push((index, svg));
        } else {
            nodes.push_str(&svg);
        }
        labels.extend(shape.label.as_ref());
        labels.extend(shape.texts.iter());
    }

    let mut path_edges: Cow<'_, [Edge]> = Cow::Borrowed(data.edges.as_slice());
    let jumps = edges::jump_style(&config.section);
    if let Some(ids) = &resolved_ids {
        let edges_mut = path_edges.to_mut();
        for (index, edge) in edges_mut.iter_mut().enumerate() {
            edge.id.clone_from(&ids.edge_ids[index]);
        }
    }
    let mut edge_paths = String::new();
    let mut coloured_markers = String::new();
    let paths = edges::prepare_paths(&path_edges, id, &data.diagram_type, &config.section, jumps);
    let mut animator = animation
        .map(|options| {
            let mut occupied_mask_ids = HashSet::new();
            for (index, node) in data.nodes.iter().enumerate() {
                let dom_id = resolved_ids
                    .as_ref()
                    .map_or(node.dom_id.as_str(), |ids| ids.nodes[index].as_str());
                if dom_id.starts_with("draw-mask-") {
                    occupied_mask_ids.insert(format!("{id}-{dom_id}"));
                }
            }
            for edge in path_edges.iter() {
                if edge.id.starts_with("draw-mask-") {
                    occupied_mask_ids.insert(format!("{id}-{}", edge.id));
                }
            }
            crate::animation::Animator::new_with_occupied_masks(
                id,
                &data.nodes,
                &path_edges,
                &paths,
                options,
                occupied_mask_ids,
            )
        })
        .transpose()?;
    for (index, mut path) in paths.into_iter().enumerate() {
        let animated = animator
            .as_mut()
            .map(|animator| animator.path(index, &mut path));
        let path = edges::finish_path(path, animated);
        cover(&mut bounds, path.bbox);
        edge_paths.push_str(&path.svg);
        coloured_markers.push_str(&path.markers);
    }
    let mut edge_labels = String::new();
    // The boxes of the edge labels in the design mode, where each interrupts its edge.
    let mut label_gaps: Vec<BBox> = Vec::new();
    for (index, edge) in data.edges.iter().enumerate() {
        let resolved_edge_id = resolved_ids
            .as_ref()
            .map_or(edge.id.as_str(), |ids| ids.edge_ids[index].as_str());
        if let Some(label) = measured.edge_labels.get(&edge.id) {
            // The design mode keeps a label's chip off the frame of a group.
            let moved = if config.section.design.is_some() || resolved_edge_id != edge.id {
                let mut cloned = edge.clone();
                if config.section.design.is_some() {
                    let frames: Vec<BBox> = painted_clusters
                        .iter()
                        .map(|cluster| cluster.bbox)
                        .collect();
                    let place = edges::label_clear_of_frames(edge, label, &frames);
                    cloned.x = Some(place.x);
                    cloned.y = Some(place.y);
                }
                if resolved_edge_id != edge.id {
                    cloned.id = resolved_edge_id.to_owned();
                }
                Some(cloned)
            } else {
                None
            };
            let edge = moved.as_ref().unwrap_or(edge);
            let painted = edges::paint_label(edge, label, &config.section);
            cover(&mut bounds, painted.bbox);
            if config.section.design.is_some() && label.background.is_some() {
                label_gaps.push(painted.bbox);
            }
            match &mut animator {
                Some(animator) => edge_labels.push_str(&animator.label(index, &painted.svg)),
                None => edge_labels.push_str(&painted.svg),
            }
            labels.push(label);
        }
        // `insertEdgeLabel` runs for an edge with a label or a terminal label (`hasEdgeLabel`,
        // `edges.js` 55-62): without text, the label has no position, and its background sits beside it.
        if edge.label.is_empty() && !edge.terminals.is_empty() {
            edge_labels.push_str(&format!(
                "<g class=\"edgeLabel\"><g class=\"label\" data-id=\"{}\" transform=\"translate(0, 0)\"><text y=\"-10.1\" text-anchor=\"middle\"><tspan class=\"text-outer-tspan row\" x=\"0\" y=\"-0.1em\" dy=\"1.1em\" text-anchor=\"middle\"></tspan></text></g></g><g><rect class=\"background\" style=\"stroke: none\"></rect></g>",
                crate::svg::escape(resolved_edge_id),
            ));
        }
        // The terminal labels of an edge follow its center label in `edgeLabels` (`insertEdgeLabel`).
        for terminal_label in &edge.terminals {
            let (svg, bbox) = terminal::paint(edge, terminal_label);
            cover(&mut bounds, bbox);
            edge_labels.push_str(&svg);
            labels.push(&terminal_label.label);
        }
    }

    let cluster_labels = painted_clusters
        .into_iter()
        .filter_map(|cluster| cluster.label)
        .collect();

    if let Some(animator) = &mut animator {
        for (index, svg) in node_parts {
            nodes.push_str(&animator.node(index, &svg));
        }
    }
    let animation = animator
        .map(crate::animation::Animator::finish)
        .unwrap_or_default();

    // The design mode leaves a gap in an edge where its label sits: the edges are masked there, so
    // the label needs no background and reads on any ground.
    if let (false, Some(drawn)) = (label_gaps.is_empty(), bounds) {
        edge_paths = edges::with_label_gaps(&edge_paths, id, drawn, &label_gaps);
    }

    Ok(PaintedParts {
        root: RootParts {
            clusters,
            edge_paths,
            edge_labels,
            nodes,
            animation,
        },
        coloured_markers,
        bounds,
        cluster_labels,
        labels,
        links,
    })
}

/// Adds a painted box to the boxes the viewBox covers.
pub fn cover(bounds: &mut Option<BBox>, bbox: BBox) {
    *bounds = Some(bounds.map_or(bbox, |all| all.union(bbox)));
}

/// The font size of labels when the theme has none.
const DEFAULT_FONT_SIZE: f64 = 16.0;

#[cfg(test)]
mod direction_tests {
    use super::*;

    #[test]
    fn the_busiest_side_counts_the_edges_into_and_out_of_a_node() {
        let node = |id: &str, parent: Option<&str>, dir: Option<&str>| Node {
            id: id.into(),
            parent_id: parent.map(str::to_owned),
            dir: dir.map(str::to_owned),
            is_group: dir.is_some(),
            ..Node::default()
        };
        let edge = |start: &str, end: &str| Edge {
            start: start.into(),
            end: end.into(),
            ..Edge::default()
        };
        let data = LayoutData {
            direction: "LR".into(),
            nodes: vec![
                node("a", None, None),
                node("b", None, None),
                node("group", None, Some("TB")),
                node("c", Some("group"), None),
            ],
            edges: vec![
                edge("a", "b"),
                edge("a", "b"),
                edge("a", "c"),
                edge("c", "b"),
                edge("b", "b"),
            ],
            ..LayoutData::default()
        };
        let sides = busiest_sides(&data);
        // `a` has three edges out: 2 * 20 + 2 * 24 along its height, from left to right.
        assert_eq!(sides[0], (88.0, true));
        // `b` has three edges in; its self-loop does not count.
        assert_eq!(sides[1], (88.0, true));
        // `c` has one edge on each side and needs nothing; its group runs from top to bottom.
        assert_eq!(sides[3], (0.0, false));
    }

    #[test]
    fn node_direction_preserves_parent_lookup_and_fallbacks() {
        let node = |id: &str, parent: Option<&str>, dir: Option<&str>| Node {
            id: id.into(),
            parent_id: parent.map(str::to_owned),
            dir: dir.map(str::to_owned),
            ..Node::default()
        };
        let data = LayoutData {
            direction: "TB".into(),
            nodes: vec![
                node("own", None, Some("RL")),
                node("leaf", Some("group"), None),
                node("group", None, Some("LR")),
                node("parentless", None, None),
                node("missing", Some("absent"), None),
                node("cycle1", Some("cycle2"), None),
                node("cycle2", Some("cycle1"), None),
            ],
            ..LayoutData::default()
        };
        let by_id = data.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        assert_eq!(node_dir(&data, 0, &by_id), Some("RL"));
        assert_eq!(node_dir(&data, 1, &by_id), Some("LR"));
        assert_eq!(node_dir(&data, 3, &by_id), None);
        assert_eq!(node_dir(&data, 4, &by_id), Some("TB"));
        assert_eq!(node_dir(&data, 5, &by_id), Some("TB"));
    }

    #[test]
    fn direction_consumers_include_all_fork_join_aliases() {
        for shape in ["fork", "join", "forkJoin"] {
            assert!(shapes::uses_direction(&Node {
                shape: shape.into(),
                ..Node::default()
            }));
        }
        assert!(!shapes::uses_direction(&Node {
            shape: "rect".into(),
            ..Node::default()
        }));
    }
}
