# TreeView diagram

A tree view draws a file tree: one row per file or directory, connected by lines, with optional icons, descriptions and highlights. It starts with `treeView-beta`. The parser is `ferrigura-syntax::tree_view` (a box-drawing preprocessor, a first-match scanner, a hand-written grammar reader and a DB), the model is `ferrigura-model::tree_view`, and the renderer is `ferrigura-render::tree_view`, which has a layout of its own: each row is a text box with its padding, the rows stack from the top, and no layout engine runs.

```text
treeView-beta
title Project
src/ :::highlight
    main.rs ## entry point
    lib.rs
README.md icon(none)
```

A tree is written in two ways that share one grammar: by indentation, or with the box-drawing characters of `tree` output (`├──`, `└──`, `│`), which a preprocessor rewrites as indentation first.

## Model

`TreeViewDiagram` is the DB after the parse, serialized as the [canonical model](../canonical-model.md#tree-view) with camelCase keys:

- `version`, `title`, `accTitle`, `accDescr`: the title and the accessibility texts, sanitized as the common setters sanitize them. `accTitle` loses its leading blanks and `accDescr` the blanks after each line break. The grammar's title wins when it is not empty, and the front matter's title is used otherwise.
- `count`: the DB's `getCount()`, which is the number of nodes plus one, because the counter starts at 1 and the root takes no number.
- `root`: a node `{ id, level, name, nodeType, icon, cssClass, description, children }`.

The root is always there. The DB starts with the synthetic root `/` (id 0, level -1, a directory), and it is the model's `root` even when the diagram has no node; the user's top-level nodes are its children, in the order they were added. The renderer draws it as the first row.

A node's `level` is the number of indentation characters of its line (a tab counts as one), or the depth of a box-drawing line. `name` has the entity placeholders of `encodeEntities` kept, and one trailing `/` removed from a directory. `nodeType` is `directory` for a name that ended in `/` and for the root, and `file` otherwise. `icon` keeps the DB's three states: `""` when the input gave none, `none` for `icon()` and `icon(none)`, and the name as written otherwise. `cssClass` and `description` are `""` when absent, and a description is sanitized when it is not empty. Names are not sanitized.

Left out: the box (`BBox`) that the reference's renderer writes into the nodes, the built-in icons and the defaults of `showIcons` and the icon maps, which are resolved when the tree is drawn, the configuration, and the front matter except its title.

## Parsing

The text goes through the shared steps of `Diagram.fromText`: front matter, directives, comment lines (`cleanup_comments`), entity encoding and a final line break. Then `preprocess_box_drawing` runs, then the grammar, then the DB is filled in the order of the reference's `parser.ts`: the header texts when they are not empty, and then one `addNode` per node.

**Lexer.** `scanner.rs` follows the token order Langium builds, which Chevrotain applies first match first, not longest match: the keyword, then `ACC_DESCR`, `ACC_TITLE`, `TITLE`, `CLASS_ANNOTATION`, `ICON_ANNOTATION`, `DESC_ANNOTATION`, `INDENTATION`, `QUOTED_NAME`, the hidden comment and line break, and `BARE_NAME` last. Each terminal's regular expression is one function. The keyword carries the word-end suffix `(?:(?=%%)|(?!\S))` and a longer alternative: a `BARE_NAME` that matches longer at the same position wins, so `treeView-beta foo`, `treeView-beta` followed by blanks and `treeView-betaX` are names, and the parser rejects them. A blank run is tested once for the annotation that may follow it, which keeps the scan linear.

**Grammar.** `grammar.rs` reads the tokens with one function per rule:

```text
TreeView:  'treeView-beta' (ACC_DESCR | ACC_TITLE | TITLE)* TreeNode*
TreeNode:  INDENTATION? (QUOTED_NAME | BARE_NAME) (CLASS | ICON | DESC)*
```

Each header terminal and each annotation assigns its property, so the last one wins, an empty one included. A token the grammar does not take is an error.

**DB.** `db.rs` mirrors `db.ts`. `addNode` pops the stack while the new level is not greater than the level of the top node, takes the next id, and becomes a child of the new top. The reference keeps children in the nodes as they are added; here a node joins its parent when it is popped, which gives the same order, because at most one child of a node is open at a time.

**Box-drawing input.** `box_drawing.rs` ports `boxDrawingPreprocessor.ts`. The keyword line is the first line whose trimmed text is `treeView-beta`. Box mode is on when a content line (not blank, a comment, a header line or only `│` and `┃`) holds any of `─━│┃└┗├┣`, anywhere. The segment width is the column of the first branch character (`└┗├┣`) that is not in column 0, or 4. A branch line gets `Math.round(column / segmentWidth) + 1` levels of four spaces, with its text taken after the branch and its dashes; decoration lines are dropped, and other lines keep their text. Columns are UTF-16 offsets. A map from output lines to input lines is kept, so that a grammar error is reported at the input line. The two errors keep the reference's wording, which its spec asserts: `Line N: Empty node — …` and `Line N: Unexpected indentation without box-drawing characters. …`. They are thrown before the grammar runs, so they win over a lexer error on an earlier line.

## Layout and rendering

`tree_view/mod.rs` ports `renderer.ts` (`draw`, `drawTree`, `positionLabel`, `positionLine`), `icons.rs` ports `icons.ts` and the icon markup of `rendering-util/icons.ts`, and `style.rs` ports `styles.ts`. The arithmetic runs in the reference's order, in three phases:

1. **Rows**, in document order, the root first. A row at depth `d` has `x = d × (rowIndent + paddingX)`, the height of its label's text box plus `2 × paddingY`, and its label at `x + paddingX`, plus 18 when it has an icon (14 for the icon, 4 for the gap). A connector runs from `x − rowIndent` to `x` at the row's center, and after the children of a node a trunk runs from the bottom of its row, at `x + paddingX`, to the center of its last child plus half the line thickness. The width is the widest `x + label width + 2 × paddingX (+ 18)`.
2. **Descriptions** share one column for every row, 16 to the right of the widest label's right edge, and widen the tree to the right edge of the widest description plus `paddingX`.
3. **Highlights** (a node whose class list holds `highlight`) are sized one after the other: a rectangle is `width so far − x + 8` wide and widens the tree to its right edge plus 2, so each highlight adds 10 and the next one is sized from the grown width.

The root's `viewBox` is `-lineThickness/2 0 width height`; `configureSvgSize` then writes `width="100%"` (and `max-width`) or, with `useMaxWidth: false`, the pixel size. The accessible title and description come first in the root as `<title>` and `<desc>`; the diagram's `title` line is stored and never drawn.

**Text.** A label's box is the box `getBBox` gives its text in `ferrigura-text`'s faces, measured on the text with the entity placeholders (`ﬂ°amp¶ß`) and drawn as the character they stand for. The face is regular, bold for a directory or the class `treeView-node-dir`, and italic for the class `treeView-node-description`, as the style rules set it; the sheet keeps `white-space: pre`, so runs of spaces count. An empty name has a box of 0 by 0, so its row is `2 × paddingY` high. The glyphs used are embedded as subsets ([0010](../decisions/0010-text-measurement-and-fonts.md)).

**Icons.** `getNodeIcon` decides: `icon none` hides the icon; an explicit icon is qualified (`pack:name` as is, a built-in name with the built-in prefix, any other name with `defaultIconPack` or the built-in prefix); with `showIcons` off there is none; for a file with `showIcons` on, `filenameIcons` is read first (the exact name), then `extensionIcons` (the last dot, not at the start, lower-cased, with the dot and then without it); the rest get the built-in `folder` or `file`. A falsy filename value falls through to the extension, and a falsy extension value gives the default icon. The icon is a `<svg>` of 14 by 14 inside `g.treeView-node-icon`, translated to `(x + paddingX, y + paddingY)`, so it is aligned to the top of the row, not centered.

**Styles.** The options are `themeVariables.treeView` (`labelFontSize`, `labelColor`, `lineColor`, `iconColor`, `descriptionColor`, `highlightBg`, `highlightStroke`) over the defaults of `styles.ts`; no shipped theme sets the group. Both the front matter and the init directive set them. Every color is a custom property on the diagram's id and is also a presentation attribute on its element ([0011](../decisions/0011-themes-as-css-variables.md)); the rules for sizes and weights name the classes `treeView-node-label`, `-dir`, `-line`, `-icon`, `-description` and `treeView-highlight-bg`, and a class a user gives a node takes the same rules. Every rule of the sheet is scoped to the diagram's id.

## Differences from Mermaid

- **Icons.** ferrigura registers no icon pack ([0006](../decisions/0006-rust-only-no-browser.md)). The built-in `folder` and `file` are drawn; any other icon name draws the reference's unknown icon (an 80 by 80 blue box with a white `?` in `ArialMT, Arial`), which is also what the reference draws for a pack it cannot load.
- **Colors** are custom properties plus presentation attributes ([0011](../decisions/0011-themes-as-css-variables.md)), not only rules in the sheet.
- **Errors** are ferrigura's own, with a position ([0007](../decisions/0007-hand-written-parsers.md)); the two box-drawing messages keep the reference's wording, and the differential run compares acceptance only.
- **The font size** (`labelFontSize`) must be a size in pixels, because only pixels are measured; another unit is `RenderError::Unsupported`.

## Known gaps

- Icon packs and `registerIconPacks`: a name that is not built in always draws the unknown icon.
- A `labelFontSize` in another unit than pixels, or without a unit, fails the render.
- A color value that holds `;` or `}` means something else in the reference, which writes the value into the rule as it is; ferrigura writes it into a custom property. No fixture has one.
- An icon map value that is not text, and a file name that reads an `Object.prototype` member (`constructor`, `toString` and the like) from a map, fail the render as they fail the reference's `qualifyIcon`; the message is ferrigura's own. The tests build the section directly; whether the typed configuration passes such a value through the facade is not checked.
- Tabs in names, and quoted names that span lines, are parsed but have no geometry probe; Chromium draws a line break in a `white-space: pre` name in a way that is not measured.

## Checking it

- `pnpm -C tools/port port:diff treeView` compares the canonical model of every input with the reference's (`export-treeView.ts`, which runs `Diagram.fromText`'s steps, the box-drawing preprocessor and the Langium grammar and reads the DB); the explicit cases in `tools/port/cases.ts` cover the quirks below. `port:diff config` and `port:diff theme` cover the `treeView` configuration and the theme variables.
- `pnpm -C tools/port port:geometry treeView` renders the fixtures under `e2e/diagrams/tree-view/` and the probes under `tools/port/geometry/probes/treeView/` in Chromium with both renderers and compares the geometry. The probes cover the empty tree, slash and empty names, highlights, the description column, entities, spaces, `useMaxWidth`, line and padding options, theme variables in a directive, class names, icon maps, explicit icons, accessible texts, box-drawing input and deep trunks. The comparison leaves the box-drawing characters out when it checks which characters of a fixture no bundled face covers, because they are syntax, not drawn text.
- Rust specs: in `ferrigura-syntax/tests`, `tree_view_parser_spec`, `tree_view_db_spec` and `tree_view_box_drawing_spec` port the reference's `parser.spec.ts`, `db.spec.ts` and `boxDrawingPreprocessor.spec.ts`, and `tree_view_grammar` pins each quirk below on its probe's input; in `ferrigura-render/tests`, `tree_view_icons_spec` ports `icons.spec.ts` and `tree_view_render_spec` pins the layout rules, which the reference has no spec for. The `remapErrorLines` message-rewriting cases are ignored, because a `ParseError` carries a position, not a message that names a line.

