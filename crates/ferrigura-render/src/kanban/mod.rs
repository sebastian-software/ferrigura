//! The kanban diagram: a port of `packages/mermaid/src/diagrams/kanban/kanbanRenderer.ts` (`draw`),
//! with `getData` (`kanbanDb.ts` 54-97) in [`data`], the section and item shapes in
//! `unified/shapes/kanban_section.rs` and `kanban_item.rs`, and the styles of `styles.ts` in
//! `style`. The research is in `docs-port/kanban/NOTES.md` (section 6).
//!
//! The layout is arithmetic on the sizes of the labels. A section is `width` wide, and the sections
//! are side by side with `padding / 2` between them. The items of a section are stacked from the
//! top of its rectangle, below the tallest title, and each item is as tall as its own group's box.
//! The rectangle of a section grows with its items. Every label is SVG text, laid out by
//! `unified::label` (decision 0004); the reference's default is HTML labels, which the harness
//! turns off (`htmlLabels: false`) and this port always uses.

pub mod data;
mod style;

use ferrigura_model::config::{KanbanDiagramConfig, MindmapDiagramConfig};
use ferrigura_model::flowchart::LabelType;
use ferrigura_model::kanban::KanbanDiagram;
use ferrigura_model::value::Value;
use ferrigura_syntax::sanitize::sanitize_text;
use ferrigura_text::{EmbeddedFamily, Run, embed_runs};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::decode_entities;
use crate::common::style::base_style_sheet;
use crate::common::viewbox::{BBox as ViewBox, setup_graph_viewbox};
use crate::context::Context;
use crate::svg::{XLINK_NAMESPACE, number};
use crate::unified::geom::BBox;
use crate::unified::label::{self, Anchor, LabelLayout, LabelSpec};
use crate::unified::shapes::kanban_item::{ItemPaint, KanbanItem, kanban_item};
use crate::unified::shapes::kanban_section::{KanbanSection, SectionPaint, kanban_section};
use data::DataNode;
use style::Schemes;

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The width of a section when the configuration gives none or zero (`sectionWidth || 200`).
const DEFAULT_SECTION_WIDTH: f64 = 200.0;

/// The gap between two sections is `padding / 2` (`kanbanRenderer.ts` 42, 51). The source says
/// "TODO set padding", so the constant is the renderer's own.
const PADDING: f64 = 10.0;

/// The padding of an item's labels: `node.padding || 10` (`kanbanItem.ts` 41). An item has no
/// padding of its own, so it is always 10.
const ITEM_PADDING: f64 = 10.0;

/// The smallest height of a section's title band (`maxLabelHeight`, `kanbanRenderer.ts` 45).
const MIN_LABEL_HEIGHT: f64 = 25.0;

/// The height of a section's rectangle is at least this (`kanbanRenderer.ts` 87).
const MIN_SECTION_HEIGHT: f64 = 50.0;

/// The corner radii of a section and an item (`kanbanRenderer.ts` 55-56, `kanbanDb.ts` 81-82).
const CORNER_RADIUS: f64 = 5.0;

/// The style of an item's labels: `text-align: left` as `styles2String` writes it (`kanbanItem.ts`
/// 29-30 and `kanbanDb.ts` 83).
const ITEM_LABEL_STYLE: &str = "text-align:left !important";

/// The sections of the configuration that the kanban renderer reads: its own section, and the
/// `mindmap` section that its viewBox takes (`kanbanRenderer.ts` 92-97).
#[derive(Debug, Clone, Copy)]
pub struct KanbanSections<'a> {
    /// The `kanban` section.
    pub kanban: &'a KanbanDiagramConfig,
    /// The `mindmap` section.
    pub mindmap: &'a MindmapDiagramConfig,
}

/// The labels of an item: its title, its ticket and its assignee, each at the item's label width.
struct ItemLabels {
    title: LabelLayout,
    ticket: LabelLayout,
    assigned: LabelLayout,
}

