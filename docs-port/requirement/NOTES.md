# P2 requirement: research notes for the port

Reference: `packages/mermaid/src/diagrams/requirement/` and the shared code it uses in `packages/mermaid/src/rendering-util/`, `utils.ts`, `config.ts` and `mermaidAPI.ts`. Line numbers are the reference's. A rule marked **[probed]** was run against the reference (how: Appendix A). A rule marked **[read]** comes from the source only. **[unverified]** marks what neither method settled.

## 0. Summary

A requirement diagram lists requirements (six kinds), elements, relationships of seven kinds between them, class and style statements, a direction, and accessibility texts. The reference parses it with a Jison grammar and a small DB, and draws it on the unified renderer that ferrigura already has for flowchart, class, state and ER: ELK Layered lays it out, one shape (`requirementBox`) draws the nodes, and four marker definitions draw the arrowheads. There are no subgraphs, so no cluster kinds are needed.

Size, roughly: the grammar has about 25 rules and 60 alternatives, and the lexer has seven exclusive states; the DB is 351 lines; the renderer is 39 + 263 + 124 lines of TypeScript plus the marker and edge code it reuses. Measured against ER (about 2,800 lines of Rust across syntax, renderer and shape, without its markers), a requirement port is about two-thirds of that, as an estimate. The layout data is already covered by 26 captured ELK requirement cases in `crates/ferrigura-elk/tests/corpus`.

**Riskiest parts, most serious first:**

1. **Lexer and grammar quirks.** The lexer takes the first matching rule in definition order (the basic example parses, which a longest-match lexer would not, since `\s+` at line 31 matches longer than NEWLINE). Several inputs that look valid fail or change meaning: whitespace before a newline after `{`, a `direction` rule that eats any text containing `direction TB`, `-` or `:` in text, duplicate properties keeping the first value. Every one is in section 2.5.
2. **The shape's geometry.** `requirementBox` stacks labels whose heights come from `getBBox`, sizes the rectangle from the group's `getBBox`, and then re-translates the labels by DOM index. The ELK layout uses those sizes, so an error in the shape moves every node.
3. **The neo look is the default.** The requirement section sets `look: neo`, so 145 of the 146 comparable fixtures draw neo (the other is classic): the neo markers, the drop-shadow filter, the dash arrays computed from `getTotalLength()`, and the per-slot colour rules.
4. **The reference's own quirks.** The renderer reads its title margin and `useMaxWidth` from the `state` section, the `requirement.rect_*` config keys are read nowhere, classDef colour reaches the labels but no class rule is written, and a relationship to an undeclared name or a requirement and element with the same name both break the reference. Section 6.8 and section 8 list them.

**Fixtures:** 172 `.mmd` files (nightly 168, pr-check 2, visual-check 2, and three `.gitkeep`). Reference results with the harness's forced settings: 170 render, 2 fail (fixture defects, section 7.3). 24 use `look: handDrawn` and are excluded by the harness rule. So 146 fixtures are comparable.

## 1. Sources

| File                                                         | Lines                     | What it holds                                                                                                   |
| ------------------------------------------------------------ | ------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `diagrams/requirement/parser/requirementDiagram.jison`       | 1–267                     | The lexer (6–103) and the grammar (107–265).                                                                    |
| `diagrams/requirement/requirementDb.ts`                      | 1–351                     | The DB: the maps, the setters the grammar calls, `clear`, `getData` (292–342), the commonDb bindings (344–350). |
| `diagrams/requirement/requirementRenderer.ts`                | 1–39                      | `draw`: the layout data, the markers, `render`, the title, the viewport.                                        |
| `diagrams/requirement/styles.js`                             | 1–124                     | `genColor` (4–48), `getStyles` (50–122).                                                                        |
| `diagrams/requirement/types.ts`                              | 1–51                      | The model interfaces and the enum strings.                                                                      |
| `diagrams/requirement/requirementDiagram.ts`                 | 1–15                      | The diagram object: parser, DB, renderer, styles.                                                               |
| `diagrams/requirement/requirementDetector.ts`                | 1–24                      | The detector `/^\s*requirementDiagram/` (line 10).                                                              |
| `diagrams/requirement/requirementDb.spec.ts`                 | 1–107                     | 10 DB tests.                                                                                                    |
| `diagrams/requirement/parser/requirementDiagram.spec.js`     | 1–850                     | 43 parser tests (section 5).                                                                                    |
| `rendering-util/rendering-elements/shapes/requirementBox.ts` | 1–263                     | The node shape.                                                                                                 |
| `rendering-util/rendering-elements/markers.js`               | 6–10, 869–959, 1017–1020  | `insertMarkers`; the four requirement markers; the exports.                                                     |
| `rendering-util/rendering-elements/edgeMarker.ts`            | 55–56, 71–138             | The arrow names and how an edge refers to its marker.                                                           |
| `rendering-util/rendering-elements/edges.js`                 | 781–806, 812–871, 883–901 | Edge classes, the style string, the neo dash array, the data attributes.                                        |
| `utils/lineWithOffset.ts`                                    | 6–25                      | Marker offsets (no requirement entries).                                                                        |
| `rendering-util/render.ts`                                   | 81–103                    | `render`: node domIds, `insertLookDefs`, the layout call.                                                       |
| `rendering-util/layout-algorithms/common/index.ts`           | 129–146, 220–245          | Markers into the first `g`; the paint order.                                                                    |
| `rendering-util/setupViewPortForSVG.ts`                      | 5–40                      | The root class, the size, the viewBox with padding.                                                             |
| `rendering-util/insertElementsForSize.js`                    | 3–18                      | `getDiagramElement`: the root `svg` by id.                                                                      |
| `utils.ts`                                                   | 793–814, 874–906          | `insertTitle`; `encodeEntities`, `decodeEntities`.                                                              |
| `mermaidAPI.ts`                                              | 144–190, 586–606          | `createCssStyles` (classDef rules); the `getClasses` call that the requirement renderer lacks (594).            |
| `config.ts`                                                  | 42–53, 79–107, 149–152    | Appearance resolution through the diagram section.                                                              |
| `diagrams/common/commonDb.ts`                                | 16–29                     | `setAccTitle`, `setAccDescription`, `setDiagramTitle`.                                                          |
| `diagrams/common/common.ts`                                  | 71–83                     | `sanitizeText` (DOMPurify, `FORBID_TAGS: ['style']`).                                                           |
| `diagrams/common/colorThemeGate.ts`                          | 17, 33–38, 53, 83–84      | `COLOR_THEMES`, `isColorTheme`, `safeLook`, `paletteSlotCount`.                                                 |
| `Diagram.ts`                                                 | 19–77                     | `fromText`: `clear` (49), the front matter title (53), `encodeEntities(source) + '\n'` (77).                    |
| `preprocess.ts`                                              | 6–15, 19–, 58–70          | `cleanupText`, front matter, directives, comments.                                                              |
| `defaultConfig.ts`                                           | 275–278                   | The `requirement` section, with `useWidth` undefined.                                                           |
| `schemas/config.schema.yaml`                                 | 1070–1110                 | `RequirementDiagramConfig`: `theme`, `look`, the `rect_*` keys.                                                 |
| `e2e/diagrams/requirement/`                                  | —                         | 172 fixtures (section 7).                                                                                       |
| `crates/ferrigura-elk/tests/corpus/requirement__*`           | —                         | 26 captured ELK inputs of requirement diagrams.                                                                 |

## 2. Parser

### 2.1 Text before the lexer [read, probed where marked]

`Diagram.fromText` (Diagram.ts:19–77) detects the type with the detector, calls `db.clear()` (49), sets the front matter title (53), and gives the lexer `encodeEntities(source) + '\n'` (77). The source is `preprocessDiagram` (preprocess.ts:58–70) applied to the text, which does, in order:

1. `cleanupText` (6–15): CRLF becomes LF; a double-quoted attribute of an HTML-like tag becomes single-quoted.
2. Front matter (`extractFrontMatter`): the `title` goes to the metadata and the `config` block to the configuration; `description` is ignored.
3. Directives are removed (`removeDirectives`), and the init directive is read.
4. `cleanupComments` (diagram-api/comments.ts:7) removes lines that start with `%%` (not `%%{`) and trims the start of the text. It does not remove `#` lines.

`encodeEntities` (utils.ts:874–892) then rewrites `style` and `classDef` lines that have a `#…;` (the final `;` goes), and `#name;` entities become placeholders (`ﬂ°name¶ß`, or `ﬂ°°n¶ß` for numbers). `decodeEntities` (904–906) reverses the placeholders when a label is drawn.

