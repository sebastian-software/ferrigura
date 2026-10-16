# M6 sequence parser: plan

Status: plan for the port of `sequenceDiagram.jison` and `sequenceDb.ts` to `crates/ferrigura-syntax/src/sequence/`. The facts are from [NOTES.md](./NOTES.md), re-read in the code; where a note disagrees with the code, the code wins and the difference is listed in section 8.

Reference: `packages/mermaid/src/diagrams/sequence/` of this worktree (`04b952b4e`). Line numbers are to those files.

## 1. Scope

- **In:** the lexer and grammar of `sequenceDiagram.jison`, the DB of `sequenceDb.ts` (state and `apply`), the header texts (`title`, `accTitle`, `accDescr`), `@{ … }` actor metadata (YAML, shared with flowchart), and the shared pipeline steps (front matter, directives, comments, entities).
- **Out:** the renderer (`sequenceRenderer.ts`, `svgDraw.js`, `actorBands.ts`), the styles and the `links`/`properties` popup menus, which need JavaScript (decision 0001). Their parse state stays in the model (section 4).
- **Options:** `Options { wrap: bool }`, default `false`. It is the `autoWrap()` value the DB has during the parse: `Diagram.fromText` calls `clear()`, which resets `wrapEnabled` (`Diagram.ts:49`), and `init` then copies a truthy top-level `wrap` into `sequence.wrap` (`sequenceDiagram.ts:21-23`). So the value is `sequence.wrap`, else `false`. The caller resolves it, as `maxEdges` is resolved for the flowchart.

## 2. Pipeline

Error positions count the lines of the original input. A line break that `cleanupComments` trims from the start is counted; a comment line that it removes is not, so a position after a removed comment line is one line too small.

Same steps as the flowchart parser, in the same order (`Diagram.fromText`, `Diagram.ts:19-83`):

1. `prepare`: front matter removed, its `title` kept (shared, `preprocess.rs`).
2. `removeDirectives`, then `cleanupComments` (`comments.ts:6`: `^\s*%%(?!{)[^\n]+\n?`, then `trimStart`). Both are shared and move from the flowchart module to `preprocess.rs`.
3. `encodeEntities(text) + "\n"` (`utils.ts:874-897`, shared, moved likewise).
4. The DB's `setDiagramTitle(front matter title)` runs before the parse (`Diagram.ts:52-54`). A body `title` replaces it.

The grammar then sees the text. `sequenceDiagram.ts` `init` only copies the top-level `wrap` into the section; that is the `Options` above.

## 3. Lexer

Jison's rules in their written order, first match wins, not longest match (`sequenceDiagram.jison:11-128`). States: `INITIAL`, `ID`, `ALIAS`, `LINE`, `CONFIG`, `acc_title`, `acc_descr`, `acc_descr_multiline` (`%x`, so each rule runs only in its own state). `begin` pushes, `popState` pops and is a no-op at the base (the `scan.rs` `StateStack`). All literals are case-insensitive (`%options case-insensitive`). A literal keyword that ends in a word character gets Jison's implicit `\b` (`scan.rs` `word_len`). Keywords have no state change unless the rule says so.

Rule groups, in order:

