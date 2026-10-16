# User journey (M6): research notes for the port

Status: research only, nothing in the repository changed. Reference: the JS reference at the checkout's HEAD `04b952b4e` (the M6 worktree is at the same commit). Diagram type `journey`, a P1 type in [../README.md](../README.md).

## 0. Summary

| Item              | Value                                                                                                                 |
| ----------------- | --------------------------------------------------------------------------------------------------------------------- |
| Reference sources | 7 files, 1,151 lines without specs (`packages/mermaid/src/diagrams/user-journey/`)                                    |
| Specs             | 2 files, 16 test cases (STATUS says about 8; corrected here)                                                          |
| Fixtures          | 18 `.mmd` files in 2 folders (7 nightly-check, 11 pr-check); `visual-check/` is empty                                 |
| Parser            | Jison, first-match lexer with start states; hand-written port recommended                                             |
| Renderer          | SVG built by d3 calls, text by `foreignObject` in the default `textPlacement: fo`; no DOM spec                        |
| Biggest risk      | The default `fo` label is HTML wrapped in a 150 px cell; the port must reproduce Chromium's line layout (section 7.6) |
| Effort            | About a tenth of the flowchart's Rust (M3 and M5 total 23,600 lines), see section 11                                  |

Verification legend used below:

- **[V]** verified by running the reference in Chromium (`packages/mermaid/dist/mermaid.esm.min.mjs`, built from this checkout on 2026-10-16), by d3-shape 3.2.0, or by the jison-lex code of the reference's own generator;
- **[S]** read from the source only;
- **[U]** not verified; stated as an open point.

Probe material, not committed: `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/m6-journey/` (`probe-ref.mjs` renders cases in Chromium and writes `ref-<id>.svg`, `measure-<id>.json`; `probe-width.mjs` measures strings; `extract.py` summarises an SVG; `arc-probe.cjs` runs d3-shape and the `Number()` table; `batch*.json` are the inputs). About 60 inputs were run.

## 1. Sources

| File                                         | Lines                                                                               | Role                                                                              |
| -------------------------------------------- | ----------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `diagrams/user-journey/parser/journey.jison` | 1-69                                                                                | Lexer (6-36), grammar (44-67)                                                     |
| `diagrams/user-journey/journeyDb.js`         | 1-130                                                                               | DB: sections, tasks, actors, header setters through `commonDb`                    |
| `diagrams/user-journey/journeyDetector.ts`   | 7-10                                                                                | Detector `/^\s*journey/`                                                          |
| `diagrams/user-journey/journeyDiagram.ts`    | 8-17                                                                                | Definition; `init` calls `setConf(cnf.journey)` and `db.clear()` (13-16)          |
| `diagrams/user-journey/journeyRenderer.ts`   | 1-356                                                                               | Layout, actor legend, bounds, viewBox                                             |
| `diagrams/user-journey/svgDraw.js`           | 1-416                                                                               | Faces, sections, tasks, text placements, marker                                   |
| `diagrams/user-journey/styles.js`            | 1-139                                                                               | CSS of the diagram (interpolated theme values)                                    |
| `diagrams/common/svgDrawCommon.ts`           | 17-46, 68-86, 109-122                                                               | `drawRect`, `drawText`, `getNoteRect`                                             |
| `diagrams/common/commonDb.ts`                | 8, 16-32                                                                            | Header setters (all call `sanitizeText`)                                          |
| `diagrams/common/common.ts`                  | 8, 48-83                                                                            | `lineBreakRegex`, `removeScript`, `sanitizeMore`, `sanitizeText`                  |
| `diagram-api/comments.ts`                    | 6-8                                                                                 | `cleanupComments`                                                                 |
| `preprocess.ts`                              | 6-17, 58-65                                                                         | `cleanupText`, `preprocessDiagram`                                                |
| `diagram-api/frontmatter.ts`                 | 58-60                                                                               | Front matter `title`                                                              |
| `Diagram.ts`                                 | 19-85                                                                               | `fromText`: init, clear, front matter title, `encodeEntities`, parse              |
| `utils.ts`                                   | 874-897, 904-906                                                                    | `encodeEntities`, `decodeEntities`                                                |
| `mermaidAPI.ts`                              | 69-70, 344-365, 397-420, 487, 571-582, 600-608, 623, 638, 645-650, 659-661, 750-757 | Render pipeline, SVG root, style insertion, cleanup, DOMPurify on output, a11y    |
| `accessibility.ts`                           | 45-65                                                                               | `<title>` and `<desc>` of the SVG                                                 |
| `styles.ts`                                  | 11-45                                                                               | Common CSS header (`& { font-family; font-size; fill }`)                          |
| `setupGraphViewbox.js`                       | 23-33, 43-46                                                                        | `configureSvgSize`                                                                |
| `assignWithDepth.ts`                         | 37-47                                                                               | Array merge (append)                                                              |
| `config.ts`                                  | 223                                                                                 | `getConfig`                                                                       |
| `schemas/config.schema.yaml`                 | 380-384, 457-470, 1757-1896                                                         | Journey section, defaults, `useMaxWidth` (470)                                    |
| `themes/*.js`                                | see section 8.3                                                                     | Theme variables used by the CSS                                                   |
| `e2e/diagrams/journey/`                      | 18 fixtures                                                                         | Screenshot tests through the generic loader `e2e/rendering/mmd-snapshots.spec.ts` |

## 2. Pipeline, from text to SVG

```
mermaid.render(id, text)                                    mermaidAPI.ts:~470-650
  processAndSetConfigs(text)                                 preprocessDiagram (preprocess.ts:58-91)
    cleanupText: CRLF/CR -> LF; <tag ="x"> -> <tag ='x'>      preprocess.ts:6-17, 60
    front matter: yaml JSON_SCHEMA; title, config             frontmatter.ts:24-69; preprocess.ts:61
    %%{...}%% directives removed; init config merged          preprocess.ts:62-63
    cleanupComments: /^\s*%%(?!{)[^\n]+\n?/gm then trimStart   comments.ts:6-8; preprocess.ts:65
  text = code.cleaned (487); maxTextSize check
  Diagram.fromText(code, {title: frontMatterTitle})          mermaidAPI.ts:571-572; Diagram.ts:19
    db.clear()                                              Diagram.ts:49
    init(config): renderer.setConf(cnf.journey); db.clear()  Diagram.ts:50; journeyDiagram.ts:13-16
    db.setDiagramTitle(frontMatterTitle) if any              Diagram.ts:52-54
    textToParse = encodeEntities(cleaned) + "\n"            Diagram.ts:77; utils.ts:874-897 [V]
    parser.parse(textToParse)                               Diagram.ts:83 (DB calls run as actions)
  on parse error: render the 'error' diagram, then rethrow  mermaidAPI.ts:573-582 (catch and error diagram), 659-661 (rethrow) [V]
  renderer.draw(text, id, version, diag)                     mermaidAPI.ts:606-608
    initGraphics (marker, taskCount = -1)                    svgDraw.js:392-405
    getTasks(); getDiagramTitle(); getActors()               journeyRenderer.ts:144-147
    actors colour and position                               journeyRenderer.ts:148-158
    legend, bounds, tasks, title, activity line, viewBox     journeyRenderer.ts:160-197
  style element inserted as first child of <svg>             mermaidAPI.ts:600 (CSS from styles.js via styles.ts)
  a11y title and desc                                       mermaidAPI.ts:623, 750-757; accessibility.ts:45-65
  cleanUpSvgCode: marker-end URLs, decodeEntities, <br>      mermaidAPI.ts:344-365 (decode at 359)
  DOMPurify over the whole SVG unless securityLevel loose    mermaidAPI.ts:645-650 (ADD_TAGS foreignobject, ADD_ATTR dominant-baseline, lines 69-70) [V]
```

Consequences observed [V]:

- The DOMPurify pass over the output removes unknown attributes (`position="fixed"` on `foreignObject` is gone) and turns `&#9829;` into the character `♥`.
- Entities are decoded only after the DB has stored the placeholders (section 4.5).

## 3. Parser

### 3.1 Preprocessing (applied before the lexer)

