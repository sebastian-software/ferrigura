# Gantt diagram

A Gantt diagram draws tasks as bars on a time axis, grouped in sections, with milestones, vertical markers, excluded days, a today marker and an optional title. It starts with the keyword `gantt`. ferrigura parses it with a hand-written parser that ports the Jison grammar `gantt.jison`, the DB `ganttDb.js` and the date rules they rely on (`crates/ferrigura-syntax/src/gantt/`), and draws it with a renderer of its own (`crates/ferrigura-render/src/gantt/`) on ports of d3-time, d3-scale, d3-time-format and d3-axis. There is no layout engine.

## Model

`GanttDiagram` (`crates/ferrigura-model/src/gantt.rs`), version 1, is the DB's state after `getTasks()`, with the dates resolved.

- `title`, `accTitle`, `accDescr`: sanitized, with the DB's whitespace rules. A `title` statement replaces the front matter title even when it sanitizes to empty. `title` keeps its blanks.
- `dateFormat`, `axisFormat`, `todayMarker`: the text after the keyword, blanks included. `tickInterval` is that text, or `null` when the statement is absent.
- `topAxis`, `inclusiveEndDates`: the flags of the statements of the same name.
- `weekday`, `weekend`: lower-case day names, `sunday` and `saturday` by default.
- `includes`, `excludes`: lower case, without duplicates, in first-seen order.
- `sections`: the section names as written, duplicates kept.
- `links`: task id to the URL of its `click … href` statement, after `sanitizeUrl`, for the tasks that exist when the line is read.
- `tasks`, in the DB's order, each with `id` (given or `task1`, `task2`, …), `task` (the text as written, untrimmed), `section` (`""` before the first section), `order` (the position among the non-vertical tasks, `-1` for a vertical one), the flags `vert`, `milestone`, `active`, `done`, `crit`, `classes` (in order, duplicates kept; `clickable` comes from `click`), `manualEndTime`, and `startTime`, `endTime` and `renderEndTime`.

Dates are epoch milliseconds in UTC. A date the DB leaves unresolved or holds as an Invalid Date is `null`. `renderEndTime` is the end the bar is drawn to, and is `null` when `excludes` do not apply.

