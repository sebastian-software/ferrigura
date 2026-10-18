//! `resolveRenderedOrthogonalCrossings` (`materializedGeometry.ts:1835-2814`): a crossing of two rendered routes
//! is removed by moving one route to another candidate path, or by moving a pair of routes together, when the
//! count of crossings goes down, or stays and the route gets shorter with fewer bends. The candidate paths are
//! in [`super::candidates`].

use crate::js::js_sort_by;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

use super::candidates::candidate_paths_for;
use super::common::*;
use crate::model::{LayoutData, Point};

const MAX_ITERATIONS: usize = 4;
const MAX_PAIR_CANDIDATES_PER_EDGE: usize = 48;
/// `ANCHOR` of the pass, the offset of the outer tracks from the outermost node.
pub(super) const ANCHOR: f64 = 20.0;

/// `resolveRenderedOrthogonalCrossings(edges, nodeByIdMap)`.
pub fn resolve_rendered_orthogonal_crossings(data: &mut LayoutData) {
    edit_points(data, |view, points| {
        for _ in 0..MAX_ITERATIONS {
            let step = match Ctx::new(view, points) {
                Some(ctx) => ctx.step(),
                None => return,
            };
            match step {
                Step::Stop => return,
                Step::Single(edge, path) => points.set(edge, path),
                Step::Paired(replacements) => {
                    for (edge, path) in replacements {
                        points.set(edge, path);
                    }
                }
            }
        }
    });
}

/// What an iteration does: stop the pass, move one route, or move a pair of routes.
pub(super) enum Step {
    Stop,
    Single(usize, Vec<Point>),
    Paired(Replacements),
}

/// The outer tracks of the diagram: the lines `ANCHOR` outside the outermost real node on each side.
#[derive(Clone, Copy)]
pub(super) struct Outside {
    top: f64,
    bottom: f64,
    left: f64,
    right: f64,
}

impl Outside {
    pub(super) fn of(&self, side: RectSide) -> f64 {
        match side {
            RectSide::Top => self.top,
            RectSide::Bottom => self.bottom,
            RectSide::Left => self.left,
            RectSide::Right => self.right,
        }
    }
}

/// The context of one iteration: the layout, the real nodes with their centres, and the outer tracks.
pub(super) struct Ctx<'v> {
    pub(super) view: &'v View<'v>,
    pub(super) points: &'v Points,
    /// `nodeInfoById`: the real nodes by id.
    pub(super) node_info: IndexMap<String, NodeBoundsInfo>,
    /// `realNodeRects`: the rectangles of the real nodes, in order.
    pub(super) real_rects: Vec<RectEntry>,
    pub(super) outside: Outside,
    /// The visible edges, in order.
    pub(super) visible: Vec<usize>,
}

/// A crossing of two visible edges, with its number of crossing segment pairs.
#[derive(Clone, Copy)]
struct Pair {
    first: usize,
    second: usize,
    count: f64,
}

/// The crossings of a set of paths: the total, the pairs that cross, the edges that cross, in order.
pub(super) struct Snapshot {
    count: f64,
    pairs: Vec<Pair>,
    edge_set: HashSet<usize>,
    edges: Vec<usize>,
}

/// The segments, bends and lengths of the visible edges in the current layout, by edge.
struct Base {
    segments: Vec<Vec<OrthogonalSegment>>,
    bends: Vec<f64>,
    lengths: Vec<f64>,
}

/// A candidate path for one edge of a pair search.
#[derive(Clone)]
pub(super) struct PairCandidate {
    path: Vec<Point>,
    segments: Vec<OrthogonalSegment>,
    shared_track_conflicts: Vec<usize>,
    total_bends: f64,
    length: f64,
}

/// The candidates of one edge in a pair search.
struct PairOption {
    edge: usize,
    candidates: Vec<PairCandidate>,
}

/// The score of a pair replacement: its crossings, bends and length.
struct PairScore {
    replacements: Replacements,
    crossings: f64,
    bends: f64,
    length: f64,
}

/// `pathLength(points)` of this pass: the sum of the horizontal and vertical distances between the points.
fn manhattan_length(points: &[Point]) -> f64 {
    let mut sum = 0.0;
    for index in 0..points.len().saturating_sub(1) {
        let point = points[index + 1];
        let previous = points[index];
        sum = sum + (point.x - previous.x).abs() + (point.y - previous.y).abs();
    }
    sum
}

