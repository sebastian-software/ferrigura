# ARCHITECTURE diagram and fcose: research notes for P2

Research only, for the port of `architecture-beta` to ferrigura (P2) and of its layout engine, fcose. Nothing in the repository was changed and nothing was committed. Reference: `aa5c4338f` (HEAD of `sebastian/port-p2`).

The reference bundle is `packages/mermaid/dist` (built 2026-10-16 11:57). It was not rebuilt. The architecture code in it matches the source: the built chunk `chunks/mermaid.esm/architectureDiagram-WDQPCYRD.mjs` has the same seeded wrapper and the same two `layout.run()` calls (lines 9020 and 9023 of that chunk).

Conventions: **[source]** = read from the code, `file:line` in the reference unless a path says otherwise. **[probed]** = checked by running the reference (Chromium through Playwright, on the dist bundle, or a library from `node_modules`); the method is named where it matters. **[unverified]** = neither could confirm it.

Short names used below:

| Name       | Path under `node_modules/.pnpm/`                                                                                                                                        |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CYTO       | `cytoscape@3.34.0/node_modules/cytoscape/src` (ESM source; the package ships it)                                                                                        |
| FCOSE      | `cytoscape-fcose@2.2.0_cytoscape@3.34.0/node_modules/cytoscape-fcose/src/fcose` (the mermaid build is `cytoscape-fcose.js` next to it; the same logic, checked by grep) |
| COSE2      | `cose-base@2.2.0/node_modules/cose-base/src` (used by fcose)                                                                                                            |
| LB2        | `layout-base@2.0.1/node_modules/layout-base/src` (used by COSE2 and FCOSE)                                                                                              |
| COSE1, LB1 | `cose-base@1.0.3`, `layout-base@1.0.2` (used by cytoscape-cose-bilkent, the other task)                                                                                 |

Scratch material (not committed): `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/arch-probe/`. The scripts are `probe.mjs` (fixture renders, determinism, seed sensitivity, geometry), `count.mjs` (random draws per call site, on an instrumented copy of the dist), `variant*.mjs` and `order.mjs` (copies of the dist with the second layout run removed or moved), `font*.mjs` (cytoscape label font), `knobs.mjs` (fcose options), `initpos.mjs` (initial positions, instrumented copy), `parseprobe*.mjs` (parser inputs), and `vitest` runs of the three architecture spec files. The instrumented copies are under that folder; the repository's `dist` was never edited.

---

## 0. Summary

`architecture-beta` is a small type. Its DB keeps groups, services, junctions, edges and `align` hints. Its renderer does not lay out anything itself: it puts every group, service and junction into a cytoscape instance and runs **fcose** (`cytoscape-fcose` 2.2.0) twice. Then it draws the SVG from the positions cytoscape reports. So the port is mostly the layout engine, plus the cytoscape pieces that the renderer reaches.

Rough size (the port estimates are my judgement, not measured):

| Part                                                                                                                                               | Reference                                                                                             | Port estimate                                                             |
| -------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Parser: Langium grammar and value converter, replaced by a hand-written parser (decision 0002)                                                     | `packages/parser/src/language/architecture/*` (about 100 lines), `architectureParser.ts` (36)         | 400-600 lines of Rust                                                     |
| DB                                                                                                                                                 | `architectureDb.ts` (425)                                                                             | 500 lines                                                                 |
| Renderer, SVG drawing, types, styles, seed, icons                                                                                                  | `architectureRenderer.ts` (645), `svgDraw.ts` (389), `architectureTypes.ts` (389), the rest about 150 | 1,500-1,800 lines, including the text helpers shared with M5/M6           |
| Layout: fcose, cose-base 2.2.0, layout-base 2.0.1 (the parts reached)                                                                              | FCOSE 1,369 lines; COSE2 3,073; LB2 5,345 (about half reached)                                        | 5,000-7,000 lines, about half of it shared with cose-bilkent (section E9) |
| cytoscape pieces reached: endpoints, segment control points, midpoints, label dimensions, compound bounds, `layoutPositions`, six style properties | about 3,600 lines of CYTO; about 1,500 reached                                                        | 1,500-2,000 lines                                                         |

Riskiest parts, most serious first:

1. **Numerics of the force phase.** A render draws between 0 and about 1,250 numbers from a seeded generator, both runs together (Section E4), and the spring embedder has discrete decisions (convergence, grid cells, sign tests). A port that does not replay the reference's operation order and draw order produces different positions. Probed: computing the XY-edge weights after the second run instead of before it changes the drawn paths of three fixtures by up to 137 px.
2. **Mermaid's two-run pipeline.** The renderer runs fcose, then runs it again inside its own `layoutstop` handler (`architectureRenderer.ts:513-580`). Probed: the second run shifts 18 of 20 fixtures by one translation (7.5-11 px vertically in most) and reshapes two. In the complex nested diagram the shift differs between nodes by up to 152 px in x and 312 px in y; in `groups-within-groups` by up to 4 px. The port must do both runs.
3. **Canvas text measurement of labels inside the layout.** cytoscape measures node labels with `measureText` in the font `Helvetica Neue, Helvetica, sans-serif` (CYTO `labels.mjs:489-545`, default from `style/properties.mjs:641`). That is not the font the harness forces (`Lexica Ultralegible`). Probed: the width of a group depends on the title of its child (165 px for a short title, 323 px for a long one). With Lexica measurements, 2 of 20 fixtures change (one group 4 px narrower; 0.25 px shift in another). Ferrigura cannot measure Helvetica Neue. This needs a decision (section 8, Q1).
4. **DOM-measured geometry.** The viewBox comes from `getBBox()` of the whole SVG (`setupGraphViewbox.js:50`), the wrapped XY-edge labels from `getBoundingClientRect()` (`svgDraw.ts:168,176`), and the text wrapping from `getComputedTextLength()` (`createText.ts:122`). All three depend on glyph metrics that M5 already measures; the XY-label path also assumes an unscaled SVG.
5. **Parser quirks.** Groups are added before services and junctions regardless of text order; edges may name a service declared later; an edge to a group id crashes with a raw `TypeError`; some keywords cannot be service ids. All probed (section 2.4).

Open questions for the coordinator are in section 8. The two that change plans: the Helvetica Neue decision (Q1), and P2-PLAN's description of the random draws and of "the same cose-base" (Q2, Q3). Both are wrong for this type; section 8 says how.

---

## 1. Sources

### 1.1 Mermaid files

| Role                 | File                                                                 | Lines                                            | Note                                                                                                                  |
| -------------------- | -------------------------------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| Detector             | `packages/mermaid/src/diagrams/architecture/architectureDetector.ts` | 7-22                                             | `/^\s*architecture/` (line 10)                                                                                        |
| Diagram registration | `architectureDiagram.ts`                                             | 7-14                                             | `db` is a getter that makes a new `ArchitectureDB` on each access (lines 9-11)                                        |
| Parser wrapper       | `architectureParser.ts`                                              | 8-18 (populate), 20-36 (parser)                  | Order: common, groups, services, junctions, edges, alignments (lines 9-17)                                            |
| Grammar              | `packages/parser/src/language/architecture/architecture.langium`     | 1-57                                             | Imports `../common/common` and `arch`                                                                                 |
| Terminals            | `packages/parser/src/language/architecture/arch.langium`             | 1-2                                              | `ARCH_ICON`, `ARCH_TITLE`                                                                                             |
| Common terminals     | `packages/parser/src/language/common/common.langium`                 | 6-34                                             | `EOL`, `ID`, `STRING`, `NEWLINE`, `WHITESPACE`, `YAML`, `DIRECTIVE`, `SINGLE_LINE_COMMENT`, `TitleAndAccessibilities` |
| Value converter      | `packages/parser/src/language/architecture/valueConverter.ts`        | 5-30                                             | `ARCH_ICON` (11-12), `ARCH_TITLE` (15-26)                                                                             |
| Keywords             | `packages/parser/src/language/architecture/tokenBuilder.ts`          | 3-7                                              | Keywords `architecture`, `architecture-beta`; the common builder adds the end rule (`common/tokenBuilder.ts:14-28`)   |
| Parse entry          | `packages/parser/src/parse.ts`                                       | 133-151 (parse), 166-190 (error text)            |                                                                                                                       |
| DB                   | `architectureDb.ts`                                                  | 41-425                                           | See section 3                                                                                                         |
| Types and helpers    | `architectureTypes.ts`                                               | 10-389                                           | Directions, arrows, shifts, group-alignment key, interfaces                                                           |
| Renderer             | `architectureRenderer.ts`                                            | 40-645                                           | See section 6                                                                                                         |
| SVG drawing          | `svgDraw.ts`                                                         | 25-389                                           | `drawEdges` 25-191, `drawGroups` 193-274, `drawServices` 276-362, `drawJunctions` 364-389                             |
| Icons                | `architectureIcons.ts`                                               | 4-43                                             | Seven icons (section 6.9)                                                                                             |
| Styles               | `architectureStyles.ts`                                              | 4-37                                             |                                                                                                                       |
| Seed                 | `architectureSeed.ts`                                                | 13-31                                            | `withSeededRandom`                                                                                                    |
| Config               | `packages/mermaid/src/schemas/config.schema.yaml`                    | 1113-1186                                        | `ArchitectureDiagramConfig` (defaults)                                                                                |
| Config type          | `packages/mermaid/src/config.type.ts`                                | 1551-1601                                        | Doc comments                                                                                                          |
| Preprocessing        | `packages/mermaid/src/preprocess.ts`                                 | 6-17, 19-31, 33-45, 58-91                        | `cleanupText`, front matter, directives, `preprocessDiagram`                                                          |
| Comments             | `packages/mermaid/src/diagram-api/comments.ts`                       | 6-8                                              |                                                                                                                       |
| Entities             | `packages/mermaid/src/utils.ts`                                      | 874-906                                          | `encodeEntities` and `decodeEntities`                                                                                 |
| Diagram              | `packages/mermaid/src/Diagram.ts`                                    | 19-93                                            | `fromText`: `db.clear()` (49), `encodeEntities(source) + '\n'` (77), `parser.parse` (80)                              |
| Common DB            | `diagrams/common/commonDb.ts`                                        | 1-32                                             | Module-level `accTitle`, `diagramTitle`, `accDescription`                                                             |
| Common population    | `diagrams/common/populateCommonDb.ts`                                | 4-14                                             | Order: accDescr, accTitle, title                                                                                      |
| Accessibility        | `accessibility.ts`                                                   | 22-30                                            | `role`, `aria-roledescription`                                                                                        |
| Render wrapper       | `mermaidAPI.ts`                                                      | 419 (empty `g`), 623 and 750-757 (accessibility) |                                                                                                                       |
| Styles assembly      | `styles.ts`                                                          | 34-38, 115                                       | Diagram styles are inserted into the stylesheet                                                                       |
| Icons                | `rendering-util/icons.ts`                                            | 20-24, 29-46, 48-77, 88-106                      | `unknownIcon`, registry, lookup, `getIconSVG`                                                                         |
| Text                 | `rendering-util/createText.ts`                                       | 114-125, 152-200, 306-380                        |                                                                                                                       |
| Viewport             | `setupGraphViewbox.js`                                               | 23-33, 49-70                                     |                                                                                                                       |
| Root selection       | `rendering-util/selectSvgElement.ts`                                 | 11-22                                            |                                                                                                                       |

### 1.2 Libraries

| Library                | Version and resolution                                                                                 | Evidence                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| cytoscape              | 3.34.0 (`packages/mermaid/package.json:75`)                                                            | `package.json` `main` is the CJS build; the ESM source (`src/`) was read |
| cytoscape-fcose        | 2.2.0 (`package.json:77`); its `dependencies` is `cose-base ^2.2.0`                                    | [source] `cytoscape-fcose/package.json`                                  |
| cose-base              | 2.2.0 for fcose; its dependency is `layout-base ^2.0.0`, resolved to 2.0.1                             | [source] `cose-base@2.2.0/.../package.json`                              |
| cytoscape-cose-bilkent | 4.1.0, dependency `cose-base ^1.0.0`, resolved to 1.0.3, which depends on `layout-base ^1.0.0` (1.0.2) | [source]                                                                 |

The mermaid package does not list `cose-base` or `layout-base` itself. The two engines therefore load **different copies**: fcose uses cose-base 2.2.0 and layout-base 2.0.1; cose-bilkent uses cose-base 1.0.3 and layout-base 1.0.2. Section E9 compares them.

---

## 2. Parser