| Step                  | Rule                                                                               | Source               |
| --------------------- | ---------------------------------------------------------------------------------- | -------------------- |
| CRLF                  | `\r\n?` -> `\n`                                                                    | preprocess.ts:10     |
| HTML attribute quotes | `<(\w+)([^>]*)>`: `="x"` -> `='x'`                                                 | preprocess.ts:12-15  |
| Front matter          | `^---…---` (JSON_SCHEMA); only `title`, `config`, `displayMode` are read           | frontmatter.ts:24-69 |
| Directives            | `%%{ … }%%` removed; `init` and `wrap` detected                                    | preprocess.ts:33-45  |
| Comment lines         | `^\s*%%(?!{)[^\n]+\n?` removed, needs one character after `%%`; then `trimStart()` | comments.ts:6-8      |
| Entities              | `#\w+;` -> `ﬂ°°<digits>¶ß` (numeric) or `ﬂ°<name>¶ß`                               | utils.ts:884-895     |

[V] A line that is only `%%` is not removed by `cleanupComments`; the lexer skips it (case `b-percent-only`). `%%` in the middle of a line stays in the text (`T: 5: a %% b` gives the actor `a %% b`).

### 3.2 Lexer (Jison, first match in definition order)

Jison's default lexer takes the first rule that matches, not the longest (`node_modules/.pnpm/jison-lex@0.3.4/node_modules/jison-lex/regexp-lexer.js:382`, `else if (!this.options.flex) break`) [S, source]. Quoted keyword literals get an implicit `\b` unless the generator runs in flex mode (`node_modules/.pnpm/lex-parser@0.1.4/node_modules/lex-parser/lex-parser.js:165-168`) [S], which the observed `journeyfoo:` and `titlefoo:` results confirm [V].

| #   | Pattern (journey.jison)     | State               | Token                     | Notes                                                                             |
| --- | --------------------------- | ------------------- | ------------------------- | --------------------------------------------------------------------------------- |
| 1   | `\%%(?!\{)[^\n]*` (14)      | any                 | skipped                   | `%%` comment line                                                                 |
| 2   | `[^\}]\%\%[^\n]*` (15)      | any                 | skipped                   | consumes one character before `%%` [S]                                            |
| 3   | `[\n]+` (16)                | any                 | NEWLINE                   | runs of newlines                                                                  |
| 4   | `\s+` (17)                  | any                 | skipped                   | includes newlines when the run starts with a blank [S]                            |
| 5   | `\#[^\n]*` (18)             | any                 | skipped                   | `#` comment to end of line                                                        |
| 6   | `"journey"` (20)            | any                 | journey                   | `journey\b`, case-insensitive (`%options case-insensitive`, 7)                    |
| 7   | `"title"\s[^#\n;]+` (21)    | any                 | title                     | `\s` may be a newline [V]; value is the match without its first 6 characters (61) |
| 8   | `accTitle\s*":"\s*` (22)    | any                 | acc_title                 | `\s*` may cross lines [V]                                                         |
| 9   | `(?!\n\|;\|#)*[^\n]*` (23)  | acc_title           | acc_title_value           | the lookahead is a no-op; value is the rest of the line; `;` allowed [V]          |
| 10  | `accDescr\s*":"\s*` (24)    | any                 | acc_descr                 |                                                                                   |
| 11  | `(?!\n\|;\|#)*[^\n]*` (25)  | acc_descr           | acc_descr_value           |                                                                                   |
| 12  | `accDescr\s*"{"\s*` (26)    | any                 | (none)                    | enters acc_descr_multiline                                                        |
| 13  | `[\}]` (27)                 | acc_descr_multiline | (none)                    | pops                                                                              |
| 14  | `[^\}]*` (28)               | acc_descr_multiline | acc_descr_multiline_value | includes newlines                                                                 |
| 15  | `"section"\s[^#:\n;]+` (29) | any                 | section                   | `section\b`; value is the match without its first 8 characters (65)               |
| 16  | `[^#:\n;]+` (30)            | any                 | taskName                  | includes trailing blanks [V]                                                      |
| 17  | `":"[^#\n;]+` (31)          | any                 | taskData                  | colon plus at least one character                                                 |
| 18  | `":"` (32)                  | any                 | `:`                       | not used by the grammar: an error                                                 |
| 19  | `<<EOF>>` (33)              | any                 | EOF                       |                                                                                   |
| 20  | `.` (34)                    | any                 | INVALID                   | catches `;` and everything else: an error [V]                                     |

Grammar (journey.jison 44-67):

```
start     : journey document EOF                 (45)
document  : /* empty */ | document line          (48-51)
line      : statement | NEWLINE | EOF            (53-58)   (SPACE is never produced)
statement : title                                (61) DB: setDiagramTitle(value)
          | acc_title acc_title_value            (62) DB: setAccTitle(trim)
          | acc_descr acc_descr_value           (63) DB: setAccDescription(trim)
          | acc_descr_multiline_value           (64) DB: setAccDescription(trim)
          | section                             (65) DB: addSection(value)
          | taskName taskData                   (66) DB: addTask(taskName, taskData)
```

Observed lexer and grammar behaviour [V], all from the reference's own output:

| Input                                                   | Result                                                                  |
| ------------------------------------------------------- | ----------------------------------------------------------------------- |
| `journeyfoo: 5: me` after the header                    | a task named `journeyfoo`                                               |
| `titlefoo: 5: me`, `section: 5: me`, `accTitlex: 5: me` | tasks named `titlefoo`, `section`, `accTitlex`                          |
| `Journey planning: 5: me` (second `journey`)            | parse error, token `journey`                                            |
| `Task; x: 5: me`, `title A &amp; B`                     | parse error, token `INVALID` (`;` ends the name)                        |
| `Make # tea: 5: me`                                     | parse error: `#` starts a comment, so `taskName` is followed by NEWLINE |
| `Task: 5 # comment: me`                                 | task with no actors, score 5                                            |
| `accTitle:` followed by `  section S`                   | accTitle becomes `section S`; no section                                |
| `title` followed by `  section S`                       | title becomes `  section S` (leading blanks kept)                       |
| `section` followed by `  T: 5: a`                       | parse error, `taskData` after `section`                                 |
| `section ` (blank name, then newline)                   | parse error (`taskData` expected, NEWLINE found)                        |
| `Section A: 5: me`                                      | parse error (the section name stops at `:`, then `taskData`)            |
| `journey` alone                                         | valid; empty SVG (viewBox `0 -25 400 20`)                               |
| `title  ` (two blanks, nothing else)                    | title is one blank `" "`; the title element is drawn                    |

Note for the port: the `journey` token is lexed anywhere, so a task in the middle of a document whose name starts with `Journey` is an error, not a task.

### 3.3 Parser behaviour

- The grammar builds no tree: each statement runs its DB call during the reduction (section 4). The order is the document order.
- The task statement needs `taskName` and `taskData` as adjacent tokens (a NEWLINE or a comment between them is an error) [V].
- `line: EOF` and `start: journey document EOF` both accept the final EOF. The conflict is resolved by Jison; the probes always reached the correct accept [S, unverified details].
- No error text is asserted by the specs. The specs use `not.toThrow()` only (section 6).

## 4. DB: what each call does, in order

### 4.1 Method table (journeyDb.js)

| Method                                | Lines          | Behaviour                                                                                                                                                                                                                                   |
| ------------------------------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `clear`                               | 18-24          | Empties `sections`, `tasks`, `rawTasks`, `currentSection`; `commonClear()` resets accTitle, accDescr, title                                                                                                                                 |
| `addSection(txt)`                     | 26-29          | `currentSection = txt`; pushes `txt` unchanged (no trim, no sanitise, duplicates kept)                                                                                                                                                      |
| `addTask(descr, taskData)`            | 61-84          | see 4.2                                                                                                                                                                                                                                     |
| `getTasks()`                          | 35-47          | runs `compileTasks` once, then up to 100 more times while the list is not all processed (36-42); no effect, because `processed` is never set; then `tasks.push(...rawTasks)`; **not idempotent**: a second call appends the tasks again [S] |
| `getActors()` / `updateActors`        | 49-59, 111-113 | unique names from `tasks[*].people`, `new Set`, then `[...].sort()` (UTF-16 code unit order [S]; the order of the tested names is [V])                                                                                                      |
| `setDiagramTitle` / `getDiagramTitle` | commonDb 28-32 | `sanitizeText(txt)` only                                                                                                                                                                                                                    |
| `setAccTitle`                         | commonDb 16-18 | `sanitizeText(txt).replace(/^\s+/g, '')`                                                                                                                                                                                                    |
| `setAccDescription`                   | commonDb 22-24 | `sanitizeText(txt).replace(/\n\s+/g, '\n')`                                                                                                                                                                                                 |
| `addTaskOrg`                          | 86-95          | dead: not called by the grammar                                                                                                                                                                                                             |
| default export `getConfig`            | 116            | `getConfig().journey`                                                                                                                                                                                                                       |

