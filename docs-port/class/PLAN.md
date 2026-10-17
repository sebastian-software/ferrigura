# M6 class parser: plan

Status: parser, DB, model and specs ported. `pnpm port:diff class`: 327 inputs, 266 accepted by the reference, 0 differences. `pnpm port:diff config`: the class group included, 0 differences. Reference: the JS tree at `04b952b4e` (the commit of `main` and of this worktree). Research: [NOTES.md](./NOTES.md). Line numbers are `file:line` in the reference.

## 1. Scope and sources

Read in full: `parser/classDiagram.jison` (lexer 8–211, grammar 221–438), `classDb.ts` (1–737; `getData`, 738–865, is rendering and left out), `classTypes.ts` (`ClassMember`, `parseGenericTypes`), `common/common.ts` (`sanitizeText`, `sanitizeMore`, `parseGenericTypes`), `commonDb.ts` (header and title setters), `utils.ts` (`formatUrl`), `Diagram.ts` (the front matter title).

Left out: the renderer, the shapes, the ELK terminal labels, `getData()` and the colour slots (see NOTES §5). The DB's callback functions (`setClickFunc`, `functions`) are not in the model; `haveCallback` is.

## 2. Architecture

- `crates/ferrigura-syntax/src/class/`: `mod.rs` (`Options`, `parse`, `parse_grammar`, `ParseError` mapping), `lexer.rs` (states, token kinds, rule table in Jison order), `grammar.rs` (recursive descent over the tokens; each DB call at the point where the Jison reduction makes it), `db.rs` (the ClassDB state and setters, `sanitize` by configuration), `member.rs` (`ClassMember` text, `parseGenericTypes`).
- `crates/ferrigura-syntax/src/url.rs`: `sanitizeUrl` of `@braintree/sanitize-url` 7.1.2 and `formatUrl`, with the `url` crate for the WHATWG serialisation (new dependency, `url = "2"`).
- The lexer reuses `scan.rs` (keyword word boundary, JS `\s` helpers) and `flowchart/unicode.rs` (`UNICODE_TEXT`: the class list is identical, 409 ranges, checked).

Hand-written, as ADR 0002 says. The lexer runs over the whole prepared text first: the Jison lexer does not depend on the parser (no action changes a state), so the token list is the same. A lexical error anywhere rejects the input, which Jison also does, since accepting needs the lexer at EOF.

## 3. Lexer

- **Jison semantics.** First matching rule in definition order, not the longest. A rule whose pattern ends in a word character gets an implicit `\b` (`style`, `class`, `note for`, `\s*o`, `_self`, `classDiagram-v2`). A rule with no state prefix is INITIAL only; `<*>` rules are active in every state. The state stack: `begin` pushes, `popState` pops and never pops the base, `less(0)` re-lexes the match in the new state.
- **States.** INITIAL, string, bqstring, generic, acc_title, acc_descr, acc_descr_multiline, callback_name, callback_args, namespace, namespace-body, class, class-body. `struct` and `href` are never entered.
- **Empty matches at the end of the input.** Jison tries the rules once more on the empty input, so `[^"]*` in the string state yields an empty STR, and then EOF. `<<EOF>>` gives EOF_IN_STRUCT inside a namespace body or a class body, else EOF.
- **Comments.** `%%` (INITIAL only) consumes the rest of the line and the line breaks after it, so the statement separator is lost. Whole-line comments are removed before lexing (`cleanupComments` is not in the parser wrapper: it is in `preprocess`, see §4).
- **Direction.** `.*direction\s+TB[^\n]*` (INITIAL) matches from the current position, so it swallows the earlier part of the line. Implemented as the anchored regex with JS character classes.
- **NEWLINE.** `\s*(\r?\n)+` ends at the last line break of the whitespace run; `\s+` is skipped in INITIAL and in the namespace, namespace-body and class states.
- **Class body.** `[\n]` is dropped; `[^{}\n]*` is one MEMBER (includes blanks and `#`); `"` still starts a string (the rule `<*>["]` comes before MEMBER).

## 4. Preprocessing and configuration

