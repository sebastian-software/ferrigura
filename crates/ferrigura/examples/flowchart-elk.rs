//! Prints the graph a flowchart hands to ELK and the graph ELK returns, for the diagram text on
//! standard input, as `{"elkInput": …, "elkOutput": …}`. The flowchart geometry harness
//! (`tools/port/geometry/flowchart-geometry.ts`) compares them with the reference's capture.
//!
//! Usage: `cargo run -p ferrigura --example flowchart-elk -- [--config file.json] [--id ID]`

use std::error::Error;
use std::io::{Read, Write};

use ferrigura::{RenderOptions, flowchart_elk_stages};
use ferrigura_model::value::Value;

fn main() -> Result<(), Box<dyn Error>> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;

    let mut config = Value::empty_object();
    let mut options = RenderOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("{argument} needs a value"))?;
        match argument.as_str() {
            "--config" => {
                config = Value::from(serde_json::from_str::<serde_json::Value>(
                    &std::fs::read_to_string(value)?,
                )?);
            }
            "--id" => options.id = Some(value),
            other => return Err(format!("unknown argument {other}").into()),
        }
    }

    let stages = flowchart_elk_stages(&text, &config, &options)?;
    std::io::stdout().write_all(serde_json::to_string(&stages)?.as_bytes())?;
    Ok(())
}
