# ishikawa (fishbone, P2): research notes for the port

Status: research only, nothing in the repository changed except this file. Reference: the TypeScript checkout at this worktree's HEAD (`aa5c4338f`). The bundle `packages/mermaid/dist` was built on 2026-10-16 11:57, after the last change of `packages/mermaid/src/diagrams/ishikawa/` (2026-05-12) and of the shared helpers it reads (2026-10-16 09:23), so the probes below used the same code as the source.

Verification legend used below:

- **[V]** verified by running the reference: the parser and DB through the source (`tools/port/reference.ts`, Vite SSR, with `preprocessDiagram` and `encodeEntities` as `Diagram.fromText` gives them), the renderer through the harness's own page (`tools/port/geometry/common.ts`: `withPage`, `renderReference`, and `geometry` at scale 1 with the whole-pixel viewBox), or the library in Node;
- **[S]** read from the source, with the file and line;
- **[U]** not verified; stated as an open point.

Probe material, not committed: `/private/tmp/claude-501/-Users-sebastian-Workspace-ferraid/a04a38c7-48dd-4c04-90fb-2087e133faea/scratchpad/p2-ishikawa/` (parser: `probe-parse.mts` and `cases-ishikawa.json`, 43 inputs, results in `out-parse.json`; configuration: `config-probe.mts`; measurement: `measure-scale1.mts`, `offsets.mts`, `measure.mts`). The rendered reference SVGs of the 13 fixtures are in `scratchpad/ref-ishikawa/`, and the render checks of the `handDrawn` seed and of the wrapping are in `scratchpad/probe-seed/`.

## 0. Summary

| Item              | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference sources | 7 files without the spec, 774 lines: `ishikawaRenderer.ts` 513, `ishikawaDb.ts` 96, `ishikawaStyles.ts` 68, `ishikawaDetector.ts` 22, `ishikawaDiagram.ts` 15, `ishikawaTypes.ts` 4, `parser/ishikawa.jison` 56                                                                                                                                                                                                                                                                                                                                                         |
| Specs             | 1 file, 3 tests (`ishikawa.spec.ts`): the grammar and the DB's root and children. No renderer test                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Fixtures          | 13 `.mmd` files: 11 in `pr-check/`, 2 in `nightly-check/`; `visual-check/` is empty                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Parser            | Jison: 8 lexer rules and 6 nonterminals. A hand-written port is the recommendation (decision 0002)                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Renderer          | SVG drawn from hand-computed geometry (a spine, branches, bones). The layout reads four `getBBox` values of the drawn text and of the root, so the layout depends on Chromium's text boxes                                                                                                                                                                                                                                                                                                                                                                              |
| Biggest risks     | (1) Replicating `getBBox` of drawn text: the anchor and the baseline come from the stylesheet, the lines are `tspan`s, and the boxes sit on a 1/64 px grid (section 6.3). (2) The root box includes the tight bounds of a quadratic curve (section 6.3). (3) `look: handDrawn` uses roughjs, which draws with `Math.random` at the default seed 0; the output is different on every render [V] (the harness excludes it). (4) The labels are drawn at the theme's font size (14 px in redux-color) while the layout uses the configured font size (16 px) (section 6.4) |
| Effort            | About 900 lines of Rust: the renderer about 550, the parser and DB about 250, the stylesheet about 70, tests about 150. roughjs for `handDrawn`, if it is in scope, is extra and not sized here [U]                                                                                                                                                                                                                                                                                                                                                                     |

## 1. Sources

