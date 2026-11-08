# Changelog

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
