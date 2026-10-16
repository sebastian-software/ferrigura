# State diagram: M6 research notes

Research for M6 (state diagram: parser, canonical model, specs, renderer, fixtures). Reference: the JS fork at commit `04b952b4e`; the main checkout and this worktree have identical sources for `diagrams/state`, `rendering-util`, `schemas` and `tools/port`.

Read-only work. No repository file was changed. Probes ran in the reference from the main checkout (this worktree has no `node_modules`), through `tools/port/reference.ts` (Vite SSR loader). Scripts and outputs are in `scratchpad/m6-state/`: `probe.mts` (parse, DB, getData summary), `probe2.mts` (nodes, edges, DB texts), `acc.mts`, `san.mts`, `cfg.mts`, `cases*.json` (inputs) and `out*.json` (outputs).

Tags used below:

- `[probe]`: run in the reference; the case name refers to `cases*.mjs`.
- `[src]`: read in the source, not run.
- `[unverified]`: neither confirmed; see section 7.

Paths: reference files are under `packages/mermaid/src/` unless written otherwise. Rust paths are under `crates/`. `M5` means the flowchart rendering on this branch.

---

## 0. Summary

| Item     | Fact                                                                                                                                                                                                                                                                      |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Grammar  | one Jison file, `diagrams/state/parser/stateDiagram.jison` (352 lines): 25 exclusive start conditions, about 60 lexer rules, 25 statement alternatives                                                                                                                    |
| Headers  | one detector (`stateDetector-V2.ts`, id `stateDiagram`, regex `/^\s*stateDiagram/`) serves `stateDiagram` and `stateDiagram-v2`; no v1 code remains; the v1 spec file tests the same grammar                                                                              |
| Pipeline | parse, then `setRootDoc` runs `extract` (DB maps). `draw` runs `extract` again, then `getData` (`dataFetcher`), then `rendering-util/render.ts` with the ELK layout, then links, title and viewport                                                                       |
| Parser   | the DB drops many statements silently (section 2.3). Each one below is a reproduced reference behaviour that the port must decide on                                                                                                                                      |
| Renderer | reuses M5 for ELK, labels, edges, notes, start, end, fork, join and choice. Missing: `rectWithTitle`, the clusters `roundedWithTitle`, `noteGroup`, `divider`, barb markers, the state stylesheet, the state title, root class and viewport, and a state `getData` module |
| Specs    | 9 files, 124 defined tests by grep (123 active, 1 `it.skip`). Parser-level and DB-level mostly; one file needs the DOM                                                                                                                                                    |
| Fixtures | 145 files in `e2e/diagrams/state-diagram/` (42) and `state-diagram-v2/` (103), of which 144 are state inputs (one is the `info` smoke test); 12 more state inputs in `e2e/platform/dev-diagrams/`                                                                         |

---

## 1. Sources

| Role                | File (reference)                                                               | Lines                     | Notes                                                                                                                                       |
| ------------------- | ------------------------------------------------------------------------------ | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | --- | ------- |
| Grammar and lexer   | `diagrams/state/parser/stateDiagram.jison`                                     | 352                       | `%options case-insensitive` at 13 [probe: upperKeywords]                                                                                    |
| Diagram definition  | `diagrams/state/stateDiagram-v2.ts`                                            | 21                        | `parser` from the Jison file; `db` getter returns `new StateDB()` per parse; `init` copies `arrowMarkerAbsolute` into `cnf.state` (line 19) |
| Detector            | `diagrams/state/stateDetector-V2.ts`                                           | 23                        | id line 7; regex line 10. Comment line 5: both headers use the unified renderer                                                             |
| Parser wrapper      | none beyond the diagram file; `parser.yy` is set by the caller                 |                           | Specs do `parser.yy = stateDb; parser.yy.clear()`                                                                                           |
| DB                  | `diagrams/state/stateDb.ts`                                                    | 782                       |                                                                                                                                             |
| getData             | `diagrams/state/dataFetcher.ts`                                                | 484                       | Module-level state: `nodeDb`, `graphItemCount`, `nextColorIndex`, `containerColorIndex` (39-47); `reset()` 479-484                          |
| Renderer            | `diagrams/state/stateRenderer-v3-unified.ts`                                   | 148                       | `getDir` 19-33, `getClasses` 35-40, `draw` 42-142                                                                                           |
| Constants           | `diagrams/state/stateCommon.ts`                                                | 110                       | edge constants 32-36, shape names 38-45, CSS names 48-59                                                                                    |
| Stylesheet          | `diagrams/state/styles.js`                                                     | 333                       | `genColor` 21-99 (palette rules), `getStyles` 101-331                                                                                       |
| Types               | `stateDb.ts`                                                                   | 44-196                    | `BaseStmt`, `StateStmt` (81-93), `Stmt` (119-127), `NodeData` (147-174), `Edge` (176-196), `StyleClass` (141-145)                           |
| Marker helpers      | `rendering-elements/markers.js`                                                | 534-578                   | `barb` 534-547, `barbNeo` 548-578 (with `-margin`)                                                                                          |
| Edge marker mapping | `rendering-elements/edgeMarker.ts`                                             | 40-45                     | `arrow_barb` and `arrow_barb_neo` both map to type `barb`                                                                                   |
| Edge margin rule    | `rendering-elements/edges.js`                                                  | 946-947                   | `useMargin = !animatedEdge && edge.look === 'neo'`                                                                                          |
| Config schema       | `schemas/config.schema.yaml`                                                   | 390-391; 1641-1709        | `state:` ref; `StateDiagramConfig`                                                                                                          |
| Config scope        | `diagram-api/diagramConfigKeys.ts`                                             | 10                        | `stateDiagram: 'state'`                                                                                                                     |
| Config resolution   | `config.ts`                                                                    | 77-106, 149-152           | `resolveAppearance` picks `theme`, `look`, `layout` from the `state` section [probe: cfg]                                                   |
| Shared render entry | `rendering-util/render.ts`                                                     | 81-103, 113-129           | `render` prefixes node domIds with the diagram id (88-93)                                                                                   |
| Node insert         | `rendering-util/rendering-elements/nodes.ts`                                   | 22-35                     | `rect` becomes `roundedRect` when `rx && ry` (22-29); unknown shape throws (33-35)                                                          |
| Cluster insert      | `rendering-elements/clusters.js`                                               | 15-846                    | `insertCluster` 841-846 uses `shape                                                                                                         |     | 'rect'` |
| Leaf vs group       | `rendering-util/createGraph.ts`                                                | 66, 161                   | `insertNode` for leaves; `isGroup` for groups                                                                                               |
| Title and viewport  | `utils.ts` `insertTitle` 793-815; `rendering-util/setupViewPortForSVG.ts` 5-16 |                           | class set at line 12                                                                                                                        |
| a11y                | `accessibility.ts`                                                             | 22-30                     | `aria-roledescription` = diagram type key                                                                                                   |
| Common DB           | `diagrams/common/commonDb.ts`                                                  | 8-29                      | `sanitizeText` wrapper (8), `setAccTitle` (16-18), `setAccDescription` (22-24)                                                              |
| Sanitiser           | `diagrams/common/common.ts`                                                    | 56-94                     | `sanitizeMore` 56-68, `sanitizeText` 71-83, `sanitizeTextOrArray` 85-94                                                                     |
| Colour gate         | `diagrams/common/colorThemeGate.ts`                                            | 17, 33-38, 83-84, 114-124 | palette only for `redux-color` and `redux-dark-color`                                                                                       |
| Id generator        | `utils.ts`                                                                     | 442-445                   | `Math.random` based; used by the divider split (`stateDb.ts` 366)                                                                           |
| Layout insert       | `rendering-util/layout-algorithms/common/index.ts`                             | about 140-147             | `insertMarkers(element, data4Layout.markers, data4Layout.type, diagramId)`                                                                  |
| ELK arrow map       | `rendering-util/layout-algorithms/elk/render.ts`                               | 163-170                   | `ARROW_MAP`; state edges already carry `arrowTypeEnd`, so this is only the fallback                                                         |
| ELK root direction  | `rendering-util/layout-algorithms/elk/render.ts`                               | 715-716                   | `data4Layout.direction` sets `elk.direction`                                                                                                |

Rust (this branch, M5): `crates/ferrigura-render/src/flowchart/` (`mod.rs`, `data.rs`, `shapes/`, `clusters.rs`, `markers.rs`, `edges.rs`, `elk_build.rs`, `elk_apply.rs`, `style.rs`), `crates/ferrigura-syntax/src/flowchart/` (`scanner.rs`, `grammar.rs`, `db.rs`), `crates/ferrigura-syntax/src/{scan.rs,sanitize.rs,lexer.rs}`, `crates/ferrigura-model/src/flowchart.rs`.

The state diagram has no Langium grammar. `packages/parser/src/language/` lists no `state` folder.

---

## 2. Parser

### 2.1 Lexer rules, in order

Rules apply in source order, first match wins (Jison default). Start conditions: the `%x` ones are exclusive. The grammar declares no inclusive `%s` state, so a rule without a condition applies to `INITIAL` only. `struct` is the body of a composite.