Probed: a CRLF input parses; a `%%` line before the header parses; a `#` line before the header is a parse error on line 1 (Q32).

### 2.2 Lexer states [read]

| State                 | Line | Entered by                                  | Used                     |
| --------------------- | ---- | ------------------------------------------- | ------------------------ |
| INITIAL               | —    | start                                       | yes                      |
| `string`              | 9    | `"` (85, 95)                                | yes                      |
| `token`               | 10   | —                                           | **never entered** (dead) |
| `unqString`           | 11   | —                                           | **never entered** (dead) |
| `style`               | 12   | `style` (77), `classDef` (88), `class` (89) | yes; left by `\n` (86)   |
| `acc_title`           | 13   | `accTitle:` (19)                            | yes                      |
| `acc_descr`           | 14   | `accDescr:` (21)                            | yes                      |
| `acc_descr_multiline` | 15   | `accDescr {` (23)                           | yes                      |

`%options case-insensitive` (line 7) makes every keyword match in any case. Names keep the case they were written in (probed: `Requirement A` gives the name `A`).

### 2.3 Lexer rules in definition order [read; first match, probed]

Jison takes the first rule, in definition order, that matches at the current position, not the longest. The basic example parses, which fixes the order: `\s+` (line 31) matches longer than `(\r?\n)+` (line 30), so a longest-match lexer would never produce NEWLINE. Keywords that end in a word character get a word boundary: probed, `requirement idx` and `id: identity` both lex as one name or value (line 43 `"id"` does not match the start of `idx` or `identity`).

| Lines   | Rule                                                                                              | Token                                                        | Note                                                                                                                                 |
| ------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| 18      | `"title"\s[^#\n;]+`                                                                               | `title`                                                      | No production uses it (section 2.5).                                                                                                 |
| 19–20   | `accTitle\s*":"\s*` then value to end of line                                                     | `acc_title`, `acc_title_value`                               | The value is trimmed in the grammar.                                                                                                 |
| 21–22   | `accDescr\s*":"\s*` then value                                                                    | `acc_descr`, `acc_descr_value`                               |                                                                                                                                      |
| 23–25   | `accDescr\s*"{"\s*`, then everything to `}`                                                       | `acc_descr_multiline_value`                                  | The multi-line form.                                                                                                                 |
| 26–29   | `.*direction\s+TB…` / BT / RL / LR                                                                | `direction_tb` etc.                                          | The `.*` at the start matches from the current position, so any text with `direction TB` is a direction token (probed, section 2.5). |
| 30      | `(\r?\n)+`                                                                                        | `NEWLINE`                                                    |                                                                                                                                      |
| 31      | `\s+`                                                                                             | (skipped)                                                    | Also skips newlines when the spaces come first (section 2.5).                                                                        |
| 32      | `\#[^\n]*`                                                                                        | (skipped)                                                    | A `#` anywhere starts a comment.                                                                                                     |
| 33      | `\%%[^\n]*`                                                                                       | (skipped)                                                    |                                                                                                                                      |
| 34      | EOF                                                                                               | `EOF`                                                        |                                                                                                                                      |
| 36      | `requirementDiagram`                                                                              | `RD`                                                         |                                                                                                                                      |
| 38–41   | `{`, `}`, `:::` (three colons), `:`                                                               | `STRUCT_START`, `STRUCT_STOP`, `STYLE_SEPARATOR`, `COLONSEP` |                                                                                                                                      |
| 43–46   | `id`, `text`, `risk`, `verifyMethod`                                                              | `ID`, `TEXT`, `RISK`, `VERIFYMTHD`                           | Word boundary (probed).                                                                                                              |
| 48–53   | six requirement kinds                                                                             | `REQUIREMENT` … `DESIGN_CONSTRAINT`                          |                                                                                                                                      |
| 55–57   | `low`, `medium`, `high`                                                                           | `LOW_RISK`, `MED_RISK`, `HIGH_RISK`                          |                                                                                                                                      |
| 59–62   | `analysis`, `demonstration`, `inspection`, `test`                                                 | `VERIFY_*`                                                   |                                                                                                                                      |
| 64      | `element`                                                                                         | `ELEMENT`                                                    |                                                                                                                                      |
| 66–72   | `contains`, `copies`, `derives`, `satisfies`, `verifies`, `refines`, `traces`                     | `CONTAINS` … `TRACES`                                        |                                                                                                                                      |
| 74–75   | `type`, `docref`                                                                                  | `TYPE`, `DOCREF`                                             |                                                                                                                                      |
| 77      | `style`                                                                                           | `STYLE`, enters `style`                                      |                                                                                                                                      |
| 78–86   | in `style`: `\w+`, `:`, `;`, `%`, `-`, `#`, space (skipped), `"` (enters `string`), `\n` (leaves) | `ALPHA`, `COLON`, `SEMICOLON`, `PERCENT`, `MINUS`, `BRKT`    | Any other character is a lexical error (probed: `(` and a tab).                                                                      |
| 88–89   | `classDef`, `class`                                                                               | `CLASSDEF`, `CLASS`, enter `style`                           |                                                                                                                                      |
| 91–93   | `<-`, `->`, `-`                                                                                   | `END_ARROW_L`, `END_ARROW_R`, `LINE`                         |                                                                                                                                      |
| 95–97   | `"` … `"`                                                                                         | `qString`                                                    | An empty quoted string does not match (probed).                                                                                      |
| 99      | `[\w][^:,\r\n\{\<\>\-\=]*`                                                                        | `unqString` (trimmed)                                        | ASCII `\w` only (probed: `Ä` is a lexical error). Spaces are allowed inside.                                                         |
| 101–103 | `\w+`, `[0-9]+`, `,` (all states)                                                                 | `ALPHA`, `NUM`, `COMMA`                                      |                                                                                                                                      |

### 2.4 Grammar and the DB calls [read]

| Production (lines)                                                                        | DB calls, in the order Jison runs them                                                                                                                                                                |
| ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `start` (111–114): `directive NEWLINE start`, `directive start`, `RD NEWLINE diagram EOF` | None. A `RD` header must be followed by a newline.                                                                                                                                                    |
| `directive` (116–120)                                                                     | `setAccTitle`, `setAccDescription` (multi-line and one-line; values trimmed).                                                                                                                         |
| `diagram` (122–133)                                                                       | Each alternative runs its own actions; `NEWLINE`, and an empty diagram, do nothing.                                                                                                                   |
| `direction` (135–144)                                                                     | `setDirection(TB, BT, RL, LR)`. The last one wins (probed).                                                                                                                                           |
| `requirementDef` (146–149)                                                                | `requirementBody` first, then `addRequirement(name, type)`; the shorthand form also `setClass([name], classes)`.                                                                                      |
| `requirementBody` (151–161)                                                               | Right-recursive. Each property action runs after the rest of the body, so the **first** property wins for a repeated key (probed: `id: 1` then `id: 2` gives `1`).                                    |
| `requirementType` (163–175), `riskLevel` (177–180), `verifyType` (182–190)                | Enum values into the DB's constant tables.                                                                                                                                                            |
| `elementDef` (192–195)                                                                    | `elementBody` first, then `addElement(name)`; the shorthand form also `setClass`.                                                                                                                     |
| `elementBody` (197–203)                                                                   | `setNewElementType`, `setNewElementDocRef`. Repeats keep the first (same right recursion).                                                                                                            |
| `relationshipDef` (205–209)                                                               | `X - T -> Y`: `addRelationship(T, X, Y)`. `X <- T - Y`: `addRelationship(T, Y, X)` (probed: `b <- contains - a` stores `a` contains `b`).                                                             |
| `relationship` (211–225)                                                                  | The seven relationship strings.                                                                                                                                                                       |
| `classDefStatement` (227–229)                                                             | `defineClass(ids, styles)`.                                                                                                                                                                           |
| `classStatement` (231–234)                                                                | `class a,b c` gives `setClass([a,b],[c])`; `a:::c` gives `setClass([a],[c])`.                                                                                                                         |
| `idList` (236–241)                                                                        | Arrays of `ALPHA`, `id` or `qString`, comma-joined.                                                                                                                                                   |
| `styleStatement` (243–245)                                                                | `setCssStyle(ids, styles)`.                                                                                                                                                                           |
| `stylesOpt` (247–250), `style` (252–255), `styleComponent` (257)                          | Style values are the tokens concatenated with no spaces (probed: `fill: red` gives `fill:red`). `styleComponent` lists `UNIT`, `SPACE`, `PCT`, `LABEL`, which the lexer never produces (section 2.5). |
| `names` (260–265)                                                                         | `unqString \| qString`.                                                                                                                                                                               |

