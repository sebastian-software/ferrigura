//! Edges: a port of `insertEdge`, `insertEdgeLabel` and the label placement in
//! `packages/mermaid/src/rendering-util/rendering-elements/edges.js`, with `edgeMarker.ts`,
//! `utils/lineWithOffset.ts` and the line hops of `lineJump.ts` and `elk/lineHops.ts`. See
//! `docs-port/flowchart/render-notes/edges.md`.
//!
//! The path `d` of a routed edge is `rounded`: `generateRoundedPath` over the points with the
//! marker offsets. The other curves come from d3's line with the marker accessors, as the
//! reference draws a straight fallback edge. Node clipping is off for ELK (`skipIntersect`).

use base64::Engine as _;
use ferrigura_model::value::Value;

use super::Section;
use super::data::{Edge, LayoutData};
use super::geom::{BBox, Point};
use super::label::{self, Anchor, LabelLayout, LabelSpec};
use super::line_jumps::{self, EdgeGeom, JUMP_RADIUS, JumpStyle, LineJumpConfig};
use super::markers::{self, Kind};
use super::path::{
    Curve, ROUNDED_RADIUS, apply_marker_offsets, curve_path, fix_corners, generate_rounded_path,
    js_number, line_with_offset, marker_offset2, path_bounds, path_length,
};
use crate::RenderError;
use crate::svg::escape;

/// A painted edge path.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgePath {
    /// The edge id.
    pub id: String,
    /// The `<path>` element.
    pub svg: String,
    /// The box of the path's geometry, as `getBBox` gives it (no stroke, no markers).
    pub bbox: BBox,
    /// The copies of the markers that this edge's stroke colour needs, which the SVG carries next
    /// to the markers of the diagram. Only the first edge of a colour carries a copy.
    pub markers: String,
}

/// A painted edge label.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeLabel {
    /// The `<g class="edgeLabel">` element, positioned.
    pub svg: String,
    /// Its box in absolute coordinates.
    pub bbox: BBox,
}

/// `ARROW_MAP` of `elk/render.ts`: the start and end arrows of an edge type, for an edge whose
/// render data does not set them.
fn arrow_map(edge_type: &str) -> (&'static str, &'static str) {
    match edge_type {
        "arrow_cross" => ("none", "arrow_cross"),
        "double_arrow_cross" => ("arrow_cross", "arrow_cross"),
        "arrow_point" => ("none", "arrow_point"),
        "double_arrow_point" => ("arrow_point", "arrow_point"),
        "arrow_circle" => ("none", "arrow_circle"),
        "double_arrow_circle" => ("arrow_circle", "arrow_circle"),
        _ => ("none", "none"),
    }
}

