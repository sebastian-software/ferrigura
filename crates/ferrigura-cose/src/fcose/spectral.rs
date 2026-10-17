//! The spectral start of cytoscape-fcose 2.2.0 (`src/fcose/spectral.js`, 427 lines), used when the
//! run has `randomize: true`. It gives each node a starting position for cose's incremental
//! spring embedder (`cose.js` 67 and 71 read `xCoords[nodeIndexes.get(id)]`).
//!
//! **What it runs on.** `spectralLayout(options)` takes `options.eles`. Mermaid gives no `eles`,
//! so cytoscape uses every element of the instance (`core/layout.mjs` 31, `cy.$()`). The input
//! is therefore every node and every edge, compound nodes included, in cytoscape's pool order
//! (`cy.nodes()` and `cy.edges()`). A compound node has no index of its own: `nodeIndexes` holds
//! the childless nodes and the dummy nodes (spectral.js 300-309). A compound takes the position of
//! its representative leaf (`parentChildMap`, spectral.js 317-336).
//!
//! **Neighbours.** A neighbour is a node joined to a node by an edge (`neighborhood()`,
//! `collection/traversing.mjs` 142-169). A compound's neighbours are the nodes joined to the
//! compound itself. Its descendants do not count, and a compound is not a neighbour of its own
//! children.
//!
//! **Dummy nodes.** `connectComponents` (`auxiliary.js` 33-131) does not add cytoscape elements.
//! It records the nodes that link the components of a graph. Each record becomes a node after the
//! childless nodes, with the key `dummy1`, `dummy2`, ... (auxiliary.js 129, spectral.js 307-309).
//! The result has one index per dummy node, after the childless nodes.
//!
//! **Draws of `Math.random`.** With greedy sampling (`samplingType: true`, the default), the run
//! draws once for the first sample (spectral.js 135), then twice per childless or dummy node in
//! the power iteration (spectral.js 210-211). Random sampling draws until it has `sampleSize`
//! distinct samples (spectral.js 46-68). The draws use [`crate::fcose::rng::Mulberry32`] in
//! the same order.
//!
//! **Mermaid's options.** Mermaid sets `quality: 'proof'` (architectureRenderer.ts 588) and keeps
//! the default `step: 'all'` (cytoscape-fcose index.js 37), so `full_layout` is true. Its
//! `sampleSize` is the default 25 (index.js 44), `piTol` is 1e-7 (index.js 48), and
//! `samplingType` is true (index.js 42). `nodeSeparation` is the renderer's config.
//!
//! **Maps.** The source's `Map`s and `Set`s iterate in insertion order. The code keeps the same
//! order in vectors: the childless nodes, then the dummy nodes, with indexes in that order.

// The index loops follow the source's loops (spectral.js, auxiliary.js), so the iterator forms
// would hide the correspondence.
#![allow(clippy::needless_range_loop)]

use std::collections::{HashSet, VecDeque};

use crate::fcose::Error;
use crate::fcose::linalg::{
    Svd, dot_product, minus_op, mult_cons, mult_gamma, mult_l, mult_mat, normalize, svd, transpose,
};
use crate::fcose::rng::Mulberry32;

/// `const infinity = 100000000` (spectral.js 33). It is a finite value, not `Infinity`.
const DISTANCE_INFINITY: f64 = 100_000_000.0;
/// `const small = 0.000000001` (spectral.js 34): the starting `previous` of the power iteration.
const SMALL: f64 = 0.000_000_001;

/// One node of the run, as cytoscape holds it when fcose calls `spectralLayout`.
#[derive(Clone, Debug)]
pub(crate) struct SpectralNode {
    /// The id. Ids are unique (cytoscape refuses duplicates). A dummy node is named `dummyN`, so
    /// a childless node with that id makes the source throw (see [`spectral_layout`]).
    pub(crate) id: String,
    /// The index of the parent in [`SpectralInput::nodes`], or `None` for a top-level node. Whether
    /// a node is a parent (`isParent()`) follows from the input: it has at least one child.
    pub(crate) parent: Option<usize>,
    /// `position('x')` and `position('y')` at the start of the run. Read only for a graph with one
    /// or two childless nodes (spectral.js 408-423) and when `full_layout` is false.
    pub(crate) x: f64,
    pub(crate) y: f64,
}

