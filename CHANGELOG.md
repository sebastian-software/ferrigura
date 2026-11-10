# Changelog

## [0.7.0](https://github.com/sebastian-software/ferrigura/compare/v0.6.0...v0.7.0) (2026-11-10)


### Features

* add a general compat option and fix diagram styling ([#85](https://github.com/sebastian-software/ferrigura/issues/85)) ([ce5d30f](https://github.com/sebastian-software/ferrigura/commit/ce5d30fed54325fefe51d13f16bb8b313114e124))
* **fonts:** render with host-supplied fonts across Rust, CLI and Node ([#94](https://github.com/sebastian-software/ferrigura/issues/94)) ([a9877dd](https://github.com/sebastian-software/ferrigura/commit/a9877dd25a537d1e1ca58f6deee86a8e5c8a1dd5))
* **render:** add optional SVG draw animation ([#86](https://github.com/sebastian-software/ferrigura/issues/86)) ([0966dcb](https://github.com/sebastian-software/ferrigura/commit/0966dcbb105aae94d06976618e659a28620937be))
* **text:** measure and embed explicit local fonts ([#93](https://github.com/sebastian-software/ferrigura/issues/93)) ([69f1b63](https://github.com/sebastian-software/ferrigura/commit/69f1b635485d91c74a3620fc4f0e0174359a7fef))


### Bug Fixes

* **fonts:** preserve regular metrics in sequence and journey ([#96](https://github.com/sebastian-software/ferrigura/issues/96)) ([01b8eb2](https://github.com/sebastian-software/ferrigura/commit/01b8eb239eb501c6a9ea553cf8b8857320044599))
* **render:** mask edge labels consistently across diagram types ([#95](https://github.com/sebastian-software/ferrigura/issues/95)) ([375fc03](https://github.com/sebastian-software/ferrigura/commit/375fc03bd7ce308eabc87f1c9d12bff6a5cc9941))
* **render:** match Mermaid's class and state colors ([8560f29](https://github.com/sebastian-software/ferrigura/commit/8560f29f09655a76950ade9bbf1f22da6a6fb17d))
* **site:** let the size table scroll on narrow screens ([f51b8ee](https://github.com/sebastian-software/ferrigura/commit/f51b8ee0a6790912ddfb7c7fe903cc2cf5830d94))

## [0.6.0](https://github.com/sebastian-software/ferrigura/compare/v0.5.0...v0.6.0) (2026-11-09)


### Features

* GitHub as the header's mark, on Ardo 6.4 ([#76](https://github.com/sebastian-software/ferrigura/issues/76)) ([bb2ae20](https://github.com/sebastian-software/ferrigura/commit/bb2ae20f1e83a4f3ae4673549ce487480ca8549d))


### Bug Fixes

* **mindmap:** size cloud and bang nodes as Chromium 158 maps their boxes ([a775fac](https://github.com/sebastian-software/ferrigura/commit/a775facf69d4a1e03e8631f0e6653239e6d872ab))
* **mindmap:** size cloud and bang nodes as Chromium 158 maps their boxes ([cd40108](https://github.com/sebastian-software/ferrigura/commit/cd40108ba7ecd0ab0e217cb70372129a7d19e916))
* **port:** check pie scopes against the page's color-scheme ([ab4f541](https://github.com/sebastian-software/ferrigura/commit/ab4f5417b38d9afbd45c779a19369a35938aa58e))
* **port:** check pie scopes against the page's color-scheme ([ef4d988](https://github.com/sebastian-software/ferrigura/commit/ef4d98895a3c156ddf63bdfa640f3d2fb8f6e6ab))

## [0.5.0](https://github.com/sebastian-software/ferrigura/compare/v0.4.0...v0.5.0) (2026-11-08)


### ⚠ BREAKING CHANGES

* **render:** an inline diagram in a page that declares no color-scheme now stays light instead of following the operating system. Pages with a dark mode should declare color-scheme.

### Features

* **homepage:** adopt the Monografie page system ([0657a0f](https://github.com/sebastian-software/ferrigura/commit/0657a0f044337f135e9c98ad54961a911351411b))
* **homepage:** adopt the Monografie page system ([bf79617](https://github.com/sebastian-software/ferrigura/commit/bf79617ea9b757b1143cd42c9e0468aa956d039d))
* **homepage:** show documentation sections in the Ferra section rail ([4a8ad09](https://github.com/sebastian-software/ferrigura/commit/4a8ad0943493cbdab5ed816a408217087d5380b2))
* **homepage:** show documentation sections in the Ferra section rail ([1f4b99c](https://github.com/sebastian-software/ferrigura/commit/1f4b99c395cdf859cd0da70c843fdd5a9e739834))
* let a program register only the diagram types it draws ([d46bc10](https://github.com/sebastian-software/ferrigura/commit/d46bc102c05500efe60e9aaab1ef98c217a868f7))
* let a program register only the diagram types it draws ([350566f](https://github.com/sebastian-software/ferrigura/commit/350566fc38aa9d2a156610dc23ada3f2c420b3d8))
* pick the diagram types of the Node addon with cargo features ([b25691b](https://github.com/sebastian-software/ferrigura/commit/b25691bbee1b5d6a32891ba01b74814f3b69d09f))
* pick the diagram types of the Node addon with cargo features ([c4ed46f](https://github.com/sebastian-software/ferrigura/commit/c4ed46f2c3f176148ff6b51ec2d424112cf06046))
* **render:** follow the page's color-scheme with light-dark() ([75527d9](https://github.com/sebastian-software/ferrigura/commit/75527d9d94dfe7b221082626fa88a054d51a8e3e))


### Performance Improvements

* **build:** cut the shipped binaries by 35 to 45 percent ([8869152](https://github.com/sebastian-software/ferrigura/commit/8869152faba508ab0b25bcf45b1e764df2af29aa))
* **build:** cut the shipped binaries by 35 to 45 percent ([1f09a90](https://github.com/sebastian-software/ferrigura/commit/1f09a903b7aee2451f01922a441b3c9846266e0b))
* **deps:** drop the regex crate for 1.2 MB smaller binaries ([8561294](https://github.com/sebastian-software/ferrigura/commit/8561294676f9d13d2dce81535185e3aacbe2fe38))
* **deps:** drop the regex crate for 1.2 MB smaller binaries ([7775af6](https://github.com/sebastian-software/ferrigura/commit/7775af65b45ab9733afbae0d7c5cf3794a3721ff))
* read only the configuration sections a diagram type renders with ([d9e5087](https://github.com/sebastian-software/ferrigura/commit/d9e508722583596fc219550fb74945f9fa2a7817))
* read only the configuration sections a diagram type renders with ([e04ebbf](https://github.com/sebastian-software/ferrigura/commit/e04ebbf8f916fbe9923a8210271e6a832b48c01c))

## [0.4.0](https://github.com/sebastian-software/ferrigura/compare/v0.3.1...v0.4.0) (2026-11-04)


### Features

* **flowchart:** improve default routing and label backgrounds ([#19](https://github.com/sebastian-software/ferrigura/issues/19)) ([3714ca8](https://github.com/sebastian-software/ferrigura/commit/3714ca8e13d9732e34eb93973e957af08b731215))

## [0.3.1](https://github.com/sebastian-software/ferrigura/compare/v0.3.0...v0.3.1) (2026-11-04)


### Bug Fixes

* **deps:** update Rust napi to v3.16.1 ([897d309](https://github.com/sebastian-software/ferrigura/commit/897d3090f505510a359cdaeaa9acbad9c186338d))

## 0.3.0 (2026-10-27)

The first public baseline. ferrigura renders the diagram text of all 33 diagram types of Mermaid 12.2.0 to SVG, in Rust and without a browser:

* the library `ferrigura` and the command-line tool `ferrigura-cli` on crates.io, and a Node.js addon with its npm packages in `node/`
* the layouts ported from their sources: ELK Layered, cose-bilkent, fcose, the tidy tree, swimlanes and Venn diagrams
* text measured with embedded fonts, so the output is the same on every machine
* parsers checked against Mermaid 12.2.0 model by model, and the geometry of the output checked against Mermaid 12.2.0 in Chromium
