//! The browser check's explicit local-font host. Reads JSON on stdin and writes SVG plus metrics.
use std::error::Error;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::sync::Arc;

use ferrigura_text::{
    Face, FontRegistry, FontStyle, FontWeight, Run, embed_runs, layout_width, measure,
    measure_line_box, text_box,
};

fn weight(value: &serde_json::Value) -> Result<FontWeight, Box<dyn Error>> {
    let value = value.get("weight").map_or(Ok(400), |weight| {
        weight.as_u64().ok_or("weight must be an integer")
    })?;
    Ok(FontWeight::Number(u16::try_from(value)?))
}
fn style(value: &serde_json::Value) -> Result<FontStyle, Box<dyn Error>> {
    match value.get("style").map_or(Ok("normal"), |style| {
        style.as_str().ok_or("style must be a string")
    })? {
        "normal" => Ok(FontStyle::Normal),
        "italic" => Ok(FontStyle::Italic),
        _ => Err("style must be normal or italic".into()),
    }
}
fn string<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("{key} must be a string").into())
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let input: serde_json::Value = serde_json::from_str(&input)?;
    let size = input["size"].as_f64().ok_or("size must be a number")?;
    let mut fonts = FontRegistry::new();
    for face in input["fonts"].as_array().ok_or("fonts must be an array")? {
        fonts.register_file(
            string(face, "family")?,
            string(face, "path")?,
            weight(face)?,
            style(face)?,
        )?;
    }
    if let Some(fallbacks) = input["fallbacks"].as_array() {
        let names: Result<Vec<_>, _> = fallbacks
            .iter()
            .map(|value| value.as_str().ok_or("fallback must be a name"))
            .collect();
        fonts.set_fallbacks(names?);
    }
    let mut owned: Vec<(String, Arc<Face>)> = Vec::new();
    for run in input["runs"].as_array().ok_or("runs must be an array")? {
        owned.push((
            string(run, "text")?.to_owned(),
            fonts.resolve(string(run, "family")?, weight(run)?, style(run)?)?,
        ));
    }
    let runs: Vec<_> = owned
        .iter()
        .map(|(text, face)| Run { text, face })
        .collect();
    let embedded = embed_runs(&runs)?;
    let mut css: String = embedded.iter().map(|family| family.css.as_str()).collect();
    css.push_str("#probe{font-synthesis:none;");
    for (variable, family) in embedded.iter().flat_map(|family| &family.variables) {
        let _ = write!(css, "{variable}:{family};");
    }
    css.push('}');
    let mut text = String::new();
    let mut advance = 0.0_f64;
    let mut ascent = 0.0_f64;
    let mut descent = 0.0_f64;
    for (shown, face) in &owned {
        let variable = face
            .css_variable()
            .ok_or("registered face needs a binding")?;
        let _ = write!(
            text,
            "<tspan x=\"{advance}\" font-family=\"var({variable})\" font-weight=\"{}\" font-style=\"{}\">{}</tspan>",
            face.weight().css_value(),
            face.style().css_keyword(),
            escape(shown)
        );
        advance += layout_width(measure(face, shown, size)?);
        let metrics = text_box(face, size)?;
        ascent = ascent.max(metrics.ascent);
        descent = descent.max(metrics.descent);
    }
    let bounds = measure_line_box(&runs, size)?;
    let svg = format!(
        "<svg id=\"probe\" xmlns=\"http://www.w3.org/2000/svg\" width=\"2000\" height=\"200\"><style>{css}</style><text x=\"0\" y=\"100\" font-size=\"{size}\">{text}</text></svg>"
    );
    let result = serde_json::json!({"svg":svg,"advance":advance,"x":bounds.left,"width":bounds.width(),"ascent":ascent,"descent":descent});
    std::io::stdout().write_all(serde_json::to_string(&result)?.as_bytes())?;
    Ok(())
}
