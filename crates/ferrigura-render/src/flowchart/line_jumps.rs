//! Line hops ("jumps") where edges cross: a port of `rendering-elements/lineJump.ts` (the crossing
//! detection and the rewrite of the path) and of `applyLineJumpsToSvg`, which the ELK layout runs
//! after painting through `elk/lineHops.ts`. See `docs-port/flowchart/render-notes/edges.md` §7.
//!
//! A hop rewrites the `d` of the edge that gives way, and, for a neo edge, the dash array of its
//! style, which the reference recomputes from the new length.

use super::geom::Point;
use super::path::{
    ROUNDED_RADIUS, RoundedCorner, apply_marker_offsets, d3_number, path_length, rounded_corner,
};

/// The radius of a hop, `JUMP_RADIUS` in `elk/lineHops.ts`.
pub const JUMP_RADIUS: f64 = 6.0;

/// `CORNER_JUMP_CLEARANCE`: the straight run a hop keeps from the bend next to it.
const CORNER_JUMP_CLEARANCE: f64 = 2.0;

/// `MIN_USEFUL_RADIUS_RATIO`: a hop shrunk below this share of its radius is dropped.
const MIN_USEFUL_RADIUS_RATIO: f64 = 0.6;

/// `ENDPOINT_EPSILON`: crossings this close to an end of a segment are not crossings.
const ENDPOINT_EPSILON: f64 = 1e-6;

/// The hop style of `elk.lineHops`: `arc` unless it is `gap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpStyle {
    /// A semicircle over the crossed edge.
    Arc,
    /// A break in the crossed edge.
    Gap,
}

/// The configuration of the hops: the radius and the style, as `applyLineJumpsToSvg` takes them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineJumpConfig {
    /// The radius of a hop before it is clamped to its room.
    pub jump_radius: f64,
    /// The style.
    pub jump_style: JumpStyle,
}

/// What the hops need of an edge: its geometry, its curve and its arrows, and its current path
/// `d` and style, which a hop may rewrite.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeGeom {
    /// The edge id.
    pub id: String,
    /// The points of the edge, as the layout left them (before the marker offsets).
    pub points: Vec<Point>,
    /// The curve after layout: `rounded` or `linear`. `None` when the curve is not a string.
    pub curve: Option<String>,
    /// The arrow at the start.
    pub arrow_type_start: String,
    /// The arrow at the end.
    pub arrow_type_end: String,
    /// The `style` the path currently has.
    pub style: String,
}

/// A place where two edges cross, with the edge that gives way (`jump_edge`) and the one it hops
/// over. Indices are positions in the slice the crossings were found in.
#[derive(Debug, Clone, PartialEq)]
pub struct Crossing {
    /// The edge that is rewritten.
    pub jump_edge: usize,
    /// The edge that is crossed.
    pub other_edge: usize,
    /// The segment of the jumping edge that holds the crossing.
    pub seg_index: usize,
    /// Where on that segment the crossing is, from 0 to 1.
    pub t: f64,
    /// The crossing point.
    pub point: Point,
}

/// The result of a hop for one edge: its new `d`, and its new style when the dash array changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Rewrite {
    /// The index of the edge in the slice the hops were applied to.
    pub index: usize,
    /// The new `d`.
    pub d: String,
    /// The new style, when a neo dash array had to be recomputed.
    pub style: Option<String>,
}

/// The segments of a polyline, as `buildSegmentList` makes them.
fn segments(points: &[Point]) -> Vec<(Point, Point)> {
    points.windows(2).map(|pair| (pair[0], pair[1])).collect()
}

