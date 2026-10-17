//! The sankey diagram: a port of `packages/mermaid/src/diagrams/sankey/sankeyRenderer.ts` (`draw`)
//! and of its `styles.js`. The layout is `d3-sankey` 0.12.3 ported in [`layout`]; its operations
//! and their order are the library's, because the last digits of the positions are part of the
//! geometry. The research, with the reference's file and line for each rule, is in
//! `docs-port/sankey/NOTES.md` (section 6).
//!
//! The labels are SVG text: the reference's `text` elements, one per node (and two per node when
//! the labels are outlined). Their boxes are measured with `ferrigura-text` the way `getBBox` takes
//! them, and they take part in the root's `viewBox` when they spill past the layout. A value that is
//! not finite gives NaN coordinates, as the reference does, and the render still succeeds.

pub mod layout;
mod style;

use std::collections::HashMap;

use ferrigura_model::config::MermaidConfig;
use ferrigura_model::sankey::SankeyDiagram;
use ferrigura_model::value::Value;
use ferrigura_text::{Face, FontFamily, FontStyle, FontWeight, Run, embed_runs, face, text_box};

use crate::RenderError;
use crate::a11y;
use crate::common::Element;
use crate::common::d3::{
    OrdinalScale, SCHEME_TABLEAU10, js_number, js_round, link_horizontal, math_max,
};
use crate::common::style::base_style_sheet_with;
use crate::common::text::{collapse_white_space, collapsed_box, collapsed_width};
use crate::common::viewbox::{BBox as ViewBox, setup_graph_viewbox};
use crate::svg::number;
use crate::theme::Themes;
use crate::unified::geom::BBox;
use crate::variables::{px, text_of};
use layout::{Align, LinkSpec, Node, Settings, sankey};
use style::{Schemes, own_rules};

/// The XML namespace of the root element.
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// The `font-size` of the labels: the reference writes it on `g.node-labels` (`sankeyRenderer.ts` 206).
const LABEL_SIZE: f64 = 14.0;

/// The gap between a node and its label (`sankeyRenderer.ts` 194-202).
const LABEL_GAP: f64 = 6.0;

/// The padding a label gets for its values: `nodePadding + 15` (`sankeyRenderer.ts` 124).
const VALUE_PADDING: f64 = 15.0;

/// What the sankey renderer draws.
#[derive(Debug, Clone, Copy)]
pub struct SankeyInput<'a> {
    /// The parsed diagram.
    pub diagram: &'a SankeyDiagram,
    /// The effective configuration, in the sankey diagram's scope.
    pub config: &'a MermaidConfig,
    /// The colours and sizes of the theme.
    pub themes: Themes<'a>,
    /// The id of the SVG element. It scopes every selector and prefixes every element id, so it
    /// must be a CSS identifier.
    pub id: &'a str,
}

/// How the links are painted (`sankeyRenderer.ts` 239-273): a gradient from the source colour to the
/// target colour, the colour of one end, or a colour the configuration gives.
#[derive(Debug, Clone, PartialEq)]
enum LinkPaint {
    Gradient,
    Source,
    Target,
    Fixed(String),
}

/// A node's label: its text, its position, and its `dy` and anchor as the reference writes them.
struct Label {
    text: String,
    x: f64,
    y: f64,
    /// The `dy` attribute: `0em` with values, `0.35em` without.
    dy: &'static str,
    /// The shift `dy` gives at the label's font size, in pixels.
    dy_px: f64,
    /// `text-anchor`: `start` or `end`.
    anchor: &'static str,
}

