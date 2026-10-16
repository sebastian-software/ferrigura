# M6 gitGraph parser: plan

Status: the parser, the DB and the model are ported in `crates/ferrigura-syntax/src/git_graph/` and `crates/ferrigura-model/src/git_graph.rs`. The renderer is ported in `crates/ferrigura-render/src/git_graph/` (section 12). Research and every reference line cited here are in [NOTES.md](./NOTES.md), which stays the source for the probes.

## 1. Scope

In: the Langium grammar (`gitGraph.langium` with `common.langium` and `reference.langium`), the token rules of the Chevrotain lexer as the probes of NOTES 2.3 show them, the AST, the DB (`gitGraphAst.ts`), the DB's sanitisation through `sanitizeText`, the detector, the config keys the DB reads (`mainBranchName`, `mainBranchOrder`).

Out: the renderer, the stylesheet, the themes, the `getBranchesAsObjArray` order for drawing (the model keeps insertion order and the order number), `setOptions` and `getOptions` (dead: no grammar path reaches them), `prettyPrint`, `reset` (commented out in `db`).

## 2. Lexer

Hand-written scanner (`git_graph/scanner.rs`), as ADR 0002 decides: Chevrotain takes the first terminal that matches, in the order below, and a keyword has `longer_alt` REFERENCE, so a longer word wins. The order is the grammar's, with the hidden terminals first (they cannot conflict with the others):

| Order | Token                                               | Rule                                                                                                                                         |
| ----- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | WHITESPACE (hidden)                                 | `[\t ]+`                                                                                                                                     |
| 2     | YAML (hidden), DIRECTIVE (hidden), comment (hidden) | front matter block, `%%{…}%%` with its line break, `%%` to the end of the line                                                               |
| 3     | NEWLINE                                             | `\r?\n`                                                                                                                                      |
| 4     | keywords with `longer_alt` REFERENCE                | literal match; REFERENCE wins only if strictly longer. `gitGraph` also needs `(?=%%)` or `(?!\S)` after it. `:`-keywords cannot be overtaken |
| 5     | ACC_DESCR, ACC_TITLE, TITLE                         | `accDescr` with `:` or `{…}`, `accTitle` with `:`, `title` alone or with its line                                                            |
| 6     | INT                                                 | `0` or `[1-9][0-9]*(?!\.)`, with the backtracking the lookahead gives (a run before `.` gives one digit less, a single digit gives no match) |
| 7     | STRING                                              | `"…"` or `'…'`, escapes take one character, and a line break is allowed inside                                                               |
| 8     | REFERENCE                                           | `\w([-./\w]*[-\w])?`, ASCII `\w`, the longest prefix that ends in `[-\w]`                                                                    |

A `title` followed by a blank takes the rest of its line; `titleX` is TITLE `title` and then a REFERENCE, so it is a parse error (NOTES 2.3). `accTitleX` is a REFERENCE.

Quirks kept (probes, NOTES 2.3): `branch 1` is a parse error (INT wins); `branch 1.0` is a REFERENCE; `branch 1-0` is a lexer error; `branch commit` is a parse error and `branch "commit"` is accepted; `branch commitX` and `checkout02` are REFERENCEs, and `checkout 02` is a lexer error (the 0 is an INT); `commit id:abc` is a parse error because `id:` takes only STRING; `commit;` is a lexer error.

## 3. Grammar and AST

Recursive descent over the token list (`git_graph/grammar.rs`). Statements end with `EOL` (`NEWLINE+` or the end of the text). Options come in any order; the last value of a `=` assignment wins, and `tag:` accumulates.

```
GitGraph:  NEWLINE* header (NEWLINE | TitleAndAccessibilities | Statement)*
header:    'gitGraph' | 'gitGraph' ':' | 'gitGraph:' | 'gitGraph' Direction ':'
Commit:    'commit' ( 'id:' STRING | 'msg:'? STRING | 'tag:' STRING | 'type:' (NORMAL|REVERSE|HIGHLIGHT) )* EOL
Branch:    'branch' (REFERENCE|STRING) ('order:' INT)? EOL
Merge:     'merge' (REFERENCE|STRING) ( 'id:' STRING | 'tag:' STRING | 'type:' … )* EOL
Checkout:  ('checkout'|'switch') (REFERENCE|STRING) EOL
CherryPick:'cherry-pick' ( 'id:' STRING | 'tag:' STRING | 'parent:' STRING )* EOL
```