| File                                      | Lines       | Role                                                                                                                                                                                                                                                                                                                                                      |
| ----------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `diagrams/ishikawa/ishikawaDiagram.ts`    | 8-15        | Definition: `parser`, `renderer`, `styles`. `get db()` (10-12) returns a new `IshikawaDB` on each read (section 3.4)                                                                                                                                                                                                                                      |
| `diagrams/ishikawa/ishikawaDetector.ts`   | 9-11        | Detector `/^\s*ishikawa(-beta)?\b/i` (line 10). Loader 13-16                                                                                                                                                                                                                                                                                              |
| `diagrams/ishikawa/parser/ishikawa.jison` | 1-15        | Lexer: `%options case-insensitive` (2), rules (6-13)                                                                                                                                                                                                                                                                                                      |
| same                                      | 17-56       | Grammar: `start` (21-24), `spaceLines` (26-30), `ishikawa` (32-35), `stop` (37-43), `document` (45-48), `statement` (50-55)                                                                                                                                                                                                                               |
| `diagrams/ishikawa/ishikawaDb.ts`         | 1-96        | DB: state (21-23), `clear` (31-36), `getRoot` (38-40), `addNode` (42-71), the common accessors (73-95)                                                                                                                                                                                                                                                    |
| `diagrams/ishikawa/ishikawaRenderer.ts`   | 1-513       | Renderer: constants (19-26), `applyPaddedViewBox` (28-34), `draw` (36-150), `sideStats` (152-165), `drawHead` (167-211), `flattenTree` (232-263), `drawCauseLabel` (265-311), `drawArrowMarker` (314-343), `drawBranch` (345-437), `splitLines` and `wrapText` (439-455), `drawMultilineText` (457-481), `lerp` (483), `drawLine` (485-511), export (513) |
| `diagrams/ishikawa/ishikawaStyles.ts`     | 3-66        | The stylesheet, a function of the theme options (default export, line 68)                                                                                                                                                                                                                                                                                 |
| `diagrams/ishikawa/ishikawaTypes.ts`      | 1-4         | `IshikawaNode { text, children }`                                                                                                                                                                                                                                                                                                                         |
| `diagrams/ishikawa/ishikawa.spec.ts`      | 1-66        | The three tests (section 5)                                                                                                                                                                                                                                                                                                                               |
| `setupGraphViewbox.js`                    | 23-46       | `calculateSvgSizeAttrs`, `configureSvgSize`                                                                                                                                                                                                                                                                                                               |
| `utils.ts`                                | 823 onwards | `parseFontSize`                                                                                                                                                                                                                                                                                                                                           |
| `diagrams/common/common.ts`               | 71-83       | `sanitizeText` (DOMPurify on every text)                                                                                                                                                                                                                                                                                                                  |
| `schemas/config.schema.yaml`              | 443-445     | Top-level `fontSize`, default 16                                                                                                                                                                                                                                                                                                                          |
| same                                      | 1221-1236   | `IshikawaDiagramConfig`: `diagramPadding` default 20 (1232), `useMaxWidth` default false (1236)                                                                                                                                                                                                                                                           |
| same                                      | 457-470     | `BaseDiagramConfig.useMaxWidth`, default true (470)                                                                                                                                                                                                                                                                                                       |
| `defaultConfig.ts`                        | 315-317     | The `ishikawa` section is the schema's defaults                                                                                                                                                                                                                                                                                                           |
| `themes/theme-redux-color.js`             | 27          | `fontSize = '14px'`, the font size of the redux-color theme (the harness theme)                                                                                                                                                                                                                                                                           |
| `themes/theme-default.js`                 | 33          | `fontSize = '16px'`                                                                                                                                                                                                                                                                                                                                       |

## 2. Parser

### 2.1 Lexer

The rules are tried in file order and the first match wins (no longest match).

| Rule (line)                             | Token     | Notes                                                                                                           |
| --------------------------------------- | --------- | --------------------------------------------------------------------------------------------------------------- |
| `\s*\%\%.*` (6)                         | SPACELINE | A `%%` comment. The leading `\s*` can take newlines with it, so a comment can remove line breaks [S, the regex] |
| `"ishikawa-beta"` (7), `"ishikawa"` (8) | ISHIKAWA  | Jison adds `\b` to a literal rule that ends in a word character [V] (below)                                     |
| `[\s]+[\n]` (9)                         | SPACELINE | Blanks that end in a newline. `\s` includes newlines                                                            |
| `[\n]+` (10)                            | NL        | A run of newlines is one token                                                                                  |
| `[\s]+` (11)                            | SPACELIST | Indentation, or blanks not followed by a newline. Its length is the level (section 3.2)                         |
| `[^\n]+` (12)                           | TEXT      | The rest of the line, blanks included                                                                           |
| `<<EOF>>` (13)                          | EOF       |                                                                                                                 |

The generated lexer rules, dumped from the reference's `grammar.lexer.rules` [V]: `^(?:\s*%%.*)`, `^(?:ishikawa-beta\b)`, `^(?:ishikawa\b)`, `^(?:[\s]+[\n])`, `^(?:[\n]+)`, `^(?:[\s]+)`, `^(?:[^\n]+)`, `^(?:$)`. The `\b` is what makes `ishikawaEffect` a TEXT (quirk I3).

### 2.2 Grammar

```
start      : ishikawa | spaceLines ishikawa                              (21-24)
spaceLines : SPACELINE | spaceLines SPACELINE | spaceLines NL            (26-30)
ishikawa   : ISHIKAWA document { return yy; }
           | ISHIKAWA NL document { return yy; }                         (32-35)
stop       : NL | EOF | SPACELINE | stop NL | stop EOF                   (37-43)
document   : document statement stop | statement stop                   (45-48)
statement  : SPACELIST TEXT  { yy.addNode($1.length, $2.trim()); }
           | TEXT            { yy.addNode(0, $1.trim()); }
           | SPACELINE | SPACELIST                                       (50-55)
```

The only DB calls are the two `addNode` actions (51-52). Every statement must be followed by a `stop`, which is a newline, the end of input, or a line of blanks.

### 2.3 Quirks a port must reproduce

Each row was run through the reference (`out-parse.json`). The text is the input after `preprocessDiagram` and `encodeEntities`, with a newline added, as `Diagram.fromText` gives it.