| Lines          | Condition                                                 | Pattern                                                                      | Token or effect                                                                                         |
| -------------- | --------------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| 69             | INITIAL                                                   | `"click"`                                                                    | `CLICK`                                                                                                 |
| 70             | INITIAL                                                   | `"href"`                                                                     | `HREF`                                                                                                  |
| 71             | INITIAL                                                   | `\"[^"]*\"`                                                                  | `STRING`                                                                                                |
| 73             | INITIAL                                                   | `"default"`                                                                  | `DEFAULT`                                                                                               |
| 75-78          | INITIAL                                                   | `.*direction\s+(TB\|BT\|RL\|LR)[^\n]*`                                       | `direction_tb` etc. Matches the whole line, anywhere in it [probe: descriptionWithDirectionWord]        |
| 80             | INITIAL                                                   | `[\n]+`                                                                      | `NL`                                                                                                    |
| 81-82          | all listed                                                | whitespace                                                                   | skipped                                                                                                 |
| 83-84          | INITIAL, ID, STATE, struct, LINE                          | `\#[^\n]*` and `\%\%(?!\{)[^\n]*`                                            | comments, skipped (`%%{` is not a comment here)                                                         |
| 85-88, 110-112 | INITIAL, struct                                           | `scale\s+` pushes SCALE; `\d+` is WIDTH; `\s+"width"` pops                   | the rule is written twice; the first wins. The statement does nothing in the DB [probe: scaleStatement] |
| 89-92          | INITIAL                                                   | `accTitle\s*":"\s*` then line value                                          | `acc_title`, `acc_title_value`; likewise `accDescr`                                                     |
| 93-95          | INITIAL                                                   | `accDescr\s*"{"\s*` then up to `}`                                           | `acc_descr_multiline_value`                                                                             |
| 97-100         | INITIAL, struct                                           | `classDef\s+`; `DEFAULT\s+` or `\w+\s+`; then `[^\n]*`                       | `classDef`, `DEFAULT_CLASSDEF_ID` or `CLASSDEF_ID`, `CLASSDEF_STYLEOPTS`                                |
| 102-104        | INITIAL, struct                                           | `class\s+`; `(\w+)+((","\s*\w+)*)`; `[^\n]*`                                 | `class`, `CLASSENTITY_IDS`, `STYLECLASS`                                                                |
| 106-108        | INITIAL, struct                                           | `style\s+`; `[\w,]+\s+`; `[^\n]*`                                            | `style`, `STYLE_IDS`, `STYLEDEF_STYLEOPTS`                                                              |
| 114            | INITIAL, struct                                           | `state\s+` pushes STATE                                                      | no token                                                                                                |
| 116-121        | STATE                                                     | `.*<<fork>>`, `<<join>>`, `<<choice>>`, `[[fork]]`, `[[join]]`, `[[choice]]` | `FORK`, `JOIN`, `CHOICE`, text before the marker, trimmed                                               |
| 123-126        | struct                                                    | `.*direction\s+XX`                                                           | direction, as at 75-78                                                                                  |
| 128            | STATE                                                     | `"`                                                                          | pushes STATE_STRING                                                                                     |
| 129            | STATE                                                     | `\s*"as"\s+`                                                                 | `AS`, pushes STATE_ID                                                                                   |
| 130            | STATE_ID                                                  | `[^\n\{]*`                                                                   | `ID`, after `processId`                                                                                 |
| 131-132        | STATE_STRING                                              | `"` pops; `[^"]*`                                                            | `STATE_DESCR`                                                                                           |
| 133            | STATE                                                     | `\w+\s+\w+.*?\{`                                                             | throws `Error: State name must be a single word. Found: "..."` [probe: errorTwoWordsComposite]          |
| 134            | STATE                                                     | `[^\n\s\{]+`                                                                 | `COMPOSIT_STATE`                                                                                        |
| 135            | STATE                                                     | `\n`                                                                         | pops                                                                                                    |
| 136            | INITIAL, STATE                                            | `\{`                                                                         | `STRUCT_START`, pushes struct                                                                           |
| 137            | struct                                                    | `\}`                                                                         | `STRUCT_STOP`, pops                                                                                     |
| 140-150        | NOTE, NOTE_ID, NOTE_TEXT, FLOATING_NOTE, FLOATING_NOTE_ID | `note\s+`, `left of`, `right of`, `"`, `as`, `:` text, `end note`            | `note`, `left_of`, `right_of`, `ID`, `NOTE_TEXT`, `AS`                                                  |
| 152-153        | INITIAL                                                   | `"stateDiagram"\s+`, `"stateDiagram-v2"\s+`                                  | `SD`                                                                                                    |
| 155            | INITIAL                                                   | `hide empty description`                                                     | `HIDE_EMPTY`                                                                                            |
| 157            | INITIAL, struct                                           | `[*]`                                                                        | `EDGE_STATE`                                                                                            |
| 158            | INITIAL, struct                                           | `[^:\n\s\-\{]+`                                                              | `ID`, after `processId` (splits a `%%` comment off)                                                     |
| 160            | INITIAL, struct                                           | `\s*":"(?:[^:\n;]\|":"[^:\n;])+`                                             | `DESCR`, trimmed of the colon                                                                           |
| 162            | INITIAL, struct                                           | `-->`                                                                        | `-->`                                                                                                   |
| 163            | struct                                                    | `--`                                                                         | `CONCURRENT`                                                                                            |
| 164            | INITIAL, struct                                           | `:::`                                                                        | `STYLE_SEPARATOR`                                                                                       |
| 165            | EOF                                                       | end of input                                                                 | `NL`                                                                                                    |
| 166            | INITIAL                                                   | `.`                                                                          | `INVALID`; no rule uses it, so it is a parse error                                                      |

Start conditions declared and never entered by a rule: `ALIAS` (21), `FORK_STATE` (18), `STYLE_IDS` (33), `STYLEDEF_STYLEOPTS` (35). `ID` (16) and `LINE` (45) are used only in the skip rules (82-84).

Consequences [probe]:

- An `ID` cannot contain `-` (line 158). `my-state --> B` is a parse error; inside a composite it is a lexical error "Unrecognized text" [probe: hyphenId, hyphenTarget, compositeHyphenInside].
- `!`, `.`, `_`, digits and keyword prefixes such as `stateX`, `classy` are IDs [probe: errorBangChar, dotUnderscoreDigitIds, keywordPrefixId].
- Keywords are case-insensitive: `STATEDIAGRAM-V2`, `ACCTITLE:`, `CLASSDEF`, `NOTE RIGHT OF`, `DIRECTION LR`, `State C <<FORK>>` all work [probe: upperKeywords]. The note `position` keeps the typed text (`RIGHT OF`), and `dataFetcher.ts:437` compares with `'left of'` in lower case. So `NOTE LEFT OF A` draws the note on the right [src, unverified].

### 2.2 Grammar productions and their DB effects

| Lines   | Production                                                                | Action                                                                                         | DB call                                                                    |
| ------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| 178-182 | `start`                                                                   | `SD document`                                                                                  | `yy.setRootDoc($2)`, which returns the document                            |
| 184-193 | `document`                                                                | appends each `line` except `'nl'`                                                              | builds the statement list in source order                                  |
| 195-199 | `line`                                                                    | `SPACE statement`, `statement`, `NL`                                                           | `SPACE` is never produced by the lexer, so only `statement` and `NL` occur |
| 202-204 | `classDefStatement`, `styleStatement`, `cssClassStatement`                | `{stmt:'classDef'\|'style'\|'applyClass', ...}`                                                | none at parse time                                                         |
| 205-207 | `idStatement`                                                             | `{stmt:'state', id, type:'default', description:''}`                                           | none                                                                       |
| 208-212 | `idStatement DESCR`                                                       | `description = trimColon(DESCR)`                                                               | none                                                                       |
| 213-218 | `idStatement '-->' idStatement`                                           | `{stmt:'relation', state1, state2}`                                                            | none                                                                       |
| 219-224 | with `DESCR`                                                              | `description` on the relation (trimmed colon)                                                  | none                                                                       |
| 225     | `HIDE_EMPTY`                                                              | no action: value is the token string                                                           | dropped                                                                    |
| 226     | `scale WIDTH`                                                             | no action                                                                                      | dropped                                                                    |
| 227     | `COMPOSIT_STATE`                                                          | no action: value is the string                                                                 | dropped (`state foo` alone creates nothing [probe: stateBareWord])         |
| 228-232 | `COMPOSIT_STATE STRUCT_START document STRUCT_STOP`                        | `{stmt:'state', id, type:'default', description:'', doc}`                                      | none                                                                       |
| 233-243 | `STATE_DESCR AS ID`                                                       | `{stmt:'state', id, description}`; an ID with `:` splits into id and a second description      | none                                                                       |
| 244-248 | `STATE_DESCR AS ID STRUCT_START document STRUCT_STOP`                     | state with `doc`                                                                               | none                                                                       |
| 249-257 | `FORK`, `JOIN`, `CHOICE`                                                  | `{stmt:'state', id, type:'fork'\|'join'\|'choice'}`                                            | none                                                                       |
| 258-260 | `CONCURRENT`                                                              | `{stmt:'state', id: yy.getDividerId(), type:'divider'}`                                        | `dividerCnt++`                                                             |
| 261-265 | `note notePosition ID NOTE_TEXT`                                          | `{stmt:'state', id, note:{position, text}}`                                                    | none                                                                       |
| 266     | `note NOTE_TEXT AS ID`                                                    | no action, value is the `note` token string                                                    | dropped [probe: noteAsForm]                                                |
| 267     | `direction`                                                               | `yy.setDirection(dir)` and `{stmt:'dir', value}`                                               | see 2.4                                                                    |
| 268-270 | `acc_title acc_title_value`, `acc_descr ...`, `acc_descr_multiline_value` | `yy.setAccTitle`, `yy.setAccDescription` on the trimmed value                                  | commonDb (sanitised, section 3)                                            |
| 271-288 | `CLICK idStatement STRING STRING NL`, `CLICK idStatement HREF STRING NL`  | `{stmt:'click', id: <idStatement object>, url, tooltip}`                                       | `addLink` at extract time                                                  |
| 291-298 | `classDef CLASSDEF_ID CLASSDEF_STYLEOPTS`                                 | `{stmt:'classDef', id, classes}`                                                               | none                                                                       |
| 295-297 | `classDef DEFAULT CLASSDEF_STYLEOPTS`                                     | uses token `DEFAULT`, but the lexer emits `DEFAULT_CLASSDEF_ID` (98)                           | parse error [probe: classDefDefault]                                       |
| 300-304 | `style STYLE_IDS STYLEDEF_STYLEOPTS`                                      | `{stmt:'style', id, styleClass}`                                                               | `handleStyleDef`                                                           |
| 306-311 | `class CLASSENTITY_IDS STYLECLASS`                                        | `{stmt:'applyClass', id, styleClass}`                                                          | `setCssClass`                                                              |
| 313-322 | `direction_*`                                                             | `yy.setDirection` and `{stmt:'dir', value}`                                                    | see 2.4                                                                    |
| 329-346 | `idStatement` alternatives                                                | `ID`, `EDGE_STATE`, `ID STYLE_SEPARATOR ID` (`classes: [ID]`), `EDGE_STATE STYLE_SEPARATOR ID` | `:::` fills `classes`, and so the DB's `addState`                          |

