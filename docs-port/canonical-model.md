# Canonical model

The canonical model is the JSON form of a parsed diagram that both the Rust parsers and the JS exporters in [`tools/port/`](../tools/port/) produce. The differential run compares the two outputs of every input ([0003](./decisions/0003-differential-testing.md)). The Rust structs live in [`crates/ferrigura-model`](../crates/ferrigura-model/); this page fixes the conventions they follow, and their doc comments refer here.

The model is ours, not a dump of the JS DB: where the DB holds something that the input does not contain, or that only rendering needs, the model leaves it out. The rules below say which, and why.

## Shape

- Every model has `"version": 1`, the constant `CANONICAL_VERSION`. It changes when a field is added, removed or changes meaning; the change is described in this page before the code changes.
- Keys are camelCase (`showData`, `accTitle`, `accDescr`), as serde writes them with `rename_all = "camelCase"`.
- Strings are always present. An absent header text is `""`, not `null`, because the DB defaults to `''` and the JS exporters read it that way.
- Numbers are JSON numbers. The differential run compares them with `===`, so `-0` and `0` agree.
- Objects are compared without regard to their key order. Where the order is part of the model, the differential run writes the object as a list of `[key, value]` pairs on both sides (the links of a sequence actor, see [Sequence](#sequence)).

## Text

The text of a value is what the grammar's value rules return from the source, and nothing more:

1. **Line breaks.** CRLF and CR become LF before anything else, in the reference's `cleanupText` and in `preprocess.rs`.
2. **HTML attribute quotes.** Inside a tag, `="x"` becomes `='x'`, as the reference's `cleanupText` does before parsing. The canonical text contains the rewritten form, because it is what the grammar sees.
3. **Strings.** Quotes are removed and escapes resolved as Langium's default value converter does: `\b \f \n \r \t \v \0`, and any other escaped character stands for itself.
4. **Single-line header values** (`title`, `accTitle:`, `accDescr:`) are trimmed, and each run of two or more spaces or tabs becomes one space. The timeline keeps its own rules, see [Timeline](#timeline).
5. **Block `accDescr { … }`** loses the leading whitespace of each line, the trailing whitespace of each line, collapses runs of blanks, and collapses runs of two or more line breaks into one line break. The order is that of the reference's converter.
6. **Markup is raw.** `<br>`, `<b>`, `&`, `&amp;` and every other character stay as written, except the flowchart's node, edge and subgraph texts, which hold the sanitised text (see Sanitisation). Escaping for SVG is the renderer's job, and labels are text in SVG ([0004](./decisions/0004-svg-text-labels.md)).

### Sanitisation

The reference's DB setters pass every text through DOMPurify (`sanitizeText`). DOMPurify returns text that has no `<` unchanged (its fast path, `purify.es.mjs` 2250), so the DB value of `AT&T` is `AT&T`. Text with markup is serialised as HTML: `AT&T <b>bold</b>` becomes `AT&amp;T <b>bold</b>`, and script tags are removed.

The flowchart DB keeps that step, because its layout data (`getData`) carries the sanitised texts: the node text of a bracket or text label, the edge text and the subgraph title are the DB values of the reference. The Rust DB applies [`sanitize.rs`](../crates/ferrigura-syntax/src/sanitize.rs), a pure port of the part of DOMPurify 3.4.12 that these texts reach, and `pnpm port:diff flowchart` and `flowchart-data` compare the result with the reference's DOMPurify on every flowchart input. The `@{ label }` text is not sanitised, as the reference does not sanitise it. The timeline DB sanitises its title and header texts with the same subset, because its setters are the common DB's (see [Timeline](#timeline)); `pnpm port:diff timeline` compares them. The other diagram types, and the header texts of the flowchart, are unchanged: the pie and info setters need a DOM that the canonical model does not provide, and the header setters are replaced in the exporters as before.

For the same reason the exporters do not call the setters for the header texts. They read the grammar's AST and apply the DB's two whitespace rules themselves ([`pie-header.ts`](../tools/port/pie-header.ts)):

- `accTitle`: leading whitespace removed (`^\s+`);
- `accDescr`: whitespace after each line break removed (`\n\s+` becomes `\n`).

These rules never change a value that the grammar gives, because the value rules already remove that whitespace. They stay so that the model follows the DB as written. [`check-header.ts`](../tools/port/check-header.ts) runs the real setters, with jsdom providing the DOM DOMPurify needs, and compares the two on every accepted input whose header texts contain no `&`, `<`, `>` or no-break space. A drift in these rules fails `pnpm port:check-header`.

## Numbers

A JSON number is read to the exact double that JavaScript reads from the same text. The default parser of `serde_json` is not correctly rounded and can be a few ulps off, which changes the last digit of a layout coordinate or a trace value. The workspace therefore enables its `float_roundtrip` feature in the root `Cargo.toml`, and every crate inherits it. The layout inputs and the reference outputs both go through the same reader, so both are exact.

When a value is written back, it is written with the shortest representation that reads back to the same double, which is what JavaScript prints. Nothing else rounds a number on the way.

JSON has no `NaN` and no infinity. A number that is not finite is written as the string `"NaN"`, `"Infinity"` or `"-Infinity"`; the configuration (`value.rs`) and the journey scores (below) do so. The exporters write the same strings, since `JSON.stringify` would write `null`.

## Title precedence

`Diagram.fromText` sets the front matter's title first, then `populateCommonDb` sets the grammar's title, but only when that is non-empty. So:

- the grammar's title wins when it is non-empty;
- otherwise the front matter's title is used;
- otherwise the title is `""`.

The grammar's own rule is that the last `title` line wins, even when it is empty; an empty last title therefore lets the front matter title through. The cases are pinned in [`cases.ts`](../tools/port/cases.ts) and in `crates/ferrigura-syntax/tests/title_precedence.rs`.

The timeline is the exception: its DB has no front matter setter, so only the grammar's title reaches the model, and a blank last title stays blank (see [Timeline](#timeline)).

The info DB has no title setter, so the front matter title does not reach the info model, and the info title is the grammar's alone.

## Front matter

Only the front matter `title` is read. The `config` block and every other key belong to the configuration port (M2), and are not in any model. The front matter itself is removed before lexing. The number of lines it occupied is kept only to report error positions against the original input.

## Pie

- `sections` are in the order of first appearance. A label given again keeps its first value and position, as `addSection` does.
- A negative value is an error, and it is checked before the duplicate test, so a negative value under a repeated label is still an error, as in the reference.
- `showData` is true only when the keyword directly follows `pie`.

## Info

The model keeps `title`, `accTitle` and `accDescr` as the grammar gives them. The keyword `showInfo` is not in the grammar's AST, so it is in no model.

## Flowchart

The flowchart model is the DB state after parsing (`flowDb.ts`), with the names of the DB in camelCase. The full field list and the exclusions are in [flowchart/PLAN.md](./flowchart/PLAN.md), sections 8 and 9; the rules here are the ones the other diagrams do not share.

- **Header texts use the flowchart's own rules.** `accTitle` is the grammar's trimmed value without leading blanks. A block or single-line `accDescr` is the grammar's trimmed value with each line break and the blanks after it reduced to one line break. Blank lines are not collapsed, which differs from the pie and info converter in section 5 above.
- **Title.** The flowchart grammar has no `title` statement, so `title` is the front matter title, or `""`.
- **Labels.** Each label carries its kind as `labelType`: `text`, `string` (double-quoted) or `markdown` (quoted, between backticks). The text is raw; Markdown is parsed at render time with Ferromark.
- **Undefined is `null`.** A DB field the input never sets is `null` (`shape`, `link`, `linkTarget`, `style`, `interpolate`, `animate`, `animation`, `dir`, `length` of an `INVALID` link, the shape data fields). An empty list is `[]`.
- **Order.** Vertices are in creation order, edges in creation order (the order of the DB's `edges` array), subgraphs in closing order (a nested subgraph closes first), with a repeated id merged into its first position. Subgraph `nodes` keep the grammar's order, in which a chain's later nodes come first (`A --> B --> C` gives `[C, B, A]`).
- **Edge ids.** The `id` is always present: the user's `e1@` id, or the generated `L_<start>_<end>_<n>`, where `n` counts the edges already drawn between the same pair. Tooltips are kept by id even when no vertex has that id.
- **Config is not in the model.** `maxEdges` is a parser option (default 500). `securityLevel`, `htmlLabels`, `inheritDir` and `curve` act at render time and are not in the model.

## Timeline

The model is the DB state after parsing, as [timeline/PLAN.md](./timeline/PLAN.md) describes it: `direction`, the header texts, `sections`, and `periods` with their `section`, `period` and `events`, in input order. The timeline's rules differ from the others in these points:

- **Header texts use the grammar's own rules.** The grammar calls `trim()` on `accTitle` and `accDescr` (`timeline.jison` 73-75) and nothing on `title`, so runs of blanks are kept, and a title keeps its leading and trailing blanks. The `accDescr` setter then removes the indentation after each line break.
- **Title.** The front matter title does not reach the model (the timeline DB has no setter). The title is the grammar's last `title` line, even when it is blank.
- **Text is encoded, not decoded.** Entities are encoded before parsing (`#quot;` is `ﬂ°quot¶ß`), and the model keeps the encoded form, as the flowchart does. Sections, periods and events are not sanitised. The title and the header texts are, as above.
- **Left out.** The task `id` (a counter that `clear` does not reset), `type` (always the section) and `score` (always 0).

## Journey

- **Texts are the reader's text.** The journey DB keeps the entities as placeholders (`ﬂ°name¶ß`), and the renderer decodes them as the HTML parser reads a character reference. The model holds that decoded text: `#hearts;` is `♥`, an unknown name stays as `&name;`, and raw text, `&amp;` included, reads as it was written. The rule is in [journey/PLAN.md](./journey/PLAN.md), section 6.
- **Sanitising.** The title and the accessibility texts go through the sanitiser of the flowchart (`sanitize.rs`, with the full HTML5 names of `entities.rs`), before the entities are read. The task names, the people and the sections are not sanitised.
- **Scores** are `Number()` of the text, as a number or one of the sentinel strings above.

## ER

The ER model is the DB state after parsing (`erDb.ts`), with the DB's names in camelCase, as the flowchart's model is. The full field list and the exclusions are in [er/PLAN.md](./er/PLAN.md), sections 4 and 5; the rules here are the ones the other diagrams do not share.

- **Sanitised texts.** The header texts (`accTitle`, `accDescr`), the subgraph titles and the front-matter title are the DB's values after DOMPurify ([`sanitize.rs`](../crates/ferrigura-syntax/src/sanitize.rs)), with the DB's whitespace rules applied after it. Entity names, aliases, attributes, roles, styles and class names are raw.
- **Ids.** An entity id is `entity-<name>-<n>`, where `n` is the number of entities before it. A relationship end is an entity id, or the id of a subgraph of that id when one exists at the point of the statement.
- **Members.** A subgraph's `nodes` are the values the grammar gives inside it, in order: entity names, nested subgraph ids, and the strings of the `classDef`, `class`, `style`, `accTitle` and `accDescr` statements. Empty and repeated strings are dropped. These are the DB's members, not what the renderer draws.
- **Cardinalities** are the DB's strings (`ZERO_OR_ONE`, `ONLY_ONE`, `MD_PARENT`, and the rest); `cardA` is the end on the right of the line, `cardB` the end on the left.
- **Classes.** An entity's `classes` are the names given by `:::` and `class`, without the `default` that the DB starts with. A class keeps its `textStyles`, which the DB derives from its styles.

## Gantt

The gantt model is the DB state after `getTasks()` (`ganttDb.js`), with the field list and the exclusions in [gantt/PLAN.md](./gantt/PLAN.md), section 6. Its own rules:

- **Dates are numbers.** `startTime`, `endTime` and `renderEndTime` are epoch milliseconds in UTC, and `null` is a date the DB leaves unresolved or an Invalid Date. The parser takes the current time as an option (`now`), for the dates that take missing parts from the clock; the differential run gives both sides the same fixed clock under `TZ=UTC`.
- **Sanitised texts.** `title`, `accTitle` and `accDescr` are the DB values after the sanitiser of the flowchart ([`sanitize.rs`](../crates/ferrigura-syntax/src/sanitize.rs)), with the DB's whitespace rules. A `title` statement overrides the front matter title even when it sanitises to empty. Task texts and section names are raw.
- **Links** map a task id to the URL of its `click … href` statement after `sanitizeUrl`, for the tasks that exist when the line is read. Callbacks are not in the model (decision 0001).
- **Statements are raw.** `dateFormat`, `axisFormat`, `tickInterval` and `todayMarker` keep the text after their keyword, blanks included; `includes` and `excludes` are lower case and de-duplicated in first-seen order.

## Sequence

The sequence model is the DB state after parsing (`sequenceDb.ts`), with the field list and the exclusions in [sequence/PLAN.md](./sequence/PLAN.md), section 7. Its own rule:

- **Links keep the key order of a JavaScript object.** The reference lists an actor's links in its menu with `for…in`, so their order is part of the model: the array indices (the canonical integers from 0 to 2^32 − 2) first, ascending, then the other keys in insertion order; a repeated key keeps its place and takes the new value. The model holds them in a `JsObject` ([`js_object.rs`](../crates/ferrigura-model/src/js_object.rs)), which serde writes as a JSON object in that order. The differential run cannot compare that order as an object: `serde_json::Value` sorts the keys, and `same` in [`diff.ts`](../tools/port/diff.ts) ignores their order. The exporter ([`export-sequence.ts`](../tools/port/export-sequence.ts), `Object.entries`) and the canonical example ([`canonical.rs`](../crates/ferrigura-syntax/examples/canonical.rs)) therefore both write `links` as a list of `[key, value]` pairs, which is compared in order. `properties` stay an object, as no output depends on their order.

## Mindmap

The mindmap model is the DB state after the parse (`mindmapDb.ts`): the tree from the first node, with the fields and the exclusions in [mindmap/PLAN.md](./mindmap/PLAN.md), sections 5 and 6. Its own rules:

- **Texts are the DB's values.** `nodeId`, `descr`, `icon` and `class` are the sanitised texts of the DB ([`sanitize.rs`](../crates/ferrigura-syntax/src/sanitize.rs)), and the entity placeholders stay as the DB keeps them (`ﬂ°°35¶ß`), which the renderer decodes.
- **Configuration values keep their type.** `width` and `padding` are the configuration's values as the reference holds them: a number, unless the text gave another type (a string or a boolean), and the doubling of the rounded, rectangle and hexagon shapes turns the value into a number. An array or an object does not replace the default number.
- **Empty and absent are `""`.** `icon` and `class` are `""` when the DB has none, and `root` is `null` when the text has no node.
- **The renderer's data is left out.** `getData`, the sections, the layout and the shape names are the renderer stage.

## Left out, and why

- **The info `version`.** The reference's info DB holds only the Mermaid build version, which is a constant of the build and not part of the input. Putting it in the model would make the differential run depend on the build.
- **Escaped text.** Escaping for SVG belongs to rendering. Sanitising is in the model for the flowchart DB only (see Sanitisation above).
- **Configuration and directives.** `%%{init: …}%%` and the front matter `config` block are read by `ferrigura-syntax`'s `config` module and resolved by `ferrigura-model`'s `config` module (the effective configuration, checked by `pnpm port:diff config`). The grammar treats directives and comments as hidden, so they never reach a model.
- **Source positions.** Errors carry them; models do not.
