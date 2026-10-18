//! Port of `packages/mermaid/src/diagrams/usecase/usecase.parser.spec.ts`, its `describe` of the
//! publication and the normalised AST (line 102 on), against `ferrigura_syntax::use_case::parse`.
//!
//! What the port changes, and why:
//! - The reference's DB is a singleton that the parser fills and `db.clear()` empties. `parse`
//!   returns a model or an error and keeps no state, so `expectEmptyPublication` and
//!   `expectRejectedWithoutPublication` are the `Err` of a rejected text. The `clear` steps have no
//!   counterpart, and the `toBeDefined` checks on the AST are not asserted: the model always has one.
//! - A rejection is asserted as `Err`. The reference's message texts are not asserted, because the
//!   errors are ours. Where the spec gives the location of an error, the `ParseError` must have the
//!   line and column of the same token (`located`). A `ParseError` has one position, so of a message
//!   with a second location (`previous declaration at`) only the first, the one the message names
//!   with `at`, is checked.
//! - `toEqual` and `toMatchObject` are `assert_json_equal` and `assert_json_matches` on the JSON of
//!   the model: the serde names, the arrays in the model's order, numbers compared as numbers.
//! - The DB's maps are the model's vectors in declaration order. The spec's `publishedModel` sorts
//!   the maps by key, so `published` sorts them by id before two parses are compared.
//! - `parse` runs the preprocessing of `Diagram.fromText`, and the spec calls the Chevrotain parser
//!   directly. So the AST's `source` ends with the line break that `Diagram.fromText` appends, the
//!   `%%` lines and the blank line before them are removed before the scanner (they are no
//!   statements), and the `#name;` codes are the placeholders of `encodeEntities`. The tests that
//!   depend on this say so.

use ferrigura_model::use_case::{
    BoundaryType, Direction, GraphAst, LabelType, UseCaseDiagram, UseCaseShape,
};
use ferrigura_syntax::ParseError;
use ferrigura_syntax::use_case::parse;
use serde_json::{Value, json};

/// The parse of a text that the spec expects to be accepted.
#[track_caller]
fn parsed(source: &str) -> UseCaseDiagram {
    parse(source).unwrap_or_else(|error| panic!("the text is rejected: {error}\n{source}"))
}

/// The error of a text that the spec expects to be rejected.
#[track_caller]
fn rejected(source: &str) -> ParseError {
    match parse(source) {
        Ok(_) => panic!("the text is accepted, a rejection is expected:\n{source}"),
        Err(error) => error,
    }
}

/// The line and the column of byte `offset` of `text`, as a `ParseError` counts them: 1-based, the
/// column in characters.
fn line_column(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
    (line, before[line_start..].chars().count() + 1)
}

/// The line and the column of the `occurrence`th `text` of `source`, moved `trim` bytes to the
/// right: the spec's `occurrenceLocation`, the position a `ParseError` must have.
#[track_caller]
fn located(source: &str, text: &str, occurrence: usize, trim: usize) -> (usize, usize) {
    let mut from = 0;
    let mut start = 0;
    for _ in 0..occurrence {
        start = from
            + source[from..]
                .find(text)
                .unwrap_or_else(|| panic!("'{text}' does not occur {occurrence} time(s)"));
        from = start + 1;
    }
    line_column(source, start + trim)
}

/// The rejection of `source` has the position `at`: the line and the column of its offending token.
#[track_caller]
fn expect_rejected_at(source: &str, at: (usize, usize)) {
    let error = rejected(source);
    assert_eq!(
        (error.line, error.column),
        at,
        "the error is {error}\n{source}"
    );
}

/// The rejection of `source` in the case `what`: a failure names the case that was accepted.
#[track_caller]
fn rejected_case(what: &str, source: &str) -> ParseError {
    match parse(source) {
        Ok(_) => panic!("{what}: the text is accepted, a rejection is expected\n{source}"),
        Err(error) => error,
    }
}

/// The rejection of `source` in the case `what` has the position `at`.
#[track_caller]
fn expect_rejected_case_at(what: &str, source: &str, at: (usize, usize)) {
    let error = rejected_case(what, source);
    assert_eq!(
        (error.line, error.column),
        at,
        "{what}: the error is {error}\n{source}"
    );
}

/// The publication the spec compares (`publishedModel`): each map sorted by id, the relationships in
/// their order, the AST left out.
fn published(model: &UseCaseDiagram) -> UseCaseDiagram {
    let mut published = model.clone();
    published
        .actors
        .sort_by(|left, right| left.id.cmp(&right.id));
    published
        .use_cases
        .sort_by(|left, right| left.id.cmp(&right.id));
    published
        .system_boundaries
        .sort_by(|left, right| left.id.cmp(&right.id));
    published
        .notes
        .sort_by(|left, right| left.id.cmp(&right.id));
    published
        .json_nodes
        .sort_by(|left, right| left.id.cmp(&right.id));
    published
        .class_defs
        .sort_by(|left, right| left.id.cmp(&right.id));
    published.ast = GraphAst::empty();
    published
}

/// The AST of a model as JSON, with the serde names.
fn ast_json(model: &UseCaseDiagram) -> Value {
    serde_json::to_value(&model.ast).expect("the AST serialises")
}

/// The statements of a model as JSON.
fn statements_json(model: &UseCaseDiagram) -> Value {
    serde_json::to_value(&model.ast.statements).expect("the statements serialise")
}

/// The items of a JSON array, as the spec's destructuring reads them: only the `keys` that are
/// present. An absent key is `undefined` in the spec, which `toEqual` ignores.
fn pick(item: &Value, keys: &[&str]) -> Value {
    Value::Object(
        keys.iter()
            .filter_map(|key| {
                item.get(*key)
                    .map(|value| ((*key).to_owned(), value.clone()))
            })
            .collect(),
    )
}

/// The text the AST is read from when the spec's source has no comment: the line break that
/// `Diagram.fromText` appends.
fn prepared(source: &str) -> String {
    format!("{source}\n")
}