| #   | Input                                                                | Reference behaviour                                                                                                                                                                                                                                                          |
| --- | -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| I1  | `ishikawa` alone                                                     | Parse error [V]                                                                                                                                                                                                                                                              |
| I2  | `ishikawa` followed by a newline (the parser sees `ishikawa` NL EOF) | Accepted, and `getRoot()` is undefined: no root, no nodes [V]. The mechanism is not traced [U]                                                                                                                                                                               |
| I3  | `ishikawaEffect` then `Cause`                                        | Parse error [V]. The `\b` of the literal rule makes `ishikawaEffect` one TEXT token, which comes before the header, and the grammar does not accept that                                                                                                                     |
| I4  | `ishikawa Effect` then `  Cause A`                                   | Root `Effect`, with child `Cause A` [V]. The header line's own text is the first statement                                                                                                                                                                                   |
| I5  | `ishikawa-betafoo` then `P`                                          | Root `-betafoo`, with child `P` [V]. `ishikawa` matches with its `\b`, and `-betafoo` is TEXT. (The detector accepts it too, `/^\s*ishikawa(-beta)?\b/i`)                                                                                                                    |
| I6  | `ishikawa  ` (blanks after the header) then `P`                      | Parse error [V]. The blanks and the newline form a SPACELINE, which the grammar does not allow after the header                                                                                                                                                              |
| I7  | `ISHIKAWA` as the header                                             | Accepted [V]. The lexer is case-insensitive, and the detector's `i` flag makes it detected too [S]                                                                                                                                                                           |
| I8  | a second `ishikawa` line                                             | Parse error [V]                                                                                                                                                                                                                                                              |
| I9  | leading blank lines and whitespace-only lines                        | Ignored [V]                                                                                                                                                                                                                                                                  |
| I10 | a `%%` line between causes, or indented                              | Ignored [V]. The comment rule can take the newline before it, but the grammar accepts a SPACELINE as a line end, so the result is the same (`P`, `Q` in `P` NL `%%x` NL `Q`)                                                                                                 |
| I11 | `50%% done` as a cause                                               | The text `50%% done`. A `%%` only starts a comment at the start of a token [V]                                                                                                                                                                                               |
| I12 | tabs as indentation                                                  | Each tab is one level unit: `\ta` is level 1, `\t\tb` level 2 [V]                                                                                                                                                                                                            |
| I13 | `P`, then `    a` (indented), then `  b`                             | Both `a` and `b` are children of `P`: the first cause sets the base level, and a dedent to a level ≤ 0 is level 1 [V]                                                                                                                                                        |
| I14 | `P`, then `  a`, then `    x`, then `  b`, then `Q`                  | `a` has child `x`; `b` and `Q` are children of `P` [V]                                                                                                                                                                                                                       |
| I15 | the first line after the header is indented: `ishikawa` then `  a`   | `a` is the root [V] (the first statement is the root whatever its indentation)                                                                                                                                                                                               |
| I16 | `accTitle: x`, `title T`, `accDescr: y` as lines                     | Ordinary causes [V] (the grammar has no such rules)                                                                                                                                                                                                                          |
| I17 | `P<br>Q`, `a<br/>b`                                                  | Kept as `P<br>Q` and `a<br>b` [V]: the sanitiser writes `<br/>` as `<br>`, and the renderer splits on `<br>` (section 6.2)                                                                                                                                                   |
| I18 | `P & <b>x</b>`                                                       | `P &amp; <b>x</b>` [V]. The text has a `<`, so DOMPurify parses and serialises it. Without a `<`, `&` stays as it is (section 3.3)                                                                                                                                           |
| I19 | `P;x`                                                                | Kept as `P;x` [V] (`;` is ordinary text)                                                                                                                                                                                                                                     |
| I20 | CRLF line endings                                                    | Accepted [V]                                                                                                                                                                                                                                                                 |
| I21 | a line with only a no-break space (` `)                              | Ignored [V]: JavaScript's `\s` includes U+00A0                                                                                                                                                                                                                               |
| I22 | `P` then `Q` at the same indentation                                 | `Q` is a child of `P`: the first cause is at level 1 and so is `Q` [V]                                                                                                                                                                                                       |
| I23 | a blank line between causes                                          | Ignored [V]                                                                                                                                                                                                                                                                  |
| I24 | no newline after the last line                                       | Accepted [V] (EOF is a stop)                                                                                                                                                                                                                                                 |
| I25 | front matter (`---` block) before the header                         | Accepted, the block is removed [V] (`preprocessDiagram`)                                                                                                                                                                                                                     |
| I26 | `%%{init: …}%%` before the header                                    | Renders correctly in the full reference pipeline [V] (`render-ref`). The parser probe could not run it: the directive code calls `detectType` (`utils.ts` 117), and the probe's detector registry has no ishikawa entry. This is a limit of the probe, not a reference quirk |

The levels in I13 and I14 follow section 3.2.

### 2.4 What the DB receives

`addNode(rawLevel, text)` with `rawLevel` the length of the SPACELIST before the text (51), or 0 (52). The text is trimmed before the call.

## 3. DB

### 3.1 State and calls

`IshikawaDB` (`ishikawaDb.ts` 20-96) holds `root` (undefined until the first node), `stack` (the open nodes, each with a level) and `baseLevel` (undefined until the second node).

