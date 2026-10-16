# M6 gantt parser: plan

Status: implemented. The parser, the DB, the date rules and the URL rules are in `crates/ferrigura-syntax/src/gantt/`; the specs are in `crates/ferrigura-syntax/tests/gantt_*_spec.rs`; `pnpm port:diff gantt` shows 125 inputs, no difference and no named exception. Open items are in section 11.

The renderer (`ganttRenderer.js`, `styles.js`) is ported in `crates/ferrigura-render/src/gantt/`; section 12 records its decisions.

## 1. Scope

The port covers what the reference computes between the text and the model: the Jison lexer and grammar, the DB (`ganttDb.js`), the date semantics (dayjs, the legacy `Date` fallback, the clock and the time zone), and the sanitising of the header texts.

Verified against the source while writing this plan: the lexer rules (`gantt.jison:6-99`) and the grammar actions (`107-186`) match NOTES 2.2 and 2.4, with two corrections below (`section`/`title` across a line break, and the tag-only task).

## 2. Lexer

A state-stack scanner (`ferrigura-syntax/src/gantt/scanner.rs`) that runs the rule table in the order of `gantt.jison`, first match wins, as Jison does (NOTES 2.3). The helpers of `scan.rs` do the matching. Keyword literals match case-insensitively (`%options case-insensitive`) and quoted keywords get the implicit word boundary; unquoted words such as `weekday\s+monday` get none.

States: `Initial`, `AccTitle`, `AccDescr`, `AccDescrMulti`, `Href`, `CallbackName`, `CallbackArgs`, `Click`. Rules that return no token are skipped.