impl<'v> Ctx<'v> {
    /// The context of an iteration, or `None` when the layout has no real node (the pass then returns).
    pub(super) fn new(view: &'v View<'v>, points: &'v Points) -> Option<Ctx<'v>> {
        let mut real_nodes: Vec<NodeBoundsInfo> = Vec::new();
        for index in view.node_indices() {
            let node = &view.nodes[index];
            if is_group(node) || is_edge_label(node) {
                continue;
            }
            let cx = or_zero(node.x);
            let cy = or_zero(node.y);
            let Some(rect) = rect_of_node_bounds(node) else {
                continue;
            };
            real_nodes.push(NodeBoundsInfo {
                id: node.id.clone(),
                cx,
                cy,
                rect,
            });
        }
        if real_nodes.is_empty() {
            return None;
        }

        let mut node_info = IndexMap::new();
        for node in &real_nodes {
            node_info.insert(node.id.clone(), node.clone());
        }
        let real_rects = real_nodes
            .iter()
            .map(|node| RectEntry {
                id: node.id.clone(),
                rect: node.rect,
            })
            .collect();
        let outside = Outside {
            top: js_min_of(real_nodes.iter().map(|node| node.rect.top)) - ANCHOR,
            bottom: js_max_of(real_nodes.iter().map(|node| node.rect.bottom)) + ANCHOR,
            left: js_min_of(real_nodes.iter().map(|node| node.rect.left)) - ANCHOR,
            right: js_max_of(real_nodes.iter().map(|node| node.rect.right)) + ANCHOR,
        };

        Some(Ctx {
            view,
            points,
            node_info,
            real_rects,
            outside,
            visible: view.visible(),
        })
    }

    /// `replacementPointsFor(edge, replacements)`.
    pub(super) fn pts(&self, replacements: &Replacements, edge: usize) -> Vec<Point> {
        points_of(self.points, replacements, edge)
    }

    /// `outwardTracksForSide(side)`: the three tracks outside the side, nearest first.
    pub(super) fn outward_tracks(&self, side: RectSide) -> Vec<f64> {
        let outward = if matches!(side, RectSide::Left | RectSide::Top) {
            -1.0
        } else {
            1.0
        };
        (0..=2)
            .map(|channel| self.outside.of(side) + outward * ANCHOR * channel as f64)
            .collect()
    }

    /// `nodeInfoById.get(id)` for a truthy id.
    pub(super) fn info(&self, id: Option<&str>) -> Option<&NodeBoundsInfo> {
        truthy_text(id).and_then(|id| self.node_info.get(id))
    }

    /// `snapshot`: the crossings of the visible edges, with the replacements.
    pub(super) fn snapshot(&self, replacements: &Replacements) -> Snapshot {
        let mut count = 0.0;
        let mut pairs = Vec::new();
        let mut edge_set = HashSet::new();
        let mut edge_order = Vec::new();
        let mut add_edge = |edge: usize| {
            if edge_set.insert(edge) {
                edge_order.push(edge);
            }
        };

        for i in 0..self.visible.len() {
            let first = self.visible[i];
            let first_points = self.pts(replacements, first);
            for j in (i + 1)..self.visible.len() {
                let second = self.visible[j];
                let pair_count = crossings_between(
                    &segments_for(&first_points),
                    &segments_for(&self.pts(replacements, second)),
                );
                if pair_count > 0.0 {
                    count += pair_count;
                    pairs.push(Pair {
                        first,
                        second,
                        count: pair_count,
                    });
                    add_edge(first);
                    add_edge(second);
                }
            }
        }

        edge_order.sort_unstable();
        Snapshot {
            count,
            pairs,
            edge_set,
            edges: edge_order,
        }
    }

    /// `crossingCountWithSingleReplacement(current, edge, path)`: the crossings after `edge` takes the
    /// segments `path_segments`, with the segments of the other edges taken from `base`. The pairs of the snapshot
    /// that `edge` does not touch are kept, as `crossingCountWithReplacements` does.
    fn count_with_single(
        &self,
        current: &Snapshot,
        edge: usize,
        path_segments: &[OrthogonalSegment],
        base: &Base,
    ) -> f64 {
        let mut current_affected = 0.0;
        for pair in &current.pairs {
            if pair.first == edge || pair.second == edge {
                current_affected += pair.count;
            }
        }

        let mut replacement_affected = 0.0;
        for i in 0..self.visible.len() {
            let first = self.visible[i];
            let first_segments = if first == edge {
                path_segments
            } else {
                &base.segments[first]
            };
            for j in (i + 1)..self.visible.len() {
                let second = self.visible[j];
                if first != edge && second != edge {
                    continue;
                }
                let second_segments = if second == edge {
                    path_segments
                } else {
                    &base.segments[second]
                };
                replacement_affected += crossings_between(first_segments, second_segments);
            }
        }

        current.count - current_affected + replacement_affected
    }

    /// `crossingComponents(snapshot)`: the groups of two or more crossing edges.
    fn crossing_components(&self, snapshot: &Snapshot) -> Vec<Vec<usize>> {
        let mut neighbors: HashMap<usize, Vec<usize>> = HashMap::new();
        for pair in &snapshot.pairs {
            neighbors.entry(pair.first).or_default().push(pair.second);
            neighbors.entry(pair.second).or_default().push(pair.first);
        }

        let mut components = Vec::new();
        let mut seen: HashSet<usize> = HashSet::new();
        for &edge in &snapshot.edges {
            if seen.contains(&edge) {
                continue;
            }
            let mut queue = vec![edge];
            let mut component = Vec::new();
            seen.insert(edge);
            while let Some(current) = queue.pop() {
                component.push(current);
                for &next in neighbors.get(&current).map_or(&[][..], Vec::as_slice) {
                    if seen.insert(next) {
                        queue.push(next);
                    }
                }
            }
            component.sort_unstable();
            if component.len() > 1 {
                components.push(component);
            }
        }
        components
    }

    /// `pairSearchGroups(snapshot)`: each crossing component with the edges that share an endpoint with it.
    fn pair_search_groups(&self, snapshot: &Snapshot) -> Vec<Vec<usize>> {
        let mut groups = Vec::new();
        for component in self.crossing_components(snapshot) {
            let component_set: HashSet<usize> = component.iter().copied().collect();
            let component_endpoints: HashSet<&str> = component
                .iter()
                .flat_map(|&edge| endpoint_ids(&self.view.edges[edge]))
                .collect();
            let mut group = component.clone();
            for &edge in &self.visible {
                if component_set.contains(&edge) {
                    continue;
                }
                if endpoint_ids(&self.view.edges[edge])
                    .iter()
                    .any(|id| component_endpoints.contains(id))
                {
                    group.push(edge);
                }
            }
            group.sort_unstable();
            groups.push(group);
        }
        groups
    }

    /// `pathHitsNode(edge, path)`: the path runs through a real node that is not an endpoint, with a 2 unit
    /// margin.
    pub(super) fn path_hits_node(&self, edge: usize, path: &[Point]) -> bool {
        let endpoints = endpoint_ids(&self.view.edges[edge]);
        segments_for(path).iter().any(|segment| {
            segment_hits_any_rect(segment.a, segment.b, &self.real_rects, &endpoints, -2.0)
        })
    }

    /// `pathHasSegmentConflict(edge, path)`: the path shares a track with a visible edge other than `edge`, with
    /// the segments of the other edges taken from `base`.
    fn segments_conflict_with(
        &self,
        edge: usize,
        path_segments: &[OrthogonalSegment],
        base: &Base,
    ) -> bool {
        for &other in &self.visible {
            if other == edge {
                continue;
            }
            for candidate_segment in path_segments {
                for other_segment in &base.segments[other] {
                    if same_axis_segment_overlap_length(candidate_segment, other_segment, 0.5)
                        >= MIN_SHARED
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// `totalBends(replacements)`.
    fn total_bends_with(&self, replacements: &Replacements) -> f64 {
        self.visible.iter().fold(0.0, |sum, &edge| {
            sum + count_orthogonal_bends(&self.pts(replacements, edge), EPS) as f64
        })
    }

    /// `totalLength(replacements)`.
    fn total_length_with(&self, replacements: &Replacements) -> f64 {
        self.visible.iter().fold(0.0, |sum, &edge| {
            sum + manhattan_length(&self.pts(replacements, edge))
        })
    }

    /// The segments, bends and lengths of the visible edges in the current layout.
    fn base(&self) -> Base {
        let empty = Replacements::new();
        let count = self.view.edges.len();
        let mut base = Base {
            segments: vec![Vec::new(); count],
            bends: vec![0.0; count],
            lengths: vec![0.0; count],
        };
        for &edge in &self.visible {
            let path = self.pts(&empty, edge);
            base.segments[edge] = segments_for(&path);
            base.bends[edge] = count_orthogonal_bends(&path, EPS) as f64;
            base.lengths[edge] = manhattan_length(&path);
        }
        base
    }

    /// One iteration: a single route move, a pair move, or the end of the pass.
    pub(super) fn step(&self) -> Step {
        let empty = Replacements::new();
        let current = self.snapshot(&empty);
        let current_crossings = current.count;
        if current_crossings == 0.0 {
            return Step::Stop;
        }

        let mut best: Option<(usize, Vec<Point>)> = None;
        let mut best_crossings = current_crossings;
        let mut best_bends = f64::INFINITY;

        let base = self.base();
        for &edge in &current.edges {
            let current_edge_bends = count_orthogonal_bends(&self.pts(&empty, edge), EPS) as f64;
            for candidate in candidate_paths_for(self, edge) {
                let candidate_hits_node = self.path_hits_node(edge, &candidate);
                let candidate_segments = segments_for(&candidate);
                let candidate_has_segment_conflict = !candidate_hits_node
                    && self.segments_conflict_with(edge, &candidate_segments, &base);
                if candidate_hits_node || candidate_has_segment_conflict {
                    continue;
                }
                let counted_segments = segments_for(&dedupe_consecutive_points(&candidate, EPS));
                let candidate_crossings =
                    self.count_with_single(&current, edge, &counted_segments, &base);
                let candidate_bends = count_orthogonal_bends(&candidate, EPS) as f64;
                let improves_current_edge = candidate_crossings < current_crossings
                    || (candidate_crossings == current_crossings
                        && candidate_bends < current_edge_bends);
                if !improves_current_edge {
                    continue;
                }
                if candidate_crossings > best_crossings
                    || (candidate_crossings == best_crossings && candidate_bends >= best_bends)
                {
                    continue;
                }
                best = Some((edge, candidate));
                best_crossings = candidate_crossings;
                best_bends = candidate_bends;
            }
        }

        if let Some((edge, path)) = best {
            return Step::Single(edge, path);
        }

        match self.best_paired_replacement(&current) {
            Some(replacements) => Step::Paired(replacements),
            None => Step::Stop,
        }
    }

    /// `bestPairedReplacement(current)`.
    fn best_paired_replacement(&self, current: &Snapshot) -> Option<Replacements> {
        let empty = Replacements::new();
        let current_bends = self.total_bends_with(&empty);
        let current_length = self.total_length_with(&empty);
        let base = self.base();
        let count = self.view.edges.len();

        let mut crossings_by_edge: HashMap<usize, f64> = HashMap::new();
        for pair in &current.pairs {
            *crossings_by_edge.entry(pair.first).or_insert(0.0) += pair.count;
            *crossings_by_edge.entry(pair.second).or_insert(0.0) += pair.count;
        }

        let groups = self.pair_search_groups(current);
        let mut options: Vec<Option<PairOption>> = (0..count).map(|_| None).collect();
        for group in &groups {
            for &edge in group {
                if options[edge].is_some() {
                    continue;
                }
                let candidates =
                    pair_candidates_for(self, edge, current, &base, &crossings_by_edge);
                if !candidates.is_empty() {
                    options[edge] = Some(PairOption { edge, candidates });
                }
            }
        }

        let mut best = PairScore {
            replacements: Replacements::new(),
            crossings: current.count,
            bends: current_bends,
            length: current_length,
        };
        let scoring = Scoring {
            current,
            current_bends,
            current_length,
            base: &base,
        };

        for group in &groups {
            let crossing_edges: HashSet<usize> = group
                .iter()
                .copied()
                .filter(|edge| current.edge_set.contains(edge))
                .collect();
            let group_options: Vec<&PairOption> = group
                .iter()
                .filter_map(|&edge| options[edge].as_ref())
                .collect();
            for i in 0..group_options.len() {
                for j in (i + 1)..group_options.len() {
                    let first = group_options[i];
                    let second = group_options[j];
                    if !crossing_edges.contains(&first.edge)
                        && !crossing_edges.contains(&second.edge)
                    {
                        continue;
                    }
                    best = best_for_pair(self, &scoring, first, second, best);
                }
            }
        }

        if best.replacements.is_empty() {
            None
        } else {
            Some(best.replacements)
        }
    }
}

/// The inputs of the pair scoring.
struct Scoring<'s> {
    current: &'s Snapshot,
    current_bends: f64,
    current_length: f64,
    base: &'s Base,
}

/// `pairCandidatesFor(edge, current, baseSegments, crossingCountByEdge)`: the candidate paths of one edge,
/// without a node hit, that add no more crossings than the layout has, the best of them first.
fn pair_candidates_for(
    ctx: &Ctx,
    edge: usize,
    current: &Snapshot,
    base: &Base,
    crossings_by_edge: &HashMap<usize, f64>,
) -> Vec<PairCandidate> {
    struct Scored {
        path: Vec<Point>,
        segments: Vec<OrthogonalSegment>,
        crossings: f64,
        bends: f64,
        length: f64,
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut scored: Vec<Scored> = Vec::new();
    for raw in candidate_paths_for(ctx, edge) {
        let candidate = simplified(&raw);
        if ctx.path_hits_node(edge, &candidate) {
            continue;
        }
        let key = path_key(&candidate);
        if seen.contains(&key) || candidate.len() < 2 {
            continue;
        }
        seen.insert(key);

        let candidate_segments = segments_for(&candidate);
        let mut replacement_affected = 0.0;
        for &other in &ctx.visible {
            if other == edge {
                continue;
            }
            replacement_affected += crossings_between(&candidate_segments, &base.segments[other]);
        }
        let crossings = current.count - crossings_by_edge.get(&edge).copied().unwrap_or(0.0)
            + replacement_affected;
        let bends = count_orthogonal_bends(&candidate, EPS) as f64;
        let length = manhattan_length(&candidate);
        scored.push(Scored {
            path: candidate,
            segments: candidate_segments,
            crossings,
            bends,
            length,
        });
    }

    scored.retain(|candidate| candidate.crossings <= current.count);
    // The reference sorts the candidates with a comparator on their numbers, which is not a total order when a
    // number is NaN. The sort is over the indices, as V8's is over the elements; the candidates are then taken in
    // the order it gives.
    let mut order: Vec<usize> = (0..scored.len()).collect();
    js_sort_by(&mut order, |i, j| {
        let (a, b) = (&scored[i], &scored[j]);
        js_or(
            js_or(a.crossings - b.crossings, || a.bends - b.bends),
            || a.length - b.length,
        )
    });
    let mut slots: Vec<Option<Scored>> = scored.into_iter().map(Some).collect();
    scored = order
        .into_iter()
        .map(|index| slots[index].take().expect("each index is taken once"))
        .collect();
    scored.truncate(MAX_PAIR_CANDIDATES_PER_EDGE);

    scored
        .into_iter()
        .map(|candidate| PairCandidate {
            shared_track_conflicts: shared_track_conflicts_for(
                ctx,
                edge,
                &candidate.segments,
                base,
            ),
            total_bends: candidate.bends,
            length: candidate.length,
            path: candidate.path,
            segments: candidate.segments,
        })
        .collect()
}

/// `sharedTrackConflictsFor(edge, candidateSegments, baseSegments)`: the visible edges that the candidate
/// shares a track with.
fn shared_track_conflicts_for(
    ctx: &Ctx,
    edge: usize,
    candidate_segments: &[OrthogonalSegment],
    base: &Base,
) -> Vec<usize> {
    let mut conflicts = Vec::new();
    for &other in &ctx.visible {
        if other == edge {
            continue;
        }
        let other_segments = &base.segments[other];
        if candidate_segments.iter().any(|candidate_segment| {
            other_segments.iter().any(|other_segment| {
                same_axis_segment_overlap_length(candidate_segment, other_segment, 0.5)
                    >= MIN_SHARED
            })
        }) {
            conflicts.push(other);
        }
    }
    conflicts
}

/// `pairCrossingCount(current, firstEdge, firstCandidate, secondEdge, secondCandidate, baseSegments)`.
fn pair_crossing_count(
    ctx: &Ctx,
    current: &Snapshot,
    first: (usize, &PairCandidate),
    second: (usize, &PairCandidate),
    base: &Base,
) -> f64 {
    let (first_edge, first_candidate) = first;
    let (second_edge, second_candidate) = second;

    let mut current_affected = 0.0;
    for pair in &current.pairs {
        if pair.first == first_edge
            || pair.second == first_edge
            || pair.first == second_edge
            || pair.second == second_edge
        {
            current_affected += pair.count;
        }
    }

    let mut replacement_affected =
        crossings_between(&first_candidate.segments, &second_candidate.segments);
    for &other in &ctx.visible {
        if other == first_edge || other == second_edge {
            continue;
        }
        let other_segments = &base.segments[other];
        replacement_affected += crossings_between(&first_candidate.segments, other_segments)
            + crossings_between(&second_candidate.segments, other_segments);
    }

    current.count - current_affected + replacement_affected
}

/// `conflictsOnlyWith(candidate, edge)`.
fn conflicts_only_with(candidate: &PairCandidate, edge: usize) -> bool {
    candidate
        .shared_track_conflicts
        .iter()
        .all(|&conflict| conflict == edge)
}

/// `candidatesShareTrack(first, second)`.
fn candidates_share_track(first: &PairCandidate, second: &PairCandidate) -> bool {
    first.segments.iter().any(|first_segment| {
        second.segments.iter().any(|second_segment| {
            same_axis_segment_overlap_length(first_segment, second_segment, 0.5) >= MIN_SHARED
        })
    })
}

/// `pairScoreIsBetter(candidate, best)`: fewer crossings, then fewer bends, then shorter.
fn pair_score_is_better(candidate: &PairScore, best: &PairScore) -> bool {
    candidate.crossings < best.crossings
        || (candidate.crossings == best.crossings
            && (candidate.bends < best.bends
                || (candidate.bends == best.bends && candidate.length < best.length)))
}

/// `scorePairReplacement(...)`: the score of one pair of candidates, when it removes crossings.
fn score_pair(
    ctx: &Ctx,
    scoring: &Scoring,
    first: &PairOption,
    first_candidate: &PairCandidate,
    second: &PairOption,
    second_candidate: &PairCandidate,
) -> Option<PairScore> {
    let crossings = pair_crossing_count(
        ctx,
        scoring.current,
        (first.edge, first_candidate),
        (second.edge, second_candidate),
        scoring.base,
    );
    if crossings >= scoring.current.count {
        return None;
    }

    let mut replacements = Replacements::new();
    replacements.insert(first.edge, first_candidate.path.clone());
    replacements.insert(second.edge, second_candidate.path.clone());
    Some(PairScore {
        replacements,
        crossings,
        bends: scoring.current_bends
            - scoring.base.bends[first.edge]
            - scoring.base.bends[second.edge]
            + first_candidate.total_bends
            + second_candidate.total_bends,
        length: scoring.current_length
            - scoring.base.lengths[first.edge]
            - scoring.base.lengths[second.edge]
            + first_candidate.length
            + second_candidate.length,
    })
}

/// `bestScoreForOptionPair(context, first, second, best)`.
fn best_for_pair(
    ctx: &Ctx,
    scoring: &Scoring,
    first: &PairOption,
    second: &PairOption,
    mut best: PairScore,
) -> PairScore {
    for first_candidate in &first.candidates {
        for second_candidate in &second.candidates {
            if !conflicts_only_with(first_candidate, second.edge)
                || !conflicts_only_with(second_candidate, first.edge)
                || candidates_share_track(first_candidate, second_candidate)
            {
                continue;
            }
            if let Some(score) = score_pair(
                ctx,
                scoring,
                first,
                first_candidate,
                second,
                second_candidate,
            ) && pair_score_is_better(&score, &best)
            {
                best = score;
            }
        }
    }
    best
}
