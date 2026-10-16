//! Ported from ELK (Eclipse Layout Kernel), commit 9bc93474e, Copyright (c) Kiel University and others.
//! SPDX-License-Identifier: EPL-2.0
//! Source: plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p5edges/orthogonal/HyperEdgeSegmentSplitter.java
//!
//! Splits hyperedge segments to break the critical dependency cycles: a segment is cut in two
//! at the centre of a free area between the connections of the other segments, and the two
//! halves are linked through that area. The area is chosen by counting the crossings and the
//! dependencies that the split would add.
//!
//! The segments to split are collected in an `IndexSet`, since Java's `LinkedHashSet` iterates in
//! insertion order and the splits happen in order of length.

use indexmap::IndexSet;

use crate::java::sort::sort_by;

use super::OrthogonalRoutingGenerator;
use super::segment::{Arena, CRITICAL_DEPENDENCY_WEIGHT, DependencyType};

/// `FreeArea`: a stretch between two segments' connections where another segment fits.
#[derive(Debug, Clone, Copy)]
struct FreeArea {
    start: f64,
    end: f64,
    size: f64,
}

impl FreeArea {
    fn new(start: f64, end: f64) -> Self {
        FreeArea {
            start,
            end,
            size: end - start,
        }
    }
}

/// `AreaRating`: what linking the split halves through an area would cost.
#[derive(Debug, Clone, Copy, Default)]
struct AreaRating {
    dependencies: i32,
    crossings: i32,
}

/// `HyperEdgeSegmentSplitter.splitSegments`: splits the segments that the cycle dependencies
/// name. `segments` gains the new split partners.
pub fn split_segments(
    generator: &OrthogonalRoutingGenerator,
    arena: &mut Arena,
    dependencies_to_resolve: &[usize],
    segments: &mut Vec<usize>,
    critical_conflict_threshold: f64,
) {
    // Only start preparations if there's going to be things to do.
    if dependencies_to_resolve.is_empty() {
        return;
    }

    let mut free_areas = find_free_areas(arena, segments, critical_conflict_threshold);

    let segments_to_split = decide_which_segments_to_split(arena, dependencies_to_resolve);

    // Split the segments from the smallest to the largest: the smallest have the fewest options
    // for where to put their connecting segments. Java sorts a stream, which is stable.
    let mut order: Vec<usize> = segments_to_split.into_iter().collect();
    sort_by(&mut order, |&first, &second| {
        arena.length(first).total_cmp(&arena.length(second))
    });
    for segment in order {
        split(
            generator,
            arena,
            segment,
            segments,
            &mut free_areas,
            critical_conflict_threshold,
        );
    }
}

/// `findFreeAreas`: the gaps between sorted connection coordinates that are at least twice the
/// critical threshold wide, shrunk by the threshold on both sides.
fn find_free_areas(
    arena: &Arena,
    segments: &[usize],
    critical_conflict_threshold: f64,
) -> Vec<FreeArea> {
    let mut coordinates: Vec<f64> = segments
        .iter()
        .flat_map(|&segment| arena.segments[segment].incoming.iter().copied())
        .chain(
            segments
                .iter()
                .flat_map(|&segment| arena.segments[segment].outgoing.iter().copied()),
        )
        .collect();
    coordinates.sort_by(f64::total_cmp);

    let mut free_areas = Vec::new();
    for pair in coordinates.windows(2) {
        if pair[1] - pair[0] >= 2.0 * critical_conflict_threshold {
            free_areas.push(FreeArea::new(
                pair[0] + critical_conflict_threshold,
                pair[1] - critical_conflict_threshold,
            ));
        }
    }
    free_areas
}

/// `decideWhichSegmentsToSplit`: for each dependency, one of its segments is split and the other
/// is remembered as the cause. Splitting prefers segments that do not represent a hyperedge.
fn decide_which_segments_to_split(arena: &mut Arena, dependencies: &[usize]) -> IndexSet<usize> {
    let mut segments_to_split = IndexSet::new();

    for &dependency in dependencies {
        let source_segment = arena.dependencies[dependency]
            .source
            .expect("a cycle dependency has a source");
        let target_segment = arena.dependencies[dependency]
            .target
            .expect("a cycle dependency has a target");

        // If either segment was already chosen for splitting, that is sufficient.
        if segments_to_split.contains(&source_segment)
            || segments_to_split.contains(&target_segment)
        {
            continue;
        }

        // One segment is split, and the other one stays between the two halves of the split.
        let mut segment_to_split = source_segment;
        let mut segment_causing_split = target_segment;

        // Prefer splitting a regular edge over a hyperedge, which has more chances to cross.
        if arena.represents_hyperedge(source_segment) && !arena.represents_hyperedge(target_segment)
        {
            segment_to_split = target_segment;
            segment_causing_split = source_segment;
        }

        segments_to_split.insert(segment_to_split);
        arena.segments[segment_to_split].split_by = Some(segment_causing_split);
    }

    segments_to_split
}