/// `options.idealEdgeLength` as the two-node branch reads it (spectral.js 419). Mermaid passes a
/// function (architectureRenderer.ts 597-601). The source then adds the function's text to a
/// number, which gives a string, not a number, so the coordinate is NaN here, and cose.js turns it
/// into 0 (cose.js 115-121). A numeric option would give a finite value; no caller passes one, and
/// it is not ported.
#[derive(Clone, Copy, Debug)]
pub(crate) enum IdealEdgeLength {
    /// A function.
    Function,
}

/// The input of the spectral start (see the module comment for what each part means).
#[derive(Clone, Debug)]
pub(crate) struct SpectralInput {
    /// `eles.nodes()`, in cytoscape's order: every node of the run, compound nodes included.
    pub(crate) nodes: Vec<SpectralNode>,
    /// `eles.edges()`, in cytoscape's order, as `(source, target)` indices into `nodes`. The
    /// degrees of the nodes are counted over these edges (`connectedEdges()`, spectral.js 328,
    /// auxiliary.js 104), a self-loop once.
    pub(crate) edges: Vec<(usize, usize)>,
    /// `options.nodeSeparation` (spectral.js 101): the length of one hop in the BFS distances.
    pub(crate) node_separation: f64,
    /// `options.sampleSize`, 25 unless set (index.js 44). Zero makes the source throw.
    pub(crate) sample_size: usize,
    /// `options.piTol`, 1e-7 (index.js 48): the tolerance of the power iteration.
    pub(crate) pi_tol: f64,
    /// `options.samplingType`: true for greedy sampling (spectral.js 133), false for random
    /// sampling (spectral.js 46-68).
    pub(crate) greedy_sampling: bool,
    /// `options.quality == "draft" || options.step == "all"` (spectral.js 393). When false, the
    /// positions of the nodes are the result instead of the spectral layout.
    pub(crate) full_layout: bool,
    /// `options.idealEdgeLength`, read only by the two-node branch (see [`IdealEdgeLength`]).
    pub(crate) ideal_edge_length: IdealEdgeLength,
}

/// The result of the spectral start, as `spectralLayout` returns it.
#[derive(Clone, Debug)]
pub(crate) struct SpectralResult {
    /// For each input node (then each dummy node, in creation order), its index in `x` and `y`.
    /// `None` for a parent: `nodeIndexes` has no entry for it (spectral.js 300-309).
    pub node_index: Vec<Option<usize>>,
    /// `xCoords` and `yCoords`, in the order of the node indexes.
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// The graph as cytoscape holds it for the spectral start, with the orders the source iterates in.
struct Graph {
    /// The parent of each node.
    parent: Vec<Option<usize>>,
    /// The children of each node, in the order they were added (`_private.children`).
    children: Vec<Vec<usize>>,
    /// `isParent()`: the node has at least one child.
    is_parent: Vec<bool>,
    /// The edges that touch each node, in edge order (`_private.edges`).
    incident: Vec<Vec<usize>>,
    /// The edges, as `(source, target)`.
    edges: Vec<(usize, usize)>,
}

impl Graph {
    fn new(input: &SpectralInput) -> Result<Graph, Error> {
        let n = input.nodes.len();
        let mut ids: HashSet<&str> = HashSet::with_capacity(n);
        for node in &input.nodes {
            if !ids.insert(node.id.as_str()) {
                return Err(Error::DuplicateNode(node.id.clone()));
            }
        }
        let mut parent = Vec::with_capacity(n);
        let mut children = vec![Vec::new(); n];
        for (i, node) in input.nodes.iter().enumerate() {
            if let Some(p) = node.parent {
                if p >= n {
                    return Err(Error::Unsupported(
                        "a parent index is not a node of the input",
                    ));
                }
                children[p].push(i);
            }
            parent.push(node.parent);
        }
        // cytoscape refuses a parent cycle (`collection/index.mjs` 513-520 walks the ancestors),
        // so a cycle here is an input the source cannot have.
        for i in 0..n {
            let mut steps = 0;
            let mut up = parent[i];
            while let Some(p) = up {
                steps += 1;
                if steps > n {
                    return Err(Error::Unsupported("the parents of the nodes form a cycle"));
                }
                up = parent[p];
            }
        }
        let mut incident = vec![Vec::new(); n];
        for (e, &(source, target)) in input.edges.iter().enumerate() {
            if source >= n || target >= n {
                return Err(Error::Unsupported(
                    "an edge names a node index out of range",
                ));
            }
            incident[source].push(e);
            // A self-loop is in one list only (`collection/index.mjs` 462-470).
            if target != source {
                incident[target].push(e);
            }
        }
        let is_parent = children.iter().map(|c| !c.is_empty()).collect();
        Ok(Graph {
            parent,
            children,
            is_parent,
            incident,
            edges: input.edges.clone(),
        })
    }

