//! The tree view diagram: a port of `packages/mermaid/src/diagrams/treeView/renderer.ts`, `draw`
//! (lines 218-236), `drawTree` (130-216) and `positionLabel` and `positionLine` (55-128), with the
//! icons of [`icons`] and the styles of [`style`]. The research, with the reference's file and line
//! for every rule, is in `docs-port/tree-view/NOTES.md`, section 6.
//!
//! Nothing is laid out in a graph: each row is a text box with its padding, the rows stack from the
//! top, and a node's children are indented from it. The arithmetic runs in the order of the reference,
//! in three phases: the rows (`drawNode`, in document order), the descriptions (one column for every
//! row, after the widest label), and the highlights (each sized to the tree's width, and each one
//! widening it for the next). The label's box is the box `getBBox` gives its text in ferrigura-text's
//! faces: regular, bold for a directory (or a class `treeView-node-dir`), italic for a class
//! `treeView-node-description`, as the styles of the reference set them.

mod icons;
mod style;

use ferrigura_model::config::TreeViewDiagramConfig;
use ferrigura_model::tree_view::{NodeType, TreeViewDiagram, TreeViewNode};
use ferrigura_syntax::sanitize::decode_references;
use ferrigura_text::{
    EmbeddedFamily, Face, FontFamily, FontStyle, FontWeight, Run, embed_runs, face, measure_box,
    text_box,
};
use icons::Icon;
use style::Options;

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::decode_entities;
use crate::common::style::base_style_sheet;
use crate::common::viewbox::configure_svg_size;
use crate::context::Context;
use crate::svg::number;

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The room an icon takes before its label: `ICON_SIZE + ICON_GAP` (`renderer.ts` 17, 18 and 70).
const ICON_OFFSET: f64 = 14.0 + 4.0;

/// The gap between the widest label and the description column (`DESC_GAP`, `renderer.ts` 19).
const DESC_GAP: f64 = 16.0;

/// The highlight rectangle is the width of the tree less the rectangle's `x`, plus 8 (`renderer.ts`
/// 207), and it widens the tree to its right edge plus 2 (`renderer.ts` 211).
const HIGHLIGHT_OVERHANG: f64 = 8.0;
const HIGHLIGHT_MARGIN: f64 = 2.0;

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error when the label font size is not a size in pixels, when a bundled font cannot be
/// read or subset, or when the configuration names a value the reference fails on (an icon map's
/// entry that is not text, or a name the map reads from `Object.prototype`).
pub fn render(
    input: &Context<'_, TreeViewDiagram, TreeViewDiagramConfig>,
) -> Result<String, RenderError> {
    let diagram = input.diagram;
    let section = input.section;
    let id = input.id;
    let tokens = input.theme.light;
    let options = Options::of(tokens.tree_view.as_ref());
    let layout = lay_out(&diagram.root, section, options.label_px()?)?;

    // The fonts: every character a label or a description draws, in the face it is set in.
    let texts: Vec<(String, &'static Face)> = layout
        .rows
        .iter()
        .flat_map(|row| {
            let label = (shown_text(&row.node.name), face_of(row.bold, row.italic));
            let description = row
                .description
                .as_ref()
                .map(|description| (shown_text(description.text), face_of(false, true)));
            std::iter::once(label).chain(description)
        })
        .collect();
    let runs: Vec<Run<'_>> = texts
        .iter()
        .map(|(text, face)| Run { text, face })
        .collect();
    let families: Vec<EmbeddedFamily> = embed_runs(&runs)?;
    let font_family = families.first().map_or_else(
        || {
            tokens
                .font_family
                .text()
                .unwrap_or_else(|| "sans-serif".to_owned())
        },
        |family| family.css_family.clone(),
    );
    let font_size = tokens.font_size.text().unwrap_or_else(|| "16px".to_owned());
    let style = base_style_sheet(
        id,
        input.theme,
        &font_family,
        &font_size,
        &families,
        &style::own_rules(id, &options),
    );

    // The accessible title and description (`mermaidAPI`), which come first in the root.
    let mut accessibility = Vec::new();
    if !diagram.acc_title.is_empty() {
        accessibility.push(
            Element::new("title")
                .with("id", a11y::title_id(id))
                .text(shown_text(&diagram.acc_title)),
        );
    }
    if !diagram.acc_descr.is_empty() {
        accessibility.push(
            Element::new("desc")
                .with("id", a11y::description_id(id))
                .text(shown_text(&diagram.acc_descr)),
        );
    }

    // `draw`'s `viewBox` (`renderer.ts` 232-234), then `configureSvgSize` (235), on the root that
    // `mermaidAPI` made with its id, width and namespace.
    let mut root = Element::new("svg")
        .with("id", id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE)
        .with(
            "viewBox",
            format!(
                "-{} 0 {} {}",
                number(section.line_thickness / 2.0),
                number(layout.width),
                number(layout.height)
            ),
        );
    configure_svg_size(
        &mut root,
        layout.height,
        layout.width,
        section.use_max_width,
    );
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "treeView");
    if !diagram.acc_descr.is_empty() {
        root.set("aria-describedby", a11y::description_id(id));
    }
    if !diagram.acc_title.is_empty() {
        root.set("aria-labelledby", a11y::title_id(id));
    }

    for element in accessibility {
        root.push_child(element);
    }
    root.push_child(Element::new("style").raw_text(style));
    // The empty group `mermaidAPI` appends before the renderer draws (`mermaidAPI.ts` 419).
    root.push_child(Element::new("g"));
    let mut tree = Element::new("g").with("class", "tree-view");
    push_subtree(&layout, 0, section, &options, &mut tree);
    root.push_child(tree);
    Ok(root.render())
}

