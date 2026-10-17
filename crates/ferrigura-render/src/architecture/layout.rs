//! The two layout runs of an architecture diagram, as `layoutArchitecture` runs them
//! (`architectureRenderer.ts` 375-608 and 513-601).
//!
//! Run 1 lays out the scene as it was added. Its `layoutstop` handler then sets the weight and the
//! distance of the one segment of every edge whose ends differ in both coordinates, from the positions
//! run 1 gave (513-579); the drawing uses them only for XY edges. Run 2 lays
//! out again from those positions, with the bounds that now include the bends (580). Each run is
//! [`fcose::layout`] with the seed of `withSeededRandom`, which restarts for each run.

use ferrigura_cose::fcose::{self, Edge as FcoseEdge, Input, Node as FcoseNode, Options};
use ferrigura_jsmath::pow;

use super::constraints::Constraints;
use super::geometry::Box2;
use super::scene::{Point, Scene};
use crate::RenderError;

/// The configuration of the layout runs: the values of `architectureRenderer.ts` 478-510.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// `randomize`: the spectral start of fcose.
    pub randomize: bool,
    /// `nodeSeparation`, read by the spectral start.
    pub node_separation: f64,
    /// `numIter`: the iteration limit of the spring embedder.
    pub num_iter: f64,
    /// `seed`: the seed of the generator of each run.
    pub seed: f64,
    /// `iconSize`.
    pub icon_size: f64,
    /// `idealEdgeLengthMultiplier`: the ideal length of an edge inside a group is this times `iconSize`.
    pub ideal_edge_length_multiplier: f64,
    /// `edgeElasticity`: the elasticity of an edge inside a group.
    pub edge_elasticity: f64,
}

/// `ToUint32` of the seed, as `withSeededRandom` takes it (`seed >>> 0`). A seed of 0 means the real
/// `Math.random` in the reference, which is not reproducible, so it is not a seed here.
fn seed_of(seed: f64) -> Result<u32, RenderError> {
    let value = if seed.is_finite() {
        let modulus = 4_294_967_296.0_f64;
        let truncated = seed.trunc();
        ((truncated % modulus) + modulus) % modulus
    } else {
        0.0
    };
    let seed = value as u32;
    if seed == 0 {
        return Err(RenderError::Unsupported(
            "a seed of 0 makes the reference draw from Math.random, which is not reproducible"
                .to_owned(),
        ));
    }
    Ok(seed)
}

/// `layoutDimensions` with `nodeDimensionsIncludeLabels: false`: a size of zero, in either axis, is
/// one by one.
fn layout_dimensions(width: f64, height: f64) -> (f64, f64) {
    if width == 0.0 || height == 0.0 {
        (1.0, 1.0)
    } else {
        (width, height)
    }
}

/// The input of one run, read from the scene as `fcoseRunInput` reads cytoscape (`architectureRenderer.ts`
/// 384-510): the nodes with their dimensions and positions, the edges with their ideal lengths and
/// elasticities, the constraints, and the bounds of the elements.
fn run_input(scene: &Scene, constraints: &Constraints, settings: &Settings, seed: u32) -> Input {
    let nodes = scene
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let (width, height) = scene.outer_size(index);
            let (width, height) = layout_dimensions(width, height);
            FcoseNode {
                id: node.id.clone(),
                parent: node.parent.map(|parent| scene.nodes[parent].id.clone()),
                width,
                height,
                padding: scene.padding(index),
                x: node.position.0,
                y: node.position.1,
            }
        })
        .collect();
    let same_group_length = settings.ideal_edge_length_multiplier * settings.icon_size;
    let cross_group_length = 0.5 * settings.icon_size;
    let edges = scene
        .edges
        .iter()
        .map(|edge| {
            let same = scene.nodes[edge.source].parent == scene.nodes[edge.target].parent;
            FcoseEdge {
                id: edge.id.clone(),
                source: scene.nodes[edge.source].id.clone(),
                target: scene.nodes[edge.target].id.clone(),
                ideal_length: if same {
                    same_group_length
                } else {
                    cross_group_length
                },
                elasticity: if same {
                    settings.edge_elasticity
                } else {
                    0.001
                },
            }
        })
        .collect();
    let bounds = scene.elements_box();
    Input {
        nodes,
        edges,
        alignment: Some(constraints.alignment.clone()),
        relative: constraints.relative.clone(),
        bounds: fcose::Bounds {
            x1: bounds.x1,
            y1: bounds.y1,
            w: bounds.w(),
            h: bounds.h(),
        },
        seed,
        options: Options {
            randomize: settings.randomize,
            node_separation: settings.node_separation,
            num_iter: settings.num_iter,
        },
    }
}

