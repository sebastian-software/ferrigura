//! The public, local-font path: ownership, resolution, fallback, metrics, and embedding.

mod support;

use ferrigura_text::{
    FontRegistry, FontStyle, FontWeight, Run, embed_runs, fallback_runs, measure, measure_box,
    measure_data, subset_face, text_box, wrap_line,
};
use support::{chromium, entries, number, size_of, style_of};

fn registry() -> FontRegistry {
    let mut fonts = FontRegistry::new();
    for (family, file, weight, style) in [
        (
            "Fixture Sans",
            "LexicaUltralegible-Regular.ttf",
            FontWeight::Normal,
            FontStyle::Normal,
        ),
        (
            "Fixture Sans",
            "LexicaUltralegible-Bold.ttf",
            FontWeight::Bold,
            FontStyle::Normal,
        ),
        (
            "Fixture Mono",
            "AtkinsonHyperlegibleMono-Regular.ttf",
            FontWeight::Normal,
            FontStyle::Normal,
        ),
        (
            "Fixture Mono",
            "AtkinsonHyperlegibleMono-Italic.ttf",
            FontWeight::Normal,
            FontStyle::Italic,
        ),
        (
            "Arrows",
            "NotoSansMath-Regular-Arrows.ttf",
            FontWeight::Normal,
            FontStyle::Normal,
        ),
        (
            "Emoji",
            "TwemojiMozilla-COLRv0-Emoji.ttf",
            FontWeight::Normal,
            FontStyle::Normal,
        ),
    ] {
        fonts
            .register_file(
                family,
                format!("{}/fonts/{file}", env!("CARGO_MANIFEST_DIR")),
                weight,
                style,
            )
            .unwrap();
    }
    fonts
}

#[test]
fn a_resolved_face_outlives_its_registry_and_supplied_bytes() {
    let face = {
        let mut fonts = registry();
        fonts.set_fallbacks(["Arrows"]);
        fonts
            .resolve("Fixture Sans", FontWeight::Normal, FontStyle::Normal)
            .unwrap()
    };
    assert_eq!(face.family_name(), "Fixture Sans");
    assert!(measure(&face, "AV →", 16.0).unwrap() > 0.0);
    assert!(!wrap_line("AV → AV", &face, 16.0, 30.0).unwrap().is_empty());
    assert!(measure_box(&face, "AV →", 16.0).unwrap().width() > 0.0);
    let families = embed_runs(&[Run {
        text: "AV →",
        face: &face,
    }])
    .unwrap();
    assert_eq!(families.len(), 2);
    assert!(families.iter().any(|family| family.source_name == "Arrows"));
}

#[test]
fn family_lists_honor_quotes_order_case_and_explicit_fallbacks() {
    let mut fonts = registry();
    fonts
        .register(
            "Name, with comma",
            include_bytes!("../fonts/AtkinsonHyperlegibleMono-Regular.ttf").to_vec(),
            FontWeight::Normal,
            FontStyle::Normal,
        )
        .unwrap();
    let face = fonts
        .resolve(
            "Missing, 'name, with comma', Fixture Sans",
            FontWeight::Normal,
            FontStyle::Normal,
        )
        .unwrap();
    assert_eq!(face.family_name(), "Name, with comma");
    assert!(
        FontRegistry::new()
            .resolve("sans-serif", FontWeight::Normal, FontStyle::Normal)
            .unwrap_err()
            .to_string()
            .contains("register")
    );
    fonts.set_fallbacks(["Arrows"]);
    assert!(
        fonts
            .resolve("Unknown", FontWeight::Normal, FontStyle::Normal)
            .is_err()
    );
    let face = fonts
        .resolve(
            "Fixture Sans, Arrows",
            FontWeight::Normal,
            FontStyle::Normal,
        )
        .unwrap();
    let runs = fallback_runs(Run {
        text: "A→B",
        face: &face,
    });
    assert_eq!(
        runs.iter()
            .map(|run| run.face.family_name())
            .collect::<Vec<_>>(),
        ["Fixture Sans", "Arrows", "Fixture Sans"]
    );
    assert_eq!(
        measure(&face, "A→B", 16.0).unwrap(),
        runs.iter()
            .map(|run| measure(run.face, run.text, 16.0).unwrap())
            .sum::<f64>()
    );
}

