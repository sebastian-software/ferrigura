//! Port of `packages/mermaid/src/rendering-util/splitText.spec.ts`: the grapheme split, the line
//! packing and its checks, with the spec's test names. The spec measures widths in characters, as
//! the functions here are given a `fits` closure that counts graphemes.

use ferrigura_text::{
    FontFamily, FontStyle, FontWeight, TextError, Word, face, split_line_to_fit_width,
    split_line_to_words, split_text_to_chars,
};

type Lines = Vec<Vec<String>>;

fn normal() -> &'static ferrigura_text::Face {
    face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
}

/// `getLineFromString` of the spec: the word segments of a string, each a normal word.
fn line_from_string(text: &str) -> Vec<Word> {
    split_line_to_words(text)
        .into_iter()
        .map(|content| Word {
            text: content.to_owned(),
            face: normal(),
        })
        .collect()
}

/// `createCheckFn` of the spec: a line fits when its joined text has at most `width` graphemes.
fn check_fn(width: usize) -> impl FnMut(&[Word]) -> Result<bool, TextError> {
    move |line: &[Word]| {
        let joined: String = line.iter().map(|word| word.text.as_str()).collect();
        Ok(split_text_to_chars(&joined).len() <= width)
    }
}

fn texts(lines: &[Vec<Word>]) -> Lines {
    lines
        .iter()
        .map(|line| line.iter().map(|word| word.text.clone()).collect())
        .collect()
}

/// The expected lines of a split: each expected string as the word segments the spec builds.
fn expected(split: &[&str]) -> Lines {
    split
        .iter()
        .map(|text| {
            line_from_string(text)
                .into_iter()
                .map(|word| word.text)
                .collect()
        })
        .collect()
}

#[test]
fn should_split_str_into_graphemes() {
    let cases: [(&str, &[&str]); 4] = [
        ("", &[]),
        ("🏳️‍⚧️🏳️‍🌈👩🏾‍❤️‍👨🏻", &["🏳️‍⚧️", "🏳️‍🌈", "👩🏾‍❤️‍👨🏻"]),
        ("ok", &["o", "k"]),
        ("abc", &["a", "b", "c"]),
    ];
    for (text, split) in cases {
        assert_eq!(split_text_to_chars(text), split, "{text:?}");
    }
}

#[test]
fn should_create_valid_check_fn() {
    let mut check5 = check_fn(5);
    assert!(check5(&line_from_string("hello")[..1]).unwrap());
    assert!(!check5(&line_from_string("hello world")).unwrap());
    let mut check1 = check_fn(1);
    assert!(
        check1(&[Word {
            text: "A".to_owned(),
            face: normal(),
        }])
        .unwrap()
    );
    assert!(
        check1(&[Word {
            text: "🏳️‍⚧️".to_owned(),
            face: normal(),
        }])
        .unwrap()
    );
    assert!(
        !check1(&[Word {
            text: "🏳️‍⚧️🏳️‍⚧️".to_owned(),
            face: normal(),
        }])
        .unwrap()
    );
}

#[test]
fn should_split_str_into_lines_of_width_characters() {
    let cases: [(&str, usize, &[&str]); 14] = [
        ("hello world", 7, &["hello", "world"]),
        ("hello world", 20, &["hello world"]),
        ("hello world", 3, &["hel", "lo", "wor", "ld"]),
        ("hello 12 world", 4, &["hell", "o 12", "worl", "d"]),
        ("hello  1 2 world", 4, &["hell", "o  1", "2", "worl", "d"]),
        ("hello  1 2 world", 6, &["hello", "  1 2", "world"]),
        ("🏳️‍⚧️🏳️‍🌈👩🏾‍❤️‍👨🏻", 0, &["🏳️‍⚧️", "🏳️‍🌈", "👩🏾‍❤️‍👨🏻"]),
        ("🏳️‍⚧️🏳️‍🌈👩🏾‍❤️‍👨🏻", 1, &["🏳️‍⚧️", "🏳️‍🌈", "👩🏾‍❤️‍👨🏻"]),
        ("🏳️‍⚧️🏳️‍🌈👩🏾‍❤️‍👨🏻", 2, &["🏳️‍⚧️🏳️‍🌈", "👩🏾‍❤️‍👨🏻"]),
        ("🏳️‍⚧️🏳️‍🌈👩🏾‍❤️‍👨🏻", 3, &["🏳️‍⚧️🏳️‍🌈👩🏾‍❤️‍👨🏻"]),
        ("中文中", 1, &["中", "文", "中"]),
        ("中文中", 2, &["中文", "中"]),
        ("中文中", 3, &["中文中"]),
        ("Flag 🏳️‍⚧️ this 🏳️‍🌈", 6, &["Flag 🏳️‍⚧️", "this 🏳️‍🌈"]),
    ];
    for (text, width, split) in cases {
        let line = line_from_string(text);
        let lines = split_line_to_fit_width(&line, &mut check_fn(width))
            .unwrap_or_else(|error| panic!("{text:?} at {width}: {error}"));
        assert_eq!(texts(&lines), expected(split), "{text:?} at {width}");
    }
}

#[test]
fn should_handle_strings_with_newlines() {
    let text = "Flag\n  🏳️‍⚧️ this 🏳️‍🌈";
    let line = line_from_string(text);
    let error = split_line_to_fit_width(&line, &mut check_fn(6))
        .expect_err("a word with a newline cannot be split");
    assert_eq!(
        error.to_string(),
        "splitLineToFitWidth does not support newlines in the line"
    );
}

#[test]
#[ignore = "port: the fallback for engines without Intl.Segmenter (code points) is not ported; the Rust port always segments graphemes"]
fn should_split_str_into_characters() {}

#[test]
#[ignore = "port: the fallback for engines without Intl.Segmenter (code points) is not ported; the Rust port always segments graphemes"]
fn should_split_str_into_lines_of_width_characters_without_intl_segmenter() {}