### 2.5 Quirks a port must reproduce [probed unless marked]

Every row was run through the reference grammar and DB (Appendix A, `parse-results.json`).

| #   | Input (body lines)                                             | Reference result                                                                                                                                       | Rule for the port                                                                                                  |
| --- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| Q1  | `title My Title` before any block                              | Parse error: `Expecting 'NEWLINE', 'EOF', 'acc_title', … got 'title'`. The `title` token has no production.                                            | Reject. A body title is not a feature.                                                                             |
| Q2  | `requirement a {` then spaces then a newline                   | Parse error `Expecting 'NEWLINE', got 'ID'`: `\s+` (31) eats the newline.                                                                              | Keep the lexer order; `\s+` takes newlines.                                                                        |
| Q3  | `requirement a {}` on one line                                 | Parse error: `{` must be followed by a newline.                                                                                                        | Reject.                                                                                                            |
| Q4  | `requirement idx {`, `id: identity`                            | Accepted; the names are `idx` and `identity`.                                                                                                          | Word boundary on keywords.                                                                                         |
| Q5  | `risk: highly`                                                 | Parse error: `Expecting 'LOW_RISK', 'MED_RISK', 'HIGH_RISK', got 'unqString'`.                                                                         | Reject.                                                                                                            |
| Q6  | `Requirement A {`, `ID: 1`, `RISK: HIGH`, `VerifyMethod: Test` | Accepted; the name is `A`.                                                                                                                             | Case-insensitive keywords; names keep their case.                                                                  |
| Q7  | `id: 1` then `id: 2` in one block                              | `requirementId` is `1`.                                                                                                                                | First property wins.                                                                                               |
| Q8  | A second `requirement a` with another kind                     | The first block's kind, id, text, risk and method stay; the second block's properties are dropped.                                                     | `addRequirement` keeps the existing entry (requirementDb.ts:127–138).                                              |
| Q9  | `id: REQ-001`                                                  | Parse error (`-` is `LINE`).                                                                                                                           | Reject.                                                                                                            |
| Q10 | `text: note: x`                                                | Parse error (`:` ends the unqString). `text: "note: x"` is accepted.                                                                                   | Reject.                                                                                                            |
| Q11 | `text: a, b`                                                   | Parse error (`,` is `COMMA`).                                                                                                                          | Reject.                                                                                                            |
| Q12 | `text: issue #12` / `text: #12`                                | `issue #12` is accepted; `text: #12` is a parse error (the `#` starts a comment and no value follows).                                                 | Comment rule only at a token start.                                                                                |
| Q13 | `text: go direction TB here`                                   | Parse error: the `direction` rule (line 26) matches from the `:` on, before `COLONSEP` (41).                                                           | Lexer order as in 2.3.                                                                                             |
| Q14 | `a - contains -> zz` with no declaration of `zz`               | Accepted; the relation stays. `getData` gives an edge with `end` undefined; the render fails inside ELK (`Id must be a string or an integer: 'null'`). | Parse OK; the render returns an error (open question 2).                                                           |
| Q15 | `requirement a {`…`}`, `element a {`…`}` (same name)           | Both stored. `getData` gives two nodes with id `a`. The render writes an SVG and logs `Expected number, "translate(undefined, undef…"`.                | Named exception, see open question 3.                                                                              |
| Q16 | `style zz,a fill:#f9f` where `zz` is missing                   | `a` is **not** styled: `setCssStyle` returns at the first missing id (requirementDb.ts:229–231).                                                       | Reproduce: return, not continue.                                                                                   |
| Q17 | `style a,zz fill:#f9f`                                         | `a` is styled.                                                                                                                                         |                                                                                                                    |
| Q18 | `style a width:50%`                                            | Parse error (`PERCENT` is not a style component).                                                                                                      | Reject.                                                                                                            |
| Q19 | `style a fill:rgb(1,2,3)`                                      | Lexical error (`(` in the `style` state).                                                                                                              | Reject.                                                                                                            |
| Q20 | `style a`, a tab, `fill:red`                                   | Lexical error (only the space is skipped in `style`).                                                                                                  | Reject.                                                                                                            |
| Q21 | `style a fill: red`                                            | Accepted; the style is `fill:red`.                                                                                                                     | Concatenate tokens with no spaces.                                                                                 |
| Q22 | `style a stroke-width:-1px`                                    | Accepted.                                                                                                                                              |                                                                                                                    |
| Q23 | `classDef c` with no styles                                    | Parse error.                                                                                                                                           | Reject.                                                                                                            |
| Q24 | `requirement a:::c` before `classDef c fill:#f9f`              | Class `c` on `a`, with its style applied when defined: `cssStyles` gets `fill:#f9f`.                                                                   | `defineClass` also pushes to nodes that already carry the class.                                                   |
| Q25 | `class a c` before `classDef c`                                | Same as Q24.                                                                                                                                           |                                                                                                                    |
| Q26 | `requirement "" {`, `element "" {`, `text: ""`                 | Parse error for each.                                                                                                                                  | Reject an empty quoted string.                                                                                     |
| Q27 | `requirement my req {`                                         | Name `my req`.                                                                                                                                         | Unquoted names take spaces, trimmed.                                                                               |
| Q28 | `requirement my-req {`                                         | Parse error.                                                                                                                                           | Reject.                                                                                                            |
| Q29 | `"x y" - contains -> z`                                        | Accepted; the relation's `src` is `x y`.                                                                                                               |                                                                                                                    |
| Q30 | `accTitle:   spaced title   `                                  | `spaced title`.                                                                                                                                        | Leading spaces skipped; the value trimmed.                                                                         |
| Q31 | `accDescr {`, two lines with indentation, `}`                  | `line one\nline two`. The indentation of each line goes (commonDb.ts:23).                                                                              | `replace(/\n\s+/g,'\n')` after sanitising.                                                                         |
| Q32 | `# comment` before the header                                  | Parse error (`cleanupComments` does not remove `#` lines).                                                                                             | Reject.                                                                                                            |
| Q33 | `# comment` inside a block                                     | Accepted (line 32).                                                                                                                                    |                                                                                                                    |
| Q34 | `%% comment` before the header                                 | Accepted.                                                                                                                                              |                                                                                                                    |
| Q35 | `direction LR` then `direction BT`                             | Direction `BT`.                                                                                                                                        | Last wins.                                                                                                         |
| Q36 | `clear()` after `direction LR`                                 | Direction stays `LR` (requirementDb.ts:216–224 never resets it). A new DB per diagram starts at `TB`.                                                  | Keep the field across `clear`.                                                                                     |
| Q37 | Front matter `title:`                                          | The title is set (Diagram.ts:53).                                                                                                                      |                                                                                                                    |
| Q38 | `requirementDiagram` alone                                     | Accepted (an empty diagram).                                                                                                                           |                                                                                                                    |
| Q39 | Empty input                                                    | Parse error on line 1 (the header is missing).                                                                                                         |                                                                                                                    |
| Q40 | `docref: x` at top level                                       | Parse error.                                                                                                                                           |                                                                                                                    |
| Q41 | `element e {` with `type: word doc`                            | Type `word doc`.                                                                                                                                       |                                                                                                                    |
| Q42 | `requirement a {`, `foo: bar`, `}`                             | Parse error.                                                                                                                                           |                                                                                                                    |
| Q43 | `requirement Ä {`                                              | Lexical error (`Ä` is not `\w`).                                                                                                                       | Reject.                                                                                                            |
| Q44 | `classy a` (a line that starts with `class` as a prefix)       | Parse error.                                                                                                                                           | Probed (`class_identity`). The `class` keyword needs a word boundary, as `id` does, or the line fails in any case. |

Dead or unreachable grammar [read]: the `token` and `unqString` lexer states (lines 10–11); the `UNIT`, `SPACE`, `PCT` and `LABEL` names in `styleComponent` (257); the `title` token (18) with no production. `ALPHA` and `NUM` in `styleComponent` are the only style-value tokens the lexer produces besides `COLON`, `SEMICOLON`, `BRKT`, `MINUS`.

### 2.6 Errors [probed]

Errors are Jison messages. Three shapes occur, with the reference's text:

- `Parse error on line N:` with the source around the position, then `Expecting <token list>, got '<token>'`. Example, Q1: `Expecting 'NEWLINE', 'EOF', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'direction_tb', … got 'title'`.
- `Lexical error on line N. Unrecognized text.` for a character no rule matches (Q19, Q20, Q43).
- `Diagrams beginning with --- are not valid…` when the front matter is malformed (section 7.3).

The expected-token lists are Jison's and depend on the grammar's tables. A port can reproduce the first line and the got-token exactly; reproducing the full expected list is open question 4.

### 2.7 Comments, directives, front matter [read, probed Q32–Q37]

`%%` lines are removed before the lexer (cleanupComments). `#` comments are lexed (line 32), so they work inside a block and not before the header (Q32). Directives (`%%{init: …}%%`) are removed before the lexer. Accessibility directives are grammar productions (116–120). The title comes from front matter only (Q37).

## 3. DB

### 3.1 State and `clear` [read]

Fields (requirementDb.ts:27–33): `relations`, `latestRequirement`, `requirements` (a `Map`), `latestElement`, `elements` (a `Map`), `classes` (a `Map`), `direction` (default `'TB'`). `clear()` (216–224) resets everything except `direction`, and calls commonDb's `clear` (title, accTitle, accDescr). Probed: `direction` survives `clear()` (Q36). Each diagram gets a new DB (`get db()` in requirementDiagram.ts:10–12), so in practice `direction` starts at `TB`.

### 3.2 Requirements and elements [read]

- `addRequirement(name, type)` (126–142): if the name is new, the entry takes the latest `requirementId`, `text`, `risk` and `verifyMethod` and `classes: ['default']`; then the latest state is reset. If the name exists, nothing changes (Q8). The returned value is the entry.
- `setNewReqId`, `setNewReqText`, `setNewReqRisk`, `setNewReqVerifyMethod` (148–170) write the latest state. The grammar calls them in the reverse of line order, so the first line wins (Q7).
- `addElement(name)` (172–186) and `setNewElementType`, `setNewElementDocRef` (192–202): the same pattern. An element has `type` and `docRef`, not id, text, risk or method.
- Requirements and elements are two maps. A name can be in both (Q15).

### 3.3 Relationships [read]

`addRelationship(type, src, dst)` (204–210) appends `{type, src, dst}` without checking that `src` and `dst` exist. The statements `X - T -> Y` and `X <- T - Y` give `src`, `dst` as in 2.4.

### 3.4 Classes and styles [read, probed Q16–Q25]

- `defineClass(ids, styles)` (257–286): creates the class if needed and appends the styles to it. It also appends the styles to every node or element that already carries the class id (`value.cssStyles.push(...)`). A style that matches `/color/` also goes into `textStyles`, with `fill` replaced by `bgFill` (267–270; probed: `color:red` ends up in `textStyles`).
- `setClass(ids, classNames)` (242–255): appends the class names to each node, and the class's styles to its `cssStyles`.
- `setCssStyle(ids, styles)` (226–240): appends the styles to each node; returns at the first id that has no node (Q16).
- `textStyles` is written and never read in the requirement code. The requirement renderer has no `getClasses`, so `mermaidAPI` never writes the class rules (section 6.7). [read, probed: the classDef render has no rule that names the class]

### 3.5 `getData` [read, probed]

`getData()` (292–342) mutates the DB's own objects and returns `{nodes, edges, other, config, direction}`:

- Nodes: all requirements in insertion order, then all elements. Each gets `id` = name, `shape: 'requirementBox'`, `look` = `config.look`, `colorIndex` = its index in the node list (303, 314), `cssClasses` = the classes joined by a space (300, 313), and `cssStyles`.
- Edges (318–339): `id` = `${src}-${dst}-${n}` where `n` counts every relation (probed: `r1-e1-0`, `e1-r2-1`, `r1-r2-2`). `label` = `&lt;&lt;${type}&gt;&gt;`, `labelType: 'markdown'`, `labelpos: 'c'`, `classes: 'relationshipLine'`, `thickness: 'normal'`, `type: 'normal'`. `style` = `['fill:none', '']` for `contains` (the empty entry is kept) and `['fill:none', 'stroke-dasharray: 10,7']` for the others. `pattern` = `normal` for `contains`, `dashed` for the others. `arrowTypeStart` = `requirement_contains` for `contains`, `arrowTypeEnd` = `requirement_arrow` for the others. `look` = `config.look`.
- An unknown endpoint gives `start` or `end` undefined (probed, Q14).

Probed, the node keys are `classes, colorIndex, cssClasses, cssStyles, id, look, name, requirementId, risk, shape, text, type, verifyMethod` for a requirement, and `… docRef` instead of the requirement fields for an element.

### 3.6 Sanitising [read]

The DB does not sanitise requirement, element, relation or class texts. The title, `accTitle` and `accDescr` go through commonDb's `sanitizeText` (common.ts:71–83, DOMPurify with `FORBID_TAGS: ['style']`). The renderer sanitises at draw time: `sanitizeText(decodeEntities(text))` (requirementBox.ts:220). So the DB holds the placeholders (`ﬂ°…¶ß`) and the drawn text has them decoded.

### 3.7 Defaults [read]

Requirement: `classes: ['default']`, empty strings for id, text, risk, method and kind. Element: `classes: ['default']`, empty type and docRef.

## 4. Canonical model proposal

Field names are camelCase, as in the P1 models, with `version: 1`. The differential compares this model against the reference's exporter (`tools/port/export-requirement.ts`, to be written in the parser task).

```
{
  "version": 1,
  "direction": "TB" | "BT" | "RL" | "LR",
  "requirements": [
    { "name", "type", "requirementId", "text", "risk", "verifyMethod", "classes": [], "cssStyles": [] }
  ],
  "elements": [ { "name", "type", "docRef", "classes": [], "cssStyles": [] } ],
  "relationships": [ { "type", "src", "dst" } ],
  "classes": [ { "id", "styles": [], "textStyles": [] } ],
  "title": "", "accTitle": "", "accDescr": ""
}
```

- `type` of a requirement is the display string (`Requirement`, `Functional Requirement`, `Interface Requirement`, `Performance Requirement`, `Physical Requirement`, `Design Constraint`). `risk` is `Low`, `Medium` or `High`; `verifyMethod` is `Analysis`, `Demonstration`, `Inspection` or `Test`. An unset value is `""`.
- `requirements`, `elements`, `relationships` and `classes` are in insertion or statement order, as the reference keeps them (Maps keep insertion order).
- `classes` lists `default` for each node, as `classes` does.
- Left out: the latest-state fields, the lexer and grammar state, and the `getData` output. The layout data is a second model (`requirement-data`), as for class, ER and state (`tools/port/export-class-data.ts` is the precedent).
- Open question 5: whether the texts keep the placeholders (what the DB holds) or are decoded (what is drawn).

## 5. Specs

### 5.1 Files and counts [probed]

Run once on HEAD with vitest (JSON reporter): **53 of 53 pass** (`vitest-req.json`, Appendix A).

| File                                | Tests | Covers                                                                                                                                                                                                                       |
| ----------------------------------- | ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `requirementDb.spec.ts`             | 10    | DB: add requirement, add element, add relationship, edge ids `src-dst-0`/`src-dst-1`, class definition (single and many), direction, styles on a requirement and an element, classes on both, styles inherited from a class. |
| `parser/requirementDiagram.spec.js` | 43    | 35 plain `it` blocks plus 2 blocks in a loop over `__proto__` and `constructor` (4 tests), plus a `test.each` over four directions (4 tests).                                                                                |

The 43 parser tests, by group: full requirement, full element, the accessibility title and description (one line and multi-line), the full relationship, the six requirement kinds, the three risk levels, the four verify methods, the seven relationship types, the ids `__proto__` and `constructor` for requirements and elements (4), styling a requirement, an element and several at once (3), defining a class, defining several (2), the class statement (2), the shorthand `:::` (3), and the four directions (4).

### 5.2 What the port must pass

- The DB tests map to the port's DB model: edge ids, class and style handling, direction.
- The parser tests map to the parser's accept and reject cases and to the DB fields they set.
- Cross-cutting specs that touch requirement (none is ported in this task):
  - `paletteCssGeneration.spec.ts`: the requirement stylesheet emits one rule per palette slot, a `path` rule and a `rect` rule, and no empty `fill:` (lines 2–16, 52–66, 179–193).
  - `colorThemeGate.spec.ts`: `requirement/styles.js` reads the palette from `getConfig()` (lines 17–36, 48–62, 81, 164).
  - `theme-redux-color-superset.spec.ts`: `requirementEdgeLabelBackground` is set by the theme (lines 9, 74–76).
  - `styles.spec.ts`, `mermaidAPI.spec.ts:872` (detection to `requirement`), `diagram-orchestration.spec.ts:30, 87–90` (only the keyword detects it), `config.appearance.spec.ts:31–32`, `defaultAppearanceDocs.spec.ts:18`, `multi-diagram-id-uniqueness.spec.ts:42–43`, `diagram-beta-policy.spec.ts:34`.

