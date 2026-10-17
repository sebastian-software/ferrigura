# C4 parser: plan

Status: parser, DB and model ported; the differential (`pnpm port:diff c4`) shows no difference on the 176 collected inputs, and `pnpm port:diff config` agrees on the C4 section. The research is in [NOTES.md](./NOTES.md), which cites every rule; this page says what the port does and which quirks it keeps. The quirks are listed by their ids `P1`-`P32` of NOTES section 2.4, and the code comments name them.

The renderer (the grid, the boundaries, the relations and the label) is not ported. Its inputs are part of the 32 fixtures and are checked with the geometry harness in the renderer task.

## 1. Sources

| File                                       | Used for                                                                                                                                                                                                                              |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/c4/parser/c4Diagram.jison`       | Lexer rules (lines 79-186), start states (8-77), grammar (196-322)                                                                                                                                                                    |
| `diagrams/c4/c4Db.ts`                      | Every DB call: `addRel` (86), `addPersonOrSystem` (151), `addContainer` (199), `addComponent` (259), boundaries (319-484), `updateElStyle` (494), `updateRelStyle` (593), `updateLayoutConfig` (641), `clear` (721), `setTitle` (773) |
| `diagrams/c4/c4Diagram.ts`                 | `init`: `db.setWrap(wrap)`                                                                                                                                                                                                            |
| `diagrams/common/commonDb.ts`              | `setAccDescription` (sanitising, `\n\s+` to `\n`)                                                                                                                                                                                     |
| `Diagram.ts` (49-77)                       | Order of `clear`, `init`, the entity encoding and the line break                                                                                                                                                                      |
| `preprocess.ts`, `diagram-api/comments.ts` | Front matter, directives, comment lines, `trimStart` (shared)                                                                                                                                                                         |

The Rust code: `crates/ferrigura-syntax/src/c4/` (`mod.rs` entry point, `scanner.rs` lexer, `grammar.rs` recursive descent, `db.rs` DB), the model in `crates/ferrigura-model/src/c4.rs`, the exporter `tools/port/export-c4.ts`.

## 2. Text before the lexer

The steps of `Diagram.fromText` after `preprocessDiagram`: CRLF to LF, front matter removed (`prepare`), directives removed (`remove_directives`), comment lines removed and the start trimmed (`cleanup_comments`), entities encoded (`encode_entities`), and one line break appended. The front matter title does not reach a C4 diagram: the C4 DB has no `setDiagramTitle`, and `Diagram.fromText` calls it with `?.` (`Diagram.ts`, `db.setDiagramTitle?.`).

The wrap flag comes from the effective configuration: `init` calls `db.setWrap(config.wrap)` after `clear()`. The Rust parser takes it as `Options::wrap`, and the canonical example resolves it from the directives and the front matter, as the sequence arm does (`c4_wrap` in `canonical.rs`). A configuration that does not set it gives `false`.

## 3. Scanner

Jison takes the first rule that matches at the current position, not the longest, and the rules without a start state apply only in the initial state (every state is exclusive). The scanner keeps that order. The rule table, in the grammar's order:

| #   | Rule (initial state unless stated)                                                                | Token                                          |
| --- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| 1   | `.*direction\s+(TB\|BT\|RL\|LR)`: a line from here holds `direction` and a direction name         | direction, always an error (P8, see section 8) |
| 2   | `title`, one white space character, then one or more characters other than `#`, a line break, `;` | title, the value after six characters (P10)    |
| 3   | `accDescription`, the same shape, the value after fifteen characters                              | accDescription (not trimmed)                   |
| 4   | `accTitle\s*:\s*`, enters the acc_title state                                                     | acc_title                                      |
| 5   | (acc_title) the rest of the line, possibly empty                                                  | acc_title_value, leaves the state              |
| 6   | `accDescr\s*:\s*`, enters the acc_descr state                                                     | acc_descr                                      |
| 7   | (acc_descr) the rest of the line                                                                  | acc_descr_value, leaves the state              |
| 8   | `accDescr\s*{\s*`, enters acc_descr_multiline                                                     | none                                           |
| 9   | (acc_descr_multiline) `}` leaves the state; otherwise everything up to `}`                        | acc_descr_multiline_value (P27)                |
| 10  | `%%` and the rest of the line, with the line breaks after it (`(?!\{)*` matches nothing)          | none (comment)                                 |
| 11  | `\s*(\r?\n)+`: white space up to its last line break                                              | NEWLINE                                        |
| 12  | `\s+`                                                                                             | none                                           |
| 13  | the five headers, each with a word boundary                                                       | header                                         |
| 14  | the keywords (elements, boundaries, nodes, relations, updates), longer first, word boundary       | keyword; enters the structure state (P30)      |
| 15  | `{`, `}`                                                                                          | LBRACE, RBRACE                                 |
| 16  | end of the input                                                                                  | EOF                                            |