/// Renders the diagram to an SVG document.
///
/// # Errors
///
/// Returns an error if the layout finds a cycle (`circular link`, the one error the reference
/// throws from the layout), or if a bundled font cannot be read or subset.
pub fn render(input: &SankeyInput<'_>) -> Result<String, RenderError> {
    let config = &input.config.sankey;
    let themes = input.themes;
    let variables = themes.primary();
    let id = input.id;
    let diagram = input.diagram;
    let sans = face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal);

    let show_values = config.show_values;
    let width = config.width;
    let links = link_specs(diagram)?;
    let settings = Settings {
        width,
        height: config.height,
        node_width: config.node_width,
        node_padding: config.node_padding + if show_values { VALUE_PADDING } else { 0.0 },
        align: alignment(&config.node_alignment)?,
    };
    let layout = sankey(diagram.nodes.len(), &links, settings)
        .map_err(|_| RenderError::Layout("circular link".to_owned()))?;

    // `getNodeColor` (`sankeyRenderer.ts` 149-151), called for each node in order by the rectangles.
    let mut scale = OrdinalScale::new(&SCHEME_TABLEAU10);
    let custom = config.node_colors.as_ref();
    let colours: Vec<String> = diagram
        .nodes
        .iter()
        .map(|node| node_colour(custom, &node.id, &mut scale))
        .collect();

    let outlined = config.label_style == "outlined";
    let central = central_layer(&layout.nodes);
    let labels: Vec<Label> = layout
        .nodes
        .iter()
        .zip(&diagram.nodes)
        .map(|(node, named)| {
            let (x, anchor) = label_position(node, outlined, central, width);
            Label {
                text: label_text(
                    &named.id,
                    node.value,
                    show_values,
                    &config.prefix,
                    &config.suffix,
                ),
                x,
                y: (node.y1 + node.y0) / 2.0,
                dy: if show_values { "0em" } else { "0.35em" },
                dy_px: if show_values { 0.0 } else { 0.35 * LABEL_SIZE },
                anchor,
            }
        })
        .collect();

    // The font subset covers the text the browser draws: its newline is a space there, so the space
    // is in the subset (without it the browser falls back to a glyph of another width).
    let drawn: Vec<String> = labels
        .iter()
        .map(|label| collapse_white_space(&label.text))
        .collect();
    let runs: Vec<Run<'_>> = drawn.iter().map(|text| Run { text, face: sans }).collect();
    let families = embed_runs(&runs)?;
    let font_family = families.first().map_or_else(
        || text_of(variables, "fontFamily").unwrap_or_else(|| "sans-serif".to_owned()),
        |family| format!("\"{}\"", family.name),
    );
    let text_size = px(variables, "fontSize").unwrap_or(16.0);
    let schemes = Schemes::new(themes);
    let style = base_style_sheet_with(
        id,
        themes,
        &font_family,
        &format!("{}px", number(text_size)),
        &families,
        &own_rules(id, &font_family, &schemes),
    );

    // The bounding box of the drawing, which `setupGraphViewbox` takes (`sankeyRenderer.ts` 281).
    let mut bounds: Option<BBox> = None;
    for node in &layout.nodes {
        include(
            &mut bounds,
            finite_box(node.x0, node.y0, node.x1 - node.x0, node.y1 - node.y0),
        );
    }
    for label in &labels {
        let x = svg_length(label.x);
        let baseline = svg_length(label.y) + label.dy_px;
        if let Some(extent) = label_box(&label.text, x, baseline, label.anchor, sans)? {
            include(&mut bounds, extent);
        }
    }
    for link in &layout.links {
        let source = &layout.nodes[link.source];
        let target = &layout.nodes[link.target];
        let (x0, y0, x1, y1) = (source.x1, link.y0, target.x0, link.y1);
        if [x0, y0, x1, y1].iter().all(|value| value.is_finite()) {
            include(
                &mut bounds,
                BBox::new(x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs()),
            );
        }
    }

    // The nodes: one group per node, with its rectangle (`sankeyRenderer.ts` 154-172). Uids come
    // in the order the reference takes them: the nodes, then the gradients of the links.
    let mut uid = 0_usize;
    let mut nodes_group = Element::new("g").with("class", "nodes");
    for (index, node) in layout.nodes.iter().enumerate() {
        uid += 1;
        nodes_group.push_child(
            Element::new("g")
                .with("class", "node")
                .with("id", format!("node-{uid}"))
                .with(
                    "transform",
                    format!("translate({},{})", js_number(node.x0), js_number(node.y0)),
                )
                .with("x", js_number(node.x0))
                .with("y", js_number(node.y0))
                .child(
                    Element::new("rect")
                        .with("height", js_number(node.y1 - node.y0))
                        .with("width", js_number(node.x1 - node.x0))
                        .with("fill", colours[index].as_str()),
                ),
        );
    }

    // The labels (`sankeyRenderer.ts` 206-225): the outlined style draws every background first,
    // then every foreground, and the legacy style one text per node.
    let palette = &schemes.primary;
    let mut labels_group = Element::new("g")
        .with("class", "node-labels")
        .with("font-size", "14");
    if outlined {
        for label in &labels {
            labels_group.push_child(label_element(
                label,
                Some("sankey-label-bg"),
                Some(("stroke", palette.label_background.as_str())),
            ));
        }
        for label in &labels {
            labels_group.push_child(label_element(
                label,
                Some("sankey-label-fg"),
                Some(("fill", palette.label_text.as_str())),
            ));
        }
    } else {
        for label in &labels {
            labels_group.push_child(label_element(label, None, None));
        }
    }

    // The links (`sankeyRenderer.ts` 228-279): a group per link, with its gradient and its path.
    let paint = link_paint(config.link_color.as_ref());
    let mut links_group = Element::new("g")
        .with("class", "links")
        .with("fill", "none")
        .with("stroke-opacity", "0.5");
    for link in &layout.links {
        let source = &layout.nodes[link.source];
        let target = &layout.nodes[link.target];
        let mut group = Element::new("g")
            .with("class", "link")
            .with("style", "mix-blend-mode: multiply;");
        let stroke = match &paint {
            LinkPaint::Gradient => {
                uid += 1;
                let gradient = format!("linearGradient-{uid}");
                group.push_child(
                    Element::new("linearGradient")
                        .with("id", gradient.as_str())
                        .with("gradientUnits", "userSpaceOnUse")
                        .with("x1", js_number(source.x1))
                        .with("x2", js_number(target.x0))
                        .child(
                            Element::new("stop")
                                .with("offset", "0%")
                                .with("stop-color", colours[link.source].as_str()),
                        )
                        .child(
                            Element::new("stop")
                                .with("offset", "100%")
                                .with("stop-color", colours[link.target].as_str()),
                        ),
                );
                format!("url(#{gradient})")
            }
            LinkPaint::Source => colours[link.source].clone(),
            LinkPaint::Target => colours[link.target].clone(),
            LinkPaint::Fixed(colour) => colour.clone(),
        };
        group.push_child(
            Element::new("path")
                .with("d", link_horizontal(source.x1, link.y0, target.x0, link.y1))
                .with("stroke", stroke)
                .with("stroke-width", js_number(math_max(1.0, link.width))),
        );
        links_group.push_child(group);
    }

    // `getBBox` returns float32 values, so the box is rounded to them.
    let bounds = bounds.unwrap_or_default().browser();
    let mut root = Element::new("svg")
        .with("id", id)
        .with("width", "100%")
        .with("xmlns", SVG_NAMESPACE);
    setup_graph_viewbox(
        &mut root,
        ViewBox {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        },
        0.0,
        config.use_max_width,
    );
    root.set("role", a11y::SVG_ROLE);
    root.set("aria-roledescription", "sankey");
    let root = root
        .child(Element::new("style").raw_text(style))
        // The empty group `mermaidAPI` appends before the renderer draws.
        .child(Element::new("g"))
        .child(nodes_group)
        .child(labels_group)
        .child(links_group);
    Ok(root.render())
}

