//! Optional, self-contained SVG draw animation (decision 0021).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fmt::Write as _;
use std::str::FromStr;

use crate::RenderError;
use crate::svg::{escape, number};
use crate::unified::data::{Edge, Node};
use crate::unified::edges::{Painted, format_path};
use crate::unified::path::path_length;

/// The drawing order of a diagram's animation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AnimationOrder {
    /// One line at a time, in source order. A node appears when first reached.
    #[default]
    Source,
    /// Graph roots and branches draw in parallel; independent chart elements start together.
    /// A graph node appears on its first incoming arrival.
    Flow,
}

impl FromStr for AnimationOrder {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "source" => Ok(Self::Source),
            "flow" => Ok(Self::Flow),
            _ => Err(format!(
                "unknown animation order {value:?}, expected source or flow"
            )),
        }
    }
}

/// A diagram's one-time SVG draw animation, disabled unless set on the render options.
///
/// Speed multiplies an average of 700 SVG user units per second; it does not fix the duration of
/// the diagram. Each path, bar, or sector eases in and out independently.
/// Nodes fade in over 80 ms. The SVG contains CSS and no script, and shows the complete drawing
/// when the viewer requests reduced motion. Supported types are flowchart, Gantt, Git graph,
/// mind map, pie (including donut), and XY chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Animation {
    // Construction excludes NaN, infinities and zero, preserving Eq for render options.
    speed: f64,
    /// The drawing order. The default draws one line at a time in source order.
    pub order: AnimationOrder,
}

impl Eq for Animation {}

impl Default for Animation {
    fn default() -> Self {
        Self {
            speed: 1.0,
            order: AnimationOrder::Source,
        }
    }
}

impl Animation {
    /// A draw animation with a speed multiplier and the default source order.
    ///
    /// # Errors
    ///
    /// Returns an option error unless `speed` is finite and greater than zero.
    pub fn new(speed: f64) -> Result<Self, RenderError> {
        if !speed.is_finite() || speed <= 0.0 {
            return Err(RenderError::Options(
                "animation speed must be finite and greater than zero".to_owned(),
            ));
        }
        Ok(Self {
            speed,
            ..Self::default()
        })
    }