In a structure state, `(` with optional spaces and a comma gives an empty attribute, `(` enters the attribute state, and `)` leaves two states (the end of an element, whether or not an attribute is open). An unmatched character is a lexical error (P1: a space before `(`).

In the attribute state the rules run as in the grammar: `,,` and `""` are empty attributes; a comma is skipped; `[ ]*"` starts a quoted text, which ends at the next quote and has no escapes (P11); `[ ]*$` starts `$key="value"` (P12, P13); and `[^,]+` is an unquoted text up to the next comma, which runs over the closing parenthesis and the line breaks (P2, P4, P5, P6, P7).

Jison's end-of-input behaviour is kept: once the input is empty, the next call reads the rules of the current state at the end, and afterwards returns the raw end symbol, which the grammar rejects. So a quoted text or an accessibility value that ends the input is an error, and so is an unquoted text at the end (P3, P4).

## 4. Grammar and DB calls

```
start      : graphConfig                    header NEWLINE statements EOF; setC4Type(header)
statements : items, the rules below
otherStatement : title | accDescription | accTitle value | accDescr value | multi-line value
diagramStatement : keyword attributes      one DB call per keyword
boundaryStatement: keyword attributes (NEWLINE)? '{' (NEWLINE)? diagramStatement+ '}'
attributes : (STR | STR_KEY STR_VALUE | empty)+
```

The statements are a list in which two statements are separated by one line break. The one exception is an other statement followed by a diagram statement on the same line, which the grammar allows (`other statement NEWLINE? diagram statement`). The only other statement that can end mid-line is the multi-line `accDescr { … }`, so `accDescr {x}Person(a,"A")` is valid. A boundary's body starts with a diagram statement; its `{` needs a line break before it or after it (P17), and its `}` may follow the last statement on the same line (`diagramStatements` takes no trailing line break).