| Method                                                                                                       | Behaviour                                                                                                                                                                                                                                         | Lines |
| ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| `addNode(rawLevel, text)`                                                                                    | `label = common.sanitizeText(text, getConfig())` (43). The first call creates the root `{text: label, children: []}`, resets the stack to `[{level 0, root}]`, and calls `setDiagramTitle(label)` (45-50). Later calls place a node (section 3.2) | 42-71 |
| `clear()`                                                                                                    | Resets `root`, `stack`, `baseLevel`, and calls the common `clear` (accTitle, accDescr, title)                                                                                                                                                     | 31-36 |
| `getRoot()`                                                                                                  | `root`                                                                                                                                                                                                                                            | 38-40 |
| `getAccTitle`, `setAccTitle`, `getAccDescription`, `setAccDescription`, `getDiagramTitle`, `setDiagramTitle` | Pass through the common DB                                                                                                                                                                                                                        | 73-95 |

The title is set from the root's sanitised text, so the diagram title is the effect text [V] (the probes: `title` equals the root text in every case).

### 3.2 Levels

For a node after the first:

1. `baseLevel ??= rawLevel` (55): the first cause fixes the base, so the effect's own indentation does not count.
2. `level = rawLevel − baseLevel + 1`, and a level ≤ 0 becomes 1 (57-60).
3. Pop while the stack has more than the root and the top's level is ≥ `level` (62-65).
4. The parent is the top of the stack; the node is appended to its `children` and pushed with `level` (67-70).

Examples the reference gave [V]: `P`, `  a`, `    x`, `  b`, `Q` gives `P` → `a` → `x`, then `b`, then `Q`, with `a` and `b` and `Q` children of `P` (I14); the same with the effect indented by four gives the same tree (the `effect indented` spec).

### 3.3 Sanitising

`common.sanitizeText` runs on every node text and on the title, through DOMPurify (`common.ts` 71-83). DOMPurify returns text with no `<` unchanged, and serialises text with `<` (I18). The DB keeps that result, and the renderer draws it as text. Checked [V] with `P & <b>x</b>` and `a & b`:

- `P & <b>x</b>` becomes `P &amp; <b>x</b>` in the DB. The head wraps it at 11 characters, and the reader sees `P &amp;` and `<b>x</b>` as text, because the `&amp;` is written once more when the SVG is serialised.
- `a & b` has no `<`, so it stays `a & b` in the DB and reads `a & b`.

### 3.4 The DB instance

`ishikawaDiagram.ts` 10-12 defines `get db()` to return `new IshikawaDB()`, and the renderer reads `diagram.db` (`ishikawaRenderer.ts` 37). A new instance per read would give the renderer an empty DB. The reference's renders show the parsed tree, so the renderer sees the instance the parser filled [V]. How `Diagram` keeps that instance is not traced [U]. The port needs one DB per diagram.

## 4. Canonical model proposal

The model is the DB state after parsing, in the conventions of `docs-port/canonical-model.md`:

```
{
  "version": 1,
  "title": "Blurry Photo",     // common DB diagramTitle: the root text, sanitised
  "accTitle": "",              // common DB, sanitised
  "accDescr": "",              // common DB, sanitised
  "root": {                    // null when no statement was read (I2)
    "text": "Blurry Photo",
    "children": [
      { "text": "Process", "children": [ { "text": "Out of focus", "children": [] } ] },
      { "text": "User",    "children": [ { "text": "Shaky hands", "children": [] } ] }
    ]
  }
}
```

- `children` keep the insertion order, which is the order of the source.
- The texts are the sanitised texts; line breaks in them (`<br>`) are raw.
- Left out: the stack and the base level (parse state), and the layout. The layout is the renderer's, and the geometry harness covers it (section 6). `title` is in the model because it is DB state, even though it equals `root.text`.

## 5. Specs

| Test (`ishikawa.spec.ts`)                                      | What it checks                                                                                               |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| "should parse a basic Ishikawa hierarchy" (13-30)              | The root text, two children, and their children, from a file with `ishikawa-beta` and four-space indentation |
| "should support an unindented root with nested causes" (32-47) | The root with an unindented first line, a nested cause, and a sibling                                        |
| "should handle effect indented more than causes" (49-65)       | The effect indented past the causes gives the same tree (the base level is the first cause)                  |

The spec sets `ishikawa.yy = new IshikawaDB()` with a `clear` before each test (7-11). It tests the grammar and the DB together, through `getRoot()`. No layout or SVG is tested.

## 6. Renderer

### 6.1 Pipeline and DOM order

`draw(_text, id, _version, diagram)` (`ishikawaRenderer.ts` 36-150) draws the classic look as follows. The root `svg` and the empty `g` come from the mermaid wrapper (`mermaidAPI.ts` 413-419), as in the quadrant notes.

