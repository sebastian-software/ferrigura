# CLASS diagram: research notes for M6

Research only, for the port of `classDiagram` to ferrigura (M6, P1). Nothing in the repository was changed. Reference: the JS tree at `04b952b4e` (the commit of `main` and of this worktree).

Conventions in this file: **[probe]** = checked by running the reference Jison grammar on raw input; **[source]** = read from the code, not run; **[unverified]** = neither could confirm it, or it needs a browser or the full pipeline. Line numbers are `file:line` in the reference unless a crate path is given.

Scratch material (not committed): `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/m6-class/`

- `gen-parser.mjs` writes `classParser.mjs`, the reference grammar generated with the repository's `jison` (same options as `.build/jisonTransformer.ts`).
- `probe-jison.mts` runs inputs through it and prints tokens and yy calls. Command: `tsx probe-jison.mts '["classDiagram\nclass A\nA <|-- B"]'`.
- `classify2.py`, `classify3.py`, `class-features.json`: fixture features and front-matter values.

Why no ClassDB run: the reference DB loads through the Vite SSR harness (`tools/port/reference.ts`), and in this worktree `packages/mermaid` has no `node_modules` (`d3` does not resolve). Running it from the main checkout would write Vite cache files under `node_modules`, which this task must not do. So the DB and getData behaviour below is from source reading. **[unverified at runtime]** wherever it matters.

---

## 0. Findings at a glance

| #   | Finding                                                                                                                                                                                                                                                         | Where    |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| 1   | The class renderer goes through the unified pipeline (`render` → ELK), as flowcharts do. Confirmed.                                                                                                                                                             | §5.1     |
| 2   | Renderer reads spacing, title margin and `useMaxWidth` from the **`state` section**, not `class`.                                                                                                                                                               | §5.8     |
| 3   | Runtime `config.class.padding` is **undefined**. The schema says 5, but `defaultConfig.ts` replaces the whole `class` object. The code falls back to 12 (classBox), 16 (namespace), 6 (note). The Rust config built from the schema would give 5.               | §6.1     |
| 4   | Class relations carry two cardinality labels as **terminal labels**. M5 has no terminal-label passes (ported as ignored tests). They are the largest renderer gap after `classBox`.                                                                             | §5.5     |
| 5   | `classBox` geometry depends on DOM `getBBox()` of groups that are translated after creation, and has a quirk: an empty members group subtracts `P/2` (−6 px) from several heights.                                                                              | §5.3     |
| 6   | Lexer: `.*direction\s+(TB\|BT\|LR\|RL)` swallows the whole line, including an edge written before it. `%%` mid-line swallows the newline (parse error). `&lt;&lt;interface&gt;&gt;` in a body is a member, not an annotation (98 fixtures use the entity form). | §2.6     |
| 7   | Markers: 5 kinds × 4 variants (start/end, plain/margin). M5 has point, circle, cross only, with `DIAGRAM_TYPE` fixed to `flowchart-v2`.                                                                                                                         | §5.4     |
| 8   | Default look for classes is **neo** (class section), not classic, although the top-level default is classic. Fixture descriptions that say "classic look" do not set `look`, so they render neo.                                                                | §6.2, §7 |
| 9   | ELK already lays out class graphs: 11 class graphs are in the M4 corpus, with TAIL/HEAD label placements.                                                                                                                                                       | §5.5     |
| 10  | Test count in STATUS (≈255) is close to the number of test blocks (≈246, each `it.each` counted once), so it looks like a block count. The vitest count is about **492** active cases (383 in `classDiagram.spec.ts`, 1 skipped).                               | §4       |

---

## 1. Sources

| Role                                          | File                                                                         | Lines | Note                                                                                                                                                                           |
| --------------------------------------------- | ---------------------------------------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Grammar + lexer                               | `packages/mermaid/src/diagrams/class/parser/classDiagram.jison`              | 440   | Jison, `%lex` 8–211, grammar 221–438                                                                                                                                           |
| Parser wrapper / registration                 | `classDiagram-v2.ts`                                                         | 1–21  | parser import L3, db L10–12, renderer L13, styles L14, `init` L15–20 (sets `class.arrowMarkerAbsolute`)                                                                        |
| Detector                                      | `classDetector-V2.ts`                                                        | 7–23  | `/^\s*classDiagram/` (L10); loader imports `classDiagram-v2.js`                                                                                                                |
| Canonical DB                                  | `classDb.ts`                                                                 | 874   | class map L39, `getData` L738–865                                                                                                                                              |
| Member / node / relation types                | `classTypes.ts`                                                              | 202   | `ClassMember` L31–146                                                                                                                                                          |
| Renderer                                      | `classRenderer-v3-unified.ts`                                                | 82    | `draw` L41–76                                                                                                                                                                  |
| Text helper + row text                        | `shapeUtil.ts`                                                               | 189   | `textHelper` L15–96, `addText` L99–189                                                                                                                                         |
| Shape                                         | `rendering-util/rendering-elements/shapes/classBox.ts`                       | 276   | registered in `shapes.ts` L77 (import), L568                                                                                                                                   |
| Note shape                                    | `rendering-util/rendering-elements/shapes/note.ts`                           | 67    | registered `shapes.ts` L53, L542                                                                                                                                               |
| Stylesheet                                    | `styles.js`                                                                  | 244   | `genColor` L14–38, `getStyles` L40–242                                                                                                                                         |
| Markers                                       | `rendering-util/rendering-elements/markers.js`                               | 1022  | class kinds: extension L12, composition L76, aggregation L136, dependency L195, lollipop L253                                                                                  |
| Marker selection                              | `rendering-util/rendering-elements/edgeMarker.ts`                            | 139   | map L39–57, margin list L59–69, id L99–100                                                                                                                                     |
| Edge and terminal labels                      | `rendering-util/rendering-elements/edges.js`                                 | 1091  | `insertEdgeLabel` L74–253, `positionEdgeLabel` L301–374, `insertEdge` marker call L946–947                                                                                     |
| ELK terminal labels                           | `rendering-util/layout-algorithms/elk/render.ts`                             | 2938+ | `terminalElkLabels` L1144–1173, passes L2085–2364 (see §5.5)                                                                                                                   |
| Terminal positions                            | `rendering-util/layout-algorithms/common/index.ts`                           | —     | `TERMINAL_LABEL_SIDES` L353–361, `terminalLabelTranslate` L364–378                                                                                                             |
| Shared text                                   | `diagrams/common/common.ts`                                                  | —     | `removeScript` L48, `sanitizeMore` L56, `sanitizeText` L71, `parseGenericTypes` L201, `processSet` L242                                                                        |
| Shared accessibility                          | `diagrams/common/commonDb.ts`                                                | 32    | `setAccTitle` L16, `setAccDescription` L22                                                                                                                                     |
| Colour slots                                  | `diagrams/common/colorThemeGate.ts`                                          | —     | `COLOR_THEMES` L17, `hasPalette` L33, `paletteSlotCount` L83, `stampColorSlot` L114–126                                                                                        |
| Node helpers                                  | `rendering-util/rendering-elements/shapes/util.ts`                           | —     | `labelHelper` L50, `updateNodeBounds` L220–247, `getNodeClasses` L249                                                                                                          |
| Hand-drawn style helpers                      | `shapes/handDrawnShapeStyles.ts`                                             | —     | `compileStyles` L31, `styles2String` L75, `userNodeOverrides` L108                                                                                                             |
| Node wrapper (links, tooltip, callback class) | `rendering-util/rendering-elements/nodes.ts`                                 | —     | `<a>` wrap and `title` L40–62, `clickable` L63–65                                                                                                                              |
| Text box                                      | `rendering-util/createText.ts`                                               | —     | `createText` L306 (options: `markdown` default true L314, `width` default 200 L319, `minWidth` L324)                                                                           |
| Viewport and title                            | `rendering-util/setupViewPortForSVG.ts` L5–40; `utils.ts` `insertTitle` L793 | —     |                                                                                                                                                                                |
| Text width helpers                            | `utils.ts`                                                                   | —     | `calculateTextWidth` L649, `calculateTextDimensions` L665 (DOM-based), `decodeEntities` L904, `getEdgeId` L908, `formatUrl` L250                                               |
| Preprocessing                                 | `preprocess.ts`                                                              | —     | `cleanupText` L6–17, `processDirectives` L33–45, `preprocessDiagram` L58–65; `diagram-api/comments.ts` L6–8                                                                    |
| Diagram entry                                 | `Diagram.ts`                                                                 | —     | front matter title → `setDiagramTitle` L52–53                                                                                                                                  |
| Config                                        | `defaultConfig.ts`                                                           | —     | class section L73–83                                                                                                                                                           |
| Config schema                                 | `schemas/config.schema.yaml`                                                 | —     | top `look` default `classic` L68–70; `layout` default `elk` L76–82; `StateDiagramConfig` L1641–1709 (`titleTopMargin` default 25 about L1681); `ClassDiagramConfig` L1710–1756 |
| Config keys                                   | `diagram-api/diagramConfigKeys.ts`                                           | 9     | `classDiagram → class`                                                                                                                                                         |
| Appearance                                    | `config.ts`                                                                  | —     | `LOOKS` L19, `isUsableAppearance` L27, `readAppearance` L42, `setDiagramConfigScope` L149–152                                                                                  |
| Renderer for state (for comparison)           | `diagrams/state/stateRenderer-v3-unified.ts`                                 | —     | same `state` section reads, L138–141                                                                                                                                           |

Fixtures: `e2e/diagrams/class-diagram/` (247 `.mmd`, see §7). Specs: `packages/mermaid/src/diagrams/class/` (5 files), plus cross-cutting specs in §4.2.

---

## 2. Parser

### 2.1 Before the lexer

Order used by `mermaidAPI`/`Diagram` for every diagram, [source]:

```
preprocessDiagram(text)                         preprocess.ts:58-65
  cleanupText: CRLF/CR -> LF; <tag a="x"> -> a='x'      :6-17
  extractFrontMatter (title, config)           processFrontmatter
  processDirectives: %%{...}%% read, then removed       :33-45 (utils.removeDirectives, utils.ts:203)
  cleanupComments: remove lines /^\s*%%(?!{)[^\n]+\n?/gm, then trimStart   comments.ts:6-8
parser.parse(code.cleaned)                      grammar sees this text
```

Consequences:

- Whole-line `%%` comments never reach the lexer. Mid-line `%%` does (see §2.6, probe 3).
- Directives never reach the lexer (`%%{init}%%` is removed; the `%%(?!\{)` in the grammar is only a safety net).
- Front matter never reaches the lexer. Its `title` goes to `db.setDiagramTitle` (`Diagram.ts:52-53`).

### 2.2 Lexer states (`%x`, classDiagram.jison L9–22)