/// `split`: splits the segment at the best position, adds the new partner to the list, and
/// rebuilds the segment's dependencies.
fn split(
    generator: &OrthogonalRoutingGenerator,
    arena: &mut Arena,
    segment: usize,
    segments: &mut Vec<usize>,
    free_areas: &mut Vec<FreeArea>,
    critical_conflict_threshold: f64,
) {
    let split_position = compute_position_to_split_and_update_free_areas(
        arena,
        segment,
        free_areas,
        critical_conflict_threshold,
    );
    let partner = arena.split_at(segment, split_position);
    segments.push(partner);

    update_dependencies(generator, arena, segment, segments);
}

/// `updateDependencies`: the segment must come before the causing segment, which must come before
/// the partner. The other segments get their dependencies back.
fn update_dependencies(
    generator: &OrthogonalRoutingGenerator,
    arena: &mut Arena,
    segment: usize,
    segments: &[usize],
) {
    let split_causing_segment = arena.segments[segment]
        .split_by
        .expect("a split segment has the segment that caused it");
    let split_partner = arena.segments[segment]
        .split_partner
        .expect("a split segment has a partner");

    arena.add_dependency(
        DependencyType::Critical,
        segment,
        split_causing_segment,
        CRITICAL_DEPENDENCY_WEIGHT,
    );
    arena.add_dependency(
        DependencyType::Critical,
        split_causing_segment,
        split_partner,
        CRITICAL_DEPENDENCY_WEIGHT,
    );

    // The three segments in the conflict have their dependencies settled; the rest are re-added.
    for &other in segments {
        if other != split_causing_segment && other != segment && other != split_partner {
            generator.create_dependency_if_necessary(arena, other, segment);
            generator.create_dependency_if_necessary(arena, other, split_partner);
        }
    }
}

/// `computePositionToSplitAndUpdateFreeAreas`: the centre of the best free area in reach of the
/// segment, or the segment's own centre if no area is in reach. The used area is removed from
/// the list and replaced by the parts of it that are still wide enough.
fn compute_position_to_split_and_update_free_areas(
    arena: &mut Arena,
    segment: usize,
    free_areas: &mut Vec<FreeArea>,
    critical_conflict_threshold: f64,
) -> f64 {
    let segment_start = arena.segments[segment].start;
    let segment_end = arena.segments[segment].end;

    // The first and last areas that the segment can reach, in ascending order.
    let mut first_possible: Option<usize> = None;
    let mut last_possible: Option<usize> = None;
    for (index, area) in free_areas.iter().enumerate() {
        if area.start > segment_end {
            // Past the possible areas.
            break;
        } else if area.end >= segment_start {
            if first_possible.is_none() {
                first_possible = Some(index);
            }
            last_possible = Some(index);
        }
    }

    // The default position is the segment's own centre.
    let mut split_position = center(segment_start, segment_end);

    if let (Some(first), Some(last)) = (first_possible, last_possible) {
        let best = choose_best_area_index(arena, segment, free_areas, first, last);

        // Use the centre of the best area, and update the area list.
        split_position = center(free_areas[best].start, free_areas[best].end);
        use_area(free_areas, best, critical_conflict_threshold);
    }

    split_position
}

/// `chooseBestAreaIndex`: among the areas from `from` to `to` (both inclusive), the one with the
/// fewest crossings, then the fewest dependencies, then the largest.
fn choose_best_area_index(
    arena: &mut Arena,
    segment: usize,
    free_areas: &[FreeArea],
    from: usize,
    to: usize,
) -> usize {
    let mut best_index = from;

    if from < to {
        // More than one area: simulate the split so that the crossings can be counted.
        let (split_segment, split_partner) = arena.simulate_split(segment);

        let mut best_area = free_areas[best_index];
        let mut best_rating = rate_area(arena, segment, split_segment, split_partner, best_area);

        for (index, &curr_area) in free_areas.iter().enumerate().take(to + 1).skip(from + 1) {
            let curr_rating = rate_area(arena, segment, split_segment, split_partner, curr_area);
            if is_better(&curr_area, &curr_rating, &best_area, &best_rating) {
                best_area = curr_area;
                best_rating = curr_rating;
                best_index = index;
            }
        }
    }

    best_index
}

