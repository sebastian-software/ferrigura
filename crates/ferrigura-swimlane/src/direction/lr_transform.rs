//! The direction transforms of `postProcessSwimlaneLayout`: `direction/lrTransform.ts` of Mermaid 12.2.0.
//!
//! The engine lays the diagram out in TB. `applyLrDirectionTransform` (`lrTransform.ts:150-327`) turns it into
//! LR or RL: the rank axis becomes x, the order axis becomes y, and the lanes are rebuilt as the horizontal
//! bands with their title bands. `applyBtDirectionTransform` (`lrTransform.ts:141-148`) mirrors y. The helpers
//! are `buildNodeMap`, the group lookups, `recomputeNestedGroupBounds` (`:79-92`), and `mirrorAxis`
//! (`:94-139`).
//!
//! The Map of the TypeScript keeps the first position of a key and the last value, which [`node_index`]
//! reproduces with `IndexMap::insert`.

use indexmap::IndexMap;

use crate::direction::geometry::{js_sort_by, truthy_text};
use crate::math::{js_max, js_min};
use crate::model::{Direction, LayoutData, Node, Point, Rect};

/// `titleBandSize` of `applyLrDirectionTransform` (`lrTransform.ts:175`).
const TITLE_BAND_SIZE: f64 = 36.0;
/// `group.padding ?? 20` in `applyGroupBounds` (`lrTransform.ts:72`).
const DEFAULT_GROUP_PADDING: f64 = 20.0;
/// `Math.max(maxPad, 10)` (`lrTransform.ts:276`).
const MIN_HORIZONTAL_MARGIN: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

/// `node.isGroup` as a flag. A group is a lane or a subgraph.
fn is_group(node: &Node) -> bool {
    node.is_group == Some(true)
}

/// `buildNodeMap(nodes)`: the position of each node by id. A later node with the same id takes the value, and
/// the first position stays, as `new Map` does.
fn node_index(nodes: &[Node]) -> IndexMap<String, usize> {
    let mut by_id = IndexMap::new();
    for (index, node) in nodes.iter().enumerate() {
        by_id.insert(node.id.clone(), index);
    }
    by_id
}

/// `resolveTopLevelGroupId(node, nodeById)` (`lrTransform.ts:11-26`): the id of the outermost group that holds
/// the node, through a chain of groups. `None` when the node is in no group.
fn resolve_top_level_group_id(
    node: &Node,
    nodes: &[Node],
    by_id: &IndexMap<String, usize>,
) -> Option<String> {
    let mut parent_id = node.parent_id.as_deref();
    let mut top_level = None;
    while let Some(id) = truthy_text(parent_id) {
        let Some(parent) = by_id.get(id).map(|&index| &nodes[index]) else {
            break;
        };
        if !is_group(parent) {
            break;
        }
        top_level = Some(parent.id.clone());
        parent_id = parent.parent_id.as_deref();
    }
    top_level
}

/// `groupDepth(group, nodeById)` (`lrTransform.ts:28-40`): the number of groups above a group.
fn group_depth(group: &Node, nodes: &[Node], by_id: &IndexMap<String, usize>) -> usize {
    let mut depth = 0;
    let mut parent_id = group.parent_id.as_deref();
    while let Some(id) = truthy_text(parent_id) {
        let Some(parent) = by_id.get(id).map(|&index| &nodes[index]) else {
            break;
        };
        if !is_group(parent) {
            break;
        }
        depth += 1;
        parent_id = parent.parent_id.as_deref();
    }
    depth
}

/// The bounds of a set of nodes (`boundsForChildren`, `lrTransform.ts:42-66`).
#[derive(Debug, Clone, Copy)]
struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

/// `boundsForChildren(children)`: the box of the nodes that have a position. A node without `x` or `y` is
/// skipped. `None` when no node has one.
fn bounds_for_children<'a>(children: impl IntoIterator<Item = &'a Node>) -> Option<Bounds> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for child in children {
        let (Some(cx), Some(cy)) = (child.x, child.y) else {
            continue;
        };
        let w = child.width.unwrap_or(0.0);
        let h = child.height.unwrap_or(0.0);
        min_x = js_min(min_x, cx - w / 2.0);
        max_x = js_max(max_x, cx + w / 2.0);
        min_y = js_min(min_y, cy - h / 2.0);
        max_y = js_max(max_y, cy + h / 2.0);
    }
    if min_x == f64::INFINITY || min_y == f64::INFINITY {
        return None;
    }
    Some(Bounds {
        min_x,
        max_x,
        min_y,
        max_y,
    })
}

/// `applyGroupBounds(group, bounds)` (`lrTransform.ts:68-77`): the group is the box of its children, padded.
fn apply_group_bounds(group: &mut Node, bounds: Bounds) {
    let pad = group.padding.unwrap_or(DEFAULT_GROUP_PADDING);
    group.x = Some((bounds.min_x + bounds.max_x) / 2.0);
    group.y = Some((bounds.min_y + bounds.max_y) / 2.0);
    group.width = Some(js_max(0.0, bounds.max_x - bounds.min_x) + pad);
    group.height = Some(js_max(0.0, bounds.max_y - bounds.min_y) + pad);
}