/// `segmentIntersection`: the crossing of two segments, or `None` when they are parallel, do not
/// meet, or meet at an end of either.
fn segment_intersection(a1: Point, a2: Point, b1: Point, b2: Point) -> Option<(Point, f64, f64)> {
    let dx_a = a2.x - a1.x;
    let dy_a = a2.y - a1.y;
    let dx_b = b2.x - b1.x;
    let dy_b = b2.y - b1.y;
    let denominator = dx_a * dy_b - dy_a * dx_b;
    if denominator == 0.0 {
        return None;
    }
    let dx = b1.x - a1.x;
    let dy = b1.y - a1.y;
    let t_a = (dx * dy_b - dy * dx_b) / denominator;
    let t_b = (dx * dy_a - dy * dx_a) / denominator;
    if t_a <= ENDPOINT_EPSILON
        || t_a >= 1.0 - ENDPOINT_EPSILON
        || t_b <= ENDPOINT_EPSILON
        || t_b >= 1.0 - ENDPOINT_EPSILON
    {
        return None;
    }
    Some((Point::new(a1.x + t_a * dx_a, a1.y + t_a * dy_a), t_a, t_b))
}

/// `isHorizontalSeg`: the segment is horizontal-dominant (ties go to horizontal).
fn is_horizontal((a, b): (Point, Point)) -> bool {
    (b.x - a.x).abs() >= (b.y - a.y).abs()
}

/// `crossingSitsInRoundedCorner`: whether a crossing lies in the stretch where a rounded edge has
/// left its polyline to round a bend.
fn crossing_sits_in_rounded_corner(edge: &EdgeGeom, seg_index: usize, t: f64) -> bool {
    if edge.curve.as_deref() != Some("rounded") {
        return false;
    }
    let points = &edge.points;
    let (Some(&a), Some(&b)) = (points.get(seg_index), points.get(seg_index + 1)) else {
        return false;
    };
    let seg_len = a.distance(b);
    let d = t * seg_len;
    let entering = if seg_index > 0 {
        rounded_corner(points[seg_index - 1], a, b, ROUNDED_RADIUS)
    } else {
        None
    };
    if let Some(corner) = entering
        && d < corner.cut_len
    {
        return true;
    }
    let leaving = if seg_index + 2 < points.len() {
        rounded_corner(a, b, points[seg_index + 2], ROUNDED_RADIUS)
    } else {
        None
    };
    leaving.is_some_and(|corner| seg_len - d < corner.cut_len)
}

/// `findEdgeIntersections`: every crossing of every pair of edges that is not in a rounded corner.
/// For an orthogonal pair the horizontal segment jumps; otherwise the later edge does.
#[must_use]
pub fn find_edge_intersections(edges: &[EdgeGeom]) -> Vec<Crossing> {
    let mut crossings = Vec::new();
    for i in 0..edges.len() {
        let segments_a = segments(&edges[i].points);
        for j in i + 1..edges.len() {
            let segments_b = segments(&edges[j].points);
            for (si, &seg_a) in segments_a.iter().enumerate() {
                for (sj, &seg_b) in segments_b.iter().enumerate() {
                    let Some((point, t_a, t_b)) =
                        segment_intersection(seg_a.0, seg_a.1, seg_b.0, seg_b.1)
                    else {
                        continue;
                    };
                    if crossing_sits_in_rounded_corner(&edges[i], si, t_a)
                        || crossing_sits_in_rounded_corner(&edges[j], sj, t_b)
                    {
                        continue;
                    }
                    let a_horizontal = is_horizontal(seg_a);
                    let b_horizontal = is_horizontal(seg_b);
                    let orthogonal_pair = a_horizontal != b_horizontal;
                    let jump_on_a = orthogonal_pair && a_horizontal;
                    crossings.push(if jump_on_a {
                        Crossing {
                            jump_edge: i,
                            other_edge: j,
                            seg_index: si,
                            t: t_a,
                            point,
                        }
                    } else {
                        Crossing {
                            jump_edge: j,
                            other_edge: i,
                            seg_index: sj,
                            t: t_b,
                            point,
                        }
                    });
                }
            }
        }
    }
    crossings
}

/// `fmt` and `pointToString` of `lineJump.ts`: three digits, comma between the coordinates.
fn point_text(point: Point) -> String {
    format!("{},{}", d3_number(point.x), d3_number(point.y))
}

/// `getArcSweepFlag`: the sweep that bumps a hop to the conventional side.
fn arc_sweep((a, b): (Point, Point)) -> u8 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dx.abs() >= dy.abs() {
        u8::from(dx >= 0.0)
    } else {
        u8::from(dy >= 0.0)
    }
}