    /// The speed multiplier. The default is 1.
    #[must_use]
    pub fn speed(self) -> f64 {
        self.speed
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Timing {
    pub(crate) start: f64,
    pub(crate) duration: f64,
}

impl Timing {
    pub(crate) fn end(self) -> f64 {
        self.start + self.duration
    }
}

const NODE_MS: f64 = 80.0;
const GAP_MS: f64 = 15.0;

struct Plan {
    nodes: Vec<Timing>,
    edges: Vec<Timing>,
    duration: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Arrival {
    at: f64,
    node: usize,
}

impl Eq for Arrival {}

impl PartialOrd for Arrival {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Arrival {
    fn cmp(&self, other: &Self) -> Ordering {
        // A min-heap, with source order breaking equal arrival times.
        other
            .at
            .total_cmp(&self.at)
            .then_with(|| other.node.cmp(&self.node))
    }
}

impl Plan {
    fn new(
        nodes: &[Node],
        edges: &[Edge],
        lengths: &[f64],
        animation: Animation,
    ) -> Result<Self, RenderError> {
        let by_id: HashMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.id.as_str(), i))
            .collect();
        let duration: Vec<f64> = lengths
            .iter()
            .map(|length| length / 700.0 / animation.speed * 1000.0)
            .collect();
        if duration.iter().any(|duration| !duration.is_finite()) {
            return Err(RenderError::Options(
                "animation speed is too small for this diagram".to_owned(),
            ));
        }
        let mut node_times: Vec<Option<Timing>> = vec![None; nodes.len()];
        let mut edge_times = vec![Timing::default(); edges.len()];
        let mut end: f64 = 0.0;
        let reveal = |times: &mut [Option<Timing>], index: usize, at: f64| {
            *times[index].get_or_insert(Timing {
                start: at,
                duration: if nodes[index].is_group { 0.0 } else { NODE_MS },
            })
        };
        match animation.order {
            AnimationOrder::Source => {
                for (i, edge) in edges.iter().enumerate() {
                    if let Some(&start) = by_id.get(edge.start.as_str()) {
                        end = end.max(reveal(&mut node_times, start, end).end());
                    }
                    let timing = Timing {
                        start: end + GAP_MS,
                        duration: duration[i],
                    };
                    edge_times[i] = timing;
                    end = timing.end();
                    if let Some(&target) = by_id.get(edge.end.as_str()) {
                        end = end.max(reveal(&mut node_times, target, end).end());
                    }
                }
                for (index, timing) in node_times.iter_mut().enumerate() {
                    if timing.is_none() {
                        *timing = Some(Timing {
                            start: end,
                            duration: if nodes[index].is_group { 0.0 } else { NODE_MS },
                        });
                        end = timing.unwrap().end();
                    }
                }
            }
            AnimationOrder::Flow => {
                let mut incoming = vec![false; nodes.len()];
                let mut outgoing = vec![Vec::new(); nodes.len()];
                for (i, edge) in edges.iter().enumerate() {
                    if let Some(&start) = by_id.get(edge.start.as_str()) {
                        outgoing[start].push(i);
                    }
                    if let Some(&target) = by_id.get(edge.end.as_str()) {
                        incoming[target] = true;
                    }
                }
                let mut queue: BinaryHeap<Arrival> = incoming
                    .iter()
                    .enumerate()
                    .filter(|(_, incoming)| !**incoming)
                    .map(|(node, _)| Arrival { node, at: 0.0 })
                    .collect();
                loop {
                    let Some(arrival) = queue.pop() else {
                        let Some(node) = node_times.iter().position(Option::is_none) else {
                            break;
                        };
                        queue.push(Arrival {
                            node,
                            at: if end == 0.0 { 0.0 } else { end + GAP_MS },
                        });
                        continue;
                    };
                    if node_times[arrival.node].is_some() {
                        continue;
                    }
                    let node = reveal(&mut node_times, arrival.node, arrival.at);
                    end = end.max(node.end());
                    for &i in &outgoing[arrival.node] {
                        let timing = Timing {
                            start: node.end() + GAP_MS,
                            duration: duration[i],
                        };
                        edge_times[i] = timing;
                        end = end.max(timing.end());
                        if let Some(&target) = by_id.get(edges[i].end.as_str())
                            && node_times[target].is_none()
                        {
                            queue.push(Arrival {
                                node: target,
                                at: timing.end(),
                            });
                        }
                    }
                }
            }
        }
        if !end.is_finite() {
            return Err(RenderError::Options(
                "animation speed is too small for this diagram".to_owned(),
            ));
        }
        Ok(Self {
            nodes: node_times.into_iter().map(Option::unwrap).collect(),
            edges: edge_times,
            duration: end,
        })
    }
}

/// Per-render output; source-order vectors keep IDs and CSS independent of hash iteration order.
pub(crate) struct Animator {
    prefix: String,
    plan: Plan,
    definitions: String,
    keyframes: String,
    rules: String,
    serial: usize,
}

impl Animator {
    pub(crate) fn new(
        id: &str,
        nodes: &[Node],
        edges: &[Edge],
        paths: &[Painted],
        options: Animation,
    ) -> Result<Self, RenderError> {
        let lengths: Vec<f64> = paths
            .iter()
            .map(|path| path.d.as_deref().map_or(0.0, path_length))
            .collect();
        Self::from_lengths(id, nodes, edges, &lengths, options)
    }

    pub(crate) fn from_lengths(
        id: &str,
        nodes: &[Node],
        edges: &[Edge],
        lengths: &[f64],
        options: Animation,
    ) -> Result<Self, RenderError> {
        Ok(Self::with_plan(
            id,
            Plan::new(nodes, edges, lengths, options)?,
        ))
    }

    fn with_plan(id: &str, plan: Plan) -> Self {
        let prefix = format!("{id}-draw");
        let keyframes = format!(
            "@keyframes {prefix}-line{{from{{stroke-dashoffset:1}}to{{stroke-dashoffset:0}}}}@keyframes {prefix}-appear{{from{{opacity:0}}to{{opacity:1}}}}"
        );
        Self {
            prefix,
            plan,
            definitions: String::new(),
            keyframes,
            rules: String::new(),
            serial: 0,
        }
    }

    fn class(&mut self, name: &str, timing: Timing, easing: &str) -> String {
        let class = format!("{}-{}", self.prefix, self.serial);
        self.serial += 1;
        write!(
            self.rules,
            ".{class}{{animation:{name} {}ms {easing} {}ms backwards}}",
            number(timing.duration),
            number(timing.start)
        )
        .unwrap();
        class
    }

    fn fade(&mut self, svg: &str, timing: Timing, part: &str) -> String {
        let class = self.class(&format!("{}-appear", self.prefix), timing, "ease-out");
        format!("<g class=\"{class}\" data-draw-part=\"{part}\">{svg}</g>")
    }

    pub(crate) fn node(&mut self, index: usize, svg: &str) -> String {
        self.fade(svg, self.plan.nodes[index], "node")
    }

    pub(crate) fn label(&mut self, index: usize, svg: &str) -> String {
        let edge = self.plan.edges[index];
        self.fade(
            svg,
            Timing {
                start: edge.start + edge.duration * 0.65,
                duration: 80.0_f64.min(edge.duration * 0.35),
            },
            "label",
        )
    }

    pub(crate) fn path(&mut self, index: usize, path: &mut Painted) -> String {
        let Some(d) = path.d.clone() else {
            return format_path(path);
        };
        let timing = self.plan.edges[index];
        let end = Timing {
            start: timing.end() - timing.duration.min(1.0),
            duration: timing.duration.min(1.0),
        };
        let mut arrows = String::new();
        for (attribute, marker, at) in [
            ("marker-start", path.marker_start.take(), timing.start),
            ("marker-end", path.marker_end.take(), end.start),
        ] {
            if let Some(marker) = marker {
                let carrier = format!(
                    "<path d=\"{}\" class=\"{}\" style=\"{};stroke:none!important;fill:none!important;animation:none!important\" {attribute}=\"{}\"/>",
                    escape(&d),
                    escape(&path.class),
                    escape(&path.style),
                    escape(&marker)
                );
                // A continuous, brief fade avoids the floating-point boundary of step-end.
                arrows.push_str(&self.fade(
                    &carrier,
                    Timing {
                        start: at,
                        duration: end.duration,
                    },
                    "marker",
                ));
            }
        }
        format!(
            "{}{arrows}",
            self.masked(&format_path(path), &d, timing, 32.0, path_length(&d))
        )
    }

    pub(crate) fn edge(&mut self, index: usize, svg: &str, d: &str) -> String {
        self.masked(svg, d, self.plan.edges[index], 32.0, path_length(d))
    }

    fn masked(&mut self, svg: &str, d: &str, timing: Timing, width: f64, length: f64) -> String {
        let serial = self.serial;
        let mask_id = format!("{}-mask-{serial}", self.prefix);
        let mask_class = self.class(
            &format!("{}-line", self.prefix),
            timing,
            "cubic-bezier(0.4,0,0.2,1)",
        );
        let bbox = crate::unified::path::path_bounds(d).unwrap_or_default();
        let padding = width / 2.0 + 32.0;
        write!(self.definitions, "<mask id=\"{mask_id}\" maskUnits=\"userSpaceOnUse\" maskContentUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" mask-type=\"luminance\"><path d=\"{}\" pathLength=\"1\" fill=\"none\" stroke=\"white\" stroke-width=\"{}\" stroke-linecap=\"butt\" stroke-dasharray=\"1\" stroke-dashoffset=\"0\" class=\"{mask_class}\" data-draw-part=\"mask\" data-draw-length=\"{}\"/></mask>", number(bbox.x - padding), number(bbox.y - padding), number(bbox.width + padding * 2.0), number(bbox.height + padding * 2.0), escape(d), number(width), number(length)).unwrap();
        let release = format!("{}-release-{serial}", self.prefix);
        write!(
            self.keyframes,
            "@keyframes {release}{{from{{mask:url(#{mask_id})}}to{{mask:none}}}}"
        )
        .unwrap();
        let end = Timing {
            start: timing.end() - timing.duration.min(1.0),
            duration: timing.duration.min(1.0),
        };
        let class = self.class(&release, end, "linear");
        format!("<g class=\"{class}\" data-draw-part=\"edge\">{svg}</g>")
    }

    pub(crate) fn finish(self) -> String {
        format!(
            "<defs data-draw-defs=\"\">{}</defs><style data-draw-style=\"\" data-draw-duration=\"{}\">{}@media (prefers-reduced-motion:no-preference){{{}}}</style>",
            self.definitions,
            number(self.plan.duration),
            self.keyframes,
            self.rules
        )
    }
}

/// The same SVG emission primitives for chart-like diagrams with ordered marks.
pub(crate) struct Drawing {
    inner: Animator,
    options: Animation,
    cursor: f64,
}

impl Drawing {
    pub(crate) fn new(id: &str, options: Animation) -> Self {
        Self {
            inner: Animator::with_plan(
                id,
                Plan {
                    nodes: Vec::new(),
                    edges: Vec::new(),
                    duration: 0.0,
                },
            ),
            options,
            cursor: 0.0,
        }
    }

    pub(crate) fn next(&mut self, length: f64) -> Result<Timing, RenderError> {
        let timing = Timing {
            start: if self.options.order == AnimationOrder::Source {
                self.cursor
            } else {
                0.0
            },
            duration: length.max(0.0) / 700.0 / self.options.speed * 1000.0,
        };
        if !timing.end().is_finite() {
            return Err(RenderError::Options(
                "animation speed is too small for this diagram".to_owned(),
            ));
        }
        self.cursor = timing.end() + GAP_MS;
        self.inner.plan.duration = self.inner.plan.duration.max(timing.end());
        Ok(timing)
    }

    pub(crate) fn fade(&mut self, svg: &str, timing: Timing, part: &str) -> String {
        self.inner.plan.duration = self.inner.plan.duration.max(timing.end());
        self.inner.fade(svg, timing, part)
    }

    pub(crate) fn legend(&mut self, svg: &str, timing: Timing) -> String {
        self.fade(
            svg,
            Timing {
                start: timing.start,
                duration: NODE_MS,
            },
            "legend",
        )
    }

    pub(crate) fn masked(
        &mut self,
        svg: &str,
        d: &str,
        timing: Timing,
        width: f64,
        length: f64,
    ) -> String {
        self.inner.masked(svg, d, timing, width, length)
    }

    pub(crate) fn path(
        &mut self,
        svg: &str,
        d: &str,
        width: f64,
    ) -> Result<(String, Timing), RenderError> {
        let length = path_length(d);
        let timing = self.next(length)?;
        Ok((self.masked(svg, d, timing, width, length), timing))
    }

    pub(crate) fn bar(
        &mut self,
        svg: &str,
        box_: crate::unified::geom::BBox,
        horizontal: bool,
    ) -> Result<(String, Timing), RenderError> {
        let (x1, y1, x2, y2, width) = if horizontal {
            (
                box_.x,
                box_.y + box_.height / 2.0,
                box_.right(),
                box_.y + box_.height / 2.0,
                box_.height + 8.0,
            )
        } else {
            (
                box_.x + box_.width / 2.0,
                box_.bottom(),
                box_.x + box_.width / 2.0,
                box_.y,
                box_.width + 8.0,
            )
        };
        if ![x1, y1, x2, y2, width]
            .iter()
            .all(|value| value.is_finite())
            || box_.width < 0.0
            || box_.height < 0.0
        {
            return Ok((svg.to_owned(), Timing::default()));
        }
        let d = format!(
            "M{},{}L{},{}",
            number(x1),
            number(y1),
            number(x2),
            number(y2)
        );
        self.path(svg, &d, width)
    }

    pub(crate) fn finish(self) -> String {
        self.inner.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(ids: &[&str], links: &[(&str, &str)]) -> (Vec<Node>, Vec<Edge>) {
        let nodes = ids
            .iter()
            .map(|id| Node {
                id: (*id).to_owned(),
                ..Node::default()
            })
            .collect();
        let edges = links
            .iter()
            .map(|(start, end)| Edge {
                start: (*start).to_owned(),
                end: (*end).to_owned(),
                ..Edge::default()
            })
            .collect();
        (nodes, edges)
    }

    #[test]
    fn speed_is_validated_and_defaults_to_one_line_at_a_time() {
        assert_eq!(Animation::default().speed(), 1.0);
        assert_eq!(Animation::default().order, AnimationOrder::Source);
        for speed in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                Animation::new(speed),
                Err(RenderError::Options(_))
            ));
        }
        assert!("typo".parse::<AnimationOrder>().is_err());
    }

