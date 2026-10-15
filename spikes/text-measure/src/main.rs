//! Spike: measure label text without a browser.
//!
//! Reads a JSON array of measurement requests on stdin and writes the results as JSON on
//! stdout, so a browser script can measure the same strings and compare.
//!
//! ```json
//! [{ "text": "Hello", "style": "regular", "size": 16, "wrapWidth": 200 }]
//! ```

use std::collections::HashMap;
use std::io::{self, Read};
use std::time::Instant;

use harfrust::font::Font;
use harfrust::{Buffer, ShapeOptions, ShaperFont, shape};
use serde::{Deserialize, Serialize};
use unicode_linebreak::{BreakOpportunity, linebreaks};
use unicode_segmentation::UnicodeSegmentation;

const FONT_FILES: [(&str, &str); 5] = [
    ("regular", "fonts/LexicaUltralegible-Regular.ttf"),
    ("italic", "fonts/LexicaUltralegible-Italic.ttf"),
    ("bold", "fonts/LexicaUltralegible-Bold.ttf"),
    ("bolditalic", "fonts/LexicaUltralegible-BoldItalic.ttf"),
    ("mono", "fonts/AtkinsonHyperlegibleMono-Regular.ttf"),
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    text: String,
    style: String,
    size: f64,
    wrap_width: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LineMetrics {
    ascender: f64,
    descender: f64,
    line_gap: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    text: String,
    style: String,
    size: f64,
    width: f64,
    line: LineMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mermaid_lines: Option<Vec<String>>,
}

struct LoadedFont {
    font: Font,
    units_per_em: f64,
}

impl LoadedFont {
    fn load(path: &str) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        let font = Font::new(data, 0).ok_or_else(|| format!("{path}: not a font"))?;
        let units_per_em = f64::from(font.units_per_em());
        Ok(Self { font, units_per_em })
    }

    /// Advance width of `text` at `size` pixels, after shaping (kerning, ligatures).
    fn width(&self, text: &str, size: f64) -> Result<f64, String> {
        let shaper = ShaperFont::new(&self.font);
        let mut buffer = Buffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        shape(&shaper, &mut buffer, ShapeOptions::new()).map_err(|e| format!("{e:?}"))?;
        let units: i64 = buffer
            .glyph_positions()
            .iter()
            .map(|p| i64::from(p.x_advance))
            .sum();
        Ok(units as f64 * size / self.units_per_em)
    }

    /// Ascender, descender and line gap at `size` pixels: typo metrics when the font asks for
    /// them, `hhea` otherwise, as browsers do.
    fn line_metrics(&self, size: f64) -> LineMetrics {
        let metrics = self.font.metrics();
        let line = if metrics.use_typo_metrics {
            metrics.typo_line.or(metrics.hhea_line)
        } else {
            metrics.hhea_line.or(metrics.typo_line)
        };
        let scale = size / self.units_per_em;
        match line {
            Some(l) => LineMetrics {
                ascender: l.ascender.to_f64() * scale,
                descender: l.descender.to_f64() * scale,
                line_gap: l.line_gap.to_f64() * scale,
            },
            None => LineMetrics {
                ascender: 0.0,
                descender: 0.0,
                line_gap: 0.0,
            },
        }
    }

    /// Greedy line breaking at Unicode (UAX #14) break opportunities. A segment wider than
    /// `max` on its own stays on its own line; character-level splitting is out of scope here.
    fn wrap(&self, text: &str, size: f64, max: f64) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        let mut line_start = 0;
        let mut previous = 0;
        for (index, opportunity) in linebreaks(text) {
            let candidate = text[line_start..index].trim_end();
            if previous > line_start && self.width(candidate, size)? > max {
                lines.push(text[line_start..previous].trim_end().to_string());
                line_start = previous;
            }
            // The end of the text is reported as a mandatory break too.
            if opportunity == BreakOpportunity::Mandatory {
                lines.push(text[line_start..index].trim_end().to_string());
                line_start = index;
            }
            previous = index;
        }
        Ok(lines)
    }
}

impl LoadedFont {
    /// Width of a line the way Mermaid renders it: one `tspan` per word, every word after
    /// the first prefixed with a space, each shaped on its own.
    fn mermaid_line_width(&self, words: &[&str], size: f64) -> Result<f64, String> {
        let mut total = 0.0;
        for (index, word) in words.iter().enumerate() {
            total += if index == 0 {
                self.width(word, size)?
            } else {
                self.width(&format!(" {word}"), size)?
            };
        }
        Ok(total)
    }

    /// Port of Mermaid's `splitLineToFitWidth` (rendering-util/splitText.ts) for SVG labels:
    /// words are split at single spaces, packed greedily, and a word that does not fit on an
    /// empty line is split into graphemes, keeping at least one per line.
    fn wrap_mermaid(&self, text: &str, size: f64, max: f64) -> Result<Vec<String>, String> {
        let mut queue: std::collections::VecDeque<String> = text
            .split(' ')
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect();
        let mut lines = Vec::new();
        let mut current: Vec<String> = Vec::new();
        while let Some(word) = queue.pop_front() {
            let mut candidate: Vec<&str> = current.iter().map(String::as_str).collect();
            candidate.push(&word);
            if self.mermaid_line_width(&candidate, size)? <= max {
                current.push(word);
            } else if !current.is_empty() {
                lines.push(current.join(" "));
                current.clear();
                queue.push_front(word);
            } else {
                let graphemes: Vec<&str> = word.graphemes(true).collect();
                let mut used = 0;
                while used < graphemes.len()
                    && self.width(&graphemes[..=used].concat(), size)? <= max
                {
                    used += 1;
                }
                let used = used.max(1);
                lines.push(graphemes[..used].concat());
                let rest = graphemes[used..].concat();
                if !rest.is_empty() {
                    queue.push_front(rest);
                }
            }
        }
        if !current.is_empty() {
            lines.push(current.join(" "));
        }
        Ok(lines)
    }
}

fn main() -> Result<(), String> {
    let mut fonts = HashMap::new();
    for (style, path) in FONT_FILES {
        fonts.insert(style, LoadedFont::load(path)?);
    }

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;
    let requests: Vec<Request> = serde_json::from_str(&input).map_err(|e| e.to_string())?;

    let started = Instant::now();
    let mut responses = Vec::with_capacity(requests.len());
    for request in requests {
        let font = fonts
            .get(request.style.as_str())
            .ok_or_else(|| format!("unknown style {}", request.style))?;
        let width = font.width(&request.text, request.size)?;
        let (lines, mermaid_lines) = match request.wrap_width {
            Some(max) => (
                Some(font.wrap(&request.text, request.size, max)?),
                Some(font.wrap_mermaid(&request.text, request.size, max)?),
            ),
            None => (None, None),
        };
        responses.push(Response {
            line: font.line_metrics(request.size),
            text: request.text,
            style: request.style,
            size: request.size,
            width,
            lines,
            mermaid_lines,
        });
    }
    let elapsed = started.elapsed();
    eprintln!(
        "measured {} strings in {:.2} ms ({:.1} µs per string)",
        responses.len(),
        elapsed.as_secs_f64() * 1e3,
        elapsed.as_secs_f64() * 1e6 / responses.len().max(1) as f64
    );
    println!(
        "{}",
        serde_json::to_string(&responses).map_err(|e| e.to_string())?
    );
    Ok(())
}