Statement alternatives: 25 (lines 202-288).

### 2.3 Quirks the port must decide on

All are reproduced by the reference. Each one is a difference the differential run would show if the port fixed it.

| ID  | Input                                                                  | Observed                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Source                                                                                                                    |
| --- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| Q1  | `A --> B` then `state A <<fork>>`                                      | A stays `default`. Reverse order gives `fork`                                                                                                                                                                                                                                                                                                                                                                                                                                        | `addState` 418-429 only sets `type` when it is falsy; `type` is always set [probe: typeSticksDefaultFirst, typeForkFirst] |
| Q2  | `direction LR` then `direction TB`                                     | `db.getDirection()` gives `LR` (first, `find`, stateDb 734-740). `getData().direction` gives `TB` (last, `getDir`, stateDb 768 and renderer 19-33). Layout uses `getData` (elk/render.ts 715-716), so the last one wins for layout                                                                                                                                                                                                                                                   | [probe: directionTwice]                                                                                                   |
| Q3  | `direction` inside a composite                                         | the last `dir` of the composite's document is the node's `dir` [probe: compositeDirTwo, directionInComposite]                                                                                                                                                                                                                                                                                                                                                                        | `getDir` 185-195                                                                                                          |
| Q4  | `class A foo`, `A:::foo`, `style A ...` for a state inside a composite | dropped at render. The DB map (`extract`) iterates only the root document (`extract` 252-271), so the inner state has no entry in `dbState`, and `dataFetcher` reads classes and styles from there (241-254) [probe: classInCompositeOnly, inlineClassInComposite: `cssClasses` has no `foo`]. `style` in a composite: same path [src]. The spec "can be applied to a state inside a composite state" checks `doc[0].state1.classes` (`state-style.spec.js` 189-203), not the render |
| Q5  | relations inside a composite                                           | not in `getRelations()`; rendered from the `doc` [probe: startInComposite]                                                                                                                                                                                                                                                                                                                                                                                                           |
| Q6  | `classDef default fill:#fff` (any case)                                | parse error: the rule at 295 expects token `DEFAULT`, the lexer emits `DEFAULT_CLASSDEF_ID` (98) [probe: classDefDefault]                                                                                                                                                                                                                                                                                                                                                            |
| Q7  | `note "text" as N1` (floating note)                                    | dropped; the statement is the string `"note "` [probe: noteAsForm]                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Q8  | `state foo` alone, or `state foo : desc`                               | no state created; the statement is a string [probe: stateBareWord, stateWithDescriptionOnly]                                                                                                                                                                                                                                                                                                                                                                                         |
| Q9  | `title Foo` in the body                                                | creates states `title` and `Foo` [probe: titleInBody]. Only the front matter sets a title                                                                                                                                                                                                                                                                                                                                                                                            |
| Q10 | `hide empty description`, `scale N width`                              | ignored; the DB holds a string [probe: hideEmpty, scaleStatement]                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Q11 | a description or label containing the words `direction TB`             | the whole line becomes a direction statement, and the state is never created [probe: descriptionWithDirectionWord: `A : foo direction TB` gives no state A]                                                                                                                                                                                                                                                                                                                          |
| Q12 | `;`                                                                    | not a statement separator; `A --> B; B --> C` gives states `B;` and `B` [probe: trailingSemicolon, semicolonSeparated]. The `eol` rule (324-327) is never used. In a description, label or note, `;` ends the text, and the rest becomes IDs [probe: descrEntities, noteFloatingSemicolon, descriptionWithSemicolon]                                                                                                                                                                 |
| Q13 | `#` or `%%` line comments                                              | skipped by the lexer (83-84) [src]. `%%{` is not a comment there. Spec coverage: `state-style.spec.js` 221-356                                                                                                                                                                                                                                                                                                                                                                       |
| Q14 | `A --> B : ` (empty label)                                             | no label [probe: errorColonOnly]                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Q15 | `A - B`, `A -> B`, `--> A`                                             | parse errors [probe: errorDashSingle, errorArrowRight, arrowAtStart]                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Q16 | a second description on a composite                                    | throws `Group nodes can only have label. Remove the additional description for node [X]` during `extract`, so during parse [probe: groupTwoDescriptions] (`stateDb.ts` 294-300)                                                                                                                                                                                                                                                                                                      |
| Q17 | a `--` as the last line of a composite                                 | the trailing divider has no `doc`, so it is a leaf with shape `divider` (nodes.ts has no `divider` shape, so `insertNode` throws `No such shape: divider`) [probe: dividerTrailingEmpty shows the leaf; the throw is src, unverified]                                                                                                                                                                                                                                                |
| Q18 | `[*]` as a statement                                                   | always `<parent>_start` (`docTranslator` 335-337 uses `first`) [probe: standaloneStart]                                                                                                                                                                                                                                                                                                                                                                                              |
| Q19 | a note inside a composite                                              | the note group has no `parentId`, so it is drawn at the root level [probe: compositeNoteInside] (`dataFetcher.ts` 405-418)                                                                                                                                                                                                                                                                                                                                                           |
| Q20 | a block note with several lines                                        | keeps the inner indentation of the lines after the first [probe: noteMultiline: `line one\n line two`]                                                                                                                                                                                                                                                                                                                                                                               |
| Q21 | `style A ...` twice                                                    | the second one replaces the first [probe: styleSplitsAndReplaces] (`stateDb.ts` 316)                                                                                                                                                                                                                                                                                                                                                                                                 |
| Q22 | `class A,B foo`                                                        | both get `foo`; repeated `class` statements append (duplicates kept) [probe: classMulti; push at 696]                                                                                                                                                                                                                                                                                                                                                                                |
| Q23 | two descriptions, one state                                            | label is the first description and `description` the rest; the shape is `rectWithTitle` [probe: twoDescriptions]. `state "Long" as S` makes `Long` the first description [probe: aliasQuoted]                                                                                                                                                                                                                                                                                        |
| Q24 | `state "x" as S1:alt`                                                  | descriptions `["x", "alt"]` [probe: asAliasWithColon]                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Q25 | `click A "url" "tip"`                                                  | the link key is the `idStatement` object; `url` and `tooltip` keep their quotes (renderer 117-121 strips them) [probe: clickLink]                                                                                                                                                                                                                                                                                                                                                    |
| Q26 | `[*] : start desc`                                                     | the start state gets the description [probe: startEndAsDesc]                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Q27 | `A --> B : label:with colon`                                           | label `label:with colon` [probe: relationLabelColon]                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Q28 | `classDef` with `color:`                                               | the attribute also goes to `textStyles` as `fill:`, which the renderer never reads [src: `textStyles` only in stateDb.ts]                                                                                                                                                                                                                                                                                                                                                            |

### 2.4 DB calls in order (one parse plus one draw)

```
parse (Jison actions, in source order):
  yy.setDirection(d)         -> rootDoc.unshift(...) on the empty array; lost at setRootDoc
  yy.setAccTitle(t)          -> commonDb.accTitle = sanitise(t) without leading blanks
  yy.setAccDescription(t)    -> commonDb.accDescr = sanitise(t) with /\n\s+/ -> \n
  yy.getDividerId()          -> dividerCnt++ ; id "divider-id-<n>"
  yy.trimColon(DESCR)
  yy.setRootDoc(doc)         -> rootDoc = doc
    docTranslator(root)      -> rewrite [*] to <parent>_start/_end (335-342);
                                split composites on dividers (349-372);
                                trailing region gets generateId() (366)
    extract(root doc):       -> clear(true)   (keeps links and commonDb)
      for each root item, in order:
        state     -> addState(id, type, doc, description, note)
        relation  -> addRelationObjs: addState(state1), addState(state2), push relation
        classDef  -> addStyleClass(id, classes)
        style     -> handleStyleDef: state.styles = list (replace); creates the state
        applyClass-> setCssClass: state.classes.push; creates the state
        click     -> addLink(id object, url, tooltip)
      dataFetcher(root, rootDoc, states, nodes, edges, altFlag=true, look, classes)
         (nodeDb, graphItemCount, colour slots reset first: resetDataFetcher, 276)
      node labels: arrays -> label = [0], description = rest; error on groups with more

draw:
  extract(getRootDocV2())   -> the same again (idempotent: second docTranslator
                               does not reassign doc, see 2.5)
  getData()                 -> wrappingWidth, minWidth, direction
  render                    -> ELK
```

Extract is run twice. The second run gives the same maps because `clear(true)` rebuilds `states`, `relations` and `classes` from the same root document, and `links` survive `clear(true)`.

### 2.5 Divider and start/end rewriting

`docTranslator` (327-375) is the only place where composites change shape:

