# M3 flowchart parser: Stage A plan

Status: Stage A, inventory and model proposal. No Rust code yet. Stage B waits for the coordinating session's answer to the open questions in section 13.

Reference commit: the JS reference as it is on `sebastian/blissful-cori-9818ff` (`b1c4caa43`). Line references below are to the files of that tree.

## 1. Scope and sources

Read in full: `parser/flow.jison` (grammar and lexer), `parser/flowParser.ts`, `flowDb.ts`, `flowDetector-v2.ts`, `flowDiagram.ts`, `types.ts`, `swimlanes/swimlanesDiagram.ts` (reuses the flowchart parser, DB and renderer), `diagram-api/comments.ts`, `utils.ts` (`getEdgeId`, `formatUrl`), `diagrams/common/commonDb.ts` (accessibility setters), `rendering-util/rendering-elements/shapes.ts` (shape registry and `isValidShape`).

Read in part, by test titles and by the inputs and expectations of the tests: the 15 parser specs (`flow-edges` and `flow-singlenode` mostly by title and inputs), `flowDb.spec.ts`, `flowDiagram.spec.ts`, `flowRenderer-*.spec.ts`, `flowDb-elk-duplicate-subgraph.spec.ts`.

Inputs counted: `e2e/diagrams/flowchart/` has 352 `.mmd` files, all of which start like a flowchart after front matter. Other e2e folders contain 55 more `.mmd` files with a `graph` or `flowchart` line; the collector has to decide by the detector, not by folder. `packages/mermaid/src/docs/syntax/flowchart.md` has 114 mermaid blocks; 14 docs files in total contain a flowchart block.

Verified by running the Jison grammar (not the DB): see section 3.4. The probe scripts are in the session scratchpad and are not committed.

Not in M3: rendering (`getData`, shape mapping, layout, `flowRenderer-*`), ELK, configuration beyond the parser options in section 9. Those belong to M4 and M5.

## 2. Grammar inventory

Jison productions, grouped. The DB call column says which DB method each action runs.

| Group         | Productions (Jison names)                                                                                                                                                                                          | DB calls                                                  |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| Header        | `graphConfig`: `GRAPH NODIR`, `GRAPH DIR FirstStmtSeparator`                                                                                                                                                       | `setDirection` (`NODIR` gives `TB`)                       |
| Document      | `document`, `line`: statement, `SEMI`, `NEWLINE`, `SPACE`, `EOF`                                                                                                                                                   | none (empty lines are dropped)                            |
| Statement     | `vertexStatement separator`, `styleStatement`, `linkStyleStatement`, `classDefStatement`, `classStatement`, `clickStatement`, three `subgraph` forms, `direction`, `acc_title`, `acc_descr`, `acc_descr_multiline` | per production, see below                                 |
| Vertex chain  | `vertexStatement: vertexStatement link node [shapeData] \| node spaceList \| node [shapeData]`                                                                                                                     | `addLink`, `addVertex` (with shape data)                  |
| Node group    | `node: styledVertex \| node [shapeData] spaceList AMP spaceList styledVertex`                                                                                                                                      | `addVertex` (with shape data)                             |
| Styled vertex | `styledVertex: vertex \| vertex STYLE_SEPARATOR idString`                                                                                                                                                          | `setClass`                                                |
| Vertex shapes | 16 shape forms plus bare `idString` (table 3)                                                                                                                                                                      | `addVertex(id, text, shape)`                              |
| Link          | `link: linkStatement arrowText \| linkStatement TESTSTR SPACE \| linkStatement arrowText SPACE \| linkStatement \| START_LINK edgeText LINK \| LINK_ID START_LINK edgeText LINK`                                   | `destructLink`                                            |
| Link token    | `linkStatement: LINK \| LINK_ID LINK`                                                                                                                                                                              | `destructLink`                                            |
| Text          | `text`, `textNoTags`, `edgeText`: text tokens, `STR`, `MD_STR`                                                                                                                                                     | none (values)                                             |
| Subgraph      | `subgraph SPACE textNoTags SQS text SQE separator document end`, `subgraph SPACE textNoTags separator document end`, `subgraph separator document end`                                                             | `addSubGraph` at `end`                                    |
| Direction     | `direction: direction_tb \| _bt \| _rl \| _lr \| _td`                                                                                                                                                              | none (returns `{stmt:'dir'}`, used only inside subgraphs) |
| Accessibility | `acc_title acc_title_value`, `acc_descr acc_descr_value`, `acc_descr_multiline_value`                                                                                                                              | `setAccTitle`, `setAccDescription`                        |
| classDef      | `CLASSDEF SPACE idString SPACE stylesOpt`                                                                                                                                                                          | `addClass`                                                |
| class         | `CLASS SPACE idString SPACE idString`                                                                                                                                                                              | `setClass`                                                |
| click         | 16 variants of `CLICK` with `CALLBACKNAME`, `CALLBACKARGS`, `HREF`, `STR`, `alphaNum`, `LINK_TARGET`                                                                                                               | `setClickEvent`, `setLink`, `setTooltip`                  |
| style         | `STYLE SPACE idString SPACE stylesOpt`                                                                                                                                                                             | `addVertex(id, undefined, undefined, styles)`             |
| linkStyle     | 6 variants: `DEFAULT` or `numList`, with `interpolate` or `stylesOpt`, or `interpolate` alone                                                                                                                      | `updateLink`, `updateLinkInterpolate`                     |
| Helper lists  | `numList`, `stylesOpt`, `style`, `styleComponent`, `idString`, `alphaNum`, `textToken`, `textNoTagsToken`, `edgeTextToken`, `keywords`                                                                             | none                                                      |
| Shape data    | `shapeData: shapeData SHAPE_DATA \| SHAPE_DATA`                                                                                                                                                                    | `addVertex(..., metadata)`                                |
| Separator     | `separator: NEWLINE \| SEMI \| EOF`                                                                                                                                                                                | none                                                      |

