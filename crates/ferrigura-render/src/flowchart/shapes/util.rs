//! Helpers of the shape handlers: the label as `labelHelper` sizes it, the styles of a node as
//! `styles2String` splits them, the node's classes, and the attribute and path strings that the
//! handlers share. See `docs-port/flowchart/render-notes/shapes-core.md` §1.

use ferrigura_model::value::Value;

use super::{Intersect, ShapeInput};
use crate::flowchart::data::Node;
use crate::flowchart::geom::{BBox, Point};
use crate::flowchart::label::LabelLayout;
use crate::svg::{escape, number};

/// What a shape handler draws, before `draw` adds the label's box to the node's box.
#[derive(Debug, Clone)]
pub(super) struct Part {
    /// The shape's own elements, in the node's coordinates.
    pub svg: String,
    /// Elements drawn after the label.
    pub after_label: String,
    /// The box of the shape's own elements, as their `getBBox` gives it in the node's
    /// coordinates: the transforms of the elements are applied, the label is not included.
    pub bbox: BBox,
    /// How edges meet the outline.
    pub intersect: Intersect,
    /// The offset of the label group, when the shape draws a label.
    pub label_offset: Option<Point>,
    /// The label, widened as the shape widens it.
    pub label: Option<LabelLayout>,
}

impl Part {
    /// A part without a label.
    pub(super) fn without_label(svg: String, bbox: BBox, intersect: Intersect) -> Self {
        Self {
            svg,
            after_label: String::new(),
            bbox,
            intersect,
            label_offset: None,
            label: None,
        }
    }

    /// A part with the label at the centred offset `translate(0, -H/2)`, which is what most
    /// shapes do.
    pub(super) fn with_centred_label(
        svg: String,
        bbox: BBox,
        intersect: Intersect,
        label: LabelLayout,
    ) -> Self {
        Self {
            svg,
            after_label: String::new(),
            bbox,
            intersect,
            label_offset: Some(centred_label_offset(&label)),
            label: Some(label),
        }
    }
}

/// The label as `labelHelper` leaves it: the label at its own size, widened to the node's minimum
/// width unless the shape ignores that (`ignoreMinWidth`). An empty label and a node with an
/// explicit width are not widened (`shapes/util.ts:92`).
pub(super) fn label_helper(input: &ShapeInput<'_>, ignore_min_width: bool) -> LabelLayout {
    let node = input.node;
    let mut label = input
        .label
        .cloned()
        .unwrap_or_else(|| LabelLayout::of_box(BBox::default()));
    let min_width =
        if !node.label.is_empty() && node.width.unwrap_or(0.0) == 0.0 && !ignore_min_width {
            node.min_width.unwrap_or(0.0)
        } else {
            0.0
        };
    label.bbox = with_min_width(label.bbox, min_width);
    label
}

/// `withMinWidth` for SVG text (`shapes/util.ts:26-48`): a box narrower than the minimum is
/// widened to it and stays centred, so its x moves left by half the difference.
#[must_use]
pub(super) fn with_min_width(bbox: BBox, min_width: f64) -> BBox {
    if bbox.width >= min_width {
        return bbox;
    }
    BBox::new(
        bbox.x - (min_width - bbox.width) / 2.0,
        bbox.y,
        min_width,
        bbox.height,
    )
}

/// The offset of a centred label: `translate(0, -H/2)` (`shapes/util.ts:147`).
pub(super) fn centred_label_offset(label: &LabelLayout) -> Point {
    Point::new(0.0, -label.bbox.height / 2.0)
}

/// The node's declarations as `compileStyles` and `styles2String` split them.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Styles {
    /// The label declarations (`labelStyles`), joined with `;`.
    pub label: String,
    /// The other declarations (`nodeStyles`), joined with `;`.
    pub node: String,
    /// Every declaration, compiled, as `(key, value)`.
    pub entries: Vec<(String, String)>,
}

/// The node's declarations as `styles2String` (`shapes/handDrawnShapeStyles.ts:75-104`) writes
/// them: each one `key:value !important`.
pub(super) fn styles2string(node: &Node) -> Styles {
    let entries = compile_styles(node);
    let mut label = Vec::new();
    let mut other = Vec::new();
    for (key, value) in &entries {
        let declaration = format!("{key}:{value} !important");
        if is_label_style(key) {
            label.push(declaration);
        } else {
            other.push(declaration);
        }
    }
    Styles {
        label: label.join(";"),
        node: other.join(";"),
        entries,
    }
}