- Each composite document is split at a `--` statement. Each divider statement (`divider-id-<n>`) takes the statements before it as its `doc` (349-360). The remaining statements form a trailing region with the id from `generateId()` (`Math.random`, see `utils.ts` 442-445). This id is random per run [src; spec `stateDiagram-colorIndex.spec.ts` 38-41 says the same].
- On the second pass (in `draw`), every item is a divider and the trailing list is empty, so `node.doc` is not reassigned. The result is idempotent [src].
- A composite with a single region and no `--` is not split.
- `[*]` gets `<parent>_start` when it is the first item of a relation or a statement, and `<parent>_end` when it is the second item of a relation (341-342). The root id is `root`, so root-level markers are `root_start` and `root_end`.

---

## 3. Canonical model proposal

Follows `docs-port/canonical-model.md`: `"version": 1`, camelCase, strings always present, numbers as JSON numbers.

### 3.1 Shape

```json
{
  "version": 1,
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "direction": "TB",
  "classes": [{ "id": "foo", "styles": ["color:#fff"], "textStyles": ["fill:#fff"] }],
  "links": [{ "id": "A", "url": "\"https://example.com\"", "tooltip": "\"tip\"" }],
  "states": [
    {
      "id": "A",
      "type": "default",
      "descriptions": ["first"],
      "classes": ["foo"],
      "styles": [],
      "textStyles": [],
      "note": { "position": "right of", "text": "a note" },
      "doc": [
        /* statements of a composite, nested, see 3.2 */
      ]
    }
  ],
  "relations": [{ "id1": "A", "id2": "B", "relationTitle": "" }],
  "document": [
    /* the root statements after docTranslator, see 3.2 */
  ]
}
```

Rules:

- `states` is the root `Map` of `extract`, in insertion order. Each entry keeps the DB's first-seen `type` (Q1).
- `relations` holds the root relations only (`addRelationObjs`, 551-581). `relationTitle` is `""` when absent.
- `classes` is the `Map` of `addStyleClass`, in order. `textStyles` is kept, because the DB holds it.
- `links` is the list of `links` entries; `id` is the state's `id` (the object key is flattened to its `id`).
- `document` is the root statement list after `docTranslator`. It is the source of truth for composites and for the render data.
- `direction` is `db.getDirection()` (first `dir`, default `TB`). The layout direction is `getData().direction` (last `dir`). Keep both in the model; the layout value can also be computed from `document` (Q2).

### 3.2 Statements inside `document` and `doc`

| `stmt`       | Fields                                                                                                                                                                                      | Notes                                                                                                       |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `state`      | `id`, `type` (`default` `fork` `join` `choice` `divider`), `description` (raw, see 3.4), `descriptions`, `classes`, `styles`, `start` (`true`/`false`, from `docTranslator`), `note`, `doc` | `start` and `doc` are present only where the reference sets them                                            |
| `relation`   | `state1`, `state2` (both `state` objects), `description`                                                                                                                                    | `description` only when there is a label                                                                    |
| `dir`        | `value`                                                                                                                                                                                     | `TB` `BT` `RL` `LR`                                                                                         |
| `classDef`   | `id`, `classes` (raw style list)                                                                                                                                                            |                                                                                                             |
| `style`      | `id`, `styleClass`                                                                                                                                                                          |                                                                                                             |
| `applyClass` | `id`, `styleClass`                                                                                                                                                                          |                                                                                                             |
| `click`      | `id` (a state object), `url`, `tooltip`                                                                                                                                                     |                                                                                                             |
| (string)     | not kept                                                                                                                                                                                    | `note `, `scale `, `hide empty description`, a stray `:` or `title` word: all dropped by the DB (Q7 to Q10) |

Divider regions: a composite whose `doc` was split has `doc` items of `stmt: "state"` with `type: "divider"`. The trailing region's `id` is written as the placeholder `"<generated>"` by the exporter, because the reference's id is random. The Rust model must use a deterministic id, and the comparison must map it.

### 3.3 Exclusions and why

| Left out                                                               | Why                                                                                   |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `dividerCnt`, `startEndCount`                                          | counters, observable only through the ids they produce                                |
| `startIdIfNeeded`, `endIdIfNeeded`, the string branch of `addRelation` | dead: `docTranslator` has already rewritten every `[*]` before `extract` [src]        |
| `setTextStyle`                                                         | dead: no grammar path calls it                                                        |
| `funs` / `bindFunctions` / `setupToolTips` (`stateDb.ts` 650-678)      | browser tooltips (decision 0001)                                                      |
| `addStyleClass` colour mapping (`fill` -> `bgFill`)                    | kept as the DB holds it; the renderer does not read `textStyles` [src]                |
| `warnings` (`log.warn` for links)                                      | not part of the input                                                                 |
| front matter `config`, `%%{init}`                                      | configuration port (M2)                                                               |
| `getData()` nodes and edges                                            | render data, compared in a separate differential (section 5.2), like `flowchart-data` |

### 3.4 Text and sanitisation

Where the DB sanitises (`common.sanitizeText` = DOMPurify with `FORBID_TAGS: ['style']`, plus `sanitizeMore` when HTML labels are on):

| Text                                    | Setter / line                  | Sanitised in the DB?                                                                                                            |
| --------------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| root state `description`                | `addDescription` 605-609 (608) | yes                                                                                                                             |
| root relation `relationTitle`           | `addRelationObjs` 579          | yes                                                                                                                             |
| root note `text`                        | `addState` 443                 | yes                                                                                                                             |
| `accTitle`, `accDescr`                  | `commonDb` 17, 23              | yes                                                                                                                             |
| node labels and descriptions at getData | `dataFetcher` 288, 320         | yes (render data)                                                                                                               |
| edge labels at getData                  | `dataFetcher` 162              | yes (render data)                                                                                                               |
| texts inside a composite (`doc`)        | never reach `extract`          | no in the DB. Descriptions and edge labels are sanitised later in `getData`; a nested note text is not (`dataFetcher` 384, raw) |

`sanitize.rs` already returns text without `<` unchanged, which is DOMPurify's own exit (`sanitize.rs` 89-92). Probes: `AT&T` stays `AT&T`; `R&D <b>x</b>` becomes `R&amp;D <b>x</b>`; `p < q & r` becomes `p &lt; q &amp; r`; `<script>bad()</script>ok` becomes `ok` [probe: san.mts, headerAmp, descrScript, noteText].

Correction for `canonical-model.md`: its Sanitisation section says that the DB value of `AT&T` is `AT&amp;T`. That is true only for text that contains `<`. The example should be fixed when M6 lands.

Header texts. The grammar trims `accTitle` and single-line `accDescr` (268-269) and the block (270). `commonDb.setAccTitle` removes leading blanks (`^\s+`); `setAccDescription` replaces `\n\s+` with `\n`. Probe `accDescrBlock`: `first line\n    second line` becomes `first line\nsecond line`. These are the flowchart's rules (canonical-model.md, flowchart section), not the pie and info rules. The pie and info exporters replace the setters; the state exporter must not, because the sanitiser is part of the setter.

### 3.5 Title

- The grammar has no `title` statement (Q9). `title` is the front matter's `title`, or `""`.
- `setDiagramTitle` (commonDb) sanitises. It is set by the common front matter path, not by any state statement [src].

### 3.6 Left alone for the DB state

- Order of `states` and `classes`: insertion order (first mention wins). `class` or `style` on an unknown id creates the state at that position (`setCssClass` 690-695; `handleStyleDef` 309-313).
- Repeated `[*]` gives the same id in one document.

---

## 4. Specs

| File                               | Defined tests (`it` calls)                     | Level                                                                                                                                        | DOM                          | Notes                                                                                                                    |
| ---------------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `parser/state-parser.spec.js`      | 17 (15 `it`; `it.each` over 2 names)           | parse, then `getStates`, `getRelations`; 2 `toThrow` with the lexer's own message (lines 37, 46)                                             | no                           | `unsafe names` uses `it.each` (198); the message text is the custom lexer `Error`, not Jison, so ADR 0002 keeps it exact |
| `parser/state-style.spec.js`       | 24                                             | parse, then `getClasses`, `getStates`, `getData` (3 calls), `getState`                                                                       | no                           | comments, `:::`, composites (189-219)                                                                                    |
| `stateDb.spec.js`                  | 6                                              | DB: `addStyleClass`, `addDescription`, `addState`, `getStates`, `getClasses`; `sanitizes on the description` (62) needs the DOMPurify window | DOMPurify only               |                                                                                                                          |
| `stateDiagram-colorIndex.spec.ts`  | 9                                              | parse, then `getData` and colour slots                                                                                                       | no                           | the comment at lines 38-41 names the random trailing region id                                                           |
| `stateDiagram-v2.spec.js`          | 31                                             | parse, then `getAccTitle`, `getAccDescription`, `getDirection`, `getRelations`, `getState`, `getStates`                                      | no                           | `DEFAULT_DIAGRAM_DIRECTION` (453-455)                                                                                    |
| `stateDiagram.spec.js`             | 29 active + 1 `it.skip` (384, `if` statements) | parse, `extract`, `getLinks`, accessors                                                                                                      | no                           | `click` test at 409 uses `extract` directly                                                                              |
| `stateNoteEdge.spec.ts`            | 3                                              | `getData` edges                                                                                                                              | no                           | note edge is dashed on the edge itself                                                                                   |
| `stateNoteNode.spec.js`            | 2                                              | `getData` nodes                                                                                                                              | no                           | a note on a composite is not a group                                                                                     |
| `stateRenderer-v3-unified.spec.js` | 2 (`it.each` over `node`, `rough-node`)        | `draw` with `render` mocked; checks `<a>` wrapping, `title`, tooltip DOM and hover class                                                     | DOM and mouse events (jsdom) | a render-level test; the rest of the port gets no render coverage from specs                                             |