    #[test]
    fn ordered_chart_marks_share_velocity_and_parallel_mode_does_not_rescale_them() {
        let mut source = Drawing::new("chart", Animation::default());
        let first = source.next(700.0).unwrap();
        let second = source.next(1400.0).unwrap();
        assert_eq!(first.duration, 1000.0);
        assert_eq!(second.duration, 2000.0);
        assert!(second.start >= first.end());
        let mut options = Animation::new(2.0).unwrap();
        options.order = AnimationOrder::Flow;
        let mut parallel = Drawing::new("chart", options);
        let first = parallel.next(700.0).unwrap();
        let second = parallel.next(1400.0).unwrap();
        assert_eq!(first.start, 0.0);
        assert_eq!(second.start, 0.0);
        assert_eq!(first.duration, 500.0);
        assert_eq!(second.duration, 1000.0);
    }

    #[test]
    fn line_duration_depends_on_length_and_speed_not_graph_size() {
        let (nodes, edges) = graph(&["a", "b", "c"], &[("a", "b"), ("b", "c")]);
        let normal = Plan::new(&nodes, &edges, &[700.0, 1400.0], Animation::default()).unwrap();
        assert_eq!(normal.edges[0].duration, 1000.0);
        assert_eq!(normal.edges[1].duration, 2000.0);
        assert!(normal.edges[1].start >= normal.edges[0].end());
        let fast = Plan::new(
            &nodes,
            &edges,
            &[700.0, 1400.0],
            Animation::new(2.0).unwrap(),
        )
        .unwrap();
        assert_eq!(fast.edges[0].duration, 500.0);
        assert_eq!(fast.edges[1].duration, 1000.0);
        assert_eq!(fast.nodes[1].duration, NODE_MS);
        let small = Plan::new(&nodes[..2], &edges[..1], &[700.0], Animation::default()).unwrap();
        assert_eq!(small.edges[0].duration, normal.edges[0].duration);
    }