    /// `connectedEdges().length` (`_private.edges`).
    fn degree(&self, node: usize) -> usize {
        self.incident[node].len()
    }

    /// `neighborhood().nodes()`: the other end of each edge of `node`, in edge order, each node
    /// once (`traversing.mjs` 142-169 and `spawn(elements, true)`).
    fn neighbours(&self, node: usize) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for &e in &self.incident[node] {
            let (source, target) = self.edges[e];
            let other = if source == node { target } else { source };
            if !out.contains(&other) {
                out.push(other);
            }
        }
        out
    }

    /// `descendants()`: children first in order, each followed by its own descendants
    /// (`collection/compounds.mjs`, the `add` function of `descendants`).
    fn descendants(&self, node: usize) -> Vec<usize> {
        let mut out = Vec::new();
        self.push_descendants(node, &mut out);
        out
    }

    fn push_descendants(&self, node: usize, out: &mut Vec<usize>) {
        for &child in &self.children[node] {
            out.push(child);
            self.push_descendants(child, out);
        }
    }

    /// `aux.getTopMostNodes` (auxiliary.js 10-31): the nodes of `set` with no ancestor in `set`,
    /// in the order of `set`.
    fn topmost(&self, set: &[usize]) -> Vec<usize> {
        let mut member = vec![false; self.is_parent.len()];
        for &i in set {
            member[i] = true;
        }
        set.iter()
            .copied()
            .filter(|&i| {
                let mut ancestor = self.parent[i];
                while let Some(a) = ancestor {
                    if member[a] {
                        return false;
                    }
                    ancestor = self.parent[a];
                }
                true
            })
            .collect()
    }

    /// The node itself or its nearest ancestor that is marked in `in_top`, as
    /// `topMostNodes.intersection(neighborNode.union(neighborNode.ancestors()))` (auxiliary.js
    /// 74). The marked nodes form an antichain, so at most one of them is on the chain.
    fn top_of(&self, node: usize, in_top: &[bool]) -> Option<usize> {
        let mut current = Some(node);
        while let Some(i) = current {
            if in_top[i] {
                return Some(i);
            }
            current = self.parent[i];
        }
        None
    }

    /// `parentChildMap` (spectral.js 317-336): the childless node that represents a parent. The
    /// search descends through the first parent of each level until a childless node is among the
    /// children, then takes the first childless node with the fewest edges.
    fn representative(&self, parent: usize) -> Result<usize, Error> {
        let missing = || Error::Unsupported("spectral.rs: a parent with no childless descendant");
        let mut level: Vec<usize> = self.children[parent].clone();
        while !level.iter().any(|&c| !self.is_parent[c]) {
            let first = *level.first().ok_or_else(missing)?;
            level = self.children[first].clone();
        }
        let mut best: Option<(usize, usize)> = None;
        for &c in &level {
            if self.is_parent[c] {
                continue;
            }
            let d = self.degree(c);
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, c));
            }
        }
        best.map(|(_, c)| c).ok_or_else(missing)
    }
}