/// A crossing on one segment, with the radius it can keep.
#[derive(Debug, Clone, Copy)]
struct JumpOnSegment {
    t: f64,
    point: Point,
    /// The distance from the start of the segment.
    d: f64,
    /// The radius, clamped to the room the segment has.
    r: f64,
}

/// `emitJump`: the path commands that hop over the crossing.
fn emit_jump(
    jump: JumpOnSegment,
    (ux, uy): (f64, f64),
    sweep: u8,
    style: JumpStyle,
) -> Vec<String> {
    let pre = Point::new(jump.point.x - ux * jump.r, jump.point.y - uy * jump.r);
    let post = Point::new(jump.point.x + ux * jump.r, jump.point.y + uy * jump.r);
    let mut out = vec![format!("L{}", point_text(pre))];
    match style {
        JumpStyle::Arc => out.push(format!(
            "A{},{} 0 0 {sweep} {}",
            d3_number(jump.r),
            d3_number(jump.r),
            point_text(post)
        )),
        JumpStyle::Gap => out.push(format!("M{}", point_text(post))),
    }
    out
}

/// The points of an edge as the rewrite reads them: the marker offsets applied to the ends.
fn offset_points(edge: &EdgeGeom) -> Vec<Point> {
    apply_marker_offsets(&edge.points, &edge.arrow_type_start, &edge.arrow_type_end)
}

/// `rewriteEdgePath`: the `d` of an edge with its hops. Rounded corners are kept (quadratics), and
/// the parts are joined with spaces, unlike `generateRoundedPath`.
#[must_use]
pub fn rewrite_edge_path(edge: &EdgeGeom, jumps: &[Crossing], config: &LineJumpConfig) -> String {
    if edge.points.len() < 2 {
        return String::new();
    }
    let points = offset_points(edge);
    let rounded = edge.curve.as_deref() == Some("rounded");
    let segs = segments(&points);

    let mut by_segment: Vec<Vec<JumpOnSegment>> = vec![Vec::new(); segs.len()];
    for jump in jumps {
        let Some(&(a, b)) = segs.get(jump.seg_index) else {
            continue;
        };
        let seg_len = a.distance(b);
        by_segment[jump.seg_index].push(JumpOnSegment {
            t: jump.t,
            point: jump.point,
            d: jump.t * seg_len,
            r: config.jump_radius,
        });
    }

    let min_useful_radius = config.jump_radius * MIN_USEFUL_RADIUS_RATIO;
    let mut parts = vec![format!("M{}", point_text(points[0]))];
    for (i, &(a, b)) in segs.iter().enumerate() {
        let seg_len = a.distance(b);
        let (ux, uy) = if seg_len == 0.0 {
            (0.0, 0.0)
        } else {
            ((b.x - a.x) / seg_len, (b.y - a.y) / seg_len)
        };
        let sweep = arc_sweep((a, b));

        let mut consumed = 0.0;
        if rounded
            && i > 0
            && let Some(corner) =
                rounded_corner(points[i - 1], points[i], points[i + 1], ROUNDED_RADIUS)
        {
            consumed = corner.cut_len;
        }

        let mut stop = seg_len;
        let mut upcoming: Option<RoundedCorner> = None;
        if rounded && i + 1 < segs.len() {
            upcoming = rounded_corner(points[i], points[i + 1], points[i + 2], ROUNDED_RADIUS);
            if let Some(corner) = upcoming {
                stop = seg_len - corner.cut_len;
            }
        }

        let mut seg_jumps: Vec<JumpOnSegment> = by_segment[i].clone();
        seg_jumps.sort_by(|x, y| x.t.total_cmp(&y.t));
        seg_jumps.retain_mut(|jump| {
            let room = (jump.d - consumed).min(stop - jump.d) - CORNER_JUMP_CLEARANCE;
            jump.r = jump.r.min(room);
            jump.r >= min_useful_radius
        });
        for k in 0..seg_jumps.len().saturating_sub(1) {
            let gap = seg_jumps[k + 1].d - seg_jumps[k].d;
            if seg_jumps[k].r + seg_jumps[k + 1].r > gap {
                let half = gap / 2.0;
                seg_jumps[k].r = seg_jumps[k].r.min(half);
                seg_jumps[k + 1].r = seg_jumps[k + 1].r.min(half);
            }
        }

        for jump in &seg_jumps {
            if jump.r < min_useful_radius {
                continue;
            }
            parts.extend(emit_jump(*jump, (ux, uy), sweep, config.jump_style));
        }

        match upcoming {
            Some(corner) if rounded => {
                parts.push(format!(
                    "L{},{}",
                    d3_number(corner.start.x),
                    d3_number(corner.start.y)
                ));
                parts.push(format!(
                    "Q{},{} {},{}",
                    d3_number(points[i + 1].x),
                    d3_number(points[i + 1].y),
                    d3_number(corner.end.x),
                    d3_number(corner.end.y)
                ));
            }
            _ => parts.push(format!("L{}", point_text(b))),
        }
    }
    parts.join(" ")
}

