//! Cuts a font file to the code points it is given, as the bundled faces are cut (see
//! `tools/port/fonts/cut-faces.ts`, which runs this).
//!
//! Usage: `cargo run -p ferrigura-text --example cut_font -- <source.ttf> <output.ttf>`, with the
//! code points on standard input, one or more per line in the form `U+2665` or `2665`. A code point
//! the source does not cover is ignored by the cut.

use std::error::Error;
use std::io::Read;

use ferrigura_text::cut_font;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [source, output] = arguments.as_slice() else {
        return Err("usage: cut_font <source.ttf> <output.ttf> < code points".into());
    };
    let mut listed = String::new();
    std::io::stdin().read_to_string(&mut listed)?;
    let mut characters = Vec::new();
    for word in listed.split_whitespace() {
        let hex = word.strip_prefix("U+").unwrap_or(word);
        let code = u32::from_str_radix(hex, 16)?;
        characters.push(char::from_u32(code).ok_or_else(|| format!("{word} is not a character"))?);
    }
    let data = std::fs::read(source)?;
    let cut = cut_font(&data, characters)?;
    std::fs::write(output, cut)?;
    Ok(())
}