## 6. Renderer

### 6.1 Pipeline [read, probed]

1. `draw` (requirementRenderer.ts:9–38) reads `getConfig()`, calls `diag.db.getData()` (14), creates the root with `getDiagramElement` (17; the `svg` with the diagram id, insertElementsForSize.js:3–18), sets `type` to `diag.type` (19; probed: the marker ids use `requirement`), sets the layout to `getRegisteredLayoutAlgorithm(layout)` (20; ELK, the default), sets the spacings from the `state` section (22–23; section 6.8), sets the markers (24–27), and calls `render` (29).
2. `render` (render.ts:81–103) prefixes each node's `domId` with the diagram id (88–93; probed: `req1-test_req`), calls `insertLookDefs` (98), and runs the ELK layout render.
3. The common layout renderer (common/index.ts:129–146) inserts the markers into the first `g` of the root, with `data4Layout.type` and `diagramId` (probed: `req1_requirement-requirement_containsStart`). It then measures, lays out and paints (`paintLayoutData`, 220–245): nodes first, in order, through `positionNode` and the shape; then edges in order.
4. `insertTitle` (31–36; utils.ts:793–814) adds the title text if there is one.
5. `setupViewPortForSVG` (38; setupViewPortForSVG.ts:5–26) sets the class to `requirementDiagram`, the size and the `max-width`, and the viewBox from the root's `getBBox` plus padding 8 (28–36, 38–40).

### 6.2 DOM order of the output [probed]

Classic and neo draw the same groups. Neo differs in the marker attributes, the divider (a polygon in neo, a line in classic), the edge dash array, `data-look`, and the style sheet. Both write the two drop-shadow filters. Attributes are as the reference wrote them.

```
svg#<id>  width="100%" xmlns class="requirementDiagram" style="max-width: <W>px;"
          viewBox="<x-8> <y-8> <W> <H>" role="graphics-document document" aria-roledescription="requirement"
  style                       the stylesheet (6.7)
  g                           the first g: markers go here
    defs > marker#<id>_requirement-requirement_containsStart    (classic or neo, 6.5)
    defs > marker#<id>_requirement-requirement_arrowEnd
    g.root
      g.clusters                (empty)
      g.edges.edgePaths
        path#<id>-<src>-<dst>-<n>  class="edge-thickness-normal edge-pattern-<p> relationshipLine"
                                   style="[neo dash array;] fill:none;…"  marker-start / marker-end
      g.edgeLabels
        g.edgeLabel  transform="translate(x, y)"
          g.label  data-id="<src>-<dst>-<n>"  transform="translate(…)"
            g > rect.background … text > tspan.text-outer-tspan.row > tspan.text-inner-tspan (the markdown runs)
      g.nodes
        g.node.default#<id>-<name>  data-color-id="color-<i>" data-look="<look>" transform="translate(x, y)"
          g.basic.label-container.outer-path  style=<node style>
            path (fill; stroke none)           the rough fill
            path (stroke)                      the rough outline
          g.label  style="<label style>"  transform  (type: <<Requirement>>)
          g.label  style="<label style>; font-weight: bold;"  (name)
          g.label  …  (ID:, Text:, Risk:, Verification: or Type:, Doc Ref:, left-aligned with text-anchor start in ELK)
          g.divider  (when a divider is drawn, 6.4)
  text.requirementDiagramTitleText  text-anchor="middle" x y="-titleTopMargin"   (only with a title)
  defs > filter#<id>-drop-shadow and filter#<id>-drop-shadow-small   (both looks, from insertLookDefs; the CSS applies them only to neo)
```

Probed: the title comes after the outer `g` and before the neo filters (`svg-front-matter-title.svg`).

### 6.3 Text measurement [read, probed for the values]

Every label is `addText` (requirementBox.ts:204–263):

- Empty text: no label group, height 0 (`return 0`, line 212).
- Otherwise a `g.label` with the given style, then `createText` with the sanitised, decoded text, `width = calculateTextWidth(rawText) + 50`, class `markdown-node-label`, and the forced `useHtmlLabels` (false, by the harness).
- The width feeds only the wrapping. It is measured on the raw text, which holds the placeholders of entities. Probed: `#lt;` in a text is drawn as `<`. [read, unverified: the effect of the placeholder length on the wrapping of long texts]
- SVG path (htmlLabels false): `bbox = text.getBBox()`, `bbox.height += 6` (line 247), then `translate(-bbox.width/2, -bbox.height/2 + yOffset)` (261). Returns `bbox.height`.
- Start alignment (`alignment === 'start'`, line 238): `text-anchor="start"` on the text and every child tspan. The body texts use start when `config.layout === 'elk'` (line 30), which is the default. Probed: the body labels have `text-anchor="start"`.
- The markdown in the text (bold, italic) is drawn as styled runs (probed on the markdown-text fixture `requirements-and-elements-with-markdown-text-default`: `font-weight="bold"` and `font-style="italic"` tspans in the body label).

The shape's measurements feed the layout. All of them are float32 `getBBox` values with Chromium's rules (M6-PLAN, text measurement).

### 6.4 The `requirementBox` shape [read, probed for the output]

Constants: `padding = 20`, `gap = 20` (lines 23–24). `isRequirementNode = 'verifyMethod' in node` (25): a requirement always has the key, an element never does.

Steps (line numbers in requirementBox.ts):

