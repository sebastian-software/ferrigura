//! Renders the diagram text on standard input to SVG on standard output.
//!
//! Usage: `cargo run -p ferrigura --example render -- [--config file.json] [--fonts local-fonts.json] [--id ID] [--scheme light|dark|auto] [--now MS] [--seed N] [--compat true|false]`
//!
//! `--config` is the `initialize` configuration as JSON; `--id` is the id of the SVG element, and
//! `--scheme` the color scheme, and `--now` the current time in milliseconds since the epoch (the
//! harness passes the clock it gives the reference). `--seed` is the seed of the generated gitGraph
//! ids and of the venn layout's random numbers, which the harness gives the same seed as the
//! reference's random numbers. `--fonts` reads host-local font inputs for the harness: a JSON
//! object with `fonts` (`family`, `path`, `weight`, `style`) and `fallbacks` family names. Paths
//! are relative to that JSON file. Without font inputs the compatible bundled defaults apply.

use std::error::Error;
use std::io::{Read, Write};

use ferrigura::{ColorScheme, FontStyle, FontWeight, RenderOptions, Renderer};
use ferrigura_model::value::Value;

fn main() -> Result<(), Box<dyn Error>> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;

    let mut config = Value::empty_object();
    let mut options = RenderOptions::default();
    let mut font_inputs = None;
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
            "--fonts" => font_inputs = Some(std::path::PathBuf::from(value)),
            "--id" => options.id = Some(value),
            "--compat" => options.compat = value.parse()?,
            "--now" => options.now = Some(value.parse()?),
            "--seed" => {
                let seed = value.parse()?;
                options.git_graph_seed = Some(seed);
                options.random_seed = Some(seed);
            }
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

    let mut builder = Renderer::builder(&config).with_all();
    if let Some(path) = font_inputs {
        let input: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
        let faces = input["fonts"]
            .as_array()
            .ok_or("fonts input needs a fonts array")?;
        for face in faces {
            let family = face["family"].as_str().ok_or("a font needs a family")?;
            let file = face["path"].as_str().ok_or("a font needs a local path")?;
            let weight = face.get("weight").map_or(Ok(400), |weight| {
                weight.as_u64().ok_or("font weight must be an integer")
            })?;
            let weight = u16::try_from(weight).map_err(|_| "font weight is out of range")?;
            let style = match face.get("style").map_or(Ok("normal"), |style| {
                style.as_str().ok_or("font style must be a string")
            })? {
                "normal" => FontStyle::Normal,
                "italic" => FontStyle::Italic,
                _ => return Err("font style must be normal or italic".into()),
            };
            let file = path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(file);
            builder = builder.font_file_face(family, file, FontWeight::Number(weight), style)?;
        }
        if let Some(fallbacks) = input["fallbacks"].as_array() {
            let names: Result<Vec<_>, _> = fallbacks
                .iter()
                .map(|name| name.as_str().ok_or("a fallback needs a family name"))
                .collect();
            builder = builder.font_fallbacks(names?);
        }
    }
    let svg = builder.build().render_with(&text, &options)?;
    std::io::stdout().write_all(svg.as_bytes())?;
    Ok(())
}
