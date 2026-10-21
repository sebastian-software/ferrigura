# Sequence diagram: research notes for M6

Read-only research for M6 (the remaining P1 diagram types). Reference: the JS tree of this worktree (`packages/mermaid/src`, commit `04b952b4e`, the same as `main`). Written 2026-10-16.

Legend: **[read]** read in the code, cited `file:line`. **[probe]** run against the reference: the Jison grammar with a recording stub, the reference's `preprocessDiagram` + `Diagram.fromText` + DB loaded through the port harness's Vite SSR loader (`tools/port/reference.ts`, read-only), or a full `mermaidAPI.render` in jsdom with a fake `getBBox`. Probe sources and outputs: `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/m6-sequence/` (not in the repo). **[derived]** follows from the code. **[unverified]** not checked in a browser or not run.

Paths below are relative to `packages/mermaid/src/` unless they start with `e2e/`, `crates/` or `tools/`.

---

## 0. Summary

| Question               | Answer                                                                                                                                                                                                                                 |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Renderer kind          | Custom. No `getData`, no ELK, no `rendering-util`. One pass over the messages with a `bounds` object (`diagrams/sequence/sequenceRenderer.ts:18-223`, `draw` at `:1043-1475`)                                                          |
| Grammar                | One Jison file, `diagrams/sequence/parser/sequenceDiagram.jison`, 423 lines, `%options case-insensitive` (`:13`). No Langium grammar, no `packages/parser` tests                                                                       |
| DB timing              | Most DB work runs in `yy.apply(document)` after the whole parse (`:139`). The title, accTitle and accDescr setters, `parseMessage` and `parseBoxData` run during the parse **[probe]**                                                 |
| Default look and theme | `neo` and `redux-color`, although the top level says `classic` and `default`. The diagram section defaults win in the resolution (`config.schema.yaml:2183-2187`, `config.ts:79-107`) **[probe]**                                      |
| Font sizes             | Actor and note text take the top-level `fontSize` (16) and `fontFamily`, not the section's 14 and its families (`sequenceRenderer.ts:876-888`). **Confirmed in a rendered SVG** (`font-size: 16px` on actor and note text) **[probe]** |
| Text sizes             | DOM-measured: `utils.calculateTextDimensions` appends an `<svg>` to `body`, reads `getBBox`, and rounds width and height to whole pixels (`utils.ts:665-730`). Not the flowchart's `createText` path                                   |
| Fixtures               | 193 `.mmd` in `e2e/diagrams/sequence/` (177 nightly, 10 pr-check, 6 visual-check). Every one has front matter; 56 have `%%{init}`                                                                                                      |
| Effort                 | About half of the flowchart M5 (estimate in section 7)                                                                                                                                                                                 |

---

## 1. Sources

| Area                   | File (lines)                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Role                                                                                                                                           |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Grammar and lexer      | `diagrams/sequence/parser/sequenceDiagram.jison` (423)                                                                                                                                                                                                                                                                                                                                                                                                                        | Lexer `:11-128`, parser `:130-423`                                                                                                             |
| Detector               | `diagrams/sequence/sequenceDetector.ts:9-11`                                                                                                                                                                                                                                                                                                                                                                                                                                  | `/^\s*sequenceDiagram/`, case-sensitive                                                                                                        |
| Diagram definition     | `diagrams/sequence/sequenceDiagram.ts:10-26`                                                                                                                                                                                                                                                                                                                                                                                                                                  | `parser`, `db` (new `SequenceDB` per access), `renderer`, `styles`, `init` (copies top-level `wrap` into `sequence.wrap`, `:17-25`)            |
| DB                     | `diagrams/sequence/sequenceDb.ts` (721)                                                                                                                                                                                                                                                                                                                                                                                                                                       | Setters, `apply` (`:549-710`), `LINETYPE` (`:34-91`)                                                                                           |
| Types                  | `diagrams/sequence/types.ts` (95)                                                                                                                                                                                                                                                                                                                                                                                                                                             | `Actor`, `Message`, `Box`, `Note`, `AddMessageParams`                                                                                          |
| Renderer               | `diagrams/sequence/sequenceRenderer.ts` (2152, `@ts-nocheck`)                                                                                                                                                                                                                                                                                                                                                                                                                 | `bounds` `:18-223`, `draw` `:1043-1475`, measurements and models `:1488-2144`                                                                  |
| SVG primitives         | `diagrams/sequence/svgDraw.js` (2169)                                                                                                                                                                                                                                                                                                                                                                                                                                         | Actor shapes `:391-1359`, `drawActor` `:1361-1443`, activation `:1477-1508`, loop `:1519-1619`, markers `:1681-2140`, text helper `:1811-1961` |
| Styles                 | `diagrams/sequence/styles.js` (156)                                                                                                                                                                                                                                                                                                                                                                                                                                           | CSS template `:3-154`                                                                                                                          |
| Neo band model         | `diagrams/sequence/actorBands.ts` (120)                                                                                                                                                                                                                                                                                                                                                                                                                                       | Glyph band, label block and datum rules, `:41-120`                                                                                             |
| Common helpers         | `diagrams/common/common.ts` (`lineBreakRegex` `:8`, `removeScript` `:48`, `sanitizeMore` `:56-70`, `sanitizeText` `:71-80`, `getMax` `:168`, `hasKatex` `:289`), `diagrams/common/commonDb.ts` (`:8-32`), `diagrams/common/svgDrawCommon.ts` (`drawRect`, `drawBackgroundRect` `:54`, `drawImage` `:88`, `drawEmbeddedImage` `:96`, `getNoteRect` `:109`)                                                                                                                     | Sanitising, title and accessibility setters, shared rectangles                                                                                 |
| Text measurement       | `utils.ts`: `wrapLabel` `:539`, `calculateTextWidth` `:649`, `calculateTextHeight` `:634`, `calculateTextDimensions` `:665-730`, `drawSimpleText` `:491`, `parseFontSize` `:823`, `encodeEntities` `:874`, `decodeEntities` `:904`, `getTextObj` `:467`                                                                                                                                                                                                                       | DOM-based measurement and entity encoding                                                                                                      |
| Pipeline               | `preprocess.ts` (`cleanupText` `:6`, `processDirectives` `:33`, `preprocessDiagram` `:58`), `diagram-api/comments.ts:6` (`cleanupComments`), `diagram-api/detectType.ts:36-47`, `Diagram.ts` (`fromText` `:19`, title from front matter `:53`, `encodeEntities(source) + '\n'` `:77`, `parser.parse` `:83`), `mermaidAPI.ts` (`processAndSetConfigs` `:72-87`, scope `:83`, parse `finally` `:114`, render `finally` `:689`, `decodeEntities` on the SVG `:359`)              | Shared path before the diagram code                                                                                                            |
| Config                 | `schemas/config.schema.yaml`: top level `theme` `:61-65` (default `default`), `look` `:68-70` (default `classic`), `fontFamily` `:271-277`, `fontSize` `:441-444` (default 16), `htmlLabels` `:267-270`; `SequenceDiagramConfig` `:2148-2317`; `defaultConfig.ts:49-70` (the `messageFont`, `noteFont`, `actorFont` functions); `config.ts` `readAppearance` `:42-56`, `resolveAppearance` `:79-107`, `setDiagramConfigScope` `:149-152`, `getEffectiveHtmlLabels` `:324-326` | Defaults, the scoping of `theme` and `look`, the font functions                                                                                |
| Theme                  | `themes/theme-redux-color.js` (sequence variables `:50-60`, palettes `:55-80`, `rectBkgColor` `:165-168`), `themes/theme-neo.js:91-111` (neo base), `themes/theme-base.js` and the other theme files (same names)                                                                                                                                                                                                                                                             | Variables read by `styles.js` and `svgDraw.js`                                                                                                 |
| Fixtures               | `e2e/diagrams/sequence/{nightly-check,pr-check,visual-check}/*.mmd`; snapshot harness `e2e/rendering/mmd-snapshots.spec.ts` with `e2e/helpers/mmd-snapshots.ts` (`DIAGRAMS_DIR`)                                                                                                                                                                                                                                                                                              | 193 inputs                                                                                                                                     |
| Browser specs          | `e2e/rendering/sequence/nightly-check/sequencediagram.spec.js` (181 lines, 6 tests)                                                                                                                                                                                                                                                                                                                                                                                           | Actor links, properties, destroy, directive overrides                                                                                          |
| Docs inputs            | `packages/mermaid/src/docs/syntax/sequenceDiagram.md` (38 mermaid blocks)                                                                                                                                                                                                                                                                                                                                                                                                     | More inputs for the differential run (not yet counted as fixtures)                                                                             |
| Rust, already in place | `crates/ferrigura-model/src/theme/redux_color.rs:90-96` (sequence variables, both palettes), `crates/ferrigura-syntax/src/lib.rs` (`DiagramType` has `Pie`, `Info`, `Flowchart`; no `Sequence` yet), `crates/ferrigura/src/lib.rs:72-73` (dispatch by type)                                                                                                                                                                                                                   | Reuse points                                                                                                                                   |

---

## 2. Parser

### 2.1 Pipeline, in order

```
text
 -> cleanupText        preprocess.ts:6-19   CRLF to LF; <tag attr="x"> -> <tag attr='x'>
 -> front matter       preprocess.ts:58-      title, config; front matter removed
 -> directives         preprocess.ts:33-      %%{init}%% and %%{wrap}%%; removed
 -> cleanupComments    comments.ts:6          drops lines matching ^\s*%%(?!{)[^\n]+\n?  (line-based, before lexing)
 -> detectType         detectType.ts:36-47    front matter and directives stripped again, then ^\s*sequenceDiagram
 -> Diagram.fromText   Diagram.ts:19-83
      db.clear(); init(config)                                    sequenceDiagram.ts:17-25
      if front matter title: db.setDiagramTitle(title)            Diagram.ts:52-54   (sanitised)
      textToParse = encodeEntities(text) + '\n'                    Diagram.ts:77; utils.ts:874-897
      parser.parse(textToParse)                                    Diagram.ts:83
        Jison actions: setDiagramTitle, setAccTitle,              (during the parse)
          setAccDescription, parseMessage, parseBoxData
        then yy.apply(document)                                    grammar :139 -> sequenceDb.ts:549
 -> renderer.draw       sequenceRenderer.ts:1043
```

**[probe]** `encodeEntities` runs before lexing, so `#quot;` reaches the DB as the placeholder `ﬂ°quot¶ß` (`utils.ts:884-895`). `decodeEntities` runs once on the final SVG string (`mermaidAPI.ts:359`, `utils.ts:904-906`). A bare `&amp;` never reaches the lexer intact, because `;` ends a statement (see 2.6).