/// The first difference between `actual` and `expected`. `partial` is `toMatchObject`: an object of
/// `expected` is a subset of the object of `actual`. Without it, `toEqual`: the same keys. Arrays
/// have the same length in both, and numbers compare as numbers.
fn difference(actual: &Value, expected: &Value, partial: bool, path: &str) -> Option<String> {
    match (actual, expected) {
        (Value::Object(actual_map), Value::Object(expected_map)) => {
            for (key, expected_item) in expected_map {
                let here = format!("{path}.{key}");
                match actual_map.get(key) {
                    Some(item) => {
                        if let Some(found) = difference(item, expected_item, partial, &here) {
                            return Some(found);
                        }
                    }
                    None => return Some(format!("{here} is missing")),
                }
            }
            if !partial
                && let Some(key) = actual_map
                    .keys()
                    .find(|key| !expected_map.contains_key(*key))
            {
                return Some(format!("{path}.{key} is not expected"));
            }
            None
        }
        (Value::Array(actual_items), Value::Array(expected_items)) => {
            if actual_items.len() != expected_items.len() {
                return Some(format!(
                    "{path} has {} items, expected {}",
                    actual_items.len(),
                    expected_items.len()
                ));
            }
            actual_items
                .iter()
                .zip(expected_items)
                .enumerate()
                .find_map(|(index, (item, expected_item))| {
                    difference(item, expected_item, partial, &format!("{path}[{index}]"))
                })
        }
        (Value::Number(actual_number), Value::Number(expected_number))
            if actual_number.as_f64() == expected_number.as_f64() =>
        {
            None
        }
        _ if actual == expected => None,
        _ => Some(format!("{path}: {actual} is not {expected}")),
    }
}

/// `expect(actual).toEqual(expected)` on JSON.
#[track_caller]
fn assert_json_equal(actual: &Value, expected: &Value) {
    if let Some(found) = difference(actual, expected, false, "$") {
        panic!("{found}\nactual: {actual:#}");
    }
}

/// `expect(actual).toMatchObject(expected)` on JSON.
#[track_caller]
fn assert_json_matches(actual: &Value, expected: &Value) {
    if let Some(found) = difference(actual, expected, true, "$") {
        panic!("{found}\nactual: {actual:#}");
    }
}

#[track_caller]
fn actor<'a>(model: &'a UseCaseDiagram, id: &str) -> &'a ferrigura_model::use_case::Actor {
    model
        .actors
        .iter()
        .find(|actor| actor.id == id)
        .unwrap_or_else(|| panic!("no actor '{id}'"))
}

#[track_caller]
fn use_case<'a>(model: &'a UseCaseDiagram, id: &str) -> &'a ferrigura_model::use_case::UseCase {
    model
        .use_cases
        .iter()
        .find(|use_case| use_case.id == id)
        .unwrap_or_else(|| panic!("no use case '{id}'"))
}

#[track_caller]
fn boundary<'a>(
    model: &'a UseCaseDiagram,
    id: &str,
) -> &'a ferrigura_model::use_case::SystemBoundary {
    model
        .system_boundaries
        .iter()
        .find(|boundary| boundary.id == id)
        .unwrap_or_else(|| panic!("no system boundary '{id}'"))
}

fn ids_of<'a>(items: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    items.collect()
}

#[test]
fn accepts_the_header_at_eof_and_publishes_an_empty_ast() {
    let model = parsed("usecase-beta");

    assert!(model.actors.is_empty());
    assert!(model.use_cases.is_empty());
    assert!(model.system_boundaries.is_empty());
    assert!(model.relationships.is_empty());
    assert!(model.notes.is_empty());
    assert!(model.json_nodes.is_empty());
    assert!(model.class_defs.is_empty());
    assert_eq!(model.direction, Direction::Lr);
    assert_json_equal(
        &ast_json(&model),
        &json!({
            "version": 1,
            "diagramType": "usecase",
            "source": prepared("usecase-beta"),
            "header": { "keyword": "usecase", "direction": "LR", "span": [0, 12] },
            "nodes": {},
            "edges": [],
            "groups": {},
            "classDefs": {},
            "statements": [],
        }),
    );
}

#[test]
fn publishes_accessibility_text_through_the_common_db_and_the_normalised_ast() {
    let source = [
        "usecase-beta",
        "accTitle: Authentication use cases",
        "accDescr {",
        "  First line",
        "  Second line",
        "}",
        "actor User",
    ]
    .join("\n");
    let model = parsed(&source);

    assert_eq!(model.acc_title, "Authentication use cases");
    assert_eq!(model.acc_description, "First line\nSecond line");
    let acc_title_end =
        source.find("accTitle").expect("accTitle") + "accTitle: Authentication use cases".len();
    assert_json_matches(
        &ast_json(&model),
        &json!({
            "accTitle": "Authentication use cases",
            "accDescr": "First line\n  Second line",
            "statements": [
                {
                    "kind": "accTitle",
                    "span": [source.find("accTitle").expect("accTitle"), acc_title_end],
                },
                {
                    "kind": "accDescr",
                    "span": [source.find("accDescr").expect("accDescr"), source.find('}').expect("}") + 1],
                },
                {
                    "kind": "node",
                    "span": [source.find("actor User").expect("actor User"), source.len()],
                },
            ],
        }),
    );

    let draft = "usecase-beta\naccTitle: Draft title\naccDescr: Draft description\nnote for Missing \"invalid\"";
    rejected(draft);
}

#[test]
fn accepts_a_final_statement_without_a_newline_and_keeps_end_exclusive_source_spans() {
    let source = "usecase-beta\nactor User";
    let model = parsed(source);

    assert_eq!(actor(&model, "User").id, "User");
    assert_eq!(actor(&model, "User").label, "User");
    assert_eq!(model.ast.source, prepared(source));
    assert_json_equal(
        &statements_json(&model),
        &json!([
            {
                "kind": "node",
                "span": [13, 23],
                "nodes": [
                    {
                        "id": "User",
                        "span": [19, 23],
                        "idSpan": [19, 23],
                        "labelSpan": [19, 23],
                        "defines": true,
                    },
                ],
            },
        ]),
    );
}

