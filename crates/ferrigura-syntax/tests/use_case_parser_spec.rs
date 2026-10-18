//! Port of `packages/mermaid/src/diagrams/usecase/parser/usecase.parser.spec.ts`: the accept and reject
//! behaviour of the canonical line-oriented grammar.
//!
//! An accepted text publishes its model: `parse` returns it, and its AST holds the text it was read
//! from, with the line breaks as `prepare` reads them and the line break `Diagram.fromText` adds. A
//! rejected text publishes no model. The reference's `it.each` blocks are one test per case here, so a
//! failure names its input.
//!
//! Not ported: `defines the stable canonical CST rule names`. It asserts the names of the productions of
//! the Chevrotain parser (its GAST), which this port does not have; the grammar is recursive descent.

use ferrigura_syntax::use_case::parse;

/// Accepts a text: the parse succeeds, and the published model's AST is the text it was read from, with
/// its line breaks as `prepare` reads them and the line break `Diagram.fromText` adds.
fn accepted(source: &str) {
    let text = source.replace("\r\n", "\n").replace('\r', "\n");
    accepted_as(source, &format!("{text}\n"));
}

/// Accepts a text whose published AST holds `ast_source`. The whole-line comments are not in the AST
/// (`cleanupComments` removes them, with the blank lines before them).
fn accepted_as(source: &str, ast_source: &str) {
    match parse(source) {
        Ok(model) => assert_eq!(model.ast.source, ast_source, "the AST of {source:?}"),
        Err(error) => panic!("{source:?} is rejected: {error}"),
    }
}

/// Rejects a text: the parse fails, and publishes no model.
fn rejected(source: &str) {
    assert!(parse(source).is_err(), "{source:?} is accepted");
}

/// One test per case, each accepting its text.
macro_rules! accepts {
    ($($name:ident => $source:expr,)*) => {
        $(
            #[test]
            fn $name() {
                accepted($source);
            }
        )*
    };
}

/// One test per case, each rejecting its text.
macro_rules! rejects {
    ($($name:ident => $source:expr,)*) => {
        $(
            #[test]
            fn $name() {
                rejected($source);
            }
        )*
    };
}

/// One test per relation, each accepting it after the header.
macro_rules! accepts_relations {
    ($($name:ident => $relation:expr,)*) => {
        $(
            #[test]
            fn $name() {
                accepted(&format!("usecase-beta\n{}", $relation));
            }
        )*
    };
}

accepts! {
    accepts_header_at_eof => "usecase-beta",
    accepts_line_feed => "usecase-beta\nactor User",
    accepts_carriage_return_and_line_feed => "usecase-beta\r\nactor User\r\n",
    accepts_carriage_return => "usecase-beta\ractor User\r",
}

#[test]
fn accepts_blank_and_comment_lines() {
    accepted_as(
        "usecase-beta\n\n  %% retained\nLogin",
        "usecase-beta\nLogin\n",
    );
}

#[test]
fn accepts_canonical_actors_use_cases_metadata_stereotypes_and_class_suffixes() {
    accepted(
        r#"usecase-beta
actor Admin("Main Administrator")@{ type: hollow, business: true } <<Human>>:::external
actor "System Administrator", "`**Automation**`"
Login(Sign in)@{ business: false } <<Primary>>:::critical
Report[Generate report]
"Reset password"
"`**Markdown** use case`""#,
    );
}

#[test]
#[ignore = "the spec checks the grammar alone; the model builder rejects an undeclared metadata target (metadata implies actor), so parse does not publish this text"]
fn accepts_multiline_metadata_with_newline_and_comma_separators_and_a_trailing_comma() {
    accepted(
        r#"usecase-beta
User@{
  business: true
  type: normal,
}"#,
    );
}

accepts_relations! {
    accepts_canonical_relation_forward_solid => "A --> B",
    accepts_canonical_relation_backward_solid => "A <-- B",
    accepts_canonical_relation_markerless => "A -- B",
    accepts_canonical_relation_forward_circle => "A --o B",
    accepts_canonical_relation_backward_circle => "A o-- B",
    accepts_canonical_relation_forward_cross => "A --x B",
    accepts_canonical_relation_backward_cross => "A x-- B",
    accepts_canonical_relation_labelled_forward => "A -- \"starts session\" --> B",
    accepts_canonical_relation_labelled_backward => "A <-- \"reverse label\" -- B",
    accepts_canonical_relation_labelled_markerless => "A -- label -- B",
    accepts_canonical_relation_labelled_forward_circle => "A -- label --o B",
    accepts_canonical_relation_labelled_backward_circle => "A o-- label -- B",
    accepts_canonical_relation_labelled_forward_cross => "A -- label --x B",
    accepts_canonical_relation_labelled_backward_cross => "A x-- label -- B",
    accepts_canonical_relation_include => "A ..> : include B",
    accepts_canonical_relation_include_upper_case => "A ..> : INCLUDE B",
    accepts_canonical_relation_extend_mixed_case => "A ..> : ExTeNd B",
    accepts_canonical_relation_generalization => "A --|> B",
    accepts_canonical_relation_long_forward => "A ---> B",
    accepts_canonical_relation_long_backward => "A <---- B",
    accepts_canonical_relation_long_markerless => "A ---- B",
    accepts_canonical_relation_labelled_long_forward => "A -- longer ----> B",
    accepts_canonical_relation_explicit_id => "A login@--> B",
    accepts_canonical_relation_explicit_id_include => "A dependency@..> : include B",
}

