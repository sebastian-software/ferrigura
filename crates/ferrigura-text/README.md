# ferrigura-text

`ferrigura-text` measures text with the bundled fonts, wraps lines the way Mermaid wraps its labels, and embeds subsets of the fonts in the SVG, so that a viewer draws the text that was measured. It bundles the faces that ferrigura uses: Lexica Ultralegible, Atkinson Hyperlegible Mono, and the symbol and emoji faces.

This crate is a part of [ferrigura](https://github.com/sebastian-software/ferrigura). Its API may change between versions. To render diagrams, use the `ferrigura` crate.

## Registering local fonts

`FontRegistry` starts empty. Register explicit local TTF or OTF bytes or files, then resolve a CSS
family list. The returned `Arc<Face>` owns its font bytes and fallback faces; it stays usable after
the registry is dropped. The diagram renderer continues to use its bundled font selection.

```rust,no_run
use ferrigura_text::{FontRegistry, FontStyle, FontWeight, Run, embed_runs, measure};

let mut fonts = FontRegistry::new();
fonts.register_file("My Font", "fonts/MyFont-Regular.ttf", FontWeight::Normal, FontStyle::Normal)?;
fonts.register_file("My Symbols", "fonts/Symbols.ttf", FontWeight::Normal, FontStyle::Normal)?;
fonts.set_fallbacks(["My Symbols"]);
let face = fonts.resolve("'My Font', sans-serif", FontWeight::Number(450), FontStyle::Normal)?;
let width = measure(&face, "Hello →", 16.0)?;
let embedded = embed_runs(&[Run { text: "Hello →", face: &face }])?;
for warning in FontRegistry::missing_glyphs(&face, "Hello →") {
    eprintln!("{warning}");
}
# Ok::<(), ferrigura_text::TextError>(())
```

Family matching ignores ASCII case. The first registered name wins; other registered names in the
list, then `set_fallbacks`, supply per-character fallback. Generic names have no implicit meaning:
register `sans-serif` explicitly if that is the name callers request. An unresolved primary list
fails even when the fallback chain contains a font. Weights use the nearest registered design in
the requested style, with equal distances choosing the lighter weight; missing italic uses the
upright design. No synthetic weight or slant is applied.

An uncovered character stays in the primary face and draws its missing glyph. `missing_glyphs`
returns one diagnostic per character with its Unicode value and family name; hosts decide how to
show those warnings. Registration rejects OpenType-SVG, invalid data, and legacy `kern` formats other than OpenType format 0. WOFF, WOFF2, system font
discovery, and downloads are not inputs. Subsets preserve OpenType layout, legacy kerning, and
color tables, and receive deterministic names derived from their bytes, weights, and styles. CFF
and CFF2 OpenType fonts embed complete: the subsetter cannot serialize their outlines, so their
embedded data is larger than a TrueType subset. An embedding rejects different font files with
the same family, weight, and style in one SVG, including handles retained across a registry
replacement. The old and new handles remain valid in separate renders.

`Run` has independent text and face lifetimes (`Run<'text, 'face>`), and `Word<'face>` borrows its
face. Code with explicit lifetime annotations must supply both lifetimes for `Run` and one for
`Word`; normal struct literals infer them.

## License

The crate is MIT-licensed, as the workspace (the `license` field of `Cargo.toml`). The bundled fonts keep their own licenses: the SIL Open Font License 1.1 for the text and symbol faces, and Creative Commons Attribution 4.0 for the Twemoji art, with its attribution. `NOTICE.md` lists each file, its source and its license.
