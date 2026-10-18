//! Port of `packages/mermaid/src/diagrams/usecase/parser/usecaseJson.spec.ts`, with the row cases of
//! `usecase.spec.ts` that `flattenJsonRows` decides (the second part of this file).
//!
//! The reference's `captureJsonError` becomes `fails(…)`, and its `toThrow` becomes `expect_err`. The
//! reference reports a line and a column, which the parser does not: it reports the byte offset of the
//! offending character, so the spec's line and column are checked as the 1-based line and column of
//! that offset (`line_and_column`), with a CRLF counted as one line break. Mapping the line and the
//! column onto the diagram's start is the caller's part, so the spec's start offsets are not asserted.
//!
//! The spec's `it.each` over the roots becomes one loop, and the reference's `Object.keys` check is
//! replaced by the key order of the `Value`: the `Value` keeps its keys sorted, so the integer-like keys
//! come as `1`, `10`, `2`, not in the JavaScript order, and the text order is checked in
//! `property_order` instead.
//!
//! The probes that the reference's `JSON.parse` decides (the grammar, the escapes, the numbers and the
//! errors' positions) are in the middle of the file, checked against Node's `JSON.parse`.

use std::collections::BTreeMap;

use ferrigura_model::use_case::JsonRow;
use ferrigura_model::value::Value;
use ferrigura_syntax::sanitize::sanitize_text;
use ferrigura_syntax::use_case::json::{
    JsonError, OrderedJsonObject, js_number_to_string, json_rows, parse_ordered_json_object,
};
use serde_json::json;

fn parses(text: &str) -> OrderedJsonObject {
    parse_ordered_json_object(text).expect("parses")
}

fn fails(text: &str) -> JsonError {
    parse_ordered_json_object(text).expect_err("fails")
}

/// The property order the spec's literals name: each pointer with its keys.
fn order(entries: Vec<(&str, Vec<&str>)>) -> BTreeMap<String, Vec<String>> {
    entries
        .into_iter()
        .map(|(pointer, keys)| {
            (
                pointer.to_owned(),
                keys.into_iter().map(str::to_owned).collect(),
            )
        })
        .collect()
}

/// The 1-based line and column of a byte offset. A CRLF is one line break, and a lone LF or CR is one
/// too, as the reference's `locationAtOffset` counts them. The texts of these tests are ASCII.
fn line_and_column(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    let mut previous = None;
    for &byte in text.as_bytes().iter().take(offset) {
        match byte {
            b'\r' => {
                line += 1;
                column = 1;
            }
            b'\n' if previous != Some(b'\r') => {
                line += 1;
                column = 1;
            }
            b'\n' => {}
            _ => column += 1,
        }
        previous = Some(byte);
    }
    (line, column)
}

fn number_of(object: &OrderedJsonObject, key: &str) -> f64 {
    object
        .value
        .get(key)
        .and_then(Value::as_f64)
        .expect("a number")
}

fn string_of(object: &OrderedJsonObject, key: &str) -> String {
    object
        .value
        .get(key)
        .and_then(Value::as_str)
        .expect("a string")
        .to_owned()
}

// ---------------------------------------------------------------------------------------------
// The spec's tests, in its order.
// ---------------------------------------------------------------------------------------------

#[test]
fn collects_escaped_json_pointer_order_for_nested_objects_and_objects_inside_arrays() {
    let result = parses(
        r#"{
        "a/b": {
          "items": [
            { "~name": "first", "id": 1 },
            { "id": 2, "details": { "enabled": true } }
          ]
        },
        "last": null
      }"#,
    );

    assert_eq!(
        result.value,
        Value::from(json!({
            "a/b": {
                "items": [
                    { "~name": "first", "id": 1 },
                    { "id": 2, "details": { "enabled": true } }
                ]
            },
            "last": null
        }))
    );
    assert_eq!(
        result.property_order,
        order(vec![
            ("", vec!["a/b", "last"]),
            ("/a~1b", vec!["items"]),
            ("/a~1b/items/0", vec!["~name", "id"]),
            ("/a~1b/items/1", vec!["id", "details"]),
            ("/a~1b/items/1/details", vec!["enabled"]),
        ])
    );
}

