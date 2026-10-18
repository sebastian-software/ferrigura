//! The treemap diagram: a port of `packages/mermaid/src/diagrams/treemap/renderer.ts` (`draw`), with
//! the layout of `d3-hierarchy` ([`common::d3::treemap`]), the value formats of `d3-format`
//! ([`common::d3::format`]) and the colour scales of `d3-scale` ([`common::d3::ordinal`]). The
//! research is in `docs-port/treemap/NOTES.md` (section 6), and the parser in `docs-port/treemap/PLAN.md`.
//!
//! A section is a node with children: its box is the tile of its own children, with a header band
//! of 25 px. A leaf is a node without children: its label and its value are fitted into its box by
//! shrinking the font, and hidden when they do not fit. The fitting decisions read the advance of
//! the text, so the text is measured as Chromium measures it ([`text`]). The viewBox is the union of
//! the boxes that are drawn; an element with `display: none` is left out, as `getBBox` leaves it out.
//!
//! Two things of the reference are kept on purpose: a `valueFormat` that is invalid after a `$`
//! fails the whole render when a value is formatted (`renderer.ts` 59-68 build the formatter inside
//! the value callback), and an invalid specifier without a `$` falls back to the default format.

mod config;
mod style;
mod text;

pub use config::TreemapSettings;

use std::cmp::Ordering;

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::treemap::{TreemapDiagram, TreemapNode};
use ferrigura_text::{EmbeddedFamily, FontFamily, FontStyle, FontWeight, Run, embed_runs, face};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::d3::OrdinalScale;
use crate::common::d3::format::{self as d3format, Formatter};
use crate::common::d3::hierarchy::{Hierarchy, Node};
use crate::common::d3::js_round;
use crate::common::d3::math_max;
use crate::common::d3::number::js_number;
use crate::common::d3::treemap::{Padding, Treemap};
use crate::common::decode_entities;
use crate::common::style::base_style_sheet_with;
use crate::common::viewbox::{BBox, setup_graph_viewbox};
use crate::svg::{escape, number};
use crate::theme::Themes;
use crate::variables::{px, text_of};
use style::{Inline, Styles, colour_as_fill, own_rules, styles_to_string};
use text::{Anchor, Baseline, Font, advance, line_box};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The height of a section's header band (`SECTION_HEADER_HEIGHT`, `renderer.ts` 19).
const SECTION_HEADER_HEIGHT: f64 = 25.0;

/// The inner padding of a section's content (`SECTION_INNER_PADDING`, `renderer.ts` 18).
const SECTION_INNER_PADDING: f64 = 10.0;

/// The height of the title band when the diagram has a title (`renderer.ts` 36).
const TITLE_HEIGHT: f64 = 30.0;

/// The title's baseline, which is half the title band (`renderer.ts` 130).
const TITLE_Y: f64 = TITLE_HEIGHT / 2.0;

/// The bottom padding of a leaf's value (`cellBottomPadding`, `renderer.ts` 510).
const CELL_BOTTOM_PADDING: f64 = 4.0;

/// The scale of a leaf's value font to its label's (`valueScaleFactor`, `renderer.ts` 398).
const VALUE_SCALE_FACTOR: f64 = 0.6;

/// The leaves above which the complex label layout is used (`renderer.ts` 314).
const COMPLEX_LEAF_COUNT: usize = 20;

/// What the treemap renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct TreemapInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a TreemapDiagram,
    /// The effective configuration, in the treemap diagram's scope.
    pub config: &'a MermaidConfig,
    /// The `treemap` section of the configuration (see [`TreemapSettings`]).
    pub settings: &'a TreemapSettings,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// The sizes of a leaf's labels: the complex layout (more than twenty leaves) has smaller ones
/// (`renderer.ts` 316-322).
#[derive(Debug, Clone, Copy, PartialEq)]
struct LeafSizes {
    base_label: f64,
    base_value: f64,
    min_label: f64,
    min_value: f64,
    padding: f64,
    min_display: f64,
    spacing: f64,
    complex: bool,
}

impl LeafSizes {
    fn of(leaf_count: usize) -> Self {
        if leaf_count > COMPLEX_LEAF_COUNT {
            Self {
                base_label: 16.0,
                base_value: 14.0,
                min_label: 4.0,
                min_value: 4.0,
                padding: 2.0,
                min_display: 8.0,
                spacing: 1.0,
                complex: true,
            }
        } else {
            Self {
                base_label: 38.0,
                base_value: 28.0,
                min_label: 8.0,
                min_value: 6.0,
                padding: 4.0,
                min_display: 10.0,
                spacing: 2.0,
                complex: false,
            }
        }
    }

    /// The value's font size for a label of `label_size`: `max(minValue, min(baseValue, round(size
    /// * 0.6)))` (`renderer.ts` 410-413 and 498-501).
    fn value_size(&self, label_size: f64) -> f64 {
        self.min_value.max(
            self.base_value
                .min(js_round(label_size * VALUE_SCALE_FACTOR)),
        )
    }
}

