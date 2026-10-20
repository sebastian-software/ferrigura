# Spike: fallback faces for emoji and the symbols block

**Question.** Which bundled face should draw the emoji (U+1F000-1FAFF, and the emoji in U+2600-27BF) and the miscellaneous symbols and dingbats (U+2600-27BF) that the bundled Lexica and Atkinson Mono faces lack, so that the geometry harness can compare the fixtures that contain them, and so that a viewer draws what was measured?

**Answer in short.** The corpus uses only eleven emoji code points, one dingbat (U+2795), and no other symbol from U+2600-27BF. Noto Sans Symbols 2 (OFL) is the right face for the symbols in any case. For the emoji, a colour COLRv0 face works in Chromium and WebKit: Twemoji Mozilla (CC BY 4.0) is the smallest choice with a known coverage gap; OpenMoji (CC BY-SA 4.0) covers everything but is share-alike. Noto Color Emoji COLRv1 has the best advance (19.92 px against Apple's 20 px) and full coverage, but WebKit draws it wrong, so it is not safe for Safari. The decisions for Sebastian are at the end.

Status: research only, nothing changed except this file; not committed. Date: 2026-10-20. Chromium 151.0.7922.34 (Playwright, launched as the harness launches it, `--font-render-hinting=none`), macOS. WebKit 26.5 (Playwright). Firefox and Linux not verified (see "Not verified").

## 1. Inventory

Scanned: 2,058 fixtures under `e2e/diagrams/**/*.mmd` and 399 probes under `tools/port/geometry/probes/**/*.mmd`, with the harness's own functions (`exclusionReason`, `uncoveredScriptReason`, run through `tsx`) for the exclusion status. A code point counts as uncovered when neither Lexica (all four styles), Atkinson Hyperlegible Mono nor the arrows face of Noto Sans Math has it in its `cmap`.

Nineteen of 2,457 files contain an uncovered code point; 45 distinct code points. Per block (code points in the drawn text, as the harness reads it: body and front-matter `title`):

| Block                                                    | Code points in the corpus                   |                                 Files | Type folders                         | Kept out of comparison today                                                                       |
| -------------------------------------------------------- | ------------------------------------------- | ------------------------------------: | ------------------------------------ | -------------------------------------------------------------------------------------------------- |
| Emoticons, U+1F600-1F64F                                 | U+1F600, U+1F60E                            |                                    13 | er-diagram 12, mindmap 1             | all 13 (`uncoveredScriptReason`; 4 ER also by look `handDrawn`)                                    |
| Misc Symbols and Pictographs, U+1F300-1F5FF              | U+1F3A8, U+1F493, U+1F4DD, U+1F4E6, U+1F534 |                                     3 | tree-view 2, treemap 1               | all 3                                                                                              |
| Transport and Map, U+1F680                               | U+1F680                                     | 1 (also in the tree-view files above) | tree-view                            | included in the 3 above                                                                            |
| Geometric Shapes Extended, U+1F7E0-1F7E1                 | U+1F7E0, U+1F7E1                            |                                     1 | treemap                              | 1                                                                                                  |
| Dingbats, U+2795 (emoji presentation)                    | U+2795                                      |                                     6 | er-diagram (group 2)                 | all 6 (4 by the CJK rule and the symbol rule; 2 by `handDrawn` and the symbol rule)                |
| Hiragana U+307C; CJK Compatibility U+3315                | U+307C, U+3315                              |                                     6 | er-diagram (group 2)                 | all 6 (CJK rule for 4, symbol rule for all 6)                                                      |
| CJK Unified Ideographs, U+4E00-9FFF                      | 28 ideographs                               |                                     2 | flowchart                            | both (`handDrawn`; CJK rule in `exclusionReason`, `common.ts:234-246`)                             |
| Box Drawing, U+2500-257F                                 | U+2500, U+2502, U+2514, U+251C              |                                     1 | treeView probe `p14-box-drawing.mmd` | none: the characters are tree syntax there, not drawn text (compared, within tolerance per STATUS) |
| U+2600-26FF                                              | none                                        |                                     0 |                                      |                                                                                                    |
| U+2700-27BF                                              | U+2795 only                                 |                                       |                                      |                                                                                                    |
| ZWJ (U+200D), VS16 (U+FE0F), keycap (U+20E3), skin tones | none                                        |                                     0 |                                      |                                                                                                    |

Totals: 18 e2e files are excluded for an uncovered code point, 16 of them by the symbol rule (`UNCOVERED_SYMBOLS`, `common.ts:194`) and 2 flowchart files by the CJK rule. Excluding `handDrawn` (4 ER files and 1 flowchart file), 13 non-handDrawn fixtures are kept out by the emoji, symbol or CJK rules: ER 8, mindmap 1, tree-view 2, treemap 1, flowchart 1. The probe is compared.

Latent gap, not in the corpus: `UNCOVERED_SYMBOLS` does not cover U+2300-23FF, U+25A0-25FF or U+2B00-2BFF, which hold emoji such as U+231A, U+23F3, U+2B50, U+25AA and U+25FB. A fixture with one of them would be compared, not excluded, and would fail. Chromium draws U+231A, U+23F3 and U+2B50 in Apple Color Emoji (20 px). Probed in section 2.

## 2. The reference: Chromium on macOS

Probe: `scratchpad/scripts/fontsinv/chromium-probe.mjs`. Each character sits in an SVG `<text>` with the reference's stack `"trebuchet ms", verdana, arial, sans-serif` at 16 px. The platform face comes from CDP `CSS.getPlatformFontsForNode`; the advance from `getComputedTextLength`. Two modes: "system" (no pin) and "harness" (the mapped families `trebuchet ms`, `verdana`, `arial`, `Helvetica Neue`, `Helvetica`, `Open Sans` each get the Lexica faces and the arrows face of Noto Sans Math under their name, as `pageHtml` does in `common.ts:413-489`).

| Group                                                                                                                                                        | Platform face (macOS, both modes unless noted)                                    | Advance at 16 px                      |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------- | ------------------------------------- |
| Emoji presentation: U+1F600, U+1F60E, U+1F3A8, U+1F493, U+1F4DD, U+1F4E6, U+1F534, U+1F680, U+1F7E0, U+1F7E1, U+2795, U+2728, U+2615, U+231A, U+23F3, U+2B50 | Apple Color Emoji (colour, sbix)                                                  | 20.000 each                           |
| Text-default symbols: U+2600, U+2605, U+26A0, U+260E                                                                                                         | Hiragino Sans                                                                     | 16.000                                |
| U+2665 heart                                                                                                                                                 | system: Arial (9.5); harness: Hiragino Sans (16.0)                                | 9.5 / 16.0                            |
| U+266A                                                                                                                                                       | system: Verdana (8.0); harness: Helvetica (8.03)                                  | 8.0 / 8.03                            |
| Dingbats, text default: U+2702, U+2713, U+2714, U+27A1, U+2764                                                                                               | Zapf Dingbats (ZapfDingbatsITC)                                                   | 15.375, 12.078, 13.547, 13.25, 14.563 |
| Box drawing U+2500, U+2502, U+2514                                                                                                                           | system: Verdana (11.34); harness: Helvetica (16.0)                                |                                       |
| Box drawing U+251C                                                                                                                                           | system: Arial (11.34); harness: Hiragino Sans (16.0)                              |                                       |
| Hiragana U+307C, CJK compatibility U+3315                                                                                                                    | Hiragino Sans                                                                     | 16.000                                |
| CJK ideograph U+4E3B                                                                                                                                         | PingFang SC                                                                       | 16.328                                |
| Trademark and copyright U+00A9, U+00AE, U+2122                                                                                                               | system: Trebuchet MS (11.41, 11.41, 10.16); harness: Lexica (12.78, 12.78, 13.58) |                                       |
| Arrow U+2192                                                                                                                                                 | system: Arial (16.0); harness: Noto Sans Math (16.203)                            |                                       |
| Outside the harness regex: U+25AA                                                                                                                            | system: Trebuchet MS (5.67); harness: Lucida Grande (8.09)                        |                                       |

Colour is used only where the code point has `Emoji_Presentation` (U+2615, U+2728, U+2795, U+231A, U+23F3, U+2B50 are colour; U+2665, U+26A0, U+2764 are not). Mapping a family changes the cascade only where that family used to supply the character: in harness mode U+2665 moves from Arial to Hiragino Sans, and the emoji still fall to Apple Color Emoji, since no mapped family covers them.

Linux (not probed; from documentation, unverified): Chromium there takes its fallback from fontconfig. Where Noto Color Emoji is installed it draws the emoji, and DejaVu or Noto Sans Symbols the symbols. The advances would then differ from these. The harness is compared on macOS only (STATUS), so these values are for macOS.

## 3. Candidate faces

Sources (official, downloaded with `curl` to `scratchpad/fonts-dl/candidates-1/`, with `SHA256SUMS.txt` beside them):

- `googlefonts/noto-emoji` HEAD (Noto Color Emoji 2.057, 2026-09-11, `2D/fonts/NotoColorEmoji.ttf` (CBDT) and `Noto-COLRv1.ttf`) and tag `v2.028` (`fonts/NotoEmoji-Regular.ttf`, the monochrome face; name table version 1.05).
- `googlefonts/noto-fonts` HEAD (`unhinted/ttf/NotoSansSymbols` 2.001, `NotoSansSymbols2` 2.003).
- `mozilla/twemoji-colr` release v0.7.0 (`Twemoji.Mozilla.ttf`, COLRv0).
- `hfg-gmuend/openmoji` HEAD (OpenMoji 18.0: `glyf_colr_0`, `glyf_colr_1`, `color-cbdt`, `black-glyf`, `color-colr1_svg`).

Not used: fonts.google.com, which returned the web application for `download?family=Noto Emoji` rather than a zip, so the current variable Noto Emoji (`NotoEmoji[wght]`) was not obtained. The monochrome row below is v2.028, which is old.

Licences, from the files in each package:

| Face                            | Licence                                                 | Notes                                                                                                                                    |
| ------------------------------- | ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Noto Color Emoji (CBDT, COLRv1) | SIL OFL 1.1 (`2D/fonts/LICENSE`)                        | Artwork source is Apache-2.0 (`2D/svg/LICENSE`).                                                                                         |
| Noto Emoji (monochrome, v2.028) | SIL OFL 1.1 (`fonts/LICENSE` at the tag)                | Old.                                                                                                                                     |
| Noto Sans Symbols, Symbols 2    | SIL OFL 1.1 (`LICENSE` in noto-fonts)                   |                                                                                                                                          |
| Twemoji Mozilla 0.7.0           | Art CC BY 4.0 (Twemoji), code Apache-2.0 (`LICENSE.md`) | Attribution. Last release 2022-10-28.                                                                                                    |
| OpenMoji 18.0                   | CC BY-SA 4.0 (`LICENSE.txt`)                            | Share-alike. Embedding a subset in an SVG that users redistribute may make that SVG an adaptation; this needs Sebastian's legal reading. |

Structure, size and skera. Each face's tables were read with a standard-library parser (`ttfinfo.py`). Subsets were cut with `skera` 0.8.0, the same crate version and the same plan as `ferrigura-text`'s `subset_characters` (no hinting, default layout features, all scripts, name records 0 to 6, legacy tables kept): scratch crate at `scratchpad/subset-probe`. Sizes in bytes; gz is gzip level 9 of the subset; the base64 column is what the `@font-face` data URL carries.

| Face                             | Format                         | upem | Covers                                                                                                                          | Inventory (11 cps)                        | Emoji_Presentation (1,219 cps) | Blocks (1,836 cps: U+1F300-1F64F, 1F680-1F6FF, 1F7E0-1F7EB, 1F900-1F9FF, 1FA70-1FAFF, 2600-27BF) |
| -------------------------------- | ------------------------------ | ---: | ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------ |
| Noto Color Emoji, CBDT           | CBDT/CBLC bitmaps, no outlines | 2048 | 1,219 of 1,219                                                                                                                  | 23,968 (gz 22,352)                        | 5,049,520 (gz 4.71 MB)         | 4,598,276                                                                                        |
| Noto Color Emoji, COLRv1         | COLRv1 + glyf + CPAL           | 1024 | 1,219 of 1,219                                                                                                                  | 14,620 (gz 9,840)                         | 4,013,612 (gz 2.32 MB)         | 2,288,684 (gz 1.43 MB)                                                                           |
| Noto Emoji v2.028, monochrome    | glyf                           | 2048 | 785 of 1,219                                                                                                                    | 4,852 (gz 3,776)                          | 390,488 (gz 276 KB)            | 372,284                                                                                          |
| OpenMoji 18.0, `glyf_colr_0`     | COLRv0 + glyf + CPAL           | 1024 | 1,219 of 1,219                                                                                                                  | 8,480 (gz 5,837)                          | 1,466,636 (gz 871 KB)          | 1,494,612 (gz 911 KB)                                                                            |
| OpenMoji 18.0, `glyf_colr_1`     | COLRv1 + glyf + CPAL           | 1024 | 1,219 of 1,219                                                                                                                  | 9,100 (gz 6,094)                          | 1,426,648 (gz 894 KB)          | 1,475,988 (gz 941 KB)                                                                            |
| OpenMoji 18.0, `color-cbdt`      | CBDT/CBLC + glyf               | 1024 | 1,219 of 1,219                                                                                                                  | 14,720 (gz 13,009)                        | 2,782,160 (gz 2.49 MB)         | 2,892,680                                                                                        |
| OpenMoji 18.0, `black-glyf`      | glyf, monochrome               | 1024 | 1,219 of 1,219                                                                                                                  | 5,828 (gz 3,916)                          | 804,032 (gz 468 KB)            | 886,848                                                                                          |
| OpenMoji 18.0, `color-colr1_svg` | COLRv1 + SVG table + glyf      | 1024 | 1,219 of 1,219                                                                                                                  | 9,100 (the SVG table is dropped by skera) |                                | 1,480,148                                                                                        |
| Twemoji Mozilla 0.7.0            | COLRv0 + glyf + CPAL           |  512 | 1,185 of 1,219 (34 missing, e.g. U+1F6D8, U+1FA75)                                                                              | 4,244 (gz 2,880)                          | 820,656 (gz 497 KB)            | 714,672                                                                                          |
| Noto Sans Symbols                | glyf, monochrome               | 1000 | 2600-26FF 135/256, 2700-27BF 35/192                                                                                             | 900, no `cmap` (0 of 11)                  | 4,804                          | 34,628; 2600-27BF alone 31,224 (gz 18.8 KB)                                                      |
| Noto Sans Symbols 2              | glyf, monochrome               | 1000 | 2600-26FF 121/256, 2700-27BF 145/192; 202 of the 1,219 emoji-presentation cps, including U+1F4E6, U+1F7E0-1F7EB, U+270B, U+2753 | 2,980 (3 of 11)                           | 90,444 (gz 51 KB)              | 240,928; 2600-27BF alone 60,052 (gz 36,955)                                                      |

Findings from the subsetting:

- skera 0.8.0 subsets every face above, COLRv0 and COLRv1 with CPAL included. It keeps CBDT/CBLC but keeps every strike, so the bitmap faces stay large: 2.8 MB for the OpenMoji CBDT emoji-presentation subset and 5.0 MB for Noto's. It does not keep the `SVG ` table: the OpenMoji `colr1_svg` subset has no SVG table, so OT-SVG cannot be embedded with this crate.
- A subset with no covered character has no `cmap` (Noto Sans Symbols for the inventory). The embedder must handle an empty family.
- Advances at 16 px from `hmtx`, and as Chromium reports them when the subset is loaded (section 5): Noto COLRv1 19.92 (Chromium 19.92); Noto CBDT 19.92 in `hmtx`, 19.97 in Chromium; OpenMoji 20.625 (both); Twemoji 16.0 (both); Noto Emoji v2.028 20.3125 (`hmtx`); Noto Sans Symbols 2 14.62 (U+2600), 15.36 (U+2605), 11.84 (U+2665), 14.82 (U+26A0), 14.27 (U+260E) (`hmtx`).

## 4. Browsers

Probe: `scratchpad/browser-probe/` (`make_pages.py` writes one page per face, with the subset embedded as `@font-face` data, an inline SVG and an `<img>` whose SVG carries the same rules; `run-browsers.mjs` screenshots each). Pixel comparison against a control page where the family is not loaded (so the system fallback draws the emoji).

| Face                              | Chromium 151, inline | Chromium 151, `<img>` | WebKit 26.5, inline                                                           | WebKit 26.5, `<img>` |
| --------------------------------- | -------------------- | --------------------- | ----------------------------------------------------------------------------- | -------------------- |
| Noto Color Emoji, CBDT            | colour               | colour                | not loaded (FontFace status `error`), so the fallback draws                   | same                 |
| Noto Color Emoji, COLRv1          | colour               | colour                | drawn wrong: flat blocky shapes, not the Noto design (checked on the montage) | same                 |
| OpenMoji, COLRv0                  | colour               | colour                | colour                                                                        | colour               |
| OpenMoji, COLRv1                  | colour               | colour                | drawn wrong: black silhouettes (base outlines, no colour)                     | same                 |
| OpenMoji, CBDT                    | colour               | colour                | loaded, nothing drawn (blank)                                                 | same                 |
| Twemoji Mozilla, COLRv0           | colour               | colour                | colour                                                                        | colour               |
| Noto Emoji v2.028, monochrome     | black                | black                 | black                                                                         | black                |
| OpenMoji black, monochrome        | black                | black                 | black                                                                         | black                |
| Noto Sans Symbols 2, text symbols | black                | black                 | black                                                                         | black                |

Inline and `<img>` are pixel-identical in both engines for every face (0 differing pixels), and Chromium's CDP output names the embedded face as the platform font in each case (for example "Noto Color Emoji [NotoColorEmoji]", "OpenMoji [OpenMojiColor]").

Documented support, not probed here, from memory and unverified: Firefox supports COLRv0, COLRv1, CBDT and OT-SVG; Safari supports sbix. WebKit 26.5, measured here, draws COLRv0 and not COLRv1 or CBDT.

- Firefox: Playwright's Firefox build (1538) did not launch in 60 s, as in the text-measurement spike. The installed Firefox 155 in headless mode failed with "Could not find profile folder" for a scratch profile. Not measured.
- Safari: probed as WebKit 26.5 (Playwright), not Safari 27.0.1 on this Mac.
- sbix: no redistributable sbix face was available; Apple Color Emoji is sbix, which is the reference's face, so it cannot be bundled.
- OT-SVG: not tested in a browser (skera drops the table).

Consequence: COLRv0 is the colour format that works in both engines probed. COLRv1 draws in Chromium only. CBDT does not work in WebKit.

## 5. Measurement parity

The harness already makes the reference measure in the bundled face: `MAPPED_FAMILIES` (`common.ts:413`) names the families the reference uses, `arrowFaces` (`common.ts:447`) adds the arrows face under each of those names with `unicode-range: U+2190-21FF`, and both sides then measure the same face. Proposed, not implemented: the same pattern for emoji and symbols. The reference page would add, for each mapped family, `@font-face` rules for the emoji face with a `unicode-range` covering the emoji-presentation code points, and the symbols face for U+2600-27BF. Ferrigura would cut runs the same way (see below).

Probe: `scratchpad/browser-probe/parity-probe.mjs`, Chromium 151, same stack, pinned as above. Advances at 16 px:

| Pinned face                              |                    U+1F600 | U+2795 |             U+2605 |                 U+2713 |             U+2665 |
| ---------------------------------------- | -------------------------: | -----: | -----------------: | ---------------------: | -----------------: |
| none (macOS system)                      | 20.000 (Apple Color Emoji) | 20.000 |  16.000 (Hiragino) | 12.234 (Lucida Grande) |      9.500 (Arial) |
| Noto Color Emoji COLRv1                  |                     19.922 | 19.922 |  16.000 (Hiragino) |                 12.234 |  16.000 (Hiragino) |
| Noto Color Emoji CBDT                    |                     19.969 | 19.969 |             16.000 |                 12.234 |             16.000 |
| OpenMoji COLRv1                          |                     20.625 | 20.625 |             16.000 |                 12.234 |             16.000 |
| Twemoji Mozilla COLRv0                   |                     16.000 | 16.000 |             16.000 |                 12.234 |             16.000 |
| Noto Sans Symbols 2 for U+2600-27BF only |             20.000 (Apple) | 20.000 | 15.375 (Symbols 2) |     13.266 (Symbols 2) | 11.844 (Symbols 2) |

Pinning works: the pinned rows draw the pinned face (CDP names it), and the emoji width equals that face's advance. So the widths can match within tolerance by construction, as the Lexica and arrows mappings already do. This changes what the comparison checks: the emoji widths then agree with the bundled face, not with Apple Color Emoji. Visually, the output differs from macOS Mermaid for every emoji by the difference in the table (Noto COLRv1 -0.08 px, OpenMoji +0.6 px, Twemoji -4 px).

Ferrigura side, for the same parity (what the code needs; nothing changed):

- `fallback_runs` (`crates/ferrigura-text/src/measure.rs:36`) splits a run where the family lacks a character and the symbol face has it; it knows one symbol family, the arrows face. It needs a second and a third fallback family, in this order: the symbols face (U+2600-27BF, non-emoji-presentation code points), then the emoji face (emoji-presentation code points). A family list whose emoji face has text-default symbols would draw U+2665 and U+26A0 in colour; the Noto and OpenMoji faces cover them (checked: Noto COLRv1 covers U+2600, U+2665, U+26A0, U+260E, U+2702, U+2714, U+2728, U+27A1, U+2764), and Chromium on macOS draws them as text. So the emoji face must be limited to the `Emoji_Presentation` code points, which the subset also needs to be.
- Order hazard: Noto Sans Symbols 2 covers 202 emoji-presentation code points, among them U+1F4E6 (in the corpus) and U+1F7E0-1F7E1 (in the corpus). If it comes first, those draw in monochrome, while the reference draws colour. Either the emoji face comes first for emoji-presentation code points, or the symbols face is restricted to the code points it draws as text.
- `embed_runs` (`crates/ferrigura-text/src/embed.rs:60`) builds `css_family` as `"sans", "symbols"` and loops `[Sans, Mono, Symbols]` (`embed.rs:65`); the enum (`fonts.rs:15-23`) and `FACES` (`fonts.rs:234`) need the new families. `Face::covers` already reads the `cmap`, so the run split can use it.
- Bundle size: measurement needs the advance table of the whole face set a diagram may use, so a source face has to be in the crate (Twemoji 0.7.0 is 1.47 MB; OpenMoji COLRv1 2.48 MB; Noto COLRv1 5.04 MB; Noto Sans Symbols 2 0.66 MB). A subset of Emoji_Presentation for Twemoji is 0.82 MB; for Symbols 2, 90 KB, and 2600-27BF alone 60 KB. Per diagram the embedded subset is the characters the diagram uses: about 0.4 KB to 0.8 KB per emoji for the COLRv0 faces, 1.3 KB for Noto COLRv1, and 1.3 KB to 2.2 KB for the CBDT faces (inventory subsets).

## 6. Options and recommendation

Corpus effect first: the 16 fixtures are excluded today. If emoji and symbols were drawn, the 13 non-handDrawn fixtures could be compared (ER 8, mindmap 1, tree-view 2, treemap 1, flowchart 1); 4 ER and 1 flowchart stay excluded by `handDrawn`. Symbol text U+2600-26FF does not occur in the corpus at all; U+2700-27BF occurs only as U+2795.

|                                  | A. Symbols only       | B. A + Twemoji COLRv0                                    | C. A + monochrome Noto Emoji                                                         | D. A + Noto Color Emoji COLRv1          |
| -------------------------------- | --------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------ | --------------------------------------- |
| Symbols (U+2600-27BF text)       | Noto Sans Symbols 2   | same                                                     | same                                                                                 | same                                    |
| Emoji                            | excluded (status quo) | colour, 1,185 of 1,219                                   | monochrome, 785 of 1,219, old version                                                | colour, 1,219 of 1,219                  |
| Size, emoji inventory            | 0                     | 4.2 KB                                                   | 4.9 KB                                                                               | 14.6 KB                                 |
| Size, all emoji-presentation cps | 0                     | 821 KB (497 KB gz)                                       | 390 KB                                                                               | 4.0 MB (2.3 MB gz)                      |
| Browsers (probed)                | all                   | Chromium and WebKit 26.5 correct                         | Chromium and WebKit correct                                                          | Chromium correct; WebKit draws it wrong |
| Advance vs Apple 20 px           |                       | 16 px (-4 px)                                            | 20.31 px (+0.3 px)                                                                   | 19.92 px (-0.08 px)                     |
| Licence                          | OFL 1.1               | CC BY 4.0 (art) with attribution; Apache-2.0 (code)      | OFL 1.1                                                                              | OFL 1.1                                 |
| Main gap                         |                       | 34 emoji-presentation cps missing; needs a rule for them | old version; 434 cps missing (incl. U+1F7E0-1F7EB); the current font was not fetched | Safari                                  |

OpenMoji COLRv0 (`glyf_colr_0`) is a fourth colour option: complete coverage, 8.5 KB for the inventory, 1.47 MB for the emoji-presentation set; CC BY-SA 4.0, see the licence note.

The CBDT options (Noto, OpenMoji) and the OpenMoji COLRv1 and sbix options fail in WebKit, so they are not candidates.

**Recommendation: B.** Noto Sans Symbols 2 for U+2600-27BF in every case (OFL, probed in both engines, no emoji-presentation code point routed to it) and Twemoji Mozilla 0.7.0 COLRv0 for the emoji. It is the only colour option that works in both engines probed, at 4 KB for the inventory. Its advance (16 px) differs from the reference by 4 px per emoji, which is the visible cost, and its 34 gaps need a decision. If the licence or the gaps are not acceptable, choose A and leave the emoji excluded: that costs no size and no licence obligation.

Decisions for Sebastian:

1. Colour or monochrome emoji. Colour (B, or OpenMoji COLRv0) matches the reference's look; monochrome (C) needs the current Noto Emoji, which I could not fetch from the listed sources.
2. Licence: accept the CC BY 4.0 attribution of Twemoji (B), or pick OpenMoji (CC BY-SA 4.0, complete coverage) and settle the share-alike question for SVG output first.
3. Parity target: match the bundled face's advances on both sides (as the Lexica and arrows mappings do; this gives exact parity and a visible difference from macOS emoji widths), or keep the reference's 20 px and accept that the emoji comparison can then not be exact.
4. The 34 emoji-presentation code points Twemoji lacks (for example U+1F6D8, U+1FA75): leave them excluded by name, or fall back to another face for them.
5. Safari. COLRv1 (Noto) draws wrong in WebKit, so it is excluded; COLRv0 works in WebKit 26.5. Whether Safari 27 behaves the same has to be checked before shipping a colour face.
6. Whether to fix the gap in `UNCOVERED_SYMBOLS` now (U+2300-23FF, U+25A0-25FF, U+2B00-2BFF) so that a future fixture is excluded rather than failing.

## Not verified

- Firefox (both the Playwright build and the installed 155): not measured. Documented support only.
- Safari 27.0.1 (only WebKit 26.5 was measured).
- Linux Chromium: documentation only, not probed.
- The current monochrome Noto Emoji (`NotoEmoji[wght]`): not obtained from the listed sources; v2.028 was measured instead.
- sbix (Apple Color Emoji): not redistributable, not measured beyond Chromium's advance and platform face.
- OT-SVG: skera drops the SVG table, and no browser was tested with a full OT-SVG face.
- WOFF2 sizes: not evaluated (the crate embeds TTF).
- The full harness run with pinned emoji: not run. Nothing in the repository was changed.
- The WebKit SVG-text width caveat in `spikes/text-measurement.md` (kerned widths in `getComputedTextLength`) was not re-checked for the emoji faces.
- The probe's "harness" mode approximates `pageHtml` (the Lexica and arrows rules only), not the full page.

## Reproduce

Scratchpad (session-specific, not committed): `scratchpad/scripts/fontsinv/` (`cmap.py`, `ttfinfo.py`, `inventory.py`, `harness-exclusions.ts`, `chromium-probe.mjs`, `blocks.py`), `scratchpad/subset-probe/` (skera subsetter; outputs in `scratchpad/subset-out/`, sizes in `sizes.txt`), `scratchpad/browser-probe/` (`make_pages.py`, `run-browsers.mjs`, `parity-probe.mjs`, `montage.html`), downloads in `scratchpad/fonts-dl/candidates-1/`.