**[read]** The entity placeholders are the only text transform in this pipeline that the DB sees. `cleanupText` also rewrites `<tag ...="x">` attributes (`preprocess.ts:11-16`), which changes the text of `<br/>` and HTML-like tags only when they carry attributes.

### 2.2 Lexer: rule table

Jison takes the first rule in definition order, not the longest match. Start states: `ID`, `ALIAS`, `LINE`, `CONFIG`, `CONFIG_DATA` (`:17`), `acc_title`, `acc_descr`, `acc_descr_multiline` (`:19-21`). Rules without a state prefix match in `INITIAL`.

| Lines   | Rule                                                                                                  | Token or action                                                                                                              | Notes                                                                                                                                     |
| ------- | ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| 24      | `[\n]+`                                                                                               | `NEWLINE`                                                                                                                    |                                                                                                                                           |
| 25      | `\s+`                                                                                                 | skip                                                                                                                         | **[read]** also skips newlines that are not at a token start; not probed                                                                  |
| 26      | `<ID,ALIAS,LINE>((?!\n)\s)+`                                                                          | skip                                                                                                                         | same-line blanks                                                                                                                          |
| 27      | `<INITIAL,ID,ALIAS,LINE>\#[^\n]*`                                                                     | skip                                                                                                                         | `#` comment to end of line; a `#` also ends the text of `title`, `box`, `loop` etc. **[probe]**                                           |
| 28      | `\%%(?!\{)[^\n]*`                                                                                     | skip                                                                                                                         | `%%` comment line                                                                                                                         |
| 29      | `[^\}]\%\%[^\n]*`                                                                                     | skip                                                                                                                         | **[read]** a `%%` after any non-`}` character in `INITIAL` starts a comment; inside message text the TXT rule (line 120) wins **[probe]** |
| 30      | `NUM` `([0-9]+(\.[0-9]{1,2})?\|\.[0-9]{1,2})(?=[ \n]+)`                                               | `NUM`                                                                                                                        | decimals up to 2 places; must be followed by blank or newline                                                                             |
| 31-34   | `CONFIG` state                                                                                        | `@{` → `CONFIG_START`; `[^\}]+` → `CONFIG_CONTENT`; `}` before `as` → `CONFIG_END` and ALIAS; `}` → `CONFIG_END`, pops twice | the content is raw text, trimmed by the grammar (`:372`)                                                                                  |
| 35-39   | `ID` state                                                                                            | `ACTOR` for `[^<>:\n,;@\s]+` before `@{` or `as`; `[^<>:\n,;@]+` before `\s*[\n;#]`; `INVALID` otherwise                     | `A:B` has no valid form; see 2.6                                                                                                          |
| 40-44   | `box`, `participant`, `actor`, `create`, `destroy`                                                    | `LINE` or `ID` state                                                                                                         | `actor` returns `participant_actor`                                                                                                       |
| 45-46   | `ALIAS` `as` / empty                                                                                  | pops and goes to `LINE`                                                                                                      |                                                                                                                                           |
| 47-58   | `loop`, `rect`, `opt`, `alt`, `else`, `par`, `par_over`, `and`, `critical`, `option`, `break`         | keyword, then `LINE`                                                                                                         | `LINE` `restOfLine` (`:58`): `(?:[:]?(?:no)?wrap:)?[^#\n;]*`                                                                              |
| 59-61   | `end`, `left of`, `right of`                                                                          | keyword                                                                                                                      |                                                                                                                                           |
| 62-65   | `links`, `link`, `properties`, `details`                                                              | keyword only with lookahead `(?=[ \t]+[^\/\\\+\(\)<>:\n,;\s\-])`                                                             | `links A: {..}` is a text statement, not `links`, because `:` follows the id                                                              |
| 66-69   | `over`, `note`, `activate`, `deactivate`                                                              | keyword; `activate`/`deactivate` enter `ID`                                                                                  |                                                                                                                                           |
| 70-71   | `"title"\s[^#\n;]+`, `"title:"\s[^#\n;]+`                                                             | `title`, `legacy_title`                                                                                                      | needs a blank and at least one character; `title:` with nothing after is a parse error **[probe]**                                        |
| 72-78   | `accTitle\s*":"\s*`, `accDescr\s*":"\s*`, `accDescr\s*"{"\s*` and their value rules                   | `acc_title`, `acc_descr`, multi-line                                                                                         | `\s*` after `:` spans newlines: `accTitle: ` followed by a line takes that line as the title **[probe]**                                  |
| 79-83   | `sequenceDiagram`, `autonumber`, `off`, `,`, `;`                                                      | `SD`, `autonumber`, `off`, `,`, `NEWLINE`                                                                                    | `;` is a statement separator                                                                                                              |
| 84      | ACTOR, generic: `[^\/\\\+\()\+<\->\->:\n,;]+(…)*`                                                     | `ACTOR`, trimmed                                                                                                             | the long regex with a negative lookahead on the arrow forms; `A-B` is one actor **[probe]**; `Good Name` is one actor **[probe]**         |
| 85-94   | `->>`, `<<->>`, `-->>`, `<<-->>`, `->`, `-->`, `-x`, `--x`, `-)`, `--)`                               | arrow tokens                                                                                                                 |                                                                                                                                           |
| 95-106  | `--\|\\`, `--\|/`, `--\\\\`, `--//` (dotted normal); `/\|--`, `\|--`, `//--`, `\\--` (dotted reverse) | arrow tokens                                                                                                                 |                                                                                                                                           |
| 107-118 | `-\|\\`, `-\|/`, `-\\\\`, `-//` (normal); `/\|-`, `\|-`, `//-`, `\\-` (reverse)                       | arrow tokens                                                                                                                 |                                                                                                                                           |
| 120-121 | `":"(?:(?:no)?wrap:)?[^#\n;]*`                                                                        | `TXT`                                                                                                                        | the text after a colon; `#` and `;` end it                                                                                                |
| 122-123 | `+`, `-`                                                                                              | activation markers                                                                                                           |                                                                                                                                           |
| 124     | `()`                                                                                                  | central connection marker                                                                                                    |                                                                                                                                           |
| 125     | `<<EOF>>`                                                                                             | `NEWLINE`                                                                                                                    |                                                                                                                                           |
| 126     | `.`                                                                                                   | `INVALID`                                                                                                                    |                                                                                                                                           |

Keywords are matched case-insensitively (`:13`). **[probe]** `SEQUENCEDIAGRAM` and `PARTICIPANT A` parse. The detector does not (`sequenceDetector.ts:10`), so a diagram needs a lowercase header in practice.

### 2.3 Grammar: statements and their actions

Each row lists the parse-time yy calls (run while parsing) and the items that `apply` later receives. **[probe]** for the item shapes.

| Statement (grammar line)                                                                                | Parse-time yy calls                                              | Items passed to `apply` (in order)                                                                                                    | DB effect in `apply`                                                                                         |
| ------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------- | ------------ | ------------------------------------------------------- |
| `SD document` (`:139`)                                                                                  | none                                                             | `yy.apply($2)` on the whole list, then returns it                                                                                     | everything below                                                                                             |
| `participant`, `actor`, with `AS` alias or config (`:263-274`)                                          | `parseMessage` on the alias                                      | `addParticipant {actor, draw, description?, config?}`                                                                                 | `addActor` (`sequenceDb.ts:155-231`)                                                                         |
| `create participant` (`:168`)                                                                           | as above                                                         | `createParticipant`                                                                                                                   | `:573-582`: error if the id exists; `lastCreated`; `createdActors[id] = messages.length`                     |
| `destroy` (`:268`)                                                                                      | none                                                             | `destroyParticipant`                                                                                                                  | `:583-586`: `lastDestroyed`; `destroyedActors[id] = messages.length`                                         |
| `signal` (`:331-356`), e.g. `A->>B: t`                                                                  | `parseMessage` on `text2` (`:420-422`)                           | `[actorA, actorB, addMessage {from, to, signalType, msg, activate?, centralConnection?}, activeStart/activeEnd/centralConnection...]` | `addActor` twice (second is a no-op), `addMessage` (`:615-647`), the activation or central connection signal |
| `note_statement` (`:276-287`)                                                                           | `parseMessage`                                                   | `[actor refs, addNote {placement, actor, text}]`                                                                                      | `addNote` (`:425-450`): pushes a note and a message of type NOTE                                             |
| `links`, `link`, `properties`, `details` (`:289-315`)                                                   | `parseMessage`                                                   | `[actor, addLinks                                                                                                                     | addALink                                                                                                     | addProperties | addDetails]` | `:452-510`, `:526-546` (details reads the DOM, see 3.3) |
| `autonumber` forms (`:175-178`)                                                                         | none                                                             | `sequenceIndex {start, step, visible}`                                                                                                | pushes a message with `type = 26`; `start`, `step` from `Number()` (`1.25` accepted)                         |
| `activate` / `deactivate` (`:179-180`)                                                                  | none                                                             | `activeStart` / `activeEnd`                                                                                                           | `addSignal` (`:272-308`); an inactive end throws                                                             |
| `box restOfLine box_section end` (`:169-173`)                                                           | `parseBoxData` on the rest of the line (`sequenceDb.ts:394-419`) | `boxStart {boxData}`, the box's participants, `boxEnd {boxText}`                                                                      | `addBox` (`:145-153`) sets `currentBox`; `boxEnd` clears it                                                  |
| `loop`, `rect`, `opt`, `alt`/`else`, `par`/`and`, `par_over`, `critical`/`option`, `break` (`:191-242`) | `parseMessage` on the rest of the line                           | `*Start {text}`, the nested items, `*End` (or `else`/`and`/`option` markers)                                                          | `addSignal` with the text; `rect` uses the text as a fill colour                                             |
| `title` (`:186-187`)                                                                                    | `setDiagramTitle(text after "title ")` (not trimmed)             | `$$` only                                                                                                                             | `commonDb.setDiagramTitle` sanitises (`:28-30`)                                                              |
| `accTitle:` (`:188`)                                                                                    | `setAccTitle(trim)`                                              |                                                                                                                                       | sanitises and drops leading blanks (`commonDb.ts:16-18`)                                                     |
| `accDescr:` and `accDescr { }` (`:189-190`)                                                             | `setAccDescription(trim)`                                        |                                                                                                                                       | sanitises, `\n\s+` → `\n` (`commonDb.ts:22-24`)                                                              |

**[probe]** Signals list the two participant declarations before the message, so `addActor` runs once per declaration reference; a repeated implicit reference returns early (`sequenceDb.ts:195-197`, the `description == null` test), so an explicit `participant A as Alice` later overwrites the record (see 2.7).

### 2.4 DB state, method by method