| Rules (lines) | Rule                                                                                                                                                            | Token or action                                                                                                                                                                                               |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 24            | `[\n]+` (INITIAL)                                                                                                                                               | NEWLINE                                                                                                                                                                                                       |
| 25            | `\s+` (INITIAL)                                                                                                                                                 | skip. **Quirk Q1:** it swallows line breaks that follow blanks                                                                                                                                                |
| 26            | `((?!\n)\s)+` (ID, ALIAS, LINE)                                                                                                                                 | skip, same-line blanks                                                                                                                                                                                        |
| 27            | `\#[^\n]*` (INITIAL, ID, ALIAS, LINE)                                                                                                                           | skip, comment to the line end                                                                                                                                                                                 |
| 28, 29        | `%%(?!\{)[^\n]*`, `[^\}]%%[^\n]*` (INITIAL)                                                                                                                     | skip                                                                                                                                                                                                          |
| 30            | `NUM`: `([0-9]+(\.[0-9]{1,2})?\|\.[0-9]{1,2})(?=[ \n]+)` (INITIAL)                                                                                              | NUM, the text (converted with `Number`)                                                                                                                                                                       |
| 31–34         | CONFIG: `@{` (ID) push; `[^}]+`; `}(?=\s+as\s)`; `}`                                                                                                            | CONFIG_START, CONFIG_CONTENT, CONFIG_END (pops CONFIG, then ALIAS is pushed, or both pops)                                                                                                                    |
| 35–39         | ID: ACTOR before `@{`, ACTOR before `as` (push ALIAS), ACTOR at line end (pop), INVALID up to `<` (pop), INVALID to line end (pop)                              | ACTOR (trimmed), INVALID                                                                                                                                                                                      |
| 40–46         | `box`, `participant`, `actor`, `create`, `destroy`; ALIAS `as`, ALIAS empty                                                                                     | keyword and state; `as` pops twice and enters LINE; empty ALIAS pops twice and gives NEWLINE                                                                                                                  |
| 47–58         | `loop`, `rect`, `opt`, `alt`, `else`, `par`, `par_over`, `and`, `critical`, `option`, `break` (enter LINE); LINE `restOfLine`                                   | keyword; `restOfLine` = `(?:[:]?(?:no)?wrap:)?[^#\n;]*` (pops LINE; may be empty)                                                                                                                             |
| 59–69         | `end`, `left of`, `right of`, `links`/`link`/`properties`/`details` (lookahead `[ \t]+[^/\\+()<>:\n,;\s-]`), `over`, `note`, `activate`/`deactivate` (enter ID) | keyword                                                                                                                                                                                                       |
| 70–71         | `title\s[^#\n;]+`, `title:\s[^#\n;]+`                                                                                                                           | title and legacy_title; the value is the text after the first character, as `substring(6)` / `substring(7)` (not trimmed)                                                                                     |
| 72–78         | `accTitle\s*:\s*` (acc_title), `accDescr\s*:\s*` (acc_descr), `accDescr\s*\{\s*` (multi-line), value rules                                                      | the value is `[^\n]*` (acc_title, acc_descr) or up to `}` (block)                                                                                                                                             |
| 79–83         | `sequenceDiagram`, `autonumber`, `off`, `,`, `;`                                                                                                                | SD, autonumber, off, `,`, NEWLINE (`;` ends a statement)                                                                                                                                                      |
| 84            | ACTOR: `[^/\\+()+<->:\n,;]+` then groups of `(?!arrow-prefix)[-]*[^+<->:\n,;]+`                                                                                 | ACTOR, trimmed. The arrow prefixes are `-x`, `--x`, `-)`, `--)`, `-\|\`, `-\`, `-/`, `-//`, `-\|/`, `/\|-`, `\|-`, `//-`, `\\-`, `/\|--`, `--\|\`, `--`, `()`. **Quirk Q2:** `A-B` is one actor, `A:B` is not |
| 85–118        | arrows: `->>`, `<<->>`, `-->>`, `<<-->>`, `->`, `-->`, `-x`, `--x`, `-)`, `--)`, the normal and reverse solid and stick heads, dotted and solid                 | arrow tokens (the table in section 6)                                                                                                                                                                         |
| 120–121       | `:(?:(?:no)?wrap:)?[^#\n;]*`                                                                                                                                    | TXT (the text of a message, a note or a block)                                                                                                                                                                |
| 122–126       | `+`, `-`, `()`, `<<EOF>>`, `.`                                                                                                                                  | the activation markers, the central connection, NEWLINE at the end of input, INVALID                                                                                                                          |

Two rules matter most for the port:

- **Q1, newline swallowed.** In INITIAL, `\s+` at line 25 comes after `[\n]+` and matches blanks and line breaks together. A line that ends in blanks loses its line break as a token, so the statement before it has no NEWLINE. Reproduced by the same rule order.
- **Q3, INVALID lines.** A lone `/`, `\`, `(`, `)`, `<` or `>` at the start of a token is INVALID (rule 126), and `line: INVALID` is accepted and dropped. Where a statement needs an actor, an INVALID token is a syntax error: `participant A:B` is INVALID in the ID state (rule 39). The grammar has no production for `participant` followed by INVALID (`sequenceDiagram.jison:263-270`), so the line is a syntax error, and only a lone INVALID line is dropped.

## 4. Grammar and DB calls

The grammar is LALR(1); the port is recursive descent with one token of lookahead, following the productions. The parser does not build the DB. It builds the same list of items the Jison actions build (`yy.apply($2)` at line 139), and the DB applies them in order after the whole parse, so a parse error wins over a DB error, as in the reference.

Items (the `type` names of the reference, lines in brackets): `addParticipant` (participant statements, and the actors of a message), `createParticipant` (`create`, which overwrites the type of a `destroy` too, line 168), `destroyParticipant`, `addMessage`, `activeStart`, `activeEnd`, `centralConnection`, `centralConnectionReverse`, `addNote`, `addLinks`, `addALink`, `addProperties`, `addDetails`, `boxStart`, `boxEnd`, `sequenceIndex`, and the block items `loopStart`/`loopEnd`, `rectStart`/`rectEnd`, `optStart`/`optEnd`, `altStart`/`else`/`altEnd`, `parStart`/`and`/`parEnd`, `criticalStart`/`option`/`criticalEnd`, `breakStart`/`breakEnd` (the `document`/`else_sections` nesting is flattened in the same order as `$3.unshift` and `$3.push`).

Parse-time actions that run in the reference during the parse, not in `apply`: `setDiagramTitle`, `setAccTitle`, `setAccDescription` (the commonDb setters, which sanitise, so they run in the same order), `parseMessage` and `parseBoxData`.

Productions that differ from their names:

- `signal` gives `[actorA, actorB, addMessage, activeStart | activeEnd | centralConnection…]`, so the implicit declarations of both actors run first.
- `note … over` coerces its actor pair to `[from, to]`.
- `box` takes `box_section`, which holds only participant statements and blank lines (line 159-162). `INVALID` and messages inside a box are syntax errors.
- Block sections (`else_sections`, `par_sections`, `option_sections`) are `document` with markers in between. A marker outside its block is a syntax error.

## 5. DB state and `apply`

State as `SequenceDB` holds it (`sequenceDb.ts:19-32`): actors (a map in insertion order), `prevActor`, `createdActors` and `destroyedActors` (actor id to the message count at that time), boxes, messages, notes, `lastCreated`, `lastDestroyed`, `currentBox`, and the sequence numbers flag. The reference's DB is not reset between parses of the same DB; the port creates a new one per parse.

The DB rules that the model depends on (with the reference quirks):

- **`addActor`** (`:155-231`). A repeated id returns early when the new declaration has no description and the name is the stored name (`:195-197`, quirk Q4 below). A declaration with a description replaces the record: `links` and `properties` are reset, `prevActor` is the previous declaration (which can be the same id), and that actor's `nextActor` is set to this id. The actor joins `currentBox.actorKeys` (repeats possible). The box of a repeated id is kept, and a different box throws (`:184-187`).
- **`addSignal`** (`:272-308`). An ACTIVE_END with no open activation throws.
- **`apply`** (`:549-710`). `addMessage` checks `lastCreated` and `lastDestroyed` first (`:615-638`). Messages from control items have no `from` and `to`.

Errors, with the exact texts of the reference (they are part of the user interface and are kept):

| Condition                                             | Text                                                                                                                                                            |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `create` of an id that exists (`:574-578`)            | `It is not possible to have actors with the same id, even if one is destroyed before the next is created. Use 'AS' aliases to simulate the behavior`            |
| message after `create` not to that actor (`:615-621`) | `The created participant X does not have an associated creating message after its declaration. Please check the sequence diagram.`, with `X` = `undefined` (Q6) |
| message after `destroy` not involving it (`:625-634`) | the same shape with `destroyed`, `X` = `undefined`                                                                                                              |
| actor in a second box (`:184-187`)                    | `A same participant should only be defined in one Box: A can't be in 'Red X' and in 'Blue Y' at the same time.` (an unnamed box is written `undefined`)         |
| ACTIVE_END with no activation (`:280-296`)            | `Trying to inactivate an inactive participant (X)`                                                                                                              |
| bad YAML in `@{ … }` (`:172`)                         | the YAML error, a parse error with a position                                                                                                                   |

