use std::error::Error;
use std::io;

use ferrigura_tala::normalize::Geometry;
use ferrigura_tala::quality::{Evaluation, PositionedGraph, evaluate};
use ferrigura_tala::seed_selection::{Scored, SeedPlan};
use ferrigura_tala::{Graph, Layout};
use serde::{Deserialize, Serialize};

mod raw_routes;

#[derive(Deserialize)]
struct PositionedEdges {
    nodes: Vec<ferrigura_tala::NodeGeometry>,
    edges: Vec<[usize; 2]>,
}

#[derive(Deserialize)]
struct RoutedCapture {
    graph: Graph,
    layout: Layout,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlacementCapture {
    graph: Graph,
    seed: i64,
    #[serde(default)]
    obstacle_order: Vec<usize>,
}

#[derive(Deserialize)]
struct Candidate {
    seed: i64,
    graph: PositionedGraph,
}

#[derive(Serialize)]
struct CandidateScores {
    evaluations: Vec<Evaluation>,
    selected_seed: i64,
}

fn score_candidates(candidates: &[Candidate]) -> Result<CandidateScores, Box<dyn Error>> {
    let evaluations = candidates
        .iter()
        .map(|candidate| evaluate(&candidate.graph, Some(&|| false)))
        .collect::<Result<Vec<_>, _>>()?;
    let seeds: Vec<_> = candidates.iter().map(|candidate| candidate.seed).collect();
    let plan = SeedPlan::new(None, Some(&seeds), 0)?;
    if plan.seeds().len() != candidates.len() {
        return Err("candidate scores require one completed graph per distinct seed".into());
    }
    let winner = plan.coordinate(
        || false,
        |index, _| {
            Ok::<_, std::convert::Infallible>(Scored {
                score: evaluations[index].score,
                candidate: &candidates[index],
            })
        },
    )?;
    Ok(CandidateScores {
        evaluations,
        selected_seed: winner.result.candidate.seed,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let input = io::stdin().lock();
    let output = io::stdout().lock();
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("--translate" | "--prescale") => {
            let input_graph = serde_json::from_reader(input)?;
            let graph = ferrigura_tala::layout_graph::checkpoint(
                &input_graph,
                command.as_deref() == Some("--prescale"),
            )?;
            serde_json::to_writer(output, &graph)?;
        }
        Some("--raw-routes") => {
            let capture = serde_json::from_reader(input)?;
            serde_json::to_writer(output, &raw_routes::trace_raw_routes(capture)?)?;
        }
        Some("--layout") => {
            let graph = serde_json::from_reader(input)?;
            serde_json::to_writer(output, &ferrigura_tala::layout(&graph)?)?;
        }
        Some("--initialize") => {
            let graph = serde_json::from_reader(input)?;
            serde_json::to_writer(output, &ferrigura_tala::initialize::initialize(&graph)?)?;
        }
        Some("--initialize-by-distance") => {
            let graph = serde_json::from_reader(input)?;
            let positions =
                ferrigura_tala::stress_initialize::initialize_by_graph_distance(&graph)?;
            serde_json::to_writer(output, &positions)?;
        }
        Some("--sizeless-first-pass") => {
            let PlacementCapture { graph, seed, .. } = serde_json::from_reader(input)?;
            let result = ferrigura_tala::sizeless::first_pass(&graph, seed)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--sizeless-phase") => {
            let PlacementCapture { graph, seed, .. } = serde_json::from_reader(input)?;
            let result = ferrigura_tala::sizeless::optimize_phase(&graph, seed)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--sized-transition") => {
            let PlacementCapture { graph, seed, .. } = serde_json::from_reader(input)?;
            let result = ferrigura_tala::sizeless::transition_to_sized(&graph, seed)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--first-sized-candidates") => {
            let PlacementCapture {
                graph,
                seed,
                obstacle_order,
            } = serde_json::from_reader(input)?;
            let result =
                ferrigura_tala::sizeless::first_sized_candidates(&graph, seed, &obstacle_order)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--first-sized-edge-costs") => {
            let PlacementCapture {
                graph,
                seed,
                obstacle_order,
            } = serde_json::from_reader(input)?;
            let result =
                ferrigura_tala::sizeless::first_sized_edge_costs(&graph, seed, &obstacle_order)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--first-sized-move") => {
            let PlacementCapture {
                graph,
                seed,
                obstacle_order,
            } = serde_json::from_reader(input)?;
            let result = ferrigura_tala::sizeless::first_sized_move(&graph, seed, &obstacle_order)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--first-sized-pass") => {
            let PlacementCapture {
                graph,
                seed,
                obstacle_order,
            } = serde_json::from_reader(input)?;
            let result = ferrigura_tala::sizeless::first_sized_pass(&graph, seed, &obstacle_order)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--first-sized-compaction") => {
            let PlacementCapture {
                graph,
                seed,
                obstacle_order,
            } = serde_json::from_reader(input)?;
            let result =
                ferrigura_tala::sizeless::first_sized_compaction(&graph, seed, &obstacle_order)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--sized-phase") => {
            let PlacementCapture {
                graph,
                seed,
                obstacle_order,
            } = serde_json::from_reader(input)?;
            let result =
                ferrigura_tala::sizeless::optimize_sized_phase(&graph, seed, &obstacle_order)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--sized-phase-snapshots") => {
            let PlacementCapture {
                graph,
                seed,
                obstacle_order,
            } = serde_json::from_reader(input)?;
            let result =
                ferrigura_tala::sizeless::sized_phase_snapshots(&graph, seed, &obstacle_order)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--place-flat-component") => {
            let PlacementCapture {
                graph,
                seed,
                obstacle_order,
            } = serde_json::from_reader(input)?;
            let result =
                ferrigura_tala::sizeless::place_flat_component(&graph, seed, &obstacle_order)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--direct-flat-component") => {
            let PlacementCapture {
                graph,
                seed,
                obstacle_order,
            } = serde_json::from_reader(input)?;
            let mut bodies =
                ferrigura_tala::sizeless::optimize_sized_phase(&graph, seed, &obstacle_order)?;
            ferrigura_tala::sizeless::direct_flat_component(&graph, &mut bodies)?;
            serde_json::to_writer(output, &bodies)?;
        }
        Some(
            "--measured-first-sized-pass"
            | "--measured-sized-transition"
            | "--measured-sized-phase"
            | "--measured-flat-placement",
        ) => {
            let PlacementCapture { graph, seed, .. } = serde_json::from_reader(input)?;
            let stage = match command.as_deref() {
                Some("--measured-first-sized-pass") => {
                    ferrigura_tala::sizeless::first_sized_pass_from_measurements
                }
                Some("--measured-sized-transition") => {
                    ferrigura_tala::sizeless::sized_transition_from_measurements
                }
                Some("--measured-sized-phase") => {
                    ferrigura_tala::sizeless::sized_phase_from_measurements
                }
                _ => ferrigura_tala::sizeless::flat_placement_from_measurements,
            };
            serde_json::to_writer(output, &stage(&graph, seed)?)?;
        }
        Some("--ovg-seed") => {
            let nodes: Vec<_> = serde_json::from_reader(input)?;
            let seed = ferrigura_tala::ovg_seed::rectangular_visibility_seed(&nodes)?;
            serde_json::to_writer(output, &seed)?;
        }
        Some("--ovg-edge-points") => {
            let PositionedEdges { nodes, edges } = serde_json::from_reader(input)?;
            let graph = ferrigura_tala::ovg_seed::rectangular_edge_visibility_seed(&nodes, &edges)?;
            serde_json::to_writer(output, &graph)?;
        }
        Some("--ovg-boundary-points") => {
            let PositionedEdges { nodes, edges } = serde_json::from_reader(input)?;
            let graph =
                ferrigura_tala::ovg_seed::rectangular_boundary_visibility_seed(&nodes, &edges)?;
            serde_json::to_writer(output, &graph)?;
        }
        Some("--ovg-tunnels") => {
            let PositionedEdges { nodes, edges } = serde_json::from_reader(input)?;
            let graph = ferrigura_tala::ovg_seed::rectangular_tunnel_visibility(&nodes, &edges)?;
            serde_json::to_writer(output, &graph)?;
        }
        Some("--ovg-links") => {
            let PositionedEdges { nodes, edges } = serde_json::from_reader(input)?;
            let graph = ferrigura_tala::ovg_seed::rectangular_linked_visibility(&nodes, &edges)?;
            serde_json::to_writer(output, &graph)?;
        }
        Some("--candidate-score") => {
            let candidates: Vec<_> = serde_json::from_reader(input)?;
            serde_json::to_writer(output, &score_candidates(&candidates)?)?;
        }
        Some("--route-output") => {
            let edges = serde_json::from_reader(input)?;
            serde_json::to_writer(output, &ferrigura_tala::route_output::project(edges)?)?;
        }
        Some("--edge-label-placement") => {
            let scenarios: Vec<_> = serde_json::from_reader(input)?;
            let selections = scenarios
                .iter()
                .map(ferrigura_tala::labeling::select)
                .collect::<Result<Vec<_>, _>>()?;
            serde_json::to_writer(output, &selections)?;
        }
        Some("--simplify") => {
            let RoutedCapture { graph, mut layout } = serde_json::from_reader(input)?;
            ferrigura_tala::simplify::simplify_routes(&graph, &mut layout)?;
            serde_json::to_writer(output, &layout)?;
        }
        Some("--balance") => {
            let RoutedCapture { graph, mut layout } = serde_json::from_reader(input)?;
            ferrigura_tala::balance::balance_routes(&graph, &mut layout)?;
            serde_json::to_writer(output, &layout)?;
        }
        None => {
            let mut geometry: Geometry = serde_json::from_reader(input)?;
            geometry.normalize()?;
            serde_json::to_writer(output, &geometry)?;
        }
        _ => {
            return Err(concat!(
                "expected --layout, --translate, --prescale, --initialize, --initialize-by-distance, ",
                "--sizeless-first-pass, --sizeless-phase, --sized-transition, --measured-sized-transition, ",
                "--first-sized-candidates, --first-sized-edge-costs, --first-sized-move, --first-sized-pass, ",
                "--measured-first-sized-pass, --measured-sized-phase, ",
                "--first-sized-compaction, --sized-phase, --sized-phase-snapshots, ",
                "--direct-flat-component, --place-flat-component, --measured-flat-placement, --ovg-seed, ",
                "--ovg-edge-points, --ovg-boundary-points, --ovg-tunnels, --ovg-links, ",
                "--candidate-score, --route-output, --raw-routes, --edge-label-placement, --simplify, --balance, or no argument"
            )
            .into());
        }
    }
    Ok(())
}
