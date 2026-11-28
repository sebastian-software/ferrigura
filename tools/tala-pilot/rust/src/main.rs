use std::error::Error;
use std::io;

use ferrigura_tala::normalize::Geometry;

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().nth(1).as_deref() == Some("--layout") {
        let graph = serde_json::from_reader(io::stdin().lock())?;
        let result = ferrigura_tala::layout(&graph)?;
        serde_json::to_writer(io::stdout().lock(), &result)?;
        return Ok(());
    }
    let mut geometry: Geometry = serde_json::from_reader(io::stdin().lock())?;
    geometry.normalize()?;
    serde_json::to_writer(io::stdout().lock(), &geometry)?;
    Ok(())
}
