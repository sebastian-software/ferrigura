# M6: ER diagram research notes

Scope: the ER diagram of the JS reference (`packages/mermaid/src/diagrams/er/`, fixtures `e2e/diagrams/er-diagram/`) as input to the Rust port. Reference commit `04b952b4e` (main, worktree `port-m6`). Research only; no repository file was changed.

Legend: **[P]** confirmed by a Jison probe, **[R]** confirmed by reading the cited line, **[U]** not confirmed (the reason is given).

Method: the grammar was generated with the repo's `jison` 0.4.18 and the settings of `.build/jisonTransformer.ts`. Tokens and DB calls were recorded with a `yy` stub. Probe scripts are in `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/m6-er/` (`gen.mjs`, `probe-parse.mjs`, `probe-tokens.mjs`, `count-specs.mjs`, `fixture-scan.mjs`, `fm-table.mjs`, `cmap.mjs`). Not done: the real `ErDB` was not executed, nothing ran in a browser, and vitest was not run (it writes caches into the repo).

## 0. Summary

- ER goes through the unified pipeline: `erRenderer-unified.ts:17` `getData` → `rendering-util/render.ts:81` → ELK. M5's ELK, label, edge, cluster, text and sanitiser code is reused. ER-only code: the parser, `getData`, the `erBox` shape, eight cardinality markers, the ER stylesheet and the ER config reads.
- Parser: a 336-line Jison grammar with six lexer states. Several quirks change the DB state of subgraphs; the port must reproduce them (sections 2.2 and 2.3).
- Specs: **660** test cases (659 run, 1 skipped), not the "≈142" in STATUS. 520 come from a loop over 104 characters (`erDiagram.spec.js:34-94`).
- Fixtures: 206. 43 use `look: handDrawn` (excluded, as for flowchart). Of the rest, 9 contain glyphs the bundled Lexica font does not cover. 154 comparable candidates remain.
- Size: ER-specific Rust is about 1.2–1.5k lines, against about 20k for M5's `crates/ferrigura-render/src/flowchart/`. The parser is about 40–50% of the M3 flowchart parser by source size (ER grammar 336 lines, flow.jison 634).

## 1. Sources

| Role              | File                                                                                                                                                                                                                                                                                                                                                                  | Lines of interest                                                                 |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Grammar and lexer | `packages/mermaid/src/diagrams/er/parser/erDiagram.jison`                                                                                                                                                                                                                                                                                                             | lexer 1–95, grammar 105–334                                                       |
| Parse entry       | `packages/mermaid/src/Diagram.ts`                                                                                                                                                                                                                                                                                                                                     | 19–54 `fromText`, 49 `db.clear`, 52–54 title, 77 `textToParse`, 83 `parser.parse` |
| Preprocess        | `preprocess.ts` 65; `diagram-api/comments.ts` 6–8; `utils.ts` 874–906                                                                                                                                                                                                                                                                                                 | `encodeEntities` / `decodeEntities`                                               |
| Detector          | `er/erDetector.ts`                                                                                                                                                                                                                                                                                                                                                    | 7 `id = 'er'`, 10 `/^\s*erDiagram/` (case-sensitive)                              |
| Definition        | `er/erDiagram.ts`                                                                                                                                                                                                                                                                                                                                                     | 9–11 `get db()` returns `new ErDB()`                                              |
| DB                | `er/erDb.ts`                                                                                                                                                                                                                                                                                                                                                          | 19–468                                                                            |
| Types             | `er/erTypes.ts`                                                                                                                                                                                                                                                                                                                                                       | 1–49                                                                              |
| Renderer          | `er/erRenderer-unified.ts`                                                                                                                                                                                                                                                                                                                                            | 10–79                                                                             |
| Entity shape      | `rendering-util/rendering-elements/shapes/erBox.ts`                                                                                                                                                                                                                                                                                                                   | 19–424                                                                            |
| Shape helpers     | `shapes/drawRect.ts` 11–83; `shapes/util.ts` `labelHelper` 50–141, `updateNodeBounds` 220; `shapes/handDrawnShapeStyles.ts` 75–130; `createText.ts` 96–256, 306–419; `utils.ts` `calculateTextWidth` 649, `calculateTextDimensions` 665–731, `handleUndefinedAttr` 934; `diagrams/common/common.ts` `sanitizeMore` 56, `sanitizeText` 71, `parseGenericTypes` 201–240 |
| Pipeline          | `rendering-util/render.ts` 81–103; `createGraph.ts` 42–68 (`measureGroupLabel`, `insertMeasuredNode`), 161–190 (groups and leaves)                                                                                                                                                                                                                                    |
| Markers           | `rendering-util/rendering-elements/markers.js` 6–10, 580–867 (definitions), 997–1021 (registry); `edgeMarker.ts` 39–139; `utils/lineWithOffset.ts` 6–25                                                                                                                                                                                                               |
| Edges             | `rendering-util/rendering-elements/edges.js` 43–45, 55–62, 74–133, 624+, 726–811, 947                                                                                                                                                                                                                                                                                 |
| ELK integration   | `rendering-util/layout-algorithms/elk/render.ts` 143–175, 678–700, 758, 1041–1066, 1068–1085, 1152–1170, 1931+ (`applyElkEdgeLayout`), 2078, 2715–2750                                                                                                                                                                                                                |
| Common DB         | `diagrams/common/commonDb.ts` 4–32 (module-level state)                                                                                                                                                                                                                                                                                                               |
| Styles            | `er/styles.ts` 1–151; `diagrams/common/colorThemeGate.ts` 16–38                                                                                                                                                                                                                                                                                                       |
| Config            | `schemas/config.schema.yaml` 392, 1564–1640 (`ErDiagramConfig`)                                                                                                                                                                                                                                                                                                       |
| Themes            | `themes/theme-base.js` 124–130 (`rowOdd`, `rowEven`)                                                                                                                                                                                                                                                                                                                  |
| Specs             | `er/erDb.spec.js`, `er/parser/erDiagram.spec.js`, `er/parser/subgraph.spec.js`                                                                                                                                                                                                                                                                                        |

## 2. Parser

### 2.1 What the parser receives [R]

```
Diagram.fromText(code, metadata)                       Diagram.ts 19
  type = detectType: /^\s*erDiagram/                   erDetector.ts 10
  db = new ErDB()        (getter, fresh per parse)     erDiagram.ts 9-11
  db.clear()                                           Diagram.ts 49 -> erDb.ts 366
  db.setDiagramTitle(metadata.title)  if set           Diagram.ts 52-54
  source = cleanupComments(withoutFrontMatterAndDirectives)   preprocess.ts 65; comments.ts 7
  textToParse = encodeEntities(source) + "\n"          Diagram.ts 77 (always ends with a newline)
  parser.parse(textToParse)                            Diagram.ts 83
```

- `cleanupComments` removes lines matching `^\s*%%(?!{)[^\n]+\n?`, so a bare `%%` line stays and fails [R]. A `%` in the middle of a line is a lexer error [P].
- `encodeEntities` rewrites `#\w+;` to `ﬂ°name¶ß` (or `ﬂ°°N¶ß` for digits) and, on a line with `style`/`classDef`, `:…#…;` drops the last `;` [R, utils.ts 874–895]. `decodeEntities` (904) reverses it at render time.
- The trailing `\n` matters: `style A color:red;` at end of input fails [P], but with a newline after it works [P]. The real pipeline always adds the newline, so the case cannot occur.

### 2.2 Lexer [R, P]

Lexer options: `%options case-insensitive` (erDiagram.jison:3), so every literal and every regex matches case-insensitively. Rules are tried in definition order and the first match wins (not longest match).

