# C4 (P2): research notes for the port

Status: research only. Nothing in the repository changed except this file (not committed). Reference: the JS reference at this branch's HEAD `aa5c4338f` (`sebastian/port-p2`). The bundle `packages/mermaid/dist` was built from this checkout on 2026-10-16 and was not rebuilt; probes ran against it.

Diagram types: the five headers `C4Context`, `C4Container`, `C4Component`, `C4Dynamic`, `C4Deployment`. They share one grammar, one DB and one renderer. The header only sets `c4Type`, and the renderer reads it in one place (numbering of `C4Dynamic` relations, `c4Renderer.ts:394-402`).

Verification legend: a rule cites the reference's file and line (`packages/mermaid/src/...`, shortened here to the path under `diagrams/` or `rendering-util/`). **probed** means the reference ran in Chromium or in the compiled grammar and the result is recorded; the probe name is given with it (section 12 for the rendering inputs, section 2.4 for the parser cases). **unverified** means read from the source only.

## 0. Summary

| Item              | Value                                                                                                                                                                                                                                                                                                                   |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference sources | 9 files in `diagrams/c4/` (2,714 lines with the grammar; 322 of them Jison) plus the shared shapes C4 uses: `c4LabelHelper.ts` (97), `person.ts` (132), `cylinder.ts` (154), `tiltedCylinder.ts` (160), `drawRect.ts` (83), `roundedRect.ts` (21). Text helpers in `utils.ts`.                                          |
| Specs             | 8 parser and DB files under `diagrams/c4/parser/`: 169 test cases once the `describe.each` rows are expanded (section 5). One label-helper spec with 2 cases. No renderer spec for the layout.                                                                                                                          |
| Fixtures          | 32 `.mmd` files in `e2e/diagrams/c4/` (20 nightly-check, 10 pr-check, 2 visual-check). None uses `look: handDrawn`, and none has text outside Latin.                                                                                                                                                                    |
| Parser            | Jison, exclusive start states, first match wins, keyword literals get a word boundary. A hand-written parser has to reproduce the lexer states and the greedy unquoted-attribute rule (section 2).                                                                                                                      |
| DB                | Plain module state, arrays of shapes, boundaries and relations. Mostly simple, with a few reuse rules that change output (section 3).                                                                                                                                                                                   |
| Renderer          | Its own grid layout (`Bounds`), boundary nesting, a stacked text label for each shape, and legacy text drawing for boundaries and relations. The grid compares absolute coordinates with `screen.availWidth`, so the output depends on the viewer's screen (section 6.3).                                               |
| Images            | None. The person is geometry (a `rect` and a `circle`), the database is a path. Three icon `symbol`s are written into every diagram and never referenced (section 6.12).                                                                                                                                                |
| Biggest risks     | (1) layout depends on `screen.availWidth`; (2) C4 text uses `"Open Sans", sans-serif`, which the harness does not override (its `fontFamily` directive does not reach the C4 section); (3) the parser's silent corruption rules; (4) shape sizes come from `getBBox` of text and of arc paths (section 8).              |
| Effort            | About 3,000 to 4,000 lines of Rust: parser and DB 1,000 to 1,300, renderer 1,100 to 1,400 (the grid, boundaries, relations, the stacked label and the adapter), model and exporter 250 to 350, shared changes 60 to 120, tests 600 to 800. Most shape and text code already exists in `ferrigura-render` (section 8.2). |

## 1. Sources