| Method (line)                                                              | State it changes                                                                                       | Notes                                                                                                                                                                                                                                                        |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `addActor(id, name, description, type, metadata)` (`:155-231`)             | `actors` (Map, insertion order), `prevActor`, the previous actor's `nextActor`, `currentBox.actorKeys` | `metadata` is YAML: `{\n…\n}` for one line (`:167-171`), `yaml.load` JSON_SCHEMA (`:172`); only `type` and `alias` are read. `config` is not stored. `type = doc.type ?? type` (`:174`); `alias` replaces the name when no description is given (`:177-179`) |
| `addBox` (`:145-153`)                                                      | `boxes`, `currentBox`                                                                                  | `name` = text (may be undefined), `fill` = colour string or `transparent`                                                                                                                                                                                    |
| `addSignal(from, to, msg, type, activate, centralConnection)` (`:272-308`) | `messages`                                                                                             | ACTIVE_END with no open activation throws `Trying to inactivate an inactive participant (X)` (`:280-296`)                                                                                                                                                    |
| `addMessage` (`:256-270`)                                                  | `messages`                                                                                             | `id = messages.length`; `wrap = message.wrap ?? autoWrap()`                                                                                                                                                                                                  |
| `addNote` (`:425-450`)                                                     | `notes`, `messages`                                                                                    | `actor` may be an array `[A, B]` for `over` (`:437-439`)                                                                                                                                                                                                     |
| `addLinks`, `addALink`, `addProperties` (`:452-520`)                       | actor `links`, `properties`                                                                            | JSON or `label @ url`, see 3.3                                                                                                                                                                                                                               |
| `apply` (`:549-710`)                                                       | everything above                                                                                       | errors below                                                                                                                                                                                                                                                 |

Errors thrown inside `apply` (exact texts from the reference):

| Condition                                                              | Text (source)                                                                                                                                                                                                                             |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `create` of an id that exists (`:574-578`)                             | `It is not possible to have actors with the same id, even if one is destroyed before the next is created. Use 'AS' aliases to simulate the behavior`                                                                                      |
| message after `create` not involving the created actor (`:615-621`)    | `The created participant X does not have an associated creating message after its declaration. Please check the sequence diagram.` **[probe]** `X` is `undefined`, because `lastCreated` holds the id string and `.name` is read (`:619`) |
| message after `destroy` not involving the destroyed actor (`:625-634`) | the same shape with `destroyed`; **[probe]** `X` is `undefined`                                                                                                                                                                           |
| actor declared in two boxes (`:184-187`)                               | `A same participant should only be defined in one Box: A can't be in 'Red X' and in 'Blue Y' at the same time.` **[probe]**                                                                                                               |

### 2.5 Comments, directives, front matter, title

- **Comments.** Lines starting with `%%` (not `%%{`) are removed before lexing (`comments.ts:6`, **[probe]** via the reference pipeline). `#` starts a comment in `INITIAL`, `ID`, `ALIAS` and `LINE` (`:27`), so `#` cuts `title`, `box`, `loop` and `rect` text. `%%` inside a message is text (`A->>B: x %% y` keeps `x %% y`, **[probe]**).
- **Directives.** `%%{init}%%` and `%%{wrap}%%` are read by `preprocess.ts:33-40` (`detectInit`, `detectDirective 'wrap'`), then removed. A `wrap` directive sets `sequence.wrap` through `init` (`sequenceDiagram.ts:21-23`) and `setConfig` (`:23`). For the `wrap` flag: `autoWrap()` (`sequenceDb.ts:366-373`) returns the `setWrap` value, else `getConfig().sequence.wrap`, else `false`.
- **Front matter.** The `title` key is read by `preprocess.ts:58-`, passed to `Diagram.fromText` (`Diagram.ts:52-54`). A body `title` replaces it. A whitespace-only body title (`title` followed by spaces) replaces it with those spaces **[probe]**: `title   ` gives `"   "`, and the renderer draws it because it is truthy (`sequenceRenderer.ts:1450`).
- **Title precedence.** The last body `title` (or legacy `title:`, `:187`) replaces the front matter title. A body `title` cannot be empty (parse error), and the grammar does not trim it (`:186`), so trailing spaces stay and a whitespace-only title replaces the front matter title **[probe]**.

### 2.6 Error behaviour

| Input                                                | Result                                                                   | Where                               |
| ---------------------------------------------------- | ------------------------------------------------------------------------ | ----------------------------------- |
| `A->>B` without `:`                                  | parse error, line 2, expects `TXT` **[probe]**                           | grammar                             |
| `Note over A` without `:`                            | parse error, expects `,` or `TXT` **[probe]**                            | grammar                             |
| `title a;b`                                          | parse error at `;` **[probe]**                                           | `;` ends the statement (`:83`)      |
| `title: ` (empty legacy title)                       | parse error **[probe]**                                                  | `:71` needs a character             |
| `A:B->>C: x`                                         | parse error **[probe]**                                                  | `:` is not in an id (`:35-39`)      |
| no `sequenceDiagram` header                          | parse error at line 1 **[probe]**                                        | `:139`                              |
| `loop` / `box` without `end`                         | parse error at end of input **[probe]**                                  | `:191-242`                          |
| `a &amp; b` in a message                             | parse error **[probe]**                                                  | `;` ends the TXT token (`:120`)     |
| `participant D@{ "type: "entity" }` (malformed YAML) | error (spec `sequenceDiagram.spec.js:2598`)                              | `yaml.load` in `addActor`           |
| `participant A@{ "type": }`                          | not an error: YAML gives `null`, so the default type is used **[probe]** | `sequenceDb.ts:174`                 |
| `links A: {bad}`                                     | silently ignored (`log.error`) **[probe]**                               | `:460-465`                          |
| `link A: nolinkhere` (no `@`)                        | stores `{"nolinkhe": "nolinkhere"}` **[probe]**                          | `sequenceDb.ts:474-478`: `sep = -1` |
| `details A: x` with no element of that id            | silently ignored **[probe]**                                             | `:526-545`, DOM                     |
| inactive `-->>-`                                     | DB error, text in 2.4 **[probe]**                                        | `:280-296`                          |
| duplicate `create`                                   | DB error **[probe]**                                                     | `:574-578`                          |

Specs check errors with `try { … } catch { error = true }` and `expect(error).toBe(true)` (`sequenceDiagram.spec.js:913-937`, `:2598-2612`). Nothing in the sequence specs asserts a Jison message text, so ADR 0002's "fails, and where" rule applies with no exceptions. DB errors carry no position (`apply` has no source locations).

### 2.7 Quirks to keep or decide (all **[probe]** unless marked)

1. **Re-declaring an actor with a description overwrites its record.** `sequenceDb.ts:208-219` sets a new record: `links` and `properties` are reset, `prevActor` is the previous declaration, and `prevActor.nextActor` is set to the actor's own id. Example: `participant A`, `links A: {...}`, `participant A as Alice` gives `links: {}` and `prevActor: "A"`, `nextActor: "A"` (a self loop). `A->>B`, `participant A as Alice`, `B->>A` gives `B.nextActor = "A"` and `B.prevActor = "A"`.
2. **Implicit re-references do not overwrite.** `A->>B` after `participant A as Alice` keeps `Alice` (`:195-197`).
3. **`createdActors`/`destroyedActors` store the message count**, not an index of actors (`sequenceDb.ts:581`, `:585`).
4. **`box` text and colour.** `box #ff0000 Team` gives no name (the `#` is a comment). `box Purple Alice & Bob` depends on the browser: `window.CSS.supports('color', …)` (`:400-404`) decides between colour and text. The jsdom probe used the `Option` path and rejected named colours, so the split is **[unverified]** in Chromium (see 7).
5. **Unknown actor type is not drawn.** `participant A@{ "type": "zzz" }` stores `type: "zzz"`, and `drawActor` has no case for it (`svgDraw.js:1376-1442`). **[probe]** No rectangle or lifeline is drawn; the actor still takes its margin.
6. **`break` end text is dropped.** The grammar gives `breakEnd` an `optText` field (`sequenceDiagram.jison:241`), but `apply` stores `breakEnd` with no text (`sequenceDb.ts:705-706`), so the stored message is `""` **[probe]**. The `breakStart` text is kept.
7. **Autonumber items** store `message: {start, step, visible}` with `from` and `to` undefined (`sequenceDb.ts:557-568`). Every other item stores `message` as a string.
8. **Entities**: `#quot;` is stored as `ﬂ°quot¶ß` (in messages, titles, link keys, box titles). `links` replaces `&equals;` and `&amp;` after sanitising (`:458-459`), which never match the placeholders, so keys keep them **[probe]**.
9. **`Link`, `links`, `properties`, `details` are valid actor ids** in a message (`keywordActors.spec.ts`, probe `Link->>links: x`).
10. **Empty `accTitle:` with a following line** takes the following line as the title **[probe]**.
11. **`accTitle` and `accDescr` keep `;` and `#`** (`acc_title_value` `:73`), unlike `title` **[probe]**.
12. **Multi-line `accDescr { }`** keeps the second line's indentation in the grammar, and `setAccDescription` removes it (`\n\s+` → `\n`) **[probe]**: `"line one\nline two"`.
13. **Sequence numbers** rounded to two decimals on every increment (`sequenceRenderer.ts:1380`).
14. **Actor `wrap`** depends on config: `description.wrap ?? autoWrap()` (`sequenceDb.ts:212`). Same for messages and notes (`:267`, `:434`).

---

## 3. Canonical model proposal

### 3.1 Shape (camelCase, version 1)

The reference DB state after `Diagram.fromText`, minus render-only fields. Every field is read from the probes' dumps (`probe-diagram.mts`).

```json
{
  "version": 1,
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "actorKeys": ["Alice", "John"],
  "actors": [
    {
      "id": "Alice",
      "name": "Alice",
      "description": "Alice",
      "type": "participant",
      "wrap": false,
      "box": null,
      "prevActor": null,
      "nextActor": "John",
      "links": {},
      "properties": {}
    }
  ],
  "boxes": [{ "name": null, "wrap": false, "fill": "transparent", "actorKeys": ["A", "B"] }],
  "messages": [
    {
      "id": "0",
      "from": "Alice",
      "to": "John",
      "type": "SOLID",
      "message": "Hello John",
      "wrap": false,
      "activate": false,
      "centralConnection": "NONE"
    }
  ],
  "createdActors": { "C": 0 },
  "destroyedActors": { "B": 1 }
}
```

Notes on the shape:

- `type` on messages: the reference uses numbers from `LINETYPE`. The model should use names (`SOLID`, `DOTTED_CROSS`, …) so that the exporter can map them; the numeric table is in `sequenceDb.ts:34-91`. Items that are not messages (`loop`, `note`, `activeStart` …) keep their own `type` name.
- `messages[].message` is a string for every item except autonumber, where it is `{start, step, visible}` (`sequenceDb.ts:557-568`). Model it as an enum or a separate variant.
- `placement`: `LEFTOF` 0, `RIGHTOF` 1, `OVER` 2 (`sequenceDb.ts:98-102`), as the names `left_of`, `right_of`, `over`.
- `actors[].box` is the box name in the reference (`box.name`), with `null` for none. Two boxes with the same name are indistinguishable there; use the box index in the model and keep the name too.
- `actors[].prevActor` and `nextActor` are DB state that the renderer reads (`sequenceRenderer.ts:1500-1507`, `:1656`). Keep them as the DB sets them, quirks included (2.7 item 1). Recomputing them from declaration order would not reproduce the reference.
- `createdActors` and `destroyedActors` are DB state, used by the renderer (`sequenceRenderer.ts:1002-1032`).

### 3.2 Left out, and why

| Left out                                                                              | Why                                                                                                                                                                                     |
| ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `actorCnt`, `rectData`, `width`, `height`, `margin`, `x`, `starty`, `stopy` on actors | Renderer state. `actorCnt` is a module global of the renderer (`svgDraw.js:366`)                                                                                                        |
| `lastCreated`, `lastDestroyed`, `prevActor` (state), `currentBox`                     | Parser or DB scratch state                                                                                                                                                              |
| `sequenceNumbersEnabled`, the `showSequenceNumbers()` value                           | Set by the renderer during the message loop (`sequenceRenderer.ts:1269-1276`), so it is render state. The model keeps the `autonumber` items and the renderer computes visibility       |
| `wrapEnabled`, `autoWrap()`                                                           | Config. Keep `wrap` as written (`true`, `false`, or `null` when not written) and let the port resolve the default, or pass the default as a parser option the way `maxEdges` is (see 7) |
| `config` of an actor                                                                  | Not stored by the DB; only `type` and `alias` survive (`sequenceDb.ts:172-179`)                                                                                                         |
| `notes` list                                                                          | The DB keeps a second copy as messages of type NOTE (`:440-449`). The model keeps the messages                                                                                          |
| SVG and DOM fields: `anchored`, `rectData`, `fromBounds`, `toBounds`                  | Render state                                                                                                                                                                            |

### 3.3 Text and sanitisation

| Text                                                                            | DB treatment                                                                                              | Source                                                               |
| ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Message text, note text, loop/alt/opt/par/rect labels, actor description, alias | **Not sanitised**; raw; `<b>` stays as written **[probe]**                                                | `addMessage` `:256-270`, `addNote` `:425-450`, `addActor` `:155-231` |
| Diagram title (front matter and body)                                           | `sanitizeText` (DOMPurify)                                                                                | `commonDb.ts:28-30`                                                  |
| `accTitle`, `accDescr`                                                          | `sanitizeText`, then leading or `\n\s+` rules                                                             | `commonDb.ts:16-24`                                                  |
| Box title                                                                       | `sanitizeText`, after `extractWrap`                                                                       | `sequenceDb.ts:413-416`                                              |
| Links and properties                                                            | `sanitizeText`, then `&equals;` and `&amp;` replacements (never matching placeholders), then `JSON.parse` | `:452-510`                                                           |
| Box colour                                                                      | not sanitised; validated by the browser's CSS colour support                                              | `:394-412`                                                           |

`sanitizeText` (`common.ts:71-80`) calls `sanitizeMore` (`:56-70`), which depends on config:

| `htmlLabels`   | `securityLevel`             | `sanitizeMore`                        | Result for `<b>T</b> & x`                                              |
| -------------- | --------------------------- | ------------------------------------- | ---------------------------------------------------------------------- |
| true (default) | strict, antiscript, sandbox | `removeScript` (DOMPurify default)    | `<b>T</b> &amp; x` **[probe]**                                         |
| true           | loose                       | none                                  | as above                                                               |
| true           | other (not loose)           | `<`, `>`, `=` escaped to placeholders | `&lt;b&gt;T&lt;/b&gt; &amp; x` **[read]**                              |
| false          | any                         | none                                  | `<b>T</b> &amp; x` **[read]**; M5's harness forces `htmlLabels: false` |

Then `DOMPurify.sanitize(…, { FORBID_TAGS: ['style'] })` (`:71-79`) in every case. So the canonical title depends on `htmlLabels` and `securityLevel`. The exporter must pin both, as M5 did.

### 3.4 Title precedence (from the probes)

| Front matter `title` | Body `title` line  | Model `title`               |
| -------------------- | ------------------ | --------------------------- |
| none                 | none               | `""`                        |
| `FM`                 | none               | `FM`                        |
| `FM`                 | `Body`             | `Body`                      |
| `FM`                 | whitespace only    | the whitespace (not `FM`)   |
| none                 | `title My # title` | `My ` (trailing space kept) |

### 3.5 Other rules

- **Order.** `actorKeys` in first-declaration order (Map insertion order, `sequenceDb.ts:325-340`); a re-declaration keeps its position (probe `redeclare`). `messages` in document order, with nested blocks flattened by `apply` (`:549-557`).
- **Ids.** Message ids are the decimal index in `messages`, as strings (`:263`, `:298`). Keep that.
- **Undefined.** `from`, `to`, `placement`, `activate`, `centralConnection` are `undefined` for control items (probe). The model uses `null`, or leaves the field out, but must not write `"undefined"`.
- **Central connections.** `centralConnection` is `0` (none), `59` (`CENTRAL_CONNECTION`), `60` (`REVERSE`) or `61` (`DUAL`), and the parser also adds `activate` (probe `central-connections`).

---

## 4. Specs

### 4.1 Files and counts

Counts are `it(` and `it.each(` blocks from grep, then expanded where a table or `forEach` runs them more than once. Expansions are estimates, not a `vitest list` run.

| Spec file                              | Blocks                                                                           | Cases (≈)      | Level                                                                                                                                                                 | DOM need                                                                                                                                                                                                |
| -------------------------------------- | -------------------------------------------------------------------------------- | -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `sequenceDiagram.spec.js` (2908 lines) | 140 `it`, 4 `it.each` (3, 3, 3, 2 rows), 2 `forEach` of 4 `textPlacement` values | ≈153           | parse and DB (`:97-1782`, 83 blocks); bounds (`:1783-1916`, 6); render through `draw()` (`:1917-2396`, 24 incl. the `forEach`); parse and DB class (`:2397-2908`, 32) | No DOM for parse and DB. `vi.mock('d3')` starts at `:20` for render: `select` returns a stub without `remove`, so `calculateTextDimensions` returns 0 for all text (`utils.ts:687` `if (!body.remove)`) |
| `svgDraw.spec.js`                      | 11                                                                               | 11             | unit, own d3 mock with `getBBox` (`:245-259`)                                                                                                                         | mocked                                                                                                                                                                                                  |
| `sequenceRenderer.spec.js`             | 1                                                                                | 1              | unit: `drawMessage` text position (`:60-95`), `svgDraw` and `utils` mocked (`:4-22`)                                                                                  | mocked                                                                                                                                                                                                  |
| `actorBandModel.spec.ts`               | 5                                                                                | 5              | render, neo band model; `getBBox` stub on `SVGElement.prototype` (`:85`)                                                                                              | yes: jsdom and a `getBBox` stub                                                                                                                                                                         |
| `actorSizing.spec.ts`                  | 5 + `it.each(['classic','neo'])`                                                 | 6              | render, figure size and label position                                                                                                                                | yes (`:18-44`)                                                                                                                                                                                          |
| `lifelineStart.spec.ts`                | 6                                                                                | 6              | render, lifelines and label baselines, classic and neo                                                                                                                | yes (`:24-37`)                                                                                                                                                                                          |
| `keywordActors.spec.ts`                | 10 + `it.each` (3 names)                                                         | 12             | parse and DB                                                                                                                                                          | no                                                                                                                                                                                                      |
| `palettePicking.spec.ts`               | 7 + `it.each` (2 themes)                                                         | 8              | unit: `paletteColor` and the palette rules (`svgDraw.js:68-69`)                                                                                                       | no                                                                                                                                                                                                      |
| `rectSectionFill.spec.ts`              | 1 + `it.each(themeNames)` ×2                                                     | 2 × themes + 1 | theme: `rectBkgColor`                                                                                                                                                 | no                                                                                                                                                                                                      |
| `noteFontWeight.spec.ts`               | 1 + `it.each(themeNames)`                                                        | themes + 1     | styles and theme                                                                                                                                                      | no                                                                                                                                                                                                      |

`themeNames = Object.keys(themes)` (`rectSectionFill.spec.ts:17`), so the theme count multiplies. The total is ≈ 205 cases plus the theme expansions. STATUS's "≈ 198" is in the same range.

There is no `packages/parser/tests` file for sequence (no Langium grammar). The Playwright spec is in section 4.2.

### 4.2 Browser specs

`e2e/rendering/sequence/nightly-check/sequencediagram.spec.js` (6 tests): directive override (2), actor links (1), links and properties (2, one not mirrored), destroy and recreate (1). The link tests use the actor popup, which is JavaScript: `onclick` toggles `display` (`svgDraw.js:142-148`, `:403-405`). That conflicts with decision 0001 (no JavaScript callbacks), so these tests cannot pass as written. Options are in section 7.

### 4.3 Level classification for the port

| Level                        | Specs                                                                                         | Port crate                                                                                                                                  |
| ---------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Parser (grammar to DB)       | parse blocks in `sequenceDiagram.spec.js`, `keywordActors.spec.ts`, the DB-class block        | `ferrigura-syntax` tests, `tests/sequence_*.rs`                                                                                             |
| DB state                     | `bounds` block and the DB getters                                                             | `ferrigura-syntax`                                                                                                                          |
| Layout, DOM-free             | the `bounds` tests, `rectSectionFill`, `noteFontWeight`, `palettePicking`, `svgDraw` (mocked) | `ferrigura-render` unit tests                                                                                                               |
| Layout with DOM              | `actorBandModel`, `actorSizing`, `lifelineStart`                                              | `ferrigura-render` tests against the reference's layout numbers (no browser needed for the layout, but the text measures are fixed numbers) |
| Render through the mocked d3 | `sequenceDiagram.spec.js` render block                                                        | Note: its expectations hold with zero text sizes, so they do not test text measurement                                                      |

---

## 5. Renderer

### 5.1 Unified or custom

Custom. `sequenceDiagram.ts:7` imports `renderer` from `sequenceRenderer.ts`. The renderer is `draw(text, id, version, diagObj)` (`sequenceRenderer.ts:1043`). It does not use `getData`, the rendering-util layouts, ELK or `rendering-util/render.ts`. Neither the flowchart's shape registry nor its markers are used. Its own markers are in `svgDraw.js:1681-2140`.

### 5.2 Configuration read by the renderer