Parse errors are the grammar's own. The port reports the position of the offending token, not Jison's text. The specs that check Jison's message text become "fails, and where" (ADR 0002).

## 6. Arrows

Each arrow token has a fixed line type (`sequenceDb.ts:34-91`, names as in the model). The table is in NOTES section 5.6; the port uses the same names:

`->>` SOLID 0, `-->>` DOTTED 1, `<<->>` BIDIRECTIONAL_SOLID 33, `<<-->>` BIDIRECTIONAL_DOTTED 34, `->` SOLID_OPEN 5, `-->` DOTTED_OPEN 6, `-x` SOLID_CROSS 3, `--x` DOTTED_CROSS 4, `-)` SOLID_POINT 24, `--)` DOTTED_POINT 25, `-|\` SOLID_TOP 41, `-|/` SOLID_BOTTOM 42, `-\\` STICK_TOP 43, `-//` STICK_BOTTOM 44, `--|\` SOLID_TOP_DOTTED 51, `--|/` SOLID_BOTTOM_DOTTED 52, `--\\` STICK_TOP_DOTTED 53, `--//` STICK_BOTTOM_DOTTED 54, `/|-` SOLID_ARROW_TOP_REVERSE 45, `\|-` SOLID_ARROW_BOTTOM_REVERSE 46, `//-` STICK_ARROW_TOP_REVERSE 47, `\\-` STICK_ARROW_BOTTOM_REVERSE 48, and the dotted reverse forms `/|--` 55, `\|--` 56, `//--` 57, `\\--` 58.

The two-backslash heads are literal `\\` in the source. The table is the grammar's, not the renderer's.

## 7. Model

`crates/ferrigura-model/src/sequence.rs`, `SequenceDiagram`, version 1, camelCase. Shape, following the DB:

- `title`, `accTitle`, `accDescr`: strings, `""` when absent (canonical-model.md). Sanitised (DOMPurify subset) as the setters do.
- `actors`: in the DB's map order. Each: `id`; `name`, the id as written, always (an alias sets only the description); `description`, the alias or the text after `as`, else the name (a YAML value when an alias is a number); `type`, `participant` or `actor`, or the metadata's `type` as YAML gives it (a JSON value, so a number stays a number); `wrap`; `box` (index into `boxes`, or null); `prevActor`; `nextActor` (null when unset); `links` (a JSON object in the key order of a JavaScript object, which the menu lists them in; canonical-model.md, Sequence) and `properties` (a JSON object, keys as the DB has them).
- `boxes`: `name` (null when the title is absent), `wrap`, `fill` (the colour as written, or `transparent`), `actorKeys` (the ids, repeats kept).
- `messages`: in `apply` order. Each: `id` (decimal index), `from`, `to` (null for the block markers and autonumber; set for messages, notes, and the activation and central connection items), `message` (text, or `{start, step, visible}` for autonumber, with null for undefined start and step), `wrap`, `type` (the line type's name, section 6), `activate` (null for notes and autonumber, which have no field in the DB), `centralConnection` (`NONE`, `CENTRAL_CONNECTION`, `CENTRAL_CONNECTION_REVERSE`, `CENTRAL_CONNECTION_DUAL`, null for notes and autonumber), `placement` (`LEFT_OF`, `RIGHT_OF`, `OVER`, null for others).
- `createdActors`, `destroyedActors`: actor id to message count.

Left out, and why:

- `notes` (the DB keeps a second copy of each note as a NOTE message).
- `actorKeys` at the top level (the order of `actors`).
- `actorCnt`, `rectData`, `lastCreated`, `lastDestroyed`, `currentBox`, `sequenceNumbersEnabled`: render or parser state.
- `answer` (never set by `apply`; `addMessage` is not called from the grammar).
- `wrapEnabled` and `autoWrap()`: the resolved `wrap` is in each item.

## 8. Quirks reproduced

Each is in the code with a comment that cites the reference line.

- **Q1** Blanks before a line break in INITIAL swallow the break (rule 25 after rule 24).
- **Q2** An actor name can contain `-`, `#`, `*`, space and other characters that are not arrow starts; `A-B` is one actor.
- **Q3** INVALID tokens are accepted and dropped at the start of a line; an ID-state line with no actor is INVALID and the state pops.
- **Q4** A re-declaration with a description overwrites the actor: `links` and `properties` reset, `prevActor` is the previous declaration (self for a repeat), and the previous actor's `nextActor` may point to itself (`sequenceDb.ts:208-225`). An implicit re-reference returns early and keeps the record (`:195-197`), but it still assigns the box (`:191-192`).
- **Q5** `box #color Team` is a comment: the box has no name, and its colour is `transparent`. Colour validation is `CSS.supports` (see section 9).
- **Q6** The "created/destroyed participant" error prints `undefined` for the name (`lastCreated` is an id string and `.name` is read).
- **Q7** `break` end text is dropped (`breakEnd` is applied with no text, `sequenceDb.ts:705-706`); its message is `""`.
- **Q8** Autonumber items store `{start, step, visible}` with undefined start and step for the bare forms, and no `activate` or `centralConnection`.
- **Q9** Entities: `#quot;` is stored as the placeholder `ﬂ°quot¶ß`, and the placeholders are not decoded in the model. Decoding is the renderer's job.
- **Q10** `links` and `addLinks`: `&equals;` and `&amp;` are replaced after sanitising, which never matches the placeholders (`:458-459`). `addALink` with no `@` takes the last two characters off as the label (`:474-478`).
- **Q11** `details` reads an element of the page. Without one (as in the exporter and in the specs that do not create one), it is a no-op with a logged error (`:526-546`).
- **Q12** A `title` takes one character after the keyword, so `title  x` keeps one leading blank, and a `title` whose value is all blanks replaces the front matter title with them (`:186`, `sequenceRenderer.ts:1450`).
- **Q13** `accTitle:` followed by a line break takes the next line as its value (the `\s*` after the colon spans lines).
- **Q14** `accTitle` and `accDescr` keep `;` and `#`, as the value rule takes the rest of the line.

## 9. Text, sanitising and colours

- Message, note, block and actor texts are raw (`addMessage` `:256-270`, `addNote` `:425-450`, `addActor` `:155-231`). Only the title, `accTitle`, `accDescr`, the box title, and the `links` and `properties` texts are sanitised. The sanitiser is `sanitize.rs`, with the config resolved as the reference does for `htmlLabels` true and `securityLevel` strict (the default); the exporter pins both.
- A box colour is valid when `CSS.supports('color', colour)` is true in Chromium. The port's rule is the exporter's (`tools/port/export-sequence.ts`, `cssSupports`): the CSS named colours, `transparent`, `currentcolor` and the CSS-wide keywords (`inherit` and the others, which Chromium accepts for any property), any ASCII case, and the functions `rgb`, `rgba`, `hsl` and `hsla` with CSS numbers and uniform argument kinds, in the comma or the space form, with an optional alpha. CSS system colours are not in the rule. The rule is the reference's own harness, not a Chromium check yet (section 12).

## 10. Specs

The parser- and DB-level blocks of `sequenceDiagram.spec.js` (`:97-1782` and `:2397-2908`), `keywordActors.spec.ts`, and the parts of the other spec files that do not render. The render blocks (`:1783-2396`, `actorBandModel`, `actorSizing`, `lifelineStart`, `svgDraw`, `sequenceRenderer`, `rectSectionFill`, `noteFontWeight`, `palettePicking`) are ignored with `#[ignore = "port: render, M6-R"]`, the same reason as the flowchart's layout tests, or passed when they are pure. The spec name of each test is kept. The Jison error texts become "fails, at a position".

## 11. Differential run

`tools/port/export-sequence.ts` runs the reference's `preprocessDiagram`, `encodeEntities`, the Jison parser and `SequenceDB` through the Vite loader (as `export-flowchart.ts`), with DOMPurify given a window. The wrap option is set as the config resolves it. Inputs: `collectInputs('sequence')` (the fixtures, the spec literals, the docs, and `cases.ts`). `pnpm port:diff sequence` compares them. `config` gets the `sequence` group. Both sides write an actor's `links` as `[key, value]` pairs, so their order is compared (canonical-model.md, Sequence); `cases.ts` has links in an order that is not sorted, repeated keys and array-index keys.

## 12. Open questions

1. Box colour validity in Chromium (section 9). The rule is the exporter's and is not checked in Chromium yet. The built-in browser pane could not run a script on a local file, so the check needs another route.
2. Resolved: an actor's `type` and `description` keep their YAML value (`Value`). The differential run confirmed a numeric type (`participant A@{ type: 5 }`).
3. Resolved: `links A: {…}` is the `links` statement. The reference agrees on the explicit `links` case of `cases.ts`.
4. Resolved: control items have `activate` false and `centralConnection` NONE, as `addSignal` gives them (`sequenceDb.ts:272-308`). NOTES 3.5 is wrong on this point; the differential run confirmed the code.

## 13. Renderer (M6-R)

The renderer is `crates/ferrigura-render/src/sequence/`. All 193 fixtures of `pnpm port:geometry sequence` are within tolerance, in both looks. The research is in NOTES section 5; this section lists what the port reproduces and where it differs.

**Reproduced (each one in the code, with the reference line):**

- **The bounds counter.** `updateBounds` shares one depth counter between the open blocks and the activations, so an activation can get a depth of zero or less (`sequenceRenderer.ts:57-80`, `bounds.rs` `update_bounds`).
- **`endActivation` with no activation of the actor.** `splice(-1, 1)` removes the last activation (`bounds.rs` `end_activation`).
- **Zero coordinates are false.** `calculateLoopBounds` tests `msgModel.startx && msgModel.stopx`, so a message at x = 0 does not widen its blocks (`layout.rs`).
- **The entity placeholders.** Text is measured as the parser encodes it, with the placeholders; the whole document is decoded at the end (`mermaidAPI.ts:359`, `mod.rs` `decode_entities`).
- **The entity shape's class.** An `entity` draws its group with the class `actor`, not `actor-man` (`svgDraw.js:891-997`, `actors.rs`).
- **The icons under sanitising.** Every security level but `loose` and `sandbox` runs DOMPurify on the output, whose allow-list has no `use`; an `@icon` is therefore not drawn (`mermaidAPI.ts:644-650`, `conf.rs` `sanitized`).
- **The root font size.** The root's `font-size` is the theme's `fontSize` (14 px in the redux themes), not the text's 16 px (`styles.ts`, `mod.rs` `write`).
- **The unknown actor type.** An actor of a type the switch does not know draws nothing and keeps its height (`svgDraw.js:1361-1404`).
- **Menu order.** `drawPopup` lists the links with `for…in` (`svgDraw.js:116`), in the key order of a JavaScript object: the array indices first, ascending, then the other keys in insertion order, with a repeated key in its first place. The model keeps that order (`JsObject`), and `actor_menu` in `sequence_render_spec.rs` checks the menu.
- **Menus with no box.** An actor with links whose type draws no box throws, as the reference reads `rectData.class` of `undefined` (`actors.rs` `draw_popup`).
- **Messages that cannot be measured.** A message whose text the measurement rejects is skipped, as the reference's `try` around the message does (`mod.rs`).
- **Section heights.** A block's section heights are computed by the reference and not drawn; the port does not keep them (no output depends on them).

**Named differences:**

- **Click handlers.** The `onclick` and `cursor` of an actor with links are not drawn (decision 0001). The menus themselves are drawn hidden (`display: none`), as the SVG of the reference, and `forceMenus` shows them.
- **Lifeline counter.** `actorCnt` is module state in the reference and carries over between renders in one page; the port counts from zero for each render. Only the `id`s of the lifelines and menus differ, which the harness does not compare.
- **Maths.** A text with `$$` is drawn as written. The reference renders KaTeX there; no fixture uses it, and ferrigura has no KaTeX.
- **`textPlacement`.** The reference reads `fo` and `old` from the configuration; the schema has no such key, so the port draws every label as `byTspan`.
- **`arrowMarkerAbsolute`.** The marker URLs are relative (`url(#id-…)`); the reference's absolute form uses `window.location`, which a server render does not have.
- **`themeCSS`.** Not applied (out of the M2 scope of the themes).