/// One row of the tree: a node and the box `positionLabel` gives it (`BBox`), with what is drawn for
/// it. The rows are in document order, the root first.
struct Row<'a> {
    node: &'a TreeViewNode,
    /// The class attribute of the label (`cssClasses`, `renderer.ts` 60-68).
    label_class: String,
    /// The icon the row shows, or none (`getNodeIcon` in `positionLabel`).
    icon: Option<Icon>,
    /// Whether the label is bold and italic, as the classes of its label make it (`styles.ts`).
    bold: bool,
    italic: bool,
    /// Whether the node's class list holds `highlight` (`renderer.ts` 96, 204).
    highlight: bool,
    /// The left edge of the box: the indent of the row's depth.
    x: f64,
    /// The top edge of the box: the heights of the rows above it.
    y: f64,
    /// The height of the box: the text box of the label and its padding.
    height: f64,
    /// The `x` of the label: after the padding and, with an icon, after the icon.
    label_x: f64,
    /// The right edge of the label (`labelRightEdge`).
    label_right: f64,
    /// The description of the node, and its `x` once the column is known.
    description: Option<Description<'a>>,
    /// The width of the highlight rectangle, set once the tree's width is known.
    rect_width: f64,
    /// The rows of the children, in order.
    children: Vec<usize>,
}

/// A description and its place: its text and its `x` in the description column.
struct Description<'a> {
    text: &'a str,
    x: f64,
}

/// The rows of the tree and the size of the drawing (`totalWidth` and `totalHeight`).
struct Layout<'a> {
    rows: Vec<Row<'a>>,
    width: f64,
    height: f64,
}

/// `drawTree` (`renderer.ts` 130-216): the rows in document order, the description column, and the
/// highlight rectangles, in three phases.
fn lay_out<'a>(
    root: &'a TreeViewNode,
    section: &TreeViewDiagramConfig,
    label_px: f64,
) -> Result<Layout<'a>, RenderError> {
    let mut layout = Layout {
        rows: Vec::new(),
        width: 0.0,
        height: 0.0,
    };
    place(&mut layout, root, 0, section, label_px)?;

    // Phase 2: the descriptions share one column, after the right edge of the widest label.
    if layout.rows.iter().any(|row| row.description.is_some()) {
        let max_label_right = layout
            .rows
            .iter()
            .map(|row| row.label_right)
            .fold(f64::NEG_INFINITY, f64::max);
        let x = max_label_right + DESC_GAP;
        for row in &mut layout.rows {
            if let Some(description) = &mut row.description {
                description.x = x;
                let width = measure(description.text, false, true, label_px)?.width;
                layout.width = layout.width.max(x + width + section.padding_x);
            }
        }
    }

    // Phase 3: each highlight is sized to the tree's width so far, and widens the tree in turn.
    let mut width = layout.width;
    for row in &mut layout.rows {
        if row.highlight {
            let rect_width = width - row.x + HIGHLIGHT_OVERHANG;
            row.rect_width = rect_width;
            width = width.max(row.x + rect_width + HIGHLIGHT_MARGIN);
        }
    }
    layout.width = width;
    Ok(layout)
}

