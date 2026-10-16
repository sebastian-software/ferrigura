//! The subgraph tree of the ELK graph: a port of `elk/find-common-ancestor.ts` and
//! `elk/subgraphFeedbackEdges.ts` in `packages/mermaid/src/rendering-util/layout-algorithms`.
//!
//! The tree is the parent lookup of [`super::elk_build::ElkState`]: `parent_by_id` maps a node to
//! its subgraph, and a node without an entry sits at the root. See
//! `docs-port/flowchart/render-notes/elk-build.md` §4.2.

use std::collections::HashMap;

/// The root of the tree, as the reference names it when no subgraph contains a node.
const ROOT: &str = "root";

/// The lowest common ancestor of two nodes of the tree, or `root` when they share none.
///
/// The reference's `findCommonAncestor`. A node is its own ancestor here, so `id1` is returned
/// when `id2` is one of its ancestors; a node with itself as both ends returns its parent.
#[must_use]
pub fn find_common_ancestor(
    id1: &str,
    id2: &str,
    parent_by_id: &HashMap<String, String>,
) -> String {
    let parent = |id: &str| {
        parent_by_id
            .get(id)
            .map(String::as_str)
            .filter(|p| !p.is_empty())
    };
    // Edge case with self edges.
    if id1 == id2 {
        return parent(id1).unwrap_or(ROOT).to_owned();
    }
    let mut visited: Vec<&str> = Vec::new();
    let mut current = Some(id1);
    while let Some(id) = current {
        visited.push(id);
        if id == id2 {
            return id.to_owned();
        }
        current = parent(id);
    }
    let mut current = Some(id2);
    while let Some(id) = current {
        if visited.contains(&id) {
            return id.to_owned();
        }
        current = parent(id);
    }
    ROOT.to_owned()
}

/// The child of `ancestor_id` on the path from `id` up to it, or `id` itself when it is a child.
fn child_on_path(id: &str, ancestor_id: &str, parent_by_id: &HashMap<String, String>) -> String {
    let mut current = id;
    loop {
        let up = parent_by_id.get(current).map_or(ROOT, String::as_str);
        if up == ancestor_id {
            return current.to_owned();
        }
        match parent_by_id.get(current) {
            Some(parent) => current = parent,
            // Not below `ancestor_id`: the reference would walk off the tree here.
            None => return current.to_owned(),
        }
    }
}

/// An edge of one level of the subgraph tree: its index in the graph, and the children of the
/// level's lowest common subgraph that its ends lie in.
struct LevelEdge {
    index: usize,
    from: String,
    to: String,
    collapsed: bool,
}

/// Depth-first search states of a level.
const ON_STACK: u8 = 1;
const DONE: u8 = 2;