Totals: 124 defined (123 active, 1 skipped). The STATUS table says about 121 (grep-based, unverified). A vitest run on the reference will settle it.

Classification for the port:

- Parser-level (ported as parse tests with model asserts): `state-parser` (17), `state-style` (24), `stateDiagram-v2` (31), `stateDiagram` (29, plus 1 ignored).
- DB-level: `stateDb` (6).
- Data-level (`getData`, in the M6 data differential): `stateDiagram-colorIndex` (9), `stateNoteEdge` (3), `stateNoteNode` (2).
- Render-level: `stateRenderer-v3-unified` (2). Needs a DOM, and the port's equivalent is the geometry comparison, not this test.

ADR 0002: the two `toThrow` tests assert the lexer's own text, which is not Jison text, so they stay exact. Tests that assert Jison messages: none in these files.

---

## 5. Renderer

### 5.1 The pipeline

`draw` (`stateRenderer-v3-unified.ts` 42-142), in order:

1. `diag.db.extract(getRootDocV2())` (48), the second extract.
2. `diag.db.getData()` (54), which returns `nodes`, `edges`, `other`, `config`, `direction`.
3. `getDiagramElement(id, securityLevel)` (57).
4. `data4Layout.type = diag.type` (59). `diag.type` is `stateDiagram` for both headers [src; the detector id is `stateDiagram`, `stateDetector-V2.ts` 7].
5. `layoutAlgorithm = getRegisteredLayoutAlgorithm(layout)` (62). `layout` defaults to `elk` (probe).
6. `nodeSpacing`, `rankSpacing` = `state.nodeSpacing || 50` (66-67). See 5.5: no ELK reader.
7. `markers = ['barbNeo']` if look is `neo`, else `['barb']` (69-73).
8. `diagramId = id` (74), then `render(data4Layout, svg)` (76).
9. Links (80-133): wraps each matched node in `<a>`.
10. `insertTitle` (135-140).
11. `setupViewPortForSVG(svg, 8, 'statediagram', useMaxWidth ?? true)` (141).

`render` (`rendering-util/render.ts` 81-103) prefixes every node `domId` with the diagram id (88-93), then calls the layout's `render` (ELK, `layout-algorithms/elk/`). The common layout code calls `insertMarkers` with `data4Layout.markers` and `data4Layout.type` (`layout-algorithms/common/index.ts`, about 140-147). Each node goes to `insertNode` (leaves, `createGraph.ts` 66) or `insertCluster` (`isGroup`, `createGraph.ts` 161).

The state diagram goes through the unified pipeline with ELK. Confirmed by source [src]; no render was run, so the claim is not probed.

### 5.2 `getData` mapping (stateDb.getData and dataFetcher)

`getData` (stateDb 755-770):

```
for node in nodes:
  node.wrappingWidth ??= config.state.wrappingWidth          (120)
  if !node.isGroup: node.minWidth ??= config.state.minNodeWidth (120)
return { nodes, edges, other: {}, config, direction: getDir(rootDocV2) }   (last dir)
```

`dataFetcher(parent, item, states, nodes, edges, altFlag, look, classes)` (241-477):

| Input                       | Rule                                                                                                                                                                                     | Lines            | Result                                                                                                                     |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------- |
| root                        | not a node                                                                                                                                                                               | 271              | recurse into children with `altFlag = !altFlag` (475)                                                                      |
| state, `start === true`     | `shape = stateStart`                                                                                                                                                                     | 274-275          | `<parent>_start`                                                                                                           |
| state, `start === false`    | `shape = stateEnd`                                                                                                                                                                       | 276-277          | `<parent>_end`                                                                                                             |
| state, `type != default`    | `shape = type` (`fork`, `join`, `choice`, `divider`)                                                                                                                                     | 279-281          |                                                                                                                            |
| otherwise                   | `shape = rect`                                                                                                                                                                           | 272              | becomes `roundedRect` via `rx` and `ry` (nodes.ts 22-29)                                                                   |
| description count           | first becomes `label`; with two or more, shape `rectWithTitle`; one, `rect`                                                                                                              | 299-330          |                                                                                                                            |
| has `doc` and no `type` yet | `type = group`, `isGroup = true`, `dir = getDir(item)` (last `dir` in `doc`), shape `roundedWithTitle`                                                                                   | 333-338          |                                                                                                                            |
| shape `divider` (region)    | group, shape `divider`, label `''`                                                                                                                                                       | 336-338, 368-370 |                                                                                                                            |
| colour slot                 | containers only (`colorSlotFor`)                                                                                                                                                         | 75-94, 342       | pre-order; a divider region inherits its parent's slot; a styled container consumes a slot and gets `undefined`            |
| `cssClasses`                | `<classStr> statediagram-state`; group adds `statediagram-cluster` and, when `altFlag`, `statediagram-cluster-alt`                                                                       | 289, 343         |                                                                                                                            |
| `nodeData`                  | `{labelStyle:'', shape, label, cssClasses, cssStyles, id, dir, domId, type, isGroup, colorIndex, padding:8, rx:10, ry:10, look, labelType:'markdown', centerLabel:true}`                 | 347-365          | `domId = state-<id>-<graphItemCount>` (`stateDomId`, 101-109)                                                              |
| parent                      | `parentId` when the parent is not root                                                                                                                                                   | 372-375          |                                                                                                                            |
| note on this state          | a note node `<id>----note-<n>` (`type: node`, `parentId: <id>----parent`, padding `flowchart.padding`), a group `<id>----parent` (shape `noteGroup`, `isGroup`, padding 16), and an edge | 379-428          | note edge `<from>-<to>` (442-468): `arrowhead none`, `arrowTypeEnd ''`, `classes 'transition note-edge'`, `pattern dashed` |
| note side                   | `left of` puts the note before the state                                                                                                                                                 | 437-440          | Q-case issue, section 2.1                                                                                                  |
| child document              | recurse with `altFlag` inverted                                                                                                                                                          | 473-476          |                                                                                                                            |

Relation edges, `setupDoc` (111-176, relation case 131-173):

| Field                   | Value                                                | Line     |
| ----------------------- | ---------------------------------------------------- | -------- |
| `id`                    | `edge<graphItemCount>`, then `graphItemCount++`      | 155, 171 |
| `start`, `end`          | `state1.id`, `state2.id` (after rewrite)             | 156-157  |
| `arrowhead`             | `normal`                                             | 158      |
| `arrowTypeEnd`          | `arrow_barb_neo` if look is `neo`, else `arrow_barb` | 159      |
| `style`                 | `fill:none`                                          | 160      |
| `labelStyle`            | `''`                                                 | 161      |
| `label`                 | `sanitizeText(description ?? '')`                    | 162      |
| `arrowheadStyle`        | `fill: #333`                                         | 163      |
| `labelpos`, `labelType` | `c`, `markdown`                                      | 164-165  |
| `thickness`             | `normal`                                             | 166      |
| `classes`               | `transition`                                         | 167      |
| `look`                  | the config look                                      | 168      |

`graphItemCount` is shared by DOM ids and edge ids. It goes up for each relation edge (171) and each note group (419), not for each node. A node's `domId` is set by every visit: `insertOrUpdateNode` overwrites the stored node with `Object.assign` (219-223), so the id is the counter at the last visit. [probe: startEnd: `A` is visited by two relations and gets `state-A-1`; `root_start` is visited once and gets `state-root_start-0`]. The port must reproduce this, because the ids are visible in the SVG.

The DOM id of a node is `<svgId>-state-<id>-<counter>` after `render` prefixes it (render.ts 88-93) [src].

Shape table (state uses; "M5" column is the status on this branch):

| Shape name                                 | Reference handler (file, lines)                          | Used for                                          | M5                                                                                                                                                                                                                                                    |
| ------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `roundedRect` (from `rect` with rx and ry) | `shapes/roundedRect.ts` 1-21, `shapes/drawRect.ts` 29-35 | plain states                                      | ported as `rect::rounded_rect` (`shapes/rect.rs` 24) with the theme radius (`radius`, default 5). Needs a `rounded` flag on the node: M5's `Node` has no `rx`/`ry` (`data.rs` 45-113), so `registry::resolve_with_radii` gets `rounded = false` today |
| `rectWithTitle`                            | `shapes/rectWithTitle.ts` 14-154                         | states with two or more descriptions              | not ported. Title and rows use `createLabel` (`markdown:false`, `width: Infinity`), not labelHelper                                                                                                                                                   |
| `stateStart`                               | `shapes/stateStart.ts` 8-78                              | `[*]` start                                       | ported (`shapes/state.rs` `state_start`)                                                                                                                                                                                                              |
| `stateEnd`                                 | `shapes/stateEnd.ts` 8-96                                | `[*]` end                                         | ported (`shapes/state.rs` `state_end`)                                                                                                                                                                                                                |
| `forkJoin` (`fork`, `join`)                | `shapes/forkJoin.ts` 8-68                                | `<<fork>>`, `<<join>>`                            | ported (`shapes/state.rs` `fork_join`). Padding uses `state.padding` (59-62) [src]: M5 does not read the state section                                                                                                                                |
| `choice`                                   | `shapes/choice.ts` 8-55                                  | `<<choice>>`                                      | ported (`shapes/symbols.rs` `choice`)                                                                                                                                                                                                                 |
| `note`                                     | `shapes/note.ts` 10-67                                   | note nodes                                        | ported (`shapes/rects.rs` `note`; class `noteLabel`, `shapes/mod.rs` 224)                                                                                                                                                                             |
| `divider` (leaf, trailing `--`)            | no entry in `shapes.ts`; `nodes.ts` 33-35 throws         | Q17                                               | not applicable (the reference throws, see section 7)                                                                                                                                                                                                  |
| `state` (`shapes/state.ts`)                | `drawRect`, rx 5 or 3                                    | not used by state diagrams (the code uses `rect`) | not ported; not needed                                                                                                                                                                                                                                |