Defaults from the schema (`SequenceDiagramConfig`, `config.schema.yaml:2148-2317`) and the effective value after `setConf` (`sequenceRenderer.ts:876-888`). The probe prints the effective values from `getConfig()` after the diagram scope is set.

| Key                      | Schema line                                               | Default                                       | Effective (probe)          | Used in                                                                    |
| ------------------------ | --------------------------------------------------------- | --------------------------------------------- | -------------------------- | -------------------------------------------------------------------------- |
| `theme`                  | `:2183`                                                   | `redux-color`                                 | `redux-color`              | colour palettes (`svgDraw.js` `COLOR_THEMES` `:47`)                        |
| `look`                   | `:2186`                                                   | `neo`                                         | `neo`                      | band model, drop shadow, rect radius, `data-look`                          |
| `hideUnusedParticipants` | `:2191`                                                   | `false`                                       | `false`                    | `sequenceRenderer.ts:1087-1094`                                            |
| `activationWidth`        | `:2194`                                                   | 10                                            | 10                         | activation and arrow geometry                                              |
| `diagramMarginX` / `Y`   | `:2199`, `:2204`                                          | 50 / 10                                       | 50 / 10                    | viewBox and size (`:1438`, `:1448`, `:1465`)                               |
| `actorMargin`            | `:2209`                                                   | 50                                            | 50                         | `calculateActorMargins` (`:1667`)                                          |
| `width`                  | `:2214` (ref `JourneyDiagramConfig/properties/width`)     | 150                                           | 150                        | actor width, note width                                                    |
| `height`                 | `:2217` (ref)                                             | 65                                            | 65                         | `conf.height`, minimum row height                                          |
| `boxMargin`              | `:2220` (ref)                                             | 10                                            | 10                         | margins everywhere (41 uses)                                               |
| `boxTextMargin`          | `:2223` (ref)                                             | 5                                             | 5                          | box and loop text                                                          |
| `noteMargin`             | `:2226` (ref)                                             | 10                                            | 10                         | notes                                                                      |
| `messageMargin`          | `:2229` (ref)                                             | 35                                            | 35                         | **not read by the renderer** (grep)                                        |
| `messageAlign`           | `:2232` (ref)                                             | `center`                                      | `center`                   | message text anchor                                                        |
| `mirrorActors`           | `:2235`                                                   | `true`                                        | `true`                     | footer actors (`:1392`)                                                    |
| `forceMenus`             | `:2240`                                                   | `false`                                       | `false`                    | popup visibility                                                           |
| `bottomMarginAdj`        | `:2245` (ref)                                             | 1                                             | 1                          | height with mirror (`:1440`)                                               |
| `rightAngles`            | `:2248` (ref)                                             | `false`                                       | `false`                    | self messages (`:428`, `:501`)                                             |
| `showSequenceNumbers`    | `:2251`                                                   | `false`                                       | `false`                    | autonumber visibility                                                      |
| `actorFontSize`          | `:2255`                                                   | 14                                            | **16**                     | `setConf` overwrites it                                                    |
| `actorFontFamily`        | `:2259`                                                   | `"Open Sans", sans-serif`                     | **top-level family**       | `setConf` overwrites it                                                    |
| `actorFontWeight`        | `:2263`                                                   | 400                                           | 400                        | actor text                                                                 |
| `noteFontSize`           | `:2268`                                                   | 14                                            | **16**                     | `setConf` overwrites it                                                    |
| `noteFontFamily`         | `:2272`                                                   | `"trebuchet ms", verdana, arial, sans-serif`  | **top-level family**       | `setConf` overwrites it                                                    |
| `noteFontWeight`         | `:2276`                                                   | 400                                           | 400                        | note text                                                                  |
| `noteAlign`              | `:2280`                                                   | `center`                                      | `center`                   | note text anchor                                                           |
| `messageFontSize`        | `:2286`                                                   | 16                                            | 16                         | message text                                                               |
| `messageFontFamily`      | `:2290`                                                   | `"trebuchet ms", verdana, arial, sans-serif`  | top-level family           |                                                                            |
| `messageFontWeight`      | `:2294`                                                   | 400                                           | 400                        |                                                                            |
| `wrap`                   | `:2299`                                                   | `false`                                       | `false`                    | `autoWrap` (`sequenceDb.ts:366-373`)                                       |
| `wrapPadding`            | `:2303` (ref `C4DiagramConfig`)                           | 10                                            | 10                         | label wrap and widths                                                      |
| `labelBoxWidth`          | `:2306`                                                   | 50                                            | 50                         | loop label box                                                             |
| `labelBoxHeight`         | `:2310`                                                   | 20                                            | 20                         | loop label box (+15 in neo)                                                |
| `useMaxWidth`            | not in the sequence section; `BaseDiagramConfig` (`:464`) | true                                          | `true`                     | root `width="100%"` and `style="max-width"` (`setupGraphViewbox.js:23-46`) |
| `arrowMarkerAbsolute`    | `:2189`                                                   | none                                          | `undefined`                | `getUrl(true)` uses `window.location` (`common.ts:144-158`)                |
| `textPlacement`          | not in schema                                             | `undefined`                                   | `byTspan`                  | `svgDraw.js:1955-1960` (`fo`, `old` select other modes)                    |
| top-level `fontSize`     | `:443`                                                    | 16                                            | 16                         | the source of the font override                                            |
| top-level `fontFamily`   | `:271`                                                    | `"trebuchet ms", verdana, arial, sans-serif;` | same                       | the source of the family override (note the trailing `;`)                  |
| top-level `look`         | `:68`                                                     | `classic`                                     | `classic` (before scoping) | overridden by the section default                                          |
| top-level `theme`        | `:64`                                                     | `default`                                     | `default` (before scoping) | overridden by the section default                                          |

**The font override, in code.** `draw` sets `conf = sequence` (`:1045`), then `bounds.init()` calls `setConf(getConfig())` (`:97`). `setConf` copies every top-level key into `conf` (`assignWithDepth`, `:877`), then, if the top-level `fontFamily` is truthy, sets `actorFontFamily = noteFontFamily = messageFontFamily = fontFamily`, and the same for `fontSize` and `fontWeight` (`:879-887`). The top-level keys always exist, so the section's own font values never apply. The render probe confirms it: actor and note text are `font-size: 16px` with the top-level family.

**[probe]** Resolution of `look` and `theme`. Every `sequenceDiagram` with no override: `look: neo`, `theme: redux-color`. `%%{init: {"look": "classic"}}%%` gives `look: classic` and keeps `theme: redux-color`. `%%{init: {"sequence": {"look": "classic"}}}%%` gives `look: classic`. Front matter `config: {look, theme}` overrides both. Rule: `readAppearance` takes the section's value before the top-level value, layer by layer (`config.ts:42-56`), with layers directives, then site config, then defaults (`:79-107`). `mermaidAPI.processAndSetConfigs` sets the scope before the directives are added (`mermaidAPI.ts:83-84`).

### 5.3 Text measurement

Every measurement the renderer makes, with the element and the rounding:

| Caller (line)                                                                                                                                | Call                                                                               | Element measured                                                                                                           | Rounding                                                                             |
| -------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `calculateActorMargins` `:1629-1631`                                                                                                         | `calculateTextDimensions(actor.description, actorFont)`                            | a `text` with one `tspan`, in a fresh `<svg>` in `body` (`drawSimpleText`, `utils.ts:491-530`), one per line, two families | width: `Math.round(max)`; height: `Math.round` per line, summed (`utils.ts:706-712`) |
| `wrapLabel` (used by actors `:1623`, messages `:2007`, boxes `:1684`, notes `:1711`, `:1761`)                                                | `calculateTextWidth` → `calculateTextDimensions`                                   | same                                                                                                                       | integer widths compared with `maxWidth` (`utils.ts:556-606`)                         |
| `getMaxMessageWidthPerActor` `:1514-1518`                                                                                                    | `calculateTextDimensions`                                                          | same                                                                                                                       | integers                                                                             |
| `buildMessageModel` `:2013`                                                                                                                  | `calculateTextDimensions(msg.message, messageFont)`                                | same                                                                                                                       | integers                                                                             |
| `buildNoteModel` `:1710-1713`, `:1742-1747`                                                                                                  | same                                                                               | same                                                                                                                       | integers                                                                             |
| `calculateActorMargins` box `:1687`                                                                                                          | same                                                                               | same                                                                                                                       | integers                                                                             |
| `drawNote` `:273-277`                                                                                                                        | `Math.round(sum of getBBox().height of the note's text elements)`                  | the `text` elements from `drawText`                                                                                        | integer                                                                              |
| `drawText` `:309-316`                                                                                                                        | `getBBox().height` per line (not rounded), only when `valign` and `textMargin > 0` | each line's `text` element                                                                                                 | none                                                                                 |
| `drawLoop` section titles `:1607-1612`                                                                                                       | `Math.round(getBBox().height)`                                                     | the section `text`                                                                                                         | integer                                                                              |
| `drawActorTypeParticipant` `:497-503` (classic only)                                                                                         | `rectElem.node().getBBox()`; neo keeps `actor.height`                              | rect                                                                                                                       | none                                                                                 |
| Database `:1113-1119`, Boundary `:1210-1213`, Control `:866-869`, Entity `:949-952`, Queue `:763-770`, Collections `:623-629` (classic only) | `getBBox()` of a glyph or path                                                     | glyph elements                                                                                                             | none                                                                                 |
| `drawKatex` `:159`, `:167`, `:1922`                                                                                                          | `getBoundingClientRect`, `calculateMathMLDimensions`                               | math                                                                                                                       | out of scope (no fixture uses `$$`)                                                  |

The rule that matters for the port: sequence sizes are **whole pixels** from `Math.round` of `getBBox`, with the width of a line being its advance. M5 used the 1/64 px ceiling of the advance (`labels.md` section 14). The sequence port needs `round(layout_width(measure(...)))` per line, and per-line `round(ascent) + round(descent)` for heights (`ferrigura-text/src/metrics.rs` `text_box`), which matches the rounding that `getBBox` gives in Chromium (`labels.md` section 6). Unverified for the sum of several lines (see 7).

**The font used for sizing.** `calculateTextDimensions` measures with two families: `'sans-serif'` and the configured family (`utils.ts:680`). The result is `dims[0]` (the sans-serif one) only when it is larger in all three of width, height and lineHeight; otherwise `dims[1]` (`utils.ts:720-729`). **[read]** So the reference uses the sans-serif metrics whenever they are larger. The port bundles only Lexica; the rule can be reproduced only if the harness pins which measurement wins (see 7).

### 5.4 Geometry model (`draw`, step by step)