/// `aux.connectComponents` (auxiliary.js 33-131). It finds the components of the top-most nodes
/// `top`, following the edges of the whole subtree of each node. When there is more than one
/// component, it records for each component the node with the fewest edges (`minDegreeNode`).
/// The records of one call form a list, which is appended to `dummies` as one dummy node.
fn connect_components(
    graph: &Graph,
    mut top: Vec<usize>,
    dummies: &mut Vec<Vec<usize>>,
) -> Result<(), Error> {
    let n = graph.is_parent.len();
    let mut visited = vec![false; n];
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut visited_top: Vec<usize> = Vec::new();
    let mut nodes_to_dummy: Vec<usize> = Vec::new();
    let mut is_connected = false;
    let mut count = 1;
    loop {
        let &current = top.first().ok_or(Error::Unsupported(
            "spectral.rs: connectComponents with no top-most node",
        ))?;
        let mut in_top = vec![false; n];
        for &t in &top {
            in_top[t] = true;
        }
        // childrenOfCurrentNode (auxiliary.js 50-56): the node, then its descendants.
        visited_top.push(current);
        for node in std::iter::once(current).chain(graph.descendants(current)) {
            queue.push_back(node);
            visited[node] = true;
        }
        while let Some(node) = queue.pop_front() {
            // neighborNodes (auxiliary.js 66-70) is built before the loop over it.
            for neighbour in graph.neighbours(node) {
                // auxiliary.js 74-76: a topmost node not visited yet takes its whole subtree.
                let Some(t) = graph.top_of(neighbour, &in_top) else {
                    continue;
                };
                if !visited[t] {
                    for member in std::iter::once(t).chain(graph.descendants(t)) {
                        queue.push_back(member);
                        visited[member] = true;
                        if in_top[member] {
                            visited_top.push(member);
                        }
                    }
                }
            }
        }
        if visited_top.len() == top.len() {
            is_connected = true;
        }
        // auxiliary.js 103: each component records once the graph has more than one (`count > 1`),
        // and the first one records too while the graph is not connected (`!isConnected`).
        if !is_connected || count > 1 {
            // auxiliary.js 104-111: the first node with the fewest edges.
            let mut best: Option<(usize, usize)> = None;
            for &node in &visited_top {
                let d = graph.degree(node);
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, node));
                }
            }
            let (_, min_node) = best.ok_or(Error::Unsupported(
                "spectral.rs: connectComponents found no node",
            ))?;
            nodes_to_dummy.push(min_node);
            // auxiliary.js 120: `topMostNodes.difference(temp)`, order kept.
            top.retain(|t| !visited_top.contains(t));
            visited_top.clear();
            count += 1;
        }
        if is_connected {
            break;
        }
    }
    // auxiliary.js 128-130: one dummy per call that recorded anything.
    if !nodes_to_dummy.is_empty() {
        dummies.push(nodes_to_dummy);
    }
    Ok(())
}