/// The links and nodes as the layout takes them: each name is the index of its node, in the order
/// the DB created them.
fn link_specs(diagram: &SankeyDiagram) -> Result<Vec<LinkSpec>, RenderError> {
    let index: HashMap<&str, usize> = diagram
        .nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.id.as_str(), position))
        .collect();
    let find = |name: &str| {
        index
            .get(name)
            .copied()
            .ok_or_else(|| RenderError::Layout(format!("missing: {name}")))
    };
    diagram
        .links
        .iter()
        .map(|link| {
            Ok(LinkSpec {
                source: find(&link.source)?,
                target: find(&link.target)?,
                value: link.value,
            })
        })
        .collect()
}

/// `nodeAlignment` (`sankeyRenderer.ts` 26-35): the four alignments of the library. Any other name
/// is not in the reference's map, so its alignment is `undefined`, and the layout fails on the
/// column lengths (`Cannot read properties of undefined (reading 'length')`, probed as
/// `p32-unknown-alignment`).
fn alignment(name: &str) -> Result<Align, RenderError> {
    match name {
        "left" => Ok(Align::Left),
        "right" => Ok(Align::Right),
        "center" => Ok(Align::Center),
        "justify" => Ok(Align::Justify),
        _ => Err(RenderError::Layout(
            "Cannot read properties of undefined (reading 'length')".to_owned(),
        )),
    }
}