| State                 | Entered by                                  | Left by                             | What it matches                                            | Lines   |
| --------------------- | ------------------------------------------- | ----------------------------------- | ---------------------------------------------------------- | ------- |
| INITIAL               | default                                     | —                                   | rules without a state prefix, and `<*>` rules              | —       |
| `string`              | `<*>["]` (L62)                              | `<string>["]` (L60)                 | `STR` = `[^"]*`                                            | 60–62   |
| `bqstring`            | `<*>[`]` (L112)                             | `<bqstring>[`]` (L110)              | `BQUOTE_STR` = `[^`]+` (class names in backticks)          | 110–112 |
| `generic`             | `<*>"~"` (L108)                             | `<generic>[~]` (L106)               | `GENERICTYPE` = `[^~]*`                                    | 106–108 |
| `acc_title`           | `accTitle\s*":"\s*` (L30)                   | after value (L31)                   | `acc_title_value` to end of line                           | 30–31   |
| `acc_descr`           | `accDescr\s*":"\s*` (L32)                   | after value (L33)                   | `acc_descr_value` to end of line                           | 32–33   |
| `acc_descr_multiline` | `accDescr\s*"{"\s*` (L34)                   | `}` (L35)                           | `[^}]*` (may span lines)                                   | 34–36   |
| `callback_name`       | `<INITIAL>"call"[\s]+` (L53)                | `(` (L54–55)                        | `CALLBACK_NAME` = `[^(]*`                                  | 53–56   |
| `callback_args`       | `(` in callback_name (L55)                  | `)` (L57)                           | `CALLBACK_ARGS` = `[^)]*`                                  | 55–58   |
| `namespace`           | `<INITIAL,namespace-body>"namespace"` (L66) | newline (L67) or `}` (L70)          | `namespace` keyword, `{` (L69)                             | 66–70   |
| `namespace-body`      | `{` in `namespace` (L69)                    | `}` (L71)                           | `class` (L77), `[*]`, newline, `EOF_IN_STRUCT` (L72)       | 71–75   |
| `class`               | `<INITIAL,namespace-body>"class"` (L77)     | newline (L78); `}` pops twice (L80) | `{` → `class-body` (L81)                                   | 77–81   |
| `class-body`          | `{` in `class` (L81)                        | `}` (L82)                           | `MEMBER` = `[^{}\n]*` (L87), `OPEN_IN_STRUCT` on `{` (L85) | 81–87   |
| `struct`, `href`      | —                                           | —                                   | declared (L12–13), **never entered**                       | —       |

After a class body's `}` the lexer is back in `class`; only a newline (L78) pops it. [source; not probed.]

### 2.3 Lexer rules, in definition order (first match wins)

Jison takes the first rule that matches, not the longest (the flowchart note, `flowchart/PLAN.md` §3.2, applies here too). Literal keywords get an implicit word boundary, which **[probe]** shows: `class order` gives `ALPHA("order")`, so `\s*o` (L124) does not split it.

| Lines   | Token                                                                                                                        | Pattern                                | Notes                                                                                                       |
| ------- | ---------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| 24      | `direction_tb`                                                                                                               | `.*direction\s+TB[^\n]*`               | matches from the current position, so it eats the text before it on the line **[probe 2]**                  |
| 25–27   | `direction_bt/rl/lr`                                                                                                         | same with BT, RL, LR                   | same behaviour                                                                                              |
| 28      | (skip)                                                                                                                       | `\%\%(?!\{)*[^\n]*(\r?\n?)+`           | `(?!\{)*` quantifies a lookahead, so it is `%%` followed by text; it **consumes the newline** **[probe 3]** |
| 29      | (skip)                                                                                                                       | `\%\%[^\n]*(\r?\n)*`                   | second comment rule, also consumes newlines                                                                 |
| 30–36   | `acc_title`, `acc_descr`, `acc_descr_multiline`                                                                              | see §2.2                               |                                                                                                             |
| 38      | `NEWLINE`                                                                                                                    | `\s*(\r?\n)+`                          | leading blanks swallowed                                                                                    |
| 39      | (skip)                                                                                                                       | `\s+`                                  |                                                                                                             |
| 41–42   | `CLASS_DIAGRAM`                                                                                                              | `"classDiagram-v2"`, `"classDiagram"`  | both accepted [probe]                                                                                       |
| 43      | `EDGE_STATE`                                                                                                                 | `"[*]"`                                | not used by any production → error [probe]                                                                  |
| 53–58   | `call` / callback tokens                                                                                                     | see §2.2                               |                                                                                                             |
| 60–62   | `STR`                                                                                                                        | `"` starts `string`; `<*>["]`          |                                                                                                             |
| 63–64   | `STYLE`, `CLASSDEF`                                                                                                          | `"style"`, `"classDef"`                | INITIAL only                                                                                                |
| 66–75   | `NAMESPACE`, `STRUCT_START`, `STRUCT_STOP`                                                                                   | namespace states                       |                                                                                                             |
| 77–87   | `CLASS`, `STRUCT_START`, `STRUCT_STOP`, `MEMBER`, `OPEN_IN_STRUCT`, `EDGE_STATE`                                             | class states                           |                                                                                                             |
| 89–96   | `CSSCLASS`, `CALLBACK`, `LINK`, `CLICK`, `NOTE_FOR` (`note for`), `NOTE`, `ANNOTATION_START` (`<<`), `ANNOTATION_END` (`>>`) | `<*>`                                  | keywords work in every state                                                                                |
| 104     | `HREF`                                                                                                                       | `<*>"href"`                            | does not change state                                                                                       |
| 106–112 | `GENERICTYPE`, `BQUOTE_STR`                                                                                                  | see §2.2                               |                                                                                                             |
| 114–117 | `LINK_TARGET`                                                                                                                | `_self`, `_blank`, `_parent`, `_top`   | `<*>`                                                                                                       |
| 119–120 | `EXTENSION`                                                                                                                  | `\s*\<\|`, `\s*\|\>`                   | `<\|` and `\|>`                                                                                             |
| 121–122 | `DEPENDENCY`                                                                                                                 | `\s*\>`, `\s*\<`                       |                                                                                                             |
| 123     | `COMPOSITION`                                                                                                                | `\s*\*`                                |                                                                                                             |
| 124     | `AGGREGATION`                                                                                                                | `\s*o` (implicit `\b`)                 | a standalone `o` only **[probe 3b: `class order` is ALPHA]**                                                |
| 125     | `LOLLIPOP`                                                                                                                   | `\s*\(\)`                              | `()`                                                                                                        |
| 126     | `LINE`                                                                                                                       | `\-\-`                                 |                                                                                                             |
| 127     | `DOTTED_LINE`                                                                                                                | `\.\.`                                 |                                                                                                             |
| 128     | `LABEL`                                                                                                                      | `":"{1}[^:\n;]+`                       | includes the colon and any quotes **[probe]**                                                               |
| 129     | `STYLE_SEPARATOR`                                                                                                            | `":"{3}`                               | `:::`                                                                                                       |
| 130–139 | `MINUS`, `DOT`, `PLUS`, `COLON`, `COMMA`, `BRKT`, `PCT`, `EQUALS`                                                            | single characters                      |                                                                                                             |
| 140     | `ALPHA`                                                                                                                      | `\w+`                                  | covers digits too                                                                                           |
| 141–142 | `SQS`, `SQE`                                                                                                                 | `[`, `]`                               |                                                                                                             |
| 143     | `PUNCTUATION`                                                                                                                | `[!"#$%&'*+,-.`?\\/]`                  |                                                                                                             |
| 144     | `NUM`                                                                                                                        | `[0-9]+`                               | **dead**: `\w+` (L140) matches first **[probe: `class 1B` is ALPHA]**                                       |
| 145–206 | `UNICODE_TEXT`                                                                                                               | explicit ranges (letters beyond ASCII) | `Ø` → UNICODE_TEXT **[probe]**                                                                              |
| 207–208 | `SPACE`                                                                                                                      | `\s`                                   |                                                                                                             |
| 209     | `EOF`                                                                                                                        | `<<EOF>>`                              |                                                                                                             |

### 2.4 Grammar, with the DB calls each production makes

| Production (lines)              | Shape                                                                                                                                  | DB call on reduce                                                                    |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `start` (221–224)               | `mermaidDoc \| statements`                                                                                                             | —                                                                                    |
| `graphConfig` (230–232)         | `CLASS_DIAGRAM NEWLINE statements EOF`                                                                                                 | a newline after the header is required; `classDiagram` alone is an error **[probe]** |
| `statements` (234–238)          | statement, NEWLINE-separated                                                                                                           | —                                                                                    |
| `statement` (260–276)           | relation, relation + `LABEL`, namespace, class, member, annotation, click, style, cssClass, note, classDef, direction, acc title/descr | per alternative (below)                                                              |
| relation + LABEL (262)          | `$1.title = cleanupLabel($2)` then `addRelation`                                                                                       | `cleanupLabel`, `addRelation`                                                        |
| `namespaceStatement` (278–281)  | `namespaceIdentifier STRUCT_START classStatements STRUCT_STOP`                                                                         | `addClassesToNamespace`, `popNamespace`                                              |
| `namespaceIdentifier` (283–286) | `NAMESPACE namespaceName [classLabel]`                                                                                                 | `addNamespace(name[, label])`                                                        |
| `classStatement` (300–309)      | id; `:::` css; body; `<<ann>>`; combinations                                                                                           | `addClass`, `setCssClass`, `addMembers`, `addAnnotation`                             |
| `classIdentifier` (311–314)     | `CLASS className [classLabel]`                                                                                                         | `addClass`, `setClassLabel`                                                          |
| `annotationStatement` (323–325) | `<<alpha>> className`                                                                                                                  | `addAnnotation(className, alpha)`                                                    |
| `members` (327–330)             | list of MEMBER; reversed order                                                                                                         | —                                                                                    |
| `memberStatement` (332–337)     | `className`; `className LABEL`; `MEMBER`; `SEPARATOR` (dead)                                                                           | `addMember(className, cleanupLabel)` for `LABEL`                                     |
| `relationStatement` (339–344)   | `className [STR] relation [STR] className`                                                                                             | returned to `statement`                                                              |
| `relation` (371–376)            | `relationType lineType relationType`, with either side optional                                                                        | type objects                                                                         |
| `noteStatement` (346–349)       | `NOTE_FOR className noteText`, `NOTE noteText`                                                                                         | `addNote(text, className)`                                                           |
| `classDefStatement` (351–353)   | `CLASSDEF classList stylesOpt`                                                                                                         | `defineClass(ids, styles)`                                                           |
| `direction` (360–369)           | `direction_tb` etc.                                                                                                                    | `setDirection('TB'/'BT'/'RL'/'LR')`                                                  |
| `clickStatement` (391–406)      | 16 forms of CALLBACK/LINK/CLICK                                                                                                        | `setClickEvent`, `setLink`, `setTooltip`                                             |
| `styleStatement` (408–410)      | `STYLE ALPHA stylesOpt`                                                                                                                | `setCssStyle(id, styles)`                                                            |
| `cssClassStatement` (412–414)   | `CSSCLASS STR ALPHA`                                                                                                                   | `setCssClass(ids, cls)`                                                              |
| acc title / descr (273–275)     | `acc_title acc_title_value` etc.                                                                                                       | `setAccTitle`, `setAccDescription` (value `.trim()` first)                           |

Relation shape from the grammar (L340–343, L371–376) and `getArrowMarker` (classDb L694–716):

| relationType token         | value | marker name   |
| -------------------------- | ----- | ------------- |
| `AGGREGATION` (`o`)        | 0     | `aggregation` |
| `EXTENSION` (`<\|`, `\|>`) | 1     | `extension`   |
| `COMPOSITION` (`*`)        | 2     | `composition` |
| `DEPENDENCY` (`<`, `>`)    | 3     | `dependency`  |
| `LOLLIPOP` (`()`)          | 4     | `lollipop`    |
| none                       | —     | `none`        |

`lineType`: `LINE` (`--`) = 0, `DOTTED_LINE` (`..`) = 1 (classDb L500–503). `relation.type1` is the marker at the start, `type2` at the end. Note the start/end naming: `A <|-- B` has EXTENSION on the **left** (type1).

### 2.5 Dead or unreachable grammar [source + grep]

| Item                                            | Where                 | Effect                                                                  |
| ----------------------------------------------- | --------------------- | ----------------------------------------------------------------------- |
| `NUM` lexer rule                                | L144                  | never produced (L140 `\w+` first)                                       |
| `SEPARATOR`                                     | grammar L336          | no lexer rule produces it                                               |
| `UNIT`                                          | `styleComponent` L426 | no lexer rule                                                           |
| `TAGSTART`, `TAGEND`, `DEFAULT`, `'=='`, `'--'` | `textToken` L430      | only used by unused `commentToken` (L428)                               |
| `emptyBody`                                     | L317–321              | unused                                                                  |
| `commentToken`, `textToken`, `textNoTagsToken`  | L428–432              | unused                                                                  |
| `EDGE_STATE`                                    | L43, 75, 84           | produced, but no production uses it → any `[*]` is an error **[probe]** |
| `OPEN_IN_STRUCT`, `EOF_IN_STRUCT`               | L83, 85               | produced, not in the grammar → error **[probe]**                        |
| states `struct`, `href`                         | L12–13                | never entered                                                           |

### 2.6 Probe results (raw input through the reference grammar)

| #   | Input (after `\n` escapes)                                                                                      | Result                         | Evidence                                                                                                                                                                                          |
| --- | --------------------------------------------------------------------------------------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `classDiagram\nclass A\nA <\|-- B`                                                                              | ok                             | `addClass(A)`, `addRelation` with `type1:1`                                                                                                                                                       |
| 2   | `classDiagram\nA --> B direction TB\nC-->D`                                                                     | ok, **edge A-->B lost**        | token `direction_tb("A --> B direction TB")`, calls: `setDirection("TB")`, then only `C-->D`                                                                                                      |
| 3   | `classDiagram\nclass A\n%% comment\nA -- B %% mid\nnamespace N {…}`                                             | **parse error line 4**         | the mid-line `%%` rule ate the newline; `A -- B` never ends. Full pipeline also leaves mid-line `%%` (only whole lines are cut), so the error stands **[probe on raw text; pipeline unverified]** |
| 3b  | `classDiagram\nclass order\norder --> item`                                                                     | ok                             | `order` is ALPHA (implicit `\b` on `o`)                                                                                                                                                           |
| 4   | `classDiagram\nclass 1B`                                                                                        | ok                             | `1B` is ALPHA; NUM dead                                                                                                                                                                           |
| 5   | `classDiagram\nclass Foo\nFoo --> Ø`                                                                            | ok                             | `Ø` is UNICODE_TEXT                                                                                                                                                                               |
| 6   | `classDiagram\ntitle My Title\nclass A`                                                                         | ok, **no DB call for `title`** | tokens ALPHA, parsed as a class-name expression with no action. Title is front matter only                                                                                                        |
| 7   | `classDiagram` alone                                                                                            | parse error                    | header needs NEWLINE (L231)                                                                                                                                                                       |
| 8   | `classDiagram\n[*] --> A`                                                                                       | parse error                    | EDGE_STATE in no production                                                                                                                                                                       |
| 9   | `classDiagram\nclass A {\n +x {\n}`                                                                             | parse error                    | OPEN_IN_STRUCT                                                                                                                                                                                    |
| 10  | `classDiagram\nclass A {\n +x\n`                                                                                | parse error                    | EOF_IN_STRUCT                                                                                                                                                                                     |
| 11  | `classDiagram\nclass A\ndirection LR\nnamespace N {\n direction TB\n class B\n}`                                | parse error                    | `direction` inside a namespace body is ALPHA (only INITIAL has the direction rules)                                                                                                               |
| 12  | `classDiagram\nclass A {\n direction TB\n +x\n}`                                                                | ok, **member** `direction TB`  | in `class-body` the text is MEMBER                                                                                                                                                                |
| 13  | `classDiagram\nclass Class01 {\n &lt;&lt;interface&gt;&gt;\n +f()\n}`                                           | ok, **member**                 | `addMembers(...)`; not an annotation. 98 fixtures use this form                                                                                                                                   |
| 14  | `classDiagram\nclass Animal["Animal with a label"]`                                                             | ok                             | `setClassLabel("Animal", "Animal with a label")`                                                                                                                                                  |
| 15  | `classDiagram\nclass A\nA <\|-- B : "quoted"`                                                                   | ok, label keeps the quotes     | `title: "\"quoted\""`                                                                                                                                                                             |
| 16  | `classDiagram\nclass Shape~T~ {…}`                                                                              | ok                             | `addClass("Shape~T~")`, GENERICTYPE                                                                                                                                                               |
| 17  | `classDiagram\nclass A\nA : +String name\nA : -int age$\nA : +run() void*`                                      | ok                             | `cleanupLabel` then `addMember`                                                                                                                                                                   |
| 18  | `classDiagram\nclass A {\n }\nclass B {}\nclass C{ +x }\nclass D <<interface>>\n<<abstract>> E`                 | ok                             | `addAnnotation(D, interface)`, `addAnnotation(E, abstract)`                                                                                                                                       |
| 19  | `classDiagram\naccTitle: My title  \naccDescr: a desc`                                                          | ok                             | `setAccTitle("My title")`, `setAccDescription("a desc")`                                                                                                                                          |
| 20  | `classDiagram\naccDescr {\n  line one\n  line two\n}`                                                           | ok                             | `setAccDescription("line one\n  line two")`, then commonDb makes it `line one\nline two` (`commonDb.ts:23`)                                                                                       |
| 21  | `classDiagram\nnote "hello"\nnote for A "n"`                                                                    | ok                             | `addNote("hello", undefined)`, `addNote("n", "A")`                                                                                                                                                |
| 22  | `classDiagram\nclass A\ncssClass "A" foo\nclass B:::bar\nstyle A fill:#f00,stroke:#333\nclassDef bar fill:#f9f` | ok                             | `setCssClass(A, foo)`, `setCssClass(B, bar)`, `setCssStyle("A", ["fill:#f00,stroke:#333"])`, `defineClass(["bar"], ["fill:#f9f"])`                                                                |
| 23  | `click A call f()`, `click A href "https://x" _blank`, `link A "https://y" "tip"`, `callback A "f"`             | ok                             | `setClickEvent(A, f)`, `setLink(A, url, _blank)`, `setLink(A, url)` + `setTooltip(A, tip)`, `setClickEvent(A, f)`                                                                                 |
| 24  | `classDiagram\nclass A\nA : +int id\nclass A {\n<<interface>>\n+foo()$\n-bar\n}`                                | ok                             | `<<interface>>` in a body is an annotation (`MEMBER` starting `<<`)                                                                                                                               |
| 25  | `namespace A.B {…}`, `namespace N { class Y }`                                                                  | ok                             | `addNamespace("A.B")`, `addClass(X)`, `addClassesToNamespace`, `popNamespace`                                                                                                                     |
| 26  | `classDiagram-v2\nclass A`                                                                                      | ok                             |                                                                                                                                                                                                   |
| 27  | `classDiagram\n\n\n  class A  \n\n`                                                                             | ok                             | blank lines absorbed                                                                                                                                                                              |
| 28  | `A "1" -- "0..*" B : has`                                                                                       | ok                             | `relationTitle1: "1"`, `relationTitle2: "0..*"`, `title: "has"`                                                                                                                                   |
| 29  | `Animal "1" --> "*" Car : owns`                                                                                 | ok                             |                                                                                                                                                                                                   |

Note 2 is the important one for the port: the direction rule can silently drop a relation. Port it as the same token rule, not as a "direction statement".

### 2.7 Error behaviour

- Parse errors are Jison's (`Parse error on line N: …`). The specs assert `Expecting 'STR'` or `Unrecognized text` (`classDiagram.spec.ts:688`, the 50 keywords after `note`). ADR 0002 turns these into "fails, at which position" in Rust; the message texts are not copied.
- The DB can throw where a class is not present: `classes.get(x)!` in `addAnnotation`, `addMember` (after `addClass`, so present) and `setClassLabel` (L101–103). The grammar always calls `addClass` first, so these are not reachable from text **[source]**.
- `lookUpDomId` throws `Class not found` (L170). Only reachable from callbacks, which do nothing under `securityLevel` strict (§3.4).
- A `getNote(name)` for an unknown name returns `undefined` (L202); `addClassesToNamespace` guards only the namespace (L658).

### 2.8 Comments, directives, front matter, header, title, accessibility

| Feature         | Behaviour                                                                 | Where                         |
| --------------- | ------------------------------------------------------------------------- | ----------------------------- |
| Whole-line `%%` | removed before lexing                                                     | `comments.ts:6-8`             |
| Mid-line `%%`   | lexer rule; eats newline → usually a parse error                          | grammar L28–29; probe 3       |
| `%%{…}%%`       | removed before lexing, read into the config                               | `preprocess.ts:33-45`         |
| Front matter    | removed; `title` → `setDiagramTitle`; `config` → configuration            | `Diagram.ts:52-53`            |
| Header          | `classDiagram` or `classDiagram-v2`, then NEWLINE                         | L41–42, L231                  |
| Title statement | none; `title x` is a silent no-op                                         | probe 6                       |
| `accTitle:`     | value trimmed, then `setAccTitle` (sanitise, drop leading whitespace)     | grammar L273; commonDb L16–18 |
| `accDescr:`     | single line, trimmed, then `setAccDescription` (sanitise, `\n\s+` → `\n`) | grammar L274; commonDb L22–24 |
| `accDescr { }`  | value trimmed; same setter                                                | grammar L275; commonDb L22–24 |

### 2.9 What each DB call does to the state (`classDb.ts`)

| Method                                                | Effect on the state                                                                                                                                                                                                                                                                 | Lines                              |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| `addClass(id)`                                        | split `~T~` into name and type; if the name exists, return; else create `{id, type, label: name, text: name + (type ? '&lt;'+type+'&gt;' : ''), shape: 'classBox', cssClasses: 'default', methods: [], members: [], annotations: [], styles: [], domId}`; `classCounter++` (global) | 112–137 (text L126, domId L133)    |
| `setClassLabel(id, label)`                            | `label` = sanitised; `text` = `label` + (type ? `<type>` with raw `<`)                                                                                                                                                                                                              | 94–104 (text L102–103)             |
| `addRelation(rel)`                                    | if `type1` is LOLLIPOP and `type2` is not a marker type: `addClass(id2)`, `addInterface(id1, id2)`, `id1 = interface<n-1>`; symmetric for `type2`; else `addClass` both ends; split generics; `relationTitle1/2` = sanitised trimmed; push                                          | 209–253 (lollipop L220–233)        |
| `addMember(cls, member)`                              | `addClass(cls)`; trimmed; `<<x>>` → annotation (sanitised); has `)` at index > 0 → method; else non-empty → attribute                                                                                                                                                               | 277–297 (method test L290)         |
| `addMembers(cls, list)`                               | **reverses** the list, then `addMember` each (the grammar built it reversed)                                                                                                                                                                                                        | 299–304                            |
| `addAnnotation(cls, a)`                               | push raw                                                                                                                                                                                                                                                                            | 263–266                            |
| `addNote(text, cls)`                                  | id `note<size>`; **text not sanitised**; `class` raw                                                                                                                                                                                                                                | 306–316                            |
| `cleanupLabel(l)`                                     | drop one leading `:`, trim, sanitise                                                                                                                                                                                                                                                | 318–323                            |
| `setCssClass(ids, c)`                                 | comma split; an id starting with a digit gets `classId-`; `cssClasses += ' ' + c`                                                                                                                                                                                                   | 331–343                            |
| `defineClass(ids, styles)`                            | style registry; `color` declarations also go to `textStyles` with `fill` → `bgFill`; every class whose `cssClasses` contains the id gets the styles appended                                                                                                                        | 345–369                            |
| `setCssStyle(id, styles)`                             | a class's `styles` gets each declaration, split at commas                                                                                                                                                                                                                           | 674–686                            |
| `setTooltip(ids, tip)`                                | `tooltip` = sanitised                                                                                                                                                                                                                                                               | 377–387                            |
| `setLink(ids, url, target)`                           | `link` = `formatUrl` (trim; `sanitizeUrl` unless `loose`; empty → undefined); `linkTarget`: sandbox → `_top`, else sanitised target, default `_blank`; then `setCssClass(ids, 'clickable')`                                                                                         | 404–425 (formatUrl `utils.ts:250`) |
| `setClickEvent(ids, fn, args)`                        | `setClickFunc` does nothing unless `securityLevel === 'loose'`; `haveCallback = true`; `clickable` class                                                                                                                                                                            | 434–449                            |
| `setDirection(d)`                                     | last one wins                                                                                                                                                                                                                                                                       | 545–551                            |
| `addNamespace(id, label)`                             | qualified with the stack top; push; create missing ancestors (`explicit` only on the leaf); link parent/child                                                                                                                                                                       | 598–635                            |
| `popNamespace`                                        | pop                                                                                                                                                                                                                                                                                 | 637–639                            |
| `addClassesToNamespace(ns, classes, notes)`           | class `parent = ns`; namespace gets the class; notes likewise                                                                                                                                                                                                                       | 657–672                            |
| `setAccTitle`, `setAccDescription`, `setDiagramTitle` | commonDb                                                                                                                                                                                                                                                                            | commonDb 16–30                     |
| `getData()`                                           | §5.2                                                                                                                                                                                                                                                                                | 738–865                            |

Module-level state: `classCounter` (L33) is not reset by `clear()`, so `domId`s differ between runs. It is a rendering detail, see §3.4.

---

## 3. Canonical model proposal

### 3.1 Shape (camelCase, `version: 1`)

```json
{
  "version": 1,
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "direction": "TB",
  "classes": [
    {
      "id": "Animal",
      "type": "",
      "label": "Animal",
      "text": "Animal",
      "cssClasses": "default",
      "annotations": [],
      "members": [
        {
          "memberType": "attribute",
          "visibility": "+",
          "classifier": "",
          "id": "name",
          "parameters": "",
          "returnType": "",
          "text": "\\+name"
        }
      ],
      "methods": [
        {
          "memberType": "method",
          "visibility": "+",
          "classifier": "$",
          "id": "mate",
          "parameters": "a, b",
          "returnType": "void",
          "text": "\\+mate(a, b) : void"
        }
      ],
      "styles": [],
      "link": null,
      "linkTarget": null,
      "tooltip": null,
      "parent": null
    }
  ],
  "relations": [
    {
      "id1": "Animal",
      "id2": "Car",
      "title": "owns",
      "relationTitle1": "1",
      "relationTitle2": "*",
      "type1": "none",
      "type2": "dependency",
      "lineType": "solid"
    }
  ],
  "notes": [{ "id": "note0", "text": "hello", "class": null, "index": 0, "parent": null }],
  "interfaces": [{ "id": "interface0", "label": "A", "classId": "I" }],
  "namespaces": [
    { "id": "A.B", "label": "A.B", "parent": null, "explicit": true, "classes": ["X"], "notes": [] }
  ],
  "styleClasses": [{ "id": "bar", "styles": ["fill:#f9f"] }],
  "defaultStyle": null
}
```

Conventions (from `canonical-model.md`): `null` for values the DB leaves `undefined`; `""` for empty strings the DB always sets; `[]` for lists. Lists keep the DB's insertion order (classes, relations, notes, namespaces, interfaces, styleClasses).

The `text` of a member is the DB's string: a backslash and the visibility when there is one (`\+name`, classTypes.ts:129), then the id, then `(params) : type` for methods, with `<`/`>` written `&lt;`/`&gt;` (classTypes.ts:130). `classifier` (`$`, `*`) is not in `text`.

Some fields are a choice; §10 asks for a decision:

- `members` and `methods` keep the input fields (`visibility`, `classifier`, `memberType`, `id`, `parameters`, `returnType`). `text` is the DB's display text (sanitised, with `&lt;` for `<`). It is DB state (`classDiagram.spec` checks it), so keep it.
- `type1`/`type2` as the marker names (`none`, `aggregation`, …) rather than numbers: the DB names them via `getArrowMarker`, and it is the rendering-facing vocabulary. Alternatively keep the numbers (`relationType` L505–511) and let the renderer map them.
- `lineType` as `solid` or `dashed` for `pattern` in getData (L857) — also a choice.

### 3.2 Text and sanitisation

Every text the DB keeps goes through `common.sanitizeText(txt, config)` (common.ts:71–83), which is:

```
sanitizeText(text, config):           [source]
  if !text: return text
  text = sanitizeMore(text, config)         // common.ts:56-69, depends on config
  text = DOMPurify.sanitize(text, {FORBID_TAGS: ['style']})     // or config.dompurifyConfig if set
  return text

sanitizeMore(text, config):           [source]
  if getEffectiveHtmlLabels(config):      // htmlLabels, default true
     level = config.securityLevel
     if level in {antiscript, strict, sandbox}: text = DOMPurify.sanitize(text)   // removeScript
     elif level !== loose:  '<br>' -> '#br#', '<' -> '&lt;', '>' -> '&gt;', '=' -> '&equals;', '#br#' -> '<br/>'
  return text
```

So the stored text depends on `htmlLabels` and `securityLevel`. The default is `htmlLabels: true`, `securityLevel: strict`. The M5 approach applies here: the harness forces `htmlLabels: false` on both sides (RENDER-PLAN.md, "Comparison setup"), so the canonical text is `DOMPurify.sanitize(text, {FORBID_TAGS:['style']})`. The crate `sanitize.rs::sanitize_text` (ferrigura-syntax) is that function, a subset of DOMPurify 3.4.12 (its header comment, sanitize.rs:1–40). Its listed "not reproduced" items are the ones that could matter for class labels, though class labels rarely contain tables or misnested formatting **[unverified how often]**.

Which texts pass through sanitising (all in `classDb.ts` unless noted):

| Text                                           | Sanitised?                                | Where                                        |
| ---------------------------------------------- | ----------------------------------------- | -------------------------------------------- |
| class id (`splitClassNameAndType`, `addClass`) | yes (several times)                       | L81, 87–88, 113, 120                         |
| class label (`setClassLabel`)                  | yes                                       | L97                                          |
| relation titles (`relationTitle1/2`)           | yes, trimmed                              | L242–250                                     |
| relation label (`title`) via `cleanupLabel`    | yes                                       | L322                                         |
| member text (`ClassMember` constructor)        | yes                                       | classTypes.ts:58                             |
| `<<annotation>>` in a member                   | yes                                       | classDb L289                                 |
| annotation from `<<x>>` line (`addAnnotation`) | **no**                                    | L263–266                                     |
| note text                                      | **no**                                    | L308–313 (sanitised later? — see note below) |
| tooltip                                        | yes                                       | L383                                         |
| link target                                    | yes (not `sandbox`)                       | L418                                         |
| link URL                                       | `formatUrl`: `sanitizeUrl` unless `loose` | utils.ts:250–260                             |
| `cssClass`, class `styles`, `classDef`         | **no**                                    | L331–369                                     |
| accessibility title/descr                      | yes                                       | commonDb L17, L23                            |

Note texts are passed raw from the DB, and the renderer sanitises the label again: `labelHelper` calls `sanitizeText(decodeEntities(label))` (shapes/util.ts:95, 177) — so the same function applies at draw time **[source]**. For the canonical model, store the note text raw (as the DB does) and let the renderer apply the same rule, which is what the flowchart does.

`decodeEntities` (utils.ts:904) turns the placeholders `ﬂ°°`, `ﬂ°`, `¶ß` into `&#`, `&`, `;`. It is used in `addText` (shapeUtil.ts:130) and in `labelHelper`.

### 3.3 Title and accessibility rules

| Field                              | Rule                                                                                                                               | Where             |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| `title`                            | the front matter title only; there is no grammar title. The grammar's `title x` line is ignored (probe 6). No precedence question. | Diagram.ts:52–53  |
| `accTitle`                         | grammar value `.trim()` (grammar L273), then sanitise, then remove leading whitespace `^\s+`                                       | commonDb.ts:16–18 |
| `accDescr` (single line and block) | grammar value `.trim()` (L274–275), then sanitise, then `\n\s+` → `\n`                                                             | commonDb.ts:22–24 |

The class rule for the block form is `replace(/\n\s+/g, '\n')` (commonDb.ts:23). Checked in node: `'a\n\nb\n  c  d '` becomes `'a\nb\nc  d '`. Because `\s` matches a newline, this removes the indentation of every line **and collapses blank lines**; it keeps trailing blanks and runs of blanks inside a line. The pie and info rule (`canonical-model.md` §5) also removes trailing blanks and collapses runs of blanks, so the two differ there. Probe 20 gives `line one\n  line two` before the DB and `line one\nline two` after.

Two other notes. The flowchart section of `canonical-model.md` says blank lines are not collapsed, and `flowchart/PLAN.md` §8.3 says "no blank-line collapsing"; both conflict with the same regex, if the flowchart DB uses it too (open question 11). The sanitising is part of the setter, so `sanitize.rs` is needed for these headers too.

### 3.4 Left out, and why

| Left out                                                                                                                                                   | Argument                                                                                                                                                        |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `haveCallback`, the callback function, `call` name and args                                                                                                | callbacks only run under `loose` and only in the browser (decision 0001). The `clickable` class stays (it is set in the same call). Probe 23 covers the syntax. |
| `domId`, `classCounter`, `namespace.domId`, `namespaceCounter`                                                                                             | DOM ids, and a module-level counter that survives `clear()`                                                                                                     |
| `cssStyle` of a member (`font-style:italic;`, `text-decoration:underline;`)                                                                                | derived from `classifier` (classTypes.ts:136–145)                                                                                                               |
| `textStyles` of a style class                                                                                                                              | derived from `styles` (classDb L345–369)                                                                                                                        |
| `shape` (always `classBox`), `type: undefined` in getData                                                                                                  | constant                                                                                                                                                        |
| `getData()` output: `colorIndex`, `isGroup`, `padding`, `look`, `cssStyles`, `labelpos`, `thickness`, `pattern`, `arrowheadStyle`, `labelStyle`, `classes` | rendering (§5.2)                                                                                                                                                |
| `hierarchicalNamespaces`, `hideEmptyMembersBox`                                                                                                            | configuration, read at render time                                                                                                                              |
| `arrowMarkerAbsolute`                                                                                                                                      | configuration (classDiagram-v2.ts:15–20)                                                                                                                        |
| `securityLevel`, `htmlLabels`                                                                                                                              | configuration; they change the text (§3.2) but are not in the model                                                                                             |

### 3.5 Points the model must still decide

- `link` and `tooltip`: the reference sanitises with `sanitizeUrl` (`formatUrl`, utils.ts:250–260). The flowchart M5 port has no `sanitizeUrl` (STATUS, M3/M5 known gaps: "tooltip and link not"). So class `link` needs a `sanitizeUrl` port first, or a decision that the model keeps the trimmed URL and the renderer sanitises. Open question §10.2.
- `ClassMember.text` is a rendering string, but it is DB state. Keep it (§3.1).

---

## 4. Specs

### 4.1 The five spec files

| File                              | Test cases (active)                                                                      | Structure     | What it checks                                                                                                                                               | Level                                                       |
| --------------------------------- | ---------------------------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------- |
| `classDiagram.spec.ts`            | **383** = 133 `it(` + 5 × `it.each(keywords)` over 50 keywords (250); 1 `it.skip` (L293) | 15 `describe` | parse + DB state: namespaces, accessibility, classes, members, annotations, comments, click/link/callback, notes (`it.each`), direction, relations, generics | parser + DB; 3 tests call `getData()` (L154, L199, L230)    |
| `classTypes.spec.ts`              | **93** `it(`                                                                             | 22 `describe` | `ClassMember` text, visibility, classifier, return type, generics for methods and attributes                                                                 | model-level text; uses `sanitizeText` (DOMPurify, so jsdom) |
| `classDiagram-colorIndex.spec.ts` | **3**                                                                                    | 1 `describe`  | colour slots: one per class in order, none for namespaces or notes                                                                                           | DB `getData()`                                              |
| `classDiagram-styles.spec.js`     | **10**                                                                                   | 2 `describe`  | `cssClass`, `classDef`, `style`, `:::` outcomes on the DB                                                                                                    | DB                                                          |
| `parser/class.spec.js`            | **3** = `it.each(['__proto__','constructor'])` (2) + 1 `it`                              | 3 `describe`  | prototype names as class ids; backtick namespace names                                                                                                       | parser + DB                                                 |

Total **492** active vitest cases in the class folder (383 + 93 + 3 + 10 + 3), plus 1 skipped. STATUS.md's "≈ 255" for `class` is about 240 below that. Recount with the vitest list before comparing with the flowchart's 958 (§10.5).

Block arithmetic: 133 + 5 + 93 + 3 + 10 + 1 + 1 = 246, so STATUS.md's "≈ 255" is close to a block count (inference, not confirmed against STATUS's method).

No spec in the class folder calls `draw`, `render` or `getBBox`. So the class folder is parser, DB and `getData` only. The render-level specs are cross-cutting (§4.2).

### 4.2 Cross-cutting specs that touch class

| File                                                                    | Cases                                            | Level                                                                | Note for M6                                                                                                       |
| ----------------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `rendering-util/layout-algorithms/elk/__tests__/terminalLabels.spec.ts` | 7 `it` + 1 `it.each` (2) = 9                     | ELK layout with hand-made class-style data, no DOM                   | the ELK terminal passes; the same 7 names are the ignored tests in `elk_apply.rs:2165–2190`                       |
| `rendering-util/rendering-elements/markers.spec.ts`                     | 1 (L22)                                          | `insertMarkers(svg, [aggregation…lollipop], 'classDiagram', 'test')` | DOM; `markerUnits="userSpaceOnUse"`                                                                               |
| `styles.spec.ts`                                                        | one loop over themes (L84–100)                   | CSS per diagram type and theme                                       | includes `classDiagram` styles                                                                                    |
| `multi-diagram-id-uniqueness.spec.ts`                                   | one generated test per type, class sample at L23 | DOM                                                                  | duplicate ids (DOM)                                                                                               |
| `unique-dom-ids.spec.ts`                                                | 3–4                                              | DOM                                                                  | class domId prefixing                                                                                             |
| `config.appearance.spec.ts`                                             | a few cases                                      | config                                                               | class takes `redux-color` and `neo` by default (L73–77); `getDiagramConfigKey('classDiagram') === 'class'` (L313) |
| `mermaidAPI.spec.ts`                                                    | a few (L857–858)                                 | type detection                                                       | `classDiagram-v2` is type `classDiagram`                                                                          |

---

## 5. Renderer

### 5.1 Pipeline (confirmed)

`classRenderer-v3-unified.ts` `draw` (L41–76):

```
draw(text, id, _version, diag):
  db.setDiagramId(id)                                  L50
  data4Layout = db.getData()                           L54
  svg = getDiagramElement(id, securityLevel)           L57
  data4Layout.type = diag.type                         L59   ('classDiagram')
  data4Layout.layoutAlgorithm = getRegisteredLayoutAlgorithm(layout)   L60   (top-level layout, default 'elk')
  data4Layout.nodeSpacing = conf.nodeSpacing || 50     L62   (conf = getConfig().state, L44)
  data4Layout.rankSpacing = conf.rankSpacing || 50     L63
  data4Layout.markers = [aggregation, extension, composition, dependency, lollipop]   L64
  data4Layout.diagramId = id                           L65
  await render(data4Layout, svg)                       L66   (rendering-util/render.ts, shared)
  utils.insertTitle(svg, 'classDiagramTitleText', conf.titleTopMargin ?? 25, db.getDiagramTitle())   L68-73
  setupViewPortForSVG(svg, 8, 'classDiagram', conf.useMaxWidth ?? true)   L75
```

`render.ts` has no class-specific branch (no `classDiagram` string in `rendering-util/` outside specs; the `'class'` hits are DOM attributes). So the class pipeline is the shared one: `createGraph` → `layout` → ELK post-processing → shapes, edges, labels. This is the same path as flowcharts (`flowRenderer-v3-unified.ts`, 57 lines). **Confirmed.**

### 5.2 `getData` mapping (classDb.ts:738–865)

| Source                                                                 | Output node or edge                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Fields set                                                                                                                                                | Lines                                                                     |
| ---------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| each namespace (skipped if `hierarchical` is false and not `explicit`) | node `{id, label, isGroup: true, padding: class.padding ?? 16, shape: 'rect', cssStyles: [], look, parentId}`                                                                                                                                                                                                                                                                                                                                                                           | `label` = namespace label (hierarchical) or id (compact)                                                                                                  | 744–760 (padding L752)                                                    |
| each class                                                             | node `{...classNode, type: undefined, isGroup: false, parentId, look, colorIndex}`                                                                                                                                                                                                                                                                                                                                                                                                      | `shape: 'classBox'`, `members`, `methods`, `annotations`, `styles`, `cssClasses`, `link`, `linkTarget`, `tooltip`, `haveCallback` come through the spread | 765–778 (colorIndex L775)                                                 |
| each note                                                              | node `{id: note.id, label: note.text, isGroup: false, shape: 'note', padding: class.padding ?? 6, cssStyles: ['text-align: left', 'white-space: nowrap', 'fill: noteBkgColor', 'stroke: noteBorderColor'], look, parentId, labelType: 'markdown'}`                                                                                                                                                                                                                                      |                                                                                                                                                           | 780–798 (padding L787)                                                    |
| each note with a known class                                           | edge `{id: 'edgeNote<index>', start: note.id, end: class id, type: 'normal', thickness: 'normal', classes: 'relation', arrowTypeStart: 'none', arrowTypeEnd: 'none', labelStyle: [''], style: ['fill: none'], pattern: 'dotted'}`                                                                                                                                                                                                                                                       |                                                                                                                                                           | 800–819                                                                   |
| each interface (lollipop)                                              | node `{id: interface<n>, label, isGroup: false, shape: 'rect', cssStyles: ['opacity: 0;'], look}`                                                                                                                                                                                                                                                                                                                                                                                       | no `labelType`                                                                                                                                            | 822–832                                                                   |
| each relation, `cnt` from 1                                            | edge `{id: getEdgeId(id1, id2, {prefix:'id', counter: cnt}) = 'id_<id1>_<id2>_<cnt>', start: id1, end: id2, type: 'normal', label: title, labelpos: 'c', thickness: 'normal', classes: 'relation', arrowTypeStart: marker(type1), arrowTypeEnd: marker(type2), startLabelRight: rel1 (or ''), endLabelLeft: rel2 (or ''), arrowheadStyle: '', labelStyle: ['display: inline-block'], style: rel.style \|\| '', pattern: lineType==1 ? 'dashed' : 'solid', look, labelType: 'markdown'}` |                                                                                                                                                           | 834–862 (ids L838–840, arrows L849–850, terminals L851–853, pattern L857) |

Returned as `{nodes, edges, other: {}, config, direction}` (L864). Node order: namespaces, classes, notes, interfaces. Edge order: note edges (during the notes loop), then relation edges.

Notes:

- `startLabelRight` and `endLabelLeft` hold the cardinality texts. A cardinality written as the literal word `none` is treated as absent, because `'none'` is the grammar's default sentinel (grammar L340–343; classDb L851–853) **[source]**.
- `label` is `title`, and is `undefined` when there is no label; `getEdgeId` ids use `cnt` counting all relations, including those ignored later.

### 5.3 `classBox` geometry (classBox.ts + shapeUtil.ts)

#### 5.3.1 Constants

| Name                            | Value                                                                               | Source                             |
| ------------------------------- | ----------------------------------------------------------------------------------- | ---------------------------------- |
| `P` (PADDING = GAP)             | `config.class.padding ?? 12`                                                        | classBox.ts:18–19; shapeUtil.ts:20 |
| `TEXT_PADDING`                  | 3 if `!useHtmlLabels`, else 0                                                       | shapeUtil.ts:22                    |
| namespace padding               | `class.padding ?? 16`                                                               | classDb.ts:752                     |
| note padding                    | `class.padding ?? 6`                                                                | classDb.ts:787                     |
| `useHtmlLabels` for rows        | `'useHtmlLabels' in node ? … : evaluate(htmlLabels) ?? true`                        | shapeUtil.ts:107–108               |
| font                            | `font-weight: bolder` on the label row; members and methods use `classifier` styles | shapeUtil.ts:48, 55, 66, 105       |
| annotation text                 | `«${annotations[0]}»` (only the first annotation)                                   | shapeUtil.ts:38–41                 |
| `createText` width for each row | `calculateTextWidth(text, config) + 50` (DOM-based, see §5.3.4)                     | shapeUtil.ts:128–137               |
| `createText` markdown           | default true (createText.ts:314), so every row is Markdown                          |                                    |
| row class                       | `markdown-node-label`                                                               | shapeUtil.ts:133                   |

Quirk: `class.padding` is undefined at runtime (§6.1), so `P = 12`, `GAP = 12`, namespace padding 16, note padding 6.

#### 5.3.2 `textHelper` (shapeUtil.ts:15–96), positions of the four groups

```
annGroup  = g.annotation-group.text   (only if annotations)
labelGroup= g.label-group.text
membersGroup = g.members-group.text
methodsGroup = g.methods-group.text

annH = annotations ? height(addText(annGroup, "«a»", y=0)) : 0          L38-45 (group bbox after the row)
labH = height(addText(labelGroup, node, y=0, ['font-weight: bolder']))   L47-50 (group bbox)
yOff = 0
for m in members:   h = addText(membersGroup, m, yOff, [classifier style]); yOff += h + TEXT_PADDING     L52-57
memH = height(membersGroup) ; if memH <= 0: memH = GAP/2                                             L58-61
yOff2 = 0; for m in methods: same with methodsGroup                                                 L63-68

annGroup.transform   = translate(-annW/2)                     L73-76
labelGroup.transform = translate(-labW/2, annH)              L79
membersGroup.transform = translate(0, annH + labH + 2*GAP)   L83-86
methodsGroup.transform = translate(0, annH + labH + (memH ? memH + 4*GAP : 2*GAP))   L88-91
return bbox = getBBox(shapeSvg)  (after the transforms)     L93-95
```

Quirk: `memH` is never 0 after L59–61 (it is at least GAP/2 = 6), so the `: 2*GAP` branch on L90 is dead when `GAP > 0`. An empty members group gives methods at `annH + labH + 6 + 48 = annH + labH + 54` **[source]**. This bbox feeds the node's size (§5.3.3), so this value matters for the size of classes that have methods but no members.

#### 5.3.3 `classBox` (classBox.ts:14–276)

```
(annH', labH', memH') are the raw getBBox heights of the three groups, minus P/2 when renderExtraBox, then "|| 0"
   annH' = getBBox(ann).h - (renderExtraBox ? P/2 : 0) || 0      L97-99
   labH' = same for label                                        L100-102
   memH' = same for members                                      L103-105
   (the "|| 0" turns NaN and -0 into 0, but a negative value stays: an empty annotation group in an empty-box class is -6)

noMM = members.length == 0 && methods.length == 0
renderExtraBox = noMM && !config.class.hideEmptyMembersBox                    L42-45
w = max(node.width ?? 0, bbox.w)                                                L57
h = max(node.height ?? 0, bbox.h); nodeHeightGreater = (node.height ?? 0) > bbox.h    L58-59
if noMM: h += P                  ; elif members && !methods: h += 2P          L60-64
x = -w/2 ; y = -h/2                                                             L66-67
extraHeight = renderExtraBox ? 2P : (noMM ? -P : 0); if nodeHeightGreater: 2P   L68-75

rect = rectangle(
   x = x - P,
   y = y - P - (renderExtraBox ? P : (noMM ? -P/2 : 0)),
   width = w + 2P,
   height = h + 2P + extraHeight)                                               L78-90
   class "basic label-container outer-path", inserted as first child           L92-93
   rectBBox = getBBox(rect)                                                     L94

methodsAreaPlacement = (annH' + labH' + y + P - (y - P - (renderExtraBox ? P : (noMM ? -P/2 : 0)))) / 2     L108-120

for each .text group (annotation, label, members, methods), with translateY = the y in its current transform:
   newY = translateY + y + P - (renderExtraBox ? P : (noMM ? -P/2 : 0))                 L138-146
   if group == methods:
       memHm = max(memH', P/2)
       if nodeHeightGreater: newY = max(methodsAreaPlacement, annH' + labH' + memHm + y + 2P + P) + 2P      L150-160
       else:                 newY = annH' + labH' + memHm + y + 4P + P                                     L161-169
   if noMM && hideEmptyMembersBox: newY = (annotations ? translateY - P : translateY)                        L171-181
   if !useHtmlLabels: newY -= 4                                                                           L182-186
   newX = x
   if group is label or annotation: newX = -getBBox(group).w / 2   (0 if any text has text-anchor middle)  L187-198
   transform = translate(newX, newY)                                                                       L200

first divider (if members || methods || renderExtraBox):
   y1 = annH' + labH' + y + P
   line from (rectBBox.x, y1) to (rectBBox.x + rectBBox.w, y1 + 0.001)          L207-220 (class "divider" + " neo-line" if look neo and !useGradient)

second divider (if renderExtraBox || members || methods):
   y2 = annH' + labH' + memH' + y + 2P + P         (GAP*2 + PADDING)              L224-225
   if nodeHeightGreater: y2 = max(methodsAreaPlacement, y2)
   line from (rectBBox.x, y2) to (rectBBox.x + rectBBox.w, y2 + 0.001)          L226-237

styles: shapeSvg paths get `styles`; rect child :nth-child(2) gets styles; divider paths get styles; spans/tspans get label styles   L240-268
bounds: updateNodeBounds(node, rect) → node.width = getBBox(rect).w, node.height = getBBox(rect).h     L270
node.intersect = rect intersection                                              L271-273
```

Worked consequences [source]:

- A class with no members and no methods, default config: `renderExtraBox` is true, so the rect is `h' + 2P + 2P` tall, where `h'` = text height + `P` (L60–61), and its top is `2P` above `y`. For the text groups the offset `P − P` cancels, so the non-method groups move by `y` alone (the formula at L138–146 with `renderExtraBox`).
- `memH' = getBBox(members).h − P/2` for an empty members group, so `−6` when `renderExtraBox`. That makes `annH'`, `labH'` and `memH'` each 6 px smaller than the boxes they describe, and it moves both dividers.
- `methodsAreaPlacement` is only used when `nodeHeightGreater` (node has a fixed height) or in the methods shift (L150–160). `node.height` is undefined for classes, so in practice it is not used. Keep it anyway: `ELK` size can be set by the caller (`node.height`).

#### 5.3.4 Row text (`addText`, shapeUtil.ts:99–189)

```
addText(parentGroup, node, yOffset, styles):
   textEl = g.label style=styles.join('; ')
   useHtml = node.useHtmlLabels ?? evaluate(htmlLabels) ?? true      L107-108
   text = node.text (ClassNode/ClassMember) else node.label           L112-116
   if !useHtml and text starts with '\\': text = text.substring(1)     L120-122   (visibility escape, §5.3.5)
   if hasKatex(text): useHtml = true                                   L124-126
   text_el = createText(textEl, sanitizeText(decodeEntities(text)),
                        {width: calculateTextWidth(text, config) + 50, classes: 'markdown-node-label', useHtmlLabels: useHtml}, config)   L128-137
   if !useHtml:
      if styles has 'font-weight: bolder': tspans font-weight = ''     L141-145
      numberOfLines = text_el.children.length                          L147
      first = text_el.children[0]
      if text_el.textContent == '' or includes('&gt'):                 L150
          first.textContent = t[0] + t.substring(1).replace('&gt;','>').replace('&lt;','<').trim()   (t = text before sanitising)
          if t[1] == ' ': first.textContent = first[0] + ' ' + first.substring(1)                     L156-158
      if first.textContent == 'undefined': first.textContent = ''      L163-165
      bbox = getBBox(text_el)                                          L168
   else:  (HTML path: not ported)
   textEl.transform = translate(0, -bbox.h / (2*numberOfLines) + yOffset)    L187
   return bbox.h                                                       L188
```

The `&gt`-test (L150) fires for every row whose sanitised text contains `&gt`, which is every row with `<` or `>` in it. For those rows the first text child is replaced by the **raw** text with `<`/`>` restored and trimmed (L151–160); the DOMPurify form then only matters for the width and the tspan count. Rows without `<` or `>` keep the sanitised text. [source]

#### 5.3.5 `ClassMember` text (classTypes.ts:31–146)

- Constructor: `sanitizeText(input)` (L58), then `parseMember`.
- Method regex (L84): `/([#+~-])?(.+)\((.*)\)([\s$*])?(.*)([$*])?/`; attribute: first char is the visibility if in `['#','+','~','-','']`, last char `$` or `*` is the classifier (L106–123).
- A member is a method if `indexOf(')') > 0` (classDb L290), so `int x)` is a method.
- Text (L129–133): `(visibility ? '\\' + visibility : '') + generic-processed id + (method ? '(params)' + (returnType ? ' : ' + type) : '')`, then `<` → `&lt;`, `>` → `&gt;` (L130); a leading `\\&lt;` becomes `~` (L131–133).
- `addText` strips the leading `\` for SVG rows (shapeUtil L120–122), which turns `\+name` into `+name`.
- `getDisplayDetails` (L62–78) is the HTML form (`displayText`); the renderer uses `text`.
- Generic types (`parseGenericTypes`, common.ts:201–229): `~` pairs become `<…>`; a single pair across a comma is rejoined (`shouldCombineSets`, common.ts:235–240); `processSet` (common.ts:242–) converts pairs of `~` into `<`, `>`, a lone leading `~` is kept.

#### 5.3.6 neo versus classic

Only these differ in the class code:

| Difference                                                                                                                                               | Where                                                                           |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `neo-line` class on both dividers when `look === 'neo'` and `!useGradient`                                                                               | classBox.ts:218, 235                                                            |
| relation markers use the margin variants when `look === 'neo'`                                                                                           | edges.js:946 (`useMargin = !animatedEdge && edge.look === 'neo'`)               |
| `.edgeLabel[data-look="neo"]` background styles                                                                                                          | styles.js:229–240                                                               |
| `data-look` attribute on the node group                                                                                                                  | nodes.ts:~56 (`handleUndefinedAttr(node.look)`)                                 |
| `handDrawn` only: rough.js keeps its own roughness (0.7 by default, from `userNodeOverrides`) and the hachure fill; otherwise roughness 0 and fill solid | classBox.ts:52–55; handDrawnShapeStyles.ts (`userNodeOverrides`, roughness 0.7) |

The default look is neo (§6.2), so the margin markers and `neo-line` are the normal case.

#### 5.3.7 Other shapes used by class

`note` (note.ts:10–67) uses `labelHelper` and a rect with `noteBkgColor`/`noteBorderColor` as the default fill and stroke, `node.cssStyles` on the paths (note.ts:47–52). `rect` is the namespace group and the interfaces. No other shape. The class DB never sets `shape` to anything but `classBox`, `note` or `rect`.

### 5.4 Relation markers (markers.js and edgeMarker.ts)

Ids: `${svgId}_${diagramType}-${kind}${Start|End}${-margin?}` with `diagramType = 'classDiagram'` (edgeMarker.ts:99–100; `markers.js` L17, L31, …). The margin variant is used when `look === 'neo'`, and only for kinds in `arrowTypesWithMarginSupport` (edgeMarker.ts:59–69), which includes all five class kinds.

| Kind        | Start: refX, refY, markerW×H, path                             | End: refX, refY, markerW×H, path                    | Start-margin                                                                   | End-margin                                                        | markers.js |
| ----------- | -------------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------- | ---------- |
| extension   | refX 18, refY 7, 20×28, `M 1,7 L18,13 V 1 Z`                   | refX 1, refY 7, 20×28, `M 1,1 V 13 L18,7 Z`         | refX 18, 20×28, viewBox `0 0 20 14`, polygon `10,7 18,13 18,1`, stroke-width 2 | refX 9, 20×28, viewBox, polygon `10,1 10,13 18,7`, stroke-width 2 | 12–74      |
| composition | refX 18, refY 7, 190×240, `M 18,7 L9,13 L1,7 L9,1 Z`           | refX 1, refY 7, 20×28, same path                    | refX 15, 190×240, path, stroke-width 0, viewBox `0 0 15 15`                    | refX 3.5, 20×28, path, stroke-width 0                             | 76–135     |
| aggregation | refX 18, refY 7, 190×240, same diamond                         | refX 1, refY 7, 20×28, same diamond                 | refX 15, 190×240, stroke-width 2                                               | refX 1, 20×28, stroke-width 2                                     | 136–194    |
| dependency  | refX 6, refY 7, 190×240, `M 5,7 L9,13 L1,7 L9,1 Z`             | refX 13, refY 7, 20×28, `M 18,7 L9,13 L14,7 L9,1 Z` | refX 4, 190×240, stroke-width 0                                                | refX 16, 20×28, stroke-width 0                                    | 195–252    |
| lollipop    | refX 13, refY 7, 190×240, `circle cx7 cy7 r6 fill transparent` | refX 1, refY 7, 190×240, same circle                | refX 13, 190×240, circle, stroke-width 2                                       | refX 1, 190×240, stroke-width 2                                   | 253–321    |

- Each marker has `markerUnits="userSpaceOnUse"` and `orient="auto"`. Marker class: `marker <kind> classDiagram`, e.g. the composition markers have `class="marker composition classDiagram"` (markers.js:81).
- `insertMarkers` runs the five functions in the order of `data4Layout.markers` (markers.js:6–10; classRenderer L64).
- The extension start-margin marker is appended to `elem` directly, not to `defs` (markers.js:42–43). The other margin markers are in `defs`. Not geometry, but the SVG structure differs.
- Marker path offsets (how far the edge is pulled back): `aggregation`, `extension`, `composition` 17.25; `dependency` 6; `lollipop` 13.5 (`lineWithOffset.ts` L6–16, tabled in `flowchart/render-notes/elk-apply.md` §3.1, line 124). M5 already has them in `elk_apply.rs:941–952` (`marker_path_offset`).
- The CSS for the markers is in `styles.js:158–216` (`[id$="-aggregationStart"], .aggregation {…}`, and so on).

### 5.5 Edge labels and terminal labels

#### 5.5.1 Middle label

`insertEdgeLabel` (edges.js:74–131) is the same as for flowcharts, with `labelType: 'markdown'` and `labelStyle: ['display: inline-block']`. Positioned by `positionEdgeLabel` (edges.js:301–325) at `resolveEdgeLabelPosition`.

#### 5.5.2 Terminal labels (cardinalities)

Creation (edges.js:139–251). Only two of the four keys are used by the class DB:

| Key (`TerminalLabelKey`, types.ts:142) | Class source                       | Created at       | Positioned at    |
| -------------------------------------- | ---------------------------------- | ---------------- | ---------------- |
| `startRight`                           | `startLabelRight` = relationTitle1 | edges.js:168–195 | edges.js:337–351 |
| `endLeft`                              | `endLabelLeft` = relationTitle2    | edges.js:196–223 | edges.js:352–362 |
| `startLeft`, `endRight`                | not set by the class DB            | —                | —                |

Each terminal is an `edgeTerminals` group with an `inner` group; the inner group is centred with `computeLabelTransform`; `recordTerminalLabelSize` stores `{width, height}` unpadded (edges.js:263–266).

ELK input (elk/render.ts:1144–1173, `terminalElkLabels`):

```
for each measured terminal (key, size) in edge.terminalLabelSizes:
   atStart = key starts with 'start'
   marker = atStart ? arrowTypeStart : arrowTypeEnd
   pad = (marker && marker != 'none') ? 6 : 0                     TERMINAL_LABEL_MARKER_CLEARANCE (L1145)
   ELK label {terminal: key, text, width: size.w + 2*pad, height: size.h + 2*pad,
              layoutOptions: {'edgeLabels.placement': atStart ? 'TAIL' : 'HEAD'}}
```

M4's corpus already contains class graphs with TAIL and HEAD placements (`ferrigura-elk/tests/corpus/class-diagram__*.json`, 11 files, e.g. `16b-should-handle-the-direction-statement-with-tb`), so `ferrigura-elk` can take them. None of the class graphs is in the `NOT_PORTED` list (`ferrigura-elk/tests/differential.rs:123`); their individual pass status is unverified here.

Positions after ELK (elk/render.ts and common/index.ts):

| Step                                                                                                      | Reference                             | M5 status                                               |
| --------------------------------------------------------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------- |
| read ELK centre of each terminal (x + off.x + w/2, y + off.y + h/2) and store `terminalLabelCenters[key]` | render.ts:2085–2096                   | not ported (`elk_apply.rs:15–18`; `elk-apply.md` §3.11) |
| `terminalLabelPorts` for the moved ends                                                                   | render.ts:2091–2098, 2119–2125        | not ported                                              |
| `followMovedEndpoints`: move each centre by the delta of its own end                                      | render.ts:2345–2364                   | not ported (test ignored, `elk_apply.rs:2189`)          |
| `slideTerminalLabelsOffFrames` (step along the end segment)                                               | render.ts:2275–2312                   | not ported (`elk_apply.rs:2185`)                        |
| `putTerminalLabelsOnTheirSide` (mirror across the end segment)                                            | render.ts:2322–2338                   | not ported                                              |
| `terminalLabelObstacles` (nodes, main labels, frames, other edges, other terminals)                       | render.ts:2179–2242                   | not ported                                              |
| `TERMINAL_LABEL_FRAME_CLEARANCE` = 2                                                                      | render.ts:2268                        | not ported                                              |
| fallback when no centre: `calcTerminalLabelPosition(arrow ? 10 : 0, side, path)`                          | common/index.ts:364–378; utils.ts:378 | not ported                                              |
| paint: `translate(x, y)` on the `edgeTerminals` group                                                     | edges.js:326–373                      | not ported                                              |
| `separateOppositeEdgeLabels` (main labels only)                                                           | render.ts:2135–2170                   | check: M5 has the centre-label pass, unverified         |

Ignored M5 tests that match these (`elk_apply.rs:2165–2190`): "keeps every cardinality label beside its own end…", "keeps a label off a neighbouring edge…", "puts start and end labels on the same sides…", "drops centres from an earlier layered run…", "keeps a label that enters another namespace clear of its frame", "does not slide a label past the far end of its end segment", "moves a placed label with an endpoint that a later pass slid along the side". These are the 7 `it` blocks of `terminalLabels.spec.ts`.

#### 5.5.3 The `edgeTerminals` paint

Created in `insertEdgeLabel`, class `edgeTerminals`, one `g.inner` each, with the label created by `createLabel(inner, text, style, false, false)` (edges.js:144–150). Font size of terminal text: `.edgeTerminals { font-size: 11px }` (styles.js:218–221).

### 5.6 Namespaces, notes, interfaces

| Element                                                  | Reference                                            | M5                                                                               | M6                                                                            |
| -------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| namespace (group, `rect`, padding 16, parentId)          | classDb 744–760; clusters.js `rect`                  | `clusters.rs` handles `rect` clusters and reads `node.padding` (clusters.rs:137) | reuse; set `padding` 16                                                       |
| compact namespace mode (`hierarchicalNamespaces: false`) | classDb 723–736 (`resolveExplicitAncestor`), 742–746 | none                                                                             | port the 3-line rule                                                          |
| note (`shape: note`)                                     | note.ts; classDb 780–820                             | `rects::note` (rects.rs:47) with `label_helper`                                  | reuse; check the `noteBkgColor` fill and the `cssStyles` path (note.ts:47–52) |
| note edge (`edgeNote<i>`, dotted, `fill: none`)          | classDb 800–819                                      | `pattern` handled (`edges.rs:297–305`)                                           | reuse                                                                         |
| interface (`rect`, `opacity: 0;`)                        | classDb 822–832                                      | `style_list` handles the trailing `;` **[unverified]**                           | reuse                                                                         |

Namespace cluster titles: `clusters.js` `rect` title is measured unwrapped (M5 notes, `labels.md` §13). The class `.cluster-label` colours are in `styles.js:54–62`.

### 5.7 Styles and colour slots

#### 5.7.1 `styles.js` (244 lines)

| Lines   | Selector group                                                                                                                                                 | Uses                                                                        |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| 14–38   | `genColor`: per slot i < `colorSlotCount`: `[data-look="<look>"][data-color-id="color-i"].node .outer-path path {stroke; fill}` and `… .divider path {stroke}` | `borderColorArray`, `bkgColorArray`, `THEME_COLOR_LIMIT`, `look` (safeLook) |
| 42–52   | `g.classGroup text` (font size 10px)                                                                                                                           | `nodeBorder`/`classText`, `fontFamily`                                      |
| 54–76   | cluster label and rect                                                                                                                                         | `titleColor`, `clusterBkg`, `clusterBorder`                                 |
| 78–97   | `.nodeLabel, .edgeLabel`, `.noteLabel`, `.edgeLabel .label rect`, `.label text`, `.labelBkg`                                                                   | `classText`, `noteTextColor`, `mainBkg`                                     |
| 102–110 | `.node rect, circle, ellipse, polygon, path` fill/stroke                                                                                                       | `mainBkg`, `nodeBorder`, `strokeWidth`                                      |
| 113–116 | `.divider`                                                                                                                                                     | `nodeBorder`                                                                |
| 118–120 | `g.clickable { cursor: pointer }`                                                                                                                              |                                                                             |
| 122–130 | `g.classGroup rect/line`                                                                                                                                       |                                                                             |
| 132–142 | `.classLabel .box`, `.classLabel .label`                                                                                                                       |                                                                             |
| 144–148 | `.relation` stroke                                                                                                                                             | `lineColor`, `strokeWidth`                                                  |
| 150–156 | `.dashed-line`, `.dotted-line` dash arrays                                                                                                                     |                                                                             |
| 158–216 | marker colours (composition, dependency, extension, aggregation, lollipop) with `!important`                                                                   | `lineColor`, `mainBkg`                                                      |
| 218–221 | `.edgeTerminals` 11px                                                                                                                                          |                                                                             |
| 223–227 | `.classTitleText`                                                                                                                                              | `textColor`                                                                 |
| 229–240 | `.edgeLabel[data-look="neo"]`                                                                                                                                  | `edgeLabelBackground`                                                       |
| 241     | `getIconStyles()`                                                                                                                                              | `globalStyles.js`                                                           |

The class stylesheet is not what M5's `style.rs` writes. `style.rs` has the flowchart rules, `palette_rules` (style.rs:464–497, cluster and collapsed-group only) and `class_rules` (style.rs:499, for `classDef`). The node palette rules (`.node .outer-path path`, `.divider path`) are missing.

#### 5.7.2 Colour slots

- `COLOR_THEMES` = `redux-color`, `redux-dark-color` (colorThemeGate.ts:17). Other themes get no slots.
- `stampColorSlot(shapeSvg, node.colorIndex, theme, borderColorArray)` writes `data-color-id="color-<index % palette length>"` (colorThemeGate.ts:114–126), and only when the theme has a palette.
- `colorIndex` counts classes only (classDb L764–776). The `classDiagram-colorIndex.spec.ts` tests pin this.
- M5's `style.rs:56–74` (`palette`, `palette_slot`) has the same palette and slot rule, and `data-color-id` for clusters (`clusters.rs:78`). Reuse it.

Result: with the default `redux-color` theme, **every class box is painted from the palette**. A class with no `classDef` gets its border and fill from the 12-slot palette. The colour is set by the stylesheet, not by an attribute.

#### 5.7.3 Theme variables read by class code

`nodeBorder`, `mainBkg`, `classText`, `titleColor`, `clusterBkg`, `clusterBorder`, `noteTextColor`, `lineColor`, `strokeWidth`, `textColor`, `edgeLabelBackground`, `fontFamily`, `noteBkgColor`, `noteBorderColor`, `borderColorArray`, `bkgColorArray`, `THEME_COLOR_LIMIT`, `useGradient`, `handDrawnSeed`. Ferrigura's `crates/ferrigura-model/src/theme/redux_color.rs` defines `noteBkgColor` (L56), `noteBorderColor` (L92), `THEME_COLOR_LIMIT` = 12 (L59) and `useGradient` = false (L72) (checked by grep). `redux_dark_color.rs` not checked. [`noteTextColor`, `edgeLabelBackground`: unverified, not grepped.]

### 5.8 Root, title and viewport

- Root: `setupViewPortForSVG(svg, 8, 'classDiagram', useMaxWidth)` (setupViewPortForSVG.ts:5–26): `class="classDiagram"`, viewBox `x−8 y−8 w+16 h+16` from the svg's bbox.
- Title: `insertTitle(svg, 'classDiagramTitleText', titleTopMargin, title)` (utils.ts:793–): `titleTopMargin` = **state** section, default 25 (`StateDiagramConfig`, schema about L1681). Title only when set, from front matter.
- `nodeSpacing`/`rankSpacing`: `conf.nodeSpacing || 50`, `conf.rankSpacing || 50`, both from the **state** section, which has no default (schema L1695, L1698).
- `useMaxWidth`: `conf.useMaxWidth ?? true` (state section; base default true).

The flowchart reads `flowchart.titleTopMargin || 0` and `useMaxWidth || false` (flowRenderer-v3-unified.ts:48, 51). Class uses the state section. This is copied from the state renderer (stateRenderer-v3-unified.ts:138–141), so it is probably an oversight in the reference; ferrigura must follow it to match.

### 5.9 Links, tooltips, callbacks (nodes.ts:40–65)

- `link` → `<a xlink:href target>` around the node (nodes.ts:44–52, `svg:a` at L46). M5 writes this for flowchart vertices (`shapes/mod.rs:246–257`), so class can reuse it.
- `tooltip` → `title` attribute on the node element (nodes.ts:57–59). M5: `shapes/mod.rs:246` (`title`).
- `data-look` on the node element (nodes.ts:55).
- `haveCallback` → adds the `clickable` class (nodes.ts:63–65). Class nodes with `click … call` get `clickable` through `setClickEvent`'s `setCssClass` as well (classDb L443).

### 5.10 M5 gap list

| Reference part                                                                 | Where                                                | In M5                                                                        | Work for M6                                                               |
| ------------------------------------------------------------------------------ | ---------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Pipeline and ELK                                                               | `render.ts`, `elk/render.ts`                         | yes                                                                          | none, except the `type` value and nodes/edges from class data             |
| `getData` for class                                                            | classDb 738–865                                      | flowchart `data.rs` only                                                     | new `class/data.rs` (§5.2)                                                |
| `classBox`                                                                     | classBox.ts                                          | no                                                                           | new (§5.3); needs `getBBox` of translated groups                          |
| `textHelper`, `addText`                                                        | shapeUtil.ts                                         | labels yes, rows no                                                          | new rows, reuse `createText` (markdown) with `LabelSpec` (label.rs:44–63) |
| `note`                                                                         | note.ts                                              | yes (`rects::note`)                                                          | reuse                                                                     |
| `rect` clusters                                                                | clusters.js                                          | yes (`clusters.rs`)                                                          | reuse; padding 16                                                         |
| markers: aggregation, extension, composition, dependency, lollipop, 4 variants | markers.js                                           | point, circle, cross only (`markers.rs:15`, `DIAGRAM_TYPE = "flowchart-v2"`) | new defs, reuse `marker_path_offset` (elk_apply.rs:941–952)               |
| marker id and class `classDiagram`                                             | edgeMarker.ts:100                                    | `flowchart-v2` hard-coded                                                    | parameterise                                                              |
| terminal labels (build, ELK passes, paint)                                     | elk/render.ts 1144–1173, 2085–2364; edges.js 139–373 | none (tests ignored)                                                         | new; largest renderer item                                                |
| palette rules for nodes (`.outer-path`, `.divider`)                            | styles.js 14–38                                      | clusters only (`style.rs:464–497`)                                           | new rules                                                                 |
| class stylesheet                                                               | styles.js                                            | flowchart sheet only                                                         | new sheet                                                                 |
| root `class="classDiagram"`, aria                                              | setupViewPortForSVG; a11y                            | `class="flowchart"`, `flowchart-v2` hard-coded (`mod.rs:323`, `327`)         | parameterise                                                              |
| title id `classDiagramTitleText`                                               | classRenderer 68–73                                  | `flowchartTitleText`                                                         | parameterise                                                              |
| spacing and title from `state` section                                         | classRenderer 44, 62–63, 71                          | flowchart reads its own                                                      | read `state`                                                              |
| `<a>`, `title`                                                                 | nodes.ts                                             | yes (`shapes/mod.rs:246–257`)                                                | reuse                                                                     |
| `sanitizeUrl` for `link`                                                       | utils.ts formatUrl                                   | not ported                                                                   | port or decide (§10)                                                      |
| text sanitising (`sanitize_text`)                                              | common.ts, DOMPurify                                 | yes, for flowchart texts (`sanitize.rs`)                                     | reuse; same function                                                      |

---

## 6. Configuration

### 6.1 The `class` section: schema against runtime

| Key                          | Schema (`ClassDiagramConfig` 1710–1756) | Runtime value in the reference                          | Read by class code                                                                     |
| ---------------------------- | --------------------------------------- | ------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `theme`                      | `redux-color` (1719–1721)               | `redux-color` (defaultConfig L78 via defaultConfigJson) | appearance (config.ts)                                                                 |
| `look`                       | `neo` (1722–1724)                       | `neo` (L79)                                             | getData `look`, appearance                                                             |
| `layout`                     | none                                    | `undefined` (L80)                                       | no                                                                                     |
| `padding`                    | **5** (1730–1732)                       | **undefined** (not in the object at L73–83)             | classBox L18 (`?? 12`), shapeUtil L20 (`?? 12`), classDb L752 (`?? 16`), L787 (`?? 6`) |
| `textHeight`                 | 10 (1733–1735)                          | undefined                                               | not read by class code (grep)                                                          |
| `nodeSpacing`, `rankSpacing` | integer, no default (1736–1741)         | undefined                                               | not read (renderer reads `state`)                                                      |
| `diagramPadding`             | ref to ER (1742–1743)                   | undefined                                               | not read (renderer hard-codes 8)                                                       |
| `titleTopMargin`             | 25 (1725–1727)                          | undefined                                               | not read (renderer reads `state`)                                                      |
| `arrowMarkerAbsolute`        | link to top (1729)                      | set by `init` (classDiagram-v2.ts:19)                   | markers url                                                                            |
| `hideEmptyMembersBox`        | false (1744–1746)                       | false (L81)                                             | classBox L45, L174                                                                     |
| `hierarchicalNamespaces`     | true (1747–1755)                        | true (L82)                                              | classDb L742                                                                           |

`defaultConfig.ts:73–83` builds the `class` object from scratch (comment at L74–77 says so: "the rest stay off, `padding` above all — the schema default of 5 would change class node dimensions"). So the reference's runtime `class` section does **not** include the schema defaults. [source; the comment is the author's own statement.]

Ferrigura: `crates/ferrigura-model/build.rs` makes the defaults from the schema. `src/config/ts_defaults.json` holds the TS-only defaults and lists `pie` and `treemap` only (checked). So as things stand, Rust `class.padding` would be 5 and `textHeight` 10. STATUS.md (M2 handover, configuration) says the same thing about `class` and `railroad`. **M6 must put `class` padding (and the other unread keys) back to "absent" in `ts_defaults.json`, or model them as `Option` with `None`.** [source; not run.]

### 6.2 Appearance: theme, look and layout per diagram

- Top-level `look` default is `classic` (schema L68–70). The class section default is `neo` (1722–1724).
- See the layer order above (config.ts L86–100): a user top-level `look` applies to classes; with no user value, the class default `neo` wins.
- `config.appearance.spec.ts:73–77` pins this: `class` takes `redux-color` and `neo` by default, the same as flowchart and state.
- Layer order, from `config.ts` L86–100 [source]: directives (latest first), then the site config delta, then the defaults. For each appearance key the first layer with a usable value wins, and within a layer the diagram section is read before the top level (`readAppearance`, L42–50, called at L94). So a user top-level `look: classic` applies to classes, while the default `neo` in the class section applies when no user value is set.
- Diagram type → section: `classDiagram` → `class` (diagramConfigKeys.ts:9). `classDiagram-v2` is detected as `classDiagram`, so it shares the section.
- Ferrigura's `effective(site, diagram_key, directive)` (ferrigura-model `config/layers.rs:45`) takes the section name, so the caller must map `classDiagram` → `class`. There is no such mapping in the Rust crates yet (grep: no `classDiagram` in `ferrigura-model/src` or `ferrigura-syntax/src`).

### 6.3 Other config read by the class path

- `securityLevel`: decides sanitising (§3.2), `setLink` target, `setClickFunc`.
- `htmlLabels`: decides `useHtmlLabels` for rows and the sanitising path; the harness forces false.
- `fontFamily`, `fontSize`: `calculateTextWidth` (the DOM measure), `createText`. `fontSize` default 16.
- `dompurifyConfig`: if set, replaces the DOMPurify config in `sanitizeText` (common.ts:75–76). Not in the fixtures.
- `flowchart.*`: not read by class code. Routed ELK edges are drawn `rounded` (elk/render.ts:2078; M5 RENDER-PLAN "Curve").

---

## 7. Fixtures

### 7.1 Counts

| Folder                     | `.mmd`  | Note                       |
| -------------------------- | ------- | -------------------------- |
| `nightly-check/`           | 125     |                            |
| `nightly-check/handdrawn/` | 67      | all have `look: handDrawn` |
| `pr-check/`                | 9       | plus a `.gitkeep`          |
| `visual-check/`            | 46      | plus a `.gitkeep`          |
| **Total**                  | **247** | matches STATUS.md          |

### 7.2 Front matter and configuration (from `classify3.py`)

| Item                                                               | Count (files)                                                                                              |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------- |
| `htmlLabels: true` / `false` / unset                               | 145 / 50 / 52 (default is true)                                                                            |
| `look: handDrawn` / `neo` / unset                                  | 67 / 30 / 150 (unset → **neo** for class)                                                                  |
| `theme` set                                                        | 42: `redux-color` 7, `redux-dark-color` 7, `redux-dark` 7, `redux` 7, `neo` 7, `neo-dark` 7                |
| `theme` names not shipped by ferrigura (fallback by name, M2 rule) | `redux`, `redux-dark`, `neo`, `neo-dark` = 28                                                              |
| `layout: elk`                                                      | 2                                                                                                          |
| `themeVariables`                                                   | 17                                                                                                         |
| `class:` section in front matter                                   | 5                                                                                                          |
| `hideEmptyMembersBox`                                              | 2                                                                                                          |
| `hierarchicalNamespaces`                                           | 3                                                                                                          |
| `classDiagram-v2` header                                           | 28                                                                                                         |
| files with `%%{…}%%` in the body (after the front matter)          | 10; the ones that set a theme use `base` (with `themeVariables`), `neutral`, `dark`, `default` or `forest` |

The `visual-check` files described as "classic look" (for example `cardinality-classic-look-redux-dark-theme`) set no `look`, so they render **neo** (§6.2). The description is wrong about the default. The fixture's behaviour is what the reference renders, so this does not change the port, but it matters when someone reads the picture.

### 7.3 Features in the bodies (files, from `classify2.py`/`classify3.py`)

| Feature                               | Files                                                                                                                             |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| class bodies `{`                      | 185                                                                                                                               |
| member text with `:` label form       | 174                                                                                                                               |
| method parentheses `()`               | 165                                                                                                                               |
| relations by arrow                    | `-->` 124, `<\|--` 104, `o--` 84, `*--` 78, `..` 69                                                                               |
| quoted strings in the body            | 98 files; 56 files have a cardinality string on a relation line                                                                   |
| generics `~T~`                        | 46                                                                                                                                |
| namespace (any)                       | 24 (10 with a dotted name)                                                                                                        |
| `note` / `note for`                   | 17 / 17                                                                                                                           |
| `direction TB\|BT\|LR\|RL` statement  | 18                                                                                                                                |
| `cssClass`                            | 11                                                                                                                                |
| `:::` shorthand                       | 10                                                                                                                                |
| `style` statement                     | 9                                                                                                                                 |
| `classDef`                            | 6                                                                                                                                 |
| `callback` / `link` statements        | 4 / 4 (no `click`, no `href`, no `_blank` in bodies)                                                                              |
| `accTitle` / `accDescr`               | 4 / 4                                                                                                                             |
| Markdown (`**`, backtick)             | 9                                                                                                                                 |
| HTML entities `&lt;` `&gt;` in bodies | **98** files, all in the `&lt;&lt;…&gt;&gt;` annotation form, which the grammar reads as a member (§2.6 probe 13)                 |
| `<<annotation>>` in raw form          | 0                                                                                                                                 |
| lollipop `()--` / `--()`              | **0** (only method `()`)                                                                                                          |
| `%%` mid-line                         | 0 (only `%%{` directives)                                                                                                         |
| non-ASCII                             | `à` in 3 files (2 outside handdrawn); `—` (U+2014) in 17 files (12 outside handdrawn) **[unverified: Lexica coverage of U+2014]** |

### 7.4 Exclusions to expect

| Set                                  | Count                                    | Reason                                              |
| ------------------------------------ | ---------------------------------------- | --------------------------------------------------- |
| `look: handDrawn`                    | 67                                       | out of scope (P3), as M5's 50 flowchart handDrawn   |
| em dash U+2014                       | 12 outside handdrawn (17 with handdrawn) | depends on the font coverage check **[unverified]** |
| shipped-theme names not in ferrigura | 28 files                                 | fall back by name (M2)                              |

So about 180 fixtures are comparable, before the em dash question. Compare with M5's 298 of 352 flowchart fixtures.

---

## 8. Reuse and effort

### 8.1 What M5 already gives class

| Module (ferrigura-render/src/flowchart/)                                                             | Reuse for class                                               |
| ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `elk_build.rs`, `elk_tree.rs`                                                                        | the graph; extend with terminal labels                        |
| `elk_apply.rs` (node positions, group frames, edge clipping, straightening, label passes that exist) | all but the terminal passes                                   |
| `elk_geometry.rs`, `elk_straighten.rs`                                                               | as is                                                         |
| `label.rs`, `markdown.rs`, `shapes/util.rs` (`label_helper`, `styles2string`)                        | row text, labels, note                                        |
| `shapes/rough.rs`, `shapes/rect.rs`, `shapes/rects.rs` (`note`), `shapes/intersect`                  | `classBox` outline, dividers (roughness 0 path), notes        |
| `clusters.rs`                                                                                        | namespaces                                                    |
| `edges.rs`, `path.rs`, `line_jumps.rs`                                                               | relation paths, line hops                                     |
| `markers.rs`                                                                                         | only the id scheme and `marker_path_offset`; the defs are new |
| `style.rs` (`palette`, `palette_slot`, `node_style`, `class_rules`)                                  | colour slots, styles                                          |
| `mod.rs` (paint, SVG root, title, viewport)                                                          | parameterised                                                 |
| `ferrigura-text` (measure, wrap, styled runs)                                                        | row widths and the Markdown lines                             |
| `ferrigura-syntax/src/sanitize.rs`                                                                   | sanitising of class texts                                     |
| `ferrigura-elk`                                                                                      | the TAIL/HEAD labels are in the corpus                        |

### 8.2 What is new

| Item                                                                               | Estimate (Rust lines, without tests) | Notes                                                                     |
| ---------------------------------------------------------------------------------- | ------------------------------------ | ------------------------------------------------------------------------- |
| parser: scanner with 22 states and about 60 rules                                  | 500–650                              | `flowchart/scanner.rs` is 1044 lines for more constructs                  |
| parser: grammar                                                                    | 500–650                              | `flowchart/grammar.rs` is 1006                                            |
| DB state and setters (§2.9)                                                        | 450–600                              | `flowchart/db.rs` is 736                                                  |
| model structs (§3.1)                                                               | 150–200                              | `flowchart.rs` is 204                                                     |
| class `getData` (§5.2)                                                             | 200–300                              |                                                                           |
| `classBox`, `textHelper`, `addText` (§5.3)                                         | 500–700                              | the bbox union and the transforms are the risk                            |
| markers: 5 kinds × 4 variants (§5.4)                                               | 150–250                              |                                                                           |
| terminal labels: ELK build (80), passes (400–500), paint (100–150)                 | 600–700                              | `elk_apply.rs` is 2191 lines; the passes are about 440 lines of reference |
| class stylesheet and palette rules (§5.7)                                          | 250–300                              |                                                                           |
| namespace compact mode, interfaces, notes wiring                                   | 100–150                              |                                                                           |
| export and harness (`tools/port/export-class.ts`, a class geometry driver, inputs) | 200–300 (TypeScript)                 |                                                                           |

Reference sizes for comparison: `flow.jison` 634 lines, `classDiagram.jison` 440; `flowDb.ts` 1358, `classDb.ts` 874 (+202 types, +189 shapeUtil, +276 classBox, +244 styles, +82 renderer).

### 8.3 Effort relative to the flowchart

- **Parser**: about half of M3 in code. Simpler grammar (no shape data, no YAML, no `getData`), but the lexer has more states, and the quirks in §2.6 need probes for each. Port with the same method (probes for the rule order, then specs).
- **Renderer**: about half of M5 in new code, and nearly all of M5's infrastructure is reused. The hard parts are `classBox` (text bboxes of translated groups, the −6 px quirks) and the terminal-label passes (about 440 reference lines of geometry, all tested only by hand-made layouts).
- **Config**: small, but the `padding` override and the `state` reads are easy to miss and change every size.
- **Verification**: the geometry harness can be reused. Each row is a text box, so the label comparison (M5 "83 labels, 0.0000 px") gives early coverage of `classBox`.

---

## 9. Risks

| #   | Risk                                                                                                              | Likelihood                           | Mitigation                                                                                    |
| --- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------ | --------------------------------------------------------------------------------------------- |
| R1  | `classBox` heights depend on `getBBox` of translated groups; any slip in the bbox union moves every box           | high                                 | build the bbox from the same transforms; test against Chromium on single-class fixtures first |
| R2  | The −6 px quirks (`P/2` on empty groups) and the 54 px methods offset are easy to "fix"                           | high                                 | keep them, with a test per quirk, and a comment that cites the line                           |
| R3  | `padding` from the schema (5) instead of undefined                                                                | high if the config is not overridden | §6.1: override in `ts_defaults.json` or `Option`                                              |
| R4  | Terminal-label passes are many small geometric rules with exact comparisons (M5 found float32 ulp effects)        | high                                 | port with the reference's order (§5.5.2), test each pass on the 7 ignored tests               |
| R5  | Row wrap width `calculateTextWidth + 50` is DOM-measured in the reference (`sans-serif` vs `fontFamily`, rounded) | medium                               | use ferrigura measure + 50, rounded; check no fixture wraps in the reference                  |
| R6  | `sanitizeText` depends on `htmlLabels` and `securityLevel`; the harness forces htmlLabels false                   | medium                               | state in the model that the text is under the harness config (as flowchart)                   |
| R7  | `sanitizeUrl` for `link` is not ported                                                                            | low (4 fixtures)                     | port, or decide that the renderer sanitises                                                   |
| R8  | Palette on every class box in `redux-color` changes every fixture's colours                                       | medium                               | colours are not compared (RENDER-PLAN), but the stylesheet must still match                   |
| R9  | `state` section reads for spacing and title                                                                       | low to medium                        | read `state` as the reference does; add a test that `class.titleTopMargin` is ignored         |
| R10 | Mid-line `%%` and direction-swallowing lexer quirks: a port that "fixes" them changes the output                  | medium                               | keep them; probe cases in §2.6 as spec tests                                                  |
| R11 | Em dash coverage in the bundled font                                                                              | unknown                              | check the font's cmap (not done)                                                              |
| R12 | The fixture descriptions say "classic" but the render is neo                                                      | low                                  | document; rely on what the reference renders                                                  |

---

## 10. Open questions

1. **Canonical text under the harness.** Confirm that the class canonical texts are `DOMPurify.sanitize(text, {FORBID_TAGS:['style']})` with `htmlLabels: false` (as flowchart). Confirm that `sanitize.rs` covers the class inputs (HTML-like text in members).
2. **`sanitizeUrl` for links.** `formatUrl` calls `sanitizeUrl` from `@braintree/sanitize-url` unless `securityLevel` is `loose` (utils.ts:250–260, import at utils.ts:1). Port it, or keep the trimmed URL in the model and sanitise in the renderer? The flowchart has the same gap.
3. **Config `padding`.** Override in `ts_defaults.json`, or represent every class key that the reference does not default as `Option` with `None`? Either way, `class` must be checked by `pnpm port:diff config` in the `class` scope (the M5 note says the scope is compared only for `flowchart` and `elk`).
4. **Canonical `members`/`methods` fields.** Keep `text` (DB state) and the input fields, or only the input fields? §3.1 proposes both.
5. **Test count.** STATUS's "≈ 255" for class counts blocks. Confirm the convention for the flowchart's 958 (is it vitest's count with `it.each` expanded?), then recount class with `vitest list`.
6. **`state` section.** Confirm that reading `state` for class spacing is wanted (it is the reference's behaviour; the M6 decision is whether to reproduce it).
7. **Terminal labels in the model.** Keep the cardinality strings (`relationTitle1/2`) as in the DB, and derive `startLabelRight`/`endLabelLeft` in the renderer? Proposal: yes, as the DB does, with the `none` sentinel mapped as getData does.
8. **Interfaces.** The lollipop interface node takes the label of the _other_ end (`A ()-- I` labels the interface `A`). No fixture uses lollipops; port it from the grammar and DB, with a probe test.
9. **Em dash.** Check the bundled font for U+2014, and whether the fixtures that use it compare with a fallback.
10. **Relation `type` numbers vs names** in the model (§3.1).
11. **Blank lines in the accessibility block.** `commonDb.ts:23` (`\n\s+` → `\n`) collapses blank lines (checked in node). `canonical-model.md` (flowchart section) and `flowchart/PLAN.md` §8.3 say blank lines are not collapsed. Check which regex the flowchart DB uses and whether the M3 exporter test covers blank lines in `accDescr`; then fix the canonical-model text if it is wrong.

---

## 11. Unverified

- The Vite-based DB run (`getData` values, `sanitizeText` output, `calculateTextWidth` values): not run; from source.
- Reference `getBBox` values for any class: not measured (needs Chromium; `tools/port/geometry` has the flowchart harness; a class variant is not written).
- Full pipeline on `%%` mid-line (probe ran on raw input).
- Whether `class` is compared by `pnpm port:diff config` (the STATUS says only `flowchart` and `elk` sections are compared so far).
- Lexer behaviour after a class body's `}` on the same line (derived from the state table, not probed).
- Equality of the class `UNICODE_TEXT` ranges with `ferrigura-syntax/src/flowchart/unicode.rs` (not compared).
- `style_list` handling of `opacity: 0;` (interface rect).
- `noteTextColor`, `edgeLabelBackground` in ferrigura's themes (not grepped).
- That every class fixture renders without an error in the reference (the probes cover syntax only; some fixtures may not render).
- Rough.js line geometry at roughness 0 (the divider is `rc.line`; rough's `_doubleLine` writes two strokes with the same geometry, `node_modules/.pnpm/roughjs@4.6.6…/bin/renderer.js:237–247`; the ferrigura version would write one path).

---

## 12. How the probes were run

```
cd /private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/m6-class
/Users/sebastian/Workspace/ferraid/node_modules/.bin/tsx gen-parser.mjs          # writes classParser.mjs (reference grammar)
/Users/sebastian/Workspace/ferraid/node_modules/.bin/tsx probe-jison.mts '["classDiagram\nclass A\nA <|-- B"]'
python3 classify2.py ; python3 classify3.py                                       # fixture features, front matter
```

The Jison probe patches `lexer.next` (not `lex`, which recurses), so each token is printed once.