/// The value of a node as `valueFormat` writes it: `valueFormat(d.value)` when the value is truthy.
enum ValueFormat {
    /// `format(specifier)`: the formatter, or the default `,` when the specifier is invalid.
    Plain(Formatter),
    /// `'$' + format(specifier)(value)`, where the specifier is read when a value is formatted. An
    /// invalid one fails the render then, as the reference's callback does.
    Dollar(String),
}

impl ValueFormat {
    /// The `valueFormat` of the reference for a specifier (`renderer.ts` 57-77).
    fn of(specifier: &str) -> Result<Self, RenderError> {
        if specifier == "$0,0" {
            return Ok(Self::Dollar(",".to_owned()));
        }
        if specifier.starts_with('$') && specifier.contains(',') {
            return Ok(Self::Dollar(format!(",{}", decimals_of(specifier))));
        }
        if let Some(rest) = specifier.strip_prefix('$') {
            return Ok(Self::Dollar(rest.to_owned()));
        }
        match d3format::format(specifier) {
            Ok(formatter) => Ok(Self::Plain(formatter)),
            Err(_) => Ok(Self::Plain(comma_format()?)),
        }
    }

    /// The text of a value (`valueFormat(value)`).
    fn call(&self, value: f64) -> Result<String, RenderError> {
        match self {
            Self::Plain(formatter) => Ok(formatter.call(value)),
            Self::Dollar(specifier) => {
                let formatter = d3format::format(specifier)
                    .map_err(|error| RenderError::Config(error.to_string()))?;
                Ok(format!("${}", formatter.call(value)))
            }
        }
    }
}

/// `format(',')`: the default value format, which no specifier fails.
fn comma_format() -> Result<Formatter, RenderError> {
    d3format::format(",").map_err(|error| RenderError::Config(error.to_string()))
}

/// The first `.<digits>` of a specifier, as `/\.\d+/` finds it, or the empty text.
fn decimals_of(specifier: &str) -> String {
    let bytes = specifier.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'.' {
            continue;
        }
        let digits = bytes[index + 1..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits > 0 {
            return specifier[index..=index + digits].to_owned();
        }
    }
    String::new()
}

/// The colour ranges of the three scales (`renderer.ts` 79-123): the fill and the stroke of a
/// section start with `transparent`, the labels do not.
struct Colours {
    fill: Vec<String>,
    peer: Vec<String>,
    label: Vec<String>,
}

impl Colours {
    fn of(variables: &ferrigura_model::value::Value) -> Self {
        let named = |prefix: &str, count: usize| -> Vec<String> {
            (0..count)
                .map(|index| text_of(variables, &format!("{prefix}{index}")).unwrap_or_default())
                .collect()
        };
        let with_transparent = |mut colours: Vec<String>| {
            colours.insert(0, "transparent".to_owned());
            colours
        };
        Self {
            fill: with_transparent(named("cScale", 12)),
            peer: with_transparent(named("cScalePeer", 12)),
            label: named("cScaleLabel", 12),
        }
    }
}