```
draw(text, id, version, diagObj)                                     sequenceRenderer.ts:1043
  conf = sequence section
  bounds.init()                                                      :1057, setConf(getConfig()) :97
  actors, createdActors, destroyedActors, boxes, actorKeys, messages, title
  maxMessageWidthPerActor = getMaxMessageWidthPerActor(...)          :1073  (1488-1587)
  conf.height = calculateActorMargins(...)                           :1074  (1614-1699)
     per actor: width = wrap ? conf.width : max(conf.width, textW + 2*wrapPadding)
                neo:     actorTextHeight = textH; height = max(44 + 6 + textH + 6, conf.height)
                classic: height = wrap ? max(textH, conf.height) : conf.height
     per actor with a next actor: margin = max(msgW + actorMargin - w/2 - nextW/2, actorMargin)
     boxes: textMaxHeight, margins
     returns max(actor height, conf.height)
  insert computer, database, clock icons                             :1076-1078
  if boxes: bump(boxMargin); if box titles: bump(textMaxHeight)      :1080-1085
  hideUnusedParticipants filter                                      :1087-1094
  addActorRenderingData(actors, actorKeys, verticalPos 0)            :1100  (743-811)
     x positions: prevWidth + prevMargin; created actors widen by width/2
     actor.starty = vpos; insert bounds; bump(maxHeight)
  loopWidths = calculateLoopBounds(messages)                         :1101  (2034-2144)
  markers, drop shadow (neo)                                         :1104-1115
  for msg in messages (switch on type):                              :1146-1383
     NOTE         -> resetVerticalPos; drawNote                        :1150-1153  (243-286)
     ACTIVE_START / central -> newActivation                           :1155-1163
     ACTIVE_END   -> activeEnd: drawActivation, insert                 :1164-1166, :1121-1138
     LOOP/ALT/OPT/PAR/CRITICAL/BREAK start -> adjustLoopHeightForWrap   (916-934)
     ALT_ELSE / PAR_AND / CRITICAL_OPTION -> addSectionToLoop
     *_END -> endLoop, drawLoop, bump(stopy - vpos)                   (svgDraw 1519-1619)
     RECT_START / RECT_END -> background, bump
     AUTONUMBER -> sequenceIndex, enableSequenceNumbers/disable         :1269-1277
     default (message) -> boundMessage, adjustCreatedDestroyedData, queue for drawMessage  :1317-1341
     after a line type: sequenceIndex = round((i + step)*100)/100       :1380
  drawActors(header)                                                 :1387  (813-849)
  drawMessage for each queued message                                :1389-1391 (461-741)
  drawActors(footer) if mirrorActors                                  :1392-1394
  backgrounds (rect), fixLifeLineHeights                              :1395-1396
  boxes: box geometry, drawBox                                        :1398-1408
  popups: drawActorsPopup                                             :1415
  box = bounds.data; height = (stopy - starty) (or menus)  + 2*diagramMarginY
     mirror: height = height - boxMargin + bottomMarginAdj             :1433-1441
  width = (stopx - startx) + 2*diagramMarginX                          :1444-1448
  title text at x = (stopx - startx)/2 - 2*diagramMarginX, y = -25     :1450-1456
  configureSvgSize(height, width, useMaxWidth)                         :1458
  viewBox = (startx - diagramMarginX, -(diagramMarginY + titleExtra),
             width, height + titleExtra + (neo && actors ? 30 : 0))   :1460-1472
```

`boundMessage` (`:407-451`):

```
bump(10)
lines = splitBreaks(message).length
dims = calculateTextDimensions(message, messageFont)           # integers
if not katex: lineHeight = dims.height / lines; msg.height += lineHeight; bump(lineHeight)
totalOffset = dims.height - 10
if startx == stopx (self message):
    lineStartY = vpos + totalOffset
    if not rightAngles: totalOffset += boxMargin; lineStartY = vpos + totalOffset
    totalOffset += 30
    dx = max(textW/2, width/2)
    insert(startx-dx, vpos-10+totalOffset, stopx+dx, vpos+30+totalOffset)
else:
    totalOffset += boxMargin; lineStartY = vpos + totalOffset
    insert(startx, lineStartY-10, stopx, lineStartY)
bump(totalOffset); msg.height += totalOffset; msg.stopy = starty + height
insert(fromBounds, starty, toBounds, stopy)
return lineStartY
```

`drawText` positions (`svgDraw.js:192-322`), used for messages, notes and loop labels:

```
if valign && textMargin > 0:
   yfunc = round(y + (prevH + H + textMargin) / 2)     # prevH == H == sum of the earlier lines' getBBox heights
   # so line i gets y_i = round(y + H_{<i} + textMargin/2)   [derived from :210-215, :314-315]
if anchor center: x = round(x + width/2), anchor middle, dominant-baseline middle
each line: <text x y_i dy="1em" class> with text (tspan only when textData.tspan)
```

Messages use `tspan = false`, notes use `tspan = true` (the probe shows `<tspan x="175">note</tspan>` inside the note text). Both use `dy = 1em`.

### 5.5 Actors

| `actor.type` (DB)       | `svgDraw.js` function (line)          | Classic                                                                                                                  | Neo (band model)                                                                             |
| ----------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------- |
| `participant` (default) | `drawActorTypeParticipant` `:391-506` | rect `rx=3`, height from getBBox                                                                                         | rect `rx=6`; the box spans the full row; label centred in the box (`headerBands` `:100-108`) |
| `actor` (stick figure)  | `drawActorTypeActor` `:1240-1359`     | fixed glyph (`ACTOR_GLYPH_TOP=-5`, `BOTTOM=60`, `:20-22`), circle `r=15`, height fixed at 65 (`:1324-1327`)              | glyph scaled to the band: `glyphScale = 44 / 65` (`:1284`), feet on the band's bottom        |
| `boundary`              | `drawActorTypeBoundary` `:1130-1238`  | circle `r=22`, torso and arm lines                                                                                       | same circle on the band, label at `labelCenterY`                                             |
| `control`               | `drawActorTypeControl` `:781-889`     | circle `r=22` at `actorY+32`, loop arrow via `filled-head-control` marker (`:831-842`), height from getBBox (`:866-869`) | circle at `glyphBottomY - 22`, arrow marker unchanged                                        |
| `entity`                | `drawActorTypeEntity` `:891-997`      | circle `r=22` at `actorY+25`, underline; legacy transform `translate(0, r/2-5)` (`:988`)                                 | circle on the band, no legacy transform                                                      |
| `database`              | `drawActorTypeDatabase` `:999-1128`   | cylinder, `w = width/3`, `h = width/3` (`:1066-1069`)                                                                    | `h = 44` (`GLYPH_BAND_HEIGHT`), bottom on the band (`:1072`)                                 |
| `collections`           | `drawActorTypeCollections` `:516-639` | main rect and a shadow copy offset by 6                                                                                  | main rect `height - 6` so the shadow does not cross the datum (`:577-580`)                   |
| `queue`                 | `drawActorTypeQueue` `:641-779`       | cylinder path with `ry = h/2`, `rx = ry/(2.5+h/50)` (`:702-703`)                                                         | same                                                                                         |
| anything else           | no case in `drawActor` (`:1376-1442`) | **nothing drawn** (quirk 2.7-5)                                                                                          | same                                                                                         |

Common to all actors (`svgDraw.js`): a lifeline `<line id="actorN" class="actor-line 200" stroke-width="0.5px" stroke="#999" data-et="life-line">` from the glyph centre to `y2=2000`, later clipped by `fixLifeLineHeights` (`:368-381`). Each actor has `g#root-N` with `data-et="participant"`, `data-type` and `data-id`. The actor counter `actorCnt` is module-level and never reset (`:366`, **[probe]**: a second render in the same process continues at `actor3`).

Neo band model (`actorBands.ts`): `GLYPH_BAND_HEIGHT = 44` (`:41`), `GLYPH_LABEL_GAP = 6` (`:44`), `LABEL_LIFELINE_GAP = 6` (`:47`). Header: `lifelineStart = actorY + rowHeight`, `labelCenter = lifelineStart - 6 - textH/2`, `glyphBottom = lifelineStart - 6 - textH - 6` (`:100-108`). Footer: `glyphTop = actorY + 6`, `glyphBottom = glyphTop + 44`, `labelCenter = glyphBottom + 6 + textH/2` (`:111-120`).

### 5.6 Messages, arrows, markers

`drawMessage` (`sequenceRenderer.ts:461-741`):

- Text: `x = min(startx, stopx)`, `y = starty + 10`, `width = |stopx - startx|`, `class messageText`, `dy 1em`, `anchor messageAlign`, `textMargin = wrapPadding` (`:469-484`).
- Non-self line: `<line x1 y1=lineStartY x2 y2=lineStartY class=messageLine0|1 data-et="message" data-id="iN" data-from data-to stroke-width=2 stroke=none style="fill:none">` (`:538-545`, `:570-582`).
- Self line, default: `<path d="M lineStartX,lineStartY C lineStartX+60,lineStartY-10 startx+60,lineStartY+30 startx,lineStartY+20">`; rightAngles: `M … H startx+max(width/2, textW/2) V lineStartY+25 H startx` (`:501-531`). `lineStartX = startx + 10` when autonumber is shown and the arrow is reverse or bidirectional (`:499`).
- Dashed (`messageLine1`, `stroke-dasharray: 3, 3` inline) for DOTTED types (`:548-568`, list below).
- Autonumber: a `line` with `marker-start=…-sequencenumber` and a `text` at `x = autonumberX`, `y = lineStartY + 4`, `font-family sans-serif`, size 12px, 9px for more than 3 digits, 7px for more than 5 (`:700-739`, `:713-720`).

Marker table (`sequenceRenderer.ts:584-644`):

| LINETYPE (value)                                    | Dashed   | marker-end             | marker-start           |
| --------------------------------------------------- | -------- | ---------------------- | ---------------------- |
| SOLID (0)                                           | no       | `arrowhead`            | —                      |
| DOTTED (1)                                          | yes      | `arrowhead`            | —                      |
| SOLID_OPEN (5), DOTTED_OPEN (6)                     | no / yes | —                      | —                      |
| SOLID_CROSS (3), DOTTED_CROSS (4)                   | no / yes | `crosshead`            | —                      |
| SOLID_POINT (24), DOTTED_POINT (25)                 | no / yes | `filled-head`          | —                      |
| SOLID_TOP (41), SOLID_TOP_DOTTED (51)               | no / yes | `solidTopArrowHead`    | —                      |
| SOLID_BOTTOM (42), SOLID_BOTTOM_DOTTED (52)         | no / yes | `solidBottomArrowHead` | —                      |
| STICK_TOP (43), STICK_TOP_DOTTED (53)               | no / yes | `stickTopArrowHead`    | —                      |
| STICK_BOTTOM (44), STICK_BOTTOM_DOTTED (54)         | no / yes | `stickBottomArrowHead` | —                      |
| SOLID_ARROW_TOP_REVERSE (45), …\_DOTTED (55)        | no / yes | —                      | `solidBottomArrowHead` |
| SOLID_ARROW_BOTTOM_REVERSE (46), …\_DOTTED (56)     | no / yes | —                      | `solidTopArrowHead`    |
| STICK_ARROW_TOP_REVERSE (47), …\_DOTTED (57)        | no / yes | —                      | `stickBottomArrowHead` |
| STICK_ARROW_BOTTOM_REVERSE (48), …\_DOTTED (58)     | no / yes | —                      | `stickTopArrowHead`    |
| BIDIRECTIONAL_SOLID (33), BIDIRECTIONAL_DOTTED (34) | no / yes | `arrowhead`            | `arrowhead`            |

