//! Renders the diagram text on standard input to SVG on standard output, with an optional
//! `initialize` configuration as a JSON file: `cargo run -p ferrigura --example render -- [config.json]`.

use std::error::Error;
use std::io::{Read, Write};

use ferrigura_model::value::Value;

fn main() -> Result<(), Box<dyn Error>> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;
    let config = match std::env::args().nth(1) {
        Some(path) => Value::from(serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(path)?,
        )?),
        None => Value::empty_object(),
    };
    let svg = ferrigura::render(&text, &config)?;
    std::io::stdout().write_all(svg.as_bytes())?;
    Ok(())
}