/// `rateArea`: what it would cost to link the simulated halves through `area`. The simulated
/// halves take the area's centre as their connection, and the rating counts the dependencies and
/// crossings that come with it.
fn rate_area(
    arena: &mut Arena,
    segment: usize,
    split_segment: usize,
    split_partner: usize,
    area: FreeArea,
) -> AreaRating {
    // The area's centre links the two halves, so it is their connection.
    let area_centre = center(area.start, area.end);

    arena.segments[split_segment].outgoing.clear();
    arena.segments[split_segment].outgoing.push(area_centre);

    arena.segments[split_partner].incoming.clear();
    arena.segments[split_partner].incoming.push(area_centre);

    // Count the dependencies and crossings that the halves would have with the segment's own.
    let mut rating = AreaRating::default();

    for dependency in arena.segments[segment].incoming_deps.clone() {
        let other_segment = arena.dependencies[dependency]
            .source
            .expect("a live dependency has a source");
        update_considering_both_orderings(arena, &mut rating, split_segment, other_segment);
        update_considering_both_orderings(arena, &mut rating, split_partner, other_segment);
    }

    for dependency in arena.segments[segment].outgoing_deps.clone() {
        let other_segment = arena.dependencies[dependency]
            .target
            .expect("a live dependency has a target");
        update_considering_both_orderings(arena, &mut rating, split_segment, other_segment);
        update_considering_both_orderings(arena, &mut rating, split_partner, other_segment);
    }

    // The two additional dependencies, split segment, causing segment and partner, keep their
    // order, so only the crossings for that order count.
    let split_by = arena.segments[segment]
        .split_by
        .expect("a split segment has the segment that caused it");
    rating.dependencies += 2;
    rating.crossings += count_crossings_for_single_ordering(arena, split_segment, split_by);
    rating.crossings += count_crossings_for_single_ordering(arena, split_by, split_partner);

    rating
}

/// `updateConsideringBothOrderings`: counts the crossings for both orders of the two segments.
/// Equal crossings, if any, are a two-cycle of two dependencies; otherwise the better order gives
/// one dependency.
fn update_considering_both_orderings(
    arena: &Arena,
    rating: &mut AreaRating,
    first: usize,
    second: usize,
) {
    let first_left_of_second = count_crossings_for_single_ordering(arena, first, second);
    let second_left_of_first = count_crossings_for_single_ordering(arena, second, first);

    if first_left_of_second == second_left_of_first {
        // If the crossings are the same, only more than zero matters: a two-cycle.
        if first_left_of_second > 0 {
            rating.dependencies += 2;
            rating.crossings += first_left_of_second;
        }
    } else {
        // One order is better, so there is a single dependency.
        rating.dependencies += 1;
        rating.crossings += first_left_of_second.min(second_left_of_first);
    }
}

/// `countCrossingsForSingleOrdering`: the crossings between `left` and `right`, with `left` on
/// the left.
fn count_crossings_for_single_ordering(arena: &Arena, left: usize, right: usize) -> i32 {
    let left_data = &arena.segments[left];
    let right_data = &arena.segments[right];
    OrthogonalRoutingGenerator::count_crossings(
        &left_data.outgoing,
        right_data.start,
        right_data.end,
    ) + OrthogonalRoutingGenerator::count_crossings(
        &right_data.incoming,
        left_data.start,
        left_data.end,
    )
}

/// `isBetter`: fewer crossings, then fewer dependencies, then a larger area.
fn is_better(
    curr_area: &FreeArea,
    curr_rating: &AreaRating,
    best_area: &FreeArea,
    best_rating: &AreaRating,
) -> bool {
    if curr_rating.crossings != best_rating.crossings {
        // First criterion: number of crossings.
        return curr_rating.crossings < best_rating.crossings;
    }
    if curr_rating.dependencies != best_rating.dependencies {
        // Second criterion: number of dependencies.
        return curr_rating.dependencies < best_rating.dependencies;
    }
    // Third criterion: size.
    curr_area.size > best_area.size
}

/// `useArea`: the used area is removed, and the parts left on either side of the new segment's
/// centre are added back, if they are still wide enough.
fn use_area(free_areas: &mut Vec<FreeArea>, used_index: usize, critical_conflict_threshold: f64) {
    let old_area = free_areas.remove(used_index);

    if old_area.size / 2.0 >= critical_conflict_threshold {
        // Insert the new areas here, in order.
        let mut insert_index = used_index;

        // This area is large enough to split, and to keep room for the segment's centre.
        let old_area_centre = center(old_area.start, old_area.end);

        // Be doubly sure that double precision does not bite.
        let new_end1 = old_area_centre - critical_conflict_threshold;
        if old_area.start <= old_area_centre - critical_conflict_threshold {
            free_areas.insert(insert_index, FreeArea::new(old_area.start, new_end1));
            insert_index += 1;
        }

        let new_start2 = old_area_centre + critical_conflict_threshold;
        if new_start2 <= old_area.end {
            free_areas.insert(insert_index, FreeArea::new(new_start2, old_area.end));
        }
    }
}

/// `center(p1, p2)`.
fn center(p1: f64, p2: f64) -> f64 {
    (p1 + p2) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn using_an_area_leaves_the_parts_that_are_wide_enough() {
        let mut areas = vec![FreeArea::new(2.0, 8.0)];
        use_area(&mut areas, 0, 2.0);
        let spans: Vec<(f64, f64)> = areas.iter().map(|area| (area.start, area.end)).collect();
        assert_eq!(spans, vec![(2.0, 3.0), (7.0, 8.0)]);
    }
}
