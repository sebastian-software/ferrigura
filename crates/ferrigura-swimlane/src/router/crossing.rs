//! The crossing reduction (`routeEdgesOrthogonal`, phase 2, lines 1618-2004): the tracks of the pipes, the
//! conflicts of two segments on a track, and the three passes that move segments to other tracks, the source
//! handles, the target handles and the pipes, repeated until none of them changes anything.
//!
//! A conflict is two segments that overlap on one track, or that cross on adjacent tracks of their edge. A swap
//! of two segments between their tracks is taken when both fit; otherwise the second segment moves to a free
//! track of the pipe, or to a new one.

use indexmap::IndexMap;

use super::{DestInfo, Orientation, Router, SegmentRef};
use crate::js::js_sort_by;

/// Which segments move when a conflict is resolved by a move: the whole chain of the edge on the pipe, or only the
/// segment itself (`moveSegmentChainToTrack` and `moveSegmentToTrack`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mover {
    Chain,
    Single,
}

/// `segmentsOverlap(s1, s2)` (`router.ts:1618-1623`): two intervals overlap when each starts before the other ends.
fn segments_overlap(from1: f64, to1: f64, from2: f64, to2: f64) -> bool {
    from1 < to2 && from2 < to1
}

impl Router {
    /// The pipe of a segment of an edge. Every segment that is in an edge's list is on a pipe.
    fn pipe_of(&self, segment: usize) -> usize {
        self.segments[segment]
            .pipe
            .expect("a segment of an edge is on a pipe")
    }

    /// `trySwapSegmentsAcrossTracks` (`router.ts:1625-1670`): swaps two segments between their tracks when each
    /// fits on the other track. Returns whether the swap was made.
    fn try_swap_segments_across_tracks(&mut self, s1: usize, s2: usize) -> bool {
        let seg1 = self.segments[s1];
        let seg2 = self.segments[s2];
        let (p1, k1) = (self.pipe_of(s1), seg1.track);
        let (p2, k2) = (self.pipe_of(s2), seg2.track);

        let can_s1_go_t2 = !self.pipes[p2].tracks[k2].iter().any(|r| {
            (r.edge != seg2.edge || r.segment != seg2.segment)
                && segments_overlap(r.from, r.to, seg1.from, seg1.to)
        });
        let can_s2_go_t1 = !self.pipes[p1].tracks[k1].iter().any(|r| {
            (r.edge != seg1.edge || r.segment != seg1.segment)
                && segments_overlap(r.from, r.to, seg2.from, seg2.to)
        });
        if !(can_s1_go_t2 && can_s2_go_t1) {
            return false;
        }

        self.segments[s1].track = k2;
        self.segments[s2].track = k1;
        // The track of s1 keeps its other segments and takes s2; the track of s2 does the same with s1. When the
        // two tracks are one, the second assignment sees the first, as it does in the reference.
        let s2_ref = SegmentRef {
            edge: seg2.edge,
            segment: seg2.segment,
            from: seg2.from,
            to: seg2.to,
        };
        let track1 = &mut self.pipes[p1].tracks[k1];
        track1.retain(|r| r.edge != seg1.edge || r.segment != seg1.segment);
        track1.push(s2_ref);
        let s1_ref = SegmentRef {
            edge: seg1.edge,
            segment: seg1.segment,
            from: seg1.from,
            to: seg1.to,
        };
        let track2 = &mut self.pipes[p2].tracks[k2];
        track2.retain(|r| r.edge != seg2.edge || r.segment != seg2.segment);
        track2.push(s1_ref);
        true
    }

    /// `createNewTrack(pipe)` (`router.ts:1672-1676`): a new empty track at the end of the pipe. Returns its index.
    fn create_new_track(&mut self, pipe: usize) -> usize {
        self.pipes[pipe].tracks.push(Vec::new());
        self.pipes[pipe].tracks.len() - 1
    }

    /// `moveSegmentToTrack(seg, trackIdx)` (`router.ts:1678-1691`): moves one segment to another track of its pipe.
    fn move_segment_to_track(&mut self, segment: usize, track: usize) {
        let seg = self.segments[segment];
        let pipe = self.pipe_of(segment);
        let old_track = seg.track;
        self.pipes[pipe].tracks[old_track]
            .retain(|r| r.edge != seg.edge || r.segment != seg.segment);
        self.segments[segment].track = track;
        self.pipes[pipe].tracks[track].push(SegmentRef {
            edge: seg.edge,
            segment: seg.segment,
            from: seg.from,
            to: seg.to,
        });
    }