/// A section with the things its layout needs: its number, its position and its title.
struct SectionLayout<'a> {
    node: &'a DataNode,
    number: usize,
    x: f64,
    label: LabelLayout,
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read or subset, or a label cannot be laid out.
pub fn render(
    input: &Context<'_, KanbanDiagram, KanbanSections<'_>>,
) -> Result<String, RenderError> {
    let id = input.id;
    let nodes = data::get_data(input.diagram, &input.globals.look);
    let width = section_width(input.section.kanban.section_width);
    let text_size = input.theme.light.font_size.px().unwrap_or(16.0);
    let schemes = Schemes::new(input.theme);
    let palette = &schemes.primary;
    let item_width = width - 1.5 * PADDING;

    // `insertCluster` for each section (`kanbanRenderer.ts` 42-63): the title is laid out at the
    // section's width, the sections are side by side, and the tallest title sets the band above
    // the items (`maxLabelHeight`).
    let mut sections = Vec::new();
    let mut max_label_height = MIN_LABEL_HEIGHT;
    for node in nodes.iter().filter(|node| node.is_group) {
        let number = sections.len() + 1;
        let label = layout_label(&node.label, width, text_size, Anchor::Start, false)?;
        max_label_height = max_label_height.max(label.bbox.browser().height);
        sections.push(SectionLayout {
            node,
            number,
            x: width * number as f64 + ((number - 1) as f64 * PADDING) / 2.0,
            label,
        });
    }

    // The items of each section, placed from the top of its band (`kanbanRenderer.ts` 65-89). The
    // children of a section are the data nodes whose parent is its id, so a repeated section id
    // draws them under each copy.
    // The top of every rectangle is `node.y - node.height / 2` with the height `3 * width`
    // (`kanbanRenderer.ts` 52-54); the items start below the title band.
    let top = -width * 3.0 / 2.0;
    let item_top = top + max_label_height;
    let mut item_groups = String::new();
    let mut item_labels = Vec::new();
    let mut links = false;
    let mut root_bounds: Option<BBox> = None;
    let mut heights = Vec::with_capacity(sections.len());
    for section in &sections {
        let mut y = item_top;
        for item in nodes.iter().filter(|node| {
            !node.is_group && node.parent_id.as_deref() == Some(section.node.id.as_str())
        }) {
            let labels = item_labels_of(item, item_width, text_size)?;
            let priority: Option<&Value> = item.priority.as_ref();
            let link = ticket_link(item, &input.section.kanban.ticket_base_url);
            links |= link.is_some();
            let shape = kanban_item(&KanbanItem {
                dom_id: &format!("{id}-{}", item.id),
                title: &labels.title,
                ticket: &labels.ticket,
                assigned: &labels.assigned,
                width: item_width,
                padding: ITEM_PADDING,
                corner_radius: item.corner_radius.unwrap_or(CORNER_RADIUS),
                priority,
                label_style: ITEM_LABEL_STYLE,
                ticket_link: link.as_deref(),
                paint: ItemPaint {
                    background: &palette.background,
                    node_border: &palette.node_border,
                    text: &palette.text_color,
                },
            });
            // `item.y = y + bbox.height / 2`, from the box of the item's group (`:81-82`).
            let item_y = y + shape.bbox.height / 2.0;
            item_groups.push_str(&shape.group(section.x, item_y));
            include(&mut root_bounds, shape.bbox.translate(section.x, item_y));
            y = item_y + shape.bbox.height / 2.0 + PADDING / 2.0;
            item_labels.push(labels);
        }
        // The rectangle's height: the items, with three paddings, and at least 50 (`:86-88`).
        let height = (y - item_top + 3.0 * PADDING).max(MIN_SECTION_HEIGHT)
            + (max_label_height - MIN_LABEL_HEIGHT);
        heights.push(height);
    }

    // The sections (`insertCluster`, with the height now known): the colour of each is its number's
    // slot in the theme.
    let mut section_groups = String::new();
    for (section, height) in sections.iter().zip(&heights) {
        let paint = palette
            .section_slot(section.number)
            .map(|slot| SectionPaint {
                fill: &slot.fill,
                label: &slot.label,
                text: &palette.text_color,
            });
        let shape = kanban_section(&KanbanSection {
            dom_id: &format!("{id}-{}", section.node.id),
            number: section.number,
            look: section.node.look.as_deref().unwrap_or_default(),
            x: section.x,
            width,
            top,
            height: *height,
            corner_radius: CORNER_RADIUS,
            label: &section.label,
            paint,
        });
        section_groups.push_str(&shape.svg);
        include(&mut root_bounds, shape.bbox);
    }

    // The fonts: every glyph of every label, in the faces the labels use.
    let mut runs: Vec<Run<'_>> = Vec::new();
    for section in &sections {
        runs.extend(section.label.text_runs());
    }
    for labels in &item_labels {
        runs.extend(labels.title.text_runs());
        runs.extend(labels.ticket.text_runs());
        runs.extend(labels.assigned.text_runs());
    }
    let families: Vec<EmbeddedFamily> = embed_runs(&runs)?;
    let font_family = families.first().map_or_else(
        || {
            input
                .theme
                .light
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned())
        },
        |family| family.css_family.clone(),
    );
    let css_font_size = format!("{}px", number(text_size));
    let style = base_style_sheet(
        id,
        input.theme,
        &font_family,
        &css_font_size,
        &families,
        &style::own_rules(id, &schemes),
    );

    // `setupGraphViewbox` with `mindmap.padding` and `mindmap.useMaxWidth` (`kanbanRenderer.ts` 92-97).
    let bounds = root_bounds.unwrap_or_default();
    let mut root = Element::new("svg")
        .with("id", id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE);
    // A linked ticket writes `xlink:href`, whose prefix the root declares after `xmlns`.
    if links {
        root.set("xmlns:xlink", XLINK_NAMESPACE);
    }
    setup_graph_viewbox(
        &mut root,
        ViewBox {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        },
        input.section.mindmap.padding,
        input.section.mindmap.use_max_width,
    );
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "kanban");
    let root = root
        .child(Element::new("style").raw_text(style))
        // The empty group `mermaidAPI` appends before the renderer draws.
        .child(Element::new("g"))
        .child(
            Element::new("g")
                .with("class", "sections")
                .raw_text(section_groups),
        )
        .child(
            Element::new("g")
                .with("class", "items")
                .raw_text(item_groups),
        );
    Ok(root.render())
}