/// `drawNode` (`renderer.ts` 139-161) and the children after it (`processNode`, 182): the row of
/// `node` at `depth`, then its children, each at the depth below. Returns the row's index.
fn place<'a>(
    layout: &mut Layout<'a>,
    node: &'a TreeViewNode,
    depth: usize,
    section: &TreeViewDiagramConfig,
    label_px: f64,
) -> Result<usize, RenderError> {
    let index = layout.rows.len();
    let indent = depth as f64 * (section.row_indent + section.padding_x);
    let icon = icons::node_icon(node, section)?;
    let offset = if icon.is_some() { ICON_OFFSET } else { 0.0 };
    let classes = label_classes(node);
    let bold = node.node_type == NodeType::Directory || classes.dir;
    let italic = classes.description;
    let label = measure(&node.name, bold, italic, label_px)?;
    let height = label.height + section.padding_y * 2.0;
    let label_x = indent + section.padding_x + offset;
    let label_right = label_x + label.width;
    let width = label.width + section.padding_x * 2.0 + offset;
    let y = layout.height;
    layout.rows.push(Row {
        node,
        label_class: classes.attribute,
        icon,
        bold,
        italic,
        highlight: node
            .css_class
            .split_whitespace()
            .any(|class| class == "highlight"),
        x: indent,
        y,
        height,
        label_x,
        label_right,
        description: (!node.description.is_empty()).then_some(Description {
            text: &node.description,
            x: 0.0,
        }),
        rect_width: 0.0,
        children: Vec::new(),
    });
    layout.width = layout.width.max(indent + width);
    layout.height += height;
    for child in &node.children {
        let child_index = place(layout, child, depth + 1, section, label_px)?;
        layout.rows[index].children.push(child_index);
    }
    Ok(index)
}

/// The classes of a label: `treeView-node-label`, then `treeView-node-dir` for a directory, then the
/// node's class (`renderer.ts` 60-68), as the attribute is written, and the two rules that set the
/// weight and the italic of the text.
struct LabelClasses {
    attribute: String,
    dir: bool,
    description: bool,
}

fn label_classes(node: &TreeViewNode) -> LabelClasses {
    let mut attribute = "treeView-node-label".to_owned();
    if node.node_type == NodeType::Directory {
        attribute.push_str(" treeView-node-dir");
    }
    if !node.css_class.is_empty() {
        attribute.push(' ');
        attribute.push_str(&node.css_class);
    }
    let classes: Vec<&str> = attribute.split_whitespace().collect();
    LabelClasses {
        dir: classes.contains(&"treeView-node-dir"),
        description: classes.contains(&"treeView-node-description"),
        attribute,
    }
}

/// The text box of a label or a description, in the face its style sets: `getBBox` of the text in
/// the reference, which is the box of the text's advance and glyphs, and its height is the text box.
/// An empty text has the empty box (`getBBox` of a `<text>` with no characters).
#[derive(Debug, Clone, Copy)]
struct Measured {
    width: f64,
    height: f64,
}

fn measure(text: &str, bold: bool, italic: bool, size: f64) -> Result<Measured, RenderError> {
    if text.is_empty() {
        return Ok(Measured {
            width: 0.0,
            height: 0.0,
        });
    }
    let face = face_of(bold, italic);
    let width = measure_box(face, text, size)
        .map_err(RenderError::Text)?
        .width();
    let height = text_box(face, size).map_err(RenderError::Text)?.height;
    Ok(Measured { width, height })
}

/// The face of a style: bold or regular, and italic or upright.
fn face_of(bold: bool, italic: bool) -> &'static Face {
    face(
        FontFamily::Sans,
        if bold {
            FontWeight::Bold
        } else {
            FontWeight::Normal
        },
        if italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
    )
}