### 2.1 The text pipeline before the grammar [source]

`mermaidAPI` calls `preprocessDiagram` and `Diagram.fromText`. The steps, in order:

1. `cleanupText`: CRLF and CR become LF; attributes in HTML-like tags get single quotes (`preprocess.ts:6-17`).
2. Front matter is extracted and its `config` merged (`preprocess.ts:19-31`). Architecture fixtures use it (`config: architecture: seed: 42` and similar).
3. Directives: the `%%{init}%%` objects and `wrap` (`preprocess.ts:33-45`).
4. `cleanupComments`: removes every line that starts with `%%` but not `%%{`, then `trimStart` (`comments.ts:6-8`).
5. `Diagram.fromText`: `detectType` (the detector regex), `db.clear()` (line 49), `encodeEntities` (line 77). `encodeEntities` rewrites `#name;` to a private form (`utils.ts:874-896`) and a newline is appended.
6. Langium parses the text. The architecture grammar ignores `YAML` and `DIRECTIVE` (`common.langium:32-33`), and `SINGLE_LINE_COMMENT` (`%%` to end of line, `common.langium:34`) is hidden.

Consequence for the port: the entity encoding happens for every diagram, so a service title with `#35;` in it is encoded and decoded before the SVG (`decodeEntities` runs in `createText` at `createText.ts:366` for the SVG branch, and `decodeHTMLEntities` at `createText.ts:213-253` decodes the words of each line). This is not exercised by any architecture fixture.

### 2.2 Grammar [source]

```
Architecture : NEWLINE* ('architecture' | 'architecture-beta') (NEWLINE | TitleAndAccessibilities | Statement)*
Statement    : Group | Service | Junction | Edge | Alignment
Group        : 'group' id=ID icon=ARCH_ICON? title=ARCH_TITLE? ('in' in=ID)? EOL
Service      : 'service' id=ID (iconText=STRING | icon=ARCH_ICON)? title=ARCH_TITLE? ('in' in=ID)? EOL
Junction     : 'junction' id=ID ('in' in=ID)? EOL
Edge         : lhsId=ID lhsGroup=ARROW_GROUP? Arrow rhsId=ID rhsGroup=ARROW_GROUP? EOL
Arrow        : ':' lhsDir=ARROW_DIRECTION  lhsInto=ARROW_INTO? ('--' | '-' title=ARCH_TITLE '-')  rhsInto=ARROW_INTO?  rhsDir=ARROW_DIRECTION ':'
Alignment    : 'align' direction=('row' | 'column') members=ID (members=ID)+ EOL
```

(Source: `architecture.langium:5-53`, `arch.langium`, `common.langium`.)

Terminals:

| Terminal          | Pattern                              | Source                    |
| ----------------- | ------------------------------------ | ------------------------- |
| `ARROW_DIRECTION` | `L`, `R`, `T`, `B`                   | `architecture.langium:55` |
| `ARROW_GROUP`     | `{group}`                            | `architecture.langium:56` |
| `ARROW_INTO`      | `<` or `>`                           | `architecture.langium:57` |
| `ARCH_ICON`       | `\([\w-:]+\)`                        | `arch.langium:1`          |
| `ARCH_TITLE`      | `\[(?:"..."\|'...'\|[^\[\]\r\n]+)\]` | `arch.langium:2`          |
| `ID`              | `[\w]([-\w]*\w)?` (no trailing dash) | `common.langium:27`       |
| `STRING`          | `"..."` or `'...'`                   | `common.langium:22`       |
| `EOL`             | `NEWLINE+` or end of input           | `common.langium:6`        |

Value conversion [source]: an icon loses its parentheses and spaces (`valueConverter.ts:11-12`); a title loses its outer brackets and spaces, then one pair of matching quotes, and `\"` and `\'` are unescaped (`valueConverter.ts:15-26`). The title regex keeps `(`, `)` and everything except brackets and line breaks.

### 2.3 Lexer and grammar quirks [probed]

The probes ran `mermaid.parse()` from the dist bundle in Chromium (`parseprobe.mjs`, `parseprobe2.mjs`, `parseprobe3.mjs`). Each row is one input; "OK" means the parser accepted it.

| #   | Input (service or edge line)                                                                              | Reference result                                                                          | Note                                                                                          |
| --- | --------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Q1  | `architecture` (no `-beta`)                                                                               | OK                                                                                        | Both keywords are accepted                                                                    |
| Q2  | `service L(server)[A]`, `service group(...)`, `service in(...)`, `service row(...)`, `service align(...)` | Parse error, "Expecting token of type 'ID'"                                               | Exact keyword ids are rejected                                                                |
| Q3  | `service rowspan(...)`, `service columnar(...)`                                                           | OK                                                                                        | Only exact words; the grammar keyword match needs the word end                                |
| Q4  | `service a-(...)` (ends with a dash)                                                                      | Parse error                                                                               | `ID` cannot end with `-`                                                                      |
| Q5  | `service a-b_c(...)`, `service 1a(...)`                                                                   | OK                                                                                        |                                                                                               |
| Q6  | `Service a(...)` (capital S)                                                                              | Parse error                                                                               | Keywords are case-sensitive                                                                   |
| Q7  | `service a "Hello"[A]` (no parentheses)                                                                   | OK                                                                                        | iconText is written **without** parentheses                                                   |
| Q8  | `service a("Hello")[A]`                                                                                   | Lexer error at `(`                                                                        | `ARCH_ICON` does not match quotes                                                             |
| Q9  | `service a( server )[A]`                                                                                  | Lexer error at `(`                                                                        | `ARCH_ICON` has no spaces                                                                     |
| Q10 | `service a(server)[]`                                                                                     | Lexer error at `[`                                                                        | `ARCH_TITLE` needs one character                                                              |
| Q11 | `service a(server)[A (b)]`, `['A x']`, `["A quoted, title"]`                                              | OK                                                                                        |                                                                                               |
| Q12 | `service a(logos:aws)[A]`, `service a(nope:thing)[A]`                                                     | OK                                                                                        | Colons are part of the icon name                                                              |
| Q13 | `service a(server)` (no title)                                                                            | OK                                                                                        |                                                                                               |
| Q14 | `service a(server)[A] %% note`                                                                            | OK                                                                                        |                                                                                               |
| Q15 | `service a(server)[A] service b(server)[B]` (two statements on one line)                                  | Parse error                                                                               | Each statement needs its own line                                                             |
| Q16 | `a:R -- L:b`                                                                                              | OK                                                                                        |                                                                                               |
| Q17 | `a:R --> L:b`, `a:R <-- L:b`, `a:R <--> L:b`, `a:L <-- R:b`                                               | OK                                                                                        | `ARROW_INTO` may sit on either side                                                           |
| Q18 | `a:R -- R:b` (same direction on both ends)                                                                | OK                                                                                        | The pair is not checked by the grammar or the DB; the BFS ignores only `LL`, `RR`, `TT`, `BB` |
| Q19 | `a -- b` (no ports)                                                                                       | Parse error, "Expecting ':'"                                                              |                                                                                               |
| Q20 | `a:X -- L:b`, `a:l -- r:b`                                                                                | Parse error                                                                               | Directions are upper-case `L R T B`                                                           |
| Q21 | `a:R -[x y]- L:b` (label with spaces)                                                                     | OK                                                                                        |                                                                                               |
| Q22 | `a:R -[]- L:b` (empty label)                                                                              | Lexer error                                                                               |                                                                                               |
| Q23 | `a:R --- L:b`                                                                                             | Parse error                                                                               |                                                                                               |
| Q24 | `a{group}:R -- L:b{group}` (across two groups)                                                            | OK                                                                                        |                                                                                               |
| Q25 | `a{group}:R -- L:b` with both in one group                                                                | DB error, "...traverse the group boundary, but the edge does not pass through two groups" | `architectureDb.ts:227-235`                                                                   |
| Q26 | Edge to a group id, `a:R -- L:g` or `g:R -- L:a`                                                          | Raw `TypeError`: "Cannot read properties of undefined (reading 'in')"                     | `architectureDb.ts:225-226`: `this.nodes.get(lhsId)!.in` on a group id                        |
| Q27 | `a:R -- L:zzz` (unknown id)                                                                               | DB error, "The right-hand id [zzz] does not yet exist..."                                 | `architectureDb.ts:219-223`                                                                   |
| Q28 | `a:X` (invalid direction)                                                                                 | Parse error, "Expecting token of type 'ARROW_DIRECTION'"                                  |                                                                                               |
| Q29 | `junction j` (no `in`)                                                                                    | OK                                                                                        |                                                                                               |
| Q30 | `junction j[J]` (title on a junction)                                                                     | Parse error                                                                               | Junctions have no title                                                                       |
| Q31 | `align row a` (one member)                                                                                | Parse error                                                                               | Grammar needs at least two members                                                            |
| Q32 | `align row a a`                                                                                           | DB error, "align row lists [a] more than once"                                            | `architectureDb.ts:276-278`                                                                   |
| Q33 | `align row g b` where `g` is a group                                                                      | DB error, "align row references [g], which is not a service or junction"                  | `architectureDb.ts:271-273`                                                                   |
| Q34 | `align column a b`                                                                                        | OK                                                                                        |                                                                                               |
| Q35 | `title My Diagram`, `accTitle: ...`, `accDescr: ...`, `accDescr { ... }`                                  | OK                                                                                        |                                                                                               |
| Q36 | `%%{init: {...}}%%` inside the body                                                                       | OK                                                                                        |                                                                                               |
| Q37 | Tabs for indentation                                                                                      | OK                                                                                        |                                                                                               |
| Q38 | No newline at the end                                                                                     | OK                                                                                        |                                                                                               |

Declaration order [probed; the order comes from `architectureParser.ts:9-17`]: `populateDb` adds **all groups first**, then all services, then all junctions, then all edges, then the alignments. So:

| #   | Input                                                   | Result                                                                   |
| --- | ------------------------------------------------------- | ------------------------------------------------------------------------ |
| O1  | `service a(server)[A] in g` then `group g(cloud)[G]`    | OK (the group is added first)                                            |
| O2  | `group g2(cloud)[G2] in g1` then `group g1(cloud)[G1]`  | DB error, "The group [g2]'s parent does not exist..."                    |
| O3  | `group g1(...)` then `group g2(...) in g1`              | OK                                                                       |
| O4  | `service s(...)` then `group g(...) in s`               | DB error, "The group [g]'s parent does not exist" (not "is not a group") |
| O5  | An edge naming a service declared later                 | OK                                                                       |
| O6  | `align row a b` before `service a`/`service b`          | OK                                                                       |
| O7  | A junction referenced by an edge before its declaration | OK                                                                       |

Consequence: the "is not a group" branch for groups (`architectureDb.ts:175-177`) can never fire on this path, because every group is registered before any service or junction. The "is not a group" branch for services and junctions (`architectureDb.ts:96-98, 133-135`) can fire only when the parent is a service declared earlier. [source for the second part; not probed].

Error texts that a port should reproduce exactly (they reach the user): the DB messages (`architectureDb.ts:84-96, 129-134, 162-177, 205-235, 264-278`) and the Langium messages through `MermaidParseError` (`parse.ts:166-190`: "Parsing failed: Lexer error on line L, column C: ... Parse error on line L, column C: ...").

### 2.4 What the DB receives from each rule [source]

| Rule      | DB call                                               | Stored                                                               |
| --------- | ----------------------------------------------------- | -------------------------------------------------------------------- |
| Group     | `addGroup({id, icon, in, title})`                     | `groups` map, `registeredIds` = `group`                              |
| Service   | `addService({...service, type: 'service'})`           | `nodes` map, `registeredIds` = `node`, `edges: []`                   |
| Junction  | `addJunction({...junction, type: 'junction'})`        | `nodes` map, `registeredIds` = `node`, `edges: []`                   |
| Edge      | `addEdge(edge)`                                       | `edges` array; pushed to each endpoint's `edges` when both are nodes |
| Alignment | `addLayoutHint({direction, members: [...]})`          | `layoutHints` array                                                  |
| Header    | `setAccDescription`, `setAccTitle`, `setDiagramTitle` | Module-level strings (see section 3)                                 |

### 2.5 Errors [probed]

Parse failures: `Parsing failed: Lexer error on line L, column C: <message>` or `Parse error on line L, column C: <message>` (`parse.ts:166-190`). DB failures: plain `Error` messages (section 2.3). Renderer: a `RangeError` with "Invalid array length" from fcose's `calcGrid` is replaced by a longer message that names `align row|column` (`architectureRenderer.ts:591-598`). No fixture reaches it, and the path is unverified.