/// `sectionWidth || 200` (`kanbanRenderer.ts` 47, 68): zero, and any value that is not a number,
/// give the default.
fn section_width(configured: f64) -> f64 {
    if configured.is_finite() && configured != 0.0 {
        configured
    } else {
        DEFAULT_SECTION_WIDTH
    }
}

/// The labels of an item (`kanbanItem.ts` 34-81): the title is the item's label, and the ticket and
/// the assignee are laid out the same way, all at the label width (the item width less 10).
fn item_labels_of(
    item: &DataNode,
    item_width: f64,
    text_size: f64,
) -> Result<ItemLabels, RenderError> {
    let label_width = item_width - 10.0;
    // `insertLabel` and `labelHelper` sanitise the decoded label again (`util.ts` 95, 177).
    let decoded = |text: &str| sanitize_text(&decode_entities(text));
    Ok(ItemLabels {
        // `labelHelper` gives the title a background when the item has an icon (`util.ts` 60-61,
        // `addBackground`); the ticket and the assignee never do (`insertLabel`).
        title: layout_label(
            &decoded(&item.label),
            label_width,
            text_size,
            Anchor::Start,
            item.icon.is_some(),
        )?,
        ticket: layout_label(
            &decoded(item.ticket.as_deref().unwrap_or_default()),
            label_width,
            text_size,
            Anchor::Start,
            false,
        )?,
        assigned: layout_label(
            &decoded(item.assigned.as_deref().unwrap_or_default()),
            label_width,
            text_size,
            Anchor::Start,
            false,
        )?,
    })
}

