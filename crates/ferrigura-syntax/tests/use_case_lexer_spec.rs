//! Port of `packages/mermaid/src/diagrams/usecase/parser/usecase.lexer.spec.ts`: the token behaviour of
//! the scanner (`use_case/scanner.rs`).
//!
//! The reference's `[tokenType.name, image]` pairs are the `Tok` kind and the image of a `Token`. Its
//! `errors` array is the `Err` of `scan`, and every expectation of this spec is that no error is
//! reported, so each `scan` here must be `Ok`. The scanner ends its list with an end-of-input token,
//! which Chevrotain's list does not have, so `tokens` drops it.
//!
//! Not ported: the line and column of a token (`startLine`, `startColumn`, `endLine`), since a token
//! exposes its byte and UTF-16 offsets and not its line and column. The `startOffset` is the `u16_start`.

use ferrigura_syntax::use_case::scanner::{Tok, Token, scan};

/// The tokens of `input`, without the end-of-input token. The scan must not fail.
fn tokens(input: &str) -> Vec<Token> {
    scan(input)
        .expect("the text lexes without an error")
        .into_iter()
        .filter(|token| token.kind != Tok::Eof)
        .collect()
}

/// The kind and the image of each token, as the spec's `[tokenType.name, image]` pairs.
fn token_images(input: &str) -> Vec<(Tok, &str)> {
    tokens(input)
        .iter()
        .map(|token| (token.kind, token.image(input)))
        .collect()
}

/// The kinds of the tokens of `input`, as the spec's `tokenType.name` list.
fn token_kinds(input: &str) -> Vec<Tok> {
    tokens(input).iter().map(|token| token.kind).collect()
}

#[test]
fn uses_longest_operator_and_delimiter_precedence() {
    assert_eq!(
        token_images("--|> ..> <<Human>> ::: ---> <--- --o o-- --x x-- --"),
        [
            (Tok::Generalization, "--|>"),
            (Tok::DependencyArrow, "..>"),
            (Tok::StereotypeStart, "<<"),
            (Tok::StereotypeText, "Human"),
            (Tok::StereotypeEnd, ">>"),
            (Tok::ClassSeparator, ":::"),
            (Tok::ForwardSolid, "--->"),
            (Tok::BackwardSolid, "<---"),
            (Tok::ForwardCircle, "--o"),
            (Tok::BackwardCircle, "o--"),
            (Tok::ForwardCross, "--x"),
            (Tok::BackwardCross, "x--"),
            (Tok::MarkerlessSolid, "--"),
        ]
    );
}

#[test]
fn keeps_exact_keywords_out_of_longer_identifiers_and_matches_semantic_words_case_insensitively() {
    assert_eq!(
        token_images("actor actorName include INCLUDE Include included extend EXTEND extended"),
        [
            (Tok::Actor, "actor"),
            (Tok::Identifier, "actorName"),
            (Tok::Include, "include"),
            (Tok::Include, "INCLUDE"),
            (Tok::Include, "Include"),
            (Tok::Identifier, "included"),
            (Tok::Extend, "extend"),
            (Tok::Extend, "EXTEND"),
            (Tok::Identifier, "extended"),
        ]
    );
}

#[test]
fn emits_one_token_per_physical_newline_for_line_feed() {
    assert_one_newline_token_each("\n");
}

#[test]
fn emits_one_token_per_physical_newline_for_carriage_return_and_line_feed() {
    assert_one_newline_token_each("\r\n");
}

#[test]
fn emits_one_token_per_physical_newline_for_carriage_return() {
    assert_one_newline_token_each("\r");
}

/// Two physical newlines of one kind are two newline tokens.
fn assert_one_newline_token_each(newline: &str) {
    let input = format!("{newline}{newline}");
    let found = tokens(&input);
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|token| token.kind == Tok::NewLine));
}

#[test]
fn preserves_multiline_markdown_and_treats_comment_lookalikes_inside_it_as_data() {
    assert_eq!(
        token_images("\"`first\n%% still markdown\nlast`\""),
        [(Tok::MarkdownString, "\"`first\n%% still markdown\nlast`\"")]
    );
}

#[test]
fn emits_indented_whole_line_comments_but_not_percent_pairs_after_another_token() {
    assert_eq!(
        token_images("  %% comment\n"),
        [(Tok::Comment, "%% comment"), (Tok::NewLine, "\n")]
    );
    assert_eq!(
        token_kinds("A %% data"),
        [Tok::Identifier, Tok::Percent, Tok::Percent, Tok::Identifier]
    );
}

