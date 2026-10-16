# M6 research: timeline, and the info renderer

Research only. No repository file was changed except this note. Reference: the JS tree of this worktree (HEAD `04b952b4e`, `packages/mermaid` 12.2.0). Line references are to that tree.

Brief items 1 to 5 are sections 1 to 5. Brief item 6 (info) is section 8, at the end, as requested. Brief items 7 and 8 are sections 6 and 7.

Verification used:

- **Chromium.** The reference bundle `packages/mermaid/dist/mermaid.min.js` (built 2026-10-16 11:57, the same commit as far as I can tell) was loaded through Playwright and used to render all 30 timeline fixtures and about 40 hand-written inputs. Every DOM, bounding-box and text-width claim below marked "verified" comes from these runs.
- **Jison.** The grammar `timeline.jison` was generated unchanged with the `jison` 0.4.18 in the main checkout, and the generated lexer and parser were run with a recording stub for the DB. Parser claims marked "verified" come from these runs.
- **Code reading.** Everything else is marked "from code".
- **Probes.** All probe scripts and outputs are in the scratchpad `m6-timeline/`. None is committed.

Result in one line: the timeline is a small grammar and a small renderer, but it has a dependency the parity plan does not cover. 22 of the 30 fixtures take the non-redux branch of the renderer, which the shipped themes (`redux-color`, `redux-dark-color`) cannot produce (section 5.3, section 7).

---

## 1. Sources

| Concern                  | File (`packages/mermaid/src/…`)                                                                                                                                                                                                               | What matters                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Grammar and lexer        | `diagrams/timeline/parser/timeline.jison`                                                                                                                                                                                                     | Lexer 6-41, grammar 43-86. Rules in section 2.2.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Detector                 | `diagrams/timeline/detector.ts`                                                                                                                                                                                                               | 9-11: `/^\s*timeline/`, case-sensitive (the lexer is not).                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Diagram definition       | `diagrams/timeline/timeline-definition.ts`                                                                                                                                                                                                    | 8-16: `draw` picks `verticalRenderer` when `getDirection() === 'TD'`, else the horizontal one. 18-23: `{db, renderer, parser, styles}`.                                                                                                                                                                                                                                                                                                                                                                                                  |
| DB                       | `diagrams/timeline/timelineDb.js`                                                                                                                                                                                                             | Whole file (114 lines). Section 2.4.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Horizontal renderer      | `diagrams/timeline/timelineRenderer.ts`                                                                                                                                                                                                       | 29-268 `draw`; 270-341 `drawTasks`; 343-380 `drawEvents`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Vertical renderer        | `diagrams/timeline/timelineRendererVertical.ts`                                                                                                                                                                                               | 29-39 constants; 41-246 `draw`; 248-296 `drawTasks`; 298-344 `drawEvents`.                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Shared node drawing      | `diagrams/timeline/svgDraw.js`                                                                                                                                                                                                                | 436-450 `initGraphics`; 456-496 `wrap`; 498-568 `drawNode`; 570-584 `getVirtualNodeHeight`; 586-610 `defaultBkg`. Lines 4-434 are journey helpers, not used by the timeline.                                                                                                                                                                                                                                                                                                                                                             |
| Styles                   | `diagrams/timeline/styles.js`                                                                                                                                                                                                                 | 5-88 `genReduxSections`; 90-146 `genSections`; 148-201 `getStyles`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Config schema            | `schemas/config.schema.yaml`                                                                                                                                                                                                                  | 386-387 (`timeline` key); 1898-2025 `TimelineDiagramConfig` (`leftMargin` 1924-1930, default 150; `padding` 1939, no default; `disableMulticolor` 2021-2024, default false); 464-470 `BaseDiagramConfig.useMaxWidth`, default true; 61-63 `theme`, default `default`; 68-70 `look`, default `classic`; 443-445 `fontSize`, default 16; 305-322 `securityLevel`, default `strict`; 267-270 `htmlLabels`, no default. The bundle contains `leftMargin:150`. `defaultConfig.ts` has no timeline entry (line 24 is `htmlLabels: undefined`). |
| Theme variables          | `themes/theme-default.js` 36-44, 113, 136-139, 146-185; `themes/theme-redux-color.js` 19-41, 55, 448                                                                                                                                          | Section 5.3.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Common DB and sanitising | `diagrams/common/commonDb.ts` 4-32; `diagrams/common/common.ts` 8, 56-83                                                                                                                                                                      | Section 3.3.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Diagram pipeline         | `Diagram.ts` 19-85; `preprocess.ts` 6-91; `diagram-api/comments.ts` 6-8; `utils.ts` ~874-908 (`encodeEntities`, `decodeEntities`); `diagram-api/detectType.ts` 36-51                                                                          | Section 2.1.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Final SVG                | `mermaidAPI.ts` 327-333 (`createUserStyles`), 344-365 (`cleanUpSvgCode`: line 359 decodes entities over the whole string, line 362 `<br>` to `<br/>`), 596-600 (the `<style>` is inserted before `draw`), 620-623 and 750-758 (accessibility) | Sections 5.9, 5.11.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Viewbox and sizes        | `setupGraphViewbox.js` 23-72; `rendering-util/selectSvgElement.ts` 11-22; `utils.ts` 823-839 (`parseFontSize`)                                                                                                                                | Section 5.8.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Colour gate              | `diagrams/common/colorThemeGate.ts` 17, 20, 37-38, 86-96                                                                                                                                                                                      | Timeline colours by `.section-N` class; the slot stamping of 114-125 is not used by the timeline.                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Accessibility helpers    | `accessibility.ts` 27-32, 45-65                                                                                                                                                                                                               | Section 3.3.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |

## 2. Parser

### 2.1 Pipeline before the lexer

The steps run in this order for every diagram. Verified against the Chromium runs.