The header line does not need an `EOL`: `gitGraph commit` is valid. `gitGraph TB` without the colon is an error.

## 4. Text pipeline

As the reference's `Diagram.fromText` (NOTES 3 and `Diagram.ts` 77): front matter and the CRLF and CR rules (`prepare`), directives removed, comment lines removed, entities encoded (`encodeEntities`, the flowchart's `encode_entities`), and a line break appended. The grammar sees that text, so the model keeps the encoded form, as the flowchart model does.

Config: the DB reads `gitGraph.mainBranchName` (default `main`) and `gitGraph.mainBranchOrder` (default 0) when it is cleared. `parse` reads them from `config::read` (front matter, then directives), as `Diagram.fromText` clears the DB after the configuration is set. `securityLevel` is not read (the top-level value is pinned to `strict`, see section 6).

## 5. DB

The DB state and its methods are those of `gitGraphAst.ts` (NOTES 2.4, 2.5). Two state facts make the port simple:

- `head` is always the head of the current branch: each commit, merge and cherry-pick sets both, and `checkout` sets `head` from the branch. So `head` is `branches[currBranch]`, kept as an id.
- The `commits` Map keeps the insertion position of an id; a repeated id replaces the value at its first position. The Rust DB keeps a `Vec` and an index, and replaces in place.

Errors: the 14 messages of NOTES 2.5 are exact (the DB messages are part of the behaviour). The error position is the start of the statement that caused them; the reference gives none.

Quirks kept, each with its probe in NOTES 2.5:

1. A merge is refused when the current branch's head commit belongs to the other branch (check (a) compares the head commit's `branch`, not the current branch name). `commit; branch b; checkout b; merge main` is refused with "Cannot merge branch 'main' into itself.".
2. `merge b` on branch `b` is refused by check (b), before the no-commits check.
3. The no-commits messages have no space before "has": `Current branch (main)has no commits`.
4. A duplicate commit id only warns (`Commit ID … already exists`), replaces the Map entry and keeps the commit's own parent, which makes a self-loop. A merge with a duplicate `id:` is an error (check (g)).
5. Cherry-pick parents are `[head, source]` even when they are equal (`["A","A"]`), and its tags drop empty strings (`filter(Boolean)`), while a commit's tags keep them.
6. The cherry-pick tag default is `cherry-pick:<source>` (plus `|parent:<p>` for a merge source).
7. `branch` sanitises the name, and `checkout` sanitises it again, as the reference does.
8. The target of a cherry-pick is always `''` (the grammar has no target), so the "cherry-pick to another commit" branch is unreachable and not ported.

## 6. Sanitisation

Two paths, both in `ferrigura-syntax/src/sanitize.rs` (NOTES 2.6):

- Ids, messages, tags, branch names, merge and cherry-pick ids and parents use the gitGraph section, which has no `securityLevel`. The reference then takes the escape branch of `sanitizeMore`: `<br>` to a placeholder, `<`, `>` and `=` escaped, the placeholder back to `<br/>`, then DOMPurify (`sanitize_section`).
- The title, `accTitle` and `accDescr` use the top-level configuration with `securityLevel` `strict` (the default, pinned): `removeScript`, which is DOMPurify, then DOMPurify with `FORBID_TAGS: style` (`sanitize_text`, the same as the strict path of the subset, because both passes forbid `style` and the second pass sees the output of the first).

The DOMPurify subset returns text without `<` as it is, so `a=b` (section) becomes `a&equals;b`, and `AT&T` stays. The probes are in the tests of `sanitize.rs` and in `tests/git_graph_sanitize.rs`.

## 7. Auto ids

The reference draws the id of a commit, merge or cherry-pick without an explicit id from `Math.random` (seven hex digits, NOTES 2.7). The port cannot reproduce random values, so it uses a deterministic scheme of the same shape:

`auto_id(seq) = "<seq>-" + hex7(fnv1a64("gitgraph-auto-id:" + seq) & 0x0fff_ffff)`, written as `{:07x}`.

The scheme is documented here and in `git_graph/db.rs`. The model marks each generated id with `autoId: true`. The seq is read before the increment, as the reference does, so the first id is `0-…`.

