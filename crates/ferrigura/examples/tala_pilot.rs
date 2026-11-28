//! Exports measured flowchart input and replays captured geometry for the TALA pilot.

use std::error::Error;
use std::io::{self, Write as _};

use ferrigura::{LayoutEngine, RenderOptions, Renderer, Value, render_with, with_flowchart_probe};
use ferrigura_render::flowchart::pilot;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [operation, input, files @ ..] = args.as_slice() else {
        return Err(
            "usage: tala_pilot <measure|elk|elk-geometry|paint|tala> input.mmd [geometry.json]"
                .into(),
        );
    };
    let source = std::fs::read_to_string(input)?;
    let config = Value::from(serde_json::json!({"htmlLabels": false}));
    let options = RenderOptions::default();
    let mut output = io::stdout().lock();
    match (operation.as_str(), files) {
        ("measure", []) => {
            let graph = with_flowchart_probe(&source, &config, &options, pilot::measured_graph)?;
            serde_json::to_writer(&mut output, &graph)?;
        }
        ("elk", []) => output.write_all(render_with(&source, &config, &options)?.as_bytes())?,
        ("tala", []) => {
            let svg = Renderer::builder(&config)
                .with_all()
                .build()
                .render_with_engine(&source, &options, LayoutEngine::Tala)?;
            output.write_all(svg.as_bytes())?;
        }
        ("elk-geometry", []) => {
            let graph = with_flowchart_probe(&source, &config, &options, pilot::elk_geometry)?;
            serde_json::to_writer(&mut output, &graph)?;
        }
        ("paint", [file]) => {
            let capture = serde_json::from_reader(std::fs::File::open(file)?)?;
            let svg = with_flowchart_probe(&source, &config, &options, |context| {
                pilot::paint_captured(context, capture)
            })?;
            output.write_all(svg.as_bytes())?;
        }
        _ => return Err("unknown operation or invalid argument count".into()),
    }
    Ok(())
}