Parse table (from the probe, `LINETYPE` values): `->` 5, `-->` 6, `->>` 0, `-->>` 1, `-x` 3, `--x` 4, `-)` 24, `--)` 25, `-|\` 41, `--|\` 51, `-|/` 42, `--|/` 52, `-\\` 43, `--\\` 53, `-//` 44, `--//` 54, `/|-` 45, `\|-` 46, `//-` 47, `\\-` 48, `/|--` 55, `\|--` 56, `//--` 57, `\\--` 58, `<<->>` 33, `<<-->>` 34.

`hasCentralConnection` circles (`:288-374`): radius 5 at `fromCenter` or `toCenter`, offset `CENTRAL_CONNECTION_CIRCLE_OFFSET = 16.5` (`:310`) with autonumber shifts. Activation rects `activation0|1|2` (`svgDraw.js:1493`, `actorActivations % 3`), width `activationWidth`, x shifted by `(stacked - 1) * activationWidth / 2` (`sequenceRenderer.ts:149-161`).

Markers (`svgDraw.js`): `arrowhead` `refX 7.9 refY 5` `path M -1 0 L 10 5 L 0 10 z` (`:1681-1694`); `filled-head` `refX 15.5 refY 7` `M 18,7 L9,13 L14,7 L9,1 Z` (`:1701-1713`); `sequencenumber` circle `r 6` (`:1720-1735`); `crosshead` `M 1,2 L 6,7 M 6,2 L 1,7` (`:1742-1761`); `solidTopArrowHead` `refY 7.25` `M 0 0 L 10 8 L 0 8 z` (`:2076-2089`); `solidBottomArrowHead` `refY 0.75` `M 0 0 L 10 0 L 0 8 z` (`:2091-2104`); `stickTopArrowHead` (`:2106-2122`), `stickBottomArrowHead` (`:2124-2140`). All use `orient="auto-start-reverse"` except `filled-head`, `sequencenumber` and `crosshead` (`orient="auto"`). `markerUnits="userSpaceOnUse"` on the arrow family.

### 5.7 Notes, loops, rect, boxes

- **Note** (`drawNote`, `:243-286`): `bump(boxMargin)`; `rect` at `startx, starty` with width `noteModel.width` and `height = round(text sum) + 2*noteMargin`; class `note`, `data-et="note"`, `data-id="iN"`; text centred (`valign center`, `textMargin = noteMargin`, `anchor = noteAlign`). Note model widths: `buildNoteModel` (`:1701-1771`): RIGHTOF, LEFTOF, self and spanning cases.
- **Loop family** (`drawLoop`, `svgDraw.js:1519-1619`): four `line.loopLine` edges, dashed section lines `3, 3`, a `polygon.labelBox` (cut 7, `drawLabel` `:324-364`), the label `text.labelText` (`txt.height = labelBoxHeight + (neo ? 15 : 0)`, `:1566`), the title `text.loopText` as `[title]` (wrapped by `adjustLoopHeightForWrap`, `:916-934`), section titles `text.sectionTitle`.
- **rect** (`RECT_START`): fill from the message, else `themeVariables.rectBkgColor || actorBkg || rgba(128,128,128,0.5)` (`:1190-1193`). Drawn after the content as a background (`drawBackgroundRect`, `svgDrawCommon.ts:54-66`, class `rect`, `lower()`).
- **Box** (`drawBox`, `svgDraw.js:1445-1462`): `drawBackgroundRect` with `box.stroke = 'rgb(0,0,0, 0.5)'` (`sequenceRenderer.ts:1406`); title `text` at `box.y + boxTextMargin + textMaxHeight/2`.

### 5.8 SVG structure

From a render of a small diagram with `look` neo (probe `render-1`, fake `getBBox`, so only structure and attributes are meaningful):

```
<svg id="<id>" width="100%" style="max-width: 650px;" viewBox="-50 -10 650 484" role="graphics-document document" aria-labelledby="chart-title-<id>">
  <title id="chart-title-<id>">T</title>                       (accTitle, when set)
  <g>  per actor, in reverse order:  <g> <line id="actorN" class="actor-line 200" …/> <g id="root-N" data-look="neo" data-et="participant" data-type="…" data-id="…">
          <rect class="actor actor-top" rx="6" data-look="neo" style="stroke:…; fill:…" filter="url(#<id>-drop-shadow)" name=…/>
          <text class="actor actor-box" style="text-anchor: middle; font-size: 16px; font-weight: 400; font-family: …">
  <style>…3796 chars…</style>
  <defs> symbols <id>-computer, <id>-database, <id>-clock
  <defs> markers arrowhead, crosshead, filled-head, sequencenumber, solidTopArrowHead, solidBottomArrowHead, stickTopArrowHead, stickBottomArrowHead
  <defs> <filter id="<id>-drop-shadow"> <feDropShadow dx=4 dy=4 stdDeviation=0 flood-opacity=0.06 …>
  <g data-et="note" data-id="iN"> <rect class="note" data-look="neo"/> <text class="noteText" dy="1em"><tspan>…</tspan></text>
  <rect class="activation0" …/>
  <g data-et="control-structure" data-id="iN"> <line class="loopLine"/>×4(+sections) <polygon class="labelBox"/> <text class="labelText"/> <text class="loopText"/>
  <text class="messageText" dy="1em">…   <line class="messageLine0|1" data-et="message" data-id="iN" data-from data-to marker-end="url(#<id>-arrowhead)" …/>
  footer groups (actor-bottom), boxes (rect.rect)
```

Attributes: messages carry `data-id="i<N>"`; every actor element has `data-et` and `data-type`. Node ids and classes are as in the reference; the reference's per-actor colours are **inline** `style="stroke:…; fill:…"` on rects and groups (`svgDraw.js:458-461`, `:595-601`), not CSS variables.

### 5.9 Styles (`styles.js`)

`getStyles(options)` (`styles.js:3-154`) emits the rules below. Theme variables are read from `options`; the neo-only rules are guarded by `data-look`.

| Selector                                                                        | Declarations (variables)                                                                                     | Lines      |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ---------- |
| `.actor`                                                                        | stroke `actorBorder`, fill `actorBkg`, stroke-width `strokeWidth` (default 1)                                | `:7-11`    |
| `rect.actor.outer-path[data-look="neo"]`, `rect.note[data-look="neo"]`          | filter `dropShadow`                                                                                          | `:13-21`   |
| `text.actor > tspan`                                                            | fill `actorTextColor`                                                                                        | `:23-26`   |
| `.actor-line`                                                                   | stroke `actorLineColor`                                                                                      | `:28-30`   |
| `.messageLine0`, `.messageLine1`                                                | stroke-width 1.5, dasharray (`none`, `2, 2`), stroke `signalColor`                                           | `:37-47`   |
| `[id$="-arrowhead"] path`, `[id$="-crosshead"] path`, `[id$="-sequencenumber"]` | fill and stroke `signalColor`                                                                                | `:49-65`   |
| `.sequenceNumber`                                                               | fill `sequenceNumberColor`                                                                                   | `:54-56`   |
| `.messageText`                                                                  | fill `signalTextColor`                                                                                       | `:67-70`   |
| `.labelBox`                                                                     | stroke `labelBoxBorderColor`, fill `labelBoxBkgColor`, filter dropShadow only on neo                         | `:72-76`   |
| `.labelText`, `.loopText`, `.sectionTitle`                                      | fill `labelTextColor` / `loopTextColor`                                                                      | `:78-91`   |
| `.loopLine`                                                                     | stroke-width 2px, dasharray `2, 2`, stroke and fill `labelBoxBorderColor`                                    | `:93-98`   |
| `.note`, `.noteText`                                                            | stroke `noteBorderColor`, fill `noteBkgColor`; text `noteTextColor`. No font-weight (deliberate, `:106-113`) | `:100-117` |
| `.activation0..2`                                                               | fill `activationBkgColor`, stroke `activationBorderColor`                                                    | `:119-132` |
| `.actorPopupMenu`, `.actorPopupMenuPanel`                                       | position absolute, fill `actorBkg`, shadow                                                                   | `:134-143` |
| `.actor-man circle, line`                                                       | fill `actorBkg`, stroke-width 2px                                                                            | `:144-147` |
| `g rect.rect`                                                                   | filter dropShadow, stroke `nodeBorder`                                                                       | `:149-152` |

The probe's stylesheet also contains the shared flowchart-era rules that the common stylesheet emits (`mermaidAPI` `createCssStyles`), for example `.edge-animation-slow`. The sequence part is the rules above.

### 5.10 d3 helpers

Only `select`, `append`, `attr`, `style`, `text`, `lower`, `node`, `select(selector)`, `getBBox`, `remove` and `_groups` are used (`sequenceRenderer.ts:2`, `utils.ts` for `select('body')`). No scales, shapes, curves or time formats. Paths are plain strings.

---

## 6. Fixtures

### 6.1 Counts

| Folder                                                | `.mmd`    | Notes                                                                                           |
| ----------------------------------------------------- | --------- | ----------------------------------------------------------------------------------------------- |
| `e2e/diagrams/sequence/nightly-check/`                | 177       |                                                                                                 |
| `e2e/diagrams/sequence/pr-check/`                     | 10        |                                                                                                 |
| `e2e/diagrams/sequence/visual-check/`                 | 6         |                                                                                                 |
| Total                                                 | 193       | all start with front matter `---`; the snapshot harness collects them (`mmd-snapshots.spec.ts`) |
| `packages/mermaid/src/docs/syntax/sequenceDiagram.md` | 38 blocks | not yet counted as inputs                                                                       |
| `sequenceDiagram.spec.js` string literals             | ≈150      | the spec inputs, for the differential run                                                       |

### 6.2 Configuration (per file)

