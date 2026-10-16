//! Renders the diagram text on standard input to SVG on standard output.
//!
//! Usage: `cargo run -p ferrigura --example render -- [--config file.json] [--id ID] [--scheme light|dark|auto]`
//!
//! `--config` is the `initialize` configuration as JSON; `--id` is the id of the SVG element, and
//! `--scheme` the colour scheme. Without them the defaults apply.

use std::error::Error;
use std::io::{Read, Write};

use ferrigura::{ColorScheme, RenderOptions, render_with};
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
            "--scheme" => {
                options.color_scheme = Some(match value.as_str() {
                    "light" => ColorScheme::Light,
                    "dark" => ColorScheme::Dark,
                    "auto" => ColorScheme::Auto,
                    other => return Err(format!("unknown colour scheme {other}").into()),
                });
            }
            other => return Err(format!("unknown argument {other}").into()),
        }
    }

    let svg = render_with(&text, &config, &options)?;
    std::io::stdout().write_all(svg.as_bytes())?;
    Ok(())
}