/// A label of the kanban: Markdown, as `createText` takes it by default, set as a node label and
/// start-anchored (the kanban stylesheet does not centre `.label text`, so each line starts at the
/// label's origin).
fn layout_label(
    text: &str,
    width: f64,
    font_size: f64,
    anchor: Anchor,
    add_background: bool,
) -> Result<LabelLayout, RenderError> {
    label::layout(&LabelSpec {
        text,
        label_type: LabelType::Markdown,
        width,
        is_node: true,
        anchor,
        wrap_in_style: true,
        add_background,
        styles: &[],
        font_size,
        title: false,
    })
}

/// The link of an item's ticket (`kanbanItem.ts` 46-47): the ticket base URL with the first
/// `#TICKET#` replaced by the ticket. The item needs a ticket and the configuration a base URL.
fn ticket_link(item: &DataNode, base_url: &str) -> Option<String> {
    let ticket = item.ticket.as_deref().filter(|ticket| !ticket.is_empty())?;
    (!base_url.is_empty()).then(|| replace_first(base_url, "#TICKET#", ticket))
}

/// `String.prototype.replace` with a string pattern: the first occurrence is replaced, and the
/// replacement's `$$`, `$&`, `` $` `` and `$'` patterns are expanded, as JavaScript expands them.
/// Any other `$` is written as it is.
fn replace_first(text: &str, pattern: &str, replacement: &str) -> String {
    let Some(position) = text.find(pattern) else {
        return text.to_owned();
    };
    let before = &text[..position];
    let after = &text[position + pattern.len()..];
    let mut expanded = String::with_capacity(replacement.len());
    let mut rest = replacement;
    while let Some(dollar) = rest.find('$') {
        expanded.push_str(&rest[..dollar]);
        let tail = &rest[dollar..];
        let (written, length) = match tail.as_bytes().get(1) {
            Some(b'$') => ("$", 2),
            Some(b'&') => (pattern, 2),
            Some(b'`') => (before, 2),
            Some(b'\'') => (after, 2),
            _ => ("$", 1),
        };
        expanded.push_str(written);
        rest = &tail[length..];
    }
    expanded.push_str(rest);
    format!("{before}{expanded}{after}")
}

/// Adds a box to the box of the root's content, as `getBBox` takes the union of the children.
fn include(bounds: &mut Option<BBox>, extra: BBox) {
    if extra.is_empty() {
        return;
    }
    *bounds = Some(bounds.map_or(extra, |so_far| so_far.union(extra)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_or_missing_section_width_is_the_default() {
        assert_eq!(section_width(0.0), 200.0);
        assert_eq!(section_width(f64::NAN), 200.0);
        assert_eq!(section_width(250.0), 250.0);
    }

    #[test]
    fn the_ticket_replaces_the_first_placeholder_as_string_replace_does() {
        assert_eq!(
            replace_first("https://x/#TICKET#/#TICKET#", "#TICKET#", "MC-1"),
            "https://x/MC-1/#TICKET#"
        );
        assert_eq!(
            replace_first("no placeholder", "#TICKET#", "a"),
            "no placeholder"
        );
        // The replacement's `$` patterns expand, as they do in JavaScript: `$'` is the text after the
        // match, which the result keeps after the replacement as well.
        assert_eq!(
            replace_first("u/#TICKET#/z", "#TICKET#", "$&$$$'"),
            "u/#TICKET#$/z/z"
        );
        assert_eq!(replace_first("u/#TICKET#", "#TICKET#", "5$x"), "u/5$x");
    }
}