- `preprocessDiagram` (front matter, directives, `cleanupComments`, CRLF, attribute quotes): `preprocess::prepare`, the same as the flowchart. No `encodeEntities`, no closing-brace rewrite and no trailing line break: the class parser is the Jison parser alone.
- The title is `Diagram.ts`'s `setDiagramTitle`, which sanitises: the model title is the sanitised front matter title (`commonDb.ts:28–30`). The flowchart keeps the raw title; the class port does not (its exporter reads the real setter).
- `Options { html_labels: bool (true), security_level: SecurityLevel (Strict) }`. `sanitizeText(text, config)` (`common.ts:56–83`) runs on every DB text:
  - `htmlLabels` and `strict` / `antiscript` / `sandbox`: DOMPurify (`removeScript`), then DOMPurify with `FORBID_TAGS: ['style']`. The port runs both passes in `sanitize_text` (`crates/ferrigura-syntax/src/sanitize.rs`), with the anchor hooks of `setupDompurifyHooks` in each pass.
  - `htmlLabels` and `loose`: no change, then DOMPurify.
  - `htmlLabels` and any other level: `<br>` becomes `#br#`, `<` and `>` become `&lt;` and `&gt;`, `=` becomes `&equals;`, `#br#` becomes `<br/>`, then DOMPurify.
  - `htmlLabels: false`: DOMPurify only.
    The differential runs the reference with the defaults (`htmlLabels` true, `securityLevel` strict), so only the default branch is compared (§9).
- `sanitizeUrl` is used by `formatUrl` unless `securityLevel` is `loose`. `setLink` sets `_top` for `sandbox`.

## 5. Model (`ferrigura-model/src/class.rs`)

Version 1, camelCase. Field names follow the DB; conversions to canonical names are in the exporter.