### 4.2 addTask, exact steps (journeyDb.js:61-84)

```
pieces = taskData.substr(1).split(':')          // drops the leading ':' (62)
if pieces.length == 1:
    score = Number(pieces[0]); people = []      // 66-68
else:
    score = Number(pieces[0])                   // 70
    people = pieces[1].split(',')               // 71; pieces[2..] are ignored [V]
people = people.map(trim)                       // 73
rawTasks.push({ section: currentSection, type: currentSection,
                people, task: descr, score })   // 75-83
```

Consequences [V unless marked]:

| Input                                                 | score             | people                                                   |
| ----------------------------------------------------- | ----------------- | -------------------------------------------------------- |
| `Task: 5: me`                                         | 5                 | `["me"]`                                                 |
| `Task:5: a: b`                                        | 5                 | `["a"]` (the second colon and what follows are dropped)  |
| `Task: ` (colon and blank)                            | 0 (`Number(" ")`) | `[]`                                                     |
| `E task: 5:`                                          | 5                 | `[""]` (an empty actor, drawn in the legend)             |
| `T: 5: me, me`                                        | 5                 | `["me", "me"]` (two circles, one legend entry)           |
| `Task: 0x10: me`, `Task: 1e1: me`, `Task: +3: me`     | 16, 10, 3         | `["me"]`                                                 |
| `Task: abc: me`, `title-styling` fixture `size: 2rem` | NaN               | `["me"]`                                                 |
| `Task: -2: a`                                         | -2                | `["a"]`                                                  |
| `Task : 5: a` (blank before the colon)                | 5                 | `["a"]`; the name keeps its trailing blank `"Task "` [V] |

`Number()` is JavaScript's `StringToNumber` [S, checked in Node]: blank and empty strings give 0, `0x`, `0b`, `0o`, `Infinity`, `+`/`-` signs, and surrounding white space are accepted; `5px`, `2rem`, `1_000`, `NaN` give NaN.

### 4.3 Names, sanitising, ordering

| Value               | Sanitised?          | Trimmed?                                                          | Evidence                                                                                  |
| ------------------- | ------------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Section name        | no (addSection)     | no: trailing blanks kept (`S `)                                   | [V] `section S `                                                                          |
| Task name           | no (addTask)        | leading blanks never reach it; trailing blanks kept (`Make tea `) | [V]                                                                                       |
| Actor name          | no                  | trimmed (people trim, 73)                                         | [V] `me  , you ` gives `me`, `you`                                                        |
| Title               | yes, `sanitizeText` | no: `title  X` gives `" X"`, `title A # B` gives `"A "`           | [V]                                                                                       |
| accTitle / accDescr | yes                 | yes (grammar `trim()`), plus the setter rules above               | [V]                                                                                       |
| `getActors` order   | n/a                 | UTF-16 code units                                                 | [S] default comparator (checked in Node, `arc-probe.cjs`); the legend order is [V] in 4.3 |

Legend order, [V]: the legend is written in `Object.keys(actors)` order (journeyRenderer.ts:25), where integer-like keys come first in ascending numeric order and the others in insertion order. The colour and position come from the sorted list (154, 153-157). For `2, 10, B, a, A, b, 1, '', Zed` the legend is `1, 2, 10, '', A, B, Zed, a, b`, and each circle's `actor-N` class is its sorted position.

### 4.4 Title precedence [V]

1. `Diagram.fromText` sets the front matter title before parsing (Diagram.ts:52-54), after `clear` and `init`.
2. Each grammar `title` line calls `setDiagramTitle` (journey.jison:61) and overwrites the value, even when the new value sanitises to `""`.
3. So: the last grammar title line wins; the front matter title applies only when the grammar has no title line; a title that sanitises to `""` removes the front matter title.

Observed: front matter `FM title` + `title <script>alert(1)</script>` gives no title element. Front matter + `title Grammar` gives `Grammar`. Front matter alone gives `FM title`.

### 4.5 Entities and sanitising of header texts [V]

- `encodeEntities` (Diagram.ts:77) turns `#9829;` into `ﬂ°°9829¶ß` before parsing. Both the title and the task names keep the placeholder in the DB.
- The decode happens on the output (mermaidAPI.ts:359, utils.ts:904-906): `ﬂ°°` -> `&#`, `ﬂ°` -> `&`, `¶ß` -> `;`. Then the strict DOMPurify pass turns the numeric reference into the character.

| Input                              | Output text                                                                                 |
| ---------------------------------- | ------------------------------------------------------------------------------------------- |
| `title #9829; x`                   | `♥ x`                                                                                      |
| task `Coffee #9829; time`          | `Coffee ♥ time`                                                                            |
| task `Task #amp; x`                | `Task & x`                                                                                  |
| task `Task #foo; x`                | `Task &foo; x` (an unknown name stays as written)                                           |
| title `A & B` (no `<` in the text) | `A & B` (DOMPurify returns text without `<` unchanged)                                      |
| title `<b>bold</b> text`           | `<b>bold</b> text` shown as text (the tag is kept by DOMPurify and then escaped in the SVG) |

`sanitizeText` (common.ts:71-83) runs `sanitizeMore` first (common.ts:56-69: under the default `htmlLabels` and `strict` it runs `removeScript`, which is DOMPurify with its default configuration; with `htmlLabels: false` it does nothing), then DOMPurify with `FORBID_TAGS: ['style']` (79). Under the default config DOMPurify therefore runs twice [S]. `ferrigura-syntax/src/sanitize.rs` implements the second pass only (its header, lines 1-30) [S].

Named references: the sanitiser decodes only the 39 names in its table (`sanitize.rs:137-180`), and any other name stays as written. A full HTML5 table is needed to decode every `#name;` the reference would decode [S; only the `foo` case was run].

## 5. Canonical model proposal

Follows [../canonical-model.md](../canonical-model.md): version 1 and camelCase keys; strings are always present; numbers are JSON numbers, with the sentinel strings described below.

```json
{
  "version": 1,
  "title": "My working day",
  "accTitle": "This is a title",
  "accDescr": "This is a description",
  "sections": ["Go to work", "Go home"],
  "tasks": [
    { "section": "Go to work", "task": "Make tea", "score": 5, "people": ["Me"] },
    { "section": "Go to work", "task": "Do work", "score": 1, "people": ["Me", "Cat"] }
  ],
  "actors": ["Cat", "Me"]
}
```

(Illustrative; the fields follow the `dark-theme` and `should-render…chart` fixtures.)

| Field                  | Type                                            | Rule                                                        |
| ---------------------- | ----------------------------------------------- | ----------------------------------------------------------- |
| `version`              | 1                                               | `CANONICAL_VERSION`                                         |
| `title`                | string                                          | title precedence, 4.4; sanitised                            |
| `accTitle`, `accDescr` | string                                          | the setter rules of 4.1; sanitised; never trimmed further   |
| `sections`             | string[]                                        | every `section` statement in order, duplicates kept, raw    |
| `tasks[].section`      | string                                          | `currentSection` at the time, `""` before the first section |
| `tasks[].task`         | string                                          | the task name as lexed (trailing blanks kept), raw          |
| `tasks[].score`        | number, or `"NaN"`, `"Infinity"`, `"-Infinity"` | `Number(pieces[0])` (4.2)                                   |
| `tasks[].people`       | string[]                                        | split on `,`, each trimmed; duplicates and `""` kept        |
| `actors`               | string[]                                        | `getActors()`: unique, sorted by UTF-16 code units          |

**Left out, and why:**

