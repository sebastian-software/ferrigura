//! The fallback faces for emoji and the text symbols: the order a browser walks the font-family list
//! in, the code points each face draws, the advances the faces give (the values Chromium gives them
//! with the same faces, `docs-port/spikes/fallback-faces.md` section 5), and the subsets that embed
//! only the glyphs a diagram uses, with their colour tables.

use base64::Engine;
use ferrigura_text::{
    Face, FontFamily, FontStyle, FontWeight, Run, cut_font, embed_runs, face, fallback_faces,
    fallback_runs, layout_width, measure, measure_data, subset_face,
};
use read_fonts::types::Tag;
use read_fonts::{FontRef, TableProvider};

fn regular() -> &'static Face {
    face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
}

fn mono() -> &'static Face {
    face(FontFamily::Mono, FontWeight::Normal, FontStyle::Normal)
}

fn arrows() -> &'static Face {
    face(FontFamily::Arrows, FontWeight::Normal, FontStyle::Normal)
}

fn text_symbols() -> &'static Face {
    face(
        FontFamily::TextSymbols,
        FontWeight::Normal,
        FontStyle::Normal,
    )
}

fn emoji() -> &'static Face {
    face(FontFamily::Emoji, FontWeight::Normal, FontStyle::Normal)
}

/// The stretches of `text` in the sans face, as (text, family) pairs.
fn stretches_in_sans(text: &str) -> Vec<(String, FontFamily)> {
    fallback_runs(Run {
        text,
        face: regular(),
    })
    .iter()
    .map(|stretch| (stretch.text.to_owned(), stretch.face.family()))
    .collect()
}

/// The first `@font-face` data URL of a CSS text, decoded.
fn bytes_of(css: &str) -> Vec<u8> {
    let start = css.find("base64,").expect("the css has a data URL") + "base64,".len();
    let end = css[start..].find(')').expect("the data URL ends") + start;
    base64::engine::general_purpose::STANDARD
        .decode(&css[start..end])
        .expect("the data URL is base64")
}

#[test]
fn the_fallback_faces_are_the_arrows_then_the_text_symbols_then_the_emoji() {
    let families: Vec<FontFamily> = fallback_faces().iter().map(|f| f.family()).collect();
    assert_eq!(
        families,
        [
            FontFamily::Arrows,
            FontFamily::TextSymbols,
            FontFamily::Emoji
        ]
    );
}

#[test]
fn a_text_is_split_by_the_face_that_draws_each_character() {
    let stretches = stretches_in_sans("a→♥😀b");
    assert_eq!(
        stretches,
        [
            ("a".to_owned(), FontFamily::Sans),
            ("→".to_owned(), FontFamily::Arrows),
            ("♥".to_owned(), FontFamily::TextSymbols),
            ("😀".to_owned(), FontFamily::Emoji),
            ("b".to_owned(), FontFamily::Sans),
        ]
    );
}

#[test]
fn emoji_presentation_goes_to_the_emoji_face_and_text_default_symbols_never_do() {
    for code in [0x231A, 0x2615, 0x2728, 0x2795, 0x1F600, 0x1F680, 0x1F7E0] {
        let character = char::from_u32(code).expect("a character");
        assert!(
            emoji().covers(character),
            "U+{code:04X} is not in the emoji face"
        );
        assert!(
            !text_symbols().covers(character),
            "U+{code:04X} is in the text symbols"
        );
        let stretches = stretches_in_sans(&character.to_string());
        assert_eq!(stretches.len(), 1);
        assert_eq!(stretches[0].1, FontFamily::Emoji, "U+{code:04X}");
    }
    for code in [0x2600, 0x2605, 0x2665, 0x26A0, 0x260E, 0x2713] {
        let character = char::from_u32(code).expect("a character");
        assert!(
            text_symbols().covers(character),
            "U+{code:04X} is not in the text symbols"
        );
        assert!(
            !emoji().covers(character),
            "U+{code:04X} is in the emoji face"
        );
        let stretches = stretches_in_sans(&character.to_string());
        assert_eq!(stretches[0].1, FontFamily::TextSymbols, "U+{code:04X}");
    }
}