| State                 | Entered by                    | Left by              | Rules (lines)                                         |
| --------------------- | ----------------------------- | -------------------- | ----------------------------------------------------- |
| INITIAL               | start                         | —                    | 12–33, 45–47, 54–94                                   |
| `acc_title`           | `accTitle\s*":"\s*` (12)      | after one token (13) | value `[^\n]*`                                        |
| `acc_descr`           | `accDescr\s*":"\s*` (14)      | after one token (15) | value `[^\n]*`                                        |
| `acc_descr_multiline` | `accDescr\s*"{"\s*` (16)      | `}` (17)             | value `[^}]*` (18), any characters including newlines |
| `block`               | `{` (28)                      | `}` (43)             | 34–44                                                 |
| `block_bq`            | backtick in block (38)        | backtick (40)        | `[^`]+` (39)                                          |
| `style`               | `style` (48), `classDef` (54) | newline (49)         | 49–53, 91–92                                          |

Lexer quirks, each probe-confirmed unless marked:

| #   | Quirk                                                                                                                                                                                                                          | Source                            | Evidence                                                                                                                      |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ----- | -------------------------- | ----- | --- | --- | --------- | ----- | --- |
| L1  | The `direction` rules are `.*direction\s+TB[^\n]*` (and BT, RL, LR). They match from the current token start to the end of the line, so the rest of the line is lost: `A                                                       |                                   | --o{ B : direction TB`gives only`setDirection("TB")`.                                                                         | 19–22 | [P]                        |
| L2  | Keywords get an implicit word boundary (Jison). `endpoint` is an entity; `end                                                                                                                                                  |                                   | --                                                                                                                            |       | B : x`fails;`class         |       | --  |     | B` fails. | 55–57 | [P] |
| L3  | `to` (87) and `optionally to` (88) are keywords. A role spelled `to` unquoted fails.                                                                                                                                           | 87–88                             | [P] (`A                                                                                                                       |       | ..o{ B : to` fails)        |
| L4  | `1` is ONLY_ONE only before whitespace and a letter/quote/underscore, before whitespace and a digit, or before `--`, `..`, `.-`, `-.`; otherwise ENTITY_ONE, an entity name. `1                                                |                                   | --                                                                                                                            |       | 2`gives entities`1`and`2`. | 75–78 | [P] |
| L5  | `u` is MD_PARENT only before `.`, `-` or `                                                                                                                                                                                     | ` (84); otherwise an entity name. | 84                                                                                                                            | [P]   |
| L6  | Quoted names: ENTITY_NAME `"[^"%\r\n\v\b\\]+"` (25). `%`, `\`, CR, LF, VT and BS are not allowed. A quoted name that breaks this rule becomes WORD (26), which is only valid as a role, so the entity fails. `"Blo%rf"` fails. | 25–26                             | [P]                                                                                                                           |
| L7  | Unquoted names: UNICODE_TEXT `([^\x00-\x7F]\|\w\|\-\|\*\|\.)+` (93). `LINE-ITEM`, `A.B`, `Å` are valid; `#`, `%`, `"`, `;` are not.                                                                                            | 93                                | [P]                                                                                                                           |
| L8  | Block state: `\s+` (34) skips newlines too, so attributes are not line-separated. `string name int id` gives two attributes.                                                                                                   | 34, 42                            | [P]                                                                                                                           |
| L9  | Generic types (36): `([^\s]*)[~].*[~]([^\s]*)` is greedy to the last `~` on the line. `map~K,V~ data "x~y"` fails. `map~K,V~ data` works.                                                                                      | 36                                | [P]                                                                                                                           |
| L10 | Attribute keys `\b(PK                                                                                                                                                                                                          | FK                                | UK)\b`(35) match in any case:`pk` is a key, and the key is kept as written (`keys: ["pk"]`). `PK` as an attribute name fails. | 35    | [P]                        |
| L11 | Attribute comments `"…"` (41) cannot contain `"` or escapes (`"a\"b"` fails). A comment must follow the keys (`… "c" PK` fails).                                                                                               | 41, 296–299                       | [P]                                                                                                                           |
| L12 | `string?` is a valid type (the `?` is a single-character token, 44, and grammar 289). `string name?` fails.                                                                                                                    | 44, 289                           | [P]                                                                                                                           |
| L13 | Blocks: `\s+` skips newlines; an empty block `A {}` and `A {\n}` are valid.                                                                                                                                                    | 34, 154–167                       | [P]                                                                                                                           |
| L14 | A line may hold several statements: there is no separator rule, so `title foo` is two entities.                                                                                                                                | 114–118                           | [P]                                                                                                                           |
| L15 | `title` is not a token (no rule in the lexer). `title: x` fails.                                                                                                                                                               | 188 (dead)                        | [P]                                                                                                                           |
| L16 | The detector is case-sensitive, so `erdiagram` as a first line never reaches the parser, although the lexer accepts it.                                                                                                        | erDetector.ts 10                  | [R]                                                                                                                           |

### 2.3 Grammar statements and DB calls [R, P]

DB calls run in the order of the reduction. Entities are created before the relationship that uses them, which fixes the `entity-NAME-N` ids.

| Form                                     | Grammar      | DB calls in order                                                                                                                    | Notes                                                                                          |
| ---------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- | --- | --- | --- | --------------------------------------------------------------------------- |
| `A rel B : role`                         | 122–128      | `addEntity(A)`, `addEntity(B)`, `addRelationship(A, role, B, relSpec)`                                                               | `relSpec` is `{cardA: right, relType, cardB: left}` (312)                                      |
| `A:::c rel B : role`                     | 129–137      | the same, then `setClass([A], c)`, `setClass([B], c2)`                                                                               | `:::` may follow A, or B, or both                                                              |
| `A rel B:::c : role`                     | 146–153      | `addEntity` ×2, `addRelationship`, `setClass([B], c)`                                                                                |                                                                                                |
| `A { … }`                                | 154–159      | `addEntity(A)`, `addAttributes(A, list)`                                                                                             | list is reversed by the grammar (`attributes`, 274–277); the DB restores source order (95–110) |
| `A:::c { … }`                            | 160–166      | `addEntity`, `addAttributes`, `setClass`                                                                                             |                                                                                                |
| `A`, `A {}`                              | 167, 169     | `addEntity(A)`                                                                                                                       |                                                                                                |
| `A:::c`                                  | 170          | `addEntity(A)`, `setClass([A], [c])`                                                                                                 |                                                                                                |
| `A["alias"]`                             | 186–187      | `addEntity(A, alias)`                                                                                                                | standalone only: `A["x"]                                                                       |     | --  |     | B : x` fails [P]; alias set only if the entity has none yet (erDb.ts 75–78) |
| `A["alias"] { … }`                       | 171–185      | `addEntity(A, alias)`, `addAttributes`, `setClass`                                                                                   |                                                                                                |
| `accTitle: text`                         | 189          | `setAccTitle(trim(value))`                                                                                                           | value `[^\n]*` (13)                                                                            |
| `accDescr: text`                         | 190          | `setAccDescription(trim(value))`                                                                                                     |                                                                                                |
| `accDescr { … }`                         | 191          | `setAccDescription(trim(value))`                                                                                                     | block ends at the first `}`                                                                    |
| `direction TB\|BT\|RL\|LR`               | 192, 218–227 | top level: `setDirection(value)` and an empty list. Inside a subgraph: returns `{stmt:'dir', value}`                                 | the choice uses `yy.subgraphDepth` (192)                                                       |
| `classDef ids styles`                    | 229–231      | `addClass(ids, styles)`; statement value is the string `classDef`                                                                    |                                                                                                |
| `class ids ids`                          | 240–242      | `setClass(ids, classes)`; statement value is the string `class`                                                                      |                                                                                                |
| `style ids styles`                       | 244–246      | `addCssStyles(ids, styles)`; statement value is the string `style`                                                                   |                                                                                                |
| `subgraph id [title]` / `subgraph title` | 199–216      | at the header: `subgraphDepth += 1` (202, 208). At `END` (196): `addSubGraph({text: id}, body, {text: title})`, `subgraphDepth -= 1` | the body list is concatenated from its statements                                              |
| empty lines                              | 116, 117     | none                                                                                                                                 |                                                                                                |

Phantom members [P]. The statement values of `classDef`, `class`, `style`, `accTitle`, `accDescr` and the `direction` object are returned into the enclosing list, and `addSubGraph` keeps them as members:

```
subgraph G
  A ||--o{ B : x
  classDef foo fill:#f9f     -> addSubGraph(G, [A, B, "classDef"], ...)
end
subgraph G / accTitle: T / A / end  -> addSubGraph(G, ["T", "A"], ...)
```

Effect [R, not executed]. `getData` maps every member name to the subgraph (`erDb.ts` 387–395), so an entity named like a phantom (`T` above) is drawn inside G. No fixture has one of these statements inside a subgraph (checked over the 15 subgraph fixtures). The subgraph specs do not test it either.

### 2.4 Cardinality tokens [R]

| Token                                                                                                  | Lines | Value                          |
| ------------------------------------------------------------------------------------------------------ | ----- | ------------------------------ |
| `\|\|` (80), `one` (72), `only one` (73), `1` before `--` etc. (75–77)                                 |       | ONLY_ONE                       |
| `\|o` (62), `o\|` (81), `zero or one` (63), `one or zero` (58)                                         |       | ZERO_OR_ONE                    |
| `o{` (82), `}o` (67), `zero or more` (64), `zero or many` (65), `many` (70), `many(0)` (68), `0+` (66) |       | ZERO_OR_MORE                   |
| `\|{` (83), `}\|` (71), `one or more` (59), `one or many` (60), `1+` (61), `many(1)` (69)              |       | ONE_OR_MORE                    |
| `u` before `.`, `-`, `\|` (84)                                                                         |       | MD_PARENT (no marker, see 5.6) |
| `--` (86), `to` (87)                                                                                   |       | IDENTIFYING                    |
| `..` (85), `optionally to` (88), `.-` (89), `-.` (90)                                                  |       | NON_IDENTIFYING                |

`A }o--|o B` gives cardB ZERO_OR_MORE (left) and cardA ZERO_OR_ONE (right) [P].

### 2.5 DB state after each call (`erDb.ts`) [R]

| Method                                                | Lines             | State and rule                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ----------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `addEntity(name, alias='')`                           | 61–81             | new entity if absent: `id = entity-${name}-${entities.size}` (64), `label = name`, `attributes []`, `alias`, `shape 'erBox'`, `look = getConfig().look ?? 'default'` (69), `cssClasses 'default'` (70), `cssStyles []`, `labelType 'markdown'`. An existing entity takes `alias` only if it has none. Returns the entity.                                                                                                                                                                                                  |
| `addAttributes(name, list)`                           | 95–110            | `addEntity(name)` first. Iterates the list from the end: each attribute gets `keys = []` and `comment = ''` if missing, then is pushed (so the source order is kept). Appends to existing attributes.                                                                                                                                                                                                                                                                                                                      |
| `addRelationship(a, role, b, spec)`                   | 120–154           | ids: a subgraph id if `subGraphLookup` has it, else the entity id (`addEntity`). Pushes `{entityA, roleA, entityB, relSpec}`. Entity ids, not names.                                                                                                                                                                                                                                                                                                                                                                       |
| `setDirection(d)`                                     | 164–166           | last one wins. `clear()` does not reset it (366–375), but production gets a new DB per parse (erDiagram.ts 9–11).                                                                                                                                                                                                                                                                                                                                                                                                          |
| `addCssStyles(ids, styles)`                           | 182–207           | for each id: entity `cssStyles.push`; subgraph `cssStyles` (created empty if absent). Unknown ids are ignored.                                                                                                                                                                                                                                                                                                                                                                                                             |
| `addClass(ids, styles)`                               | 209–227           | create class `{id, styles: [], textStyles: []}` if absent; each style goes to `styles`; a style matching `/color/` also goes to `textStyles` with `fill` replaced by `bgFill` (first occurrence, 220).                                                                                                                                                                                                                                                                                                                     |
| `addSubGraph(id, list, title)`                        | 229–297           | `id = id.trim()` (234); members flattened, `{stmt:'dir'}` removed (last dir wins), empty and duplicate strings removed (`uniq`, 237–267); `title = sanitizeText(title)` (277) then trimmed (283); `subCount++` (278); `labelType` is always 'markdown' from the grammar (`_title.type` is undefined; 355–364); `nodes = makeUniq(...)` (293), removing members of earlier subgraphs (338–349, warning 343). Pushed to `subGraphs` (294), lookup set (295). A repeated id therefore creates a second group node (U, see 7). |
| `setClass(ids, classes)`                              | 303–319           | entity: `cssClasses += ' ' + name` for each class (308); subgraph: `classes.push` (315). Unknown ids are ignored.                                                                                                                                                                                                                                                                                                                                                                                                          |
| `getData()`                                           | 377–459           | see section 5.2.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `setAccTitle`, `setAccDescription`, `setDiagramTitle` | commonDb.ts 16–30 | **module-level state**, shared by every diagram; sanitise (`sanitizeText`), then `^\s+` removed (title), `\n\s+`→`\n` (description)                                                                                                                                                                                                                                                                                                                                                                                        |

Subgraph ids and entity names share one namespace. `G ||--o{ B` with a subgraph `G` adds an entity `G` (the grammar calls `addEntity` first), which `getData` skips (417–421), while the relationship uses the subgraph id [P, R].

### 2.6 Errors [P, R]

- A parse error is a Jison exception ("Parse error on line N: …") raised at the first token that cannot continue. No model is produced. A `subgraph` without `end` fails [P]; `subgraph G` followed directly by `end` gives an empty subgraph [P].
- DB calls made before the error stay in the DB (the trace of `A xxx B : has` shows `addEntity(A)`, `addEntity(xxx)` before the error). The render stops, so nothing sees them.
- The ER DB throws nothing. Its only messages are `log.warn` (makeUniq, erDb.ts 343) and, at render, `Unknown arrow type: md_parent` (edgeMarker.ts 92).
- The specs use `toThrow()` with no message (7 cases), `toThrowError()` (1) and `.not.toThrow()` (1). ADR 0002 applies: the port asserts failure and position, not Jison text.

### 2.7 Other grammar points [P]

- Subgraph title: `subgraph id [My Title Here]` gives id `id` and title `My Title Here` (words joined by one space, 215) [P].
- `subgraph "Group 1.5"` gives id and title `Group 1.5` (quotes stripped, 267) [P].
- `accTitle:   spaced title   ` gives `spaced title`; `accDescr {` with `line one`, a blank line, `  line two` and `}` gives `line one\n\n  line two` from the grammar [P], then `line one\nline two` after the DB setter (`\n\s+`→`\n`, commonDb.ts 23).

## 3. Canonical model proposal

Follows `docs-port/canonical-model.md` (version 1, camelCase, `""` for absent text, `null` only where the DB leaves `undefined`). Model after parsing, the DB state:

```json
{
  "version": 1,
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "direction": "TB",
  "entities": [
    {
      "id": "entity-CUSTOMER-0",
      "name": "CUSTOMER",
      "alias": "",
      "attributes": [{ "type": "string", "name": "name", "keys": ["PK"], "comment": "" }],
      "classes": ["myClass"],
      "styles": ["color:red"]
    }
  ],
  "relationships": [
    {
      "entityA": "entity-CUSTOMER-0",
      "roleA": "places",
      "entityB": "entity-ORDER-1",
      "relSpec": { "cardA": "ZERO_OR_MORE", "relType": "IDENTIFYING", "cardB": "ONLY_ONE" }
    }
  ],
  "classes": [{ "id": "default", "styles": ["fill:#f9f"], "textStyles": [] }],
  "subgraphs": [
    { "id": "G", "title": "G", "nodes": ["A", "B"], "classes": [], "styles": [], "dir": null }
  ]
}
```

| Field                     | DB source                                                  | Rule                                                                                                                                                      |
| ------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `title`                   | `setDiagramTitle` from the front matter (Diagram.ts 52–54) | front matter only: the grammar has no title statement (L15). Sanitised (commonDb 28–30).                                                                  |
| `accTitle`                | commonDb 16–18                                             | grammar `trim` (189), then sanitise and `^\s+` removed                                                                                                    |
| `accDescr`                | commonDb 22–24                                             | grammar `trim` (190, 191), then sanitise and `\n\s+`→`\n`                                                                                                 |
| `direction`               | `direction` (27), 164–166                                  | default `TB`; last top-level statement wins; a `direction` in a subgraph is not here                                                                      |
| `entities[].id`           | erDb.ts 64                                                 | keep: the DB uses it as the node id and in relationships                                                                                                  |
| `entities[].name`         | 63, 66                                                     | the name as written (quotes removed, 267)                                                                                                                 |
| `entities[].alias`        | 75–78                                                      | `""` when none                                                                                                                                            |
| `entities[].attributes`   | 95–110                                                     | source order; `keys` as written (L10); `comment` `""` default                                                                                             |
| `entities[].classes`      | `cssClasses` (70, 308)                                     | the list of class names, without the `default` in the string                                                                                              |
| `entities[].styles`       | 186–207                                                    | the `cssStyles` list                                                                                                                                      |
| `relationships`           | 120–154                                                    | `entityA` / `entityB`: entity id or subgraph id (both are the DB's values)                                                                                |
| `relationships[].relSpec` | 312                                                        | cardinalities and relType as the DB strings (`MD_PARENT` included)                                                                                        |
| `classes`                 | 209–227                                                    | `id`, `styles`; `textStyles` is derived by a pure function of `styles` and can be left out                                                                |
| `subgraphs`               | 229–297                                                    | `id`, `title` (sanitised, trimmed), `nodes` (order kept, including the phantoms of 2.3), `classes`, `styles` (`cssStyles`), `dir` (`null` when undefined) |

Left out, and why:

| Left out                                                     | Reason                                                                                                 |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| `shape` (`erBox`), `labelType` (always `markdown`)           | constants of the DB; the renderer decides                                                              |
| `look` (`addEntity` line 69)                                 | taken from configuration at parse time; config is not in the model (canonical-model.md, configuration) |
| `colorIndex`, `cssCompiledStyles` (425–426)                  | set by `getData`, a render step                                                                        |
| `subgraphDepth`, `subCount`                                  | parser state; the effect of `subCount` is in ids that are not generated (see 5.2)                      |
| `textStyles`                                                 | derived (see above)                                                                                    |
| `layoutDirection`, `fontSize`, `stroke`, `fill` of ER config | not read by the DB or by the renderer (5.8)                                                            |

Sanitisation: the ER DB sanitises the header texts (`accTitle`, `accDescr`), the subgraph title (erDb.ts 277) and the front matter title. It does **not** sanitise entity names, aliases, attribute texts, roles, class styles or subgraph ids at parse time. At render time, the entity label of a box without attributes is sanitised (`labelHelper` → `sanitizeText(decodeEntities(label))`, util.ts 95). The `sanitizeText` path depends on configuration: `sanitizeMore` (common.ts 56–69) applies `removeScript` when `htmlLabels` is on and `securityLevel` is strict; the final DOMPurify call uses `{FORBID_TAGS: ['style']}` (71–83). The exporter must pin `htmlLabels` and `securityLevel` as the harness does (M5's decision table). `crates/ferrigura-syntax/src/sanitize.rs` covers the final call.

The `N` in `entity-NAME-N` is the entity count at creation, so ids follow first appearance in the text. `clear()` resets the count.

## 4. Specs

### 4.1 Files and counts

Counts are derived from the source by script (`count-specs.mjs`). Running vitest was not possible without writing caches.

| Spec file                     | `it(` lines                 | Loops                                                                                                                                                                                                                                    | Skipped      | Run cases                     | Level                                                                                                                |
| ----------------------------- | --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ----------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `er/erDb.spec.js`             | 8                           | none                                                                                                                                                                                                                                     | 0            | **8**                         | DB (6): lines 12–29, 33–50, 54–64, 119–128. getData (2): lines 68–99 (nested clusters), 101–115 (edges to subgraphs) |
| `er/parser/erDiagram.spec.js` | 110 (+1 `it.each`, 3 cases) | `forEach` over 104 characters (line 40): 5 `it` per character (46, 55, 62, 69, 76) = **520**. The `it` at line 88 (all characters together) is outside the loop and is one of the 110. `forEach` at 113 and 239 run inside one test each | 1 (line 124) | **633 run + 1 skipped = 634** | parser with DB state; no DOM                                                                                         |
| `er/parser/subgraph.spec.js`  | 18                          | none                                                                                                                                                                                                                                     | 0            | **18**                        | parser with DB state                                                                                                 |
| **Total**                     |                             |                                                                                                                                                                                                                                          |              | **659 run + 1 skipped = 660** |                                                                                                                      |

- The `chars` list (line 34–…) has 104 entries, with `ˆ` and `π` each twice; so 10 test titles appear twice. The port needs unique names or an index suffix.
- 2 tests have empty bodies (`/* TODO */`, lines 575 and 579) and pass trivially. Port them as ignored with that reason.
- STATUS's "≈ 142" is the count of `it(` lines (8 + 115 + 18 = 141, plus the skip), not of cases.

### 4.2 Classification

| Level                          | Examples                                                                            | Needs                                     |
| ------------------------------ | ----------------------------------------------------------------------------------- | ----------------------------------------- |
| Parser (grammar, lexer)        | all of `erDiagram.spec.js` apart from the DB accessors; `subgraph.spec.js`          | the parser and the DB state               |
| DB                             | `erDb.spec.js` add/makeUniq/relationship/classes                                    | the DB only                               |
| getData (render data, M5-type) | `erDb.spec.js` lines 68–115                                                         | `getData` (5.2)                           |
| DOM / SVG                      | none in the ER spec files                                                           | —                                         |
| Config                         | `setConfig({securityLevel:'strict'})` (erDiagram.spec.js 5–7, subgraph.spec.js 5–7) | the config model, only for the sanitiser  |
| Module state                   | `accTitle` tests depend on `commonDb` state (clear in `beforeEach`)                 | port the module state with the same clear |

### 4.3 Specs outside `diagrams/er` that touch ER

| File                                                                                                                                                                               | What it tests for ER                                                                                                                                                                                             | Level                 |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- |
| `diagrams/paletteCssGeneration.spec.ts` (52, 178–200)                                                                                                                              | palette rules of the ER stylesheet (`er` in `SLOT_STYLESHEETS`)                                                                                                                                                  | style, stylesheet     |
| `diagrams/common/colorThemeGate.spec.ts` (48)                                                                                                                                      | the gate for ER                                                                                                                                                                                                  | style                 |
| `rendering-util/layout-algorithms/elk/__tests__/render.spec.ts` (380–400)                                                                                                          | `prepareLayoutForElk` keeps ER edge classes and arrow types                                                                                                                                                      | ELK input             |
| `config.appearance.spec.ts` (30), `defaultTheme.spec.ts` (93, 102)                                                                                                                 | ER scope for look and theme                                                                                                                                                                                      | config                |
| `rendering-util/multi-diagram-id-uniqueness.spec.ts` (33)                                                                                                                          | ER ids on one page                                                                                                                                                                                               | render                |
| `e2e/rendering/er/nightly-check/*.spec.js`                                                                                                                                         | 3 (`erDiagram.spec.js`) and 6 (`erDiagram-unified.spec.js`, three options × two tests) Playwright cases with screenshots, and style checks: `width="100%"`, `max-width` 140 ± 10% (lines 25–46), `width` (48–64) | browser, not portable |
| `e2e/other/pr-check/xss.spec.js` (190), `e2e/rendering/common-tests/…/per-diagram-appearance.spec.ts` (13), `e2e/rendering/requirement/…` and `e2e/rendering/error/…` (mention ER) | XSS and appearance                                                                                                                                                                                               | e2e                   |

`terminalLabels.spec.ts` builds class-diagram data (`type: 'classDiagram'`), not ER data.

## 5. Renderer

### 5.1 Pipeline [R]

`erRenderer-unified.ts` `draw` (10–79) does, in order:

1. `diag.db.getData()` (17) → `data4Layout` with `nodes`, `edges`, `config`, `direction`.
2. `type = diag.type` (22) = `'er'`. `layoutAlgorithm = getRegisteredLayoutAlgorithm(layout)` (23).
3. `config.flowchart.nodeSpacing = er.nodeSpacing || 140` and `rankSpacing = er.rankSpacing || 80` (26–27). **These are written to the shared flowchart config, and no ELK code reads them** (see 5.8).
4. `direction = db.getDirection()` (28).
5. Markers: `neo` → `only_one_neo, zero_or_one_neo, one_or_more_neo, zero_or_more_neo`; otherwise the four classic names (32–41).
6. `diagramId = id` (42); `await render(data4Layout, svg)` (43) → `rendering-util/render.ts` (81–103): node domIds are prefixed with the diagram id (`<svgId>-<id>`, 86–93), `insertLookDefs`, then the layout loader (ELK).
7. Background nodes for handDrawn (53–68).
8. Title: `utils.insertTitle(svg, 'erDiagramTitleText', er.titleTopMargin ?? 25, db.getDiagramTitle())` (71–76). The title text is placed at `y = -titleTopMargin` and `x` at the centre of the diagram box (utils.ts 793–813).
9. `setupViewPortForSVG(svg, 8, 'erDiagram', er.useMaxWidth ?? true)` (78). The viewBox is the **svg's `getBBox()` (which includes the title)** plus 8 on each side (setupViewPortForSVG.ts 28–40). `diagramPadding` is **not** the viewBox padding in ER.

Accessibility (`<title>`, `<desc>`) is added for all diagrams in `mermaidAPI.ts:757`.

### 5.2 `getData` mapping [R]

| Field                                    | Subgraph node (erDb.ts 398–415)                      | Entity node (419–433)                            | Edge (436–457)                                                                                                          |
| ---------------------------------------- | ---------------------------------------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| `id`                                     | subgraph id                                          | `entity-NAME-N` (64)                             | `getEdgeId(A, B, {prefix:'id', counter: k})` → `id_<A>_<B>_<k>` (utils.ts 908–926), `k` counts relationships, not pairs |
| `label`                                  | title                                                | name (alias replaces it in erBox, 22–24)         | `roleA`                                                                                                                 |
| `labelType`                              | `labelType` (always markdown)                        | `markdown`                                       | `markdown`                                                                                                              |
| `parentId`                               | `parentDB.get(subgraph id)` (405)                    | `parentDB.get(name)` (429)                       | —                                                                                                                       |
| `padding`                                | 8                                                    | —                                                | —                                                                                                                       |
| `shape`                                  | `rect`                                               | `erBox`                                          | —                                                                                                                       |
| `isGroup`                                | true                                                 | false                                            | —                                                                                                                       |
| `dir`                                    | `subgraph.dir` (undefined when none; layout default) | —                                                | —                                                                                                                       |
| `cssClasses`                             | `classes.join(' ')`                                  | `'default' + ' cls'…`                            | `relationshipLine` (449)                                                                                                |
| `cssStyles`                              | `subgraph.cssStyles`                                 | `entity.cssStyles`                               | —                                                                                                                       |
| `cssCompiledStyles`                      | `getCompiledStyles(classes)` (407)                   | `getCompiledStyles(cssClasses.split(' '))` (425) | —                                                                                                                       |
| `look`                                   | `config.look` (413)                                  | entity `look`, set at parse time (69)            | `config.look`                                                                                                           |
| `colorIndex`                             | —                                                    | 0, 1, … among entities (418–426)                 | —                                                                                                                       |
| `start`, `end`                           | —                                                    | —                                                | `entityA`, `entityB` (ids, or the subgraph id)                                                                          |
| `arrowTypeStart`                         | —                                                    | —                                                | `relSpec.cardB.toLowerCase()` (450)                                                                                     |
| `arrowTypeEnd`                           | —                                                    | —                                                | `relSpec.cardA.toLowerCase()` (451)                                                                                     |
| `pattern`                                | —                                                    | —                                                | IDENTIFYING → `solid`, else `dashed` (452)                                                                              |
| `thickness`, `type`, `curve`, `labelpos` | —                                                    | —                                                | `normal`, `normal`, `basis`, `c` (437–447)                                                                              |

Order: subgraph nodes in reverse closing order, then entities in insertion order, then edges in insertion order. `getData` also mutates the entity objects (`cssCompiledStyles`, `colorIndex`).

`parentDB` maps each member name to the subgraph that lists it (387–395). `makeUniq` makes the member lists disjoint, so the order of the loop does not change the result.

Skipped: an entity whose name is also a subgraph id (417–421).

### 5.3 `erBox` geometry [R, not measured]

Notation: `PAD` = `er.diagramPadding` (default 20, schema), `TPAD` = `er.entityPadding` (default 15). Both are scaled by 1.25 when `htmlLabels` is false, **but only on the attribute path** (82–85). The harness forces `htmlLabels: false`.

**Path A: no attributes, non-empty label** (52–80)

```
PAD_A = PAD                                  (unscaled, line 45 value)
if calc(label) + 2*PAD_A < minEntityWidth (100): node.width = 100        61-66
drawRect(labelPaddingX = PAD_A, labelPaddingY = 1.5*PAD_A, rx = ry = 0)  67
  bbox = labelHelper(markdown label, wrap = node.width || node.wrappingWidth || flowchart.wrappingWidth (200))
  w = max(bbox.w + 2*PAD_A, node.width || 0)
  h = max(bbox.h + 3*PAD_A, node.height || 0)
  <rect x=-w/2 y=-h/2 width=w height=h class="basic label-container">   drawRect.ts 51-61
  label translate(0, -bbox.h/2) (svg); text translate(-bbox.w/2, 0)     util.ts 147, erBox.ts 74-78
node size: getBBox of the group (insertMeasuredNode, createGraph.ts 61-68) = the rect
```

Note: `calc` is `calculateTextWidth` (utils.ts 649), an integer (`Math.round`, 694), so the minimum-width test uses a rounded value. The wrap width is 200 for entities without attributes (`flowchart.wrappingWidth`), and their names wrap there. Path B does not wrap (see 5.4).

**Path B: at least one attribute** (87–318)

```
PAD = diagramPadding (scaled ×1.25 if htmlLabels false)   45, 82-85
TPAD = entityPadding (scaled the same way)              46, 83-84

name:    text(name) measured, height nameH = h_name + TPAD   98-99
per attribute i (109-156):
  wType_i, wName_i, wKeys_i, wComment_i, heights h_*  (texts: type, name, keys.join(','), comment)
  colType  = max(colType, wType_i + PAD)                                  119
  colName  = max(colName, wName_i + PAD)                                  129
  colKeys  = max(colKeys, wKeys_i + PAD)                                  139
  colComm  = max(colComm, wComment_i + PAD)                               149
  rowH_i   = max(h_type, h_name, h_keys, h_comment) + TPAD                151-153
  rowY_i   = sum of rowH before i                                          154-155

keysPresent    = colKeys > PAD;  if not: colKeys = 0,  sections--         159-163
commentPresent = colComm > PAD;  if not: colComm = 0,  sections--        164-168
sections starts at 4

shapeBBoxW = max width over all texts (all start at x = 0)               170
diff = nameW + 2*PAD - (colType + colName + colKeys + colComm)           172-177
if diff > 0: each present column += diff / sections                      178-185

sumCols = colType + colName + colKeys + colComm                          188
w = max(shapeBBoxW + 2*PAD, node.width || 0, sumCols)                    203
h = max(sum rowH + nameH, node.height || 0)                              204
x = -w/2; y = -h/2                                                       205-206
```

Positions (208–239): name text at `(-nameW/2, y + TPAD/2)`. Attribute text at `(x + PAD/2 + offset, y + rowY_i + nameH + TPAD/2)`, where `offset` is 0 for type, `colType` for name, `colType+colName` for keys, `colType+colName+colKeys` for comment.

Shapes (246–316):

| Element         | Geometry                                                                                                                                                                          | Lines   |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| outer rect      | rough rectangle `(x, y, w, h)`, roughness 0, solid fill; class `outer-path`                                                                                                       | 246–250 |
| row rects       | for each row, `(x, nameH + y + rowY_i, w, rowH_i)`; fill `rowOdd` for i = 0, 2, … and `rowEven` for i = 1, 3, … (contentRowIndex parity, 254–260); inserted before the name group | 254–266 |
| top divider     | horizontal line at `y = nameH + y`, from `x` to `x + w`                                                                                                                           | 273–278 |
| type divider    | vertical at `x = x + colType`, from `nameH + y` to `y + h`                                                                                                                        | 280–285 |
| keys divider    | vertical at `x + colType + colName`, only if `keysPresent`                                                                                                                        | 287–295 |
| comment divider | vertical at `x + colType + colName + colKeys`, only if `commentPresent`                                                                                                           | 297–305 |
| extra divider   | `yOffsets` holds only `0` (252), so one more horizontal line at the top divider's y                                                                                               | 308–316 |

Dividers are polygons 0.0001 wide (`thickness`, 270, `lineToPolygon` 401–424). The node size comes from `getBBox()` of the group (`insertMeasuredNode`, createGraph.ts 64–67): the union of the rect, the rows and the texts, which equals `(x, y, w, h)` because the dividers lie inside the rect. [R]

Quirks to keep:

- `g:not(:first-child)` (209) selects every label group except the first; the first is the name, so the name is centred separately (237–239).
- `nameBBox` in the column adjustment (172–173) is the header's (98), because the loop's `nameBBox` (120) is block-scoped.
- `keys.join()` joins with a comma and no space (132).
- An empty text has a zero box in Chromium (U): a comment-free row takes its height from the other cells.

### 5.4 Text per cell [R]

- `addText` (343–399) calls `createText` with `width = calc(text) + 100` (370), so attribute text never wraps (its own width is below `calc + 100`). `markdown` is not passed, so it defaults to `true` (createText.ts 314): **attribute texts are Markdown**. The name header is Markdown too.
- Attribute text is not sanitised (no `sanitizeText` in the SVG path, createText.ts 364–370). Only `decodeEntities` runs (366).
- `parseGenericTypes` (common.ts 201–266) turns `~K,V~` into `<K,V>`; `<`/`>` are escaped for the parser and restored afterwards (erBox.ts 359–363, 378–386). The width is measured on the decoded text (`decodeHTMLEntities`, createText.ts 213–228).
- Cell boxes come from `text.getBBox()` (388), the SVG text box, not from `calc`.
- Each SVG line is `text.text-outer-tspan` (`x = 0`, `y = i*1.1em - 0.1em`, `dy = 1.1em`) with `tspan.text-inner-tspan` per word (createText.ts 96–112, 159–160, 237–256). M5's `label::layout` implements this.

### 5.5 `neo` and `classic` [R]

| Aspect                              | `neo` (default for ER)                    | `classic`                    | Lines                         |
| ----------------------------------- | ----------------------------------------- | ---------------------------- | ----------------------------- |
| marker names                        | `*_neo` (`only_one_neo` …)                | `only_one`, `zero_or_one`, … | erRenderer 32–41              |
| marker units                        | `userSpaceOnUse`                          | default (`strokeWidth`)      | markers.js 717, 732, 752 …    |
| marker stroke width                 | `themeVariables.strokeWidth` on each path | none                         | markers.js 720, 735, 758–843  |
| zero-or-one / zero-or-more circles  | `fill: mainBkg`                           | `fill: white`                | markers.js 754, 774, 835, 858 |
| zero-or-more circle x (start / end) | 45.5 / 11                                 | 48 / 9                       | markers.js 685, 698, 836, 859 |
| entity box stroke-width (styles.ts) | `strokeWidth`                             | `1px`                        | styles.ts 117, 122            |
| `relationshipLine` stroke-width     | `strokeWidth`                             | `1px`                        | styles.ts 122                 |
| `[data-look=neo].labelBkg`          | tertiary colour rule                      | —                            | styles.ts 131–133             |
| `handDrawn`                         | rough paths, the background copy          | —                            | erBox.ts 30–41, 194           |

`erBox` itself does not branch on `neo`/`classic`. The look is read from `config.look` (top-level) and resolved through the `er` section, which defaults to `neo` (`config.ts` 79–103; `ErDiagramConfig.look` default `neo`, schema 1564–1580). The top-level default is `classic` (schema 68–70), so ER gets `neo` only through this resolution. The Rust `config::effective` takes a diagram key and needs `"er"` (layers.rs 45–103).

### 5.6 Edges, labels, markers [R, P for ids]

Edge pipeline (ELK):

- `buildEdgeData` (elk/render.ts 2715–2750): `minlen = edge.length ?? 1` (ER has none, so 1); `text = label`; `arrowTypeStart`, `arrowTypeEnd` from the DB; `thickness`, `pattern`, `classes` from the DB; `curve` from `interpolateToCurve` (basis) and overwritten after layout.
- Terminal labels: none (`startLabelLeft` etc. are not set by ER, elk/render.ts 1152–1170 returns `[]`).
- Routed edges get `curve = 'rounded'` (elk/render.ts 2078), so the path is drawn as `generateRoundedPath(…, 5)` (edges.js 809–811). `basis` is not drawn, despite `curve: 'basis'` in getData.
- No marker offsets: `markerOffsets` (lineWithOffset.ts 6–16) and `markerOffsets2` (21–25) have no ER entries, and `markerPathOffset` returns 0 (elk/render.ts 143–150). Edges meet the box edge exactly.

Markers:

- Attachment: `addEdgeMarker` (edgeMarker.ts 71–139) puts `marker-start` / `marker-end` = `url(#<svgId>_er-<type><Start|End>)` (99–100). `diagramType` is `er`, from `data4Layout.type` (detector id, `erDetector.ts` 7; `diagram-orchestration.spec.ts` 26 maps `erDiagram` to `er`). Flowchart uses `flowchart-v2` (`crates/…/markers.rs` 15), so ER is not the same prefix.
- Type map (edgeMarker.ts 51–54): `only_one` → `onlyOne`, `zero_or_one` → `zeroOrOne`, `one_or_more` → `oneOrMore`, `zero_or_more` → `zeroOrMore`; all `fill: false`.
- `md_parent`: not in the map, so `Unknown arrow type: md_parent` is logged and no marker is written (92–94). Reproduce the warning-free absence; the warning is not part of the output.
- `-margin` variants: `arrowTypesWithMarginSupport` (59–69) lists none of the ER types, so there are no margin ids.
- `none` / empty: no marker (84–86).

Marker definitions (markers.js, both sets write 8 `marker` elements with the ids above; the registry chooses the four pairs):

| Type                            | Start (refX, refY, w×h, path)                                             | End (refX, refY, w×h, path)                                                |
| ------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| only one, classic (580–606)     | 0, 9, 18×18, `M9,0 L9,18 M15,0 L15,18`                                    | 18, 9, 18×18, `M3,0 L3,18 M9,0 L9,18`                                      |
| zero or one, classic (608–644)  | 0, 9, 30×18, circle (21,9) r 6 white + `M9,0 L9,18`                       | 30, 9, 30×18, circle (9,9) r 6 white + `M21,0 L21,18`                      |
| one or more, classic (646–672)  | 18, 18, 45×36, `M0,18 Q 18,0 36,18 Q 18,36 0,18 M42,9 L42,27`             | 27, 18, 45×36, `M3,9 L3,27 M9,18 Q27,0 45,18 Q27,36 9,18`                  |
| zero or more, classic (674–700) | 18, 18, 57×36, circle (48,18) r 6 white + `M0,18 Q18,0 36,18 Q18,36 0,18` | 39, 18, 57×36, circle (9,18) r 6 white + `M21,18 Q39,0 57,18 Q39,36 21,18` |
| only one, neo (702–736)         | as classic, `userSpaceOnUse`, path stroke-width                           | as classic                                                                 |
| zero or one, neo (738–781)      | as classic; circles `fill: mainBkg`, positions as classic                 | as classic                                                                 |
| one or more, neo (783–816)      | as classic, `userSpaceOnUse`                                              | as classic                                                                 |
| zero or more, neo (818–867)     | circle (45.5, 18) r 6 mainBkg, `M0,18 Q18,0 36,18 Q18,36 0,18`            | circle (11, 18) r 6 mainBkg, `M21,18 Q39,0 57,18 Q39,36 21,18`             |

Edge labels:

- `insertEdgeLabel` (edges.js 74–133): `createText` with `markdown` (labelType), `isNode: false`, `addSvgBackground: true`, width default 200 (createText.ts 319). Class `edgeLabel`, then `label`. An edge has a label only if `roleA` is non-empty (`hasEdgeLabel`, 55–62).
- ELK places the label at the edge centre (`edgeLabels.placement: CENTER`, elk/render.ts 1120–1135). Label size comes from `getMeasuredLabelData`, with `wrappingWidth = flowchart.wrappingWidth` (1068–1085).
- Edge classes: `edge-thickness-normal edge-pattern-solid|dashed relationshipLine` (edges.js 780–806, 842, 865). `.edge-pattern-dashed` is `stroke-dasharray: 8,8` (styles.ts 106–108).

### 5.7 Styles and themes [R]

- `er/styles.ts` `getStyles` (64–149) emits rules for `.entityBox`, `.relationshipLabelBox`, `.labelBkg`, `.edgeLabel`, `.label`, `.edge-pattern-dashed`, `.node rect/circle/ellipse/polygon`, `.relationshipLine`, `.marker`, `.cluster rect`, `.cluster text`, `.cluster-label text`.
- `genColor` (22–62) emits, per palette slot, `[data-look][data-color-id="color-i"].node path` and `… rect` rules, only when `isColorTheme(theme, borderColorArray)` (colorThemeGate.ts 37–38: theme `redux-color` or `redux-dark-color` and a non-empty palette). Slot count = palette length (42).
- Colour stamping: `data-color-id = color-${colorIndex % borderColorArray.length}` (erBox.ts 68–71, 241–244), with `colorIndex` from the entity order.
- Theme variables used: `rowEven`, `rowOdd`, `nodeBorder`, `borderColorArray`, `background` (handDrawn only), `mainBkg`, `lineColor`, `tertiaryColor`, `edgeLabelBackground`, `textColor`, `nodeTextColor`, `fontFamily`, `strokeWidth`, `clusterBkg`, `clusterBorder`, `titleColor`.
- **`rowEven` and `rowOdd` are set only in `theme-base.js` 124–130 (darkMode-dependent). The Rust redux themes do not set them** (`crates/ferrigura-model/src/theme/*.rs` has no match; `ts_keys.txt` lists them). Needed for the row fills.
- Non-shipped theme names (`neo`, `neo-dark`, `redux`, `redux-dark`) fall back by the rule in M2 (names ending in `dark` take `redux-dark-color`, others `redux-color`).

### 5.8 ER config: what is read [R]

| Key (schema default)                                | Read by                               | Effect                                                                                                                                                                                                                                                                                            |
| --------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagramPadding` (20)                               | erBox.ts 45                           | cell and box padding (not the viewBox)                                                                                                                                                                                                                                                            |
| `entityPadding` (15)                                | erBox.ts 46                           | row padding                                                                                                                                                                                                                                                                                       |
| `minEntityWidth` (100)                              | erBox.ts 61–65                        | minimum width of a box without attributes                                                                                                                                                                                                                                                         |
| `titleTopMargin` (25)                               | erRenderer 74                         | title position                                                                                                                                                                                                                                                                                    |
| `useMaxWidth` (true)                                | erRenderer 78                         | svg size (`width="100%"` and `max-width`)                                                                                                                                                                                                                                                         |
| `nodeSpacing` (140), `rankSpacing` (80)             | erRenderer 26–27 → `config.flowchart` | **no ELK reader**: the ELK path reads `config.elk.*` and its own spacing options (elk/render.ts, for example 658–1000). `flowchart.nodeSpacing` is read only by the swimlane and DDLT layouts (`swimlanes/layoutCore.ts` 22–23, `ddlt/parseToLayoutData.ts` 33–36) [R, grep]. Dead for ER on ELK. |
| `layoutDirection` (TB)                              | —                                     | not read; the DB direction is used (erRenderer 28)                                                                                                                                                                                                                                                |
| `fontSize` (12), `stroke` (gray), `fill` (honeydew) | —                                     | not read by the ER code                                                                                                                                                                                                                                                                           |
| `theme`, `look` (per-section defaults)              | config.ts 79–103                      | scope resolution (5.5)                                                                                                                                                                                                                                                                            |

Other ER-visible config: `flowchart.wrappingWidth` (200) for labels, `flowchart.curve` (edges.js 44, only for string-less curves), `htmlLabels` (forced false by the harness), `securityLevel`.

One fixture sets `er: { useMaxWidth: false }` (pr-check/should-render-entities-that-have-no-relationships-usemaxwidth-false.mmd).

### 5.9 What M5 has, and what ER needs [R, from M5 notes and sources]

| Piece                                                                     | ER needs                                         | M5 status                                                                                | Evidence                                                                   |
| ------------------------------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `getData` mapping                                                         | erDb.ts 377–459                                  | flowchart only (`data.rs`)                                                               | —                                                                          |
| `erBox` shape                                                             | erBox.ts 19–424                                  | **missing**: the registry has no `erBox`; `isValidShape` passes it only in the reference | `shapes/registry.rs` (Kind list 12–60, names 79–159)                       |
| SVG text (`createText`, markdown)                                         | yes                                              | yes                                                                                      | `label.rs` 44–184 (`LabelSpec`, `layout`)                                  |
| `labelHelper`, min width                                                  | yes                                              | yes                                                                                      | `shapes/util.rs` 66–85                                                     |
| `drawRect` (plain rect)                                                   | yes                                              | yes                                                                                      | `shapes/rect.rs` 34                                                        |
| roughjs rectangle and polygon at roughness 0                              | outer box, dividers                              | yes                                                                                      | `shapes/rough.rs` 33 (polygon), 42 (rectangle)                             |
| relation markers (8 defs, neo and classic)                                | yes                                              | **missing**: `markers.rs` has point, circle, cross only                                  | `markers.rs` 19–49                                                         |
| `Kind::from_arrow`                                                        | only_one, zero_or_one, one_or_more, zero_or_more | point, circle, cross only                                                                | `markers.rs` 32–39                                                         |
| marker offsets, end-pull-back                                             | none for ER (zero)                               | present for flowchart                                                                    | `elk_apply.rs` 964–1000; `edges.rs` 477–478                                |
| ELK, edges, labels, clusters, line hops, rounded paths                    | yes                                              | yes                                                                                      | `elk_build.rs`, `elk_apply.rs`, `edges.rs`, `clusters.rs`, `line_jumps.rs` |
| ER stylesheet (`styles.ts`)                                               | yes                                              | **missing**: `style.rs` is the flowchart sheet                                           | —                                                                          |
| palette slots (`stampColorSlot`)                                          | ER `erBox` stamps                                | partial (`clusters.rs`, `shapes/rect.rs`)                                                | `grep`                                                                     |
| `insertTitle`, ER viewBox (padding 8 around the bbox including the title) | yes                                              | flowchart viewBox only                                                                   | — [U whether M5 handles a title]                                           |
| front matter title                                                        | yes                                              | yes (M1 `preprocess.rs`)                                                                 | `preprocess.rs` 18–45                                                      |
| header and subgraph sanitiser                                             | yes                                              | yes (`sanitize.rs`)                                                                      | `sanitize.rs` 86–99                                                        |
| theme variables `rowEven`, `rowOdd`                                       | yes                                              | **missing**                                                                              | 5.7                                                                        |
| configuration keys `er.*`, scope `er`                                     | yes                                              | generic config: needs the `er` key                                                       | `config/layers.rs` 45–103                                                  |
| node sizes by `getBBox` of the group                                      | yes                                              | flowchart: `measured boxes are float32` (RENDER-PLAN 21)                                 | —                                                                          |

## 6. Fixtures

### 6.1 Counts [P for the script, R for sources]

| Folder          | `.mmd` files             |
| --------------- | ------------------------ |
| `nightly-check` | 30                       |
| `pr-check`      | 10 (+ `.gitkeep`)        |
| `visual-check`  | 166 (+ `.gitkeep`)       |
| **Total**       | **206** (matches STATUS) |

Front matter: all 206 have one, with `description:` (197 blocks; the `>-` folded scalar is read by the YAML module, which the M2/M3 yaml differential covers). The `config:` block is in 192 of them.

| Front matter key                                   | Fixtures                                                                                                |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `look: handDrawn`                                  | 43 (excluded)                                                                                           |
| `look: neo` (explicit)                             | 54                                                                                                      |
| no `look` (default `neo` through the `er` section) | 109                                                                                                     |
| `layout: elk`                                      | 43 (the rest use the default `elk`)                                                                     |
| `htmlLabels: false`                                | 43 (the rest default to true; the harness forces false)                                                 |
| `theme:`                                           | 9 each of `neo`, `neo-dark`, `redux`, `redux-color`, `redux-dark`, `redux-dark-color` (54); 152 default |
| `er:` section                                      | 1 (`useMaxWidth: false`)                                                                                |
| `title:`                                           | 0 (`pr-check/1433-…-with-a-title.mmd` has the word in its description only)                             |
| `%%{init}%%` directive, `%%` comment lines         | 0                                                                                                       |

Body features (by script, `fm-table.mjs`): attribute blocks 101; aliases `A["…"]` 19; generic types `~` 18; `direction` 16; subgraphs 15; `classDef`/`class`/`style`/`:::` 15 (none inside a subgraph); `<br>` 9; `u` (MD_PARENT) 1 (`u--o{` in `nightly-check/…u-character…`); markdown emphasis in labels: present in some (not counted precisely: U).

Non-ASCII (`fixture-scan.mjs`): 14 files. Glyph coverage of the bundled `LexicaUltralegible-Regular.ttf` and `-Bold.ttf` (cmap read by `cmap.mjs`): covered U+00D8 Ø, U+03B6 ζ, U+1EBD ẽ. **Not covered**: U+2192 →, U+2795 ➕, U+307C ぼ, U+3315 ㌕ (CJK compatibility), U+1F600 😀.

| Group                                                                      | Files  | Comparable (not handDrawn) | Needs a fallback font |
| -------------------------------------------------------------------------- | ------ | -------------------------- | --------------------- |
| `a-cyclical-…` (→)                                                         | 2      | 1 (elk)                    | 1                     |
| `an-er-diagram-with-unicode-text-*` (ideographs, kana, ➕, emoji, Ø, ζ, ẽ) | 6      | 4                          | 4                     |
| `an-er-diagram-with-relationships-with-unicode-text-*` (emoji)             | 6      | 4                          | 4                     |
| **Total**                                                                  | **14** | **9**                      | **9**                 |

So 9 comparable fixtures need glyphs the port does not have (no fallback fonts yet, M2 "Open" list). The flowchart excluded its CJK fixture with a named reason; the same applies here.

### 6.2 Browser features ferrigura does not have

- Fallback fonts for the 9 glyph fixtures above (not in `ferrigura-text`, which states Latin only).
- `htmlLabels: true` is the default for 163 fixtures. The harness forces `htmlLabels: false` (as for flowchart), so no HTML label path is needed for the comparison. [R]
- handDrawn (43): roughjs at `roughness 0.7` with random seeds; out of scope like flowchart (the RENDER-PLAN covers `handDrawn`).
- Themes not shipped (`neo`, `neo-dark`, `redux`, `redux-dark`; 36 fixtures): replaced on both sides by the fallback, as in M5.
- Playwright and screenshot checks (`e2e/rendering/er`): not portable.

### 6.3 Comparable set

206 − 43 (handDrawn) − 9 (glyphs) = **154** candidate fixtures for the geometry comparison, before the ELK and text checks.

## 7. Risks, effort, open questions

### 7.1 Risks

| #   | Risk                                                                    | Why                                                                                                                                             | Mitigation                                                                                                                    |
| --- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| R1  | Phantom subgraph members (2.3)                                          | DB state depends on statement values; the port must reproduce them, or a subgraph with `classDef` differs                                       | reproduce; a differential case in `cases.ts`                                                                                  |
| R2  | `calc` is an integer DOM measure (utils.ts 649–731)                     | the minimum-width test and the wrap width use `Math.round` and two font families (`sans-serif` and the configured one, index chosen at 720–730) | port the rule for the Lexica case, record the sans-serif choice as a named risk [U whether the index is always 1 in Chromium] |
| R3  | Node size is `getBBox` of the group, float32                            | M5 found the exact `>=` tests of post-processing sensitive to one ulp                                                                           | the same float32 rule as M5                                                                                                   |
| R4  | Empty texts have zero boxes (U)                                         | rows and column widths depend on it                                                                                                             | check in Chromium once                                                                                                        |
| R5  | `nodeSpacing`/`rankSpacing` dead for ELK                                | grep finds no ELK reader [R]; the port should not read them for ER                                                                              | confirm with one reference run with a changed `nodeSpacing` (U, not run)                                                      |
| R6  | Duplicate subgraph ids make two group nodes (erDb.ts 294, 295)          | ELK may reject duplicate ids [U]                                                                                                                | check with the reference (not run)                                                                                            |
| R7  | `colorIndex` order differs from the subgraph slot order                 | `erBox` uses entity order only (418–426)                                                                                                        | keep the DB order                                                                                                             |
| R8  | Lexer quirks L1, L9 (greedy)                                            | a port with longest-match or line-based lexing differs                                                                                          | hand-written lexer, first-match table (as flowchart, 0002)                                                                    |
| R9  | `accDescr` loses blank lines (`\n\s+`)                                  | the setter is not the block converter of pie/info                                                                                               | ER uses its own rule (canonical doc section for ER)                                                                           |
| R10 | Sanitiser depends on `htmlLabels` and `securityLevel` (common.ts 56–69) | output can differ with configuration                                                                                                            | the exporter pins both                                                                                                        |
| R11 | `calc` vs SVG width of markdown text                                    | `calc` is plain text; `labelHelper` measures the Markdown runs                                                                                  | keep both, as the reference does                                                                                              |
| R12 | Phantom node and the `entity G` collision                               | entity `G` skipped in `getData`, subgraph `G` used by relationships                                                                             | port exactly                                                                                                                  |

### 7.2 Effort relative to the flowchart

| Part                      | ER                                                        | Flowchart reference                                  | Estimate                                |
| ------------------------- | --------------------------------------------------------- | ---------------------------------------------------- | --------------------------------------- |
| Grammar, lexer, DB (Rust) | 336-line grammar, 468-line DB, six states                 | flow.jison 634, flowDb.ts 1358; M3 Rust ≈ 3.6k lines | ≈ 1.5–2k lines with tests, 40–50% of M3 |
| Specs                     | 660 cases, 520 generated                                  | 958 cases                                            | mechanical, except the loop naming      |
| Renderer, ER-specific     | erBox 424, markers 290, styles 151, getData 80, render 79 | M5 ≈ 20k Rust lines                                  | ≈ 1.2–1.5k lines, about a quarter of M5 |
| Shared reuse              | ELK, labels, edges, clusters, text, config, sanitiser     | —                                                    | large part of the work is done          |

### 7.3 Open questions for the coordinating session

1. Relationships in the model: keep the DB's entity ids (`entity-A-0`) or names? The notes propose ids, because the DB stores ids.
2. Phantom members and the entity/subgraph name collision: reproduce both (as DB behaviour), or name them as exceptions? The notes propose reproducing them.
3. `nodeSpacing`/`rankSpacing` and the write to `config.flowchart`: not ported, with a named reason, or ported as written to config?
4. Fixtures with glyphs the bundled font lacks (9): excluded with a named reason, or a fallback font first?
5. Theme variables `rowEven`/`rowOdd`: add to the Rust redux themes (theme-base.js 124–130)? Colour only, but they are in every row fill.
6. Classic markers: needed only by the contract (no fixture uses them); port both, as M5 did for flowchart?
7. Sanitisation of entity names, aliases and roles: none in the DB (canonical rule), so the model keeps them raw. Confirm.
8. `md_parent`: no marker and a warning in the reference. Reproduce the absence (proposed), and drop the warning.
9. Attribute keys in the model: raw (`pk` stays `pk`), as the DB does. Confirm.

### 7.4 Unverified (U) list

- Chromium widths and bboxes of every ER text, including empty ones (R4) and the `calc` index choice (R2).
- The integer rounding of `calculateTextWidth` and where it changes a result.
- Whether `updateNodeBounds` and `insertMeasuredNode` differ by a float32 ulp for ER boxes (R3).
- The reference's output for duplicate subgraph ids (R6) and the `nodeSpacing` non-effect (R5). Both are from reading only.
- Whether M5's viewBox code covers the ER title and padding 8 (5.9).
- Test counts: derived from the source. Vitest was not run.
- Glyph coverage: read from the cmap with a minimal parser, not with a font library. The count is reliable for these code points.
- Markdown emphasis fixtures: counted by a rough pattern, not exact.
