# Notice: the bundled fonts

The crate's sources are MIT-licensed, as the workspace is. The fonts in `fonts/` are third-party
works under the licences below. Each font file is a cut of its source (only some code points, the
hinting dropped, some name records kept); the cut is made by `pnpm port:cut-fonts`
(`tools/port/fonts/cut-faces.ts`), which checks each source's SHA-256 first and writes the cut with
`cut_font` (`examples/cut_font.rs`).

| Bundled file                               | What it draws                                                               | Source and version                                                                                                                                                                                         | Licence                                                           | SHA-256 of the source                                              | SHA-256 of the bundled file                                        |
| ------------------------------------------ | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------ |
| `TwemojiMozilla-COLRv0-Emoji.ttf`          | Emoji with emoji presentation (1,185 of the 1,219 the Unicode property has) | Twemoji Mozilla 0.7.0, `twemoji-colr/Twemoji.Mozilla.v0.7.0.ttf` from <https://github.com/mozilla/twemoji-colr/releases/tag/v0.7.0>, last released 2022-10-28                                              | Art CC BY 4.0, code Apache-2.0: `fonts/LICENSE-TwemojiMozilla.md` | `6d90152ee0d29e82fe2a87793af5aa4b7ad13e6538360889e141e81ed299ee8e` | `ac0cb417936739fd190631e30a7c2e212b185d3122a0c2a6284331200756e143` |
| `NotoSansSymbols2-Regular-TextSymbols.ttf` | Text-default symbols of U+2600-27BF (252 code points)                       | Noto Sans Symbols 2, version 2.003, `unhinted/ttf/NotoSansSymbols2/NotoSansSymbols2-Regular.ttf` from <https://github.com/googlefonts/noto-fonts>, as it stood on 2026-10-20 (the commit was not recorded) | SIL Open Font License 1.1: `fonts/OFL-NotoSansSymbols2.txt`       | `882d142b9a1ef3fd7fa4225dbe95c10fab6664206eb4964c8ff705a4f6d02988` | `d65509d5d2637602bb1458df8bd71ccb0f67be0520f0926658904fe514fe23a4` |
| `NotoSansMath-Regular-Arrows.ttf`          | The arrows, U+2190-21FF (112 glyphs)                                        | Noto Sans Math 3.000, `googlefonts/ttf/NotoSansMath-Regular.ttf`, see `fonts/OFL-NotoSansMath.txt`                                                                                                         | SIL Open Font License 1.1: `fonts/OFL-NotoSansMath.txt`           | `fe380412e05c2cca4b712644c58ed38ed7b1e3153bc483a8b38725fa487b8830` | `e58bb5269af4dde2fb5fec207f9794b227ddae597dd56a50fd86224d72d2ed36` |

The text faces, Lexica Ultralegible and Atkinson Hyperlegible Mono, are credited in
`fonts/OFL-Lexica.txt` and `fonts/OFL-AtkinsonMono.txt`.

## Attribution

Emoji art in `TwemojiMozilla-COLRv0-Emoji.ttf` comes from Twemoji (<https://twitter.github.io/twemoji>),
used and redistributed under the Creative Commons Attribution 4.0 International licence
(<https://creativecommons.org/licenses/by/4.0/>), as the Twemoji project offers it. The font's code
(the build of Twemoji Mozilla) is Copyright 2016-2018 Mozilla Foundation, under the Apache License,
Version 2.0. The licence file of the build is `fonts/LICENSE-TwemojiMozilla.md`.

Changes made to the Twemoji art, as CC BY 4.0 asks them to be indicated: the font holds only the
1,185 emoji with emoji presentation that the source covers, its hinting is dropped, and its name
records are cut to 0 to 6, 13 and 14. The colour tables (COLR, CPAL) and the glyph outlines are
otherwise as the source has them.

The Noto Sans Symbols 2 cut is changed in the same way: 252 code points, no hinting, name records 0
to 6, 13 and 14. Its name table reads "Copyright 2017 Google Inc. All Rights Reserved."; its
licence text is `fonts/OFL-NotoSansSymbols2.txt`, which begins with the copyright line of the
package.

## Subsetting at embedding

A diagram's labels embed a subset of these faces (`embed_runs`) under a family name of their own.
A subset is a cut of the bundled file and is under the same licence as the file. The embedded
subsets hold the glyphs a diagram draws, not the whole file.