#[test]
fn an_emoji_the_emoji_face_lacks_is_drawn_by_no_face() {
    // U+1F6D8 has emoji presentation and Twemoji Mozilla does not cover it: no bundled face does,
    // so the run keeps the sans face, which draws its missing glyph (the harness excludes it).
    let character = '\u{1F6D8}';
    assert!(!emoji().covers(character));
    let stretches = stretches_in_sans(&character.to_string());
    assert_eq!(stretches, [(character.to_string(), FontFamily::Sans)]);
}

#[test]
fn the_text_faces_keep_the_characters_they_cover() {
    // Lexica covers the copyright sign and Atkinson Mono covers the eighth note: a run in either
    // face is not split for them, and no fallback face takes them.
    assert!(regular().covers('©'));
    assert_eq!(stretches_in_sans("©"), [("©".to_owned(), FontFamily::Sans)]);
    assert!(mono().covers('♪'));
    let stretches = fallback_runs(Run {
        text: "♪ x",
        face: mono(),
    });
    assert_eq!(stretches.len(), 1);
    assert_eq!(stretches[0].face.family(), FontFamily::Mono);
    // A symbol the sans face lacks is not in the mono face either, so the mono run falls back.
    assert!(!mono().covers('♥'));
    let stretches = fallback_runs(Run {
        text: "♥",
        face: mono(),
    });
    assert_eq!(stretches[0].face.family(), FontFamily::TextSymbols);
}

#[test]
fn the_fallback_faces_are_not_split_again() {
    for symbol in [arrows(), text_symbols(), emoji()] {
        let stretches = fallback_runs(Run {
            text: "→♥😀",
            face: symbol,
        });
        assert_eq!(stretches.len(), 1, "{}", symbol.file_name());
    }
}

#[test]
fn the_emoji_face_covers_the_code_points_that_twemoji_covers_with_presentation() {
    let covered = (0..=0x10FFFF_u32)
        .filter_map(char::from_u32)
        .filter(|character| emoji().covers(*character))
        .count();
    assert_eq!(covered, 1185);
    let symbols = (0x2600..=0x27BF_u32)
        .filter_map(char::from_u32)
        .filter(|character| text_symbols().covers(*character))
        .count();
    assert_eq!(symbols, 252);
    let outside = (0..=0x10FFFF_u32)
        .filter_map(char::from_u32)
        .filter(|character| text_symbols().covers(*character))
        .filter(|character| !(0x2600..=0x27BF).contains(&(*character as u32)))
        .count();
    assert_eq!(outside, 0, "the text symbols only cover U+2600-27BF");
}

#[test]
fn the_emoji_and_symbol_faces_draw_with_the_advances_of_their_faces() {
    // Twemoji is 512 units to the em, and an emoji is 16 px wide at 16 px, as the reference's
    // measurement with the same face gives (`fallback-faces.md`, section 5).
    assert_eq!(measure(regular(), "😀", 16.0).expect("measures"), 16.0);
    assert_eq!(
        measure(regular(), "😀", 16.0).expect("measures"),
        measure_data(emoji().data(), "😀", 16.0).expect("measures")
    );
    // Noto Sans Symbols 2 draws U+2665 at 740 units (11.84 px) and U+2605 at 960 (15.36 px); the
    // layout of Chromium rounds them up to 1/64 px (11.844 and 15.375 in the reference).
    let heart = measure(regular(), "♥", 16.0).expect("measures");
    assert!((heart - 11.84).abs() < 1e-9, "{heart}");
    assert_eq!(layout_width(heart), 11.84375);
    let star = measure(regular(), "★", 16.0).expect("measures");
    assert!((star - 15.36).abs() < 1e-9, "{star}");
    assert_eq!(layout_width(star), 15.375);
}

#[test]
fn a_line_with_symbols_and_emoji_is_the_sum_of_its_stretches() {
    let text = "a♥ 😀 b→";
    let whole = measure(regular(), text, 14.0).expect("measures");
    let parts: f64 = fallback_runs(Run {
        text,
        face: regular(),
    })
    .iter()
    .map(|stretch| measure(stretch.face, stretch.text, 14.0).expect("measures"))
    .sum();
    assert!((whole - parts).abs() < 1e-9, "{whole} vs {parts}");
}

