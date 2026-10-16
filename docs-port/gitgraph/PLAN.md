# M6 gitGraph parser: plan

Status: the parser, the DB and the model are ported in `crates/ferrigura-syntax/src/git_graph/` and `crates/ferrigura-model/src/git_graph.rs`. Rendering is not in this plan; the render-level specs are ignored with a reason. Research and every reference line cited here are in [NOTES.md](./NOTES.md), which stays the source for the probes.

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
- The `gitGraphParser.ts` in-source tests and the `gitGraphRenderer.ts` in-source tests are not parser tests: the parser ones are ported in the same files as parser tests, the renderer ones are ignored with the reason `port: renderer, M6 geometry`.
- Config tests (5, gitGraph.spec.ts 1361-1391) check the renderer keys of the `getConfig()` section, which the parser does not read; they are ignored with that reason. The DB reads two keys, `mainBranchName` and `mainBranchOrder`, through `options_of`.

## 10. Differential

`tools/port/export-gitgraph.ts` loads the reference's DB and parser with the detector registered and the DOM for DOMPurify, and runs every collected input (`pnpm port:diff gitgraph`). The inputs are the 135 e2e fixtures, the 39 docs blocks, the spec string literals and the explicit cases of `cases.ts`. The comparison uses the normalisation of section 7. Exceptions go in `exceptions.ts` under `gitgraph`, and none are expected yet.

## 11. Open questions

1. The `default` theme and the renderer: out of M6's parser scope; the renderer is a later step (NOTES 7.1).
2. `securityLevel` from front matter or directives changes the title path (`loose` skips sanitising). Pinned to `strict` here; the inputs of the differential decide whether a fixture needs more.