Dead grammar, to leave out of the Rust code and keep as a note in the Rust grammar doc:

- `TESTSTR` appears in `link` but no lexer rule produces it.
- `PCT` and `UNIT` appear in `styleComponent` but no lexer rule produces them. `%` is lexed as part of `NODE_STRING`.
- `vertex` and `shapeDataEndBracket` are declared as lexer states but no rule enters them. `href` is declared, but `"href"` returns `HREF` in `INITIAL` without a state change.
- The commented-out `GRAPH SPACE TAGEND` forms in `graphConfig`.
- The `double_arrow` branch in `destructLink` is not reached by any token sequence I could derive from the lexer rules.

### 2.1 Token-level details the parser depends on

- An id is the concatenation of `idStringToken`s, which include `NUM`, `NODE_STRING`, `DOWN` (`v`), `MINUS`, `DEFAULT`, `COMMA`, `COLON`, `AMP`, `BRKT` (`#`), `MULT`, `UNICODE_TEXT`. So `A&B` is one id (verified), `A,B` is one id that `class` and `click` split at commas, `1test` and `va` are ids.
- A text is the concatenation of the raw source between the delimiters, so the Rust side can slice the source instead of concatenating tokens. Strings (`STR`, `MD_STR`) replace the text entirely; mixing them with text tokens is an error ("got 'STR'").
- A chain's node list is built with `$node.concat($vertexStatement.nodes)`, so the later nodes of a chain come first. `A --> B --> C` gives `[C, B, A]`; `A & B --> C` gives `[C, A, B]`; `A & B` alone gives `[A, B]` (verified with the probe).

## 3. Lexer

### 3.1 Start conditions

Exclusive states (`%x`): `string`, `md_string`, `acc_title`, `acc_descr`, `acc_descr_multiline`, `dir`, `text`, `ellipseText`, `trapText`, `edgeText`, `thickEdgeText`, `dottedEdgeText`, `click`, `callbackname`, `callbackargs`, `shapeData`, `shapeDataStr`. Declared but never entered: `vertex`, `href`, `shapeDataEndBracket`.

