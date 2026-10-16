# M6 state parser: plan

Status: parser, DB and model written from the reference's sources; the differential run follows. The research (grammar, DB call order, quirk probes) is in [NOTES.md](./NOTES.md). This plan fixes what the port does with it. Reference paths are under `packages/mermaid/src/diagrams/state/` unless written otherwise. Quirks marked `[NOTES]` were probed there; `[src]` ones were read in the source and are checked by the differential run.

## 1. Scope

- Both headers, `stateDiagram` and `stateDiagram-v2`, through one detector (`stateDetector-V2.ts`, `/^\s*stateDiagram/`). Config scope `state` (`diagramConfigKeys.ts` 10).
- The Jison grammar (`parser/stateDiagram.jison`, 352 lines) and `stateDb.ts`, up to the state the DB holds after parsing. `getData()` and `dataFetcher` are the render data: not in the model, except the parse-time error they can raise (section 5.4).
- Out of scope: rendering, `getData()` nodes and edges, `setupToolTips`, `bindFunctions`, `getConfig`, the configuration (the `state` section is M2's), the random trailing divider id beyond its placeholder.

## 2. Pipeline (the text the lexer sees)

1. `prepare` (shared with pie and info, `preprocess.rs`): CRLF to LF, HTML attribute quotes, front matter (its `title` is the title).
2. Directives `%%{ … }%%` removed, then comment lines removed (`cleanupComments`), as `preprocessDiagram` does. Both are the flowchart's functions, shared.
3. `encodeEntities` (`utils.ts`), then one `\n` appended (`Diagram.fromText`, line 75). The state parser has no closing-brace rewrite; the flowchart's `}\s*\n` rule does not apply.
4. Tokens: one scanner pass over the whole text (`state/scanner.rs`). The lexer's state changes never depend on the parser, so the token sequence does not either (the flowchart's argument, `PLAN.md` of M3, section 3.4).
5. Parse: recursive descent over the tokens (`state/grammar.rs`). The DB calls of the actions run in source order: `getDividerId` (the `divider-id-<n>` counter), `setAccTitle`, `setAccDescription`. `setDirection` is replayed from the document (section 5.2), so its unshift on the empty `rootDoc` leaves no trace.
6. `setRootDoc`: `docTranslator` (section 5.1), then `extract` (section 5.2). The reference runs `docTranslator` twice during the parse: `setRootDoc` calls `getRootDocV2`, and so does the `dataFetcher` call inside `extract` (`stateDb.ts` 277). The second pass matters (Q39). The port runs `extract` once: the draw's second `extract` is idempotent (NOTES 2.5).
7. Title: the front matter title through `setDiagramTitle` (sanitised), `""` otherwise.

## 3. Scanner

Jison's rules in file order, first match wins, with the states of `%x` and the rule conditions as written. A rule with no condition applies in `INITIAL` only (there is no inclusive `%s` state). The table in `scanner.rs` lists every rule with its line, in the order of the file; the duplicates at lines 110-112 (`scale`, `SCALE`) never match and are left out.

Decisions that follow from Jison's runtime, not from the grammar:

- **Keyword literals** get the implicit `\b` when they end in a word character (`click`, `href`, `default`, `hide empty description`, `left of`, `right of`, `end note`). Literals followed by `\s+` or `\s*` (`classDef`, `stateDiagram`, `as`) have none.
- **Case-insensitive** (`%options case-insensitive`): every literal and every `direction`, `accTitle` and `as` match in any case; `\w` and `\s` are unaffected.
- **Zero-length matches** are tokens (CLASSDEFID, CLASS_STYLE, STYLEDEF_STYLES, acc value, STATE_ID, FLOATING_NOTE_ID, STATE_DESCR, the empty tail of `acc_descr_multiline`). Jison returns them, and `done` is set when the input is empty, so the last attempt happens at end of input and is followed by EOF. [src: jison-lex `next()`]
- **End of input** in `INITIAL` gives one `NL` (rule 165, `<<EOF>>`); in another state it gives EOF straight away. The text always ends in `\n`, so this matters only for the zero-length tokens.
- **`processId`**: an ID match with `%%` at offset `i > 0` is cut there and the rest is put back (`unput`); at offset 0 the match is consumed with no token and no state change.
- **Skipped newlines**: `[\s]+` (line 81) skips whitespace including line breaks, so a newline that follows a space is lost (`NL` comes only from a run that starts with `\n`). Inside a composite, `struct` skips all newlines (line 138) and there are no `NL` tokens. [NOTES 2.1 for the trailing-space case: src here]
- **`.*direction\s+WORD[^\n]*`** (lines 75-78 in INITIAL, 123-126 in `struct`) matches from the current position to the end of the line whenever `direction`, a whitespace run (which may cross lines) and the word follow on the same line. The whole line becomes one `direction_*` token. [NOTES Q11]
- **`.*<<fork>>`** and the other marker rules (116-121) take the text before the last marker on the line, trimmed, as the value.
- **State name error** (line 133): `\w+\s+\w+.*?\{` in `STATE` throws `Error: State name must be a single word. Found: "…"`. The message is kept exact (ADR 0002 keeps the lexer's own texts).

## 4. Grammar

The productions of lines 178-352, as recursive descent. Lookahead decides between:

- `idStatement` alone, `idStatement DESCR`, `idStatement '-->' idStatement [DESCR]` (a relation keeps the description, a state keeps none).
- `COMPOSIT_STATE` alone (a string statement) or with `{ document }` (a state with `doc`).
- `STATE_DESCR AS ID` with or without `{ document }`. Without, the ID is split at `:` and the description becomes `[trimmed description, parts[1]]` (`[NOTES Q24]`); with, the description and the ID are kept raw (`description: $1`, `id: $3`, no split).
- `note`: `note left of|right of ID NOTE_TEXT` (a state with `note`, no `type`), or `note NOTE_TEXT AS ID` (no action: the string `"note "`).
- `CLICK idStatement STRING STRING NL` and `CLICK idStatement HREF STRING NL`: the `NL` is required.
- `classDef CLASSDEF_ID CLASSDEF_STYLEOPTS` (the `DEFAULT` form cannot be reached: the lexer gives `DEFAULT_CLASSDEF_ID`, `[NOTES Q6]`), `style`, `class`, the three direction tokens, `HIDE_EMPTY` and `scale WIDTH` (strings).
- `acc_title`, `acc_descr` and `acc_descr_multiline_value` return their trimmed value as a string statement, which the document keeps.

`document` is `line*`, where `line` is a statement or `NL`, and no separator is needed between statements. A `Text("nl")` statement is dropped as `document` drops `'nl'`. Inside `{ … }` the document ends at `}`.

## 5. DB

### 5.1 docTranslator (`stateDb.ts` 327-375)

- A relation translates its two states: `state1` with `first = true`, `state2` with `false`.
- A state with id `[*]` becomes `<parent>_start` (first) or `<parent>_end`, and gets `start: first`. The root parent id is `root`. Other ids are trimmed.
- A state with a `doc` (and the root) is split at its concurrency dividers. Each divider takes the statements before it as its `doc`. The statements after the last divider become a trailing divider with a generated id, but only when at least one divider was found and the trailing list is not empty. Otherwise the node keeps its original list (`[NOTES Q17]`: a trailing `--` stays a leaf).
- Each child of the (new) `doc` is translated with the composite as parent.

The generated id is `Math.random` in the reference. The port writes `<generated>`, and the exporter writes the same placeholder for every divider whose id is not `divider-id-<n>` (documented normalisation, `export-state.ts`).

### 5.2 extract (`stateDb.ts` 249-302), root document only

For each root statement, in order:

- `state`: `addState(trim(id), type, doc, description, note)`. `type` is `default` when the statement has none (note statements). Classes of an inline `:::` are not passed here `[src: extract 254]`.
- `relation`: `addRelationObjs(state1, state2, description)`: `addState` for both (with their `classes`, the only path where `:::` classes reach the DB), then a relation `{id1, id2, relationTitle: sanitise(description ?? '')}`.
- `classDef`: `addStyleClass(trim(id), classes)`.
- `style`: `handleStyleDef`: `styles` of each listed state replaced by the split `styleClass` with `;` removed; a missing state is created.
- `applyClass`: `setCssClass(trim(id), styleClass)`: each comma part pushed to the state's `classes`, created when missing.
- `click`: `addLink(id, url, tooltip)`, keyed by the idStatement object: one link per statement.

`addState`: a new state takes `type`, `doc`, `note` and empty lists; an existing one keeps its first `type` (always set, so `Q1`) and takes `doc` only when it has none. `descriptions` are added through `addDescription` (leading `:` removed, trimmed, sanitised). A note sets `note` and sanitises `note.text` in place, so the root document's note is sanitised too.

`direction`: the DB's `getDirection()` is the first root `dir` statement, `TB` when none. The layout's `getData().direction` is the last root `dir` (`getDir`, `stateRenderer-v3-unified.ts` 19-33). Both are kept (`direction`, `layoutDirection`).

### 5.3 Header texts (parse time)

`setAccTitle(v)`: `sanitise(v).replace(/^\s+/, '')`. `setAccDescription(v)`: `sanitise(v).replace(/\n\s+/g, '\n')`. Both on the value the grammar trimmed. `sanitise` is `sanitize.rs::sanitize_text`, the flowchart's DOMPurify subset (no `<` means no change).

### 5.4 The one parse-time error from extract

`extract` runs `dataFetcher` and then throws `Group nodes can only have label. Remove the additional description for node [X]` when a group node (a state with a `doc`) has two or more descriptions after the traversal. The port simulates `dataFetcher`'s description accumulation (`stateDb.ts` and `dataFetcher.ts` 241-370, 372-477):

- Each visited state starts with `sanitise(id)` as its description; the first description turns it into a list (`[id, d]` when the id is not `sanitise(id)`, else `[d]`); later ones are appended, nested lists flattened and each sanitised.
- A state named `root` is not visited (`itemId !== 'root'`), its children still are.
- Ids `</join></fork>` and `</choice>`, and the empty id, are not nodes.
- The traversal is the reference's `setupDoc` order: a state, then its `doc`; a relation's two states.

## 6. Canonical model (`ferrigura-model/src/state.rs`, version 1)

```
StateDiagram { version, title, accTitle, accDescr, direction, layoutDirection,
               document: [Statement], states: [StateEntry], relations: [Relation],
               classes: [ClassDef], links: [Link] }
Statement   = state | relation | dir | classDef | style | applyClass | click | string
state       = { stmt: "state", id, type?, description?: string|[string], classes?, doc?, note?, start? }
relation    = { stmt: "relation", state1, state2, description? }
click       = { stmt: "click", id: state, url, tooltip }
StateEntry  = { id, type, descriptions, doc?, note?, classes, styles, textStyles }
```

The fields are the JavaScript objects' own keys, present only where the reference sets them (`undefined` is omitted). `string` statements are JSON strings: the reference keeps them in `doc` and `rootDoc`, where they count for the trailing-divider test. `states` are the root `Map` in insertion order; `links` are one entry per click statement in source order (keyed by the statement object in the reference).

Left out: `dividerCnt`, `startEndCount`, `links`' Map keys beyond their `id`, `funs`, the tooltip and `bindFunctions`, `getData()`, and the DB's `stmt` field on entries.

## 7. Quirks reproduced

Source of each: `[NOTES Qn]` probed in the reference; `[src]` read in the source, to be confirmed by the differential run. Every one is a difference the port would show if it fixed it.

| ID  | Behaviour                                                                                                                                                                            | Where                                                            |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| Q1  | A state keeps the first `type`; `A --> B` then `state A <<fork>>` leaves `A` as `default`                                                                                            | `addState` 418-429 [NOTES Q1]                                    |
| Q2  | `direction LR` then `direction TB`: `direction` is `LR` (first), layout is `TB` (last)                                                                                               | `getDirection`, `getDir` [NOTES Q2]                              |
| Q3  | A `dir` inside a composite is the composite's own direction (last one)                                                                                                               | `getDir` [NOTES Q3]                                              |
| Q4  | `class`, `:::`, `style` on a state inside a composite are not in the DB (only root statements are extracted)                                                                         | `extract` 252-271 [NOTES Q4]                                     |
| Q5  | Relations inside a composite are not in `relations`                                                                                                                                  | [NOTES Q5]                                                       |
| Q6  | `classDef default …` is a parse error                                                                                                                                                | lexer 98 vs grammar 295 [NOTES Q6]                               |
| Q7  | `note "text" as N` (floating note) is the string `"note "`; nothing is created                                                                                                       | grammar 266 [NOTES Q7]                                           |
| Q8  | `state foo` alone, or `state foo : d`, creates nothing                                                                                                                               | grammar 227 [NOTES Q8]                                           |
| Q9  | `title Foo` in the body creates states `title` and `Foo`                                                                                                                             | no grammar rule [NOTES Q9]                                       |
| Q10 | `hide empty description`, `scale N width` are strings                                                                                                                                | grammar 225-226 [NOTES Q10]                                      |
| Q11 | Any line containing `direction TB` (etc.) from the current position is one direction token                                                                                           | lexer 75-78 [NOTES Q11]                                          |
| Q12 | `;` is not a separator; it is part of an ID, and it ends a description or note text                                                                                                  | lexer 158-160 [NOTES Q12]                                        |
| Q13 | `#` and `%%` comments; `%%{` is not a comment                                                                                                                                        | lexer 83-84 [NOTES Q13]                                          |
| Q14 | `A --> B : ` (empty label) is no label                                                                                                                                               | [NOTES Q14]                                                      |
| Q15 | `A - B`, `A -> B`, `--> A` are parse errors                                                                                                                                          | lexer 158, 166 [NOTES Q15]                                       |
| Q16 | Two descriptions on a composite is a parse error (group label)                                                                                                                       | 5.4 [NOTES Q16]                                                  |
| Q17 | A trailing `--` in a composite is a leaf divider with no `doc`                                                                                                                       | 5.1 [NOTES Q17]                                                  |
| Q18 | `[*]` as a statement is always `<parent>_start`                                                                                                                                      | docTranslator 335-337 [NOTES Q18]                                |
| Q19 | A note inside a composite has no parent in the render; only the DB side is kept here                                                                                                 | render data, not in the model [NOTES Q19]                        |
| Q20 | A block note keeps the inner indentation of its later lines                                                                                                                          | `trim` only [NOTES Q20]                                          |
| Q21 | `style A …` twice replaces the first                                                                                                                                                 | handleStyleDef 316 [NOTES Q21]                                   |
| Q22 | `class A,B foo` gives both `foo`; repeated `class` lines append                                                                                                                      | setCssClass 696 [NOTES Q22]                                      |
| Q23 | Two descriptions: the first is the label (render), the rest the description list; `state "Long" as S` makes `Long` the first                                                         | [NOTES Q23]                                                      |
| Q24 | `state "x" as S1:alt` gives descriptions `["x", "alt"]` and id `S1`                                                                                                                  | grammar 233-243 [NOTES Q24]                                      |
| Q25 | `click A "url" "tip"` keeps the quotes of `url` and `tooltip`                                                                                                                        | [NOTES Q25]                                                      |
| Q26 | `[*] : desc` gives the start state a description                                                                                                                                     | [NOTES Q26]                                                      |
| Q27 | `A --> B : label:with colon` keeps the colon                                                                                                                                         | DESCR regex 160 [NOTES Q27]                                      |
| Q28 | `classDef … color:x` puts `fill:x` into `textStyles`, which only the DB holds                                                                                                        | addStyleClass 627-644 [NOTES Q28]                                |
| Q29 | A standalone `A:::cls` (not in a relation) gets no class in the DB                                                                                                                   | extract 254 [src]                                                |
| Q30 | A note statement has no `type` and so `default` in the DB                                                                                                                            | grammar 261-265 [src]                                            |
| Q31 | `accTitle:` and `accDescr:` values are string statements in the document; the value of `accTitle:` followed by a line break is the next line                                         | grammar 268-270, lexer 89 [src]                                  |
| Q32 | No separator is needed between statements: `A B` gives two states; a newline inside a composite is not a token                                                                       | grammar 184-199, lexer 138 [src]                                 |
| Q33 | Spaces before a line break swallow it: `click A "x" "y"  ` followed by another line is an error                                                                                      | lexer 81 [src]                                                   |
| Q35 | `note right of A  : text` (two blanks before the colon) gives the note the text `: text`                                                                                             | lexer 149, `substr(2)` [src]                                     |
| Q36 | A state id containing `<` (sanitised differently) with a description, inside a composite group, is a group-label error                                                               | 5.4 [src]                                                        |
| Q37 | A state named `root` is not visited by the group check; `root_start` is                                                                                                              | dataFetcher 272 [src]                                            |
| Q38 | Ids may contain `,`, `}`, `"` (in INITIAL and composites, `}` only closes a composite)                                                                                               | lexer 158 [src]                                                  |
| Q39 | A composite id with a trailing blank (`state "x" as [*] {`) becomes `<parent>_start` with `start: true`: the second `docTranslator` pass turns the trimmed `[*]` into a start marker | `setRootDoc` and `dataFetcher` 277 [src, found by fuzzing]       |
| Q40 | The DB keys a root composite `state "x" as [*] {` as `[*]` (the first pass), while the document shows `<parent>_start` (the second pass)                                             | `extract` 254 and the `dataFetcher` call [src, found by fuzzing] |

## 8. Specs

| Spec file                                        | Tests       | Port                                                                                                                                 |
| ------------------------------------------------ | ----------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `parser/state-parser.spec.js`                    | 17          | parser; `toThrow` as "fails"; exact lexer message for the single-word error                                                          |
| `parser/state-style.spec.js`                     | 24          | parser and model; `getData` tests (3) ignored: `port: getData, dataFetcher not ported`                                               |
| `stateDb.spec.js`                                | 6           | DB functions through the model (`addStyleClass`, `addDescription`, `addState`); the DOMPurify test is ported through `sanitize_text` |
| `stateDiagram-v2.spec.js`                        | 31          | parser and DB accessors                                                                                                              |
| `stateDiagram.spec.js`                           | 29 + 1 skip | parser and DB accessors; the skipped test stays `#[ignore]`                                                                          |
| `stateDiagram-colorIndex.spec.ts`                | 9           | `port: getData colour slots, M6 data differential`                                                                                   |
| `stateNoteEdge.spec.ts`, `stateNoteNode.spec.js` | 3, 2        | `port: getData, M6 data differential`                                                                                                |
| `stateRenderer-v3-unified.spec.js`               | 2           | `port: DOM and mouse events (jsdom), render`                                                                                         |

## 9. Differential

Run: `pnpm port:diff state` (389 inputs, 368 accepted by the reference, no difference) and `pnpm port:diff config` (the state group, 387 inputs, the `state` section compared, no difference). Beyond the collected inputs, 23,000 generated inputs were compared by a throwaway fuzzer (structured documents with composites, dividers, notes, classes and directives; character-level noise with the lexer's own punctuation, CRLF, U+2028 and front matter); the disagreements it found are in section 7 (Q39, Q40) and in the fixes listed below.

- Exporter `tools/port/export-state.ts`: `detectType` first (the header is case-sensitive there), then `preprocessDiagram`, `encodeEntities(cleaned) + '\n'`, the Jison parser with `yy` set to a `StateDB`, `clear()`, `setDiagramTitle` from the front matter, as `Diagram.fromText` does. The model is a JSON snapshot of the DB taken before `getData()`, because `getData()` runs `docTranslator` again. `getData().direction` is read after the snapshot for the layout.
- Normalisation (documented in the exporter): divider ids not of the form `divider-id-<n>` become `<generated>`.
- Inputs: the collector reads the two fixture folders, the state inputs of the platform's dev folder and the JavaScript specs (`.spec.js` is read for state only), keeps the inputs the state detector accepts, and adds the state cases of `cases.ts`. `reference.ts` registers the state detector.
- Named exceptions: none. Everything the reference does is reproduced.
- Shared change found by this differential: the directive free text of the reference's regex stops at U+2028, U+2029 and a bare CR (`preprocess.rs`, `loose_directive_len`). The flowchart differential is unchanged after the fix.

## 10. Open questions

1. `layoutDirection` as a model field (layout's last `dir`) is kept next to `direction`. The notes proposed only `direction`; the task asks for both.
2. Q36 needs `sanitise(id) != id`, which for DOMPurify means a `<` in the id. The case is rare, but it is in the reference's logic.
3. The trailing divider id: `<generated>` is a placeholder, not a value a user can write. Document, do not compare.