#[test]
fn isolates_success_failure_and_a_second_success_across_parses() {
    let first = parsed("usecase-beta\nactor First\nFirst --> Login\nnote for Login \"First note\"");
    assert_eq!(
        ids_of(first.actors.iter().map(|a| a.id.as_str())),
        ["First"]
    );
    assert_eq!(
        ids_of(first.use_cases.iter().map(|u| u.id.as_str())),
        ["Login"]
    );
    assert_eq!(
        ids_of(first.relationships.iter().map(|r| r.id.as_str())),
        ["edge-0"]
    );
    assert_eq!(
        ids_of(first.notes.iter().map(|n| n.id.as_str())),
        ["note-0"]
    );

    // The spec's `db.clear()` has no counterpart: a parse keeps no state to clear.

    rejected("usecase-beta\nactor Draft\nDraft --> Pending\nnote for Missing \"invalid\"");

    let second =
        parsed("usecase-beta\nactor Second\nSecond --> Done\nnote for Done \"Second note\"");
    assert_eq!(
        ids_of(second.actors.iter().map(|a| a.id.as_str())),
        ["Second"]
    );
    assert_eq!(
        ids_of(second.use_cases.iter().map(|u| u.id.as_str())),
        ["Done"]
    );
    assert_eq!(
        ids_of(second.relationships.iter().map(|r| r.id.as_str())),
        ["edge-0"]
    );
    assert_eq!(
        ids_of(second.notes.iter().map(|n| n.id.as_str())),
        ["note-0"]
    );
    assert!(second.ast.source.contains("Second"));
    assert!(!second.ast.source.contains("First"));
    assert!(!second.ast.source.contains("Draft"));
}

#[test]
fn resolves_forward_refinements_independently_of_declaration_order() {
    let cases = [
        ("actor", "actor User", Some("User"), UseCaseShape::Ellipse),
        (
            "rectangular use case",
            "Login[Sign in]",
            None,
            UseCaseShape::Rect,
        ),
    ];
    for (refinement, declaration, expected_actor, shape) in cases {
        let relation = "User --> Login";
        let declaration_first = parsed(&format!("usecase-beta\n{declaration}\n{relation}"));
        let relation_first = parsed(&format!("usecase-beta\n{relation}\n{declaration}"));

        assert_eq!(
            published(&relation_first),
            published(&declaration_first),
            "{refinement}"
        );
        assert_eq!(
            ids_of(relation_first.actors.iter().map(|a| a.id.as_str())),
            expected_actor.into_iter().collect::<Vec<_>>(),
            "{refinement}"
        );
        assert_eq!(
            use_case(&relation_first, "Login").shape,
            shape,
            "{refinement}"
        );
    }
}

#[test]
fn finalizes_unresolved_relation_endpoints_as_ellipse_use_cases_without_actor_inference() {
    let model = parsed("usecase-beta\nLeft --> Right");

    assert!(model.actors.is_empty());
    assert_json_equal(
        &serde_json::to_value(&model.use_cases).expect("the use cases serialise"),
        &json!([
            {
                "id": "Left",
                "label": "Left",
                "labelType": "text",
                "shape": "ellipse",
                "business": false,
                "classes": [],
                "styles": [],
            },
            {
                "id": "Right",
                "label": "Right",
                "labelType": "text",
                "shape": "ellipse",
                "business": false,
                "classes": [],
                "styles": [],
            },
        ]),
    );
}

#[test]
fn merges_equivalent_repeated_declarations_including_equivalent_parent_ownership() {
    let model = parsed(
        &[
            "usecase-beta",
            "systemBoundary Auth",
            "actor User(\"Person\")@{ type: hollow } <<Human>>",
            "actor User(\"Person\")@{ type: hollow } <<Human>>",
            "Login[Sign in] <<Primary>>",
            "Login[Sign in] <<Primary>>",
            "end",
        ]
        .join("\n"),
    );

    assert_json_equal(
        &serde_json::to_value(&model.actors).expect("the actors serialise"),
        &json!([
            {
                "id": "User",
                "label": "Person",
                "labelType": "text",
                "type": "hollow",
                "business": false,
                "stereotype": "Human",
                "parentId": "Auth",
                "classes": [],
                "styles": [],
            },
        ]),
    );
    assert_json_equal(
        &serde_json::to_value(&model.use_cases).expect("the use cases serialise"),
        &json!([
            {
                "id": "Login",
                "label": "Sign in",
                "labelType": "text",
                "shape": "rect",
                "business": false,
                "stereotype": "Primary",
                "parentId": "Auth",
                "classes": [],
                "styles": [],
            },
        ]),
    );
    assert_eq!(boundary(&model, "Auth").members, ["User", "Login"]);
}

#[test]
fn rejects_conflicting_declarations_transactionally() {
    let cases = [
        ("kind", "actor Shared\nShared"),
        ("shape", "Login(Sign in)\nLogin[Sign in]"),
        ("label", "Login(First label)\nLogin(Second label)"),
        ("stereotype", "Login <<Primary>>\nLogin <<Secondary>>"),
        (
            "parent",
            "systemBoundary First\nLogin\nend\nsystemBoundary Second\nLogin\nend",
        ),
    ];
    for (conflict, body) in cases {
        rejected_case(conflict, &format!("usecase-beta\n{body}"));
    }
}

#[test]
fn rejects_an_id_collision_in_the_diagram_global_namespace() {
    let cases = [
        ("generated ID", "\"A-B\"\n\"A B\""),
        ("generated and explicit ID", "\"A B\"\nA_B(Explicit)"),
        ("global cross-kind ID", "actor Shared\njson Shared@{}"),
        ("element and explicit edge ID", "actor link\nA link@--> B"),
        ("duplicate explicit edge ID", "A link@--> B\nB link@--> C"),
    ];
    for (collision, body) in cases {
        rejected_case(collision, &format!("usecase-beta\n{body}"));
    }
}

#[test]
fn rejects_an_unresolved_note_class_style_or_metadata_target() {
    let cases = [
        "note for Ghost \"Missing\"",
        "class Ghost important",
        "style Ghost fill:red",
        "Ghost@{ type: package }",
    ];
    for statement in cases {
        rejected_case(statement, &format!("usecase-beta\n{statement}"));
    }
}

#[test]
fn publishes_nothing_when_final_semantic_resolution_fails() {
    let previous = parsed("usecase-beta\nactor Previous\nPrevious --> Published");
    assert_eq!(previous.use_cases.len(), 1);

    rejected(
        &[
            "usecase-beta",
            "direction TB",
            "classDef important fill:red",
            "actor User:::important",
            "Login[Sign in]",
            "User link@--> Login",
            "link@{ animation: fast }",
            "json Payload@{\"ok\": true}",
            "note for Missing \"late failure\"",
        ]
        .join("\n"),
    );
}

