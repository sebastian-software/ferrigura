//! The layout's constraints, as `architectureRenderer.ts` builds them from the DB's spatial maps and
//! group alignments: `getAlignments` (155-289) and `getRelativeConstraints` (291-373).
//!
//! The maps are insertion-ordered in the reference (JavaScript `Map` and object keys), and the order
//! reaches the constraint handler, so each map here keeps its entries in the order they were first
//! set, and an entry set again keeps its place.

use std::collections::{HashMap, HashSet};

use ferrigura_cose::fcose::{Alignment, Relative};
use ferrigura_model::architecture::{AlignDirection, ArchitectureDiagram, Axis, Direction};

/// The constraints of one layout run: the alignment lists and the relative placements.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Constraints {
    /// `alignmentConstraint`: the horizontal and the vertical lists.
    pub alignment: Alignment,
    /// `relativePlacementConstraint`: the placements, in order.
    pub relative: Vec<Relative>,
}

/// An insertion-ordered map of string keys, as JavaScript's `Map` keeps its entries.
#[derive(Debug, Clone)]
struct OrderedMap<K, V> {
    entries: Vec<(K, V)>,
}

impl<K: PartialEq, V> OrderedMap<K, V> {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// The value of a key, inserting `default` when the key is new (`map.get(k) ?? new`, then `set`).
    fn entry_or(&mut self, key: K, default: impl FnOnce() -> V) -> &mut V {
        let position = match self.entries.iter().position(|(k, _)| *k == key) {
            Some(position) => position,
            None => {
                self.entries.push((key, default()));
                self.entries.len() - 1
            }
        };
        &mut self.entries[position].1
    }

    /// `map.set(key, value)`: replaces the value of a key, which keeps its place.
    fn set(&mut self, key: K, value: V) {
        match self.entries.iter().position(|(k, _)| *k == key) {
            Some(position) => self.entries[position].1 = value,
            None => self.entries.push((key, value)),
        }
    }
}

/// The group a node is in, as `db.getNode(id)?.in ?? 'default'` gives it.
fn group_of(diagram: &ArchitectureDiagram) -> HashMap<&str, &str> {
    let mut groups = HashMap::new();
    for service in &diagram.services {
        groups.insert(service.id.as_str(), service.parent.as_str());
    }
    for junction in &diagram.junctions {
        groups.insert(junction.id.as_str(), junction.parent.as_str());
    }
    groups
}

/// `architectureGroupAlignmentKey(a, b)`: the two ids sorted, each JSON-quoted, joined by `-`.
fn alignment_of(alignments: &HashMap<String, Axis>, a: &str, b: &str) -> Option<Axis> {
    let (lower, upper) = if a <= b { (a, b) } else { (b, a) };
    let quote = |id: &str| serde_json::to_string(id).unwrap_or_default();
    alignments
        .get(&format!("{}-{}", quote(lower), quote(upper)))
        .copied()
}

/// `flattenAlignments` (architectureRenderer.ts 172-211): the nodes of one row or column, merged
/// across the groups that align, and split apart where the groups do not.
fn flatten(
    axis_map: &OrderedMap<i64, OrderedMap<String, Vec<String>>>,
    axis: Axis,
    alignments: &HashMap<String, Axis>,
) -> OrderedMap<String, Vec<String>> {
    let mut flattened: OrderedMap<String, Vec<String>> = OrderedMap::new();
    for (numeric, groups) in &axis_map.entries {
        let dir = numeric.to_string();
        if groups.entries.len() == 1 {
            flattened.set(dir.clone(), groups.entries[0].1.clone());
            continue;
        }
        let mut count = 0usize;
        let entries = &groups.entries;
        for i in 0..entries.len().saturating_sub(1) {
            for j in (i + 1)..entries.len() {
                let (a_group, a_nodes) = &entries[i];
                let (b_group, b_nodes) = &entries[j];
                let aligned = alignment_of(alignments, a_group, b_group) == Some(axis);
                if aligned || a_group == "default" || b_group == "default" {
                    let mut merged = flattened.entry_or(dir.clone(), Vec::new).clone();
                    merged.extend(a_nodes.iter().cloned());
                    merged.extend(b_nodes.iter().cloned());
                    flattened.set(dir.clone(), merged);
                } else {
                    let key_a = format!("{dir}-{count}");
                    count += 1;
                    flattened.set(key_a, a_nodes.clone());
                    let key_b = format!("{dir}-{count}");
                    count += 1;
                    flattened.set(key_b, b_nodes.clone());
                }
            }
        }
    }
    flattened
}