#[test]
fn weight_and_italic_choose_registered_designs_without_synthesis() {
    let fonts = registry();
    let face = fonts
        .resolve("Fixture Sans", FontWeight::Number(620), FontStyle::Italic)
        .unwrap();
    assert_eq!(face.weight(), FontWeight::Bold);
    assert_eq!(face.style(), FontStyle::Normal);
    let face = fonts
        .resolve("Fixture Mono", FontWeight::Bold, FontStyle::Italic)
        .unwrap();
    assert_eq!(face.weight(), FontWeight::Normal);
    assert_eq!(face.style(), FontStyle::Italic);
}

#[test]
fn registered_mono_metrics_match_chromium_at_every_probed_size() {
    let fonts = registry();
    let regular = fonts
        .resolve("Fixture Mono", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let italic = fonts
        .resolve("Fixture Mono", FontWeight::Normal, FontStyle::Italic)
        .unwrap();
    let fixture = chromium();
    let mut checked = 0;
    for entry in entries(&fixture, "boxes") {
        let face = match style_of(&entry) {
            "mono-regular" => &regular,
            "mono-italic" => &italic,
            _ => continue,
        };
        let found = text_box(face, size_of(&entry)).unwrap();
        assert_eq!(found.ascent, number(&entry, "ascent"));
        assert_eq!(found.descent, number(&entry, "descent"));
        checked += 1;
    }
    assert!(checked > 60);
}

#[test]
fn registered_subsets_keep_widths_and_have_distinct_stable_names() {
    let fonts = registry();
    let sans = fonts
        .resolve("Fixture Sans", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let mono = fonts
        .resolve("Fixture Mono", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let text = "AVATAR";
    let subset = subset_face(&sans, text).unwrap();
    assert_eq!(
        measure(&sans, text, 16.0).unwrap(),
        measure_data(&subset, text, 16.0).unwrap()
    );
    let embedded = embed_runs(&[Run { text, face: &sans }, Run { text, face: &mono }]).unwrap();
    assert_eq!(embedded.len(), 2);
    assert_ne!(embedded[0].name, embedded[1].name);
    assert_eq!(
        embedded,
        embed_runs(&[Run { text, face: &mono }, Run { text, face: &sans }]).unwrap()
    );
}

#[test]
fn missing_glyphs_are_observable_and_remain_in_the_primary_face() {
    let fonts = registry();
    let face = fonts
        .resolve("Fixture Sans", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let missing = FontRegistry::missing_glyphs(&face, "漢漢");
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].character, '漢');
    assert!(missing[0].to_string().contains("U+6F22"));
    assert!(missing[0].to_string().contains("Fixture Sans"));
    let runs = fallback_runs(Run {
        text: "漢",
        face: &face,
    });
    assert_eq!(runs.len(), 1);
    assert!(std::ptr::eq(runs[0].face, face.as_ref()));
    let subset = subset_face(&face, "漢").unwrap();
    assert_eq!(
        measure(&face, "漢", 16.0).unwrap(),
        measure_data(&subset, "漢", 16.0).unwrap()
    );
}

#[test]
fn color_tables_survive_registration_and_embedding() {
    use read_fonts::{FontRef, TableProvider};
    let fonts = registry();
    let face = fonts
        .resolve("Emoji", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let subset = subset_face(&face, "😀").unwrap();
    let font = FontRef::new(&subset).unwrap();
    assert!(font.colr().is_ok());
    assert!(font.cpal().is_ok());
}

#[test]
fn malformed_data_and_weights_fail_at_registration() {
    let mut fonts = FontRegistry::new();
    assert!(
        fonts
            .register("Bad", vec![0, 1, 2], FontWeight::Normal, FontStyle::Normal)
            .is_err()
    );
    assert!(
        fonts
            .register("", vec![0], FontWeight::Normal, FontStyle::Normal)
            .is_err()
    );
    assert!(
        fonts
            .register("Bad", vec![0], FontWeight::Number(1001), FontStyle::Normal)
            .is_err()
    );
    assert!(
        fonts
            .register_file(
                "Missing",
                "/does/not/exist.ttf",
                FontWeight::Normal,
                FontStyle::Normal
            )
            .is_err()
    );
}

/// Replaces GPOS with a small synthetic table while keeping the fixture's outlines and cmap.
/// FontRef and the shaper read table offsets; sorting the directory keeps its tag lookup valid.
fn replace_gpos(tag: &[u8; 4], replacement: &[u8]) -> Vec<u8> {
    let mut data = include_bytes!("../fonts/LexicaUltralegible-Regular.ttf").to_vec();
    let count = u16::from_be_bytes(data[4..6].try_into().unwrap()) as usize;
    let mut records: Vec<[u8; 16]> = data[12..12 + count * 16].as_chunks::<16>().0.to_vec();
    let record = records
        .iter_mut()
        .find(|record| &record[..4] == b"GPOS")
        .unwrap();
    let offset = u32::from_be_bytes(record[8..12].try_into().unwrap()) as usize;
    let length = u32::from_be_bytes(record[12..16].try_into().unwrap()) as usize;
    assert!(replacement.len() <= length);
    data[offset..offset + replacement.len()].copy_from_slice(replacement);
    record[..4].copy_from_slice(tag);
    record[12..16].copy_from_slice(&(replacement.len() as u32).to_be_bytes());
    records.sort_by_key(|record| <[u8; 4]>::try_from(&record[..4]).unwrap());
    for (index, record) in records.iter().enumerate() {
        data[12 + index * 16..28 + index * 16].copy_from_slice(record);
    }
    data
}

#[test]
fn a_legacy_kern_only_font_keeps_kerning_after_registration_and_subsetting() {
    use read_fonts::{FontRef, TableProvider};
    let original = FontRef::new(include_bytes!("../fonts/LexicaUltralegible-Regular.ttf")).unwrap();
    let cmap = original.cmap().unwrap();
    let a = cmap.map_codepoint('A').unwrap().to_u32() as u16;
    let v = cmap.map_codepoint('V').unwrap().to_u32() as u16;
    // One horizontal Microsoft kern-v0 pair with a -100 unit adjustment.
    let kern: Vec<u8> = [0_u16, 1, 0, 20, 1, 1, 6, 0, 0, a, v, (-100_i16) as u16]
        .into_iter()
        .flat_map(u16::to_be_bytes)
        .collect();
    let data = replace_gpos(b"kern", &kern);
    let font = FontRef::new(&data).unwrap();
    assert!(font.gpos().is_err());
    assert!(font.kern().is_ok());
    let mut fonts = FontRegistry::new();
    fonts
        .register("Legacy", data, FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let face = fonts
        .resolve("Legacy", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let kerned = measure(&face, "AV", 16.0).unwrap();
    let separate = measure(&face, "A", 16.0).unwrap() + measure(&face, "V", 16.0).unwrap();
    assert!(kerned < separate);
    let subset = subset_face(&face, "AV").unwrap();
    assert!(FontRef::new(&subset).unwrap().kern().is_ok());
    assert_eq!(measure_data(&subset, "AV", 16.0).unwrap(), kerned);
}

#[test]
fn opentype_svg_is_rejected_before_shaping_or_subsetting() {
    let mut fonts = FontRegistry::new();
    let data = replace_gpos(b"SVG ", &[0; 10]);
    let error = fonts
        .register("SVG", data, FontWeight::Normal, FontStyle::Normal)
        .unwrap_err();
    assert!(error.to_string().contains("OpenType-SVG"));
}

#[test]
fn metrics_select_typo_hhea_and_win_extents_from_supplied_font_tables() {
    use read_fonts::FontRef;
    use read_fonts::types::Tag;
    use write_fonts::FontBuilder;
    let original = FontRef::new(include_bytes!("../fonts/LexicaUltralegible-Regular.ttf")).unwrap();
    for (name, use_typo, hhea, typo, expected) in [
        (
            "Hhea",
            false,
            [900_i16, -200, 100],
            [700_i16, -100, 50],
            [900.0, 200.0, 100.0],
        ),
        (
            "Typo",
            true,
            [900_i16, -200, 100],
            [700_i16, -100, 50],
            [700.0, 100.0, 50.0],
        ),
        ("Win", false, [0_i16; 3], [0_i16; 3], [1100.0, 300.0, 0.0]),
    ] {
        let mut hhea_data = original
            .table_data(Tag::new(b"hhea"))
            .unwrap()
            .as_bytes()
            .to_vec();
        for (index, value) in hhea.into_iter().enumerate() {
            hhea_data[4 + index * 2..6 + index * 2].copy_from_slice(&value.to_be_bytes());
        }
        let mut os2 = original
            .table_data(Tag::new(b"OS/2"))
            .unwrap()
            .as_bytes()
            .to_vec();
        let mut selection = u16::from_be_bytes(os2[62..64].try_into().unwrap());
        if use_typo {
            selection |= 1 << 7;
        } else {
            selection &= !(1 << 7);
        }
        os2[62..64].copy_from_slice(&selection.to_be_bytes());
        for (index, value) in typo.into_iter().enumerate() {
            os2[68 + index * 2..70 + index * 2].copy_from_slice(&value.to_be_bytes());
        }
        os2[74..76].copy_from_slice(&1100_u16.to_be_bytes());
        os2[76..78].copy_from_slice(&300_u16.to_be_bytes());
        let mut builder = FontBuilder::new();
        builder
            .add_raw(Tag::new(b"hhea"), hhea_data)
            .add_raw(Tag::new(b"OS/2"), os2)
            .copy_missing_tables(original.clone());
        let mut fonts = FontRegistry::new();
        fonts
            .register(name, builder.build(), FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        let face = fonts
            .resolve(name, FontWeight::Normal, FontStyle::Normal)
            .unwrap();
        let found = ferrigura_text::font_metrics(&face, 1000.0).unwrap();
        assert_eq!(
            [found.ascent, found.descent, found.line_gap],
            expected,
            "{name}"
        );
    }
}

#[test]
fn a_registered_cff_font_embeds_with_the_same_advances() {
    use read_fonts::{FontRef, TableProvider};
    let bytes = include_bytes!("fixtures/SourceCodePro-Regular.otf");
    let source = FontRef::new(bytes).unwrap();
    assert!(source.cff().is_ok());
    let mut fonts = FontRegistry::new();
    fonts
        .register(
            "Source Code Pro",
            bytes.to_vec(),
            FontWeight::Normal,
            FontStyle::Normal,
        )
        .unwrap();
    let face = fonts
        .resolve("Source Code Pro", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let subset = subset_face(&face, "Hello CFF").unwrap();
    assert!(FontRef::new(&subset).unwrap().cff().is_ok());
    assert_eq!(
        measure(&face, "Hello CFF", 16.0).unwrap(),
        measure_data(&subset, "Hello CFF", 16.0).unwrap()
    );
    let embedded = embed_runs(&[Run {
        text: "Hello CFF",
        face: &face,
    }])
    .unwrap();
    assert!(embedded[0].css.contains("data:font/otf;base64,"));
}

#[test]
fn replacing_a_face_keeps_old_handles_valid_but_rejects_conflicting_embedding() {
    let mut fonts = registry();
    let old = fonts
        .resolve("Fixture Sans", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    fonts
        .register(
            "fixture sans",
            include_bytes!("../fonts/AtkinsonHyperlegibleMono-Regular.ttf").to_vec(),
            FontWeight::Normal,
            FontStyle::Normal,
        )
        .unwrap();
    let new = fonts
        .resolve("Fixture Sans", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    assert_ne!(
        measure(&old, "AV", 16.0).unwrap(),
        measure(&new, "AV", 16.0).unwrap()
    );
    assert!(
        embed_runs(&[Run {
            text: "AV",
            face: &old
        }])
        .is_ok()
    );
    assert!(
        embed_runs(&[Run {
            text: "AV",
            face: &new
        }])
        .is_ok()
    );
    let error = embed_runs(&[
        Run {
            text: "AV",
            face: &old,
        },
        Run {
            text: "AV",
            face: &new,
        },
    ])
    .unwrap_err();
    assert!(error.to_string().contains("different font data"));
    assert!(error.to_string().contains("Fixture Sans"));
}

#[test]
fn all_fallback_text_retains_the_primary_line_metrics() {
    let mut fonts = registry();
    fonts.set_fallbacks(["Arrows"]);
    let primary = fonts
        .resolve("Fixture Sans", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    assert!(!primary.covers('→'));
    let families = embed_runs(&[Run {
        text: "→",
        face: &primary,
    }])
    .unwrap();
    assert_eq!(families.len(), 2);
    let origin = families
        .iter()
        .find(|family| family.source_name == "Fixture Sans")
        .unwrap();
    let fallback = families
        .iter()
        .find(|family| family.source_name == "Arrows")
        .unwrap();
    assert_eq!(
        origin.css_family,
        format!("\"{}\", \"{}\"", origin.name, fallback.name)
    );
    assert!(
        families
            .iter()
            .flat_map(|family| &family.variables)
            .any(
                |(name, value)| Some(name.as_str()) == primary.css_variable()
                    && value == &origin.css_family
            )
    );
}

#[test]
fn resolving_a_warmed_fallback_as_primary_resets_its_chain_identity() {
    let mut fonts = registry();
    fonts.set_fallbacks(["Fixture Mono"]);
    let first = fonts
        .resolve("Fixture Sans", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let _ = first.css_variable();
    fonts.set_fallbacks(["Arrows"]);
    let warmed = fonts
        .resolve("Fixture Mono", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let mut clean = registry();
    clean.set_fallbacks(["Arrows"]);
    let expected = clean
        .resolve("Fixture Mono", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    assert_eq!(warmed.css_variable(), expected.css_variable());
    clean.set_fallbacks(std::iter::empty::<String>());
    let no_chain = clean
        .resolve("Fixture Mono", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    assert_ne!(warmed.css_variable(), no_chain.css_variable());
}

#[test]
fn variable_faces_embed_the_default_axes_and_keep_exact_tie_metrics() {
    let mut fonts = FontRegistry::new();
    fonts
        .register(
            "Variable",
            include_bytes!("fixtures/Inter-DefaultAxes.ttf").to_vec(),
            FontWeight::Bold,
            FontStyle::Normal,
        )
        .unwrap();
    let face = fonts
        .resolve("Variable", FontWeight::Bold, FontStyle::Normal)
        .unwrap();
    let families = embed_runs(&[Run {
        text: "Hamburgefontsiv",
        face: &face,
    }])
    .unwrap();
    assert!(families[0].css.contains("font-variation-settings:"));
    assert!(families[0].css.contains("\"wght\" 400"));
    assert!(families[0].css.contains("\"opsz\" 14"));
    assert_eq!(text_box(&face, 16.0).unwrap().ascent, 16.0);
}

#[test]
fn cff_line_metrics_keep_their_own_fractional_tie_precision() {
    let mut fonts = FontRegistry::new();
    fonts
        .register(
            "Code",
            include_bytes!("fixtures/SourceCodePro-Regular.otf").to_vec(),
            FontWeight::Normal,
            FontStyle::Normal,
        )
        .unwrap();
    let face = fonts
        .resolve("Code", FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    assert_eq!(text_box(&face, 62.5).unwrap().ascent, 62.0);
    assert!(measure_box(&face, "jf AVATAR", 16.0).unwrap().width() > 0.0);
}