#[test]
fn an_embedded_emoji_subset_keeps_its_colour_tables_and_only_its_glyphs() {
    let runs = [Run {
        text: "a😀",
        face: regular(),
    }];
    let families = embed_runs(&runs).expect("embeds");
    assert_eq!(
        families.iter().map(|f| f.family).collect::<Vec<_>>(),
        [FontFamily::Sans, FontFamily::Emoji]
    );
    let emoji_family = &families[1];
    assert!(
        emoji_family.name.ends_with("-emoji"),
        "{}",
        emoji_family.name
    );
    assert_eq!(
        families[0].css_family,
        format!("\"{}\", \"{}\"", families[0].name, emoji_family.name)
    );

    let bytes = bytes_of(&emoji_family.css);
    assert!(
        bytes.len() < emoji().data().len() / 20,
        "{} bytes",
        bytes.len()
    );
    let font = FontRef::new(&bytes).expect("the subset parses");
    assert!(font.table_data(Tag::new(b"COLR")).is_some(), "COLR is kept");
    assert!(font.table_data(Tag::new(b"CPAL")).is_some(), "CPAL is kept");
    let glyphs = font.maxp().expect("maxp").num_glyphs();
    assert!(glyphs < 40, "{glyphs} glyphs for one emoji");
    assert_eq!(
        measure_data(&bytes, "😀", 16.0).expect("measures"),
        16.0,
        "the subset draws the emoji with its advance"
    );
}

#[test]
fn an_embedded_fallback_names_follow_the_order_of_the_faces() {
    let runs = [Run {
        text: "a→♥😀",
        face: regular(),
    }];
    let families = embed_runs(&runs).expect("embeds");
    let order: Vec<FontFamily> = families.iter().map(|f| f.family).collect();
    assert_eq!(
        order,
        [
            FontFamily::Sans,
            FontFamily::Arrows,
            FontFamily::TextSymbols,
            FontFamily::Emoji
        ]
    );
    let names: Vec<&str> = families.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        families[0].css_family,
        format!(
            "\"{}\", \"{}\", \"{}\", \"{}\"",
            names[0], names[1], names[2], names[3]
        )
    );
    for fallback in &families[1..] {
        assert_eq!(fallback.css_family, format!("\"{}\"", fallback.name));
    }
}

#[test]
fn a_text_symbol_subset_names_only_itself() {
    let runs = [Run {
        text: "♥",
        face: regular(),
    }];
    let families = embed_runs(&runs).expect("embeds");
    assert_eq!(families.len(), 1, "the sans face has no stretch of its own");
    assert_eq!(families[0].family, FontFamily::TextSymbols);
    assert_eq!(families[0].css_family, format!("\"{}\"", families[0].name));
    let bytes = bytes_of(&families[0].css);
    assert_eq!(
        measure_data(&bytes, "♥", 16.0).expect("measures"),
        measure(regular(), "♥", 16.0).expect("measures")
    );
}

#[test]
fn a_subset_of_the_emoji_face_is_the_same_as_cutting_it_again() {
    let subset = subset_face(emoji(), "😀").expect("subsets");
    let cut = cut_font(emoji().data(), ['😀']).expect("cuts");
    assert_eq!(subset.len(), cut.len());
    let font = FontRef::new(&cut).expect("the cut parses");
    assert!(font.table_data(Tag::new(b"COLR")).is_some());
    assert!(font.table_data(Tag::new(b"CPAL")).is_some());
}

#[test]
fn the_bundled_symbol_faces_keep_the_name_records_of_the_cut() {
    // The cut keeps the name records 0 to 6, 13 and 14 of its source. Noto Sans Symbols 2 has its
    // licence text in record 13; Twemoji Mozilla's source has no record 13.
    for bundled in [text_symbols(), emoji()] {
        let font = FontRef::new(bundled.data()).expect("parses");
        let name = font.name().expect("name table");
        let ids: Vec<u16> = name
            .name_record()
            .iter()
            .map(|record| record.name_id().to_u16())
            .collect();
        assert!(
            ids.contains(&1),
            "{}: name 1 is missing",
            bundled.file_name()
        );
        assert!(
            ids.iter().all(|id| *id <= 6 || *id == 13 || *id == 14),
            "{}: {ids:?}",
            bundled.file_name()
        );
    }
    let licence = FontRef::new(text_symbols().data()).expect("parses");
    let has_licence = licence
        .name()
        .expect("name table")
        .name_record()
        .iter()
        .any(|record| record.name_id().to_u16() == 13);
    assert!(has_licence, "Noto Sans Symbols 2 keeps its licence record");
}
