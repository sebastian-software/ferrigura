//! The Sugiyama phases on the default path (`pipeline.ts`, `sugiyamaLayout`, lines 34-77).
//!
//! The phases run in order: cycle removal, the lane-aware layering, the dummy nodes of long edges, the
//! ordering, and the coordinates. The non-default options are refused before any phase runs.
//!
//! Not ported: the gravity layering (`ignoreCrossLaneEdges: false`) and the automatic lane ordering
//! (`automaticLaneOrdering: true`). `optimizeRanksByCrossings` and `compactSingleInput` are read only by the
//! non-default layering, so they are not in [`LayoutOptions`].

use crate::error::Error;
use crate::helpers::{Coordinates, EdgeRef, Graph, Layering, OrderedLayers};
use crate::model::Direction;
use crate::phase1::remove_cycles_dfs;
use crate::phase2_dummies::make_proper_layering;
use crate::phase2_layering::assign_layers_lane_aware_compact;
use crate::phase3::order_layers;
use crate::phase4::assign_coordinates;

/// The options of the Sugiyama phases that the default path reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutOptions {
    /// The gap between nodes of a lane (`nodeGap`).
    pub node_gap: f64,
    /// The gap between layers (`layerGap`).
    pub layer_gap: f64,
    /// `ignoreCrossLaneEdges`. The default path needs `true`.
    pub ignore_cross_lane_edges: bool,
    /// `automaticLaneOrdering`. The default path needs `false`.
    pub automatic_lane_ordering: bool,
    /// The direction of the diagram.
    pub direction: Direction,
}

/// The result of [`sugiyama_layout`] (`LayoutResult`).
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutResult {
    /// `acyclic`: the graph without its back edges, before the dummy nodes.
    pub acyclic: Graph,
    /// `reversed`: the back edges, in their original orientation.
    pub reversed: Vec<EdgeRef>,
    /// `layering`: the layers with the dummy nodes.
    pub layering: Layering,
    /// `ordered`: the nodes of each layer, in order.
    pub ordered: OrderedLayers,
    /// `coordinates`: the centre of each node.
    pub coordinates: Coordinates,
}

/// Refuses the options that the default path does not run, before any work (`layoutCore.ts:24` and
/// `pipeline.ts:38`, `:47`). `ignoreCrossLaneEdges` is checked first, since it selects the layering.
pub(crate) fn check_default_path(
    ignore_cross_lane_edges: bool,
    automatic_lane_ordering: bool,
) -> Result<(), Error> {
    if !ignore_cross_lane_edges {
        return Err(Error::NonDefaultOption("ignoreCrossLaneEdges: false"));
    }
    if automatic_lane_ordering {
        return Err(Error::NonDefaultOption("automaticLaneOrdering: true"));
    }
    Ok(())
}

/// `sugiyamaLayout`: the phases on the graph view of the layout.
pub fn sugiyama_layout(g: &Graph, opts: &LayoutOptions) -> Result<LayoutResult, Error> {
    check_default_path(opts.ignore_cross_lane_edges, opts.automatic_lane_ordering)?;

    // Phase 1: cycle removal. The ordering of the lanes is not automatic, so there is no lane order.
    let cycle = remove_cycles_dfs(g);

    // Phase 2: the lane-aware layering, then the dummy nodes of the edges that span several layers.
    let layering = assign_layers_lane_aware_compact(&cycle.acyclic, opts.direction);
    let (proper_layering, graph_with_dummies) = make_proper_layering(&layering, &cycle.acyclic);

    // Phase 3: the ordering of each layer.
    let ordered = order_layers(&proper_layering, &graph_with_dummies, None);

    // Phase 4: the coordinates.
    let coordinates = assign_coordinates(
        &ordered,
        &graph_with_dummies,
        opts.layer_gap,
        opts.node_gap,
        opts.direction,
        None,
    );

    Ok(LayoutResult {
        acyclic: cycle.acyclic,
        reversed: cycle.reversed,
        layering: proper_layering,
        ordered,
        coordinates,
    })
}
