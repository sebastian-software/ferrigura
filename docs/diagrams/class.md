# Class diagram

A class diagram draws classes with their attributes and methods, the relations between them, and optionally notes and namespaces. It starts with `classDiagram` or `classDiagram-v2`; both headers are one diagram type with one configuration section, `class`. ferrigura parses it with a hand-written lexer and recursive-descent parser in `ferrigura-syntax` (`src/class/`), draws it with the unified renderer in `ferrigura-render` (`src/class/`, with the class box in `src/unified/shapes/class_box.rs` and the cardinality labels in `src/unified/terminal.rs`), and lays it out with ELK Layered. The shared machinery (shapes, edges, labels, clusters and the ELK glue) is in [flowchart.md](./flowchart.md); this page covers what is specific to classes.

## Model

`ferrigura_model::class::ClassDiagram` is the Diagram DB after parsing (`classDb.ts`), with the DB's names in camelCase. A field the DB leaves `undefined` is `null`.

- **Header:** `title`, `accTitle`, `accDescr` and `direction` (`TB` by default, the last `direction` statement otherwise).
- **`classes`**, in creation order: `id` (the name without its generic type), `type` (the generic type, `""` when none), `label`, `text` (what the box shows: the label, then the generic type as `&lt;T&gt;` when the class is added, as `<T>` when a label is set), `cssClasses` (`default`, then each applied class after a space), `annotations`, `members` (attributes) and `methods`, `styles`, `parent` (the namespace), `link`, `linkTarget`, `tooltip` and `haveCallback` (`true` once a callback is set, `null` otherwise).
- **A member** (`ClassMember`) has `memberType` (`method` when the text has a `)` after its first character), `visibility`, `classifier` (`$` static, `*` abstract), `id`, `parameters` and `returnType` (`null` for an attribute) and `text`, the string the renderer draws: a backslash and the visibility, the name, and `(params) : type` for a method, with `<` and `>` written as `&lt;` and `&gt;`.
- **`relations`:** `id1`, `id2`, `relationTitle1` and `relationTitle2` (the cardinalities, the string `none` when absent, as the DB holds them), `title` (`null` without a label), `type1` and `type2` (the marker names `none`, `aggregation`, `extension`, `composition`, `dependency`, `lollipop`) and `lineType` (`solid` or `dotted`).
- **`notes`:** `id` (`note<index>`), `class` (the text as written, `null` for a plain note), `text` (raw, never sanitized), `index` and `parent`.
- **`interfaces`:** the lollipop nodes, with `id` (`interface<n>`), `label` and `classId`.
- **`namespaces`**, ancestors before descendants: the qualified `id`, `label`, `parent`, `explicit` (declared, as opposed to created as the ancestor of a dotted name), `classes` and `notes` (member ids in first-seen order).
- **`styleClasses`:** `id` and `styles` of each `classDef`.

The model leaves out what only rendering needs or what follows from other fields: `domId`, the module-level `classCounter` and `namespaceCounter`, `shape` (always `classBox`), `children` of a namespace (derived from `parent`), the `textStyles` of a style class (derived from `styles`) and the callback functions ([0006](../decisions/0006-rust-only-no-browser.md)). The title and the accessibility texts are sanitized as the DB's setters do (the title is sanitized here, unlike the flowchart's); the harness runs the real `commonDb` setters, so those rules are compared and not copied. The conventions common to all models are in [canonical-model.md](../canonical-model.md).

## Parsing

The text goes through the shared preparation (front matter, directives) and whole-line comment removal. The class parser takes no other step: no entity encoding, no closing-brace rewrite and no trailing line break. `parse_grammar` runs the lexer and grammar alone, as the ported specs call the reference's parser.

**Lexer** (`class/lexer.rs`). The rules of `classDiagram.jison` run in their written order on a state stack (`scan.rs`, [0007](../decisions/0007-hand-written-parsers.md)): first match wins, a keyword ending in a word character gets Jison's implicit word boundary, and `begin`, `popState` and `less(0)` behave as in Jison. The states are INITIAL, `string`, `bqstring`, `generic`, the three accessibility states, `callback_name`, `callback_args`, `namespace`, `namespace-body`, `class` and `class-body`. No action of the grammar changes a state, so the lexer runs over the whole text before the parser, and a lexical error anywhere rejects the input. At the end of the input the rules run once more on the empty rest, as Jison's `next()` does, so a still-open `string` state yields an empty `STR` before `EOF`.

**Grammar** (`class/grammar.rs`). Recursive descent over the tokens; each DB call is made where Jison's reduction makes it. `class/db.rs` is the DB with its setters, `class/member.rs` is `ClassMember` and `parseGenericTypes`, and `src/url.rs` is `sanitizeUrl` of `@braintree/sanitize-url` and `formatUrl`.