/// `spectralLayout` (spectral.js 10-427): the spectral start for the nodes of `input`. It returns
/// one position per childless node and per dummy node.
///
/// # Errors
///
/// `DuplicateNode` for two equal ids, `Unsupported` for an index out of range, a parent cycle,
/// a dummy key that is also a childless node's id (the source then throws a TypeError at
/// spectral.js 357-368, when it indexes past its arrays), a graph with no node, `sampleSize` 0
/// (the source throws in the SVD), and a matrix that is not finite (the source's SVD and power
/// iteration then never end).
pub(crate) fn spectral_layout(
    input: &SpectralInput,
    rng: &mut Mulberry32,
) -> Result<SpectralResult, Error> {
    let graph = Graph::new(input)?;
    let n = graph.is_parent.len();
    // spectral.js 293-296: the top-level nodes, then the top-most descendants of each parent.
    let all: Vec<usize> = (0..n).collect();
    let mut dummies: Vec<Vec<usize>> = Vec::new();
    connect_components(&graph, graph.topmost(&all), &mut dummies)?;
    for parent in 0..n {
        if graph.is_parent[parent] {
            let descendants = graph.descendants(parent);
            connect_components(&graph, graph.topmost(&descendants), &mut dummies)?;
        }
    }

    // spectral.js 299-309: the childless nodes get indices in order, then the dummy nodes.
    let leaves: Vec<usize> = (0..n).filter(|&i| !graph.is_parent[i]).collect();
    let leaf_ids: HashSet<&str> = leaves.iter().map(|&i| input.nodes[i].id.as_str()).collect();
    let mut leaf_pos = vec![0usize; n];
    let mut node_index: Vec<Option<usize>> = vec![None; n + dummies.len()];
    for (pos, &i) in leaves.iter().enumerate() {
        leaf_pos[i] = pos;
        node_index[i] = Some(pos);
    }
    for d in 0..dummies.len() {
        // A key that is already a childless id would overwrite its index (`Map.set`), and the
        // source then indexes past the arrays allocated for `nodeIndexes.size` (spectral.js
        // 313 and 357-368), which throws. The error is the faithful result.
        if leaf_ids.contains(format!("dummy{}", d + 1).as_str()) {
            return Err(Error::Unsupported(
                "a childless node has the id of a dummy node (dummyN): spectral.js throws here",
            ));
        }
        node_index[n + d] = Some(leaves.len() + d);
    }
    let node_size = leaves.len() + dummies.len();

    // spectral.js 317-336: the entry index of each node. A parent's entry is its representative.
    let mut rep: Vec<Option<usize>> = vec![None; n];
    for parent in 0..n {
        if graph.is_parent[parent] {
            rep[parent] = Some(graph.representative(parent)?);
        }
    }
    let entry: Vec<usize> = (0..n)
        .map(|i| match rep[i] {
            Some(r) => leaf_pos[r],
            None => leaf_pos[i],
        })
        .collect();

    // spectral.js 339-355: the neighbour lists. Each node's list is that of its entry.
    let mut all_nb: Vec<Vec<usize>> = vec![Vec::new(); node_size];
    for i in 0..n {
        for neighbour in graph.neighbours(i) {
            all_nb[entry[i]].push(entry[neighbour]);
        }
    }
    // spectral.js 357-368: the dummy nodes join their nodes, in both directions.
    for (d, list) in dummies.iter().enumerate() {
        let key = leaves.len() + d;
        for &id in list {
            let disconnected = entry[id];
            all_nb[key].push(disconnected);
            all_nb[disconnected].push(key);
        }
    }

    if node_size > 2 {
        // spectral.js 381: `nodeSize < options.sampleSize ? nodeSize : options.sampleSize`.
        let sample_size = if node_size < input.sample_size {
            node_size
        } else {
            input.sample_size
        };
        if !input.full_layout {
            // spectral.js 400-405: the positions of the nodes, in the order of the indexes.
            let mut x: Vec<f64> = Vec::with_capacity(node_size);
            let mut y: Vec<f64> = Vec::with_capacity(node_size);
            for &i in &leaves {
                x.push(input.nodes[i].x);
                y.push(input.nodes[i].y);
            }
            // `position()` of a dummy node is undefined, so its coordinates are NaN.
            for _ in 0..dummies.len() {
                x.push(f64::NAN);
                y.push(f64::NAN);
            }
            return Ok(SpectralResult { node_index, x, y });
        }
        if sample_size == 0 {
            return Err(Error::Unsupported(
                "sampleSize 0: the source's SVD reads A[0].length of an empty matrix",
            ));
        }
        let (c, samples) = all_bfs(
            input.greedy_sampling,
            sample_size,
            node_size,
            &all_nb,
            input.node_separation,
            rng,
        );
        // spectral.js 161-165, PHI in `allBFS`: PHI[i][j] = C[samples[j]][i].
        let phi: Vec<Vec<f64>> = (0..sample_size)
            .map(|i| (0..sample_size).map(|j| c[samples[j]][i]).collect())
            .collect();
        if phi.iter().flatten().any(|v| !v.is_finite()) {
            return Err(Error::Unsupported(
                "a node is unreachable from the samples: the spectral SVD never ends (spectral.js 170)",
            ));
        }
        let inv = sample_inverse(phi);
        let (x, y) = power_iteration(&c, &inv, node_size, input.pi_tol, rng)?;
        return Ok(SpectralResult { node_index, x, y });
    }

    // spectral.js 407-423: one or two childless nodes. Their positions and sizes are the result.
    let first = &input.nodes[*leaves.first().ok_or(Error::Unsupported(
        "spectral.rs: no childless node; the source throws at spectral.js 410-411",
    ))?];
    let mut x = vec![first.x];
    let mut y = vec![first.y];
    if node_size == 2 {
        // spectral.js 417-419: the second node is childless, since a dummy node would make three
        // indexes. Its value is the string described at `IdealEdgeLength`.
        if leaves.len() < 2 {
            return Err(Error::Unsupported(
                "spectral.rs: a two-node graph with a dummy node cannot occur",
            ));
        }
        let value = match input.ideal_edge_length {
            IdealEdgeLength::Function => f64::NAN,
        };
        x.push(value);
        y.push(first.y);
    }
    Ok(SpectralResult { node_index, x, y })
}