| File                                                                     | Lines                                                                      | What it holds                                                                                                                                                                                                                                                                                                                      |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | ----------- | --------- | ----------------------------------------------------------- |
| `diagrams/c4/parser/c4Diagram.jison`                                     | 1-322                                                                      | Lexer states 11-77, lexer rules 79-186, grammar 196-322                                                                                                                                                                                                                                                                            |
| `diagrams/c4/c4Db.ts`                                                    | 1-813                                                                      | DB: element, boundary and relation storage (`addPersonOrSystem` 151, `addContainer` 199, `addComponent` 259, `addPersonOrSystemBoundary` 319, `addContainerBoundary` 370, `addDeploymentNode` 421, `popBoundaryParseStack` 486, `updateElStyle` 494, `updateRelStyle` 593, `updateLayoutConfig` 641, getters 670-707, `clear` 721) |
| `diagrams/c4/c4Renderer.ts`                                              | 1-663                                                                      | Renderer: `Bounds` 69-172, `calcC4ShapeTextWH` 204-244, `drawBoundary` 246-280, `drawC4ShapeArray` 282-350, `drawRels` 387-433, `drawInsideBoundary` 435-560, `draw` 565-656                                                                                                                                                       |
| `diagrams/c4/svgDraw.ts`                                                 | 1-461                                                                      | Legacy drawing: `drawRels` 26-126, `drawBoundary` 135-211, icon `symbol`s 213-256, markers 261-340, text placements 342-446 (lines 213-256 and 342-446 include long path data; the `d` strings are not copied)                                                                                                                     |
| `diagrams/c4/c4ShapeAdapter.ts`                                          | 1-197                                                                      | Shape choice 69-93, stereotype text 95-114, element colours 142-156, `buildC4Node` 164-197                                                                                                                                                                                                                                         |
| `diagrams/c4/c4Types.ts`                                                 | 1-146                                                                      | Types. `C4Text.y` (line 15) is written and never read (`c4Renderer.ts:267`)                                                                                                                                                                                                                                                        |
| `diagrams/c4/styles.js`                                                  | 1-70                                                                       | Style sheet: per-type label fonts through CSSOM (8-34), static rules (36-68)                                                                                                                                                                                                                                                       |
| `diagrams/c4/c4Detector.ts`                                              | 1-24                                                                       | Detector `/^\s\*(?:C4Context                                                                                                                                                                                                                                                                                                       | C4Container | C4Component | C4Dynamic | C4Deployment)/` (line 10): a prefix match, no word boundary |
| `diagrams/c4/c4Diagram.ts`                                               | 1-18                                                                       | Definition. `init` calls `renderer.setConf(c4)` and `db.setWrap(wrap)` (lines 14-17)                                                                                                                                                                                                                                               |
| `rendering-util/rendering-elements/shapes/c4LabelHelper.ts`              | 1-97                                                                       | Stacked label of a C4 shape: one section per name, stereotype and description line                                                                                                                                                                                                                                                 |
| `rendering-util/rendering-elements/shapes/util.ts`                       | 50-59, 220-247                                                             | `labelHelper` sends any node with a `stereotype` to `c4LabelHelper` (line 57); `updateNodeBounds` reads `getBBox` (line 241)                                                                                                                                                                                                       |
| `rendering-util/rendering-elements/shapes/person.ts`                     | 13-132                                                                     | Person shape                                                                                                                                                                                                                                                                                                                       |
| `rendering-util/rendering-elements/shapes/cylinder.ts`                   | 9-25, 57-154                                                               | Database shape (path and sizes)                                                                                                                                                                                                                                                                                                    |
| `rendering-util/rendering-elements/shapes/tiltedCylinder.ts`             | 9-24, 57-160                                                               | Queue shape (`h-cyl`)                                                                                                                                                                                                                                                                                                              |
| `rendering-util/rendering-elements/shapes/drawRect.ts`, `roundedRect.ts` | 11-83, 5-21                                                                | Rounded rectangle (the default shape of C4 elements)                                                                                                                                                                                                                                                                               |
| `rendering-util/rendering-elements/shapes/handDrawnShapeStyles.ts`       | 75-87                                                                      | `styles2String`: every style gets `!important`                                                                                                                                                                                                                                                                                     |
| `diagrams/common/svgDrawCommon.ts`                                       | 17-46, 124-135                                                             | `drawRect` and `getTextObj` (the legacy rectangle and the measuring object)                                                                                                                                                                                                                                                        |
| `utils.ts`                                                               | 36, 467-489, 491-520, 539-578, 585-621, 634-655, 665-732, 823-839, 874-906 | `ZERO_WIDTH_SPACE` (36), `getTextObj` (467), `drawSimpleText` (491), `wrapLabel` (539), `breakString` (585), `calculateTextHeight` (634), `calculateTextWidth` (649), `calculateTextDimensions` (665), `parseFontSize` (823), `encodeEntities` and `decodeEntities` (874, 904)                                                     |
| `diagrams/common/common.ts`                                              | 8, 102-105                                                                 | `lineBreakRegex` = `/<\/?br\s*\/?>/gi`; `hasBreaks`                                                                                                                                                                                                                                                                                |
| `setupGraphViewbox.js`                                                   | 23-46                                                                      | `calculateSvgSizeAttrs`, `configureSvgSize`                                                                                                                                                                                                                                                                                        |
| `Diagram.ts`                                                             | 49-50, 77                                                                  | `db.clear()` then `init(config)` (so `init`'s `setWrap` runs after `clear`); `encodeEntities` before parsing                                                                                                                                                                                                                       |
| `preprocess.ts`, `diagram-api/comments.ts`                               | comments.ts 6-8                                                            | `cleanupComments` removes `%%` lines that do not start with `%%{`                                                                                                                                                                                                                                                                  |
| `mermaidAPI.ts`                                                          | 359, 419, 757                                                              | Decodes entity placeholders in the final SVG string (359); the generic `svg.append('g')` (419); accessibility title and description (757)                                                                                                                                                                                          |
| `schemas/config.schema.yaml`                                             | 507-~1000                                                                  | `C4DiagramConfig` (defaults in section 6.2)                                                                                                                                                                                                                                                                                        |
| `themes/theme-redux-color.js`                                            | 20                                                                         | `radius = 12`, the `rx` of the rounded rectangle                                                                                                                                                                                                                                                                                   |

## 2. Parser

### 2.1 Pipeline before the lexer

`Diagram.fromText` detects the type, calls `db.clear()`, then `init(config)`, then `encodeEntities(source) + '\n'` (`Diagram.ts:49-50, 77`). Front matter and `%%{init}` directives are removed by the generic preprocessing before that (not read here). `cleanupComments` removes whole `%%` lines (`comments.ts:6-8`). `encodeEntities` rewrites every `#name;` into a placeholder `ﬂ°name¶ß` (or `ﬂ°°123¶ß` for numbers), `utils.ts:874-887`. The placeholders reach the DB. Two places decode them: `c4LabelHelper.ts:61` decodes the element section texts before layout, and `mermaidAPI.ts:359` decodes the final SVG string, which is what the legacy texts (boundaries, relations, `techn`) see. Probed in the serialized SVG (`q18-entities`): an element label keeps the entity as text, so `#quot;S#quot;` is written as `&amp;quot;S&amp;quot;` and is drawn as the characters `&quot;S&quot;`, while `#amp;` is drawn as `&`. A legacy label is decoded by the final string decode, so `#quot;` is drawn as `"` and `#lt;b#gt;` as `<b>`. A raw `&amp;` in a boundary label is drawn as `&amp;`.

### 2.2 Lexer states (Jison, `%x` exclusive)

Every state is exclusive: a rule without a state prefix applies only in `INITIAL`.

| States                                                                                                                                                                                                                                                                                                                                                     | Declared at | Used by                                                                                                                                                                                     |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `person`, `person_ext`, the six `system*` and `system_ext*` kinds, `boundary`, `enterprise_boundary`, `system_boundary`, the six `container*` kinds and `container_boundary`, the six `component*` kinds, `node`, `node_l`, `node_r`, `rel`, `birel`, `rel_u/d/l/r/b`, `rel_index`, `index`, `update_el_style`, `update_rel_style`, `update_layout_config` | 11-69       | The element and relation macros begin a state (`this.begin(...)`), and the state runs until the closing `)` (line 164) pops it. `<<EOF>>` in any of them returns `EOF_IN_STRUCT` (line 161) |
| `attribute`, `string`, `string_kv`, `string_kv_key`, `string_kv_value`                                                                                                                                                                                                                                                                                     | 69-73       | Arguments                                                                                                                                                                                   |
| `acc_title`, `acc_descr`, `acc_descr_multiline`                                                                                                                                                                                                                                                                                                            | 75-77       | Accessibility text                                                                                                                                                                          |

Lexer rules, in order, first match wins (line 79 onwards):

1. Direction (81-84): `.*direction\s+TB[^\n]*`, and BT, RL, LR. The rule matches the whole line that contains the words, anywhere in the line.
2. `title` (87): `"title"\s[^#\n;]+`. The token is the whole rest of the line up to `#`, `;` or the end of the line, and `substring(6)` removes `title` plus one character.
3. `accDescription` (88), `accTitle`/`accDescr` with `:` (89-92), `accDescr {` (93-95).
4. Comments (98-99), newlines (101, `\s*(\r?\n)+` returns `NEWLINE`), spaces (102), the five headers (103-107).
5. Element macros (109-136), boundary macros (118-120, 129, 136), node macros (138-141), relation macros (144-155), update macros (157-159). Keywords are longer-first where a prefix exists (`Person_Ext` before `Person`, `SystemDb_Ext` before `SystemDb`).
6. In the structure states: `EOF_IN_STRUCT` (161), `(` followed by a comma returns `ATTRIBUTE_EMPTY` and begins `attribute` (162), `(` begins `attribute` (163), `)` pops two states (164).
7. In `attribute` (166-179): `,,` returns `ATTRIBUTE_EMPTY`; a comma is skipped (167); `""` returns `ATTRIBUTE_EMPTY` (168); `"` begins `string` (169); in `string`, `"` pops and `[^"]*` returns `STR` (170-171); `$` begins `string_kv` (173); `string_kv` reads a key up to `=` (174), the value is `[^"]+` (176); and `[^,]+` returns `STR` (179).
8. `{` and `}` return `LBRACE`/`RBRACE` (181-182); spaces (184), newlines (185), `<<EOF>>` (186).

Two rules decide most of the quirks below:

- A keyword literal ends with a word boundary unless `flex` is set, so `Node` does not match the start of `Node_L` (the gantt notes verified this in `jison-lex`, and the probes agree).
- `[^,]+` in the `attribute` state is greedy. It does not stop at `)` or at a newline. Any argument that is not quoted runs up to the next comma, however far that is.

### 2.3 Grammar and the DB calls it makes

| Rule                | Lines   | Action                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | -------------- |
| `graphConfig`       | 219-225 | `C4_* NEWLINE statements EOF` calls `setC4Type`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `statements`        | 227-231 | At least one statement. A header alone is a parse error (probed: `C4Context` followed by nothing)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `otherStatement`    | 239-245 | `title` calls `setTitle(substring(6))`; `accDescription` calls `setAccDescription(substring(15))`; `acc_title` and `acc_descr` values call `setTitle` and `setAccDescription` (242-243). **The `accTitle` value is stored as the visible title** (probed), which is why `mermaidAPI.spec.ts:856` has a TODO                                                                                                                                                                                                                                                                                                                                                          |
| `boundaryStatement` | 247-249 | `boundaryStart diagramStatements boundaryStopStatement`; an empty body is a parse error (probed)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `boundaryStart`     | 257-265 | `ENTERPRISE_BOUNDARY` splices `'ENTERPRISE'` in as argument 2; `SYSTEM_BOUNDARY` splices `'SYSTEM'`; `CONTAINER_BOUNDARY` splices `'CONTAINER'`; plain `BOUNDARY` passes its arguments on unchanged, so its third argument is the type (P15). `NODE`, `NODE_L`, `NODE_R` call `addDeploymentNode('node'                                                                                                                                                                                                                                                                                                                                                              | 'nodeL' | 'nodeR', ...)` |
| `diagramStatement`  | 277-310 | One DB call per macro, with the attributes spread. Kinds: `person`, `external_person`, `system`, `system_db`, `system_queue`, `external_system`, `external_system_db`, `external_system_queue`, `container`, `container_db`, `container_queue`, `external_container`, `external_container_db`, `external_container_queue`, `component`, `component_db`, `component_queue`, `external_component`, `external_component_db`, `external_component_queue`. Relations: `rel`, `birel`, `rel_u`, `rel_d`, `rel_l`, `rel_r`, `rel_b`. `Rel_Up` and `Rel_U` are the same token (`REL_U`); `Rel_Down`/`Rel_D`, `Rel_Left`/`Rel_L` and `Rel_Right`/`Rel_R` pair up the same way |
| `REL_INDEX`         | 306     | `RelIndex` drops its first argument (the index) and calls `addRel('rel', ...)`. The number comes from the renderer                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `update*`           | 307-309 | `UpdateElementStyle`, `UpdateRelStyle`, `UpdateLayoutConfig`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `attribute`         | 317-322 | `STR` (trimmed), `STR_KEY STR_VALUE` (a one-entry object `{key: value}`, both trimmed), `ATTRIBUTE` (trimmed), `ATTRIBUTE_EMPTY` (`""`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |

Start rules: `start` is `mermaidDoc` or `direction` (198-212). `direction` calls `yy.setDirection`, which the DB does not export (`c4Db.ts:778-813`). A file that is only a `direction` line never reaches the grammar because the detector needs a header.

### 2.4 Quirks a port must reproduce (every line probed unless marked)

Each row gives the input, the probe name (a case in `parser/cases1.json` to `cases5.json`, or an input file in `inputs/`, section 12) and what the reference does.

| #   | Input                                                                                                                     | Result                                                                                                                                                                                                 |
| --- | ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| P1  | `Person (a, "A")` (space before `(`)                                                                                      | Lexical error: no rule matches in state `person` (`space-before-paren`)                                                                                                                                |
| P2  | `Rel(a, b)` followed by another line (`rel-no-label-then-more`)                                                           | No error. The relation's target becomes `b)\nSystem(c`, and `System(c, ...)` is lost. Unquoted arguments run to the next comma (line 179)                                                              |
| P3  | `Rel(a, b)` at the end of the file (`rel-no-label-at-end`)                                                                | Parse error                                                                                                                                                                                            |
| P4  | `System(a)` (one argument)                                                                                                | Parse error (`alias-only`), for the same greedy reason                                                                                                                                                 |
| P5  | `System(a, "A" )` followed by another line (`space-before-close-paren`)                                                   | No error. The space is read as an unquoted argument, so `descr` becomes `)\nSystem(b` and `sprite` becomes `B`. The same input as the last argument is a parse error (`space-before-close-paren-last`) |
| P6  | `System(a, "A" , "desc")` (`space-before-comma`)                                                                          | No error. The space is a positional argument, so `descr` is `""` and `sprite` is `"desc"`                                                                                                              |
| P7  | `System(a, Label)` or `System(a, Label, "d")`                                                                             | Unquoted text in the last position is a parse error. Unquoted text in the middle works (`unquoted-middle`)                                                                                             |
| P8  | `System(a, "go direction TB now")` (`direction-word-in-label`)                                                            | Parse error: rule 1 (line 81) matches the line                                                                                                                                                         |
| P9  | `title A;B` or `title A#B` (`title-semicolon`, `title-hash`)                                                              | Lexical error (rule 2 stops at `#` and `;`, and nothing else matches)                                                                                                                                  |
| P10 | `title   Two  spaces` (`title-leading-spaces`)                                                                            | Title `"  Two  spaces"`: one blank removed by the rule, the rest kept. Trailing blanks are kept too (`title-trailing-spaces`)                                                                          |
| P11 | `System(a, "say \"hi\"")` (`escaped-quote`)                                                                               | Parse error. Strings have no escapes                                                                                                                                                                   |
| P12 | `$descr=""` (`descr-only-named-no-value`)                                                                                 | Parse error (`string_kv_value` needs one character)                                                                                                                                                    |
| P13 | `System(a, "A", "d", $tags = "v1")` (`key-space-before-equals`)                                                           | Works: the key is trimmed                                                                                                                                                                              |
| P14 | `System_Boundary(sb, "SB", "kind") {...}` (`system-boundary-positional`)                                                  | Type `SYSTEM`; `"kind"` goes to `tags`. The same for `Enterprise_Boundary` (`ENTERPRISE`) and `Container_Boundary` (`CONTAINER`)                                                                       |
| P15 | `Boundary(bb, "BB", "kind")` (`bare-boundary-type-arg`)                                                                   | Type `kind`                                                                                                                                                                                            |
| P16 | `Node_L(srv, "App Server", "Ubuntu 22.04", "descr")` (`node-l-positional`)                                                | Type `Ubuntu 22.04` (the third argument), `descr` `descr`, `nodeType` `nodeL`                                                                                                                          |
| P17 | `Node(n, "N") {` on the same line or `{` on the next (`node-brace-same-line`, `node-brace-next-line`)                     | Both work                                                                                                                                                                                              |
| P18 | `Enterprise_Boundary(e, "E") {` followed by `}` on the next line (`empty-boundary`)                                       | Parse error                                                                                                                                                                                            |
| P19 | `System(a, "")` (`empty-label`)                                                                                           | Label `""`, no error                                                                                                                                                                                   |
| P20 | Re-declared alias with a different macro (`duplicate-alias-kind-change`): `System(a,"A","desc")` then `Container(a,"C2")` | One object: label `C2`, type `container`, `descr` `desc` kept (the new macro gives no `descr`, so the old one stays), `techn` `""`                                                                     |
| P21 | `Rel(a,b,"x")` then `BiRel(a,b,"y")` (`rel-merge-birel`)                                                                  | One relation, type `birel`, label `y`. Relations merge on `(from, to)` whatever their type (`c4Db.ts:112-117`)                                                                                         |
| P22 | `RelIndex(7, a, b, "lbl")` (`relindex-splice`)                                                                            | Type `rel`, label `lbl`                                                                                                                                                                                |
| P23 | `Rel_Back(a, b, "x")`                                                                                                     | Type `rel_b`                                                                                                                                                                                           |
| P24 | `UpdateLayoutConfig($c4ShapeInRow="0", $c4BoundaryInRow="2x")`                                                            | Shape row stays 4 (0 is below 1), boundary row becomes 2 (`parseInt`). `"abc"` is `NaN`, which is ignored (`c4Db.ts:662-667`)                                                                          |
| P25 | `UpdateElementStyle(nobody, $bgColor="red")`                                                                              | Ignored, no error (`c4Db.ts:510-514`)                                                                                                                                                                  |
| P26 | `accTitle: Hello there`, `accDescr: described` (`acc-title-sets-title`; `inputs/q13-acc-title`)                           | The title becomes `Hello there` and the description becomes a `<desc>`. There is no `<title>` element                                                                                                  |
| P27 | `accDescr { line one` / `  line two }`                                                                                    | Value `"line one\n  line two"`: inner indentation is kept, only the ends are trimmed                                                                                                                   |
| P28 | Title given twice                                                                                                         | The last one wins                                                                                                                                                                                      |
| P29 | `%% comment` after a macro                                                                                                | Comment, no error                                                                                                                                                                                      |
| P30 | Name of an element is a keyword (`System(System, "System")`)                                                              | Works                                                                                                                                                                                                  |
| P31 | `Boundary(global, "G")` or a boundary re-declared inside itself (`q14-self-nesting`, `q15-global-alias`)                  | No error, and the diagram renders with nothing in it. The re-declared boundary gets itself as parent, so `getBoundaries('')` returns nothing (`c4Db.ts:696-703`)                                       |
| P32 | `Rel(a, "b")`, two arguments (`rel-two-args`, `rel-two-args-then-shape`)                                                  | No error and no relation: `addRel` returns when the label is `undefined` (`c4Db.ts:98-109`). The shapes around it are parsed normally                                                                  |

## 3. DB

### 3.1 Module state and `clear()`

All state is module-level (`c4Db.ts:64-74`): `c4ShapeArray`, `boundaries`, `rels`, `title`, `wrapEnabled`, `c4ShapeInRow = 4`, `c4BoundaryInRow = 2`, `boundaryParseStack = ['']`, `currentBoundaryParse = 'global'`, `parentBoundaryParse = ''`, and `c4Type`.

`clear()` (`c4Db.ts:721-734`) resets the arrays, the boundaries (with a new global boundary), the parse stack, `title`, `wrapEnabled = false` and both row counts. It does **not** reset `c4Type` (line 74 is outside `clear`). Every successful parse sets it again, so this only shows after a failed parse.

Order at parse time (`Diagram.ts:49-50`): `clear()`, then `init(config)`, which runs `setWrap(config.wrap)` (`c4Diagram.ts:16`). So the `wrap` flag each element stores is the top-level `wrap` setting. The parser specs call `clear()` only, so their elements show `wrap: false`.

### 3.2 Global boundary

`createGlobalBoundary` (`c4Db.ts:53-62`): alias `global`, label `global`, type `global`, `tags: null`, `link: null`, `parentBoundary: ''`. It is in `boundaries` before any declaration. Its label and type are measured and drawn into the layout offset of the first row, although the global boundary itself is never drawn (section 6.4).

### 3.3 Calls, what they store, and sanitising

No DB text goes through `sanitizeText` except the diagram title and `c4Type` (`setTitle`, `c4Db.ts:773-776`; `setC4Type`, 80-83). The accessibility texts go through the common setters. Shape, boundary and relation texts are stored raw, with entity placeholders (section 2.1). Labels are sanitised and decoded only at render time (`c4LabelHelper.ts:61`).

| Call                                                                                                                     | Stores                                                                                  | Defaults and notes                                                                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `addRel(type, from, to, label, techn?, descr?, sprite?, tags?, link?)` (86-148)                                          | Relation keyed on `(from, to)`: an existing one is reused and its fields overwritten    | Returns early if any of type, from, to, label is `undefined` or `null`. Reachable: `Rel(a, "b")` has no third quoted argument, so the label is `undefined` and the relation is dropped (P32). `techn` and `descr` become `{text}` when positional or when the key is a text field, otherwise the named value is stored as given. Empty `techn`/`descr` become `{text: ''}` if not already set. `wrap = autoWrap()` |
| `addPersonOrSystem(type, alias, label, descr?, sprite?, tags?, link?)` (151-196)                                         | Element reused by alias (any kind)                                                      | `label` `{text}` (`''` if absent). `descr` `{text}` (`??=` so an earlier value stays). `typeC4Shape = {text: type}`. `parentBoundary = currentBoundaryParse`. `wrap`                                                                                                                                                                                                                                               |
| `addContainer(...)` (199-256), `addComponent(...)` (259-316)                                                             | Same, with `techn` before `descr`                                                       | `techn` `{text}` (`??=`)                                                                                                                                                                                                                                                                                                                                                                                           |
| `addPersonOrSystemBoundary(alias, label, type?, tags?, link?)` (319-367)                                                 | Boundary reused by alias; pushes the parse stack                                        | `type` default `system` (349-351)                                                                                                                                                                                                                                                                                                                                                                                  |
| `addContainerBoundary(...)` (370-418)                                                                                    | Same                                                                                    | `type` default `container` (400-402)                                                                                                                                                                                                                                                                                                                                                                               |
| `addDeploymentNode(nodeType, alias, label, type?, descr?, sprite?, tags?, link?)` (421-484)                              | Same, plus `nodeType`                                                                   | `type` default `node` (454-456); `descr` `??=`                                                                                                                                                                                                                                                                                                                                                                     |
| `popBoundaryParseStack()` (486-491)                                                                                      | Restores the current boundary                                                           | The grammar makes the braces match, so it is never called on an empty stack                                                                                                                                                                                                                                                                                                                                        |
| `updateElStyle(type, name, bg?, font?, border?, shadow?, shape?, sprite?, techn?, legendText?, legendSprite?)` (494-590) | Style fields on a shape, or on a boundary when no shape has that alias (lookup 507-515) | Values are stored as bare strings, not `{text}`: named ones as `old[key] = value`, and a positional `techn` as `old.techn = techn` (`c4Db.ts:569-571`). Probed: `$techn="X"` removes the technology from the stereotype (section 6.5)                                                                                                                                                                              |
| `updateRelStyle(type, from, to, textColor?, lineColor?, offsetX?, offsetY?)` (593-638)                                   | Style fields on the first relation with that `(from, to)`                               | Named `$offsetX` lands in the `textColor` slot (a one-entry object), so it is stored as the raw string (`old[key] = value`, line 609). Only positional offsets are `parseInt`ed (lines 625-635)                                                                                                                                                                                                                    |
| `updateLayoutConfig(type, shapeRow, boundaryRow)` (641-668)                                                              | Sets the module rows if the value is at least 1                                         | `parseInt` on each value. The DB keeps the rows it is given; the grammar's `UpdateLayoutConfig` is the only writer                                                                                                                                                                                                                                                                                                 |
| `setWrap`, `autoWrap` (713-719)                                                                                          | The top-level `wrap` flag                                                               | Used only when storing elements                                                                                                                                                                                                                                                                                                                                                                                    |
| `setTitle` (773-776), `setC4Type` (80-83)                                                                                | Sanitised                                                                               | `title` is not trimmed                                                                                                                                                                                                                                                                                                                                                                                             |

### 3.4 Other DB rules that reach the output

- A re-declared alias keeps every field that the new declaration does not set (`??=` for `descr` and `techn`, and `assignAttributes` skips `undefined`, `c4Db.ts:35-38`). Probed: `System(a,"A","desc")` then `Container(a,"C2")` keeps `desc` (P20).
- Re-declaring a boundary alias sets its `parentBoundary` to the current boundary. A boundary that becomes its own parent, or the `global` alias, disappears from the tree (P31).
- `getC4Shape(alias)` checks shapes first, then boundaries (`c4Db.ts:686-692`, the regression of issue 4864). Relations can therefore end at a boundary (`pr-check/c4-9`).
- `getC4ShapeArray(parent)` and `getBoundaries(parent)` filter by `parentBoundary` and keep insertion order (`c4Db.ts:677-703`). The renderer relies on that order for placement.

## 4. Canonical model proposal

The differential compares a JSON form (camelCase, `version: 1`, as the P1 models). The exporter reads the reference's DB after `parse` and before `draw`, with `clear()` and `init()` run as `Diagram.fromText` does. Layout fields are absent because they are written by the renderer on the same objects. The exporter must not render first.

```json
{
  "version": 1,
  "c4Type": "C4Context",
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "c4ShapeInRow": 4,
  "c4BoundaryInRow": 2,
  "shapes": [
    {
      "alias": "a",
      "typeC4Shape": "system",
      "label": "A",
      "descr": "",
      "techn": "...",
      "sprite": "...",
      "tags": "...",
      "link": "...",
      "parentBoundary": "global",
      "wrap": false,
      "bgColor": "...",
      "fontColor": "...",
      "borderColor": "...",
      "shadowing": "...",
      "shape": "...",
      "legendText": "...",
      "legendSprite": "..."
    }
  ],
  "boundaries": [
    { "alias": "global", "label": "global", "type": "global", "parentBoundary": "", "wrap": false },
    {
      "alias": "b",
      "label": "B",
      "type": "system",
      "nodeType": "node",
      "descr": "...",
      "tags": "...",
      "link": "...",
      "sprite": "...",
      "parentBoundary": "global",
      "wrap": false,
      "bgColor": "...",
      "fontColor": "...",
      "borderColor": "...",
      "shadowing": "...",
      "shape": "...",
      "legendText": "...",
      "legendSprite": "..."
    }
  ],
  "rels": [
    {
      "type": "rel",
      "from": "a",
      "to": "b",
      "label": "x",
      "techn": "",
      "descr": "",
      "sprite": "...",
      "tags": "...",
      "link": "...",
      "wrap": false,
      "textColor": "...",
      "lineColor": "...",
      "offsetX": "5",
      "offsetY": "-10"
    }
  ]
}
```

Decisions behind the proposal:

- **Text fields are strings.** The DB's `{text}` wrapper is dropped for `label`, `descr`, `techn`, `type`. The wrapper matters only for the bare-string case below.
- **Order is part of the model.** `shapes`, `boundaries` and `rels` are lists in insertion order, which is the order the reference's arrays hold. The global boundary is included because it is in the DB and it drives the first row.
- **Bare strings.** When the DB holds a bare string where a text field is expected (`UpdateElementStyle` positional or named `$techn`, section 3.3), the canonical `techn` is absent. The reference's stereotype reads `techn.text`, which is then `undefined` (`c4ShapeAdapter.ts:112-113`). Open question 4.
- **Offsets are strings.** `offsetX` and `offsetY` keep the DB's raw value, because named offsets are strings there. The renderer applies `parseInt` as the reference does (`svgDraw.ts:35-36`). A non-numeric offset gives `NaN` (section 6.7).
- **`wrap` is a boolean.** `undefined` and `false` are the same for the renderer (`wrap && conf.wrap`), so the exporter writes `false` for both.
- **`link` and `tags`** are absent when the DB holds `undefined`. The global boundary's `null` values are written as `null`.
- **Left out:** the render fields (`x`, `y`, `width`, `height`, `margin`, `image`, `intersect`, `startPoint`, `endPoint`, `textLines`, `Y`, and the `width`/`height` of each text), the parser state (`boundaryParseStack`, `parentBoundaryParse`, `currentBoundaryParse`), and the renderer's counters. The dynamic numbering (`1: ...`) is not in the DB, so the model keeps the label as written.
- **Numbers.** `c4ShapeInRow` and `c4BoundaryInRow` are numbers.

The fields of `typeC4Shape`, `type` (for boundaries) and `nodeType` are the strings listed in section 2.3.

## 5. Specs

### 5.1 Files and counts

| File                               | `describe` rows                                                        | Cases                  | Tests       |
| ---------------------------------- | ---------------------------------------------------------------------- | ---------------------- | ----------- |
| `parser/c4Boundary.spec.ts`        | 1 (`Boundary`)                                                         | 7                      | 7           |
| `parser/c4Component.spec.ts`       | 6 (`Component`, `ComponentDb`, `ComponentQueue`, and the three `_Ext`) | 2                      | 12          |
| `parser/c4Container.spec.ts`       | 6 (the container kinds)                                                | 8                      | 48          |
| `parser/c4Diagram.spec.ts`         | none                                                                   | 3                      | 3           |
| `parser/c4NamedAttributes.spec.ts` | none                                                                   | 8 literal, 3 `it.each` | 42 (8 + 34) |
| `parser/c4Person.spec.ts`          | none                                                                   | 8                      | 8           |
| `parser/c4PersonExt.spec.ts`       | none                                                                   | 7                      | 7           |
| `parser/c4System.spec.ts`          | 6 (the system kinds)                                                   | 7                      | 42          |
| **Total**                          |                                                                        |                        | **169**     |

`rendering-util/rendering-elements/shapes/c4LabelHelper.spec.ts` has 2 cases (lines 47 and 55), run in jsdom with a fake `getBBox` (7 px per character). They check that element labels wrap by default and that `c4.wrap: false` stops that.

### 5.2 What each test checks

- All eight files parse with `c4Db` and assert on the DB. They cover the header and title, each element, its alias, label, technology, description, sprite, link and tags, boundary fields, relation endpoints (`c4Boundary.spec.ts:114-133`, the #4864 regression), and the named-attribute slots (`c4NamedAttributes.spec.ts`: each argument can arrive through a positional text slot, and the value must still be a string for `tags`, `sprite` and `link` and `{text}` for the others, 91-145).
- The parser specs do not run the renderer, and no spec covers the layout of boundaries, relations or the grid. The layout is checked only by the 32 fixtures and the geometry harness.
- The label-helper spec covers the wrap of element labels only.

### 5.3 Other suites that touch C4

- `styles.spec.ts:17, 90`: the C4 style function is in the common style test.
- `rendering-util/multi-diagram-id-uniqueness.spec.ts:160`: a C4 sample in the id uniqueness test.
- `mermaidAPI.spec.ts:856`: the C4 case is commented out with the TODO "setAccTitle not called in C4 jison parser" (section 2.3, P26).
- `usecaseShapes.spec.ts:227-228, 411`: the usecase actor must not get the `.c4-name` or `.c4-type` sections. The check depends on the stereotype dispatch in `labelHelper` (section 8.1, risk 6).

The titles of the 169 cases are the spec for the parser port. Keep them when the specs are ported.

## 6. Renderer

### 6.1 Entry, inputs and the pipeline

`draw(text, id, version, diagObj)` (`c4Renderer.ts:565-656`):

1. `conf = getRequiredConfig('c4')` (566). The renderer's `conf` object is replaced on every draw.
2. `db.setWrap(conf.wrap)` (573), which changes the flag for later parses only. The shapes already parsed keep their own flags.
3. `c4ShapeInRow` and `c4BoundaryInRow` are copied from the DB (575-576).
4. `diagram = root.select('[id=id]')`. The three icon `symbol`s are inserted (582-584).
5. `screenBounds` is seeded at `(diagramMarginX, diagramMarginY)` and `widthLimit = screen.availWidth` (586-595).
6. `drawInsideBoundary(diagram, '', screenBounds, db.getBoundaries(''), diagObj)` (603). Its recursion draws the boundaries, their shapes and their child boundaries, in declaration order per parent.
7. `insertArrowHead`, `insertArrowEnd`, `insertArrowCrossHead`, `insertArrowFilledHead` (608-611).
8. `drawRels(diagram, db.getRels(), db.getC4Shape, diagObj, id)` (613).
9. The size: `width = (globalMaxX - startX) + 2*diagramMarginX`, `height = (globalMaxY - startY) + 2*diagramMarginY` (615-630).
10. The title, if any: `text` at `x = boxWidth/2 - 4*diagramMarginX`, `y = startY + diagramMarginY` (632-638). It has no class.
11. `configureSvgSize(diagram, height, width, useMaxWidth)` (640), then `viewBox = "{startX - marginX} -{marginY + extra} {width} {height + extra}"`, where `extra = 60` when there is a title (642-653).

Nothing in the renderer is random or reads the clock. The C4 code has no `Math.random`, `Date` or `performance`.

### 6.2 Configuration

Defaults from `schemas/config.schema.yaml` (`C4DiagramConfig` starts at line 507):

| Key                                                      | Default                                 | Line        | Read by the renderer                                                                                                                                                                                                |
| -------------------------------------------------------- | --------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagramMarginX`                                         | 50                                      | 525         | Yes: the screen margin and the gap between boundaries                                                                                                                                                               |
| `diagramMarginY`                                         | 10                                      | 531         | Yes                                                                                                                                                                                                                 |
| `c4ShapeMargin`                                          | 50                                      | 537         | Yes: the gap around each shape and boundary                                                                                                                                                                         |
| `c4ShapePadding`                                         | 20                                      | 542         | Yes: padding of each shape                                                                                                                                                                                          |
| `width`                                                  | 216                                     | 547         | Yes: width of each element (the minimum box)                                                                                                                                                                        |
| `height`                                                 | 60                                      | 552         | Only as a fallback, never used (every element sets its height)                                                                                                                                                      |
| `boxMargin`                                              | 10                                      | 557         | No                                                                                                                                                                                                                  |
| `c4ShapeInRow`                                           | 4                                       | 562         | No: the DB hard-codes 4 (`c4Db.ts:72`). Only `UpdateLayoutConfig` changes it                                                                                                                                        |
| `c4BoundaryInRow`                                        | 2                                       | 571         | No: as above (`c4Db.ts:73`)                                                                                                                                                                                         |
| `nextLinePaddingX`                                       | 0                                       | 567         | Yes: added to the x of a wrapped row                                                                                                                                                                                |
| `wrap`                                                   | true                                    | 851         | Yes: the element label wrap (`c4LabelHelper.ts:52`). The legacy boundary and relation wrap needs this key and the element's flag (top-level `wrap`, section 3.1: `boundary.wrap && conf.wrap`, `c4Renderer.ts:269`) |
| `wrapPadding`                                            | 10                                      | 855         | No                                                                                                                                                                                                                  |
| `useMaxWidth`                                            | true (BaseDiagramConfig)                | 464         | Yes: `width="100%"` and `max-width` (probed)                                                                                                                                                                        |
| `<type>_bg_color`, `<type>_border_color`                 | per type (table below)                  | 863 onwards | Yes: fill and stroke of each element                                                                                                                                                                                |
| `<type>FontFamily`, `<type>FontSize`, `<type>FontWeight` | `"Open Sans", sans-serif`, 14, `normal` | 584 onwards | Yes: the CSS of each element type (`styles.js`)                                                                                                                                                                     |
| `boundaryFont*`                                          | `"Open Sans", sans-serif`, 14, `normal` | 676         | Yes: boundaries (the label is `+2` and bold)                                                                                                                                                                        |
| `messageFont*`                                           | `"Open Sans", sans-serif`, 12, `normal` | 688         | Yes: relations                                                                                                                                                                                                      |

Element colours: `person` `#08427B`/`#073B6F`, `external_person` `#686868`/`#8A8A8A`, `system` `#1168BD`/`#3C7FC0`, `external_system` `#999999`/`#8A8A8A`, `container` `#438DD5`/`#3C7FC0`, `external_container` `#B3B3B3`/`#A6A6A6`, `component` `#85BBF0`/`#78A8D8`, `external_component` `#CCCCCC`/`#BFBFBF`. The `_db` and `_queue` kinds have the colours of their base kind in the schema.

Keys read but not in the schema: `c4.fontFamily`, `c4.fontSize`, `c4.fontWeight` (they set only the person, system and message font triple, `c4Renderer.ts:177-185`).

### 6.3 The grid (`Bounds`, `c4Renderer.ts:69-172`)

A `Bounds` holds two boxes, `data` (the extent of what has been drawn) and `nextData` (the cursor for the next shape). Both have `startx`, `stopx`, `starty`, `stopy`, and `nextData` also has `cnt`. `setData(startx, stopx, starty, stopy)` (93-98) sets both boxes. It does **not** reset `cnt`. `updateVal` takes the min for the starts and the max for the stops (100-111).

`insert(shape)` (113-147), with `m = c4ShapeMargin` (50) and the shape's width `w` and height `h` from the measuring pass:

```text
cnt        = cnt + 1
startx     = (nextData.startx === nextData.stopx) ? nextData.stopx + m : nextData.stopx + 2m
starty     = nextData.starty + 2m
stopx      = startx + w ; stopy = starty + h
if startx >= widthLimit or stopx >= widthLimit or cnt > c4ShapeInRow:     # 125
    startx      = nextData.startx + m + nextLinePaddingX                  # 126
    starty      = nextData.stopy + 2m                                     # 127
    nextData.stopx = stopx = startx + w                                   # 129
    nextData.starty = nextData.stopy                                      # 130
    nextData.stopy = stopy = starty + h                                   # 131
    cnt = 1                                                               # 132
shape.x = startx ; shape.y = starty                                       # 135-136
updateVal(data, ...) and updateVal(nextData, ...)                          # 138-146
```

Three properties of this grid matter for the port:

- **`widthLimit` is compared with absolute coordinates.** The top level uses `screen.availWidth` (line 595). A child level uses the parent's `widthLimit` divided by `min(c4BoundaryInRow, number of sibling boundaries)` (line 445-446). A row of shapes wraps when its right edge reaches that value, even if the shape is near the left of its boundary. Probed: four 216 px systems on a 1200 px screen fit three to a row, the fourth wraps (`q3-four-row`). The same four on a 1920 px screen fit four to a row (probe2, 1920). With two sibling boundaries on a 1200 px screen the limit is 600 for both, every shape wraps, and the shapes of both boundaries stack in one column (`q1-sibling-cnt`).
- **`cnt` is shared by the siblings of one level.** The shapes of every child boundary of one parent go through the same `Bounds` (`currentBounds` in `drawInsideBoundary`, line 443), so the counter does not restart for each sibling. Probed: with `c4BoundaryInRow = 1`, two shapes in the first boundary and three in the second make the third shape of the second wrap (`q1b-sibling-cnt-row1`, `b3` on a new row).
- **A wrapped row keeps the boundary's left edge.** The wrap x is the row's left edge plus `m` (`nextData.startx`), not the previous shape.

### 6.4 Boundaries (`drawInsideBoundary`, `c4Renderer.ts:435-560`)

The function is recursive. For one parent `P` (the screen or a boundary), it creates `cur = new Bounds()` with the widthLimit above, then for each child boundary `C` in order (index `i`):

1. **Header height `Y`** (452-502). If `C` has a sprite, `Y = 48` (the image box is 48 by 48, which reserves the space, but nothing is drawn; unverified). Then the label: `label.Y = Y + 8`, `Y = label.Y + label.height`. The label is measured with the boundary label font (`size + 2`, bold) and the width limit (line 471). Then the type: if non-empty, its text becomes `[type]` (line 477), `type.Y = Y + 5`, `Y = type.Y + type.height`. Then the description if non-empty: `descr.Y = Y + 20`, `Y = descr.Y + descr.height`.
2. **Position** (504-519). Index 0, or an index that is a multiple of `c4BoundaryInRow`: a new row at `x = P.startx + marginX`, `y = P.stopy + marginY + Y` (506-507). Otherwise the next column: `x = cur.stopx + marginX` if `cur.stopx !== cur.startx`, else `cur.startx`; `y = cur.starty` (513-516). Then `cur.setData(x, x, y, y)`.
3. **Own shapes**: `drawC4ShapeArray(cur, ...)` for the shapes whose `parentBoundary` is `C` (521-531).
4. **Children**: `drawInsideBoundary(diagram, C, cur, children of C)` (532-544). This writes the extents of the grandchildren into `cur.data`.
5. **Box**: `drawBoundary(C, cur)` unless `C` is `global` (546-548). The box is `x = cur.startx`, `y = cur.starty`, `width = cur.stopx - cur.startx`, `height = cur.stopy - cur.starty`.
6. **Parent extents** (549-558): `P.stopy = max(cur.stopy + m, P.stopy)`, the same for x, and `globalMaxX/Y` take the maximum.

The `global` boundary goes through the same steps but is not drawn. Its label `global` and type `[global]` are measured, which moves every first row down by their height. The first boundary's top is `10 + 10 + Y_global`, where `Y_global` = 8 + label height + 5 + type height. The probed positions of `pr-check/c4-1` agree with this.

In `pr-check/c4-1` the outer boundary top is 124. The header of that boundary is 47 (a 16 px bold label of line height 18, then 8 and 5 of gap, then a 16 px type line), so 124 = (10 + 10 + 47) + 10 + 47. The shapes inside start at the boundary top plus `2m` (224 = 124 + 100, probed).

`drawBoundary` (`svgDraw.ts:135-211`) writes, in order: a `g`, a `rect` (`x`, `y`, `fill` = `bgColor` or `none`, `stroke` = `borderColor` or `#444444`, `width`, `height`, `rx=2.5`, `ry=2.5`, `stroke-width=1`, `stroke-dasharray=7.0,7.0`), then the label, type and description as text (section 6.8). A deployment node (`nodeType` set) has no `stroke-dasharray` (probed). The label text is always `#444444`: a `$fontColor` on a boundary does not change it (probed: `q11-boundary-style`), because the legacy text placement does not use `fontColor`.

`drawBoundary` also sets `boundary.label.y = c4ShapeMargin - 35` (`c4Renderer.ts:267`). Nothing reads it (`C4Text.y`, `c4Types.ts:14-15`).

Title: none of the layout uses it, except the 60 px added to the viewBox.

### 6.5 Shapes (`drawC4ShapeArray`, `c4Renderer.ts:282-350`, and the adapter)

Each element is drawn twice:

1. **Measure**: `buildC4Node` (`c4ShapeAdapter.ts:164-197`), then the shape handler draws into the root, the size is read back (`node.width`, `node.height`), and the drawing is removed (311-321).
2. **Position**: `insert` (324-326, section 6.3).
3. **Draw**: a new node with `x`, `y` set to the centre, drawn into `g[transform=translate(cx, cy)]`, which is appended in declaration order before the handler runs (330-347). `c4Shape.intersect = node.intersect`.

The adapter (`c4ShapeAdapter.ts`):

- **Shape** (`resolveNodeShape`, 69-93): `$shape` then `$sprite` (keywords, case-insensitive: `person`, `box` and `rounded` give `rounded`, `cylinder`, `database` and `db` give `cylinder`, `queue` and `pipe` give `h-cyl`, `component` gives `fr-rect`). Then the comma-separated `$tags` (the first keyword wins). Then the type: `person` and `external_person` give `person`, the `_db` kinds give `cylinder`, the `_queue` kinds give `h-cyl`, everything else `rounded`. Probed (fixture `nightly-check/update-element-shape-override`): `$shape="cylinder"` changes a container, and `$shape="folder"` is ignored.
- **Classes**: `c4-shape`, `c4-<type>`, and `c4-external` for external kinds.
- **Stereotype**: `[Kind]` or `[Kind: techn]`, where `Kind` is `Person`, `Software System`, `Container` or `Component`, with the `external_` and `_db`/`_queue` suffix dropped (`stereotypeLabel`, 103-106). Probed: `[Container: Tech]`, `[Software System]`, `[Person]`.
- **Description**: one line, the `descr` text, when not empty (186).
- **Colours**: `fill` from `bgColor` or the type's `_bg_color`, `stroke` from `borderColor` or the type's border colour, and `color` from `fontColor` or `#FFFFFF` (142-156). The strings are `fill:…`, `stroke:…`, `color:…` and each gets `!important` (`styles2String`).
- **Corner radius**: rounded and `fr-rect` get `rx:12px` and `ry:12px` in the style (178-181). The `rx` attribute is the theme `radius` (12).

Sizes, for a node with label bounding box `bbox` and padding `p = 20` (the grid cell is `width = 216`):

- **Rounded** (`drawRect.ts:21-22`, `roundedRect.ts`): `w = max(bbox.w + 2p, 216)`, `h = bbox.h + 2p`, drawn as a `rect` at `(-w/2, -h/2)`. Probed: `rect x=-108 width=216 height=113.63` for a three-line label of 73.63 px (dump of `pr-check/c4-3`).
- **Person** (`person.ts:17-33`): `w = max(bbox.w + 2p, 216, 100)`; head radius `r = min(max(0.23w, 16), 56)`; `overlap = 0.27r`; body height `bh = bbox.h + 2p` (no node height is set in the measure pass); body radius `min(0.177w, 0.45bh)`; `total = bh + 2r - overlap`; `top = -total/2`; `bodyTop = top + 2r - overlap`. The body is a `rect` (`x=-w/2`, `y = bodyTop`, `width = w`, `height = bh`, `rx = ry = body radius`) and the head a `circle` at `(0, top + r)`. Probed: `r = 49.68`, `bh = 88.07`, `total = 174.01` (dump of `nightly-check/elements/person-and-person-ext`).
- **Database** (`cylinder.ts:57-126`): `node.width = 216 - 20 = 196`, no node height, `w = max(196, bbox.w) + 20`, `rx = w/2`, `ry = rx/(2.5 + w/50)`, `h = max(0, bbox.h) + 20 + ry`. The path is `createCylinderPathD(0, 0, w, h, rx, ry)` at `translate(-w/2, -(h/2 + ry))`, and the size is `getBBox` of that path (line 121). Probed: `216 x 115.57` for `ContainerDb`. The label is at `translate(-(bbox.w/2) - bbox.x, -(bbox.h/2) + padding/1.5 - bbox.y)` (123-126). **Unexplained (unverified):** for the `ContainerDb` label the formula gives -10.32 and the DOM has -10.70 (raw values in the dump: `bbox.h = 47.6088`, `bbox.y = -0.1515`, `padding = 20`). The difference of 0.38 is not in the source I read. Resolve it with the differential before the cylinder is trusted.
- **Queue** (`tiltedCylinder.ts:57-87`): `node.width = 216 - 10` (the padding is halved, and `rx*3` is 0 with no height), `h = max(0, bbox.h) + 10`, `ry = h/2`, `rx = ry/(2.5 + h/50)`, `w = max(206, bbox.w) + rx + 10`, and the size is the path's `getBBox`, which is wider than `w`. Probed: a queue of 239.79 px with a path that starts at x = -7.93 (`nightly-check/elements/container-variants`). The grid uses that width, so the queue takes more room in its row.

The label box comes from `c4LabelHelper` (section 6.6), and the shapes place it with the formulas above.

Edge intersection (`intersect`): rounded and `fr-rect` use the rectangle, the person uses the polygon outline of the head arc and the body corners (`person.ts:83-129`), the database and queue use their own curves (`cylinder.ts:128` on). Relations use the centre-to-centre line (`c4Renderer.ts:375-385`).

### 6.6 Element label (`c4LabelHelper.ts`)

The label is a stack of sections, each a `g` with a class, centred and with a 3 px gap (`SECTION_GAP`, line 9):

1. `c4-name` with the element label (only when not empty, line 44).
2. `c4-type` with the stereotype (always).
3. `c4-descr`, one section per description array element (the adapter gives at most one).

For each section `createText` lays out the text (`useHtmlLabels: false`, `markdown: false`, `isNode: true`, the width below, the node's label style), with `sanitizeText(decodeEntities(text))` first (lines 55-70). The text-anchor of each outer tspan is set to `middle` (72), and the font weight and style of the inner tspans are cleared (75-78), so the classes decide the weight.

- **Wrap width**: `max(216 - 2*20, 32) = 176` when the node has a width (46-48). The wrap is on unless `c4.wrap` is false (52-53). Probed: a long name wraps at 176 px (`q4-long-label`, and `nightly-check/description-wrapping` for the description). Top-level `wrap` does not matter here.
- **Font size per section**, from CSS (`styles.js` 43-59): name bold at 14 px (the element's size), type `0.75em` (10.5 px at the default 14), description `0.82em` (11.48 px). Probed with `getComputedStyle`: 14 px bold, 10.5 px, 11.48 px (probe5).
- **Layout**: each section's box is `getBBox` of its `g`. Its translation is `(total/2 - box.x - box.w/2, y - box.y)`, where `total` is the widest section, and `y` stacks the sections with the gap (83-88). The whole label is then moved by `(-total/2, -height/2)` (92). Probed: the label box has `x` = 0.05 and width 111.61 for the person (the centred description).
- **Empty rectangle**: `labelEl.insert('rect', ':first-child')` adds a rect with no attributes (line 91). Probed: such a rect does not change `getBBox` in Chromium (probe4), so it can be written and ignored.
- **Result**: `{shapeSvg, bbox, halfPadding, label}` (96). `bbox` is `getBBox` of the label group before the shape moves it.

Text of each line: `createText` gives `text[y=-10.1][style=fill:#FFFFFF !important]` with `tspan.text-outer-tspan.row` (`x=0`, `y=-0.1em` or `1em`, `dy=1.1em`, `text-anchor=middle`) and one `tspan.text-inner-tspan` per word, plus a `rect.background` (`style="stroke: none"`). Probed (dumps). The line height is 1.1 em, the same as the unified labels.

### 6.7 Relations (`drawRels`, `c4Renderer.ts:387-433`, and `svgDraw.ts:26-126`)

Measuring (`c4Renderer.ts:387-414`): the label is measured with the message font, and for `C4Dynamic` the label becomes `i + ': ' + label`, with `i` counting from 1 over the relation array (400-402; probed: `1: x`, `2: second`). The techn and descr are measured only when non-empty.

Endpoints (416-431): `getIntersectPoints` gives the start on the source element's edge toward the target's centre, and the end on the target's edge toward the source's centre. A boundary is a valid endpoint (its `intersect` is the rectangle, `c4Renderer.ts:256-265`).

Drawing:

- **Path**: the first relation in the array is a `line` (`x1, y1, x2, y2`), with `stroke-width="1"` and `stroke` = `lineColor` or `#444444`, `fill: none`. Every later relation is a `path` with `d = M sx,sy Q cx,cy ex,ey`, where `cx = sx + dx/4` and `cy = sy + dy/2` (`svgDraw.ts:57-79`). Probed: the second relation of `q6-dynamic` is `M466,202.53 Q441,202.53 366,202.53`.
- **Markers**: `marker-end` = `url(#<id>-arrowhead)` unless the type is `rel_b`. `marker-start` = `url(#<id>-arrowend)` for `birel` and `rel_b` (`svgDraw.ts:49-54, 80-85`). Probed: `Rel_Back` gives `marker-start` only, `BiRel` both.
- **Label**: a `text` for each line of the label (lines are split by `<br>`). `x = min(sx,ex) + |dx|/2 + offsetX`, `y = min(sy,ey) + |dy|/2 + offsetY`, the text is `text-anchor: middle` with the width `labelWidth` (the rounded width). Since the anchor is the middle of `x + labelWidth/2`, **the visible label starts at the midpoint and extends right** (`svgDraw.ts:94-104`, byTspan at 376-378). Probed: midpoint 416 and text at 415.84 for `Contacts` (47 px wide, `x = 439.5`).
- **Techn**: `[techn]` in italics, with `y = mid + offsetY + messageFontSize + 5` and width `max(labelWidth, techn.width)` (`svgDraw.ts:106-124`). Probed: `[t]` at y 219.53 = 202.53 + 12 + 5.
- **Offsets**: `offsetX` and `offsetY` are read as `rel.offsetX ? parseInt(String(rel.offsetX)) : 0` (`svgDraw.ts:35-36`). A named `$offsetX="abc"` is stored as the string `"abc"` (`c4Db.ts:609`), which is truthy, so `parseInt` gives `NaN` and the text gets `x="NaN"`, drawn at the origin (probed: `q12-rel-offset-nan`). `$offsetX="5px"` gives 5. Open question 3.
- **Colours**: the label fill and the techn fill are `textColor` or `#444444`. The techn is italic.
- **Empty label**: a `text` with empty content is still written.

### 6.8 Legacy text (byTspan, `svgDraw.ts:361-395`)

Used for boundary labels, types and descriptions and for relation labels. The placement is `textPlacement`, which is not in the C4 schema, so `conf.textPlacement` is undefined and the default `byTspan` is used (`svgDraw.ts:443-445`). The `fo` and `old` branches are unreachable from the schema.

For each line of the text (split by `lineBreakRegex`), one `text` element:

- `x = x0 + width/2`, `y = y0`, `text-anchor: middle`, `dominant-baseline: middle` (attribute), `font-size`, `font-weight`, `font-family` as styles. `font-size` is written as a number and Chromium gives `12px` (probe6).
- One `tspan` with `dy = i*fontSize - fontSize*(lines-1)/2` and `alignment-baseline="mathematical"`.
- The attributes passed in (`fill`, `font-style` for the techn) go on the `text`.

Probed for a boundary: `x = box centre`, `y = boundary.y + label.Y`, so the label of a box at `y=124` with `label.Y = 8` sits at 132.

### 6.9 Root `svg`, size and viewBox

- `configureSvgSize` (`setupGraphViewbox.js:43-46`): with `useMaxWidth` true (default), `width="100%"` and `style="max-width: {width}px;"`. Otherwise `width` and `height` attributes. The probes show the first form.
- The `viewBox` (section 6.1, step 11). Probed: `0 -70 932 988.58` for `pr-check/c4-1` (title, so `-(10 + 60)`).
- Empty and title-only diagrams: a header with no statements is a parse error. A title-only diagram renders with `viewBox 0 -70 200 187` and the title at `x = -150` (probe `q17-title-only`).
- The generic `a11y`: `role="graphics-document document"`, `aria-roledescription="c4"`, and `aria-describedby` with a `<desc>` when `accDescr` is set (`mermaidAPI.ts:757`).

### 6.10 Styles (`styles.js`)

The generic sheet (common to all diagrams) is followed by the diagram's rules at the place where a diagram's own rules go (between the `p` rule and the `neo` rules, as `ferrigura-render/src/common/style.rs` already describes). The diagram's own rules:

1. `.person { stroke: <personBorder>; fill: <personBkg> }`, from the theme (lines 37-40). The rule is not used by any element (the person's classes are `basic label-container`).
2. For each of the 20 element types (`C4_ELEMENT_TYPES`, `c4ShapeAdapter.ts:117-130`) and the external kinds, a rule `.c4-shape.c4-<type> .label { font-family; font-size; font-weight }`, written through CSSOM (`styles.js:8-34`). Only the values that are set are written. The serialisation is CSSOM's (`"Open Sans",sans-serif`, no space after the comma).
3. `.c4-shape .label, .c4-shape .label text { color: inherit; fill: currentColor }`, then `.c4-name { font-weight: bold }`, `.c4-type { font-size: 0.75em }`, `.c4-descr { font-size: 0.82em }`, and `stroke-width: 2px` for the basic shapes and the primitives of `.c4-shape` (`styles.js:43-67`).

The root sheet also has `#id :root { --mermaid-font-family: ... }` (generic).

### 6.11 Element group, DOM order and markup

In DOM order (probed on the dumps):

```text
<svg id width="100%" xmlns style="max-width: Wpx;" viewBox role aria-roledescription="c4" [aria-describedby]>
  [<desc id="chart-desc-{id}">accDescr</desc>]              generic a11y
  <style>...</style>                                         section 6.10
  <g/>                                                       generic (mermaidAPI.ts:419)
  <defs><symbol id="{id}-computer" width="24" height="24"><path transform="scale(.5)" d=.../></symbol></defs>
  <defs><symbol id="{id}-database" fill-rule="evenodd" clip-rule="evenodd"><path transform="scale(.5)" d=.../></symbol></defs>
  <defs><symbol id="{id}-clock" width="24" height="24"><path transform="scale(.5)" d=.../></symbol></defs>
  <g>                                                         one per shape, in the order of drawInsideBoundary
    <g class="node c4-shape c4-{type}[ c4-external]" id="{id}-{alias}">   (positioned by the outer g)
      <rect|path|circle ... class="basic label-container" style="fill:...;stroke:... !important;rx:12px ..."/>
      <g class="label" style="color:#FFFFFF !important" transform="translate(..)">
        <rect/>                                                   empty rect, first child
        <g class="c4-name" transform="translate(..)">
          <g>
            <rect class="background" style="stroke: none"/>
            <text y="-10.1" style="fill:#FFFFFF !important">
              <tspan class="text-outer-tspan row" x="0" y="-0.1em" dy="1.1em" text-anchor="middle">
                <tspan class="text-inner-tspan">word</tspan> ...
  </g>
  <g>                                                         one per boundary (after its children)
    <rect x y fill stroke width height rx="2.5" ry="2.5" stroke-width="1" stroke-dasharray="7.0,7.0"/>
    <text x y dominant-baseline="middle" fill="#444444" style="text-anchor: middle; font-size: 16px; font-weight: bold; font-family: ...">
      <tspan dy="0" alignment-baseline="mathematical">Label</tspan>
  </g>
  <defs><marker id="{id}-arrowhead|arrowend|crosshead|filled-head">…</marker></defs>   four separate defs (svgDraw.ts:261-340)
  <g>                                                         relations
    <line .../> or <path .../>  then the label text(s) and techn text(s)
  </g>
  <text x y>title</text>                                       only when there is a title, no class
</svg>
```

Notes:

- Element `g`s and boundary `g`s are appended as the recursion goes. A boundary's own `g` is appended after the groups of its children, so the outer boundary comes last in its subtree.
- The `c4-name` and the rest of the label carry the classes the CSS needs. The `text` of a line has no class.
- The four markers are `arrowhead` (`M 0 0 L 10 5 L 0 10 z`, refX 9), `arrowend`, `crosshead` and `filled-head`, one `defs` each. Only `arrowhead` and `arrowend` are referenced.

### 6.12 Images, icons and symbols

There are no raster or embedded images in C4. Probed: no `<image>`, no `href`, and no `<use>` in any of the 40 captured SVGs. The person is the geometric `person` shape (section 6.5). The database is the `cylinder` path, with class `basic label-container outer-path`.

The three `symbol`s (`computer`, `database`, `clock`, `svgDraw.ts:213-256`) are in every C4 SVG (probed in 40 of 40 captures) and nothing references them. They have no effect on the image. The geometry harness skips `defs` (`SKIPPED` in `diagram-geometry.ts`), so it does not compare them. A port should write them anyway, with the same ids, so the file matches the reference. Copy the three `d` strings from `svgDraw.ts` by script, not by hand.

`drawEmbeddedImage` and `drawImage` exist in `svgDrawCommon.ts` but C4 does not call them.

### 6.13 Randomness, clock, and DOM dependencies

- Randomness: none. The clock: none.
- DOM dependencies that a server renderer must replace:

| Dependency                                                                               | Where                                                                       | What a port does                                                                                                                                                                     |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `screen.availWidth`                                                                      | `c4Renderer.ts:595`                                                         | An explicit `screen_width` option (open question 1)                                                                                                                                  |
| Text width and height, `getBBox` on an svg text appended to `body`                       | `utils.ts:665-732` (through `calculateTextWidth` and `calculateTextHeight`) | `ferrigura-text` (`text_dimensions`, already ported in `common/text.rs`)                                                                                                             |
| `getBBox` of the section groups of a label                                               | `c4LabelHelper.ts:79, 94`                                                   | Text box of the runs of the section, as `unified::label` gives                                                                                                                       |
| `getBBox` of a shape (`updateNodeBounds`) and of the arc paths of the cylinder and queue | `util.ts:241`, `cylinder.ts:121`, `tiltedCylinder.ts`                       | Analytic boxes that match Chromium's tight bounds (section 8.1, risk 5)                                                                                                              |
| `CSSStyleSheet`, `insertRule`, `cssText`                                                 | `styles.js:8-34`                                                            | Written by hand in the same serialisation                                                                                                                                            |
| `sanitizeText` (DOMPurify) on element label texts                                        | `c4LabelHelper.ts:61`; the title and `c4Type` in `c4Db.ts:773-776`          | The sanitiser port of `ferrigura-syntax/src/sanitize.rs`. Probed on element labels (`q18-entities`): `&amp;` becomes `&`, and `&quot;` stays as text. Legacy texts are not sanitised |
| `getDiagramRoot` (sandbox)                                                               | `c4Renderer.ts:569`                                                         | Not needed                                                                                                                                                                           |

### 6.14 Text measurement rules used by C4

There are two measuring paths, and they round differently:

- **Legacy text** (boundary labels, types and descriptions; relation labels, techn and descr): `calculateTextWidth` and `calculateTextHeight` (`utils.ts:634-655`), which call `calculateTextDimensions` (`utils.ts:665-732`). An empty string is `0 x 0` (lines 72-74). Each line is measured as an svg `text` in `body` with `font-size` in px, `font-weight`, and `font-family` set. An empty line is measured as a zero-width space (line 698). The width is `Math.round` of the widest line's `getBBox` width, and the height is the sum of `Math.round` of each line's `getBBox` height (lines 710-714). The reference also measures in `sans-serif` and keeps that result only when it is larger in width, height and line height at once (lines 719-729). On this machine the configured font always won in the probed texts (the note in `ferrigura-render/src/common/text.rs`). Because of the rounding, every legacy width and height is a whole number of pixels. Probed: the relation label `Contacts` is 47 px wide (`x = 439.5`, the midpoint plus 23.5).
- **Element labels** (shapes): the lines are laid out by `createText` in an svg `text`, and the box is `getBBox` of the section group (no rounding, `c4LabelHelper.ts:79, 94`). The shapes read those boxes directly.

`wrapLabel` (`utils.ts:539-578`) is used only where wrapping is on. Legacy wrapping needs top-level `wrap` (section 3.1), and element labels wrap at 176 px by default.

## 7. Fixtures

The fixtures are the 32 files under `e2e/diagrams/c4/`. Each has a front matter `description`, and one of them (`nightly-check/description-wrapping.mmd`) also has `config: wrap: true`. The fixtures use no init directive and no `look`, and their diagram text is ASCII. Non-ASCII characters appear only in front matter descriptions.

| Folder           | Count | Files and what they exercise                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ---------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pr-check/`      | 10    | `c4-1` (Enterprise_Boundary nesting, Person, System, System_Ext, BiRel, Rel, UpdateElementStyle, UpdateRelStyle with offsets, accTitle, accDescr); `c4-2` (System_Ext and Person with `$tags`, Container_Boundary, Rel); `c4-3` (Container, Container_Boundary, Component, Rel_Back, UpdateRelStyle `offsetY`); `c4-6` (ComponentQueue_Ext in an Enterprise_Boundary); `c4-9` (a relation whose end is a boundary); `component-variants` (six component kinds); `container-boundary`; `directional-relationships` (Rel, Rel_U, Rel_D, Rel_L, Rel_R); `dynamic-diagram` (C4Dynamic, manual numbers in the labels); `update-element-style-colors`                                                                                                                                                                                                                                 |
| `nightly-check/` | 20    | `boundaries/` (4): Enterprise, generic `Boundary` with a type, nested `System_Boundary` in `Enterprise_Boundary`, `System_Boundary` in C4Container. `c4-4` (C4Dynamic with a Container_Boundary and UpdateRelStyle offsets). `c4-5` (C4Deployment with Deployment_Node nesting and a Container). `description-wrapping` (long Person and System descriptions with `wrap: true`). `diagram-types/` (2): deployment, and `Node`/`Node_L`/`Node_R`. `elements/` (3): container variants (six), person and person_ext, system variants (six). `link-attribute-not-rendered`, `tags-attribute-not-rendered`, `sprite-attribute-not-rendered` (the attributes are parsed and not drawn). `relationships/` (3): BiRel, Rel_Back, Rel with a technology. `styling/update-relationship-style`. `update-element-shape-override` (`$shape="folder"` is ignored, `$shape="cylinder"` works) |
| `visual-check/`  | 2     | `update-layout-config-2-shapes-per-row` and `-4-` (UpdateLayoutConfig sets the row to 2 or 4). Probed at 1200 px: the 2-row case gives rows of 2, as the fixture says. The 4-row case gives rows of 3 and 1, not one row of 4 as the fixture's description says, because the 1200 px width limit wraps the fourth shape (section 6.3). The fixture's expectation does not hold for the reference at this viewport                                                                                                                                                                                                                                                                                                                                                                                                                                                               |

Coverage of the features: all five headers; the 20 element kinds; the seven relation kinds; the four boundary macros and the three node macros; UpdateElementStyle (colours, shape); UpdateRelStyle (colours, offsets); UpdateLayoutConfig; the title, accTitle and accDescr; `$tags`, `$sprite` and `$link` (not drawn); the wrap of descriptions.

Not covered by any fixture: `look: handDrawn` (out of scope), `$techn` on UpdateElementStyle, `$fontColor` on a boundary, the NaN offset, `System_Boundary` with a positional third argument, and every quirk in section 2.4. Those need the probes of section 12 as extra cases if the port is to be checked for them.

Harness points (the generic geometry harness, `tools/port/geometry/common.ts`):

- **Fonts.** `FORCED_SETTINGS` sets `fontFamily: '"Lexica Ultralegible"'` (line 61). That is a top-level setting. `addDirective` copies it into `themeVariables.fontFamily` (`config.ts:279-284`), and the C4 renderer does not read it. The C4 section keeps `"Open Sans", sans-serif` on both measurement and CSS. Probed: `"Open Sans", sans-serif`, `sans-serif`, `Arial` and `Helvetica` all measure the same width on this Mac (168.09 px for the probe string), which is the system's sans-serif fallback. Forcing the 22 `*FontFamily` keys of the C4 section to Lexica changes the widths and the `viewBox` (probe3: 511.0 to 515.9). So the harness has to set the C4 section too, or C4 is not comparable. Open question 2.
- **Screen.** The harness viewport is 1200 by 900 and `screen.availWidth` is 1200 there (probed), so the reference uses 1200. A port must use the same value for the comparison.
- **Exclusions.** None needed by the rules in `common.ts`: no handDrawn, no CJK, no symbol in the fixtures' diagram text.
- **Folder map.** `TYPE_FOLDERS` in `tools/port/geometry/diagram-geometry.ts` (line 97) has no `c4` entry, so `pnpm port:geometry c4` prints the usage and stops. Adding `c4: ['c4']` is a shared-file change. The fixture walk is recursive (`mmdFiles`, `common.ts:266-272`), so the three subfolders, `visual-check` included, all come in.
- **Skipped elements.** The harness's `SKIPPED` set (`diagram-geometry.ts`) is `style`, `defs`, `marker`, `title` and `desc`. So the `<desc>` of accDescr and the icon `symbol`s are not compared. The root title is a `text` element and is compared.

## 8. Risks, effort, open questions

### 8.1 Risks, most serious first

1. **Screen dependence.** The grid compares absolute x with `screen.availWidth` (section 6.3). The same text draws differently on a 1200 px and a 1920 px screen (probed). A port needs a fixed value. Without one the output is not reproducible.
2. **Fonts.** The default C4 font is not bundled (`"Open Sans"`), and its fallback is the system's sans-serif. The reference's widths match Helvetica or Arial on this machine, and ferrigura only has Lexica (decision 0005). Only forcing the C4 section to Lexica makes the comparison meaningful. Shapes and the boundary label widths both change.
3. **Parser corruption.** The greedy unquoted-argument rule (P2 to P7) makes some broken input parse without an error and draw something else. A port that is only "correct" on valid input will not match. The lexer states must be reproduced exactly, including the first-match order and the word boundary of keywords.
4. **Grid state.** The shared `cnt` and the absolute `widthLimit` (section 6.3) change the position of shapes in ways that are easy to get wrong. The probes `q1`, `q1b` and `q3` are the cases to test first.
5. **Measured sizes.** A shape's size is a `getBBox` of its label group (text, sections, gaps) and, for the cylinder and queue, of an arc path. Chromium's `getBBox` for a path with arcs is the tight bound. ferrigura's shapes compute analytic boxes (`shapes/cylinder.rs:50`, `tilted_cylinder` at `:172`). A queue measured 239.79 px wide (probed), which should follow from the analytic `w + 2rx` (unverified). The database label offset is 0.38 px off from the source formula (section 6.5), and its cause is not yet known.
6. **Shared dispatch.** `labelHelper` sends any node with a `stereotype` to the C4 label (`util.ts:57-59`), and the usecase actor must not take that path (`usecaseShapes.spec.ts:227`). ferrigura's `shapes/util.rs` `label_helper` has no such branch. The port adds it, which is a shared-file change.
7. **Entity placeholders.** Element labels and legacy labels show the same entity differently (`&quot;` stays as text in an element label and becomes `"` in a legacy label, section 2.1). The decode point is part of the output, so a port must decode at the same two points as the reference.
8. **Wrap flags.** The top-level `wrap` gates the legacy boundary and relation wrap, and the element label wrap uses `c4.wrap`. Both defaults matter. Top-level `wrap` has no default in the schema, so it is off unless set.

### 8.2 Effort and what exists

Already in ferrigura and reusable: `common/text.rs` (`text_dimensions`, `wrap_label`, `break_string`, the memo), `unified::label::layout` (createText for `htmlLabels: false`, with a style string for per-section sizes), the shapes `person`, `rounded` (as `rect`), `cylinder`, `tilted_cylinder` and `subroutine` in `unified/shapes/`, `common/viewbox.rs` (`configure_svg_size`), `common/style.rs` (the base sheet), `a11y.rs`, `common/draw.rs` (`draw_rect`, `get_text_obj`), and `decode_entities`.

To write:

| Part                                                                                                | Estimate             | Notes                                                                |
| --------------------------------------------------------------------------------------------------- | -------------------- | -------------------------------------------------------------------- |
| Parser: lexer states and rules, grammar, the attribute mini-language, and the DB                    | 1,000 to 1,300 lines | Hand-written (decision 0002). The quirks of section 2.4 are the spec |
| Model and exporter (`tools/port/export-c4.ts`), canonical model                                     | 250 to 350           | Section 4                                                            |
| Renderer: grid, boundaries, relations, legacy text, the stacked label, the adapter, the style sheet | 1,100 to 1,400       | Sections 6.3 to 6.10                                                 |
| Shared changes: `labelHelper` dispatch, screen option, C4 font keys in the harness                  | 60 to 120            | Named in the report                                                  |
| Tests: the 169 parser cases and the 2 label cases, the fixtures, the probes of section 12           | 600 to 800           | The probe inputs are in the scratch folder                           |

Total: about 3,000 to 4,000 lines. Most of the risk is in the parser and the grid, not the shapes.

### 8.3 Open questions for the coordinator

1. **Screen width.** Propose a `screen_width` render option with default 1200, which is what the harness and the reference's comparison use. Confirm the default and whether it is exposed to users (the reference reads the viewer's screen).
2. **C4 font.** Propose that ferrigura draws all C4 text in Lexica (the only bundled sans), and that the harness sets the 22 `*FontFamily` keys of the C4 section to `"Lexica Ultralegible"` on the reference side, so both sides measure the same font. Alternative: exclude every C4 fixture from the geometry check, which loses most of the comparison.
3. **NaN offsets.** The reference writes `x="NaN"` (probed). The M6 rule names an exception where the reference produces `NaN`. Propose reproducing it (one format string), so no exception is needed.
4. **Canonical text fields.** Confirm section 4: text as strings, the bare-string case as absent, offsets as raw strings.
5. **`wrap` flags.** Confirm that the port reads top-level `wrap` for the element flag and the legacy gate, and `c4.wrap` for element labels, as the reference does.
6. **Icons.** The harness does not compare `defs`, so the three unreferenced `symbol`s cannot fail the comparison. Propose writing them anyway (section 6.12).
7. **Shared changes.** The stereotype dispatch in `labelHelper` and the tight arc bounds for the cylinder and queue need a change in `unified/`. Confirm the coordinator accepts it, and that the flowchart's 298 of 298 stays green.
8. **The 0.38 px database label offset.** Decide whether to resolve it now (a further probe on the pass-2 label box) or in the differential.

### 8.4 Unverified items

- The sprite area of a boundary (`Y = 48`, `c4Renderer.ts:454-459`). No fixture has a boundary sprite.
- The `subroutine` (`fr-rect`) size. It is reached only through the `component` keyword, which no fixture uses.
- The wrap branch of `calcC4ShapeTextWH` for boundaries and relations (`c4Renderer.ts:215-225`). Not probed with a label that actually wraps: in `q5-wrap-on` the boundary label stayed on one line because the width limit was 1200.
- `useMaxWidth: false` (`configureSvgSize`), and the `height` attribute.
- The cylinder label offset (section 6.5).
- The queue path `getBBox` in general (probed for one size only).

## 9. Probes that confirm the quirk list

The probes, in order of the sections:

- Parser (section 2.4): `probe-parse.ts` runs the grammar compiled with the same `jison` options as `.build/jisonTransformer.ts`, and a copy of `c4Db.ts` with stubs for `sanitizeText` (identity for texts without `<`) and the config. Cases `cases1.json` to `cases5.json`.
- Rendering (sections 6.3 to 6.11): `capture.mjs` renders with the reference bundle in Chromium, with the harness's settings and its page (`harness.mjs`, the bundled fonts, viewport 1200 by 900, `timezoneId` UTC). The 32 fixtures and the inputs in `inputs/` are written to `ref/` as `.svg` and `.dump.json` (every element with its attributes and `getBBox`). `summary.mjs` and `show.mjs` read the dumps.
- Fonts and screen (section 7): `probe2.mjs` (font widths, the 1920 px viewport), `probe3.mjs` (the C4 section forced to Lexica), `probe5.mjs` (computed label sizes and the deployment `stroke-dasharray`), `probe6.mjs` (`font-size` as a number).
- `probe4.mjs`: an empty rect does not change `getBBox`.

Nothing was committed. The only repository file written is this `NOTES.md`.

## 10. Reproducing the probes

The probe folder is `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/c4/`. To rerun the renders:

```sh
cd /private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/c4
node capture.mjs                       # the 32 fixtures into ref/
node capture.mjs inputs/q1-sibling-cnt.mmd   # one input
node show.mjs ref/<name>.dump.json     # the DOM with boxes
node summary.mjs ref/<name>.dump.json  # shapes, boundaries and relations
cd parser && CASES="$(cat cases1.json)" /Users/sebastian/Workspace/ferraid/.claude/worktrees/port-p2/node_modules/.bin/tsx probe-parse.ts
```

`parser/gen.mjs` compiles the grammar with the `jison` in `node_modules`. `parser/c4Db.ts` is a copy of the DB with its imports replaced by `parser/stubs/`, and it is regenerated from the source if the source changes.

## 11. Read in part

`preprocess.ts` (`cleanupText`, lines 6-17, which rewrites CRLF and HTML quotes before the grammar runs; `preprocessDiagram` around line 58) and `diagram-api/comments.ts` (lines 6-8). The generic front-matter and directive handling was not read. The C4 specs were read in full, but only their titles and counts are summarised.

## 12. Probe inputs

Named inputs, all in `inputs/` (or in the parser's `cases*.json`):

| Name                                                                                   | Used in                                                                                         |
| -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `q1-sibling-cnt`, `q1b-sibling-cnt-row1`                                               | 6.3 (widthLimit, cnt)                                                                           |
| `q3-four-row`, `q9-screen`                                                             | 6.3 (width at 1200 px)                                                                          |
| `q4-long-label`, `q5-wrap-on`                                                          | 6.6 (label wrap, boundary wrap)                                                                 |
| `q6-dynamic`, `q7-rel-back-birel`                                                      | 6.7 (relations, markers, numbering)                                                             |
| `q8-no-label`                                                                          | 2.4 (P2, the greedy argument rule; the probe case is `rel-no-label-then-more` in `cases1.json`) |
| `q2-outer-inner-only`                                                                  | 6.4 (nesting)                                                                                   |
| `q10-style-techn`, `q11-boundary-style`, `q12-rel-offset-nan`                          | 3.3, 6.4, 6.7                                                                                   |
| `q13-acc-title`, `q14-self-nesting`, `q15-global-alias`, `q16-empty`, `q17-title-only` | 2.4, 6.9                                                                                        |
| `q18-entities`                                                                         | 2.1                                                                                             |
| `pr-check/*`, `nightly-check/*`, `visual-check/*`                                      | the 32 fixtures (6.x and 7)                                                                     |