**Sanitizing.** Every DB text goes through `sanitizeText`, which depends on `htmlLabels` and `securityLevel` (`Options` in `class/mod.rs`): DOMPurify for `strict`, `antiscript` and `sandbox`, nothing for `loose` before DOMPurify, and an escaping step for other levels. The reference runs DOMPurify twice for `strict` (script removal, then a `style` filter); one pass of `sanitize.rs` with `FORBID_TAGS: ['style']` gives the same result on every checked input.

Quirks of the reference's grammar that are kept on purpose, because a fix would show as a difference:

- Blanks between identifiers are dropped in INITIAL, so `Foo Bar --> Baz` has the class `FooBar`.
- A line that holds `direction TB` (or `BT`, `RL`, `LR`) is one `direction` token from the current position, so the text before it on the line is lost: `A --> B direction TB` drops the relation. A `direction` inside a namespace body is not a direction (the text fails to parse), and inside a class body it is a member.
- A `%%` in the middle of a line consumes the line break after it, which usually breaks the statement; only whole-line comments are removed beforehand.
- The header must be followed by a line break; statements are line-break separated; a namespace body with no statement is an error (`namespace N {}`).
- `[*]` is an error everywhere. `title x` is a class name with no action, so no title is set; the title comes from the front matter only. A bare class name (`A`) adds nothing.
- `NUM` never lexes, because `\w+` (`ALPHA`) matches digits first. `o` is the aggregation marker only as a whole word, so `class order` is a name.
- `<<x>>` in a class body is an annotation, but `&lt;&lt;x&gt;&gt;` there is a member.
- Members are reversed by the grammar and reversed again by `addMembers`, so source order stays; an empty member is dropped.
- `A~B~C~` has the generic type `B` (the segment between the first two `~`).
- `setCssClass`, `setLink` and the `clickable` mark look an id that starts with a digit up as `classId-<id>`; `setTooltip` and the callback flag do not.
- `defineClass` appends a style to every class whose `cssClasses` contains the style class name as a substring.
- In a lollipop relation (`A ()-- I`) the end next to `()` is not a class: it becomes an interface node labeled with its raw id (`label`), and the other end is the class (`classId`).
- `accTitle:` and `accDescr:` skip the line break after the colon, so the value may come from the next line.
- A DB error where the reference's JavaScript throws a `TypeError` is a parse error: a member with `)` but no `(`, and a setter on a class that sanitizing renamed.

## Layout and rendering

`class/mod.rs` is `classRenderer-v3-unified.ts`: it builds the layout data with `class/data.rs`, sets the diagram type that names the markers, and runs the unified pipeline (measure, ELK layout and post-processing, paint). The root is `<svg class="classDiagram">` with the five relation markers, the title as `classDiagramTitleText`, and a viewBox padded by 8.

**Layout data** (`class/data.rs`, `getData` of `classDb.ts`). Nodes come in this order: namespaces (group nodes of shape `rect`, padding 16), classes (shape `classBox`), notes (shape `note`, padding 6, a Markdown label), interfaces (an invisible `rect`, `opacity: 0`). Edges: a dotted `edgeNote<index>` from each note to its class, then one edge per relation with the id `id_<id1>_<id2>_<n>`, Markdown label, the two markers from `type1` and `type2`, the cardinalities as `startLabelRight` and `endLabelLeft`, and the pattern `dashed` for `..`. With `hierarchicalNamespaces: false`, only declared namespaces are drawn and a class sits in its nearest declared ancestor. Only classes take a color slot (`colorIndex`), so namespaces and notes do not shift the palette.

**The class box** (`unified/shapes/class_box.rs`, `classBox.ts` with `shapeUtil.ts`'s `textHelper` and `addText`). A box is up to four text groups (annotation, name, members, methods), two dividers and an outline. Each row is Markdown laid out as SVG text; the name is bold, the first annotation is written `«name»`, a static member is underlined and an abstract one italic. Every size the reference reads from a `getBBox` is rounded to float32, as the browser returns it, and an empty box is left out of a union as `getBBox` leaves it. The padding is 12 (the box's `class.padding ?? 12`).

**Edges and markers** (`unified/markers.rs`, `unified/path.rs`). The five relation kinds each have a start and an end marker, plus the margin variants that the neo look uses; the ids are `<svgId>_classDiagram-<kind>Start|End[-margin]`. An edge is pulled back from its node by the marker's path offset (17.25 for aggregation, extension and composition, 6 for dependency, 13.5 for lollipop).

**Cardinalities** (`unified/terminal.rs`). The two cardinalities of a relation are terminal labels. ELK gets them as labels with the placement `TAIL` or `HEAD` and the end's marker clearance of 6 around their size, measured at the 11 px of `.edgeTerminals`. After the layout, passes move each label with its end (`followMovedEndpoints`), slide it off group frames and put it on its own side of the end segment, avoiding nodes, labels, frames and other edges; labels that ELK did not place sit at `calcTerminalLabelPosition`'s point beside the path. Only `startRight` and `endLeft` exist for classes.

**Style sheet** (`class/style.rs`, `styles.js` and the shared rules). Colors are custom properties on the diagram id ([0011](../decisions/0011-themes-as-css-variables.md)). In `redux-color` and `redux-dark-color` every class box is painted from the 12-slot palette by its `data-color-id`, and a `classDef` overrides it.