/// `linkColor` (`sankeyRenderer.ts` 239, 261-273): `gradient` by default, `source` and `target`, or
/// the colour the configuration gives, written as it is.
fn link_paint(value: Option<&Value>) -> LinkPaint {
    match value {
        None | Some(Value::Null) => LinkPaint::Gradient,
        Some(Value::String(text)) => match text.as_str() {
            "gradient" => LinkPaint::Gradient,
            "source" => LinkPaint::Source,
            "target" => LinkPaint::Target,
            other => LinkPaint::Fixed(other.to_owned()),
        },
        Some(Value::Number(number_value)) => LinkPaint::Fixed(js_number(*number_value)),
        Some(Value::Bool(flag)) => LinkPaint::Fixed(flag.to_string()),
        Some(_) => LinkPaint::Fixed("[object Object]".to_owned()),
    }
}

/// `nodeColors[nodeId] ?? colorScheme(nodeId)` (`sankeyRenderer.ts` 149-151). The lookup is on a
/// plain object, so a name of `Object.prototype` gives the member, which is written as the text of
/// that member (see [`object_prototype_member`]).
fn node_colour(custom: Option<&Value>, name: &str, scale: &mut OrdinalScale<'_>) -> String {
    custom_colour(custom, name)
        .or_else(|| object_prototype_member(name).map(str::to_owned))
        .unwrap_or_else(|| scale.apply(name).to_owned())
}

