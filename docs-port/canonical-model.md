# Canonical model

The canonical model is the JSON form of a parsed diagram that both the Rust parsers and the JS exporters in [`tools/port/`](../tools/port/) produce. The differential run compares the two outputs of every input ([0003](./decisions/0003-differential-testing.md)). The Rust structs live in [`crates/ferrigura-model`](../crates/ferrigura-model/); this page fixes the conventions they follow, and their doc comments refer here.

The model is ours, not a dump of the JS DB: where the DB holds something that the input does not contain, or that only rendering needs, the model leaves it out. The rules below say which, and why.

## Shape

- Every model has `"version": 1`, the constant `CANONICAL_VERSION`. It changes when a field is added, removed or changes meaning; the change is described in this page before the code changes.
- Keys are camelCase (`showData`, `accTitle`, `accDescr`), as serde writes them with `rename_all = "camelCase"`.
- Strings are always present. An absent header text is `""`, not `null`, because the DB defaults to `''` and the JS exporters read it that way.
- Numbers are JSON numbers. The differential run compares them with `===`, so `-0` and `0` agree.

## Text

The text of a value is what the grammar's value rules return from the source, and nothing more:

1. **Line breaks.** CRLF and CR become LF before anything else, in the reference's `cleanupText` and in `preprocess.rs`.
2. **HTML attribute quotes.** Inside a tag, `="x"` becomes `='x'`, as the reference's `cleanupText` does before parsing. The canonical text contains the rewritten form, because it is what the grammar sees.
3. **Strings.** Quotes are removed and escapes resolved as Langium's default value converter does: `\b \f \n \r \t \v \0`, and any other escaped character stands for itself.
4. **Single-line header values** (`title`, `accTitle:`, `accDescr:`) are trimmed, and each run of two or more spaces or tabs becomes one space.
5. **Block `accDescr { … }`** loses the leading whitespace of each line, the trailing whitespace of each line, collapses runs of blanks, and collapses runs of two or more line breaks into one line break. The order is that of the reference's converter.
6. **Markup is raw.** `<br>`, `<b>`, `&`, `&amp;` and every other character stay as written. Nothing is escaped or interpreted: escaping for SVG is the renderer's job, and labels are text in SVG ([0004](./decisions/0004-svg-text-labels.md)).

### Sanitisation

The reference's DB setters pass every text through DOMPurify (`sanitizeText`). DOMPurify serialises text as HTML, so the DB value of `AT&T` is `AT&amp;T`, and it removes script tags. That is a rendering concern, and it needs a DOM, so the canonical model does not apply it.

For the same reason the exporters do not call the setters for the header texts. They read the grammar's AST and apply the DB's two whitespace rules themselves ([`pie-header.ts`](../tools/port/pie-header.ts)):

- `accTitle`: leading whitespace removed (`^\s+`);
- `accDescr`: whitespace after each line break removed (`\n\s+` becomes `\n`).

These rules never change a value that the grammar gives, because the value rules already remove that whitespace. They stay so that the model follows the DB as written. [`check-header.ts`](../tools/port/check-header.ts) runs the real setters, with jsdom providing the DOM DOMPurify needs, and compares the two on every accepted input whose header texts contain no `&`, `<`, `>` or no-break space. A drift in these rules fails `pnpm port:check-header`.

## Title precedence

`Diagram.fromText` sets the front matter's title first, then `populateCommonDb` sets the grammar's title, but only when that is non-empty. So:

- the grammar's title wins when it is non-empty;
- otherwise the front matter's title is used;
- otherwise the title is `""`.

The grammar's own rule is that the last `title` line wins, even when it is empty; an empty last title therefore lets the front matter title through. The cases are pinned in [`cases.ts`](../tools/port/cases.ts) and in `crates/ferrigura-syntax/tests/title_precedence.rs`.

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

## Left out, and why

- **The info `version`.** The reference's info DB holds only the Mermaid build version, which is a constant of the build and not part of the input. Putting it in the model would make the differential run depend on the build.
- **Sanitised and escaped text.** See Sanitisation above: it belongs to rendering.
- **Configuration and directives.** `%%{init: …}%%` and the front matter `config` block are read by `ferrigura-syntax`'s `config` module and resolved by `ferrigura-model`'s `config` module (the effective configuration, checked by `pnpm port:diff config`). The grammar treats directives and comments as hidden, so they never reach a model.
- **Source positions.** Errors carry them; models do not.