#[test]
fn allocates_anonymous_edge_and_note_ids_deterministically_in_source_order() {
    let source = [
        "usecase-beta",
        "actor User",
        "User --> Login",
        "note for User \"first\"",
        "Login --> Logout",
        "note for Logout \"second\"",
    ]
    .join("\n");

    for _ in 0..2 {
        let model = parsed(&source);
        assert_eq!(
            ids_of(model.relationships.iter().map(|r| r.id.as_str())),
            ["edge-0", "edge-1"]
        );
        assert_eq!(
            ids_of(model.notes.iter().map(|n| n.id.as_str())),
            ["note-0", "note-1"]
        );
    }
}

#[test]
fn distinguishes_generated_and_explicit_ids_and_records_their_exact_occurrences() {
    let source = "usecase-beta\n\"Reset password\"\nLogin(\"Reset password\")";
    let model = parsed(source);

    assert_eq!(
        ids_of(model.use_cases.iter().map(|u| u.id.as_str())),
        ["Reset_password", "Login"]
    );
    let quoted = source.find("\"Reset password\"").expect("the quoted label");
    let login = source.find("Login").expect("Login");
    let second_label = source
        .rfind("\"Reset password\"")
        .expect("the second label");
    assert_json_matches(
        &ast_json(&model),
        &json!({
            "nodes": {
                "Reset_password": {
                    "label": "Reset password",
                    "shape": "ellipse",
                    "attrs": { "kind": "usecase", "labelType": "text" },
                },
                "Login": {
                    "label": "Reset password",
                    "shape": "ellipse",
                    "attrs": { "kind": "usecase", "labelType": "text" },
                },
            },
        }),
    );
    assert_json_matches(
        &statements_json(&model),
        &json!([
            {
                "nodes": [
                    {
                        "id": "Reset_password",
                        "idSpan": [quoted + 1, quoted + 15],
                        "labelSpan": [quoted + 1, quoted + 15],
                    },
                ],
            },
            {
                "nodes": [
                    {
                        "id": "Login",
                        "idSpan": [login, login + 5],
                        "labelSpan": [second_label + 1, source.len() - 2],
                    },
                ],
            },
        ]),
    );
}

#[test]
fn names_exact_locations_for_generated_and_explicit_id_collisions() {
    let generated = "usecase-beta\n\"A-B\"\n\"A B\"";
    expect_rejected_at(generated, located(generated, "\"A B\"", 1, 1));

    let generated_and_explicit = "usecase-beta\n\"A B\"\nA_B(Explicit)";
    expect_rejected_at(
        generated_and_explicit,
        located(generated_and_explicit, "A_B", 1, 0),
    );

    let generated_kinds = "usecase-beta\nactor \"A-B\"\n\"A B\"";
    expect_rejected_at(generated_kinds, located(generated_kinds, "\"A B\"", 1, 1));

    let generated_boundaries =
        "usecase-beta\nsystemBoundary \"A-B\"\nend\nsystemBoundary \"A B\"\nend";
    expect_rejected_at(
        generated_boundaries,
        located(generated_boundaries, "\"A B\"", 1, 1),
    );

    let explicit_edges = "usecase-beta\nA link@--> B\nB link@--> C";
    expect_rejected_at(explicit_edges, located(explicit_edges, "link", 2, 0));
}

#[test]
fn preserves_entity_codes_and_literal_backslash_n_and_accepts_newlines_only_in_markdown_labels() {
    let source = [
        "usecase-beta",
        "classDef decorated fill:#fee",
        "actor User(\"`**User**",
        "role`\") <<Human>>:::decorated",
        "Login(\"`Sign",
        "in`\") <<Primary>>:::decorated",
        "Escaped(\"Open #40;safe#41; #91;now#93;\")",
        "Literal(\"First\\nSecond\")",
    ]
    .join("\n");
    let model = parsed(&source);

    let user = actor(&model, "User");
    assert_eq!(user.label, "**User**\nrole");
    assert_eq!(user.label_type, LabelType::Markdown);
    assert_eq!(user.stereotype.as_deref(), Some("Human"));
    assert_eq!(user.classes, ["decorated"]);

    let login = use_case(&model, "Login");
    assert_eq!(login.label, "Sign\nin");
    assert_eq!(login.label_type, LabelType::Markdown);
    assert_eq!(login.stereotype.as_deref(), Some("Primary"));
    assert_eq!(login.classes, ["decorated"]);

    // `encodeEntities` writes `#40;` as the placeholder `ﬂ°°40¶ß`, which the DB keeps: the reference's
    // DB keeps the placeholders too (the renderer decodes them), and this is the text it publishes.
    assert_eq!(
        use_case(&model, "Escaped").label,
        "Open ﬂ°°40¶ßsafeﬂ°°41¶ß ﬂ°°91¶ßnowﬂ°°93¶ß"
    );
    assert_eq!(use_case(&model, "Literal").label, "First\\nSecond");
    assert_json_matches(
        &ast_json(&model),
        &json!({
            "nodes": {
                "User": {
                    "label": "**User**\nrole",
                    "classes": ["decorated"],
                    "attrs": { "labelType": "markdown", "stereotype": "Human" },
                },
                "Login": {
                    "label": "Sign\nin",
                    "classes": ["decorated"],
                    "attrs": { "labelType": "markdown", "stereotype": "Primary" },
                },
                "Escaped": {
                    "label": "Open ﬂ°°40¶ßsafeﬂ°°41¶ß ﬂ°°91¶ßnowﬂ°°93¶ß",
                    "attrs": { "labelType": "text" },
                },
                "Literal": { "label": "First\\nSecond", "attrs": { "labelType": "text" } },
            },
        }),
    );

    rejected(&["usecase-beta", "Broken(\"First", "Second\")"].join("\n"));
}