```
svg  width="100%" style="max-width: Wpx;" viewBox="…" role aria-roledescription="ishikawa"
  style …
  g                                     (empty, mermaidAPI.ts 419)
  g.ishikawa                            (line 52)
    defs > marker#ishikawa-arrow-<id>   viewBox 0 0 10 10, refX 0, refY 5, markerWidth 6,
                                        markerHeight 6, orient auto (64-74)
      path.ishikawa-arrow               d="M 10 0 L 0 5 L 10 10 Z" (75-77)
    line.ishikawa-spine                 x1 = spineX, y1 = y2 = spineY, x2 = 0 (80-86, 121, 145)
    g.ishikawa-head-group               transform="translate(0,spineY)" (123, 176-179)
      path.ishikawa-head                d = "M 0 -h/2 L 0 h/2 Q 2.4w 0 0 -h/2 Z" (194, 208)
      text.ishikawa-head-label          text-anchor="start", x=0, y=-(lines-1)·lh/2,
                                        transform = translate(…) (181-189, 210)
        tspan x=0 dy (0, then lh)
    g.ishikawa-pair                     one per pair of causes (127)
      line.ishikawa-branch              upper cause, marker-start (362; 147)
      g.ishikawa-label-group            (274)
        rect.ishikawa-label-box         first child (304-309)
        text.ishikawa-label.cause       middle (275-283)
      g.ishikawa-sub-group              one per descendant, in walk order (396)
        line.ishikawa-sub-branch        marker-start (408 or 419; 147)
        text.ishikawa-label.align|up|down  (412, 423)
      line.ishikawa-branch              lower cause
      g.ishikawa-label-group …
      g.ishikawa-sub-group …
```

