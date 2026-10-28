# Railroad diagram

A railroad diagram draws grammar rules as tracks: each rule is a name followed by a path of terminals, nonterminals and specials, with arcs for choices, optionals and repetitions. It has four syntaxes that share one model, one DB and one renderer: `railroad-beta` (Mermaid's own form, written as calls such as `sequence(...)` and `terminal("a")`), `railroad-ebnf-beta` (ISO 14977 and W3C-style EBNF), `railroad-abnf-beta` (RFC 5234) and `railroad-peg-beta` (PEG). ferrigura reads each syntax with a hand-written parser in `ferrigura-syntax` ([0007](../decisions/0007-hand-written-parsers.md)) and draws the rules with a renderer of the type's own in `ferrigura-render`; no layout engine is involved, since every size comes from measured text, the configured spacing and a few constants.

## Model

The model is the DB state after the parse of any of the four syntaxes ([canonical-model.md](../canonical-model.md#railroad); `ferrigura-model`, `railroad.rs`). The syntax is not a field of it.

- `title`, `accTitle` and `accDescr`: the last line of each kind in the text, sanitized, or `""`. The last line wins even when it is empty, so an earlier title is lost. The front matter title never reaches the model: the reference's parser clears the DB after the front matter has set it, in all four syntaxes.
- `rules`: every rule in text order, duplicates included. A rule is `{ name, definition }`.
- An expression is an object with a `type`: `terminal` (`value`), `nonterminal` (`name`), `special` (`text`), `sequence` (`elements`), `choice` (`alternatives`), `optional` (`element`) and `repetition` (`element`, `min`, `max`). A sequence or a choice of one element is that element. `min` and `max` are the grammar's numbers as given; a minimum above a maximum is kept, and `Infinity` is written as the string `"Infinity"`.
- Rule names, terminal values, nonterminal names, special texts, the title and the accessibility texts are the DB's sanitized values: `<script>` is removed, `&` is written `&amp;`, and the entity placeholders stay. This departs from the canonical text rule that markup is raw, because the reference's DB sanitizes here as the flowchart's does ([sanitize.rs](../../crates/ferrigura-syntax/src/sanitize.rs)).
- The strings differ from the canonical text rule for escapes. The strings and quoted titles of `railroad-beta`, EBNF and PEG decode only `\n`, `\r` and `\t`; any other escaped character stands for itself, so `\b`, `\f`, `\v` and `\0` are the letters. ABNF strings and numeric values decode nothing.
- Left out: a rule's comment and a repetition's separator (no grammar sets them), the syntax, the configuration and the styles.

## Parsing

`ferrigura_syntax::railroad::parse` prepares the text as `Diagram.fromText` does: front matter and directives are removed, whole-line `%%` comments are cleaned, entities are encoded as placeholders and a line break is appended. The header picks the grammar, in any letter case, as the reference's four detectors do; the grammar then needs the exact lowercase header. `detector_id` returns the detector id of the text's syntax (`railroad`, `railroadEbnf`, `railroadAbnf`, `railroadPeg`), which the renderer needs for the SVG.

The four Langium grammars are lexed by Chevrotain, which takes the first token type that matches at a position, not the longest. `scanner.rs` runs the same order for each syntax: the keywords, then the terminals, then the hidden tokens (white space, a `---` YAML block, a directive, comments). A keyword with a `LONGER_ALT` gives way to its alternative when that matches more text. `grammar.rs` holds one recursive-descent parser per syntax that builds the reference's AST directly (its `transform*` functions), and `db.rs` is the shared DB: the header texts are set only when non-empty, then every rule is added with its texts sanitized.

Quirks of the grammars that ferrigura keeps:

- **The header** is case-sensitive and must be followed by a blank, a line end or `%%`: `railroad-betax` is rejected, as is `RAILROAD-BETA`, which the detector accepts. A block comment before the header leaves the text undetected, because the detector skips only directives and `%%` lines.
- **A title eats the prefix of a word.** The title terminal matches `title` with an empty tail, so `titles = terminal("a") ;` is the rule `s` with an empty title. `accTitle` and `accDescr` need a colon or a brace.
- **Word keywords need the longer identifier.** In `railroad-beta`, `sequenceX` is a name, but `sequence` or `nonterminal` as a rule name is an error; `Terminal` with a capital is not a keyword.
- **Header lines come first.** `title`, `accTitle` and `accDescr` after the first rule are errors.
- **EBNF.** `A - B` is the sequence `A`, `"-"`, `B`, which is drawn as three boxes. A special sequence `? … ?` needs a non-blank character and no `;` between the marks, is trimmed and keeps a line break inside. Postfixes nest in order: `{ "a" }*` is a repetition of a repetition. A comma with no term after it is an error.
- **ABNF `;`** ends a rule, but it is a comment when text follows it on its line, so `r = "a" ; comment`, `r = "a" ; ` and `r = "a";x` are errors, while `r = "a";` at the end of its line is accepted. A repeat keeps its bounds: `*1` and `0*1` are optionals, `*0` is a repetition from 0 to 0, `2*1` a repetition from 2 to 1. `%x41-5A` is one terminal with its text, and a quoted title keeps its backslashes.
- **PEG.** A lookahead `&` or `!` makes a special whose text is the operator and a label of the operand: a quoted terminal (its quote is not escaped), a rule name, a special's text, or `(...)` for any other expression. Prefixes and suffixes are single: `&&"a"`, `! & "a"` and `"a"?*` are errors.
- **Names** are `[A-Za-z_][\w-]*` in `railroad-beta`, EBNF and PEG, and `[A-Za-z][A-Za-z0-9-]*` in ABNF.
- **Hidden text.** Comments, directives and a closed YAML block are skipped anywhere between tokens; an unclosed YAML block is an error. Lines end only at `\n` or `\r`, so a U+2028 stays inside a comment or a title. A raw line break inside a string is accepted.

Error messages are ferrigura's own; the differential run compares acceptance, not text ([0007](../decisions/0007-hand-written-parsers.md)).

## Layout and rendering

`ferrigura-render`, `railroad/` ports `railroadRenderer.ts` (`draw` and the `RailroadRenderer` class) in `layout.rs`, `styles.ts` and the options of `railroadTypes.ts` in `style.rs`, and the text measurement in `text.rs`; `mod.rs` assembles the SVG root, the accessibility elements and the style sheet. The renderer takes the model and a `RailroadSection` (the `railroad` configuration section and the detector id).

- **Boxes.** A terminal, nonterminal or special is a rectangle of the measured text plus `padding` on each side, with the text at its center. A terminal has rounded corners (`rx = ry = 10`), a special shows its text as `? text ?`. The height of a text is the line box of the font, whatever the text.
- **Sequence.** The items sit side by side on a common center line, `horizontalSeparation` apart, with a line between them.
- **Choice.** The alternatives stand one under the other, `verticalSeparation` apart and centered in a width of the widest plus `4 * arcRadius`; each joins the center line by an arc on the left and one on the right, or by a straight line when it sits on the center line.
- **Optional and repetition.** An optional adds a bypass arc above its element. A repetition adds a loop below it and, when `min` is 0, a bypass above. Only `min` is read; the maximum is not drawn.
- **Rule.** The name (`name =`) and the definition, between a start and an end marker: the start marker sits at the name's width plus 20, the definition starts 20 after it, the baseline is at least 20 down, the rule is at least 40 high, and the end marker follows the definition by 10. The first rule is at `y = padding`, each next one a rule height plus `verticalSeparation` further down; the diagram is the widest rule plus `2 * padding` wide, with one trailing `verticalSeparation` in its height. A diagram without rules is 200 by 100.
- **Markup.** Elements are appended in the reference's order: `g.railroad-rule` holds the definition group, the name group, the start and end groups, and the two joining lines. The root has class `railroad-diagram`, `width="100%"` with a `max-width` style (or `width` and `height` for `useMaxWidth: false`), `viewBox="0 0 W H"`, and `aria-roledescription` set to the detector id.
- **Style.** The classes of the reference's style sheet (`railroad-terminal`, `railroad-nonterminal`, `railroad-special` with its dashed stroke, `railroad-line`, `railroad-start` and `railroad-end`, `railroad-rule-name` in bold). Colors come from the `railroad` section when it holds a valid one, else from the theme variables, else from the defaults of `DEFAULT_RAILROAD_CONFIG`. A color is valid when it matches the reference's color pattern (hex of 3, 4, 6 or 8 digits, a color function, or a bare name); a number is valid when finite and not negative. Font size falls back from the section to the theme's to 14.
- **Text.** Each text is measured once and cached by its content, as the reference's `measureText` does: the width is the box of the bundled face in Chromium's rules (`ferrigura-text`), rounded to float32 as `getBBox` returns it. The texts measured are terminal values, nonterminal names, the `? … ?` of specials and the rule names with ` =`.

## Differences from Mermaid

- **Colors** are CSS custom properties on the diagram's id, one set per scheme, and each element also carries its light color as a presentation attribute; `colorScheme: auto` writes both sets ([0011](../decisions/0011-themes-as-css-variables.md)). The reference writes concrete values into its style sheet.
- **Fonts.** The text is drawn in the bundled face, embedded as a subset, so the `fontFamily` key is not read; `fontSize` is ([0010](../decisions/0010-text-measurement-and-fonts.md)).
- **Themes.** Only `redux-color` and `redux-dark-color` ship. The theme variables the style sheet reads that a theme does not set take the default theme's values, as the reference merges them.
- **Errors** are ferrigura's own messages ([0007](../decisions/0007-hand-written-parsers.md)).
- **The configuration** has no default for the sixteen color and typography keys, as the reference's `defaultConfig.ts` leaves them `undefined` so that the theme supplies them: `RAILROAD_KEYS_WITHOUT_DEFAULT` in `ferrigura-model/build.rs`. The reference's own schema defaults generator skips the `railroad` section, which would make its front matter and directives drop options only railroad has (`arcRadius`, `verticalSeparation`, `horizontalSeparation`, `compactMode`); the reference's patch `05-railroad-keep-railroad-only-options-in-front-matter-and-dire.patch` fixes that ([0014](../decisions/0014-upstream-reference-at-a-pin.md)).

## Known gaps

None specific to the type. Fixtures that use scripts the bundled faces lack need registered fallback fonts, as for every type ([0010](../decisions/0010-text-measurement-and-fonts.md)).

## Checking it

- `pnpm -C tools/port port:diff railroad` compares the canonical models of the reference and ferrigura on the fixtures, the specs and documentation of Mermaid, and the edge cases of `tools/port/cases.ts` (the `railroad` list: the quirks above, each as a text of one of the four syntaxes). `tools/port/export-railroad.ts` is the reference side: it runs `Diagram.fromText` and reads the DB.
- `pnpm -C tools/port port:diff config` compares the resolved `railroad` section, on the same inputs.
- `pnpm -C tools/port port:geometry railroad` renders the fixtures under `e2e/diagrams/railroad` and the probes under `tools/port/geometry/probes/railroad` in Chromium: the `c` probes cover each construct and the four syntaxes, the `e` probes entities, escapes and repetition bounds, the `g` probes configuration keys (spacing, colors, font, stroke width, bad numbers, inert keys) and two syntax cases (an EBNF special sequence and PEG lookaheads).
- The Rust specs: `railroad_spec.rs` (the reference's parser specs of the four syntaxes), `railroad_db_spec.rs` (its DB spec, driven through `parse`) and `railroad_grammar.rs` (the quirks above, with the case ids that `cases.ts` shares) in `ferrigura-syntax/tests`; `railroad_render_spec.rs` (the reference's renderer and style specs, measured with the reference's stub, `getBBox` as the text length times 8 and 16 high) and `railroad_quirks.rs` (the renderer's quirks, each naming its geometry probe) in `ferrigura-render/tests`.

## Learnings

- **The maximum of a repetition is not drawn.** The reference passes only `min` to the renderer, so `3DIGIT`, `1*3DIGIT` and `oneOrMore(a)` are the same loop, and `*0DIGIT` is a star with its bypass. The model keeps both bounds; the renderer must not use `max`.
- **The rule name is measured regular and drawn bold.** `measureText` has no weight, so the name's width, and with it the whole rule's x offset, is that of the regular face although `.railroad-rule-name` is bold.
- **Entities are measured as placeholders and drawn decoded.** The DB holds `ﬂ°amp¶ß` for `#amp;`; the layout measures that text and the SVG shows `&`. A `&lt;` that the sanitizer keeps is drawn as `&lt;`, because only the placeholders are decoded. `shown_text` is the shared function.
- **Markup is shown as text.** The renderer writes the stored text as a text node, so `<b>x</b>` is drawn with its tags, and `&amp;` with its five characters.
- **Text boxes.** Widths are multiples of 1/64 px and kept fractional in the output (`27.375`); the height does not depend on the text; the empty text measures 0 by 0, so an empty terminal is `2 * padding` square; a line break in a value measures as a space.
- **The theme underlay.** The reference merges the default theme's variables under the active theme's, so a variable the active theme does not set takes the default theme's value: `redux-color` sets neither `secondBkg` nor `labelBackground`, and its terminals are `#ffffde`. `DEFAULT_THEME_VARIABLES` in `style.rs` holds the seventeen names the style sheet reads.
- **Inert and dead settings.** `compactMode` and `showMarkers` are read and never used (the markers are always drawn), the `railroad-comment` rules and the comment colors style no element, `.railroad-group` is empty, and `look` is not read.
- **The title is never drawn.** The accessibility title and description become `title` and `desc` with ids `chart-title-<id>` and `chart-desc-<id>`; the diagram title, and a description in the front matter, appear nowhere.
- **The DB sanitizes after the label is made.** A PEG lookahead's special is built from the unsanitized operand and sanitized afterward with the rest of the rule.
