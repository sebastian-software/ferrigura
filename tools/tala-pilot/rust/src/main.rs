use std::error::Error;
use std::io;

use ferrigura_tala::Geometry;

fn main() -> Result<(), Box<dyn Error>> {
    let mut geometry: Geometry = serde_json::from_reader(io::stdin().lock())?;
    geometry.normalize()?;
    serde_json::to_writer(io::stdout().lock(), &geometry)?;
    Ok(())
}