## Learnings

**Parser and lexer**

- The first matching terminal wins, not the longest. `titlepage.md` is `TITLE` and then the name `page.md`, and `title.txt` is the name `.txt`. A title line after a node, and an indented name that starts with `title` after a node, are errors, because `TITLE` takes the indentation. A title-prefixed name after a title line leaves the title empty, and the front matter title then stays.
- A line of blanks is an `INDENTATION` that the next node takes (three for spaces, one for a tab); before a title it is an error. Blanks after a quoted name are an `INDENTATION` the grammar cannot place, an error; after a bare name they are trimmed.
- `%%` inside a line is part of the name; an unindented `%%` alone is a comment between siblings and an indented one an error; a comment line with text is removed by the core. The core's `cleanupComments` is `^\s*%%…` with the multi-line flag, and in JavaScript `^` also matches after U+2028 and U+2029, so a `%%` after a line separator starts a comment; the shared `cleanup_comments` looks after every line terminator for this reason.
- A quoted name is `"[^"]*"` or `'[^']*'`, may span lines, and ends at the next quote; the rest of the line is a bare name. `""` is a file with an empty name, `"src/"` is a directory, `/` is a directory with an empty name, and `a//` is the directory `a/`.
- `:::1cls` and `:::` are part of the name (a class needs a letter or `_` first). A repeated class or icon keeps the last. A description takes the rest of the line, annotations included. A line that starts with `:::`, `icon(` or `##`, and an icon with a dot or three parts, are lexer errors.
- Levels only have to grow to nest: a deeper node under uneven indentation becomes a child of the nearest smaller level, and a first node that is indented is still a root item. A box-drawing character anywhere in a content line switches the whole diagram to box mode. In a diagram written by indentation, a name with a `├` in it then loses the text before the character, and an indented line without a box character is an error.
- Entities are encoded before the grammar and decoded only in the final SVG, so names and descriptions hold placeholders in the model and the layout measures them.

**Rendering**

- The reference always draws the root `/`, a bold directory, even for an empty diagram; its own documentation shows no such row. A connector of a top-level row starts at x = -10, so only the part right of the `viewBox` origin (-0.5) shows.
- Each highlighted row adds 10 to the width, and the rectangles depend on the rows before them; the order of the three phases is the whole trick.
- The description column is the same for every row, also for rows without a description.
- Every rule of the real render is scoped with the diagram's id. Calling the reference's `getStyles` on its own leaves the first rule, `.treeView-node-label`, without a scope; that is an artifact of calling it directly, not of a render.
- `themeVariables.treeView` is honored from the init directive as well as from the front matter.
