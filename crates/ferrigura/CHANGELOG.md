# Changelog

## [0.2.0](https://github.com/sebastian-software/ferrigura/compare/ferrigura-v0.1.0...ferrigura-v0.2.0) (2026-10-22)


### Features

* **port:** accept flowchart-elk and the -beta keywords, as upstream Mermaid does ([d465e8d](https://github.com/sebastian-software/ferrigura/commit/d465e8d52b515c1bcdf0544f60762078abc83720))
* **port:** take the reference from REFERENCE_ROOT ([457fdce](https://github.com/sebastian-software/ferrigura/commit/457fdce94aa20e4993c6a25e863f31b0e8e2ebda))


### Bug Fixes

* **port:** architecture is a name after the header, as in upstream Mermaid ([a80f23a](https://github.com/sebastian-software/ferrigura/commit/a80f23aefd8d414bade91796c3dd7e6e39963808))

## 0.1.0 (2026-10-21)


### Features

* **port:** ferrigura re-exports Value ([fb9ebd6](https://github.com/sebastian-software/ferrigura/commit/fb9ebd6ecc0e9e47156dca75e3508b19e4d9f55a))
* **port:** ferrigura renders the 33 diagram types of Mermaid 12.2.0 to SVG, without a browser ([ae0a7e5](https://github.com/sebastian-software/ferrigura/commit/ae0a7e52e6d97cf11b03a58e8444cf69906e7dba))
* **port:** light, dark and automatic colour schemes, the automatic one with both colour sets ([ae0a7e5](https://github.com/sebastian-software/ferrigura/commit/ae0a7e52e6d97cf11b03a58e8444cf69906e7dba))
* **port:** text is measured in bundled fonts, and the SVG embeds the glyphs it draws, emoji and symbols included ([ae0a7e5](https://github.com/sebastian-software/ferrigura/commit/ae0a7e52e6d97cf11b03a58e8444cf69906e7dba))
* **port:** the command-line tool ferrigura, in the crate ferrigura-cli ([ae0a7e5](https://github.com/sebastian-software/ferrigura/commit/ae0a7e52e6d97cf11b03a58e8444cf69906e7dba))
* **port:** the Renderer resolves a configuration once and renders on several threads at once ([ae0a7e5](https://github.com/sebastian-software/ferrigura/commit/ae0a7e52e6d97cf11b03a58e8444cf69906e7dba))