/// A section, with what its drawing needs.
struct Section {
    /// Its index in the level order of the sections (`i` of `section<i>`, `clip-section-…-<i>`).
    index: usize,
    /// Its depth; the root is at depth 0 and its drawing is hidden.
    depth: usize,
    /// The box of the section in the layout.
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    /// The name.
    name: String,
    /// The value, the sum of its children.
    value: f64,
    /// The colours and the declarations of the section.
    fill: String,
    stroke: String,
    styles: Styles,
    /// The label and the value text, as they are drawn after the fitting.
    label: String,
    label_style: Inline,
    value_text: String,
    value_style: Inline,
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset, a label cannot be measured, or a
/// `valueFormat` with a `$` names an invalid specifier and a value is formatted with it.
pub fn render(input: &TreemapInput<'_>) -> Result<String, RenderError> {
    let settings = input.settings;
    let variables = input.themes.primary();
    let id = input.id;
    let diagram = input.diagram;
    let title = diagram.title.as_str();
    let title_height = if title.is_empty() { 0.0 } else { TITLE_HEIGHT };
    let width = settings.layout_width();
    let height = settings.layout_height();
    let format = ValueFormat::of(&settings.value_format)?;
    let colours = Colours::of(variables);

    // `hierarchy(root).sum(value).sort(by value, descending)` (`renderer.ts` 144-146).
    let mut hierarchy = Hierarchy::new(&diagram.tree, children_of);
    hierarchy.sum(|node| node.value.unwrap_or(0.0));
    hierarchy.sort(|a, b| b.value - a.value);

    // The layout (`renderer.ts` 149-161): a section pads its children by its header and 10 px.
    let inner = settings.padding;
    Treemap {
        width,
        height,
        round: true,
    }
    .layout(&mut hierarchy, |node| {
        let branch = node.children.is_some();
        Padding {
            inner,
            top: if branch {
                SECTION_HEADER_HEIGHT + SECTION_INNER_PADDING
            } else {
                0.0
            },
            right: if branch { SECTION_INNER_PADDING } else { 0.0 },
            bottom: if branch { SECTION_INNER_PADDING } else { 0.0 },
            left: if branch { SECTION_INNER_PADDING } else { 0.0 },
        }
    });

    let nodes = hierarchy.nodes();
    let section_nodes: Vec<usize> = hierarchy
        .descendants()
        .into_iter()
        .filter(|&index| nodes[index].children.is_some())
        .collect();
    let leaf_nodes = hierarchy.leaves();
    let sizes = LeafSizes::of(leaf_nodes.len());

    // The colour scales, each called in the order of the reference: the sections' fill and stroke
    // first (in level order), then the labels of the sections, then the leaves in pre-order.
    let mut fill_scale = OrdinalScale::new(&colours.fill);
    let mut peer_scale = OrdinalScale::new(&colours.peer);
    let mut label_scale = OrdinalScale::new(&colours.label);
    let mut sections = Vec::with_capacity(section_nodes.len());
    for (index, &node_index) in section_nodes.iter().enumerate() {
        let node = &nodes[node_index];
        let name = node.data.name.clone();
        let fill = fill_scale.apply(&name).to_owned();
        let stroke = peer_scale.apply(&name).to_owned();
        sections.push(Section {
            index,
            depth: node.depth,
            x0: node.x0,
            y0: node.y0,
            x1: node.x1,
            y1: node.y1,
            name,
            value: node.value,
            fill,
            stroke,
            styles: styles_to_string(node.data.css_compiled_styles.as_deref().unwrap_or_default()),
            label: String::new(),
            label_style: Inline::default(),
            value_text: String::new(),
            value_style: Inline::default(),
        });
    }

    // The section labels and values (`renderer.ts` 218-309), with the label fitted to the header.
    let mut section_groups = Vec::with_capacity(sections.len());
    let mut section_runs: Vec<(String, &'static ferrigura_text::Face)> = Vec::new();
    for section in &mut sections {
        let root = section.depth == 0;
        let label_colour = if root {
            String::new()
        } else {
            label_scale.apply(&section.name).to_owned()
        };
        section.label_style = if root {
            Inline::parse("display: none;")
        } else {
            let base = format!(
                "dominant-baseline: middle; font-size: 12px; fill:{label_colour}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;"
            );
            Inline::parse(&format!("{base}{}", colour_as_fill(&section.styles.label)))
        };
        section.label = if root {
            String::new()
        } else {
            section.name.clone()
        };
        if !root {
            fit_section_label(section, settings)?;
        }
        if settings.show_values {
            section.value_text = if section.value != 0.0 && !section.value.is_nan() {
                format.call(section.value)?
            } else {
                String::new()
            };
            section.value_style = if root {
                Inline::parse("display: none;")
            } else {
                let base = format!(
                    "text-anchor: end; dominant-baseline: middle; font-size: 10px; fill:{label_colour}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;"
                );
                Inline::parse(&format!("{base}{}", colour_as_fill(&section.styles.label)))
            };
        }
    }
    for section in &sections {
        section_runs.push((
            section.label.clone(),
            face_of(font_of(&section.label_style, Font::BOLD)),
        ));
        if settings.show_values && !section.value_text.is_empty() {
            section_runs.push((
                section.value_text.clone(),
                face_of(font_of(&section.value_style, Font::ITALIC)),
            ));
        }
        section_groups.push(section_group(id, section, settings.show_values));
    }

    // The leaves (`renderer.ts` 311-524): their boxes, their labels and their values.
    let mut leaf_groups = Vec::with_capacity(leaf_nodes.len());
    let mut leaf_draws = Vec::with_capacity(leaf_nodes.len());
    for (index, &node_index) in leaf_nodes.iter().enumerate() {
        let node = &nodes[node_index];
        let leaf_name = node.data.name.clone();
        // A leaf takes its parent's colour, or its own name's when it is the root.
        let key = match node.parent {
            Some(parent) => nodes[parent].data.name.as_str(),
            None => leaf_name.as_str(),
        };
        let fill = fill_scale.apply(key).to_owned();
        let styles = styles_to_string(node.data.css_compiled_styles.as_deref().unwrap_or_default());
        let label_colour = label_scale.apply(&leaf_name).to_owned();
        let draw = leaf_draw(
            &LeafInput {
                name: &leaf_name,
                width: node.x1 - node.x0,
                height: node.y1 - node.y0,
                value: node.value,
                label_colour: &label_colour,
                styles: &styles,
                sizes,
                show_values: settings.show_values,
            },
            &format,
        )?;
        leaf_draws.push(draw.clone());
        leaf_groups.push(leaf_group(id, index, node, &fill, &styles, &draw));
    }

    // The viewBox: the union of the boxes that are drawn, the container's at its translation.
    let mut bounds: Option<BBox> = None;
    if !title.is_empty() {
        let title_box = line_box(
            title,
            (width / 2.0, TITLE_Y),
            14.0,
            Font::REGULAR,
            Anchor::Middle,
            Baseline::Middle,
        )?;
        include(&mut bounds, title_box);
    }
    // The boxes as the browser takes them from the markup: a translation with a value that is not a
    // number is not a transform (the group keeps the container's origin), and a coordinate or a size
    // that is not a number reads as zero (`svg` attributes with NaN, as an infinite value gives).
    for section in &sections {
        if section.depth == 0 {
            continue;
        }
        let (x, y) = group_origin(section.x0, section.y0, title_height);
        let (w, h) = (
            valid(section.x1 - section.x0),
            valid(section.y1 - section.y0),
        );
        include(&mut bounds, Some(rect_box(x, y, w, SECTION_HEADER_HEIGHT)));
        include(&mut bounds, Some(rect_box(x, y, w, h)));
        let label_font = font_of(&section.label_style, Font::BOLD);
        let label_size = size_of(&section.label_style, 12.0);
        include(
            &mut bounds,
            line_box(
                &section.label,
                (x + 6.0, y + SECTION_HEADER_HEIGHT / 2.0),
                label_size,
                label_font,
                Anchor::Start,
                Baseline::Middle,
            )?,
        );
        if settings.show_values {
            include(
                &mut bounds,
                line_box(
                    &section.value_text,
                    (
                        x + valid(section.x1 - section.x0 - 10.0),
                        y + SECTION_HEADER_HEIGHT / 2.0,
                    ),
                    size_of(&section.value_style, 10.0),
                    font_of(&section.value_style, Font::ITALIC),
                    Anchor::End,
                    Baseline::Middle,
                )?,
            );
        }
    }
    for (leaf, &node_index) in leaf_draws.iter().zip(&leaf_nodes) {
        let node = &nodes[node_index];
        let (x, y) = group_origin(node.x0, node.y0, title_height);
        let (w, h) = (valid(node.x1 - node.x0), valid(node.y1 - node.y0));
        include(&mut bounds, Some(rect_box(x, y, w, h)));
        if !leaf.label_hidden {
            include(
                &mut bounds,
                line_box(
                    &leaf.label,
                    (
                        x + valid((node.x1 - node.x0) / 2.0),
                        y + valid((node.y1 - node.y0) / 2.0),
                    ),
                    size_of(&leaf.label_style, sizes.base_label),
                    font_of(&leaf.label_style, Font::REGULAR),
                    Anchor::Middle,
                    Baseline::Middle,
                )?,
            );
        }
        if let Some(value) = &leaf.value
            && !value.hidden
        {
            include(
                &mut bounds,
                line_box(
                    &value.text,
                    (x + valid((node.x1 - node.x0) / 2.0), y + valid(value.y)),
                    size_of(&value.style, sizes.base_value),
                    Font::REGULAR,
                    Anchor::Middle,
                    Baseline::Hanging,
                )?,
            );
        }
    }

    // The fonts: every text the diagram draws, in the faces it is drawn in.
    let mut runs: Vec<(String, &'static ferrigura_text::Face)> = section_runs
        .into_iter()
        .map(|(text, face)| (shown(&text), face))
        .collect();
    if !title.is_empty() {
        runs.push((shown(title), face_of(Font::REGULAR)));
    }
    for leaf in &leaf_draws {
        runs.push((
            shown(&leaf.label),
            face_of(font_of(&leaf.label_style, Font::REGULAR)),
        ));
        if let Some(value) = &leaf.value {
            runs.push((
                shown(&value.text),
                face_of(font_of(&value.style, Font::REGULAR)),
            ));
        }
    }
    let run_refs: Vec<Run<'_>> = runs.iter().map(|(text, face)| Run { text, face }).collect();
    let families: Vec<EmbeddedFamily> = embed_runs(&run_refs)?;
    let font_family = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| family.css_family.clone(),
    );
    let css_font_size = format!("{}px", number(px(variables, "fontSize").unwrap_or(16.0)));
    let style = base_style_sheet_with(
        id,
        input.themes,
        &font_family,
        &css_font_size,
        &families,
        &own_rules(id, input.themes),
    );

    // The root: the viewBox and the size (`setupViewPortForSVG`, `renderer.ts` 525-526), then the
    // accessible names, the style, the empty group `mermaidAPI` appends, the title and the layout.
    let mut root = Element::new("svg")
        .with("id", id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE);
    setup_graph_viewbox(
        &mut root,
        bounds.unwrap_or(BBox {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }),
        settings.diagram_padding,
        settings.use_max_width,
    );
    root.set("class", "flowchart");
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "treemap");
    if !diagram.acc_descr.is_empty() {
        root.set("aria-describedby", a11y::description_id(id));
    }
    if !diagram.acc_title.is_empty() {
        root.set("aria-labelledby", a11y::title_id(id));
    }

    let mut inner_markup = a11y::children(id, &diagram.acc_title, &diagram.acc_descr);
    inner_markup.push_str(&Element::new("style").raw_text(style).render());
    inner_markup.push_str(&Element::new("g").render());
    if !title.is_empty() {
        let title_element = Element::new("text")
            .with("x", number(width / 2.0))
            .with("y", number(TITLE_Y))
            .with("class", "treemapTitle")
            .with("fill", text_of(variables, "titleColor").unwrap_or_default())
            .with("text-anchor", "middle")
            .with("dominant-baseline", "middle")
            .raw_text(markup(title));
        inner_markup.push_str(&title_element.render());
    }
    let mut container = Element::new("g")
        .with(
            "transform",
            format!("translate(0, {})", number(title_height)),
        )
        .with("class", "treemapContainer");
    for group in section_groups {
        container.push_child(group);
    }
    for group in leaf_groups {
        container.push_child(group);
    }
    inner_markup.push_str(&container.render());
    Ok(root.raw_text(inner_markup).render())
}

/// The children of a node of the diagram (`hierarchy`'s `objectChildren`). A node with an empty
/// list has none.
fn children_of(node: &TreemapNode) -> Option<&[TreemapNode]> {
    node.children.as_deref()
}

/// The `.attr` of a rectangle's box: a rectangle at `(x, y)` of `width` by `height`.
fn rect_box(x: f64, y: f64, width: f64, height: f64) -> BBox {
    BBox {
        x,
        y,
        width,
        height,
    }
}

/// The origin of a group in the container, from its `translate(x0, y0)` (the container is at
/// `translate(0, title_top)`). A translation with a value that is not finite is not a transform, so
/// the group keeps the container's origin.
fn group_origin(x0: f64, y0: f64, title_top: f64) -> (f64, f64) {
    if x0.is_finite() && y0.is_finite() {
        (x0, y0 + title_top)
    } else {
        (0.0, title_top)
    }
}

/// A length or a coordinate as the browser reads an SVG attribute: a value that is not finite is
/// not a number, and reads as zero.
fn valid(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

/// Adds a box to the union of the boxes drawn. An empty box (no width or no height) is left out, as
/// the union of `getBBox` leaves it out.
fn include(bounds: &mut Option<BBox>, extra: Option<BBox>) {
    let Some(extra) = extra else {
        return;
    };
    if extra.width <= 0.0 || extra.height <= 0.0 {
        return;
    }
    *bounds = Some(match *bounds {
        None => extra,
        Some(so_far) => {
            let left = so_far.x.min(extra.x);
            let top = so_far.y.min(extra.y);
            let right = (so_far.x + so_far.width).max(extra.x + extra.width);
            let bottom = (so_far.y + so_far.height).max(extra.y + extra.height);
            BBox {
                x: left,
                y: top,
                width: right - left,
                height: bottom - top,
            }
        }
    });
}

/// The face the text of a font is drawn in, from the bundled sans faces.
fn face_of(font: Font) -> &'static ferrigura_text::Face {
    face(FontFamily::Sans, font.weight, font.style)
}

/// The font of a text: the attribute's font, where its inline style does not say otherwise.
fn font_of(style: &Inline, attribute: Font) -> Font {
    let weight = match style.get("font-weight") {
        Some(value) if value == "bold" || value.parse::<f64>().is_ok_and(|n| n >= 600.0) => {
            FontWeight::Bold
        }
        Some(value) if value == "normal" || value.parse::<f64>().is_ok_and(|n| n < 600.0) => {
            FontWeight::Normal
        }
        _ => attribute.weight,
    };
    let style_of = match style.get("font-style") {
        Some(value) if value == "italic" || value == "oblique" => FontStyle::Italic,
        Some(value) if value == "normal" => FontStyle::Normal,
        _ => attribute.style,
    };
    Font {
        weight,
        style: style_of,
    }
}

/// The computed font size of a text: the cascade of its inline style, in pixels, or `default` when
/// the style does not give one.
fn size_of(style: &Inline, default: f64) -> f64 {
    style
        .get("font-size")
        .map_or(default, |value| js_parse_float(&value))
}

/// `parseFloat` of a text: the longest decimal prefix, or NaN.
fn js_parse_float(text: &str) -> f64 {
    let trimmed = text.trim_start();
    let bytes = trimmed.as_bytes();
    let mut end = 0;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    let mut has_digits = end > digits_start;
    if end < bytes.len() && bytes[end] == b'.' {
        let fraction_start = end + 1;
        let mut fraction_end = fraction_start;
        while fraction_end < bytes.len() && bytes[fraction_end].is_ascii_digit() {
            fraction_end += 1;
        }
        if fraction_end > fraction_start {
            has_digits = true;
            end = fraction_end;
        }
    }
    if !has_digits {
        return f64::NAN;
    }
    trimmed[..end].parse().unwrap_or(f64::NAN)
}

/// `parseInt(text, 10)`: the integer prefix of a text, or NaN.
fn js_parse_int(text: &str) -> f64 {
    let whole = js_parse_float(text);
    if whole.is_nan() { whole } else { whole.trunc() }
}

/// The `font-size` a node's inline style gives, as `style('font-size')` reads it: the first
/// declaration of the name.
fn first_font_size(style: &Inline) -> String {
    style.get("font-size").unwrap_or_default()
}

/// Fits a section's label into its header (`renderer.ts` 240-283): the label is cut to the room the
/// header leaves, with `...` after it, and emptied when even the ellipsis does not fit.
fn fit_section_label(section: &mut Section, settings: &TreemapSettings) -> Result<(), RenderError> {
    let total_header_width = section.x1 - section.x0;
    let space = if settings.show_values && section.value != 0.0 && !section.value.is_nan() {
        let value_ends_at = total_header_width - 10.0;
        let estimated_value_width = 30.0;
        let gap = 10.0;
        let label_must_end_before = value_ends_at - estimated_value_width - gap;
        label_must_end_before - 6.0
    } else {
        total_header_width - 6.0 - 6.0
    };
    let available = math_max(15.0, space);
    let font = font_of(&section.label_style, Font::BOLD);
    let size = size_of(&section.label_style, 12.0);
    let original = section.name.clone();
    // `if (width > available)`: a NaN room (from a NaN width) cuts nothing.
    if advance(&original, size, font)?.partial_cmp(&available) != Some(Ordering::Greater) {
        return Ok(());
    }
    let characters: Vec<char> = original.chars().collect();
    let mut length = characters.len();
    loop {
        if length == 0 {
            break;
        }
        length -= 1;
        if length == 0 {
            section.label = "...".to_owned();
            if advance("...", size, font)? > available {
                section.label.clear();
            }
            break;
        }
        let shown: String = characters[..length].iter().collect::<String>() + "...";
        section.label = shown;
        if advance(&section.label, size, font)? <= available {
            break;
        }
    }
    Ok(())
}

/// The inputs of a leaf's drawing.
struct LeafInput<'a> {
    name: &'a str,
    width: f64,
    height: f64,
    value: f64,
    label_colour: &'a str,
    styles: &'a Styles,
    sizes: LeafSizes,
    show_values: bool,
}

