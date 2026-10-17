# Mindmap: research notes

P2 research for the `mindmap` diagram, per [P2-RESEARCH.md](../P2-RESEARCH.md). Paths are under `packages/mermaid/src/` unless they start with another root. Every rule cites the reference's file and line. Two labels mark the evidence:

- **probed**: checked by running the reference. Parser and DB probes ran the reference's own grammar and DB through the port tooling's Vite loader (`tools/port/reference.ts`, `tsx`). Render probes ran the built bundle `packages/mermaid/dist` in Chromium through the geometry harness's page (`tools/port/geometry/common.ts`: `withPage`, `renderReference`, `patchText`). The probe scripts are in the scratch directory `scratchpad/mindmap/` (outside the repository), with their outputs as `parser.json`, `render.json`, `render2.json`, `dom-simple.txt`, `elk-stack.txt`, `html-default.json`. The dist was built on 2026-10-16 11:57; the strings checked against the current source match it.
- **unverified**: read from the source only.

## 0. Summary

Mindmap is a small diagram: an indentation-based tree of nodes, six shapes, and one layout call. The reference's pieces are a Jison grammar (127 lines), a DB of about 430 lines, a renderer of about 120 lines, and styles. The lexer has six states (the initial state and five exclusive ones), and the grammar is short. The renderer runs the unified pipeline with cose-bilkent (or ELK, or a tidy tree that this fork no longer has).

Size (an estimate): the mindmap-specific port is roughly 1,700 to 2,100 lines of Rust: the parser and DB with the canonical model, `getData` and the renderer glue, the cloud, bang and circle shapes with their path arithmetic, the style sheet, and the ported specs (49 runs). The layout engines are separate work: cose-bilkent with `layout-base` 2.0.1 (5,345 lines in `src`), `cose-base` 2.2.0 (3,073) and the plugin (457), about 8,900 lines of JavaScript in all; and the tidy tree (653 lines, removed from this fork, see section 6.12).

Riskiest parts, in order:

1. **The cose-bilkent hand-off.** Every node position comes from the engine, and the edge endpoints do too, from cytoscape's own edge geometry, not from the node boxes (section 6.5, probed). The cose-bilkent port must reproduce that geometry, not only the node positions.
2. **Bounding boxes of the path shapes.** Cloud and bang size themselves by `getBBox()` of an arc path, and the group's box is the layout input (section 6.2, 6.4). Chromium's arc extents must be reproduced.
3. **Text measurement.** The reference's default labels are HTML (`foreignObject`), measured with `getBoundingClientRect`; the harness forces SVG text. Decision 0004 covers the emulation, but the wrap widths differ per shape (section 6.3).
4. **Markdown labels.** Every label is `labelType: markdown` (`mindmapDb.ts:276`), so `marked`'s inline lexing (strong, em, code, `<br>`) runs on the fixtures.
5. **Reference bugs and quirks that a port must decide on**: `layout: elk` throws for every mindmap (probed), `layout: tidy-tree` renders as cose-bilkent because the package is gone (probed), icons are stored and never drawn (probed), and a diagram-scoped `mindmap.layout` is ignored (probed). See section 8.

## 1. Sources