    /// `moveSegmentChainToTrack(seg, trackIdx)` (`router.ts:1693-1702`): moves every segment of the edge that is on
    /// the same pipe as `seg` to the track.
    fn move_segment_chain_to_track(&mut self, segment: usize, track: usize) {
        let pipe = self.segments[segment].pipe;
        let indices = self.edge_segments[self.segments[segment].edge].clone();
        for index in indices {
            if self.segments[index].pipe == pipe {
                self.move_segment_to_track(index, track);
            }
        }
    }

    /// `getAdjacentSegmentsAlongEdge(seg)` (`router.ts:1704-1715`): the segments before and after `segment` in the
    /// route of its edge. A segment that is not in the route is at position -1, which gives the first segment as
    /// the next one, as the reference's `indices[idxInList + 1]` does.
    fn adjacent_segments_along_edge(&self, segment: usize) -> Vec<usize> {
        let indices = &self.edge_segments[self.segments[segment].edge];
        let position: isize = indices
            .iter()
            .position(|&index| index == segment)
            .map_or(-1, |position| position as isize);
        let mut adjacent = Vec::new();
        if position > 0 {
            adjacent.push(indices[(position - 1) as usize]);
        }
        if position < indices.len() as isize - 1 {
            adjacent.push(indices[(position + 1) as usize]);
        }
        adjacent
    }

    /// `haveAnyCrossing(segA, segB)` (`router.ts:1717-1726`): a horizontal and a vertical segment cross when each
    /// pipe's coordinate lies strictly within the other segment.
    fn have_any_crossing(&self, a: usize, b: usize) -> bool {
        let seg_a = self.segments[a];
        let seg_b = self.segments[b];
        if seg_a.orientation == seg_b.orientation {
            return false;
        }
        let (h, v) = if seg_a.orientation == Orientation::Horizontal {
            (seg_a, seg_b)
        } else {
            (seg_b, seg_a)
        };
        v.pipe_coord > h.from && v.pipe_coord < h.to && h.pipe_coord > v.from && h.pipe_coord < v.to
    }

    /// `findAvailableTrack(pipe, seg)` (`router.ts:1728-1740`): the first track of the pipe where the segment
    /// overlaps no other segment, or -1.
    fn find_available_track(&self, pipe: usize, segment: usize) -> isize {
        let seg = self.segments[segment];
        for (index, track) in self.pipes[pipe].tracks.iter().enumerate() {
            let overlap = track.iter().any(|r| {
                (r.edge != seg.edge || r.segment != seg.segment)
                    && segments_overlap(r.from, r.to, seg.from, seg.to)
            });
            if !overlap {
                return index as isize;
            }
        }
        -1
    }

    /// `segmentsConflict(s1, s2)` (`router.ts:1742-1750`): two segments on one track conflict when they overlap;
    /// on different tracks, when a segment next to one of them crosses a segment next to the other.
    fn segments_conflict(&self, s1: usize, s2: usize) -> bool {
        if self.segments[s1].track == self.segments[s2].track {
            let a = self.segments[s1];
            let b = self.segments[s2];
            return segments_overlap(a.from, a.to, b.from, b.to);
        }
        let adjacent1 = self.adjacent_segments_along_edge(s1);
        let adjacent2 = self.adjacent_segments_along_edge(s2);
        adjacent1
            .iter()
            .any(|&a1| adjacent2.iter().any(|&a2| self.have_any_crossing(a1, a2)))
    }

    /// `resolveTrackConflict(s1, s2, move)` (`router.ts:1752-1770`): a swap when it fits; otherwise `s2` moves to
    /// the first free track of the pipe, or to a new track.
    fn resolve_track_conflict(&mut self, s1: usize, s2: usize, mover: Mover) {
        if self.try_swap_segments_across_tracks(s1, s2) {
            return;
        }
        let pipe = self.pipe_of(s1);
        let available = self.find_available_track(pipe, s2);
        let track = if available != -1 {
            available as usize
        } else {
            self.create_new_track(pipe)
        };
        match mover {
            Mover::Chain => self.move_segment_chain_to_track(s2, track),
            Mover::Single => self.move_segment_to_track(s2, track),
        }
    }