Quirks the rule table gives, which the port keeps (each has a test that pins the reference's result):

| Id  | Input                                                         | Reference result                                                                                                                                                                            |
| --- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1  | `A%B :a, 2014-01-01, 1d` (a character followed by `%`)        | rule 29 skips the line (NOTES 2.5 Q1)                                                                                                                                                       |
| Q2  | `section` or `title`, a line break, then a non-empty line     | Confirmed by the reference: `"section"\s[^\n]+` takes the line break as its `\s`, so the next line is the section name (`section "A :a1, 2014-01-01, 1d"`, no task). `title` does the same. |
| Q3  | `accTitle:` then a line break, then `section X`               | the `\s*` after the colon eats the line break; the value is the next line (NOTES Q2)                                                                                                        |
| Q4  | `call f` without parentheses, or an unclosed `click … href "` | `[^(]*` runs across line breaks to the end of the text; the reference rejects the input at the end (parse error).                                                                           |
| Q5  | unclosed `accDescr {` block                                   | Rejected by the reference at the end of the text (the exporter probe). The scanner ends the text in a start condition with an `Invalid` token.                                              |
| Q6  | `:active`, `:done,milestone`: a task whose data is only tags  | **Correction**: `getTaskTags` reads `data[0]` after the array is empty, a TypeError during the parse (not in `getTasks`). Rejected.                                                         |
| Q7  | `;` or `#` in task data                                       | parse error (NOTES Q5)                                                                                                                                                                      |
| Q8  | `dateFormat` alone, `section` alone, `title ` (empty)         | parse error (NOTES Q7)                                                                                                                                                                      |

## 3. Grammar and DB calls

The grammar accepts statements without separators, after one `gantt` keyword at the start (`start: gantt document EOF`). The actions call the DB as in NOTES 2.4. The value of a keyword is the token after `substr(n)` with the keyword length plus one character: `dateFormat` 11, `axisFormat` 11, `tickInterval` 13, `excludes` and `includes` 9, `todayMarker` 12, `title` 6, `section` 8. `substr` counts UTF-16 units; for the keyword part and the one whitespace character (`\s` is a BMP character) that is the same as counting characters.

Task statements: `addTask($1, $2)` with `$1` (the task text) untrimmed and `$2` the data starting with `:`.

## 4. DB state, `getTasks` and the clock

- The DB state is per parse. The reference's module state is cleared by `clear()` except `taskDb` (NOTES 3.1). The port does not reproduce that leak: a diagram's result depends on its text and the clock only. The exporter reloads the DB module for every input, so the reference's results are those of a fresh process.
- `taskDb` maps an id to the position of its last definition (NOTES 3.8). The key `__proto__` is never stored (assignment to the prototype), so its lookup fails; `constructor` and other `Object.prototype` names are stored, as own properties, and resolve as the reference does.
- `getTasks` runs `compileTasks` once and then up to 10 more passes while a task is unprocessed (at most 11). Each pass recomputes every task from its raw data in array order, and reads the `endTime` of the previous task as it stands at that moment. The port keeps the per-task state (`startTime`, `endTime`, `renderEndTime`, `processed`, `manualEndTime`) and updates it in place, so a reference to a task that is compiled later reads the value of the previous pass.
- A value the reference holds as an Invalid Date (a timestamp such as `dateFormat x` with `9999999999999999`, `new Date` of a number above 8.64e15) is `NaN` internally, and it is `null` in the model, as an unresolved value is. Its task counts as processed, as in the reference.
- An error thrown by `getTasks` in the reference (NOTES 2.6 b) is a rejection of the input: the port returns an error from `parse`, because the model holds the resolved times.

## 5. Dates

Everything is computed in UTC. The reference runs in UTC in the differential run (`TZ=UTC`), so the local time of the reference equals UTC. `now` is a parse option (`Options::now`, milliseconds since the epoch); its default is the system clock. Every use of the clock in the reference comes from `now`: the parts that dayjs fills in from `new Date()` for a format without a year, month or day, the `after` and `until` fallbacks (today at 00:00), and nothing else in the parser. The fixed clock of the differential run is `-30270326400000` (1010-10-10T00:00Z), the same instant as the e2e clock of `mmd-snapshots.spec.ts`.

### 5.1 Strict parsing (`dayjs(str, fmt, true)`)

Port of dayjs 1.11.21 `customParseFormat` with `advancedFormat` (for `X` and `x`). A parse fills the missing parts from `now`, builds the time in UTC, and is valid only when formatting the result with the same format gives the input back exactly (NOTES 3.3). The empty format never matches. The token set is the one of the plugin: `YYYY`, `YY`, `M`, `MM`, `MMM`, `MMMM`, `D`, `DD`, `Do`, `DDD`, `DDDD`, `d`, `dd`, `ddd`, `dddd`, `H`, `HH`, `h`, `hh`, `m`, `mm`, `s`, `ss`, `S`, `SS`, `SSS`, `A`, `a`, `Z`, `ZZ`, `X`, `x`, and bracketed text. Any other character of the format is a literal.

### 5.2 Durations (`parseDuration`)

`/^(\d+(?:\.\d+)?)([Mdhmswy]|ms)$/` on the trimmed text (NOTES 3.3, probed). No sign, no exponent. Units: `M` month, `d` day, `h` hour, `m` minute, `s` second, `w` week, `y` year, `ms` millisecond.

`add` (dayjs 1.11.21, NOTES 3.3): `ms`, `s`, `m`, `h` add a fixed number of milliseconds (a fraction is kept until the Date truncates it); `d` and `w` use `Math.round` of the count and move the calendar day; `M` and `y` move the month or year and clamp the day to the month's length.

### 5.3 The legacy `new Date(string)` fallback

Reached when the strict parse fails (`getStartDate`, step 5). V8 decides these results; the port implements the forms that the fixtures and the specs reach, and rejects the rest as an Invalid Date (which the reference reports as `Invalid date:<text>` when the year is out of range or the result is NaN).

| Form                                                | Result                                                                       | Where it is reached                                       |
| --------------------------------------------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------- |
| ISO date only `YYYY-MM-DD` (and `YYYY`, `YYYY-MM`)  | UTC midnight; day and month roll over                                        | visual-check, issue-1060, `2024-02-30` probe              |
| non-ISO `Y-M-D` with one-digit fields (`2019-09-1`) | local midnight (UTC here)                                                    | issue-1060                                                |
| `Y/M/D`                                             | local midnight                                                               | probe                                                     |
| a lone number `N` (`0`, `202`, `202304`)            | year `N` (below 50: 2000 + N; 50 to 99: 1900 + N), January 1, local midnight | `dateFormat ss` fixture (`0`), typo fixture (`202-12-01`) |
| `Y-M-D` with a year below 1000 (`202-12-01`)        | year as written (202), local midnight                                        | typo fixture                                              |
| ISO date and time, with or without offset           | as ECMA-262: no offset is local, `Z` or an offset is exact                   | not in the fixtures; kept for the subset                  |

Years outside ±10000 are an error (`Invalid date:`), as the reference checks them after the fallback. Everything else is NaN.

### 5.4 Other date rules

`isInvalidDate`, `checkTaskDates` and `fixTaskDates` (NOTES 3.4) are ported as they are written, with the same arithmetic. The `excludes`/`includes` token lists are lower-cased and de-duplicated in first-seen order, and `weekend` and `weekday` are the grammar's words.

## 6. Canonical model

`GanttDiagram` in `crates/ferrigura-model/src/gantt.rs`, `version: 1`, camelCase.

| Field                                              | Rule                                                                                                                                                                                                                                                                                                                                                     |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `title`, `accTitle`, `accDescr`                    | the DB values, sanitised with the DOMPurify subset (`sanitize.rs`). `title`: the last `title` statement (it overrides the front matter title even when it sanitises to empty, as `setDiagramTitle` does), else the front matter title, else empty; `title` keeps its blanks. `accTitle` without leading blanks; `accDescr` with `\n\s+` reduced to `\n`. |
| `dateFormat`, `axisFormat`, `todayMarker`          | the raw DB value, blanks included                                                                                                                                                                                                                                                                                                                        |
| `tickInterval`                                     | the raw value, or `null` when the statement is absent                                                                                                                                                                                                                                                                                                    |
| `topAxis`, `inclusiveEndDates`                     | the DB flags                                                                                                                                                                                                                                                                                                                                             |
| `weekday`, `weekend`                               | the DB words (`sunday` and `saturday` by default)                                                                                                                                                                                                                                                                                                        |
| `includes`, `excludes`                             | lower case, de-duplicated, first-seen order                                                                                                                                                                                                                                                                                                              |
| `sections`                                         | the section names as written (`addSection`), duplicates kept                                                                                                                                                                                                                                                                                             |
| `links`                                            | object: task id to the sanitised URL (`sanitizeUrl`, strict mode), only for tasks that exist when the line is read                                                                                                                                                                                                                                       |
| `tasks`                                            | the `getTasks()` result, in DB order                                                                                                                                                                                                                                                                                                                     |
| task `id`, `task`, `section`, `order`              | the DB values; `task` raw and untrimmed, `section` the current section (`''` before any section)                                                                                                                                                                                                                                                         |
| task `vert`, `milestone`, `active`, `done`, `crit` | flags from the tags                                                                                                                                                                                                                                                                                                                                      |
| task `classes`                                     | in order, duplicates kept (`clickable` from `click`)                                                                                                                                                                                                                                                                                                     |
| task `manualEndTime`                               | the DB flag (`false` for an unprocessed task)                                                                                                                                                                                                                                                                                                            |
| task `startTime`, `endTime`, `renderEndTime`       | epoch milliseconds (integers, UTC); `null` when unresolved or an Invalid Date                                                                                                                                                                                                                                                                            |

Left out, and why: `type` (equal to `section`), `raw`, `prevTaskId`, `processed` (derivable from `startTime` and `endTime`), `taskDb`, `taskCnt`, `lastOrder`, `funs` and click callbacks (no effect in strict mode, decision 0001), the `displayMode` (configuration; `getDisplayMode` is always `''` in the pipeline, NOTES 2.8), the renderer's compact `order` (render time), and `useWidth`, `tickInterval` defaults (configuration).

The header texts are sanitised by the DB (`commonDb.ts`), which the canonical rule of `canonical-model.md` allows for the flowchart only. For gantt, `title`, `accTitle` and `accDescr` are sanitised and task and section names are not. The gantt paragraph of `canonical-model.md` states this.

## 7. Errors and rejections

- Parse errors (Jison) are ours: "fails at the offending token", with a position (ADR 0002).
- Errors that the reference raises while compiling dates (`Invalid date:`, `Failed to find a valid date … after 10,000 iterations`, a TypeError for a first task with no end to refer to, or for a task with four fields) are rejections with a message. The exporter reports them as rejections, and the port rejects the same inputs.
- Warnings (unknown `after` and `until` ids, an unparsable duration, an exclude span above five years, more than 10000 ticks) change nothing in the model. The port does not log them.

## 8. Exceptions policy

A named exception in `tools/port/exceptions.ts` only where the reference's output is garbage that the port does not reproduce (NaN dates from a timestamp the reference cannot represent, a crash). Everything else agrees, including the legacy forms of section 5.3 that the reference accepts.

## 9. Specs

| Spec file              | Tests                           | Port                                                                                                                                                                                                                                                                                                             |
| ---------------------- | ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `parser/gantt.spec.js` | 15 `it`, 3 `it.each` (29 cases) | `crates/ferrigura-syntax/tests/gantt_parser_spec.rs`. Parse-only tests keep their names. Spy tests (`setTodayMarker`, `setClickEvent`) become outcome tests (the model). `click` tests check `classes` and `links`.                                                                                              |
| `ganttDb.spec.ts`      | 22 `it`, 6 `it.each` (53 cases) | `crates/ferrigura-syntax/tests/gantt_db_spec.rs`. DB calls become diagram text, through `parse_with` with the fixed clock. The `TZ=America/Los_Angeles` test (line 488) is `#[ignore = "port: local zone, the port is UTC"]`. Warning tests (`log.warn` spies) assert the model only; the messages are not kept. |

Titles are the reference's. Exact Jison message texts are not copied: a test that expects one asserts that parsing fails.

## 10. Differential run

- `tools/port/export-gantt.ts`: runs the reference's Jison parser (`gantt.jison`) with the DB of a fresh module instance per input (the reference's `taskDb` leak is not part of the output), `TZ=UTC`, the clock `Date.now()` and `new Date()` fixed to `now`, and `getTasks()` after the parse. A rejection is either a parse error or a throw in `getTasks`.
- Inputs: `e2e/diagrams/gantt/` (52 fixtures, all with front matter), the `gantt` spec literals, the `gantt` mermaid blocks of the docs, and the explicit cases in `tools/port/cases.ts`.
- `ferrigura-syntax`'s `canonical` example runs `gantt::parse_with` with the same `now`.
- Target: zero differences, and no named exception except the ones section 8 allows.