Cluster table (`rendering-elements/clusters.js`, `shapes` map 806-815):

| Cluster shape                                                     | Lines                                 | Used for                                | M5                                                                                                                                                       |
| ----------------------------------------------------------------- | ------------------------------------- | --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rect` (and `squareRect` alias)                                   | 15-143                                | not used by states; flowchart subgraphs | ported as `clusters.rs` (the only cluster M5 paints)                                                                                                     |
| `roundedWithTitle`                                                | 175-306 (theme destructuring 178-186) | composite states                        | not ported. Title label via `createLabel` (`isTitle`), a title strip (`rect.outer`, `rect.inner`), `node.offsetY = bbox.height - padding/2`, `labelBBox` |
| `noteGroup`                                                       | 144-173                               | note groups                             | not ported. Empty rect, `fill: none`, reads `getBBox` back into `node.width` and `node.height`; no label (`UNTITLED_CLUSTER_SHAPES` 823)                 |
| `divider`                                                         | 419-522                               | concurrency regions                     | not ported. Class `divider`; dashed outline under handDrawn (`strokeLineDash [5]`)                                                                       |
| `kanbanSection`, `flowGroup`, `usecaseSystemBoundary`, `swimlane` |                                       | not used by states                      | none                                                                                                                                                     |

M5 already knows the two exceptions in layout: `cluster_paints_title` excludes `noteGroup` and `divider` (`elk_apply.rs` 801-805; `elk_build.rs` 341), and the note-group test (`elk_build.rs` 1280). Only painting is missing.

Edges and markers:

| Item                                       | Reference                                                                                                                                                                                                                                     | M5                                                                                                                       |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| arrow types `arrow_barb`, `arrow_barb_neo` | `edgeMarker.ts` 42-43 (both map to type `barb`, fill true)                                                                                                                                                                                    | `arrow_barb_neo` offset 5.5 in `elk_apply.rs` 941-950 and `path.rs` 31-32 (ported). `arrow_barb` offset 0 (`path.rs` 31) |
| marker defs                                | `markers.js` 534-578: `barb` (refX 19, refY 7, 20 by 14, path `M 19,7 L9,13 L14,7 L9,1 Z`, userSpaceOnUse); `barbNeo` (refX 19; path `M 19,7 L11,14 L13,7 L11,0 Z`; strokeWidth units) and `barbNeo-margin` (refX 17, fill `transitionColor`) | not ported. `markers.rs` has point, circle and cross only (`Kind` 17-23, `from_arrow` 32-39)                             |
| marker id                                  | `<id>_<type>-barbEnd` and `-margin`; `<type>` is `stateDiagram` [src]                                                                                                                                                                         | `markers.rs` 15 `DIAGRAM_TYPE = "flowchart-v2"`; must become a parameter                                                 |
| margin variant                             | used only when `look === neo` (`edges.js` 946-947)                                                                                                                                                                                            | M5 already has `has_margin` per kind; barb must join the margin set (`edgeMarker.ts` 59-68 lists `barb`)                 |
| `pattern: dashed`                          | dash array and classes                                                                                                                                                                                                                        | ported (`edges.rs` 297-322, 479-489)                                                                                     |
| `edgeMarker` URL prefix                    | `edges.js` 934-943: `window.location` when `arrowMarkerAbsolute` (flowchart or state)                                                                                                                                                         | not applicable without a window. Fixtures do not set it (0 of 145)                                                       |

Note: edge label `label` is absent (undefined) for note edges (`dataFetcher` 442-468). The JSON `null` in probes reflects that.

### 5.3 Stylesheet

`styles.js` (333 lines): `genColor` (21-99) writes one rule group per palette slot, only for `redux-color` and `redux-dark-color` (`isColorTheme`, colorThemeGate 37-38). `getStyles` (101-331) writes the rest. These are the selectors:

- `g.stateGroup text`, `g.stateGroup rect`, `g.stateGroup line`: `stateGroup` appears nowhere else in the reference, so these selectors match nothing [src].
- `.transition`, `[id$="-barbEnd"]`, `defs [id$="-barbEnd"]`, `[id$="-dependencyStart"]`.
- `.node circle.state-start`, `.node circle.state-end`, `.end-state-inner`, `.node .fork-join`, `.node rect`, `.node polygon`.
- `.statediagram-cluster rect`, `.statediagram-cluster rect.outer`, `.statediagram-cluster .inner`, `.statediagram-cluster-alt .inner`, `.statediagram-state .divider`, `.statediagram-state rect.divider`, `.statediagram-state rect.basic`, `.statediagram-state .title-state`.
- `.note-edge { stroke-dasharray: 5 }`, `.state-note`, `.statediagram-note rect` and `text`, `.statediagram-note .nodeLabel`.
- `.edgeLabel` (background, rect, text), `.label div .edgeLabel`, `.statediagram .edgeLabel { color: red; ... }` (302-303). The `red` is literal. It does not affect SVG text, which is fill-based [src].
- `.statediagramTitleText` (312-316): `text-anchor: middle; font-size: 18px; fill: textColor`.
- `[data-look="neo"].statediagram-cluster rect` and `rect.outer`, with `rx`/`ry` = `radius` and the drop-shadow filter (`url(${svgId}-drop-shadow)`).

The CSS has `//` comments and nested `text {}` inside `.state-note` (lines 150-160). Both are copied verbatim by the reference. The port must not emit them as CSS.

Theme variables read by the stylesheet and by shapes:

| Variable                                                                                                                                                                                                                                                                                                      | Read in                                                                                                      |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `lineColor`                                                                                                                                                                                                                                                                                                   | `stateStart.ts` 13; `stateEnd.ts` 16; `forkJoin.ts` (`themeVariables`); `styles.js` (`.transition`, markers) |
| `nodeShadow`                                                                                                                                                                                                                                                                                                  | `stateStart.ts` 65-69 and `stateEnd.ts` 83-87 (filter for nodes under 25 wide)                               |
| `stateBorder`, `nodeBorder`                                                                                                                                                                                                                                                                                   | `stateEnd.ts` 16, 59; `clusters.js` 178-186 and 423 (`nodeBorder`); styles                                   |
| `noteBkgColor`, `noteBorderColor`                                                                                                                                                                                                                                                                             | `note.ts` 32-33; styles                                                                                      |
| `altBackground`, `compositeBackground`, `compositeTitleBackground`                                                                                                                                                                                                                                            | `clusters.js` 178-186 (roundedWithTitle), 423 and 459 (divider); styles                                      |
| `borderColorArray`, `bkgColorArray`                                                                                                                                                                                                                                                                           | `colorThemeGate.ts`; `stampColorSlot` (114-124)                                                              |
| `mainBkg`, `stateBkg`, `textColor`, `stateLabelColor`, `noteTextColor`, `edgeLabelBackground`, `labelBackgroundColor`, `transitionLabelColor`, `tertiaryTextColor`, `transitionColor`, `specialStateColor`, `innerEndBackground`, `background`, `strokeWidth`, `radius`, `dropShadow`, `useGradient`, `svgId` | `styles.js` only                                                                                             |

M2 ships `redux-color` and `redux-dark-color`. The theme files (`crates/ferrigura-model/src/theme/redux_color.rs`, `redux_dark_color.rs`) define the state-only names (spot check: `specialStateColor`, `innerEndBackground`, `compositeTitleBackground`, `stateBorder`, `useGradient`, `dropShadow` appear in `redux_color.rs`). M6 must check the full list in the table above against both files.

### 5.4 Title, root, viewport, a11y and links

| Item                   | Reference                                                                                                                                                                        | M5 flowchart value (this branch)                                                | M6 needs                                                                                       |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| root class             | `statediagram` (`setupViewPortForSVG` 12)                                                                                                                                        | `flowchart` (`mod.rs` 327)                                                      | parameter                                                                                      |
| title class            | `statediagramTitleText` (renderer 137)                                                                                                                                           | `flowchartTitleText` (`mod.rs` 288)                                             | parameter                                                                                      |
| title position         | `y = -state.titleTopMargin` (default 25), `x` = centre of the content box, `text-anchor: middle` (`utils.ts` 793-815)                                                            | `text-anchor="start"`, computed `x`, `y = -title_top_margin` (`mod.rs` 281-291) | the same technique as M5: explicit `x`, `text-anchor start` (decision 0005)                    |
| title font             | `18px` (styles.js 312-316)                                                                                                                                                       | the title font size constant (`mod.rs` 346)                                     | parameter                                                                                      |
| viewport padding       | fixed `8` (renderer 77)                                                                                                                                                          | `flowchart.diagramPadding`                                                      | fixed 8                                                                                        |
| `useMaxWidth`          | `state.useMaxWidth ?? true` (renderer 141). The key is not in the state object's defaults [probe: cfg]                                                                           | `flowchart.useMaxWidth` (`mod.rs` 308)                                          | read `state.useMaxWidth`; default true                                                         |
| `aria-roledescription` | `stateDiagram` (`accessibility.ts` 22-30, type key)                                                                                                                              | `flowchart-v2` (`mod.rs` 323)                                                   | `stateDiagram`                                                                                 |
| links                  | `<a xlink:href target="_blank" title>` wraps the `g` after render; `title` set on both the `a` and the `g`; no `transform` on `a`; surrounding quotes stripped (renderer 80-133) | nodes are wrapped at shape time with `transform` (`shapes/mod.rs` 257)          | a post-render wrap that matches the reference: `a` without `transform`, `title` on `a` and `g` |

