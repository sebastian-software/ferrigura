# GitGraph diagram

A gitGraph diagram draws the history of a repository: branches as lanes, commits along them, and the merges and cherry-picks between them, with commit ids, messages and tags. It starts with the keyword `gitGraph`, optionally with a direction (`gitGraph TB:`, `gitGraph BT:`; `LR` is the default). ferrigura parses it with a hand-written lexer and recursive-descent parser that port the Langium grammar `gitGraph.langium` (`crates/ferrigura-syntax/src/git_graph/`), with a DB that follows `gitGraphAst.ts`, and draws it with a renderer of its own (`crates/ferrigura-render/src/git_graph/`) that ports `gitGraphRenderer.ts` and `styles.js`. There is no layout engine: the positions are arithmetic over the commit order and the branch order.

## Model

`GitGraphDiagram` (`crates/ferrigura-model/src/git_graph.rs`), version 1, is the DB's state after parsing.

- `title`, `accTitle`, `accDescr`: the header texts after the sanitizer, `""` when absent. The grammar's `title` wins when it is not empty, else the front matter's title.
- `direction`: `LR`, `TB` or `BT`.
- `currentBranch`: the branch that commits go to.
- `branches`: in the DB map's insertion order, the main branch first, each `{ name, order, head }`. `order` is a number (for the main branch, the `mainBranchOrder` setting) and `head` is the id of the branch's head commit, or `null` while it has none.
- `commits`: in the order their ids were first stored, each `{ id, seq, branch, type, customType, customId, autoId, message, tags, parents }`. `type` is `NORMAL`, `REVERSE`, `HIGHLIGHT`, `MERGE` or `CHERRY_PICK`. `customType` is the `type:` option of a merge and `null` elsewhere; `customId` is `true` or `false` on a merge and `null` elsewhere, because the DB writes it only on merges. `message` is as the input gives it, not trimmed.

`autoId` marks an id that ferrigura generated because the input gave none. The reference draws such ids from `Math.random`, so the differential run replaces each with a placeholder (see Checking it).

The model leaves out the DB's global `seq` counter (each commit keeps its `seq`), `head` (always the head of the current branch, so derivable), the options, the warnings and the render state. Texts are the encoded text the DB holds, and ids, messages, tags, branch names and the like are sanitized ([canonical-model.md](../canonical-model.md) describes the shared conventions).

## Parsing

**Lexer.** `scanner.rs` follows Chevrotain's rules: the first terminal that matches, in the grammar's order, wins, and a keyword is overtaken by `REFERENCE` only when `REFERENCE` is strictly longer (`longer_alt`). Whitespace, comments, directives and front matter are hidden. The terminals are `NEWLINE`, the keywords (`gitGraph` also needs `(?=%%)` or `(?!\S)` after it), `accDescr`, `accTitle`, `title`, `INT`, `STRING` (single or double quoted, escapes take one character, a line break is allowed inside) and `REFERENCE` (`\w([-./\w]*[-\w])?`, ASCII `\w`).

**Grammar.** `grammar.rs` is recursive descent. Statements end with an end of line, a line break or the end of the text. The header does not need one (`gitGraph commit` is valid), and `gitGraph TB` without the colon is an error. Options come in any order; the last `=` assignment wins, and `tag:` accumulates.

- `commit`: `id:` STRING, an optional `msg:` then STRING, `tag:` STRING, `type:` NORMAL, REVERSE or HIGHLIGHT.
- `branch`: a REFERENCE or STRING name, `order:` INT.
- `merge`: a branch, then `id:`, `tag:` and `type:`.
- `checkout` or `switch`: a branch.
- `cherry-pick`: `id:`, `tag:` and `parent:`.

**DB.** `db.rs` follows `gitGraphAst.ts` with the reference's checks and messages, which are part of the behavior. The maps keep insertion order, so branches and commits are vectors with an index, and a repeated commit id replaces the commit at its first position. The error position is the start of the statement that caused the error; the reference gives none.

**Pipeline.** The steps of `Diagram.fromText`: front matter, directives, comment lines, entity encoding and a final line break. The DB reads `gitGraph.mainBranchName` (`main`) and `gitGraph.mainBranchOrder` (0) from the text's configuration when it is cleared (`options_of`).

**Sanitizing.** There are two paths in `sanitize.rs`. Ids, messages, tags, branch names and the merge and cherry-pick ids and parents use the gitGraph section, which has no `securityLevel`, so the reference takes the escape branch of `sanitizeMore`: `<br>` to a placeholder, `<`, `>` and `=` escaped, the placeholder back to `<br/>`, then DOMPurify (`sanitize_section`). The title and the accessibility texts use the top-level configuration, with `securityLevel` pinned to `strict` (`sanitize_text`). Text without `<` comes back unchanged, so `a=b` in a section path becomes `a&equals;b` and `AT&T` stays.