## 11. Open questions and follow-ups

- The front matter exception is resolved: the shared YAML reader now accepts the later lines of a quoted scalar at any indentation, as js-yaml does (it only warns), and `pnpm port:diff yaml` covers the cases.
- Chains: the exporter probe found 11 of 12 after-links resolving in the reference (NOTES 3.8 says 11 resolve, 12 do not); the explicit chain case in `tools/port/cases.ts` agrees with the reference, so the port follows the reference.
- `dayjs` strict parsing covers the token table of customParseFormat, with `w`, `z` and `gggg` throwing as in the reference; legacy `Date` covers the forms of section 5.3.

1. The V8 forms of section 5.3 beyond the listed ones are rejected. The differential run decides whether the inputs reach any form that is not listed.
2. The title sanitising of gantt follows the DB (DOMPurify subset). The other header texts are not sanitised in other types; this is a gantt paragraph in `canonical-model.md`.
3. `securityLevel` is `strict` by default. `loose` (click callbacks run in the browser) is not ported; the model has no callbacks.

## 12. Renderer

The renderer draws the model. The reference's rules are in `NOTES.md` section 6 and the code is in `crates/ferrigura-render/src/gantt/`: `mod.rs` (dimensions, task order, compact rows, root), `draw.rs` (excluded days, grids, bars and labels, section labels, today marker, title), `time.rs` and `scale.rs` (d3-time 3.1.0 and d3-scale 4.0.2 in UTC), `format.rs` (d3-time-format 4.1.0, en-US), `axis.rs` (d3-axis 3.0.0), `dayjs.rs` (the dayjs operations the renderer uses), `text.rs` (labels, entity placeholders, `<br>` splits, widths), `style.rs` (the sheet and the colours).