/// One run of fcose on the scene: the position of every node, in the scene's order, and the bounds
/// the run starts from.
fn run(
    scene: &Scene,
    constraints: &Constraints,
    settings: &Settings,
    seed: u32,
) -> Result<(Vec<Point>, Box2), RenderError> {
    let input = run_input(scene, constraints, settings, seed);
    let bounds = scene.elements_box();
    let positions =
        fcose::layout(&input).map_err(|error| RenderError::Layout(error.to_string()))?;
    Ok((
        positions
            .iter()
            .map(|position| (position.x, position.y))
            .collect(),
        bounds,
    ))
}

/// `getSegmentWeights` (architectureRenderer.ts 533-575): the distance and the weight of the bend of
/// an edge, from its ends and the corner that the two ends and the point (`point_x`, `point_y`) make.
#[must_use]
pub fn segment_weights(source: Point, target: Point, point_x: f64, point_y: f64) -> (f64, f64) {
    let (sx, sy) = source;
    let (tx, ty) = target;
    let mut distance = (point_y - sy + ((sx - point_x) * (sy - ty)) / (sx - tx))
        / (1.0 + pow((sy - ty) / (sx - tx), 2.0)).sqrt();
    let mut weight = (pow(point_y - sy, 2.0) + pow(point_x - sx, 2.0) - pow(distance, 2.0)).sqrt();
    let dist_ab = (pow(tx - sx, 2.0) + pow(ty - sy, 2.0)).sqrt();
    weight /= dist_ab;
    // The side of the line that the corner is on, and whether it is behind the source: JavaScript's
    // `switch (true)` leaves a NaN unchanged.
    let sign = |value: f64| {
        if value >= 0.0 {
            1.0
        } else if value < 0.0 {
            -1.0
        } else {
            value
        }
    };
    let delta1 = sign((tx - sx) * (point_y - sy) - (ty - sy) * (point_x - sx));
    let delta2 = sign((tx - sx) * (point_x - sx) + (ty - sy) * (point_y - sy));
    distance = distance.abs() * delta1;
    weight *= delta2;
    (distance, weight)
}

/// The handler of run 1's `layoutstop` (architectureRenderer.ts 513-580): every edge whose ends differ
/// in both coordinates gets the segment weight and distance of its bend.
fn set_segments(scene: &mut Scene) {
    for index in 0..scene.edges.len() {
        let edge = &scene.edges[index];
        let source_position = scene.nodes[edge.source].position;
        let target_position = scene.nodes[edge.target].position;
        if source_position.0 == target_position.0 || source_position.1 == target_position.1 {
            continue;
        }
        // `isArchitectureDirectionY` of the source side.
        let vertical = !edge.source_dir.is_horizontal();
        let geometry = scene.edge_geometry(index);
        let source_end = geometry.arrow_start;
        let target_end = geometry.arrow_end;
        let point = if vertical {
            (source_end.0, target_end.1)
        } else {
            (target_end.0, source_end.1)
        };
        let (distance, weight) = segment_weights(source_end, target_end, point.0, point.1);
        let edge = &mut scene.edges[index];
        edge.distance = distance;
        edge.weight = weight;
    }
}

/// Lays out the scene: run 1, the segment handler, and run 2 (`layoutArchitecture`). Returns the bounds
/// that each run starts from, as fcose reads them.
///
/// # Errors
///
/// Returns an error if the seed is 0, or if the layout engine refuses the input.
pub fn lay_out(
    scene: &mut Scene,
    constraints: &Constraints,
    settings: &Settings,
) -> Result<[Box2; 2], RenderError> {
    let seed = seed_of(settings.seed)?;
    let (first, first_bounds) = run(scene, constraints, settings, seed)?;
    scene.set_leaf_positions(&first);
    set_segments(scene);
    let (second, second_bounds) = run(scene, constraints, settings, seed)?;
    scene.set_leaf_positions(&second);
    Ok([first_bounds, second_bounds])
}