Also: `setupViewPortForSVG` takes its box from `getBBox` of the whole SVG (`setupViewPortForSVG.ts`, `calculateDimensionsWithPadding`). M5 computes the box from the painted geometry (`mod.rs` 305-310). The state diagram needs the same.

### 5.5 Config read by state code

| Key (state section)                                                      | Schema default                        | Read at                                                                                                    | Effect                                                                                                             | Status                                                                                            |
| ------------------------------------------------------------------------ | ------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| `wrappingWidth`                                                          | 120                                   | `stateDb.ts` 758                                                                                           | `node.wrappingWidth` on every node                                                                                 | used. M5's `elk_build.rs` 804, 809 reads `flowchart.wrappingWidth` only; must read the node value |
| `minNodeWidth`                                                           | 120                                   | `stateDb.ts` 760                                                                                           | `node.minWidth` on non-groups                                                                                      | M5 has `min_width` (`data.rs` 93, 551) from flowchart only                                        |
| `titleTopMargin`                                                         | 25                                    | renderer 138                                                                                               | title baseline                                                                                                     | used                                                                                              |
| `useMaxWidth`                                                            | not in the object; renderer `?? true` | renderer 141                                                                                               | viewport                                                                                                           | used                                                                                              |
| `padding`                                                                | 8                                     | `forkJoin.ts` 59-62                                                                                        | bar grows by `padding/2`                                                                                           | used                                                                                              |
| `nodeSpacing`, `rankSpacing`                                             | none; renderer sets 50 (66-67)        | read only by `ddlt/parseToLayoutData.ts` 35-36 and `swimlanes/layoutCore.ts` 22-23 (flowchart). Not by ELK | no effect with ELK [src]                                                                                           |
| `arrowMarkerAbsolute`                                                    | none                                  | `diagram init` 19; `edges.js` 934                                                                          | browser only (window location)                                                                                     | drop, see section 7                                                                               |
| `look`                                                                   | `neo` (section default) [probe]       | `config.ts` `resolveAppearance` (77-106)                                                                   | the default look of a state diagram is `neo` [probe: stateDiagram scope gives `neo`; global scope gives `classic`] |
| `theme`                                                                  | `redux-color` [probe]                 | same                                                                                                       |                                                                                                                    |
| `layout`                                                                 | `elk` [probe]                         | same                                                                                                       |                                                                                                                    |
| `noteMargin`, `textHeight`, `labelHeight`, `fontSize` (24), `radius` (5) | schema                                | not read by any state code [src: grep over `rendering-util` and `diagrams/state`]                          | none                                                                                                               |
| `flowchart.padding`                                                      | 15 [probe]                            | `dataFetcher.ts` 400 (note node padding); group padding is 16 (415)                                        | the state notes take the flowchart section's padding. Probe: note `padding` 15, note group 16                      |

The configuration port must add the `state` scope to `tools/port/diff-config.ts` (M2 compares sections only when ported). The scope mapping is `diagramConfigKeys.ts:10`.

### 5.6 Theme colours in the palette

`stampColorSlot` (114-124) writes `data-color-id` only when `isColorTheme(theme, palette)` and the slot is defined. `colorSlotFor` (75-94) assigns containers slots in pre-order. M5 has `style::palette_slot` and `palette_rules` for flowchart subgraphs (`style.rs` 56-72, 464-470). The state rules differ (section 5.3).

### 5.7 M5 gaps, in one list

1. Shapes: `rectWithTitle`.
2. Clusters: `roundedWithTitle`, `noteGroup`, `divider` (`clusters.rs` implements `rect` only).
3. Markers: `barb` and `barbNeo` with margin variants; a diagram type parameter (`markers.rs` 15, 17-23).
4. Stylesheet: the state rules (section 5.3), including the palette rules of `genColor`.
5. Data: a `getData` for states (`dataFetcher.ts` and `stateDb.getData`), with `rx`, `ry`, `wrappingWidth`, `minWidth`, `colorIndex`, `parentId`, `isGroup`, `dir` on the layout node. M5's `Node` (`data.rs` 45-113) has no `rx`, `ry` or `wrapping_width`.
6. Layout input: `elk_build.rs` reads `config.flowchart` for wrapping width and min width, and must take the values from the node.
7. Title, root class, `aria-roledescription`, viewport padding and `useMaxWidth` (section 5.4).
8. Links: a post-render wrap that matches the reference (section 5.4).
9. Labels: `createLabel` with `markdown:false` and `width: Infinity` for composite titles and multi-description rows; the rest use labelHelper (Markdown).
10. Ids: `graphItemCount` for every domId and edge id; `state-<id>-<n>` (section 5.2).

---

### 5.8 Found while rendering (M6 renderer task)

Checked by the geometry run (`pnpm port:geometry state`, 137 of 139 comparable fixtures within tolerance; 5 handDrawn excluded, 1 info smoke test not a state diagram):