/// `compileStyles` and `styles2Map` (`shapes/handDrawnShapeStyles.ts:31-51`): the classes'
/// declarations, then the node's, then its label's, the later ones winning. A declaration keeps
/// the position of its key. Only the text between the first two colons is the value, as
/// `style.split(':')` destructures it.
fn compile_styles(node: &Node) -> Vec<(String, String)> {
    let label_styles = node
        .label_style
        .split(';')
        .map(str::trim)
        .filter(|style| !style.is_empty());
    let mut entries: Vec<(String, String)> = Vec::new();
    for style in node
        .css_compiled_styles
        .iter()
        .map(String::as_str)
        .chain(node.css_styles.iter().map(String::as_str))
        .chain(label_styles)
    {
        let mut parts = style.split(':');
        let key = parts.next().unwrap_or_default().trim().to_owned();
        let value = parts
            .next()
            .map(|value| value.trim().to_owned())
            .unwrap_or_default();
        match entries.iter_mut().find(|(existing, _)| *existing == key) {
            Some(entry) => entry.1 = value,
            None => entries.push((key, value)),
        }
    }
    entries
}

/// `isLabelStyle` (`shapes/handDrawnShapeStyles.ts:53-74`): the declarations that style the text.
fn is_label_style(key: &str) -> bool {
    matches!(
        key,
        "color"
            | "font-size"
            | "font-family"
            | "font-weight"
            | "font-style"
            | "text-decoration"
            | "text-align"
            | "text-transform"
            | "line-height"
            | "letter-spacing"
            | "word-spacing"
            | "text-shadow"
            | "text-overflow"
            | "white-space"
            | "word-wrap"
            | "word-break"
            | "overflow-wrap"
            | "hyphens"
    )
}

/// The inline style of a node's `<text>` (`createText.ts:377-388`): the label declarations
/// without stroke and fill, with `color` as `fill`.
#[must_use]
pub(super) fn node_text_style(label_style: &str) -> String {
    let mut style = label_style.to_owned();
    if let Some(index) = style.find("stroke:") {
        style.replace_range(index..index + "stroke:".len(), "lineColor:");
    }
    let style = remove_declarations(&style, "stroke:");
    let style = remove_declarations(&style, "stroke-width:");
    let style = remove_declarations(&style, "fill:");
    style.replace("color:", "fill:")
}

/// Removes every `key[^;]+;?` of a style text, as the reference's global regular expressions do.
fn remove_declarations(text: &str, key: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(index) = rest.find(key) {
        out.push_str(&rest[..index]);
        let after = &rest[index + key.len()..];
        let run = after.find(';').unwrap_or(after.len());
        if run == 0 {
            // `[^;]+` needs one character, so the key stays where it is.
            out.push_str(key);
            rest = after;
            continue;
        }
        let consumed = if after[run..].starts_with(';') {
            run + 1
        } else {
            run
        };
        rest = &after[consumed..];
    }
    out.push_str(rest);
    out
}

/// The class of a node's group: `node` (or `rough-node` for the hand-drawn look) and the node's
/// classes, as `getNodeClasses` writes them (`shapes/util.ts:249-250`). The reference's probe
/// reads `node default` for an unclassed vertex, with no trailing space.
#[must_use]
pub(super) fn node_classes(node: &Node) -> String {
    let base = if node.look == "handDrawn" {
        "rough-node"
    } else {
        "node"
    };
    format!("{base} {}", node.css_classes.trim())
}

/// The style of a shape's paths, as the reference sets it with `selectAll('path')`: the node's
/// declarations when there are any, else the declarations the node was given as a list. It is
/// always present, as the reference writes it for every outline that takes the node's style.
#[must_use]
pub(super) fn path_style(node: &Node, styles: &Styles) -> String {
    if styles.node.is_empty() {
        node.css_styles.join(",")
    } else {
        styles.node.clone()
    }
}

/// The stroke width of a rough outline: the node's `stroke-width` without `px`, else 1.3
/// (`userNodeOverrides`, `shapes/handDrawnShapeStyles.ts:108-130`).
#[must_use]
pub(super) fn stroke_width(styles: &Styles) -> String {
    styles
        .entries
        .iter()
        .find(|(key, _)| key == "stroke-width")
        .map(|(_, value)| value.replace("px", ""))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "1.3".to_owned())
}

/// The attribute `name="value"` with its leading space, the value escaped.
#[must_use]
pub(super) fn attribute(name: &str, value: &str) -> String {
    format!(" {name}=\"{}\"", escape(value))
}

/// A list of points as an SVG `points` attribute: `x,y x,y ...`.
#[must_use]
pub(super) fn points_attribute(points: &[Point]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", number(point.x), number(point.y)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `createPathFromPoints` (`shapes/util.ts:252-256`): `M x,y L x,y ... Z`.
#[must_use]
pub(super) fn create_path_from_points(points: &[Point]) -> String {
    let mut pieces: Vec<String> = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            format!(
                "{}{},{}",
                if index == 0 { "M" } else { "L" },
                number(point.x),
                number(point.y)
            )
        })
        .collect();
    pieces.push("Z".to_owned());
    pieces.join(" ")
}