/// The drawing of a leaf after its fitting: the label and the value, with their styles.
#[derive(Debug, Clone)]
struct LeafDraw {
    label: String,
    label_style: Inline,
    label_hidden: bool,
    value: Option<LeafValue>,
}

/// The value of a leaf: its text, its y and its style, and whether it is hidden.
#[derive(Debug, Clone)]
struct LeafValue {
    text: String,
    y: f64,
    style: Inline,
    hidden: bool,
}

/// Fits a leaf's label and value into its box (`renderer.ts` 383-524).
fn leaf_draw(input: &LeafInput<'_>, format: &ValueFormat) -> Result<LeafDraw, RenderError> {
    let sizes = input.sizes;
    let label_style_text = format!(
        "text-anchor: middle; dominant-baseline: middle; font-size: {}px;fill:{};{}",
        js_number(sizes.base_label),
        input.label_colour,
        colour_as_fill(&input.styles.label)
    );
    let mut label_style = Inline::parse(&label_style_text);
    let label = input.name.to_owned();
    fit_leaf_label(&label, input.width, input.height, sizes, &mut label_style)?;
    let label_hidden = label_style.get("display").as_deref() == Some("none");

    // The value's text is formatted when the node has one, whether or not it is drawn.
    let value_text = if input.show_values {
        Some(if input.value != 0.0 && !input.value.is_nan() {
            format.call(input.value)?
        } else {
            String::new()
        })
    } else {
        None
    };
    let value = value_text.map(|text| {
        let value_style_text = format!(
            "text-anchor: middle; dominant-baseline: hanging; font-size: {}px;fill:{};{}",
            js_number(sizes.base_value),
            input.label_colour,
            colour_as_fill(&input.styles.label)
        );
        LeafValue {
            text,
            y: input.height / 2.0,
            style: Inline::parse(&value_style_text),
            hidden: false,
        }
    });
    let value = match value {
        None => None,
        Some(mut value) => {
            if label_hidden {
                value.style.set("display", "none");
                value.hidden = true;
            } else {
                let final_label = js_parse_float(&first_font_size(&label_style));
                let actual = sizes.value_size(final_label);
                value
                    .style
                    .set("font-size", &format!("{}px", js_number(actual)));
                let centre = input.height / 2.0;
                let value_top = centre + final_label / 2.0 + sizes.spacing;
                value.y = value_top;
                let available_width = input.width - 2.0 * sizes.padding;
                let max_bottom = input.height - CELL_BOTTOM_PADDING;
                let too_wide = advance(
                    &value.text,
                    size_of(&value.style, actual),
                    font_of(&value.style, Font::REGULAR),
                )? > available_width;
                let too_low = value_top + actual > max_bottom;
                let too_small = actual < sizes.min_value;
                if too_wide || too_low || too_small {
                    value.style.set("display", "none");
                    value.hidden = true;
                } else {
                    value.style.remove("display");
                    value.hidden = false;
                }
            }
            Some(value)
        }
    };
    Ok(LeafDraw {
        label,
        label_style,
        label_hidden,
        value,
    })
}