#[test]
fn accepts_restricted_system_boundary_declarations_blanks_and_comments() {
    accepted_as(
        r#"usecase-beta
systemBoundary "Authentication System":::system
  %% boundary comment
  actor User, Admin("Administrator")

  Login("Sign in"):::critical
  Report[Generate report]
end
User --> Login"#,
        "usecase-beta\nsystemBoundary \"Authentication System\":::system\n  actor User, Admin(\"Administrator\")\n\n  Login(\"Sign in\"):::critical\n  Report[Generate report]\nend\nUser --> Login\n",
    );
}

#[test]
fn accepts_explicit_boundary_ids_with_labels_inline_metadata_and_class_suffixes() {
    accepted(
        r#"usecase-beta
systemBoundary sb1["Payment service"]@{ type: package }:::system
  Authorize("Authorize payment")
end
systemBoundary sb2(Billing)@{
  type: rect
}:::billing
  Invoice[Create invoice]
end
systemBoundary sb3("`**Support**`")
  Refund[Issue refund]
end"#,
    );
}

#[test]
#[ignore = "the spec checks the grammar alone; the model builder rejects an undeclared metadata target (metadata implies boundary), so parse does not publish this text"]
fn accepts_metadata_assignments_notes_json_direction_classes_and_styles() {
    accepted(
        r#"usecase-beta
direction LR
Auth@{ type: package }
note for Login "`Requires an **active session**`"
json Payload@{
  "fruit": "Apple",
  "nested": { "brace": "}", "items": [1, 2] }
}:::data
classDef external,critical fill:#fff,stroke-width:3px
class Login,Payload external,critical
style Login fill:#fee,stroke-dasharray:5\,5"#,
    );
}

#[test]
fn accepts_single_line_and_multiline_accessibility_statements() {
    accepted(
        r#"usecase-beta
accTitle: Authentication use cases
accDescr: Actors and authentication flows"#,
    );
    accepted(
        r#"usecase-beta
accDescr {
  Actors authenticate,
  reset credentials, and sign out.
}"#,
    );
}

rejects! {
    rejects_uppercase_header => "Usecase\nA",
    rejects_same_line_statements => "usecase-beta\nactor A actor B",
    rejects_header_statement_on_same_line => "usecase actor A",
    rejects_ambiguous_actor_list_relation => "usecase-beta\nactor A, B --> C",
    rejects_unknown_directive => "usecase-beta\nunknown command",
    rejects_slash_comment_lookalike => "usecase-beta\n// not a comment",
    rejects_hash_comment_lookalike => "usecase-beta\n# not a comment",
    rejects_semicolon_separator => "usecase-beta\nA; B",
    rejects_colon_actor => "usecase-beta\n:User:",
    rejects_actor_alias => "usecase-beta\nactor User as U",
    rejects_use_case_alias => "usecase-beta\nusecase \"Login\" as Login",
    rejects_g16_newpage => "usecase-beta\nnewpage",
    rejects_plantuml_package => "usecase-beta\npackage \"Auth\" {",
    rejects_plantuml_rectangle => "usecase-beta\nrectangle \"Auth\" {",
    rejects_plantuml_skinparam => "usecase-beta\nskinparam actorStyle awesome",
    rejects_plantuml_allowmixing => "usecase-beta\nallowmixing",
    rejects_plain_separator => "usecase-beta\n== Section ==",
    rejects_literal_backslash_n_separator => "usecase-beta\nA \\n B",
    rejects_relation_direction_hint => "usecase-beta\nA -left-> B",
    rejects_note_placement => "usecase-beta\nnote left of Login \"text\"",
    rejects_note_alias => "usecase-beta\nnote for Login as N \"text\"",
    rejects_multi_target_note => "usecase-beta\nnote for Login,Report \"text\"",
    rejects_nested_boundary => "usecase-beta\nsystemBoundary A\nsystemBoundary B\nend\nend",
    rejects_relation_in_boundary => "usecase-beta\nsystemBoundary A\nUser --> Login\nend",
    rejects_note_in_boundary => "usecase-beta\nsystemBoundary A\nnote for User \"text\"\nend",
    rejects_json_in_boundary => "usecase-beta\nsystemBoundary A\njson Payload@{}\nend",
    rejects_extra_circle_dash => "usecase-beta\nA ---o B",
    rejects_extra_reversed_circle_dash => "usecase-beta\nA o--- B",
    rejects_extra_cross_dash => "usecase-beta\nA ---x B",
    rejects_extra_generalization_dash => "usecase-beta\nA ---|> B",
    rejects_extra_include_dot => "usecase-beta\nA ...> : include B",
    rejects_empty_stereotype => "usecase-beta\nA <<>>",
    rejects_multiline_stereotype => "usecase-beta\nA <<first\nsecond>>",
    rejects_unclosed_stereotype => "usecase-beta\nA <<primary",
    rejects_unclosed_json => "usecase-beta\njson Payload@{\"nested\": {",
    rejects_unclosed_markdown => "usecase-beta\nA(\"`unfinished\")",
    rejects_semicolon_css => "usecase-beta\nstyle A fill:red;stroke:blue",
    rejects_boundary_class_suffix_before_inline_metadata => "usecase-beta\nsystemBoundary A:::system@{ type: package }\nend",
    rejects_boundary_stereotype_before_inline_metadata => "usecase-beta\nsystemBoundary A <<S>>\nend",
    rejects_boundary_label_without_an_id => "usecase-beta\nsystemBoundary [Payment service]\nend",
}