/// `generateCirclePoints` (`shapes/util.ts:295-323`): `count` points over the arc from
/// `start_degrees` to `end_degrees`. Both coordinates are negated, as the reference does.
#[must_use]
pub(super) fn generate_circle_points(
    centre_x: f64,
    centre_y: f64,
    radius: f64,
    count: usize,
    start_degrees: f64,
    end_degrees: f64,
) -> Vec<Point> {
    let start = start_degrees * std::f64::consts::PI / 180.0;
    let end = end_degrees * std::f64::consts::PI / 180.0;
    let step = (end - start) / (count - 1) as f64;
    (0..count)
        .map(|index| {
            let angle = start + index as f64 * step;
            let x = centre_x + radius * angle.cos();
            let y = centre_y + radius * angle.sin();
            Point::new(-x, -y)
        })
        .collect()
}

/// `insertPolygonShape` (`shapes/insertPolygonShape.ts`): a `polygon.label-container` with the
/// points, translated to `(-w/2, h/2)`. The style is written when there is one. Returns the
/// element and its box in the node's coordinates.
#[must_use]
pub(super) fn polygon_shape(
    points: &[Point],
    width: f64,
    height: f64,
    style: Option<&str>,
) -> (String, BBox) {
    let translate_x = -width / 2.0;
    let translate_y = height / 2.0;
    let style = style.map_or_else(String::new, |style| attribute("style", style));
    let svg = format!(
        "<polygon points=\"{}\" class=\"label-container\" transform=\"translate({},{})\"{style}/>",
        points_attribute(points),
        number(translate_x),
        number(translate_y),
    );
    let bbox = BBox::from_points(points.iter().copied())
        .unwrap_or_default()
        .translate(translate_x, translate_y);
    (svg, bbox)
}

/// A style of a theme variable that is a number, or a text that starts with one, as `parseFloat`
/// reads it (`fontSize` is `14px`).
#[must_use]
pub(super) fn theme_number(theme: &Value, key: &str) -> Option<f64> {
    match theme.get(key)? {
        Value::Number(number) => Some(*number),
        Value::String(text) => leading_number(text),
        _ => None,
    }
}

/// A theme variable read as a flag: a `true`, a non-zero number or a non-empty text.
#[must_use]
pub(super) fn theme_flag(theme: &Value, key: &str) -> bool {
    match theme.get(key) {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => *number != 0.0 && !number.is_nan(),
        Some(Value::String(text)) => !text.is_empty(),
        _ => false,
    }
}

/// `parseFloat` of the leading number of a text, if there is one.
fn leading_number(text: &str) -> Option<f64> {
    let trimmed = text.trim_start();
    let mut end = 0;
    let bytes = trimmed.as_bytes();
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    let has_digit = trimmed[digits_start..end]
        .bytes()
        .any(|byte| byte.is_ascii_digit());
    if !has_digit {
        return None;
    }
    trimmed[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // shapes/util.spec.ts, "withMinWidth".

    #[test]
    fn leaves_a_box_that_already_meets_the_minimum_untouched() {
        let bbox = BBox::new(-20.0, 0.0, 40.0, 16.0);
        assert_eq!(with_min_width(bbox, 40.0), bbox);
    }

    #[test]
    fn keeps_svg_text_centred_and_does_not_invent_left_top() {
        // getBBox() on SVG text is centred on x = 0 and has no left or top.
        let bbox = BBox::new(-20.0, 1.0, 40.0, 16.0);
        assert_eq!(
            with_min_width(bbox, 120.0),
            BBox::new(-60.0, 1.0, 120.0, 16.0)
        );
    }

    #[test]
    fn a_text_with_a_font_size_reads_its_leading_number() {
        assert_eq!(leading_number("14px"), Some(14.0));
        assert_eq!(leading_number("  -2.5em"), Some(-2.5));
        assert_eq!(leading_number("px"), None);
    }

    #[test]
    fn the_text_style_takes_the_colour_as_fill() {
        // The first `stroke:` becomes `lineColor:` (createText.ts:371-374), which the later
        // removals do not match, so it stays, as in the reference.
        assert_eq!(
            node_text_style(
                "color:#333 !important;stroke:red !important;font-weight:bold !important"
            ),
            "fill:#333 !important;lineColor:red !important;font-weight:bold !important"
        );
    }

    #[test]
    fn a_later_declaration_of_a_key_wins_and_keeps_its_place() {
        let node = Node {
            css_compiled_styles: vec!["fill:#111".to_owned(), "stroke:#222".to_owned()],
            css_styles: vec!["fill:#333".to_owned()],
            ..Node::default()
        };
        let styles = styles2string(&node);
        assert_eq!(styles.node, "fill:#333 !important;stroke:#222 !important");
        assert_eq!(styles.label, "");
    }

    #[test]
    fn a_path_is_m_then_l_then_z() {
        let points = [Point::new(0.0, 1.0), Point::new(2.0, 3.0)];
        assert_eq!(create_path_from_points(&points), "M0,1 L2,3 Z");
    }
}