    #[test]
    fn flow_order_handles_parallel_branches_first_arrival_cycles_and_disconnected_nodes() {
        let (nodes, edges) = graph(
            &["a", "b", "c", "isolated"],
            &[("a", "b"), ("a", "c"), ("b", "c"), ("c", "a")],
        );
        let options = Animation {
            order: AnimationOrder::Flow,
            ..Animation::default()
        };
        let plan = Plan::new(&nodes, &edges, &[700.0, 700.0, 1400.0, 350.0], options).unwrap();
        assert_eq!(
            plan.nodes[0].start,
            NODE_MS + GAP_MS,
            "a rootless component starts at its first node after the preceding root finishes"
        );
        assert_eq!(plan.edges[0].start, plan.edges[1].start);
        assert_eq!(
            plan.nodes[2].start,
            plan.edges[1].end(),
            "the shorter incoming route reveals c"
        );
        assert!(plan.edges[3].start > plan.nodes[2].start);
        assert_eq!(
            plan.nodes[3].start, 0.0,
            "an isolated root can appear immediately"
        );
        assert!(plan.duration >= plan.edges[2].end());
    }

    #[test]
    fn group_frames_do_not_add_invisible_waits_and_extreme_speeds_fail_cleanly() {
        let (mut nodes, edges) = graph(&["a", "b", "frame"], &[("a", "b")]);
        nodes[2].is_group = true;
        let plan = Plan::new(&nodes, &edges, &[700.0], Animation::default()).unwrap();
        assert_eq!(plan.nodes[2].duration, 0.0);
        assert_eq!(plan.duration, plan.nodes[1].end());
        assert!(matches!(
            Plan::new(
                &nodes,
                &edges,
                &[700.0],
                Animation::new(f64::MIN_POSITIVE).unwrap()
            ),
            Err(RenderError::Options(_))
        ));
    }
}