| Step               | Where                                                     | Effect on the timeline                                                                                                                                                                       |
| ------------------ | --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cleanupText`      | `preprocess.ts` 6-17                                      | CRLF or CR to LF. `<tag attr="x">` becomes `<tag attr='x'>`.                                                                                                                                 |
| Front matter       | `preprocess.ts` 19-31                                     | YAML block removed. `config` is merged (`theme`, `look` verified in 16 fixtures). `title` is read but never reaches the timeline DB (section 3.3).                                           |
| Directives         | `preprocess.ts` 33-45                                     | `%%{init: …}%%` removed and merged into the config (`timeline.disableMulticolor` and `themeVariables` verified).                                                                             |
| `cleanupComments`  | `comments.ts` 7: `^\s*%%(?!{)[^\n]+\n?`, then `trimStart` | Removes lines whose first non-blank characters are `%%` and at least one more character. A bare `%%` line stays for the lexer.                                                               |
| Detection          | `Diagram.ts` 31 → `detectType` 36-51                      | Runs on the cleaned text. `/^\s*timeline/` (case-sensitive): `Timeline` at the start is "no diagram type detected" (verified).                                                               |
| `db.clear()`       | `Diagram.ts` 49                                           | `timelineDb.clear` (section 2.4).                                                                                                                                                            |
| Front matter title | `Diagram.ts` 52-54                                        | `db.setDiagramTitle?.(…)`. The timeline DB has no such export, so the call does nothing (verified: front matter title is not drawn).                                                         |
| Entity encoding    | `Diagram.ts` 77: `encodeEntities(source) + '\n'`          | Every `#name;` becomes `ﬂ°name¶ß`, and every `#123;` becomes `ﬂ°°123¶ß`. Also `style…:…#…;` and `classDef…:…#…;` lose their last `;` (`utils.ts` 874-895). The parser sees the encoded text. |
| Parse              | `Diagram.ts` 83                                           | `parser.parse(text)`; the DB is the parser's `yy`.                                                                                                                                           |

Rendering later decodes the whole SVG string: `decodeEntities` (`utils.ts` 904-906: `ﬂ°°`→`&#`, `ﬂ°`→`&`, `¶ß`→`;`) runs in `cleanUpSvgCode` (`mermaidAPI.ts` 359). So `#quot;` is drawn as `"` and `#9829;` as ♥ (verified). The width used for wrapping is the width of the encoded text (`svgDraw.js` 515-516 measures `node.descr`, which is encoded). That width difference is from code reading and not measured.

### 2.2 Lexer rules

Jison keeps the first rule (in file order) that matches at the current position, not the longest. The generated rule list in the scratchpad (`timeline-parser.cjs`, `rules:`) shows the regexes Jison actually built. Two details matter:

- Jison adds `\b` after a rule that ends in a word character. The three `timeline` rules get it. `timeline TDx` is therefore `timeline` plus the period `TDx` (verified).
- The source `\%%(?!\{)[^\n]*` is generated as `%(?!\{)[^\n]*`: one `%` only. Verified: a line `%abc` is a comment.

| #   | Source line | Generated regex (`i` flag)        | Token and effect                                                                                           |
| --- | ----------- | --------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 0   | 14          | `%(?!\{)[^\n]*`                   | skipped (comment to end of line)                                                                           |
| 1   | 15          | `[^\}]%%[^\n]*`                   | skipped. Consumes the character before `%%` as well, so `x%%c` is a comment and `abc%%c` is not (verified) |
| 2   | 16          | `[\n]+`                           | `NEWLINE`                                                                                                  |
| 3   | 17          | `\s+`                             | skipped. Also swallows newlines after spaces, so no `NEWLINE` appears there                                |
| 4   | 18          | `\#[^\n]*`                        | skipped                                                                                                    |
| 5   | 20          | `timeline[ \t]+LR\b`              | `timeline_lr`                                                                                              |
| 6   | 21          | `timeline[ \t]+TD\b`              | `timeline_td`                                                                                              |
| 7   | 22          | `timeline\b`                      | `timeline`                                                                                                 |
| 8   | 23          | `title\s[^\n]+`                   | `title` (the token keeps `title`; the action cuts 6 characters)                                            |
| 9   | 24          | `accTitle\s*:\s*`                 | begins state `acc_title`, token `acc_title`. The `\s*` may take the next line (verified)                   |
| 10  | 25          | `[^\n]*` in `acc_title`           | pops to INITIAL, token `acc_title_value`                                                                   |
| 11  | 26          | `accDescr\s*:\s*`                 | token `acc_descr`                                                                                          |
| 12  | 27          | `[^\n]*` in `acc_descr`           | pops, token `acc_descr_value`                                                                              |
| 13  | 28          | `accDescr\s*\{\s*`                | begins `acc_descr_multiline`, no token                                                                     |
| 14  | 29          | `\}` in `acc_descr_multiline`     | pops, no token                                                                                             |
| 15  | 30          | `[^\}]*` in `acc_descr_multiline` | token `acc_descr_multiline_value`; may match the empty string                                              |
| 16  | 31          | `section\s[^:\n]+`                | `section` (the name keeps trailing blanks)                                                                 |
| 17  | 34          | `:\s(?:[^:\n]\|:(?!\s))+`         | `event`. The `\s` may be a newline (verified: `:` then newline then text)                                  |
| 18  | 35          | `[^#:\n]+`                        | `period`. Keeps trailing blanks and tabs                                                                   |
| 19  | 38          | `$` (`<<EOF>>`, INITIAL only)     | `EOF`                                                                                                      |
| 20  | 39          | `.`                               | `INVALID`, which the grammar does not accept                                                               |

Other facts about the lexer:

- Case-insensitive (`%options case-insensitive`, line 7). Verified: `TIMELINE`, `SECTION A`, `TITLE x`, `ACCTITLE: y` all parse.
- No state prefixes except the four `%x` states (lines 8-10). The `acc_title` and `acc_descr` rules at 9-12 are INITIAL rules and fire from INITIAL.
- Rule 17 (event) requires a colon followed by whitespace. A colon anywhere else is `INVALID` (rule 20) and a parse error (verified: `2004:x`, `t1:e1`, `2004 : ` at end of line).
- Keyword rules are the only place with `\b`. `section` and `title` have none, but `section\s…` and `title\s…` require a blank, so `sectionA` and `titleX` are periods.

Dead grammar: `line: SPACE statement` (grammar line 65) is unreachable, since no rule returns `SPACE`.

### 2.3 Grammar and DB calls

Actions run as each production reduces. The order is the input order.

| Production (line)                | Action                                                                                                 | DB effect (`timelineDb.js`)                                                                                                                                              |
| -------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `timeline_header` (53-57)        | `timeline_lr` → `setDirection('LR')`; `timeline_td` → `setDirection('TD')`; plain `timeline` → nothing | Direction stays as it was (LR after `clear`). Lines 21-23                                                                                                                |
| `title` (72)                     | `setDiagramTitle($1.substr(6))`                                                                        | Not the timeline DB: `commonDb.setDiagramTitle` = `sanitizeText` (commonDb 28-30). Last one wins                                                                         |
| `acc_title acc_title_value` (73) | `$2.trim()` then `setAccTitle`                                                                         | commonDb 16-18: sanitise, remove leading `\s`                                                                                                                            |
| `acc_descr acc_descr_value` (74) | `$2.trim()` then `setAccDescription`                                                                   | commonDb 22-24: sanitise, `\n\s+` to `\n`                                                                                                                                |
| `acc_descr_multiline_value` (75) | `$1.trim()` then `setAccDescription`                                                                   | as above                                                                                                                                                                 |
| `section` (76)                   | `addSection($1.substr(8))`                                                                             | `currentSection = txt; sections.push(txt)` (29-32). No trim                                                                                                              |
| `period_statement` (80-82)       | `addTask($1, 0, '')`                                                                                   | Creates a raw task (52-63): `id = currentTaskId++`, `section = type = currentSection`, `task = period`, `score = 0`, `events = []`                                       |
| `event_statement` (83-85)        | `addEvent($1.substr(2))`                                                                               | `rawTasks.find(t => t.id === currentTaskId - 1).events.push(event)` (65-70). No trim. Attaches to the last task created. With no task it throws a `TypeError` (verified) |

`substr(2)` on an event removes the colon and one character, which is the blank, or the newline in the `:`-newline form. `substr(8)` on a section removes `section` and one character. `substr(6)` on a title removes `title` and one character.

### 2.4 DB behaviour, detail

- `clear()` (12-19) resets everything except `currentTaskId`. The ids keep growing across renders in one JS realm. The ids are never drawn.
- `getTasks()` (38-50) pushes `rawTasks` into the module array `tasks` on every call. Calling it twice without `clear()` duplicates the tasks. The renderers call it once, so this is latent.
- `compileTasks()` (89-101) checks a `processed` flag that nothing sets. It does no work (the loop runs 101 times and changes nothing).
- The default export (103-114) has no `getAccTitle`, `getAccDescription` or `setDiagramTitle`. This is why the accessibility texts are not drawn (section 5.12, R7) and why the front matter title is ignored (R6).
- `addTaskOrg` (72-81) is unused by the grammar.

### 2.5 Observed parser results (Jison, DB stub)

Each row was run through the generated parser. "calls" is the DB call sequence.

| Input after preprocessing                                      | Result                                                                                                       |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `title` NEWLINE `X`                                            | title `X` (the `\s` of the title rule takes the newline)                                                     |
| `title ` (one trailing blank)                                  | not a title: period task `title `                                                                            |
| `title  ` (two blanks)                                         | title ` ` (a blank title is set)                                                                             |
| `section` NEWLINE `X`                                          | section `X`                                                                                                  |
| `section  ` (only blanks)                                      | section ` `                                                                                                  |
| `section A : x`                                                | section `A ` (trailing blank kept), then event `x` (fails at DB level if no task)                            |
| `2004 : a` NEWLINE `  : b`                                     | events `a` and `b` on task `2004 `                                                                           |
| `2004 :` NEWLINE `  x`                                         | event `  x` (the newline is the separator; the indentation stays)                                            |
| `2004 : ` at end of line                                       | parse error (`:` is INVALID)                                                                                 |
| `2004 : a : b`                                                 | events `a ` (trailing blank kept) and `b`                                                                    |
| `2004 : a:b`                                                   | event `a:b`                                                                                                  |
| `2004   `                                                      | period `2004   ` (not trimmed)                                                                               |
| `2004 # x`                                                     | period `2004 ` (comment after `#`)                                                                           |
| `period # hash`                                                | period `period `                                                                                             |
| `#`-led line                                                   | comment                                                                                                      |
| `abc%%c`                                                       | period `abc%%c`                                                                                              |
| `x%%c` (one character then `%%`)                               | whole line dropped                                                                                           |
| `%abc`, `%%abc`                                                | whole line dropped                                                                                           |
| `timeline TD2`, `timeline TDx`                                 | header `timeline` (LR), then a task `TD2` or `TDx`                                                           |
| `timeline LR:`                                                 | parse error                                                                                                  |
| first line `timelineX`                                         | parse error (no header)                                                                                      |
| `TIMELINE` / `SECTION A` / `TITLE x` / `ACCTITLE: y`           | parses (case-insensitive)                                                                                    |
| `accTitle:` NEWLINE `foo`                                      | accTitle `foo` (the value is on the next line)                                                               |
| `accTitle:` NEWLINE `  2004`                                   | accTitle `2004`; the task is swallowed                                                                       |
| `accTitle: ` or `accDescr: ` as the last line                  | parse error (verified in Chromium). The lexer returns `$end` where the grammar wants `EOF`                   |
| `accDescr {` NEWLINE `  line1` NEWLINE `  line2` NEWLINE `  }` | value `line1\n  line2` (the grammar trims the ends; the DB then gives `line1\nline2`, from commonDb 23)      |
| `accDescr {` block containing `# x` or `%% y`                  | kept as text (the block state has no comment rules)                                                          |
| `accDescr {` never closed                                      | parse error                                                                                                  |
| `accDescr {}`                                                  | nothing                                                                                                      |
| indented `%%{init: …}%%` in the body                           | comment (directives are removed earlier in the real pipeline)                                                |
| `section\tA`, `2004\t: a`                                      | section `A`; period `2004\t` (tab kept)                                                                      |
| blank lines between statements                                 | ignored; `NEWLINE` tokens with no effect                                                                     |
| event with no task before it (in the reference)                | `TypeError` from `addEvent` (verified in Chromium: "Cannot read properties of undefined (reading 'events')") |

### 2.6 Errors

| Source                                                                                   | Behaviour                                                                                                                                                                   |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lexer `INVALID` (only for a colon that does not start an event) and any grammar mismatch | Jison throws "Parse error on line N". Mermaid turns it into an error rendering; ferrigura returns an error. Jison's expectation list is not part of the contract (ADR 0002) |
| `addEvent` with no task                                                                  | `TypeError` from the DB, thrown during the parse (verified)                                                                                                                 |
| Empty `accTitle:`/`accDescr:` at the end of input                                        | parse error (verified)                                                                                                                                                      |
| Unterminated `accDescr {`                                                                | parse error (verified). No hang: the lexer returns `$end` once the input is empty                                                                                           |

### 2.7 Front matter, comments, directives: summary

- Front matter: `config` (theme, look, and every other key) applies. `title` is ignored by the timeline. Other keys (`description`, all fixtures) are ignored.
- Comments: `%%` lines removed by `cleanupComments`; `%` at the start of a token to end of line (lexer rule 0); `%%` after one character (rule 1); `#` to end of line (rule 4). Hash inside a title, a section or an event is text.
- Directives: `init`/`initialize` and `wrap` (`processDirectives`, preprocess 33-45). Only the `config` part reaches the timeline.

## 3. Canonical model proposal

### 3.1 Shape

```json
{
  "version": 1,
  "direction": "LR",
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "sections": ["Early Era", "Moon Era"],
  "tasks": [
    {
      "section": "Early Era",
      "task": "1957 ",
      "events": ["Sputnik 1 launched", "First artificial satellite"]
    }
  ]
}
```

- `direction`: `"LR"` or `"TD"`. `LR` is the default (timelineDb 4, 17).
- `sections`: creation order, duplicates kept, untrimmed (2.3). Duplicates matter for drawing (5.6).
- `tasks`: creation order. `section` is the section current when the task was created, or `""` before the first section. `task` is the period, untrimmed. `events` is the list in order, untrimmed, with the encoded text (see 3.2).
- Field names camelCase, as the other models (`accTitle`, `accDescr`).

### 3.2 Text rules

| Value                                  | Rule                                                                                                                                         | Source                                                                                    |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Section names, task names, event texts | Raw, after the entity encoding of `Diagram.ts` 77. No sanitisation, no trim. Keep the encoded form (`ﬂ°quot¶ß`) as the reference DB holds it | timelineDb.js (no sanitiser call); `flowchart/mod.rs` 92, 177 already encode the same way |
| Title, `accTitle`, `accDescr`          | Sanitised by `sanitizeText` with the configuration (3.3)                                                                                     | commonDb 8, 16-30                                                                         |
| Line breaks                            | CRLF to LF before everything (preprocess 6-17)                                                                                               |                                                                                           |
| Markup                                 | Raw. `<br>`, `<b>`, `<BR>` stay as written; drawing decides what they mean (5.4)                                                             |                                                                                           |

Decision for the coordinating session: keep the encoded text in the model, as the reference DB does, and decode when the SVG is written (`decode_entities` over the whole SVG, as `mermaidAPI.ts` 359 does). The wrapping then measures the encoded text, as the reference does. A decoded model would need the wrapping to measure a different string, which is a deviation.

### 3.3 Title, accTitle, accDescr

- **Title precedence (differs from the pie).** The timeline's title is the grammar's `title` line only. The front matter title is dropped (verified: `fm-title-only` has no title text, because `db.setDiagramTitle?.` finds nothing, `Diagram.ts` 52-54). The last `title` line wins, even when it is blank: `title  ` gives a title of one blank (verified: `fm-title-blank-title` draws a blank title over a front matter title). This contradicts the general rule in `canonical-model.md` 44-52, which must say that the timeline reads the grammar only.
- **Sanitisation.** `sanitizeText(text, config)` (common.ts 71-83): empty text is returned as is; with `htmlLabels` on (the default, `getEffectiveHtmlLabels` = `config.htmlLabels ?? true`, `config.ts` 324-326) `sanitizeMore` runs (common.ts 56-69). For `strict`, `antiscript` and `sandbox` it removes script tags (`removeScript`). Any other level except `loose` escapes `<`, `>` and `=` instead; no config value reaches that branch in practice. Then DOMPurify runs with `FORBID_TAGS: ['style']`. The default level is `strict` (schema 322).
- **DOMPurify fast path (correction).** DOMPurify 3.4.12 returns text without `<` unchanged (`purify.es.mjs` 2248-2251: `dirty.indexOf('<') === -1`). `canonical-model.md` line 27 says that `AT&T` becomes `AT&amp;T`, which is only true when the text contains markup. Verified in Chromium: the title `AT&T` shows as `AT&T`; the title `AT&T <b>bold</b>` shows as `AT&amp;T <b>bold</b>`, double-escaped. `sanitize.rs` 90 already has the fast path. The doc should be fixed.
- **Setters.** `setAccTitle` removes leading blanks after sanitising (commonDb 17). `setAccDescription` replaces `\n\s+` with `\n` after sanitising (commonDb 23).
- **Drawing.** Only the title is drawn. `accTitle` and `accDescr` are not drawn. The reference's `addA11yInfo` calls `diag.db.getAccTitle?.()` (mermaidAPI 621-622), which the timeline DB does not export, and the renderer's accessibility call is commented out (timelineRenderer 267). Verified: no `<title>` or `<desc>` element, no `aria-labelledby` or `aria-describedby`. The model keeps the two values (they are input content), but the renderer does not write them. Ferrigura must decide whether to write them (see section 7).

### 3.4 Left out, and why

| Left out                           | Reason                                                                                                           |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `id` of tasks                      | Global counter (timelineDb 3, 54), not reset by `clear`. Depends on what was rendered before in the same process |
| `type` of tasks                    | Always equal to `section` (timelineDb 56)                                                                        |
| `score` of tasks                   | Always 0 from the grammar (58). Nothing sets it                                                                  |
| `processed`, `getTasks` duplicates | Not input (2.4)                                                                                                  |
| Layout values                      | Rendering                                                                                                        |

## 4. Specs

### 4.1 `diagrams/timeline/timeline.spec.js`: 11 tests, all DB level

No test renders anything. Every test calls `timeline.parse(str)` with `timelineDB` as `yy`, then reads `getDirection`, `getSections`, `getTasks` or `commonDb.getDiagramTitle`. Most assertions sit inside `forEach` or `switch` blocks, so a parser that returns no tasks passes them (verified by reading; these tests have no count check).

| Line | Test                                         | Level | Pins                                                                                    |
| ---- | -------------------------------------------- | ----- | --------------------------------------------------------------------------------------- |
| 13   | default LR direction                         | DB    | `getDirection`                                                                          |
| 21   | TD direction                                 | DB    | `getDirection`                                                                          |
| 29   | LR direction                                 | DB    | `getDirection`                                                                          |
| 37   | simple section                               | DB    | `getSections` (exact)                                                                   |
| 45   | section and two tasks                        | DB    | tasks inside `forEach` (no count)                                                       |
| 57   | two sections, two tasks each                 | DB    | sections exact; tasks inside `forEach`                                                  |
| 78   | task and its events                          | DB    | events of `task1` and `task2` inside `forEach`/`switch`                                 |
| 102  | events with a markdown link                  | DB    | the link text kept raw                                                                  |
| 126  | multi-line events (`:` continuation)         | DB    | four events in order for `task2`                                                        |
| 151  | title, section, task, events with semicolons | DB    | title `;my;title;`; section; snapshot of events (`;ev;ent1; ` keeps its trailing blank) |
| 169  | title, section, task, events with hashtags   | DB    | title `#my#title#`; section; period `task1`; snapshot of events                         |

Port: 11 Rust tests with the same names, on the parser and the DB model. Assertions that the reference makes inside loops become exact checks in the port (a stronger test than the reference). The snapshots at 160 and 179 stay as exact values.

### 4.2 Other specs that touch the timeline

| Spec                                                                  | What it does for the timeline                                                                                                                             | Level    |
| --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| `rendering-util/multi-diagram-id-uniqueness.spec.ts` 149-152, 313-322 | Renders two timelines and checks `.task-line` ids. Vacuous: the timeline draws no `.task-line` element (verified, 0 in both SVGs), so the loop never runs | DOM      |
| `diagrams/paletteCssGeneration.spec.ts` 33, 39, 44-50                 | Timeline excluded from slot assertions; invalid-CSS checks still apply                                                                                    | CSS text |
| `diagrams/common/colorThemeGate.spec.ts` 20, 39, 43-46                | Same exclusion                                                                                                                                            | CSS text |
| `styles.spec.ts` 28, 103                                              | Imports `timeline/styles.js`                                                                                                                              | CSS text |
| `diagram-api/diagram-orchestration.spec.ts` 34                        | `timeline` detected as `timeline`                                                                                                                         | detector |
| `diagram-api/diagram-beta-policy.spec.ts` 37                          | Timeline listed in the policy list                                                                                                                        | config   |
| `themes/theme-redux-color-superset.spec.ts` 42                        | Comment: the timeline uses the categorical scale                                                                                                          | comment  |

### 4.3 Info specs (for section 8)

`diagrams/info/info.spec.ts`: 4 tests, all parser: two accept `info` and `info showInfo`, two check the Langium error text of `info unsupported` (the two rejection tests are identical). No renderer spec exists.

### 4.4 Parser tests in `packages/parser/tests/`

None for timeline (the directory has `info.test.ts`, not a timeline file).

## 5. Renderer

### 5.1 Entry and dispatch

```
draw(text, id, version, diagObj):                       // both renderers
  conf = getConfig()
  LEFT = conf.timeline.leftMargin ?? 50                 // 150 in effect (schema default, bundle "leftMargin:150")
  tasks  = db.getTasks()                                // once
  title  = commonDb.getDiagramTitle()
  sections = db.getSections()
  svg = select('#' + id)                                // horizontal (timelineRenderer 49)
        or selectSvgElement(id)                         // vertical (timelineRendererVertical 48)
  svg.append('g')                                       // an empty group (51 / 50)
  initGraphics(svg[, id])                               // marker, see R1
  …layout…
  setupGraphViewbox(svg, padding = conf.timeline.padding ?? 50, useMaxWidth = conf.timeline.useMaxWidth ?? false)
```

`conf.timeline.useMaxWidth` is `true` in practice, because the schema default applies (the `?? false` never runs). Verified: default timelines have `width="100%"` and `style="max-width: …px;"`.

### 5.2 Configuration read by the timeline

| Key                                                           | Read at                                      | Effective default      | Verified                                         |
| ------------------------------------------------------------- | -------------------------------------------- | ---------------------- | ------------------------------------------------ |
| `timeline.leftMargin`                                         | 34 / 44                                      | 150                    | left edge at x=200 (50+150)                      |
| `timeline.padding`                                            | 263 / 243                                    | none, so 50            | viewBox x = 150-50                               |
| `timeline.useMaxWidth`                                        | 264 / 244                                    | true (schema via base) | `width="100%"`                                   |
| `timeline.disableMulticolor`                                  | 334 / 292                                    | false                  | pr-check/4: only `section--1` used               |
| `fontSize` (top level)                                        | svgDraw 523, 581; vertical 222               | 16 (number)            | heights use 8.8                                  |
| `look`                                                        | timelineRenderer 213, 234; svgDraw 539       | classic                | `data-look="neo"`, title position, gradient defs |
| `theme`                                                       | svgDraw 500, 588, 601; styles 151-152        | `default`              | redux branch, see 5.3                            |
| `themeVariables.useGradient`, `gradientStart`, `gradientStop` | timelineRenderer 33, 234-257; styles 157-173 | redux: false           | gradients verified in `lr-gradient`              |
| `themeVariables.THEME_COLOR_LIMIT`                            | svgDraw 501; styles 96, 159, 24              | 12                     | `section-K` wraps every 12                       |
| `securityLevel`, `htmlLabels`, `dompurifyConfig`              | commonDb sanitise                            | strict, true, none     | section 3.3                                      |

### 5.3 Theme variables, and the branch that matters

`theme.includes('redux')` selects the redux branch in four places (svgDraw 500, 588, 601; styles 151). Only the theme name matters, not the values. The two branches differ in geometry:

| Item                                  | Redux branch (`redux-color`, `redux-dark-color`, `redux`, `redux-dark`)    | Classic branch (`default`, `base`, `dark`, `forest`, `neutral`, `neo`, `neo-dark`, no theme) |
| ------------------------------------- | -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Corner radius `r` of the node path    | 0 (svgDraw 588)                                                            | 5                                                                                            |
| Node path form                        | straight segments                                                          | with `q` corners                                                                             |
| `node-line-K` element under each node | absent (601)                                                               | present, from (0,h) to (W,h)                                                                 |
| Text offset y, tasks and sections     | padding, 20                                                                | padding/2, 10                                                                                |
| Text offset y, events                 | padding/2 + 3, 13 (svgDraw 528-534)                                        | padding/2, 10                                                                                |
| Drop-shadow filter                    | created in the root `defs` once, `#ID-drop-shadow` (546-562)               | not created                                                                                  |
| Section fills                         | `borderColorArray[(K+1) % len]`, fill `mainBkg` in dark (styles 31-35, 41) | `cScale(K+1)` (styles 113)                                                                   |

Theme variables used by the timeline:

| Variable                                             | Used at                                                                                          | Source of values                                                                          |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| `cScale0`..`cScale11`                                | styles 113 (fill), 122-124 (section edge), 125-127 (`edge-depth` width is `17-3i`, not a colour) | classic: `theme-default.js` 146-157 (`adjust` on primary, hues 0 to 330), darken loop 162 |
| `cScaleInv0`..`cScaleInv11`                          | styles 128-131 (line stroke, width 3)                                                            | `theme-default.js` 167                                                                    |
| `cScaleLabel0`..`cScaleLabel11`                      | styles 116 (section text), 118-121 (icon colour), 133-135 (`.lineWrapper line`, last rule wins)  | `theme-default.js` 182-185                                                                |
| `lineColorN`                                         | set in styles 98-105, not read by the rules                                                      | derived from `cScaleInv`                                                                  |
| `borderColorArray`                                   | redux styles 31-35                                                                               | `theme-redux-color.js` 55                                                                 |
| `mainBkg`, `nodeBorder`, `strokeWidth`, `fontWeight` | styles 41, 43, 48-49, 71-72                                                                      | default 1 / 2 / `normal`; redux `fontWeight` 600 (`theme-redux-color.js` 448)             |
| `tertiaryColor`, `clusterBorder`                     | styles 78, 82, 138, 141 (disabled state)                                                         | `theme-default.js` and `theme-redux-color.js` 37, 41                                      |
| `git0`, `gitBranchLabel0`                            | styles 183-186 (`.section-root`, unused by the timeline)                                         |                                                                                           |
| `useGradient`, `gradientStart`, `gradientStop`       | timelineRenderer 233-257; styles 157-173                                                         |                                                                                           |
| `dropShadow`                                         | not read by the timeline (the drop shadow is a filter element, not a variable)                   |                                                                                           |
| `THEME_COLOR_LIMIT`                                  | styles 24, 96, 159; svgDraw 501                                                                  | 12 in both themes                                                                         |

Consequence: colours need the classic theme's derived values (`cScale*`, `cScaleInv*`, `cScaleLabel*`), which M2 does not port (M2 ports the two redux themes). Geometry needs only the branch.

### 5.4 Text measurement and wrapping

Text is measured with the DOM (`getComputedTextLength`, `getBBox`). The stylesheet is in the DOM before drawing (mermaidAPI 596-600), so the measured font is the one the CSS selects (`font-family` from the common rules, `font-size` 16px, and `font-weight` of `.section-N text`, which is 600 in redux). The title's `font-size` is `4ex` (timelineRenderer 214).

`wrap(text, width)` (svgDraw 456-496), in pseudo-code:

```
words = split(text, /(\s+|<br>)/)            // separators are kept as tokens; a trailing "" appears
line = []; tspan = first tspan (dy = 1em, x = 0)
for w in words (original order):
    line.push(w)
    tspan.text = join(line, ' ').trim()     // join inserts a space between every token
    if measure(tspan) > width or w == '<br>':
        line.pop()
        tspan.text = join(line, ' ').trim()
        line = (w == '<br>') ? [''] : [w]
        tspan = new tspan (dy = 1.1em, x = 0), text = w (not trimmed)
```

`measure(tspan)` is `getComputedTextLength` of one tspan. It collapses runs of blanks (verified: `a   b` and `a b` both give 22.14 px; leading and trailing blanks count as nothing). So the measured width is that of the collapsed string, and it is one run per line, not a sum of words.

Observed rules (verified in Chromium unless marked):

| Case                                  | Result                                                                                                                 |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Break test                            | a line breaks before the word that makes it wider than `width` (verified on two texts at width 150 and 300)            |
| Width                                 | horizontal tasks and events 150; sections `200·max(n,1) - 50`; vertical tasks 200, events 300, sections 580 (5.6, 5.7) |
| Lowercase `<br>`                      | forced break; the tokens after it start a new line                                                                     |
| `<br/>`, `<br />`, `<BR>`             | literal text (the regex is case-sensitive and has no slash). `<br />` splits as `four<br` and `/>five` at the blank    |
| Trailing `<br>`                       | the last line is empty (token `""`)                                                                                    |
| Leading `<br>` or overlong first word | an empty first line; the text starts one line lower (verified: tspans `["", "abc"]`)                                   |
| Overlong word                         | not split. It takes a line of its own, wider than `width` (verified: 684 px for a 150 px width)                        |
| Empty tspans in `getBBox`             | they add no height (verified: 2 tspans, one empty, height 19)                                                          |

Text box height, verified at 16px: one non-empty line is 19 px; each further line adds 17.6 px (`1.1em`). For 4 lines the height is 71.8; for 6 lines 107; for 7 lines 124.6; for 3 vertical lines 54.2. In general the height is the union of the boxes of the non-empty lines, so it is `19 + 17.6·(k-1)` for `k` contiguous non-empty lines (font-specific: 19 is the box for the reference font, so the port must use `text_box(face, 16)` of `ferrigura-text`).

Test vector, horizontal width 150, reference font (verified widths): `"Britain   is   a   long"` 111.875; `"event   text   with"` 111; `"many   words   that"` 119.906; `"will   wrap   over"` 102.359; `"several   rows   of   text"` 142.688; `"in   the   horizontal"` 119.266; `"layout"` 44.688. Vertical width 300: `"Britain   is   a   long   event   text   with   many"` 270.675; `"words   that   will   wrap   over   several   rows   of"` 297.019; `"text   in   the   vertical   layout   and   more"` 257.612. These widths belong to the font Chromium resolved on this machine, so they are reference-font vectors, not targets for the harness font.

Ferrigura's existing `wrap_line` (`ferrigura-text/src/wrap.rs`) is the label algorithm (`splitLineToFitWidth`, one tspan per word with a leading space). It is not the same algorithm. The timeline needs its own wrap in `ferrigura-render`, reusing `measure` and `layout_width` (`measure.rs` 30-40). Note that `layout_width` snaps each run once, and the timeline measures one run per line.

Other measurement facts: the title uses `font-size: 4ex`. Ferrigura's `FontMetrics` has no x-height (`metrics.rs` 13-18), so `4ex` needs a new metric. Verified only that the title exists at `4ex`; the x-height used by Chromium is unverified.

### 5.5 Node geometry

```
drawNode(elem, node, fullSection, pad, width w, maxH, isEvent):
  section = (fullSection % N) - 1                  // N = THEME_COLOR_LIMIT ?? 12   (svgDraw 501-502)
  text    = wrap(node.descr, w)                    // width w, not w + 2·pad      (svgDraw 521)
  tb      = getBBox(text).height                   // see 5.4                      (522)
  h       = max(tb + fontSize·1.1·0.5 + pad, maxH)  // fontSize·0.55 = 8.8 at 16px  (524-525)
  W       = w + 2·pad                              // (526)
  text at translate(W/2, redux ? (isEvent ? pad/2 + 3 : pad) : pad/2)   // (528-534)
  path    (5.5.1), classic: line node-line-section from (0,h) to (W,h)  // (601-609)
```

`getVirtualNodeHeight` (svgDraw 570-584) is the same measurement without the `maxH` clamp and without the drawing. It removes its temporary text afterwards.

Node path (`defaultBkg`, 592-595), with `r = 0` for redux and `5` otherwise, and `rd = 5`:

```
r > 0:  M0 {h-rd} v{-h+2rd} q0,-{r},{r},-{r} h{W-2rd} q{r},0,{r},{r} v{h-rd} H0 Z
r = 0:  M0 {h-rd} v{-(h-rd)} h{W} v{h} H0 Z
```

Verified for one task (h 67.8, W 190, classic): `M0 62.8 v-57.8 q0,-5,5,-5 h180 q5,0,5,5 v62.8 H0 Z`.

Node classes: `node-bkg node-undefined` on the path, because `node.type` is never set (svgDraw 599 reads `node.type`; the callers never pass it). Verified in all 41 renders. The group is `timeline-node section-K` (svgDraw 505-507), where `K = (fullSection % N) - 1`, so section 0 gets `section--1`.

### 5.6 Horizontal layout (timelineRenderer.ts)

Constants: `LEFT` (5.2). Pre-pass maxima:

```
maxSectionH = max over sections s: virt(s, w=150, pad=20) + 20          // 82-94; the +20 is at 93
maxTaskH    = max over tasks t:    virt(t.task, w=150, pad=20) + 20     // 103-114
maxEventLen = max over tasks t:    Σ_e virt(e, w=150, pad=20) + 10·(|events|-1)   // 119-135, virt has no maxH clamp
```

Sections exist (`sections.length > 0`, 141):

```
masterX = 50 + LEFT          // 200                                   (73)
sectionBeginY = 50                                                    (76)
for (k, s) in sections (in order):                                    (142)
    tasksFor = tasks.filter(t => t.section === s)                     (144)   // drops tasks before the first section; repeats duplicates
    header: node(descr s, w = 200·max(|tasksFor|,1) - 50, pad 20, maxH = maxSectionH, colour k)
            at translate(masterX, 50)                                 (155-159)  // plain <g> wrapper
    masterY = 50 + maxSectionH + 50                                   (161)   // 167.8 for one-line sections
    if tasksFor: drawTasks(tasksFor, colour k, x = masterX, y = masterY, maxTaskH)   (165-178)
    masterX += 200·max(|tasksFor|,1)                                  (181)
    masterY = 50                                                      (183)
```

No sections (`else`, 186-203): `drawTasks(tasks, colour 0, x = masterX, y = 50, maxTaskH)`, and the colour increments per task (unless `disableMulticolor`).

`drawTasks(tasks, colour, x, y, maxH)` (270-341):

```
for t in tasks:
    node(t.task, w = 150, pad 20, maxH = maxH) at (x, y)              // taskWrapper    (290-304)
    maxH = max(maxH, h)                                               // (307)
    drawEvents(t.events, x, y + 100)                                  // (315-318): events start at y + 200
    dashed line, ALWAYS (the test is `if (task.events)`, always true) // (310-330)
        x1 = x + 95, y1 = y + maxH, x2 = x + 95, y2 = y + maxH + 100 + maxEventLen + 100
        stroke-width 2, stroke black, dasharray "5,5", marker-end url(#ID-arrowhead)
    x += 200                                                          // (333)
    if no sections and not disableMulticolor: colour += 1             // (334-336)
```

`drawEvents(events, x, y)` (343-380) starts at `y + 100` (its `masterY + 100` at 354), so the first event is at `y + 200`. Each event is `node(e, w = 150, pad 20, maxH = 50)`, with `h = max(·, 50)`, and the next event starts `h + 10` lower (375). Events are on the task's `x`.

Axis and title (after all nodes, 206-231):

```
box = getBBox(svg)                     // before the title and the axis (206)
depthY = sections ? maxSectionH + maxTaskH + 150 : maxTaskH + 100      // (219)
title (if non-empty): text at x = (neo) box.x·2 + LEFT : box.width/2 - LEFT,
                      y = 20, font-size 4ex, font-weight bold          // (209-217)
axis: line x1 = LEFT, y1 = depthY, x2 = box.width + 3·LEFT, y2 = depthY,
      stroke-width 4, stroke black, marker-end url(#ID-arrowhead)      // (221-231)
```

Verified (pr-check/1, default look, LEFT 150): tasks at x 200, 400, 600, 800 and y 50; `maxTaskH` 67.8; events at y 250 and 310 (Google, the second event, is 60 lower: 50 + h 50 + 10); dashed lines at x 295, 495, 695, 895 from y 117.8 to 423.4 (`maxEventLen` 105.6); axis at y 167.8 (`67.8 + 100`), x 150 to 1240 (box width 790 + 450); title at x 245 (`790/2 - 150`), y 20.

Verified (section fixture, `simple-lr-section`): section header at y 50, tasks at y 167.8 (`50 + 67.8 + 50`), event of the second task at y 367.8 (`167.8 + 200`).

Verified (redux, neo): title at x 550 (`200·2 + 150`).

### 5.7 Vertical layout (timelineRendererVertical.ts)

Constants (29-39): `NODE_WIDTH` 200, `NODE_PADDING` 5, `NODE_TOTAL_WIDTH` 210, `EVENT_WIDTH` 300, `EVENT_TOTAL_WIDTH` 310, `EVENT_SPACING` 10, `EVENT_VERTICAL_GAP` 0, `SECTION_TASK_GAP` 20, `TASK_AXIS_GAP` 20, `TASK_VERTICAL_GAP` 30, `EVENT_AXIS_GAP` 50.

```
leftWidth  = 210 + 20 = 230                                            (73)
rightWidth = 310 + 50 = 360                                            (74)
axisX      = (LEFT + 50) + leftWidth                                  (75)
timelineX  = hasSections ? axisX : masterX + leftWidth    // both = LEFT + 280    (78)
sectionWidth = max(50, leftWidth + rightWidth - 10) = 580              (80)
maxSectionH = max over s: virt(s, w = 580, pad 5)                      (82-94; no +20)
maxTaskH    = max over t: virt(t.task, w = 200, pad 5)                 (100-111)
maxEventStack = max over t: Σ_e virt(e, w = 300, pad 5) + 10·(|events|-1)   (115-130)
taskBlockH = max(maxTaskH, maxEventStack); taskSpacing = taskBlockH + 30 (136-137)
```

Sections (139-184), with `masterY` starting at 50:

```
for (k, s) in sections:
    tasksFor = filter
    header: node(s, w = 580, pad 5, maxH = maxSectionH) at (timelineX - 230, masterY)   (154-158)
    taskStartY = masterY + node.h + 20                                  (160)
    drawTasks(tasksFor, colour k, x = timelineX, y = taskStartY, spacing = taskSpacing)  (163-173)
    sectionH = node.h + 20 + taskSpacing·max(n,1) - (n > 0 ? 60 : 0)   (177-181)
    masterY += sectionH                                                 (182)
```

No sections: `drawTasks(tasks, 0, timelineX, 50, taskSpacing)` (187-197).

`drawTasks` (248-296): each task is a node of width 200 with `maxH = maxTaskH`. Its x is `timelineX - 20 - node.w`, so 200 for `timelineX` 430 (278). Events, when present (285): `drawEvents(events, axisX = timelineX, eventsX = timelineX + 50, startY = y)`. Then `y += taskSpacing` (291) and the colour increments unless disabled.

`drawEvents` (298-344): for each event, `node(e, w = 300, pad 5, maxH = 0)` at `(eventsX, cy)`, `lineY = cy + h/2`, a dashed line from `(axisX, lineY)` to `(eventsX, lineY)` with `marker-end url(#arrowhead)` (330-339), then `cy += h + 10` (341).

Axis and title (201-245):

```
box = getBBox(svg)                                                     (201)
title (if non-empty): x = box.width/2 - LEFT, y 20, font-size 4ex, bold   (207-214)   // no neo branch
box = getBBox(svg)  again                                              (215)
f = fontSize (16); top = 2f = 32; bottom = 0.5f + 20 = 28              (222-224)
axis: x1 = x2 = timelineX; y1 = 50 - 32 = 18; y2 = box.y + box.h + 28; stroke-width 4; stroke black;
      marker-end url(#arrowhead)                                       (226-236)
lineWrapper.lower()                                                    (237)   // moved to the front
viewBox as in 5.8                                                      (240-245)
```

Verified (`simple-td`, no sections, `TD`): axis at x 430, from y 18 to 259.2; tasks at x 200, y 50 and 155.6 (`50 + 105.6`, `taskSpacing = 75.6 + 30`); events at x 480; the second event of the second task at y 198.4 (`155.6 + 32.8 + 10`); event node height 32.8 (no 50 minimum). Verified (`td-section`): first section header at y 50, its task at y 102.8 (`50 + 32.8 + 20`).

Vertical has no gradient defs (see R12).

### 5.8 Viewbox

`setupGraphViewbox(undefined, svg, padding, useMaxWidth)` (setupGraphViewbox.js 49-72):

```
b = getBBox(svg)
width  = b.width  + 2·padding
height = b.height + 2·padding
if useMaxWidth: width="100%", style="max-width: {width}px;"       (23-33, no height attribute)
else:           height="{height}", width="{width}"
viewBox = "{b.x - padding} {b.y - padding} {b.width + 2·padding} {b.height + 2·padding}"
```

Verified: pr-check/1 has `viewBox="100 -61 1190 534.4000244140625"`, `style="max-width: 1190px;"`, `width="100%"`. The bounding box is the union of the drawn geometry (path boxes, text boxes, line extents). It does not include `<style>`, `<defs>` or markers.

### 5.9 SVG structure and classes

Verified skeleton (horizontal, classic, from the Chromium output of `simple-lr`, abbreviated):

```
<svg id="ID" width="100%" xmlns="http://www.w3.org/2000/svg" style="max-width: Wpx;"
     viewBox="x y w h" role="graphics-document document" aria-roledescription="timeline">
  <style>…</style>                                  (inserted by mermaidAPI 596-600, before the drawing)
  <g></g>                                           (core)
  <g></g>                                           (timelineRenderer 51)
  <defs><marker id="ID-arrowhead" refX="5" refY="2" markerWidth="6" markerHeight="4" orient="auto">
          <path d="M 0,0 V 4 L6,2 Z"></path></marker>
        [<filter id="ID-drop-shadow" …>] [<linearGradient id="ID-gradient" …>]</defs>
  <g transform="translate(200, 50)">                                  (section header wrapper, plain g)
    <g class="timeline-node section--1">
      <g><path id="ID-node-0" class="node-bkg node-undefined" d="…"/>
         <line class="node-line--1" x1="0" y1="h" x2="W" y2="h"/></g>   (classic only)
      <g transform="translate(W/2, pad/2)"><text dy="1em" alignment-baseline="middle"
          dominant-baseline="middle" text-anchor="middle"><tspan x="0" dy="1em">…</tspan>…</text></g>
    </g>
  </g>
  <g class="taskWrapper" transform="translate(200, 167.8)">…node…</g>
  <g class="lineWrapper"><line x1 y1 x2 y2 stroke-width="2" stroke="black" marker-end="url(#ID-arrowhead)" stroke-dasharray="5,5"/></g>
  <g class="eventWrapper" transform="translate(200, 367.8)">…node…</g>
  <text x="245" font-size="4ex" font-weight="bold" y="20">…</text>         (title, before the axis)
  <g class="lineWrapper"><line x1 y1 x2 y2 stroke-width="4" stroke="black" marker-end="…"/></g>   (axis)
</svg>
```

Notes:

- The first `<g></g>` before `defs` is not the timeline's; the timeline adds the second (verified: `info` has one).
- Section headers are plain `<g transform>` wrappers. Tasks are `g.taskWrapper`, events `g.eventWrapper`, dashed event lines and the axis `g.lineWrapper`.
- The vertical renderer writes its axis `lineWrapper` first (`lower()`), and each event has its own `lineWrapper` group.
- Text is not sanitised or escaped beyond the DOM: `.text(node.descr)` sets text content, so `&` is written as `&amp;` in the output and displayed as `&`.
- Titles use `.text(title)`, so their `&` is escaped once, but a title that already contains `&amp;` is escaped twice (R13).

### 5.10 Number formatting

Coordinates are JavaScript doubles written with the shortest round-trip form, and the reference's arithmetic gives values such as `423.40000000000003` and `198.39999999999998` (verified). A geometry comparison with tolerance (the harness uses 0.05 px) does not need the same bit pattern, but a text comparison would. Keep the same order of operations where it is cheap.

### 5.11 CSS

From `styles.js` (values generated per theme; `i` runs over `THEME_COLOR_LIMIT`):

| Rule family                                                                   | Lines                                        | Effect                                                                                                                                                     |
| ----------------------------------------------------------------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.edge { stroke-width: 3 }`, `.edge { fill: none }`                           | 177-179, 194-196                             | Edge defaults (no edges in timelines)                                                                                                                      |
| `.section-(i-1)` `rect`, `path`, `circle`: fill, stroke, stroke-width, filter | 37-45 (redux)                                | Node colours; the filter is `url(#ID-drop-shadow)`                                                                                                         |
| `.section-(i-1)` `rect`, `path`, `circle`: fill                               | 107-114 (classic)                            | `cScale(i)`                                                                                                                                                |
| `.section-(i-1) text`                                                         | 47-50 (redux), 115-117 (classic)             | Text fill; redux adds `font-weight` `fontWeight`                                                                                                           |
| `.node-icon-(i-1)`                                                            | 52-55, 118-121                               | Unused by the timeline                                                                                                                                     |
| `.section-edge-(i-1)`                                                         | 57-59, 122-124                               | Unused                                                                                                                                                     |
| `.edge-depth-(i-1)`                                                           | 61-63, 125-127                               | Unused by geometry (stroke width `17-3i`)                                                                                                                  |
| `.section-(i-1) line`                                                         | 65-68, 128-131                               | Stroke `cScaleInv(i)`, width 3 (the node-line)                                                                                                             |
| `.lineWrapper line`                                                           | 70-73 (redux: nodeBorder), 133-135 (classic) | Axis and dashed line colour. The classic rules repeat for all `i`, so the last one (`cScaleLabel11`) wins, and it overrides the `stroke="black"` attribute |
| `.disabled …`                                                                 | 75-83, 137-142                               | Not used by the timeline                                                                                                                                   |
| gradient rules                                                                | 160-173                                      | Only for `useGradient`, neo, not neutral, `THEME_COLOR_LIMIT` set. Use `url(#ID-gradient)`                                                                 |
| `.section-root …`                                                             | 182-186                                      | Unused                                                                                                                                                     |
| `.icon-container`                                                             | 188-193                                      | Unused                                                                                                                                                     |
| `.eventWrapper { filter: brightness(120%) }`                                  | 197-199                                      | Event groups are brightened (visual only)                                                                                                                  |

Redux: `filter: url(#ID-drop-shadow)` on every section, and the `feDropShadow` values depend on dark or light (svgDraw 554-562).

Verified cascade detail: `.lineWrapper line` has twelve rules in the classic theme, and the last one sets black for the default theme (the `#ffffff` rule comes first). The CSS wins over the `stroke="black"` attribute.

### 5.12 Reference defects, and what the port should do

| #   | Defect                                                                                                                                                                                                                 | Evidence                                                                                                                                                             | Options                                                                                   |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| R1  | Vertical renderer: marker id is `undefined-arrowhead` (`initGraphics` is called without the id, vertical 60; svgDraw 442), but the references use `url(#arrowhead)` (vertical 236, 338). No arrowheads on TD timelines | Verified: marker id and the `marker-end` values in `simple-td` and `td-section`. Absence of the arrowhead follows from the SVG rules (not inspected in a screenshot) | Keep for exact output, or fix (recommended: fix, and record the exception in the harness) |
| R2  | Vertical nodes: `drawNode` without diagram id gives `undefined-node-N` ids (vertical 154, 274, 324; svgDraw 598), which repeat across diagrams on one page                                                             | Verified: `undefined-node-0`… in both TD fixtures                                                                                                                    | Same as R1                                                                                |
| R3  | Horizontal: every task gets its dashed line, even with no events (`if (task.events)`, horizontal 310)                                                                                                                  | Verified: dashed lines equal the task count (pr-check/1: 4 tasks, 4 lines; `simple-lr-section`: 2 and 2)                                                             | Keep (it is visible output)                                                               |
| R4  | Tasks before the first `section` are not drawn, in both renderers (timelineRenderer 144; vertical 142)                                                                                                                 | Verified: `tasks-before-section` and `td-tasks-before-section` lose `t0`                                                                                             | Keep, document                                                                            |
| R5  | Duplicate section names draw their tasks under every occurrence                                                                                                                                                        | Verified: `duplicate-sections` gives 5 task nodes from 3 sections                                                                                                    | Keep                                                                                      |
| R6  | Front matter `title` ignored (`Diagram.ts` 52-54 finds no setter)                                                                                                                                                      | Verified: `fm-title-only`                                                                                                                                            | Keep, document (differs from pie)                                                         |
| R7  | `accTitle`, `accDescr` not drawn (mermaidAPI 621-622; timeline 267 commented)                                                                                                                                          | Verified: no `<title>`, `<desc>`                                                                                                                                     | Keep; the model keeps the values                                                          |
| R8  | Only lowercase `<br>` breaks lines (svgDraw 461)                                                                                                                                                                       | Verified: `<br/>`, `<br />`, `<BR>` literal                                                                                                                          | Keep                                                                                      |
| R9  | Trailing and leading `<br>` make empty lines                                                                                                                                                                           | Verified (`trail-br`, `lead-br`)                                                                                                                                     | Keep                                                                                      |
| R10 | Overlong words are not split                                                                                                                                                                                           | Verified: 684 px wide                                                                                                                                                | Keep                                                                                      |
| R11 | `#name;` entities are displayed decoded but measured encoded                                                                                                                                                           | Verified display (`#quot;` → `"`); the measurement difference is from code (svgDraw 515-516)                                                                         | Keep (measure encoded)                                                                    |
| R12 | Vertical with `useGradient`: the CSS refers to `url(#ID-gradient)`, which is never defined (no defs in the vertical renderer)                                                                                          | Verified: `td-gradient` has `url(#d1-gradient)` and no `linearGradient`                                                                                              | Fix or keep. Keep means the nodes have no stroke                                          |
| R13 | Titles with markup are escaped twice (`title-amp`: `AT&amp;amp;T &lt;b&gt;`), because DOMPurify output is set as text                                                                                                  | Verified                                                                                                                                                             | Keep or fix. The fix changes sanitising semantics                                         |
| R14 | Capitalised `Timeline` is not detected (`/^\s*timeline/` has no `i`)                                                                                                                                                   | Verified: "no diagram type detected"                                                                                                                                 | Keep (same text, same result)                                                             |
| R15 | Event with no task: `TypeError`                                                                                                                                                                                        | Verified                                                                                                                                                             | Error (same as the reference)                                                             |
| R16 | Empty `accTitle:` or `accDescr:` as the last line: parse error                                                                                                                                                         | Verified                                                                                                                                                             | Keep                                                                                      |
| R17 | `accTitle:` takes the next line as its value                                                                                                                                                                           | Verified at DB level (Jison)                                                                                                                                         | Keep                                                                                      |
| R18 | Section names keep trailing blanks (`A `)                                                                                                                                                                              | Verified at DB level; drawn name collapses the blank                                                                                                                 | Keep                                                                                      |
| R19 | `currentTaskId` not reset (ids grow across renders)                                                                                                                                                                    | From code (timelineDb 3, 13-18, 54). Not visible in SVG                                                                                                              | Do not model `id`                                                                         |
| R20 | `getTasks()` is not idempotent                                                                                                                                                                                         | From code (47)                                                                                                                                                       | Not reachable from the renderers                                                          |
| R21 | Classic and redux branches are chosen by theme name, not by look                                                                                                                                                       | Verified (5.3). Affects every theme that is not shipped                                                                                                              | Decision needed (section 7)                                                               |
| R22 | The multi-diagram id test passes vacuously for timelines                                                                                                                                                               | Verified (no `.task-line`)                                                                                                                                           | Add a timeline check to our harness                                                       |

### 5.13 Verified numbers, for the first test

From `pr-check/1` (default theme, classic look, LR), the reference output:

| Item              | Value                                                                                                        |
| ----------------- | ------------------------------------------------------------------------------------------------------------ |
| Tasks (translate) | (200,50), (400,50), (600,50), (800,50)                                                                       |
| Task box          | 190 × 67.8                                                                                                   |
| Events            | (200,250) LinkedIn; (400,250) Facebook; (400,310) Google; (600,250) YouTube; (800,250) Twitter; box 190 × 50 |
| Dashed lines      | x 295, 495, 695, 895; y 117.8 to 423.4; `marker-end` on each                                                 |
| Axis              | y 167.8, x 150 to 1240, stroke-width 4                                                                       |
| Title             | x 245, y 20, `font-size="4ex"`, `font-weight="bold"`                                                         |
| viewBox           | `100 -61 1190 534.4000244140625`; `style="max-width: 1190px;"`; `width="100%"`                               |
| Task path         | `M0 62.8 v-57.8 q0,-5,5,-5 h180 q5,0,5,5 v62.8 H0 Z`                                                         |
| Task text         | `translate(95, 10)`, `<tspan x="0" dy="1em">2002</tspan>`                                                    |

## 6. Fixtures

### 6.1 Count and location

- 30 `.mmd` fixtures: `e2e/diagrams/timeline/pr-check/` (10) and `nightly-check/` (20). `visual-check/` is empty. `pr-check/.gitkeep` is not a fixture.
- All 30 have a title line and a YAML front matter block (`description`, and `config` in 16).
- `info`: `e2e/diagrams/info/pr-check/` has 2 fixtures (`info` and `info showInfo`).

### 6.2 Features

| Feature                        | Fixtures                                                                                                                                                                                             |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Horizontal, no sections        | pr-check 1, 4, 5, 6, 7, 8; nightly 9, 10 (8)                                                                                                                                                         |
| Horizontal, sections           | pr-check 2, 3, 12; nightly 11; the 16 `space-exploration-*` (20 in all)                                                                                                                              |
| Vertical (`timeline TD`)       | pr-check 14 (sections, `<br>`), nightly 13 (no sections). Only two, so the vertical renderer has very little coverage                                                                                |
| Event continuation lines (`:`) | pr-check 3, 12; nightly 11; the 16 `space-exploration-*`                                                                                                                                             |
| `<br>` inside text             | pr-check 2, 3, 14                                                                                                                                                                                    |
| `%%{init}%%` directive         | 7: pr-check 4 (`base`, `disableMulticolor: true`), 5 (`default`, `themeVariables` `cScale0`-`cScale2` red, green, blue), 6 (`base`), 7 (`default`), 8 (`dark`); nightly 9 (`neutral`), 10 (`forest`) |
| Front matter `config`          | 16 `space-exploration-*` (`theme` and `look`)                                                                                                                                                        |
| Plain (no directive)           | pr-check 1, 2, 3, 12, 14; nightly 11, 13 (7)                                                                                                                                                         |
| `accTitle`, `accDescr`         | none                                                                                                                                                                                                 |
| Entities, markup in text       | none                                                                                                                                                                                                 |

### 6.3 Theme and branch

| Theme name in fixture | Count | Shipped by ferrigura?                 | Branch (5.3) |
| --------------------- | ----- | ------------------------------------- | ------------ |
| none (default)        | 7     | no (`default`)                        | classic      |
| `default`             | 3     | no                                    | classic      |
| `base`                | 2     | no                                    | classic      |
| `dark`                | 2     | no                                    | classic      |
| `forest`              | 2     | no                                    | classic      |
| `neutral`             | 2     | no                                    | classic      |
| `neo`                 | 2     | no                                    | classic      |
| `neo-dark`            | 2     | no                                    | classic      |
| `redux`               | 2     | no (falls back to `redux-color`)      | redux        |
| `redux-dark`          | 2     | no (falls back to `redux-dark-color`) | redux        |
| `redux-color`         | 2     | yes                                   | redux        |
| `redux-dark-color`    | 2     | yes                                   | redux        |

Totals: 22 fixtures take the classic branch, 8 the redux branch, and 4 of those 8 are shipped themes. The M2 fallback rule (`STATUS.md`, M2 handover: names not shipped take `redux-color`, or `redux-dark-color` when they end in `dark`) sends the 22 classic fixtures to the redux branch. Their text would sit at y 20 instead of 10, and their node paths would change. Colours are not compared, but geometry is. The M2 fallback is correct for pie (the pie has no branch) and wrong for the timeline.

Looks: 21 classic (15 with no `look` line, which defaults to classic, and 6 explicit), 10 neo. The `neo` look adds `data-look="neo"`, the title formula of 5.6, and with a redux theme the drop-shadow filter.

### 6.4 Browser features needed

| Feature                                       | Where                           | Note                                                                                                                                         |
| --------------------------------------------- | ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `getComputedTextLength` with collapsed blanks | every node                      | Verified: `a   b` = `a b`                                                                                                                    |
| `getBBox` height ignoring empty tspans        | every node height               | Verified                                                                                                                                     |
| `font-weight` 600 for redux section text      | redux fixtures                  | `theme-redux-color.js` 448. The bundled faces have only 400 and 700 (`fonts.rs` 35-47). Chromium's choice for 600 is unverified (likely 700) |
| `4ex` font size                               | title                           | Needs x-height (not in `ferrigura-text`). Unverified how Chromium computes `ex` for the bundled face                                         |
| `linearGradient`                              | neo fixtures with `useGradient` | Standard                                                                                                                                     |
| `drop-shadow` filter (`feDropShadow`)         | redux fixtures                  | Standard. The filter is not part of the geometry                                                                                             |

No fixture needs an unshipped font, a script fallback, or a browser feature beyond these.

### 6.5 Inputs for the differential run

The collector (`tools/port/inputs.ts`) would take: the 30 fixtures; the 11 `let str` inputs of `timeline.spec.js` and one timeline text in `multi-diagram-id-uniqueness.spec.ts` (line 149); the 14 `mermaid` blocks of `packages/mermaid/src/docs/syntax/timeline.md`; and the explicit cases of `cases.ts`. Roughly 55 to 60 inputs after de-duplication (estimate). Add a detector entry for `/^\s*timeline/` (compare `inputs.ts` 18 for info).

## 7. Risks, effort, open questions

### 7.1 Risks

| #   | Risk                                                                                                               | Size              | Mitigation                                                                                                                                            |
| --- | ------------------------------------------------------------------------------------------------------------------ | ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Classic branch not reachable through the shipped themes. 22 of 30 fixtures would be compared with the wrong branch | High              | Decision below. Do not let the harness replace classic themes with redux on both sides and call it covered: the classic branch would then be untested |
| 2   | Text box heights from `getBBox` depend on the font and on empty tspans                                             | Medium            | Use `text_box` for one line and the 17.6 step; test with the harness font                                                                             |
| 3   | Wrap widths depend on Chromium's collapsed-blank measurement                                                       | Medium            | Collapse blanks before `measure` (5.4)                                                                                                                |
| 4   | Redux `font-weight` 600 on section text, and the bundled faces (400 and 700)                                       | Medium            | Check which face Chromium uses for 600 with the harness font                                                                                          |
| 5   | `4ex` needs the x-height of the face                                                                               | Low (title only)  | Add an x-height metric, or measure it in Chromium                                                                                                     |
| 6   | Vertical renderer has two fixtures                                                                                 | Medium (coverage) | Add cases to `cases.ts`: TD with events on several tasks, TD with sections and no events                                                              |
| 7   | Entity placeholders measured before decoding                                                                       | Low               | Keep the encoded text (3.2)                                                                                                                           |
| 8   | `info` version string depends on a build constant                                                                  | Low               | Decision below                                                                                                                                        |

### 7.2 Effort (estimate, relative to the flowchart)

Estimates from the code sizes above, not measured. The flowchart (M3 parser, M5 renderer) is the reference point.

| Part                                                                                                | Estimate                                                                                                  |
| --------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Scanner (first-match, states) plus parser and DB, timeline only                                     | about a quarter of M3. The grammar is 87 lines of Jison but has many quirks (2.5)                         |
| Model and exporter                                                                                  | small (a few hundred lines)                                                                               |
| Renderer: both layouts, wrapping, node drawing, viewbox, SVG writing                                | about a quarter of M5 in size. No ELK, no edges, no clusters                                              |
| Classic theme (`default`) with the khroma functions it uses, for colours and for the classic branch | new work outside M5. Needed by 22 of the 30 fixtures, and by any other diagram that uses the classic look |
| Tests (11 DB tests, render checks, geometry harness)                                                | moderate                                                                                                  |

Overall: smaller than M5, with the classic theme as the one real extra. It depends on the decision in 7.3 (1).

### 7.3 Open questions for the coordinating session

1. **Classic theme.** Port `default` (and the other classic themes) now, or accept a harness that covers only the redux branch? The fallback rule of M2 cannot give the classic branch: it has to map a non-redux name to a non-redux theme, not to `redux-color`. Recommendation: port `default` (its values are needed for colours anyway) and map classic names to it in the fallback. This also needs a look at mindmap, which selects its styles by the same name test (`diagrams/mindmap/styles.ts`, `mindmapDb.ts`).
2. **Vertical defects R1, R2, R12.** Reproduce the reference output, or fix? Recommendation: fix the marker id and the node ids (they are plain bugs and do not change geometry), and record R1 and R2 as named exceptions in the harness, as the flowchart did. R12 (undefined gradient) changes the visible stroke; decide with the visual check.
3. **Info version.** The reference writes `v12.2.0` (verified). Options: a constant from `packages/mermaid/package.json`, or a `RenderOptions` field with that default. Recommendation: the constant, because the compatibility promise is that the same text gives the same diagram.
4. **Title and accessibility.** Keep "front matter title ignored" (R6) and "accTitle and accDescr not drawn" (R7) as reference behaviour, or write the accessibility texts into the SVG as the other diagrams do? Recommendation: keep the reference behaviour for parity, and record it in `canonical-model.md`.
5. **Entities.** Keep the encoded text in the model (recommended, 3.2), and share `encode_entities`/`decode_entities` (`flowchart/mod.rs` 177 and `label.rs` 368 already have copies).
6. **`canonical-model.md` corrections.** Line 27 (the `AT&T` example) and lines 44-52 (title precedence: the timeline reads the grammar title only).
7. **Lexer.** Hand-written, first-match, like the flowchart (logos cannot express first-match with the `%%` rule, the `\s` that takes newlines, or the `title\s` rule). Recommendation: yes.

### 7.4 Not verified

- DOMPurify on text with `<` and entities (only the fast path was checked, and two markup cases in Chromium).
- The cause of a 0.26 px difference in one bbox height across renders (19 vs 19.261) in the overlong-word case. Probably a font fallback for a glyph in the long word; not investigated.
- Whether Chromium renders `font-weight: 600` with the 700 face (likely, not checked).
- How Chromium computes `4ex`.
- Arrowhead absence in the vertical output (from the id mismatch, not checked on screen).
- The 19 px line box is the reference font's; the harness font will differ.
- The `timeline` DB's `id` growth across renders (from code only).

## 8. Info renderer (short, last)

`diagrams/info/infoRenderer.ts` (30 lines) draws:

```
svg = selectSvgElement(id)                           // 16
configureSvgSize(svg, 100, 400, useMaxWidth = true)  // 17; setupGraphViewbox.js 43-46
group = svg.append('g')                              // 19
group.append('text')                                 // 20-27
  x 100, y 40, class "version", font-size 32, style text-anchor: middle
  text "v" + version
```

Verified output (`info` and `info showInfo`, both the same):

```
<svg id="ID" width="100%" xmlns="…" style="max-width: 400px;" role="graphics-document document" aria-roledescription="info">
  <style>…common rules…</style>
  <g></g>
  <g><text x="100" y="40" class="version" font-size="32" style="text-anchor: middle;">v12.2.0</text></g>
</svg>
```

- No `height` attribute, no `viewBox`. The height is whatever the browser gives for an SVG with neither.
- `configureSvgSize` takes (svg, height, width, useMaxWidth); the call passes 100 as height and 400 as width, so `max-width: 400px` comes from the width.
- The `<style>` is the common rules with `fill: #333` and the font of the theme (verified), not a diagram rule set. The info renderer has no styles.
- `version` is `injected.version` (`infoDb.ts` 4), a build constant. The bundle gives `12.2.0`.
- No measurement is needed. The only geometry is the text anchor at (100, 40).
- Accessibility: `info` has no `getAccTitle`, so nothing is written (verified: no `<title>` or `<desc>`).
- Detection: `/^\s*info/` (`infoDetector.ts` 10) also matches `information` and `infographic`. `ferrigura-syntax` `detect` (`lib.rs` 81) does the same with `starts_with("info")`, which is the same behaviour.

Ferrigura today: `ferrigura/src/lib.rs` 74-78 returns an error "the info diagram is not rendered yet". The change is in that match arm, plus a small renderer module in `ferrigura-render` that writes the attributes above with the common style block (the `fill` comes from the theme's text colour, so it depends on the theme).

Effort: small, a few dozen lines plus the style block. The one decision is the version (section 7.3, item 3).

---

## Probe log (scratchpad `m6-timeline/`, not committed)

| File                                                                                                                  | What it did                                                                                                                                                         |
| --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `probe-ref.cjs`, `make-inputs.cjs`, `inputs-all.json`, `ref-out.json`                                                 | Rendered all 30 fixtures and 11 extra inputs in Chromium with the reference bundle; collected SVG attributes, bboxes, marker ids, node classes, dashed lines, texts |
| `probe-dump.cjs`, `svg-ref*/`                                                                                         | Wrote the reference SVG for selected inputs                                                                                                                         |
| `probe-wrap.cjs`                                                                                                      | Per-node tspans, `dy`, text `getBBox` height                                                                                                                        |
| `probe-lex.cjs`, `timeline-parser.cjs` (generated by `gen.cjs` from the unchanged `timeline.jison`), `cases-lex.json` | Jison tokens and parse results with a recording DB stub                                                                                                             |
| `probe-measure.cjs`, `probe-serial.cjs`                                                                               | Chromium: blank collapsing in `getComputedTextLength`; serialisation of `&amp;` in text                                                                             |
| `extra.json`, `extra2.json`, `inputs2.json`, `inputs3.json`, `wrap-inputs.json`, `br-inputs.json`                     | The probe inputs                                                                                                                                                    |