/// `getAlignments`: the alignment lists of every spatial map, and then the declared hints.
#[must_use]
pub fn alignment_constraint(diagram: &ArchitectureDiagram) -> Alignment {
    let groups = group_of(diagram);
    let alignments: HashMap<String, Axis> = diagram
        .derived
        .group_alignments
        .iter()
        .map(|(key, axis)| (key.clone(), *axis))
        .collect();
    let mut horizontal_raw: Vec<Vec<String>> = Vec::new();
    let mut vertical_raw: Vec<Vec<String>> = Vec::new();
    for spatial_map in &diagram.derived.spatial_maps {
        let mut by_row: OrderedMap<i64, OrderedMap<String, Vec<String>>> = OrderedMap::new();
        let mut by_column: OrderedMap<i64, OrderedMap<String, Vec<String>>> = OrderedMap::new();
        for position in spatial_map {
            let group = groups
                .get(position.id.as_str())
                .copied()
                .filter(|group| !group.is_empty())
                .unwrap_or("default")
                .to_owned();
            for axis_map in [
                by_row.entry_or(position.y, OrderedMap::new),
                by_column.entry_or(position.x, OrderedMap::new),
            ] {
                axis_map
                    .entry_or(group.clone(), Vec::new)
                    .push(position.id.clone());
            }
        }
        let rows = flatten(&by_row, Axis::Horizontal, &alignments);
        let columns = flatten(&by_column, Axis::Vertical, &alignments);
        horizontal_raw.extend(
            rows.entries
                .into_iter()
                .map(|(_, list)| list)
                .filter(|list| list.len() > 1),
        );
        vertical_raw.extend(
            columns
                .entries
                .into_iter()
                .map(|(_, list)| list)
                .filter(|list| list.len() > 1),
        );
    }

    // A heuristic group that holds a declared member is dropped: the declared hint wins.
    let declared: HashSet<&str> = diagram
        .alignments
        .iter()
        .flat_map(|hint| hint.members.iter().map(String::as_str))
        .collect();
    let keep = |list: &Vec<String>| !list.iter().any(|id| declared.contains(id.as_str()));
    let mut horizontal: Vec<Vec<String>> = horizontal_raw.into_iter().filter(keep).collect();
    let mut vertical: Vec<Vec<String>> = vertical_raw.into_iter().filter(keep).collect();

    for hint in &diagram.alignments {
        if hint.members.len() < 2 {
            continue;
        }
        match hint.direction {
            AlignDirection::Row => horizontal.push(hint.members.clone()),
            AlignDirection::Column => vertical.push(hint.members.clone()),
        }
    }
    Alignment {
        horizontal,
        vertical,
    }
}

/// `getRelativeConstraints`: the declared hints as chains of placements, then one placement for each
/// pair of neighbours that a breadth-first search over each spatial map finds.
#[must_use]
pub fn relative_constraints(
    diagram: &ArchitectureDiagram,
    icon_size: f64,
    ideal_multiplier: f64,
) -> Vec<Relative> {
    let hint_gap = ideal_multiplier * icon_size;
    let gap = ideal_multiplier * icon_size;
    let mut relative = Vec::new();
    // Every (a, b) pair that a hint declares, in both orders: the search skips those pairs.
    let mut declared: HashSet<(String, String)> = HashSet::new();
    for hint in &diagram.alignments {
        for pair in hint.members.windows(2) {
            let (a, b) = (pair[0].clone(), pair[1].clone());
            declared.insert((a.clone(), b.clone()));
            declared.insert((b.clone(), a.clone()));
            match hint.direction {
                AlignDirection::Row => relative.push(Relative {
                    left: Some(a),
                    right: Some(b),
                    gap: Some(hint_gap),
                    ..Relative::default()
                }),
                AlignDirection::Column => relative.push(Relative {
                    top: Some(a),
                    bottom: Some(b),
                    gap: Some(hint_gap),
                    ..Relative::default()
                }),
            }
        }
    }

    for spatial_map in &diagram.derived.spatial_maps {
        // The inverse map: the id at each grid position. A later entry at the same position wins.
        let mut at: HashMap<(i64, i64), &str> = HashMap::new();
        for position in spatial_map {
            at.insert((position.x, position.y), position.id.as_str());
        }
        // The breadth-first search from (0, 0), with a queue that may hold a position twice, and the
        // visited set filled when a position leaves the queue. The directions are in the order L, R,
        // T, B (architectureRenderer.ts 332-337).
        let mut queue = std::collections::VecDeque::from([(0i64, 0i64)]);
        let mut visited: HashSet<(i64, i64)> = HashSet::new();
        let directions = [
            (Direction::L, (-1i64, 0i64)),
            (Direction::R, (1, 0)),
            (Direction::T, (0, 1)),
            (Direction::B, (0, -1)),
        ];
        while let Some((x, y)) = queue.pop_front() {
            visited.insert((x, y));
            let Some(&current) = at.get(&(x, y)) else {
                continue;
            };
            for (dir, (dx, dy)) in directions {
                let next = (x + dx, y + dy);
                let Some(&neighbour) = at.get(&next) else {
                    continue;
                };
                if visited.contains(&next) {
                    continue;
                }
                queue.push_back(next);
                if declared.contains(&(current.to_owned(), neighbour.to_owned())) {
                    continue;
                }
                let placement = match dir {
                    // `left: newId, right: currId` for L, and the mirror for the others.
                    Direction::L => Relative {
                        left: Some(neighbour.to_owned()),
                        right: Some(current.to_owned()),
                        gap: Some(gap),
                        ..Relative::default()
                    },
                    Direction::R => Relative {
                        right: Some(neighbour.to_owned()),
                        left: Some(current.to_owned()),
                        gap: Some(gap),
                        ..Relative::default()
                    },
                    Direction::T => Relative {
                        top: Some(neighbour.to_owned()),
                        bottom: Some(current.to_owned()),
                        gap: Some(gap),
                        ..Relative::default()
                    },
                    Direction::B => Relative {
                        bottom: Some(neighbour.to_owned()),
                        top: Some(current.to_owned()),
                        gap: Some(gap),
                        ..Relative::default()
                    },
                };
                relative.push(placement);
            }
        }
    }
    relative
}

/// Both constraints of a diagram.
#[must_use]
pub fn constraints(
    diagram: &ArchitectureDiagram,
    icon_size: f64,
    ideal_multiplier: f64,
) -> Constraints {
    Constraints {
        alignment: alignment_constraint(diagram),
        relative: relative_constraints(diagram, icon_size, ideal_multiplier),
    }
}
