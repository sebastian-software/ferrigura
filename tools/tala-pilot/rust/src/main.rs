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

fn main() -> Result<(), Box<dyn Error>> {
    let input = io::stdin().lock();
    let output = io::stdout().lock();
    match std::env::args().nth(1).as_deref() {
        Some("--layout") => {
            let graph = serde_json::from_reader(input)?;
            serde_json::to_writer(output, &ferrigura_tala::layout(&graph)?)?;
        }
        Some("--simplify") => {
            let RoutedCapture { graph, mut layout } = serde_json::from_reader(input)?;
            ferrigura_tala::simplify::simplify_routes(&graph, &mut layout)?;
            serde_json::to_writer(output, &layout)?;
        }
        None => {
            let mut geometry: Geometry = serde_json::from_reader(input)?;
            geometry.normalize()?;
            serde_json::to_writer(output, &geometry)?;
        }
        _ => return Err("expected --layout, --simplify, or no argument".into()),
    }
    Ok(())
}