| File                                                                                                                                       | Lines             | What it holds                                                                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------ | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/mindmap/parser/mindmap.jison`                                                                                                    | 127               | Lexer (lines 6-64) and grammar (66-126)                                                                                                        |
| `diagrams/mindmap/mindmapDb.ts`                                                                                                            | 426               | Node tree, `addNode`, `decorateNode`, `getData` (357-420), colour sections                                                                     |
| `diagrams/mindmap/mindmapRenderer.ts`                                                                                                      | 120               | `draw` (33-116): `getData`, shape overrides, `render`, viewport                                                                                |
| `diagrams/mindmap/styles.ts`                                                                                                               | 146               | Style sheet (`genSections` 5-74, `getStyles` 92-145)                                                                                           |
| `diagrams/mindmap/mindmapTypes.ts`                                                                                                         | 22                | `MindmapNode` type                                                                                                                             |
| `diagrams/mindmap/detector.ts`                                                                                                             | 23                | Detector `/^\s*mindmap/` (line 9)                                                                                                              |
| `diagrams/mindmap/mindmap-definition.ts`                                                                                                   | 15                | Assembles `db`, `renderer`, `parser`, `styles`                                                                                                 |
| `rendering-util/render.ts`                                                                                                                 | 129               | Unified `render` (81-103), layout registry (31-79), `getRegisteredLayoutAlgorithm` (113-129)                                                   |
| `rendering-util/layout-algorithms/cose-bilkent/`                                                                                           | 1,050 with tests  | `render.ts` (197), `layout.ts` (77), `cytoscape-setup.ts` (207): the engine's hand-off                                                         |
| `rendering-util/layout-algorithms/elk/render.ts`                                                                                           | 3,021             | ELK path; mindmap edge curve at 2729-2750                                                                                                      |
| `rendering-util/rendering-elements/nodes.ts`                                                                                               | 100               | `insertNode` (14-67)                                                                                                                           |
| `rendering-util/rendering-elements/shapes.ts`                                                                                              | 610               | Shape registry: `rect` 111, `rounded` 120, `circle` 197, `bang` 205, `cloud` 213, `hexagon` 231, `mindmapCircle` 565, `defaultMindmapNode` 566 |
| `rendering-util/rendering-elements/shapes/util.ts`                                                                                         | 387               | `labelHelper` (50-154), `updateNodeBounds` (220-247)                                                                                           |
| `rendering-util/rendering-elements/shapes/{roundedRect,drawRect,squareRect,circle,mindmapCircle,cloud,bang,hexagon,defaultMindmapNode}.ts` | 21-94 each        | The shape handlers (section 6.2)                                                                                                               |
| `rendering-util/createText.ts`                                                                                                             | 419               | `createText` (306-419), HTML path `addHtmlSpan` (34-85), SVG path `createFormattedText` (152-202)                                              |
| `rendering-util/handle-markdown-text.ts`                                                                                                   | 147               | `markdownToLines` (46), `markdownToHTML` (120), `marked`                                                                                       |
| `rendering-util/rendering-elements/edges.js`                                                                                               | 1,091             | `insertEdgeLabel` (74-135), `insertEdge` (624-end)                                                                                             |
| `rendering-util/rendering-elements/markers.js`                                                                                             | ~1,000            | The `point` markers (323-390)                                                                                                                  |
| `rendering-util/insertLookDefs.ts`                                                                                                         | ~60               | Drop-shadow `defs` (24-47), gradient (49-51)                                                                                                   |
| `rendering-util/setupViewPortForSVG.ts`, `setupGraphViewbox.js`                                                                            | 40, 72            | Root `svg` size and viewBox                                                                                                                    |
| `rendering-util/insertElementsForSize.js`                                                                                                  | 18                | `getDiagramElement`                                                                                                                            |
| `diagram-api/detectType.ts`, `Diagram.ts`, `preprocess.ts`, `diagram-api/comments.ts`                                                      | -                 | Parse entry and preprocessing (section 2.1)                                                                                                    |
| `config.ts`, `mermaidAPI.ts`, `schemas/config.schema.yaml`                                                                                 | 330, - , -        | Config layers (section 3.5, 6.9)                                                                                                               |
| `themes/theme-redux-color.js`, `themes/theme-base.js`                                                                                      | -                 | `radius` 12 (line 20), `useGradient` false (32), `THEME_COLOR_LIMIT` 12 (base line 18)                                                         |
| `diagrams/common/common.ts`                                                                                                                | -                 | `sanitizeText` (71-83), `sanitizeMore` (56)                                                                                                    |
| `diagrams/mindmap/*.spec.ts`, `*.test.ts`                                                                                                  | 365, 125, 346, 37 | Specs (section 5)                                                                                                                              |
| `e2e/diagrams/mindmap/`                                                                                                                    | 28 `.mmd`         | Fixtures (section 7)                                                                                                                           |

Removed from this fork: `packages/mermaid-layout-tidy-tree/` (`git show 2d85261d6^:packages/mermaid-layout-tidy-tree/src/…`, `layouts.ts` 13 lines, `render.ts` 180, `layout.ts` 653). Commit `2d85261d6`, FORK.md line 61.

## 2. Parser

### 2.1 What the parser receives

1. `Diagram.fromText` (`Diagram.ts:19-31`) takes the `DiagramCode` from `preprocessDiagram` (`preprocess.ts:58-65`), which strips front matter and directives, then `cleanupComments` (`diagram-api/comments.ts:6-8`): every line that starts with `%%` (not `%%{`) is removed, and the whole text is `trimStart()`ed. A leading blank line or indented root therefore loses its indentation (probed: `leading_blank_lines`, `leading_comment_line`).
2. The parser gets `encodeEntities(cleaned) + '\n'` (`Diagram.ts:77`). `encodeEntities` (`utils.ts:874`) turns `#name;` into placeholders, so `#35;` reaches the DB as `ﬂ°°35¶ß` (probed: `entity_hash`). The renderer decodes them.
3. The DB is cleared before the parse (`Diagram.ts:45-49`). `mindmap-definition.ts` creates a fresh `MindmapDB` for each diagram (`get db()`).
4. Mermaid's directive handling (`mermaidAPI.ts:74-86`) runs before the parse: `reset()`, `setDiagramConfigScope(type)`, `addDirective(config)`. The parse and the render both run it.

### 2.2 Lexer

The lexer is a Jison lexer with `%options case-insensitive` (line 8). It has five exclusive start states (`%x`, lines 13-17): `NODE`, `NSTR` (quoted description), `NSTR2` (backtick-in-quotes description), `ICON`, `CLASS`. Rules without a state apply only in the initial state (`INITIAL`), because the states are exclusive. Rules are tried in file order, the first match wins, and each token's value is `yytext` unless an action sets it.

| Line      | Pattern                                                 | Token or action                                                                       |
| --------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| 21        | `\s*\%\%.*`                                             | `SPACELINE` (comment at the start of a token)                                         |
| 23        | `"mindmap"`                                             | `MINDMAP`                                                                             |
| 24-26     | `":::"`, then `<CLASS>.+` / `<CLASS>\n`                 | Begins `CLASS`; `.+` returns `CLASS` (rest of the line), newline pops without a token |
| 28, 31-32 | `"::icon("`, `<ICON>[^\)]+`, `<ICON>\)`                 | Begins `ICON`; text up to the first `)` (may span lines)                              |
| 29        | `[\s]+[\n]`                                             | `SPACELINE` (blank line with spaces)                                                  |
| 30        | `[\n]+`                                                 | `NL`                                                                                  |
| 33-40     | `-)`, `(-`, `))`, `)`, `((`, `{{`, `(`, `[`             | `NODE_DSTART`, begins `NODE`. Action names are kept in `$1`.                          |
| 41        | `[\s]+`                                                 | `SPACELIST` (indentation, its length is the level)                                    |
| 43        | `[^\(\[\n\)\{\}]+`                                      | `NODE_ID`: everything up to a bracket, including `:`, `%`, spaces and `-`             |
| 44        | `<<EOF>>`                                               | `EOF`                                                                                 |
| 45-50     | `"`+backtick, `<NSTR2>`, `"`, `<NSTR>`                  | Quoted descriptions (`NODE_DESCR`)                                                    |
| 51-58     | `))`, `)`, `]`, `}}`, `(-`, `-)`, `((`, `(` (in `NODE`) | `NODE_DEND`, pops `NODE`                                                              |
| 59-60     | `<NODE>[^\)\]\(\}]+`, `<NODE>.+(?!\(\()`                | `NODE_DESCR` (may contain newlines and spaces)                                        |

### 2.3 Grammar

| Rule (lines)                                                                    | Action (what the DB receives)                                             |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `start` (70-74): `mindMap` or `spaceLines mindMap`                              | -                                                                         |
| `mindMap` (82-85): `MINDMAP document` or `MINDMAP NL document`                  | `return yy`                                                               |
| `document` (94-97), `stop` (87-93)                                              | Statements separated by `NL`, `EOF`, `SPACELINE`                          |
| `statement` (99-108): `SPACELIST node`                                          | `yy.addNode($1.length, id, descr, type)`: level is the indentation length |
| `statement`: `node` (no indentation)                                            | `yy.addNode(0, …)`                                                        |
| `statement`: `SPACELIST ICON` / `ICON`                                          | `yy.decorateNode({icon})` on the last node                                |
| `statement`: `SPACELIST CLASS` / `CLASS`                                        | `yy.decorateNode({class})` on the last node                               |
| `statement`: `SPACELIST` alone, `SPACELINE`                                     | Nothing                                                                   |
| `nodeWithId` (122-126): `NODE_ID` or `NODE_ID NODE_DSTART NODE_DESCR NODE_DEND` | `type = yy.getType(start, end)`                                           |
| `nodeWithoutId` (117-120): `NODE_DSTART NODE_DESCR NODE_DEND`                   | `id = descr`, `type = getType(start, end)`                                |

### 2.4 Quirks a port must reproduce (all probed)

The inputs are in `scratchpad/mindmap/probe-parser.mts` (66 cases; `parser.json` holds the trees). "id" is the DB's `nodeId`, "descr" its `descr`.

**Structure and errors**

| Input                                                  | Result                                                                                                       |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| `mindmap\n  root`                                      | Root `root`, type 0                                                                                          |
| `mindmap` (nothing after)                              | Parse error on line 2                                                                                        |
| `mindmap\n` (no nodes)                                 | Parses, no tree; `draw` returns early (6.6)                                                                  |
| `mindmap root` (one line)                              | Root `root`                                                                                                  |
| `MindMap\n  root`                                      | Works (keyword case-insensitive)                                                                             |
| `mindmap\nmindmapper`                                  | Node `mindmapper` is a plain id (the keyword is not split out; the mechanism is unverified)                  |
| `mindmap\nroot\nfake`                                  | Error: `There can be only one root. No parent could be found for ("fake")`                                   |
| `mindmap\n    root\n  child`                           | Same error for `child` (its level would be negative)                                                         |
| `mindmap\n    root\n        a\n      b`                | `a` and `b` are both children of `root`; levels 4 and 2                                                      |
| `mindmap\n\troot\n\t\tchild`                           | Tabs count one column each: level 1                                                                          |
| `mindmap\nroot\n  %% c\n  child %% trailing`           | `%%` line removed. The id is `child %% trailing`: after a plain id, `%%` is text                             |
| `mindmap\nroot\n  :::`                                 | Parses; no class                                                                                             |
| `mindmap\n  ::icon(x)\n  root`                         | Raw `TypeError`: `Cannot set properties of undefined (setting 'icon')` (the same for `:::`, setting `class`) |
| `mindmap\nroot\n  ::icon(a\nb)`                        | Icon `a\nb` (the icon may span lines)                                                                        |
| `mindmap\nroot\n  ::icon(mdi:fire)`                    | Icon `mdi:fire` (colon kept)                                                                                 |
| `mindmap\nroot\n  :::hot   `                           | Class `hot   `, trailing spaces kept                                                                         |
| `mindmap\nroot\n  :::m-4 p-8`                          | Class `m-4 p-8`                                                                                              |
| `…root[R]\n  ::icon(a)\n  ::icon(b)\n  :::c1\n  :::c2` | Last icon and last class win (`b`, `c2`)                                                                     |
| `mindmap\nroot\n  a\n  ::icon(x)\n  b`                 | The icon goes to the previous node `a`                                                                       |
| `mindmap\nroot\n  child:::hot`                         | Id `child:::hot`, no class: `:::` starts a class only at a token start on its own line                       |
| `mindmap\nroot\n  two words`                           | Id `two words`                                                                                               |
| `mindmap\nroot   ` (trailing spaces)                   | Id `root   ` (spaces kept in `nodeId` and `descr`)                                                           |
| `mindmap\n  root   \n    child   `                     | Both keep their trailing spaces                                                                              |
| `\r\n` line ends                                       | Parse fine                                                                                                   |

**Shapes** (the start and end tokens decide `type`: `getType`, `mindmapDb.ts:129-147`)

| Input                       | id           | descr        | type           | Why                                                                            |
| --------------------------- | ------------ | ------------ | -------------- | ------------------------------------------------------------------------------ |
| `root[Root]`                | root         | Root         | 2 RECT         | `[`                                                                            |
| `root(Root)`                | root         | Root         | 1 ROUNDED_RECT | `(` with `)`                                                                   |
| `root((Root))`              | root         | Root         | 3 CIRCLE       | `((`                                                                           |
| `root)Root(`                | root         | Root         | 4 CLOUD        | `)`                                                                            |
| `root))Root((`              | root         | Root         | 5 BANG         | `))`                                                                           |
| `root{{Root}}`              | root         | Root         | 6 HEXAGON      | `{{`                                                                           |
| `root(Root))`               | root         | Root         | 4 CLOUD        | `(` with `))`: only `)` end gives ROUNDED                                      |
| `root(Root]`                | root         | Root         | 4 CLOUD        | `(` with `]`                                                                   |
| `root[Root)`                | root         | Root         | 2 RECT         | `[` with `)`                                                                   |
| `root((Root)`               | root         | Root         | 3 CIRCLE       | `((` with `)`                                                                  |
| `root(-Root-)`              | root         | `Root-`      | 0 DEFAULT      | `(-` is not in `getType`; the `-` lands in the description; `-)` never matches |
| `root-)Root(-`              | `root-`      | Root         | 4 CLOUD        | `-` belongs to the id; `(-` is never used as an end                            |
| `(Root)` (no id)            | Root         | Root         | 1              | `nodeWithoutId`: id = descr                                                    |
| `[Root]` (no id)            | Root         | Root         | 2              | same                                                                           |
| `Default text` (plain line) | Default text | Default text | 0              | default                                                                        |

**Descriptions**

| Input                          | Result                                                                                          |
| ------------------------------ | ----------------------------------------------------------------------------------------------- |
| `root["String containing []"]` | descr `String containing []`                                                                    |
| `root["a(b)c"]`                | descr `a(b)c`                                                                                   |
| `root["a"b]`                   | Parse error                                                                                     |
| `root["`**bold** text`"]`      | descr `**bold** text` (backticks removed)                                                       |
| `root[`**bold**`]`             | descr `` `**bold**` `` (backticks kept, as text)                                                |
| `root(\n  The root\n)`         | descr `\n  The root\n` (newlines and indentation kept)                                          |
| `root((grand<br/>child))`      | descr `grand<br>child` (DOMPurify rewrites `<br/>`)                                             |
| `root(a)b`                     | Parse error (text after a shape)                                                                |
| `root[a(b]`                    | Parse error (`(` ends the description)                                                          |
| `root[a{b}c]`                  | Parse error                                                                                     |
| `root(a[b]c)`                  | Parse error (`]` ends the description)                                                          |
| `root[A & B]`                  | descr `A & B` (no encoding: DOMPurify returns text without `<` unchanged, `canonical-model.md`) |
| `root[50%% off]`               | descr `50%% off` (not a comment inside brackets)                                                |
| `root[<script>x</script>y]`    | descr `y` (script removed with its content)                                                     |
| `root[a #35; b]`               | descr `a ﬂ°°35¶ß b` (placeholder, decoded at render)                                            |
| `root[😎 smile]`               | descr `😎 smile`                                                                                |

**Comments.** The lexer's `%%` rule (line 21) matches only at a token start. After a shape, ` %% This is a comment` is a comment (spec MMP-22, `mindmap.spec.ts:311`). After a plain id it is text (probed above).

### 2.5 Errors

- The grammar's own errors are Jison's `Parse error on line N: …` with the expected-token list. A port cannot reproduce the text exactly; it should reject the same inputs (section 4).
- `addNode` throws `There can be only one root. No parent could be found for ("<descr>")` (`mindmapDb.ts:122-124`). This message is exact and tested (`mindmap.spec.ts:61-78`).
- `decorateNode` with no node yet throws a JS `TypeError` (`mindmapDb.ts:162`, `this.nodes[this.nodes.length - 1]` is `undefined`). Probed for icon and class. The text is an engine message.

## 3. DB

### 3.1 Calls, in order

| Call                                       | Who calls                           | What it does (`mindmapDb.ts`)                                                                                                                                                                                                                                                                         |
| ------------------------------------------ | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `clear()` (57-62)                          | `Diagram.fromText` before the parse | Empties `nodes`, `count`, `elements` and `baseLevel`                                                                                                                                                                                                                                                  |
| `addNode(level, id, descr, type)` (77-127) | parser                              | First node: root, `baseLevel = level`, level 0. Later: `level - baseLevel`. Padding from config (91-100), doubled for types 1, 2 and 6. `nodeId` and `descr` through `sanitizeText`. `width = maxNodeWidth`. Attached to the nearest earlier node with a smaller level (`getParent`, 64-71) or thrown |
| `getType(start, end)` (129-147)            | parser                              | The table in 2.4                                                                                                                                                                                                                                                                                      |
| `decorateNode({icon?, class?})` (156-169)  | parser                              | Last node; `sanitizeText` on both                                                                                                                                                                                                                                                                     |
| `getData()` (357-420)                      | renderer                            | Section 3.5                                                                                                                                                                                                                                                                                           |

### 3.2 What is stored, and sanitising

- `nodeId`, `descr`, `icon`, `class` go through `sanitizeText(text, config)` (`common.ts:71-83`): `sanitizeMore` (line 56), then DOMPurify with `FORBID_TAGS: ['style']`. Probed: scripts are removed with their content; `<br/>` becomes `<br>`; `&` is kept.
- `descr` is sanitised again in `labelHelper` (`shapes/util.ts:95`) after `decodeEntities`. The DB value is the sanitised one, not the rendered one.
- Node `id` is a counter from 0 in `addNode` order (`mindmapDb.ts:103`). It is also the DOM id suffix (`node_<id>`, `mindmapDb.ts:274`).
- `width` and `padding` are fixed at parse time from the config in force then (`mindmapDb.ts:91-110`).

### 3.3 Defaults (`schemas/config.schema.yaml`, `MindmapDiagramConfig` 1187-1219)

| Key                       | Default                            | Read by                                                               |
| ------------------------- | ---------------------------------- | --------------------------------------------------------------------- |
| `mindmap.padding`         | 10                                 | `addNode` (92), `setupViewPortForSVG` (`mindmapRenderer.ts:112`)      |
| `mindmap.maxNodeWidth`    | 200                                | `addNode` (109): the node's `width`                                   |
| `mindmap.colorBy`         | `branch`                           | `getData` (379) through `assignSections`                              |
| `mindmap.layoutAlgorithm` | `cose-bilkent`                     | **Not read** by the mindmap code (grep: only in the schema). Dead key |
| `mindmap.useMaxWidth`     | true (probed: `width="100%"`)      | `mindmapRenderer.ts:114`                                              |
| top-level `layout`        | `elk` (`config.schema.yaml:76-82`) | `getData` (361-367)                                                   |

### 3.4 clear()

`clear()` resets all four fields. Nothing else persists between parses. `elements`, `setElementForId` and `getElementById` (152-154) are never called by mindmap code: dead.

### 3.5 getData (357-420)

1. **Layout selection** (361-367). `hasUserDefinedLayout` is true when the top-level `layout` is set in `initialize()` or in a directive or front matter (`getUserDefinedConfig`, `config.ts:305-317`). If it is not set, `config.layout` becomes `cose-bilkent`. Probed table:

   | How the layout is set                              | Layout used                                                                                                                |
   | -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
   | (nothing)                                          | cose-bilkent                                                                                                               |
   | front matter `config.layout: elk`                  | elk                                                                                                                        |
   | front matter `config.layout: tidy-tree`            | tidy-tree, which renders as cose-bilkent (6.12)                                                                            |
   | `%%{init: {"layout": "tidy-tree"}}%%`              | same as front matter                                                                                                       |
   | `%%{init: {"mindmap": {"layout": "tidy-tree"}}}%%` | **cose-bilkent**: the diagram-scoped key is resolved into the config but not into `userDefinedConfig`, so it is overridden |
   | front matter `config.mindmap.layout: tidy-tree`    | **cose-bilkent** (same reason)                                                                                             |
   | `initialize({layout: 'elk'})`                      | elk                                                                                                                        |
   | `mermaid.updateSiteConfig({layout: 'tidy-tree'})`  | **cose-bilkent**: `updateSiteConfig` (`config.ts:184`) writes only the site delta                                          |
   | `layout: swimlane`, `dagre`, `elk.stress`          | Passed through as the name; the render falls back (6.12)                                                                   |

   The resolution of the diagram-scoped `layout` is `resolveAppearance` (`config.ts:79`) and `setDiagramConfigScope` (`config.ts:149`), which `mermaidAPI.ts:83` calls with the detected type.

2. **Empty tree**: returns `nodes: []`, `edges: []`, `config` and no `rootNode` (369-375).
3. **Sections** (379): `assignSections` (199-220). The root has no section (`undefined`). Under `colorBy: branch` a child of the root gets `index % 11` and its descendants inherit it. Under `colorBy: depth` every node gets `(depth - 1) % 11`, with the root at depth 0. `MAX_SECTIONS` is 12 (line 12), so 11 colours.
4. **Flattening** (385, 227-306). Pre-order. Each node: `id` (string), `domId` `node_<id>`, `label` = descr, `labelType: 'markdown'`, `isGroup: false`, `shape` (table below), `width` (maxNodeWidth), `height` 0, `padding`, `cssClasses`, `cssStyles: []`, `centerLabel: true`, `look: conf.look`, `icon`, `x`/`y` (undefined), `level`, `nodeId`, `type`, `section`. The `x: node.x` and `y: node.y` keys are present with `undefined` values (lines 289-290). Section 8 explains why that matters for ELK.
5. **Classes** (320-329, 230-245): `mindmap-node`; for the root `section-root section--1`; otherwise `section-<n>`; then the node's own class, joined by one space. A class with trailing spaces keeps them (probed: `'mindmap-node section-root section--1 hot   '`).
6. **Shape map** (248-270, probed):

   | Node `type`    | `shape`                                   | With redux theme | Other themes                                   |
   | -------------- | ----------------------------------------- | ---------------- | ---------------------------------------------- |
   | 0 DEFAULT      | `rounded` (redux) or `defaultMindmapNode` | `rounded`        | `defaultMindmapNode` (not ported, M6 decision) |
   | 1 ROUNDED_RECT | `rounded`                                 | same             | same                                           |
   | 2 RECT         | `rect`                                    | same             | same                                           |
   | 3 CIRCLE       | `mindmapCircle`                           | same             | same                                           |
   | 4 CLOUD        | `cloud`                                   | same             | same                                           |
   | 5 BANG         | `bang`                                    | same             | same                                           |
   | 6 HEXAGON      | `hexagon`                                 | same             | same                                           |

   `isReduxTheme` is `conf.theme` lowercased, containing `redux` (249-250).

7. **Edges** (313-350). Pre-order, one per child: `id = edge_<parent>_<child>`, `start`/`end` (string ids), `type: 'normal'`, `curve: 'basis'`, `thickness: 'normal'`, `look`, `classes` = `edge` + ` section-edge-<child.section>` (if defined) + ` edge-depth-<parent.level + 1>`, `depth: parent.level`, `section: child.section`. The depth class is built from the **indentation delta**, not from the tree depth (probed: in the simple fixture the edge under `Photograph` has `edge-depth-3`). Its stroke width comes from `.edge-depth-N` in the style sheet (6.8).
8. **Shapes map** (392-401), **`rootNode`** (408), **`markers: ['point']`** (410), **`direction: 'TB'`** (411), **`nodeSpacing` and `rankSpacing`** (412-413), **`diagramId: 'mindmap-' + v4()`** (418), **`type: 'mindmap'`** (417).
   - `shapes` is read by nothing (grep). Dead.
   - `rootNode` is read by the cose-bilkent validation (`cose-bilkent/layout.ts:64-66`: a missing root throws). Needed.
   - `markers` create the four `point` markers (6.6). Needed for the DOM.
   - `direction` is passed to `insertNode` as `dir` (`cose-bilkent/render.ts:72`) and not read there. Dead.
   - `nodeSpacing` and `rankSpacing` are read only by the DDLT path (`ddlt/parseToLayoutData.ts`), not by the mindmap. Dead.
   - `diagramId` is overwritten by `draw` before the render (`mindmapRenderer.ts:51`). Dead; the random uuid is never used. uuid 14's browser `rng` uses `crypto.getRandomValues` (`node_modules/.pnpm/uuid@14.0.0/node_modules/uuid/dist/rng.js:2`), not `Math.random`, so it does not disturb the seeded sequence (unverified against the browser build; the render probes were repeatable).
9. **Dead methods**: `type2Str` (171-190, with the typo `hexgon`) is never called. `_updateNodeDimensions` in the renderer (15-31) is never called.

## 4. Canonical model (proposal)

The differential compares the parser's result for each input (`tools/port/`, [canonical-model.md](../canonical-model.md)).

```json
{
  "version": 1,
  "root": {
    "id": 0,
    "nodeId": "root",
    "descr": "root",
    "type": 0,
    "level": 0,
    "icon": "",
    "class": "",
    "children": [
      {
        "id": 1,
        "nodeId": "a",
        "descr": "a",
        "type": 2,
        "level": 2,
        "icon": "x",
        "class": "",
        "children": []
      }
    ]
  }
}
```

- `id` is the DB counter; `level` is the indentation delta (needed for the parent search and the depth class); `type` is the number; `icon` and `class` are `""` when absent (the canonical rule for strings). A class keeps its trailing spaces, as the DB does.
- Left out: `isRoot` (the root is the first node), `width` and `padding` (config-derived; they belong to `getData`), `section` (render-time), `x` and `y`.
- Errors: a parse error is `{ "ok": false }`. The DB's own message (`There can be only one root…`) must match exactly. Jison's text and the engine's `TypeError` text are not compared (section 8, question 4).
- A second, optional model for `getData` (for `port:diff`): `nodes` with `id`, `label`, `shape`, `cssClasses`, `icon`, `level`, `section`, `width`, `padding`; `edges` with `id`, `start`, `end`, `classes`, `depth`, `section`; and `layout` after section 3.5's rules. Left out: `diagramId`, `shapes`, `rootNode`, `markers`, `direction`, `nodeSpacing`, `rankSpacing`, `type` (dead or random).

## 5. Specs

| File                              | Tests                                               | Covers                                                                                                                                                                                   |
| --------------------------------- | --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mindmap.spec.ts` (365 lines)     | 26                                                  | Parser through the Jison parser and the DB: hierarchy (MMP-1 to 6), shapes (MMP-7 to 12-a), decorations (13-16), descriptions (17-19), rows and comments (20-25). Parser only            |
| `mindmapColors.spec.ts` (125)     | 7 (8 runs, `it.each` over `undefined` and `branch`) | Sections under `colorBy` `branch` and `depth`; custom classes and icons survive; the same tree under two modes; empty and root-only trees. DB and `getData`                              |
| `mindmapDb.getData.test.ts` (346) | 12                                                  | `getData` on a hand-built tree: empty, shape of records, edge ids, `centerLabel`, section classes, custom classes, the 11-section wrap. It mocks `getConfig` to a config without a theme |
| `mindmapLayout.spec.ts` (37)      | 3                                                   | The layout selection of section 3.5, rule 1 (the third test repeats the second)                                                                                                          |

Outside the diagram folder, and mentioning mindmaps:

- `shapes/defaultMindmapNode.spec.ts`: one test, throws without `domId` (not ported with the classic branch).
- `cose-bilkent/layout.test.ts`: 11 tests; mindmap-shaped data with `rootNode` (`:69`, `:107`), and the empty-mindmap case (`:214`). For the cose-bilkent task.
- `multi-diagram-id-uniqueness.spec.ts:300`: skips mindmap ("cytoscape crashes in JSDOM").
- `styles.spec.ts`, `config.spec.ts`, `config.appearance.spec.ts`, `diagram-orchestration.spec.ts`, `diagram-beta-policy.spec.ts`, `themes/theme-redux-color-superset.spec.ts`: mention mindmap (2 to 4 lines each).
- `e2e/rendering/mindmap/`: three Playwright specs (depth colours, neo look), each with one or two tests, that check the rendered DOM.

The port's specs: the parser and DB specs (49 runs) port directly. The renderer has no spec of its own, so the geometry harness is its test (section 7).

## 6. Renderer

### 6.1 Pipeline

`draw` (`mindmapRenderer.ts:33`):

1. `db.getData()` (41), then `getDiagramElement(id)` (44: the placeholder `svg` the caller created, `insertElementsForSize.js:3-17`).
2. `layoutAlgorithm = getRegisteredLayoutAlgorithm(config.layout, {fallback: 'cose-bilkent'})` (47-49). Unregistered names fall back with a warning (`rendering-util/render.ts:113-129`).
3. `diagramId = id` (51). If there is no root, return here (53-56): the `svg` keeps only its placeholder (probed: `mindmap\n` gives an `svg` with no class, no viewBox, no style and an empty `g`).
4. Shape overrides (58-74), applied to the layout records before the render:

   | Shape     | Override                                                                    | Effect                                                                                        |
   | --------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
   | `rounded` | `radius = 15`, `taper = 15`, `stroke = 'none'`, `width = 0`, `padding = 15` | `radius`, `taper` and `stroke` have no reader (grep). `width = 0` and `padding = 15` are used |
   | `circle`  | `padding = 10`                                                              | Dead: mindmap circles have shape `mindmapCircle`, never `circle`                              |
   | `rect`    | `width = 0`, `padding = 10`                                                 | Used                                                                                          |
   | `hexagon` | `width = 0`, `height = 0`                                                   | Used                                                                                          |

5. `render(data4Layout, svg)` (77): prefixes every node's `domId` with the diagram id (`render.ts:86-93`), calls `insertLookDefs` (98), and runs the layout renderer.
6. The gradient (83-107): only when `useGradient`, `gradientStart` and `gradientStop` are set. The shipped redux themes have `useGradient = false` (`theme-redux-color.js:32`), so this is inactive for them. It duplicates `insertLookDefs`' gradient id (`insertLookDefs.ts:55`) when it does run.
7. `setupViewPortForSVG(svg, mindmap.padding, 'mindmapDiagram', useMaxWidth)` (110-115), section 6.6.

The layout renderer, for cose-bilkent, is `cose-bilkent/render.ts` (6.4). `render.ts` is the same for ELK and the tidy tree.

### 6.2 Node shapes

Every shape starts with `labelHelper` (`shapes/util.ts:50-154`), which creates `g.node` and `g.label`, measures the label (6.3), and inserts an empty `rect` first in the label group (152).

| Shape (`shapes.ts` name → file)                    | Size formula (from the measured label `bw`, `bh`, and the node padding `p`)                                                                                                                                                                                                                                                             | Element                                                                                         | Node `width`/`height` after the shape            |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `rounded` → `roundedRect.ts` → `drawRect.ts`       | `labelPaddingX = labelPaddingY = p` (`roundedRect.ts:16-17`). Total = `max(bw + 2p, node.width or 0)` × `max(bh + 2p, node.height or 0)` (`drawRect.ts:21-22`). With `width = 0`, `p = 15`: `bw + 30` × `bh + 30`. Corner radius = `themeVariables.radius` (12 in redux, `roundedRect.ts:10`; the renderer's `radius = 15` is not read) | `rect.basic.label-container`, `rx`/`ry` = 12, x and y = −total/2                                | Known bounds, no `getBBox` (`drawRect.ts:68-72`) |
| `rect` → `squareRect.ts` → `drawRect.ts`           | `labelPaddingX = 2p` (classic) and `labelPaddingY = p` (`squareRect.ts:9-10`); with `p = 10` total is `bw + 40` × `bh + 20`. Neo: 16 and 12                                                                                                                                                                                             | `rect`, `rx`/`ry` removed (`rx` is 0 and falsy in `drawRect.ts:33`, so the attribute is `null`) | Known bounds                                     |
| `mindmapCircle` → `mindmapCircle.ts` → `circle.ts` | `radius = sqrt(bw² + bh²)/2 + p` (`circle.ts:23-29`), with `labelHelper(…, {ignoreMinWidth: true})` (18-20). Neo: `+ 16`                                                                                                                                                                                                                | `circle.basic.label-container` with `r`, `cx = 0`, `cy = 0`                                     | `getBBox()` of the circle (52)                   |
| `cloud` → `cloud.ts`                               | `hp = halfPadding = p/2` (`labelHelper`, `util.ts:116`); `w = bw + 2hp`, `h = bh + 2hp` (20-21). Path from `cloud.ts:33-47` with radii `0.15w, 0.25w, 0.35w, 0.2w`. Label `translate(-bw/2, -bh/2)` (64), path `translate(-w/2, -h/2)` (67)                                                                                             | `path.basic.label-container` with `d` (the path string has embedded newlines and spaces)        | `getBBox()` of the path (69)                     |
| `bang` → `bang.ts`                                 | With `hp` as for cloud: `w = bw + 10hp`, `h = bh + 8hp` (20-21), `minWidth = bw + 20`, `minHeight = bh + 20` (25-28), effective = max. Path 33-51, `translate(-ew/2, -eh/2)` (69)                                                                                                                                                       | `path.basic.label-container`                                                                    | `getBBox()` of the path (71)                     |
| `hexagon` → `hexagon.ts`                           | `labelPaddingX = labelPaddingY = p` (35-36, classic). `h = bh + p` (46), `m = h/4` (47), `w = bw + 2m + p` (49). Points `(m,0) (w−m,0) (w,−h/2) (w−m,−h) (m,−h) (0,−h/2)` (50-57)                                                                                                                                                       | Polygon from `insertPolygonShape` (77)                                                          | `w`, `h` (84-85)                                 |
| `defaultMindmapNode` → `defaultMindmapNode.ts`     | Classic themes only (`w = bw + 8hp`, `h = bh + 2hp`). **Not ported** (M6 decision: classic-theme branches are not ported)                                                                                                                                                                                                               |                                                                                                 |                                                  |

Note on `p`: the renderer changes `p` after the DB sets it (`mindmapRenderer.ts:58-74`). The DB value is 10 for `circle`, `cloud` and `bang`, and `2 × 10 = 20` for `rect`, `rounded` and `hexagon` (`mindmapDb.ts:94-100`). Then `rounded` is set to 15 and `rect` to 10 by `draw`. Only the hexagon keeps 20.

Node intersect functions: `rect` for rect, rounded, cloud and bang (`drawRect.ts:78-80`, `cloud.ts:74-76`, `bang.ts:75-77`); a circle for `mindmapCircle` (`circle.ts:57-60`); the polygon for hexagon (`hexagon.ts:89-91`). They are set on the DB record that `insertNode` receives, not on the copy the layout code keeps (6.5).

### 6.3 Text measurement

`createText` (`createText.ts:306-419`) with `markdown: true`, `width` and `classes: 'markdown-node-label'` (`util.ts:93-106`).

`width` = `node.width || node.wrappingWidth || flowchart.wrappingWidth` (`util.ts:98`). Probed consequences:

- rounded, rect, hexagon and default labels (width zeroed) wrap at `flowchart.wrappingWidth`, which is **120** (probed: `max-width: 120px` on the foreignObject divs). `mindmap.maxNodeWidth` does not change them.
- circle, cloud and bang keep `width = maxNodeWidth` = **200** (probed: `max-width: 200px` on the root circle).

The measurements, in the order they run:

| What                                                                                                                                           | Where                          | Measured                                                                              | Feeds                                                               |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| HTML label, default (`htmlLabels` not false): `div.getBoundingClientRect()` after `fastdom.measure`                                            | `util.ts:119-133`              | Box of the div                                                                        | Node size; `foreignObject` width and height are set to it (132-133) |
| HTML wrap test: `div.getBoundingClientRect()`                                                                                                  | `createText.ts:73-82`          | Width against `width`: equal means `display: table; white-space: break-spaces; width` | Line breaking (the DOM only)                                        |
| SVG label (`htmlLabels: false`): `text.getBBox()`                                                                                              | `util.ts:134-141`              | Box of the text element                                                               | Node size and label translate                                       |
| SVG wrap: `getComputedTextLength()` of a test `tspan` per candidate line                                                                       | `createText.ts:114-125`        | Advance of the line                                                                   | Whether `splitLineToFitWidth` runs (`splitText.ts`)                 |
| Background (only with an icon): `textElement.getBBox()`                                                                                        | `createText.ts:187-196`        | Box of the text                                                                       | Background rect at −2 and +4                                        |
| Edge label (every edge, even empty): HTML `getBoundingClientRect()` (`edges.js:114`), or SVG `getBBox()` of the label and its `text` (119-126) | `edges.js:111-126`             | Box                                                                                   | `edge.width` and `height` (unused) and the label `transform` (130)  |
| Root `svg`: `getBBox()` of the svg element                                                                                                     | `setupViewPortForSVG.ts:29`    | Union of the children                                                                 | viewBox and max-width                                               |
| Edge path, neo look only: `getTotalLength()`                                                                                                   | `edges.js:879-892`             | Path length                                                                           | Dash array                                                          |
| Layout input: `getBBox()` of each node group                                                                                                   | `cose-bilkent/render.ts:76-79` | Group box (the label and the shape, with transforms)                                  | Width and height handed to the engine                               |

Markdown: `markdownToLines` (SVG) and `markdownToHTML` (HTML) use `marked.lexer` (`handle-markdown-text.ts:46-48`, `:120-121`). Labels are passed through `sanitizeText(decodeEntities(label))` first (`util.ts:95`).

Text in HTML mode is not the same as SVG mode: decision 0004 draws it as SVG text and emulates the line breaks.

### 6.4 The layout hand-off (cose-bilkent)

`cose-bilkent/render.ts` (197 lines):

1. Markers (41-43): `insertMarkers(element, ['point'], 'mindmap', diagramId)`.
2. Four container groups, in this order: `subgraphs`, `edgePaths`, `edgeLabels`, `nodes` (46-49). Their order in the DOM is the paint order: nodes paint over edges.
3. Nodes (54-84): for each node, a copy `{...node}` is stored in `nodeDb` (66-67) **before** `insertNode` runs; `insertNode` (70-73) draws the shape into the `nodes` group; the group's `getBBox()` gives `width` and `height` (76-78). `domId` is the group element (79). All the nodes run under `Promise.all`; the DOM order is the data order (probed).
4. Layout input (90-100): each record is `{...node, width, height}`, with the group sizes.
5. `executeCoseBilkentLayout(updatedLayoutData, config)` (102).
6. Positions (107-123): `transform = translate(x, y)` with the engine's centre; `node.x`, `node.y` stored on the copy.
7. Edge points (125-135): `edge.points = [{startX, startY}, {x: midX, y: midY}, {endX, endY}]`.
8. Edges (140-194): `insertEdgeLabel` (143), then `insertEdge` with the copies `startNode` and `endNode` (159-167), then `positionEdgeLabel` (170).

**What the mindmap hands the engine** (`cytoscape-setup.ts`, `layout.ts`):

- `validateLayoutData` (`layout.ts:55-77`) requires `config`, `rootNode`, `nodes` and `edges`.
- Each node becomes a cytoscape node with `id` (the string), `labelText` (the label), `height`, `width`, `padding` (default 0), every other own key of the record (`level`, `nodeId`, `type`, `section`, `shape`, `cssClasses`, `cssStyles`, `look`, `icon`, `centerLabel`, `labelType`, `isGroup`, `domId`, `labelStyle`, and in JavaScript the `intersect` and `calcIntersect` functions from the shape), and a position `x ?? 0`, `y ?? 0` (`cytoscape-setup.ts:36-62`). The mindmap nodes have no `x` or `y`, so every start is 0,0.
- Each edge becomes a cytoscape edge with `id`, `source`, `target` and every other key (`curve`, `thickness`, `look`, `classes`, `depth`, `section`, `type`) (70-90).
- `layoutDimensions` returns the node's `width` and `height` (`cytoscape-setup.ts:122-127`): the engine lays out with the group sizes, not with cytoscape's style.
- The layout options are `{ name: 'cose-bilkent', quality: 'proof', styleEnabled: false, animate: false }` (`cytoscape-setup.ts:130-136`). Everything else is the engine's default, including `randomize: true` (`cytoscape-cose-bilkent.js:119`). The mindmap passes no `layout` options of its own.
- The cytoscape container is a hidden `div#cy` appended to `body` and removed after construction (`cytoscape-setup.ts:100-115`). That is a DOM dependency of the reference's layout.

**What the mindmap reads back**: `extractPositionedNodes` (152-173): `x` and `y` are the node's **centre**. `extractPositionedEdges` (180-207): `startX…endY` from `edge._private.rscratch` (183-195).

### 6.5 Edges: no clipping, endpoints from cytoscape's default node

`insertEdge` (`edges.js:624`) clips the endpoints to the node shapes only when `head.intersect && tail.intersect && !skipIntersect` (line 699). The `startNode` and `endNode` it receives are the copies made before `insertNode` (6.4), and those copies have no `intersect`: the shape functions were set on the record `insertNode` received. So the clipping is skipped. The path uses the engine's three points as they are (probed).

The probed data (`data-points` of each edge, base64 JSON, and the group boxes): the start point is at distance 15.0 from the parent's centre, and the end point is at distance 15.0 from the child's centre. The node boxes are 47 px high and 55 to 101 px wide. So the endpoints are on a circle of radius 15, the half size of cytoscape's default 30 px node, not on the node boxes. The edge runs through the node; the nodes are painted after the edges, so the join is hidden under the fill. The port has to reproduce cytoscape's rscratch endpoints for default-size nodes, which is the cose-bilkent task's job.

Then the path (`edges.js:724-811`): `fixCorners` (729-731, for non-`rounded` curves), `curveBasis` through the three points (737-738, `curve: 'basis'`), `lineFunction` (777-778).

Path element (857-901, probed): `id = <diagramId>-<edge.id>` (860); class `" edge-thickness-normal edge-pattern-solid edge section-edge-N edge-depth-K"` with a leading space (863-867); `style` = `"undefined;;;undefined"` (853-856, probed; see below); `data-look`, `data-id`, `data-et="edge"`, `data-edge="true"`, `data-points` (705). `data-points` is `btoa(JSON.stringify(points))`, so the number formatting matters.

The style bug: `edge.style` is undefined, so `edgeStyles` is `[undefined]` (812), `styles` is `"undefined;"` (851), and `pathStyle` becomes `undefined;;;undefined`. The browser ignores it. The port must reproduce the attribute if the diff compares styles. Probed for mindmap edges; any edge without a `style` field takes the same path, but other diagrams are unverified.

Edge labels (`insertEdgeLabel`, `edges.js:74-135`): an empty `g.edgeLabel` with `g.label[data-id]` (84), a `createText` call with an empty label (88-101), and the measure (6.3). In SVG mode the label's `g > rect.background` is left in the `edgeLabels` group as a sibling (`createText.ts:160-161` creates it in the group the text is appended to, and only the text moves). The port should reproduce it if the DOM comparison covers it.

### 6.6 Markers, root svg, empty trees

- Markers (`markers.js:323-390`, `point`): `<id>_mindmap-pointEnd`, `-pointStart`, `-pointEnd-margin`, `-pointStart-margin`. Appended to the root `g`, not to `defs` (probed). No edge refers to them (no arrow types), so they are dead output that must still be there.
- Root `svg`: `class="mindmapDiagram"` (`setupViewPortForSVG.ts:12`); `width="100%"` and `style="max-width: <w>px;"` (`setupGraphViewbox.js:23-33`, `useMaxWidth` true); `viewBox="<x−p> <y−p> <w> <h>"` (38-40), where `w = bounds.width + 2p`, bounds from `getBBox()` of the svg, `p = mindmap.padding` (28-36). Probed: `class`, `width`, `style`, `viewBox`, plus `role="graphics-document document"` and `aria-roledescription="mindmap"` set by the caller, and `xmlns`.
- `insertLookDefs` appends two `defs` to the root svg, each with a `feDropShadow` filter `#<id>-drop-shadow` and `#<id>-drop-shadow-small` (`insertLookDefs.ts:32-47`). Probed: they come after the root `g`.
- Empty tree: no class, no style, no viewBox (section 2.4, 6.1 step 3).

### 6.7 DOM, in order (probed, forced SVG labels, redux-color)

```
svg#<id>.mindmapDiagram  width="100%" style="max-width: W px;" viewBox="…" role aria-roledescription xmlns
  style                                              (CSS: 6.8)
  g                                                  (root group, appended by `mermaidAPI.ts:419`)
    marker#<id>_mindmap-pointEnd.marker.mindmap      (4 markers, 6.6)
      path.arrowMarkerPath  style="stroke-width: 1; stroke-dasharray: 1, 0;"
    …
    g.subgraphs                                      (empty)
    g.edgePaths
      path#<id>-edge_<p>_<c>.edge-thickness-normal.edge-pattern-solid.edge.section-edge-N.edge-depth-K
           data-look="classic" data-id data-et="edge" data-edge="true" data-points style="undefined;;;undefined" d
    g.edgeLabels
      g.edgeLabel
        g.label transform="translate(0, 0)" data-id
          text text-anchor="middle" y="-10.1" > tspan.text-outer-tspan.row(x=0 y=-0.1em dy=1.1em)
      g                                              (orphan: rect.background, SVG mode only)
    g.nodes
      g#<id>-node_<n>.node.mindmap-node.section-<n>  transform="translate(x, y)" data-look="classic"
        rect.basic.label-container style="" width height x y rx ry      (rounded; or path/circle/rect)
        g.label transform="translate(-w/2, -h/2)" style=""
          rect                                       (empty, from labelHelper)
          g
            rect.background style="stroke: none"
            text style="" y="-10.1" > tspan.text-outer-tspan.row > tspan.text-inner-tspan
  defs (#<id>-drop-shadow)
  defs (#<id>-drop-shadow-small)
```

Node class strings: `node mindmap-node section-root section--1` and `node mindmap-node section-0` (probed: the DOM attribute has no trailing space). `getNodeClasses` (`util.ts:249-250`) builds a trailing space, so something removes it between the two; the cause is unverified, and a port should match the DOM value.

The HTML-label variant (default): `g.label` holds `foreignObject` (width and height set from the measured box), a `div` with `max-width`, `display: table-cell` or `table`, `white-space: nowrap` or `break-spaces`, and a `span.nodeLabel.markdown-node-label` (probed: `html-default.json`, 5 `foreignObject` for 3 nodes and 2 edges).

### 6.8 Style sheet (`styles.ts`)

`getStyles(options)` returns (after the global rules `styles.ts:104-143`):

- `.edge { stroke-width: 3; fill: none }` (104-106, 123-125).
- For each section `i` from 0 to 11 (`THEME_COLOR_LIMIT` = 12, `theme-base.js:18`), selectors `.section-<i-1>` with fill from `cScale<i>`, text from `cScaleLabel<i>`, `span` colour, `.node-icon-<i-1>` (40 px, unused by the DOM: no node gets that class), `.section-edge-<i-1>` stroke from `cScale<i>` (42-44), `.edge-depth-<i-1>` stroke width 17 − 3i (classic) or max(10 − 2(i−1), 2) (neo) (20, 45-47), and `.section-<i-1> line` (48-51, unused by mindmap DOM).
- `.disabled` rules (53-58), neo rules (59-70), `.section-root` rect, path, circle and polygon fill from `git0` (108-110), text from `gitBranchLabel0` (111-113), span colour (114-116).
- `.mindmap-node-label` (126-132).
- Neo (133-143): filters, stroke and fill, and the gradient (`genGradient`, 76-89) when `useGradient`.

`genSections` mutates its option copy: `lineColor<i>` is set from `cScaleInv<i>` and lightened or darkened (10-18). The copy is the spread in `styles.ts:38` (`{...options, svgId}`), so the theme is not changed. The inputs the sheet reads from the theme are `THEME_COLOR_LIMIT`, `cScale0..11`, `cScaleLabel0..11`, `cScaleInv0..11`, `git0`, `gitBranchLabel0`, `mainBkg`, `nodeBorder`, `strokeWidth`, `dropShadow`, `useGradient`, `look` and `theme`.

The geometry harness measures boxes, positions, text and path samples, not colours or style properties (the measure source in `diagram-geometry.ts` from line 117). The sheet therefore matters for the appearance, which only a visual check covers.

### 6.9 Configuration

- Diagram section `mindmap` (`MindmapDiagramConfig`, section 3.3): `padding` 10, `maxNodeWidth` 200, `colorBy` `branch`, `useMaxWidth` (base).
- Read from the top level: `layout` (section 3.5), `look` (`conf.look`, every node and edge `look` field), `theme` (shape choice, `getShapeFromType`, `mindmapDb.ts:249`), `htmlLabels` (labels: `evaluate(undefined)` is true, `config.ts:62-63`, so **HTML labels by default** in the reference), `flowchart.wrappingWidth` (120, section 6.3), `securityLevel` (`getDiagramElement`), `fontFamily` (style).
- Which layer wins for `layout` and `theme`: the directive, then `initialize()`, then the diagram default, then the top-level default (`config.ts:79-107`, `APPEARANCE_KEYS` line 12). The mindmap `layout` is then overridden (section 3.5).

### 6.10 Randomness and the clock

- Layout: the cose-bilkent start positions are random (`randomize: true`, `cytoscape-cose-bilkent.js:119`). `layout-base` has its own `RandomSeed` (`node_modules/.pnpm/layout-base@2.0.1/node_modules/layout-base/src/util/RandomSeed.js`); the cose-bilkent task covers which draws it makes. The harness replaces `Math.random` with a seeded function for the render (`common.ts`, `pageHtml` around 441-472) and fakes `Date` with the clock. Probed: two renders with the same clock and seed are byte-equal, and `layout: tidy-tree` equals the default render with the same seed.
- `uuid` (`diagramId`): unused output (section 3.5, rule 8).
- No other randomness or clock in the mindmap path (the mindmap code has no `Date` or `Math.random`; the shapes are deterministic).

### 6.11 DOM dependencies a server renderer must replace

- `getBBox()` of shapes and groups (6.3, 6.4), `getBoundingClientRect()` (HTML labels), `getComputedTextLength()` (SVG wraps), `getTotalLength()` (neo edges).
- `document` and `window` in cytoscape (`cytoscape-setup.ts:100-115`) and in DOMPurify (the sanitiser runs in the DB at parse time, `common.ts:76-82`).
- `btoa` for `data-points` (`edges.js:705`).
- The unified renderer already has `label.rs` (text layout), `markdown.rs` (Ferromark), `markers.rs` (`point`), `path.rs` (basis), `edges.rs`, `elk_*.rs`, and the shape registry (`shapes/registry.rs`: `rect`, `rounded`, `circle`, `hexagon`, `bang`, `cloud`). `mindmapCircle` needs an alias for the `circle` kind with the mindmap padding rule, and `defaultMindmapNode` is out of scope.

### 6.12 Layout engines the mindmap picks

- **cose-bilkent** (default): section 6.4 and 6.5.
- **ELK** (`layout: elk`): `render.ts` in ELK reads `edge.curve` through `interpolateToCurve` (`elk/render.ts:2745-2750`) and `edge.type` (2729-2731). The mindmap's edges are fine there. The reference **crashes for every mindmap** (probed, section 8, question 2): the stack reads `"x"` from the ELK node (elk chunk line 34434), and the mindmap records have `x: undefined` and `y: undefined` (`mindmapDb.ts:289-290`). A control run with those two keys removed from the dist's mindmap chunk lays out fine (probed: `probe-patched.mts`).
- **Tidy tree** (`layout: tidy-tree`): **not registered in this fork** (`rendering-util/render.ts:65-79` registers `swimlane`, `cose-bilkent` and the ELK names only). The lookup falls back to `cose-bilkent` with a warning (`render.ts:113-129`). Probed: the SVG for `layout: tidy-tree` is byte-equal to the default render (same seed). The removed package had its own `render.ts` (a copy of the cose-bilkent pattern, sizes `width ?? 100` and `height ?? 50` at lines 99-100). Its parents come from the edges (`layout.ts:91-103`: `parents.set(edge.end, edge.start)`). The hand-off to it is the same records and the same `rootNode`.

## 7. Fixtures

`e2e/diagrams/mindmap/` holds 28 `.mmd` files: 24 in `nightly-check/`, 4 in `pr-check/`, none in `visual-check/` (the folder has only `.gitkeep`).

The features they exercise:

- All seven shapes in one fixture, in six variants: `all-shapes-neo-look-<theme>` for the themes `neo`, `neo-dark`, `redux`, `redux-dark`, `redux-color` and `redux-dark-color`, each with `look: neo`.
- Icons: `::icon(...)` with `mdi` names (`all-shapes-with-mdi-fire-icon`, `bang-and-cloud-shape`, `bang-and-cloud-shape-with-icons`, `branches-with-shapes-and-labels`, `a-root-with-an-icon`, `text-should-wrap-with-icon`). They are stored and not drawn (section 8), so they exercise the label background only.
- Wrapping: long text on a root (`a-root-with-wrapping-text-*`, two fixtures), and a long word that exceeds the wrap width (`…long-words-that-exceed-width`).
- Multi-line shape text (`circle-shape`, `rounded-rect-shape`, `square-shape`).
- Colour sections: `level-2-nodes-exceeding-11` (the wrap at 11), `depth-colors` (`pr-check`).
- Labels by issue: `…-issue-8387` (centring of every shape), `formatted-label-with-…-emojis` (markdown and an emoji).

Harness (`tools/port/geometry/`):

- `diagram-geometry.ts` has no `mindmap` entry in `TYPE_FOLDERS` (lines 97-109). It needs `mindmap: ['mindmap']`.
- `exclusionReason` (`common.ts:197-217`) excludes a fixture with `look: handDrawn`, a non-layered layout (`cose-bilkent` is in `NON_LAYERED_LAYOUTS`, lines 160-169, so a fixture that sets `layout: cose-bilkent` would be excluded), or CJK. It does not exclude emoji, but `uncoveredScriptReason` does (`UNCOVERED_SYMBOLS`, lines 181-182, covers U+1F000 to U+1FAFF). That excludes **one** fixture: `formatted-label-with-linebreak-and-a-wrapping-label-and-emojis.mmd`.
- No fixture mentions `handDrawn` or `layout`, and none contains CJK characters (grep). So 27 fixtures are compared.
- The harness forces `htmlLabels: false` (`common.ts:59-63`), so the comparison is SVG-text labels on both sides, and the HTML path is covered only by the probes.
- The `theme` of the `neo` fixtures becomes `redux-color` for both sides (`forcedSiteConfig`, `common.ts:93-102`), as the harness does for any theme ferrigura does not ship.

## 8. Risks, effort, open questions

### Risks

1. **cose-bilkent fidelity** (other task, but the mindmap's whole geometry depends on it). The edge endpoints come from cytoscape's own default-size edge geometry (6.5): the engine port must reproduce them, not only the positions.
2. **Path bounding boxes.** Cloud and bang node sizes are the `getBBox()` of their arc paths (6.2, 6.3), and the group box is the engine's input (6.4). Chromium's extents for elliptical arcs (converted to curves, then extrema) must be matched, or every cloud and bang node is off. The harness checks the `getBBox` of each element, so this is visible.
3. **Text measurement under decision 0004.** The reference's default is HTML; the harness checks SVG text. Wrap widths and line breaks differ (6.3). The mindmap's wrap rules (`flowchart.wrappingWidth` 120 for most shapes, 200 for the others) must be reproduced.
4. **Markdown.** Labels are markdown (`mindmapDb.ts:276`). `marked` lexes `**bold**`, `*em*`, backtick code, `<br>` and newlines in the multi-line fixtures. `markdown.rs` has to match on these.
5. **The grammar.** The lexer is state-based and context-sensitive (`-)`, `(-`, `:::`, `::icon(`, quoted descriptions and the `%%` rule). The quirks in 2.4 are the tests. The Jison error text is not reproduced (section 4).
6. **Shared sanitising.** `sanitizeText` (DOMPurify) decides the ids, descriptions and icons. The rules needed here (`<br/>`, script removal, the `&` rule, entity placeholders) should come from the existing `ferrigura-syntax` sanitiser.

### Effort (rough)

- Parser and DB, the canonical model and the parser differential: one task, about 900 to 1,100 lines of Rust with the 49 spec runs ported.
- `getData`, the renderer glue, the four ported shapes (`rounded`, `rect`, `mindmapCircle`, `hexagon`) plus `cloud` and `bang`, the style sheet, the root svg and the DOM details: one task, about 800 to 1,000 lines.
- cose-bilkent and the tidy tree are separate tasks.

### Open questions for the coordinator

1. **Tidy tree.** P2-PLAN says `layout: tidy-tree` gives Mermaid's tidy tree, but the reference at HEAD renders it as cose-bilkent (probed, byte-equal output), because `2d85261d6` removed the package and nothing registers `tidy-tree`. Options: (a) match the reference now, `tidy-tree` falls back to cose-bilkent with a warning, and port the tidy tree later, when the reference has it again; (b) port the removed package (`git show 2d85261d6^:packages/mermaid-layout-tidy-tree/src/…`). With (b) the harness has no reference to compare against, so the tidy tree could only be checked against its own tests. Leaning: (a) for P2, with the tidy tree as a separate, later decision. Needs a decision either way.
2. **ELK crashes for every mindmap** in the reference (probed, all inputs, `layout: elk`). The cause is the `x` and `y` keys with `undefined` (`mindmapDb.ts:289-290`); removing them makes ELK render (probed). M6's rule names an exception where the reference crashes. Option: reproduce the crash, or name it as an exception and lay out the mindmap with ELK. Leaning: name it as an exception, because the crash is not a rendering rule. P2-PLAN's "as the reference does" needs the same decision.
3. **Icons are never drawn** (probed: no icon glyph and no `<i>` in either label mode; the DB stores them). The port should reproduce this, which means the icon fixtures render without icons, as the reference does. Confirm.
4. **Error texts.** Jison's parse error text and the engine's `TypeError` texts (icon or class before any node) are not reproduced. Proposal: the differential compares `ok` for every parse error, and the exact text only for the DB's own message (`There can be only one root…`). Confirm.
5. **Diagram-scoped layout is ignored** (`mindmap.layout`, and `updateSiteConfig({layout})`, probed). The rule is a quirk: reproduce it, and list it in the mindmap's `PLAN.md`. Confirm.
6. **Non-redux themes.** The default node shape `defaultMindmapNode` is used only when the theme is not a redux theme (`mindmapDb.ts:265`, probed). M6 decided not to port classic-theme branches, so this is out of scope; confirm, since the harness fallback makes the redux branch the one compared.
7. **Cross-task dependency.** The cose-bilkent port must return cytoscape's rscratch endpoints for default-size nodes (6.5). Tell the cose-bilkent task.
8. **`layout: swimlane` and the ELK names** on a mindmap. The layout name reaches the render unchanged: probed for `swimlane`, `dagre` and `elk.stress` (`probe-layout.mts`, the resolved name). The swimlane layout would then receive mindmap records, and `elk.stress` runs the ELK path, which crashes (question 2). Not compared by the harness; decide whether to reproduce or stop at the fallback.
9. **Untracked `docs-port/sankey/`** appeared in this worktree during the run (15:25). It is not from this task, and I left it alone.