- A pair is the upper cause (even index in the root's children) and the lower one (odd index), in that order (128-131). The pair's lower cause has no branch if there is none.
- The marker is only added to the branch and sub-branch lines, after everything is drawn (147). The spine has none [V].
- In handDrawn mode the markers are not created: the arrow is drawn by roughjs (section 6.6).

### 6.2 Layout

Constants (`ishikawaRenderer.ts` 19-26): `FONT_SIZE_DEFAULT = 14`, `SPINE_BASE_LENGTH = 250`, `BONE_STUB = 30`, `BONE_BASE = 60`, `BONE_PER_CHILD = 5`, `ANGLE = 82·π/180`, `COS_A = cos(ANGLE)`, `SIN_A = sin(ANGLE)`. In V8 `Math.cos(ANGLE)` is `0.13917310096006569` and `Math.sin(ANGLE)` is `0.99026806874157025` (17 significant digits) [V]. The port must reach these values, or the layout drifts in the last bits.

`fontSize = parseFontSize(config.fontSize)[0] ?? 14` (45), so the layout font is the configured one (16 by default).

1. **Spine start.** `spineX = 0`, then `spineX = −20` before the pairs (97). `spineY` is set below (119).
2. **Sides.** Causes at even indexes are the upper side, the others the lower side (99-100). `sideStats` gives each side its total of descendants and the largest count under one cause (152-165).
3. **Side lengths.** With `descendantTotal = upper.total + lower.total`: if it is positive, `upperLen = max(75, 500·upper.total/total)` and the same for the lower side (106-113); otherwise both are 250 (106-107). Then each length is at least `maxDescendants · 2·fontSize` (115-117).
4. **Spine height.** `spineY = max(upperLen, 250)` (119). The lower length does not enter it [S, the code].
5. **Branch.** For a cause with `n` children, `lineLen = length · (n > 0 ? 1 : 0.2)` (356). The branch runs from `(startX, startY)` to `(startX − COS_A·lineLen, startY + SIN_A·lineLen·dir)` (357-361), with `dir` = −1 up, +1 down.
6. **Descendants.** `flattenTree` (232-263) lists the descendants by depth. The children of an upper cause are visited in reverse (236). Even depths are placed in pre-order and odd depths in post-order (246-258). Each entry gets a `y` on the branch: `y = startY + dy·(slot+1)/(count+1)`, where `slot` is the entry's place in that order (372-377).
7. **Horizontal bones (even depth).** `bx0` is the point on the parent's line at height `y`, by linear interpolation (405). `bx1 = bx0 − (60 + 5·n)` for `n` children, or `bx0 − 30` for a leaf (407). The label is drawn `end`-anchored at `bx1` (412).
8. **Diagonal bones (odd depth).** `k` counts the children already drawn from the parent (415). `bx0 = lerp(parent.x0, parent.x1, (n − k)/(n+1))` (416), `by0 = parent.y0`, and `bx1 = bx0 + (−COS_A)·((y − by0)/(SIN_A·dir))` (418). The label is drawn `end`-anchored at `bx1` with the class `up` or `down` (391, 423).
9. **Cause labels.** The cause text is drawn `middle`-anchored at `(endX, endY + 11·dir)` (279). Its text is not wrapped (section 6.5).
10. **Spine x.** After each pair, `spineX` becomes the smallest `getBBox().x` of the pair's text elements (136-139). The spine's left end is `spineX` at the end (145).
11. **Root.** The padded viewBox (section 6.3).

Worked check [V], fixture 1 (`Blurry Photo`, `Process` → `Out of focus`, `User` → `Shaky hands`): one pair, both sides have one descendant, so both lengths are 250 and `spineY = 250`. The upper branch ends at `(−20 − 250·COS_A, 250 − 250·SIN_A)` = `(−54.79327524001642, 2.432982814607442)`. Its child is drawn at `y = 250 − 247.567/2 = 126.2165`, with `bx0 = −37.3966` and `bx1 = −67.3966` (`bx1 = bx0 − 30`, a leaf). The reference's SVG has these values.

### 6.3 Text measurement

The renderer reads a box of drawn text in four places. The rest of the geometry is arithmetic.

| Site                 | Line    | Element                         | What it sets                                        |
| -------------------- | ------- | ------------------------------- | --------------------------------------------------- |
| `applyPaddedViewBox` | 29      | the root `svg`                  | the root's width and height, and the viewBox origin |
| pair spine           | 137-139 | the `text` elements of one pair | `spineX`, the x of the spine's left end             |
| `drawHead`           | 190     | the head's `text`               | the head's `w` and `h`, and the text's transform    |
| `drawCauseLabel`     | 284     | the cause `text`                | the label box `rect`                                |

What the reference's measurements are, checked at scale 1 with the harness's own `geometry` path [V]:

- Each text box is the union of its `tspan` lines. The `x` of a box follows the anchor: for `text-anchor: middle` the box is centred on `x`, for `end` it ends at `x`.
- The anchor and the baseline come from the stylesheet, not from the presentation attributes. The head's attribute is `text-anchor="start"`, but the rule `.ishikawa .ishikawa-head-label { text-anchor: middle }` wins, so the computed anchor is `middle` [V] (the head box is centred on 0 and the transform's `+3` and `(w − tb.width)/2` centre it).
- Box tops relative to the `y` of the text (14 px, Lexica Ultralegible, measured from the reference's SVG in Chromium) [V]:

| Baseline (computed)      | Classes                                                               | Box top       | Box height |
| ------------------------ | --------------------------------------------------------------------- | ------------- | ---------- |
| `middle`                 | `ishikawa-head-label`, `ishikawa-label cause`, `ishikawa-label align` | `y − 9.53125` | 17         |
| `auto` (from `baseline`) | `ishikawa-label up`                                                   | `y − 13`      | 17         |
| `hanging`                | `ishikawa-label down`                                                 | `y − 2.59375` | 17         |

- Widths are on Chromium's 1/64 px grid: `Process` is 48.640625, `Out of focus` 75.03125, `Blurry` 39.3125 in the bold face.
- The head's font weight is 600 (`ishikawaStyles.ts` 39). Chromium draws it in the bold face: `Blurry` is 39.3125 wide at 600 and 35.40625 at 400 [V]. The port must map 600 to the bold face (the nearest heavier face).
- The root's box is the union of its children: text, lines, rects and paths. For the head path `M 0 −53.8 L 0 53.8 Q 144 0 0 −53.8 Z`, Chromium's box is `[0, −53.8, 72, 107.6]`: the x extent is 72, the curve's extreme, not the control point's 144 [V]. So the root box uses the tight bounds of quadratic curves.

The viewBox then pads the root's box by `diagramPadding` on each side (`applyPaddedViewBox`, 28-34). For fixture 1 the box is `[−144.5373, −20.0983, 216.5373, 538.134]`, and the viewBox is `[−164.5373, −40.0983, 256.5373, 578.134]` [V].

The numbers in the reference's SVG (for example `width="48.640625"` in the label rect, which is the cause box plus 40) lie on the 1/64 grid, and the harness's scale-1 path gives the same numbers [V]. A view scaled by `max-width` gives off-grid boxes (`48.6115` for the same label), so the port should be compared at scale 1.

### 6.4 Styles, fonts and theme variables

`ishikawaStyles.ts` (3-66) is a function of the theme options. The options it reads: `lineColor` (4-28), `mainBkg` (21, 27), `fontFamily` (33), `fontSize` (34), `textColor` (35). The rules:

- `.ishikawa text { font-size: <fontSize>; fill: <textColor>; font-family: <fontFamily> }` (32-36). With redux-color the value is `14px` (`theme-redux-color.js` 27), so the labels are drawn at 14 px [V] (computed size `14px`). With `default` it is `16px` (`theme-default.js` 33).
- `.ishikawa-head-label` has `font-weight: 600; text-anchor: middle; dominant-baseline: middle; font-size: 14px` (38-43). The `14px` is written, so the head is 14 px whatever the theme.
- `.ishikawa-label` is `text-anchor: end` (45-47); `.cause` is `middle` with `middle` baseline (49-52); `.align` is `end` with `middle` (54-57); `.up` is baseline (59-61); `.down` is hanging (63-65).

The layout uses the configured `fontSize` (16) for the line step `lh = fontSize·1.05` = 16.8 (`dy` in the `tspan`s, 467, 477), for the head's line length (175) and for the minimum spacing (115). So the drawn text is 14 px and the spacing is that of 16 px. The port must take the stylesheet's size and the configuration's size from their own sources.

The renderer reads `themeVariables.lineColor` and `mainBkg` only for handDrawn (59-60).

### 6.5 Text wrapping

`wrapText(text, maxChars)` (441-455): if `text.length ≤ maxChars` the text is returned unchanged. Otherwise the text is split at `\s+` and the words are packed greedily: a word joins the last line when `line.length + 1 + word.length ≤ maxChars`, otherwise it starts a new line. A single word longer than `maxChars` stays on its own line.

| Use                                    | `maxChars`                                                    | Line     |
| -------------------------------------- | ------------------------------------------------------------- | -------- |
| The head (root text)                   | `max(6, floor(110 / (fontSize · 0.6)))`, which is 11 at 16 px | 175, 183 |
| Sub-causes (descendants of the causes) | 15                                                            | 242      |
| Top-level causes                       | not wrapped: `drawBranch` passes `node.text`                  | 366      |

Checked [V] with the text `A long effect title for wrapping` (head: `A long`, `effect`, `title for`, `wrapping`), `A very long top level cause label text` (one line) and `a sub cause that is quite long indeed` (`a sub cause`, `that is quite`, `long indeed`).

`length` is JavaScript's UTF-16 length: an emoji is 2 (the port must count UTF-16 units). `\s` is JavaScript's whitespace set, which includes U+00A0 and U+3000 and excludes U+180E.

`splitLines` (439) splits on `<br>`, `<br/>` or `<br />` (the regex is `<br\s*\/?>`, case-sensitive) and on `\n`. Each line is one `tspan` with `x` and `dy` (0, then `lh`), and the whole text is moved up by `(lines − 1)·lh/2` (466-473).

### 6.6 handDrawn and randomness

`look: handDrawn` (46) draws the spine, the head, the branches, the labels and the arrows with roughjs (`rough.svg`, roughjs 4.6.6, patched, `package.json`). The seed is `handDrawnSeed` (58), which defaults to **0** (`config.schema.yaml` 71-75), and roughjs draws from `Math.random` when the seed is 0 [S, the roughjs behaviour; the render differences are [V]].

- Two renders of fixture 6 with the default seed differ [V]. The harness excludes handDrawn (`common.ts` 172).
- With `handDrawnSeed: 42` two renders are identical [V] (`probe-seed/seeded.mmd`). A seeded handDrawn port is therefore testable, but only against roughjs's own algorithm, which the port would have to reproduce [U, size not assessed].
- The classic look has no randomness and no clock [S].

### 6.7 Configuration and defaults

`getConfig()` for this diagram [V] (`config-probe.mts`): `ishikawa.useMaxWidth` is **true**, `ishikawa.diagramPadding` is 20, top-level `fontSize` 16, `handDrawnSeed` 0, `look` `classic`, `theme` `default`.

- `diagramPadding` is the schema's `IshikawaDiagramConfig` default (1232).
- `useMaxWidth` has the schema default **false** (`IshikawaDiagramConfig`, 1236) but the reference's effective value is **true**, and the renders use the max-width form (section 6.8). The fixtures say so too: `should-render-usemaxwidth-true` is "(the default)". The likely mechanism is that the base's default (470) wins when the two defaults are merged; that is not traced [U].
- The renderer reads `ishikawa.diagramPadding ?? 20` and `ishikawa.useMaxWidth ?? false` (49-50). The `?? false` never applies, since the default is true.

### 6.8 Root sizing

`applyPaddedViewBox` (28-34): `w = bbox.width + 2·pad`, `h = bbox.height + 2·pad`, then `configureSvgSize(svg, h, w, useMaxWidth)` and `viewBox = (bbox.x − pad, bbox.y − pad, w, h)`. With `useMaxWidth` the root gets `width="100%"` and `style="max-width: <w>px;"`; without it, `height` and `width` in pixels (`setupGraphViewbox.js` 23-46). The `useMaxWidth: false` fixture shows the fixed form [V] (`width="256.5372619628906" height="578.134033203125"`).

### 6.9 DOM dependencies to replace

`selectSvgElement(id)` (51) and the `svg`, `g`, `select`, `append`, `insert(':first-child')`, `selectAll('text').nodes()` calls (the rendering uses d3 throughout), `getBBox` (four sites), roughjs. A server renderer draws the tree directly and needs the four measurements from the text port (section 6.3). The `insert(':first-child')` calls matter for order: the head path and the label rect come before their texts.

## 7. Fixtures

| Fixture                                                                        | What it exercises                                                                                                                                            |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `pr-check/1-should-render-a-simple-ishikawa-diagram`                           | The baseline: two causes, one child each (section 6.2's worked check)                                                                                        |
| `pr-check/2-should-render-with-many-causes-on-both-sides`                      | Several causes on each side, so the side lengths and the spine height come from the totals                                                                   |
| `pr-check/3-should-render-with-deeply-nested-causes`                           | Odd and even depths: diagonal and horizontal bones                                                                                                           |
| `pr-check/4-should-render-with-a-single-cause`                                 | One cause: one pair with one branch                                                                                                                          |
| `pr-check/5-should-render-with-no-children-root-only`                          | A root with no causes: the spine is a zero-length line, the head is drawn, and the viewBox pads it                                                           |
| `pr-check/6-should-render-with-handdrawn-look`                                 | `look: handDrawn`: roughjs paths. Excluded from the geometry harness, and random at seed 0                                                                   |
| `pr-check/7-should-render-with-forest-theme`                                   | `theme: forest` in the front matter: the harness forces `redux-color`                                                                                        |
| `pr-check/8-should-render-with-dark-theme`                                     | `theme: dark`: the harness forces `redux-dark-color`                                                                                                         |
| `pr-check/9-should-render-with-custom-diagrampadding`                          | `diagramPadding: 50` in the front matter                                                                                                                     |
| `pr-check/12-should-render-correctly-when-effect-is-indented-more-than-causes` | The effect's indentation is not a level (I13)                                                                                                                |
| `pr-check/13-should-render-a-very-deep-nested-diagram`                         | Twelve top-level causes, a seven-level chain, wide nodes (eight and six children), and a repeated label. Its head is `Very deep` (9 characters, not wrapped) |
| `nightly-check/should-render-usemaxwidth-true`                                 | `useMaxWidth` set to true explicitly                                                                                                                         |
| `nightly-check/should-render-usemaxwidth-false`                                | `useMaxWidth: false`: the fixed root form                                                                                                                    |

The fixture numbers skip 10 and 11 (there are no such files). No fixture has text the bundled faces do not cover: the only non-ASCII characters are the em dashes of the front-matter descriptions. The generic harness excludes the handDrawn fixture (`common.ts` 172) and needs no other exclusion [S].

## 8. Risks, effort and open questions

### 8.1 Risks

1. **The text boxes** (section 6.3). The four `getBBox` sites decide the viewBox, the spine position and the label boxes. The port must reproduce the box of each drawn text: the stylesheet's anchor and baseline, the `tspan` lines, the 1/64 grid, and the three baseline offsets in the table. The offsets at 14 px are known; the rule that produces them from the font's metrics needs the same metric choice as `ferrigura-text`'s `text_box` [U, not checked against Chromium's rule].
2. **Tight curve bounds** (section 6.3): the root box uses the extreme of the head's quadratic curve (72, not 144). The port's path bounds must do the same.
3. **The font sizes** (section 6.4): the stylesheet's `font-size` (the theme's) and the layout's (the configuration's) differ in redux-color. The port must keep the two.
4. **`useMaxWidth`** (section 6.7): the effective default is true, so the port's default must be true, not the schema's false.
5. **roughjs** (section 6.6): `look: handDrawn` is a different renderer. Excluding it from the port is a decision for the coordinator.
6. **UTF-16 lengths and JavaScript whitespace** in `wrapText` (section 6.5).
7. **`Math.cos` and `Math.sin`** of 82°. The 17-digit values are in section 6.2. A Rust `f64::cos` may differ in the last bit on some platforms; the port should check its values against these.
8. **The `\b` of the literal header rule** (section 2.1): it is what makes `ishikawaEffect` an error, and the port's lexer must test the word boundary the same way.