**Links.** A class with a link is wrapped in `<a xlink:href target>`; a tooltip is the `title` attribute; `clickable` is added to the class.

## Differences from Mermaid

- **Labels are SVG text**, also with `htmlLabels: true` ([0009](../decisions/0009-svg-text-labels.md)). The row texts take the geometry of the reference's `htmlLabels: false` path whatever the setting: a text padding of 3 and a shift of 4.
- **`layout: dagre`** is laid out with ELK ([0012](../decisions/0012-layout-engines-ported-exactly.md)); `look: handDrawn` takes the classic geometry.
- **Callbacks** (`click A call f()`) set `haveCallback` and the `clickable` class, and the function is not kept ([0006](../decisions/0006-rust-only-no-browser.md)).
- **`sanitizeUrl`** follows `@braintree/sanitize-url` 7.1.2 on the `url` crate, which differs in two places: it does not percent-encode a caret in the path of an `http(s)` URL, and it rejects an `xn--` label whose punycode does not decode. A lone surrogate that an entity could produce becomes U+FFFD, since a Rust string cannot hold it.
- **DOM ids** of classes are `classId-<name>` without the page-wide counter that the reference appends; names are unique in a diagram already.

## Known gaps

- The facade parses with the default `Options`: `securityLevel` and `htmlLabels` from the configuration do not reach the class DB. The other branches of `sanitizeText` are written from the source and are not in a differential run; `dompurifyConfig` is not supported.
- The ported specs leave out the callback tests, the reference's `it.skip` and the JavaScript own-property test.
- Fixtures with `look: handDrawn` and with scripts the bundled faces do not cover are not compared in the geometry run.

## Checking it

- `pnpm -C tools/port port:diff class` compares the canonical models on the fixtures, the string literals of the specs and the docs; the reference's header setters run for real.
- `pnpm -C tools/port port:diff class-data` compares `getData` field by field (`export-class-data.ts`, the `class_data` example); `pnpm -C tools/port port:diff config` compares the `class` section.
- `pnpm -C tools/port port:geometry class` renders every fixture with both in Chromium and compares geometry; `--filter 'class-diagram/**'` narrows it.
- Rust: the spec files `class_*_spec.rs` in `ferrigura-syntax/tests` (ported from `classDiagram.spec.ts` and its siblings), `class_db_get_data_spec.rs` and `class_types_display_spec.rs` in `ferrigura-render/tests`, and the unit tests of `terminal.rs`. The class graphs among the captured inputs of `ferrigura-elk` (`tests/corpus/class-diagram__*`) check the layout, including the `TAIL` and `HEAD` labels.

## Learnings

- **The `class` section has no `padding` at run time.** The schema says 5, but `defaultConfig.ts` builds the `class` object from scratch, so the reference falls back to 12 (class box and rows), 16 (namespace) and 6 (note). The configuration build drops the schema defaults for the keys the reference leaves undefined (`CLASS_KEYS_WITHOUT_DEFAULT` in `ferrigura-model/build.rs`); keeping the 5 would change every box.
- **Spacing, title margin and `useMaxWidth` come from the `state` section**, not from `class`. This is copied from the state renderer in the reference, and ferrigura follows it: `state.titleTopMargin` (25) places the title and `state.useMaxWidth` decides the size attributes. The class section's own keys of that name are read nowhere.
- **The default look is neo**, although the top-level default is classic: the class section defaults it. A fixture whose description says "classic look" without setting `look` renders neo.
- **The empty-group quirks of the class box are kept.** With no members and no methods the box is drawn with an extra empty compartment and `P/2` (6 px) is taken off the heights of the annotation, name and members groups, which can leave a negative height that the reference's `|| 0` does not clamp. An empty members group has a height of at least `P/2`, so a methods group sits at `annotation + name + 54` below an empty members compartment. The second divider is placed from the members' height, not from the box that `textHelper` measured.
- **Rows that contain `<` or `>` show the raw text.** The reference's `&gt` test fires for every row whose sanitized text has an entity, and then replaces the first text child with the unsanitized text, trimmed. The sanitized form only decides width and line count.
- **Row wrapping width is the text's width plus 50**, so a row practically never wraps. The reference measures that width in the DOM and rounds it.
- **The e2e fixtures decode entities before parsing.** Many class fixtures write `&lt;&lt;interface&gt;&gt;`; the viewer decodes them, and the harness does the same (`entityDecode`), which makes them annotations again. Written as entities inside a class body, they are members.
- **`getBBox` of translated groups decides sizes.** The class box moves its groups after creating them and then reads the union, so the arithmetic must follow the same order and the same float32 rounding.
- **The cardinality labels are many small geometric rules with exact comparisons**, so a float32 step in a box moves a label to the other side; the passes keep the reference's order.
- **Both headers are one diagram type.** `classDiagram-v2` is detected as `classDiagram`, so it shares the `class` section, the root class and the marker ids.