- **Label anchor.** State node labels start at the label origin: the state stylesheet has no `.node .label text { text-anchor: middle }` (that rule is the flowchart's). The label group is at `translate(-W/2, -H/2)`, with W the label box widened to `state.minNodeWidth` (120). `rectWithTitle` titles and the cluster titles start at their origin too. Ferrigura takes the anchor from the diagram (`pipeline::node_anchor`).
- **Rough outlines (`unified/shapes/rough.rs` `solid_polygon`).** rough.js's solid fill of a polygon is `M p0 L p1 … L pn`, open (a rectangle's fill is three sides), while a `polygon` path (the choice's `rc.path` with `Z`) is closed. Every stroke side is drawn twice (`_doubleLine`). The control points of each pass lie on its side at roughness 0, so the passes are straight for length and sampling; the random divergence point (`_line`, `Math.random` for seed 0) moves control points only. Fork bars, notes and choices use this.
- **Note outline.** The note's size is `max(label box + 2·padding, node.width)`, where the node width comes from the measure step's minimum width of 120 (`state.minNodeWidth` applies to non-groups). The note padding is `flowchart.padding` (15 by default).
- **rectWithTitle rows.** The rows are measured by their first line (`text` child of the first `tspan`): the rows sit at `(titleW − lineW)/2` under the title, and the title shifts by `−(titleW − lineW)/2` when the first line is wider. The `text` element, not its group, carries the transform.
- **Links.** A clicked state's node is wrapped after the render as `<a xlink:href title>` around its `<g>` (which also carries the title), with no transform on the anchor (`stateRenderer-v3-unified.ts` 80-133). Only leaves are wrapped (`g.node`, not `g.cluster`).
- **Curve.** The reference's ELK edge routing reads `conf.curve` at the top level of the configuration, a key no configuration sets (the `curve` of the schema is in the flowchart section), so state edges take the default curve. Ferrigura takes `flowchart.curve`, which is the same unless the user sets it for a state diagram (open).
- **Residual.** Two fixtures (`should-render-state-descriptions`, pr-check and v2) differ by 0.81 px in the width of the first row "New line" (52.30 in the reference, 53.11 here) while the layout's own box is 52.30. The run widths and the title match, so the cause is in how Chromium lays out the explicit `x` of a run after the first; flowing the runs instead made the title worse.

## 6. Fixtures

### 6.1 Counts

| Folder                                         | `.mmd` files | Notes                                                                                                                 |
| ---------------------------------------------- | ------------ | --------------------------------------------------------------------------------------------------------------------- |
| `e2e/diagrams/state-diagram/nightly-check/`    | 32           |                                                                                                                       |
| `e2e/diagrams/state-diagram/pr-check/`         | 10           | `visual-check/` is empty (`.gitkeep`)                                                                                 |
| `e2e/diagrams/state-diagram-v2/nightly-check/` | 93           | includes 3 under `elk/`                                                                                               |
| `e2e/diagrams/state-diagram-v2/pr-check/`      | 10           | `visual-check/` is empty                                                                                              |
| total in the two folders                       | 145          | 1 is the `info` smoke test (`v2-should-render-a-simple-info.mmd`, not a state diagram): 144 state inputs              |
| `e2e/platform/dev-diagrams/`                   | 12           | 1 in `layout-tests/`, 11 in `diagrams/state-diagram/`. Outside the two folders; the collector must decide by detector |

### 6.2 Feature classes (files out of 145)

| Feature                        | Files | Feature                               | Files                                   |
| ------------------------------ | ----- | ------------------------------------- | --------------------------------------- |
| `[*]`                          | 108   | composite opening brace (`state X {`) | 64                                      |
| relation label (`-->` ... `:`) | 39    | concurrency `--` line                 | 29                                      |
| description `:`                | 30    | `note` statements                     | 25                                      |
| `as` alias                     | 19    | multi-line `end note`                 | 15                                      |
| `classDef`                     | 12    | `class`                               | 12                                      |
| `style`                        | 9     | `%%` comment                          | 9                                       |
| `<<fork>>` / `<<join>>`        | 6 / 6 | `<<choice>>`                          | 1 (`[[choice]]` 1)                      |
| `accTitle` / `accDescr`        | 8 / 8 | `click`                               | 2 (both `clickable-state-with-tooltip`) |
| `direction`                    | 2     | `scale` (ignored by the DB)           | 2                                       |
| `title` in the body            | 0     | `hide empty description`              | 0                                       |
| `:::`                          | 0     | `htmlLabels` / `themeVariables`       | 0                                       |

### 6.3 Configs used

| Source                           | Count | Values                                                                                                 |
| -------------------------------- | ----- | ------------------------------------------------------------------------------------------------------ |
| front matter (all 145)           | 145   | `description` (all), `config` (132), `fontFamily: courier` (68), `logLevel` (60)                       |
| front matter `theme`             | 62    | `redux-dark` 14, `redux` 14, `redux-dark-color` 8, `redux-color` 8, `neo-dark` 8, `neo` 8, `default` 2 |
| front matter `look`              | 39    | `neo` 32, `handDrawn` 5, `classic` 2                                                                   |
| front matter `layout`            | 4     | `elk`                                                                                                  |
| front matter `state.useMaxWidth` | 4     | `true` 2, `false` 2 (the `state:` section)                                                             |
| front matter `securityLevel`     | 2     | `loose`                                                                                                |
| `%%{init}`                       | 8     | theme `dark` 2, `default` 2, `forest` 2, `neutral` 2 (plus `logLevel`)                                 |

Themes not shipped by ferrigura: `redux`, `redux-dark`, `neo`, `neo-dark`, `default`, `dark`, `forest`, `neutral`. The M5 harness rule applies: both sides get the fallback theme (names ending in `dark` take `redux-dark-color`, others `redux-color`). That changes 46 of the 62 front-matter theme uses (the other 16 are `redux-color` and `redux-dark-color`) and the 8 `%%{init}` ones.

### 6.4 Browser features and out-of-scope features

| Feature                   | Files                                                                                                                                                          | Handling                                                                               |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `look: handDrawn`         | 5 (`clickable-state-with-tooltip-handdrawn-look`, `concurrency-composite-containers-handdrawn-look-*` (2), `nested-composite-containers-handdrawn-look-*` (2)) | excluded from the geometry comparison, as M5 excludes `handDrawn` (roughjs randomness) |
| `click` with tooltip      | 2 (`clickable-state-with-tooltip`, `-handdrawn-look`)                                                                                                          | `<a>` wrap plus `title` attribute; no JavaScript tooltip                               |
| `arrowMarkerAbsolute`     | 0                                                                                                                                                              | not needed                                                                             |
| non-ASCII text            | 0                                                                                                                                                              | no fallback font needed                                                                |
| `securityLevel: loose`    | 2                                                                                                                                                              | no callbacks in the fixtures                                                           |
| `layout` other than `elk` | 0                                                                                                                                                              | none                                                                                   |
| `fontFamily: courier`     | 68                                                                                                                                                             | overridden by the harness, as in M5                                                    |
| `scale`                   | 2                                                                                                                                                              | ignored by the DB; nothing to draw                                                     |

Fixture categories for the M6 geometry comparison: the same harness as M5 (`pnpm port:geometry-flowchart`), with a state flavour. The fixtures include composite states nested up to three levels, concurrency with two and three regions, notes on states and on composites, forks and joins, choice, and the `elk-*` cases that test the ELK edge handling already ported.

---

## 7. Risks, effort and open questions

### 7.1 Risks

| #   | Risk                                                                                                             | Mitigation                                                                                                                                                                               |
| --- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Random ids: trailing divider region (`generateId`, `utils.ts` 442-445); the DOM ids of its children depend on it | exporter writes `<generated>`; the geometry comparison maps the trailing region by its position in the composite                                                                         |
| R2  | A trailing `--` (empty last region) may crash the reference (`No such shape: divider`, `nodes.ts` 33-35)         | check with the Chromium harness; if it throws, the port returns an error with the same position                                                                                          |
| R3  | Several parse-time behaviours look like bugs (Q1, Q2, Q4, Q7 to Q12)                                             | reproduce all of them, record the decision in an ADR (compatibility promise). The `title`, `direction`, `classDef default` and class-in-composite cases are the ones a user would report |
| R4  | `direction`: the DB and the layout disagree (Q2)                                                                 | keep both in the model                                                                                                                                                                   |
| R5  | `dataFetcher` keeps module state (`graphItemCount`, `nodeDb`, colour slots)                                      | the port keeps it per render; the counts are part of the DOM ids                                                                                                                         |
| R6  | `extract` runs twice (parse, draw)                                                                               | the port builds the DB once; the second pass must give the same result (it does, section 2.5)                                                                                            |
| R7  | M5 is flowchart-specific (`flowchart::data::Node`, `elk_build.rs` reads `config.flowchart`)                      | section 5.7 (items 5 and 6); question 2 in section 7.3                                                                                                                                   |
| R8  | the note edge is `pattern: dashed` and `classes` with `note-edge`; the `neo` look writes an inline dash array    | already ported in `edges.rs`; the state test must cover it                                                                                                                               |
| R9  | `handDrawn` (5 fixtures) uses roughjs                                                                            | excluded, as in M5                                                                                                                                                                       |
| R10 | the state geometry harness does not exist yet                                                                    | M5's harness is the base; the state flavour needs the title, root class and link rules of section 5.4                                                                                    |

### 7.2 Effort

Relative to the flowchart work (M3 parser plus M5 renderer). Estimates, not measured.

| Part               | Reference size                                                                                                                         | Relative to flowchart                                         |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Lexer and grammar  | `stateDiagram.jison` 352 lines, 25 start conditions, about 60 rules                                                                    | about 0.5 of `flow.jison` (634 lines)                         |
| DB and getData     | `stateDb.ts` 782 plus `dataFetcher.ts` 484                                                                                             | about 0.9 of `flowDb.ts` (1358)                               |
| Specs              | 124 tests in 9 files                                                                                                                   | about 0.4 of the flowchart's roughly 318 tests (STATUS table) |
| Renderer, new code | `rectWithTitle`, three cluster shapes, barb markers, state stylesheet (333 lines), title and root parameters, state getData, link wrap | about 0.3 to 0.5 of M5's render work                          |
| Renderer, reused   | ELK, labels, edge paths, arrow offsets, note shape, state, fork, join, choice                                                          | already done                                                  |

Overall: about 40 to 50 percent of M5 in effort. The biggest single risk is R7: generalising M5's flowchart-only types (section 5.7, item 5, and section 7.3, question 2).

### 7.3 Open questions for the coordinator

1. Reproduce every quirk in section 2.3, or fix some? The compatibility promise (`docs-port/README.md`) says the same text renders the same diagram, so the default is to reproduce, with an ADR per exception.
2. Shared layout types: move `flowchart::data::Node` and `Edge` to a layout-neutral module before M6, or copy them for state? Copying duplicates the flowchart types and the ELK build.
3. Trailing `--`: reproduce the error or draw something? The reference throws (src); the Chromium harness can confirm (R2).
4. Tooltips and links: the README (runtime interactivity) says tooltips become a native SVG `<title>`. The reference writes a `title` attribute on `a` and `g`. Which one?
5. `arrowMarkerAbsolute`: no window in ferrigura. Drop the URL prefix, or write the value given? No fixture uses it.
6. `nodeSpacing` and `rankSpacing`: no effect with ELK. Keep them in the configuration (they are in the schema), with a note.
7. The canonical model: `document` plus the derived maps (this proposal), or `document` only? The derived maps are what the DB exposes, which makes the differential easier to read.
8. `classDef default` is a parse error in the reference; reproduce (Q6)? Yes by default.
9. The configuration `look` for a state diagram is `neo` by default (section scope). Confirm that M2's `effective` already takes the diagram type, so that the `state` scope is applied.

### 7.4 Unverified (and why)

| #   | Claim                                                                              | Status                                                                                     |
| --- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| U1  | trailing `--` throws `No such shape: divider`                                      | src only (`nodes.ts` 33-35, `createGraph.ts` 66); needs a render run (Chromium)            |
| U2  | marker ids use `stateDiagram` as the type                                          | src (`markers.js` 538, `markers.rs` 240-247 for the flowchart pattern); needs a render run |
| U3  | spec counts                                                                        | grep-based (`it` calls, `it.each` expanded by hand); needs a vitest run on the reference   |
| U4  | nested note text is not sanitised, and `labelHelper` sanitises it again when drawn | src (`dataFetcher` 384, `labelHelper` in `shapes/util.ts`, not read in detail)             |
| U5  | `NOTE LEFT OF` draws the note on the right                                         | src (`dataFetcher` 437)                                                                    |
| U6  | `style` inside a composite is dropped at render                                    | src (same path as the probed `class`)                                                      |
| U7  | `classDef ... color:` has no effect on state text                                  | src (`textStyles` is never read by rendering)                                              |
| U8  | `#` comments behave as `%%` ones                                                   | src (line 83); covered by `state-style.spec.js` 221-356                                    |
| U9  | `state` shape (`shapes/state.ts`) is not reachable from state diagrams             | src: `dataFetcher` uses `rect` (272)                                                       |
| U10 | `diag.type` is `stateDiagram` for `stateDiagram-v2` input                          | src (detector id 7; probe did not run the detector)                                        |
| U11 | the layout uses `getData().direction` for the root                                 | src (`elk/render.ts` 715-716); the probe shows the value, not the layout                   |
| U12 | `look` default `neo` applies to the rendered SVG                                   | probe on the configuration (cfg.mts); not on a render                                      |

### 7.5 Probe artefacts

Inputs and outputs are in `scratchpad/m6-state/`: `cases.mjs` and `out.json` (37 cases: types, directions, composites, dividers, notes, descriptions, links, errors), `cases2.mjs` and `out2.json` (nodes and edges of 23 cases), `cases3.mjs` and `out3.json` (composite descriptions, styles, nesting), `cases4.mjs` and `out4.json` (identifiers and separators), `cases5.mjs` and `out5.json` (sanitisation), `cases6.mjs` and `out6.json` (bare states, notes), `cases7.mjs` and `out7.json` (case-insensitive keywords), plus `acc.mts`, `san.mts` and `cfg.mts`. Run from the main checkout with `PROBE_OUT` set: `pnpm exec tsx probe.mts "$(cat cases.json)"` for `cases.json`, `cases4.json`, `cases6.json` and `cases7.json`; `probe2.mts` for `cases2.json`, `cases3.json` and `cases5.json`; `acc.mts`, `san.mts` and `cfg.mts` take no input.

Nothing was rendered: the Chromium harness was not run, so every geometry claim is `[src]`.