/// `recomputeNestedGroupBounds(nodes)` (`lrTransform.ts:79-92`): the nested groups, the deepest first, get the
/// box of their children, which may include groups that were already recomputed. The sort is stable, as the
/// TypeScript sort is.
fn recompute_nested_group_bounds(nodes: &mut [Node]) {
    let by_id = node_index(nodes);
    let mut groups: Vec<(usize, usize)> = (0..nodes.len())
        .filter(|&index| {
            is_group(&nodes[index]) && truthy_text(nodes[index].parent_id.as_deref()).is_some()
        })
        .map(|index| (index, group_depth(&nodes[index], nodes, &by_id)))
        .collect();
    // `groupDepth(b) - groupDepth(a)`: the deeper group comes first.
    groups.sort_by_key(|group| std::cmp::Reverse(group.1));

    for (group_index, _) in groups {
        let group_id = nodes[group_index].id.clone();
        let bounds = bounds_for_children(
            nodes
                .iter()
                .filter(|node| node.parent_id.as_deref() == Some(group_id.as_str())),
        );
        if let Some(bounds) = bounds {
            apply_group_bounds(&mut nodes[group_index], bounds);
        }
    }
}

fn axis_of(node: &Node, axis: Axis) -> Option<f64> {
    match axis {
        Axis::X => node.x,
        Axis::Y => node.y,
    }
}

fn set_axis_of(node: &mut Node, axis: Axis, value: f64) {
    match axis {
        Axis::X => node.x = Some(value),
        Axis::Y => node.y = Some(value),
    }
}

