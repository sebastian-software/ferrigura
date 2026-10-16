# Labels with htmlLabels false: from label text to SVG text and its size

Research notes for M5 (flowchart labels). Read-only research.

- Reference: the JS in this worktree, `packages/mermaid/src/`. Paths below are relative to that folder unless they start with `/` or `crates/` or `docs-port/`.
- Port side: `crates/ferrigura-text/src/`.
- **Confirmed**: read in code. Cited as `file:line`.
- **Probe**: run with Node 24, `marked@16.3.0` and `ts-dedent@2.2.0` from `node_modules/.pnpm`, using a verbatim copy of the function bodies from `handle-markdown-text.ts`. No repository file was written.
- **Derived**: follows from the code plus SVG 1.1 text rules. Not checked in a browser.
- **Unverified**: could not be confirmed. Listed in section 15.

Corrections to the brief:

- `rendering-util/utils.ts` does not exist. The helpers it names (`decodeEntities`, `encodeEntities`) are in `src/utils.ts:874`, `:904`.
- This fork has no dagre. `render.ts:110-112` says so. The default layout is `elk` (`schemas/config.schema.yaml:76-82`, `diagrams/flowchart/flowRenderer-v3-unified.ts:35`).
- The flowchart default look is **neo**, not classic: `schemas/config.schema.yaml:2456-2460` (theme `redux-color`, look `neo`), resolved onto the top level by `config.ts` `resolveAppearance` (`:79-107`). Neo is the primary look here; classic is the second. Section 16 lists every place where they differ. handDrawn is out of scope.

---

## 1. Summary

- Width limits differ by element: node 120 (`flowchart.wrappingWidth`), edge 200 (`createText` default), plain cluster title never wraps, markdown cluster title wraps at the cluster's layout width at paint (section 4; the paint-time width is unverified, section 15).
- A label of N lines has baselines at `S·(1 + 1.1·i)` and a text box of height `1.1·S·(N−1) + A + D`, where `A`, `D` are the rounded ascent and descent (section 6).
- The measured box is `<text>` for nodes and clusters, and the label group (box plus 2 px per side) for edges (section 11).
- A node label's centre sits `bbox.y` (= `S − A`) below the node centre, because its transform uses only `bbox.height` (section 11, `rendering-elements/shapes/util.ts:147`).
- In the SVG path, marked 16 drops codespan, link, del, escape, image and hard-break tokens, and keeps block tokens as raw source (section 7, probe).
- ferrigura-text has measurement, rounded metrics and a one-face wrapper. The gaps are listed in section 14.
- Neo (the flowchart default) changes only the padding around a label, per shape, and the node's CSS and edge dashes. Label creation, label measuring, edge labels and cluster frames have no look branch. The default `[text]` shape (squareRect) is 16 × 12 in neo and 30 × 15 in classic (section 16).

---

## 2. Configuration that shapes labels

| Setting                         | Default and where                                                                                | Read where                                                                                                                                                                     | Effect with `htmlLabels: false`                                               |
| ------------------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| `htmlLabels` (top level only)   | no schema default (`schemas/config.schema.yaml:267-270`); `undefined` (`defaultConfig.ts:24`)    | `config.ts:324-325` `evaluate(config.htmlLabels ?? true)`; `shapes/util.ts:61` `node.useHtmlLabels \|\| evaluate(getConfig()?.htmlLabels)`                                     | SVG path everywhere. No `flowchart.htmlLabels` key is read (grep).            |
| `flowchart.wrappingWidth`       | 120 (`schemas/config.schema.yaml:2537`)                                                          | `shapes/util.ts:98`                                                                                                                                                            | Node label wrap width, when `node.width` and `node.wrappingWidth` are unset   |
| `flowchart.minNodeWidth`        | 120 (`schema:2547`)                                                                              | `diagrams/flowchart/flowDb.ts:1099` sets `node.minWidth`; `shapes/util.ts:92`                                                                                                  | Label area is at least this wide for leaf nodes                               |
| `flowchart.padding`             | 15 (`schema:2529`)                                                                               | `flowDb.ts:1071` `padding: config.flowchart?.padding \|\| 8`                                                                                                                   | `node.padding` (shape padding, cluster padding)                               |
| `flowchart.subGraphTitleMargin` | top 0, bottom 0 (`schema:2475-2477`)                                                             | `utils/subGraphTitleMargins.ts:12-14`                                                                                                                                          | Cluster title y (top), edge label y (top+bottom over 2)                       |
| `markdownAutoWrap`              | true (`schema:446-448`)                                                                          | `handle-markdown-text.ts:19-24` (branch commented out); `:125-127` (HTML path only)                                                                                            | No effect on the SVG path                                                     |
| `fontSize`                      | 16 (`schema:444-445`), set on the root by `styles.ts:44`                                         | em in every `dy`/`y` (section 6)                                                                                                                                               | `S` = 16 px by default                                                        |
| `look`                          | flowchart default `neo` (`schema:2458-2460`); base default `classic` (`schema:68-70`, root only) | `config.ts:79-107` `resolveAppearance`, reading the diagram section before the top level (`readAppearance` `:42-53`); `flowDb.ts:1287` falls back to `classic` only when unset | Padding around labels per shape (section 16); edge dashes; CSS only otherwise |
| `.label` font family            | theme `fontFamily` (`flowchart/styles.ts:143-146`)                                               | Used by the measuring `<text>` in the same group                                                                                                                               | The port uses its bundled face (ADR 0005)                                     |

---

## 3. From flowchart text to labelType

Grammar: `diagrams/flowchart/parser/flow.jison`.