/// Finds the edges that close a cycle only because a subgraph is laid out as one node.
///
/// The reference's `findSubgraphFeedbackEdges`. `edges` are the `(source, target)` pairs of the
/// ELK edges in declaration order; the result has one flag per edge, true where the edge should be
/// laid out reversed. Only feedback edges with an end inside a collapsed subgraph are flagged, so a
/// cycle between plain nodes is left to ELK. Self loops and edges between a subgraph and its own
/// members are never flagged.
#[must_use]
pub fn find_subgraph_feedback_edges(
    edges: &[(&str, &str)],
    parent_by_id: &HashMap<String, String>,
) -> Vec<bool> {
    let mut flags = vec![false; edges.len()];
    // The levels in the order the reference's Map yields them: by first edge.
    let mut level_order: Vec<String> = Vec::new();
    let mut levels: HashMap<String, Vec<LevelEdge>> = HashMap::new();

    for (index, &(source, target)) in edges.iter().enumerate() {
        if source == target {
            continue;
        }
        let ancestor = find_common_ancestor(source, target, parent_by_id);
        if ancestor == source || ancestor == target {
            // An edge between a subgraph and its own member crosses no boundary of the parent graph.
            continue;
        }
        let from = child_on_path(source, &ancestor, parent_by_id);
        let to = child_on_path(target, &ancestor, parent_by_id);
        let collapsed = from != source || to != target;
        if !levels.contains_key(&ancestor) {
            level_order.push(ancestor.clone());
        }
        levels.entry(ancestor).or_default().push(LevelEdge {
            index,
            from,
            to,
            collapsed,
        });
    }

    for ancestor in &level_order {
        let level = &levels[ancestor];
        // The nodes of the level in first-appearance order, and the successors of each node.
        let mut nodes: Vec<&str> = Vec::new();
        let mut successors: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for (position, edge) in level.iter().enumerate() {
            for node in [edge.from.as_str(), edge.to.as_str()] {
                if !successors.contains_key(node) {
                    successors.insert(node, Vec::new());
                    nodes.push(node);
                    in_degree.insert(node, 0);
                }
            }
            successors
                .get_mut(edge.from.as_str())
                .expect("the source was inserted above")
                .push(position);
            *in_degree
                .get_mut(edge.to.as_str())
                .expect("the target was inserted above") += 1;
        }

        let mut state: HashMap<&str, u8> = HashMap::new();
        let roots: Vec<&str> = nodes
            .iter()
            .copied()
            .filter(|node| in_degree[node] == 0)
            .chain(nodes.iter().copied())
            .collect();
        for root in roots {
            if state.contains_key(root) {
                continue;
            }
            state.insert(root, ON_STACK);
            // (node, index of the next successor to visit)
            let mut stack: Vec<(&str, usize)> = vec![(root, 0)];
            while let Some(&(node, next)) = stack.last() {
                let out = &successors[node];
                if next >= out.len() {
                    state.insert(node, DONE);
                    stack.pop();
                    continue;
                }
                stack.last_mut().expect("the frame is on the stack").1 += 1;
                let edge = &level[out[next]];
                match state.get(edge.to.as_str()) {
                    Some(&ON_STACK) => flags[edge.index] = edge.collapsed,
                    Some(_) => {}
                    None => {
                        state.insert(edge.to.as_str(), ON_STACK);
                        stack.push((edge.to.as_str(), 0));
                    }
                }
            }
        }
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the parent lookup the renderer derives from `node.parentId`.
    fn tree(parents: &[(&str, &str)]) -> HashMap<String, String> {
        parents
            .iter()
            .map(|(child, parent)| ((*child).to_owned(), (*parent).to_owned()))
            .collect()
    }

    fn edges<'a>(pairs: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
        pairs.to_vec()
    }

    mod find_subgraph_feedback_edges {
        use super::*;

        #[test]
        fn flags_the_edge_that_re_enters_a_subgraph_from_a_node_fed_by_it() {
            // A -> B inside G, then B -> X -> C with C inside G: G -> X -> G is a cycle once G is collapsed
            let flags = find_subgraph_feedback_edges(
                &edges(&[("S", "A"), ("A", "B"), ("B", "X"), ("X", "C")]),
                &tree(&[("A", "G"), ("B", "G"), ("C", "G")]),
            );
            assert_eq!(flags, vec![false, false, false, true]);
        }

        #[test]
        fn leaves_a_cycle_between_plain_nodes_to_elk() {
            let flags = find_subgraph_feedback_edges(
                &edges(&[("A", "B"), ("B", "C"), ("C", "A")]),
                &tree(&[]),
            );
            assert_eq!(flags, vec![false, false, false]);
        }

        #[test]
        fn leaves_a_cycle_inside_one_subgraph_to_elk() {
            let flags = find_subgraph_feedback_edges(
                &edges(&[("A", "B"), ("B", "A")]),
                &tree(&[("A", "G"), ("B", "G")]),
            );
            assert_eq!(flags, vec![false, false]);
        }

        #[test]
        fn flags_nothing_when_the_collapsed_graph_is_acyclic() {
            let flags = find_subgraph_feedback_edges(
                &edges(&[("S", "A"), ("A", "B"), ("B", "X"), ("S", "C")]),
                &tree(&[("A", "G"), ("B", "G"), ("C", "G")]),
            );
            assert_eq!(flags, vec![false, false, false, false]);
        }

        #[test]
        fn looks_at_each_edge_on_the_graph_of_its_lowest_common_subgraph() {
            // inside OUTER: G -> Y -> G is a cycle once G is collapsed; the root level sees none
            let flags = find_subgraph_feedback_edges(
                &edges(&[("S", "A"), ("A", "Y"), ("Y", "B")]),
                &tree(&[("A", "G"), ("B", "G"), ("G", "OUTER"), ("Y", "OUTER")]),
            );
            assert_eq!(flags, vec![false, false, true]);
        }

        #[test]
        fn ignores_edges_between_a_subgraph_and_its_own_members_and_self_loops() {
            let flags = find_subgraph_feedback_edges(
                &edges(&[("G", "A"), ("A", "G"), ("A", "A")]),
                &tree(&[("A", "G")]),
            );
            assert_eq!(flags, vec![false, false, false]);
        }

        #[test]
        fn leaves_a_cycle_through_a_subgraph_to_elk_when_a_plain_edge_closes_it() {
            // X -> G -> Y -> X once G is collapsed, closed by the plain Y -> X.
            let flags = find_subgraph_feedback_edges(
                &edges(&[("S", "X"), ("X", "B"), ("A", "Y"), ("Y", "X")]),
                &tree(&[("A", "G"), ("B", "G")]),
            );
            assert_eq!(flags, vec![false, false, false, false]);
        }

        #[test]
        fn keeps_the_earlier_declared_direction_when_two_subgraphs_feed_each_other() {
            let flags = find_subgraph_feedback_edges(
                &edges(&[("A", "C"), ("D", "B")]),
                &tree(&[("A", "G"), ("B", "G"), ("C", "H"), ("D", "H")]),
            );
            assert_eq!(flags, vec![false, true]);
        }
    }

    mod find_common_ancestor {
        use super::*;

        #[test]
        fn returns_the_parent_of_a_node_with_itself_as_both_ends() {
            let parents = tree(&[("A", "G")]);
            assert_eq!(find_common_ancestor("A", "A", &parents), "G");
            assert_eq!(find_common_ancestor("X", "X", &parents), "root");
        }

        #[test]
        fn returns_the_shared_subgraph_or_root() {
            let parents = tree(&[
                ("A", "G"),
                ("B", "G"),
                ("G", "OUTER"),
                ("C", "OUTER"),
                ("D", "H"),
            ]);
            assert_eq!(find_common_ancestor("A", "B", &parents), "G");
            assert_eq!(find_common_ancestor("A", "C", &parents), "OUTER");
            assert_eq!(find_common_ancestor("A", "D", &parents), "root");
        }

        #[test]
        fn returns_an_ancestor_that_contains_the_other_end() {
            let parents = tree(&[("A", "G"), ("G", "OUTER")]);
            assert_eq!(find_common_ancestor("A", "G", &parents), "G");
            assert_eq!(find_common_ancestor("G", "A", &parents), "G");
        }
    }
}