#[test]
fn accepts_markdown_markers_punctuation_and_non_ascii_characters_in_unquoted_labels() {
    let source = [
        "usecase-beta",
        "actor Reviewer(*Reviewer*)",
        "Literal(**Literal markers**)",
        "Priced[Costs 19.95 EUR, incl. VAT!]",
        "Progress(Ärende — 50% klart)",
        "Reviewer -- opens **form** --> Literal",
        "Reviewer --> Priced",
        "Reviewer --> Progress",
        "note for Literal Read the guide; then sign off.",
    ]
    .join("\n");
    let model = parsed(&source);

    assert_eq!(actor(&model, "Reviewer").label, "*Reviewer*");
    assert_eq!(use_case(&model, "Literal").label, "**Literal markers**");
    assert_eq!(
        use_case(&model, "Priced").label,
        "Costs 19.95 EUR, incl. VAT!"
    );
    assert_eq!(use_case(&model, "Progress").label, "Ärende — 50% klart");
    assert_eq!(
        model.relationships[0].label.as_deref(),
        Some("opens **form**")
    );
    assert_eq!(model.notes[0].label, "Read the guide; then sign off.");
    assert_json_matches(
        &ast_json(&model),
        &json!({
            "nodes": {
                "Reviewer": { "label": "*Reviewer*", "attrs": { "labelType": "text" } },
                "Literal": { "label": "**Literal markers**", "attrs": { "labelType": "text" } },
            },
        }),
    );
}

#[test]
fn still_rejects_a_grammar_token_inside_an_unquoted_label() {
    let cases = [
        ("a parenthesis", "Literal(Literal (nested) markers)", "(", 2),
        ("a bracket", "Literal[Literal [nested] markers]", "[", 2),
        ("a brace", "Literal(Literal {nested} markers)", "{", 1),
        (
            "a relationship operator",
            "Literal(Literal -- markers)",
            "--",
            1,
        ),
        ("the class suffix", "Literal(Literal ::: markers)", ":::", 1),
        (
            "the metadata suffix",
            "Literal(Literal @{ markers)",
            "@{",
            1,
        ),
    ];
    for (name, statement, token, occurrence) in cases {
        let source = format!("usecase-beta\n{statement}");
        expect_rejected_case_at(name, &source, located(&source, token, occurrence, 0));
    }
}

#[test]
fn still_rejects_a_quote_inside_an_unquoted_label() {
    for statement in ["Literal(Literal \"markers)", "Literal(Reviewer's markers)"] {
        rejected_case(statement, &format!("usecase-beta\n{statement}"));
    }
}

#[test]
fn reports_exact_actor_metadata_and_incompatible_icon_locations() {
    let invalid_type = "usecase-beta\nactor User@{ type: giant }";
    expect_rejected_at(invalid_type, located(invalid_type, "type", 1, 0));

    let invalid_key = "usecase-beta\nactor User@{ fillColor: red }";
    expect_rejected_at(invalid_key, located(invalid_key, "fillColor", 1, 0));

    let icon_and_awesome = "usecase-beta\nactor User@{ icon: \"fa:user\", type: awesome }";
    expect_rejected_at(icon_and_awesome, located(icon_and_awesome, "User", 1, 0));
}

#[test]
fn rejects_mixed_kind_generalization_and_actor_include_at_the_relation_span() {
    let generalization = "usecase-beta\nactor User\nLogin\nUser --|> Login";
    expect_rejected_at(generalization, located(generalization, "--|> Login", 1, 0));

    let actor_include = "usecase-beta\nactor User\nactor Admin\nUser ..> : include Admin";
    expect_rejected_at(
        actor_include,
        located(actor_include, "..> : include Admin", 1, 0),
    );
}

#[test]
fn rejects_duplicate_and_unknown_edge_class_and_style_targets_with_locations() {
    let duplicate = "usecase-beta\nA link@--> B\nB link@--> C";
    expect_rejected_at(duplicate, located(duplicate, "link", 2, 0));

    let unknown_class = "usecase-beta\nA known@--> B\nclass missingEdge decorated";
    expect_rejected_at(unknown_class, located(unknown_class, "missingEdge", 1, 0));

    let unknown_style = "usecase-beta\nA known@--> B\nstyle missingEdge stroke:red";
    expect_rejected_at(unknown_style, located(unknown_style, "missingEdge", 1, 0));
}

#[test]
fn publishes_exact_animation_state_and_rejects_invalid_values() {
    let source = [
        "usecase-beta",
        "A trueEdge@--> B",
        "trueEdge@{ animate: true }",
        "A fastEdge@--> B",
        "fastEdge@{ animation: fast }",
        "A slowEdge@--> B",
        "slowEdge@{ animation: slow }",
        "A offEdge@--> B",
        "offEdge@{ animate: false }",
    ]
    .join("\n");
    let model = parsed(&source);

    let expected = json!([
        { "id": "trueEdge", "animate": true },
        { "id": "fastEdge", "animate": true, "animation": "fast" },
        { "id": "slowEdge", "animate": true, "animation": "slow" },
        { "id": "offEdge", "animate": false },
    ]);
    let published_edges = serde_json::to_value(&model.relationships).expect("the edges serialise");
    let published_edges: Vec<Value> = published_edges
        .as_array()
        .expect("an array")
        .iter()
        .map(|edge| pick(edge, &["id", "animate", "animation"]))
        .collect();
    assert_json_equal(&Value::Array(published_edges), &expected);

    let ast_edges: Vec<Value> = ast_json(&model)["edges"]
        .as_array()
        .expect("the AST edges")
        .iter()
        .map(|edge| {
            let mut state = json!({ "id": edge["id"] });
            for key in ["animate", "animation"] {
                if let Some(value) = edge["attrs"].get(key) {
                    state[key] = value.clone();
                }
            }
            state
        })
        .collect();
    assert_json_equal(&Value::Array(ast_edges), &expected);

    let invalid_animation = "usecase-beta\nA link@--> B\nlink@{ animation: medium }";
    expect_rejected_at(
        invalid_animation,
        located(invalid_animation, "animation", 1, 0),
    );

    let invalid_animate = "usecase-beta\nA link@--> B\nlink@{ animate: fast }";
    expect_rejected_at(invalid_animate, located(invalid_animate, "animate", 1, 0));
}

#[test]
fn locates_rejected_content_inside_a_boundary() {
    let cases = [
        ("relation", "User --> Login", "-->", 1),
        ("note", "note for User \"inside\"", "note", 1),
        ("JSON", "json Payload@{}", "json Payload@", 1),
        (
            "nested boundary",
            "systemBoundary Nested\nend",
            "systemBoundary",
            2,
        ),
    ];
    for (kind, statement, unexpected, occurrence) in cases {
        let source = format!("usecase-beta\nsystemBoundary Auth\n{statement}\nend");
        expect_rejected_case_at(kind, &source, located(&source, unexpected, occurrence, 0));
    }
}