    /// `resolveHandleConflicts(handles)` (`router.ts:1772-1789`): every pair of handles on one pipe that conflicts
    /// is counted and resolved. Returns the number of conflicts.
    fn resolve_handle_conflicts(&mut self, handles: &[usize]) -> usize {
        let mut crossings = 0;
        for i in 0..handles.len() {
            for j in i + 1..handles.len() {
                let h1 = handles[i];
                let h2 = handles[j];
                if self.segments[h1].pipe != self.segments[h2].pipe {
                    continue;
                }
                if self.segments_conflict(h1, h2) {
                    crossings += 1;
                    self.resolve_track_conflict(h1, h2, Mover::Chain);
                }
            }
        }
        crossings
    }

    /// `getDestInfo(edgeIdx)` (`router.ts:1798-1824`): the destination of the edge, from its first horizontal
    /// segment after the first. The result is cached, and the cache is never cleared.
    pub(super) fn dest_info(&self, edge: usize) -> DestInfo {
        let cached = self.dest_info_cache.borrow().get(&edge).copied();
        if let Some(info) = cached {
            return info;
        }
        let indices = self.edge_segments[edge].clone();
        let info = if indices.is_empty() {
            DestInfo {
                dest: 0.0,
                deviation: 0.0,
                delta: 0.0,
            }
        } else {
            let base = self.segments[indices[0]].pipe_coord;
            let mut dest = base;
            for &index in &indices[1..] {
                let seg = self.segments[index];
                if seg.orientation == Orientation::Horizontal {
                    let candidate_a = seg.from;
                    let candidate_b = seg.to;
                    dest = if (candidate_a - base).abs() > (candidate_b - base).abs() {
                        candidate_a
                    } else {
                        candidate_b
                    };
                    break;
                }
            }
            DestInfo {
                dest,
                deviation: (dest - base).abs(),
                delta: dest - base,
            }
        };
        self.dest_info_cache.borrow_mut().insert(edge, info);
        info
    }

    /// The length of the edge's straight distance between its nodes (`getEdgeDistance`, `router.ts:1842-1855`).
    fn edge_distance(&self, edge: usize) -> f64 {
        let edge = &self.edges[edge];
        let (Some(start), Some(end)) = (edge.start.as_deref(), edge.end.as_deref()) else {
            return 0.0;
        };
        let (Some(src), Some(dst)) = (self.geo(start), self.geo(end)) else {
            return 0.0;
        };
        let dx = dst.x - src.x;
        let dy = dst.y - src.y;
        dx.abs() + dy.abs()
    }

    /// The comparator of the sources (`router.ts:1859-1901`): the edges that stay near the centre come first, then
    /// the longer spans, the fewer segments, and the shorter first segment.
    fn compare_sources(&self, a: usize, b: usize) -> f64 {
        let info_a = self.dest_info(a);
        let info_b = self.dest_info(b);
        if (info_a.deviation - info_b.deviation).abs() > 1.0 {
            return info_a.deviation - info_b.deviation;
        }
        if (info_a.dest - info_b.dest).abs() > 1.0 {
            return info_a.dest - info_b.dest;
        }

        let dist_a = self.edge_distance(a);
        let dist_b = self.edge_distance(b);
        if (dist_a - dist_b).abs() > 1.0 {
            return dist_b - dist_a;
        }

        let len_a = self.edge_segments[a].len();
        let len_b = self.edge_segments[b].len();
        if len_a != len_b {
            return len_a as f64 - len_b as f64;
        }

        if len_a == 1 {
            let seg_a = self.segments[self.edge_segments[a][0]];
            let seg_b = self.segments[self.edge_segments[b][0]];
            let dist_a = (seg_a.to - seg_a.from).abs();
            let dist_b = (seg_b.to - seg_b.from).abs();
            if (dist_a - dist_b).abs() > 1.0 {
                return dist_a - dist_b;
            }
        }
        0.0
    }

