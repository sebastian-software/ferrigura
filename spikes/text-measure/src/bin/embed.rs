//! Spike, part 2: subset the bundled fonts to the characters a set of labels uses, embed the
//! subsets in an SVG, and record the widths measured in Rust, so a browser can check that it
//! draws every label at exactly that width.
//!
//! Usage: `embed <requests.json> <out-dir>`. Writes into `<out-dir>`:
//!
//! - `all.svg`: every request without a wrap width, one label per row, its Rust width in
//!   `data-width`, a red outline of that width, and the subsets as data-URL web fonts.
//! - `half-a.svg`, `half-b.svg`: the same labels split into two diagrams with their own subsets
//!   and unique font family names; `shared-a.svg`, `shared-b.svg`: the same with one shared name.
//! - `sizes.json`: original and subset font sizes per face.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use base64::Engine;
use harfrust::font::Font;
use harfrust::{Buffer, ShapeOptions, ShaperFont, shape};
use serde::{Deserialize, Serialize};
use skera::{DEFAULT_LAYOUT_FEATURES, Plan, SubsetFlags, subset_font};
use write_fonts::read::FontRef;
use write_fonts::read::collections::IntSet;
use write_fonts::read::types::NameId;

/// Style name, file, CSS family suffix, weight and style descriptors.
const FACES: [(&str, &str, &str, u16, &str); 5] = [
    (
        "regular",
        "fonts/LexicaUltralegible-Regular.ttf",
        "sans",
        400,
        "normal",
    ),
    (
        "italic",
        "fonts/LexicaUltralegible-Italic.ttf",
        "sans",
        400,
        "italic",
    ),
    (
        "bold",
        "fonts/LexicaUltralegible-Bold.ttf",
        "sans",
        700,
        "normal",
    ),
    (
        "bolditalic",
        "fonts/LexicaUltralegible-BoldItalic.ttf",
        "sans",
        700,
        "italic",
    ),
    (
        "mono",
        "fonts/AtkinsonHyperlegibleMono-Regular.ttf",
        "mono",
        400,
        "normal",
    ),
];

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Request {
    text: String,
    style: String,
    size: f64,
    wrap_width: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FaceSize {
    style: String,
    original: usize,
    subset: usize,
    characters: usize,
}

struct Face {
    style: &'static str,
    data: Vec<u8>,
    family: &'static str,
    weight: u16,
    font_style: &'static str,
}

/// Advance width of `text` at `size` pixels, shaped with `data`.
fn width(data: &[u8], text: &str, size: f64) -> Result<f64, String> {
    let font = Font::new(data.to_vec(), 0).ok_or("not a font")?;
    let shaper = ShaperFont::new(&font);
    let mut buffer = Buffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    shape(&shaper, &mut buffer, ShapeOptions::new()).map_err(|e| format!("{e:?}"))?;
    let units: i64 = buffer
        .glyph_positions()
        .iter()
        .map(|p| i64::from(p.x_advance))
        .sum();
    Ok(units as f64 * size / f64::from(font.units_per_em()))
}

/// Ascender and descender in font units, as browsers choose them.
fn vertical(data: &[u8]) -> Result<(f64, f64), String> {
    let font = Font::new(data.to_vec(), 0).ok_or("not a font")?;
    let metrics = font.metrics();
    let line = if metrics.use_typo_metrics {
        metrics.typo_line.or(metrics.hhea_line)
    } else {
        metrics.hhea_line.or(metrics.typo_line)
    };
    let line = line.ok_or("no vertical metrics")?;
    Ok((line.ascender.to_f64(), line.descender.to_f64()))
}

/// Subsets `data` to the given characters, keeping the default layout features (kerning,
/// ligatures) and dropping hinting. Legacy `kern` tables are kept, unlike HarfBuzz's default.
fn subset(data: &[u8], characters: &BTreeSet<char>) -> Result<Vec<u8>, String> {
    let font = FontRef::new(data).map_err(|e| e.to_string())?;
    let mut unicodes = IntSet::empty();
    for c in characters {
        unicodes.insert(u32::from(*c));
    }
    let mut scripts = IntSet::empty();
    scripts.invert();
    let mut features = IntSet::empty();
    features.extend(DEFAULT_LAYOUT_FEATURES.iter().copied());
    let mut name_ids = IntSet::empty();
    name_ids.insert_range(NameId::from(0)..=NameId::from(6));
    let mut languages = IntSet::empty();
    languages.insert(0x0409);
    let plan = Plan::new(
        &IntSet::empty(),
        &unicodes,
        &font,
        SubsetFlags::SUBSET_FLAGS_NO_HINTING,
        &IntSet::empty(),
        &scripts,
        &features,
        &name_ids,
        &languages,
    );
    subset_font(&font, &plan).map_err(|e| e.to_string())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Writes one SVG with the labels and the faces subset to them. Returns the subsets.
fn write_svg(
    path: &Path,
    faces: &[Face],
    requests: &[Request],
    prefix: &str,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut characters: BTreeMap<&str, BTreeSet<char>> = BTreeMap::new();
    for request in requests {
        characters
            .entry(request.style.as_str())
            .or_default()
            .extend(request.text.chars());
    }
    // Browsers set `white-space: nowrap` on `text` in their own stylesheet, so a rule on the root
    // does not reach it; spaces at the ends of a label are kept only with a rule on `text`.
    let mut css = String::from("text{white-space:pre}");
    let mut subsets = BTreeMap::new();
    for face in faces {
        let Some(chars) = characters.get(face.style) else {
            continue;
        };
        let data = subset(&face.data, chars)?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
        let _ = write!(
            css,
            "@font-face{{font-family:\"{prefix}-{}\";font-weight:{};font-style:{};src:url(data:font/ttf;base64,{encoded})}}",
            face.family, face.weight, face.font_style
        );
        subsets.insert(face.style, data);
    }

    let mut body = String::new();
    let mut y = 10.0;
    let mut max_width: f64 = 0.0;
    for (index, request) in requests.iter().enumerate() {
        let face = faces
            .iter()
            .find(|f| f.style == request.style)
            .ok_or("unknown style")?;
        let w = width(&face.data, &request.text, request.size)?;
        let w_subset = width(&subsets[face.style], &request.text, request.size)?;
        if w != w_subset {
            return Err(format!(
                "subset changes the width of {:?}: {w} vs {w_subset}",
                request.text
            ));
        }
        let (ascender, descender) = vertical(&face.data)?;
        if vertical(&subsets[face.style])? != (ascender, descender) {
            return Err(format!(
                "subset changes the vertical metrics of {}",
                face.style
            ));
        }
        let upem = f64::from(
            Font::new(face.data.clone(), 0)
                .ok_or("not a font")?
                .units_per_em(),
        );
        let top = (ascender * request.size / upem).round();
        let bottom = (-descender * request.size / upem).round();
        let baseline = y + top;
        let _ = writeln!(
            body,
            "<rect x=\"10\" y=\"{y}\" width=\"{w}\" height=\"{}\" fill=\"none\" stroke=\"#e33\" stroke-width=\"0.5\"/>\
             <text id=\"t{index}\" data-width=\"{w}\" data-height=\"{}\" x=\"10\" y=\"{baseline}\" font-family=\"{prefix}-{}\" font-size=\"{}\" font-weight=\"{}\" font-style=\"{}\">{}</text>",
            top + bottom,
            top + bottom,
            face.family,
            request.size,
            face.weight,
            face.font_style,
            escape(&request.text)
        );
        y += top + bottom + 6.0;
        max_width = max_width.max(w);
    }
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><style>{css}</style>\n{body}</svg>\n",
        (max_width + 20.0).ceil(),
        (y + 10.0).ceil()
    );
    std::fs::write(path, svg).map_err(|e| e.to_string())?;
    Ok(subsets
        .into_iter()
        .map(|(style, data)| (style.to_string(), data))
        .collect())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let [_, requests_path, out_dir] = args.as_slice() else {
        return Err("usage: embed <requests.json> <out-dir>".into());
    };
    let out = Path::new(out_dir);
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;

    let faces: Vec<Face> = FACES
        .iter()
        .map(|&(style, path, family, weight, font_style)| {
            Ok(Face {
                style,
                data: std::fs::read(path).map_err(|e| format!("{path}: {e}"))?,
                family,
                weight,
                font_style,
            })
        })
        .collect::<Result<_, String>>()?;

    let input = std::fs::read_to_string(requests_path).map_err(|e| e.to_string())?;
    let requests: Vec<Request> = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    let labels: Vec<Request> = requests
        .into_iter()
        .filter(|r| r.wrap_width.is_none())
        .collect();

    let subsets = write_svg(&out.join("all.svg"), &faces, &labels, "fg0")?;
    let sizes: Vec<FaceSize> = subsets
        .iter()
        .map(|(style, data)| {
            let face = faces
                .iter()
                .find(|f| f.style == style)
                .expect("face exists");
            let characters: BTreeSet<char> = labels
                .iter()
                .filter(|r| r.style == *style)
                .flat_map(|r| r.text.chars())
                .collect();
            FaceSize {
                style: style.clone(),
                original: face.data.len(),
                subset: data.len(),
                characters: characters.len(),
            }
        })
        .collect();

    // Two diagrams with different character sets: the labels of each half, regular style only,
    // so the subsets differ.
    let regular: Vec<Request> = labels
        .iter()
        .filter(|r| r.style == "regular")
        .cloned()
        .collect();
    let (a, b) = regular.split_at(regular.len() / 2);
    let pick = |half: &[Request], wanted: fn(char) -> bool| -> Vec<Request> {
        half.iter()
            .filter(|r| r.text.chars().all(wanted))
            .cloned()
            .collect()
    };
    // Half A avoids the letters q, x and z; half B uses only labels that contain one of them.
    let a = pick(a, |c| !matches!(c, 'q' | 'x' | 'z' | 'Q' | 'X' | 'Z'));
    let b: Vec<Request> = b
        .iter()
        .filter(|r| {
            r.text
                .chars()
                .any(|c| matches!(c, 'q' | 'x' | 'z' | 'Q' | 'X' | 'Z'))
        })
        .cloned()
        .collect();
    write_svg(&out.join("half-a.svg"), &faces, &a, "fg1")?;
    write_svg(&out.join("half-b.svg"), &faces, &b, "fg2")?;
    write_svg(&out.join("shared-a.svg"), &faces, &a, "fg")?;
    write_svg(&out.join("shared-b.svg"), &faces, &b, "fg")?;

    std::fs::write(
        out.join("sizes.json"),
        serde_json::to_string_pretty(&sizes).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    eprintln!(
        "{} labels, half A {}, half B {}",
        labels.len(),
        a.len(),
        b.len()
    );
    for size in &sizes {
        eprintln!(
            "{:<11} {:>6} bytes -> {:>6} bytes ({} characters)",
            size.style, size.original, size.subset, size.characters
        );
    }
    Ok(())
}