Rules without a state prefix match in `INITIAL` only. Rules with `<*>` match in every state. The important `<*>` rules are the quote rules (`"` pushes `string`, `"` followed by a backtick pushes `md_string`), the brackets `(`, `[`, `{`, `|`, `(-`, `[[`, `(["`, `[(`, `(((`, `[/`, `[\`, which push `text`, `ellipseText` or `trapText`, and the invisible link `~~~`.

Inside `text`, only `TEXT` (`[^\[\]\(\)\{\}\|\"]+`) and the closing tokens match, so a nested bracket is a `PS`/`SQS` error. Inside `edgeText` the rule `[^-]` comes before the `<*>` brackets, so `(`, `)` and `[` are plain edge text (verified: `A-- a (b) -->B` gives `EDGE_TEXT` for each character), while the quote rules come before it, so `"q"` in an edge is `STR`.

### 3.2 Rule order and the implicit word boundary

Jison takes the first rule in definition order that matches, not the longest. The discriminating case is `A-->12ab`: `NUM` is defined before `NODE_STRING`, so the lexer gives `NUM("12")` and then `NODE_STRING("ab")`, where a longest-match lexer gives one `NODE_STRING("12ab")` (verified). Two consequences, both verified:

1. Literal keyword rules get an implicit word boundary (`"style"` behaves as `style\b`). `styles A fill:#f00`, `styleA`, `classDefX`, `endpoint`, `vertex` all lex as `NODE_STRING` (verified). `"end"` has an explicit `\b`.
2. The rule `.*direction\s+TB[^\n]*` matches from the current position, including earlier characters of the line. `graph TD\nA-->B direction TB\nC-->D` lexes the second line as `direction_tb("A-->B direction TB")`. The edge `A-->B` is lost and the parse succeeds. `graph TD;A-->B direction TB` on one line is a parse error, because the token swallows the `;` that the header needs. Both behaviours are in the reference and must be kept.

logos chooses the longest match and has no start conditions, lookahead or first-match order. Several rules here need lookahead (`[^\s\"]+\@(?=[^\{\"])`, `\-(?!\-)+`, `\.(?!-)`, `\=(?!=)`, `\-(?=[^\>\-\.])`) and the states change which rules are live. See open question 1.

### 3.3 Lexer rules that the specs pin

Grouped by the test they come from:

- **Arrows with x and o as a prefix of the target.** `A--xB` is `arrow_cross` to `B`; `A--xv(my text)`, `A--xcsv(...)` and `A--xava(...)` give node ids `v`, `csv`, `ava`, with the `x` consumed by the link (flow-text, "v in node ids").
- **Edge text and its terminator.** `A-- text --xB`, `A-. test text with == .->B`, `A== test text with - ==>B`. The terminator is `\s*[xo<]?\-\-+[-xo>]\s*` in `edgeText`; a `-` that is not followed by `-` stays text.
- **Slash, keywords and quotes in edge text.** `A -- text with / should work --x B`, `A-- text including graph space and v --xB[blav]`, `V-- "test string()" -->a[v]`.
- **Minus before `>`.** `A_node-->odd->Vertex Text];` gives the id `odd-` and an `odd` shape (`\-(?=[^\>\-\.])`).
- **Lean and inverse trapezoid bodies.** `[/…/]` lean_right, `[\…\]` lean_left, `[/…\]` trapezoid, `[\…/]` inv_trapezoid; slashes and backslashes in the body are text.
- **Unicode and punctuation in text.** `C{Chimpansen hoppar åäö-ÅÄÖ}`, `A(,.?!+-*)-->|,.?!+-*|C`, `A(-…-)` ellipse, `A(?)-->|?|C`, `Начало`, `éèêàçô`. Unicode letters use the explicit range table of the grammar (`UNICODE_TEXT`), not Rust's `char::is_alphabetic`.
- **Keywords inside every shape.** `A_graph_node-->B(This node has a graph as text)` for each shape and for `[|borders:lt|…]`. Keywords are `TEXT` inside the bracket states.
- **Errors pinned by exact Jison text.** `A[This is a () in text]` gives "got 'PS'"; `A(this node has "string" and text)-->|…|C` gives "got 'STR'"; `A[This is a \"()\" in text]` gives "got 'STR'"; `A["This is a "()" in text"]` gives "Expecting 'SQE'"; `node[hello ) world] --> works` gives "got 'PE'"; `X(- My Text (` must fail. The specs assert these texts (flow-text, last block). Section 10 describes how to port them.
- **Ellipse nesting.** The "should not freeze" test is about an unclosed `(-`, which must fail, not hang.
- **Long and repeated dashes.** `A -----…- B` with `length` 1 to 3 (edge length = dashes minus 2, capped at 10 in `addSingleLink`).
- **Edge ids.** `A e1@-->B` is `LINK_ID` (`e1@`) then `LINK`. `e1@{ animate: true }` is an `@{` shape-data statement on an edge id.
- **Shape data.** `@{` pushes `shapeData`. Unquoted text is `SHAPE_DATA` up to `}` or `"`. Quoted text (`shapeDataStr`) is `SHAPE_DATA` with every `\n\s*` replaced by `<br/>`; so `label: "a` newline `b"` becomes `a<br/>b`. A `^` in unquoted shape data is a lexical error (verified: `A@{ shape: a^b }`), because the class `[^}^"]` leaves it out and no other rule matches it in that state.
- **Comments.** `%%` mid-line is not a comment (`%` is a `NODE_STRING` character; verified). Line comments are removed before lexing (section 4).
- **Newlines.** `(\r?\n)+` is one `NEWLINE` token (verified for `\n\n`). `NODIR` needs a newline after the header.

### 3.4 Probe results (Jison grammar, DB stubbed)

These were produced by the Jison parser generated from `flow.jison` in the session scratchpad, with `yy` stubbed to record calls. They show the lexer and reduction order, not the DB.

- Tokens: `graph TD;styleA-->B;` gives `NODE_STRING(styleA)`. `graph TD;A--xB;` gives `LINK(--x)`. `graph TD;A&B-->C;` gives `NODE_STRING(A&B)`. `graph TD;A:::cls-->B` gives `STYLE_SEPARATOR`.
- Reduction order for `subgraph X / A --> B --> C / end`: `addVertex(A)`, `addVertex(B)`, `addLink(A,B)`, `addVertex(C)`, `addLink(B,C)`, `addSubGraph([C,B,A])`. A vertex is created when its node reduces, which happens after the next token is read; a link is added when the following node has been read. The Rust parser must make the DB calls in this order, because edge ids and `linkStyle` indices depend on it.
- Accepted: `graph TD\nA\n`, `graph TD;A;B`, `graph TD;click A "x"`, `graph LR\nA-->B\nlinkStyle 5 stroke:red` (the DB rejects the index later), `graph TD;A[(x)]`, `graph TD;A[|borders:lt|x]`, `graph TD;A-- x --o B`.
- Rejected: `graph TD;A-->graph;` (GRAPH is not a node token), `graph TD;A-->style;`, `graph TD;A-->B direction TB`, `graph TD\nA[This is a () in text]`.

## 4. Preprocessing

Applied in this order before lexing (`preprocess.ts` plus the parser wrapper):

1. Front matter and directives are removed (shared with M1, `preprocess.rs`). Directives `%%{…}%%` are kept for M2.
2. `cleanupComments`: `text.replace(/^\s*%%(?!{)[^\n]+\n?/gm, '').trimStart()`. A line-based regex, not a token. It needs at least one character after `%%`, so a bare `%%` line stays and is lexed as `NODE_STRING`. The M1 approach (comments as skipped tokens) gives different results here, so the flowchart needs this regex-equivalent pass.
3. `flowParser.ts`: `src.replace(/}\s*\n/g, '}\n')`. This also rewrites text inside quoted shape data. Keep it exactly.
4. CRLF: `preprocess.rs` already normalises line breaks as the canonical model says.

## 5. Lexer and parser design (for Stage B)

Proposal: a hand-written scanner with an explicit state stack, rule table in Jison order, first match wins, implicit word boundary on keyword literals. The parser is recursive descent over the token stream, and it performs DB calls in the reduction order from section 3.4. See open question 1.

The shared base keeps `Source`, `ParseError`, the cursor, string and escape decoding, and the title and accessibility value rules. The pie and info lexers stay logos-based.

## 6. Shape data (`@{ … }`)

The DB reads the text with `js-yaml` (`JSON_SCHEMA`). For a single line the text is wrapped as `{\n…\n}`; a multi-line text is used as it is with a trailing newline. The Rust side needs a YAML subset that covers:

- flow mappings `{ k: v, k: v }` and block mappings with indentation;
- plain, single-quoted and double-quoted scalars with the JSON-schema typing (numbers, booleans, null);
- literal block scalars `|` (the spec expects `This is a\nmultiline string\n` for `label: |`).

Value typing matters: `w` and `h` are `Number(doc.w)`; `label` as a number would be a number in the reference. The Rust side stores the label as the YAML text and decides the number case when it sees it (question 2).

Keys read by the DB: `shape` (validated), `label`, `labelType`, `icon`, `form`, `pos`, `img`, `constraint`, `w`, `h`. For subgraphs and edges: `view` (subgraph), `animate`, `animation`, `curve` (edge), and any other key for subgraph metadata (`algorithm: elk.box` is read by the layout).

Shape validation (`addVertex`): `shape` must equal its lowercase form, must not contain `_`, and must be a key of the `shapes` registry (documented and undocumented shapes, short names, aliases, internal aliases). The list is data and must be generated or checked against `shapes.ts`. Quirk: the registry is a plain object built with `Object.fromEntries`, so `'constructor' in shapes` is true and `constructor` is accepted as a shape (open question 5).

## 7. DB setters: behaviour per method

| Method                                                             | What it does to the value                                                                                                                                                                                                                                                                                                                                                                                                     | Defaults and ids                                                                                                                                                                     | Duplicates and order                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `addVertex(id, text, shape, style, classes, dir, props, metadata)` | Empty id returns. Shape data first parsed as YAML. A subgraph id with shape data merges the metadata into the subgraph and returns (no vertex). An existing edge id with shape data sets `animate`, `animation`, `curve` (interpolate) and returns. Otherwise the vertex is created with `labelType 'text'`, text = id. `vertexCounter` increments on every call. Text: trim, sanitise, strip one pair of surrounding quotes. | `labelType` 'text'; `text` = id when no text is given; `styles`, `classes` []; `domId` (left out).                                                                                   | Existing vertex is updated: text replaced when given, `type` replaced when given, styles and classes appended, props merged. `dir` is never passed by the grammar, so vertex `dir` is dead.                                                             |
| `addSingleLink(start, end, type, id)`                              | Text: trim, sanitise, strip quotes, `labelType` (default 'markdown' via `sanitizeNodeLabelType` only when the label type is not text/string/markdown). `length` capped at 10.                                                                                                                                                                                                                                                 | Id: the user id when it is not used yet; otherwise `L_<start>_<end>_<n>` where `n = 0` for the first edge of the pair, else count + 1. `interpolate` = the default at creation time. | Edge limit: when `edges.length >= maxEdges` (default 500) it throws, after the id is computed.                                                                                                                                                          |
| `addLink(starts, ends, data)`                                      | Strips `@` from the data id.                                                                                                                                                                                                                                                                                                                                                                                                  | The user id goes only to the pair (last start, first end).                                                                                                                           | Pairs in order: outer start, inner end.                                                                                                                                                                                                                 |
| `destructLink(link, startLink)`                                    | Pure: arrow type, stroke, length from the tokens. Mismatched heads give `INVALID` for both type and stroke, without an error.                                                                                                                                                                                                                                                                                                 |                                                                                                                                                                                      |                                                                                                                                                                                                                                                         |
| `setDirection(dir)`                                                | Trim; substring tests: `<` gives `RL`, `^` gives `BT`, `>` gives `LR`, `v` gives `TB`; `TD` gives `TB`. `BR` stays `BR`.                                                                                                                                                                                                                                                                                                      | Default `TB` via `NODIR`.                                                                                                                                                            | Last one wins.                                                                                                                                                                                                                                          |
| `addSubGraph(id, list, title)`                                     | Flattens the list; `dir` statements are removed and the last one is the direction. Id: the text trimmed, or `subGraph<n>` when there is none (see below). Title: sanitise, then trim.                                                                                                                                                                                                                                         | `subCount` increments on every call, named or not; the generated id uses it.                                                                                                         | Members already in another subgraph are removed (`makeUniq`), and the subgraph's own id is removed from its nodes. A repeated id merges its members into the first declaration, which keeps its title and direction. Subgraph list is in closing order. |
| `setClass(ids, cls)`                                               | Split ids at commas. Applies to a vertex, to the first edge with that id, and to a subgraph with that id. Missing ids are ignored.                                                                                                                                                                                                                                                                                            |                                                                                                                                                                                      |                                                                                                                                                                                                                                                         |
| `addClass(ids, styles)`                                            | Joins, replaces `\,` with a placeholder, `,` with `;`, back; splits at `;`. Each style containing `color` also goes to `textStyles` with `fill` replaced by `bgFill`.                                                                                                                                                                                                                                                         | Class created on first use.                                                                                                                                                          | Styles appended.                                                                                                                                                                                                                                        |
| `setLink(ids, url, target)`                                        | URL trimmed; empty gives no link. Sanitised by `securityLevel` at render time. Applies only to existing vertices, then `setClass(ids, 'clickable')`.                                                                                                                                                                                                                                                                          |                                                                                                                                                                                      | Link before the vertex is declared is dropped (verified in the code; test in Stage B).                                                                                                                                                                  |
| `setClickEvent(ids, name, args)`                                   | Callback only under `securityLevel: loose`, and only for vertices. Always `setClass(ids, 'clickable')`.                                                                                                                                                                                                                                                                                                                       |                                                                                                                                                                                      | Left out except the class (section 9).                                                                                                                                                                                                                  |
| `setTooltip(ids, tip)`                                             | Sanitised. Stored per id, for vertices, subgraphs, edges and absent ids alike.                                                                                                                                                                                                                                                                                                                                                |                                                                                                                                                                                      | Map insertion order.                                                                                                                                                                                                                                    |
| `updateLink(positions, styles)`                                    | Index out of range throws the message from the spec. Default sets `defaultStyle`; a numbered edge gets `style` and, when the style is non-empty and has no `fill…`, `fill:none` appended.                                                                                                                                                                                                                                     |                                                                                                                                                                                      | Same array shared by several edges, equivalent to copying values.                                                                                                                                                                                       |
| `updateLinkInterpolate(positions, text)`                           | Default sets `defaultInterpolate`; a numbered edge sets its `interpolate`. No bounds check: a missing index is a JS TypeError, which the Rust side reports as an error.                                                                                                                                                                                                                                                       |                                                                                                                                                                                      |                                                                                                                                                                                                                                                         |
| `setAccTitle`, `setAccDescription`                                 | Flowchart-specific: title is the grammar's trimmed value with leading whitespace removed; block and single-line description is the grammar's trimmed value with `\n\s+` replaced by `\n`. No blank-line collapsing, which differs from the pie and info rules (section 8.3).                                                                                                                                                  |                                                                                                                                                                                      |                                                                                                                                                                                                                                                         |
| `addClass`, `setClass`, `getData`                                  | `getData` is rendering (M5), out of scope here.                                                                                                                                                                                                                                                                                                                                                                               |                                                                                                                                                                                      |                                                                                                                                                                                                                                                         |

Callback, `getEdgeId` for rendering (`counter: index`), `getTooltip`, `getData`, `lookUpDomId`, `indexNodes`, `getDepthFirstPos` and gen-1 branches are rendering or dead, see section 9.

Errors and their texts. The reference's DB messages are part of the user interface, and the specs check them (`No such shape: …`, `The index 1 for linkStyle is out of bounds …`, edge limit). The Rust side keeps those texts exactly. Jison's own messages ("got 'PS'") are not part of that promise: the specs asserting them become "fails, and where" (ADR 0002).

## 8. Canonical model proposal

### 8.1 Shape

```json
{
  "version": 1,
  "title": "",
  "accTitle": "",
  "accDescr": "",
  "direction": "TB",
  "vertices": [
    {
      "id": "A",
      "text": "A",
      "labelType": "text",
      "shape": null,
      "styles": [],
      "classes": [],
      "link": null,
      "linkTarget": null,
      "props": {},
      "icon": null,
      "form": null,
      "pos": null,
      "img": null,
      "constraint": null,
      "assetWidth": null,
      "assetHeight": null
    }
  ],
  "edges": [
    {
      "id": "L_A_B_0",
      "isUserDefinedId": false,
      "start": "A",
      "end": "B",
      "type": "arrow_point",
      "stroke": "normal",
      "length": 1,
      "text": "",
      "labelType": "text",
      "classes": [],
      "style": null,
      "interpolate": null,
      "animate": null,
      "animation": null
    }
  ],
  "subgraphs": [
    {
      "id": "A",
      "title": "",
      "labelType": "text",
      "nodes": [],
      "dir": null,
      "classes": [],
      "metadata": {}
    }
  ],
  "classes": [{ "id": "cls", "styles": [] }],
  "tooltips": [{ "id": "A", "tooltip": "" }],
  "defaultStyle": null,
  "defaultInterpolate": null
}
```

Field names follow the DB's names in camelCase, so that the exporter reads one name per field. `metadata` is an object with JSON values; `props` is an object.

Conventions applied (canonical-model.md):

- Strings that the DB always sets are present (`text`, `labelType`, `title`, `accTitle`, `accDescr`, `direction`, `id`, `start`, `end`, `type`, `stroke`, `text` of an edge). Values the DB leaves `undefined` are `null`: `shape`, `link`, `linkTarget`, `style`, `interpolate`, `animate`, `animation`, `dir`, `length` of an `INVALID` edge, and the shape-data fields. This keeps `undefined` apart from `''`.
- `labelType` is `text`, `string` or `markdown`. Label kind is a field; markdown is not parsed in the model. Markdown is parsed at render time with Ferromark.
- Text: raw. Only the grammar's value rules apply (quotes, escapes, trim) plus the DB's own trim and quote-stripping, in the DB order. No sanitisation (see 9.2).
- Order: `vertices` in the order the DB creates them (first call that creates); `edges` in creation order; `subgraphs` in closing order, with a repeated id merged into its first position; `classes` and `tooltips` in insertion order.
- Node lists keep the reversed chain order (section 2.1). It is part of the output and must not be sorted.
- Edge `type` is the DB's arrow type string (`arrow_open`, `arrow_point`, `arrow_cross`, `arrow_circle`, `double_*`, or `INVALID`). The DB encodes both ends in it; keeping the string avoids a second encoding.
- Edge `id` is always present: the generated id or the user id. Section 7 gives the rules. Edge ids are referred to by `class`, `addVertex` with shape data and `linkStyle`, so they belong to the model.

### 8.2 Left in the model, although only rendering reads them

These are input content, not render output. The model keeps them so the Rust side does not decide what the renderer will need later:

- shape-data fields `icon`, `form`, `pos`, `img`, `constraint`, `assetWidth`, `assetHeight` (from `w` and `h`);
- `props` from `[|key:value|text]`, merged across statements; the renderer reads them only in commented-out code, but the grammar accepts them, so the value is part of the input;
- subgraph `metadata` with every key, including those the layout reads (`algorithm`, `view`);
- `defaultStyle` and `defaultInterpolate`, which the renderer applies to every edge at render time;
- edge `animate`, `animation`, `interpolate` (the value at creation time, plus later changes).

### 8.3 Header texts: flowchart rules differ from pie and info

`canonical-model.md` section 5 (block `accDescr`) describes the langium converter of pie and info: it collapses blanks and line breaks. The flowchart does not do that. Its block description is the grammar's `trim()` plus the DB's `\n\s+` → `\n`. `accTitle` is the grammar's `trim()` plus `^\s+`. The canonical model keeps per-diagram value rules, so this is a flowchart section in `canonical-model.md` with its own rules. Stage B adds the section once the first model lands.

The title: the flowchart grammar has no `title` rule (checked: `title` appears only in `accTitle`), so the title is the front matter title, through `Diagram.fromText`. No grammar title, no precedence question. A `title:` line in a flowchart body is node syntax.

## 9. What is left out, and why

| Left out                                                                                                                                                                                                                                                                            | Argument                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Click callbacks (`call` name and args, `haveCallback`)                                                                                                                                                                                                                              | Callbacks are JS functions that run in the browser. `securityLevel: loose` decides whether they exist at all, so the result depends on config. Decision 0001 removes JavaScript callbacks. The grammar still accepts the syntax, so existing diagrams parse, and the `clickable` class (set by the reference in the same call) stays in the model. Specs that spy on `setClickEvent` become outcome tests (classes, link, tooltip). |
| DOM ids (`domId`, `vertexCounter`, `diagramId`, `lookUpDomId`)                                                                                                                                                                                                                      | Rendering. `lookUpDomId` is used only in gen-1 branches and callbacks. gen-1 is never set in this fork (`setGen` is called with `gen-2` only), so those branches are dead.                                                                                                                                                                                                                                                          |
| Sanitised text, sanitised tooltip, DOMPurify, `sanitizeUrl` (text: ported as `sanitize.rs`; tooltip and link not)                                                                                                                                                                   | Need `securityLevel` and a DOM. Canonical rule (`canonical-model.md`).                                                                                                                                                                                                                                                                                                                                                              |
| `formatUrl` sanitising                                                                                                                                                                                                                                                              | Only the trim and "empty means no link" are kept, because they do not depend on config.                                                                                                                                                                                                                                                                                                                                             |
| `getData()` and everything it builds: node and edge mapping, `squareRect`, collapsed subgraphs, colour slots (`colorIndex`), `getEdgeId` with render index, `arrowTypeStart/End`, `labelpos`, `thickness`, `pattern`, `cssClasses`, edge curve fallback to `config.flowchart.curve` | Rendering (M5). Keeps only model fields. The spec's `getData()` expectations are the M5 part of the port (section 10).                                                                                                                                                                                                                                                                                                              |
| `textStyles` of classes                                                                                                                                                                                                                                                             | Derived from `styles` by a pure function. The renderer can compute it.                                                                                                                                                                                                                                                                                                                                                              |
| `inheritDir` and `config.direction` in `addSubGraph`                                                                                                                                                                                                                                | Configuration. The model keeps only the explicit `dir`. The inheritance is a rendering step, or M2 passes it as an option.                                                                                                                                                                                                                                                                                                          |
| `maxEdges`                                                                                                                                                                                                                                                                          | Passed as a parser option, default 500. It is a limit, not part of the diagram.                                                                                                                                                                                                                                                                                                                                                     |
| `htmlLabels`, `padding`, `minNodeWidth`, `wrappingWidth`, `look`, `layout`, `arrowMarkerAbsolute`                                                                                                                                                                                   | Configuration and rendering.                                                                                                                                                                                                                                                                                                                                                                                                        |
| `defaultWidth`, `imageAspectRatio` on vertices                                                                                                                                                                                                                                      | Set only by rendering shapes.                                                                                                                                                                                                                                                                                                                                                                                                       |
| `vertex.dir`                                                                                                                                                                                                                                                                        | Dead: no grammar path passes `dir` to `addVertex`.                                                                                                                                                                                                                                                                                                                                                                                  |
| `indexNodes`, `getDepthFirstPos`, `posCrossRef`, `secCount`                                                                                                                                                                                                                         | Dead in the flowchart: no caller outside `flowDb.ts` (agentflow has its own copy).                                                                                                                                                                                                                                                                                                                                                  |
| gen-1 branches (`setGen`, `version`)                                                                                                                                                                                                                                                | Dead (section above).                                                                                                                                                                                                                                                                                                                                                                                                               |
| Warnings (`log.warn` for a style on an unknown node)                                                                                                                                                                                                                                | Not part of the input's meaning. The parse result carries a `warnings` list, which the specs check. Not in the model.                                                                                                                                                                                                                                                                                                               |
| `firstGraphFlag`                                                                                                                                                                                                                                                                    | Parser state: the first graph keyword starts the header; in the Rust parser the header is always the first token.                                                                                                                                                                                                                                                                                                                   |
| `subCount` as a value                                                                                                                                                                                                                                                               | Its effect is in the generated ids, which are in the model.                                                                                                                                                                                                                                                                                                                                                                         |

## 10. Specs

### 10.1 Mapping

| Spec file                                                                                                                             | `it` blocks       | Target                                                                                                                                                                                                                                       | Notes                                                                                                                      |
| ------------------------------------------------------------------------------------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `parser/flow-singlenode.spec.js`                                                                                                      | 31                | M3                                                                                                                                                                                                                                           | Keywords and special chars in ids.                                                                                         |
| `parser/flow-text.spec.js`                                                                                                            | 59 (+2 todo)      | M3                                                                                                                                                                                                                                           | Largest; includes the Jison error texts (10.3).                                                                            |
| `parser/flow-edges.spec.js`                                                                                                           | 29                | M3                                                                                                                                                                                                                                           | Loops over 18 regular and 9 double-ended edge kinds and 3 lengths, several per kind; more cases than blocks.               |
| `parser/flow-arrows.spec.js`                                                                                                          | 14                | M3                                                                                                                                                                                                                                           | Direction in the header and double arrows.                                                                                 |
| `parser/flow-vertice-chaining.spec.js`                                                                                                | 7                 | M3                                                                                                                                                                                                                                           | Chains and `&` groups; pins the node order.                                                                                |
| `parser/flow-lines.spec.js`                                                                                                           | 12                | M3 for syntax; the `linkStyle` interpolate effects are model tests.                                                                                                                                                                          |                                                                                                                            |
| `parser/flow-style.spec.js`                                                                                                           | 25                | M3                                                                                                                                                                                                                                           | `classDef`, `class`, `style`, `:::`; one warning test (10.2).                                                              |
| `parser/flow-interactions.spec.js`                                                                                                    | 13                | M3                                                                                                                                                                                                                                           | 13 spy-based tests become outcome tests (10.2).                                                                            |
| `parser/flow-md-string.spec.js`                                                                                                       | 2                 | M3                                                                                                                                                                                                                                           | Markdown labels, including subgraph titles.                                                                                |
| `parser/flow-comments.spec.js`                                                                                                        | 9                 | M3                                                                                                                                                                                                                                           | `cleanupComments` is ported as a preprocessing step.                                                                       |
| `parser/flow.spec.js`                                                                                                                 | 22                | M3                                                                                                                                                                                                                                           | Keywords, unsafe ids (`__proto__`, `constructor`), accessibility.                                                          |
| `parser/flow-direction.spec.js`                                                                                                       | 4                 | M3                                                                                                                                                                                                                                           | Subgraph direction.                                                                                                        |
| `parser/subgraph.spec.js`                                                                                                             | 23 (+2 skip/todo) | M3                                                                                                                                                                                                                                           | One test uses `getData` for the collapsed subgraph (`view: collapsed`): the parse-level part is M3, the render part is M5. |
| `parser/flow-node-data.spec.js`                                                                                                       | 30 (+2 skip/todo) | M3 for the model; 29 `getData()` calls in the file                                                                                                                                                                                           | See 10.2.                                                                                                                  |
| `parser/flow-huge.spec.js`                                                                                                            | 0 (1 `it.skip`)   | M3 as an ignored test, plus a linear test of our own                                                                                                                                                                                         | See section 11.                                                                                                            |
| `flowDb.spec.ts`                                                                                                                      | about 30          | Parse-level: 12 (subgraphs `exist` and `makeUniq` 5, `addClass` 2, direction 2, repeated ids 3). M5: 17 that call `getData()` (interpolate 3, collapsible 6, colour slots 4, repeated ids 4). JS-only: 1 (the own-property test of `class`). | Counted by test title; recount in Stage B.                                                                                 |
| `flowDb-elk-duplicate-subgraph.spec.ts`, `flowDiagram.spec.ts`, `flowRenderer-elk-default.spec.ts`, `flowRenderer-v3-unified.spec.ts` | 1, 3, 3, 1        | M4 and M5                                                                                                                                                                                                                                    | Layout and rendering, not parsing.                                                                                         |

### 10.2 Translation rules

- `getData()` expectations (`nodes[0].shape === 'squareRect'`, `label` values): assert the model equivalent in M3 (vertex text, shape name raw), and mark the rendering-only expectation as `#[ignore = "port: getData mapping, M5"]`. Open question 3.
- Spy-based expectations (`expect(flowDb.setLink).toHaveBeenCalledWith(…)`) become assertions on the model result: the link, the target, the tooltip, the `clickable` class.
- `warn` spies become assertions on `warnings`.
- Tests that expect a Jison message (`toThrowError("got 'PS'")`) become "fails, and at which position" (ADR 0002). The position is the byte offset of the offending token, and the test checks the line and column.
- DB messages (`No such shape`, linkStyle index, edge limit) stay exact.
- `it.skip` and `it.todo` stay as `#[ignore = "port: …"]` with the reason.

### 10.3 Jison texts

Throw-tests, counted by `toThrow` in the files (recount in Stage B; some may be `.not`): flow-text 6, flow-node-data 2, flow.spec 4, flow-singlenode 1, flow-style 1. The ones that quote Jison text (`got 'PS'`, `got 'STR'`, `Expecting 'SQE'`, `got 'PE'`) check failure and position under ADR 0002; the message texts are not copied. The DB messages are kept exactly.

## 11. Performance

The reference is quadratic on large input. Where:

- `addVertex` runs `this.edges.find(e => e.id === id)` on every call, not only for shape data. Each `A-->B` line makes two `addVertex` calls.
- `addSingleLink` runs `this.edges.some(...)` and `this.edges.filter(...)` for each edge.
- `addSubGraph`'s `makeUniq` runs `exists` over all subgraphs for every member.
- `setClass` runs `this.edges.find` per id.
- A chain's node list is `concat`-ed at every link, so a chain of length k costs O(k²).

flow-huge builds 47,917 edges between two vertices. It is `it.skip` upstream "because it takes like 2 minutes or more". The Rust design: a `HashMap<String, usize>` from edge id to first index, a `HashMap<(start, end), count>` for generated ids, a `HashSet` of subgraph members, `Vec`s for statement node lists built in order (reversed at the end of each chain, which is linear). Then every operation is O(1) amortised and the whole parse is O(n).

Proposed tests: the ignored port of flow-huge (same assertions, `#[ignore = "port: skipped upstream, takes minutes"]`), plus an unignored test with about 50,000 edges that checks counts and completes. No timing assertions in unit tests. Open question 6 covers whether the big test runs in CI.

## 12. Size and porting order

Estimates, not measurements:

| Part                                                                                                             | Lines (Rust, without tests)                                   |
| ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Scanner, states, rule table, Unicode range table                                                                 | 600 to 800                                                    |
| Parser (statements, vertices, links, text, styles, classes, clicks, linkStyle, subgraphs, header, accessibility) | 1,100 to 1,400                                                |
| DB state and setters (section 7)                                                                                 | 500 to 700                                                    |
| YAML subset for shape data                                                                                       | 350 to 500 (shared with M2 for front matter, open question 2) |
| Model structs (section 8)                                                                                        | 200 to 250                                                    |
| Exporter `tools/port/export-flowchart.ts` and harness glue                                                       | 150 to 200 (TypeScript)                                       |
| Ported specs                                                                                                     | roughly 3,000 lines of Rust tests                             |

For scale: the current `ferrigura-syntax` (M1) is about 1,300 lines for pie and info together. The flowchart is several times larger.

Porting order (Stage B), with the spec files each step adds:

1. **Skeleton and header.** Model structs, scanner, `cleanupComments`, the `}\s*\n` rewrite, the header and the statement loop, empty lines and separators. Specs: flow-comments, the header parts of flow-direction, the basic parts of flow.spec.
2. **Nodes, shapes, text.** All 16 shape forms, bare ids, `&` groups without links, `:::` styles, the three label kinds, keywords in text, unsafe ids, special characters. Specs: flow-singlenode, the vertex parts of flow-text and flow-md-string, the rest of flow.spec.
3. **Edges and arrows.** Link tokens, edge text, lengths, `LINK_ID`, `&` on both sides, chaining, `INVALID`. Specs: flow-arrows, flow-edges (with loops), the edge parts of flow-text, flow-vertice-chaining, the syntax parts of flow-lines.
4. **Subgraphs.** Three forms, generated ids, `makeUniq`, repeated ids, nesting, subgraph direction. Specs: subgraph.spec (parse-level), flow-direction, the parse-level parts of flowDb.spec.

   First differential run: after steps 1 to 4, over every input that uses only these constructs. The harness reports the rest as "not yet" (section 12.1).

5. **Styles and classes.** `classDef`, `class`, `style`, `linkStyle` (default, numbered, interpolate, `fill:none`), `addClass` textStyles are not in the model. Specs: flow-style, the linkStyle parts of flow-lines, the addClass parts of flowDb.spec.
6. **Interactions.** `click`, `href`, `call` (syntax, with callbacks dropped), tooltips, link targets. Specs: flow-interactions, as outcome tests.
7. **Node data and edge data.** The YAML subset, `@{…}` on vertices, subgraphs and edges, `animate`, `animation`, `curve`, the `view` metadata, accessibility. Specs: flow-node-data (model parts), the metadata test of subgraph.spec, the accessibility tests of flow.spec, the edge-id tests of flow-edges.

Final differential run over all flowchart inputs after step 7; no difference except the named exceptions in section 14.

### 12.1 Differential harness

- Exporter: `tools/port/export-flowchart.ts`, loaded through Vite SSR as the pie and info exporters are. Configuration pinned: the defaults of the reference (`securityLevel` strict, `maxEdges` 500, `inheritDir` false), detectors registered first (`reference.ts`).
- Inputs: the flowchart detector of the reference (`^\s*(graph|flowchart)` after front matter and directives), over the 352 e2e fixtures, the 114 docs blocks, the spec string literals, and `cases.ts`.
- A rejection on both sides counts as agreement (existing rule). A Rust-side "not yet supported" result is not a rejection: it is counted separately as "not yet" and is zero at the end.

## 13. Open questions for the coordinating session

1. **Lexer.** Proposal: hand-written scanner for flowchart (first-match rule table, explicit state stack, implicit keyword boundary), with the shared base limited to the cursor, errors, string decoding, and the title and accessibility value rules. Reason: start conditions, lookahead and first-match order cannot be expressed with logos (section 3.2). This changes the shared lexer base, so it needs a decision record (amend 0002 or add 0008). Recommendation: yes, hand-written for flowchart only; pie and info keep logos.
2. **YAML subset.** Shape data needs flow and block mappings, `|` block scalars and JSON-schema typing. M2 needs YAML for the front matter `config` block. Recommendation: one module `yaml.rs` in `ferrigura-syntax`, written by M3 (the first user), reused by M2. Please confirm with the M2 session.
3. **`getData`-dependent expectations** (about 30 tests in flow-node-data, one in subgraph, about 17 in flowDb.spec). Option (a): assert the model equivalent in M3 and mark the rendering mapping ignored until M5. Option (b): port the node mapping into M3. Recommendation: (a), which keeps M3 a parser milestone. The M3 "specs pass" count then has about 30 ignored tests with a clear reason.
4. **Click callbacks.** Dropped from the model; the `clickable` class and the syntax stay. Confirm: this makes the 13 spy-based interaction tests outcome tests.
5. **JS bugs the port does not copy.** `constructor` is accepted as a shape (plain-object `in`), and a `__proto__` node repeated in one subgraph is not de-duplicated (`obj['__proto__'] = true` is ignored). Recommendation: fix both in Rust, and list them as named exceptions in the harness with a reason; "no difference" then means "no difference except the named exceptions". Confirm the exception mechanism, because the done criterion says "no difference".
6. **Large test in CI.** Recommendation: unignored 50k-edge test in the Rust job, no timing assertion. Confirm the CI cost is acceptable.
7. **Parser options.** Proposal: `Options { max_edges: usize }` (default 500), passed by the caller; M2 fills it from the configuration. `inheritDir`, `securityLevel`, `htmlLabels`, `curve` stay out of the parser. Confirm.
8. **Keywords kept through M3.** `graph` is a candidate for removal in FORK.md, but the detector and the grammar accept it, and the specs use it; keep it. `swimlane-beta` stays a graph keyword: the swimlanes diagram reuses this parser. Confirm.
9. **Bare `subgraph` with no title.** The grammar reduces `subgraph separator document end` to `addSubGraph(undefined, …)`, and the DB then reads `_id.text`, which throws a TypeError. Recommendation: treat as an error in Rust and check it in the differential run. Not verified by running the DB; from reading only.
10. **Model names.** `styles` on vertices and `style` on edges, as in the DB; `null` for undefined and `[]` for empty lists. Confirm before the exporter is written.

## 14. Shared files M3 will touch

Small and additive, as the brief says:

- `crates/ferrigura-syntax/src/lib.rs`: a `Flowchart` variant in `DiagramType` and `Diagram`, and the detector rule `^\s*(graph|flowchart)` after the hidden prefix.
- `crates/ferrigura-model/src/lib.rs`: `pub mod flowchart`.
- `tools/port/inputs.ts`: a `flowchart` detector entry. `diff.ts` and `status.ts`: the type name.
- `tools/port/export-flowchart.ts` (new), `tools/port/cases.ts` (explicit cases from section 3.3).
- `docs-port/STATUS.md`: the M3 row and the flowchart line of the P1 table.
- `docs-port/canonical-model.md`: a flowchart section with the header rules of 8.3 (coordinator-owned; written when the first model lands).
- CI: the `port-diff` job gets flowchart in its matrix.

## 15. Setup note

The worktree at `.claude/worktrees/blissful-cori-9818ff` has no `node_modules` and no build output. Stage B needs `pnpm install` in the worktree (or a shared store) before `pnpm port:diff flowchart` runs. Stage A used the main checkout's `node_modules/jison` only to generate the probe parser, from a copy of `flow.jison` in the scratchpad, and changed nothing in the repository.