### Status

- `pnpm port:geometry gantt`: 52 of 52 fixtures within tolerance, 0 outside, 0 not rendered, 0 excluded. The element counts are the reference's.
- Computed styles (`fill`, `stroke`, font, anchor, transform, opacity and the rest) match the reference's on every element of the 52 fixtures (a scratch probe over the same pages, not in the repository).
- Unit tests in `time.rs`, `scale.rs`, `format.rs`, `dayjs.rs`, `text.rs`; the element-level spec in `crates/ferrigura-render/tests/gantt_render_spec.rs`. The reference has no renderer spec for gantt (`NOTES.md` 5.1), so there is none to port.

### Width and clock

- The reference takes `elem.parentElement.offsetWidth` when `useWidth` is unset (`ganttRenderer.js:98`). ferrigura has no container, so it uses 1200 (the reference's own fallback, `:101-103`). The harness page has no body margin, so the reference's container is the 1200 px viewport: a probe measures a block appended to the body at 1200 px, and every fixture without `useWidth` has `viewBox` width 1200 on both sides. The page already had `body { margin: 0 }`; the change is a comment in `tools/port/geometry/common.ts`. `pnpm port:geometry-pie` 24 of 24 and `pnpm port:geometry-flowchart` 298 of 298 are unchanged.
- The harness runs the reference in UTC with the clock `Date.now()` and passes the same instant to ferrigura (`--now`), so the today marker agrees.

### Decisions

| Question                     | Decision                                                                                                                                                                    | Why                                                                                                                                                                                                                |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Axis offset                  | 0.5 (device pixel ratio 1)                                                                                                                                                  | `d3-axis` uses 0 above DPR 1. The harness renders at DPR 1; the reference's e2e baseline uses DPR 2 (`playwright.config.ts:54`). A server renderer has no device, so the port writes the value the harness checks. |
| Label width                  | the text laid out with the default `white-space` (runs of blanks collapse, blanks at the ends are dropped), measured in the sans face at 1/64 px                            | The reference's `getBBox` does not count trailing blanks (`width-76.03125` for `Completed task` followed by twelve blanks).                                                                                        |
| Entities                     | the placeholders the parser writes (`ﬂ°°N¶ß`, `ﬂ°name¶ß`) are read back as the character they name in the SVG and in the width; an unknown name is the text `&name;`        | `utils.ts:874-906`: the reference decodes the placeholders in the SVG string. No fixture uses an entity code.                                                                                                      |
| Attributes DOMPurify removes | `text-height` is not written; class values are trimmed                                                                                                                      | The reference's shipped SVG passes through `DOMPurify.sanitize` (`mermaidAPI.ts:645`), which drops `text-height` (`ganttRenderer.js:437`) and trims attribute values.                                              |
| Colours                      | the reference's rules, scoped to `#id`, with `var(--token)`; the concrete colour of each coloured element as a presentation attribute, chosen by the cascade of `styles.js` | decision 0007. The tick text's `fill="#000"` of the reference is overridden by the reference's own CSS, so the port writes the text colour.                                                                        |
| Links                        | `<a xlink:href target="_top">` around the bar and its label, at the end of the group (the sandbox branch's structure)                                                       | Decision 0001 keeps links as `<a>`. The strict-mode reference binds a JavaScript click instead (`ganttDb.js:744-778`, `ganttRenderer.js:508-531`). No fixture uses `click`.                                        |
| Callbacks                    | not drawn                                                                                                                                                                   | Decision 0001 (`click … call` is in the model only as `clickable`).                                                                                                                                                |
| Unresolved tasks             | drawn with `NaN` in `x`, `width` and `transform-origin`, as the reference does                                                                                              | The compatibility promise; not in any fixture. Open question 8 of section 8 is answered for the renderer: the port reproduces `NaN`.                                                                               |
| Sorting                      | a stable insertion sort with the reference's comparator                                                                                                                     | Matches `Array.prototype.sort` for consistent comparators and cannot panic on `NaN` ones.                                                                                                                          |

### Quirks reproduced

- A run of excluded days that is still open at the last day of the span is not drawn (`ganttRenderer.js:580-590`: the open range is never pushed). Pinned by a spec.
- `renderEndTime || endTime` takes the render end whenever it is set, including a time of 0 (a Date is truthy).
- Milestone labels are measured in the regular face (the class with `milestoneText` is not set when the reference measures), and drawn in italic (`.milestoneText`).
- The label class and the label's `x` use different ends for a milestone (`ganttRenderer.js:400-506`): `x` uses the shifted start and the render end; `class` uses the end time.
- `drawRects` sorts the task array in place, markers last; `drawExcludeDays` and the bars read it in that order.
- `getMaxIntersections` writes the compact row into the task (`order`), and the section label's `y` accumulates the counts of the earlier sections (`prevGap`).
- The custom tick interval is ignored when its estimate exceeds 10000 ticks, and the top axis then uses the default ticks (`ganttRenderer.js:683-713`, `753`).
- `dateFormat` with a single `D` changes the axis format to `%d`; the axis format and tick text keep their blanks (`' %d/%m'` gives `' 07/01'`).
- The grid group and tick text carry the d3 attributes the reference writes (`fill="none"`, `font-family="sans-serif"`, `text-anchor` as a presentation attribute and as a style).

### Not reproduced, or approximated

- The `<a>` wrapping of a linked task is written, but the reference's strict mode has no anchor; see the links row.
- A `font-weight: bold` of `.taskText.clickable` is in the sheet, but the embedded faces are regular only, so Chromium synthesises the bold.
- Entity names other than the five of XML stay as text (`&name;`). The browser would decode the HTML entity names; no fixture has one.
- The reference's `weekday` names that are not a day of the week make the tick function undefined and throw. The port returns a render error for them.

### Shared files changed

- `crates/ferrigura-syntax/src/gantt/db.rs` and `mod.rs`: the DB's `isInvalidDate` is a free function (`invalid_date`) and `gantt::is_invalid_date(&GanttDiagram, f64)` exposes it. The renderer's excluded days need the reference's dayjs formatting, which the parser already has. The method's behaviour is unchanged.
- `tools/port/geometry/common.ts`: a comment on the page's zero body margin (see Width and clock). No behaviour change.