Seeded ids (the geometry harness only): `Options::auto_id_seed` (`#[doc(hidden)]`, `RenderOptions::git_graph_seed` in the facade, `--seed` in the `render` example) makes the DB draw the generated ids from a mulberry32 generator, the one the reference page uses when `renderReference` seeds `Math.random` (`tools/port/geometry/common.ts`). Each generated id draws seven values, `Math.floor(Math.random() * 16)` per hex digit, in the order the DB makes the ids, so the ids are the reference's. The default path is unchanged. A merge or cherry-pick without an `id:` shows its id in its class name, which is part of the geometry key, so the seed decides whether those elements match; no commit statement of the fixtures lacks an `id:`, so the labels never depend on it. The reference draws no random number before its parse (checked on the 135 fixtures).

Differential normalisation (named, in `tools/port/export-gitgraph.ts` and in the Rust example, the same rule on both sides): every commit with `autoId: true` gets the placeholder `auto#k`, where `k` counts the distinct auto ids in model order (a repeated id keeps the placeholder of its first occurrence; the two sides count the same, as the seq prefix makes ids unique). Every parent and branch head that equals such an id gets the same placeholder. The JS exporter sets `autoId` from the DB: a commit's id was generated when the `Math.random` count rose during its DB call. The random values themselves are not compared.

## 8. Model

`GitGraphDiagram` in `ferrigura-model/src/git_graph.rs`, camelCase, version 1 (`docs-port/canonical-model.md` conventions):

- `title`, `accTitle`, `accDescr`: strings, `""` when absent.
- `direction`: `LR`, `TB` or `BT`.
- `currentBranch`: string.
- `branches`: insertion order of the DB Map, each `{name, order, head}`; `order` is a number, `head` is an id or `null`.
- `commits`: insertion order of the DB Map, each `{id, seq, branch, type, customType, customId, autoId, message, tags, parents}`. `type` is `NORMAL`, `REVERSE`, `HIGHLIGHT`, `MERGE` or `CHERRY_PICK`; `customType` is set on merges only (`null` otherwise); `customId` is `true` or `false` on merges and `null` otherwise (the DB writes it only on merges).

Left out: the DB's global `seq` counter (each commit keeps its `seq`), `head` (derivable, see 5), `options`, the warnings, the render state.

## 9. Specs

- `packages/mermaid/src/diagrams/git/gitGraph.spec.ts` (73 declared, 70 active by the loop count) and `packages/parser/tests/gitGraph.test.ts` (27) are ported as parser and DB tests (`crates/ferrigura-syntax/tests/git_graph_*.rs`), with the reference's test names.
- The three recovery tests (NOTES 2.8) assert that parsing fails, with a position.
- Error tests keep the exact DB message (`parse_error_message` comparison) and check the statement position for the parser errors.
- The `gitGraphParser.ts` in-source tests are parser tests, ported in the same files as parser tests. The `gitGraphRenderer.ts` in-source tests are ported as unit tests of the renderer (`layout.rs`, `text.rs`, section 12); the ignored stubs in `tests/git_graph_render.rs` are gone.
- Config tests (5, gitGraph.spec.ts 1361-1391) check the renderer keys of the `getConfig()` section, which the parser does not read; they are ignored with that reason. The DB reads two keys, `mainBranchName` and `mainBranchOrder`, through `options_of`.

## 10. Differential

`tools/port/export-gitgraph.ts` loads the reference's DB and parser with the detector registered and the DOM for DOMPurify, and runs every collected input (`pnpm port:diff gitgraph`). The inputs are the 135 e2e fixtures, the 39 docs blocks, the spec string literals and the explicit cases of `cases.ts`. The comparison uses the normalisation of section 7. Exceptions go in `exceptions.ts` under `gitgraph`, and none are expected yet.

## 11. Open questions

1. The `default` theme and the renderer: out of M6's parser scope; the renderer is a later step (NOTES 7.1).
2. `securityLevel` from front matter or directives changes the title path (`loose` skips sanitising). Pinned to `strict` here; the inputs of the differential decide whether a fixture needs more.

## 12. Renderer

Ported in `crates/ferrigura-render/src/git_graph/`: `tree.rs` (the elements, and the box each one has to `getBBox`), `text.rs` (rows, `drawText`, the line boxes), `palette.rs` (the colours of a theme set), `style.rs` (`styles.js`, colour theme branch), `layout.rs` (positions, bullets, labels, tags, arrows, branches) and `mod.rs` (the root, the title and the viewBox). The research is NOTES 5. Only the redux themes ship (M6-PLAN), so the classic-theme branches of the renderer and of `styles.js` are not ported; every geometry branch is the redux one.