#[test]
fn rejects_notes_on_json_boundary_and_edge_ids_with_locations() {
    let json = "usecase-beta\njson Payload@{}\nnote for Payload \"invalid\"";
    expect_rejected_at(json, located(json, "Payload", 2, 0));

    let boundary = "usecase-beta\nsystemBoundary Auth\nLogin\nend\nnote for Auth \"invalid\"";
    expect_rejected_at(boundary, located(boundary, "Auth", 2, 0));

    let edge = "usecase-beta\nA link@--> B\nnote for link \"invalid\"";
    expect_rejected_at(edge, located(edge, "link", 2, 0));
}

#[test]
fn locates_a_stereotype_on_invalid_syntax() {
    let cases = [
        "systemBoundary Auth <<System>>\nLogin\nend",
        "Login\nnote for Login \"text\" <<Memo>>",
        "json Payload@{} <<Data>>",
        "A link@--> B\nlink@{ animate: true } <<Link>>",
    ];
    for body in cases {
        let source = format!("usecase-beta\n{body}");
        expect_rejected_case_at(body, &source, located(&source, "<<", 1, 0));
    }
}

#[test]
fn locates_an_unterminated_declaration_or_edge_at_the_end_of_input() {
    for body in ["actor", "A -->"] {
        let source = format!("usecase-beta\n{body}");
        // The spec's `sourceLocation(source, source.length, source.length)`: the position just past
        // the body. The error is the line break that `parse` appends, which sits at that position.
        expect_rejected_case_at(body, &source, line_column(&source, source.len()));
    }
}

#[test]
fn rejects_business_icon_awesome_and_rectangular_declarations_with_locations() {
    let icon = "usecase-beta\nactor Icon@{ icon: \"fa:user\", business: true }";
    expect_rejected_at(icon, located(icon, "Icon", 1, 0));

    let awesome = "usecase-beta\nactor Awesome@{ type: awesome, business: true }";
    expect_rejected_at(awesome, located(awesome, "Awesome", 1, 0));

    let rectangle = "usecase-beta\nReport[Generate report]@{ business: true }";
    expect_rejected_at(rectangle, located(rectangle, "Report", 1, 0));
}

#[test]
fn rejects_json_in_boundaries_and_json_semantic_or_circle_relations_with_locations() {
    let boundary = "usecase-beta\nsystemBoundary Auth\njson Payload@{}\nend";
    expect_rejected_at(boundary, located(boundary, "json Payload@", 1, 0));

    let semantic = "usecase-beta\njson Payload@{}\nInspect\nPayload ..> : include Inspect";
    expect_rejected_at(semantic, located(semantic, "..> : include Inspect", 1, 0));

    let circle = "usecase-beta\njson Payload@{}\nInspect --o Payload";
    expect_rejected_at(circle, located(circle, "--o Payload", 1, 0));
}

#[test]
fn declares_a_boundary_with_an_id_a_label_inline_metadata_and_classes() {
    let model = parsed(
        &[
            "usecase-beta",
            "classDef system fill:#f8f8ff",
            "systemBoundary sb1[\"Payment service\"]@{ type: package }:::system",
            "  actor Clerk(\"Payment clerk\")",
            "  Authorize(\"Authorize payment\")",
            "end",
            "Clerk --> Authorize",
        ]
        .join("\n"),
    );

    assert_json_equal(
        &serde_json::to_value(boundary(&model, "sb1")).expect("the boundary serialises"),
        &json!({
            "id": "sb1",
            "label": "Payment service",
            "labelType": "text",
            "type": "package",
            "members": ["Clerk", "Authorize"],
            "classes": ["system"],
            "styles": [],
        }),
    );
}

#[test]
fn accepts_each_boundary_label_form() {
    let cases = [
        (
            "bracketed text",
            "sb1[Payment service]",
            "Payment service",
            LabelType::Text,
        ),
        (
            "bracketed plain string",
            "sb1[\"Payment service\"]",
            "Payment service",
            LabelType::Text,
        ),
        (
            "bracketed Markdown string",
            "sb1[\"`**Payment** service`\"]",
            "**Payment** service",
            LabelType::Markdown,
        ),
        (
            "parenthesized text",
            "sb1(Payment service)",
            "Payment service",
            LabelType::Text,
        ),
        (
            "parenthesized plain string",
            "sb1(\"Payment service\")",
            "Payment service",
            LabelType::Text,
        ),
        (
            "parenthesized Markdown string",
            "sb1(\"`**Payment** service`\")",
            "**Payment** service",
            LabelType::Markdown,
        ),
    ];
    for (form, declaration, label, label_type) in cases {
        let model = parsed(&format!(
            "usecase-beta\nsystemBoundary {declaration}\nAuthorize(\"Authorize payment\")\nend"
        ));
        let sb1 = boundary(&model, "sb1");
        assert_eq!(sb1.id, "sb1", "{form}");
        assert_eq!(sb1.label, label, "{form}");
        assert_eq!(sb1.label_type, label_type, "{form}");
    }
}

#[test]
fn keeps_plain_ids_derived_ids_and_separate_metadata_statements_working() {
    let model = parsed(
        &[
            "usecase-beta",
            "classDef system fill:#f8f8ff",
            "systemBoundary \"Payment service\":::system",
            "  Authorize(\"Authorize payment\")",
            "end",
            "systemBoundary Billing",
            "  Invoice[Create invoice]",
            "end",
            "Payment_service@{ type: package }",
        ]
        .join("\n"),
    );

    let payment = boundary(&model, "Payment_service");
    assert_eq!(payment.label, "Payment service");
    assert_eq!(payment.boundary_type, BoundaryType::Package);
    assert_eq!(payment.classes, ["system"]);
    assert_eq!(payment.members, ["Authorize"]);

    let billing = boundary(&model, "Billing");
    assert_eq!(billing.label, "Billing");
    assert_eq!(billing.boundary_type, BoundaryType::Rect);
    assert_eq!(billing.members, ["Invoice"]);
}