/// The label of a leaf: shrink the font until the label is no wider than the box, then until the
/// label and its value are no taller, and hide it when it still does not fit (`renderer.ts`
/// 383-455). `label_style` carries the font size the loops set.
fn fit_leaf_label(
    text: &str,
    node_width: f64,
    node_height: f64,
    sizes: LeafSizes,
    label_style: &mut Inline,
) -> Result<(), RenderError> {
    let available_width = node_width - 2.0 * sizes.padding;
    let available_height = node_height - 2.0 * sizes.padding;
    if available_width < sizes.min_display || available_height < sizes.min_display {
        label_style.set("display", "none");
        return Ok(());
    }
    let measure = |style: &Inline| -> Result<f64, RenderError> {
        advance(
            text,
            size_of(style, sizes.base_label),
            font_of(style, Font::REGULAR),
        )
    };
    let mut size = js_parse_int(&first_font_size(label_style));

    // 1. The width.
    while measure(label_style)? > available_width && size > sizes.min_label {
        size -= 1.0;
        label_style.set("font-size", &format!("{}px", js_number(size)));
    }

    // 2. The height of the label and the value together.
    let mut prospective = sizes.value_size(size);
    let mut combined = size + sizes.spacing + prospective;
    while combined > available_height && size > sizes.min_label {
        size -= 1.0;
        prospective = sizes.value_size(size);
        if prospective < sizes.min_value && size == sizes.min_label {
            break;
        }
        label_style.set("font-size", &format!("{}px", js_number(size)));
        combined = size + sizes.spacing + prospective;
    }
    label_style.set("font-size", &format!("{}px", js_number(size)));

    // 3. Whether the label is drawn at all.
    let hidden = if sizes.complex {
        size < sizes.min_label || available_height < sizes.min_label
    } else {
        measure(label_style)? > available_width || size < sizes.min_label || available_height < size
    };
    if hidden {
        label_style.set("display", "none");
    }
    Ok(())
}