The model leaves out `type` (equal to `section`), `raw`, `prevTaskId`, `processed`, the task map and counters, click callbacks (decision [0006](../decisions/0006-rust-only-no-browser.md)), `displayMode` (configuration), and the compact row the renderer computes. Task texts and section names are raw ([canonical-model.md](../canonical-model.md#gantt)).

## Parsing

**Lexer.** `scanner.rs` runs the rules of `gantt.jison` in their written order over a state stack: first match wins, as Jison's does, with case-insensitive keywords and the word boundary Jison adds to quoted literals (unquoted words such as `weekday monday` get none). The states are `Initial`, `AccTitle`, `AccDescr`, `AccDescrMulti`, `Href`, `CallbackName`, `CallbackArgs` and `Click`. A character no rule starts with is an `Invalid` token, which the grammar rejects.

**Grammar.** `parser.rs` reads the statements of `gantt.jison`: `dateFormat`, `axisFormat`, `tickInterval`, `includes`, `excludes`, `todayMarker`, `weekday`, `weekend`, `topAxis`, `inclusiveEndDates`, `title`, the accessibility lines, `section`, `click` and the task lines `name : data`. Each statement makes the DB call its action makes. A statement keyword's value is the text after the keyword and one more character, as the actions' `substr` takes it.

**DB.** `db.rs` holds the state of one parse. `get_tasks` compiles the dates as the reference does: one `compileTasks` pass, then up to ten more while a task is unprocessed, each pass recomputing every task from its raw data in order and reading the previous task's end as it stands. An error that `getTasks` throws in the reference (`Invalid date:`, no valid date after 10,000 iterations, a first task with no end to refer to) rejects the input, because the model holds the resolved times.

**Dates.** All dates are computed in UTC, with the clock as an option (`Options::now`, milliseconds since the epoch, the system clock by default). Every use of the clock in the reference comes from it: the parts that a format without a year, month or day takes from today, and the fallback of `after` and `until` without a known task. `date.rs` holds the rules:

- `strict_parse` is dayjs 1.11.21's strict `customParseFormat` with `advancedFormat`: missing parts come from the clock, and the parse is valid only when formatting the result with the same format returns the input. The format tokens `w`, `wo`, `ww`, `gggg`, `z` and `zzz` call methods of plugins the reference does not load, so they throw and reject the diagram.
- Durations are `^(\d+(?:\.\d+)?)([Mdhmswy]|ms)$` on the trimmed text. Days and weeks round the count and move the calendar day; months and years clamp the day to the month's length; the smaller units add milliseconds.
- `legacy_parse` is V8's `new Date(string)` for the forms that fixtures and specs reach: digits alone as a year (below 50 is 2000 plus, 50 to 99 is 1900 plus), `YYYY`, `YYYY-MM`, `Y-M-D` and `Y/M/D`, an ISO date with time and zone, and `M-D-Y`. Other forms are an Invalid Date.
- `isInvalidDate`, `checkTaskDates` and `fixTaskDates` are ported as written, with the same arithmetic. `is_invalid_date` is public so that the renderer uses the same rule for excluded days.

**Quirks kept on purpose.**

- A line with a character followed by `%` (`A%B :a, 2014-01-01, 1d`) is skipped as a comment.
- `section` and `title` take the next line when only a line break follows (`\s` matches it), and `accTitle:` followed by a line break takes the next line as its value.
- `call f` without parentheses and an unclosed `click … href "` run to the end of the text and fail there.
- A task whose data is only tags (`:active`, `:done,milestone`) is an error during the parse.
- `;` or `#` in task data is an error; in names, titles and sections it is text.
- A tag counts only at the start of the data, in any order, and only in lower case (`Done` is not a tag).
- `taskDb` maps an id to the position of its last definition; `__proto__` is never stored, while `constructor` and the other `Object.prototype` names are.
- The reference's DB module keeps `taskDb` across diagrams. ferrigura keeps no state between parses, so a diagram's result depends on its text and the clock only; the exporter loads a fresh DB module for every input.

## Layout and rendering

The renderer ports `ganttRenderer.js` and `styles.js`. `mod.rs` sizes the diagram and orders the tasks, and `draw.rs` writes the elements in the reference's order, each with its attributes in the order the reference sets them. The modules `time.rs` (d3-time 3.1.0), `scale.rs` (d3-scale 4.0.2), `format.rs` (d3-time-format 4.1.0, en-US), `axis.rs` (d3-axis 3.0.0) and `dayjs.rs` run in UTC, where the local calendar of the reference's code is the UTC calendar. `text.rs` handles labels and `style.rs` the style sheet.

- **Width.** The width is `gantt.useWidth` when set, else 1200 px. The reference takes its container's `offsetWidth` and falls back to 1200 itself; the harness makes the container 1200 px wide.
- **Scale.** The scale is a time scale over the earliest start and latest end of all tasks (vertical ones included, unresolved ones skipped), mapped with `rangeRound` to `[0, width - leftPadding - rightPadding]`. Every scale output is an integer, and the text of a bar sits at half-integers.
- **Order.** The task array is sorted by start with a stable insertion sort and the reference's comparator, which cannot panic on a `NaN` comparison, and vertical tasks are drawn last. The categories of the section rows are the unique section names, in the key order of a JavaScript object (array indices first).
- **Rows.** Each non-vertical task is a row of `barHeight + barGap`. In `displayMode: compact`, `getMaxIntersections` writes the compact row of each task, and a section label's `y` accumulates the counts of the sections before it.
- **Bars and labels.** A milestone is a rotated square; a vertical task is a thin line down the diagram. A label sits inside its bar when it fits and outside it when it does not: to the right of the bar, or to its left when the label would pass the right edge of the diagram. The label's class and its `x` use different ends for a milestone: `x` the shifted start and the render end, the class the end time.
- **Excluded days.** `excludes` and `includes` give ranges drawn as rectangles behind the bars. A run of excluded days that is still open at the last day of the span is not drawn, as in the reference.
- **Axes.** `axisFormat` formats the ticks with d3-time-format; `tickInterval` (`1day`, `2week`, …) selects the interval, with `weekday` as the first day of a week; `topAxis` adds a second axis. A custom interval whose estimate exceeds 10,000 ticks is ignored and the default ticks are used. The axis offset is 0.5, the value at a device pixel ratio of 1.
- **Today marker.** A line at `now`, unless `todayMarker` is `off`. The text after the keyword is the line's CSS, with commas written as semicolons.
- **Links.** A task with a `click … href` is wrapped in `<a xlink:href target="_top">` at the end of its group.
- **Title.** The title text is written at the top center, also when the diagram has none.
- **Root.** The viewBox is `0 0 width height`. With `useMaxWidth` the root has `width="100%"` and a `max-width`.

## Differences from Mermaid

- **Links are anchors.** The reference in strict mode binds a JavaScript click; ferrigura writes an `<a>` and drops callbacks ([0006](../decisions/0006-rust-only-no-browser.md)).
- **UTC only.** The reference reads dates in the browser's zone; ferrigura runs in UTC, and the `TZ=America/Los_Angeles` test of the DB spec is ignored. Its comparison runs under `TZ=UTC` with a fixed clock.
- **Colors** are custom properties with attribute fallbacks, and only the two redux themes ship ([0011](../decisions/0011-themes-as-css-variables.md)). The tick text takes the text color, where the reference's attribute `#000` is overridden by its own CSS.
- **Attributes the sanitizer drops.** The reference's SVG passes through DOMPurify, which drops `text-height` and trims attribute values; ferrigura writes neither.
- **The axis offset is fixed at 0.5.** A server render has no device pixel ratio.
- **Text widths** are measured in the bundled face, with the label laid out as an SVG `text` collapses blanks ([0009](../decisions/0009-svg-text-labels.md), [0010](../decisions/0010-text-measurement-and-fonts.md)).

## Known gaps

- Entity names other than the five of XML stay as text (`&name;`); a browser would decode the HTML names. Placeholders written by the parser (`ﬂ°°N¶ß`, `ﬂ°name¶ß`) are read back to the character they name.
- `legacy_parse` leaves out the V8 forms `Jan 1 2014`, `2014/1`, `1/6`, `1.5`, signed years and `Y-M` with a one-digit year; the differential run reports an input that reaches one.
- `.taskText.clickable` is bold in the style sheet, but the embedded faces are regular, so Chromium synthesizes the bold.
- A `weekday` that is not a day name makes the reference's tick function undefined and throw; ferrigura returns a render error.
- A task whose dates cannot be resolved is drawn with `NaN` in `x`, `width` and `transform-origin`, as the reference does; no fixture has one.

## Checking it

- `pnpm -C tools/port port:diff gantt` runs the reference's parser and DB over every input with `TZ=UTC` and the clock fixed to 1010-10-10T00:00Z, a fresh DB module per input, and compares the canonical models with ferrigura's `canonical` example under the same clock. A rejection on both sides agrees.
- `pnpm -C tools/port port:diff config` covers the `gantt` section.
- `pnpm -C tools/port port:geometry gantt` renders every fixture on both sides in Chromium, in UTC with the same clock, and compares every element within 0.05 px.
- The Rust specs: `crates/ferrigura-syntax/tests/gantt_parser_spec.rs` (`parser/gantt.spec.js`) and `gantt_db_spec.rs` (`ganttDb.spec.ts`, with the DB calls turned into diagram text and the log spies into outcomes of the model). `crates/ferrigura-render/tests/gantt_render_spec.rs` pins the structure the geometry comparison relies on, and the modules `time.rs`, `scale.rs`, `format.rs`, `dayjs.rs` and `text.rs` have unit tests. The reference has no renderer spec for the Gantt diagram.

## Learnings

- **The hard part is the dates, not the drawing.** They depend on dayjs's strict round trip, the clock, V8's `new Date(string)` and the time zone. Of the fixtures, the epoch formats, the formats that take the day from the clock (`HH:mm`, `ss`, `SSS`, `D`) and the ISO date-only strings through the legacy path (UTC midnight) change under another zone.
- **`getTasks` is not idempotent in the reference,** and it sorts and mutates its array in place. The renderer works on a copy and the model stays immutable.
- **Trailing blanks do not count.** `getBBox` of a label ignores trailing blanks (`Completed task` followed by twelve blanks measures as `Completed task`), so the label is laid out with the default `white-space` before it is measured. A milestone's label is measured in the regular face, because the class that makes it italic is not set when the reference measures.
- **`rangeRound` makes every position an integer,** so a bar's text lands on a half-integer; a coordinate computed another way is off by the rounding.
- **`renderEndTime || endTime` is not a null check:** it takes the render end whenever it is set, including a time of 0, since a Date is truthy.
- **A single `D` in `dateFormat` changes the axis format to `%d`,** and the axis format and tick text keep their blanks (`' %d/%m'` gives `' 07/01'`).
- **The harness page has no body margin,** so the reference's container is the 1200 px viewport. Without that, every fixture without `useWidth` would differ in width.
- **d3-axis offsets by the device pixel ratio.** The offset is 0.5 at a ratio of 1 and 0 above it; the reference's own end-to-end baselines are taken at a ratio of 2.