/// `curveSupportsLineHops`: `rounded`, `linear` and the steps take hops; the smoothing curves do
/// not. An empty curve takes them, as the reference's `!curve` test says.
#[must_use]
pub fn curve_supports_line_hops(curve: &str) -> bool {
    curve.is_empty()
        || matches!(
            curve,
            "linear" | "rounded" | "step" | "stepBefore" | "stepAfter"
        )
}

/// The points of an edge as the rendered `data-points` carries them: the points that are finite,
/// when two or more remain (JSON writes a non-finite coordinate as `null`, which is dropped).
fn rendered_points(points: &[Point]) -> Vec<Point> {
    let finite: Vec<Point> = points
        .iter()
        .copied()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .collect();
    if finite.len() >= 2 {
        finite
    } else {
        points.to_vec()
    }
}

/// The dash array of a neo edge, `stroke-dasharray: 0 S L E`, read as the reference's regular
/// expression reads it: the first match of `stroke-dasharray\s*:\s*0\s+([\d.]+)\s+[\d.]+\s+([\d.]+)`.
/// Returns the captured `S` and `E`.
fn dash_ends(style: &str) -> Option<(f64, f64)> {
    let chars: Vec<char> = style.chars().collect();
    let literal: Vec<char> = "stroke-dasharray".chars().collect();
    for start in 0..chars.len() {
        if chars.len() - start < literal.len() || chars[start..start + literal.len()] != literal[..]
        {
            continue;
        }
        let mut i = start + literal.len();
        i = skip_spaces(&chars, i);
        if chars.get(i) != Some(&':') {
            continue;
        }
        i = skip_spaces(&chars, i + 1);
        if chars.get(i) != Some(&'0') {
            continue;
        }
        i += 1;
        let Some(after_zero) = spaces_end(&chars, i) else {
            continue;
        };
        let (s_token, after_s) = token(&chars, after_zero);
        let Some(after_s_space) = spaces_end(&chars, after_s) else {
            continue;
        };
        let (_, after_l) = token(&chars, after_s_space);
        if after_l == after_s_space {
            continue;
        }
        let Some(after_l_space) = spaces_end(&chars, after_l) else {
            continue;
        };
        let (e_token, _) = token(&chars, after_l_space);
        if e_token.is_empty() || s_token.is_empty() {
            continue;
        }
        return Some((parse_float(&s_token), parse_float(&e_token)));
    }
    None
}

fn skip_spaces(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

/// The index after one or more whitespace characters, or `None` when there are none.
fn spaces_end(chars: &[char], i: usize) -> Option<usize> {
    let end = skip_spaces(chars, i);
    (end > i).then_some(end)
}

/// The maximal run of digits and dots from `i`, and the index after it.
fn token(chars: &[char], i: usize) -> (String, usize) {
    let mut end = i;
    while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.') {
        end += 1;
    }
    (chars[i..end].iter().collect(), end)
}

/// `parseFloat` of a run of digits and dots: the longest prefix that is a number, else NaN.
fn parse_float(text: &str) -> f64 {
    (1..=text.len())
        .rev()
        .find_map(|end| text[..end].parse::<f64>().ok())
        .unwrap_or(f64::NAN)
}