| Left out                                                                 | Why                                                         |
| ------------------------------------------------------------------------ | ----------------------------------------------------------- |
| `type` of a task                                                         | always equals `section` (journeyDb.js:76, 78)               |
| `addTaskOrg`                                                             | dead code (86-95)                                           |
| `x`, `y`, `width`, `height`, `colour`, `fill`, `num`, `actors` of a task | written by the renderer (journeyRenderer.ts:338-345)        |
| `bounds`, `verticalPos`, `sequenceItems`, `taskCount`                    | renderer state                                              |
| Journey config                                                           | renderer input; `setConf` at init (journeyDiagram.ts:13-16) |

**Config dependency:** the header texts depend on `htmlLabels` and `securityLevel` through `sanitizeText`. The exporter pins the defaults (strict, htmlLabels on) as the flowchart exporter does [S].

**Score as JSON.** A JSON number cannot hold NaN or the infinities. The reference's score is a JS number; the sentinel strings are the same convention M2 uses for non-finite configuration numbers (STATUS.md, the M2 handover's known gaps, lists them). This needs a line in `canonical-model.md`; it does not change the meaning of the field [U whether the coordinator accepts it].

## 6. Specs

| File                                           |                    Cases | Level                                   | What they check                                                                                                                                                                                                 |
| ---------------------------------------------- | -----------------------: | --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/user-journey/journeyDb.spec.js`      | 9 (8 `it.each` rows + 1) | DB                                      | `clear` empties tasks, accTitle, accDescription, sections, actors (rows 17-44); the full task list and actor order `['Child#1','Child#2','Dad','Mum']` (46-91)                                                  |
| `diagrams/user-journey/parser/journey.spec.js` |                        7 | Parser, with DB effects via `parser.yy` | all 7 call `parse` and expect no throw; 3 also check the DB state: the accDescr block and title (31-46), accTitle (47-55), and the eight tasks with exact `score`, `people`, `section`, `task`, `type` (74-149) |

- No spec renders, measures or touches the DOM. Rendering is covered only by the e2e screenshots (section 9). Rendering needs its own geometry check (section 10).
- The `it.each` table in `journeyDb.spec.js` uses `convert` from `tests/util.js`.
- STATUS.md says `≈ Tests 8` for journey. The correct count is 16. Update the row when M6 starts.

Port mapping (proposal): the Rust spec files `crates/ferrigura-syntax/tests/journey_*.rs`, each test with the reference's name (README, test strategy 1). The DB-level tests become parse-and-inspect tests of the model, since the port has no DB. The `clear` tests become "a new parse starts empty"; `tasks and actors should be added` becomes a parse of the equivalent text. The `not.toThrow()` tests become `parse(...).is_ok()`. The task-object test asserts the canonical fields, without `type`.

## 7. Renderer

### 7.1 Output skeleton [V, from the 18 reference SVGs]

```
<svg id="ID" width="100%" xmlns=… style="max-width: Wpx;" viewBox="X -25 W H+extra" preserveAspectRatio="xMinYMin meet" height="H+extra+25" role="graphics-document document" aria-roledescription="journey" [aria-describedby aria-labelledby]>
  <title id="chart-title-ID">accTitle</title>          (only with accTitle; a11y, inserted first)
  <desc id="chart-desc-ID">accDescr</desc>             (only with accDescr; inserted before style, so after title)
  <style>…</style>                                     (inserted before the first child, mermaidAPI.ts:600)
  <g></g>                                              (empty group from appendDivSvgG, mermaidAPI.ts:419)
  <defs><marker id="ID-arrowhead" refX="5" refY="2" markerWidth="6" markerHeight="4" orient="auto"><path d="M 0,0 V 4 L6,2 Z"/></marker></defs>   (svgDraw.js:396-404)
  legend, per actor in Object.keys order:
    <circle cx="20" cy=yPos class="actor-POS" fill=colour stroke="#000" r="7"/>
    <text x="40" y=yPos+7+20i class="legend"><tspan x="50">LINE</tspan></text>        (one per line)
  per section and task, in task order:
    section group <g>: rect + text (7.4)
    task group <g>: <line id="ID-taskK" …/>, face, task rect, actor circles, text (7.5, 7.6)
  <text x=leftMargin font-size=titleFontSize font-weight="bold" y="25" fill=titleColor font-family=titleFontFamily>TITLE</text>
  <line x1=leftMargin y1=4*height x2=W-leftMargin-4 y2=4*height stroke-width="4" stroke="black" marker-end="url(#ID-arrowhead)"/>
</svg>
```

[V] The order of the title and activity line is last; the legend comes before any task; the empty `<g>` and `<defs>` follow `<style>`.

### 7.2 Constants and config (see section 8)

Values below are the schema defaults unless the theme says otherwise.

| Name              | Value                                        | Use                                                |
| ----------------- | -------------------------------------------- | -------------------------------------------------- |
| `diagramMarginX`  | 50                                           | section width gap, width margin                    |
| `diagramMarginY`  | 10                                           | section band height, height margin                 |
| `leftMargin`      | 150                                          | minimum left margin, legend                        |
| `maxLabelWidth`   | 360                                          | legend wrap width                                  |
| `width`           | 150                                          | task box width                                     |
| `height`          | 50                                           | task box height                                    |
| `taskMargin`      | 50                                           | gap between tasks                                  |
| `boxTextMargin`   | 5                                            | legend text offset (`tspan x = 40 + 2*5`)          |
| `actorColours`    | 6 colours                                    | legend and circles, cycle by position              |
| `sectionFills`    | 7 colours                                    | presentation attribute only (CSS overrides, 7.8)   |
| `sectionColours`  | `['#fff']`                                   | fallback text colour only (7.6)                    |
| `titleColor`      | `''`                                         | title `fill` (written as `fill=""`)                |
| `titleFontFamily` | `"trebuchet ms", verdana, arial, sans-serif` | title                                              |
| `titleFontSize`   | `4ex`                                        | title                                              |
| `taskFontSize`    | 14                                           | fallback text only (`font-size: 14px` written [V]) |
| `taskFontFamily`  | `"Open Sans", sans-serif`                    | fallback text only                                 |
| `textPlacement`   | `fo`                                         | which text form is used (7.6)                      |
| `useMaxWidth`     | true (base config, schema 470)               | `width="100%"` plus `max-width`                    |

Derived constants: task band y = `2*height + diagramMarginY` = 110 (svgDraw; renderer 287-288); stop-y of every task box = 450 (`300 + 5*30`, renderer 349, svgDraw 204); face centre y = `300 + (5 - score)*30` (svgDraw 218); activity line y = `4*height` = 200 (renderer 187).

### 7.3 Layout, step by step (journeyRenderer.ts)

```
actors := sorted unique names (getActors)                  (147, 152-158)
  colour(pos) := actorColours[pos mod len]                  (154)
