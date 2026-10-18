//! The bundled font registry: every choice maps to its own face, and the data is a TrueType file.

use ferrigura_text::{Face, FontFamily, FontStyle, FontWeight, face, fonts::FACES};

const FAMILIES: [FontFamily; 2] = [FontFamily::Sans, FontFamily::Mono];
const WEIGHTS: [FontWeight; 2] = [FontWeight::Normal, FontWeight::Bold];
const STYLES: [FontStyle; 2] = [FontStyle::Normal, FontStyle::Italic];

#[test]
fn every_choice_names_its_own_face() {
    for family in FAMILIES {
        for weight in WEIGHTS {
            for style in STYLES {
                let found: &Face = face(family, weight, style);
                assert_eq!(found.family(), family);
                assert_eq!(found.weight(), weight);
                assert_eq!(found.style(), style);
            }
        }
    }
}

#[test]
fn the_registry_holds_nine_distinct_faces() {
    let mut names: Vec<&str> = FACES.iter().map(|f| f.file_name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 9);
}

#[test]
fn file_names_match_the_bundled_fonts() {
    assert_eq!(
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal).file_name(),
        "LexicaUltralegible-Regular.ttf"
    );
    assert_eq!(
        face(FontFamily::Sans, FontWeight::Normal, FontStyle::Italic).file_name(),
        "LexicaUltralegible-Italic.ttf"
    );
    assert_eq!(
        face(FontFamily::Sans, FontWeight::Bold, FontStyle::Normal).file_name(),
        "LexicaUltralegible-Bold.ttf"
    );
    assert_eq!(
        face(FontFamily::Sans, FontWeight::Bold, FontStyle::Italic).file_name(),
        "LexicaUltralegible-BoldItalic.ttf"
    );
    assert_eq!(
        face(FontFamily::Mono, FontWeight::Normal, FontStyle::Normal).file_name(),
        "AtkinsonHyperlegibleMono-Regular.ttf"
    );
}

#[test]
fn every_face_is_a_truetype_file() {
    for found in FACES {
        let data = found.data();
        assert!(data.len() > 1000, "{} is too small", found.file_name());
        // sfnt version 1.0, as used by TrueType outlines.
        assert_eq!(&data[..4], &[0, 1, 0, 0], "{}", found.file_name());
    }
}

#[test]
fn css_names_follow_the_choice() {
    assert_eq!(FontFamily::Sans.css_suffix(), "sans");
    assert_eq!(FontFamily::Mono.css_suffix(), "mono");
    assert_eq!(FontFamily::Symbols.css_suffix(), "symbols");
    assert_eq!(FontWeight::Normal.css_value(), 400);
    assert_eq!(FontWeight::Bold.css_value(), 700);
    assert_eq!(FontStyle::Normal.css_keyword(), "normal");
    assert_eq!(FontStyle::Italic.css_keyword(), "italic");
}

#[test]
fn the_license_texts_are_bundled() {
    assert!(include_str!("../fonts/OFL-Lexica.txt").contains("SIL OPEN FONT LICENSE"));
    assert!(include_str!("../fonts/OFL-AtkinsonMono.txt").contains("SIL OPEN FONT LICENSE"));
    assert!(include_str!("../fonts/OFL-NotoSansMath.txt").contains("SIL OPEN FONT LICENSE"));
}