/// Rewrites the dash array of a neo style for a new path length, as `applyLineJumpsToSvg` does:
/// the `S` and `E` hidden ends are kept, the stroke between them takes the rest. The style is
/// returned with every `stroke-dasharray` replaced and `;;` runs collapsed. `None` when the style
/// has no dash array of that shape.
#[must_use]
pub fn rewrite_dash_array(style: &str, length: f64) -> Option<String> {
    let (s, e) = dash_ends(style)?;
    let on_length = (length - s - e).max(0.0);
    let dash = format!(
        "0 {} {} {}",
        super::path::js_number(s),
        super::path::js_number(on_length),
        super::path::js_number(e)
    );
    let replaced = replace_dash_declarations(style, &dash);
    Some(collapse_semicolons(&replaced))
}

/// `/stroke-dasharray\s*:[^;]*;?/g` replaced by `stroke-dasharray: <dash>;`.
fn replace_dash_declarations(style: &str, dash: &str) -> String {
    let chars: Vec<char> = style.chars().collect();
    let literal: Vec<char> = "stroke-dasharray".chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars.len() - i >= literal.len() && chars[i..i + literal.len()] == literal[..] {
            let mut j = skip_spaces(&chars, i + literal.len());
            if chars.get(j) == Some(&':') {
                j += 1;
                while j < chars.len() && chars[j] != ';' {
                    j += 1;
                }
                if j < chars.len() {
                    j += 1;
                }
                out.push_str(&format!("stroke-dasharray: {dash};"));
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `/;\s*;+/g` replaced by `;`.
fn collapse_semicolons(style: &str) -> String {
    let chars: Vec<char> = style.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ';' {
            let mut j = skip_spaces(&chars, i + 1);
            if chars.get(j) == Some(&';') {
                while j < chars.len() && chars[j] == ';' {
                    j += 1;
                }
                out.push(';');
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `applyLineJumpsToSvg` for a diagram: the crossings of all edges, and the rewritten `d` (and
/// style) of every edge that has a hop it can take. The edges that the reference leaves alone are
/// not in the result.
#[must_use]
pub fn apply_jumps(edges: &[EdgeGeom], config: &LineJumpConfig) -> Vec<Rewrite> {
    let rendered: Vec<EdgeGeom> = edges
        .iter()
        .map(|edge| EdgeGeom {
            points: rendered_points(&edge.points),
            ..edge.clone()
        })
        .collect();
    let crossings = find_edge_intersections(&rendered);
    if crossings.is_empty() {
        return Vec::new();
    }
    let mut rewrites = Vec::new();
    for (index, edge) in rendered.iter().enumerate() {
        let jumps: Vec<Crossing> = crossings
            .iter()
            .filter(|crossing| crossing.jump_edge == index)
            .cloned()
            .collect();
        if jumps.is_empty() {
            continue;
        }
        // The curve of an edge the layout did not route is a function in the reference, which
        // `curveSupportsLineHops` refuses; `None` here stands for that.
        match edge.curve.as_deref() {
            Some(curve) if curve_supports_line_hops(curve) => {}
            _ => continue,
        }
        let d = rewrite_edge_path(edge, &jumps, config);
        let style = rewrite_dash_array(&edge.style, path_length(&d));
        rewrites.push(Rewrite { index, d, style });
    }
    rewrites
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn edge(id: &str, points: Vec<Point>, curve: Option<&str>) -> EdgeGeom {
        EdgeGeom {
            id: id.to_owned(),
            points,
            curve: curve.map(str::to_owned),
            arrow_type_start: "none".to_owned(),
            arrow_type_end: "none".to_owned(),
            style: String::new(),
        }
    }

    fn config(style: JumpStyle) -> LineJumpConfig {
        LineJumpConfig {
            jump_radius: JUMP_RADIUS,
            jump_style: style,
        }
    }

    #[test]
    fn a_horizontal_edge_hops_over_a_vertical_one() {
        let mut a = edge("a", vec![p(0.0, 50.0), p(100.0, 50.0)], Some("linear"));
        a.arrow_type_end = "arrow_point".to_owned();
        let b = edge("b", vec![p(50.0, 0.0), p(50.0, 100.0)], Some("linear"));
        let crossings = find_edge_intersections(&[a.clone(), b.clone()]);
        assert_eq!(crossings.len(), 1);
        assert_eq!(crossings[0].jump_edge, 0);
        assert_eq!(crossings[0].other_edge, 1);
        assert_eq!(crossings[0].seg_index, 0);
        assert!((crossings[0].t - 0.5).abs() < 1e-12);
        assert_eq!(crossings[0].point, p(50.0, 50.0));
        let jumps = crossings;
        assert_eq!(
            rewrite_edge_path(&a, &jumps, &config(JumpStyle::Arc)),
            "M0,50 L44,50 A6,6 0 0 1 56,50 L96,50"
        );
        assert_eq!(
            rewrite_edge_path(&a, &jumps, &config(JumpStyle::Gap)),
            "M0,50 L44,50 M56,50 L96,50"
        );
    }

    #[test]
    fn a_rounded_edge_keeps_its_corner_after_the_hop() {
        let a = edge(
            "a",
            vec![p(0.0, 50.0), p(100.0, 50.0), p(100.0, 90.0)],
            Some("rounded"),
        );
        let jumps = vec![Crossing {
            jump_edge: 0,
            other_edge: 1,
            seg_index: 0,
            t: 0.5,
            point: p(50.0, 50.0),
        }];
        let mut a = a;
        a.arrow_type_end = "arrow_point".to_owned();
        assert_eq!(
            rewrite_edge_path(&a, &jumps, &config(JumpStyle::Arc)),
            "M0,50 L44,50 A6,6 0 0 1 56,50 L92.929,50 Q100,50 100,57.071 L100,86"
        );
    }

    #[test]
    fn a_crossing_too_close_to_the_start_is_dropped() {
        let a = edge("a", vec![p(0.0, 50.0), p(100.0, 50.0)], Some("linear"));
        let jumps = vec![Crossing {
            jump_edge: 0,
            other_edge: 1,
            seg_index: 0,
            t: 0.04,
            point: p(4.0, 50.0),
        }];
        assert_eq!(
            rewrite_edge_path(&a, &jumps, &config(JumpStyle::Arc)),
            "M0,50 L100,50"
        );
    }

    #[test]
    fn a_smoothing_curve_takes_no_hop() {
        assert!(curve_supports_line_hops("linear"));
        assert!(curve_supports_line_hops("rounded"));
        assert!(curve_supports_line_hops(""));
        assert!(!curve_supports_line_hops("basis"));
    }

    #[test]
    fn the_neo_dash_array_keeps_its_ends() {
        let style = "stroke-dasharray: 0 0 96 4; stroke-dashoffset: 0;;stroke:#f00";
        assert_eq!(dash_ends(style), Some((0.0, 4.0)));
        let rewritten = rewrite_dash_array(style, 100.0).expect("rewritten");
        assert_eq!(
            rewritten,
            "stroke-dasharray: 0 0 96 4; stroke-dashoffset: 0;stroke:#f00"
        );
    }

    #[test]
    fn the_dotted_dash_array_reads_the_first_pattern_value_as_the_end() {
        // The reference's regular expression captures the first `2` of the pattern as `E`, so a
        // dotted neo edge with a hop loses its dots. The port keeps that behaviour.
        let style = "stroke-dasharray: 0 0 2 2 2 2 4; stroke-dashoffset: 0;";
        assert_eq!(dash_ends(style), Some((0.0, 2.0)));
    }

    #[test]
    fn a_style_without_a_dash_array_is_left_alone() {
        assert_eq!(rewrite_dash_array("stroke:#f00;;;stroke:#f00", 10.0), None);
    }

    #[test]
    fn apply_jumps_rewrites_only_the_jumping_edge() {
        let a = edge("a", vec![p(0.0, 50.0), p(100.0, 50.0)], Some("linear"));
        let b = edge("b", vec![p(50.0, 0.0), p(50.0, 100.0)], Some("linear"));
        let rewrites = apply_jumps(&[a, b], &config(JumpStyle::Arc));
        assert_eq!(rewrites.len(), 1);
        assert_eq!(rewrites[0].index, 0);
        assert_eq!(rewrites[0].d, "M0,50 L44,50 A6,6 0 0 1 56,50 L100,50");
    }
}