/// `mirrorAxis(layout, axis)` (`lrTransform.ts:94-139`): reflects one axis about the middle of the content
/// nodes, on the nodes, the title bands and the edge points. `false` when the content has no finite extent, and
/// then nothing changes.
fn mirror_axis(layout: &mut LayoutData, axis: Axis) -> bool {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for node in layout.nodes.iter().filter(|node| !is_group(node)) {
        if let Some(value) = axis_of(node, axis) {
            min = js_min(min, value);
            max = js_max(max, value);
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return false;
    }
    let mirror = move |value: f64| min + max - value;

    for node in &mut layout.nodes {
        if let Some(value) = axis_of(node, axis) {
            set_axis_of(node, axis, mirror(value));
        }
        if let Some(title) = node.group_title_rect {
            node.group_title_rect = Some(match axis {
                Axis::X => Rect {
                    left: mirror(title.right),
                    right: mirror(title.left),
                    ..title
                },
                Axis::Y => Rect {
                    top: mirror(title.bottom),
                    bottom: mirror(title.top),
                    ..title
                },
            });
        }
    }
    for edge in &mut layout.edges {
        for point in edge.points.iter_mut().flatten() {
            let coordinate = match axis {
                Axis::X => &mut point.x,
                Axis::Y => &mut point.y,
            };
            *coordinate = mirror(*coordinate);
        }
    }
    true
}

/// `applyBtDirectionTransform(layout)` (`lrTransform.ts:141-148`): `BT` mirrors y. A layout without content
/// nodes is left as it is, and reported as transformed.
pub fn apply_bt_direction_transform(layout: &mut LayoutData) -> bool {
    if !layout.nodes.iter().any(|node| !is_group(node)) {
        return true;
    }
    mirror_axis(layout, Axis::Y)
}

/// One lane of the `LR` and `RL` transform, with the vertical extent of its children.
#[derive(Debug, Clone, Copy)]
struct LaneBound {
    lane: usize,
    content_top: f64,
    content_bottom: f64,
    center_y: f64,
}

/// `applyLrDirectionTransform(layout, direction)` (`lrTransform.ts:150-327`): `LR` and `RL`. Each content node
/// and each edge point takes the rank as x and the order as y, scaled by the mean shape of the content. The
/// lanes then become the bands of the lanes, and `RL` mirrors x. `false` when the content has no finite
/// position, and then the nodes are not changed.
pub fn apply_lr_direction_transform(layout: &mut LayoutData, direction: Direction) -> bool {
    let content: Vec<usize> = (0..layout.nodes.len())
        .filter(|&index| !is_group(&layout.nodes[index]))
        .collect();

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    for &index in &content {
        let node = &layout.nodes[index];
        let x0 = node.x.unwrap_or(0.0);
        let y0 = node.y.unwrap_or(0.0);
        if x0 < min_x {
            min_x = x0;
        }
        if y0 < min_y {
            min_y = y0;
        }
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        return false;
    }

    let mut total_width = 0.0;
    let mut total_height = 0.0;
    for &index in &content {
        let node = &layout.nodes[index];
        total_width += node.width.unwrap_or(0.0);
        total_height += node.height.unwrap_or(0.0);
    }
    let avg_width = total_width / content.len() as f64;
    let avg_height = total_height / content.len() as f64;
    let horizontal_scale_factor = if avg_height > 0.0 {
        js_max(1.0, avg_width / avg_height)
    } else {
        1.0
    };

    for &index in &content {
        let node = &mut layout.nodes[index];
        let x0 = node.x.unwrap_or(0.0);
        let y0 = node.y.unwrap_or(0.0);
        node.x = Some((y0 - min_y) * horizontal_scale_factor + TITLE_BAND_SIZE);
        node.y = Some(x0 - min_x);
    }

    for edge in &mut layout.edges {
        for point in edge.points.iter_mut().flatten() {
            let x0 = point.x;
            let y0 = point.y;
            *point = Point {
                x: (y0 - min_y) * horizontal_scale_factor + TITLE_BAND_SIZE,
                y: x0 - min_x,
            };
        }
    }

    recompute_nested_group_bounds(&mut layout.nodes);

    let lane_nodes: Vec<usize> = (0..layout.nodes.len())
        .filter(|&index| {
            let node = &layout.nodes[index];
            is_group(node) && truthy_text(node.parent_id.as_deref()).is_none()
        })
        .collect();
    if lane_nodes.is_empty() {
        if direction == Direction::RL {
            mirror_axis(layout, Axis::X);
        }
        return true;
    }

    let by_id = node_index(&layout.nodes);
    let mut children_by_lane: IndexMap<String, Vec<usize>> = IndexMap::new();
    for (index, node) in layout.nodes.iter().enumerate() {
        if is_group(node) {
            continue;
        }
        let Some(lane_id) = resolve_top_level_group_id(node, &layout.nodes, &by_id) else {
            continue;
        };
        if lane_id.is_empty() {
            continue;
        }
        children_by_lane.entry(lane_id).or_default().push(index);
    }

    let mut max_pad: f64 = 0.0;
    for &lane in &lane_nodes {
        let pad = layout.nodes[lane].padding.unwrap_or(0.0);
        if pad > max_pad {
            max_pad = pad;
        }
    }

    let mut lane_bounds: Vec<LaneBound> = Vec::new();
    let mut global_min_x_child = f64::INFINITY;
    let mut global_max_x_child = f64::NEG_INFINITY;
    for &lane in &lane_nodes {
        let children: &[usize] = children_by_lane
            .get(&layout.nodes[lane].id)
            .map_or(&[], Vec::as_slice);
        let Some(bounds) = bounds_for_children(children.iter().map(|&index| &layout.nodes[index]))
        else {
            continue;
        };
        global_min_x_child = js_min(global_min_x_child, bounds.min_x);
        global_max_x_child = js_max(global_max_x_child, bounds.max_x);
        lane_bounds.push(LaneBound {
            lane,
            content_top: bounds.min_y,
            content_bottom: bounds.max_y,
            center_y: (bounds.min_y + bounds.max_y) / 2.0,
        });
    }

    if global_min_x_child == f64::INFINITY || global_max_x_child == f64::NEG_INFINITY {
        return true;
    }

    let full_content_width = js_max(0.0, global_max_x_child - global_min_x_child);
    let horizontal_margin = js_max(max_pad, MIN_HORIZONTAL_MARGIN);
    let body_width = full_content_width + 2.0 * horizontal_margin;
    let lane_width = TITLE_BAND_SIZE + body_width;
    let body_center = (global_min_x_child + global_max_x_child) / 2.0;
    let body_left = body_center - body_width / 2.0;
    let lane_left = body_left - TITLE_BAND_SIZE;
    let center_x = lane_left + lane_width / 2.0;
    let vertical_margin = js_max(max_pad, TITLE_BAND_SIZE);

    js_sort_by(&mut lane_bounds, |a, b| a.center_y - b.center_y);

    let count = lane_bounds.len();
    for (index, curr) in lane_bounds.iter().enumerate() {
        let lane_top = if index == 0 {
            curr.content_top - vertical_margin
        } else {
            (lane_bounds[index - 1].content_bottom + curr.content_top) / 2.0
        };
        let lane_bottom = if index + 1 == count {
            curr.content_bottom + vertical_margin
        } else {
            (curr.content_bottom + lane_bounds[index + 1].content_top) / 2.0
        };
        let lane_height = js_max(0.0, lane_bottom - lane_top);
        let center_y = (lane_top + lane_bottom) / 2.0;

        let lane = &mut layout.nodes[curr.lane];
        lane.x = Some(center_x);
        lane.y = Some(center_y);
        lane.width = Some(lane_width);
        lane.height = Some(lane_height);
        lane.swimlane_content_top = Some(curr.content_top);
        lane.group_title_rect = Some(Rect {
            left: lane_left,
            right: lane_left + TITLE_BAND_SIZE,
            top: lane_top,
            bottom: lane_bottom,
        });
    }

    if direction == Direction::RL {
        mirror_axis(layout, Axis::X);
    }
    true
}