| Source form                        | Grammar rule                                                                                     | labelType             | Path             |
| ---------------------------------- | ------------------------------------------------------------------------------------------------ | --------------------- | ---------------- |
| `A[text]`, unquoted text           | `text: textToken` `flow.jison:506`                                                               | `text`                | plain            |
| `A["text"]` (quoted)               | `STR` `flow.jison:510`                                                                           | `string`              | plain            |
| ``A["`md`"]``                      | `MD_STR` (lexer `flow.jison:82`, `[^`"]+`) `:512`                                                | `markdown`            | markdown         |
| `-- text -->`                      | `edgeText` `:484`                                                                                | `text`                | plain            |
| `-- "x" -->` / `"`md`"` on an edge | `:488` / `:490`                                                                                  | `string` / `markdown` | plain / markdown |
| `subgraph id [title]`              | `text` `:506-512`                                                                                | per token             | per token        |
| `subgraph id` with no brackets     | `textNoTags` `:522-528`; a quoted string here is `text` `:526`                                   | `text`                | plain            |
| Implicit node (no label)           | `flowDb.ts:190` default `text`; text = id `:209-210`                                             | `text`                | plain            |
| Edge or subgraph with no type      | `sanitizeNodeLabelType(undefined)` gives `markdown` (`flowDb.ts:90-100`, used at `:307`, `:745`) | `markdown`            | markdown         |

Rules:

- Only `labelType === 'markdown'` takes the marked path: `shapes/util.ts:89`, `rendering-elements/edges.js:86`, `clusters.js:41`. `string` and `text` both take the plain path.
- Node vertex labels take the parser's type unchanged (`flowDb.ts:202`). Edge text, subgraph title and doc labels go through `sanitizeNodeLabelType` (`:247`, `:307`, `:745`).

Sanitising before render:

- At parse time, `flowDb.ts:86-88` calls `common.sanitizeText` on node text (`:201`), edge text (`:301`) and subgraph title (`:736`). With `htmlLabels: false`, `sanitizeMore` is skipped (`diagrams/common/common.ts:56-58`), so only DOMPurify runs (`common.ts:71-83`, `FORBID_TAGS: ['style']`).
- At render time, node labels are sanitised again: `shapes/util.ts:95` `sanitizeText(decodeEntities(label))`. Edge labels (`edges.js:88-101`) and cluster labels (`clusters.js:49`, `createLabel.js`) are passed raw.
- Entity placeholders: `Diagram.ts:77` runs `encodeEntities` on the source (`utils.ts:874`). `decodeEntities` (`utils.ts:904-905`) turns `ﬂ°°`→`&#`, `ﬂ°`→`&`, `¶ß`→`;` back.

---

## 4. Which call draws which label

| Label                                   | Call (file:line)                                                   | markdown                                        | Width used to wrap (px)                                                                                                                                                                          | Wraps?                   | Horizontal anchor                                                                                                                 | Background                            | Measured element                                                       |
| --------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------ | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------- |
| Leaf node                               | `labelHelper` `shapes/util.ts:50-154`, `createText` call `:93-106` | `labelType === 'markdown'` (`:89`, `:103`)      | `node.width \|\| node.wrappingWidth \|\| flowchart.wrappingWidth` (`:98`) = 120                                                                                                                  | Always                   | CSS `.node .label text { text-anchor: middle }` (`flowchart/styles.ts:171-172`); no attribute                                     | Only with icon or img (`:88`, `:102`) | `<text>` (`:135-141`)                                                  |
| Edge label                              | `insertEdgeLabel` `edges.js:74-137`, `createText` `:88-101`        | `labelType === 'markdown'` (`:86`, `:96`)       | `width: undefined` (`:98`), so `createText` default `width = 200` (`createText.ts:319`)                                                                                                          | Always, both kinds       | `text-anchor: middle` attribute on `<text>` and tspans (`createText.ts:108-110`, `:163-165`, from `centerText = !isNode`, `:375`) | Always (`addSvgBackground: true`)     | Label group (`edges.js:122`) for size; `<text>` (`:124`) for transform |
| Cluster title, plain                    | `rect()` `clusters.js:49` → `createLabel` `createLabel.js:31-53`   | `markdown: false` (`createLabel.js:47`)         | `Infinity` (`createLabel.js:49`)                                                                                                                                                                 | Never                    | Start (no rule)                                                                                                                   | None                                  | `<text>` (`clusters.js:53`)                                            |
| Cluster title, markdown                 | `rect()` `clusters.js:41-47`                                       | default `markdown = true` (`createText.ts:314`) | `node.width` (`clusters.js:46`); if undefined, the destructuring default 200 applies (`createText.ts:319`)                                                                                       | Yes                      | Start                                                                                                                             | None                                  | `<text>` (`:53`)                                                       |
| Cluster measure before ELK layout       | `measureGroupLabel` `createGraph.ts:42-59` → `labelHelper`         | as above                                        | ELK passes `unwrapGroupLabels: true` (`elk/render.ts:757-758`): plain gets `Infinity` (`createGraph.ts:170-171`); markdown gets `node.width \|\| wrappingWidth` = 120 (no `width` before layout) | Plain: no. Markdown: yes | CSS middle (in `.node`)                                                                                                           | None                                  | `<text>` (`shapes/util.ts:135-141`)                                    |
| Terminal labels (`startLabelLeft` etc.) | `createLabel` `edges.js`                                           | false                                           | `Infinity`                                                                                                                                                                                       | Never                    | Start                                                                                                                             | None                                  | n/a for flowcharts                                                     |

Notes:

- The measure pass and the paint pass build the cluster title twice. Only the measure result (`labelBBox`) feeds ELK (section 12).
- `createText` destructuring defaults apply to `undefined`, so `width: undefined` (edges, markdown clusters with no `node.width`) means 200 (`createText.ts:319`).

---

## 5. createText, SVG path: pseudo-code

```
createText(el, text, {markdown=true, isNode=true, width=200, addSvgBackground=false, style, ...}):   # createText.ts:306-325
  # useHtmlLabels is false here; HTML branch is out of scope (:340-363)
  sanitizeBR = decodeEntities(text with /<br\s*\/?>/g replaced by "<br/>")                         # :366
  lines = markdown ? markdownToLines(sanitizeBR.replace("<br>", "<br/>")) : nonMarkdownToLines(sanitizeBR)   # :367-369
  svg = createFormattedText(width, el, lines, addBackground = text ? addSvgBackground : false, centre = !isNode)   # :370-376
  node  -> style on <text> = style minus stroke/fill, color->fill (:377-388)                         # font-size/weight from style stays
  edge  -> style on rect and text (:389-410)
  tspan.text-outer-tspan gets class "row" or "title-row" (:411-416; no size effect)
  return svg                                                                                       # :417

createFormattedText(width, g, lines, addBackground, centre):                                       # :152-202
  labelGroup = g.append("g")
  bkg = labelGroup.insert("rect").class("background").style("stroke: none")                          # :160-161 (first child)
  textEl = labelGroup.append("text").attr("y", "-10.1")                                              # :162 (inert, see section 6)
  if centre: textEl.attr("text-anchor", "middle")                                                    # :163-165
  lineIndex = 0
  for line in lines:                                                                                 # :166-181
    prepared = checkWidth(line) ? [line] : splitLineToFitWidth(line, checkWidth)                      # :172-174
    for p in prepared:
      tspan = createTspan(textEl, lineIndex, 1.1, centre); setWords(tspan, p); lineIndex++            # :176-180
  if addBackground:                                                                                  # :182-198
    b = textEl.getBBox(); pad = 2
    bkg.attr(x = b.x-2, y = b.y-2, width = b.width+4, height = b.height+4)                           # :187-196
    return labelGroup                                                                                # :198
  return textEl                                                                                      # :200

checkWidth(line) = computeWidthOfText(labelGroup, 1.1, line) <= width                                # :172-173
computeWidthOfText: append <text><tspan>, setWords, getComputedTextLength(), remove                   # :114-125

createTspan(textEl, i, lh, centre):                                                                  # :96-112
  <tspan class="text-outer-tspan" x="0" y="(i*lh - 0.1)em" dy="lh em" [text-anchor=middle]>          # :104-110

setWords(tspan, words):                                                                              # :237-256
  for k, word in words:
    <tspan class="text-inner-tspan" font-style=(em ? italic : normal) font-weight=(strong ? bold : normal)>   # :246-248
    text = (k == 0 ? "" : " ") + decodeHTMLEntities(word.content)                                    # :249-254
decodeHTMLEntities: replace /&(amp|lt|gt);/g only                                                    # :213-228
```

---

## 6. Line geometry

Source: `createText.ts:106-107` (y, dy), `:162` (text y), `:159` (lh = 1.1). Rules from SVG 1.1: an absolute `y` on a tspan sets the baseline; `em` resolves on the tspan's font size; `dy` is then added.

- Baseline of line i, in the label group's coordinates (**derived**):
  `b_i = (1.1·i − 0.1)·S + 1.1·S = S·(1 + 1.1·i)`. For S = 16: b0 = 16, b1 = 33.6, b2 = 51.2.
- The `<text y="-10.1">` is overridden by the first tspan's absolute `y` (**derived**, unverified in a browser).
- Text box of the `<text>` element from `getBBox` (rounding confirmed by the spike; multi-line composition unverified, see below):
  - `top = b0 − A`, `bottom = b_{N−1} + D`.
  - `A` = round(ascent), `D` = round(descent), each rounded to whole pixels on its own, a tie going down (28.5 becomes 28), at the size floored to 1/100 px (`docs-port/spikes/text-measurement.md:30`; `crates/ferrigura-text/src/metrics.rs`; Chromium evidence in `crates/ferrigura-text/tests/fixtures/chromium.json`, boxes).
  - `H = 1.1·S·(N−1) + A + D`; `top y = S − A`.
  - Width `W = max_i w_i`; with per-line anchors the box is `[−W/2, W/2]` (middle) or `[0, W]` (start).
- The spike reports box height and top as exact after rounding (`spikes/text-measurement.md:22-23`). It does not say whether any of its boxes were multi-line, so the formula above is **unverified** for N > 1.

---

## 7. Markdown path (marked 16.3.0), SVG use

Preprocess (`handle-markdown-text.ts:12-26`):

1. `<br/>` → newline (`:14`).
2. `\n{2,}` → single newline (`:16`). Blank lines are removed.
3. `ts-dedent` (`:18`). For a string argument it removes a leading newline, a trailing newline, and the common indentation after newlines (`ts-dedent/dist/index.js`). It does not trim spaces.
4. `markdownAutoWrap` branch is commented out (`:19-24`).

Then `marked.lexer`, and `processNode` (`:52-74`) walks the tree:

- Top-level `paragraph`: each inline token goes to `processNode` (`:77-80`).
- Top-level `html`: pushed as one word (`:81-82`).
- Any other top-level block: `treeNode.raw` pushed as one word (`:83-85`).
- In `processNode`: `text` splits on newline (new line per piece, `:54-58`) and on space (`:60-63`); empty words dropped; `&#39;`→`'` (`:61`).
- `strong` and `em` recurse with their type (`:67-70`). The innermost type wins (probe).
- Inline `html`: pushed as one word (`:71-73`).
- No branch for other inline types, so they are dropped.

Probe results (`marked@16.3.0`, verbatim logic):

| Input                                     | Result                    | Rule                                                             |
| ----------------------------------------- | ------------------------- | ---------------------------------------------------------------- |
| `plain  text` (two spaces)                | `plain`, `text`           | Spaces collapse (`:60-63`)                                       |
| `a\nb`                                    | two lines                 | Soft break (`:54-58`)                                            |
| `x<br/>**y**`                             | `x` / `y` (strong)        | `<br/>` becomes newline before marked (`:14`)                    |
| `line one<br/><br/>line three`            | two lines                 | `\n{2,}` collapses (`:16`)                                       |
| `a  ⏎b` (two trailing spaces, hard break) | one line: `a b`           | Inline `br` token is dropped (no branch)                         |
| `**a *b* c**`                             | a:strong, b:em, c:strong  | Innermost type (`:67-70`)                                        |
| `***x***`                                 | x:strong                  | Innermost type                                                   |
| `__u__ _e_`                               | u:strong, e:em            | Underscore forms                                                 |
| ``a `code` span``                         | `a span`                  | **codespan dropped** (no branch)                                 |
| `a [l](u) here`                           | `a here`                  | **link dropped, text too**                                       |
| `x ~~d~~ y`                               | `x y`                     | **del dropped, text too**                                        |
| `a \*esc\* star`                          | `a escaped star`          | **escape dropped, the asterisks too**                            |
| `x ![alt](i.png) y`                       | token `image`, dropped    | **image dropped** (token seen; no branch)                        |
| `&amp;`, `&lt;c&gt;`                      | kept as literal text      | Restored later by `decodeHTMLEntities` (`createText.ts:213-228`) |
| `__a__ &amp; b`                           | a:strong, `&amp;`, b      | As above                                                         |
| `# Heading text`                          | one word `# Heading text` | Block `heading` gives raw (`:83-85`)                             |
| `- a\n- b`                                | one word `- a\n- b`       | Block `list` gives raw; newline is inside the word               |
| fenced code, `> quote`, table, `---`      | raw source as one word    | Block tokens give raw (**probe** token types; output by code)    |

Differences to keep in mind:

- `markdownToHTML` (`:120-147`, HTML path) handles `escape` (`:139-140`) and logs `Unsupported markdown` for others (`:142-143`), emitting raw. The SVG path drops codespan, link, escape. The two paths disagree on the same input.
- A raw block token is one unbreakable word. It is split only by graphemes if wider than the limit (section 10).

---

## 8. Plain path (nonMarkdownToLines)

`handle-markdown-text.ts:33-41`:

```
lines = text.split(/\\n|\n|<br\s*\/?>/gi)                                  # :34  (literal backslash-n counts as a break)
for each line: tokens = line.trim().match(/<[^>]+>|[^\s<>]+/g) ?? []     # :35-39
  # a "<...>" token may contain spaces and is one unbreakable word
  # lone "<" and ">" are dropped; \s includes tab and NBSP
```

Probe results (`createText` sanitises with `<br\s*\/?>` → `<br/>` first, `:366`):

| Input                                        | Lines (tokens)                                                     |
| -------------------------------------------- | ------------------------------------------------------------------ |
| `a<br/><br/>b`                               | `[a]`, `[]`, `[b]`: the empty middle line is kept                  |
| `trailing<br/>`                              | `[trailing]`, `[]`                                                 |
| `a<br>b`, `a<br />b`                         | `[a]`, `[b]`                                                       |
| `a\nb`, `a\\nb`                              | two lines (both)                                                   |
| `a < b`, `a > b`                             | `[a, b]`: lone `<`/`>` dropped                                     |
| `x<y>z`                                      | `[x, <y>, z]`                                                      |
| `x < y and a > b` (raw `<` with a later `>`) | `[x, "< y and a >", b]`: the span between `<` and `>` is one token |
| `a` TAB `b`                                  | `[a, b]`                                                           |
| `a` NBSP `b`                                 | `[a, b]`                                                           |
| `it&#39;s &amp; co`                          | `[it&#39;s, &amp;, co]` (`&amp;` is decoded later, `&#39;` is not) |
| `""`, `"   "`                                | `[[]]`: one empty line                                             |

Consequences:

- A plain label keeps empty lines (`createFormattedText` keeps an empty line as an empty tspan; section 9).
- Plain labels use `\s`, so tab and NBSP separate words. The markdown path splits on space only (`:61`).
- `&#39;` and `&#35;` are not decoded here. Node labels get DOMPurify first (`shapes/util.ts:95`), which decodes them. Edge and cluster labels do not (**derived**: edge text is only sanitised at parse, `flowDb.ts:301`).

---

## 9. Spacing, empty lines, entities

- Every word after the first in a line is emitted with one leading space, in its own inner tspan, styled as that word (`createText.ts:249-254`). The space therefore takes the word's font weight and style. `ferrigura-text/src/measure.rs` (`measure_line` doc) says the same.
- `checkWidth([])` is `0 <= width`, so an empty line is kept as `[[]]` and gets its own `lineIndex` (`createText.ts:172-180`).
- Markdown never produces an empty line from blank input (`:16`), so `[[]]` is only produced for an empty label (`""` gives one empty line, probe).
- Whether a trailing empty line adds a box to `getBBox` is **unverified**.
- The cosmetic `title-row` / `row` classes (`createText.ts:411-416`) do not change size.
- Label-level `style` (for example `font-size` or `font-weight` from a classDef) is set on the `<text>` only after wrapping (`createText.ts:377-410`), so the wrap decision does not see it there. For nodes the wrap does see it, because the `.label` group carries `node.labelStyle` (`shapes/util.ts:78`, set from `drawRect.ts:17`). For edges and clusters it does not (**derived**). The final box always uses the style on `<text>`. The size `S` and face are therefore per label and per element.

---

## 10. Wrapping

`splitText.ts` (`splitLineToFitWidth`, `splitWordToFitWidth`) with `createFormattedText`:

```
split_line(words, fits):                            # splitText.ts:90-135
  queue = words; cur = []; lines = []
  while queue:
    w = queue.pop_front()
    if fits(cur + [w]):                             # :116; "fits" = tspan width of the candidate <= width  (createText.ts:172-173)
      cur.push(w); continue
    if cur not empty:                               # :122-125
      lines.push(cur); cur = []; queue.push_front(w)
    else:                                           # :126-133
      (first, rest) = split_word(fits, w)
      lines.push([first]); if rest: queue.push_front(rest)
  if cur not empty: lines.push(cur)                 # :97-103
  return lines

candidate width = width(first word) + sum(width(" " + w_k)) for k > 0    # createText.ts:244-254; leading space in w_k's face

split_word(fits, w):                                # splitText.ts:38-72
  used = []
  for g in graphemes(w):                            # splitTextToChars, :6-11 (Intl.Segmenter)
    if fits(used + [g]): used.push(g)              # :58-61
    else:
      if used empty: used.push(g)                  # :63-66 first grapheme is kept even if too wide
      break
  return (join(used), join(rest of graphemes))
```

Joiner code (`splitText.ts:104-113`) is dead for the current producers: no token is exactly `' '` (spaces are split out in both paths).

Comparison with `ferrigura-text/src/wrap.rs`:

| Point                                      | JS (reference)                                                                              | wrap.rs                                | Status                                 |
| ------------------------------------------ | ------------------------------------------------------------------------------------------- | -------------------------------------- | -------------------------------------- |
| Greedy packing, `<=` limit                 | `splitText.ts:116`, `createText.ts:173`                                                     | `wrap.rs:50`                           | Same                                   |
| Candidate width                            | tspan per word, leading space in the word's face                                            | `wrap.rs:48`, one face                 | **Gap**: styled words (STATUS.md:120)  |
| Grapheme split, first grapheme always kept | `:58-66`                                                                                    | `wrap.rs:59-68`, `used.max(1)` (`:66`) | Same                                   |
| Word separator                             | markdown: space only (`handle-markdown-text.ts:60`); plain: `\s` and `<...>` tokens (`:38`) | `wrap.rs:35` splits on `' '` only      | **Gap**: tab, NBSP and `<...>` tokens  |
| Empty line                                 | kept as one empty line (`createText.ts:172-180`)                                            | `wrap_line("")` returns `[]`           | **Gap**: caller must add an empty line |
| Lines joined with space                    | tspans                                                                                      | `wrap.rs:54` and `:75` join with `" "` | Same for one face                      |

Practical rule: `wrap_line` must take tokens from the tokenizer (section 7 or 8), not a raw string.

---

## 11. Size taken from the reference

| Element                                                            | Measured element                                                         | What the box contains                                                      | Size written                                                                                                                                                        | Transform                                                                                                                           | Source                   |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------ | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| Leaf node label                                                    | `<text>` (createText returns it when no background, `createText.ts:200`) | All tspans; no padding                                                     | `bbox` = text box, widened to `minWidth` (`withMinWidth`, `shapes/util.ts:26-48`, applied at `:140`)                                                                | `translate(0, −bbox.height/2)` (`util.ts:147`)                                                                                      | `util.ts:135-147`        |
| Icon or img node label                                             | label group with background rect                                         | Text box plus 2 px each side (`createText.ts:191-196`)                     | bbox height +4                                                                                                                                                      | `translate(0, −(H+4)/2)`                                                                                                            | `util.ts:88,102,135-147` |
| Node (whole shape)                                                 | outer `<g class="node">`                                                 | Shape rect and label                                                       | `node.width/height = getBBox` (`createGraph.ts:66-69`), overwriting any earlier value                                                                               | n/a                                                                                                                                 | `createGraph.ts:61-71`   |
| Shape size, `squareRect` (default `[text]`, `flowDb.ts:1011-1014`) | n/a                                                                      | label box plus X and Y padding (`squareRect.ts:8-10`, `drawRect.ts:21-22`) | Neo: `max(bbox.w + 32, node.width)`, `max(bbox.h + 24, node.height)`. Classic: `max(bbox.w + 60, node.width)`, `max(bbox.h + 30, node.height)`. Both at padding 15. | n/a                                                                                                                                 | `drawRect.ts:21-22`      |
| Edge label                                                         | Label group (`edges.js:122`) for size; `<text>` (`:124`) for transform   | Text box plus 2 px each side (`createText.ts:191-196`)                     | `edge.width = W+4`, `edge.height = H+4` (`edges.js:136-137`)                                                                                                        | `translate(−(x+W/2), −(y+H/2))` with `x = −W/2` (middle), so `translate(0, −(S−A+H/2))` (`edges.js:130`, `labelTransform.ts:31-33`) | `edges.js:121-137`       |
| Cluster title (measure)                                            | `<text>`                                                                 | Text box, no padding                                                       | `node.labelBBox = {width: W, height: H}` (`createGraph.ts:54`)                                                                                                      | n/a                                                                                                                                 | `createGraph.ts:50-54`   |
| Cluster title (paint)                                              | `<text>` (`clusters.js:53`)                                              | Text box, no padding                                                       | Used for layout inside `rect()`                                                                                                                                     | `translate(node.x − W/2, node.y − node.height/2 + topMargin)` (`clusters.js:109-113`)                                               | `clusters.js:53-113`     |

Why the node label sits low (**derived**): `translate(0, −H/2)` moves the origin so that the box centre lands at `bbox.y` (= `S − A`) below the node centre. For `S = 16` and an ascent that rounds to 14, that is 2 px. The exact value depends on the font's rounded ascent.

Edge labels are the exception: the transform centres the text box itself (`labelTransform.ts:31-33`), so the label sits centred on the edge label point.

ELK consumer (`elk/render.ts:1068-1082`): cluster label `width = labelBBox.width`, `height = max(0, labelBBox.height − 2)` (`:1074-1077`); the −2 has no comment (see section 15). Edge label: `width: edge.width ?? 0`, `height: edge.height ?? 0` (`:1121-1124`).

---

## 12. Measure step (createGraph, cloneLayoutDataForMeasure)

Pipeline (`layout-algorithms/common/index.ts`): prepare → **measure** (`:166-168`) → core layout (`:173`) → paint (`:191`, `paintLayoutData` `:220`).

`createGraphWithElements` (`createGraph.ts:122-215`), when a DOM exists (`:156`):

| Step              | What is inserted into the DOM                                                                         | What is written back to LayoutData                                                                     | Lines                |
| ----------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | -------------------- |
| Cluster (isGroup) | `labelHelper` builds `g.node.default > g.label > <text>` in the nodes group, then `shapeSvg.remove()` | `node.labelBBox = {width, height}` on the original node (`:54`). **Does not** set `node.width/height`. | `:162-172`, `:42-59` |
| Leaf node         | Full shape via `insertNode` into the nodes group; kept (`nodeElements`, `:180`)                       | `node.width`, `node.height` = outer `g` `getBBox` (`:66-69`); overwrites any prior value               | `:175-181`, `:61-71` |
| Edge with a label | `insertEdgeLabel` into the edge-labels group; kept (`edgeLabels.set`, `edges.js:133`)                 | `edge.width`, `edge.height` (`edges.js:136-137`); `edge.labelStyle` (`:78`)                            | `:187-191`           |
| All               | `graph.setNode`/`setEdge` store copies (`:173`, `:182`, `:192`)                                       | Nothing else                                                                                           | `:138-141`           |

- `unwrapGroupLabels` only changes plain cluster labels to `Infinity` (`:170-171`), and only ELK sets it (`elk/render.ts:757-758`).
- ELK's layout data is mutated in place. `cloneLayoutDataForDomMeasure` (`cloneLayoutDataForMeasure.ts:7-24`) is used only by DDLT (`layout-algorithms/ddlt/index.ts:36-37`), not the flowchart ELK path.
- Paint re-creates cluster frames (`insertCluster`, via `paintLayoutNode` at `common/index.ts:247`), which re-measures the title inside `rect()`. Leaves are only positioned (`positionNode`).
- Paint writes the painted rect size back: `node.width = rectBox.width`, `node.height = rectBox.height` (`clusters.js:124-126`), and `node.offsetY` (`:128`).

---

## 13. Cluster (subgraph) rect, both looks

`rect()` (`clusters.js:15-135`). Clusters use shape `rect` by default (`clusters.js:841-846`, `flowDb.ts:1092`, `:1266`). Neo and classic give the same geometry: `rect()` reads `node.look` only for the `data-look` attribute (`clusters.js:28`) and the handDrawn branch. The neo branch at `clusters.js:478-481` sits in `divider` and both of its arms set class `divider`, so it changes nothing (section 16).

```
rect(node):
  label = labelType === "markdown"
            ? createText(markdown, width = node.width)     # clusters.js:41-47 (200 if node.width undefined)
            : createLabel(plain, width = Infinity)         # clusters.js:49; createLabel.js:49
  W = textBox.width; H = textBox.height                     # clusters.js:53 (no padding)
  width = node.width <= W + node.padding ? W + node.padding : node.width        # :63
  diff  = node.width <= W + node.padding ? (width − node.width)/2 − node.padding : −node.padding   # :64-69
  height = node.height                                       # :71
  x = node.x − width/2 ; y = node.y − height/2               # :70-72
  rect.attr(x, y, width, height)
  labelEl.transform = translate(node.x − W/2, node.y − node.height/2 + subGraphTitleTopMargin)   # :108-113
  rectBox = rect.getBBox(); node.width = rectBox.width; node.height = rectBox.height             # :124-126
  node.offsetY = H − node.padding/2                          # :128
```

Padding: `node.padding = flowchart.padding` (default 15, `flowDb.ts:1071`). The rect grows to `W + padding` when the layout width is narrower than that (`:63`).

Title margins (`utils/subGraphTitleMargins.ts:12-14`):

- Top: moves the title only (`clusters.js:108-112`, also `:391-395`, `:619-622`, `:775-784` in other cluster shapes).
- Bottom: no use in cluster frames. Top plus bottom divided by 2 shifts edge label y (`edges.js:304-319`, `layout-algorithms/common/index.ts:328-337`).
- Defaults are 0, so none of this moves anything by default.

`node.offsetY` is written in several shapes and read nowhere in `packages/mermaid/src` (grep, non-spec). Treated as dead for this fork (**derived**).

Cluster width limits:

| Case                                              | Wrap width                    | Wraps                                                                                                     | Source                                    |
| ------------------------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| Plain title, paint                                | Infinity                      | No                                                                                                        | `createLabel.js:49`                       |
| Plain title, measure, ELK                         | Infinity                      | No                                                                                                        | `createGraph.ts:170-171`                  |
| Plain title, measure, default measure (no unwrap) | 120                           | Yes. The result (`labelBBox`) is read by ELK (`elk/render.ts:1074`) and by no other flowchart path (grep) | `shapes/util.ts:98`, `createGraph.ts:171` |
| Markdown title, paint                             | `node.width` if set, else 200 | Yes                                                                                                       | `clusters.js:46`, `createText.ts:319`     |
| Markdown title, measure                           | 120 (no width before layout)  | Yes                                                                                                       | `shapes/util.ts:98`                       |

---

## 14. ferrigura-text: what exists, what the Rust side must add

Existing (`crates/ferrigura-text/src/`):

| Function                           | File:line       | What it gives                                                            |
| ---------------------------------- | --------------- | ------------------------------------------------------------------------ |
| `measure(face, text, size)`        | `measure.rs:26` | Advance width of one shaped run (kerning and ligatures inside the run)   |
| `measure_line(runs, size)`         | `measure.rs:54` | Sum of per-run widths; each run should start with its leading space      |
| `wrap_line(text, face, size, max)` | `wrap.rs:28`    | Greedy wrap by spaces; one face; graphemes for long words                |
| `font_metrics(face, size)`         | `metrics.rs:42` | Unrounded ascent, descent, line gap                                      |
| `text_box(face, size)`             | `metrics.rs:64` | Rounded `ascent`, `descent`, `top = ascent`, `height = ascent + descent` |

Gaps (what the Rust side must compute):

| Need                            | Reference                                                                                                | Port today                                                                                    | To add                                                                                                                             |
| ------------------------------- | -------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Wrap styled words               | Per-word tspan with its own face (section 10)                                                            | One face (`wrap.rs:28`); STATUS.md:120                                                        | Wrap over runs, leading space in the word's face                                                                                   |
| Tokens                          | Markdown: space split; plain: `\s`, `<...>` (section 7, 8)                                               | Space split only (`wrap.rs:35`)                                                               | Tokenizer output as input to wrap                                                                                                  |
| Empty line                      | Kept (section 9)                                                                                         | `wrap_line("")` returns `[]` (`wrap.rs:28-74`)                                                | Emit one empty line                                                                                                                |
| Multi-line box                  | `H = 1.1·S·(N−1) + A + D`, baselines `S(1+1.1i)` (section 6)                                             | Single-line only                                                                              | Add this formula; keep `A`, `D` from `text_box`                                                                                    |
| Per-line x                      | Node and edge: `x_i = −w_i/2` (CSS or attribute middle); cluster: 0                                      | None                                                                                          | Emit `text-anchor="start"` and explicit `x` (ADR 0005:17)                                                                          |
| Edge background                 | Box plus 2 px each side (`createText.ts:191`)                                                            | None                                                                                          | `edge.width = W+4`, `height = H+4`                                                                                                 |
| Min node width                  | Box widened to 120 (`util.ts:26-48`)                                                                     | None                                                                                          | `max(W, 120)` for the leaf label box                                                                                               |
| Markdown                        | marked 16 quirks (section 7) vs Ferromark                                                                | None                                                                                          | Decision needed (section 15, item 1)                                                                                               |
| Entities                        | `&amp; &lt; &gt;`, `&#39;`, placeholders (section 3, 7)                                                  | None                                                                                          | Implement the same decode steps                                                                                                    |
| Sanitise                        | DOMPurify for node labels (`util.ts:95`), and for node, edge and title texts at parse time (`flowDb.ts`) | Ported as a subset: `ferrigura-syntax/src/sanitize.rs`, applied in the DB and in `node_label` | The subset and what it leaves out are in the module documentation; the reference's output is checked in `port:diff flowchart-data` |
| Label style in wrap measurement | Node: inherited from the `.label` group (`shapes/util.ts:78`); edge and cluster: not seen (section 9)    | Not modelled                                                                                  | Take `S` and face per element, and apply the same rule for wrap and box                                                            |
| Look-dependent shape padding    | Neo and classic differ per shape (section 16)                                                            | None                                                                                          | Take `look` per diagram (or per node, `flowDb.ts:1287`) and implement both branches for each shape the port draws                  |
| Bold and italic metrics         | Chromium takes the union over glyph boxes of all runs                                                    | `text_box` uses one face                                                                      | Verify bold and italic `A`, `D` (section 15)                                                                                       |

Pseudo-code for the port (derived from sections 6, 11):

```
label_box(lines, S, anchor, faces):
  N = max(1, len(lines))
  for line in lines: w_i = sum(measure(run) for run in line)         # measure_line
  A, D = text_box(regular, S).ascent, .descent                        # bold/italic: unverified
  H = 1.1*S*(N-1) + A + D
  W = max(w_i)
  x_i = anchor==middle ? -w_i/2 : 0 ;  baseline_i = S*(1 + 1.1*i)
  text_box = {x: min(x_i) (or 0), y: S - A, width: W, height: H}
  node:    box = label box + shape padding (look-dependent, section 16); transform = (0, -H/2)
           # centre sits at text_box.y = S - A below node centre
  edge:    size = (W+4, H+4);  transform = (0, -(S - A + H/2))
  cluster: transform = (cx - W/2, cy - Hc/2 + topMargin)
```

---

## 15. Open questions and unverified items

1. **Markdown tokenizer.** Port uses Ferromark. The reference's SVG path drops codespan, link, del, escape, image and hard break (section 7), and keeps block tokens as raw source. Decide whether to match (the differential harness needs it) or to render the parsed tokens. This is a decision for the port, not a finding.
2. **Multi-line box.** Not verified in Chromium. The spike does not state whether its boxes were multi-line (`spikes/text-measurement.md:21-23`). Needs a fixture with 2 to 3 lines, including one empty line and a trailing empty line.
3. **Bold and italic metrics.** Whether Lexica's bold, italic and bold-italic faces share ascent and descent with regular. If not, the union of glyph boxes uses different `A`, `D`. Not checked; check `crates/ferrigura-text/src/fonts.rs` and the font files.
4. **Markdown cluster title width at paint.** Whether `node.width` is set by ELK before `rect()` runs. If it is, a markdown title wraps at the layout width while it was measured at 120 (section 13). Not verified.
5. **`elk/render.ts:1077`, `labelBBox.height − 2`.** No comment says why. The text box has no padding, so the −2 is unexplained.
6. **Stale comments.** `elk/render.ts:754` says the flowchart wrapping width is 200 px; the schema says 120 (`schema:2537`). `createText.ts:321-323` says `minWidth` applies only to HTML; `shapes/util.ts:140` applies it to SVG.
7. **`htmlLabels: null`.** `shapes/util.ts:61` gives SVG (`evaluate(null)` is false), while `getEffectiveHtmlLabels` gives HTML (`null ?? true`). Only matters if a diagram sets `null`.
8. **Entities in edge and cluster labels.** `&#35;` in an edge label is drawn literally as `&#35;` (derived from sections 3 and 8). In a node label it is drawn as `#` (DOMPurify). Confirm in Chromium before the port copies it.
9. **Empty subgraph title.** A title of `""` gives an empty `<text>`. Whether `getBBox` returns zero height, and therefore whether the title strip is empty, is unverified.
10. **Raw block tokens.** How Chromium lays out a raw token with embedded newlines (`- a\n- b`, probe) is unverified. SVG collapses whitespace, so the newline most likely becomes a space.
11. **`node.offsetY`.** Written in several shapes, read nowhere in `packages/mermaid/src` (grep). Confirm no reader in the ELK path before dropping it.
12. **Label `style` font overrides.** A `classDef` with `font-size` or `font-weight` changes the final box of the label, and for nodes also the wrap (section 9). For edges and clusters the wrap uses the unstyled font. Derived; confirm in Chromium with one fixture per element before the port copies the split.
13. **Dedent.** `ts-dedent` 2.2.0 removes common indentation computed over lines after newlines only (`ts-dedent/dist/index.js`). A first line keeps its own indentation. Not probed.
14. **Which look the reference fixtures use.** The flowchart default is neo (section 16), but the reference harness may set `look` explicitly. The harness config was not read. Confirm before comparing node sizes.
15. **Neo edge markers.** `edges.js:946-947` passes `useMargin` (neo only) to `addEdgeMarkers`. What it changes in `edgeMarker.ts` was not read, so any effect on edge endpoints is unverified.

---

## 16. Neo (primary) and classic: what differs

### 16.1 Which look applies

- Flowchart schema: theme `redux-color` (`schemas/config.schema.yaml:2457`), look `neo` (`:2460`).
- Root schema: look `classic` (`:68-70`), which applies only outside the flowchart section.
- `resolveAppearance` (`config.ts:79-107`) takes each key from the first layer that defines it, in this order: directives and frontmatter, `initialize()` (`siteConfigDelta`), the diagram default, the global default. `readAppearance` (`config.ts:42-53`) reads the diagram section (`flowchart.look`) before the top-level `look` in each layer. It writes the result to top-level `look` and, when the section defines it, to `flowchart.look` (`:95-103`).
- Result: a flowchart with no look setting is neo. A top-level `look: classic` in `initialize()` or frontmatter makes it classic, because that layer is consulted before the defaults layer (**derived** from the layer order).
- `flowDb.ts:1287` falls back to `classic` only when `config.look` is unset, which `resolveAppearance` prevents for flowcharts (**derived**).

### 16.2 Where neo and classic differ

| Area                                                          | Neo vs classic                                                                                  | Source                                           | Changes label size?               |
| ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------ | --------------------------------- |
| Label text (`createText`: wrap, lines, tspans, background)    | No look branch                                                                                  | `createText.ts` (no `look` match)                | No                                |
| Label placement and centring (`labelHelper` transform)        | No look branch                                                                                  | `shapes/util.ts:50-154`                          | No                                |
| Edge label box and placement                                  | No look branch                                                                                  | `edges.js:74-137`, `:296-322`                    | No                                |
| Label measuring (`createGraph`)                               | No look branch; measured node size follows the shape                                            | `createGraph.ts:42-71`                           | Indirectly, through shape padding |
| Node shape padding around the label                           | **Branch per shape** (table 16.3)                                                               | `shapes/*.ts`                                    | **Yes**                           |
| Node edge dashes and marker margin                            | Neo: dash arrays (`edges.js:883-890`), `useMargin` to markers (`:946-947`)                      | `edges.js`                                       | Edge geometry only                |
| Cluster frame `rect()`                                        | Same geometry; `data-look` attribute only                                                       | `clusters.js:15-135`, `:28`                      | No                                |
| Cluster `divider` neo branch                                  | Both arms set class `divider` (no-op)                                                           | `clusters.js:478-481` (inside `divider`, `:419`) | No                                |
| `roundedWithTitle`, `noteGroup`, `kanbanSection`, `flowGroup` | No look geometry branch                                                                         | `clusters.js:144-523`, `:646-660`                | No                                |
| ELK layout                                                    | No look branch in `elk/render.ts`; swimlane helpers pass `look` through                         | `layout-algorithms/swimlanes/helpers.ts:125`     | Only through node sizes           |
| Global CSS                                                    | Neo: gradient stroke, drop-shadow filter, `stroke-width` on node paths; no font or padding rule | `styles.ts:120-160`                              | No                                |
| Flowchart colour slots                                        | `[data-look="…"]` selectors (redux-color theme)                                                 | `diagrams/flowchart/styles.ts:54-61`             | No                                |
| Neo `<defs>`                                                  | Gradient and drop-shadow definitions                                                            | `rendering-util/insertLookDefs.ts`               | No                                |
| `state` shape corners                                         | rx, ry 3 vs 5                                                                                   | `shapes/state.ts:7-8`                            | Corner radius only                |

### 16.3 Shape padding by look (flowchart shapes)

`P` = `node.padding` = `flowchart.padding` (15 by default, `flowDb.ts:1071`). Label box = text box from section 6. `minNodeWidth` (120) widens the label box first, for every shape that calls `labelHelper` without `ignoreMinWidth` (`shapes/util.ts:92`; circle and doubleCircle opt out, `circle.ts:19`, `doubleCircle.ts:21`).

| Flowchart shape (syntax)                               | Neo                                              | Classic                                                             | Source                                                  |
| ------------------------------------------------------ | ------------------------------------------------ | ------------------------------------------------------------------- | ------------------------------------------------------- |
| `squareRect` `[text]` (default, `flowDb.ts:1011-1014`) | X 16, Y 12. Box W = w + 32, H = h + 24           | X 2P = 30, Y P = 15. Box W = w + 60, H = h + 30                     | `shapes/squareRect.ts:9-10`, `shapes/drawRect.ts:21-22` |
| `roundedRect` `(text)`                                 | No branch: X = P, Y = P (W = w + 30, H = h + 30) | Same                                                                | `shapes/roundedRect.ts:16-17`                           |
| `question` `{text}`                                    | No branch: w + P, h + P                          | Same                                                                | `shapes/question.ts:25-26`                              |
| `stadium` `([text])`                                   | X 40, Y 24                                       | X P, Y P                                                            | `shapes/stadium.ts:39-40`                               |
| `subroutine` `[[text]]`                                | X 28, Y 12                                       | X P, Y P                                                            | `shapes/subroutine.ts:18-19`                            |
| `cylinder` `[(text)]`                                  | X 24, Y 24                                       | X P, Y P                                                            | `shapes/cylinder.ts:62-63`                              |
| `doubleCircle` `(((text)))`                            | Gap 12, label padding 16                         | Gap 5, label padding P                                              | `shapes/doubleCircle.ts:15-18`                          |
| `circle` `((text))`                                    | Radius = label radius + 16                       | Radius = label radius + padding (half P by default, `circle.ts:24`) | `shapes/circle.ts:23-29`                                |
| `hexagon` `{{text}}`                                   | Label padding from factor 3.5 (`wa`, `ha`)       | Factor 4, label padding P                                           | `shapes/hexagon.ts:29-36`                               |

Consequences for the port:

- The measured node size (`createGraph.ts:66-69`) follows these formulas, so neo and classic give different node sizes for the same label. The default `[text]` box is 152 px wide at minimum in neo (120 + 32) and 180 px in classic (120 + 60).
- Labels, edges and clusters need no look branch, so the label code can be shared. The shape code needs both branches.
- The look is per diagram in practice, read once from the resolved config. Per-node `look` (`flowDb.ts:1287`) carries the same value.

### 16.4 Not in scope

- handDrawn: rough shapes and their paddings. The brief excludes it.
- Colour themes beyond what `styles.ts` emits for the default redux-color theme.
