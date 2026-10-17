# Treemap (P2): research notes for the port

Status: research only. Nothing in the repository changed except this file. Reference: the TypeScript at this worktree's HEAD `aa5c4338f`. The grammar is Langium in `packages/parser`, and the diagram is in `packages/mermaid/src/diagrams/treemap/`. The built bundle `packages/mermaid/dist` is from 2026-10-16; the probes ran on it, and its treemap defaults (`chunk-DJLRBVLP.mjs` 5709-5720) match the source.

Legend. A rule with a `file:line` citation was read from that code. **Probed** means the rule was run: in Chromium on the built bundle (`mermaid.render` through `renderReference` and `patchText` of `tools/port/geometry/common.ts`, with the harness's forced settings); through the Langium parser itself (`packages/parser/src/parse.ts`, run with `tsx`, for the AST); or with a library in `node_modules` (d3-hierarchy 3.1.2, d3-format 3.1.0). **Unverified** means inferred from code and not run. The probe scripts and their outputs are in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p2-treemap/`: `probe-ast.mts` and `cases-ast2.mts` (the AST cases, labelled T and C below), `probe-treemap.mts` (26 fixtures and the variants W, labelled `out/`), `probe-treemap2.mts` (X), `probe-treemap3.mts` (Y), `probe-treemap4.mts` (Z), `d3-hier-check.mjs`, and `d3format-check.mjs`.

## 0. Summary

| Item              | Value                                                                                                                                                                                                                                                                          |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Reference sources | Renderer and DB (`diagrams/treemap/`): 976 lines of TypeScript, 1,076 with the spec; grammar and services (`packages/parser/src/language/treemap/`): 294 lines; and the layout and format libraries in `node_modules` (d3-hierarchy 3.1.2, d3-format 3.1.0)                    |
| Specs             | `packages/parser/tests/treemap.test.ts` (17 cases, 238 lines, parser and AST); `packages/mermaid/src/diagrams/treemap/utils.test.ts` (4 cases, `buildHierarchy`). No DB or renderer spec                                                                                       |
| Fixtures          | 26 `.mmd` files: 16 in `nightly-check/`, 10 in `pr-check/`; `visual-check/` is empty. Fixture 19 has emoji and is excluded by the harness rule for uncovered symbols (section 7)                                                                                               |
| Parser            | Langium (Chevrotain lexer, LL parser). A hand-written tokenizer and parser must reproduce its token rules, including quirks the probes show (sections 2.2 and 2.8)                                                                                                             |
| Layout            | `d3-hierarchy` 3.1.2 `treemap()` with the squarify tile (ratio φ), `paddingInner` 10, per-node padding, and integer rounding. Probed: the library reproduces the reference's leaf boxes exactly (`d3-hier-check.mjs`, W05)                                                     |
| Renderer          | SVG built with d3 calls; text sizes are fitted with `getComputedTextLength` in four loops (section 6.6); the root `viewBox` is the bounding box of the visible drawing plus 8 px                                                                                               |
| Biggest risks     | (1) text widths in the fitting loops (section 6.6); (2) the Langium token and parse rules, including the indentation quirks (section 2); (3) the `d3-format` subset the `valueFormat` option reaches (section 6.5); (4) reproducing the `display: none` exclusion from the box |
| Effort            | Medium to large: about 1,500 lines of Rust without tests (section 8)                                                                                                                                                                                                           |

## 1. Sources

| File                                                                                                                                                   | Lines                             | Role                                                                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `packages/parser/src/language/treemap/treemap.langium`                                                                                                 | 9-90                              | Grammar: terminals (17-21, 50-62, 82-89), rules (44-87)                                                                        |
| `packages/parser/src/language/treemap/valueConverter.ts`                                                                                               | 4-43                              | Number and string conversion; `classDef` regex (5); `NUMBER2` comma removal (15); `INDENTATION` length (23)                    |
| `packages/parser/src/language/treemap/module.ts`                                                                                                       | 45-56, 72-91                      | Services: token builder, value converter, validator                                                                            |
| `packages/parser/src/language/treemap/treemap-validator.ts`                                                                                            | 8-60                              | `checkSingleRoot`: the multiple-root check. **Not run** by the mermaid parse path (section 2.7)                                |
| `packages/parser/src/language/treemap/tokenBuilder.ts`                                                                                                 | 3-6                               | Keyword list `['treemap']`                                                                                                     |
| `packages/parser/src/language/generated/grammar.ts`                                                                                                    | 46                                | The compiled grammar, the same rules as the `.langium` file                                                                    |
| `packages/parser/src/parse.ts`                                                                                                                         | 100-103, 133-147                  | `treemap` initialiser; `parse` throws `MermaidParseError` on any lexer or parser error                                         |
| `packages/mermaid/src/diagrams/treemap/parser.ts`                                                                                                      | 13-108                            | `populate`: common DB, classes (28-32), items (35-59), `buildHierarchy` (62), insertion (65-74); `parse('treemap', text)` (94) |
| `packages/mermaid/src/diagrams/treemap/utils.ts`                                                                                                       | 8-64                              | `buildHierarchy`: stack of section nodes (40-42, 58-60); leaves get `children: undefined` (28)                                 |
| `packages/mermaid/src/diagrams/treemap/db.ts`                                                                                                          | 17-95                             | `TreeMapDB`: `getConfig` (28-35), `addNode` (37-44), `getRoot` (46-48), `addClass` (50-70), `clear` (80-87)                    |
| `packages/mermaid/src/diagrams/treemap/renderer.ts`                                                                                                    | 24-535                            | `draw` (24-527) and `getClasses` (529-533). Constants 17-19                                                                    |
| `packages/mermaid/src/diagrams/treemap/styles.ts`                                                                                                      | 7-58                              | Style defaults (7-17), theme variables (22-24), CSS (32-55)                                                                    |
| `packages/mermaid/src/diagrams/treemap/types.ts`                                                                                                       | 4-80                              | `TreemapNode` (4-11), config interface (70-80)                                                                                 |
| `packages/mermaid/src/diagrams/treemap/detector.ts`                                                                                                    | 9-11                              | Detector `/^\s*treemap/` (no word boundary)                                                                                    |
| `packages/mermaid/src/diagrams/treemap/diagram.ts`                                                                                                     | 7-14                              | Definition: parser, `db` getter, renderer, styles                                                                              |
| `packages/mermaid/src/diagrams/common/populateCommonDb.ts`                                                                                             | 4-14                              | Sets `accDescr`, `accTitle`, `title` on the DB when non-empty                                                                  |
| `packages/mermaid/src/diagrams/common/commonDb.ts`                                                                                                     | 16-32                             | Setters: `sanitizeText`, then trim rules                                                                                       |
| `packages/mermaid/src/rendering-util/setupViewPortForSVG.ts`                                                                                           | 5-40                              | Class `flowchart`, bounding box plus padding, `viewBox`                                                                        |
| `packages/mermaid/src/rendering-util/selectSvgElement.ts`                                                                                              | 11-22                             | Selects the root `svg` by id                                                                                                   |
| `packages/mermaid/src/rendering-util/rendering-elements/shapes/handDrawnShapeStyles.ts`                                                                | 18-18, 31-42, 44-51, 53-73, 75-95 | `normalizeStyleList` (18), `compileStyles` (31), `styles2Map` (44), `isLabelStyle` (53), `styles2String` (75)                  |
| `packages/mermaid/src/defaultConfig.ts`                                                                                                                | 323-334                           | Treemap defaults, hand-written (not in `config.schema.yaml`)                                                                   |
| `packages/mermaid/src/setupGraphViewbox.js`                                                                                                            | 23-46                             | `configureSvgSize`                                                                                                             |
| `node_modules/.pnpm/d3-hierarchy@3.1.2/…/src/treemap/index.js`                                                                                         | 18-94                             | `treemap()` driver: `positionNode` (29-51), `round` (25), padding setters (65-91)                                              |
| `…/d3-hierarchy/src/treemap/squarify.js`                                                                                                               | 4-66                              | `phi` (4), `squarifyRatio` (6-53), default `squarify(phi)` (66)                                                                |
| `…/d3-hierarchy/src/treemap/dice.js`, `slice.js`, `round.js`                                                                                           | 1-12, 1-12, 1-5                   | Row layout and rounding                                                                                                        |
| `…/d3-hierarchy/src/hierarchy/index.js`, `sum.js`, `sort.js`, `iterator.js`, `leaves.js`, `eachBefore.js`                                              | see section 6.4                   | Hierarchy, sums (children summed last to first), sort, traversal orders                                                        |
| `node_modules/.pnpm/d3-format@3.1.0/…/src/locale.js`, `formatTypes.js`, `formatDecimal.js`, `formatRounded.js`, `formatPrefixAuto.js`, `formatTrim.js` | see section 6.5                   | The `format` used by `valueFormat`                                                                                             |
| `node_modules/.pnpm/d3-scale@4.0.2/…/src/ordinal.js`                                                                                                   | 1-40                              | `scaleOrdinal`: the domain grows on first call, range index `i % range.length`                                                 |
| `packages/mermaid/src/diagrams/treemap/utils.test.ts`, `packages/parser/tests/treemap.test.ts`                                                         | —                                 | Specs (section 5)                                                                                                              |
| `e2e/diagrams/treemap/`                                                                                                                                | 26 fixtures                       | Screenshot tests                                                                                                               |

The renderer's `d3` import is `d3` 7.9.0 (`renderer.ts` 11), which gives `d3-hierarchy` 3.1.2 and `d3-format` 3.1.0 (the only versions in the store).

## 2. Parser

### 2.1 Pipeline

The text goes through the general preprocessing first (`preprocessDiagram`, `preprocess.ts` 58-65, run by the render pipeline before `Diagram.fromText`, as for the sankey notes): CRLF to LF, front matter (its `config` and `title` are read), directives removed, then `cleanupComments`, which removes every line that starts, after blanks, with `%%` followed by at least one character other than the `{` of a directive (`diagram-api/comments.ts` 6-8), with its leading blanks. A line of only `%%` stays, and a `%%` after text on a line stays. Then `encodeEntities` (`utils.ts` 884-893) turns `#name;` into placeholders with non-ASCII characters. The treemap parser has no wrapper (unlike the sankey one), so the Langium parser receives that text.

`parser.parse` runs `parse('treemap', text)` (`parser.ts` 94). The generic `parse` (`packages/parser/src/parse.ts`) runs the Langium parser, and throws `MermaidParseError` when `lexerErrors` or `parserErrors` is not empty (147). Validation is not run (2.7).

### 2.2 Terminals

Chevrotain takes the first terminal that matches, in definition order (unverified as Chevrotain's documented rule; the probes below agree with it). The terminals of `treemap.langium`, in order:

| #   | Terminal (line)        | Pattern                                                        | Hidden | Notes                                                             |
| --- | ---------------------- | -------------------------------------------------------------- | ------ | ----------------------------------------------------------------- |
| 1   | `BOOLEAN` (17)         | `true` \| `false`                                              | no     | Not used by any rule                                              |
| 2   | `ACC_DESCR` (19)       | `[\t ]*accDescr(?:[\t ]*:(...)\|\s*{...})`                     | no     | Leading blanks are part of the token                              |
| 3   | `ACC_TITLE` (20)       | `[\t ]*accTitle[\t ]*:...`                                     | no     |                                                                   |
| 4   | `TITLE` (21)           | `[\t ]*title(?:[\t ][^\n\r]*?(?=%%)\|[\t ][^\n\r]*\|)`         | no     | Blank title allowed (T26)                                         |
| 5   | `TREEMAP_KEYWORD` (50) | `treemap-beta` \| `treemap`                                    | no     | `treemap-beta` listed first                                       |
| 6   | `CLASS_DEF` (52)       | `classDef\s+([a-zA-Z_][a-zA-Z0-9_]+)(?:\s+([^;\r\n]*))?(?:;)?` | no     | Name at least 2 characters; `\s` crosses line breaks (2.8)        |
| 7   | `STYLE_SEPARATOR` (53) | `:::`                                                          | no     |                                                                   |
| 8   | `SEPARATOR` (54)       | `:`                                                            | no     |                                                                   |
| 9   | `COMMA` (55)           | `,`                                                            | no     |                                                                   |
| 10  | `INDENTATION` (58)     | `[ \t]{1,}`                                                    | no     | Before `WS`, so every run of blanks is this token outside strings |
| 11  | `WS` (60)              | `[ \t]+`                                                       | yes    | Never produced, since `INDENTATION` matches first                 |
| 12  | `ML_COMMENT` (61)      | `%%[^\n]*`                                                     | yes    |                                                                   |
| 13  | `NL` (62)              | `\r?\n`                                                        | yes    |                                                                   |
| 14  | `ID2` (82)             | `[a-zA-Z_][a-zA-Z0-9_]*`                                       | no     | Class names                                                       |
| 15  | `NUMBER2` (84)         | `[0-9_.,]+`                                                    | no     | Values                                                            |
| 16  | `STRING2` (89)         | `"[^"]*"` \| `'[^']*'`                                         | no     | Names; no escapes                                                 |

### 2.3 Grammar

```
Treemap      : TREEMAP_KEYWORD ( TitleAndAccessibilities | TreemapRows+=TreemapRow )*   (44-49)
TitleAndAccessibilities : (accDescr=ACC_DESCR | accTitle=ACC_TITLE | title=TITLE)+        (13-15)
TreemapRow   : indent=INDENTATION? ( item=Item | ClassDef )                             (64-65)
ClassDef     : CLASS_DEF                                                                  (68-69)
Item         : Leaf | Section                                                             (71-72)
Section      : name=STRING2 ( STYLE_SEPARATOR classSelector=ID2 )?                        (75-76)
Leaf         : name=STRING2 INDENTATION? (SEPARATOR | COMMA) INDENTATION? value=MyNumber
               ( STYLE_SEPARATOR classSelector=ID2 )?                                    (78-79)
MyNumber     : NUMBER2                                                                    (87)
```

Notes. `TitleAndAccessibilities` may appear before or between rows. A `Leaf` needs a separator, `:` or `,`, between its name and value, and a `Section` takes no value. The optional `INDENTATION` tokens around the separator are what the grammar tolerates; any other blank is an error.

### 2.4 Value conversion

- `NUMBER2`: commas are removed, then `parseFloat` (`valueConverter.ts` 15). So `1,000.5` is 1000.5, `1,2,3` is 123, `1_000` is 1 (`parseFloat` stops at `_`), `1..2` is 1, `1.` is 1, `.5` is 0.5, `007.50` is 7.5 (probed: T07, T08, T09, T13, T14, T10, T49, T50).
- `STRING2`: the first and last characters are removed (`valueConverter.ts` 18-21). So `''` gives an empty name (T19), and `'single quoted'` works (T21). A `"` cannot appear inside `"…"` (T20 is an error).
- `INDENTATION`: the number of characters (`valueConverter.ts` 22-23). A tab counts as one.
- `ClassDef`: `classDefRegex` (5) gives `className` and `styleText`; the text is `undefined` when no style text follows.

### 2.5 Accepted and rejected inputs (probed through the parser)

| Input (`\n` between lines)                                | Result                                                                                                     | Probe                   |
| --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------- |
| `treemap`, `treemap-beta`                                 | Empty tree                                                                                                 | T01, T02                |
| `treemap` followed by `"Root"`                            | Section named `Root`                                                                                       | T03                     |
| Leaf with `:` or `,` separator                            | Leaf                                                                                                       | T05, T42                |
| Unquoted name `A: 1`                                      | Error                                                                                                      | T06                     |
| `-5`, `1e3`, `1 2`, `"A":`, `"A"  1`                      | Error: `-` is not a character of any terminal; `e3` is an identifier after the number; a missing separator | T11, T12, T16, T43, T44 |
| `"A": 1:::cls` (class on a leaf)                          | Leaf with `classSelector`                                                                                  | T39                     |
| A value of only separators: `"A": .`, `"A",.`, `"A": ._.` | Leaf with value `NaN` (`parseFloat` of no digits); `"A": _` alone is an error                              | N01, N05, N06, N02      |
| `"A"` followed by `:::cls` on the same line (no blank)    | Section with `classSelector`                                                                               | T18                     |
| `"A" :::cls` (blank before `:::`)                         | Error                                                                                                      | T54                     |
| Two classes, `1:::a:::b`                                  | Error                                                                                                      | T53                     |
| `treemapX`                                                | Error: the keyword is followed by `X` (the detector still accepts it, `detector.ts` 10)                    | T41                     |
| Two roots `"Root1"` and `"Root2"`                         | Both accepted                                                                                              | T22                     |
| `title T`                                                 | Title `T`; `title` with nothing after it gives `""`                                                        | T25, T26                |
| `accTitle: x` and `accDescr: y`                           | Both kept                                                                                                  | T27                     |
| `"A": 1 %%x` (comment after the value)                    | Accepted; see the trailing-blank rule (2.6)                                                                | T60                     |

### 2.6 Indentation (the trailing-blank rule)

Indentation is the length of an `INDENTATION` token that starts a row. Two consequences, both probed:

1. **Trailing blanks become the next row's indent.** `INDENTATION` is a token, so the blanks after a value or a name on one line are a token before the newline (which is hidden). If the next row has no indentation, the parser reads that token as the next row's indent: `"A": 1 \n"B": 2` gives `B` an indent of 1 (C16, C18, C19, T60).
2. **Two indentation tokens in a row are an error.** `"A": 1  \n  "B": 2` (trailing blanks, then indented row) fails (C17). With the pipeline's preprocessing (which does not touch trailing blanks), the same text fails in the diagram (W08, W09, W11).

A trailing blank at the end of the file is an error too (T37: `"A" : 1 \n`).

### 2.7 Errors and validation

Errors are the Langium messages wrapped by `MermaidParseError` (`parse.ts`): `Parsing failed:` followed by `Lexer error on line L, column C: unexpected character: …` or `Parse error on line L, column C: Expecting …` (the probes show the unknown positions as `?` where Chevrotain gives none: T24, T37). The multiple-root validator (`treemap-validator.ts` 30-60) is registered in the services (`module.ts` 88, and `treemap-validator.ts` 15), which means Langium's document validation would run it. `parse()` does not run that step, so two roots are accepted and drawn (X01, X02, W02, T22). Its only reference in `packages/` is that registration (grep).

### 2.8 Quirks to reproduce

| Quirk                                                                  | Test input                                        | Reference result                                                                                                                      |
| ---------------------------------------------------------------------- | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Name must be quoted                                                    | `treemap\nA: 1`                                   | Error (T06)                                                                                                                           |
| Empty name                                                             | `treemap\n""`                                     | Section named `""` (T19)                                                                                                              |
| Digits, commas, dots and underscores in values                         | `treemap\n"A": 1,2,3`                             | 123 (T08)                                                                                                                             |
| `classDef` name at least two characters, starting with a letter or `_` | `classDef c fill:red`                             | Error (C06-C08 for `c`, `a-b`, `1a`); `_x` is accepted (C07)                                                                          |
| Bare `classDef` followed by a line                                     | `classDef cc\n"A":::cc`                           | The next line is the style text: `styleText` is `"A":::cc`, and `A` disappears (C02). The `\s+` of `CLASS_DEF` crosses the line break |
| Bare `classDef` at the end                                             | `…\nclassDef cc`                                  | The leaf gets `style=": !important"` (Z01)                                                                                            |
| `classDef` ends at `;`; commas are kept                                | `classDef cc fill:red;` / `fill:red, stroke:blue` | `fill:red`; `fill:red, stroke:blue` (C03, C04)                                                                                        |
| Duplicate `classDef` accumulates                                       | two `classDef cc` lines                           | Styles of both: `fill:red !important;stroke:blue !important` (Z07, C05)                                                               |
| `classDef` after its use works                                         | `"A":::cc` before `classDef cc fill:red`          | Styled (Z02)                                                                                                                          |
| Trailing blanks feed the next indent                                   | `"A": 1 \n"B": 2`                                 | `B` at indent 1 (C16)                                                                                                                 |
| Two indent tokens are an error                                         | `"A": 1  \n  "B": 2`                              | Error (C17, W08)                                                                                                                      |
| Comment line removes its blanks                                        | `  %% c\n"A": 1` (through the pipeline)           | Removed (W10); the bare parser sees the blanks (C20)                                                                                  |
| Tabs count as one                                                      | `\t"A": 1\n\t\t"B": 2`                            | Indents 1 and 2 (T34)                                                                                                                 |
| Empty lines between rows                                               | `"A": 1\n\n"B": 2`                                | Both leaves (T33, C09)                                                                                                                |
| Two roots accepted                                                     | `"Root1"\n"Root2"`                                | Both drawn (W02, X01, X02)                                                                                                            |
| Names are not sanitised                                                | `"<b>C</b>"`                                      | Shown as the characters `<b>C</b>` (Z03)                                                                                              |
| Unknown class on a row                                                 | `"A":::nope`                                      | Accepted; no style (W16, W24)                                                                                                         |
| Keyword prefix                                                         | `treemapX`                                        | Error (T41)                                                                                                                           |
| Value of only separators                                               | `"A": .`                                          | Leaf with value `NaN`: the layout counts it as 0, and the leaf is zero-width with its label hidden (NN1, through the pipeline)        |

## 3. DB

`TreeMapDB` (`db.ts`) keeps:

- `nodes`: every node in the order `addNode` is called, which is depth-first, parents before children (`parser.ts` 65-74);
- `outerNodes`: the nodes of level 0, in order (`addNode`, 40-42), and `root` (the first of them; unused by the renderer);
- `classes`: a `Map` of `{ id, styles, textStyles }`;
- the common header texts (`commonDb.ts`).

Calls, in order (`parser.ts` 13-75): `populateCommonDb` (the grammar's `title`, `accTitle`, `accDescr`, each only when non-empty, `populateCommonDb.ts` 4-14); then `addClass` for every `ClassDefStatement` row, in order (28-32); then the items, each giving `{ level, name, type, value, classSelector, cssCompiledStyles }` (35-59); then `buildHierarchy` (62); then `addNode` for each node (65-74).

- `level` is `row.indent` or 0 (`parser.ts` 42), so `level` counts blanks.
- `name` is `String(item.name)`, and it is not sanitised (`getItemName`, 82-84).
- `cssCompiledStyles` is `db.getStylesForClass(classSelector)` when that list is not empty (46-47). A class that is used but never defined gives `undefined`.

`addClass(id, style)` (50-70): the style text has `\,` protected, then each `,` becomes `;`, then the text is split on `;`. Every piece goes to `styles` (even `''`); a piece that `isLabelStyle` also goes to `textStyles`. Calling it again for the same id appends to the existing lists (Z07). Consequence (probed, Z01): a bare `classDef` gives styles `['']`, and the leaf style becomes `: !important`.

`getRoot()` (46-48) returns `{ name: '', children: outerNodes }`. `getConfig()` (28-35) merges `DEFAULT_CONFIG.treemap` with the `treemap` section of the effective configuration and calls `cleanAndMerge`. `clear()` (80-87) resets the common header texts and every list.

`buildHierarchy` (`utils.ts` 8-64): each item becomes a node with `name`, `children` (`undefined` for a `Leaf`, `[]` for a `Section`), `classSelector`, `cssCompiledStyles` and, for a leaf, `value`. A stack holds the sections; a row pops every section whose level is at least its own level, and becomes the child of the top section, or a root when the stack is empty. Leaves are never pushed. Probed: W05 (`"Root"` with leaves `A` and `B` at indent 2).

## 4. Canonical model proposal

The DB after parsing, with the names of the port's model in camelCase and `version: 1`:

```json
{
  "version": 1,
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "classes": [{ "id": "cc", "styles": ["fill:red"], "textStyles": [] }],
  "tree": {
    "name": "",
    "children": [
      {
        "name": "Root",
        "children": [
          { "name": "A", "value": 10, "classSelector": "cc", "cssCompiledStyles": ["fill:red"] },
          { "name": "B", "value": 20 }
        ]
      }
    ]
  }
}
```

- `title`: the grammar's title if non-empty, else the front matter's (`Diagram.fromText`, then `populateCommonDb`). Probed: Z04 (`Grammar` wins), Z05 (blank grammar title keeps `Front`), W12 (front matter alone is drawn). The sankey notes differ (there nothing is drawn). This is the same rule as [canonical-model.md](../canonical-model.md#title-precedence).
- `accTitle`, `accDescr`: sanitised as the common setters do (`commonDb.ts` 16-23), and `""` when unset.
- `classes`: the DB's map as a list in insertion order, with `styles` and `textStyles` as stored (including `""` entries).
- `tree`: the hierarchy as `buildHierarchy` gives it. The model keeps the DB's distinction between `children` absent (a leaf) and `[]` (a section with no rows). The layout and the renderer treat the two alike (d3 sees no children in either case, and `renderer.ts` 152-157 and 164 test `length > 0`), so the distinction is kept only for parity with the DB. A `value` is kept only for leaves.
- Names: raw. Values: JSON numbers. A value can be `NaN` (`"A": .`, N01), which the model writes as the string `"NaN"` by the rule of [canonical-model.md](../canonical-model.md#numbers).
- Left out: `levels`, `root`, the flat `nodes` list (it is the tree in DFS order, so it adds nothing), and the configuration, which the configuration module owns.

## 5. Specs

- `packages/parser/tests/treemap.test.ts` (238 lines, 17 cases): empty and section trees, leaves (`"Child1" , 100`, `"Child2" : 200`), `'string'` and number values (`123.45`), two roots (`parserErrors` is empty; the validator is not called), `title`, `accTitle`, `accDescr`, the combination of the three, `classDef` (three cases that check only the type `ClassDefStatement` and skip the error check), two class-assignment cases (`class myNode myClass`, which check only that a value is returned), and `:::` on a section and on a leaf. The probes show `class A c` is an error (T32).
- `packages/mermaid/src/diagrams/treemap/utils.test.ts` (100 lines, 4 cases): `buildHierarchy` for a flat list, an empty list, root nodes only, and complex nesting.

The specs were not run (vitest writes caches under `node_modules`, and the worktree was to stay as it was). The port should run them as written, and port the four `buildHierarchy` cases as the DB's own tests.

## 6. Renderer

### 6.1 Pipeline and the root element

`draw` (`renderer.ts` 24-527) runs in this order:

1. `getConfig()` of the DB (26), the title (28), the root (29), and `themeVariables` of the effective configuration (30). With no root it returns (31-33); the root always exists, so this never happens.
2. The root `svg` is selected by id (38, `selectSvgElement.ts`).
3. Sizes (40-44): `width = nodeWidth ? nodeWidth * 10 : 960`, `height = nodeHeight ? nodeHeight * 10 : 500`, `svgHeight = height + titleHeight` (`titleHeight` is 30 with a title and 0 without), and the `viewBox` is `0 0 svgWidth svgHeight`, then `configureSvgSize` (47-48). Defaults (`nodeWidth` 100, `nodeHeight` 40): 1000 by 400.
4. The value formatter (51-77), the three colour scales (79-123), and the title (125-135).
5. The container `g` (138-141), the hierarchy and the layout (144-161), the sections (163-284), the leaves (311-455), the values (457-524), and the viewport (525-526).

The root `svg` (`mermaidAPI.ts` 411-419) starts with `width="100%"`, `xmlns` and an empty `g`. The final `viewBox` is set by `setupViewPortForSVG` (6.8), which also sets `class="flowchart"` on the root (`setupViewPortForSVG.ts` 12).

### 6.2 Element order (probed on W05 and fixtures)

```
svg#id  width="100%" xmlns  viewBox  style="max-width: …px;"  class="flowchart"  role  aria-roledescription="treemap"
  style                                    (the CSS of 6.7; inserted first, mermaidAPI.ts 596-600)
  g                                        (empty; mermaidAPI.ts 419)
  text.treemapTitle  x=svgWidth/2 y=15 class text-anchor="middle" dominant-baseline="middle"   (only with a title, 127-134)
  g.treemapContainer  transform="translate(0, titleHeight)"                                       (138-141)
    g.treemapSection  transform="translate(x0,y0)"    one per node with children, BFS order  (165-171)
      rect.treemapSectionHeader  width  height="25"  class fill="none" fill-opacity="0.6" stroke-width="0.6" style  (174-188)
      clipPath id="clip-section-<id>-<i>" > rect width max(0, w-12) height 25                  (191-196)
      rect.treemapSection.section<i>  width height fill stroke fill-opacity stroke-width="2" stroke-opacity="0.4" style  (198-217)
      text.treemapSectionLabel  x="6" y="12.5" dominant-baseline font-weight="bold" clip-path style  (219-239)
      text.treemapSectionValue  x=w-10 y="12.5" text-anchor="end" dominant-baseline font-style="italic" style  (287-309, only with showValues)
    g.treemapNode.treemapLeafGroup.leaf<i>x [ .<classSelector>]  transform  one per leaf, DFS order (324-332)
      rect.treemapLeaf  width height class fill style fill-opacity="0.3" stroke stroke-width="3"  (335-355)
      clipPath id="clip-<id>-<i>" > rect width max(0,w-4) height max(0,h-4)                    (358-363)
      text.treemapLabel  x=w/2 y=h/2 style clip-path content name                                (366-381)
      text.treemapValue  x=w/2 y (dynamic) style clip-path content value                         (458-477)
```

Points a port must reproduce:

- The section `class` is `treemapSection section<i>`, where `i` is the index in the BFS list (202-203).
- The leaf `class` ends in `x`: `leaf0x`, and `leaf1 ccx` for a class `cc` (the class is appended before the `x`, 330). Probed (W07).
- The root is a section (depth 0) with `display: none` on its rectangle, header, label and value (182-188, 210-214, 228-232, 297-301), so it is not in the box (6.8). Its name is `''`, and its label is `''` (225).
- A node with no children is a leaf, including a section with no rows (`Section` with `children: []`): a zero-value leaf (W03).
- An empty `treemap` draws the root as a leaf: a rectangle 1000 by 400 (W01). The fill is `transparent` (its colour is the first call of the scale, 6.3).
- The `clip-…` ids use the diagram id and the index in the DOM list of their kind (`i` in the `.attr('id')` callbacks, 193 and 360).

### 6.3 Colours

Three ordinal scales (`renderer.ts` 79-123):

| Scale             | Range                                                           | Used by                                                                   |
| ----------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `colorScale`      | `transparent`, `cScale0` … `cScale11` (13 entries)              | Section fill (205); leaf fill and stroke use the parent's name (343, 353) |
| `colorScalePeer`  | `transparent`, `cScalePeer0` … `cScalePeer11`                   | Section stroke (207)                                                      |
| `colorScaleLabel` | `cScaleLabel0` … `cScaleLabel11` (12 entries, no `transparent`) | Section label and value (235, 304), leaf label (375) and leaf value (470) |

An ordinal scale takes the index of its first call with each key, then the range entry `index % length` (`ordinal.js` 1-40, probed through the output). The calls, in order:

- Sections, in BFS order (`descendants()`), for the fill, the stroke, the section label and the section value. The root's name `''` is the first call, so it takes `transparent`, and the first real section takes `cScale0`. Probed: W05 (`Root` is `#f4a8ff`), fixture 14 (`Products` `#f4a8ff`, `Electronics` `#46ecd5`, `Furniture` `#ffb86a`).
- Leaves, in DFS order, for their labels and values (`colorScaleLabel` with their own names). A leaf's fill and stroke use the parent's name, which is already in the scale.

Beyond the scales, a `classDef` adds `style` text to the element (6.5): `fill`, `stroke`, `stroke-width` and `stroke-dasharray` go to the `style` attribute with `!important` (`styles2String`, `handDrawnShapeStyles.ts` 75-95), and a label property (`color`, `font-*`, `text-*` and the others of `isLabelStyle`, 53-73) goes to the label. Probed: fixture 12 (`fill:#ff0000 !important`), fixture 14 (the label `fill:#ffffff !important`, from `color:#ffffff` by `.replace('color:', 'fill:')`, `renderer.ts` 238, 307, 378, 473).

The theme variables come from `themeVariables` (`renderer.ts` 30) and the CSS from `styles.ts` 22-24 (6.7).

### 6.4 Layout

Hierarchy and layout (`renderer.ts` 144-161), with the d3 sources:

- `hierarchy(root)` (`hierarchy/index.js` 15-43) builds nodes from `children`; a `children` array with no entries makes no children, so such a node is a leaf.
- `.sum(d => d.value ?? 0)` (`hierarchy/sum.js` 1-9): each node's value is its own value plus its children's values, summed from the last child to the first (`while (--i >= 0)`). The order matters for floating-point sums.
- `.sort((a, b) => (b.value ?? 0) - (a.value ?? 0))` (`sort.js`): each node's children, descending by value. JavaScript's sort is stable.
- `treemap()` (`treemap/index.js`): `size([1000, 400])` for the defaults, `round(true)`, `paddingInner(padding)` with `padding` = 10 by default, and for each node with children, `paddingTop` 35 (25 + 10), `paddingLeft`, `paddingRight`, `paddingBottom` 10, else 0 (`renderer.ts` 149-158).
- `positionNode` (`treemap/index.js` 29-51): the node's box is shrunk by its parent's half padding `p = paddingInner/2`, then for a node with children, `p` is stored for depth+1, and the children box is the node's box with its own padding (`paddingLeft - p`, and so on) and the tile is laid out inside it. An inverted box is collapsed to its midpoint (35-36, 47-48).
- The tile is `squarify` with ratio φ = (1+√5)/2 (`squarify.js` 4, 6-53, 66): rows of nodes, each row laid out with `dice` (when the row is wider than tall, `dice.js`) or `slice` (`slice.js`). A zero value is skipped to find the first node of a row (line 28).
- `round` (`treemap/round.js` 1-5): `Math.round` of the four coordinates of every node, after all positions (`index.js` 25). Math.round is JavaScript's.

Probed: `d3-hier-check.mjs` runs `d3-hierarchy` 3.1.2 on the W05 tree with the calls above and gives `Root` at (10, 35) with 980 by 355, `B` at (20, 70) with 683 by 310, `A` at (713, 70) with 267 by 310, the same as the reference SVG. The leaves are in DFS order (`leaves()`: `eachBefore`), the sections in BFS order (`descendants()`: the iterator, `iterator.js` 1-15).

### 6.5 Values and `valueFormat`

`config.valueFormat || ','` (`renderer.ts` 54) goes through these branches (51-77):

- `'$0,0'`: `'$' + format(',')(v)`, which keeps the decimals (W17: `$1,234,567.891`);
- starts with `$` and has a comma: `'$' + format(',' + precision)(v)`, where the precision is the first `.n` of the string (62-64; fixture 8d `$,.2f` gives `$1.6e+6`, Y04);
- starts with `$`: `'$' + format(rest)(v)`, with `rest` the string without `$` (65-68; `$.2f` gives `$369.96`, fixture 8c; `$` alone gives `$1234567`, Y02);
- else `format(s)`.

Only the last branch (`format(formatStr)`, line 71) is inside the `try` (52-77): an invalid specifier there, such as `xyz` (W20), falls back to `format(',')` (line 76). The three `$` branches build their formatter inside the value callback (lines 59, 64, 68), outside the `try`, so an invalid specifier after `$` throws while the values are drawn, and the whole render fails (Y01: `invalid format: xyz`). The port must reproduce both.

The formats the d3 library gives, probed on `d3-format` 3.1.0 (`d3format-check.mjs`): `,` on `1234567.891` is `1,234,567.891`; `.2f` on `1.005` is `1.00` (binary rounding); `.2s` on `1234567` is `1.2M` and on `0.0000123` is `12µ`; `,.2` on `1645789.25` is `1.6e+6`; `.1%` on `0.1234` is `12.3%`; `.0f` on `2.5` is `3`. The default `''` type is an alias for `.12~g` (`locale.js` 40-41: the precision becomes 12 and the trailing zeros are removed), so `,` keeps up to 12 significant digits. Negative numbers use `−` (U+2212) (`locale.js` `minus`); a treemap value cannot be negative, since `NUMBER2` has no sign (Y05).

Value text: `d.value ? valueFormat(d.value) : ''` (295, 477), so zero shows nothing (X06). The section value is `d.value` after the sum (W05: `3.5` for the root).

### 6.6 Text measurement (where the widths decide)

Four places read `getComputedTextLength()` (Chromium's advance width of the text, in the element's current font):

1. **Section label truncation** (`renderer.ts` 240-284). The label is the name in bold at 12 px (`font-weight="bold"`, 226, and the style at 234). The space is the header width minus 6 (left) and 6 (right), or minus the value's width (30) and a gap (10) when `showValues` and the value is non-zero (251-261), with a minimum of 15 (262-263). If the text is wider, `…` is appended after shortening one character at a time (268-282); if even `…` does not fit, the text is emptied (273-275). Probed: fixture 19, `🟡 Low Activity (15%)` shows as `🟡 Lo...`.
2. **Leaf label font size, step 1** (`renderer.ts` 401-407): while the text is wider than `nodeWidth - 2 * labelPadding`, the font size drops by 1, down to `minLabelFontSize`.
3. **Leaf label and value heights** (410-436): the value font is `max(minValue, min(baseValue, round(label * 0.6)))` and the label shrinks while the two lines do not fit the height.
4. **Value visibility** (514-522): a value wider than the leaf, or below the bottom, or at the minimum size, is hidden with `display: none`; the value's font is `round(labelSize * 0.6)` clamped (498-501).

Constants (`renderer.ts` 316-322): the complex layout is used when there are more than 20 leaves (314; probed W23, 21 leaves, labels at 16 px and values at 10 px). Simple: base label 38, base value 28, minimum label 8, minimum value 6, padding 4, threshold 10, spacing 2. Complex: 16, 14, 4, 4, 2, 8, 1.

A leaf's label is `clip-path`ed, and clipping does not change `getBBox`, so the box of a clipped label is its full text.

The ferrigura port needs `getComputedTextLength` for the bold face at 12 px, and for the regular face at the sizes 4 to 38 px, in the same units Chromium uses (the M6 rules). A width off by a fraction of a pixel can change a fit decision, so the port's text widths must match Chromium's exactly enough that no threshold flips (section 8).

### 6.7 Style sheet

`styles.ts` 19-56 builds the CSS for the treemap's classes:

- `themeVariables` = the default theme, overridden by the effective `themeVariables` (22-24). Probed: the label fill is `#28253D` under redux-color and `#ccc` under the dark theme (W14), and the title fill `#000000` and `#F9FFFE`;
- rules: `.treemapNode.section` and `.treemapNode.leaf` (dead: no element has these classes), `.treemapLabel` (fill, `font-size: 12px`), `.treemapValue` (fill, `10px`), `.treemapTitle` (fill, `14px`).

The `TreemapStyleOptions` defaults (`styles.ts` 7-17) are always used: `mermaidAPI.ts` 327-331 passes only the theme variables and `theme` and `look` to the styles provider, so no configuration key reaches them (read). The root `#id` prefix comes from `stylis` as for every diagram (`mermaidAPI.ts` 211-228).

### 6.8 Viewport

`setupViewPortForSVG(svg, diagramPadding, 'flowchart', useMaxWidth || false)` (`renderer.ts` 525-526):

1. `class="flowchart"` (`setupViewPortForSVG.ts` 12);
2. the bounds `svg.getBBox()` (29) of everything that is rendered: `display: none` elements are not in the box;
3. `width = bounds.width + 2 * padding`, `height = bounds.height + 2 * padding` (31-32);
4. `configureSvgSize(svg, height, width, useMaxWidth)` (18): `width="100%"` and `style="max-width: Wpx;"` when `useMaxWidth` (default true), else `width` and `height` attributes;
5. `viewBox = "(x - padding) (y - padding) width height"` (38-40).

Probed: W05 has `viewBox="2 27 996 371"` (box `[10, 35, 980, 355]` plus 8). W21 (`useMaxWidth: false`, `nodeWidth: 50`, `nodeHeight: 20`) has `width="496" height="171"`. W12 (a title) has `viewBox="2 -2.53125 996 430.53125"`: the title text extends above 0.

The bounding box includes text, so the label boxes (font metrics) are part of the viewBox, the same risk as the sankey notes describe. Bounding boxes ignore clipping, so text clipped by its clip path still counts in full.

### 6.9 Configuration and defaults

`defaultConfig.ts` 323-334 (not in `config.schema.yaml`: a search for `treemap` there finds nothing, so the schema does not validate treemap keys):

| Key                                             | Default   | Read by                                                                                                                                                      |
| ----------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `useMaxWidth`                                   | `true`    | `renderer.ts` 526 (via `config.useMaxWidth`)                                                                                                                 |
| `padding`                                       | 10        | 27 (inner padding of the layout)                                                                                                                             |
| `diagramPadding`                                | 8         | 525 (viewport)                                                                                                                                               |
| `showValues`                                    | `true`    | 251, 287, 458                                                                                                                                                |
| `nodeWidth`                                     | 100       | 40 (`× 10`)                                                                                                                                                  |
| `nodeHeight`                                    | 40        | 41 (`× 10`)                                                                                                                                                  |
| `valueFormat`                                   | `','`     | 54                                                                                                                                                           |
| `borderWidth`, `valueFontSize`, `labelFontSize` | 1, 12, 14 | **Not read** by the renderer: the sizes in use are constants (`renderer.ts` 316-318, 469, 374). Probed: `labelFontSize: 30` (W22) leaves the labels at 38 px |

The built defaults (`chunk-DJLRBVLP.mjs` 5709-5720) match the source.

The `treemap` section is merged into `config` through `addDirective` and `sanitizeDirective` like every other section: `treemap` is among the default keys (`configKeys`), so front matter `valueFormat` works (probed: fixtures 8, 8a–8f, W17–W19).

### 6.10 Randomness, clock, DOM

- No randomness, no clock. The renderer uses `Math.max`, `Math.min` and `Math.round` (JavaScript's round, half up: 412, 421, 500).
- DOM: `select`, `selectAll`, `append`, `attr`, `style` (the inline style is re-serialised by Chromium after `style(…)` changes: `fill:#28253D` becomes `fill: rgb(40, 37, 61)` in the value text, W05). The port writes the final attribute values; the geometry harness does not compare attributes, so this does not change the result (section 8).
- `getBBox` (viewport), `getComputedTextLength` (6.6), `getComputedStyle` via `style('display')` and `style('font-size')` (397, 490, 495).

## 7. Fixtures

26 files: `nightly-check/` 16 and `pr-check/` 10. All render in the reference (probed).

| Fixture                       | Features                                                                                             | Probe                                                                                                                           |
| ----------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| 2, 4, 6                       | Hierarchy (three levels), long leaf text, four levels                                                | Render (26, 8 and 20 texts)                                                                                                     |
| 8, 8a, 8b, 8c, 8d, 8e, 8f, 8g | `valueFormat` through front matter (`$0,0`, `.1%`, `.2f`, `$.2f`, `$,.2f`, default, `$0,0`, default) | Probed values: `$1,500,000`; `35.0%`; `1.31`; `$369.96`; `$6.9e+6`; `1,387,459`; `$23,333,321`; `1,001,234,567` and `0.0000123` |
| 12, 14, 15, 16                | `classDef` on leaves and sections, with `fill`, `color`, `stroke`, `stroke-width`, nested            | Inline `!important` styles (probed)                                                                                             |
| 17, 18                        | Described as dark and neutral theme; no theme in the text, so the harness forces `redux-color`       | Render; the description is not tested                                                                                           |
| 1, 5                          | Basic; described as forest theme, no theme in the text                                               | Render                                                                                                                          |
| 10                            | Classes on a section and a leaf, a `classDef` at the end                                             | Render                                                                                                                          |
| 11, 13                        | `%%` comment lines (removed by `cleanupComments`); 13 uses `stroke-dasharray`                        | Render (W10)                                                                                                                    |
| 3, 7, 9                       | Classes on sections and leaves; 7 has `stroke-dasharray`                                             | Render                                                                                                                          |
| 19                            | 3 sections with emoji names, 41 leaves, shrinking labels                                             | Excluded: U+1F534 etc. are in the uncovered range (`tools/port/geometry/common.ts`, `UNCOVERED_SYMBOLS`)                        |

Harness notes. The forced directive (`patchText`) sets `htmlLabels: false`, the Lexica font and `securityLevel: strict`. The treemap uses none of these, so they do not change the output. Fixtures 5, 17 and 18 name a theme only in their descriptions, so they render with `redux-color` in the harness; the port's own tests need a `theme` directive to test dark text (W14).

Fixtures that exercise the quirks of section 2.8: 11 and 13 (comment lines), and the `classDef` forms (12–16, 3, 7, 9, 10). No fixture has a bare `classDef`, a trailing blank, `$xyz`, a value of 0 or a value that is only separators.

## 8. Risks, effort, open questions

Risks, in order of cost:

1. **Text widths in the fitting loops** (6.6). Every leaf label, value and section label decides its size or visibility from `getComputedTextLength`, at 12 px bold and at sizes 4 to 38 px. A width that differs by a fraction of a pixel can flip a decision (a label that shrinks one more step, or a truncation by one character), and that changes text content and boxes. High risk for the 25 comparable fixtures, since almost all have leaves.
2. **Langium tokens and parse rules** (section 2). The quirks (trailing blanks, bare `classDef`, the name rule) are easy to miss, and the probes are the test set. Medium.
3. **`d3-format`** (6.5). The default `''` type, `g` with 12 significant digits, the `$` branches and their errors. A subset is enough for the fixtures, but the reference's error on `$xyz` must be kept. The library is in `node_modules` and can be tested directly. Medium-low.
4. **Layout** (6.4). Arithmetic in the reference's order (the sum from the last child, the stable sort, the squarify rows, `Math.round`). The library is the oracle, and the probe matches it. Low.
5. **Viewport** (6.8). The box must leave out `display: none` items. Low, once the exclusion is in the model of the box.
6. **Inline style re-serialisation** (6.10). Not compared by the harness. Low.

Effort. Rust without tests: the tokenizer and parser (about 350 lines, with the terminals of 2.2 and the rules of 2.3), the DB and `buildHierarchy` (about 150), the hierarchy, squarify, treemap and round (about 250), `d3-format` (about 350 for the full type set; about 200 for the subset used), the renderer (about 450: the DOM of 6.2, the scales, the fitting loops, the viewport), the style sheet (about 60). About 1,600 lines, with about 60 more for the `valueFormat` branches. The text function is shared with the other types.

Open questions for the coordinator:

1. **Error text.** The Langium messages carry Chevrotain's expectations (`Expecting: one of these possible Token sequences: …`), and the `?` positions. The same question as the sankey notes: reproduce them, or keep only the accept and reject result with a line number?
2. **Multiple roots.** The validator exists but is not run on the parse path, so two roots are valid (probed). Keep the validator out of the port, and record that its check is dead on this path?
3. **`$xyz` (Y01).** The reference fails the whole render when a `$` format is invalid after the first branch. Reproduce the error (render fails), or fall back to `,`? Reproducing it keeps the port the same as the reference, but the error comes from inside the value loop.
4. **Bare `classDef` (C02, Z01).** Reproduce the swallowed line and the `: !important` style, or record them as differences? They are reproducible; leave them in, as the other quirks are.
5. **`treemap` is not in `config.schema.yaml`.** The configuration port has to take the keys and defaults from `defaultConfig.ts` 323-334. Decide: port the three unread keys (`borderWidth`, `valueFontSize`, `labelFontSize`) as they are, or drop them from the model?
6. **The `flowchart` class and the `x` suffix** (6.2) are visible in the DOM but do not change geometry. Keep both, since the class is set by the shared viewport function.
7. **Front matter title is drawn** for the treemap (W12, Z05), unlike the sankey. The canonical model follows the reference (section 4); the coordinator should confirm the sankey and treemap are both right as written.
8. **Fixture 19** (emoji) is excluded by the uncovered-symbol rule until fallback fonts exist (`tools/port/geometry/common.ts`). Fixtures 5, 17 and 18 do not render their named themes. Decide whether the port's fixture set uses a `theme` directive for them, which would be a change to the fixture file or the harness.
9. **Style options.** The `TreemapStyleOptions` defaults are the only ones that reach the style sheet (section 6.7). Confirm that no configuration key needs to change them.