The grammar calls the DB when it reduces a statement, in document order: a boundary calls its DB function before its body and pops the boundary parse stack after its `}`. The relation index keyword drops its first argument. `Boundary` passes its arguments unchanged; `Enterprise_Boundary`, `System_Boundary` and `Container_Boundary` splice their kind in as the third argument (JavaScript's `splice(2, 0, …)`, so the kind goes at the end when there are fewer than two arguments), which makes the positional third argument of `System_Boundary` the tags (P14, P15). `Node`, `Deployment_Node`, `Node_L` and `Node_R` are boundaries too: a deployment node has a body (P16).

The `direction` line (rule 1) reaches a DB function that the C4 DB does not export: `setDirection` is not in its default export (`c4Db.ts:778-813`). So the grammar's `start : direction` action throws, and every text with a direction line is rejected. The port rejects it at the token.

## 5. DB

| Call                                                | Stores                                                                                                                                                                                                                                                                                                     |
| --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `setC4Type`, `setTitle`                             | sanitised (`sanitize_text`); `accTitle` calls `setTitle` (P26)                                                                                                                                                                                                                                             |
| `setAccDescription`                                 | sanitised, `\n\s+` to `\n`; no trim for `accDescription`, trimmed for the other forms                                                                                                                                                                                                                      |
| `addPersonOrSystem`, `addContainer`, `addComponent` | An existing alias is reused and keeps the fields the new declaration does not set (P20). `label` is set; `descr` (and `techn` for containers and components) is `??=`, so a default `""` only when nothing is there; the sprite, tags and link slots set their field when the argument is given (P19, P20) |
| `addPersonOrSystemBoundary`, `addContainerBoundary` | `type` defaults to `system` or `container`; the tags and link slots are set when given; the boundary becomes the current one (P31)                                                                                                                                                                         |
| `addDeploymentNode`                                 | `type` defaults to `node`; `descr` is `??=`; the tags and link slots are set; **the sprite slot is read and never assigned** (`c4Db.ts:476`), so it is skipped here too                                                                                                                                    |
| `addRel`                                            | Keyed by `(from, to)`: an existing relation is reused. A relation without a label is dropped (P32). `techn` and `descr` are `??=` with `""`, and the sprite, tags and link slots set their fields. `wrap` is the configured flag (P21)                                                                     |
| `updateElStyle`                                     | On the shape with the alias, else on the boundary with it, else nothing (P25). The positional `techn` and every named text field become a **bare string**, which the model shows as absent                                                                                                                 |
| `updateRelStyle`                                    | The first relation with both endpoints. Positional offsets and named offsets in the offset slots are `parseInt`ed (P24 style); a named offset in a colour slot is stored as its raw text (`q12-rel-offset-nan`)                                                                                            |
| `updateLayoutConfig`                                | `parseInt` of each value; a value below one is ignored (P24). Both rows are stored when valid                                                                                                                                                                                                              |
| `popBoundaryParseStack`                             | Restores the parent and the boundary stack exactly as `c4Db.ts:486-491` does                                                                                                                                                                                                                               |

Named attributes (`$key="value"`) follow `assignAttributes` and the typed writes of the reference. In a declaration, a named value lands in the field whose name is its key, wrapped in `{ text }` when the key is one of `label`, `descr`, `techn` or `type`, whatever slot it arrived through (the spec `c4NamedAttributes.spec.ts` checks this for each slot). In a style statement the write is a raw assignment, so a text field becomes a bare string. A named key with no field of its own goes to the object as a string (`extra`), or as `{ text }` when it is a text key on a shape.

The accessibility description is the common DB's. The C4 `clear()` does not reset it (`c4Db.ts:721-734` and `commonDb.ts:11-12`), so in the reference a description given to one diagram is still there for the next diagram on the same page. The model is per parse: the exporter clears the common DB before each input, so every diagram is compared as if it were the only one.

## 6. Model

The model (`ferrigura-model::c4`) is the DB state after the parse, camelCase, with `version: 1`, as NOTES section 4 proposes, with these decisions:

- **A text field is a string, or absent.** The DB's `{ text }` is its string. A bare string (written by a style statement) and a field that was never set are both absent. The reference's `??=` keeps a bare string, so the port keeps a flag for `descr` and `techn` to tell them apart.
- **A label is `string | { key: value }`.** A named argument in the label slot is stored by the reference as the text (`c4Person.spec.ts`: `label.text` is `{ sprite: 'users' }`), so the model writes that object (`C4Text::Named`). Nothing else in the model is an object.
- **Offsets are strings.** The model holds `String(value)` of the DB value: a positional offset is the `parseInt` result (`"5"`, `"-10"`, `"NaN"`), a named one is its raw text. The renderer applies `parseInt` as the reference does (`svgDraw.ts:35-36`).
- **`tags` and `link` of a boundary may be `null`.** The global boundary has `null` for both; an absent field is left out.
- **`wrap` is a boolean.** `undefined` and `false` are the same for the renderer, so both are `false`.
- **`c4ShapeInRow` and `c4BoundaryInRow` are numbers.** They are `parseInt` results, so they are whole numbers or `NaN`, which the guard ignores.
- **Extra keys.** A named key with no field of its own is kept as a string, and as `{ text }` for a text key on a shape, so the DB state is complete.
- **Left out:** the layout fields, the parser's stack and current boundary, and the common DB's accessibility title, which C4 never sets (always `""`).

## 7. Quirks the port reproduces

Each quirk is covered by a case in `tools/port/cases.ts` (the probes of NOTES section 2.4) or by a spec test.

| Id  | Quirk                                                                                        | Where                                          |
| --- | -------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| P1  | A space before `(` is a lexical error                                                        | scanner, structure state                       |
| P2  | An unquoted argument runs to the next comma, over `)` and line breaks                        | scanner, attribute state                       |
| P3  | `Rel(a, b)` at the end of the input is an error                                              | scanner (raw end) and DB (no label)            |
| P4  | `System(a)` is an error, for the same greedy reason                                          | scanner                                        |
| P5  | `"A" )` followed by a line swallows the next line into the description                       | scanner, attribute state                       |
| P6  | `"A" , "desc"` makes the space a positional argument                                         | scanner, attribute state                       |
| P7  | An unquoted last argument is an error                                                        | grammar (`expected an attribute`)              |
| P8  | `direction` anywhere in a line is an error                                                   | scanner rule 1, grammar                        |
| P9  | `title` stops at `#` and `;`, which is a lexical error                                       | scanner rule 2                                 |
| P10 | `title` keeps its blanks, one character removed                                              | scanner rule 2                                 |
| P11 | Strings have no escapes                                                                      | scanner, string state                          |
| P12 | `$descr=""` is an error                                                                      | scanner, key-value states                      |
| P13 | A key's blanks are trimmed                                                                   | grammar (`attributes`)                         |
| P14 | `System_Boundary`, `Enterprise_Boundary`, `Container_Boundary` splice their kind             | grammar (`splice`)                             |
| P15 | `Boundary` takes its third argument as the type                                              | grammar                                        |
| P16 | `Node_L` takes its third argument as the type                                                | DB (`add_deployment_node`)                     |
| P17 | `{` on the same line or the next                                                             | grammar (`boundary_body`)                      |
| P18 | An empty boundary is an error                                                                | grammar (a diagram statement first)            |
| P19 | `System(a, "")` has an empty label                                                           | DB                                             |
| P20 | A re-declared alias keeps the fields it does not set, and takes the new kind                 | DB (`shape_position`, `??=`)                   |
| P21 | `Rel` and `BiRel` on the same pair merge into one relation of the last type                  | DB (`rel_position`)                            |
| P22 | `RelIndex` drops its first argument                                                          | grammar                                        |
| P23 | `Rel_Back` is `rel_b`                                                                        | scanner keyword table                          |
| P24 | `UpdateLayoutConfig` ignores values below one and reads with `parseInt`                      | DB (`update_layout_config`)                    |
| P25 | `UpdateElementStyle` of an unknown alias is ignored                                          | DB (`update_el_style`)                         |
| P26 | `accTitle` sets the title                                                                    | grammar (`other_statement`)                    |
| P27 | An `accDescr { … }` block keeps its inner indentation, and its ends are trimmed              | grammar                                        |
| P28 | The last `title` wins                                                                        | DB (`set_title`)                               |
| P29 | A `%%` comment after a statement is a comment                                                | scanner, comment rule                          |
| P30 | An element can be named with a keyword (`System(System, "System")`)                          | scanner (no keyword rule in a structure state) |
| P31 | A boundary re-declared inside itself, or `global`, is accepted and gets itself as its parent | DB (`boundary_position`, `open_boundary`)      |
| P32 | `Rel(a, "b")` is accepted and makes no relation                                              | DB (`add_rel`, no label)                       |

Also reproduced: a deployment node's sprite argument is not stored (`c4Db.ts:476`), a named key in an update statement is a raw assignment (bare text fields), and the `??=` defaults keep bare values.

## 8. Not ported, and open questions

- **Named arguments in the alias, the endpoints, or the `wrap`/`typeC4Shape`/`parentBoundary` fields.** The reference stores a JavaScript object in these, or replaces a structural field, and the model holds strings. The port reports a parse error for them (`is not ported`). No collected input uses them: the only named argument in an identity slot is a label, which is supported. An explicit named exception would be needed if such an input were collected.
- **`$type` on a relationship declaration** replaces `type` with an object: not ported for the same reason.
- **The `direction` error.** A direction line is an error in both (a `TypeError` in the reference), so the agreement is by rejection; the message differs.
- **Common accessibility state across diagrams (section 5).** Reproduced per diagram, not across diagrams. The renderer task must decide whether a page renders each C4 diagram in isolation (the port's behaviour) or inherits the description (the reference's).
- **Offsets for the renderer.** A positional `NaN` offset is falsy in the reference, which draws offset `0`, and the model's `"NaN"` text is truthy. The renderer must treat the positional NaN as `0` (or the model must say which it is). A named `$offsetX="abc"` in the text colour slot is truthy in the reference, which draws `x="NaN"`.
- **Infinite layout values.** `parseInt` of a 400-digit number gives `Infinity`, which the model cannot write as a JSON number. No input has one.

## 9. Verification

- `pnpm port:diff c4`: 176 inputs (32 fixtures, 30 C4 literals of the specs, 6 C4 blocks of the docs, and 108 explicit cases, of which 34 are the named-attribute texts that the spec builds from its tables), 153 accepted by the reference, 0 differences, 0 named exceptions.
- `pnpm port:diff config`: the C4 section is compared (126 of 126 accepted by the reference in its group).
- The specs: 169 cases in eight Rust files, listed in `tools/port/status.ts`. The label helper's two cases stay ignored in `c4_label_spec.rs`, because they test the renderer's label layout.
- The parser's unit tests (scanner, DB helpers) and the model's tests run with the crate.