/// The breadth-first search from `pivot` (spectral.js 73-103). It fills column `index` of `c`
/// with the squared hop distances times `nodeSeparation`. Only the nodes the search reaches are
/// filled; the others stay NaN, which is what the source's `undefined` becomes.
fn bfs_column(
    pivot: usize,
    index: usize,
    c: &mut [Vec<f64>],
    all_nb: &[Vec<usize>],
    node_separation: f64,
) {
    let node_size = all_nb.len();
    let mut distance = vec![DISTANCE_INFINITY; node_size];
    let mut path = vec![pivot];
    let mut front = 0;
    distance[pivot] = 0.0;
    while front < path.len() {
        let current = path[front];
        front += 1;
        for &temp in &all_nb[current] {
            if distance[temp] == DISTANCE_INFINITY {
                distance[temp] = distance[current] + 1.0;
                path.push(temp);
            }
        }
        c[current][index] = distance[current] * node_separation;
    }
}

/// `randomSampleCR` (spectral.js 46-68) and `allBFS` (spectral.js 122-167): the sample nodes and
/// the squared distance columns. Greedy sampling (spectral.js 133-146) takes the node farthest
/// from the samples so far; each step draws nothing after the first. The returned `c` is squared.
fn all_bfs(
    greedy: bool,
    sample_size: usize,
    node_size: usize,
    all_nb: &[Vec<usize>],
    node_separation: f64,
    rng: &mut Mulberry32,
) -> (Vec<Vec<f64>>, Vec<usize>) {
    let mut c = vec![vec![f64::NAN; sample_size]; node_size];
    let mut samples = vec![0usize; sample_size];
    if greedy {
        // spectral.js 135: `Math.floor(Math.random() * nodeSize)`.
        let mut sample = random_index(rng, node_size);
        let mut min_distances = vec![DISTANCE_INFINITY; node_size];
        for i in 0..sample_size {
            samples[i] = sample;
            bfs_column(sample, i, &mut c, all_nb, node_separation);
            // spectral.js 105-118: the closest distance to any sample so far, and the farthest
            // node by that distance. `max_ind` starts at 1 (spectral.js 82).
            for node in 0..node_size {
                if c[node][i] < min_distances[node] {
                    min_distances[node] = c[node][i];
                }
            }
            let mut max_dist = 0.0;
            let mut max_ind = 1;
            for (node, &distance) in min_distances.iter().enumerate() {
                if distance > max_dist {
                    max_dist = distance;
                    max_ind = node;
                }
            }
            sample = max_ind;
        }
    } else {
        let mut count = 0;
        while count < sample_size {
            let candidate = random_index(rng, node_size);
            if !samples[..count].contains(&candidate) {
                samples[count] = candidate;
                count += 1;
            }
        }
        for i in 0..sample_size {
            bfs_column(samples[i], i, &mut c, all_nb, node_separation);
        }
    }
    // spectral.js 152: `C[i][j] *= C[i][j]`.
    for row in c.iter_mut() {
        for value in row.iter_mut() {
            *value = *value * *value;
        }
    }
    (c, samples)
}

