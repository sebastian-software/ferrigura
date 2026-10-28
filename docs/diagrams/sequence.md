# Sequence diagram

A sequence diagram draws participants as columns and the messages between them as arrows from top to bottom, with notes, `loop`, `alt`, `opt`, `par`, `critical`, `break` and `rect` blocks, activations, boxes around participants, and sequence numbers. It starts with the keyword `sequenceDiagram`. ferrigura parses it with a hand-written parser that ports the Jison grammar `sequenceDiagram.jison` and the DB `sequenceDb.ts` (`crates/ferrigura-syntax/src/sequence/`), and draws it with a renderer of its own (`crates/ferrigura-render/src/sequence/`). There is no layout engine: one pass over the messages places everything, as `sequenceRenderer.ts` does.

## Model

`SequenceDiagram` (`crates/ferrigura-model/src/sequence.rs`), version 1, is the DB's state after parsing. Field names are the DB's, in camelCase; a field the DB leaves `undefined` is `null`.

- `title`, `accTitle`, `accDescr`: the DB's values after the sanitizer, with the DB's whitespace rules. A body `title` replaces the front matter title even when it is blank.
- `actors`: in the DB's order, each with `id`; `name`, the id as written (an alias sets only the description); `description`, the alias or the text after `as`, else the name; `type`, `participant`, `actor` or the `type` of the `@{ … }` metadata as YAML gives it (a number stays a number); `wrap`; `box`, the index into `boxes` or `null`; `prevActor` and `nextActor`; `links`, in the key order of a JavaScript object; and `properties`.
- `boxes`: `name` (`null` without a title), `wrap`, `fill` (the color as written, or `transparent`) and `actorKeys`, the ids in the box with repeats kept.
- `messages`: every item in the order `apply` stores it, each with `id`, `from`, `to`, `message`, `wrap`, `type`, `activate`, `centralConnection` and `placement`. `type` is the name of the DB's `LINETYPE` (`SOLID`, `DOTTED_OPEN`, `LOOP_START`, `NOTE`, `AUTONUMBER` and so on). `from` and `to` are `null` for the block markers and for `autonumber`; `activate`, `centralConnection` and `placement` are `null` where the DB sets no such field (notes and `autonumber`). The `message` of an `autonumber` item is `{ start, step, visible }`, with `null` for a number the text left out.
- `createdActors`, `destroyedActors`: the actor id to the number of messages before its creation or destruction.

