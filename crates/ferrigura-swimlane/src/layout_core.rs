//! The seam of the swimlane layout: `runSwimlaneLayoutCore` (`layoutCore.ts`, lines 9-58).
//!
//! The core reads the configuration with the reference's fallbacks, refuses the non-default options and the
//! ids outside printable ASCII, runs the Sugiyama phases, writes the result back, routes the edges, and
//! post-processes them.

use crate::error::Error;
use crate::helpers::{to_graph_view, write_back_to_layout_data};
use crate::model::{Direction, LayoutData};
use crate::pipeline::{LayoutOptions, check_default_path, sugiyama_layout};
use crate::post_processing::post_process_swimlane_layout;
use crate::router::route_edges_orthogonal;

/// `flowchart.nodeSpacing ?? 40` (`layoutCore.ts:22`). The schema always gives the key, so the fallback is
/// not reached in a normal configuration.
const FALLBACK_NODE_SPACING: f64 = 40.0;
/// `flowchart.rankSpacing ?? 100` (`layoutCore.ts:23`).
const FALLBACK_RANK_SPACING: f64 = 100.0;

/// Lays out the swimlane diagram of `data` on the default path, and returns its direction.
///
/// The phases position the nodes, and the write-back stores their result in `data`. The edge points are
/// deleted before the edges are routed, so the routing is the only source of edge geometry. On an error, the
/// layout holds whatever the phases wrote before the error, and it is not a finished layout.
///
/// `optimizeRanksByCrossings` is read by the reference only on the non-default layering, so the core does
/// not read it.
///
/// # Errors
///
/// [`Error::NonDefaultOption`] for `ignoreCrossLaneEdges: false` or `automaticLaneOrdering: true`;
/// [`Error::NonAsciiId`] and [`Error::NotPrintableAscii`] for an id the collation is not ported for;
pub fn run_swimlane_layout_core(data: &mut LayoutData) -> Result<Direction, Error> {
    let node_gap = data
        .config
        .flowchart
        .as_ref()
        .and_then(|section| section.node_spacing)
        .unwrap_or(FALLBACK_NODE_SPACING);
    let layer_gap = data
        .config
        .flowchart
        .as_ref()
        .and_then(|section| section.rank_spacing)
        .unwrap_or(FALLBACK_RANK_SPACING);
    let ignore_cross_lane_edges = data
        .config
        .swimlane
        .as_ref()
        .and_then(|section| section.ignore_cross_lane_edges)
        .unwrap_or(true);
    let automatic_lane_ordering = data
        .config
        .swimlane
        .as_ref()
        .and_then(|section| section.automatic_lane_ordering)
        .unwrap_or(false);
    // `getSwimlaneDirection` takes the layout data's direction as it is, `TB` when there is none
    // (`layoutCore.ts` 10-13), and every pass compares it with one of the four names, so another
    // string, such as the `BR` that the flowchart parser keeps as written, lays out as `TB`
    // (`postProcessing.ts` 51 and 59, `phase4.coordinates.ts` 22, `router.ts` 158,
    // `materializedGeometry.ts` 759 and 887).
    let direction = data
        .direction
        .as_deref()
        .and_then(Direction::parse)
        .unwrap_or(Direction::TB);

    check_default_path(ignore_cross_lane_edges, automatic_lane_ordering)?;
    check_ids(data)?;

    let g = to_graph_view(data);
    let result = sugiyama_layout(
        &g,
        &LayoutOptions {
            node_gap,
            layer_gap,
            ignore_cross_lane_edges,
            automatic_lane_ordering,
            direction,
        },
    )?;
    write_back_to_layout_data(
        &g,
        data,
        &result.ordered,
        &result.coordinates,
        layer_gap,
        node_gap,
    );

    // The phases position nodes only and emit no edge routing. The points carried on the input are deleted,
    // so that the routing below is the single source of the edge geometry.
    for edge in &mut data.edges {
        edge.points = None;
    }
    route_edges_orthogonal(data, direction)?;

    for edge in &mut data.edges {
        if matches!(edge.curve.as_deref(), None | Some("") | Some("basis")) {
            edge.curve = Some("rounded".to_owned());
        }
    }

    post_process_swimlane_layout(data, direction)?;

    // `validateSwimlanesLayout` (`direction/validation.ts`) is not called here. The reference calls it
    // after the post-processing, and it only reports issues and logs them: it changes no field of the
    // layout, so it has no effect on the output. Its port belongs with the geometry passes.
    Ok(direction)
}

/// The ids of the diagram that the layout sorts or compares: the node ids, the edge ids and their
/// endpoints, and the label node ids. Each must be printable ASCII, the range the collation is ported for.
fn check_ids(data: &LayoutData) -> Result<(), Error> {
    let ids = data
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .chain(data.edges.iter().flat_map(|edge| {
            [
                Some(edge.id.as_str()),
                edge.start.as_deref(),
                edge.end.as_deref(),
                edge.label_node_id.as_deref(),
            ]
            .into_iter()
            .flatten()
        }));
    for id in ids {
        if !id.is_ascii() {
            return Err(Error::NonAsciiId(id.to_owned()));
        }
        if id.bytes().any(|byte| !(0x20..=0x7e).contains(&byte)) {
            return Err(Error::NotPrintableAscii(id.to_owned()));
        }
    }
    Ok(())
}