---

## 3. DB

### 3.1 Calls in order [source]

`populateDb` (`architectureParser.ts:8-18`):

1. `populateCommonDb(ast, db)`: `setAccDescription` if present, then `setAccTitle`, then `setDiagramTitle` (`populateCommonDb.ts:4-13`).
2. `addGroup` for each group, in text order.
3. `addService` for each service, in text order.
4. `addJunction` for each junction, in text order.
5. `addEdge` for each edge, in text order.
6. `addLayoutHint` for each alignment, in text order.

### 3.2 What each call stores and checks [source]

- `addService` (`architectureDb.ts:75-112`): checks the id is free (84-86), the parent is not the id (88-90), the parent exists (91-95), the parent is a group (96-98). Stores `{id, type: 'service', icon, iconText, title, edges: [], in}`. The `title` is stored as given.
- `addJunction` (`118-146`): the same checks, minus the icon and title. Stores `{id, type: 'junction', edges: [], in}`.
- `addGroup` (`160-188`): the same checks. Stores `{id, icon, title, in}` in `groups`.
- `addEdge` (`192-257`): both directions must be L, R, T or B (203-212); both endpoints must exist as a node or a group (214-223); the `{group}` modifier must cross two groups (225-236). Stores the edge as `{lhsId, lhsDir, lhsInto, lhsGroup, rhsId, rhsDir, rhsInto, rhsGroup, title}` and pushes the same object into both endpoints' `edges`, when both are nodes (250-256).
- `addLayoutHint` (`263-282`): at least two members (264-268); each member is a service or junction (271-273); no member twice (276-278). The check of "at least two" in the DB is reached only through the API; the grammar already guarantees it.

Defaults [source]: an absent icon, iconText, title or parent is `undefined`. Edge flags are `undefined` when the arrow has no `>`/`<` or `{group}`. Renderer code tests them for truth, so `undefined` and `false` behave the same. A port's canonical model writes `""` and `false` (section 4).

### 3.3 Sanitising [source]

- Header texts pass through `sanitizeText` in the setters (`commonDb.ts:16-29`). `setAccTitle` also strips leading whitespace; `setAccDescription` strips whitespace after each line break.
- Service, group and junction texts are **not** sanitised in the DB. They are sanitised in the renderer only where the renderer chooses HTML. The architecture renderer always uses SVG text (`useHtmlLabels: false`, `svgDraw.ts:148, 249, 293`), so the text is not run through `sanitizeText` at all. The only sanitised architecture text is the `iconText` (`svgDraw.ts:332`) and every icon SVG (`icons.ts:105`).

### 3.4 Clear and state [source]

`clear()` (`architectureDb.ts:63-73`) resets the maps, the arrays, `registeredIds`, the cached `dataStructures`, the element map and the diagram id, then calls `commonClear()`. The common module keeps `accTitle`, `diagramTitle` and `accDescription` as **module-level variables shared by every diagram type**; `clear()` is the only reset. `Diagram.fromText` calls `db.clear()` before every parse (`Diagram.ts:49`). The constructor also clears (`architectureDb.ts:51-53`).

Configuration is read at call time through `getConfig()` (`architectureDb.ts:395-400`), which merges the defaults with the user's `architecture` object. Nothing is cached.

### 3.5 `getDataStructures` [source]

Memoised in `this.dataStructures` (`architectureDb.ts:293-385`), reset only by `clear()`. It computes three things:

1. **Group alignments** (`groupAlignments`, `architectureDb.ts:302-313`): for every edge at every node whose two endpoints lie in different groups, and whose direction pair is not a bend, the key is `architectureGroupAlignmentKey(groupA, groupB)` (`architectureTypes.ts:283-286`: the two group ids sorted, each JSON-quoted, joined by `-`). The value is `horizontal` or `vertical`. A later edge overwrites an earlier one for the same group pair.
2. **Adjacency** (`adjList`, lines 315-331): for each node, a map from direction pair (for example `RL`) to the neighbour id. The pair is built from the node's own direction first (`getArchitectureDirectionPair(lhsDir, rhsDir)` for the left endpoint). Invalid pairs `LL`, `RR`, `TT`, `BB` are dropped (`architectureTypes.ts:103-107, 119-125`). A second edge with the same pair from the same node **overwrites** the first, so one of the two edges is invisible to the BFS. [source]
3. **Spatial maps** (lines 333-376): a BFS from the first unvisited node in insertion order (`this.nodes`, services and junctions interleaved as added). The start is at `[0, 0]`. Each neighbour is placed at the current position shifted by the pair (`shiftPositionByArchitectureDirectionPair`, `architectureTypes.ts:133-152`). `visited` is set when a node is **dequeued** (line 344), not when it is queued. So a node that is queued twice is placed again by the second path, and its position is overwritten (lines 359-364). One spatial map is produced per connected component.