/// The configured colour of a node, when the configuration has a string for its name.
fn custom_colour(custom: Option<&Value>, name: &str) -> Option<String> {
    match custom?.get(name)? {
        Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

/// The text of a member of `Object.prototype`, as `String` gives it: the functions are written as
/// their source, which is `[native code]`, and `__proto__` is the prototype itself. A name that is
/// not one of them has no member, and the lookup gives `undefined`.
fn object_prototype_member(name: &str) -> Option<&'static str> {
    Some(match name {
        "constructor" => "function Object() { [native code] }",
        "__proto__" => "[object Object]",
        "__defineGetter__" => "function __defineGetter__() { [native code] }",
        "__defineSetter__" => "function __defineSetter__() { [native code] }",
        "hasOwnProperty" => "function hasOwnProperty() { [native code] }",
        "__lookupGetter__" => "function __lookupGetter__() { [native code] }",
        "__lookupSetter__" => "function __lookupSetter__() { [native code] }",
        "isPrototypeOf" => "function isPrototypeOf() { [native code] }",
        "propertyIsEnumerable" => "function propertyIsEnumerable() { [native code] }",
        "toString" => "function toString() { [native code] }",
        "valueOf" => "function valueOf() { [native code] }",
        "toLocaleString" => "function toLocaleString() { [native code] }",
        _ => return None,
    })
}

/// `Math.round(value * 100) / 100` (`sankeyRenderer.ts` 178), with the prefix and the suffix, on a
/// second line of the text. Without values, the text is the name.
fn label_text(name: &str, value: f64, show_values: bool, prefix: &str, suffix: &str) -> String {
    if show_values {
        format!(
            "{name}\n{prefix}{}{suffix}",
            js_number(js_round(value * 100.0) / 100.0)
        )
    } else {
        name.to_owned()
    }
}

/// `findCentralNodeLayer` (`sankeyRenderer.ts` 44-57): the layer of the node with the largest value.
fn central_layer(nodes: &[Node]) -> usize {
    let mut max_value = 0.0;
    let mut central = 0;
    for node in nodes {
        if node.value > max_value {
            max_value = node.value;
            central = node.layer;
        }
    }
    central
}

/// `getLabelPosition` (`sankeyRenderer.ts` 189-203): the x and the anchor of a label. An outlined
/// label is placed by the layer, the others by the position of the node.
fn label_position(node: &Node, outlined: bool, central: usize, width: f64) -> (f64, &'static str) {
    if outlined {
        if node.layer < central {
            (node.x0 - LABEL_GAP, "end")
        } else {
            (node.x1 + LABEL_GAP, "start")
        }
    } else if node.x0 < width / 2.0 {
        (node.x1 + LABEL_GAP, "start")
    } else {
        (node.x0 - LABEL_GAP, "end")
    }
}

/// The `text` element of a label, with its class and its paint (the presentation attribute of the
/// outlined labels' colour, decision 0007), when it has them. The attributes come in the order the
/// reference sets them.
fn label_element(label: &Label, class: Option<&str>, paint: Option<(&str, &str)>) -> Element {
    let mut element = Element::new("text");
    if let Some(class) = class {
        element = element.with("class", class);
    }
    element = element
        .with("x", js_number(label.x))
        .with("y", js_number(label.y))
        .with("dy", label.dy)
        .with("text-anchor", label.anchor);
    if let Some((name, value)) = paint {
        element = element.with(name, value);
    }
    element.text(label.text.as_str())
}

/// The box `getBBox` gives a label at `x` and `baseline`, with its anchor: the glyph box of the text
/// (`collapsed_box`), moved by the advance for `end`, and between the text box's ascent and descent
/// around the baseline. A label with no glyph has no box.
fn label_box(
    text: &str,
    x: f64,
    baseline: f64,
    anchor: &str,
    sans: &'static Face,
) -> Result<Option<BBox>, RenderError> {
    let extent = collapsed_box(sans, text, LABEL_SIZE)?;
    if extent.width() <= 0.0 {
        return Ok(None);
    }
    let start = if anchor == "end" {
        x - collapsed_width(sans, text, LABEL_SIZE)?
    } else {
        x
    };
    let metrics = text_box(sans, LABEL_SIZE)?;
    Ok(Some(BBox::new(
        start + extent.left,
        baseline - metrics.ascent,
        extent.width(),
        metrics.height,
    )))
}

/// A coordinate as the browser reads an SVG attribute for a box: a value that is not a number
/// (NaN, or infinite) is not a length, so it is `0`.
fn svg_length(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

/// The box of a rectangle at `(x, y)` with the given size, or nothing when any of the four is not
/// finite: the browser drops such a rectangle, which then adds nothing to the drawing's box.
fn finite_box(x: f64, y: f64, width: f64, height: f64) -> Option<BBox> {
    [x, y, width, height]
        .iter()
        .all(|value| value.is_finite())
        .then(|| BBox::new(x, y, width, height))
}

/// Adds a box to the box of the drawing, as `getBBox` takes the union of the children. An empty
/// box, and nothing, leave it as it is.
fn include(bounds: &mut Option<BBox>, extra: impl Into<Option<BBox>>) {
    let Some(extra) = extra.into() else {
        return;
    };
    if extra.is_empty() {
        return;
    }
    *bounds = Some(bounds.map_or(extra, |so_far| so_far.union(extra)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_has_the_value_on_its_second_line() {
        assert_eq!(label_text("a", 10.0, true, "", ""), "a\n10");
        assert_eq!(label_text("a", 10.0, false, "$", "B"), "a");
    }

    #[test]
    fn a_value_is_rounded_to_two_places_as_math_round_rounds_it() {
        assert_eq!(label_text("a", 0.597, true, "", ""), "a\n0.6");
        assert_eq!(label_text("a", 2.675, true, "$", "B"), "a\n$2.68B");
        assert_eq!(label_text("a", f64::NAN, true, "", ""), "a\nNaN");
    }

    #[test]
    fn a_colour_of_the_prototype_is_the_text_the_reference_writes() {
        assert_eq!(
            object_prototype_member("constructor"),
            Some("function Object() { [native code] }")
        );
        assert_eq!(
            object_prototype_member("__proto__"),
            Some("[object Object]")
        );
        assert_eq!(object_prototype_member("name"), None);
    }

    #[test]
    fn the_configured_colour_of_a_name_is_used_before_the_scheme() {
        let mut custom = Value::empty_object();
        if let Some(map) = custom.as_object_mut() {
            map.insert("b".to_owned(), Value::String("#00ff00".to_owned()));
        }
        let mut scale = OrdinalScale::new(&SCHEME_TABLEAU10);
        assert_eq!(node_colour(Some(&custom), "b", &mut scale), "#00ff00");
        assert_eq!(node_colour(Some(&custom), "a", &mut scale), "#4e79a7");
    }

    #[test]
    fn a_non_finite_length_is_zero_for_a_box() {
        assert_eq!(svg_length(f64::NAN), 0.0);
        assert_eq!(svg_length(3.5), 3.5);
    }

    #[test]
    fn a_box_with_a_non_finite_value_is_left_out() {
        assert!(finite_box(0.0, f64::NAN, 10.0, 10.0).is_none());
        let mut bounds = None;
        include(&mut bounds, finite_box(0.0, 0.0, 0.0, 10.0));
        assert_eq!(bounds, None);
    }
}