#[test]
fn lets_a_later_metadata_statement_override_inline_boundary_metadata() {
    let model = parsed(
        &[
            "usecase-beta",
            "systemBoundary \"Payment service\"@{ type: package }",
            "  Authorize(\"Authorize payment\")",
            "end",
            "Payment_service@{ type: rect }",
        ]
        .join("\n"),
    );

    assert_eq!(
        boundary(&model, "Payment_service").boundary_type,
        BoundaryType::Rect
    );
}

#[test]
fn records_the_boundary_id_title_and_inline_metadata_spans_in_the_ast() {
    let source = [
        "usecase-beta",
        "systemBoundary sb1[\"Payment service\"]@{ type: package }",
        "Authorize(\"Authorize payment\")",
        "end",
    ]
    .join("\n");
    let model = parsed(&source);

    let id_start = source.find("sb1").expect("sb1");
    let title_start = source.find("Payment service").expect("the title");
    let type_start = source.find("type").expect("type");
    assert_json_matches(
        &statements_json(&model)[0],
        &json!({
            "kind": "group",
            "group": "sb1",
            "idSpan": [id_start, id_start + 3],
            "titleSpan": [title_start, title_start + "Payment service".len()],
            "metadata": [
                {
                    "key": "type",
                    "span": [type_start, type_start + "type: package".len()],
                    "keySpan": [type_start, type_start + 4],
                    "valueSpan": [type_start + 6, type_start + "type: package".len()],
                },
            ],
        }),
    );
}

#[test]
fn requires_inline_boundary_metadata_before_the_class_suffix() {
    let source = "usecase-beta\nsystemBoundary sb1:::system@{ type: package }\nAuthorize(\"Authorize payment\")\nend\nclassDef system fill:#f8f8ff";
    expect_rejected_at(source, located(source, "@{", 1, 0));
}

#[test]
fn rejects_inline_boundary_metadata_that_a_boundary_does_not_accept() {
    let source = "usecase-beta\nsystemBoundary sb1[\"Payment service\"]@{ animate: true }\nAuthorize(\"Authorize payment\")\nend";
    expect_rejected_at(source, located(source, "animate", 1, 0));
}

#[test]
fn publishes_a_serializable_ast_v1_with_nodes_groups_and_edges() {
    let source = [
        "usecase-beta",
        "",
        "%% representative",
        "systemBoundary Auth",
        "actor User@{ type: hollow } <<Human>>",
        "Login[Sign in]",
        "end",
        "Auth@{ type: package }",
        "json Payload@{\"ok\": true}",
        "Login includeEdge@..> : include Login",
        "includeEdge@{ animation: fast }",
        "note for Login \"Remember\"",
    ]
    .join("\n");
    let model = parsed(&source);
    let ast = ast_json(&model);

    // The comment line and the blank line before it are removed before the scanner (see the module
    // doc), so the prepared text is the source without them, with the line break appended.
    let prepared_source = [
        "usecase-beta",
        "systemBoundary Auth",
        "actor User@{ type: hollow } <<Human>>",
        "Login[Sign in]",
        "end",
        "Auth@{ type: package }",
        "json Payload@{\"ok\": true}",
        "Login includeEdge@..> : include Login",
        "includeEdge@{ animation: fast }",
        "note for Login \"Remember\"",
    ]
    .join("\n");
    assert_json_matches(
        &ast,
        &json!({
            "version": 1,
            "diagramType": "usecase",
            "source": prepared(&prepared_source),
            "header": { "keyword": "usecase", "direction": "LR", "span": [0, 12] },
            "classDefs": {},
            "nodes": {
                "User": {
                    "shape": "actor-hollow",
                    "attrs": {
                        "kind": "actor",
                        "actorType": "hollow",
                        "business": false,
                        "stereotype": "Human",
                        "labelType": "text",
                        "parentId": "Auth",
                    },
                },
                "Login": {
                    "label": "Sign in",
                    "shape": "rect",
                    "attrs": {
                        "kind": "usecase",
                        "useCaseShape": "rect",
                        "business": false,
                        "labelType": "text",
                        "parentId": "Auth",
                    },
                },
                "Payload": {
                    "label": "Payload",
                    "shape": "json-table",
                    "attrs": {
                        "kind": "json",
                        "value": { "ok": true },
                        "propertyOrder": { "": ["ok"] },
                    },
                },
                "note-0": {
                    "label": "Remember",
                    "shape": "note",
                    "attrs": { "kind": "note", "target": "Login", "labelType": "text" },
                },
            },
            "groups": {
                "Auth": {
                    "nodes": ["User", "Login"],
                    "attrs": {
                        "kind": "systemBoundary",
                        "boundaryType": "package",
                        "labelType": "text",
                    },
                },
            },
        }),
    );

    assert_json_equal(
        &ast["edges"][0],
        &json!({
            "id": "includeEdge",
            "source": "Login",
            "target": "Login",
            "label": "include",
            "attrs": {
                "relationshipType": "include",
                "arrowType": 0,
                "minlen": 1,
                "explicitId": true,
                "animate": true,
                "animation": "fast",
                "labelType": "text",
            },
        }),
    );
    assert_json_matches(
        &ast["edges"][1],
        &json!({
            "source": "note-0",
            "target": "Login",
            "attrs": { "relationshipType": "note", "internal": true },
        }),
    );

    let round_trip: ferrigura_model::use_case::GraphAst =
        serde_json::from_str(&serde_json::to_string(&model.ast).expect("the AST serialises"))
            .expect("the AST reads back");
    assert_eq!(round_trip, model.ast);
}