**Auto ids.** `auto_id(seq)` is `<seq>-` and seven hex digits of an FNV-1a hash of `gitgraph-auto-id:<seq>`, read before the increment, so the first id is `0-…`. It is deterministic where the reference's is random. For the geometry harness only, `Options::auto_id_seed` (`RenderOptions::git_graph_seed` in the facade, `--seed` in the `render` example) draws the ids from the mulberry32 generator the reference's page uses, seven values per id, in the order the DB makes them, so the ids are the reference's.

Quirks of the reference that the parser and DB keep on purpose:

- `branch 1` is a parse error (`INT` wins); `branch 1.0` is a `REFERENCE`; `branch 1-0` is a lexer error; `branch commit` is a parse error and `branch "commit"` is accepted; `checkout02` is a `REFERENCE` and `checkout 02` is a lexer error; `commit id:abc` is a parse error, because `id:` takes only a STRING; `commit;` is a lexer error. `titleX` is `title` and then a `REFERENCE`, so it is a parse error.
- A merge is refused when the head commit of the current branch belongs to the other branch (the check compares the commit's `branch`, not the current branch name): `commit; branch b; checkout b; merge main` fails with "Cannot merge branch 'main' into itself.". `merge b` on branch `b` is refused by the next check, before the no-commits check.
- The no-commits messages have no space before "has": `Current branch (main)has no commits`.
- A duplicate commit id only warns, replaces the entry and keeps the commit's own parent, which makes a self-loop. A merge with a duplicate `id:` is an error.
- Cherry-pick parents are `[head, source]` even when they are equal, and its tags drop empty strings, while a commit's tags keep them. The tag defaults to `cherry-pick:<source>`, plus `|parent:<p>` for a merge source. The target of a cherry-pick is always empty, so the reference's "cherry-pick to another commit" branch is unreachable and not ported.
- `branch` sanitizes the name, and `checkout` sanitizes it again.

## Layout and rendering

The drawing is built as a tree of elements (`tree.rs`) that knows the box `getBBox` gives each one, because the reference places the label boxes and the title from boxes and takes the viewBox from the box of the root. `layout.rs` has the positions and the elements, `text.rs` the rows and the line boxes, `palette.rs` the colors of one theme set, `style.rs` the style sheet, and `mod.rs` the root, the title and the viewBox.

- **Passes.** The reference draws the commits twice: first without the elements, to place them, then with them. `draw` measures the branch names, runs `drawCommits` without drawing, draws the branches (which fills the lanes), draws the arrows, and runs `drawCommits` again with drawing. The root therefore has two `commit-bullets` and two `commit-labels` groups; the first of each is empty.
- **Branch positions.** `setBranchPosition` gives each branch a lane `pos`, advancing by 50, plus 40 with `rotateCommitLabel`, plus half the label's width in `TB` and `BT`. The width is that of the same branch's own name, measured before the increment.
- **Commit positions.** Time runs along x in `LR` and along y in `TB` and `BT`. Commit `i` is at `10 + 50 i` without `parallelCommits`; with it, each commit is placed 40 beyond its closest parent. `BT` reverses the commit order, with `setParallelBTPos` placing the commits first when `parallelCommits` is set; it runs in both passes.
- **Bullets.** A commit is a circle of radius 7 (the redux geometry); a highlight is a square; a reverse commit has a cross; a merge has an inner circle; a cherry-pick has a circle with two eyes and two lines. The color index comes from the branch's index, skipping color 0 in the color themes.
- **Labels and tags.** A label is drawn for every commit but a cherry-pick, and for a merge only with its own id, when `showCommitLabel` is set. It is a text at 10 px with a background rectangle, rotated by -45° when `rotateCommitLabel` is set. Tags are polygons with a hole and a text, stacked 20 apart, and are reversed before drawing, so the last tag is nearest the commit.
- **Arrows.** For every commit and every parent, `drawArrow` draws a path with arcs of radius 20 (10 when rerouted) from the parent to the commit. An arrow is rerouted around a commit of the same branch between the two in `seq` order. The lane of a rerouted arrow comes from `findLane`, which is shared with `drawBranches`: the branch lines are drawn before the arrows, so the reroute positions depend on the lanes of the branches. The color is the destination branch's, or the source branch's in the reroute's `else` branch and for a merge whose arrow comes from a branch other than its first parent's.
- **Branches.** With `showBranches`, each branch has a line from 0 to `maxPos` and a label with a background rectangle. The branch order is the stable sort by `order`.
- **Title and root.** The title is a text at 18 px at the middle of the box of what is drawn so far, `titleTopMargin` above it. The viewBox is the box of the root plus `diagramPadding`; with `useMaxWidth` the root has `width="100%"` and a `max-width`.
- **Style sheet.** The rules of `styles.js` for a color theme, scoped to the diagram's id, with every color a custom property. The reference's rule order is kept: the color rules first, then the common rules, so `.commit-highlight-inner` and `.commit-merge` win over the color of their index. Two dark-only differences are custom properties: the cherry-pick eyes (`--gitEye`) and the branch label fill (`--gitLabelFill{i}`).

## Differences from Mermaid

- **Only the redux themes are drawn.** The reference branches on the theme's name in the renderer and in `styles.js` (redux geometry, color themes, dark themes, neo themes). ferrigura ships `redux-color` and `redux-dark-color`; the other names fall back by darkness, so a diagram that sets `theme: default` is drawn with the redux geometry, where the reference uses its default theme's geometry and root font ([0011](../decisions/0011-themes-as-css-variables.md)). The harness applies the same fallback to the reference.
- **Auto ids are deterministic** (see Parsing), except under the harness seed.
- **Labels are SVG text** in the bundled face ([0009](../decisions/0009-svg-text-labels.md), [0010](../decisions/0010-text-measurement-and-fonts.md)). The root text is the theme's `fontSize` (14 px in the redux themes), the commit labels and tags 10 px, the title 18 px.
- **`securityLevel` is pinned to `strict`** for the title path; a `loose` level from front matter or a directive, which skips sanitizing in the reference, is not read.
- **The id of the drop-shadow filter in a tag background is `url(#drop-shadow)`,** as the reference writes it. That id names no element, so no shadow is drawn there.

## Known gaps

- A theme that is not one of the two shipped themes is drawn with a shipped one (see Differences).
- Fonts the bundled faces do not cover need a registered fallback font ([0010](../decisions/0010-text-measurement-and-fonts.md)).

## Checking it

- `pnpm -C tools/port port:diff gitgraph` runs the reference's parser and DB over every input (`export-gitgraph.ts`: the fixtures under `e2e/diagrams/git`, the string literals and docs of the reference, and the cases of `cases.ts`) and compares the canonical models with the generated ids normalized: every commit with `autoId` gets the placeholder `auto#k` (`k` counts the distinct auto ids in model order), and every parent and branch head that equals such an id gets the same one. The JS exporter sets `autoId` from the DB, when the count of `Math.random` calls rose during the commit's call. The same rule is in `canonical.rs`.
- `pnpm -C tools/port port:diff config` covers the `gitGraph` section.
- `pnpm -C tools/port port:geometry gitgraph` compares every element within 0.05 px, with the reference's clock and `Math.random` seeded, and ferrigura given the same seed (`--seed`). A merge or cherry-pick without an `id:` shows its id in its class name, which is part of the geometry key, so the seed decides whether those elements match.
- The Rust specs: `crates/ferrigura-syntax/tests/git_graph_spec.rs` (`gitGraph.spec.ts`), `git_graph_parser.rs` (`packages/parser/tests/gitGraph.test.ts`), `git_graph_populate.rs` and the sanitizing tests in `sanitize.rs`. The ignored tests are those the reference skips (reset, fast-forward merge) and the ones that read renderer keys from `getConfig()`. The in-source tests of `gitGraphRenderer.ts` are unit tests in `layout.rs` and `text.rs`, with the LR cases checking the LR geometry: the reference's LR block expects the TB numbers, because its direction is shared state that its `describe` blocks set while they are collected.

## Learnings

- **Branch positions measure the regular face, the label box the bold one.** The lane step (`setBranchPosition`) measures a label with the class `branch-label` and no index, which the style sheet does not set bold; the label box in `drawBranches` has the index, so it gets the weight 600 of `.branch-label0`. In a vertical graph the two widths differ. Commit labels are bold as well (weight 600 from `noteFontWeight`), and the bold face is the one bundled for 600, as Chromium's matching picks the heavier face above 500.
- **An empty box counts in the union.** A vertical branch line has zero width and reaches `maxPos`, and the viewBox's bottom is the end of that line. Arcs use their tight box; a transformed box is the box of its four corners.
- **`getBBox` width is the sum of rounded advances, height the line box.** Chromium rounds each advance up to 1/64 px and the ascent and descent each to a whole pixel.
- **The expression order matters.** JavaScript evaluates `((w + 10) / 25) * 9.5` left to right in doubles; `translate(-21.939999999999998, 19.520000000000003)` for a 28 px label is what the reference writes, so the Rust arithmetic keeps the order.
- **Reverse and merge classes win by rule order,** not by specificity.
- **Colors wrap differently for commits and branches.** Bullets and arrows use a limit of 8 and, in the color themes, skip index 0; branch lines and labels use the theme's `THEME_COLOR_LIMIT` (12) for the redux geometry, so `branch8` to `branch11` have classes of their own while their commits wrap into 1 to 7.
- **Tags are reversed in place.** The reference reverses the commit's tag list while drawing, so the last tag is the one nearest the commit.
- **The reference's `getBranchesAsObjArray` fallback is dead:** every branch has a number as its order, so the order is the stable sort by `order`.