    /// `fixSourceHandleCrossings` (`router.ts:1826-1908`): the edges that leave a node are sorted, and their first
    /// segments are made not to conflict.
    fn fix_source_handle_crossings(&mut self) -> usize {
        let mut crossings = 0;
        let mut by_source: IndexMap<String, Vec<usize>> = IndexMap::new();
        for index in 0..self.edges.len() {
            if self.edge_segments[index].is_empty() {
                continue;
            }
            let Some(start) = self.edges[index].start.clone() else {
                continue;
            };
            by_source.entry(start).or_default().push(index);
        }

        for mut group in by_source.into_values() {
            js_sort_by(&mut group, |a, b| self.compare_sources(a, b));
            let handles: Vec<usize> = group
                .iter()
                .map(|&edge| self.edge_segments[edge][0])
                .collect();
            crossings += self.resolve_handle_conflicts(&handles);
        }
        crossings
    }

    /// The length of the segment before the last one of an edge (`getDist` in `fixTargetHandleCrossings`).
    fn target_distance(&self, edge: usize) -> f64 {
        let indices = &self.edge_segments[edge];
        if indices.len() < 2 {
            return 0.0;
        }
        let prev = self.segments[indices[indices.len() - 2]];
        (prev.to - prev.from).abs()
    }

    /// `fixTargetHandleCrossings` (`router.ts:1910-1954`): the edges that arrive at a node are sorted by the length
    /// of their last bend, and their last segments are made not to conflict.
    fn fix_target_handle_crossings(&mut self) -> usize {
        let mut crossings = 0;
        let mut by_target: IndexMap<String, Vec<usize>> = IndexMap::new();
        for index in 0..self.edges.len() {
            if self.edge_segments[index].is_empty() {
                continue;
            }
            let Some(end) = self.edges[index].end.clone() else {
                continue;
            };
            by_target.entry(end).or_default().push(index);
        }

        for mut group in by_target.into_values() {
            js_sort_by(&mut group, |a, b| {
                let score_a = self.target_distance(a);
                let score_b = self.target_distance(b);
                if (score_a - score_b).abs() > 0.1 {
                    score_a - score_b
                } else {
                    a as f64 - b as f64
                }
            });
            let handles: Vec<usize> = group
                .iter()
                .map(|&edge| {
                    let indices = &self.edge_segments[edge];
                    indices[indices.len() - 1]
                })
                .collect();
            crossings += self.resolve_handle_conflicts(&handles);
        }
        crossings
    }

    /// `fixPipeCrossings` (`router.ts:1956-1990`): on each pipe, every pair of segments that conflict is resolved,
    /// in the order of the edges and of the segments.
    fn fix_pipe_crossings(&mut self) -> usize {
        let mut crossings = 0;
        for pipe in 0..self.pipes.len() {
            let refs: Vec<SegmentRef> = self.pipes[pipe]
                .tracks
                .iter()
                .flat_map(|track| track.iter().copied())
                .collect();
            let mut pipe_segments: Vec<usize> = Vec::new();
            for r in refs {
                let found = self.edge_segments[r.edge]
                    .iter()
                    .copied()
                    .find(|&index| self.segments[index].segment == r.segment);
                if let Some(index) = found {
                    pipe_segments.push(index);
                }
            }

            js_sort_by(&mut pipe_segments, |a, b| {
                let (seg_a, seg_b) = (self.segments[a], self.segments[b]);
                let by_edge = seg_a.edge as f64 - seg_b.edge as f64;
                if by_edge != 0.0 {
                    by_edge
                } else {
                    seg_a.segment as f64 - seg_b.segment as f64
                }
            });

            for i in 0..pipe_segments.len() {
                for j in i + 1..pipe_segments.len() {
                    let s1 = pipe_segments[i];
                    let s2 = pipe_segments[j];
                    if self.segments_conflict(s1, s2) {
                        crossings += 1;
                        self.resolve_track_conflict(s1, s2, Mover::Single);
                    }
                }
            }
        }
        crossings
    }

    /// The main loop of the reduction (`router.ts:1992-2004`): the three passes, up to ten rounds, until a round
    /// changes nothing.
    pub(super) fn reduce_crossings(&mut self) {
        let max_iterations = 10;
        let mut iterations = 0;
        while iterations < max_iterations {
            let mut changed = 0;
            changed += self.fix_source_handle_crossings();
            changed += self.fix_target_handle_crossings();
            changed += self.fix_pipe_crossings();
            if changed == 0 {
                break;
            }
            iterations += 1;
        }
    }
}