| Source                               | Files | Values                                                                                                                                                                                                                                                                                                                                               |
| ------------------------------------ | ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| front matter `config:` block         | 116   | `look: classic` (27 files), `look: neo` (22), `theme: redux-color` (7), `redux-dark-color` (5), `redux` (5), `redux-dark` (5); keys in these blocks, counted as lines: `sequence` 84, `diagramMarginX` and `Y` 63 each, `look` 49, `theme` 22, `useMaxWidth` 6, `noteAlign` 5, `messageAlign` 2, `fontSize` 2, `showSequenceNumbers` 1, `themeCSS` 1 |
| `%%{init}%%` directive               | 56    | `'theme':'base'` (44 files), `dark` (4), `default` (1), `forest` (1), `neutral` (1); `wrap` (4 files); `fontFamily` `Menlo` and `fontSize` 18 or 24 (2 files); `noteAlign`: none                                                                                                                                                                     |
| `theme: redux-color`                 | 7     | shipped                                                                                                                                                                                                                                                                                                                                              |
| `theme: redux-dark-color`            | 5     | shipped                                                                                                                                                                                                                                                                                                                                              |
| `theme: redux` / `theme: redux-dark` | 5 / 5 | **not shipped** by ferrigura; M2 falls back by name (`redux` to `redux-color`, `redux-dark` to `redux-dark-color`)                                                                                                                                                                                                                                   |
| `theme: base`                        | 44    | not shipped; falls back to `redux-color`                                                                                                                                                                                                                                                                                                             |
| `themeCSS`                           | 1     | a stylesheet override; not in the M2 scope (see 7)                                                                                                                                                                                                                                                                                                   |

### 6.3 Syntax (files that use each form)

| Form                       | Files | Form                              | Files        |
| -------------------------- | ----- | --------------------------------- | ------------ |
| `->>` (solid arrow)        | 158   | `@{ … }` shape data               | 63           |
| `participant X@{…}` config | 63    | `as` alias                        | 18           |
| `autonumber`               | 44    | `Note`                            | 51           |
| central connections `()`   | 44    | `loop`                            | 22           |
| `alt`                      | 20    | `par`                             | 26           |
| `opt`                      | 4     | `critical`                        | 1            |
| `break`                    | 1     | `rect`                            | 11           |
| `box`                      | 9     | `create` / `destroy`              | 8 / 8        |
| `wrap:` prefix             | 15    | `<br>`                            | 24           |
| `accTitle` / `accDescr`    | 5     | `#name;` entity                   | 1            |
| `hideUnusedParticipants`   | 1     | `useMaxWidth`                     | 6            |
| `showSequenceNumbers`      | 2     | `-x`, `--x` / `-)` / `-\\` arrows | 39 / 15 / 20 |

The link and properties statements (`links`, `link`, `properties`, `details`) appear in no fixture; they are only in the Playwright spec (4.2). No fixture uses `$$` math, an image, an icon, `click` or `classDef`.

### 6.4 Browser features needed

| Feature                                   | Fixtures                                         | Needed for                                       |
| ----------------------------------------- | ------------------------------------------------ | ------------------------------------------------ |
| HTML labels (`foreignObject`)             | 0 by default; `textPlacement` `fo` only in specs | not needed (`textPlacement` defaults to `tspan`) |
| Images                                    | 0                                                | not needed                                       |
| Math                                      | 0                                                | not needed                                       |
| JavaScript (popup menus)                  | 0 in fixtures, 6 Playwright tests                | decision 0001                                    |
| `window.location` (`arrowMarkerAbsolute`) | 0                                                | not needed                                       |
| DOM for box colours and `details`         | box colours in 9 files; `details` in 0           | see 7                                            |

---

## 7. Risks, effort, open questions

### 7.1 Risks (ordered)

1. **Measurement rules.** Sizes are integer pixels from `calculateTextDimensions` with the sans-serif-or-configured choice (5.3). The port has only Lexica, so the choice must be pinned in the harness. Wrapping compares these integer widths, so one pixel moves a line break.
2. **Font override** (5.2). The reference renders actor and note text at 16 px. A port that follows the schema (14 px) fails every fixture with an actor name. Decide to reproduce it (recommended: yes, the reference is the spec).
3. **Two looks and eight actor types.** `classic` (27 fixtures) and `neo` (default) differ in most shapes (5.5). Each needs its own geometry. The neo band model is recent (`actorBands.ts`, the fork's most recent sequence work).
4. **Quirks in the DB** (2.7). The prev and next pointers, the lost links on re-declaration and the `undefined` names in errors must be copied or deliberately changed and listed as named exceptions, as the flowchart harness does.
5. **Entities and `decodeEntities`** (2.1). Text in the model is the placeholder form, and the SVG writer must decode it at the end, as `mermaidAPI.ts:359` does. Otherwise `#quot;` shows as `ﬂ°quot¶ß`.
6. **Browser-dependent parse** (`parseBoxData`, `addDetails`). Named colours and the `details` lookup depend on the browser; the port cannot reproduce `details` from text.
7. **Global state.** `actorCnt` (`svgDraw.js:366`) makes ids depend on earlier renders in the same page. The harness must render each fixture in a fresh page, or the port must document a deviation.
8. **Spec coverage of geometry is thin.** The `draw` spec runs with zero text sizes, so text errors pass it. The e2e snapshots and the geometry harness are the real check.

### 7.2 Effort, relative to the flowchart

| Part             | Flowchart reference                                                                                                | Sequence estimate                                                                                                                                                                                                            |
| ---------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lexer and parser | `flow.jison` plus M3's scanner (600-800 Rust lines) and parser (1,100-1,400 lines, `flowchart/PLAN.md` section 12) | Jison is 423 lines, with 8 states and the ACTOR regex. Rust 900-1,300 lines, about 0.5x                                                                                                                                      |
| DB and model     | 500-700 lines (DB) and 200-250 (model)                                                                             | 350-500 and 150-200, about 0.6x                                                                                                                                                                                              |
| YAML for `@{}`   | shared with M3 (`ferrigura-syntax/src/yaml.rs`)                                                                    | reuse; no new work                                                                                                                                                                                                           |
| Theme and config | redux-color and config sections already generated                                                                  | reuse; add the `sequence` section to `pnpm port:diff config`                                                                                                                                                                 |
| Renderer         | M5 renderer with ELK, getData, shapes, edges, markers, line hops, clusters                                         | No ELK, getData, edges or clusters. About 2,000-2,800 Rust lines: 9 actor shapes in two looks, messages and markers, notes, loops, activations, boxes, central connections, layout, and the text rules. About 0.4-0.6x of M5 |
| Specs            | 953 ported (flowchart)                                                                                             | ≈ 205 cases; about 40% need jsdom or mocks, so port the layout expectations as numbers                                                                                                                                       |
| Harness          | flowchart geometry in Chromium, 298 fixtures compared                                                              | sequence geometry in Chromium, 193 fixtures; integer sizes make tolerances tighter but the harness is simpler                                                                                                                |

Overall, roughly half of M5, dominated by the renderer and the measurement rules.

### 7.3 Open questions for the coordinator

1. **Keep the DB pointers and quirks?** Recommend yes (2.7 item 1, 3.1). Without them the differential run fails on any re-declared actor.
2. **Config-dependent parse values.** `wrap` defaults, `htmlLabels` and `securityLevel` change the parsed text (3.3, 3.5). Recommend parser options, like flowchart's `maxEdges`, and the harness pins them.
3. **Box colour validation.** Chromium's `CSS.supports` decides whether a token is a colour. Options: a table of CSS named colours and `rgb`/`hsl` syntax, or accept any identifier. Needs a Chromium check (**unverified**).
4. **Sans-serif measurement choice** (5.3). Needs a Chromium probe of `calculateTextDimensions` with Lexica and `sans-serif`, to see which family wins for typical labels.
5. **Popup menus** (`links`, `properties`; decision 0001). Options: render the menus hidden with their links and no script, or drop them and mark the six Playwright tests as named exceptions.
6. **Unknown actor type.** Reproduce the invisible actor (2.7-5), or report a warning and draw nothing (same visible result, but explicit).
7. **`details`.** Recommend a no-op with a warning: the reference reads an element from the page.
8. **`themeCSS`** (1 fixture). In scope or a named exception?
9. **Font override.** Reproduce the 16 px actor and note text (5.2)? Recommend yes.
10. **The `actorCnt` counter.** Per render, or reproduce the page-wide counter? Recommend per render, as a named difference (ids only).
11. **Model names.** Message types as names (1 table) rather than `LINETYPE` numbers. Confirm before the exporter is written.
12. **Harness.** Pin `htmlLabels: false` (as M5 did) and check that the sanitised texts still match, and add the sequence section to the config differential.

### 7.4 Not verified

- Box colour split in Chromium (the probe used jsdom's colour check, which rejected named colours).
- Sum of several text heights in Chromium (`getBBox` of each line in `drawText`).
- The integer rounding of `calculateTextDimensions` against Chromium's advance (the rounding rule itself is read, not measured).
- The `\s+` skip at `sequenceDiagram.jison:25` across newlines (read, not probed).
- Every SVG coordinate: the probe's `getBBox` is fake, so only structure and the font attributes come from it.
- That the `classic` look draws the same shapes in a browser as the code says (read only).

---

## Appendix A: probe files (scratchpad, not in the repo)

| File                                                     | What it does                                                                                                                                                            |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `m6-sequence/probe-parse.cjs`                            | Jison grammar of `sequenceDiagram.jison` with a recording yy stub; prints parse-time calls and the statement list given to `apply`                                      |
| `m6-sequence/cases-1.json`, `cases-2.json`               | Syntax cases for the stub                                                                                                                                               |
| `m6-sequence/probe-diagram.mts`                          | The reference pipeline: `preprocessDiagram` → `Diagram.fromText` → DB dump (run with `node_modules/.bin/tsx` from the main checkout, through `tools/port/reference.ts`) |
| `m6-sequence/cases-db*.json`                             | DB cases                                                                                                                                                                |
| `m6-sequence/probe-look.mts`                             | Resolution of `look` and `theme` through `configApi` (mirrors `mermaidAPI.processAndSetConfigs`)                                                                        |
| `m6-sequence/probe-config.mts`                           | Effective config values                                                                                                                                                 |
| `m6-sequence/probe-render.mts`, `probe-render-twice.mts` | `mermaidAPI.render` in jsdom with a fake `getBBox`                                                                                                                      |
| `m6-sequence/dump-svg.cjs`                               | Element tree of an SVG                                                                                                                                                  |
| `m6-sequence/render-*.mmd`                               | Small diagrams for the render probes                                                                                                                                    |

The probe with `probe-diagram.mts` is a working prototype of the sequence exporter for `tools/port/export-sequence.ts`: it uses the same loader and the same steps as `export-flowchart.ts`, and needs the `sequence` detector registered with `registerLazyLoadedDiagrams` (`probe-diagram.mts:11-12`).