/// `Math.floor(Math.random() * size)`, the index of a node (spectral.js 52 and 135).
fn random_index(rng: &mut Mulberry32, size: usize) -> usize {
    (rng.next_f64() * size as f64).floor() as usize
}

/// `sample()` (spectral.js 169-194): the inverse of PHI from its singular value decomposition,
/// with the singular values damped as in the source. `phi` is not empty, and it is finite.
fn sample_inverse(phi: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    let Svd { u, v, s } = svd(phi);
    let sample_size = s.len();
    let max_s = s[0] * s[0] * s[0];
    let mut a_sig = vec![vec![0.0; sample_size]; sample_size];
    for i in 0..sample_size {
        a_sig[i][i] = s[i] / (s[i] * s[i] + max_s / (s[i] * s[i]));
    }
    mult_mat(&mult_mat(&v, &a_sig), &transpose(&u))
}

/// `powerIteration` (spectral.js 197-287): the two leading vectors of `C` with `INV`, found by the
/// power iteration with the mean removed. The second vector is kept orthogonal to the first.
/// The start vectors are drawn in the order of `Y1[i]` then `Y2[i]` (spectral.js 208-212).
fn power_iteration(
    c: &[Vec<f64>],
    inv: &[Vec<f64>],
    node_size: usize,
    pi_tol: f64,
    rng: &mut Mulberry32,
) -> Result<(Vec<f64>, Vec<f64>), Error> {
    let nan_error =
        || Error::Unsupported("the spectral power iteration reached NaN: the source never ends");
    let mut y1 = vec![0.0; node_size];
    let mut y2 = vec![0.0; node_size];
    for (a, b) in y1.iter_mut().zip(y2.iter_mut()) {
        *a = rng.next_f64();
        *b = rng.next_f64();
    }
    let mut y1 = normalize(&y1);
    let mut y2 = normalize(&y2);

    // spectral.js 216-240: the first vector.
    let mut previous = SMALL;
    let mut v1: Vec<f64>;
    let mut theta1: f64;
    loop {
        v1 = y1.clone();
        y1 = mult_gamma(&mult_l(&mult_gamma(&v1), c, inv));
        theta1 = dot_product(&v1, &y1);
        y1 = normalize(&y1);
        let current = dot_product(&v1, &y1);
        // Once `current` is NaN, the vectors stay NaN, so `temp` is NaN in every later iteration
        // and the source's test never passes (spectral.js 239).
        if current.is_nan() {
            return Err(nan_error());
        }
        let temp = (current / previous).abs();
        if temp <= 1.0 + pi_tol && temp >= 1.0 {
            break;
        }
        previous = current;
    }
    v1 = y1.clone();

    // spectral.js 241-268: the second vector, orthogonal to the first.
    previous = SMALL;
    let mut v2: Vec<f64>;
    let mut theta2: f64;
    loop {
        let copy = y2.clone();
        let projection = dot_product(&v1, &copy);
        v2 = minus_op(&copy, &mult_cons(&v1, projection));
        y2 = mult_gamma(&mult_l(&mult_gamma(&v2), c, inv));
        theta2 = dot_product(&v2, &y2);
        y2 = normalize(&y2);
        let current = dot_product(&v2, &y2);
        if current.is_nan() {
            return Err(nan_error());
        }
        let temp = (current / previous).abs();
        if temp <= 1.0 + pi_tol && temp >= 1.0 {
            break;
        }
        previous = current;
    }
    v2 = y2.clone();

    // spectral.js 285-286.
    Ok((
        mult_cons(&v1, theta1.abs().sqrt()),
        mult_cons(&v2, theta2.abs().sqrt()),
    ))
}