/// The text of an element as the SVG carries it. The reference decodes the entity placeholders of the
/// whole SVG string once it is rendered (`mermaidAPI.ts` 359), after the layout measured the text
/// with the placeholders in it, so the measurements use the names as they are and the markup is
/// decoded here.
fn markup(text: &str) -> String {
    decode_entities(&escape(text))
}

/// The characters a text shows in the browser: the markup of [`markup`], read back as XML. The fonts
/// are subset to these, so a character the placeholders hide (the quote of `#quot;`) is embedded.
fn shown(text: &str) -> String {
    let markup = markup(text);
    let mut out = String::with_capacity(markup.len());
    let mut rest = markup.as_str();
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let Some(end) = tail.find(';') else {
            out.push_str(tail);
            return out;
        };
        let entity = &tail[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            _ => entity
                .strip_prefix('#')
                .and_then(|number| number.parse::<u32>().ok())
                .and_then(char::from_u32),
        };
        match decoded {
            Some(character) => out.push(character),
            None => out.push_str(&tail[..=end]),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

/// The `<g>` of a section (`renderer.ts` 163-309): its header band, the clip of the header, the
/// rectangle, the label and, when the values are shown, the value. The root's drawing is hidden.
fn section_group(id: &str, section: &Section, show_values: bool) -> Element {
    let hidden = section.depth == 0;
    let w = section.x1 - section.x0;
    let h = section.y1 - section.y0;
    let header_style = if hidden { "display: none;" } else { "" };
    let section_style = if hidden {
        "display: none;".to_owned()
    } else {
        format!(
            "{};{}",
            section.styles.node,
            section.styles.border.join(";")
        )
    };
    let mut group = Element::new("g")
        .with("class", "treemapSection")
        .with(
            "transform",
            format!("translate({},{})", number(section.x0), number(section.y0)),
        )
        .child(
            Element::new("rect")
                .with("width", number(w))
                .with("height", number(SECTION_HEADER_HEIGHT))
                .with("class", "treemapSectionHeader")
                .with("fill", "none")
                .with("fill-opacity", "0.6")
                .with("stroke-width", "0.6")
                .with("style", header_style),
        )
        .child(
            Element::new("clipPath")
                .with("id", format!("clip-section-{id}-{}", section.index))
                .child(
                    Element::new("rect")
                        .with("width", number((w - 12.0).max(0.0)))
                        .with("height", number(SECTION_HEADER_HEIGHT)),
                ),
        )
        .child(
            Element::new("rect")
                .with("width", number(w))
                .with("height", number(h))
                .with("class", format!("treemapSection section{}", section.index))
                .with("fill", section.fill.clone())
                .with("fill-opacity", "0.6")
                .with("stroke", section.stroke.clone())
                .with("stroke-width", "2")
                .with("stroke-opacity", "0.4")
                .with("style", section_style),
        )
        .child(
            Element::new("text")
                .with("class", "treemapSectionLabel")
                .with("x", "6")
                .with("y", number(SECTION_HEADER_HEIGHT / 2.0))
                .with("dominant-baseline", "middle")
                .with("font-weight", "bold")
                .with(
                    "clip-path",
                    format!("url(#clip-section-{id}-{})", section.index),
                )
                .with("style", section.label_style.to_attribute())
                .raw_text(markup(&section.label)),
        );
    if show_values {
        group.push_child(
            Element::new("text")
                .with("class", "treemapSectionValue")
                .with("x", number(w - 10.0))
                .with("y", number(SECTION_HEADER_HEIGHT / 2.0))
                .with("text-anchor", "end")
                .with("dominant-baseline", "middle")
                .with("font-style", "italic")
                .with("style", section.value_style.to_attribute())
                .raw_text(markup(&section.value_text)),
        );
    }
    group
}

/// The `<g>` of a leaf (`renderer.ts` 324-524): its rectangle, its clip, its label and its value.
fn leaf_group(
    id: &str,
    index: usize,
    node: &Node<'_, TreemapNode>,
    fill: &str,
    styles: &Styles,
    draw: &LeafDraw,
) -> Element {
    let w = node.x1 - node.x0;
    let h = node.y1 - node.y0;
    let class = match &node.data.class_selector {
        Some(selector) => format!("treemapNode treemapLeafGroup leaf{index} {selector}x"),
        None => format!("treemapNode treemapLeafGroup leaf{index}x"),
    };
    let mut group = Element::new("g")
        .with("class", class)
        .with(
            "transform",
            format!("translate({},{})", number(node.x0), number(node.y0)),
        )
        .child(
            Element::new("rect")
                .with("width", number(w))
                .with("height", number(h))
                .with("class", "treemapLeaf")
                .with("fill", fill)
                .with("style", styles.node.clone())
                .with("fill-opacity", "0.3")
                .with("stroke", fill)
                .with("stroke-width", "3"),
        )
        .child(
            Element::new("clipPath")
                .with("id", format!("clip-{id}-{index}"))
                .child(
                    Element::new("rect")
                        .with("width", number((w - 4.0).max(0.0)))
                        .with("height", number((h - 4.0).max(0.0))),
                ),
        );
    group.push_child(
        Element::new("text")
            .with("class", "treemapLabel")
            .with("x", number(w / 2.0))
            .with("y", number(h / 2.0))
            .with("style", draw.label_style.to_attribute())
            .with("clip-path", format!("url(#clip-{id}-{index})"))
            .raw_text(markup(&draw.label)),
    );
    if let Some(value) = &draw.value {
        group.push_child(
            Element::new("text")
                .with("class", "treemapValue")
                .with("x", number(w / 2.0))
                .with("y", number(value.y))
                .with("style", value.style.to_attribute())
                .with("clip-path", format!("url(#clip-{id}-{index})"))
                .raw_text(markup(&value.text)),
        );
    }
    group
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dollar_format_reads_its_specifier_when_a_value_is_formatted() {
        let format = ValueFormat::of("$xyz").expect("the format is built");
        assert!(format.call(5.0).is_err());
    }

    #[test]
    fn an_invalid_specifier_falls_back_to_the_comma_format() {
        let format = ValueFormat::of("xyz").expect("the format is built");
        assert_eq!(format.call(1234.0).ok().as_deref(), Some("1,234"));
    }

    #[test]
    fn the_dollar_formats_write_the_prefix_and_the_first_decimals() {
        let plain = ValueFormat::of("$0,0").expect("built");
        assert_eq!(plain.call(1_500_000.0).ok().as_deref(), Some("$1,500,000"));
        let decimals = ValueFormat::of("$,.2f").expect("built");
        assert_eq!(decimals.call(1_645_789.25).ok().as_deref(), Some("$1.6e+6"));
        let rest = ValueFormat::of("$.2f").expect("built");
        assert_eq!(rest.call(369.956).ok().as_deref(), Some("$369.96"));
    }

    #[test]
    fn the_first_decimals_of_a_specifier_are_taken_as_matched() {
        assert_eq!(decimals_of("$,.2f"), ".2");
        assert_eq!(decimals_of("$,"), "");
    }

    #[test]
    fn parse_float_reads_the_prefix_as_javascript_does() {
        assert_eq!(js_parse_float("38px"), 38.0);
        assert_eq!(js_parse_float("1.5em"), 1.5);
        assert!(js_parse_float("inherit").is_nan());
        assert_eq!(js_parse_int("38.9px"), 38.0);
    }

    #[test]
    fn a_leaf_too_small_for_its_label_hides_it() {
        let sizes = LeafSizes::of(3);
        let mut style = Inline::parse("font-size: 38px");
        fit_leaf_label("Label", 10.0, 10.0, sizes, &mut style).expect("fits");
        assert_eq!(style.get("display").as_deref(), Some("none"));
    }
}