/// The element of a row and of its trunk, in the order `drawTree` appends them: the row's group, the
/// connector of the row, the rows of the children, and the trunk that joins them to the row.
fn push_subtree(
    layout: &Layout<'_>,
    index: usize,
    section: &TreeViewDiagramConfig,
    options: &Options,
    out: &mut Element,
) {
    let row = &layout.rows[index];
    out.push_child(row_group(row, section, options));
    let center = row.y + row.height / 2.0;
    out.push_child(line(
        row.x - section.row_indent,
        center,
        row.x,
        center,
        section,
        options,
    ));
    for &child in &row.children {
        push_subtree(layout, child, section, options, out);
    }
    if let Some(&last) = row.children.last() {
        let last = &layout.rows[last];
        out.push_child(line(
            row.x + section.padding_x,
            row.y + row.height,
            row.x + section.padding_x,
            last.y + last.height / 2.0 + section.line_thickness / 2.0,
            section,
            options,
        ));
    }
}

/// The group of a row (`positionLabel`): the highlight rectangle first, then the icon, the label and
/// the description.
fn row_group(row: &Row<'_>, section: &TreeViewDiagramConfig, options: &Options) -> Element {
    let mut group = Element::new("g");
    if row.highlight {
        group.push_child(
            Element::new("rect")
                .with("x", number(row.x))
                .with("y", number(row.y + 1.0))
                .with("width", number(row.rect_width))
                .with("height", number(row.height - 2.0))
                .with("rx", "3")
                .with("class", "treeView-highlight-bg")
                .with("fill", options.highlight_bg.as_str())
                .with("stroke", options.highlight_stroke.as_str()),
        );
    }
    if let Some(icon) = row.icon {
        group.push_child(
            Element::new("g")
                .with("class", "treeView-node-icon")
                .with(
                    "transform",
                    format!(
                        "translate({}, {})",
                        number(row.x + section.padding_x),
                        number(row.y + section.padding_y)
                    ),
                )
                .with("color", options.icon_color.as_str())
                .raw_text(icon.markup()),
        );
    }
    group.push_child(
        Element::new("text")
            .with("dominant-baseline", "middle")
            .with("class", row.label_class.as_str())
            .with("x", number(row.label_x))
            .with("y", number(row.y + row.height / 2.0))
            .with("fill", options.label_color.as_str())
            .text(shown_text(&row.node.name)),
    );
    if let Some(description) = &row.description {
        group.push_child(
            Element::new("text")
                .with("dominant-baseline", "middle")
                .with("class", "treeView-node-description")
                .with("x", number(description.x))
                .with("y", number(row.y + row.height / 2.0))
                .with("fill", options.description_color.as_str())
                .text(shown_text(description.text)),
        );
    }
    group
}

/// `positionLine` (`renderer.ts` 118-128): a connector or a trunk, in the line's own colour.
fn line(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    section: &TreeViewDiagramConfig,
    options: &Options,
) -> Element {
    Element::new("line")
        .with("x1", number(x1))
        .with("y1", number(y1))
        .with("x2", number(x2))
        .with("y2", number(y2))
        .with("stroke-width", number(section.line_thickness))
        .with("class", "treeView-node-line")
        .with("stroke", options.line_color.as_str())
}

/// The text an SVG `text` element shows for a string of the diagram. The encoding wrote a
/// placeholder for each `&` and `#` entity the user typed (`encodeEntities`), and the reference turns
/// the placeholders into the references they stand for when it writes the markup (`decodeEntities`,
/// `utils.ts` 904-906, and `cleanUpSvgCode`); the browser reads those references. Every other
/// character is a character of the text, so it is escaped first, as the serialiser writes a text
/// node. The same rule as the packet renderer's.
fn shown_text(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    decode_references(&decode_entities(&escaped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_class_list_is_built_as_the_reference_builds_it() {
        let node = TreeViewNode {
            id: 1,
            level: 0,
            name: "src".to_owned(),
            node_type: NodeType::Directory,
            icon: String::new(),
            css_class: "highlight".to_owned(),
            description: String::new(),
            children: Vec::new(),
        };
        assert_eq!(
            label_classes(&node).attribute,
            "treeView-node-label treeView-node-dir highlight"
        );
    }
}