Result of `pnpm port:geometry gitgraph` (135 fixtures): 134 within tolerance, 1 outside (fixture below), none not rendered, none excluded, none failed in the reference.

Decisions and quirks, each with the reference's line:

1. The text is measured with the theme's sizes, not the constants of the research: the root text is `fontSize` (14px in the redux themes, NOTES 5.12 assumed 16px), the commit labels and tags `commitLabelFontSize` and `tagLabelFontSize` (10px), the title 18px.
2. Branch positions measure the branch name in the regular face, and the label box in the bold face. The lane step (`setBranchPosition`, measured in the first loop of `draw`, renderer 1059-1071) uses a label with the class `branch-label` and no index, which the stylesheet does not set bold; the label box of `drawBranches` has the index, so it gets the weight 600 of `.branch-label0` (styles 91). In a vertical graph the two widths differ.
3. Commit labels are bold (weight 600 from `noteFontWeight`, styles 140); the bold face is the one bundled for weight 600, as Chromium's font matching picks the heavier face above 500.
4. The label box of a commit is measured in the bold face, and its box height is the rounded ascent plus descent of that face (`text::line_height`).
5. `getBBox` union: an empty box (zero width or zero height) is united like any other box. A vertical branch line has zero width and reaches `maxPos`, and the reference's viewBox bottom is that line's end (checked on fixture 20: the viewBox height is 418 with the line, 400 without).
6. The viewBox is the box of the root: the union of the children's boxes, each mapped through its transform as the axis-aligned box of its four corners. Arcs use their tight box (circle extremes inside the sweep).
7. The reverse commit's cross is a path of the two diagonals with its box from the four corners (renderer 226-329).
8. Tags are reversed before drawing (renderer 410-416: `tags.reverse()` on the commit), so the last tag is the one nearest the commit.
9. The branch order is the stable sort by `order` (`getBranchesAsObjArray`, gitGraphAst 457-469); the parser always gives a branch an order (`branch.order ?? 0`).
10. Arrows keep the reference's colour rule: the colour of the destination branch, or of the source branch in the reroute's `else` branch and for a merge whose arrow comes from another branch than its first parent (renderer 676-679).
11. Lanes (`findLane`) are shared between `drawBranches` and `drawArrow`: the branch lines are drawn before the arrows, so the reroute positions depend on the lanes of the branches, as in the reference.
12. The stylesheet keeps the reference's rule order: colour rules first, then the common rules, so `.commit-highlight-inner` and `.commit-merge` win over the colour of their index. The drop shadow filter's id is `url(#drop-shadow)` in the tag background (`dropShadow` of the theme), as the reference writes it: that id names no element, so no shadow is drawn there.
13. Colours are CSS custom properties on the diagram's id (decision 0007). Two dark-only differences are carried by properties: the cherry-pick eyes (`--gitEye`, `#000000` in a dark theme, `#fff` otherwise) and the branch label fill (`--gitLabelFill{i}`, the main background in a dark theme). The dark theme is the theme's name, as the reference's `DARK_THEMES` takes it; with two schemes the dark set is dark.

The specs ported (26 unit tests: 24 in `layout.rs`, 2 in `text.rs`): `should_draw_text` and a rows test, the two `setBranchPosition` tests (LR and TB with the reference's stubbed widths), the 8 TB `getCommitPosition` cases, 3 LR cases, the 8 class type cases, the two BT parallel tests and `add`. The reference's LR block has 8 cases, but they expect the TB numbers: its `dir` is shared state that the describe blocks set while they are collected, so the LR cases here check the LR geometry (x is the time, y is the lane plus 7) on three of the commits. The BT parallel tests use the reference's expected values as they are.

Remaining problem (1 fixture): `git/nightly-check/57-…bottom-to-top.mmd`. Its directive sets `theme: 'default'` with `themeVariables`, and the reference keeps the default theme's root font (`"trebuchet ms", verdana, arial, sans-serif` at 16px, read from its SVG); the labels are measured in that fallback face, which the bundled faces do not cover, so the label boxes and the viewBox differ (the reference's text box of `main` is 35 px wide, ferrigura's Lexica is 32.6). Cause: fonts (no fallback faces yet, M2). Not excluded: the rest of the drawing matches.