### 8.2 Effort

About 900 lines of Rust: the renderer about 550 (the layout, the bones, the text placement and the four measurements), the parser and DB about 250, the stylesheet text about 70 (the CSS is written into the SVG), and about 150 lines of tests (the 3 specs and the layout checks). roughjs is extra if `handDrawn` is in scope; it was not sized.

### 8.3 Open questions for the coordinator

1. **handDrawn.** Port roughjs (a library with its own generators; the seed must be the same), or exclude `look: handDrawn` from the renderer for now, as the harness does? The default seed 0 is random in the reference, so no differential can test it without a seed.
2. **`useMaxWidth`.** The schema says false and the reference's effective value is true. Follow the effective value (true), which the renders show, and fix the schema or the note? The port's default must be true either way.
3. **The configuration differential.** `pnpm port:diff config` covers pie, flowchart, timeline, journey, gitGraph, ER, class, sequence, state and gantt (`tools/port/README.md`), not ishikawa or quadrantChart. Should `ishikawa` be added, so that `useMaxWidth` (section 6.7) is compared, with its defaults traced?
4. **The `getBBox` rule.** The text port needs a box for a drawn `text` with `text-anchor`, the three baselines and `tspan` lines, on the 1/64 grid. This is the same kind of measurement as the M5 `calculateTextDimensions`, but for elements. Is there a spike to extend, or a new one (in the style of `docs-port/decisions/0005`)?
5. **The layout font** (section 6.4). Keep the two font sizes (stylesheet 14, layout 16 in redux-color) as the reference has them. The port must then read the theme's `fontSize` for the stylesheet and the configuration's for the layout, and test both.
6. **The `Diagram` DB instance** (section 3.4). Confirm that the port creates one DB per diagram, and that the reference's `get db()` is read once. A probe on `Diagram.ts` would close this.