#[test]
fn preserves_source_order_for_integer_like_property_names() {
    let result = parses(r#"{"10":"ten","2":"two","1":"one","plain":true}"#);

    // The value keeps its keys sorted, as a `BTreeMap` does.
    let keys: Vec<&str> = result
        .value
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["1", "10", "2", "plain"]);
    assert_eq!(
        result.property_order,
        order(vec![("", vec!["10", "2", "1", "plain"])])
    );
}

#[test]
fn keeps_a_duplicate_key_first_in_order_while_retaining_its_last_value_and_nested_order() {
    // The fourth key is `second` written with an escape, so it repeats the key of the second value.
    let result = parses(
        r#"{
        "entry": { "discarded": { "old": true } },
        "tail": 0,
        "entry": { "second": 2, "first": 1, "\u0073econd": 3 }
      }"#,
    );

    assert_eq!(
        result.value,
        Value::from(json!({ "entry": { "second": 3, "first": 1 }, "tail": 0 }))
    );
    assert_eq!(
        result.property_order,
        order(vec![
            ("", vec!["entry", "tail"]),
            ("/entry", vec!["second", "first"]),
        ])
    );
    assert!(!result.property_order.contains_key("/entry/discarded"));
}

#[test]
fn records_empty_objects_and_accepts_empty_arrays_and_strings_as_property_values() {
    let result = parses(r#"{"emptyObject":{},"emptyArray":[],"emptyString":""}"#);

    assert_eq!(
        result.value,
        Value::from(json!({ "emptyObject": {}, "emptyArray": [], "emptyString": "" }))
    );
    assert_eq!(
        result.property_order,
        order(vec![
            ("", vec!["emptyObject", "emptyArray", "emptyString"]),
            ("/emptyObject", vec![]),
        ])
    );
}

#[test]
fn ignores_braces_brackets_and_escaped_quotes_inside_string_values() {
    let result = parses(
        r#"{"text":"a } and { plus [brackets] and \"quotes\"","nested":{"value":"{\"key\": 1}"}}"#,
    );

    assert_eq!(
        result.value,
        Value::from(json!({
            "text": "a } and { plus [brackets] and \"quotes\"",
            "nested": { "value": "{\"key\": 1}" }
        }))
    );
    assert_eq!(
        result.property_order,
        order(vec![
            ("", vec!["text", "nested"]),
            ("/nested", vec!["value"])
        ])
    );
}

#[test]
fn rejects_a_root_that_is_not_an_object() {
    let roots = [
        ("scalar", "true"),
        ("null", "null"),
        ("array", r#"[{"nested":true}]"#),
        ("empty array", "[]"),
        ("number", "1"),
        ("string", r#""text""#),
        ("false", "false"),
        ("padded null", " \t\n null "),
    ];
    for (description, text) in roots {
        let error = fails(text);
        assert_eq!(
            error.offset, 0,
            "{description}: the root error is at the start"
        );
        assert!(
            error
                .message
                .contains("JSON value must have an object root"),
            "{description}: {}",
            error.message
        );
    }
}

#[test]
fn maps_a_malformed_json_offset_across_physical_lines_into_diagram_coordinates() {
    let text = "{\n  \"valid\": true,\n  \"invalid\":\n}";

    let error = fails(text);

    // The `}` on the fourth line is the offending character: the line is 4 and the column 1.
    assert_eq!(text.as_bytes().get(error.offset), Some(&b'}'));
    assert_eq!(line_and_column(text, error.offset), (4, 1));
    assert!(error.message.contains("unexpected"), "{}", error.message);
}

#[test]
fn adds_the_token_start_column_to_malformed_json_on_its_first_line() {
    let text = r#"{"key":,}"#;

    let error = fails(text);

    // The `,` is the seventh byte, in column 8 of the first line.
    assert_eq!(error.offset, 7);
    assert_eq!(line_and_column(text, error.offset), (1, 8));
}

#[test]
fn maps_crlf_as_one_physical_line_break() {
    let text = "{\r\n  \"key\": true,\r\n}";

    let error = fails(text);

    assert_eq!(text.as_bytes().get(error.offset), Some(&b'}'));
    assert_eq!(line_and_column(text, error.offset), (3, 1));
}

// ---------------------------------------------------------------------------------------------
// Validity, the offsets of the failures, and the values, as `JSON.parse` decides them. The
// offsets are those of Node 24's `JSON.parse` messages (`at position N`), or of the first
// offending character when the message gives no position.
// ---------------------------------------------------------------------------------------------

#[test]
fn rejects_malformed_json_at_the_offending_character() {
    let cases: [(&str, usize); 36] = [
        ("", 0),
        ("  ", 2),
        ("{} x", 3),
        ("{}}", 2),
        ("{\"a\":1,}", 7),
        ("{\"a\":[1,]}", 8),
        ("{\"a\" 1}", 5),
        ("{\"a\":1 \"b\":2}", 7),
        ("{\"a\":1,\"a\"}", 10),
        ("{\"a\":\"\\x\"}", 7),
        ("{\"a\":\"\\u12G4\"}", 10),
        ("{\"a\":\"\\u12\"}", 10),
        ("{\"a\":\"x\ny\"}", 7),
        ("{\"a\":\"\u{1}\"}", 6),
        ("{\"a\":\"abc", 9),
        ("{\"a\":\"\\", 7),
        ("{\"\\q\":1}", 3),
        ("{\"a\":01}", 6),
        ("{\"a\":-}", 6),
        ("{\"a\":1.}", 7),
        ("{\"a\":1e}", 7),
        ("{\"a\":+1}", 5),
        ("{\"a\":.5}", 5),
        ("{\"a\":NaN}", 5),
        ("{\"a\":-Infinity}", 6),
        ("{\"a\":tru}", 8),
        ("{\"a\":trux}", 8),
        ("{\"a\":nul}", 8),
        ("{\"a\":1}}", 7),
        ("\u{feff}{}", 0),
        ("\u{c}{}", 0),
        ("{'a':1}", 1),
        ("{", 1),
        ("[1 x", 3),
        ("true x", 5),
        ("{\"a\":[1,{\"b\":}]}", 13),
    ];
    for (text, offset) in cases {
        assert_eq!(fails(text).offset, offset, "offset of {text:?}");
    }
}

#[test]
fn reports_a_byte_offset_after_multibyte_characters() {
    // The `x` is the eighth byte, after the two bytes of `é`.
    let text = "{\"é\": x}";

    let error = fails(text);

    assert_eq!(error.offset, 7);
    assert_eq!(text.as_bytes().get(error.offset), Some(&b'x'));
}

#[test]
fn reports_an_unterminated_object_or_array_at_the_end() {
    assert_eq!(fails("[1,2").offset, 4);
    assert_eq!(fails("{\"a\":[1,2").offset, 9);
}

#[test]
fn accepts_the_four_json_whitespace_characters_and_nothing_else() {
    let result = parses("\t\r\n {\"a\" \t:\r\n 1 }\r\n");

    assert_eq!(number_of(&result, "a"), 1.0);
    assert_eq!(fails("{\"a\":1}\u{a0}").offset, 7);
}

#[test]
fn accepts_an_empty_object_and_records_it_with_no_keys() {
    let result = parses("{}");

    assert_eq!(result.value, Value::empty_object());
    assert_eq!(result.property_order, order(vec![("", vec![])]));
}

#[test]
fn reads_numbers_as_javascript_reads_them() {
    let result = parses(
        r#"{"neg":-0,"big":12345678901234567890,"huge":1e400,"tiny":1e-400,"exp":-1.5E+2,"frac":123.456e-2,"one":0.1e1}"#,
    );

    assert!(number_of(&result, "neg") == 0.0 && number_of(&result, "neg").is_sign_negative());
    assert_eq!(number_of(&result, "big"), 12345678901234567890.0);
    assert_eq!(number_of(&result, "huge"), f64::INFINITY);
    assert_eq!(number_of(&result, "tiny"), 0.0);
    assert_eq!(number_of(&result, "exp"), -150.0);
    assert_eq!(number_of(&result, "frac"), 1.23456);
    assert_eq!(number_of(&result, "one"), 1.0);

    let negative = parses(r#"{"a":-1e400}"#);
    assert_eq!(number_of(&negative, "a"), f64::NEG_INFINITY);
}

#[test]
fn decodes_every_escape_and_keeps_raw_text() {
    let result = parses(r#"{"a":"\"\\\/\b\f\n\r\t\u0041\u00e9\u00E9","raw":"é😀\u007f"}"#);

    assert_eq!(
        string_of(&result, "a"),
        "\"\\/\u{8}\u{c}\n\r\tA\u{e9}\u{e9}"
    );
    assert_eq!(string_of(&result, "raw"), "é😀\u{7f}");
}

#[test]
fn joins_a_surrogate_pair_and_replaces_a_lone_surrogate_with_the_replacement_character() {
    let pair = parses(r#"{"a":"\uD83D\uDE00"}"#);
    assert_eq!(string_of(&pair, "a"), "😀");

    let lone_high = parses(r#"{"a":"\uD800"}"#);
    assert_eq!(string_of(&lone_high, "a"), "\u{FFFD}");

    let lone_low = parses(r#"{"a":"\uDC00x"}"#);
    assert_eq!(string_of(&lone_low, "a"), "\u{FFFD}x");

    let high_then_plain = parses(r#"{"a":"\uD800\u0041"}"#);
    assert_eq!(string_of(&high_then_plain, "a"), "\u{FFFD}A");

    let two_high = parses(r#"{"a":"\uD800\uD800"}"#);
    assert_eq!(string_of(&two_high, "a"), "\u{FFFD}\u{FFFD}");
}

#[test]
fn reads_an_escaped_key_as_its_decoded_text() {
    let result = parses(r#"{"\u0073econd": 1}"#);

    assert_eq!(result.property_order, order(vec![("", vec!["second"])]));
}

#[test]
fn a_repeated_key_with_a_scalar_drops_the_subtree_of_the_earlier_object() {
    let result = parses(r#"{"a":{"x":1},"a":2}"#);

    assert_eq!(result.value, Value::from(json!({ "a": 2 })));
    assert_eq!(result.property_order, order(vec![("", vec!["a"])]));
}

#[test]
fn a_repeated_key_with_an_object_replaces_the_earlier_subtree() {
    let result = parses(r#"{"a":{"x":1,"deep":{"d":1}},"a":{"y":2}}"#);

    assert_eq!(result.value, Value::from(json!({ "a": { "y": 2 } })));
    assert_eq!(
        result.property_order,
        order(vec![("", vec!["a"]), ("/a", vec!["y"])])
    );
}

#[test]
fn reads_a_proto_key_as_an_ordinary_key() {
    let result = parses(r#"{"__proto__":{"x":1}}"#);

    assert!(result.value.get("__proto__").is_some());
    assert_eq!(
        result.property_order,
        order(vec![("", vec!["__proto__"]), ("/__proto__", vec!["x"])])
    );
}

#[test]
fn escapes_tilde_and_slash_in_pointers_in_that_order() {
    // `~1` is the key `~1`, which is written `~01`; `a/b` is written `a~1b`.
    let result = parses(r#"{"~1":{"b":1,"a":2},"a/b":{"y":2}}"#);

    assert_eq!(
        result.property_order,
        order(vec![
            ("", vec!["~1", "a/b"]),
            ("/~01", vec!["b", "a"]),
            ("/a~1b", vec!["y"]),
        ])
    );
}

#[test]
fn records_objects_inside_nested_arrays_by_their_index_path() {
    let result = parses(r#"{"x":[[{"k":1}]]}"#);

    assert_eq!(
        result.property_order,
        order(vec![("", vec!["x"]), ("/x/0/0", vec!["k"])])
    );
}

#[test]
fn accepts_512_levels_of_nesting_and_rejects_the_513th() {
    let accepted = format!("{{\"a\":{}{}}}", "[".repeat(511), "]".repeat(511));
    parses(&accepted);

    let rejected = format!("{{\"a\":{}{}}}", "[".repeat(512), "]".repeat(512));
    // The 512th array opens at byte 516, the first byte past the 511 arrays before it.
    assert_eq!(fails(&rejected).offset, 516);
}

#[test]
fn reads_each_number_grammar_form_as_javascript_does() {
    let result = parses(r#"{"zero":0,"negzero":-0.0,"exp":5E-3,"plusexp":2e+2}"#);

    assert_eq!(number_of(&result, "zero"), 0.0);
    assert_eq!(number_of(&result, "exp"), 0.005);
    assert_eq!(number_of(&result, "plusexp"), 200.0);
}

// ---------------------------------------------------------------------------------------------
// `String(number)`, the Number-to-String conversion of ECMA-262 (6.1.6.1.20).
// ---------------------------------------------------------------------------------------------

#[test]
fn number_strings_follow_the_ecmascript_number_to_string_rules() {
    let cases: [(f64, &str); 22] = [
        (0.0, "0"),
        (-0.0, "0"),
        (1.0, "1"),
        (0.1, "0.1"),
        (1e-7, "1e-7"),
        (1e-6, "0.000001"),
        (1.5e-7, "1.5e-7"),
        (-1e-7, "-1e-7"),
        (0.000001234, "0.000001234"),
        (100.0, "100"),
        (1e20, "100000000000000000000"),
        (123_456_789_012_345_680_000.0, "123456789012345680000"),
        (1e21, "1e+21"),
        (1.5e21, "1.5e+21"),
        (1.5e300, "1.5e+300"),
        (12_345_678_901_234_567_890.0, "12345678901234567000"),
        (f64::from_bits(1), "5e-324"),
        (f64::MAX, "1.7976931348623157e+308"),
        (9_007_199_254_740_992.0, "9007199254740992"),
        (f64::NAN, "NaN"),
        (f64::INFINITY, "Infinity"),
        (f64::NEG_INFINITY, "-Infinity"),
    ];
    for (number, expected) in cases {
        assert_eq!(js_number_to_string(number), expected, "String({number:e})");
    }
}

// ---------------------------------------------------------------------------------------------
// The rows of a JSON table: `flattenJsonRows` (`usecaseDb.ts` 149-211) and the row cases of
// `usecase.spec.ts`. Each row is (key, accessible key, value).
// ---------------------------------------------------------------------------------------------

fn row(key: &str, accessible_key: &str, value: &str) -> JsonRow {
    JsonRow {
        key: key.to_owned(),
        accessible_key: accessible_key.to_owned(),
        value: value.to_owned(),
    }
}

/// The rows of a JSON text, from its own property order.
fn rows_of(text: &str) -> Vec<JsonRow> {
    let object = parses(text);
    json_rows(&object.value, &object.property_order)
}

#[test]
fn flattens_a_source_ordered_table_with_a_repeated_key_and_an_array_of_scalars() {
    let rows = rows_of(
        r#"{
  "2": "second",
  "1": "first",
  "colors": ["Red", "Green"],
  "items": [{ "name": "A" }, {}],
  "empty": {},
  "duplicate": "old",
  "duplicate": "new"
}"#,
    );

    assert_eq!(
        rows,
        vec![
            row("2", "2", "second"),
            row("1", "1", "first"),
            row("colors", "colors", "Red"),
            row("", "colors", "Green"),
            row("items[0].name", "items[0].name", "A"),
            row("items[1]", "items[1]", "{}"),
            row("empty", "empty", "{}"),
            row("duplicate", "duplicate", "new"),
        ]
    );
}

#[test]
fn flattens_json_rows_depth_first_using_the_source_property_order() {
    let rows = rows_of(
        r#"{"2":"second","1":"first","colors":["Red","Green"],"address":{"city":"Oslo"},"items":[{"name":"A"},{}],"emptyArray":[],"emptyObject":{},"enabled":true,"count":2,"nothing":null}"#,
    );

    assert_eq!(
        rows,
        vec![
            row("2", "2", "second"),
            row("1", "1", "first"),
            row("colors", "colors", "Red"),
            row("", "colors", "Green"),
            row("address.city", "address.city", "Oslo"),
            row("items[0].name", "items[0].name", "A"),
            row("items[1]", "items[1]", "{}"),
            row("emptyArray", "emptyArray", "[]"),
            row("emptyObject", "emptyObject", "{}"),
            row("enabled", "enabled", "true"),
            row("count", "count", "2"),
            row("nothing", "nothing", "null"),
        ]
    );
}

#[test]
fn an_array_of_scalars_is_one_column_with_the_key_on_its_first_row_only() {
    assert_eq!(
        rows_of(r#"{"list":[1,null,"x",true]}"#),
        vec![
            row("list", "list", "1"),
            row("", "list", "null"),
            row("", "list", "x"),
            row("", "list", "true"),
        ]
    );
}

#[test]
fn an_array_with_an_object_is_listed_item_by_item_with_its_index_in_the_path() {
    assert_eq!(
        rows_of(r#"{"mixed":[1,{"a":2}]}"#),
        vec![
            row("mixed[0]", "mixed[0]", "1"),
            row("mixed[1].a", "mixed[1].a", "2")
        ]
    );
}

#[test]
fn a_nested_array_is_not_a_scalar_so_it_is_listed_item_by_item() {
    assert_eq!(
        rows_of(r#"{"grid":[[1,2],[3]],"e":[[]]}"#),
        vec![
            row("grid[0]", "grid[0]", "1"),
            row("", "grid[0]", "2"),
            row("grid[1]", "grid[1]", "3"),
            row("e[0]", "e[0]", "[]"),
        ]
    );
}

#[test]
fn an_empty_root_is_one_row_with_an_empty_key() {
    assert_eq!(rows_of("{}"), vec![row("", "", "{}")]);
}

#[test]
fn an_empty_array_and_an_empty_object_are_each_one_row() {
    assert_eq!(
        rows_of(r#"{"a":[],"b":{}}"#),
        vec![row("a", "a", "[]"), row("b", "b", "{}")]
    );
}

#[test]
fn a_repeated_key_lists_its_last_value_at_the_first_position() {
    assert_eq!(
        rows_of(r#"{"a":1,"b":{"x":1},"a":{"y":2}}"#),
        vec![row("a.y", "a.y", "2"), row("b.x", "b.x", "1")]
    );
    assert_eq!(
        rows_of(r#"{"a":{"x":{}},"a":"s"}"#),
        vec![row("a", "a", "s")]
    );
}

#[test]
fn numbers_and_literals_are_written_as_javascript_writes_them() {
    assert_eq!(
        rows_of(
            r#"{"n":1e21,"m":0.1,"z":-0,"big":12345678901234567890,"inf":1e400,"ninf":-1e400,"f":1.5,"t":true}"#
        ),
        vec![
            row("n", "n", "1e+21"),
            row("m", "m", "0.1"),
            row("z", "z", "0"),
            row("big", "big", "12345678901234567000"),
            row("inf", "inf", "Infinity"),
            row("ninf", "ninf", "-Infinity"),
            row("f", "f", "1.5"),
            row("t", "t", "true"),
        ]
    );
}

#[test]
fn a_pointer_with_escapes_finds_its_property_order() {
    assert_eq!(
        rows_of(r#"{"~1":{"b":1,"a":2},"x/y":{"k":[{"j":true}]}}"#),
        vec![
            row("~1.b", "~1.b", "1"),
            row("~1.a", "~1.a", "2"),
            row("x/y.k[0].j", "x/y.k[0].j", "true"),
        ]
    );
}

#[test]
fn every_cell_of_a_row_is_sanitised() {
    let key = "<b>k</b>";
    let raw_value = "<script>alert(1)</script>x";
    let text = format!("{{\"{key}\": \"{raw_value}\"}}");

    let rows = rows_of(&text);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, sanitize_text(key));
    assert_eq!(rows[0].accessible_key, sanitize_text(key));
    assert_eq!(rows[0].value, sanitize_text(raw_value));
    assert!(!rows[0].value.contains("<script"));
}

#[test]
fn without_a_property_order_an_object_lists_its_keys_in_sorted_order() {
    let value = Value::from(json!({ "b": 1, "a": 2 }));

    assert_eq!(
        json_rows(&value, &BTreeMap::new()),
        vec![row("a", "a", "2"), row("b", "b", "1")]
    );
}

#[test]
fn a_key_in_the_property_order_without_a_value_is_written_as_undefined() {
    let value = Value::from(json!({ "a": 1 }));
    let property_order = order(vec![("", vec!["ghost", "a"])]);

    assert_eq!(
        json_rows(&value, &property_order),
        vec![row("ghost", "ghost", "undefined"), row("a", "a", "1")]
    );
}