The BFS order (the iteration order of `this.nodes` and of each node's `adjList` map, both insertion order) determines the grid positions, and so the alignment and relative constraints. A port must keep both orders.

---

## 4. Canonical model proposal

The differential compares the DB output with the canonical model in `docs-port/canonical-model.md`. Fields in camelCase, `version: 1`. Absent strings are `""` and absent booleans are `false`. The renderer treats `""` and `undefined` alike (`if (service.title)` at `svgDraw.ts:287`), so this loses nothing.

```json
{
  "version": 1,
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "groups": [{ "id": "", "icon": "", "title": "", "in": "" }],
  "services": [{ "id": "", "icon": "", "iconText": "", "title": "", "in": "" }],
  "junctions": [{ "id": "", "in": "" }],
  "edges": [
    {
      "lhsId": "",
      "lhsDir": "L",
      "lhsInto": false,
      "lhsGroup": false,
      "rhsId": "",
      "rhsDir": "R",
      "rhsInto": false,
      "rhsGroup": false,
      "title": ""
    }
  ],
  "alignments": [{ "direction": "row", "members": [""] }],
  "derived": {
    "groupAlignments": [["\"A\"-\"B\"", "horizontal"]],
    "spatialMaps": [[{ "id": "", "x": 0, "y": 0 }]]
  }
}
```

Order: groups, services, junctions and alignments keep text order (the canonical rule for ordered lists); services and junctions are separate lists, so their interleaving in `this.nodes` is not part of the model. The `derived` block exists because the layout inputs depend on it and the BFS quirks of section 3.5 are otherwise invisible.

Left out, and why: `edges` on each node (a cycle through the model); the element map (a renderer object); the diagram id (set by the renderer); the configuration (compared by the config differential, like the other types); `adjList` (an intermediate of the BFS, reproduced through `spatialMaps`).

---

## 5. Specs

Three files, 40 test cases, run here with `node_modules/.bin/vitest run packages/mermaid/src/diagrams/architecture` (nothing written):

| File                       | Blocks | Cases                     | Result here | What it covers                                                                                                                                                                                                                                                                                                              |
| -------------------------- | ------ | ------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `architecture.spec.ts`     | 30     | 32 (one `it.each` over 3) | 32 pass     | Keyword and minimal definitions; title and accessibility lines; a cytoscape stylesheet warning check (headless cytoscape); the `randomize` default; the fcose knob defaults and round-trip; missing group; `__proto__` group and service names; service order; `align` parsing and errors; junction validation; issue #7408 |
| `architectureSeed.spec.ts` | 7      | 7                         | 7 pass      | `withSeededRandom`: same seed, same sequence; different seeds differ; values in `[0,1)`; restore on return and on throw; seed 0 leaves `Math.random` alone                                                                                                                                                                  |
| `svgDraw.spec.ts`          | 1      | 1                         | **1 fails** | "should add ids" renders a diagram in jsdom. cytoscape's canvas renderer needs a 2D context: "Could not create canvas of type 2d" (`cytoscape.esm.mjs:1507`). The jsdom canvas is not available in this checkout. [probed by running vitest]                                                                                |

What the specs do not cover: the layout itself. Nothing in the unit suite checks node positions, edge geometry or label sizes; those are covered only by the e2e fixtures (section 7). The port needs its own tests for the layout (section E7). The parser test titles in `architecture.spec.ts` (for example "should reject a service whose id is the exact reserved keyword align") are worth keeping as the port's test names.

---

## 6. Renderer

### 6.1 The pipeline in `draw` [source]

`draw` (`architectureRenderer.ts:610-643`):

1. `db.setDiagramId(id)` (614); read the services, junctions, groups, edges and data structures (616-620).
2. Select the root SVG with `selectSvgElement(id)` (622). It is `#id` in `body`, or in the sandbox iframe.
3. Append three groups to the root, in this order: `architecture-edges`, `architecture-services`, `architecture-groups` (624-631).
4. `drawServices` into the services group, then `drawJunctions` (633-634). These measure each element but do not use the measurements (6.8).
5. `layoutArchitecture` (636): creates a cytoscape instance, adds groups, services, junctions and edges, runs fcose twice (section E3) and resolves when cytoscape is ready.
6. `drawEdges` (638), `drawGroups` (639): read the laid-out cytoscape elements and draw SVG.
7. `positionNodes` (640): sets `transform="translate(x,y)"` on each service and junction from the cytoscape node position (lines 82-94).
8. `setupGraphViewbox` (642): sizes and frames the root SVG from `getBBox()` (section 6.10).

The diagram title is never drawn (`architectureRenderer.ts:611`, "TODO: Add title support"). `title` only reaches the DB. The accessibility title and description are drawn (6.2).

### 6.2 The SVG, element by element [probed]

Probe: `probe.mjs` (E4) rendered the fixture `a:R -- L:b` and the titled fixture; the element tree below comes from those SVGs, with the source for each element in brackets. Ids are prefixed with the diagram id; `<id>` below stands for it.

```
svg#<id>  width="100%"  xmlns  style="max-width: Wpx;"  viewBox  role="graphics-document document"
          aria-roledescription="architecture"  [mermaidAPI.ts:755-757; accessibility.ts:22-30]
          aria-describedby="chart-desc-<id>" aria-labelledby="chart-title-<id>"   (only with accDescr/accTitle)
  title#chart-title-<id>  "Accessibility Title"          (only with accTitle)
  desc#chart-desc-<id>    "Accessibility Description"    (only with accDescr)
  style                 theme CSS + architecture rules  [styles.ts:34-38, 115; architectureStyles.ts]
  g                     empty; the render wrapper's root group   [mermaidAPI.ts:419]
  g.architecture-edges
    g
      path.edge#<id>-L_<src>_<tgt>_0   d="M sx,sy L mx,my L ex,ey"        [svgDraw.ts:94-97]
      polygon.arrow  points=...  transform="translate(x,y)"   (source, then target)  [svgDraw.ts:99-124]
      g                                                          (label, only with a title) [svgDraw.ts:143-159]
        g [transform translate(mx,my) or rotate(-90)]  rect.background, text > tspan.text-outer-tspan.row > tspan.text-inner-tspan
  g.architecture-services
    g#<id>-service-<sid>.architecture-service  transform="translate(x,y)"   [positionNodes; svgDraw.ts:352-354]
      g  transform="translate(40,80)"  (only with a title)               [svgDraw.ts:288-306]
        g > rect.background (style "stroke: none") + text[y=-10.1] > tspan.text-outer-tspan.row > tspan.text-inner-tspan
      g  (the icon, or the default background)
        g > svg[width=80, height=80, viewBox="0 0 80 80"] > g > (icon body)    [svgDraw.ts:309-317]
      or path.node-bkg#<id>-node-<sid>  d="M0,80 V5 Q0,0 5,0 H75 Q80,0 80,5 V80 Z"   (no icon, no iconText) [svgDraw.ts:341-351]
      or g > g > foreignObject > div.node-icon-text (iconText)              [svgDraw.ts:318-340]
    g.architecture-junction  transform="translate(x,y)"
      g > rect#<id>-node-<jid> fill-opacity="0" width=80 height=80         [svgDraw.ts:374-382]
  g.architecture-groups
    rect#<id>-group-<gid>.node-bkg  x y width height                       [svgDraw.ts:211-220]
    g (group icon, only with icon) > g > g [transform] > svg[width=30,height=30,...]   [svgDraw.ts:222-242]
    g (group label, only with title) > g [transform] > g > rect.background + text ...   [svgDraw.ts:244-268]
```

Two details the probe showed. First, the default `node-bkg` path of a service without an icon is the rounded square of `svgDraw.ts:348`. Second, a junction's `rect` is transparent (`fill-opacity="0"`); it exists so that edges have a box to attach to.

Text inside an SVG label follows `createFormattedText` (`createText.ts:152-200`): a `rect.background` with `style="stroke: none"` (no size, since `addBackground` is false), then `text[y=-10.1]` with one `tspan.text-outer-tspan.row` per line, each containing `tspan.text-inner-tspan` for the words.

### 6.3 Text measurement in the architecture renderer

Each row is a measurement and what it feeds. "Used" means the value changes the output.

| Measurement                                                                                                                        | Where                                                                                                                                                          | Feeds                                                                                                                                               | Used?            |
| ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| Width of each wrapped line: `getComputedTextLength()` on a test tspan (`createText.ts:114-125`, called by `checkWidth` at 170-172) | Service titles (wrap width `iconSize * 1.5` = 120, `svgDraw.ts:294`); group labels (width = group width, `svgDraw.ts:250`); edge labels (`svgDraw.ts:133-141`) | Which words go on which line, so the number of tspans and the label height                                                                          | Yes              |
| `getBBox()` of the text (`createText.ts:186-190`)                                                                                  | Only with `addBackground`, which architecture never sets                                                                                                       |                                                                                                                                                     | No               |
| `getBBox()` of each service group (`svgDraw.ts:356-358`)                                                                           | Sets `service.width` and `service.height`                                                                                                                      | Nothing: the layout uses `iconSize` (`architectureRenderer.ts:58-59`) and nothing else reads these fields                                           | No (dead)        |
| `getBBox()` of each junction group (`svgDraw.ts:384`)                                                                              | Sets `junction.width` and `junction.height`                                                                                                                    | Nothing                                                                                                                                             | No (dead)        |
| `getBBox()` of the root SVG (`setupGraphViewbox.js:50`)                                                                            | The size and viewBox (6.10)                                                                                                                                    | The whole frame                                                                                                                                     | Yes              |
| `getBoundingClientRect()` before and after rotation of an XY-edge label (`svgDraw.ts:168, 176`)                                    | The label's translation                                                                                                                                        | Position of the label on XY edges                                                                                                                   | Yes              |
| `getComputedStyle(div).font-size` (`svgDraw.ts:333-339`)                                                                           | The `-webkit-line-clamp` of an iconText                                                                                                                        | Only the clamp of an `iconText` (not used by any fixture)                                                                                           | Yes, unexercised |
| cytoscape `measureText` for every label (`labels.mjs:489-545`)                                                                     | Node and group labels inside the layout                                                                                                                        | Compound bounds (`bounds.mjs:66-84`), so group rects (`drawGroups` uses `node.boundingBox()`, `svgDraw.ts:211`) and the initial frame of the layout | Yes (6.5)        |

### 6.4 Edges [source, probed where marked]

`drawEdges` (`svgDraw.ts:25-191`):

- The start and end points are cytoscape's `sourceEndpoint()` and `targetEndpoint()` (50-52), and the middle is `midpoint()` (51). They are model coordinates: cytoscape computes edge geometry from `src.position()` (`edge-control-points.mjs:89-102`), not from the viewport. [probed: an edge `a:R -- L:b` starts at x = 0.4067 = the right edge of the drawn icon of `a`.]
- For edges that leave a group (`{group}`), the start moves out by `padding + 4` (44 px) horizontally, or by `44 + 18` for a bottom side (`svgDraw.ts:54-71`). The 18 is the service label height (the comment at line 56 says so).
- For edges that end at a junction without `{group}`, the point moves by half an icon, `40`, towards the junction centre (`svgDraw.ts:73-87`).
- Arrows are polygons (`ArchitectureDirectionArrow`, `architectureTypes.ts:36-41`), `arrowSize = iconSize / 6` (`svgDraw.ts:34`), shifted by `ArchitectureDirectionArrowShift` (`architectureTypes.ts:43-48`).
- The label is drawn at the middle. Its width is `|dx|` for an X edge, `|dy| / 1.5` for a Y edge, and `|dx| / 2` for an XY edge (`svgDraw.ts:133-141`). Y labels are rotated by -90 degrees; XY labels get a rotation of `-x*y*45` degrees and are moved using two `getBoundingClientRect()` calls (`svgDraw.ts:161-186`).

The cytoscape side of the endpoints [source]:

- `source-endpoint` and `target-endpoint` are data strings (`'0 50%'`, `'100% 50%'`, `'50% 0'`, `'50% 100%'`, `architectureRenderer.ts:126-145`). With `edge-distances: endpoints` (set by the stylesheet's `segments` rule) the endpoints are converted by `manualEndptToPx`: the node position plus the percentage times the outer size (`edge-endpoints.mjs:8-52`). The position is the node's centre in cytoscape, which is the drawn icon's top-left corner (6.6).
- `straight` edges (both directions X or both Y) and `segments` edges (XY) are the two curve styles (`architectureRenderer.ts:116-118`). For `segments`, the path is one bend point: `findSegmentsPoints` (`edge-control-points.mjs:119-160`) gives the bend from the segment weight (`w`) and distance (`d`). `findMidptPtsEtc` (13-62) picks the two points that the weight interpolates between.
- The weight and distance of each XY edge are computed in the `layoutstop` handler (`architectureRenderer.ts:514-579`) and written with `edge.style('segment-distances'|'segment-weights', ...)`. They are computed from the positions after the **first** run (see 6.7 and E3). [probed: changing that order changes three fixtures.]

### 6.5 The layout input [source]

`layoutArchitecture` (`architectureRenderer.ts:375-608`):

- The cytoscape instance is created on a `div#cy` that is appended to `body` and removed at once (384, 463). It has a canvas renderer because a container exists (`core/index.mjs:47-48, 84`).
- Groups, then services, then junctions, then edges are added with `cy.add` (465-468). Nodes carry `width` and `height` equal to `iconSize` (80); groups have a `parent` and a `label`. No node gets a `position`, so every simple node starts at **(0, 0)** and each compound at a position derived from its children, which include their labels (`element.mjs:38, 87-88`). [probed: `initpos.mjs` on a group with two services and a junction: the services and the junction at (0,0), the group at (0, 8.5), the bounding box of all elements (-82.5, -82.5) to (82.5, 99.5).]
- Edges have `source`, `target`, `sourceDir`, `targetDir`, the endpoint strings, `sourceGroup`, `targetGroup`, `sourceArrow` and `targetArrow`, and the class `segments` or `straight` (112-153).

fcose options (`architectureRenderer.ts:485-510`):

| Option                        | Value                                                                                                                | Effect (where it is read)                                                                                                                                                                                               |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `name`                        | `fcose`                                                                                                              |                                                                                                                                                                                                                         |
| `quality`                     | `proof`                                                                                                              | `LayoutConstants.QUALITY = 2`, slow cooling (FCOSE `cose.js:198-201`; `tick` in COSE2 `CoSELayout.js:185-199`)                                                                                                          |
| `randomize`                   | config, default `false`                                                                                              | Spectral start on or off (FCOSE `index.js:161-164`, `184-189`); also the cose start (FCOSE `cose.js:27-31`)                                                                                                             |
| `nodeSeparation`              | config, default 75                                                                                                   | **Read only by the spectral code** (FCOSE `spectral.js:38, 101`). With `randomize: false` it has no effect. [probed: `nodeSeparation: 200` leaves 19 of 20 fixtures unchanged; the 20th is the randomize-true fixture.] |
| `numIter`                     | config, default 2500                                                                                                 | `MAX_ITERATIONS` (FCOSE `cose.js:184-185`); `maxIterations = max(5n, numIter)` (LB2 `FDLayout.js:102-103`)                                                                                                              |
| `styleEnabled`, `animate`     | `false`                                                                                                              | `animate: false` makes `layoutPositions` write positions at once (CYTO `collection/layout.mjs:157-178`). `styleEnabled` is not a fcose option and is ignored.                                                           |
| `nodeDimensionsIncludeLabels` | `false`                                                                                                              | Node dimensions exclude labels (FCOSE `index.js:31`)                                                                                                                                                                    |
| `idealEdgeLength`             | function: 120 for an edge inside one group (`iconSize * idealEdgeLengthMultiplier`), 40 otherwise (`0.5 * iconSize`) | `architectureRenderer.ts:478-479, 496-501`                                                                                                                                                                              |
| `edgeElasticity`              | function: `edgeElasticity` (default 0.45) inside one group, 0.001 otherwise                                          | `architectureRenderer.ts:480, 502-507`. Probed: 0.6 changes 17 of 20 fixtures.                                                                                                                                          |
| `alignmentConstraint`         | `{horizontal, vertical}`                                                                                             | Section 6.6                                                                                                                                                                                                             |
| `relativePlacementConstraint` | list                                                                                                                 | Section 6.6                                                                                                                                                                                                             |

Other fcose defaults are not overridden by mermaid. `padding` (30) only sets the viewport fit (`fit` is true, `index.js:27-28`); the viewport does not change model coordinates. `nodeRepulsion` (4500), `gravity` (0.25), `gravityRange` (3.8), `gravityCompound` (1.0), `gravityRangeCompound` (1.5), `initialEnergyOnIncremental` (0.3), `nestingFactor` (0.1), `tilingPaddingVertical/Horizontal` (10), `sampleSize` (25), `samplingType` (greedy), `piTol` (1e-7) are fcose's own defaults (FCOSE `index.js:10-97`).

### 6.6 Constraints [source]

`getAlignments` (`architectureRenderer.ts:155-289`) turns the spatial maps into alignment lists:

- For each spatial map, nodes are grouped by row (y) and column (x), and within a row or column by their group (`in`, or `'default'`), lines 213-237.
- `flattenAlignments` (172-211) merges the groups of one row or column into one list when their groups are aligned (`groupAlignments`) or when one of them is `default`. Otherwise it splits them into separate lists with keys `dir-0`, `dir-1` and so on (lines 197-206). The result drops lists with one member (240-247).
- Lists that contain a member of a user `align` hint are dropped (264-270). The hints are appended as alignments for `row` (horizontal) or `column` (vertical) (272-283).

`getRelativeConstraints` (`architectureRenderer.ts:291-373`) emits one relative constraint per adjacent pair of the BFS grid:

- Each user hint `align row a b c` gives `left: a, right: b, gap: hintGap` and `left: b, right: c` (308-320); `hintGap = idealEdgeLengthMultiplier * iconSize` (303).
- The grid BFS starts at `[0, 0]` for each spatial map and visits the directions in the order `L, R, T, B` (`directions`, 332-337). For each grid neighbour not already declared by a hint, it emits `left`/`right` or `top`/`bottom` with `gap = idealEdgeLengthMultiplier * iconSize` (364-365). The BFS queue and the `visited` record are the same as in section 3.5, so duplicates are possible; the port must keep them.

How fcose uses the constraints [source, outline only]: `ConstraintHandler.handleConstraints` (COSE2 `ConstraintHandler.js:9-847`) builds a directed graph from the relative constraints per component, places the nodes from it (lines 70-281), and, when the transform flag is set, rotates or reflects the draft by an orthogonal Procrustes fit (lines 384-437, behind the flag at line 438). Enforcement and the final coordinates follow (lines 611 and 839). The flag `TRANSFORM_ON_CONSTRAINT_HANDLING` is set to `randomize` (FCOSE `cose.js:233-237`), so the Procrustes fit runs only with `randomize: true`. The port task must read the 847 lines in full.

### 6.7 The two layout runs [source and probed]

`layoutArchitecture` calls `layout.run()` twice:

1. Inside `withSeededRandom`, in the `try` (`architectureRenderer.ts:582-601`). fcose lays out the graph and calls `layoutPositions`, which writes the positions and emits `layoutstart`, `layoutready` and `layoutstop` at once (CYTO `collection/layout.mjs:157-177`). Those events run the handler registered with `layout.one('layoutstop', ...)` (line 513), **inside the first call**.
2. The handler (513-581) computes the edge weights and distances from the positions of the first run (514-579), then calls `withSeededRandom(seed, () => layout.run())` (580). That is the second run. It starts from the first run's positions. Its `layoutstop` finds no handler (the `one` registration is spent), so it ends there.

After the second run, the first run's own `parentsWithoutChildren` loop continues. It writes the positions of childless compound nodes from the first run's results (FCOSE `index.js:401-405`), which overwrite the second run's values for those nodes.

Probed effects [probed: `variant.mjs` and `variant3.mjs` compare the reference with a copy where the second run is removed; `order.mjs` and `order2.mjs` compare with a copy where the weights are computed after the second run]:

- Removing the second run moves 18 of 20 fixtures by one translation: dx and dy are constant across the nodes (dy 7.5-11 px; dx 0, 0.75, -0.75 or 11 px; two fixtures have a spread of 0.025 px). Two fixtures are not translations: the complex nested diagram (`should-render-a-deterministic-layout-for-a-complex-deeply-nested-diagram`, node shifts from -19 to 132 px in x and -213 to 99 px in y) and `groups-within-groups` (x shifts from -3.9 to 0.02 px).
- Computing the weights after the second run changes the path coordinates of three fixtures (`directional-arrows`, `edge-labels`, `split-directioning`) by up to 137 px. The mechanism is unverified; the sign tests in `getSegmentWeights` (lines 533-553) are the likely place, because a tiny change in a position can flip them.

### 6.8 Services and junctions [source]

- Each service is placed at its cytoscape centre, and the drawn icon's top-left corner is at that centre. The icon is `80 x 80` and the cytoscape node is `80 x 80` centred there, so the visible box lies to the lower right of the layout node. This is intentional: the endpoints (`'100% 50%'` is the right side of the node box from its top-left) and the group rectangles (`x1 + halfIconSize`, `svgDraw.ts:216`) compensate for it. [probed: the `R` endpoint of `a` is at x = 0.4067, the right edge of the drawn icon.]
- A service's title is wrapped at 120 px and placed at `translate(40, 80)` (`svgDraw.ts:294, 306`). Its text is `text-anchor: middle`.
- Junctions have no title and no icon. They are 80 x 80 transparent boxes.

### 6.9 Icons [source, probed for the output]

Icon names resolve through `getIconSVG` (`rendering-util/icons.ts:88-106`) and the registry (29-77). Architecture registers one pack, `mermaid-architecture`, with `architectureIcons` (`architectureRenderer.ts:40-45`). An icon name without a prefix falls back to that pack (`fallbackPrefix`, `svgDraw.ts:316`).

The seven built-in icons (`architectureIcons.ts:13-41`), each a body wrapped in an 80 x 80 blue rectangle (`wrapIcon`, lines 4-6, fill `#087ebf`):

| Name       | Used by                                                                       | Source lines |
| ---------- | ----------------------------------------------------------------------------- | ------------ |
| `database` | services in fixtures                                                          | 14-16        |
| `server`   | services                                                                      | 19-21        |
| `disk`     | services                                                                      | 24-26        |
| `internet` | services                                                                      | 29-31        |
| `cloud`    | groups in fixtures                                                            | 34-36        |
| `unknown`  | fallback (also `unknownIcon` in `rendering-util/icons.ts:20-24`, a "?" glyph) | 38           |
| `blank`    | the background of an `iconText` service                                       | 40           |

Fallback [source]: an unknown name, or a name with a prefix that is not registered, makes `getRegisteredIconData` throw, `getIconSVG` logs the error and uses `unknownIcon` (`icons.ts:94-99`). Fixtures: `iconnamedoesntexist` and `s3` take this path. [probed: both fixtures render; the glyph itself was not inspected.]

What the output looks like [probed]: `<svg width="80" height="80" viewBox="0 0 80 80">` with a `g` holding the rectangle and the body, and element ids replaced by `IconifyId<hex>` (`@iconify/utils`, `replaceIDs`).

What ferrigura would have to bundle: the seven icon bodies (about 5 KB of SVG path data in `architectureIcons.ts`) and the `unknownIcon` body (`icons.ts:21`). No icon set is needed beyond these: no fixture uses another pack, and user packs are the user's own registrations. Decision 0001 (no network) is met. What the port must reproduce: `iconToSVG` for an 80 x 80 icon at 30 x 30 for groups (`svgDraw.ts:228`), and `replaceIDs`, whose ids are random (section E4 and 6.12).

### 6.10 Root sizing [source and probed]

`setupGraphViewbox` (`setupGraphViewbox.js:49-70`): the bounding box of the root SVG is read with `getBBox()`. With `useMaxWidth` (default `true`, `config.schema.yaml:457-470` through the base config), the root gets `width="100%"` and `style="max-width: W px;"`, where `W = bbox.w + 2 * padding`. The viewBox is `(bbox.x - padding, bbox.y - padding, bbox.w + 2 padding, bbox.h + 2 padding)`. The padding is the architecture `padding` (40 by default). [probed: the simple diagram `a:R -- L:b` gives `viewBox="-119.59 -22 360.69 181.47"` and `max-width: 360.69px`.]

`getBBox()` on the root is the union of the boxes of all children: the edges, services, junctions and groups, including text. It excludes `title`, `desc` and `style`. Stroke width is not included. The empty `g` has no box.

### 6.11 Styles [source]

`architectureStyles.ts:4-37` gives the architecture rules: `.edge` (stroke width `archEdgeWidth`, stroke `archEdgeColor`, no fill), `.arrow` (fill `archEdgeArrowColor`), `.node-bkg` (no fill, stroke `archGroupBorderColor`, width `archGroupBorderWidth`, dash array 8), `.node-icon-text` (flex, centred) and `.node-icon-text > div` (white text, 1 px margin, clamped with `-webkit-box`).

Theme values for `redux-color` (the one ferrigura ships with `redux-dark-color`): `archEdgeColor` and `archEdgeArrowColor` are `lineColor` (`themes/theme-redux-color.js:44-45, 193-194`); `archEdgeWidth` is `'3'` (46); `archGroupBorderColor` is `primaryBorderColor` (47); `archGroupBorderWidth` is `'2px'` (48). The probe output matches: `.edge{stroke-width:3;stroke:#000000;fill:none}` and `.node-bkg{stroke:hsl(247.5, 0%, 9.2156862745%);stroke-width:2px;stroke-dasharray:8}`.

### 6.12 Randomness, clock and page state [source and probed]

- The layout's only random source is `Math.random`, replaced for the duration of each `layout.run()` by `withSeededRandom(seed, fn)` (`architectureSeed.ts:13-31`). The generator is mulberry32 with state `seed` (lines 19-25). Each call starts from the same state. Nothing outside the call sees the replacement; `Math.random` is restored in a `finally` (lines 26-30).
- The seed is `config.architecture.seed`, default 1 (`config.schema.yaml:1177-1186`). Two runs per render means two replays of the same stream (section E4).
- `@iconify/utils` draws ids with `Date.now()` and `Math.random()`: `randomPrefix` once at load (`lib/svg/id.js:23`) and a suffix per id (line 36). These draws happen outside `withSeededRandom`, so they do not affect the layout. They do change the id text of every icon from render to render.
- No architecture file uses `Date` or the clock (grep: none).
- The geometry harness installs its own seeded `Math.random` and a fixed `Date` during `renderReference` (`tools/port/geometry/common.ts:441-472`). The architecture wrapper overrides the harness's generator during the layout, so the harness's seed has no effect on architecture layouts. It only affects the icon ids.
- Page state [probed: `probe.mjs` E1]: 20 fixtures rendered on fresh pages, on one shared page in order, and on the same page in reverse order give identical SVG once the icon ids are masked. So the reference's architecture output does not depend on earlier renders on the page. P2-PLAN's statement that the force-directed layouts "draw from the page's generator" is therefore wrong for this type (section 8, Q3).

### 6.13 DOM dependencies a server renderer must replace [source]

| Dependency                                                      | Where                                                           | Replacement                                                                                                                                                                                                                                                                   |
| --------------------------------------------------------------- | --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `document.getElementById('cy')` and a cytoscape canvas renderer | `architectureRenderer.ts:384-386`                               | The geometry is in model coordinates, so no view is needed. A headless cytoscape would skip label boxes (CYTO `bounds.mjs:265-268` returns early when headless), which changes the compound bounds [source; not run]. The label measurement is the real dependency (6.5, Q1). |
| `select('body')`, `selectSvgElement`                            | `architectureRenderer.ts:384`, `selectSvgElement.ts`            | The SVG is written to a string; no DOM.                                                                                                                                                                                                                                       |
| `getBBox()` of the root and of text                             | `setupGraphViewbox.js:50` and `createText.ts:186` (unused here) | Glyph metrics for the wrapped text (M5 rules), union of boxes.                                                                                                                                                                                                                |
| `getBoundingClientRect()` for XY labels                         | `svgDraw.ts:168, 176`                                           | The bounding box of rotated text in user units. It equals the screen box only when the SVG is drawn at scale 1.                                                                                                                                                               |
| `getComputedTextLength()`                                       | `createText.ts:122`                                             | The M5 text advance rules.                                                                                                                                                                                                                                                    |
| `getComputedStyle()` for iconText                               | `svgDraw.ts:333-339`                                            | Not used by any fixture.                                                                                                                                                                                                                                                      |
| `measureText()` for labels in the layout                        | `labels.mjs:489-545`                                            | See Q1: needs Helvetica Neue advances.                                                                                                                                                                                                                                        |

---

## 7. Fixtures

Twenty fixtures: 8 in `nightly-check`, 10 in `pr-check`, 2 in `visual-check` (the folder names are the upstream test tiers; the geometry harness compares all of them). The harness does not yet know the type: the list in `tools/port/geometry/common.ts` and `diagram-geometry.ts` has no `architecture`.

| Fixture (after the `should-render-` prefix)                                       | Features                                                                                                        |
| --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| nightly: `align-siblings-in-a-row-when-their-edges-feed-a-common-downstream-node` | Four servers; two `align row` hints; edges into a common node; no groups                                        |
| nightly: `a-grid-via-combined-row-column-alignments`                              | A cloud group with databases and servers; `align row` and `align column` (six hints)                            |
| nightly: `a-simple-architecture-diagram-with-titleandaccessibilities`             | `title`, `accTitle`, `accDescr`; a group with a database, two disks and a server; edges                         |
| nightly: `an-architecture-diagram-with-directional-arrows`                        | Five servers; the arrow forms `-->` and `<--`                                                                   |
| nightly: `an-architecture-diagram-with-simple-junction-edges`                     | Two junctions; disks and internet; a `{group}` modifier; edges through the junctions                            |
| nightly: `an-architecture-diagram-with-the-fallback-icon`                         | One service with an unknown icon name                                                                           |
| nightly: `edges-at-correct-length`                                                | Three services, one without an icon (the default background path), edges R to L and B to T                      |
| nightly: `stack-three-same-port-databases-in-a-column-without-overlap`            | Three databases on the same port; `align` hints; a cloud group                                                  |
| pr: `a-deterministic-layout-for-a-complex-deeply-nested-diagram`                  | Eight groups nested up to four deep; `randomize: false` set in front matter; 12 services (the spec counts them) |
| pr: `a-deterministic-layout-with-an-explicit-seed-override`                       | `seed: 42` in front matter; three groups, one nested; three services                                            |
| pr: `a-simple-architecture-diagram-with-groups`                                   | A group; four services and an internet node; group edges                                                        |
| pr: `an-architecture-diagram-with-a-reasonable-height`                            | Two groups; six databases, four servers and eight junctions; 16 edges                                           |
| pr: `an-architecture-diagram-with-complex-junction-edges`                         | Junctions and `{group}` edges across three groups                                                               |
| pr: `an-architecture-diagram-with-edge-labels`                                    | Five servers in a star; edge titles `-[Label]-`; XY edges (the rotated label path)                              |
| pr: `an-architecture-diagram-with-group-edges`                                    | Five clouds, five disks; six `{group}` modifiers                                                                |
| pr: `an-architecture-diagram-with-groups-within-groups`                           | Nested groups; a database, an internet node, two servers                                                        |
| pr: `an-architecture-diagram-with-split-directioning`                             | A database, two disks, an unknown icon `s3`, two servers; mixed directions                                      |
| pr: `with-randomize-true-without-errors`                                          | The complex nested diagram with `randomize: true` (the spectral path)                                           |
| visual: `a-linear-chain-with-an-increased-idealedgelengthmultiplier`              | Three servers; `idealEdgeLengthMultiplier: 3`; the description contains an arrow `→`                            |
| visual: `a-linear-chain-with-default-fcose-knobs`                                 | Three servers; the description contains `→`                                                                     |

Handling for the harness:

- **Exclusion by the harness rule.** `uncoveredScriptReason` (`common.ts:189-194`) tests the whole fixture text, front matter included, against the arrow block `U+2190-U+21FF`. The two visual-check fixtures contain `→` only in their descriptions, so the rule excludes them although the diagram is plain ASCII. The harness should test the diagram body after the front matter, or the exclusion must be named as an artefact. Decision for the coordinator (Q5).
- **Randomness.** Both `randomize` fixtures, and the default-config fixtures with relative constraints, need the seeded draws (E4). Nothing is excluded for randomness; the harness's own generator does not reach the layout (6.12).
- **Font.** Two fixtures change under Lexica measurement (section 0, item 3). They will fail the 0.05 px tolerance unless Helvetica Neue metrics are reproduced.
- **Icon ids.** `IconifyId` ids differ between renders. The harness compares structural keys, not ids, so this is harmless for the comparison, but the id text should be masked in any text diff.
- **Forced settings.** The harness forces `fontFamily` for mermaid text. It does not reach the cytoscape label measurement (`labels.mjs:530` uses the style's font family).

Probed outputs for the fixtures: all 20 render in Chromium in 15-124 ms each, with no errors, on the dist bundle (`probe.mjs` E1).

---

## 8. Risks, effort and open questions

### 8.1 Risks

| #   | Risk                                                                                                                                              | Size                    | Mitigation                                                                                                                                          |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Layout numerics: the force phase, the grid, the constraint handler and the draw order must match exactly; discrete tests amplify tiny differences | Largest                 | Port the fcose engine first with a differential on captured inputs (E7); keep the operation order; use the same libm functions for the cooling (E6) |
| R2  | Two-run pipeline and the ordering of the weights (6.7)                                                                                            | Medium                  | Port both runs and the handler order exactly; test with the three fixtures that flip                                                                |
| R3  | Helvetica Neue metrics in the layout's label bounds                                                                                               | Medium, a decision (Q1) | See Q1                                                                                                                                              |
| R4  | DOM-measured geometry (viewBox, XY labels, wrapping)                                                                                              | Medium                  | Reuse the M5 text rules; assume scale 1 for XY labels                                                                                               |
| R5  | Parser quirks (2.3)                                                                                                                               | Low to medium           | Probed table as the test list                                                                                                                       |
| R6  | cytoscape's collection order, `Object.keys` order and `Set` order in the force code                                                               | Medium, unverified      | Confirm by capture (E7); keep insertion order everywhere                                                                                            |
| R7  | Tiling code reached when there are no constraints (section E3), including for single nodes                                                        | Medium, unverified      | Trace the `fallback-icon` fixture; port tiling if reached                                                                                           |

### 8.2 Effort (rough)

Parser and DB: about one week. Renderer and SVG: about two weeks, with M5's text rules available. The layout (fcose, cose-base 2.2.0, layout-base 2.0.1, plus the constraint handler and the spectral path if `randomize: true` is kept): about three to four weeks, of which about half is shared with the cose-bilkent port. The cytoscape subset: about one week. The total is roughly six to eight weeks for one person, not counting the Helvetica decision.

### 8.3 Open questions for the coordinator

- **Q1. Label measurement in the layout.** cytoscape measures labels with `Math.ceil(measureText(...).width)` in `Helvetica Neue, Helvetica, sans-serif` at `fontSize` px (`labels.mjs:503-533`; default family at `style/properties.mjs:641`). The harness forces `Lexica Ultralegible`. Options: (a) bundle a width table for Helvetica Neue and Helvetica (the licence needs checking, and the table is per platform); (b) measure the labels with the bundled face and accept the 2 fixtures that move (`reasonable-height` by 4 px in one group, `edge-labels` by 0.25 px); (c) exclude the two fixtures with a named reason, as M5 did for scripts. Recommendation: (c) for now, and a decision before the geometry work starts.
- **Q2. "The same cose-base".** The P2-PLAN table says fcose is "ported after cose-bilkent, on the same cose-base". The versions differ (section 1.2): fcose uses cose-base 2.2.0 and layout-base 2.0.1; cose-bilkent uses 1.0.3 and 1.0.2. Their force phases differ (E9). The plan should say: one shared graph model and grid code, two parameter sets (or two modules).
- **Q3. Random draws.** P2-PLAN's "Random draws" row and decision 0008 say the reference's generator state lives for the page and that a diagram on a fresh page is the reference. For architecture neither holds: the draws are seeded per run by `withSeededRandom` (probed: identical output on shared and fresh pages), and `layout-base`'s `RandomSeed` is not reached by fcose (E4). The plan should say that for architecture the seed is `config.architecture.seed` (default 1), reset for each run. The cose-bilkent task should check its own draw sites.
- **Q4. Spectral path.** Keep the `randomize: true` path (the fixture uses it, and users may too)? It needs the power iteration (FCOSE `spectral.js:197-288`; the whole file is 427 lines), `SVD` and `Matrix` from LB2 (639 and 164 lines), and the Procrustes fit in the constraint handler. My recommendation: keep it, port after the default path.
- **Q5. Harness rules.** The arrow exclusion catches the two visual fixtures through their descriptions (section 7). Should the exclusion look only at the diagram body?
- **Q6. Tiling.** Is the tiling code (COSE2 `CoSELayout.js:1069-1447` and `1798-1819`) reached for the fixtures? It runs whenever the tile flag is set, which is whenever there are no constraints (FCOSE `index.js:136-142`; `CoSEConstants.TILE`). Probe needed: trace `groupZeroDegreeMembers` for `fallback-icon`. Until then the port has to include it.
- **Q7. Failing spec.** `svgDraw.spec.ts` fails in this checkout because cytoscape's canvas renderer has no 2D context in jsdom (section 5). Is that a known problem with the `canvas` package on this machine? It does not block the port, which will not use cytoscape.
- **Q8. Second run.** Confirm that the port should reproduce the second run and the weight order exactly, rather than a single run with a correction. Probed: a single run changes 18 fixtures by translation alone, so a translation correction would fit most but not the complex nested one.

---

## E. fcose as a layout engine

Structure follows the layout-engine template of `P2-RESEARCH.md`, with the sections of that template in order.

### E0. Summary

fcose (`cytoscape-fcose` 2.2.0) is a force-directed layout for cytoscape. It has three phases: an optional spectral start (only with `randomize: true`), a constraint phase for alignment and relative placement, and a spring embedder from cose-base (CoSE, with grid repulsion, gravity and cooling). Mermaid uses it with `quality: 'proof'`, constraints, idealised edge lengths and elasticity per edge, and `randomize` false by default. The output is the centre of each node's rectangle, moved so that the layout's bounding box has the same centre as the bounding box of the elements before the layout.

What the port has to replicate: the default path (constraints, spring embedder, grid repulsion, gravity, cooling, convergence, the draw order of the constraint shuffle), the relocation, the two runs of the renderer, and the spectral path if `randomize: true` is kept (Q4).

### E1. Sources

| File                                                       | Lines         | Role                                                                                                                                                                                                                                                                                                                                                                |
| ---------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| FCOSE `index.js`                                           | 1-414         | Defaults (10-97), `run` (104-411); packing (174-331) is not reached                                                                                                                                                                                                                                                                                                 |
| FCOSE `cose.js`                                            | 1-261         | Builds cose-base nodes and edges, sets the global constants, runs cose (15-172, 180-258)                                                                                                                                                                                                                                                                            |
| FCOSE `spectral.js`                                        | 1-427         | Spectral start: BFS sampling (46-166), SVD regularisation (169-194), power iteration (197-288), main (290-426)                                                                                                                                                                                                                                                      |
| FCOSE `auxiliary.js`                                       | 1-268         | `getTopMostNodes` (10), `connectComponents` (33), `relocateComponent` (136-198), `calcBoundingBox` (200), `calcParentsWithoutChildren` (252)                                                                                                                                                                                                                        |
| COSE2 `CoSELayout.js`                                      | 1-2037        | Constructor (20-25), `initParameters` (51-83), `initSpringEmbedder` (84-93), `classicLayout` (106-161), `tick` (163-260), `runSpringEmbedder` (278-296), `moveNodes` (298-322), `initConstraintVariables` (324-587, the shuffle at 419-430), `updateDisplacements` (588-800, the shuffle use at 635-642), tiling (1069-1447, 1798-1819), tree reduction (1820-2030) |
| COSE2 `ConstraintHandler.js`                               | 1-847         | `handleConstraints` (9-847): gaps (37-49), graph and positions (70-281), Procrustes (384-437, flag at 438), enforcement (611), assignment (839)                                                                                                                                                                                                                     |
| COSE2 `CoSENode.js`                                        | 1-127         | `calculateDisplacement` (13-49), `move` (70-90)                                                                                                                                                                                                                                                                                                                     |
| COSE2 `CoSEConstants.js`                                   | 1-26          | Defaults (11-24)                                                                                                                                                                                                                                                                                                                                                    |
| COSE2 `CoSEGraph.js`, `CoSEGraphManager.js`, `CoSEEdge.js` | 12 lines each | Identical to COSE1                                                                                                                                                                                                                                                                                                                                                  |
| LB2 `Layout.js`                                            | 1-672         | `checkLayoutSuccess` (91), `runLayout` (97), `doPostLayout` (143), `initParameters` (258), `positionNodesRandomly` (309)                                                                                                                                                                                                                                            |
| LB2 `LayoutConstants.js`                                   | 1-~20         | Defaults: `DEFAULT_INCREMENTAL = false` (line 13), `QUALITY = 1` (line 7); fcose overrides both                                                                                                                                                                                                                                                                     |
| LB2 `fd/FDLayout.js`                                       | 1-529         | The force phase: setup (3-38), ideal lengths (40-78), spring embedder setup (80-111), springs (112-123, 191-233), repulsion driver (124-168), gravity (169-179, 316-356), move (180-189), repulsion force (235-314), convergence (358-373), grid (407-468), grid repulsion (469-523), range (525-527)                                                               |
| LB2 `fd/FDLayoutConstants.js`                              | 1-34          | Defaults (11-32)                                                                                                                                                                                                                                                                                                                                                    |
| LB2 `LNode.js`                                             | —             | `scatter` (279-295) uses `RandomSeed`; never called by fcose (E4)                                                                                                                                                                                                                                                                                                   |
| LB2 `util/RandomSeed.js`                                   | 1-12          | The page-wide generator `sin(seed++) * 10000`                                                                                                                                                                                                                                                                                                                       |
| LB2 `util/IGeometry.js`                                    | 1-621         | `calcSeparationAmount` (21), `getIntersection2` (168), `getIntersection` (456)                                                                                                                                                                                                                                                                                      |
| LB2 `util/IMath.js`                                        | 1-30          | `sign` (7-15): 1, -1, or 0                                                                                                                                                                                                                                                                                                                                          |
| LB2 `util/SVD.js`, `util/Matrix.js`                        | 639, 164      | Spectral start and Procrustes                                                                                                                                                                                                                                                                                                                                       |
| CYTO files                                                 | see section 1 | `layoutPositions`, label bounds, compound bounds, edges                                                                                                                                                                                                                                                                                                             |

### E2. Input

What Mermaid builds (`architectureRenderer.ts:465-510`, section 6.5): a cytoscape instance with elements in this order: groups, services, junctions, edges. Simple nodes are `80 x 80` (`width`, `height` data), compounds have `parent` links, every node starts at (0, 0) (compounds at the centre of their children). Edges carry `source` and `target`, and the layout asks for an ideal length and an elasticity per edge. The constraint arrays come from `getAlignments` and `getRelativeConstraints` (6.6). The options are in the table of section 6.5.

In fcose itself, `run` (`index.js:104`) first sanitises the constraints (118-134), turns off tiling and packing if any constraint exists (136-142), and checks for `cy.layoutUtilities` (147-152). Mermaid does not register that extension (grep: nothing in `packages/mermaid/src`), so packing is off. The whole graph is laid out as one component (154-173, 399).

Inside cose-base, `processChildrenList` (FCOSE `cose.js:48-130`) creates one `CoSENode` per cytoscape node. For a simple node the rectangle is centred on the node's position (`position - dims/2`, `cose.js:85-87` for the non-random case) with the dims from `layoutDimensions`, which are the outer width and height (`CYTO collection/layout.mjs:26-29`). Compound nodes take their dims from cytoscape, which include labels (section 6.3). `processEdges` (`cose.js:132-161`) skips self-loops and duplicate pairs (`getEdgesBetween`, line 139), and sets each edge's ideal length and elasticity from the functions. It also sets the global `DEFAULT_EDGE_LENGTH` to the **mean ideal length over the edges** (line 152), and derives `MIN_REPULSION_DIST` and `DEFAULT_RADIAL_SEPARATION` from it (158-159).

Consequence: the global edge length depends on the whole diagram: 120 for edges inside one group, 40 for others, averaged. Repulsion range, the displacement threshold (`3 * DEFAULT_EDGE_LENGTH / 100` per node, LB2 `FDLayout.js:15, 106`), the minimum repulsion distance and the layout's `idealEdgeLength` all come from that mean. A port must compute it per run.

### E3. Algorithm

The phases, with the reference's order and constants.

**Setup (fcose).** `run` sets `tile = false` and `packComponents = false` when a constraint exists (`index.js:136-142`). It computes `componentCenters` from `options.eles.boundingBox()` (157-159): the centre of the bounding box of all elements, labels and edges included (`CYTO collection/dimensions/bounds.mjs:886`, the default of `includeLabels` is true). This centre is the target of the relocation at the end.

**Spectral start (only if `randomize`).** `spectralLayout` (`spectral.js:10-426`): connects components with dummy nodes (`connectComponents`, 293-297), numbers the nodes, builds the neighbourhood lists (339-369), then, for graphs of more than two nodes, samples `sampleSize` (25) pivots greedily by BFS distance (`allBFS`, 122-166), forms the distance matrix squared (150-155) and the matrix PHI (156-165), regularises the SVD of PHI (169-194), and runs the power iteration for the two largest eigenvectors (197-288). The result is the x and y coordinates `Matrix.multCons(V, sqrt(|theta|))`. Graphs of 1 or 2 nodes skip this (409-425). Tolerance `piTol` = 1e-7 (`spectral.js:36`).

The numerics, for a port: this is a pivot-based spectral embedding (in the style of Pivot MDS), not a full eigen-decomposition. The operator is the double-centred `-1/2 J C INV C^T J`, with `J` the centring of `Matrix.multGamma` (LB2 `util/Matrix.js:110-127`) and `multL` (lines 131-150). `C` holds the BFS distances to the pivots times `nodeSeparation` (`spectral.js:101`), squared (`150-155`); this is the only place where `nodeSeparation` is read; `PHI` is the pivot-by-pivot block (156-165); `INV` is the pseudo-inverse of `PHI` from an SVD (`SVD.svd`, LB2 `util/SVD.js:216`), with the regularisation `s / (s^2 + max_s / s^2)` where `max_s` is the cube of the largest singular value (`spectral.js:177-190`). The two leading eigenvectors come from power iteration: the first vector is iterated until the ratio of successive projections is in `[1, 1 + piTol]` (`spectral.js:224-244`); the second vector is first made orthogonal to the first (line 259) and iterated the same way (252-273). The eigenvalues are the Rayleigh-type dot products (`theta1`, `theta2`, lines 232 and 261). Every floating-point operation in that order is part of the output, and the random start vectors are the draws of E4.

**Layout setup (cose-base).** `coseLayout.runLayout()` (LB2 `Layout.js:97-127`): `initParameters` (CoSE `CoSELayout.js:51-83`, LB2 `Layout.js:258-270`): `layoutQuality = LayoutConstants.QUALITY` (2 for proof), `incremental = LayoutConstants.DEFAULT_INCREMENTAL`, `uniformLeafNodeSizes`; the CoSE part sets `idealEdgeLength` from the global edge length (with a floor of 10), the gravity constants, and the tree-growth counters. Then the optional `tilingPreLayout` (CoSE 1798-1808; runs only if `CoSEConstants.TILE`, which fcose sets to `options.tile`), and `checkLayoutSuccess` (LB2 `Layout.js:91-95`: no root, no nodes, or an invalid edge stops the layout).

**`classicLayout` (CoSE `CoSELayout.js:106-161`).** Computes gravity eligibility (`calculateNodesToApplyGravitationTo`, 802-823), the nesting and LCAs, `calcIdealEdgeLengths` (LB2 `FDLayout.js:40-78`, only for inter-graph edges, which Mermaid does not produce), and the estimated sizes. Then, because `incremental` is always true for fcose (`cose.js:213` sets `LayoutConstants.DEFAULT_INCREMENTAL = true` after the earlier assignments at 204-205), the branch at line 138 runs: `reduceTrees` when `TREE_REDUCTION_ON_INCREMENTAL` is true (FCOSE `cose.js:242-247` sets it to false when constraints exist, so this runs only for unconstrained graphs). Constraints are handled at 150-153 (ConstraintHandler, section 6.6). Then `initSpringEmbedder` (155) and `runSpringEmbedder` (156-158).

**`initSpringEmbedder` (CoSE 84-93, LB2 80-111).** Incremental branch: the cooling factor is 0.3 (`initialEnergyOnIncremental`), reduced only above 1000 nodes; `maxNodeDisplacement = 100` (`MAX_NODE_DISPLACEMENT_INCREMENTAL`); `maxIterations = max(5n, numIter)`; `displacementThresholdPerNode = 3 * DEFAULT_EDGE_LENGTH / 100`; `totalDisplacementThreshold = threshold * n`; `repulsionRange = calcRepulsionRange()`, which in CoSE is `2 * (level + 1) * idealEdgeLength` = `2 * idealEdgeLength` (`CoSELayout.js:1061-1064`). CoSE then sets `maxCoolingCycle = maxIterations / 100`, `finalTemperature = 0.04`, `coolingAdjuster = 1` (CoSE 89-91).

**`runSpringEmbedder` (CoSE 278-296).** Loops `tick()` until it returns true. The `animate` option is false, so there is no animation.

**`tick` (CoSE 163-260).** One iteration:

1. `totalIterations++`. If it reaches `maxIterations` and no tree is growing, stop (or start growing when there are pruned nodes; 166-172).
2. Every 100 iterations (`CONVERGENCE_CHECK_PERIOD`, LB2 `FDLayoutConstants.js:29`), not while growing: test `isConverged()` (LB2 `FDLayout.js:358-373`: `totalDisplacement < totalDisplacementThreshold`, or, after `maxIterations / 3`, `|totalDisplacement - old| < 2`); if converged, stop (or grow the pruned nodes); then update the cooling. With `layoutQuality == 2` (proof) the adjuster stays 1, so `coolingFactor = max(initial - cycle^(log(100*(initial - final)) / log(maxCoolingCycle)) / 100 * 1, final)` (line 199), with `initial = 0.3` and `final = 0.04`. `animationPeriod` is computed and unused. The growth branches (201-257) run only for pruned nodes, which exist only for unconstrained graphs.
3. `updateBounds()` on the graph manager (compound rectangles from their children, including their margins).
4. `calcSpringForces` (LB2 `FDLayout.js:112-123`): for each edge, `springForce = edgeElasticity * (length - idealLength)`, applied along the edge (`191-233`). Edges of zero length are skipped (LB2 `FDLayout.js:218`).
5. `calcRepulsionForces(gridUpdateAllowed, forceToNodeSurroundingUpdate)` (124-168): the grid variant (`useFRGridVariant`, set from `DEFAULT_USE_SMART_REPULSION_RANGE_CALCULATION`, true). The grid is rebuilt every 10 iterations (`totalIterations % 10 == 1`, lines 130-134) with cells of size `repulsionRange` (`calcGrid` 407-428, `addNodeToGrid` 430-451, `updateGrid` 453-467). For each node, the list of "surrounding" nodes is recomputed on the same schedule (469-523, the set in insertion order, `[...surrounding]` at 519). Each pair in a surrounding list gets `calcRepulsionForce` (235-314):
   - if the rectangles overlap, a separation amount from `calcSeparationAmount(rectA, rectB, overlap, DEFAULT_EDGE_LENGTH / 2.0)` (`FDLayout.js:251-254`), doubled, and distributed by products of `noOfChildren`;
   - otherwise the distance between the clip points (`IGeometry.getIntersection`), floored at `MIN_REPULSION_DIST` on each axis (`FDLayoutConstants.MIN_REPULSION_DIST = DEFAULT_EDGE_LENGTH / 10`, line 28 of the constants file, reassigned in fcose `cose.js:158`), and a force `(nodeRepulsion_A / 2 + nodeRepulsion_B / 2) * childrenA * childrenB / d^2` (line 300-302). fcose's `nodeRepulsion` is 4500 by default.
6. `calcGravitationalForces` (169-179): for each node to which gravity applies, the force towards the owner's centre when the node is outside `estimatedSize * gravityRangeFactor` (3.8 for the root, 1.5 for compounds), scaled by `gravityConstant` (0.25 from fcose; `cose.js:182-183`) and, in a compound, by `compoundGravityConstant` (1.0) (316-356).
7. `moveNodes` (CoSE 298-322): `calculateDisplacement` for every node (CoSE `CoSENode.js:13-49`): `displacement += coolingFactor * (spring + repulsion + gravity) / noOfChildren`, capped at `coolingFactor * maxNodeDisplacement` per axis (`IMath.sign`). With constraints, `updateDisplacements` (CoSE 588-800) adjusts the displacements to satisfy alignment and relative placement; it contains the shuffle, every 10 iterations, of the lists of nodes in relative placements (635-642). Then `move` (CoSE `CoSENode.js:70-90`): simple nodes move by their displacement, `totalDisplacement += |dx| + |dy|`, and all force accumulators are reset.
8. `animate()` (no effect).

**Constraint phase in detail.** The ConstraintHandler runs once, after `classicLayout`'s tree step and before the spring embedder (CoSE `CoSELayout.js:150-153`). It is the only place that writes the constraint geometry into the initial positions. The port must follow it line by line (section 6.6 and E8).

**Output (fcose).** Once `runLayout` returns, the rectangle centres are read (`getPositions`, `index.js:336-390`, quality default or proof: `result[id].getRect().getCenterX/Y()`). `relocateComponent` (`auxiliary.js:136-198`) first moves all rectangles so that the centre of their combined bounding box equals `componentCenters[0]` (the centre of the elements before the layout). Then `layoutPositions` writes the positions (CYTO `collection/layout.mjs:159`) and fires the layout events (177). fit (162) changes the viewport only.

### E4. Randomness

Every draw, from which generator, in which order, and what survives between layouts.

Generators:

- `Math.random` replaced by `withSeededRandom` (mulberry32 seeded with `config.architecture.seed`, default 1). Replaced at the start of each `layout.run()` and restored after it (`architectureSeed.ts:13-31`).
- `RandomSeed` (LB2 `util/RandomSeed.js`): a page-wide generator with a counter starting at 1 that the page never resets. It is used only by `LNode.scatter` (LB2 `LNode.js:279-295`), which is called only from `positionNodesRandomly` (LB2 `Layout.js:309-337`), which is called only when `!this.incremental` (COSE `CoSELayout.js:115-136`). fcose sets `DEFAULT_INCREMENTAL` to true (`cose.js:213`), so `RandomSeed` is never used by fcose. **[source; the probe agrees: no page-state dependence]**

Draw sites in the default path (`randomize: false`), by source:

| Site                                         | Source                                                                                                                                                                           | Draws                                                           |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Shuffle of the relative-placement node lists | COSE2 `CoSELayout.js:419-430` (defined when `RELAX_MOVEMENT_ON_CONSTRAINTS`, true by default, `CoSEConstants.js:20`), called every 10 iterations at 639-641                      | `j = floor(random * (i + 1))` for `i` from `n-1` down to `2n/3` |
| Pruned-node placement                        | COSE2 `CoSELayout.js:1960` (a 2-way choice) and `2011` (a 4-way choice), in `findPlaceforPrunedNode`                                                                             | Only in tree reduction, which needs no constraints              |
| Spectral start                               | FCOSE `spectral.js:135` (first pivot), `210-211` (two starting vectors per node, interleaved), and the sample on line 52 (`randomSampleCR`, used only with `samplingType` false) | Only with `randomize: true`                                     |

Measured by counting draws per call site on an instrumented copy of the dist (`count.mjs`). Each render runs `withSeededRandom` twice, as the code says:

| Fixtures                                                              | Draws per render (both runs) | Kind                                                                |
| --------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------------- |
| 9 fixtures                                                            | 18 each                      | Shuffle only                                                        |
| 5 fixtures                                                            | 36 each                      | Shuffle only                                                        |
| `reasonable-height`                                                   | 72                           | Shuffle only                                                        |
| The complex nested diagram (`randomize: false`)                       | 954                          | Shuffle only                                                        |
| `titleandaccessibilities`, `fallback-icon`, `edges-at-correct-length` | 0                            | None                                                                |
| `randomize: true` (the complex nested diagram)                        | 1256                         | 1194 shuffle, 62 spectral (2 samples and 60 power-iteration starts) |

The pruned-node draws never fired in the 20 fixtures, so the tree-reduction path is not exercised by them.

Effect on output [probed: `probe.mjs` E2, with the seed set to 1 and 2]: changing the seed changes the output of 3 fixtures: the complex nested diagram, `reasonable-height`, and the randomize-true fixture. The other 17 are unchanged. So the seeded shuffle is the only random input that reaches the output in the default path, and only in some diagrams.

What survives between layouts on one page: nothing, for architecture. Every run starts from the same mulberry32 state, and the cose-base and layout-base constants are set again on each run (`cose.js:180-215`; the conditional assignments are all set on every call except `TILING_COMPARE_BY`, which fcose sets only when given, so the default `undefined` stays). Probed: identical output across fresh and shared pages (E1).

Clock: none in the layout.

### E5. Output

fcose writes node positions as rectangle centres, after relocation. Compound nodes are not written except for childless compounds, which take the first run's values (E3, `index.js:401-405`). Edges have no positions of their own: cytoscape computes their geometry from the node positions and the style (6.4). The segment weights and distances are written by Mermaid after the first run (6.7).

Inside cose-base the output is the rectangle of each node (`CoSENode.getRect()`), with `x`, `y`, `width` and `height`. Mermaid reads only the centres.

### E6. Determinism and floating point

What a port must do to give the same numbers:

- **Operation order.** Floating-point addition is not associative. The force sums (`repulsionForceX -= ...` order), the displacement accumulation, the bounding-box min and max (order-independent), and the sums in the edge-length mean must run in the reference's order. The order of nodes and edges is the cytoscape collection order (insertion order, groups, services, junctions, edges) and the order of the cose-base lists (`getTopMostNodes` order and the order of `processChildrenList`). **[source; cytoscape's collection order: unverified, confirm by capture]**
- **Set and Map order.** `surrounding` is a `Set`, converted with `[...surrounding]` (LB2 `FDLayout.js:519`), so the iteration order is insertion order. `processedNodeSet` is a `Set`. The DB's `Map`s are insertion order.
- **`Object.keys`.** Integer-like keys come first in JavaScript objects. The relocation's `Object.keys(componentResult)` (`auxiliary.js:154-174`) only takes min and max, so its order does not matter. `idToLNode` is an object keyed by id and is read by key. Ids such as `"10"` and `"20"` would change `Object.keys` order; the port should not iterate such objects in any order that matters.
- **Math functions.** `Math.sqrt`, `Math.abs`, `Math.floor`, `Math.ceil`, `Math.max`, `Math.min`: exact or IEEE-defined. `Math.pow` and `Math.log`: used only in the cooling (CoSE 199) and in CoSE 1512 (tiling). The argument domain of the cooling is small: the constants `log(100 * 0.26)` and `log(25)`, and `pow(k, e)` with integer `k` from 1 to 25. [probed: over `k = 1..60`, `Math.pow(k, e) / 100` is bit-identical between V8 and the platform's libm; the two `log` constants are bit-identical too.] Over random inputs V8's `Math.log` differs from the platform's libm in about 2 percent of cases by one ulp (3941 of 200,000), and `Math.atan` in about 5 percent (`IGeometry.angleOfVector` uses it, but nothing calls that function, so it is unreachable from fcose). A port in Rust that uses `f64::ln` must agree on the cooling constants; the probe says it does on this machine.
- **`Math.sign`.** `IMath.sign(0)` is 0 (LB2 `util/IMath.js:7-15`). Used in the displacement cap and in the separation amounts. A port must not return 1 for zero.
- **`parseInt`.** Node paddings come from `parseInt(css('padding'))` (FCOSE `cose.js:97-100`), which gives 40 for groups and 0 for simple nodes. `isNaN` checks set a rectangle coordinate to 0 (FCOSE `cose.js:115-121`). Any NaN that reaches the layout is silently zeroed in these places and nowhere else.
- **Iteration limits.** `maxIterations = max(5n, numIter)`, the convergence test every 100 iterations, the grid every 10, the shuffle every 10. All are integer tests on `totalIterations`; the port must count iterations the same way (the first `tick` makes `totalIterations` 1).
- **Discrete decisions.** The grid cell of a node (`floor((x - left) / range)`), the surrounding test (`distanceX <= range`), the convergence test, the sign tests in `getSegmentWeights` (6.7) and the constraint handler's choices all depend on values near thresholds. A last-bit difference can flip one. The probe that moved the weight computation changed 137 px, so these tests are where a port will drift.

### E7. How to test a port

Capture at the boundary of `layout.run()`, then replay.

1. **Capture the inputs.** On an instrumented copy of the dist (the way `count.mjs` instruments it), log for each run: the node list in order with `id`, `parent`, `position`, the dims from `layoutDimensions`; the edge list in order with `source`, `target`, `idealLength`, `edgeElasticity`; the constraint arrays; the options; and the centre of `options.eles.boundingBox()`. Log the outputs: the rectangle centres after `runLayout`, and the positions after `layoutPositions`. Record both runs.
2. **Replay the engine alone** with the libraries from `node_modules` (not a re-implementation of mermaid): load `cytoscape`, `cytoscape-fcose`, `cose-base` and `layout-base` in Node or in Chromium, build the same cytoscape elements from the capture, call the same options, and compare the positions. This tests the engine's own code against itself on the captured data.
3. **Differential.** The Rust `ferrigura-cose` crate takes the same captured inputs and must reproduce the positions to a stated tolerance (the first step can be bit-exact on the default path, which the probes show is possible).
4. **Determinism check.** Render each of the 20 fixtures twice on fresh pages and once on a shared page; the outputs must match (probed: they do, after the icon ids are masked).
5. **Ablations** to test the mechanisms, as the probes did: `numIter` 100 (18 unchanged), `nodeSeparation` 200 (19 unchanged), `edgeElasticity` 0.6 (17 changed), `idealEdgeLengthMultiplier` 3 (18 changed), seed 2 (3 changed), and the two-run order (3 changed).

### E8. Constraint handler: what a port must read

`ConstraintHandler.handleConstraints` (`ConstraintHandler.js`, 847 lines) is not described in detail here; this note gives its outline, from comments and line markers, and the flags that switch parts on for the default path:

- Lines 9-49: collects the constraints, the node index map (by `node.getChild() == null`) and the draft coordinates (the node centres). Gives a default gap to a relative constraint without one (37-49).
- Lines 51-68: helper functions (position difference, average position).
- Lines 70-281: `find an appropriate positioning for the nodes in a given graph according to relative placement constraints` (comment at line 70). Works on a DAG built from the constraints, with in-degree counts (85) and a positioning pass (166-281). Fixed nodes and alignments are taken into account.
- Lines 282-315: decides whether a transform is needed (both alignment and relative constraints, or a too-small largest component).
- Lines 316-383: weakly connected components, undirected and reversed DAG helpers.
- Lines 384-437: `solve the Orthogonal Procrustean Problem` (SVD, Borg & Groenen chapter 20.2 as cited at line 386). Behind `CoSEConstants.TRANSFORM_ON_CONSTRAINT_HANDLING` (line 438), which is `randomize` for the `step: all` case (FCOSE `cose.js:233-237`). With `randomize: false` this block is skipped.
- Lines 439-610: the transform application and the relative placement passes (not read in full).
- Lines 611-838: `ENFORCE_CONSTRAINTS` (line 611; true for `step: all`): enforces the constraints on the draft.
- Line 839: assigns the new coordinates to the nodes.

The default path (`randomize: false`) therefore runs lines 9-281, 611-839, and the DAG code. Port it line by line, and capture the constraint inputs to test it (E7).

Two more details, from `CoSELayout.js`: the constraint variables are set up in `initConstraintVariables` (324-587), and `updateDisplacements` (588-800) applies the per-iteration corrections with the shuffle at 635-642.

### E9. What fcose shares with cose-bilkent

Compared by `diff` of the two copies (`cose-base@1.0.3` against `2.2.0`, `layout-base@1.0.2` against `2.0.1`). The other task (`docs-port/cose/NOTES.md`) owns the cose-bilkent side.

| File                                                                                     | Difference                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Meaning for a shared port                                                                                                |
| ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| COSE `CoSEGraph.js`, `CoSEGraphManager.js`, `CoSEEdge.js`                                | Identical                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Shared as is                                                                                                             |
| COSE `CoSEConstants.js`                                                                  | 9 lines                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Only the defaults list changes                                                                                           |
| COSE `CoSENode.js`                                                                       | 73 lines                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | `calculateDisplacement`, `move`, the paddings: a version parameter is needed                                             |
| COSE `CoSELayout.js`                                                                     | 1080 lines differ; 7 functions added in 2.2.0 (`moveNodes`, `initSpringEmbedder`, `initConstraintVariables`, `updateDisplacements`, `calcIdealRowWidth`, `getOrgRatio`, `tileNodesByFavoringDim`); 13 shared names have new bodies (`tick`, `classicLayout`, `initParameters`, `reduceTrees`, `growTree`, `findPlaceforPrunedNode`, `tileNodes`, `tileCompoundMembers`, `adjustLocations`, `canAddHorizontal`, `clearZeroDegreeMembers`, `repopulateCompounds`, `repopulateZeroDegreeMembers`) | Different: the cooling in `tick` depends on `PURE_INCREMENTAL` in 2.2.0 (lines 217, 239), and the constraint code is new |
| COSE `ConstraintHandler.js`                                                              | Only in 2.2.0                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | fcose only                                                                                                               |
| LB2 `FDLayout.js`                                                                        | 24 lines: the spring force uses the per-edge elasticity (2.0.1, `springForce = edge.edgeElasticity * ...`) instead of a global constant; the ideal length is kept per edge; the repulsion uses the per-node repulsion value; the displacement threshold is recomputed in `initSpringEmbedder`                                                                                                                                                                                                  | The force phase differs. The two engines do not produce the same numbers for the same input                              |
| LB2 `LGraphManager.js`, `LNode.js`, `FDLayoutEdge.js`, `FDLayoutNode.js`, `IGeometry.js` | 11, 29, 3, 5 and 54 lines                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Small differences; `IGeometry` changes the separation amount                                                             |
| LB2 `util/Matrix.js`, `util/SVD.js`                                                      | Only in 2.0.1                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Spectral and Procrustes only                                                                                             |

Conclusion: the graph model (`CoSEGraph`, `CoSEGraphManager`, `CoSEEdge`), the grid code's shape, the tiling framework and the layout driver are shared in structure. The force phase and the constraint handling are not. The two engines need either one crate with two parameter sets (`ferrigura-cose` with a version enum) or two modules over shared graph types. Decision for the coordinator (Q2).

The random draws also differ: cose-bilkent 1.0.3 reaches `RandomSeed` through `positionNodesRandomly` when `incremental` is false (its `randomize` path). **[unverified: the cose-bilkent task should confirm.]**

---

## Appendix: what was probed, and how

| Probe             | Method                                                                                           | Result                                                                                  |
| ----------------- | ------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------- |
| E1 determinism    | 20 fixtures on fresh pages, on one page in order, and on one page in reverse; icon ids masked    | Identical                                                                               |
| E2 seed           | Seeds 1, 2 and 0 (site config) on each fixture                                                   | 3 fixtures change with the seed                                                         |
| Draw counts       | Instrumented copy; counters at each `Math.random` call site                                      | Section E4                                                                              |
| Second run        | Copy with the second `layout.run()` removed                                                      | 18 translations, 2 reshaped                                                             |
| Weight order      | Copy with the handler's run moved before the weight loop                                         | 3 fixtures change                                                                       |
| Initial positions | Copy logging positions after `cy.add`                                                            | All at (0, 0); group at (0, 8.5)                                                        |
| Label font        | Copy with cytoscape's label font set to Lexica                                                   | 2 fixtures change (4 px and 0.25 px)                                                    |
| Group width       | `a` against a long title                                                                         | 165 px against 323 px                                                                   |
| Knobs             | Site config changes (`nodeSeparation`, `numIter`, `edgeElasticity`, `idealEdgeLengthMultiplier`) | Section 6.5                                                                             |
| Parser            | `mermaid.parse()` in Chromium                                                                    | Section 2.3                                                                             |
| Libm              | 200,000 random inputs in V8 and in Python on macOS (platform libm)                               | `log` 2 percent, `atan` 5 percent differ by one ulp; `pow` and the cooling domain agree |
| Specs             | `vitest run` on the three architecture spec files                                                | 39 pass, 1 fails (canvas in jsdom)                                                      |