The model leaves out the second copy of each note that the DB keeps in `notes`, the top-level `actorKeys` (the order of `actors`), the state that only rendering or the parser uses (`actorCnt`, `lastCreated`, `lastDestroyed`, `currentBox`, `sequenceNumbersEnabled`) and `answer`, which `apply` never sets. Message, note, block and actor texts are raw, with entities as placeholders (`#quot;` is `ﬂ°quot¶ß`); the title, the accessibility texts, the box title and the `links` and `properties` texts are sanitized ([canonical-model.md](../canonical-model.md#sequence)).

`links` is a `JsObject`, which serializes in the key order of a JavaScript object: array indices ascending first, then the other keys in insertion order. The reference lists an actor's links in its menu in that order. The exporter and the `canonical` example write the links as `[key, value]` pairs, so the differential run compares the order.

## Parsing

**Lexer.** `lexer.rs` runs the rule table of the Jison grammar in its written order; the first rule that matches wins, not the longest. It has the states `INITIAL`, `ID`, `ALIAS`, `LINE`, `CONFIG`, `acc_title`, `acc_descr` and `acc_descr_multiline`, a stack of them (`scan.rs`), and Jison's implicit word boundary after a keyword literal. Keywords are case-insensitive. The ACTOR rule accepts any run of characters that does not start an arrow, so `A-B` is one actor name.

**Grammar.** `grammar.rs` is recursive descent with one token of lookahead. It builds the same list of items as the Jison actions (`addParticipant`, `addMessage`, `activeStart`, `loopStart`, …); the DB applies them in order after the whole parse, so a syntax error wins over a DB error. The actions that run during the parse in the reference (`setDiagramTitle`, `setAccTitle`, `setAccDescription`, `parseMessage`, `parseBoxData`) run here as well. A box holds only participant statements.

**DB.** `db.rs` follows `sequenceDb.ts` method by method, and its errors carry the reference's text, with the position of the item added: creating an existing id, a message after `create` or `destroy` that does not involve the actor, an actor in two boxes, ending an activation that is not open, and a YAML error in `@{ … }`.

**Pipeline.** `parse_with` runs the steps of `Diagram.fromText`: front matter, directives, comment lines, entity encoding and a final line break. `Options::wrap` is the value of `autoWrap()`: the facade reads it from a top-level `wrap`, then `sequence.wrap`, else `false`.

Quirks of the reference that the parser keeps on purpose:

- **Blanks swallow a line break.** In `INITIAL`, `\s+` comes after `[\n]+`, so a line that ends in blanks loses its line break as a token and the statement before it has no `NEWLINE`.
- **A lone `/`, `\`, `(`, `)`, `<` or `>` at the start of a token is `INVALID`,** and a line that is only that is accepted and dropped. Where a statement needs an actor, `INVALID` is a syntax error: `participant A:B` fails.
- **Re-declaring an actor with a description overwrites it.** `links` and `properties` are reset, `prevActor` is the previous declaration (the same actor for a repeat), and the previous actor's `nextActor` can point to itself. An implicit re-reference (an actor in a message) keeps the record but still assigns the box.
- **`box #color Team` is a comment:** the box has no name and the color is `transparent`. A color is valid when Chromium's `CSS.supports('color', …)` accepts it. The harness's rule is the function `cssSupports` of `export-sequence.ts`, and the parser applies the same rule: named colors, `transparent`, `currentcolor`, the CSS-wide keywords, and `rgb`, `rgba`, `hsl` and `hsla` with CSS numbers.
- **The error for a created or destroyed participant prints `undefined`** for the name, as the reference reads `.name` of an id string.
- **The `end` of a `break` keeps no text,** and its message is `""`.
- **`title` takes one character after the keyword,** so `title  x` keeps a leading blank. `accTitle:` followed by a line break takes the next line as its value. `accTitle` and `accDescr` keep `;` and `#`.
- **`details` reads an element of the page,** so it is a no-op here. `links` replaces `&equals;` and `&amp;` after sanitizing, which never matches the placeholders.
- **Error positions** count the lines of the input. A comment line that `cleanupComments` removes does not count, so a position after one is a line too small.

## Layout and rendering

The renderer ports `sequenceRenderer.ts`, `svgDraw.js`, `styles.js` and `actorBands.ts`. It follows the reference's three passes: the layout of widths and margins (`layout.rs`: `getMaxMessageWidthPerActor`, `calculateActorMargins`, `addActorRenderingData`, `calculateLoopBounds`, `buildNoteModel`, `buildMessageModel`), a loop over the messages that bounds every item and draws notes and blocks (`mod.rs`, `bounds.rs`, `blocks.rs`), and the drawing of the actor rows and the messages (`actors.rs`, `messages.rs`). The modules also hold the configuration (`conf.rs`), the state of one render (`model.rs`), the definitions of arrowheads, icons and the drop shadow (`markers.rs`, `icons.rs`), the labels (`text.rs`) and the style sheet (`style.rs`).

- **Looks.** The default look is `neo`, whose participants sit on a band of fixed height (`actorBands.ts`: a 44 px glyph band, with 6 px between glyph, label and lifeline) and have a drop shadow. `classic` draws each shape at its legacy coordinates and takes its height from the shape's box.
- **Actor types.** `participant`, `actor` (the stick figure), `boundary`, `control`, `entity`, `database`, `collections` and `queue`. An actor of another type draws nothing and keeps its height. The `entity` group has the class `actor`, not `actor-man`.
- **Messages and markers.** Every `LINETYPE` maps to a line, a dash pattern and a marker at the start, the end or both; self messages are a curve, or three segments with `rightAngles`. Central connections are circles, `autonumber` is a marker and a number in 12 px, 9 px above three digits and 7 px above five. The numbers are rounded to two decimals at every step.
- **Bounds.** `bounds.rs` keeps one depth counter for the open blocks and the open activations, as `updateBounds` does, so an activation can get a depth of zero or less. A message at x = 0 does not widen its blocks (a zero coordinate is false in `calculateLoopBounds`). `endActivation` for an actor with no activation removes the last activation of any actor.
- **Text.** Text is measured as whole pixels: the width of a line is the rounded `getBBox` width, a height is the rounded ascent plus the rounded descent per line. Texts are measured as the parser encodes them, with the placeholders, and the whole document is decoded at the end (`mermaidAPI.ts`). Messages, notes and blocks are placed as `drawText` places them; actor and box labels as `byTspan`. Actor, note and message text is set at the top-level `fontSize` (16 px by default) in the top-level font family, whatever `actorFontSize` and its siblings say, because `setConf` copies the top-level values over them.
- **Order of the root's children.** d3's `lower()` moves an element to the front of its parent, so the renderer keeps two lists, the lowered children and the others, and writes them in the reference's order.
- **Root.** The root has `width="100%"` and a `max-width` with `useMaxWidth`; the viewBox starts at the content's left edge minus `diagramMarginX` and has extra height for the title and for the neo footers. The root's `font-size` is the theme's `fontSize` (14 px in the redux themes), not the 16 px of the text.
- **Menus.** An actor with `links` or `properties` gets a popup menu drawn hidden (`display: none`), listed in the key order of its links; `forceMenus` shows them. An actor with links whose type draws no box throws, as the reference reads the `class` of an undefined box.
- **Style sheet.** The rules of `styles.js` and the neo rules, scoped to the diagram's id, with the colors as custom properties and the same colors on the elements as attributes ([0011](../decisions/0011-themes-as-css-variables.md)).

## Differences from Mermaid

- **No click handlers.** The `onclick` and `cursor` of an actor with links are not written ([0006](../decisions/0006-rust-only-no-browser.md)); the menus are in the SVG.
- **Math is not rendered.** A text with `$$` is drawn as written; math in labels is out of scope ([0009](../decisions/0009-svg-text-labels.md)).
- **Labels are always `byTspan`.** The reference selects `fo` or `old` with a `textPlacement` that the configuration schema does not have.
- **Marker URLs are relative** (`url(#id-arrowhead)`). The reference's absolute form reads `window.location`, which a server render lacks.
- **`themeCSS` is not applied.**
- **The lifeline counter starts at zero for every render.** The reference's `actorCnt` is module state that carries over between renders in one page. Only the ids of lifelines and menus differ.
- **The text memo lives for one render.** The reference's lives as long as its page, so a key collision across renders is not reproduced (see Learnings).
- **Text is measured in the configured font only.** The reference measures each text in `sans-serif` and in the configured font and keeps the `sans-serif` result only when it is larger in width, height and line height at once ([0010](../decisions/0010-text-measurement-and-fonts.md)). ferrigura has no system `sans-serif` face. On macOS the reference keeps the configured font for every text probed (`tools/port/text/common-fixture.ts`).
- **Colors are custom properties** with attribute fallbacks, and only the two redux themes ship ([0011](../decisions/0011-themes-as-css-variables.md)).

## Known gaps

- The box color rule is the harness's own function, not Chromium's `CSS.supports`; CSS system colors are not in it.
- The `details` statement does nothing, because it needs a page element.
- An `@icon` of an actor is not drawn at any security level but `loose` and `sandbox`: the reference's DOMPurify pass has no `use` in its allow-list, and ferrigura does the same.
- The `sans-serif` measurement is not reproduced, and the reference's choice has been probed on macOS only; on Linux and Windows the system `sans-serif` may be larger.

## Checking it

- `pnpm -C tools/port port:diff sequence` compares the canonical models on every input: the fixtures, the string literals and docs of the reference, and the cases in `tools/port/cases.ts` (which has links in unsorted order, repeated keys and array-index keys). The exporter runs the reference's `preprocessDiagram`, `encodeEntities`, parser and `SequenceDB` with the configuration steps of `mermaidAPI`.
- `pnpm -C tools/port port:diff config` covers the `sequence` section.
- `pnpm -C tools/port port:geometry sequence` renders every fixture with both sides in Chromium and compares every element, paired by its structural key, within 0.05 px; fixtures in both looks are included.
- The Rust specs: `crates/ferrigura-syntax/tests/sequence_parse_spec.rs`, `sequence_statements_spec.rs`, `sequence_db_spec.rs` and `sequence_keyword_actors_spec.rs` port the parse and DB blocks of the reference's specs, with a test per quirk. `crates/ferrigura-render/tests/sequence_render_spec.rs` ports the rendering blocks, and the bounds and `drawText` unit tests are in `bounds.rs` and `text.rs`.

## Learnings

- **Sizes are whole pixels.** `calculateTextDimensions` rounds each line's `getBBox` width and height, and wrapping compares those integers with the maximum width, so one pixel moves a line break.
- **The configured fonts never apply.** `setConf` copies the top-level `fontFamily` and `fontSize` over the section's three font groups, so the reference draws actor and note text at 16 px with the top-level family. Following the schema's 14 px fails every fixture that has an actor.
- **`look` and `theme` resolve section first.** `readAppearance` takes the section's value before the top-level value, layer by layer, so a diagram without an override is `neo` with `redux-color`, and `%%{init: {"look": "classic"}}%%` keeps the theme.
- **A memo key can collide.** The reference memoizes `calculateTextDimensions` and `wrapLabel` by a string built from the arguments, and two different requests can build the same key; the second then gets the first result. `TextMemo` keeps the same keys per render so that the same collisions happen.
- **Ids depend on the page in the reference.** `actorCnt` is never reset, so a second render in one page continues at `actor3`. A harness that compares ids must render each fixture in a fresh page; the geometry key does not use ids.
- **DOMPurify sees the output.** Every security level but `loose` and `sandbox` sanitizes the SVG string, which drops `use` elements.
- **Lowered groups change the document order.** The reference draws the `rect` backgrounds and the lifeline groups and moves them to the front of the root with `lower()`. The geometry key counts siblings in document order, so an element in the wrong place shows as a difference.
- **The unknown actor type is quiet.** The switch in `drawActor` has no default, so the diagram still reserves the actor's column.