- `title`, `accTitle`, `accDescr`, `direction` (`TB` by default).
- `classes` in creation order: `id`, `type` (generic, `""` when none), `label`, `text`, `cssClasses`, `annotations`, `members`, `methods`, `styles`, `parent`, `link`, `linkTarget`, `tooltip`, `haveCallback` (`null` unless a callback was set).
- `members` and `methods` are `ClassMember`: `memberType`, `visibility`, `classifier`, `id`, `parameters` and `returnType` (`null` for attributes, as the DB leaves them undefined), `text` (the DB's `text`, which the renderer shows).
- `relations`: `id1`, `id2`, `relationTitle1`, `relationTitle2` (`"none"` when no cardinality, as the DB holds it), `title` (`null` without a label), `type1`, `type2` (marker names `none`, `aggregation`, `extension`, `composition`, `dependency`, `lollipop`, from the DB's numbers), `lineType` (`solid` or `dotted`).
- `notes`: `id`, `class` (raw, `null` for a plain note), `text` (raw, not sanitised), `index`, `parent`.
- `interfaces`: `id`, `label` (raw), `classId`.
- `namespaces` in creation order: `id`, `label`, `parent`, `explicit`, `classes`, `notes` (member ids in insertion order). `children` is derived from `parent` and left out.
- `styleClasses`: `id`, `styles`. `textStyles` is derived from `styles` (a pure function) and left out.

Left out: `domId`, `classCounter`, `namespaceCounter`, `shape` (always `classBox`), the callback functions, `getData()`.

## 6. Header texts

`accTitle` is the grammar's `.trim()` value, then `sanitizeText`, then `^\s+` removed (`commonDb.ts:16–18`). `accDescr` (single and block) is the trimmed value, then `sanitizeText`, then `\n\s+` replaced by `\n` (`commonDb.ts:22–24`). The model holds the sanitised text. The exporter calls the real `commonDb` setters with jsdom (`provideDomForSanitising`), so the header rules are compared, not replicated. This differs from the flowchart, which replaces the setters (its canonical-model note).

## 7. Errors

Parse errors (Jison's and the lexer's) become "fails, at the position". DB exceptions that the reference throws become errors too, where the JS throws a TypeError: `setClassLabel`, `addAnnotation`, `addMember` and `addClassesToNamespace` on a missing class (possible only when `sanitizeText` changes a class name), and `addMember` on a member with `)` but no matching `(` (the method regex does not match, so `this.id.startsWith` throws). The differential counts a rejection on both sides as agreement.

## 8. Quirks reproduced

Each is a test in the port (the test name cites the reference line).

- Q1 Blanks between identifiers are dropped in INITIAL: `Foo Bar --> Baz` is the class `FooBar` (INITIAL `\s+` is skipped, the two ALPHA tokens join in `className`).
- Q2 Mid-line `%%` consumes the newline: `A -- B %% c` followed by a line is a parse error (probe 3).
- Q3 A line with `direction TB|BT|RL|LR` is one direction token: the text before it is lost (`A --> B direction TB` drops the edge).
- Q4 The header must be the first token and be followed by NEWLINE; `classDiagram` alone or without a newline is an error; statements are NEWLINE-separated; a namespace body with no statement is an error (`namespace N {}`).
- Q5 `[*]` is an error everywhere.
- Q6 `title x` is a className with no action: no DB call.
- Q7 A bare className statement (`A`) adds nothing.
- Q8 `relationTitle1/2` default to the string `none`; the model keeps it.
- Q9 Members: the list is reversed by the grammar and reversed again by `addMembers`, so the source order is kept; an empty member (after trim) is dropped; `<<x>>` in a body is an annotation with the inner text sanitised.
- Q10 `splitClassNameAndType` takes segment 1 between the first two `~`: `A~B~C~` has type `B`.
- Q11 `addClass` writes the generic as `&lt;T&gt;` in `text`; `setClassLabel` writes `<T>` raw.
- Q12 `setCssClass`, `setLink` and `setClickEvent`'s class mark: an id starting with a digit is looked up as `classId-<id>` (`setTooltip` and the callback flag do not).
- Q13 `defineClass` appends a style to every class whose `cssClasses` contains the style-class id as a substring.
- Q14 `setCssStyle` looks the class up without splitting the id and splits each style at commas.
- Q15 Note text is not sanitised; a note's `class` is the raw class text (generics included).
- Q16 Namespaces: qualified ids, ancestors created with `explicit` only on the leaf; re-declaration sets `explicit` and keeps the first position; classes and notes keep their first position in a namespace.
- Q17 A callback sets `haveCallback` only on an existing class, and adds `clickable`; the function is not kept.
- Q18 `setLink` target is `_blank` by default and `_top` under `sandbox`; the URL goes through `formatUrl`.
- Q19 `accTitle:` and `accDescr:` skip the line break after the colon (`\s*`), so the value may come from the next line.
- Q20 A lollipop relation with type2 `none` gives an interface whose label is the raw first id and whose `classId` is the second id; the test is against the number list (`invalidTypes`).
- Q21 The annotation test on a member is `startsWith('<<') && endsWith('>>')` after trim, so `<<>>` gives an empty annotation.

## 9. Open items and approximations

- `sanitizeUrl` (`url.rs`) follows the node oracle except where the `url` crate (2.5.8) differs: a caret in an http(s) path is not percent-encoded, and an `xn--` label whose punycode does not decode is rejected. A lone surrogate from an entity becomes U+FFFD, since a Rust string cannot hold it.
- Config: the class section drops the schema defaults that `defaultConfig.ts` does not carry (`padding`, `textHeight`, `diagramPadding`, `titleTopMargin`, `useMaxWidth`, `arrowMarkerAbsolute`), in `build.rs`. `arrowMarkerAbsolute` is set by the reference's `init` at parse time, which the config export does not run.
- Specs: the ignored `classTypes.spec.ts` displayText and cssStyle tests (93) are ported to `ferrigura-render/tests/class_types_display_spec.rs`, with `display_text` and `css_style` in `ferrigura-render/src/class/member.rs`. The getData specs (colour slots, compact namespaces) are in `ferrigura-render/tests/class_db_get_data_spec.rs`. The ignores left in the parser crate are: callbacks (decision 0001), `it.skip`, and the JavaScript own-property test.
- Header texts are sanitised by the real `commonDb` setters in the exporter (section 6), so they are compared, not replicated.

- The single FORBID pass stands for the reference's two DOMPurify passes in the default configuration (§4).
- Non-default configuration (`htmlLabels: false`, `loose`, others) is implemented from the source and is not in the differential: the harness runs the defaults.
- `dompurifyConfig` is not supported (no fixture uses it).
- Surrogates in `sanitizeUrl`'s `fromCharCode` cannot be a Rust `char`: they become U+FFFD (in `url.rs`).