#[test]
fn balances_nested_json_while_ignoring_escaped_quotes_and_braces_in_strings() {
    let source =
        "json Payload@{\n  \"text\": \"} \\\" {\",\n  \"nested\": {\"items\": [1, 2]}\n}:::data";
    // The reference also checks the position of the object: `startLine` 1, `startColumn` 14 and
    // `endLine` 4. Those are line and column numbers, which a token does not carry, so they are not
    // ported.
    assert_eq!(
        token_kinds(source),
        [
            Tok::JsonDeclarationStart,
            Tok::JsonObjectLiteral,
            Tok::ClassSeparator,
            Tok::Identifier,
        ]
    );
}

#[test]
fn consumes_an_unclosed_json_object_once_from_its_opening_brace() {
    let input = format!("json Payload@{{{}", "{".repeat(32_000));
    let found = tokens(&input);
    let last = found.last().expect("the object is a token");
    assert_eq!(last.kind, Tok::UnclosedJsonObjectLiteral);
    // The reference's `startOffset` 13 is the offset of the opening brace. Its `startLine` 1 and
    // `startColumn` 14 are not ported (see the module comment).
    assert_eq!(last.u16_start, 13);
}

#[test]
fn consumes_unclosed_markdown_and_stereotypes_once_instead_of_rescanning_every_suffix() {
    let markdown = tokens(&"\"`x".repeat(32_000));
    assert_eq!(markdown.len(), 1);
    assert_eq!(markdown[0].kind, Tok::UnclosedMarkdownString);

    let stereotype = tokens(&format!("<<{}", "x".repeat(128_000)));
    assert_eq!(
        stereotype
            .iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>(),
        [Tok::StereotypeStart, Tok::UnclosedStereotypeText]
    );
}

#[test]
fn preserves_strict_strings_colors_css_units_and_escaped_commas() {
    // IDENTIFIER is `\w+` so ids may start with a digit, which means whole-number CSS
    // units lex as IDENTIFIER rather than NUMBER. Both are accepted by `styleComponent`
    // and `styleValue` rebuilds its text from the source, so the value is unaffected.
    assert_eq!(
        token_images("\"%%\" 'plain' #f96 4px 50% red\\,blue"),
        [
            (Tok::PlainString, "\"%%\""),
            (Tok::PlainString, "'plain'"),
            (Tok::HashColor, "#f96"),
            (Tok::Identifier, "4px"),
            (Tok::Identifier, "50"),
            (Tok::Percent, "%"),
            (Tok::Identifier, "red"),
            (Tok::CssEscapedComma, "\\,"),
            (Tok::Identifier, "blue"),
        ]
    );
}

#[test]
fn lexes_leftover_punctuation_and_non_ascii_characters_as_label_text() {
    assert_eq!(
        token_images("*bold* ; < > ` \\ Ärlig 日本語"),
        [
            (Tok::CssPunctuation, "*"),
            (Tok::Identifier, "bold"),
            (Tok::CssPunctuation, "*"),
            (Tok::LabelPunctuation, ";"),
            (Tok::LabelPunctuation, "<"),
            (Tok::LabelPunctuation, ">"),
            (Tok::LabelPunctuation, "`"),
            (Tok::LabelPunctuation, "\\"),
            (Tok::LabelSymbol, "Ä"),
            (Tok::Identifier, "rlig"),
            (Tok::LabelSymbol, "日本語"),
        ]
    );
    // The fallback sits last, so every operator that shares a character still wins.
    assert_eq!(
        token_images("<-- --> ..> ::: @{ \\,"),
        [
            (Tok::BackwardSolid, "<--"),
            (Tok::ForwardSolid, "-->"),
            (Tok::DependencyArrow, "..>"),
            (Tok::ClassSeparator, ":::"),
            (Tok::MetadataStart, "@{"),
            (Tok::CssEscapedComma, "\\,"),
        ]
    );
}

#[test]
fn lexes_digit_leading_ids_as_identifiers_while_decimals_stay_numbers() {
    assert_eq!(
        token_images("1 1mg a1 _x"),
        [
            (Tok::Identifier, "1"),
            (Tok::Identifier, "1mg"),
            (Tok::Identifier, "a1"),
            (Tok::Identifier, "_x"),
        ]
    );
    // The longer_alt keeps decimals intact instead of splitting them into `1` `.` `5`.
    assert_eq!(
        token_images("1.5 1.5px .5"),
        [
            (Tok::NumberLiteral, "1.5"),
            (Tok::NumberLiteral, "1.5px"),
            (Tok::NumberLiteral, ".5"),
        ]
    );
}