1. Group `g` with the class from `getNodeClasses` (`node` and the node's `cssClasses`, probed: `node default`) and `id = domId` (33–36).
2. Type label: `<<${type}>>` for a requirement, `<<Element>>` for an element (40–48). Height `typeHeight`; accumulated height starts there.
3. Name label, with `labelStyle + '; font-weight: bold;'` (51–57). Then `accumulated += nameHeight + 20`.
4. Requirement body (59–92): `ID: <id>`, `Text: <text>`, `Risk: <risk>`, `Verification: <method>`, each added at the accumulated height; `ID`, `Text` and `Risk` add their heights to the accumulated height, `Verification` does not. Empty values give no label.
5. Element body (93–110): `Type: <type>` (adds its height), then `Doc Ref: <docRef>` (does not add).
6. `totalWidth = getBBox().width + 20`, `totalHeight = getBBox().height + 20` (112–113), taken from the group with the labels as positioned. Rectangle at `(-W/2, -H/2)` (114–115).
7. Roughjs options: `userNodeOverrides` (fill = the node's `fill` style, else `mainBkg`; stroke = its `stroke` style, else `nodeBorder`; strokeWidth = its `stroke-width` without `px`, else 1.3; the dash array from `stroke-dasharray`), and for a non-handDrawn look `roughness 0` and `fillStyle 'solid'` (119–125; handDrawnShapeStyles.ts:108–130). The `1.3` is what the probes show.
8. Rectangle: `rc.rectangle`, inserted as the first child with class `basic label-container outer-path` and `style = nodeStyles` (128–131).
9. `data-color-id = color-${colorIndex % borderColorArray.length}` when the palette is non-empty (133–136; probed: `color-0` for the first node).
10. Label re-translation (138–165): for each `.label` group in DOM order, with `i` its index: the translate is `(translateX, translateY - H/2 + 20)` for `i` 0 and 1 (the type and the name, centred), and `(x + 10, translateY - H/2 + 20)` for the rest. Note that the index counts `.label` groups only. An empty name would shift the index (not reachable, Q26).
11. Divider (167–185): drawn when `accumulated > typeHeight + nameHeight + 20`. For a requirement that means at least one of ID, Text, Risk is non-empty (not Verification alone). For an element, only when `Type` is non-empty (not Doc Ref alone) [read]. Position `y = -H/2 + typeHeight + nameHeight + 20`. Neo: a polygon 0.001 thick (`rc.polygon`). Classic: a line (`rc.line`). Class `divider` (184). Probed: the basic example has one divider; the empty requirement and element have none.
12. `updateNodeBounds`, `node.intersect = intersect.rect` (187–191).
13. If there are node styles and the look is not handDrawn and the requirement edge-label background or the border palette is set, every `path` in the group gets `style = nodeStyles` (193–199).

Measured, the group's `getBBox` includes the transforms of the labels, so the widths are the labels' extents, not the rectangle's.

### 6.5 Markers [read, probed for the output]

Four definitions. An edge refers to them by `${svgId}_${diagramType}-${name}`: `requirement_arrow` as `requirement_arrowEnd` (for `arrowTypeEnd`), `requirement_contains` as `requirement_containsStart` (for `arrowTypeStart`) (edgeMarker.ts:55–56, 97–100).

| Name                             | Classic (`markers.js`)                                                                                                                                          | Neo (`markers.js`)                                                                                                                                        |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `requirement_arrow` (869–887)    | `marker` refX 20, refY 10, markerWidth 20, markerHeight 20, orient auto; path `M0,0 L20,10 M20,10 L0,20` (the path text keeps its line breaks and indentation). | Adds `markerUnits="userSpaceOnUse"`, `stroke-width` = theme `strokeWidth` (2 in redux-color), `viewBox="0 0 25 20"`, `stroke-linejoin="miter"` (889–915). |
| `requirement_contains` (917–934) | refX 0, refY 10, markerWidth 20, markerHeight 20, orient auto; a `g` with a circle (10, 10, r 9, fill none) and two lines (1 to 19).                            | Adds `markerUnits="userSpaceOnUse"`; every child gets `stroke-width` = theme `strokeWidth` (936–958).                                                     |

The fill and stroke of the markers come from the stylesheet (`marker{fill;stroke}`, 6.7). The contains edge uses `marker-start`; the others `marker-end` (probed).

Insertion: `insertMarkers` runs the names in `data4Layout.markers` (markers.js:6–10). The markers are not inside a `g` of their own: each is in its own `defs` (probed).

Offsets: none. `markerOffsets` and `markerOffsets2` (lineWithOffset.ts:6–25) have no requirement entries, so requirement edges are not shortened under a marker. [read]

Colour variants: none. Edges have no `linkStyle`, so `addEdgeMarker` is never called with a stroke colour (edges.js:871 looks for `stroke:` in the path style; the requirement edge style has none). [read]

### 6.6 Edges [read, probed]

`insertEdge` in edges.js (624–906):

- Thickness class (781–793) and pattern class (794–806): `edge-thickness-normal`, and `edge-pattern-dashed` or `edge-pattern-solid`.
- The path style is `pathStyle` (850–871): the style entries joined with `;`, so `contains` gives `fill:none;;;;fill:none;` in the classic output (probed). The `''` entry is kept.
- Neo (883–892): a dashed edge gets `stroke-dasharray: <dash>; stroke-dashoffset: 0;` before the style, where the dash comes from `generateDashArray(len, start, end)` and `len` is the path's `getTotalLength()`. A solid edge gets `0 <start> <len-start-end> <end>`. The offsets come from `markerOffsets2`, which has no requirement entry, so they are 0. Probed: `0 0 2 2 2 … 0`.
- Data attributes (896–901): `data-edge`, `data-et="edge"`, `data-id`, `data-points`, `data-look`.
- Label (labelType markdown, labelpos `c`): `createText` with the markdown; the group is `g.edgeLabel` with a `g.label` inside, a `rect.background`, and the text. Probed: the SVG source holds `&lt;&lt;satisfies&gt;&gt;`, which shows as `<<satisfies>>`.

### 6.7 Style sheet [read, probed for the output]

`getStyles(options)` (styles.js:50–122) returns the rules below, and `styles.ts` (36–38) calls it for the diagram. `compileCSS` (mermaidAPI.ts:226) prefixes each selector with `#<id>` through stylis, and `createUserStyles` (mermaidAPI.ts:319–333) calls it for the diagram's rules. Selector lists are split (probed: `.label text,span` becomes `#id .label text` and `#id span`).

| Rule (styles.js)                                                                                                                                                               | Lines   | Note                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `genColor()`: `[data-look="<look>"][data-color-id="color-i"].node path` and `… rect`, with `stroke` = border palette `i` and `fill` = background palette `i mod length` (4–48) | 4–48    | Gate: theme in {redux-color, redux-dark-color} and a non-empty border palette (`isColorTheme`, colorThemeGate.ts:33–38). One pair per border slot (12 in redux-color). No `fill` when there is no background palette (redux-dark-color). The look goes through `safeLook`. Probed: 12 pairs in neo; classic has the same rules with `data-look="classic"`. |
| `marker`, `marker.cross`                                                                                                                                                       | 56–63   | fill and stroke = `relationColor`; cross stroke = `lineColor`.                                                                                                                                                                                                                                                                                             |
| `svg`                                                                                                                                                                          | 65–68   | font family and size.                                                                                                                                                                                                                                                                                                                                      |
| `.reqBox`, `.reqTitle, .reqLabel`, `.reqLabelBox`, `.req-title-line`                                                                                                           | 70–88   | **Emitted but unused**: no element of the neo output has these classes (checked on the probe SVG).                                                                                                                                                                                                                                                         |
| `.relationshipLine`                                                                                                                                                            | 89–92   | `stroke` = `relationColor`; `stroke-width` = `strokeWidth` on neo, `1px` on classic (probed: `2` and `1px`).                                                                                                                                                                                                                                               |
| `.relationshipLabel`                                                                                                                                                           | 93–95   | Unused in the output.                                                                                                                                                                                                                                                                                                                                      |
| `.edgeLabel`, `.edgeLabel .label rect`, `.edgeLabel .label text`                                                                                                               | 96–104  | Background and text colours of edge labels.                                                                                                                                                                                                                                                                                                                |
| `.divider`                                                                                                                                                                     | 105–108 | `stroke` = `nodeBorder`, width 1.                                                                                                                                                                                                                                                                                                                          |
| `.label`, `.label text,span`                                                                                                                                                   | 109–116 | Font family; text fill and colour: `nodeTextColor` or `textColor`.                                                                                                                                                                                                                                                                                         |
| `.labelBkg`                                                                                                                                                                    | 117–119 | `requirementEdgeLabelBackground` or `edgeLabelBackground`.                                                                                                                                                                                                                                                                                                 |

Things the sheet does not contain: no rule for a classDef class (no `getClasses` on the renderer, see 6.8), and no `reqBox`-style classes on the nodes. The common header (edge animation, markers, `neo` shadows, `:root` font variable) comes from the shared styles and `mermaidAPI` (probed: the output has the neo drop-shadow rules and `:root{--mermaid-font-family}`).

Theme variables read: `relationColor`, `lineColor`, `fontFamily`, `fontSize`, `requirementBackground`, `requirementBorderColor`, `requirementBorderSize`, `requirementTextColor`, `relationLabelBackground`, `strokeWidth`, `relationLabelColor`, `edgeLabelBackground`, `nodeBorder`, `nodeTextColor`, `textColor`, `requirementEdgeLabelBackground`, and the palettes `borderColorArray` and `bkgColorArray`.

### 6.8 Configuration [probed]

| Key                                                                                                                                                                           | Effective value          | Where it is read                                                                                             | Note                                                                                                                                                                                                                              |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `look`                                                                                                                                                                        | `neo` by default         | `getConfig().look` (`resolveAppearance`, config.ts:79–107)                                                   | The requirement section's default is `neo` (config.schema.yaml:1082–1084; defaultConfig.ts:275–278). Probed: no look gives `neo`; a site `look: classic` gives `classic`; a site `requirement.look: handDrawn` gives `handDrawn`. |
| `theme`                                                                                                                                                                       | `redux-color` by default | themeVariables                                                                                               | The requirement section's default (schema 1079–1081). Probed.                                                                                                                                                                     |
| `layout`                                                                                                                                                                      | `elk`                    | `getRegisteredLayoutAlgorithm` (render) and `config.layout === 'elk'` (body alignment, requirementBox.ts:30) | A layout other than `elk` gives centred body text. [read]                                                                                                                                                                         |
| `state.titleTopMargin`                                                                                                                                                        | 25                       | `insertTitle` (requirementRenderer.ts:31–36)                                                                 | **The requirement renderer reads the `state` section** (requirementRenderer.ts:12). Probed: `state.titleTopMargin: 40` moves the title to `y=-40`.                                                                                |
| `state.useMaxWidth`                                                                                                                                                           | `true`                   | `setupViewPortForSVG` (requirementRenderer.ts:38)                                                            | Read from the source: the `state` value is passed. Both sections default to `true`, so the output does not show which one is read.                                                                                                |
| `state.nodeSpacing`, `state.rankSpacing`                                                                                                                                      | not in `state` → 50      | `data4Layout.nodeSpacing`, `rankSpacing` (22–23)                                                             | Probed: setting `state.nodeSpacing` and `rankSpacing` to 10 does not move the nodes of a two-node diagram.                                                                                                                        |
| `requirement.useMaxWidth`, `rect_fill`, `rect_border_size`, `rect_border_color`, `rect_min_width`, `rect_min_height`, `rect_padding`, `text_color`, `fontSize`, `line_height` | —                        | **Nothing** (styles.js:123 has a leftover comment with `conf.rect_fill`)                                     | [read] A grep of `packages/mermaid/src` finds no reads of these keys in the requirement code or the rendering utilities.                                                                                                          |
| `htmlLabels`                                                                                                                                                                  | false (harness)          | `addText`: `config.htmlLabels ?? true` (requirementBox.ts:216)                                               | The default path (`getBoundingClientRect` in a foreignObject, 255) is not ported (decision 0004).                                                                                                                                 |
| `handDrawnSeed`                                                                                                                                                               | 0                        | `userNodeOverrides`                                                                                          | Used by the handDrawn look only.                                                                                                                                                                                                  |
| `securityLevel`                                                                                                                                                               | strict (harness)         | `getDiagramElement`                                                                                          |                                                                                                                                                                                                                                   |

Dead or misleading config: `requirement.rect_*`, `text_color`, `fontSize` and `line_height` have no effect. `state.titleTopMargin` and `state.useMaxWidth` apply to requirement diagrams.

### 6.9 Randomness, clock and DOM dependencies [read, probed for the sizes]

- Randomness: none in the requirement code, except roughjs with a seed (`handDrawnSeed`) for the handDrawn look, which the harness excludes.
- Clock: none.
- DOM and browser measurements a server renderer must replace: `getBBox` (the label groups, the shape's group and rectangle, the svg root for the viewBox, and text measurement in `calculateTextDimensions`); `getTotalLength` (the neo dash arrays, edges.js:879); `getBoundingClientRect` (only the HTML label path, not ported).

### 6.10 Unified renderer: what is there and what requirement needs

Comparison with `crates/ferrigura-render/src/unified/` (checked on the code, not run):

| Needed by requirement                                                                      | In the unified renderer now                                                                                                                                                                                                    | Gap                                                                                                                                                                                                                                              |
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Shape `requirementBox`                                                                     | `shapes/registry.rs` has no name `requirementBox`; the nearest handlers are `classBox` (`shapes/class_box.rs`) and `erBox` (`shapes/er_box.rs`), which draw their own labels (`Kind::draws_label` excludes them, registry.rs). | **New**: a `Kind::RequirementBox` and `shapes/requirement_box.rs`, which draws its own labels like `erBox`. The helpers exist: `util::styles2string`, `rough::rectangle`, `rough::polygon`, `rough::line`, `label::layout` with `Anchor::Start`. |
| Markers `requirement_arrow`, `requirement_contains` (classic and neo)                      | `markers.rs` has the flowchart kinds, the ER cardinalities and barb. `Kind::from_arrow` (markers.rs:47–58) has no requirement entry.                                                                                           | **New**: four definitions, with `er_markers`-style `neo` handling (markers.rs `er_markers`, 574–). The `Kind` lookup and `definition_name` need the two names.                                                                                   |
| Cluster kinds                                                                              | Clusters exist (`clusters.rs`)                                                                                                                                                                                                 | **None needed**: the reference writes an empty `g.clusters`.                                                                                                                                                                                     |
| Edge: `arrowTypeStart: requirement_contains`, `arrowTypeEnd: requirement_arrow`            | Edge arrow names are looked up in `Kind::from_arrow`                                                                                                                                                                           | Covered by the marker gap.                                                                                                                                                                                                                       |
| Edge: style with an empty entry (`''`)                                                     | `edges.rs` builds the path style the way `edges.js:850–871` does (`edges.rs` 425–470)                                                                                                                                          | Check with a probe when the port exists (classic output shows `;;;;`).                                                                                                                                                                           |
| Edge: `labelType: markdown`, `labelpos: c`, `thickness`, `pattern dashed`, neo dash arrays | Present (`edges.rs` 301–308, 313–316, 483–500; `markdown.rs`)                                                                                                                                                                  | None.                                                                                                                                                                                                                                            |
| Node data: requirement and element fields                                                  | `data.rs` has `entity: Option<EntityFields>` and `class: Option<ClassNodeData>` for the per-type fields                                                                                                                        | **New**: a `requirement` field group on `Node` (the pattern of `entity`).                                                                                                                                                                        |
| `getData`                                                                                  | None for requirement                                                                                                                                                                                                           | **New** module, from 3.5.                                                                                                                                                                                                                        |
| Title `requirementDiagramTitleText`                                                        | Written inline by `er/mod.rs` (74–100) with `er.title_top_margin`                                                                                                                                                              | **Reuse** that writer with `state.title_top_margin` (quirk 6.8).                                                                                                                                                                                 |
| Root attributes, `aria-roledescription="requirement"`                                      | `a11y::root_attributes(id, type, title, descr)` takes the diagram name                                                                                                                                                         | None.                                                                                                                                                                                                                                            |
| Root `class="requirementDiagram"`, padding 8                                               | ER writes padding 8 and its class (er/mod.rs 43–44, 114–126)                                                                                                                                                                   | Same pattern.                                                                                                                                                                                                                                    |
| Style sheet                                                                                | `er/style.rs` `palette_rules` (379) writes per-slot `path` and `rect` rules; `common/style.rs` `base_style_sheet` (30)                                                                                                         | **New** `requirement/style.rs` with the rules of 6.7; reuse `palette_rules`, check its look selector.                                                                                                                                            |
| Neo drop shadow                                                                            | Present (`common/style.rs` writes the filter rules; the state shape uses it)                                                                                                                                                   | None.                                                                                                                                                                                                                                            |
| Markdown in labels                                                                         | Present (`markdown.rs`, Ferromark mapping)                                                                                                                                                                                     | None.                                                                                                                                                                                                                                            |
| Rough fill and outline                                                                     | Present (`rough.rs`, `util::stroke_width` defaults 1.3)                                                                                                                                                                        | None.                                                                                                                                                                                                                                            |
| Layout                                                                                     | ELK Layered (M4), 26 requirement corpus cases                                                                                                                                                                                  | None, once the shape sizes match.                                                                                                                                                                                                                |

Only one shape and four markers are new. Nothing in the unified renderer needs a new cluster kind or edge kind.

### 6.11 Reference harness rules that apply [read]

`tools/port/geometry/common.ts` forces `htmlLabels: false`, `fontFamily: "Lexica Ultralegible"`, `securityLevel: strict` and a shipped theme (`FORCED_SETTINGS` 59–63, `SHIPPED_THEMES` 66, `forcedDirective` 77–85, `patchText` 113–125). Fixtures with `look: handDrawn` are excluded (172, 199–200), as are non-layered layouts and CJK or symbol text (common.ts:150–224). Requirement fixtures use neither CJK nor symbols (section 7.4).

## 7. Fixtures

### 7.1 Counts [read, probed]

172 `.mmd` files: `nightly-check` 168, `pr-check` 2, `visual-check` 2 (plus `.gitkeep` in each folder; 175 files in total). The brief's 172 is the `.mmd` count.

### 7.2 Features [read, counted with a script]

| Feature                                                                                                                                    | Count                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------- |
| Front matter: `description` / `config` / `title`                                                                                           | 170 / 154 / 6                                                         |
| `look` in front matter: `neo` / `handDrawn` / `classic` / none                                                                             | 96 / 24 / 1 / 51 (none means neo, 6.8)                                |
| `layout`: `elk` / none                                                                                                                     | 24 / 148 (none means elk)                                             |
| `theme` in front matter: `default` 1, `neo` 16, `neo-dark` 16, `redux` 16, `redux-color` 16, `redux-dark` 16, `redux-dark-color` 16 / none | 97 / 75                                                               |
| `htmlLabels: false`                                                                                                                        | 27                                                                    |
| `direction`: TB, BT, LR, RL                                                                                                                | 9 each (36 files)                                                     |
| `classDef` / `class` statement / `style` statement / `:::` shorthand                                                                       | 30 / 12 / 18 / 19 files                                               |
| Requirements by kind: requirement / functional / performance / interface / physical / design constraint                                    | 232 / 22 / 19 / 12 / 12 / 12 (309)                                    |
| Elements                                                                                                                                   | 200                                                                   |
| Relationships: satisfies / verifies / contains / traces / derives / refines / copies                                                       | 85 / 39 / 34 / 22 / 36 / 18 / 22 (256)                                |
| Verify methods: test / analysis / inspection / demonstration                                                                               | 175 / 60 / 46 / 19                                                    |
| Quoted values (names, text, type, docref)                                                                                                  | many; 18 values with Markdown emphasis, in the markdown-text fixtures |
| Non-ASCII in the body                                                                                                                      | 0                                                                     |
| Body `title`, accTitle, accDescr                                                                                                           | 0                                                                     |

### 7.3 Reference results [probed, Appendix A]

With the harness's settings (`render-all.json`): 170 render; 2 fail in the reference.

- `visual-check/with-a-theme-default.mmd`: a blank first line, then front matter indented by two spaces. The reference reads `---` as the start of a diagram: `Parse error on line 1`. The `theme: forest` is not read.
- `visual-check/with-a-title-default.mmd`: the closing `---` and the `requirementDiagram` header are indented. The reference reports `Diagrams beginning with --- are not valid`.

Neither file is a port question. The harness would classify both as "reference failed" (common.ts:225–227). Open question 1.

### 7.4 Exclusions the harness will apply [read]

- handDrawn: 24 fixtures (`exclusionReason`, common.ts:196–201).
- Non-layered layouts: none (all layouts are `elk` or unset).
- CJK or uncovered symbols: none (non-ASCII count 0).
- So **146** fixtures are comparable: 172 minus 24 handDrawn minus 2 reference failures.

### 7.5 Features the geometry will exercise [probed]

- Neo look: 147 fixtures (96 explicit plus 51 by default), of which 145 are comparable (the two failing fixtures have no look). Neo markers, the drop shadow, dash arrays from path length, the border palette rules, `data-look="neo"`.
- Classic: 1 explicit fixture, and any site look.
- Markdown in the body (18 values): bold and italic runs (6.3).
- Front matter titles: 6 fixtures, the title text at `y = -25` and the viewBox including it (probed).
- Element with empty body, requirement with empty body: no divider (probed).
- Direction: 36 fixtures. The direction goes to ELK (read); an `LR` diagram renders (probed).

## 8. Risks, effort and open questions

### 8.1 Risks, most serious first

| #   | Risk                                                                                                                                                      | Effect                                                                                                   | Mitigation                                                                                                                           |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| R1  | Lexer and grammar quirks (section 2.5) are easy to miss, and users hit some by accident (a trailing space after `{`, the word `direction` inside a text). | Parse differential fails; users see different errors.                                                    | Port the lexer as an ordered rule list (first match, word-boundary keywords). Use the 43 parser specs and Q1–Q44 as the first tests. |
| R2  | The shape sizes (text stack, `getBBox`, the +6 and +20 and +50 constants, the re-translation by DOM index) feed ELK.                                      | Every node moves; geometry fails across all fixtures.                                                    | Port the stack exactly; check against the 26 ELK corpus cases and the geometry harness on the 146 fixtures.                          |
| R3  | Neo is the default look (145 comparable fixtures).                                                                                                        | Most of the output (markers, shadow, dash arrays, palettes) must be right to pass.                       | Build classic first (smaller), then neo; neo dash arrays reuse the unified code.                                                     |
| R4  | Stylesheet: stylis prefixing of `styles.js`; unused rules must still be emitted; the palette rules must follow the look.                                  | Geometry is unaffected, but the structure check (`tag.class` keys) and the style-dependent colours fail. | Port the rules in order; reuse `palette_rules`.                                                                                      |
| R5  | Reference quirks in 6.8 (state section, dead keys, classDef text colour without class rules).                                                             | Titles and spacing differ for diagrams that set `state`; classDef tests fail if a class rule is written. | Reproduce as listed; document in `PLAN.md`.                                                                                          |
| R6  | Named exceptions: an undeclared relation endpoint and a shared name make the reference fail or print garbage.                                             | Differential and geometry show differences unless excepted.                                              | Open question 2 and 3.                                                                                                               |
| R7  | `getTotalLength` for neo dash arrays.                                                                                                                     | Dash strings differ in the last digit if path length differs.                                            | Reuse the unified path-length code (the flowchart neo edges already use it).                                                         |

### 8.2 Effort

About two-thirds of the ER port in Rust (ER is about 2,800 lines across the syntax, the renderer and the shape, without its markers). Estimate by part, in lines of Rust including tests:

- Lexer, grammar and DB: about 900 (ER's scanner, grammar and DB are about 1,400; the requirement lexer has fewer rules, but the quirks take the difference).
- Model and `getData`: about 250.
- `requirement_box.rs`: about 400 (the `erBox` analogue is 602).
- Markers: about 120 (`er_markers` is about 200).
- Style: about 200 (the ER style module is 438).
- Root and renderer module: about 150.
- Tests: about 400 (the 53 specs plus the probes).

### 8.3 Open questions for the coordinator

1. **Two malformed fixtures.** `visual-check/with-a-theme-default.mmd` and `visual-check/with-a-title-default.mmd` fail in the reference (7.3). Fix the fixtures in e2e, or name them "reference failed" with a reason?
2. **Undeclared relation endpoint** (Q14). The reference fails inside ELK. Should the port return a render error (named exception), or reproduce some output? Recommendation: a named exception, since there is no output to reproduce.
3. **A requirement and an element with the same name** (Q15). The reference writes an SVG with two nodes of the same id and a `translate(undefined…)` error on the console. Named exception (garbage), or reproduce the duplicate ids? Recommendation: a named exception.
4. **Error messages.** Reproduce the first line and the got-token only, or the full Jison `Expecting` list (2.6)? The full list depends on the table of the Jison grammar, which the hand-written parser does not have.
5. **Canonical texts.** Keep the placeholders (`ﬂ°…¶ß`) that the DB holds (recommended, as the reference DB holds them), or decode them as the drawing does?
6. **`state` section quirk.** Reproduce the title margin and `useMaxWidth` read from `state` (recommended, the reference does it), and the dead `rect_*` keys (accepted, no effect)?
7. **Default look `neo`.** Confirmed by probe. Reproduce (recommended).
8. **Layout other than `elk`.** Body text would be centred, as in the reference (requirementBox.ts:30). No fixture reaches it. Reproduce the check, or ignore? Recommendation: reproduce.
9. **`htmlLabels: true`.** Not ported (decision 0004). Confirm.
10. **Direction kept across `clear`** (Q36). No effect in practice (new DB per diagram). Keep the field in the model, or drop it?

## Appendix A. How the probes were run

Probe scripts and outputs are in the session scratch directory, outside the repository. The only file this task wrote in the repository is `docs-port/requirement/NOTES.md`. Other untracked `docs-port/<type>/NOTES.md` folders (c4, cose, mindmap, packet, and others) appeared in this worktree during the session; they are not from this task, and the stash stack was not touched.

- **Parser and DB** (`probe-lib.mts`, `probe-parse.mts`, `probe-more.mts`, `probe-data.mts`, `probe-errors.mts`, `probe-config.mts`): run with `tsx` from the repository root. The reference modules load through `tools/port/reference.ts` (`openReference`, Vite SSR with the Jison and schema plugins), as the exporters do. The text goes through `preprocessDiagram`, then `encodeEntities` and a newline, then the Jison parser with the DB as `yy`, as `Diagram.fromText` does (Diagram.ts:49–77). Outputs: `parse-results.json`, `data-results.json`.
- **Rendering** (`render-probe.mjs`, `render-all.mjs`): the built bundle `packages/mermaid/dist/mermaid.esm.mjs` served from a local HTTP server, loaded in Playwright's Chromium (installed) with the harness's forced settings (`tools/port/geometry/common.ts:391, 542, 577`), then `mermaid.render`. The bundle was built on 2026-10-16 and was not rebuilt. Outputs: `svg-*.svg`, `render-all.json`.
- **Specs**: `vitest run packages/mermaid/src/diagrams/requirement` with the JSON reporter, 53 of 53 passing (`vitest-req.json`).
- **Counts** (features, relations, values): a Python script over the fixtures (`classify.py`, `fm.py` and inline counts).

Unverified, and why: the wrap width under placeholders (6.3), the classic and neo divider stroke geometry beyond the probed counts (the port's rough output has to match Chromium's `getBBox`), the element divider rule with Doc Ref only (read, not rendered), and the `htmlLabels: true` path (not run).