legend:  yPos := 60; maxWidth := 0
  for name in Object.keys(actors):                          (25)
    circle(cx=20, cy=yPos, r=7, fill=colour, stroke=#000)  (27-35)
    lines := wrapLegend(name, maxLabelWidth)                (38-92; 7.3.1)
    for i, line: text(x=40, y=yPos+7+20i); lineWidth := measure(line)   (94-105)
      if lineWidth > maxWidth and lineWidth > conf.leftMargin - lineWidth:  (109)
        maxWidth := lineWidth
    yPos += max(20, 20*len(lines))                           (114)
leftMargin := conf.leftMargin + maxWidth                    (161)
bounds.insert(0, 0, leftMargin, 50*len(actors))            (162)
tasks: section and task drawing (7.4, 7.5)                  (163; 296-350)
box := bounds
height := box.stopy - box.starty + 2*diagramMarginY         (178)
width  := leftMargin + box.stopx + 2*diagramMarginX         (179)
extra  := title ? 70 : 0                                    (194)
viewBox := `${box.startx} -25 ${width} ${height+extra}`     (195)
height attribute := height + extra + 25                     (197)
```

`bounds.insert(x0, y0, x1, y1)` (renderer 256-268) keeps the min and max of each side; the `updateBounds` branch (227-255) runs over `sequenceItems`, which is always empty [V by reading]. So `bounds` is only min and max.

Task inserts (349): `insert(task.x, 110, task.x + diagramMarginX + taskMargin, 450)`. Note the right edge is `x + 100`, not `x + width`; the viewBox still contains the last box because `width` adds `leftMargin` and `2*diagramMarginX` [V: `Sit down` x=950, viewBox width 1300].

Checked [V]: the default chart (`W = 1300`, `H = 470`, title present: viewBox `0 -25 1300 540`, height attribute 565); `simple-test` (no tasks: `0 -25 400 90`, height 115); `E-Commerce` with `useMaxWidth: false` (`width="700"`, no `style`, viewBox `0 -25 700 540`, height 565).

#### 7.3.1 Legend wrap (journeyRenderer.ts:45-92) [V]

```
wrapLegend(name, maxW):
  if measureRaw(name) <= maxW: return [name]            (45-46; measureRaw uses name as given, 38-40)
  words := name.split(' ')                                (49; empty words kept)
  current := ''; lines := []
  for word in words:                                       (53-85)
    test := current ? current + ' ' + word : word          (55)
    if measure(test) > maxW:                               (57-59)
      if current: lines.push(current)                      (61-62)
      current := word                                      (64)
      if measure(word) > maxW:                             (67-68)
        broken := ''
        for ch in codePoints(word):                        (70)
          broken += ch                                     (71)
          if measure(broken + '-') > maxW:                 (72-73)
            lines.push(broken[:-1] + '-')                  (75)   <- see the quirk below
            broken := ch                                   (76)
        current := broken                                  (79)
    else: current := test                                  (83)
  if current: lines.push(current)                          (88-90)
```

- [V] The stray line. With `maxLabelWidth: 5` and the name `AB`, the reference draws `-`, `A-`, `B` (`w-tiny`). The first character plus its hyphen is already too wide, so `broken[:-1]` is empty and `-` is pushed. The port must reproduce it.
- [V] `Supercalifragilisticexpialidocious` at 14 px Lexica, maxLabelWidth 100: `Supercalifragili-` (97.547), `sticexpialidoci-` (93.844), `ous` (21.828). Trace: the first prefix whose `-` form exceeds 100 px is `Supercalifragilis`, so `Supercalifragili-` is pushed.
- [V] Trailing blanks: `measure("to check … is ")` equals `measure("… is")` (probe-width). A doubled blank splits into an empty word; the joined line keeps the double blank, which Chromium collapses when it draws.
- `measure` (the reference's `getBoundingClientRect().width` of the text) ignores leading and trailing blanks and collapses blank runs to one [V: `"ab "` = `"ab"` = 17.484375; `"a b"` = `"a b"` = 21.953125 at 16 px].

Test vectors [V, harness config: Lexica 14 px, `redux-color`]:

| Input                                                                                                                         | maxLabelWidth | Lines                                                                                                                                 | Widths                     |
| ----------------------------------------------------------------------------------------------------------------------------- | ------------: | ------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| `This is a long label that will be split into multiple lines to test the wrapping functionality`                              |           320 | `This is a long label that will be split into multiple` / `lines to test the wrapping functionality`                                  | 297.688 / 234.375          |
| `This is another long label that will be split into multiple lines to test the wrapping functionality`                        |           320 | `This is another long label that will be split into` / `multiple lines to test the wrapping functionality`                            | 283.453 / 288.172          |
| `Darkoob userf deliberately increasing the size of this label to check if distance between legend and diagram is  maintained` |           360 | `Darkoob userf deliberately increasing the size of this label` / `to check if distance between legend and diagram is ` / `maintained` | 357.125 / 317.563 / 69.219 |

With the default theme (16 px, trebuchet ms, not bundled) the same text wraps into three lines, against two for the harness (14 px, Lexica): font and size both differ, which is the dependency of section 10.

#### 7.3.2 Measured values and the 1/64 px grid

- Widths are sums of runs rounded up to 1/64 px: `455.453125` (`leftMargin` with the 305.453 legend line) [V]. `measure.rs:30-33` (`layout_width`) does the same rounding.
- `leftMargin` = 150 + the widest legend line that is over 75 px [V: 455.453125 = 150 + 305.453125; the title x equals it].
- The legend text is measured in the font of the root (the `.legend` class sets `font-family`, the root sets `font-size`) [V: root `font-size: 14px` for the redux themes, `16px` for the default theme].

### 7.4 Sections (svgDraw.js:161-189; renderer 296-326)

```
section starts where task.section != lastSection (renderer 297), lastSection starts as ''
  so tasks before any section get no header (checked [V], `Early: 5: me`)
  fill    := fills[n mod len(sectionFills)]; num := n mod len(sectionFills); colour := textColours[n mod len(sectionColours)]   (298-300)
  count   := consecutive tasks with the same section                        (303-311)
  x       := i*taskMargin + i*width + leftMargin      (i = index of the first task, 313-315)
  rect: x, y=50, width = width*count + diagramMarginX*(count-1), height = 50
        rx=ry=3, class "journey-section section-type-N", fill=fill, stroke=#666   (169-176)
  text: centre x + width/2, y = 75, class "journey-section section-type-N"       (178-188)
  n += 1
```

[V] `section-type-N` equals `n mod 7`, not the section name: two sections named `S` and `S` (non-consecutive) get types 0, 1, 2 (`repeat-section`).

[V] `fills` and `textColours` are module constants taken from the config when the module loads (renderer 281-282), so a later `sectionFills` changes nothing in the drawing; the CSS overrides the attribute anyway (7.8). Not isolated by a test [U].

### 7.5 Tasks (svgDraw.js:200-262; renderer 328-349)

```
for i, task:
  center := x_i + width/2                    x_i = i*taskMargin + i*width + leftMargin
  g:
    <line id="ID-task{k}" x1=center y1=110 x2=center y2=450 class="task-line"
          stroke-width="1px" stroke-dasharray="4 2" stroke="#666"/>       (205-214)
    face at (center, cy = 300 + (5 - score)*30)                           (216-220)
    <rect x=x_i y=110 width=150 height=50 rx=3 ry=3 fill=fill class="task task-type-N" stroke="#666"/>  (222-231)
    for person in task.people (duplicates included):                      (233-249)
       <circle cx=x_i+14+10*j cy=110 r=7 fill=colour(person) stroke=#000 class="actor-POS"><title>person</title></circle>
    text (7.6)
```

`k` is `taskCount`: starts at -1 and is reset by `initGraphics` (svgDraw.js:393), then incremented per task (203) [V: `chart-default-task0` … `task4`].

#### 7.5.1 Face (svgDraw.js:8-95)

| Part                           | Attributes                                                                                                      | Source |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------- | ------ |
| Face circle                    | `cx, cy, class="face", r=15, stroke-width=2, overflow="visible"`; no `fill` attribute (CSS `#FFF8DC`)           | 10-17  |
| Left eye                       | `cx-5, cy-5, r=1.5, stroke-width=2, fill=#666, stroke=#666`                                                     | 21-29  |
| Right eye                      | `cx+5, cy-5`, same                                                                                              | 31-39  |
| Smile (score > 3)              | `<path class="mouth" d="…" transform="translate(cx,cy+2)">`, arc `start=π/2, end=3π/2, inner=7.5, outer=15/2.2` | 42-54  |
| Sad (score < 3)                | same class, `start=3π/2, end=5π/2`, transform `translate(cx,cy+7)`                                              | 57-69  |
| Ambivalent (score = 3, or NaN) | `<line class="mouth" stroke="#666" x1=cx-5 y1=cy+7 x2=cx+5 y2=cy+7 stroke-width="1px">`                         | 72-84  |

- [V] d3-shape 3.2.0 gives `M7.5,0A7.5,7.5,0,1,1,-7.5,0L-6.818,0A6.818,6.818,0,1,0,6.818,0Z` for the smile and `M-7.5,0A7.5,7.5,0,1,1,7.5,0L6.818,0A6.818,6.818,0,1,0,-6.818,0Z` for the sad, identical to the reference output.
- The inner radius (7.5) is larger than the outer (6.818): d3's `arc` swaps them (`d3-shape/src/arc.js:101`, `if (r1 < r0) r = r1, r1 = r0, r0 = r`). `svg::arc_path` (ferrigura `svg.rs:49-147`) does not swap; the caller must.
- [V] Coordinates are rounded to three decimals by `d3-path` 3.1.0's `pathRound(3)`, `Math.round(x*1000)/1000`: `6.8181818…` is written `6.818`. `svg::arc_path` writes full precision (`number()`), so the faces need a rounding step if the path string must match; the geometry tolerance (0.05 px) does not need it [U for the tolerance decision].
- The ambivalent mouth's `stroke` attribute is written twice (`2`, then `#666`); the last wins [S, 76 and 83].
- NaN and infinite scores: `cy` is `NaN`, `-Infinity` and so on in the attribute text [V: `cy="NaN"` for `size: 2rem`, `cy="-30"` for `0x10`; `Infinity` [S]].

### 7.6 Text placements (svgDraw.js:274-390)

| `textPlacement`                  | Function            | Output                                                                                                                                                                                                                                                                                                                              | Visible?                               |
| -------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| `fo` (default, schema 1864-1869) | `byFo` (346-372)    | `<switch>` with `<foreignObject x y width height>` (position attribute dropped by DOMPurify [V]) containing `<div xmlns=XHTML class=… style="display: table; height: 100%; width: 100%;"><div class="label" style="display: table-cell; text-align: center; vertical-align: middle;">TEXT</div></div>`, then the `byTspan` fallback | the `foreignObject` (HTML)             |
| `tspan`                          | `byTspan` (307-334) | one `<text>` per `<br>` piece, each with `<tspan x dy=…>`                                                                                                                                                                                                                                                                           | yes, SVG text                          |
| `old`                            | `byText` (285-294)  | one `<text x y=y+h/2+5 class … style="text-anchor: middle;">`                                                                                                                                                                                                                                                                       | yes; `font-color` style is dropped [V] |

`byTspan` details [V]: `lines = content.split(/<br\s*\/?>/gi)` (310); `dy = i*fontSize - fontSize*(n-1)/2` (312); `y = y + h/2` with `dominant-baseline=central`, `alignment-baseline=central` (327-330); `text-anchor: middle`; `font-size: 14px` and `font-family: "Open Sans", sans-serif` written as styles (316-320; the `14px` unit is observed, its origin is [U]).

#### 7.6.1 The default `fo` text is HTML [V]

- The visible label is the `div.label`; the SVG `text` is a fallback that Chromium does not draw.
- The cell is the foreignObject box (`width = 150`, `height = 50`); lines are centred horizontally (`text-align: center`) and, when the block is shorter than the box, vertically (`vertical-align: middle`).
- Line box height: 17 px for Lexica at the harness size (14 px) [V, Range rects]. The port needs the line height from `ferrigura-text` (`text_box`, `metrics.rs:67`) [U].
- Block taller than the box: the block starts at the top of the box and grows downward [V: 4 lines, box height 50, div height 68].
- Break opportunities: spaces, and after a hyphen [S]. A single word wider than the cell: its overflow and centring are [U].
- `<br>` is not a break here: both `<br/>` and `<br>` are shown as literal text, on one line [V: `Make<br/>tea<br>now`]. The `tspan` fallback is the only place where `<br>` splits.
- An unknown named entity stays as text: `Task &foo; x` [V].

Test vectors [V, Lexica 14 px, width 150, line box 17]:

| Text                                                                               | Lines                                                                                                                         |
| ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `This is a very long task name that should wrap across several lines in the box`   | `This is a very long task` / `name that should wrap` / `across several lines in` / `the box` (block 68 px, starts at the top) |
| `Gustavo Fring is played by Giancarlo Esposito and is a character in Breaking Bad` | `Gustavo Fring is played` / `by Giancarlo Esposito` / `and is a character in` / `Breaking Bad`                                |
| `Make tea`                                                                         | one line, centred: x centre = task centre, y = top + (50 - 17)/2                                                              |

#### 7.6.2 Section and task text, in all placements

- Section text centre x = section centre; y = 75 (`byTspan`/`byFo` use the same `y + height/2`).
- Task text centre x = task centre; y = 135.

### 7.7 SVG numbers and strings

- Numbers: JS `String(number)`; `NaN`, `Infinity`, `-Infinity` are written as such [V].
- Width and height: `1/64 px` grid through `layout_unit` (`pie.rs:447-452`) for the widths built from text [V: all `viewBox` widths in the harness are multiples of 1/64].
- Colours: `fill` and `stroke` are presentation attributes; CSS (7.8) overrides several of them [V].
- The text is escaped by `textContent` (raw `<` and `&` in names become `&lt;`, `&amp;`); `svg::escape` (`svg.rs:9-22`) does the same.

### 7.8 CSS (styles.js with the common header of styles.ts:42-45)

Rules that change what is drawn (theme variable names from the theme; `[V]` means the reference CSS was checked for the default theme):

| Selector                                 | Declaration                                                     | Gate                                                  | Effect                                                                                                                          |
| ---------------------------------------- | --------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `#ID` (styles.ts:42-45)                  | `font-family: fontFamily; font-size: fontSize; fill: textColor` | none                                                  | root text colour and size [V]                                                                                                   |
| `.label` (styles.js 4-7, 24-26)          | `font-family: fontFamily; color: textColor`                     | none                                                  | colour of the `fo` text [V]                                                                                                     |
| `.mouth` (8-10)                          | `stroke: #666`                                                  | none                                                  | mouth colour (overrides the attribute)                                                                                          |
| `line` (12-14)                           | `stroke: textColor`                                             | none                                                  | **all lines**: the task lines and the activity line take the text colour, not `#666` / `black` [V: `#333` on the default theme] |
| `.legend` (16-19)                        | `fill: textColor; font-family: fontFamily`                      | none                                                  | legend text [V]                                                                                                                 |
| `.label text` (21-23)                    | `fill: #333`                                                    | none                                                  | matches no element of the journey markup [S]                                                                                    |
| `.face` (28-31)                          | `fill: faceColor or #FFF8DC; stroke: #999`                      | `faceColor`                                           | face colour [V]                                                                                                                 |
| `.task-type-N, .section-type-N` (93-116) | `fill: fillTypeN`                                               | gated by **`fillType0`** for every N (94, 97, 100, …) | the visible task and section colour; overrides the presentation attribute [V]                                                   |
| `.actor-N` (118-135)                     | `fill: actorN`                                                  | gated by `actorN`                                     | no theme defines `actor0..5`; the rule is empty and dropped [V]                                                                 |

Shared rules in the same block (`.node`, `.edgeLabel`, `.cluster`, `div.mermaidTooltip`, `.label-icon`, the neo look) are not used by the journey markup [V by reading].

The journey's `.task-type-N` and `.section-type-N` use `fillTypeN`, which the themes define: the redux themes from `bkgColorArray` (`theme-redux-color.js:70`, loop 305-306), the others by `adjust` (`theme-base.js:208-215`, `theme-default.js:248-253`). Note: the redux comment says the journey `.label text` rule "does not win" (`theme-redux-color.js:298-304`) and that was checked against the DOM [S].

### 7.9 Accessibility

- `accTitle` and `accDescr` go to `<title id="chart-title-ID">` and `<desc id="chart-desc-ID">`; the root gets `aria-labelledby` and `aria-describedby` (accessibility.ts:45-65; mermaidAPI.ts:750-757). The insertion order makes the title first, then the desc, then the style [V].
- `role="graphics-document document"` and `aria-roledescription="journey"` (`setA11yDiagramInfo`) [V].
- `ferrigura-render/src/a11y.rs` already has `root_attributes` (line 28) and `children` (56); the pie uses it (M2 handover), and the journey can reuse it unchanged.

## 8. Configuration

### 8.1 Journey keys (schema lines in brackets)

| Key                                                                                                | Default (line)                                           | Read by                             | Notes                                                                     |
| -------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------- |
| `diagramMarginX`                                                                                   | 50 (1784)                                                | svgDraw 169-171; renderer 179, 340  |                                                                           |
| `diagramMarginY`                                                                                   | 10 (1787)                                                | renderer 178, 287                   |                                                                           |
| `leftMargin`                                                                                       | 150 (1791)                                               | renderer 109, 161                   | [V: `leftMargin: 300` changes the viewBox]                                |
| `maxLabelWidth`                                                                                    | 360 (1796)                                               | renderer 21, 45, 59                 |                                                                           |
| `width`                                                                                            | 150 (1800)                                               | svgDraw 169-171, 201, 226           |                                                                           |
| `height`                                                                                           | 50 (1805)                                                | svgDraw 172, 227; renderer 187, 287 |                                                                           |
| `boxMargin`                                                                                        | 10 (1810)                                                | renderer 238-249 only               | effectively unused: `updateBounds` runs over an empty list [V by reading] |
| `boxTextMargin`                                                                                    | 5 (1815)                                                 | renderer 100                        | legend text offset                                                        |
| `noteMargin`, `messageMargin`, `messageAlign`, `bottomMarginAdj`, `rightAngles`, `activationWidth` | 10, 35, center, 1, false, 10 (1820-1863)                 | not read                            | unused by the journey code [S]                                            |
| `taskFontSize`                                                                                     | 14 (1851-1853, type `*fontSizeType`)                     | svgDraw 308, 312, 319               | fallback only                                                             |
| `taskFontFamily`                                                                                   | `"Open Sans", sans-serif` (1854-1856)                    | svgDraw 308, 320                    | fallback only                                                             |
| `taskMargin`                                                                                       | 50 (1857-1859)                                           | renderer 314, 338, 349              |                                                                           |
| `textPlacement`                                                                                    | `fo` (1864-1869)                                         | svgDraw 387-389                     | 7.6                                                                       |
| `actorColours`                                                                                     | 6 colours (1870-1874)                                    | renderer 154                        | array merge, 8.2                                                          |
| `sectionFills`                                                                                     | 7 colours (1875-1879)                                    | renderer 281                        | captured at load [U]                                                      |
| `sectionColours`                                                                                   | `['#fff']` (1880-1884)                                   | renderer 282                        | captured at load; fallback colour only                                    |
| `titleColor`                                                                                       | `''` (1885-1888)                                         | renderer 123, 174                   |                                                                           |
| `titleFontFamily`                                                                                  | `"trebuchet ms", verdana, arial, sans-serif` (1889-1892) | renderer 125, 175                   | not bundled (section 10)                                                  |
| `titleFontSize`                                                                                    | `4ex` (1893-1896)                                        | renderer 124, 171                   |                                                                           |
| `useMaxWidth`                                                                                      | true (base, 470)                                         | renderer 181                        |                                                                           |

Not in the journey section but used: `fontSize` (root CSS comes from the theme, `theme-redux-color.js:27` = `14px`, `theme-default.js:33` = `16px`, `theme-base.js:24`).

Unused by the code: `activationWidth`, `messageMargin`, `messageAlign`, `noteMargin`, `bottomMarginAdj`, `rightAngles`, and `boxMargin` in effect. A port that reads the schema has to decide whether to keep them (M2's generator writes every schema key, `build.rs`, so keeping them costs nothing [S]).

### 8.2 How config reaches the renderer [V]

- `init(cnf)` runs on every parse with the resolved config and calls `setConf(cnf.journey)` (journeyDiagram.ts:13-16; `setConf` at renderer 7-13 copies every key into the module-level `conf`, which was taken from `getConfig().journey` at load, renderer 119).
- Reads at draw time use `getConfig().journey` (renderer 20, 122-125, 285) or the module `conf` (most of `draw`).
- [V] `initialize({journey: {leftMargin: 300}})` and front matter `leftMargin: 400` change the layout.
- [V] A second render in the same page goes back to the defaults: the config is reset per render.
- **Array merge** (assignWithDepth.ts:40-46): when both sides are arrays, the user's items are appended if missing. `actorColours: ['#123456']` is therefore `[default six..., '#123456']`: actors 0..5 keep their colours, and the seventh actor gets `#123456` [V]. ferrigura's M2 `layers.rs:181-188` implements the same append rule [S].

### 8.3 Theme variables used

| Variable                              | Used by                                                  | Defined in                                                                                                                     |
| ------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `fontFamily`, `fontSize`, `textColor` | `#ID`, `.label`, `.legend`                               | every theme (`theme-base.js:24` etc.)                                                                                          |
| `fillType0..7`                        | `.task-type-N`, `.section-type-N` (gated by `fillType0`) | `theme-base.js:208-215`; redux `theme-redux-color.js:305-306` (from `bkgColorArray`, line 70); `theme-redux-dark-color.js:321` |
| `faceColor`                           | `.face`                                                  | not defined by any shipped theme [S]                                                                                           |
| `actor0..5`                           | `.actor-N`                                               | not defined by any shipped theme [V: the rule is empty]                                                                        |
| `fontSize` for the redux themes       | root                                                     | `14px` (`theme-redux-color.js:27`, `theme-redux-dark-color.js:30,43`)                                                          |

## 9. Fixtures

18 files: 7 in `e2e/diagrams/journey/nightly-check/` and 11 in `pr-check/`; `visual-check/` is empty. The loader is generic (`e2e/rendering/mmd-snapshots.spec.ts` with `e2e/helpers/mmd-snapshots.ts`); there is no journey-specific e2e spec. Each fixture has front matter with `description` (ignored by mermaid) and sometimes `config`.

| Fixture                                   | Folder  | Config or directive                                                     | Feature                                                                                                         |
| ----------------------------------------- | ------- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| left-margin-150px                         | nightly | `journey.maxLabelWidth: 320`, `useMaxWidth: true`                       | empty actor (`Sign Up: 5:`); no legend wrap                                                                     |
| legend-diagram-spacing-long-labels        | nightly | `useMaxWidth: true`                                                     | long legend line; title with a leading blank; repeated section name `Darkoob`; a doubled blank in a legend text |
| single-word-hyphenation                   | nightly | `maxLabelWidth: 100`, `useMaxWidth: true`                               | char-level hyphen break                                                                                         |
| title-styling                             | nightly | `titleColor`, `titleFontFamily: Times New Roman`, `titleFontSize: 5rem` | NaN score (`size: 2rem`)                                                                                        |
| usemaxwidth-true                          | nightly | `useMaxWidth: true`                                                     | plain                                                                                                           |
| whitespace-wrapping-no-hyphens            | nightly | `maxLabelWidth: 200`, `useMaxWidth: true`                               | legend wrap at spaces                                                                                           |
| wrap-long-labels-multiline-margins        | nightly | `maxLabelWidth: 320`, `useMaxWidth: true`                               | legend wrap, three lines                                                                                        |
| dark-theme                                | pr      | `%%{init: {'theme': 'dark'}}%%` (single quotes)                         | theme `dark` falls back to redux-dark-color; `accTitle`, `accDescr`                                             |
| default-theme                             | pr      | `%%{init: {'theme': 'default'}}%%`                                      | falls back to redux-color                                                                                       |
| forest-theme                              | pr      | `theme: forest`                                                         | falls back to redux-color                                                                                       |
| neutral-theme                             | pr      | `theme: neutral`                                                        | falls back to redux-color                                                                                       |
| redux-color-theme                         | pr      | front matter `theme: redux-color`                                       | 8 sections (fillType0..7)                                                                                       |
| redux-dark-color-theme                    | pr      | `theme: redux-dark-color`                                               | 8 sections                                                                                                      |
| redux-dark-theme                          | pr      | `theme: redux-dark`                                                     | ends with `dark`: redux-dark-color                                                                              |
| redux-theme                               | pr      | `theme: redux`                                                          | redux-color                                                                                                     |
| should-render-a-user-journey-chart        | pr      | none                                                                    | the standard chart, one shared actor                                                                            |
| should-render-…-when-usemaxwidth-is-false | pr      | `useMaxWidth: false`                                                    | `width` attribute, no `style`                                                                                   |
| simple-test                               | pr      | none                                                                    | a section with no tasks                                                                                         |

Counts: `maxLabelWidth` in 4 fixtures (320, 100, 200, 320); `useMaxWidth: true` in 6, `false` in 1; themes in 8 (4 via directive, 4 via front matter); `titleColor` and friends in 1; `accTitle` and `accDescr` in 4; NaN score in 1; an empty actor in 1 (`left-margin-150px`).

### 9.1 Browser features the fixtures need

- `foreignObject` inside `switch`, with HTML line layout (7.6). All 18 fixtures use it for every task and section.
- Font families that are not bundled: `"trebuchet ms", verdana, arial` (default title and labels), `"Open Sans"` (fallback only), `Times New Roman` (title-styling) [V].
- DOM measurement: `getBoundingClientRect` of text (legend, 7.3.1) [V].
- Nothing else: no images, no icons, no markers other than the arrowhead.

## 10. Reference harness (M6 geometry check)

There is no journey geometry comparison yet. The M5 harness (`tools/port/geometry/`, `flowchart-geometry.ts`) is the model. For journey:

- Reference: the built bundle `packages/mermaid/dist` in Chromium, as `probe-ref.mjs` does [V]; the harness must pin: `fontFamily: "Lexica Ultralegible"`, `journey.titleFontFamily: "Lexica Ultralegible"`, `htmlLabels: false`, `securityLevel: strict`; `theme`: `redux-color` for every fixture whose theme ferrigura does not ship (the same fallback M2 uses, which maps `dark` to `redux-dark-color`). `taskFontFamily` is only the hidden fallback and need not be pinned.
- Keep `textPlacement: fo` (the default): it is the reference's visible output.
- Compare: root viewBox and attributes; legend circles and text boxes (widths of drawn lines), section rect and label box, task rects and lines, faces (cx, cy, mouth translate), actor circles, title box, activity line; tolerance 0.05 px, angles 1e-5.
- For the `fo` labels, compare the lines (Range rects) with ferrigura's line layout (7.6.1), not the element structure.

Without these pins the default-theme fixtures depend on the local fonts (trebuchet): the reference result changes with the machine. Confirmed by the wrap widths: the same legend text wraps into three lines in trebuchet 16 px and into two in Lexica 14 px [V] (font and size both differ).

## 11. Risks, effort, and open questions

### 11.1 Risks

| #   | Risk                                                                                           | Level            | Note                                                                                                                                     |
| --- | ---------------------------------------------------------------------------------------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | `fo` labels are HTML: line breaking and line box height in a 150 px cell                       | high             | the default of every fixture; test vectors in 7.6.1                                                                                      |
| R2  | Entity decode needs more of the HTML5 named table than `sanitize.rs` has (39 names)            | medium           | full table, or document the subset                                                                                                       |
| R3  | Default fonts (trebuchet, Open Sans, Times) are not bundled                                    | medium           | the harness pins Lexica; the default output of ferrigura will differ from the reference's default, as it does for the flowchart fixtures |
| R4  | No geometry harness for journey yet; e2e fixtures are screenshots                              | medium           | build the harness (section 10)                                                                                                           |
| R5  | Quirks that the port must keep (legend order, `__proto__`, NaN, stray hyphen, title overrides) | low each         | section 11.3                                                                                                                             |
| R6  | `getTasks` is not idempotent in the reference                                                  | none in practice | the renderer calls it once per render                                                                                                    |

### 11.2 Effort, relative to the flowchart

| Part                                                            |                                                            Reference lines |                      Rust estimate | Comparison        |
| --------------------------------------------------------------- | -------------------------------------------------------------------------: | ---------------------------------: | ----------------- |
| Journey code, no specs                                          | 1,151 (jison 69, DB 130, renderer 356, svgDraw 416, styles 139, others 41) |                                    |                   |
| Flowchart parser and DB                                         |                                           1,992 (flow.jison and flowDb.ts) |  ferrigura-syntax flowchart: 3,647 | M3                |
| Flowchart renderer                                              |                                                               (many files) | ferrigura-render flowchart: 19,936 | M5                |
| Journey parser (scanner, grammar, DB, model)                    |                                                          200 (DB, grammar) |                         500 to 700 | about 1/5 of M3   |
| Journey renderer (layout, legend wrap, faces, CSS, `fo` layout) |                                                                        900 |                       900 to 1,200 | about 1/15 of M5  |
| Config, harness, tests                                          |                                                                            |                         400 to 600 | reuses M5 harness |

Overall: about a tenth of M3 plus M5 in code (estimate 2,000 to 2,500 Rust lines including tests; M3 plus M5 is about 23,600 lines). The risk is concentrated in R1, R2 and the harness, which is a larger share of the work than the code size suggests.

### 11.3 Quirks to decide (the port keeps or documents each)

| #   | Quirk                                                                                                        | Where                              | Verified                                 | Proposal                                         |
| --- | ------------------------------------------------------------------------------------------------------------ | ---------------------------------- | ---------------------------------------- | ------------------------------------------------ |
| Q1  | Legend order: integer-like names first, ascending                                                            | renderer 25                        | [V]                                      | keep (cheap)                                     |
| Q2  | `__proto__` actor is missing from the legend but still draws a circle (colour and position of the prototype) | renderer 25, 153, 330              | [V]                                      | keep; document                                   |
| Q3  | Stray `-` line when one character plus `-` exceeds the width                                                 | renderer 72-76                     | [V]                                      | keep                                             |
| Q4  | Score `NaN` and non-finite values in `cy`, and ambivalent face                                               | svgDraw 86-92                      | [V]                                      | keep; sentinel strings in the model              |
| Q5  | Pieces after a second `:` are dropped                                                                        | journeyDb 70-72                    | [V]                                      | keep                                             |
| Q6  | Title sanitised to `""` removes the front matter title                                                       | Diagram.ts 52-54, journey.jison 61 | [V]                                      | keep                                             |
| Q7  | `sectionFills` and `sectionColours` captured at module load                                                  | renderer 281-282                   | [U] (attribute unchanged, CSS overrides) | do not replicate; document                       |
| Q8  | Array config keys append to the defaults                                                                     | assignWithDepth 40-46              | [V]                                      | keep (M2 already does)                           |
| Q9  | Task and section names keep trailing blanks; actors are trimmed                                              | journeyDb 73, journey.jison 30     | [V]                                      | keep                                             |
| Q10 | Double DOMPurify pass on header texts                                                                        | common.ts 56-83                    | [S]                                      | check against `sanitize.rs`                      |
| Q11 | `<br>` in a `fo` label is shown literally                                                                    | svgDraw 346-372                    | [V]                                      | keep                                             |
| Q12 | Parse errors render an error diagram, then reject                                                            | mermaidAPI 573-582, 659-661        | [V]                                      | ferrigura returns an error (M2 policy); document |

### 11.4 Open questions for the coordinator

1. **Decision: the `fo` text.** ferrigura draws text as SVG, not HTML (decision 0004). Option A: draw the default `fo` as SVG text with wrapping that reproduces Chromium's HTML layout (7.6.1), and keep `textPlacement: tspan` and `old` as they are; this follows 0004 and the geometry check compares lines, not elements. Option B: emit `foreignObject` as the reference does; this keeps HTML inside the SVG, which ferrigura's output does not otherwise need, and the geometry depends on a browser. Recommendation: A, recorded as a decision (0008, or an amendment to 0004).
2. **Named entities.** Full HTML5 table, or the 39-name subset with a note? (R2)
3. **Score encoding** in the canonical model: the sentinel strings (section 5). Confirm it is an allowed convention.
4. **Quirks Q1-Q12**: confirm the proposals in 11.3, especially Q2 and Q7.
5. **Harness pins**: confirm the list in section 10 (Lexica for `fontFamily` and `titleFontFamily`, `redux-color` for unshipped themes).
6. **Effort**: a single milestone for journey, or a split (parser first, then renderer after the `fo` decision)?

### 11.5 Unverified items

- [U] The line box height of ferrigura's `text_box` for Lexica at 14 px equals the reference's 17 px.
- [U] Break opportunities in Chromium's HTML layout are spaces and hyphens only (the test vectors support it, but no hyphenated `fo` label was run).
- [U] The `14px` in the fallback `font-size` comes from a conversion of the numeric config value; no effect on the output.
- [U] Whether `sanitize.rs` needs the first DOMPurify pass (Q10).
- [U] Module-level capture (Q7): the attribute is the default in both renders, which the array append rule alone explains.
- [U] `Infinity` scores (only `NaN` was rendered in the browser; the rest from `Number()` in Node).
- [U] The `arrowMarkerAbsolute` option and the `loose` security level were not run.
- [U] Font-dependent defaults on other platforms (Linux, Windows); the reference was run on macOS.

## 12. Reproducing the probes

```
cd /private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/m6-journey
node probe-ref.mjs "$(cat batch2.json)"          # reference SVGs in ref-<id>.svg
python3 extract.py ref-<id>.svg                  # summary: viewBox, legend, sections, tasks, faces
node probe-width.mjs '["ab ","ab"]'              # Chromium widths of strings (Lexica 16 px)
node arc-probe.cjs                               # d3-shape arc strings and Number() table
```

The probe runner serves `packages/mermaid/dist` and `crates/ferrigura-text/fonts` over HTTP; it writes only to the scratchpad. The dist bundle is the built reference of this checkout; a rebuild is not needed.
