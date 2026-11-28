use std::error::Error;
use std::io;

use ferrigura_tala::normalize::Geometry;
use ferrigura_tala::{Graph, Layout};
use serde::Deserialize;

#[derive(Deserialize)]
struct RoutedCapture {
    graph: Graph,
    layout: Layout,
}

#[derive(Deserialize)]
struct PlacementCapture {
    graph: Graph,
    seed: i64,
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
            let PlacementCapture { graph, seed } = serde_json::from_reader(input)?;
            let result = ferrigura_tala::sizeless::first_pass(&graph, seed)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--sizeless-phase") => {
            let PlacementCapture { graph, seed } = serde_json::from_reader(input)?;
            let result = ferrigura_tala::sizeless::optimize_phase(&graph, seed)?;
            serde_json::to_writer(output, &result)?;
        }
        Some("--ovg-seed") => {
            let nodes: Vec<_> = serde_json::from_reader(input)?;
            let seed = ferrigura_tala::ovg_seed::rectangular_visibility_seed(&nodes)?;
            serde_json::to_writer(output, &seed)?;
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
                "--sizeless-first-pass, --sizeless-phase, --ovg-seed, --simplify, --balance, or no argument"
            )
            .into());
        }
    }
    Ok(())
}