/// `lineBreakRegex` of `diagrams/common/common.ts` (`/<\/?br\s*\/?>/gi`), replaced by a newline.
fn replace_line_breaks(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if let Some(end) = line_break_at(&chars, i) {
            out.push('\n');
            i = end;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// The end of a `<br>`, `</br>`, `<br/>` or `<br />` tag at `start`, matched without regard to case.
fn line_break_at(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    if chars.get(i) != Some(&'<') {
        return None;
    }
    i += 1;
    if chars.get(i) == Some(&'/') {
        i += 1;
    }
    let tag = ['b', 'r'];
    for expected in tag {
        if !chars
            .get(i)
            .is_some_and(|c| c.eq_ignore_ascii_case(&expected))
        {
            return None;
        }
        i += 1;
    }
    while chars.get(i).is_some_and(|c| c.is_whitespace()) {
        i += 1;
    }
    if chars.get(i) == Some(&'/') {
        i += 1;
    }
    (chars.get(i) == Some(&'>')).then_some(i + 1)
}

/// Sets what the reference's `applyElkEdgeRenderData` sets on every edge before layout: the arrow
/// types, the stroke, the arrowhead, and the label with `<br>` as a newline.
///
/// The curve is a d3 factory in the reference (`interpolateToCurve`), which the paint does not
/// read as a name. It is `None` here, and the paint then takes `flowchart.curve` (as
/// `resolveEdgeCurveType` does for a function); the layout sets `rounded` or `linear` for every
/// edge it routes.
pub fn prepare(data: &mut LayoutData) {
    for edge in &mut data.edges {
        if edge.arrowhead.is_none() {
            let arrowhead = if edge.edge_type == "arrow_open" {
                "none"
            } else {
                "normal"
            };
            edge.arrowhead = Some(arrowhead.to_owned());
        }
        let (start, end) = arrow_map(&edge.edge_type);
        if edge.arrow_type_start.is_empty() {
            edge.arrow_type_start = start.to_owned();
        }
        if edge.arrow_type_end.is_empty() {
            edge.arrow_type_end = end.to_owned();
        }
        // `computeStroke` with no stroke: the thickness and the pattern of a plain link.
        if edge.thickness.is_empty() {
            edge.thickness = "normal".to_owned();
        }
        if edge.pattern.is_empty() {
            edge.pattern = "solid".to_owned();
        }
        if edge.arrowhead_style.is_empty() {
            edge.arrowhead_style = "fill: #333".to_owned();
        }
        edge.curve = None;
        edge.label = replace_line_breaks(&edge.label);
    }
}

/// `isLabelStyle` of `shapes/handDrawnShapeStyles.ts`: the declarations that style the label.
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

/// `styles2String(edge).labelStyles`: the label declarations of the edge, from its compiled class
/// styles and its label style, each as `key:value !important`, joined with `;`. Later values of a
/// key replace earlier ones and keep their place (`styles2Map`).
fn label_style_text(edge: &Edge) -> String {
    let mut map: Vec<(String, Option<String>)> = Vec::new();
    for style in edge
        .css_compiled_styles
        .iter()
        .chain(edge.label_style.iter())
    {
        let mut parts = style.split(':');
        let key = parts.next().unwrap_or("").trim().to_owned();
        let value = parts.next().map(|value| value.trim().to_owned());
        match map.iter_mut().find(|(existing, _)| *existing == key) {
            Some(entry) => entry.1 = value,
            None => map.push((key, value)),
        }
    }
    map.iter()
        .filter(|(key, _)| is_label_style(key))
        .map(|(key, value)| format!("{key}:{} !important", value.as_deref().unwrap_or("")))
        .collect::<Vec<_>>()
        .join(";")
}

/// The styles `createText` gives the background rectangle and the text of an edge label
/// (`createText.ts` 391-409): the label style without its `stroke`, `stroke-width` and `fill`
/// declarations, with `background:` read as `fill:` for the rectangle and `color:` read as `fill:`
/// for the text.
fn edge_label_styles(style: &str) -> (String, String) {
    let stripped = ["stroke:", "stroke-width:", "fill:"]
        .iter()
        .fold(style.to_owned(), |style, name| {
            without_declarations(&style, name)
        });
    (
        stripped.replace("background:", "fill:"),
        stripped.replace("color:", "fill:"),
    )
}

/// `style.replace(/<name>[^;]+;?/g, '')`: every `name` followed by a value, with the semicolon
/// after it, removed. A `name` without a value stays.
fn without_declarations(style: &str, name: &str) -> String {
    let mut out = String::with_capacity(style.len());
    let mut rest = style;
    while let Some(at) = rest.find(name) {
        let after = &rest[at + name.len()..];
        let value = after.find(';').unwrap_or(after.len());
        if value == 0 {
            // `[^;]+` needs a character: the name stays, and the search goes on after it.
            out.push_str(&rest[..at + name.len()]);
            rest = after;
        } else {
            out.push_str(&rest[..at]);
            rest = after[value..].strip_prefix(';').unwrap_or(&after[value..]);
        }
    }
    out.push_str(rest);
    out
}

/// The `data-points` of an edge's path: `btoa(JSON.stringify(points))` (`edges.js` 705), the
/// points as `[{"x":…,"y":…},…]` in base64, with `null` for a number that JSON cannot hold.
#[must_use]
pub fn data_points(points: &[Point]) -> String {
    let json_number = |value: f64| {
        if value.is_finite() {
            js_number(value)
        } else {
            "null".to_owned()
        }
    };
    let items: Vec<String> = points
        .iter()
        .map(|point| {
            format!(
                "{{\"x\":{},\"y\":{}}}",
                json_number(point.x),
                json_number(point.y)
            )
        })
        .collect();
    base64::engine::general_purpose::STANDARD.encode(format!("[{}]", items.join(",")))
}

/// The declarations of a label style string, as the label layout reads them.
fn declarations(style: &str) -> Vec<String> {
    style
        .split(';')
        .map(str::trim)
        .filter(|declaration| !declaration.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Lays out an edge's label as `insertEdgeLabel` does when it measures it: wrapped at 200 pixels,
/// with a background, in `font_size`, the theme's `fontSize` that the root rule of the reference's
/// SVG sets and every label inherits. The caller stores the size on the edge before layout (see
/// [`label_size`]). `None` for an edge without a label.
///
/// # Errors
///
/// Returns an error if a bundled font cannot be read.
pub fn label(edge: &Edge, font_size: f64) -> Result<Option<LabelLayout>, RenderError> {
    if edge.label.is_empty() {
        return Ok(None);
    }
    let styles = declarations(&label_style_text(edge));
    let spec = LabelSpec {
        text: &edge.label,
        label_type: edge.label_type,
        width: 200.0,
        is_node: false,
        add_background: true,
        anchor: Anchor::Middle,
        wrap_in_style: false,
        styles: &styles,
        font_size,
        title: false,
    };
    let measured = label::layout(&spec)?;
    // A diagram whose stylesheet sets the edge labels' font size (the use case's `.edgeLabel`) gets the
    // background measured at the theme's size, as the reference does before its class is applied, and the
    // text drawn at the stylesheet's size, wrapped at the theme's size. The group's size is the background
    // and the text's box centres the group (`positionRenderedEdgeLabel`).
    let Some(drawn_size) = edge.font_size else {
        return Ok(Some(measured));
    };
    let mut drawn_styles = styles.clone();
    drawn_styles.push(format!("font-size: {}px", crate::svg::number(drawn_size)));
    let drawn = label::layout(&LabelSpec {
        styles: &drawn_styles,
        ..spec
    })?;
    // The text's own box is the drawn one (it centres the group), and the background is the measured
    // one, which holds the group's size.
    Ok(Some(LabelLayout {
        background: measured.background,
        ..drawn
    }))
}

/// The size `insertEdgeLabel` records for a label: the label group's box, which holds the text and
/// its background.
#[must_use]
pub fn label_size(label: &LabelLayout) -> (f64, f64) {
    let group = match label.background {
        Some(background) => label.bbox.union(background),
        None => label.bbox,
    };
    // `edge.width` and `edge.height` are the `getBBox` of the label's group (edges.js 122), so
    // they are float32 values.
    let group = group.browser();
    (group.width, group.height)
}

/// The `d` of an edge, as `insertEdge` builds it for the points after layout: `rounded` through
/// `generateRoundedPath`, otherwise the d3 curve with the marker accessors on the corner-fixed
/// points. `None` when the curve has no points.
fn path_data(edge: &Edge, curve_name: &str) -> Option<String> {
    let line_data: Vec<Point> = edge
        .points
        .iter()
        .copied()
        .filter(|p| !p.y.is_nan())
        .collect();
    let (start, end) = (&edge.arrow_type_start, &edge.arrow_type_end);
    if curve_name == "rounded" {
        let offset = apply_marker_offsets(&line_data, start, end);
        return Some(generate_rounded_path(&offset, ROUNDED_RADIUS));
    }
    let fixed = fix_corners(&line_data);
    let accessor = line_with_offset(&fixed, start, end);
    curve_path(
        Curve::from_name(curve_name).unwrap_or(Curve::Basis),
        &accessor,
    )
}

/// `strokeClasses` of `insertEdge`: the thickness and the pattern classes of the edge.
fn stroke_classes(thickness: &str, pattern: &str) -> String {
    let thickness = match thickness {
        "thick" => "edge-thickness-thick",
        "invisible" => "edge-thickness-invisible",
        _ => "edge-thickness-normal",
    };
    let pattern = match pattern {
        "dotted" => "edge-pattern-dotted",
        "dashed" => "edge-pattern-dashed",
        _ => "edge-pattern-solid",
    };
    format!("{thickness} {pattern}")
}

/// `generateDashArray` of `edges.js`: the neo dash array of a dotted or dashed edge.
fn generate_dash_array(length: f64, start: f64, end: f64) -> String {
    let middle = length - start - end;
    let raw = (middle / 4.0).floor();
    let pairs = if raw.is_finite() {
        raw.max(0.0) as usize
    } else {
        0
    };
    let middle_pattern = vec!["2 2"; pairs].join(" ");
    format!("0 {} {middle_pattern} {}", js_number(start), js_number(end))
}

/// The first `stroke:` declaration of a style, as `pathStyle.match(/stroke:([^;]+)/)`.
fn stroke_colour(style: &str) -> Option<&str> {
    let mut from = 0;
    while let Some(position) = style[from..].find("stroke:") {
        let start = from + position + "stroke:".len();
        let rest = &style[start..];
        let end = rest.find(';').unwrap_or(rest.len());
        if end > 0 {
            return Some(&rest[..end]);
        }
        from = start;
    }
    None
}

/// The line hops of the ELK layout: `elk.lineHops` (`elk/lineHops.ts`).
pub fn jump_style(section: &Section<'_>) -> Option<JumpStyle> {
    jump_style_of(section.elk.line_hops.as_ref())
}

/// The line hops that a `lineHops` option gives: `None` when it is `false`; otherwise the style, `gap`
/// for `'gap'` and `arc` for anything else. The same reading serves `elk.lineHops` and
/// `swimlane.lineHops` (`applySwimlaneLineJumps`, `adjustLayout.ts` 9-14).
#[must_use]
pub fn jump_style_of(line_hops: Option<&Value>) -> Option<JumpStyle> {
    match line_hops {
        Some(Value::Bool(false)) => None,
        Some(value) if value.as_str() == Some("gap") => Some(JumpStyle::Gap),
        _ => Some(JumpStyle::Arc),
    }
}

/// The tolerance of the swimlane clipping, `TOLERANCE` in `insertEdge` (`edges.js` 675).
const SWIMLANE_TOLERANCE: f64 = 0.5;

/// The points of a swimlane edge once its ends are clipped to the outlines of its nodes: the swimlane
/// branch of `insertEdge` (`edges.js` 654-680). A straight edge has both of its ends clipped. A longer
/// edge keeps its inner bends, and only its first and last segments are clipped: an end that lands on
/// its inner point within the tolerance is dropped, so that no segment is left with no length.
///
/// `tail` and `head` are the outlines of the start and end nodes (`node.intersect`). `points` has at
/// least two points.
#[must_use]
pub fn swimlane_points(
    points: &[Point],
    tail: impl Fn(Point) -> Point,
    head: impl Fn(Point) -> Point,
) -> Vec<Point> {
    if let [first, last] = points {
        return vec![tail(*first), head(*last)];
    }
    let inner = &points[1..points.len() - 1];
    let (first_inner, last_inner) = (inner[0], inner[inner.len() - 1]);
    let end = points[points.len() - 1];

    let last_is_pinned = near(end, last_inner);
    let new_first = tail(first_inner);
    let new_last = if last_is_pinned {
        last_inner
    } else {
        head(last_inner)
    };
    let first_is_duplicate = near(new_first, first_inner);
    let last_is_duplicate = near(new_last, last_inner);

    let mut clipped = Vec::with_capacity(points.len());
    if !first_is_duplicate {
        clipped.push(new_first);
    }
    clipped.extend_from_slice(inner);
    if !last_is_duplicate {
        clipped.push(new_last);
    }
    clipped
}

/// Whether two points are within the swimlane tolerance on both axes (`insertEdge`'s `Math.abs(...) < TOLERANCE`).
fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < SWIMLANE_TOLERANCE && (a.y - b.y).abs() < SWIMLANE_TOLERANCE
}

/// What the paint of an edge holds before the line hops, and what the hops may change.
struct Painted {
    id: String,
    /// The path's `id`: `<diagramId>-<edge id>` (`edges.js` 837 and 860), unless the diagram names
    /// the path itself.
    path_id: String,
    /// The `data-points` of the path: its points before the marker offsets, as [`data_points`].
    points: String,
    /// The attributes after the markers, each with its leading space.
    path_attributes: String,
    d: Option<String>,
    style: String,
    class: String,
    look: String,
    marker_start: Option<String>,
    marker_end: Option<String>,
    markers: String,
}

/// Paints the edges of a laid-out diagram: one path per edge, then the line hops over all of
/// them, as `paintLayoutData` and `applyElkLineJumps` do. `diagram_type` names the markers.
#[must_use]
pub fn paint_paths(
    edges: &[Edge],
    svg_id: &str,
    diagram_type: &str,
    section: &Section<'_>,
) -> Vec<EdgePath> {
    paint_paths_with(edges, svg_id, diagram_type, section, jump_style(section))
}

/// [`paint_paths`], with the line hops given: the swimlane layout's come from `swimlane.lineHops`, not
/// from `elk.lineHops`. `section` still gives the default curve.
#[must_use]
pub fn paint_paths_with(
    edges: &[Edge],
    svg_id: &str,
    diagram_type: &str,
    section: &Section<'_>,
    jumps: Option<JumpStyle>,
) -> Vec<EdgePath> {
    let default_curve = section.flowchart.curve.clone();
    let mut created = std::collections::HashSet::new();
    let mut painted: Vec<Painted> = edges
        .iter()
        .map(|edge| paint_edge(edge, svg_id, diagram_type, &default_curve, &mut created))
        .collect();

    if let Some(style) = jumps {
        let indices: Vec<usize> = edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.points.len() >= 2)
            .map(|(index, _)| index)
            .collect();
        let geometries: Vec<EdgeGeom> = indices
            .iter()
            .map(|&index| EdgeGeom {
                id: edges[index].id.clone(),
                points: edges[index].points.clone(),
                curve: edges[index].curve.clone(),
                arrow_type_start: edges[index].arrow_type_start.clone(),
                arrow_type_end: edges[index].arrow_type_end.clone(),
                style: painted[index].style.clone(),
            })
            .collect();
        let config = LineJumpConfig {
            jump_radius: JUMP_RADIUS,
            jump_style: style,
        };
        for rewrite in line_jumps::apply_jumps(&geometries, &config) {
            let target = &mut painted[indices[rewrite.index]];
            target.d = Some(rewrite.d);
            if let Some(style) = rewrite.style {
                target.style = style;
            }
        }
    }

    painted
        .into_iter()
        .map(|painted| {
            let bbox = painted
                .d
                .as_deref()
                .and_then(path_bounds)
                .unwrap_or_default();
            let svg = format_path(&painted);
            EdgePath {
                id: painted.id,
                svg,
                bbox,
                markers: painted.markers,
            }
        })
        .collect()
}

/// `insertEdge`'s path, before the line hops: the `d`, the style with the neo dash array, the
/// classes and the markers.
fn paint_edge(
    edge: &Edge,
    svg_id: &str,
    diagram_type: &str,
    default_curve: &str,
    created: &mut std::collections::HashSet<String>,
) -> Painted {
    let curve_name = edge.curve.as_deref().unwrap_or(default_curve);
    let d = path_data(edge, curve_name);

    let stroke = stroke_classes(&edge.thickness, &edge.pattern);
    let animation = if edge.animate == Some(true) {
        "edge-animation-fast".to_owned()
    } else {
        edge.animation
            .as_deref()
            .filter(|animation| !animation.is_empty())
            .map(|animation| format!("edge-animation-{animation}"))
            .unwrap_or_default()
    };
    let class_styles = edge.css_compiled_styles.join(";");
    let animated = edge.animate == Some(true)
        || edge
            .animation
            .as_deref()
            .is_some_and(|animation| !animation.is_empty())
        || class_styles.contains("animation");

    let styles: String = edge.style.iter().map(|style| format!("{style};")).collect();
    let path_style = format!(
        "{}{}",
        if class_styles.is_empty() {
            styles.clone()
        } else {
            format!("{class_styles};{styles};")
        },
        std::iter::once(";".to_owned())
            .chain(edge.style.iter().map(|style| format!(";{style}")))
            .collect::<String>()
    );

    let mut class = format!(" {stroke}");
    if !edge.classes.is_empty() {
        class.push(' ');
        class.push_str(&edge.classes);
    }
    if !animation.is_empty() {
        class.push(' ');
        class.push_str(&animation);
    }

    let mut style = path_style.clone();
    if edge.look == "neo" && !animated {
        let length = d.as_deref().map_or(0.0, path_length);
        let start_offset = marker_offset2(&edge.arrow_type_start).unwrap_or(0.0);
        let end_offset = marker_offset2(&edge.arrow_type_end).unwrap_or(0.0);
        let dash = if matches!(edge.pattern.as_str(), "dotted" | "dashed") {
            generate_dash_array(length, start_offset, end_offset)
        } else {
            format!(
                "0 {} {} {}",
                js_number(start_offset),
                js_number(length - start_offset - end_offset),
                js_number(end_offset)
            )
        };
        style = format!("stroke-dasharray: {dash}; stroke-dashoffset: 0;{path_style}");
    }

    let margin = !animated && edge.look == "neo";
    let colour = stroke_colour(&path_style).filter(|colour| !colour.trim().is_empty());
    let mut markers_out = String::new();
    let mut marker = |arrow: &str, start: bool, markers_out: &mut String| -> Option<String> {
        // A class relation's marker is named by its kind, and a class relation has no colour
        // (`edgeMarker.ts` names the marker after the diagram type, and the relation's style is
        // not a stroke colour of the class syntax).
        // The use case's extension marker is its own, named by its diagram (`usecaseRenderer`'s
        // marker set), not the class diagram's.
        if diagram_type == markers::USECASE && arrow == "extension" {
            return Some(format!(
                "url(#{})",
                markers::usecase_extension_id(svg_id, start, margin)
            ));
        }
        if let Some(class) = markers::ClassKind::from_arrow(arrow) {
            return Some(format!(
                "url(#{})",
                markers::class_marker_id(svg_id, class, start, margin)
            ));
        }
        let kind = Kind::from_arrow(arrow)?;
        let id = markers::marker_id(svg_id, diagram_type, kind, start, margin);
        match colour {
            Some(colour) => {
                let coloured = markers::coloured_id(&id, colour);
                if created.insert(coloured.clone()) {
                    markers_out.push_str(&markers::coloured_marker(
                        svg_id,
                        diagram_type,
                        kind,
                        start,
                        margin,
                        colour,
                    ));
                }
                Some(format!("url(#{coloured})"))
            }
            None => Some(format!("url(#{id})")),
        }
    };
    let marker_start = marker(&edge.arrow_type_start, true, &mut markers_out);
    let marker_end = marker(&edge.arrow_type_end, false, &mut markers_out);

    Painted {
        id: edge.id.clone(),
        path_id: edge
            .path_id
            .clone()
            .unwrap_or_else(|| format!("{svg_id}-{}", edge.id)),
        points: data_points(&edge.points),
        path_attributes: edge.path_attributes.clone(),
        d,
        style,
        class,
        look: edge.look.clone(),
        marker_start,
        marker_end,
        markers: markers_out,
    }
}

/// The `<path>` element of a painted edge.
fn format_path(painted: &Painted) -> String {
    let mut attributes = String::new();
    if let Some(d) = &painted.d {
        attributes.push_str(&format!(" d=\"{}\"", escape(d)));
    }
    attributes.push_str(&format!(" id=\"{}\"", escape(&painted.path_id)));
    attributes.push_str(&format!(" class=\"{}\"", escape(&painted.class)));
    attributes.push_str(&format!(" style=\"{}\"", escape(&painted.style)));
    attributes.push_str(&format!(
        " data-edge=\"true\" data-et=\"edge\" data-id=\"{}\" data-points=\"{}\"",
        escape(&painted.id),
        painted.points
    ));
    if !painted.look.is_empty() {
        attributes.push_str(&format!(" data-look=\"{}\"", escape(&painted.look)));
    }
    if let Some(marker) = &painted.marker_start {
        attributes.push_str(&format!(" marker-start=\"{}\"", escape(marker)));
    }
    if let Some(marker) = &painted.marker_end {
        attributes.push_str(&format!(" marker-end=\"{}\"", escape(marker)));
    }
    attributes.push_str(&painted.path_attributes);
    format!("<path{attributes}/>")
}

/// Paints an edge's label at its laid-out position, as `positionRenderedEdgeLabel` does: the
/// outer group at the edge's centre (plus half the subgraph title margin), the inner group centred
/// on its text.
#[must_use]
pub fn paint_label(edge: &Edge, label: &LabelLayout, section: &Section<'_>) -> EdgeLabel {
    let margin = section
        .flowchart
        .sub_graph_title_margin
        .as_ref()
        .map_or(0.0, |margin| {
            (margin.top.unwrap_or(0) + margin.bottom.unwrap_or(0)) as f64
        });
    let x = edge.x.unwrap_or(0.0);
    let y = edge.y.unwrap_or(0.0) + margin / 2.0;
    let text = label.bbox;
    let (text_x, text_y) = (text.x + text.width / 2.0, text.y + text.height / 2.0);
    let inner = match label.background {
        // `createText` restyles the background and the text of a label that has a background,
        // which an edge label with text has; without one it leaves both as they are.
        Some(_) => {
            let (background, text) = edge_label_styles(&label_style_text(edge));
            label.svg_with_background(&text, &background)
        }
        None => label.svg(&label_style_text(edge)),
    };
    let group = match label.background {
        Some(background) => text.union(background),
        None => text,
    };
    let absolute = group.translate(x - text_x, y - text_y);
    let svg = format!(
        "<g class=\"edgeLabel\" transform=\"translate({}, {})\"><g class=\"label\" data-id=\"{}\" transform=\"translate({}, {})\">{inner}</g></g>",
        js_number(x),
        js_number(y),
        escape(&edge.id),
        js_number(-text_x),
        js_number(-text_y),
    );
    EdgeLabel {
        svg,
        bbox: absolute,
    }
}

/// The marker definitions the flowchart uses, as `insertMarkers` writes them for `flowchart-v2`.
#[must_use]
pub fn markers(svg_id: &str) -> String {
    markers::markers(svg_id, markers::FLOWCHART)
}

#[cfg(test)]
mod tests {
    use ferrigura_model::config::MermaidConfig;

    use super::*;

    #[test]
    fn a_line_break_tag_becomes_a_newline() {
        assert_eq!(replace_line_breaks("a<br>b"), "a\nb");
        assert_eq!(replace_line_breaks("a<BR/>b<br />c</br>d"), "a\nb\nc\nd");
        assert_eq!(replace_line_breaks("a<b>c"), "a<b>c");
    }

    #[test]
    fn the_label_style_keeps_only_label_declarations() {
        let edge = Edge {
            css_compiled_styles: vec!["color:red".to_owned(), "fill:#fff".to_owned()],
            label_style: vec!["font-size:20px".to_owned(), "color:blue".to_owned()],
            ..Edge::default()
        };
        assert_eq!(
            label_style_text(&edge),
            "color:blue !important;font-size:20px !important"
        );
    }

    /// The pairs of `createText`'s replacements as Node 24 gives them for these styles.
    #[test]
    fn an_edge_label_restyles_its_background_and_text_as_create_text_does() {
        let cases = [
            (
                "color:Sienna !important",
                "color:Sienna !important",
                "fill:Sienna !important",
            ),
            ("stroke: #333; stroke-width: 1.5px;fill:none;", " ", " "),
            (
                "background:yellow !important;color:red !important;fill:blue !important",
                "fill:yellow !important;color:red !important;",
                "background:yellow !important;fill:red !important;",
            ),
            (
                "stroke:;fill:;color:x",
                "stroke:;fill:;color:x",
                "stroke:;fill:;fill:x",
            ),
            (
                "font-size:20px !important;stroke-width:4px !important;background-color:pink !important",
                "font-size:20px !important;background-color:pink !important",
                "font-size:20px !important;background-fill:pink !important",
            ),
        ];
        for (style, background, text) in cases {
            assert_eq!(
                edge_label_styles(style),
                (background.to_owned(), text.to_owned()),
                "{style}"
            );
        }
    }

    /// `btoa(JSON.stringify(points))` in Node 24 for the points of a reference edge, and with a
    /// coordinate that JSON writes as `null`.
    #[test]
    fn the_data_points_are_the_points_as_json_in_base64() {
        assert_eq!(
            data_points(&[Point::new(502.3125, 694.5), Point::new(648.5625, 694.5)]),
            "W3sieCI6NTAyLjMxMjUsInkiOjY5NC41fSx7IngiOjY0OC41NjI1LCJ5Ijo2OTQuNX1d"
        );
        assert_eq!(
            data_points(&[Point::new(f64::NAN, 1e-7)]),
            "W3sieCI6bnVsbCwieSI6MWUtN31d"
        );
    }

    #[test]
    fn the_label_declarations_skip_empty_entries() {
        assert_eq!(
            declarations("color:red !important;;"),
            vec!["color:red !important"]
        );
        assert!(declarations("").is_empty());
    }

    #[test]
    fn the_label_size_is_the_group_box() {
        let text = BBox::new(1.0, 1.0, 20.0, 17.0);
        let layout = LabelLayout {
            lines: Vec::new(),
            bbox: text,
            background: Some(BBox::new(-1.0, -1.0, 24.0, 21.0)),
            font_size: 14.0,
            title: false,
            anchor: Anchor::Middle,
            first_line_class: None,
        };
        assert_eq!(label_size(&layout), (24.0, 21.0));
        assert_eq!(label_size(&LabelLayout::of_box(text)), (20.0, 17.0));
    }

    #[test]
    fn a_stroke_colour_is_the_first_match() {
        assert_eq!(
            stroke_colour("fill:none;stroke:#f00;;;stroke:#0f0"),
            Some("#f00")
        );
        assert_eq!(stroke_colour("stroke:;stroke:red"), Some("red"));
        assert_eq!(stroke_colour("stroke-width:4px"), None);
    }

    #[test]
    fn the_dash_array_of_a_dotted_edge_has_pairs_between_its_ends() {
        let pairs = vec!["2 2"; 24].join(" ");
        assert_eq!(
            generate_dash_array(100.0, 0.0, 4.0),
            format!("0 0 {pairs} 4")
        );
        assert_eq!(generate_dash_array(3.0, 0.0, 4.0), "0 0  4");
    }

    #[test]
    fn the_stroke_classes_follow_the_thickness_and_pattern() {
        assert_eq!(
            stroke_classes("normal", "normal"),
            "edge-thickness-normal edge-pattern-solid"
        );
        assert_eq!(
            stroke_classes("dotted", "dotted"),
            "edge-thickness-normal edge-pattern-dotted"
        );
        assert_eq!(
            stroke_classes("invisible", "invisible"),
            "edge-thickness-invisible edge-pattern-solid"
        );
    }

    #[test]
    fn the_jump_style_follows_the_configuration() {
        let mut config = MermaidConfig::default();
        assert_eq!(jump_style(&Section::of(&config)), Some(JumpStyle::Arc));
        config.elk.line_hops = Some(Value::from(serde_json::json!("gap")));
        assert_eq!(jump_style(&Section::of(&config)), Some(JumpStyle::Gap));
        config.elk.line_hops = Some(Value::from(serde_json::json!(false)));
        assert_eq!(jump_style(&Section::of(&config)), None);
    }

    #[test]
    fn the_markers_of_the_diagram_are_twelve() {
        let svg = markers("svg");
        assert_eq!(svg.matches("<marker ").count(), 12);
    }
}
