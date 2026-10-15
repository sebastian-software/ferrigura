//! Port of `packages/parser/tests/info.test.ts`. Each `it.each` table is one test that runs every
//! input of the table, under the table's title.

use ferrigura_syntax::info::parse;

#[test]
fn should_handle_empty_info() {
    for input in ["info", "\n    info", "info\n    ", "\n    info\n    "] {
        parse(input).unwrap_or_else(|error| panic!("{input:?} should parse: {error}"));
    }
}

#[test]
fn should_handle_show_info() {
    for input in [
        "info showInfo",
        "info showInfo\n    ",
        "\n    info showInfo",
        "info\n    showInfo",
        "info\n    showInfo\n    ",
        "\n    info\n    showInfo\n    ",
        "\n    info\n    showInfo",
        "\n    info showInfo\n    ",
    ] {
        parse(input).unwrap_or_else(|error| panic!("{input:?} should parse: {error}"));
    }
}