#[test]
#[ignore = "the spec reads the statements of the raw Chevrotain parse. parse runs cleanup_comments first, as Diagram.fromText does, so the blank and comment statements are not published and the spans after the removed lines are shifted"]
fn ast_statements_and_exact_spans_of_a_commented_diagram() {
    let source = [
        "usecase-beta",
        "",
        "%% representative",
        "systemBoundary Auth",
        "actor User@{ type: hollow } <<Human>>",
        "Login[Sign in]",
        "end",
        "Auth@{ type: package }",
        "json Payload@{\"ok\": true}",
        "Login includeEdge@..> : include Login",
        "includeEdge@{ animation: fast }",
        "note for Login \"Remember\"",
    ]
    .join("\n");
    let model = parsed(&source);

    assert_json_equal(
        &Value::Array(
            statements_json(&model)
                .as_array()
                .expect("statements")
                .iter()
                .map(|statement| pick(statement, &["kind", "span"]))
                .collect(),
        ),
        &json!([
            { "kind": "blank", "span": [13, 14] },
            { "kind": "comment", "span": [14, 31] },
            { "kind": "group", "span": [32, 108] },
            { "kind": "metadata", "span": [109, 131] },
            { "kind": "json", "span": [132, 157] },
            { "kind": "edge", "span": [158, 195] },
            { "kind": "edgeMetadata", "span": [196, 227] },
            { "kind": "note", "span": [228, 253] },
        ]),
    );

    let statements = statements_json(&model);
    let group = &statements[2];
    assert_json_matches(
        group,
        &json!({ "group": "Auth", "idSpan": [47, 51], "titleSpan": [47, 51], "endSpan": [105, 108] }),
    );
    assert_json_equal(
        &Value::Array(
            group["children"]
                .as_array()
                .expect("the children")
                .iter()
                .map(|child| pick(child, &["kind", "span"]))
                .collect(),
        ),
        &json!([
            { "kind": "node", "span": [52, 89] },
            { "kind": "node", "span": [90, 104] },
        ]),
    );
    assert_json_matches(
        &group["children"][0],
        &json!({
            "nodes": [
                {
                    "id": "User",
                    "idSpan": [58, 62],
                    "stereotypeSpan": [82, 87],
                    "metadata": [
                        { "key": "type", "span": [65, 77], "keySpan": [65, 69], "valueSpan": [71, 77] },
                    ],
                },
            ],
        }),
    );
    assert_json_matches(
        &group["children"][1],
        &json!({ "nodes": [{ "id": "Login", "idSpan": [90, 95], "labelSpan": [96, 103] }] }),
    );
    assert_json_matches(
        &statements[3],
        &json!({
            "nodes": [{ "id": "Auth", "idSpan": [109, 113] }],
            "metadata": [{ "key": "type", "span": [116, 129], "keySpan": [116, 120], "valueSpan": [122, 129] }],
        }),
    );
    assert_json_matches(
        &statements[4],
        &json!({ "nodes": [{ "id": "Payload", "idSpan": [137, 144] }] }),
    );
    assert_json_matches(
        &statements[5],
        &json!({
            "nodes": [
                { "id": "Login", "idSpan": [158, 163] },
                { "id": "Login", "idSpan": [190, 195] },
            ],
            "edges": [{ "id": "includeEdge", "idSpan": [164, 175], "labelSpan": [182, 189] }],
        }),
    );
    assert_json_matches(
        &statements[6],
        &json!({
            "edges": [{ "id": "includeEdge", "idSpan": [196, 207] }],
            "metadata": [
                { "key": "animation", "span": [210, 225], "keySpan": [210, 219], "valueSpan": [221, 225] },
            ],
        }),
    );
    assert_json_matches(
        &statements[7],
        &json!({ "ref": "note-0", "refSpan": [244, 252], "nodes": [{ "id": "Login", "idSpan": [237, 242] }] }),
    );
}

#[test]
fn locates_the_json_id_even_when_it_is_a_substring_of_the_json_keyword() {
    for id in ["o", "n", "s", "j", "jo", "so", "on", "son", "json"] {
        let source = format!("usecase-beta\njson {id} @{{\"a\": 1}}");
        let model = parsed(&source);

        let id_start = source.find(&format!("json {id}")).expect("the JSON node") + "json ".len();
        let last = model.ast.statements.last().expect("a statement");
        assert_eq!(
            last.nodes.as_ref().map(|nodes| nodes[0].id.as_str()),
            Some(id)
        );
        assert_json_matches(
            &serde_json::to_value(last).expect("the statement serialises"),
            &json!({
                "kind": "json",
                "nodes": [{ "id": id, "idSpan": [id_start, id_start + id.len()] }],
            }),
        );
    }
}

#[test]
fn preserves_the_separators_inside_multi_word_style_and_class_def_values() {
    let model = parsed(
        &[
            "usecase-beta",
            "Checkout(\"Place order\")",
            "classDef big font-family:Arial Black",
            "style Checkout border:1px solid red,fill:#eee",
        ]
        .join("\n"),
    );

    assert_eq!(
        use_case(&model, "Checkout").styles,
        ["border:1px solid red", "fill:#eee"]
    );
    let big = model
        .class_defs
        .iter()
        .find(|class| class.id == "big")
        .expect("big");
    assert_eq!(big.styles, ["font-family:Arial Black"]);
}

#[test]
fn reports_the_source_position_of_a_lexer_failure() {
    let source = "usecase-beta\nactor \"Bob";
    expect_rejected_at(source, (2, 7));
}

#[test]
fn accepts_ids_that_start_with_a_digit() {
    let model = parsed(
        &[
            "usecase-beta",
            "actor 1(\"Customer\")",
            "actor 1mg(\"Milligram\")",
            "2(\"Place order\")",
            "3rd(\"Third party\")",
            "1 --> 2",
            "1mg --> 3rd",
            "json 4 @{\"a\": 1}",
            "2 --> 4",
        ]
        .join("\n"),
    );

    assert_eq!(
        ids_of(model.actors.iter().map(|a| a.id.as_str())),
        ["1", "1mg"]
    );
    assert_eq!(
        ids_of(model.use_cases.iter().map(|u| u.id.as_str())),
        ["2", "3rd"]
    );
    assert_eq!(
        ids_of(model.json_nodes.iter().map(|j| j.id.as_str())),
        ["4"]
    );
    assert_eq!(actor(&model, "1").label, "Customer");
    assert_eq!(use_case(&model, "3rd").label, "Third party");
    assert_eq!(
        model
            .relationships
            .iter()
            .map(|r| format!("{}->{}", r.source, r.target))
            .collect::<Vec<_>>(),
        ["1->2", "1mg->3rd", "2->4"]
    );
}

#[test]
fn keeps_decimal_style_values_intact_now_that_ids_may_start_with_a_digit() {
    let model = parsed(
        &[
            "usecase-beta",
            "A(\"x\")",
            "style A stroke-width:1.5px,opacity:0.5,margin:2",
        ]
        .join("\n"),
    );

    assert_eq!(
        use_case(&model, "A").styles,
        ["stroke-width:1.5px", "opacity:0.5", "margin:2"]
    );
}
